//! 内容源自rich_text的快照，可滚动的浏览的组件。

use std::cell::RefCell;
use std::collections::{HashMap};
use std::ops::Deref;
use std::rc::Rc;
use fltk::draw::draw_rect_fill;
use fltk::enums::{Align, Color, Cursor, Event};
use fltk::frame::Frame;
use fltk::group::{Scroll, ScrollType};
use fltk::prelude::{GroupExt, WidgetBase, WidgetExt};
use fltk::{app, draw, widget_extends};
use crate::{Coordinates, LinedData, LinePiece, PADDING, RichData, UserData};

#[derive(Clone, Debug)]
pub struct RichReviewer {
    pub(crate) scroller: Scroll,
    panel: Frame,
    data_buffer: Rc<RefCell<[RichData]>>,
    background_color: Rc<RefCell<Color>>,
}
widget_extends!(RichReviewer, Scroll, scroller);

impl RichReviewer {
    pub const SCROLL_BAR_WIDTH: i32 = 10;
    pub const PANEL_MAX_HEIGHT: i32 = 10;

    pub fn new<T>(x: i32, y: i32, w: i32, h: i32, title: T) -> Self
        where T: Into<Option<&'static str>> + Clone {
        let mut scroller = Scroll::new(x, y, w, h, title);
        scroller.set_type(ScrollType::Vertical);
        scroller.set_scrollbar_size(Self::SCROLL_BAR_WIDTH);
        scroller.set_align(Align::Bottom);

        let mut panel = Frame::new(x, y, w, h, None);

        scroller.end();
        // scroller.resizable(&panel);
        // scroller.set_clip_children(true);

        let data_buffer: Rc<RefCell<[RichData]>> = Rc::new(RefCell::new([]));
        let background_color = Rc::new(RefCell::new(Color::Black));
        let visible_lines = Rc::new(RefCell::new(HashMap::<Coordinates, usize>::new()));
        let notifier: Rc<RefCell<Option<tokio::sync::mpsc::Sender<UserData>>>> = Rc::new(RefCell::new(None));

        // panel.draw({
        //     let data_buffer_rc = data_buffer.clone();
        //     let scroll_rc = scroller.clone();
        //     let visible_lines_rc = visible_lines.clone();
        //     let bg_rc = background_color.clone();
        //     move |ctx| {
        //         // 滚动条滚动的高度在0到(panel.height - scroll.height)之间。
        //         let base_y = scroll_rc.yposition();
        //         let window_width = scroll_rc.width();
        //         let window_height = scroll_rc.height();
        //         let (mut top_y, mut bottom_y) = (base_y, base_y + window_height);
        //
        //         let offset_y = top_y - PADDING.top;
        //
        //         // 填充黑色背景
        //         draw_rect_fill(0, 0, window_width, window_height, *bg_rc.borrow());
        //
        //         let tmp = data_buffer_rc.borrow_mut();
        //         let mut data = tmp.as_ref();
        //
        //         /*
        //         先试算出可显示的行，再真正绘制可显示的行。
        //         试算从数据队列的尾部向头部取数，试算位置从窗口底部向顶部堆积。
        //          */
        //         let (mut from_index, mut to_index, total_len) = (0, data.len(), data.len());
        //         let mut set_to_index = false;
        //         let mut begin_check_from_index = false;
        //         for (seq, rich_data) in data.iter().rev().enumerate() {
        //             if !set_to_index && rich_data.is_visible(top_y, bottom_y) {
        //                 // 待绘制的内容超出窗口底部边界
        //                 to_index = total_len - seq;
        //                 set_to_index = true;
        //                 begin_check_from_index = true;
        //             }
        //
        //             if begin_check_from_index && !rich_data.is_visible(top_y, bottom_y) {
        //                 // 待绘制内容已经向上超出窗口顶部边界，可以停止处理前面的数据了。
        //                 from_index = total_len - seq;
        //                 break;
        //             }
        //         }
        //
        //         for (idx, rich_data) in data[from_index..to_index].iter().enumerate() {
        //             rich_data.draw(offset_y, idx, visible_lines_rc.clone());
        //         }
        //     }
        // });

        /*
        跟随新增行自动滚动到最底部。
         */
        scroller.handle({
            let buffer_rc = data_buffer.clone();
            let last_window_size = Rc::new(RefCell::new((0, 0)));
            let visible_lines_rc = visible_lines.clone();
            let notifier_rc = notifier.clone();
            move |scroller, evt| {
                match evt {
                    Event::Resize => {
                        // 缩放窗口后重新计算分片绘制信息。
                        let (current_width, current_height) = (scroller.width(), scroller.height());
                        let (last_width, last_height) = *last_window_size.borrow();
                        if last_width != current_width || last_height != current_height {
                            last_window_size.replace((current_width, current_height));
                            let drawable_max_width = current_width - PADDING.left - PADDING.right;
                            let mut last_piece = LinePiece::init_piece();
                            for rich_data in buffer_rc.borrow_mut().iter_mut() {
                                rich_data.line_pieces.clear();
                                last_piece = rich_data.estimate(last_piece, drawable_max_width);
                            }
                        }
                    }
                    Event::Move => {
                        // 检测鼠标进入可互动区域，改变鼠标样式
                        let mut enter_piece = false;
                        for area in visible_lines_rc.borrow().keys() {
                            let (x, y, w, h) = area.to_rect();
                            if app::event_inside(x, y, w, h) {
                                enter_piece = true;
                                break;
                            }
                        }
                        if enter_piece {
                            draw::set_cursor(Cursor::Hand);
                        } else {
                            draw::set_cursor(Cursor::Default);
                        }
                    }
                    Event::Released => {
                        // 检测鼠标点击可互动区域，执行用户自定义操作
                        for (area, idx) in visible_lines_rc.borrow().iter() {
                            let (x, y, w, h) = area.to_rect();
                            if app::event_inside(x, y, w, h) {
                                if let Some(rd) = buffer_rc.borrow().get(*idx) {
                                    let sd = rd.into();
                                    if let Some(notifier) = notifier_rc.borrow().as_ref() {
                                        let notifier = notifier.clone();
                                        tokio::spawn(async move {
                                            if let Err(e) = notifier.send(sd).await {
                                                eprintln!("send error: {:?}", e);
                                            }
                                        });
                                    }
                                }
                                break;
                            }
                        }
                    }
                    _ => {}
                }
                false
            }
        });


        Self { scroller, panel, data_buffer, background_color }
    }
}
