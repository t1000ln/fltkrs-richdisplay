//! 从左到右、从上到下的类似瀑布流布局管理器。

use std::cell::RefCell;
use std::collections::VecDeque;
use std::ops::Deref;
use std::rc::Rc;
use fltk::draw::{Coord, measure, set_font};
use fltk::enums::{Color, Event, FrameType};
use fltk::group::Group;
use fltk::output::Output;
use fltk::prelude::{GroupExt, InputExt, WidgetBase, WidgetExt};
use fltk::widget::Widget;
use fltk::widget_extends;
use crate::rich_text::{DataType, RichData};

#[derive(Debug, Clone)]
pub struct Padding {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}
impl Default for Padding {
    fn default() -> Self {
        Self {
            left: 1,
            top: 1,
            right: 1,
            bottom: 1,
        }
    }
}
impl Padding {
    pub fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }
}

#[derive(Debug, Default)]
pub struct Gap {
    h: i32,
    v: i32,
}

#[derive(Debug, Clone)]
pub struct EndPos {
    pub x: i32,
    pub y: i32,
    pub line_height: i32,
    pub line_spacing: i32,
}
impl EndPos {
    pub fn tuple(&self) -> (i32, i32, i32, i32) {
        (self.x, self.y, self.line_height, self.line_spacing)
    }
}

pub struct UnitShape {
    pub w: i32,
    pub h: i32,
    pub end_pos: EndPos,
}

pub trait HorizontalStretch {

    /// 从左向右拉伸。
    ///
    /// # Arguments
    ///
    /// * `max_width`: 最大宽度。
    ///
    /// returns: UnitShape - 返回组件拉伸后的长宽及右下角的x/y坐标。对于文本型组件，这个坐标并非组件矩形区域右下角，而是文本末尾位置的坐标。
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn stretch(&mut self, suggest_pos: &mut UnitShape, max_width: i32);

    /// 组件的单行高度。
    fn line_height(&mut self) -> i32;

    fn widget(&self) -> Widget;
}


pub struct LinedFlow {
    pub inner: Group,
    pub padding: Padding,
    pub gap: Gap,
    pub current_line_height: i32,
    pub next_start_coord: EndPos,
    pub buffer: Rc<RefCell<VecDeque<RichData>>>,
}

widget_extends!(LinedFlow, Group, inner);

impl LinedFlow {
    pub fn new<T>(x: i32, y: i32, w: i32, h: i32, title: T) -> Self
        where T: Into<Option<&'static str>> + Clone{
        let mut inner = Group::new(x, y, w, h, title);
        inner.set_frame(FrameType::FlatBox);
        inner.set_clip_children(true);

        let padding = Padding::default();
        let gap = Gap::default();
        let current_line_height = 1;
        // let next_start_coord = UnitShape {w: 1, h: 1, end_pos: EndPos {x: padding.left, y: padding.top, line_height: 1, line_spacing: 0}};
        let next_start_coord = EndPos {x: padding.left, y: padding.top, line_height: 1, line_spacing: 0};

        let buffer = Rc::new(RefCell::new(VecDeque::with_capacity(2000))) ;

        inner.handle({
            let buffer_rc = buffer.clone();
            let last_width = Rc::new(RefCell::new(w));
            move |grp, evt| {
                match evt {
                    Event::Resize => {
                        if *last_width.borrow()!= grp.width() {
                            println!("LinedFlow::handle resize to {}:{}", grp.width(), grp.height());
                            *last_width.borrow_mut() = grp.width();
                        }
                        // grp.clear();
                        // buffer_rc.borrow().iter().for_each({
                        //     move |rd| {
                        //
                        //     }
                        // });
                    }
                    _ => {}
                }
                false
            }
        });

        Self {
            inner,
            padding,
            gap,
            current_line_height,
            next_start_coord,
            buffer,
        }
    }

    pub fn append<T>(&mut self, wid: &mut T) where T: HorizontalStretch + Clone {
        // let max_width = self.inner.parent().unwrap().width() - self.padding.left - self.padding.right;
        // wid.stretch(&mut self.next_start_coord, max_width);
        // // wid.resize(self.next_start_coord.x, self.next_start_coord.y, self.next_start_coord.y, shape.w, shape.h);
        // // self.next_start_coord.x = shape.end_pos.x + self.padding.left + self.gap.h;
        // // self.next_start_coord.y += shape.end_pos.y;
        // let wid_ext = wid.widget();
        // self.add(&wid_ext);
    }

    pub fn append_text(&mut self, rich_data: RichData) {
        self.buffer.borrow_mut().push_back(rich_data.clone());
        self.append_child(rich_data);
    }

    pub fn append_child(&mut self, rich_data: RichData) {
        let max_width = self.inner.parent().unwrap().width() - self.padding.left - self.padding.right;
        match rich_data.data_type {
            DataType::Text => {
                let text = rich_data.text;
                set_font(rich_data.font, rich_data.font_size);
                text.split_inclusive("\n").for_each(|line| {
                    let (line_width, line_height) = measure(line, false);

                    if line_height > self.current_line_height {
                        self.current_line_height = line_height;
                    }

                    if self.next_start_coord.x + line_width > max_width {
                        println!("需要处理截断换行");
                    } else {
                        println!("无需要处理截断换行：{}", line);
                        let mut o = Output::new(self.next_start_coord.x + self.padding.left, self.next_start_coord.y, line_width, line_height, None);
                        o.set_frame(FrameType::FlatBox);
                        o.set_readonly(true);
                        o.set_tab_nav(false);
                        o.clear_visible_focus();
                        if let Some(bg_color) = rich_data.bg_color {
                            o.set_color(bg_color);
                        } else {
                            o.set_color(Color::Black);
                        }
                        o.set_text_font(rich_data.font);
                        o.set_text_size(rich_data.font_size);
                        o.set_text_color(rich_data.fg_color);
                        o.set_value(line);
                        self.add(&o);

                        self.next_start_coord.x = self.next_start_coord.x + line_width + self.gap.v;
                        if line.ends_with("\n") {
                            self.next_start_coord.x = self.padding.left;
                            self.next_start_coord.y += line_height + self.gap.h;
                            self.current_line_height = 1;
                        }
                    }
                });
            }
            _ => {

            }
        }
    }

    pub fn set_padding(&mut self, padding: (i32, i32, i32, i32)) {
        self.padding.left = padding.0;
        self.padding.top = padding.1;
        self.padding.right = padding.2;
        self.padding.bottom = padding.3;

        self.next_start_coord.x = self.padding.left;
        self.next_start_coord.y = self.padding.top;
    }
}

#[cfg(test)]
mod tests {
    use fltk::{app, window};
    use fltk::enums::{Color, Font};
    use fltk::output::MultilineOutput;
    use fltk::prelude::{GroupExt, InputExt, WidgetBase, WindowExt};
    use crate::text::TextUnit;
    use super::*;

    #[tokio::test]
    pub async fn test_flow() {
        let app = app::App::default();
        let mut win = window::Window::default()
            .with_size(800, 400)
            .with_label("draw by notice")
            .center_screen();
        win.make_resizable(true);

        let mut flow = LinedFlow::new(0, 0, 1, 1, None).size_of_parent();
        flow.set_color(Color::Black);
        flow.set_padding((5, 5, 5, 5));
        flow.end();
        win.end();
        win.show();

        let d1 = RichData::new_text("安全并且高效地处理并发编程".to_string());
        let d2 = RichData::new_text("是Rust的另一个主要目标。".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)).set_font(Font::HelveticaBold, 18);
        flow.append_text(d1);
        flow.append_text(d2);
        while app.wait() {
            app::sleep(0.016);
            app::awake();
        }
    }
}
