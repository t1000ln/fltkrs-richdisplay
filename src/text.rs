//! 用于展示纯文本的组件。

use std::cell::RefCell;
use std::cmp::{max, min};
use std::rc::Rc;
use fltk::enums::{Align, Color, Font, FrameType};
use fltk::output::{Output};
use fltk::prelude::{InputExt, WidgetBase, WidgetExt};
use fltk::{widget_extends};
use fltk::draw::{descent, draw_rounded_rectf, draw_text2, measure, set_draw_color, set_font};
use fltk::widget::Widget;
use crate::lined_flow::{EndPos, HorizontalStretch, UnitShape};

#[derive(Clone)]
pub struct TextUnit {
    pub inner: Output,
    pub suggest_pos: Rc<RefCell<EndPos>>,
    pub max_width: Rc<RefCell<i32>>,
    pub end_pos: Rc<RefCell<EndPos>>,
}
widget_extends!(TextUnit, Output, inner);

impl TextUnit {
    pub fn new<T>(x: i32, y: i32, w: i32, h: i32, title: T) -> Self
        where T: Into<Option<&'static str>> + Clone {
        let mut inner = Output::new(x, y, w, h, None);

        inner.set_frame(FrameType::FlatBox);
        inner.set_color(Color::Black);
        inner.set_text_color(Color::White);

        let max_width = Rc::new(RefCell::new(1));

        let suggest_pos = Rc::new(RefCell::new(EndPos { x: 0, y: 0, line_height: 1, line_spacing: 0 }));
        let end_pos = Rc::new(RefCell::new(EndPos { x: 0, y: 0, line_height: 1, line_spacing: 0 }));

        inner.draw({
            let sug_rc = suggest_pos.clone();
            let mut end_rc = end_pos.clone();
            let max_width_rc = max_width.clone();
            move |ctx| {
                let value = ctx.value();
                let (x, y, sug_line_height, sug_line_spacing) = sug_rc.borrow().tuple();
                set_font(ctx.text_font(), ctx.text_size());

                let (_, mh) = measure("A", false);
                // if mh > sug_line_height {
                //     end_rc.borrow_mut().line_height = mh;
                // }
                let line_spacing = min(sug_line_spacing, descent());

                value.split_inclusive("\n").for_each(|line| {
                    let (mw, _) = measure(line, false);

                    println!("line: {}, mw: {}, max_width: {}", line, mw, max_width_rc.borrow());

                    if mw + sug_rc.borrow().x > *max_width_rc.borrow() {
                        println!("draw 需要处理超宽")
                    } else {
                        println!("draw 不需要处理超宽");
                        set_draw_color(ctx.color());
                        draw_rounded_rectf(x, y - line_spacing, mw, mh, 4);

                        set_draw_color(ctx.text_color());
                        // 绘制文本
                        draw_text2(line, x, y, mw, ctx.text_size(), Align::Left);

                        // if line.ends_with("\n") {
                        //     end_rc.borrow_mut().x = 0;
                        //     end_rc.borrow_mut().y += end_rc.borrow().line_height;
                        //     end_rc.borrow_mut().line_height = 1;
                        // } else {
                        //     end_rc.borrow_mut().x += mw;
                        // }
                    }
                });
            }
        });

        Self { inner, suggest_pos, max_width, end_pos }
    }

    // pub fn set_font(&mut self, font: Font, font_size: i32) {
    //     self.font_size = font_size;
    //     self.font = font;
    //     self.inner.set_text_font(font);
    //     self.inner.set_font_size(font_size);
    // }

    pub fn evaluate(&mut self, shape: &mut UnitShape) {
        let value = self.value();
        let (x, y, sug_line_height, sug_line_spacing) = shape.end_pos.tuple();
        set_font(self.text_font(), self.text_size());

        let (_, mh) = measure("A", false);
        if mh > sug_line_height {
            shape.end_pos.line_height = mh;
        }
        let line_spacing = min(sug_line_spacing, descent());

        let mut width = 0;
        let mut height = 0;
        value.split_inclusive("\n").for_each(|line| {

            let (mw, _) = measure(line, false);
            if mw + shape.end_pos.x > *self.max_width.borrow() {
                println!("evaluate 需要处理超宽")
            } else {
                println!("evaluate 不需要处理超宽");
                if line.ends_with("\n") {
                    shape.end_pos.x = 0;
                    shape.end_pos.y += shape.end_pos.line_height;
                    shape.end_pos.line_height = 1;

                } else {
                    shape.end_pos.x += mw;
                }
            }
            height += mh;
            width = max(width, mw);
        });
        shape.w = width;
        shape.h = height;
    }

}

impl HorizontalStretch for TextUnit {
    fn stretch(&mut self, suggest_pos: &mut UnitShape, max_width: i32) {
        self.max_width.replace(max_width);
        self.suggest_pos.replace(suggest_pos.end_pos.clone());

        let x = suggest_pos.end_pos.x;
        let y = suggest_pos.end_pos.y;
        self.evaluate(suggest_pos);
        self.resize(x, y, suggest_pos.w, suggest_pos.h);
    }

    fn line_height(&mut self) -> i32 {
        todo!()
    }

    fn widget(&self) -> Widget {
        self.as_base_widget()
    }
}
