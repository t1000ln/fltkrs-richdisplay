//! 内容源自rich_text的快照，可滚动浏览的组件。

use std::cell::{Cell, RefCell};
use std::cmp::{max, min, Ordering};
use std::collections::{HashMap};
use std::rc::{Rc, Weak};
use std::time::{Duration};
use fltk::draw::{draw_rect_fill, draw_xyline, LineStyle, Offscreen, set_draw_color, set_line_style};
use fltk::enums::{Align, Color, Cursor, Event};
use fltk::frame::Frame;
use fltk::group::{Scroll, ScrollType};
use fltk::prelude::{GroupExt, WidgetBase, WidgetExt};
use fltk::{app, draw, widget_extends};
use log::{debug, error};
use throttle_my_fn::throttle;
use crate::{Rectangle, disable_data, LinedData, LinePiece, LocalEvent, mouse_enter, PADDING, RichData, RichDataOptions, update_data_properties, UserData, ClickPoint, select_text2, locate_target_rd, clear_selected_pieces, BlinkState, BLINK_INTERVAL};
use crate::rich_text::{PANEL_PADDING};

#[derive(Clone, Debug)]
pub struct RichReviewer {
    pub(crate) scroller: Scroll,
    pub(crate) panel: Frame,
    pub(crate) data_buffer: Rc<RefCell<Vec<RichData>>>,
    background_color: Rc<Cell<Color>>,
    visible_lines: Rc<RefCell<HashMap<Rectangle, LinePiece>>>,
    clickable_data: Rc<RefCell<HashMap<Rectangle, usize>>>,
    reviewer_screen: Rc<RefCell<Offscreen>>,
    notifier: Rc<RefCell<Option<tokio::sync::mpsc::Sender<UserData>>>>,
    search_string: Option<String>,
    /// 查找结果，保存查询到的目标数据段在data_buffer中的索引编号。
    search_results: Vec<usize>,
    current_highlight_focus: Option<(usize, usize)>,
    blink_flag: Rc<Cell<BlinkState>>,
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
        let visible_lines = Rc::new(RefCell::new(HashMap::<Rectangle, LinePiece>::new()));
        let clickable_data = Rc::new(RefCell::new(HashMap::<Rectangle, usize>::new()));
        let notifier: Rc<RefCell<Option<tokio::sync::mpsc::Sender<UserData>>>> = Rc::new(RefCell::new(None));
        let reviewer_screen = Rc::new(RefCell::new(Offscreen::new(w, h).unwrap()));
        let scroll_panel_to_y_after_resize = Rc::new(Cell::new(0));
        let resize_panel_after_resize = Rc::new(Cell::new((0, 0, 0, 0)));

        let search_results = Vec::<usize>::new();
        let search_str = None::<String>;
        let current_highlight_focus = None::<(usize, usize)>;

        let blink_flag = Rc::new(Cell::new(BlinkState::new()));
        let blink_handler = {
            let blink_flag_rc = blink_flag.clone();

            #[cfg(target_os = "linux")]
            let scroller_rc = scroller.clone();

            #[cfg(not(target_os = "linux"))]
            let mut scroller_rc = scroller.clone();

            move |handler| {
                if !scroller_rc.was_deleted() {
                    let (should_toggle, bs) = blink_flag_rc.get().toggle_when_on();
                    if should_toggle {
                        blink_flag_rc.set(bs);
                        // debug!("from reviewer blink flag: {:?}", blink_flag_rc.get());

                        #[cfg(target_os = "linux")]
                        if let Some(mut parent) = scroller_rc.parent() {
                            parent.set_damage(true);
                        }

                        #[cfg(not(target_os = "linux"))]
                        scroller_rc.set_damage(true);
                    }
                    app::repeat_timeout3(BLINK_INTERVAL, handler);
                } else {
                    app::remove_timeout3(handler);
                }
            }
        };
        app::add_timeout3(BLINK_INTERVAL, blink_handler);

        panel.draw({
            let data_buffer_rc = data_buffer.clone();
            let scroll_rc = scroller.clone();
            let visible_lines_rc = visible_lines.clone();
            let clickable_data_rc = clickable_data.clone();
            let bg_rc = background_color.clone();
            let screen_rc = reviewer_screen.clone();
            let blink_flag_rc = blink_flag.clone();
            move |_| {
                /*
                先离线绘制内容面板，再根据面板大小复制所需区域内容。这样做是为了避免在线绘制时，会出现绘制内容超出面板边界的问题。
                 */
                Self::draw_offline(screen_rc.clone(), &scroll_rc, visible_lines_rc.clone(), clickable_data_rc.clone(), data_buffer_rc.clone(), bg_rc.get(), blink_flag_rc.clone());

                screen_rc.borrow().copy(scroll_rc.x(), scroll_rc.y(), scroll_rc.width(), scroll_rc.height(), 0, 0);
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
            let last_window_size = Rc::new(Cell::new((w, h)));
            let notifier_rc = notifier.clone();
            let screen_rc = reviewer_screen.clone();
            let panel_rc = panel.clone();
            let new_scroll_y_rc = scroll_panel_to_y_after_resize.clone();
            let resize_panel_after_resize_rc = resize_panel_after_resize.clone();
            let clickable_data_rc = clickable_data.clone();
            let mut selected = false;
            let mut push_from_point = ClickPoint::new(0, 0);
            let mut select_from_row = 0;
            let selected_pieces = Rc::new(RefCell::new(Vec::<Weak<RefCell<LinePiece>>>::new()));
            move |scroller, evt| {
                match evt {
                    Event::Close => {
                        debug!("Closing");
                    }
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
                            if old_scroll_y > 0 && last_height > 0 {
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
                        if mouse_enter(clickable_data_rc.clone()) {
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
                        for (area, idx) in clickable_data_rc.borrow().iter() {
                            let (x, y, w, h) = area.tup();
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
                    Event::Push => {
                        let (push_from_x, push_from_y) = app::event_coords();
                        if selected {
                            // debug!("清除选区");
                            clear_selected_pieces(selected_pieces.clone());
                            scroller.set_damage(true);
                            selected = false;
                            select_from_row = 0;
                        }

                        let (p_offset_x, p_offset_y) = (scroller.x(), scroller.y());
                        let mut offset_y = scroller.yposition() - PANEL_PADDING;
                        // 处理数据相对位移
                        if let Some(first) = buffer_rc.borrow().first() {
                            offset_y += first.v_bounds.get().0;
                        }
                        push_from_point.x = push_from_x - p_offset_x;
                        push_from_point.y = push_from_y + offset_y - p_offset_y + PADDING.top;

                        // 尝试检测起始点击位置是否位于某个数据段内，可减少后续划选过程中的检测目标范围
                        let index_vec = (0..buffer_rc.borrow().len()).collect::<Vec<usize>>();
                        let push_rect = push_from_point.as_rect();
                        if let Some(row) = locate_target_rd(&mut push_from_point, &push_rect, scroller.w(), buffer_rc.clone(), &index_vec) {
                            select_from_row = row;
                        }

                        #[cfg(target_os = "linux")]
                        if let Some(mut parent) = scroller.parent() {
                            parent.set_damage(true);
                        }

                        return true;
                    }
                    Event::Drag => {
                        let yp = scroller.yposition();
                        let cy = app::event_y();
                        let max_scroll = panel_rc.height() - scroller.height();
                        let (current_x, current_y) = app::event_coords();

                        // 拖动时如果鼠标超出scroll组件边界，但滚动条未到达底部或顶部时，自动滚动内容。
                        if cy > (scroller.y() + scroller.h()) && yp < max_scroll {
                            scroller.scroll_to(0, min(yp + 10, max_scroll));
                        } else if cy < scroller.y() && yp > 0 {
                            scroller.scroll_to(0, max(yp - 10, 0));
                        }

                        let (p_offset_x, p_offset_y) = (scroller.x(), scroller.y());
                        let mut offset_y = scroller.yposition() - PANEL_PADDING;
                        // 处理数据相对位移
                        if let Some(first) = buffer_rc.borrow().first() {
                            offset_y += first.v_bounds.get().0;
                        }
                        if offset_y < 0 {offset_y = 0;}

                        if let Some(_) = Self::redraw_after_drag(
                            push_from_point,
                            select_from_row,
                            ClickPoint::new(current_x - p_offset_x, current_y + offset_y - p_offset_y + PADDING.top),
                            buffer_rc.clone(),
                            selected_pieces.clone(),
                            scroller,
                        ) {
                            selected = !selected_pieces.borrow().is_empty();
                            // debug!("拖选结果：{selected}");
                            #[cfg(target_os = "linux")]
                            if let Some(mut parent) = scroller.parent() {
                                parent.set_damage(true);
                            }
                        }

                        return true;
                    }
                    _ => {}
                }
                false
            }
        });

        Self { scroller, panel, data_buffer, background_color, visible_lines, clickable_data, reviewer_screen, notifier, search_string: search_str, search_results, current_highlight_focus, blink_flag }
    }

    #[throttle(1, Duration::from_millis(50))]
    fn redraw_after_drag(
        push_from_point: ClickPoint,
        select_from_row: usize,
        current_point: ClickPoint,
        data_buffer: Rc<RefCell<Vec<RichData>>>,
        selected_pieces: Rc<RefCell<Vec<Weak<RefCell<LinePiece>>>>>,
        scroller: &mut Scroll,) -> bool {

        let mut down = true;
        let index_vec = if current_point.y >= push_from_point.y {
            // 向下选择
            (select_from_row..data_buffer.borrow().len()).collect::<Vec<usize>>()
        } else {
            // 向上选择
            down = false;
            (0..=select_from_row).collect::<Vec<usize>>()
        };
        // debug!("开始查找结束点所在数据段: {:?}", index_vec);
        let mut point = current_point.clone();
        if let Some(select_to_row) = locate_target_rd(&mut point, &current_point.as_rect(), scroller.w(), data_buffer.clone(), &index_vec) {
            let rd_range = if down {
                select_from_row..=(select_from_row + select_to_row)
            } else {
                select_to_row..=select_from_row
            };
            select_text2(&push_from_point, point, data_buffer, rd_range, selected_pieces);
            scroller.set_damage(true);
            return true;
        }
        false
    }

    pub fn set_background_color(&self, color: Color) {
        self.background_color.replace(color);
    }

    pub(crate) fn set_data(&mut self, data: Vec<RichData>) {
        // 更新回看数据
        self.data_buffer.replace(data);

        let (scroller_width, scroller_height) = (self.panel.width(), self.scroller.height());

        // 设置新的窗口尺寸
        let panel_height = Self::calc_panel_height(self.data_buffer.clone(), scroller_height);
        self.panel.resize(self.panel.x(), self.panel.y(), scroller_width, panel_height);
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
    pub(crate) fn calc_panel_height(buffer_rc: Rc<RefCell<Vec<RichData>>>, scroller_height: i32) -> i32 {
        let buffer = &*buffer_rc.borrow();
        let (mut top, mut bottom) = (0, 0);
        if let Some(first) = buffer.first() {
            top = first.v_bounds.get().0;
        }
        if let Some(last) = buffer.last() {
            bottom = last.v_bounds.get().1;
        }
        let content_height = bottom - top + PADDING.bottom + PADDING.top;
        if content_height > scroller_height {
            content_height
        } else {
            scroller_height
        }
    }

    fn draw_offline(
        screen: Rc<RefCell<Offscreen>>,
        scroller: &Scroll,
        visible_lines: Rc<RefCell<HashMap<Rectangle, LinePiece>>>,
        clickable_data: Rc<RefCell<HashMap<Rectangle, usize>>>,
        data_buffer: Rc<RefCell<Vec<RichData>>>,
        background_color: Color,
        blink_flag: Rc<Cell<BlinkState>>
        ) {

        screen.borrow().begin();
        let (scroller_x, scroller_y, window_width, window_height) = (scroller.x(), scroller.y(), scroller.width(), scroller.height());
        let drawable_height = window_height - PANEL_PADDING;

        let mut vl = visible_lines.borrow_mut();
        let mut cd = clickable_data.borrow_mut();
        vl.clear();
        cd.clear();

        // 滚动条滚动的高度在0到(panel.height - scroll.height)之间。
        let mut base_y = scroller.yposition();
        if base_y < 0 {
            base_y = 0;
        }

        let (mut top_y, mut bottom_y) = (base_y, base_y + drawable_height);

        // 处理数据相对位移
        if let Some(first) = data_buffer.borrow().first() {
            let y = first.v_bounds.get().0;
            top_y += y;
            bottom_y += y;
        }

        let offset_y = top_y - PADDING.top;

        // 填充背景色
        draw_rect_fill(0, 0, window_width, window_height, background_color);

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

        let mut need_blink = false;
        for (idx, rich_data) in data[from_index..to_index].iter().enumerate() {
            rich_data.draw(offset_y, blink_flag.get());

            if !need_blink && (rich_data.blink || rich_data.search_highlight_pos.is_some()) {
                // debug!("需要闪烁");
                need_blink = true;
            }

            for piece in rich_data.line_pieces.iter() {
                let piece = &*piece.borrow();
                let x = piece.x + scroller_x;
                let y = piece.y - offset_y + scroller_y;
                vl.insert(Rectangle::new(x, y, piece.w, piece.h), piece.clone());
                if rich_data.clickable {
                    cd.insert(Rectangle::new(x, y, piece.w, piece.h), idx + from_index);
                }
            }
        }

        /*
        绘制分界线
         */
        draw_rect_fill(0, drawable_height, window_width, PANEL_PADDING, background_color);
        set_draw_color(Color::White);
        set_line_style(LineStyle::DashDotDot, (PANEL_PADDING as f32 / 3f32).floor() as i32);
        draw_xyline(0, drawable_height + (PANEL_PADDING / 2), scroller_x + window_width);
        set_line_style(LineStyle::Solid, 1);

        // 填充顶部边界空白
        draw_rect_fill(0, 0, window_width, PADDING.top, background_color);

        screen.borrow().end();

        // 更新闪烁标记
        if need_blink {
            let bs = blink_flag.get();
            if !bs.is_on() {
                blink_flag.set(bs.on());
            }
        } else {
            let bs = blink_flag.get();
            if bs.is_on() {
                blink_flag.set(bs.off());
            }
        }
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

    fn draw_offline2(&self) {
        Self::draw_offline(
            self.reviewer_screen.clone(),
            &self.scroller,
            self.visible_lines.clone(),
            self.clickable_data.clone(),
            self.data_buffer.clone(),
            self.background_color.get(),
            self.blink_flag.clone()
        );
    }

    /// 更改数据属性。
    ///
    /// # Arguments
    ///
    /// * `id`: 数据ID。
    /// * `clickable`:
    /// * `underline`:
    /// * `expired`:
    /// * `text`:
    /// * `fg_color`:
    /// * `bg_color`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn update_data(&mut self, options: RichDataOptions) {
        let mut find_out = false;
        let mut target_idx = 0;
        if let Ok(idx) = self.data_buffer.borrow().binary_search_by_key(&options.id, |rd| rd.id) {
            target_idx = idx;
            find_out = true;
        }

        if find_out {
            if let Some(rd) = self.data_buffer.borrow_mut().get_mut(target_idx) {
                update_data_properties(options, rd);
            }
            // Self::draw_offline(
            //     self.reviewer_screen.clone(),
            //     &self.scroller,
            //     self.visible_lines.clone(),
            //     self.clickable_data.clone(),
            //     self.data_buffer.clone(),
            //     self.background_color.get(),
            // );
            self.draw_offline2();
        }
    }

    pub fn disable_data(&mut self, id: i64) {
        let mut find_out = false;
        let mut target_idx = 0;
        if let Ok(idx) = self.data_buffer.borrow().binary_search_by_key(&id, |rd| rd.id) {
            target_idx = idx;
            find_out = true;
        }

        if find_out {
            if let Some(rd) = self.data_buffer.borrow_mut().get_mut(target_idx) {
                disable_data(rd);
            }

            // Self::draw_offline(
            //     self.reviewer_screen.clone(),
            //     &self.scroller,
            //     self.visible_lines.clone(),
            //     self.clickable_data.clone(),
            //     self.data_buffer.clone(),
            //     self.background_color.get(),
            // );
            self.draw_offline2();
        }
    }

    /// 查找目标字符串，并高亮显示第一个或最后一个查找到的目标。
    ///
    /// # Arguments
    ///
    /// * `search_str`: 目标字符串。
    /// * `forward`: true正向，false反向查找。
    ///
    /// returns: bool 是否找到目标。
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub(crate) fn search_str(&mut self, search_str: String, forward: bool) -> bool {
        let find_out = if let Some(ref old) = self.search_string {
            if old.eq(&search_str) {
                // 查询字符串未发生变化，则尝试定位到下一个目标
                !self.search_results.is_empty()
            } else {
                self._search_target(search_str)
            }
        } else {
            self._search_target(search_str)
        };

        if find_out {
            // debug!("找到目标字符串，定位并显示");
            if forward {
                self.highlight_next();
            } else {
                self.highlight_previous();
            }
            self.show_search_results();
        }
        find_out
    }

    /// 倒序(从下向上，从右向左)查找高亮下一个目标。
    fn highlight_previous(&mut self) {
        // debug!("查询目标：\"{:?}\"，已知的目标数据段：{:?}", self.search_string, self.search_results);
        if let Some((old_rd_idx, old_result_idx)) = self.current_highlight_focus {
            // debug!("上一次定位的数据段索引：{}，目标编号：{}", old_rd_idx, old_result_idx);
            let (mut scroll_to_next, mut next_rd_pos) = (false, 0);
            if let Some(rd) = self.data_buffer.borrow_mut().get_mut(old_rd_idx) {
                if let Some(ref result_pos_vec) = rd.search_result_positions {
                    let next_result_idx = old_result_idx + 1;
                    if result_pos_vec.get(next_result_idx).is_some() {
                        // 在当前数据段中定位到下一个目标位置
                        // debug!("在当前数据段中定位到下一个目标位置");
                        self.current_highlight_focus.replace((old_rd_idx, next_result_idx));
                        rd.search_highlight_pos.replace(next_result_idx);
                    } else {
                        // 在当前数据段中已经没有更多目标，则跳到下一个数据段；如果没有更多数据段则跳到第一个数据段。
                        // debug!("在当前数据段中已经没有更多目标，则跳到下一个数据段；如果没有更多数据段则跳到第一个数据段。");
                        let next_idx  = if let Ok(old_idx) = self.binary_search_with_desc_order(old_rd_idx) {
                            old_idx + 1
                        } else {
                            0
                        };

                        scroll_to_next = true;
                        if let Some(next_rd_idx) = self.search_results.get(next_idx) {
                            // debug!("下一个数据段索引：{}，目标序号：{}", next_rd_idx, next_idx);
                            next_rd_pos = *next_rd_idx;
                        } else {
                            if let Some(next_rd_idx) = self.search_results.first() {
                                next_rd_pos = *next_rd_idx;
                            }
                            // debug!("回归到循环开始位置，下一个数据段索引：{}， 目标序号：0", next_rd_pos);
                        }

                        rd.search_highlight_pos = None;
                    }
                }
            }

            if scroll_to_next {
                self.current_highlight_focus.replace((next_rd_pos, 0));
                if let Some(rd) = self.data_buffer.borrow_mut().get_mut(next_rd_pos) {
                    rd.search_highlight_pos = Some(0);
                }
            }
        } else {
            if let Some(rd_idx) = self.search_results.first() {
                if let Some(rd) = self.data_buffer.borrow_mut().get_mut(*rd_idx) {
                    if rd.search_result_positions.is_some() {
                        // debug!("首次定位到第一个目标");
                        self.current_highlight_focus = Some((*rd_idx, 0));
                        rd.search_highlight_pos = Some(0);
                    }
                }
            }
        }
    }

    /// 顺序(从上向下，从左到右)查找高亮下一个目标。
    fn highlight_next(&mut self) {
        if let Some((old_rd_idx, old_result_idx)) = self.current_highlight_focus {
            // debug!("上一次定位的数据段索引：{}，目标编号：{}", old_rd_idx, old_result_idx);
            let (mut scroll_to_next, mut next_rd_pos) = (false, 0);
            if let Some(rd) = self.data_buffer.borrow_mut().get_mut(old_rd_idx) {
                if old_result_idx >= 1 {
                    // 在当前数据段中定位到下一个目标位置
                    self.current_highlight_focus.replace((old_rd_idx, old_result_idx - 1));
                    rd.search_highlight_pos.replace(old_result_idx - 1);
                } else {
                    // 在当前数据段中已经没有更多目标，则跳到下一个数据段；如果没有更多数据段则跳到第一个数据段。
                    let next_idx  = if let Ok(old_idx) = self.binary_search_with_desc_order(old_rd_idx) {
                        if old_idx >= 1 {
                            old_idx - 1
                        } else {
                            self.search_results.len() - 1
                        }
                    } else {
                        self.search_results.len() - 1
                    };
                    scroll_to_next = true;
                    if let Some(next_rd_idx) = self.search_results.get(next_idx) {
                        // debug!("下一个数据段索引：{}，目标序号：{}", next_rd_idx, next_idx);
                        next_rd_pos = *next_rd_idx;
                    } else if let Some(next_rd_idx) = self.search_results.last() {
                        next_rd_pos = *next_rd_idx;
                        // debug!("回归到循环开始位置，下一个数据段索引：{}， 目标序号：0", next_rd_pos);
                    }

                    rd.search_highlight_pos = None;
                }
            }

            if scroll_to_next {
                if let Some(rd) = self.data_buffer.borrow_mut().get_mut(next_rd_pos) {
                    rd.search_highlight_pos = Some(0);
                    if let Some(ref pos_vec) = rd.search_result_positions {
                        self.current_highlight_focus.replace((next_rd_pos, pos_vec.len() - 1));
                    }
                }
            }

        } else {
            if let Some(rd_idx) = self.search_results.last() {
                if let Some(rd) = self.data_buffer.borrow_mut().get_mut(*rd_idx) {
                    if let Some(ref srp) = rd.search_result_positions {
                        let len = srp.len();
                        // debug!("首次定位到第一个目标");
                        self.current_highlight_focus = Some((*rd_idx, len - 1));
                        rd.search_highlight_pos = Some(len - 1);
                    }
                }
            }
        }
    }

    /// 在倒序排列的数组中，查找目标数据。
    ///
    /// # Arguments
    ///
    /// * `old_rd_idx`: 目标数据。
    ///
    /// returns: Result<usize, usize> 返回目标所在位置，若未找到则返回应该所在位置。
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn binary_search_with_desc_order(&self, target: usize) -> Result<usize, usize> {
        self.search_results.binary_search_by(|&a| {
            if a == target {
                Ordering::Equal
            } else if a > target {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        })
    }

    /// 查找目标字符串，并记录目标位置。
    ///
    /// # Arguments
    ///
    /// * `search_str`: 目标字符串。
    ///
    /// returns: bool
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn _search_target(&mut self, search_str: String) -> bool {
        let mut find_out = false;
        self._clear_search_results();
        let s = self.search_string.insert(search_str).as_str();

        let len = s.chars().count();
        for (idx, rd) in self.data_buffer.borrow_mut().iter_mut().enumerate() {
            if rd.text.contains(s) {
                find_out = true;
                self.search_results.push(idx);
                let mut s_idx_vec: Vec<(usize, usize)> = vec![];
                rd.text.rmatch_indices(s).for_each(|(s_idx, _)| {
                    let chars = rd.text[0..s_idx].chars().count();
                    s_idx_vec.push((chars, chars + len))
                });
                if !s_idx_vec.is_empty() {
                    rd.search_result_positions = Some(s_idx_vec);
                }
            }
        }
        if find_out {
            self.search_results.reverse();
        }
        find_out
    }

    /// 清除上一次查询的缓存记录。
    fn _clear_search_results(&mut self) {
        self.search_results.iter().for_each(|idx| {
            if let Some(rd) = self.data_buffer.borrow_mut().get_mut(*idx) {
                rd.search_result_positions = None;
                rd.search_highlight_pos = None;
            }
        });
        self.search_results.clear();
        self.current_highlight_focus = None;
    }

    /// 清除查询缓存，并刷新界面。
    pub(crate) fn clear_search_results(&mut self) {
        self._clear_search_results();
        self.search_string = None;
        self.scroller.set_damage(true);
    }

    /// 定位到下一个查询目标并显示在可见区域。
    fn show_search_results(&mut self) {
        if let Some((rd_idx, result_idx)) = self.current_highlight_focus {
            let mut piece_idx = 0;
            if let Some(rd) = self.data_buffer.borrow().get(rd_idx) {
                if let Some(ref s) = self.search_string {
                    if let Some((pos, _)) = rd.text.rmatch_indices(s).nth(result_idx) {
                        let mut processed_len = 0usize;
                        for (i, piece_rc) in rd.line_pieces.iter().enumerate() {
                            let piece = &*piece_rc.borrow();
                            let pl = piece.line.len();
                            if pos >= processed_len && pos < processed_len + pl {
                                piece_idx = i;
                                break;
                            }
                            processed_len += pl;
                        }
                    }
                }
            }
            // debug!("当前定位的数据段索引：{}，目标顺序：{}，位于分片{}内", rd_idx, result_idx, piece_idx);
            self.show_piece(rd_idx, piece_idx);
        }
    }

    /// 滚动显示区域到指定的数据段下的数据分片。
    /// 滚动时向显示区域底部靠近。
    ///
    /// # Arguments
    ///
    /// * `rd_idx`:
    /// * `piece_idx`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn show_piece(&mut self, rd_idx: usize, piece_idx: usize) {
        if let Some(rd) = self.data_buffer.borrow().get(rd_idx) {
            if piece_idx < rd.line_pieces.len() {
                if let Some(piece_rc) = rd.line_pieces.get(piece_idx) {
                    let piece = &*piece_rc.borrow();
                    // debug!("piece.top_y: {}, panel_height: {}, scroller.yposition: {}, piece.line: {}", piece.top_y, self.panel.h(), self.scroller.yposition(), piece.line);
                    let scroller_y = self.scroller.yposition();
                    if piece.y < scroller_y || piece.y + piece.h >= scroller_y + self.scroller.h() {
                        let mut scroll_to_y = piece.y - self.scroller.h() + piece.h * 2 + PADDING.top + 3;
                        if scroll_to_y < 0 {
                            scroll_to_y = 0;
                        } else if scroll_to_y > self.panel.h() - self.scroller.h() {
                            scroll_to_y = self.panel.h() - self.scroller.h();
                        }
                        // debug!("无法看到，滚动到: {}", scroll_to_y);
                        self.scroller.scroll_to(0, scroll_to_y);
                    }
                }
            }
        }
    }
}
