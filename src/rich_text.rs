//! 富文本编辑器组件。



use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use fltk::draw::draw_rect_fill;
use fltk::enums::{Color, Event, Font};
use fltk::frame::Frame;
use fltk::group::{Scroll, ScrollType};
use fltk::prelude::{GroupExt, WidgetBase, WidgetExt};
use fltk::{widget_extends};
use crate::{LinedData, LinePiece, PADDING, RichData, UserData};


pub struct RichText {
    scroller: Scroll,
    panel: Frame,
    data_buffer: Rc<RefCell<VecDeque<RichData>>>,
    background_color: Rc<RefCell<Color>>,
    /// 自动滚动到底部的标记
    auto_scroll: Rc<RefCell<bool>>,

    buffer_max_lines: usize,
}
widget_extends!(RichText, Scroll, scroller);


impl RichText {
    pub fn new<T>(x: i32, y: i32, w: i32, h: i32, title: T) -> Self
        where T: Into<Option<&'static str>> + Clone {
        let mut scroller = Scroll::new(x, y, w, h, title);
        scroller.set_type(ScrollType::Vertical);
        scroller.set_scrollbar_size(Self::SCROLL_BAR_WIDTH);

        let mut panel = Frame::default().size_of_parent().center_of_parent();

        scroller.end();
        scroller.resizable(&panel);
        scroller.set_clip_children(true);

        let auto_scroll = Rc::new(RefCell::new(false));

        let data_buffer = Rc::new(RefCell::new(VecDeque::<RichData>::with_capacity(200)));
        let background_color = Rc::new(RefCell::new(Color::Black));

        let buffer_max_lines = 100;

        /*
        利用tokio::spawn的协程特性，在app主线程中异步执行滚动操作。
        在非主线程的其他独立线程中，是无法执行滚动操作的。
         */
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<i32>(10);
        let mut scroll_rc = scroller.clone();
        tokio::spawn(async move {
            while let Some(y) = receiver.recv().await {
                scroll_rc.scroll_to(0, y);
            }
        });

        panel.draw({
            let data_buffer_rc = data_buffer.clone();
            let scroll_rc = scroller.clone();
            let bg_rc = background_color.clone();
            let auto_scroll_rc = auto_scroll.clone();
            move |ctx| {
                // 滚动条滚动的高度在0到(panel.height - scroll.height)之间。
                let base_y = scroll_rc.yposition();
                // println!("base_y: {}", base_y);
                let window_width = scroll_rc.width();
                let window_height = scroll_rc.height();
                let panel_height = ctx.height();
                let (mut top_y, mut bottom_y) = (base_y, base_y + window_height);

                if auto_scroll_rc.replace(false) {
                    bottom_y = panel_height;
                    top_y = panel_height - window_height;
                    let sender = sender.clone();
                    tokio::spawn(async move {
                        if let Err(e) = sender.send(top_y).await {
                            println!("发送滚动信号失败！{:?}", e);
                        }
                    });
                } else {
                    if top_y < 0 {
                        top_y = 0;
                        let sender = sender.clone();
                        tokio::spawn(async move {
                            if let Err(e) = sender.send(0).await {
                                println!("发送滚动信号失败！{:?}", e);
                            }
                        });
                    } else if top_y > panel_height - window_height {
                        top_y = panel_height - window_height;
                        let sender = sender.clone();
                        tokio::spawn(async move {
                            if let Err(e) = sender.send(top_y).await {
                                println!("发送滚动信号失败！{:?}", e);
                            }
                        });
                    }
                }
                let offset_y = top_y - PADDING.top;

                // 填充黑色背景
                draw_rect_fill(0, 0, window_width, window_height, *bg_rc.borrow());

                let mut data = data_buffer_rc.borrow_mut();

                /*
                先试算出可显示的行，再真正绘制可显示的行。
                试算从数据队列的尾部向头部取数，试算位置从窗口底部向顶部堆积。
                 */
                let (mut from_index, mut to_index, total_len) = (0, data.len(), data.len());
                let mut set_to_index = false;
                let mut begin_check_from_index = false;
                for (seq, rich_data) in data.iter_mut().rev().enumerate() {
                    if !set_to_index && rich_data.is_visible(top_y, bottom_y) {
                        // 待绘制的内容超出窗口底部边界
                        to_index = total_len - seq;
                        set_to_index = true;
                        begin_check_from_index = true;
                    }

                    if begin_check_from_index && !rich_data.is_visible(top_y, bottom_y) {
                        // 待绘制内容已经向上超出窗口顶部边界，可以停止处理前面的数据了。
                        from_index = total_len - seq;
                        break;
                    }
                }

                for rich_data in data.range_mut(from_index..to_index) {
                    rich_data.draw(offset_y);
                }
            }
        });

        /*
        跟随新增行自动滚动到最底部。
         */
        scroller.handle({
            let mut panel_rc = panel.clone();
            // let total_height_rc = total_height.clone();
            let buffer_rc = data_buffer.clone();
            let last_window_size = Rc::new(RefCell::new((0, 0)));
            move |scroller, evt| {
                match evt {
                    Event::Resize => {
                        let (current_width, current_height) = (scroller.width(), scroller.height());
                        let (last_width, last_height) = *last_window_size.borrow();
                        if last_width != current_width || last_height != current_height {
                            last_window_size.replace((current_width, current_height));
                            let window_width = scroller.width();
                            let window_height = scroller.height();
                            let drawable_max_width = window_width - PADDING.left - PADDING.right;
                            let mut init_piece = LinePiece::init_piece();
                            let mut last_piece = &mut init_piece;
                            for rich_data in buffer_rc.borrow_mut().iter_mut() {
                                rich_data.line_pieces.clear();
                                rich_data.estimate(last_piece, drawable_max_width);
                                if let Some(piece) = rich_data.line_pieces.iter_mut().last() {
                                    last_piece = piece;
                                }
                            }

                            if let Some(rich_data) = buffer_rc.borrow().iter().last() {
                                if let Some(piece) = rich_data.line_pieces.last() {
                                    let new_content_height = piece.y + piece.h + piece.spacing;
                                    let new_total_height = new_content_height + PADDING.top + PADDING.bottom;

                                    if new_total_height > window_height {
                                        panel_rc.resize(panel_rc.x(), scroller.y(), window_width, new_total_height);
                                    } else {
                                        panel_rc.resize(panel_rc.x(), scroller.y(), window_width, window_height);
                                    }
                                }
                            }
                        }
                    }

                    _ => {}
                }
                false
            }
        });


        Self { scroller, panel, data_buffer, background_color, auto_scroll, buffer_max_lines }
    }

    /// 向数据缓冲区中添加新的数据。新增数据时会计算其绘制所需信息，包括起始坐标和高度等。
    ///
    /// # Arguments
    ///
    /// * `rich_data`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn append(&mut self, user_data: UserData) {
        let mut rich_data: RichData = user_data.into();
        let window_width = self.scroller.width();
        let window_height = self.scroller.height();
        let drawable_max_width = window_width - PADDING.left - PADDING.right;

        /*
        试算单元绘制信息
         */
        if !self.data_buffer.borrow().is_empty() {
            if let Some(rd) = self.data_buffer.borrow_mut().iter_mut().last() {
                if let Some(last_piece) = rd.line_pieces.iter_mut().last() {
                    rich_data.estimate(last_piece, drawable_max_width);
                }
            }
        } else {
            // 首次添加数据
            let mut last_piece = LinePiece::init_piece();
            rich_data.estimate(&mut last_piece, drawable_max_width);
        }

        if let Some(piece) = rich_data.line_pieces.last() {
            let new_content_height = piece.y + piece.h + piece.spacing;
            let new_total_height = new_content_height + PADDING.top + PADDING.bottom;
            if new_total_height > window_height {
                self.panel.resize(self.panel.x(), self.y(), self.panel.width(), new_total_height);
                self.auto_scroll.replace(true);
            }
        }

        self.data_buffer.borrow_mut().push_back(rich_data);

        self.panel.redraw();

        // self.scroller.redraw();
    }

    pub fn set_background_color(&mut self, background_color: Color) {
        self.background_color.replace(background_color);
    }

    pub fn set_buffer_max_lines(&mut self, max_lines: usize) {
        self.buffer_max_lines = max_lines;
    }
}

pub enum GlobalMessage {
    UpdatePanel,
    ScrollToBottom,
    ContentData(UserData),
}


#[cfg(test)]
mod tests {
    use std::time::Duration;

    use fltk::{app, window};
    use fltk::prelude::{GroupExt, WidgetExt, WindowExt};
    use super::*;

    #[tokio::test]
    pub async fn test_run_app() {
        let app = app::App::default();
        let mut win = window::Window::default()
            .with_size(800, 400)
            .with_label("draw by notice")
            .center_screen();
        win.make_resizable(true);

        let mut rich_text = RichText::new(0, 0, 800, 400, None).size_of_parent();

        let (global_sender, mut global_receiver) = app::channel::<GlobalMessage>();

        let (data_sender, data_receiver) = tokio::sync::mpsc::channel::<RichData>(64);

        tokio::spawn(async move {
            for _ in 0..1000 {
                let mut data: VecDeque<UserData> = VecDeque::from([
                    UserData::new_text("安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。\r\n".to_string()),
                    UserData::new_text("在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程。类似地，程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。\r\n".to_string()).set_font(Font::HelveticaItalic, 32),
                    UserData::new_text("由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                    UserData::new_text("由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                    UserData::new_text("安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。\r\n".to_string()),
                    UserData::new_text("在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程。类似地，程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。\r\n".to_string()).set_font(Font::HelveticaItalic, 32),
                    UserData::new_text("由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                    UserData::new_text("由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                    UserData::new_text("安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。\r\n".to_string()),
                    UserData::new_text("在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程。类似地，程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。\r\n".to_string()).set_font(Font::HelveticaItalic, 32),
                    UserData::new_text("由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                    UserData::new_text("由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                ]);
                while let Some(data_unit) = data.pop_front() {
                    global_sender.send(GlobalMessage::ContentData(data_unit));
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }

            println!("Sender closed");
        });


        win.end();
        win.show();

        while app.wait() {
            if let Some(msg) = global_receiver.recv() {
                match msg {
                    GlobalMessage::UpdatePanel => {
                        rich_text.redraw();
                    }
                    GlobalMessage::ContentData(data) => {
                        rich_text.append(data);
                    }
                    _ => {}
                }
            }
            app::sleep(0.016);
            app::awake();
        }
    }
}
