//! 内容源自rich_text的快照，可滚动的浏览的组件。

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use fltk::draw::draw_rect_fill;
use fltk::enums::{Color, Event};
use fltk::frame::Frame;
use fltk::group::{Scroll, ScrollType};
use fltk::prelude::{GroupExt, WidgetBase, WidgetExt};
use fltk::widget_extends;
use crate::{LinedData, LinePiece, PADDING};
use crate::rich_text::RichData;

pub struct RichSnapshot {
    scroller: Scroll,
    panel: Frame,
    data_buffer: Rc<RefCell<VecDeque<RichData>>>,
    background_color: Rc<RefCell<Color>>,
    /// 自动滚动到底部的标记
    auto_scroll: Rc<RefCell<bool>>,
}
widget_extends!(RichSnapshot, Scroll, scroller);

impl RichSnapshot {
    pub const SCROLL_BAR_WIDTH: i32 = 10;
    pub const PANEL_MAX_HEIGHT: i32 = 10;

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


        Self { scroller, panel, data_buffer, background_color, auto_scroll, }
    }
}
