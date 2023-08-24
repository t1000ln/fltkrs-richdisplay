//! 内容源自rich_text的快照，可滚动的浏览的组件。

use std::cell::{Cell, RefCell};
use std::collections::{HashMap};
use std::rc::Rc;
use fltk::draw::{draw_rect_fill, draw_rounded_rectf, draw_xyline, LineStyle, Offscreen, set_draw_color, set_line_style};
use fltk::enums::{Align, Color, Cursor, Event};
use fltk::frame::Frame;
use fltk::group::{Scroll, ScrollType};
use fltk::prelude::{GroupExt, WidgetBase, WidgetExt};
use fltk::{app, draw, widget_extends};
use log::{error};
use crate::{Coordinates, LinedData, LinePiece, LocalEvent, mouse_enter, PADDING, RichData, UserData};
use crate::rich_text::{MAIN_PANEL_FIX_HEIGHT, PANEL_PADDING};

#[derive(Clone, Debug)]
pub struct RichReviewer {
    pub(crate) scroller: Scroll,
    pub(crate) panel: Frame,
    data_buffer: Rc<RefCell<Vec<RichData>>>,
    background_color: Rc<Cell<Color>>,
    reviewer_screen: Rc<RefCell<Offscreen>>,
    notifier: Rc<RefCell<Option<tokio::sync::mpsc::Sender<UserData>>>>,
    pub resize_panel_after_resize: Rc<Cell<(i32, i32, i32, i32)>>,
    pub scroll_panel_to_y_after_resize: Rc<Cell<i32>>,
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
        scroller.end();

        let mut panel = Frame::new(x, y, w, h, None);
        scroller.add_resizable(&panel);

        let data_buffer: Rc<RefCell<Vec<RichData>>> = Rc::new(RefCell::new(vec![]));
        let background_color = Rc::new(Cell::new(Color::Black));
        let visible_lines = Rc::new(RefCell::new(HashMap::<Coordinates, usize>::new()));
        let notifier: Rc<RefCell<Option<tokio::sync::mpsc::Sender<UserData>>>> = Rc::new(RefCell::new(None));
        let reviewer_screen = Rc::new(RefCell::new(Offscreen::new(w, h).unwrap()));
        let scroll_panel_to_y_after_resize = Rc::new(Cell::new(0));
        let resize_panel_after_resize = Rc::new(Cell::new((0, 0, 0, 0)));

        panel.draw({
            let data_buffer_rc = data_buffer.clone();
            let scroll_rc = scroller.clone();
            let visible_lines_rc = visible_lines.clone();
            let bg_rc = background_color.clone();
            let screen_rc = reviewer_screen.clone();
            move |_| {
                /*
                先离线绘制内容面板，再根据面板大小复制所需区域内容。这样做是为了避免在线绘制时，会出现绘制内容超出面板边界的问题。
                 */
                Self::draw_offline(screen_rc.clone(), &scroll_rc, data_buffer_rc.clone(), bg_rc.get(), visible_lines_rc.clone());

                let (x, y, window_width, window_height) = (scroll_rc.x(), scroll_rc.y(), scroll_rc.width(), scroll_rc.height());
                screen_rc.borrow().copy(x, y, window_width, window_height, 0, 0);
            }
        });

        /*
        处理自定义事件，主要解决缩放窗口时需要重新计算面板大小并滚动到恰当位置的逻辑。
        之所以需要自定义事件，是因为外部容器缩放时，内部面板并不会自动缩放，而是需要计算新的尺寸后再通过自定义事件来实现内部面板的缩放处理。
        如果在外部容器的缩放事件处理过程中直接进行内部面板的缩放会出现外观不同步的问题，因此需要通过发出自定义事件来在app的全局事件处理循环中来逐个处理，才能避免该问题。
         */
        panel.handle({
            let new_scroll_y_rc = scroll_panel_to_y_after_resize.clone();
            let mut scroller_rc = scroller.clone();
            let resize_panel_after_resize_rc = resize_panel_after_resize.clone();
            move |ctx, evt| {
                if evt == LocalEvent::RESIZE.into() {
                    let (x, y, w, h) = resize_panel_after_resize_rc.get();
                    // 强制滚动到最顶部，避免scroll.yposition()缓存，在窗口不需要滚动条时仍出现滚动条的问题。
                    scroller_rc.scroll_to(0, 0);
                    ctx.resize(x, y, w, h);
                    true
                } else if evt == LocalEvent::SCROLL_TO.into() {
                    scroller_rc.scroll_to(0, new_scroll_y_rc.get());
                    true
                } else {
                    false
                }
            }
        });

        scroller.handle({
            let buffer_rc = data_buffer.clone();
            let last_window_size = Rc::new(Cell::new((w, h - MAIN_PANEL_FIX_HEIGHT - PANEL_PADDING)));
            let notifier_rc = notifier.clone();
            let screen_rc = reviewer_screen.clone();
            let panel_rc = panel.clone();
            let new_scroll_y_rc = scroll_panel_to_y_after_resize.clone();
            let resize_panel_after_resize_rc = resize_panel_after_resize.clone();
            let visible_lines_rc = visible_lines.clone();
            move |scroller, evt| {
                match evt {
                    Event::Resize => {
                        // 缩放窗口后重新计算分片绘制信息。
                        let (current_width, current_height) = (scroller.width(), scroller.height());
                        let last_panel_height = panel_rc.height();
                        let (last_width, last_height) = last_window_size.get();
                        if last_width != current_width || last_height != current_height {
                            last_window_size.replace((current_width, current_height));

                            let old_scroll_y = scroller.yposition();

                            let mut new_panel_height = current_height;
                            if last_width != current_width {
                                // 当窗口宽度发生变化时，需要重新计算数据分片坐标信息。
                                let drawable_max_width = current_width - PADDING.left - PADDING.right;
                                let mut last_piece = LinePiece::init_piece();
                                for rich_data in buffer_rc.borrow_mut().iter_mut() {
                                    rich_data.line_pieces.clear();
                                    last_piece = rich_data.estimate(last_piece, drawable_max_width);
                                }

                                new_panel_height = Self::calc_panel_height(buffer_rc.clone(), current_height);

                                // 同步缩放回顾内容面板
                                resize_panel_after_resize_rc.replace((scroller.x(), scroller.y(), current_width, new_panel_height));
                                if let Err(e) = app::handle_main(LocalEvent::RESIZE) {
                                    error!("发送缩放信号失败:{e}");
                                }
                            }

                            // 按照新的窗口大小重新生成绘图板
                            if let Some(offs) = Offscreen::new(current_width, current_height) {
                                screen_rc.replace(offs);
                            } else {
                                error!("创建离线绘图板失败！");
                            }

                            /*
                            该事件执行完毕时会自动重绘并滚动到缩放前的滚动偏移量，但这不合理！
                            需要获取缩放前的滚动偏移量比例，并按照同比在缩放完成重绘后强制滚动到对应比例处。
                            这个操作需要延迟到自动滚动完毕后再执行，此处通过异步信号来达成预期效果。
                             */
                            if old_scroll_y > 0 {
                                let pos_percent = old_scroll_y as f64 / (last_panel_height - last_height) as f64;
                                let new_scroll_y = ((new_panel_height - current_height) as f64 * pos_percent).round() as i32;
                                new_scroll_y_rc.replace(new_scroll_y);
                                if let Err(e) = app::handle_main(LocalEvent::SCROLL_TO) {
                                    error!("发送滚动信号失败:{e}");
                                }
                            }
                        }
                    }
                    Event::Move => {
                        // 检测鼠标进入可互动区域，改变鼠标样式
                        if mouse_enter(visible_lines_rc.clone()) {
                            draw::set_cursor(Cursor::Hand);
                        } else {
                            draw::set_cursor(Cursor::Default);
                        }
                    }
                    Event::Leave => {
                        draw::set_cursor(Cursor::Default);
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
                                                error!("发送用户操作失败: {:?}", e);
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

        Self { scroller, panel, data_buffer, background_color, reviewer_screen, scroll_panel_to_y_after_resize, resize_panel_after_resize, notifier }
    }

    pub fn set_background_color(&self, color: Color) {
        self.background_color.replace(color);
    }

    pub fn set_data(&mut self, data: Vec<RichData>) {
        // 更新回看数据
        self.data_buffer.replace(data);

        let (scroller_width, scroller_height) = (self.panel.width(), self.scroller.height());

        // 设置新的窗口尺寸
        let panel_height = Self::calc_panel_height(self.data_buffer.clone(), scroller_height);
        self.panel.resize(self.panel.x(), self.panel.y(), scroller_width, panel_height);
        // self.resize_panel_after_resize.replace((self.scroller.x(), self.scroller.y(), scroller_width, panel_height));
        // if let Err(e) = app::handle_main(LocalEvent::RESIZE) {
        //     error!("发送缩放信号失败:{e}");
        // }



        // 滚动到最底部
        // self.scroller.scroll_to(0, panel_height - scroller_height);
        // self.scroll_panel_to_y_after_resize.replace(panel_height - scroller_height);
        // if let Err(e) = app::handle_main(LocalEvent::SCROLL_TO) {
        //     error!("发送滚动信号失败:{e}");
        // }
    }

    /// 根据当前回顾`scroller`窗口大小创建对应的离线绘图板，并设置滚动条到最底部。
    ///
    /// # Arguments
    ///
    /// * `w`:
    /// * `h`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn renew_offscreen(&mut self, w: i32, h: i32) {
        if let Some(offs) = Offscreen::new(w, h) {
            self.reviewer_screen.replace(offs);
            // 滚动到最底部
            self.scroller.scroll_to(0, self.panel.height() - self.scroller.height());
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroller.scroll_to(0, self.panel.height() - self.scroller.height());
    }

    /// 计算数据内容所需的面板高度。
    ///
    /// # Arguments
    ///
    /// * `buffer`:
    /// * `scroller_height`:
    ///
    /// returns: i32
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn calc_panel_height(buffer_rc: Rc<RefCell<Vec<RichData>>>, scroller_height: i32) -> i32 {
        let buffer = &*buffer_rc.borrow();
        let (mut top, mut bottom) = (0, 0);
        if let Some(first) = buffer.first() {
            if let Some((top_y, _, _)) = first.v_bounds {
                top = top_y;
            }
        }
        if let Some(last) = buffer.last() {
            if let Some((_, bottom_y, _)) = last.v_bounds {
                bottom = bottom_y;
            }
        }
        let content_height = bottom - top + PADDING.bottom + PADDING.top;
        if content_height > scroller_height {
            content_height
        } else {
            scroller_height
        }
    }

    pub fn draw_offline(screen: Rc<RefCell<Offscreen>>, scroller: &Scroll, data_buffer: Rc<RefCell<Vec<RichData>>>, background_color: Color, visible_lines: Rc<RefCell<HashMap<Coordinates, usize>>>) {
        screen.borrow().begin();
        let (scroller_x, scroller_y, window_width, window_height) = (scroller.x(), scroller.y(), scroller.width(), scroller.height());
        let drawable_height = window_height - PANEL_PADDING;
        visible_lines.borrow_mut().clear();
        // 滚动条滚动的高度在0到(panel.height - scroll.height)之间。
        let mut base_y = scroller.yposition();
        if base_y < 0 {
            base_y = 0;
        }

        let (mut top_y, mut bottom_y) = (base_y, base_y + drawable_height);

        // 处理数据相对位移
        if let Some(first) = data_buffer.borrow().first() {
            if let Some((y, _, _)) = first.v_bounds {
                top_y += y;
                bottom_y += y;
            }
        }

        let offset_y = top_y - PADDING.top;

        // 填充背景色
        draw_rect_fill(0, 0, window_width, drawable_height, background_color);

        let data = &*data_buffer.borrow();

        /*
        先试算出可显示的行，再真正绘制可显示的行。
        试算从数据队列的尾部向头部取数，试算位置从窗口底部向顶部堆积。
         */
        let (mut from_index, mut to_index, total_len) = (0, data.len(), data.len());
        let mut set_to_index = false;
        let mut begin_check_from_index = false;
        for (seq, rich_data) in data.iter().rev().enumerate() {
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

        for (idx, rich_data) in data[from_index..to_index].iter().enumerate() {
            rich_data.draw(offset_y, idx, visible_lines.clone(), scroller_x, scroller_y);
        }

        /*
        绘制分界线
         */
        // 象牙白底色
        set_draw_color(Color::from_rgb(235, 229, 209));
        draw_rounded_rectf(0, drawable_height, window_width, PANEL_PADDING, 4);

        // 暗金色虚线
        set_draw_color(Color::from_rgb(136, 102, 0));
        set_line_style(LineStyle::DashDotDot, PANEL_PADDING / 3);
        draw_xyline(scroller_x + 6, drawable_height + (PANEL_PADDING / 2), scroller_x + window_width - 6);
        set_line_style(LineStyle::Solid, 1);

        screen.borrow().end();
    }

    /// 设置互动消息发送器。
    ///
    /// # Arguments
    ///
    /// * `notifier`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn set_notifier(&mut self, notifier: tokio::sync::mpsc::Sender<UserData>) {
        self.notifier.replace(Some(notifier));
    }

}
