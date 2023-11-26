//! 富文本查看器组件。

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::{Rc, Weak};

use fltk::draw::{draw_rect_fill, Offscreen};
use fltk::enums::{Color, Cursor, Event, Font};
use fltk::prelude::{FltkError, GroupExt, WidgetBase, WidgetExt};
use fltk::{app, draw, widget_extends};
use fltk::app::{MouseWheel};
use fltk::group::{Flex};
use fltk::widget::Widget;
use crate::{Rectangle, disable_data, LinedData, LinePiece, LocalEvent, mouse_enter, PADDING,
            RichData, RichDataOptions, update_data_properties, UserData, BLINK_INTERVAL,
            BlinkState, Callback, DEFAULT_FONT_SIZE, WHITE, clear_selected_pieces, ClickPoint,
            locate_target_rd, update_selection_when_drag};

use idgenerator_thin::{IdGeneratorOptions, YitIdHelper};
use log::{error};
use crate::rich_reviewer::RichReviewer;
use crate::utils::ID_GENERATOR_INIT;


pub const MAIN_PANEL_FIX_HEIGHT: i32 = 200;
pub const PANEL_PADDING: i32 = 8;

/// rich-display主面板结构。
///
#[derive(Debug, Clone)]
pub struct RichText {
    panel: Widget,
    data_buffer: Rc<RefCell<VecDeque<RichData>>>,
    background_color: Rc<Cell<Color>>,
    buffer_max_lines: usize,
    notifier: Rc<RefCell<Option<Callback>>>,
    inner: Flex,
    reviewer: Rc<RefCell<Option<RichReviewer>>>,
    panel_screen: Rc<RefCell<Offscreen>>,
    clickable_data: Rc<RefCell<HashMap<Rectangle, usize>>>,
    /// 主面板上可见行片段的集合容器，在每次离线绘制时被清空和填充。
    visible_lines: Rc<RefCell<HashMap<Rectangle, LinePiece>>>,
    blink_flag: Rc<Cell<BlinkState>>,
    /// 默认字体。
    text_font: Rc<Cell<Font>>,
    /// 默认字体颜色。
    text_color: Rc<Cell<Color>>,
    text_size: Rc<Cell<i32>>,
    piece_spacing: Rc<Cell<i32>>,
}
widget_extends!(RichText, Flex, inner);


impl RichText {
    pub fn new<T>(x: i32, y: i32, w: i32, h: i32, title: T) -> Self
        where T: Into<Option<&'static str>> + Clone {
        let _ = ID_GENERATOR_INIT.get_or_init(|| {
            // 初始化ID生成器。
            let options = IdGeneratorOptions::new(1);
            YitIdHelper::set_id_generator(options);
            0
        });

        let text_font = Rc::new(Cell::new(Font::Helvetica));
        let text_color = Rc::new(Cell::new(WHITE));
        let text_size = Rc::new(Cell::new(DEFAULT_FONT_SIZE));
        let piece_spacing = Rc::new(Cell::new(0));

        let background_color = Rc::new(Cell::new(Color::Black));
        let reviewer = Rc::new(RefCell::new(None::<RichReviewer>));

        // let mut inner = Flex::new(x, y, w, h, title).column(); // fltk 1.4.15变更为私有函数
        let mut inner = <Flex as WidgetBase>::new(x, y, w, h, title).column();
        inner.set_pad(0);
        inner.end();


        let mut panel = Widget::new(x, y, w, h, None);

        inner.add(&panel);

        let panel_screen = Rc::new(RefCell::new(Offscreen::new(w, h).unwrap()));

        let buffer_max_lines = 100;
        let data_buffer = Rc::new(RefCell::new(VecDeque::<RichData>::with_capacity(buffer_max_lines + 1)));

        let visible_lines = Rc::new(RefCell::new(HashMap::<Rectangle, LinePiece>::new()));
        let clickable_data = Rc::new(RefCell::new(HashMap::<Rectangle, usize>::new()));
        let notifier: Rc<RefCell<Option<Callback>>> = Rc::new(RefCell::new(None));
        let selected = Rc::new(Cell::new(false));
        let should_resize_content = Rc::new(Cell::new(0));


        let blink_flag = Rc::new(Cell::new(BlinkState::new()));
        let blink_handler = {
            let blink_flag_rc = blink_flag.clone();
            let mut panel_rc = panel.clone();
            let screen_rc = panel_screen.clone();
            let visible_lines_rc = visible_lines.clone();
            let clickable_data_rc = clickable_data.clone();
            let bg_rc = background_color.clone();
            let buffer_rc = data_buffer.clone();
            move |handler| {
                if !panel_rc.was_deleted() {
                    let (should_toggle, bs) = blink_flag_rc.get().toggle_when_on();
                    if should_toggle {
                        blink_flag_rc.set(bs);
                        // debug!("from main panel blink flag: {:?}", blink_flag_rc.get());
                        Self::draw_offline(
                            screen_rc.clone(),
                            &panel_rc,
                            visible_lines_rc.clone(),
                            clickable_data_rc.clone(),
                            bg_rc.get(),
                            buffer_rc.clone(),
                            blink_flag_rc.clone(),
                        );
                        panel_rc.set_damage(true);
                    }
                    app::repeat_timeout3(BLINK_INTERVAL, handler);
                } else {
                    app::remove_timeout3(handler);
                }
            }
        };
        app::add_timeout3(BLINK_INTERVAL, blink_handler);

        panel.draw({
            let screen_rc = panel_screen.clone();
            let resize_to = should_resize_content.clone();
            let flex = inner.clone();
            let visible_lines_rc = visible_lines.clone();
            let clickable_data_rc = clickable_data.clone();
            let bg_rc = background_color.clone();
            let buffer_rc = data_buffer.clone();
            let blink_flag_rc = blink_flag.clone();
            move |ctx| {
                let h = resize_to.replace(0);
                if h != 0 {
                    Self::new_offline(
                        flex.width(),
                        h,
                        screen_rc.clone(),
                        ctx,
                        visible_lines_rc.clone(),
                        clickable_data_rc.clone(),
                        bg_rc.get(),
                        buffer_rc.clone(),
                        blink_flag_rc.clone(),
                    );
                }

                screen_rc.borrow().copy(ctx.x(), ctx.y(), ctx.width(), ctx.height(), 0, 0);
            }
        });

        /*
        处理主面板容器的动作事件，打开或关闭回顾区。
         */
        inner.handle({
            let last_window_size = Rc::new(Cell::new((0, 0)));
            let panel_rc = panel.clone();
            let reviewer_rc = reviewer.clone();
            let buffer_rc = data_buffer.clone();
            let bg_rc = background_color.clone();
            let notifier_rc = notifier.clone();
            let should_resize = should_resize_content.clone();
            let selected_rc = selected.clone();
            move |flex, evt| {
                if evt == LocalEvent::DROP_REVIEWER_FROM_EXTERNAL.into() {
                    // 隐藏回顾区
                    Self::should_hide_reviewer(
                        reviewer_rc.clone(),
                        flex,
                        &panel_rc,
                        should_resize.clone()
                    );
                    true
                } else if evt == LocalEvent::OPEN_REVIEWER_FROM_EXTERNAL.into() {
                    let mut reviewer = RichReviewer::new(0, 0, flex.width(), flex.height() - MAIN_PANEL_FIX_HEIGHT, None);
                    reviewer.set_background_color(bg_rc.get());
                    if let Some(notifier_rc) = notifier_rc.borrow().as_ref() {
                        reviewer.set_notifier(notifier_rc.clone());
                    }
                    let snapshot = Vec::from(buffer_rc.borrow().clone());
                    reviewer.set_data(snapshot);
                    flex.insert(&reviewer.scroller, 0);
                    flex.fixed(&panel_rc, MAIN_PANEL_FIX_HEIGHT);
                    flex.recalc();

                    // 替换新的离线绘制板
                    should_resize.set(MAIN_PANEL_FIX_HEIGHT);

                    reviewer.scroll_to_bottom();
                    reviewer_rc.replace(Some(reviewer));
                    true
                } else {
                    match evt {
                        Event::Resize => {
                            let (current_width, current_height) = (flex.width(), flex.height());
                            let (last_width, last_height) = last_window_size.get();
                            if last_width != current_width || last_height != current_height {
                                last_window_size.replace((current_width, current_height));
                                let panel_height = if reviewer_rc.borrow().is_some() {
                                    MAIN_PANEL_FIX_HEIGHT
                                } else {
                                    current_height
                                };
                                flex.fixed(&panel_rc, panel_height);
                                if panel_height != current_height {
                                    // 包含有回顾区，在fltk-rs 1.4.12版本中，需要手动设置其尺寸
                                    if let Some(rv) = &*reviewer_rc.borrow() {
                                        flex.fixed(&rv.scroller, current_height - panel_height);
                                    }
                                }
                                // flex.recalc();
                            }
                        }
                        Event::MouseWheel => {
                            /*
                            显示或隐藏回顾区。
                             */
                            if app::event_dy() == MouseWheel::Down && !buffer_rc.borrow().is_empty() && reviewer_rc.borrow().is_none() {
                                // 显示回顾区
                                let mut reviewer = RichReviewer::new(0, 0, flex.width(), flex.height() - MAIN_PANEL_FIX_HEIGHT, None);
                                reviewer.set_background_color(bg_rc.get());
                                if let Some(notifier_rc_ref) = notifier_rc.borrow_mut().as_mut() {
                                    let cb = notifier_rc_ref.clone();
                                    reviewer.set_notifier(cb);
                                }

                                let mut snapshot = Vec::from(buffer_rc.borrow().clone());
                                if selected_rc.get() {
                                    snapshot.iter_mut().for_each(|rd| {
                                        rd.line_pieces.iter_mut().for_each(|piece| {
                                            piece.borrow().deselect();
                                        })
                                    });
                                }

                                reviewer.set_data(snapshot);
                                flex.insert(&reviewer.scroller, 0);
                                // flex.resizable(&reviewer.scroller);
                                flex.fixed(&panel_rc, MAIN_PANEL_FIX_HEIGHT);
                                flex.recalc();

                                should_resize.set(MAIN_PANEL_FIX_HEIGHT);

                                reviewer.scroll_to_bottom();
                                reviewer_rc.replace(Some(reviewer));

                            } else if app::event_dy() == MouseWheel::Up && reviewer_rc.borrow().is_some() {
                                // 隐藏回顾区
                                Self::should_hide_reviewer(
                                    reviewer_rc.clone(),
                                    flex,
                                    &panel_rc,
                                    should_resize.clone()
                                );
                            }
                        }
                        _ => {}
                    }
                    false
                }
            }
        });

        /*
        处理主面板缩放及鼠标操作事件。
         */
        panel.handle({
            let buffer_rc = data_buffer.clone();
            let last_window_size = Rc::new(Cell::new((0, 0)));
            let visible_lines_rc = visible_lines.clone();
            let clickable_data_rc = clickable_data.clone();
            let notifier_rc = notifier.clone();
            let screen_rc = panel_screen.clone();
            let bg_rc = background_color.clone();
            let selected = selected.clone();
            let mut select_from_row = 0;
            let mut push_from_point = ClickPoint::new(0, 0);
            let selected_pieces = Rc::new(RefCell::new(Vec::<Weak<RefCell<LinePiece>>>::new()));
            let should_resize = should_resize_content.clone();
            let blink_flag_rc = blink_flag.clone();
            move |ctx, evt| {
                match evt {
                    Event::Resize => {
                        // 缩放窗口后重新计算分片绘制信息。
                        let (current_width, current_height) = (ctx.width(), ctx.height());
                        let (last_width, last_height) = last_window_size.get();
                        if last_width != current_width || last_height != current_height {
                            last_window_size.replace((current_width, current_height));
                            if last_width != current_width {
                                // 当窗口宽度发生变化时，需要重新计算数据分片坐标信息。
                                let drawable_max_width = current_width - PADDING.left - PADDING.right;
                                let mut last_piece = LinePiece::init_piece();
                                for rich_data in buffer_rc.borrow_mut().iter_mut() {
                                    rich_data.line_pieces.clear();
                                    last_piece = rich_data.estimate(last_piece, drawable_max_width);
                                }
                            }

                            // 替换新的离线绘制板
                            should_resize.set(current_height);
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
                                    let sd: UserData = rd.into();
                                    if let Some(cb) = notifier_rc.borrow_mut().as_mut() {
                                        cb.notify(sd);
                                    }
                                }
                                break;
                            }
                        }
                    }
                    Event::Push => {
                        let (push_from_x, push_from_y) = app::event_coords();
                        if selected.replace(false) {
                            // debug!("清除选区");
                            clear_selected_pieces(selected_pieces.clone());
                            Self::draw_offline(
                                screen_rc.clone(),
                                &ctx,
                                visible_lines_rc.clone(),
                                clickable_data_rc.clone(),
                                bg_rc.get(),
                                buffer_rc.clone(),
                                blink_flag_rc.clone(),
                            );
                            ctx.set_damage(true);
                            select_from_row = 0;
                        }
                        let (p_offset_x, p_offset_y) = (ctx.x(), ctx.y());
                        push_from_point.x = push_from_x - p_offset_x;
                        push_from_point.y = push_from_y - p_offset_y + PADDING.top;

                        // 尝试检测起始点击位置是否位于某个数据段内，可减少后续划选过程中的检测目标范围
                        let index_vec = (0..buffer_rc.borrow().len()).collect::<Vec<usize>>();
                        let rect = push_from_point.as_rect();
                        if let Some(row) = locate_target_rd(&mut push_from_point, rect, ctx.w(), buffer_rc.borrow().as_slices().0, index_vec) {
                            select_from_row = row;
                        }

                        #[cfg(target_os = "linux")]
                        if let Some(mut parent) = scroller.parent() {
                            parent.set_damage(true);
                        }

                        return true;
                    }
                    Event::Drag => {
                        let (current_x, current_y) = app::event_coords();
                        let (p_offset_x, p_offset_y) = (ctx.x(), ctx.y());
                        let mut current_point = ClickPoint::new(current_x - p_offset_x, current_y - p_offset_y + PADDING.top);
                        if let Some(_) = update_selection_when_drag(
                            push_from_point,
                            select_from_row,
                            &mut current_point,
                            buffer_rc.borrow().as_slices().0,
                            selected_pieces.clone(),
                            ctx
                        ) {
                            // selected.set(ret);
                            let need_redraw = !selected_pieces.borrow().is_empty();
                            selected.set(need_redraw);
                            if need_redraw {
                                // debug!("{need_redraw}");
                                Self::draw_offline(
                                    screen_rc.clone(),
                                    ctx,
                                    visible_lines_rc.clone(),
                                    clickable_data_rc.clone(),
                                    bg_rc.get(),
                                    buffer_rc.clone(),
                                    blink_flag_rc.clone(),
                                );
                            }

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

        Self { panel, data_buffer, background_color, buffer_max_lines, notifier, inner, reviewer, panel_screen, visible_lines, clickable_data, blink_flag, text_font, text_color, text_size, piece_spacing}
    }

    /// 检查是否应该关闭回顾区，若满足关闭条件则关闭回顾区并记录待销毁的回顾区组件。
    fn should_hide_reviewer(
        reviewer_rc: Rc<RefCell<Option<RichReviewer>>>,
        flex: &mut Flex,
        panel_rc: &Widget,
        should_resize: Rc<Cell<i32>>
    ) {
        let mut should_remove = false;
        if let Some(reviewer) = &*reviewer_rc.borrow() {
            let dy = reviewer.scroller.yposition();
            if dy == reviewer.panel.height() - reviewer.scroller.height() {
                let h = flex.h();
                flex.remove(&reviewer.scroller);
                flex.fixed(panel_rc, h);
                flex.recalc();

                // 替换新的离线绘制板
                should_resize.set(h);
                flex.set_damage(true);
                should_remove = true;
            }
        }

        if should_remove {
            if let Some(rv) = reviewer_rc.replace(None) {
                app::delete_widget(rv.scroller);
            }
        }
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
        let default_font_text = !user_data.custom_font_text;
        let default_font_color = !user_data.custom_font_color;
        let mut rich_data: RichData = user_data.into();
        rich_data.piece_spacing = self.piece_spacing.get();
        if default_font_text {
            rich_data.font = self.text_font.get();
            rich_data.font_size = self.text_size.get();
        }
        if default_font_color {
            rich_data.fg_color = self.text_color.get();
        }
        let window_width = self.panel.width();
        let drawable_max_width = window_width - PADDING.left - PADDING.right;

        /*
        试算单元绘制信息
         */
        if !self.data_buffer.borrow().is_empty() {
            if let Some(rd) = self.data_buffer.borrow_mut().iter_mut().last() {
                if let Some(last_piece) = rd.line_pieces.iter().last() {
                    rich_data.estimate(last_piece.clone(), drawable_max_width);
                }
            }
        } else {
            // 首次添加数据
            let last_piece = LinePiece::init_piece();
            rich_data.estimate(last_piece, drawable_max_width);
        }

        self.data_buffer.borrow_mut().push_back(rich_data);
        if self.data_buffer.borrow().len() > self.buffer_max_lines {
            self.data_buffer.borrow_mut().pop_front();
        }

        Self::draw_offline_2(&self);

        // self.panel.redraw();
        self.panel.set_damage(true);
    }



    /// 删除最后一个数据段。
    pub fn delete_last_data(&mut self) {
        self.data_buffer.borrow_mut().pop_back();
        Self::draw_offline_2(&self);
        self.panel.set_damage(true);
    }


    /// 查询目标字符串，并自动显示第一个或最后一个目标所在行。
    /// 若以相同参数重复调用该方法，则每次调用都会自动定位到下一个查找到的目标位置。
    ///
    /// # Arguments
    ///
    /// * `search_str`: 目标字符串。如果给定一个空字符，则清空查询缓存。
    /// * `forward`: true正向查找，false反向查找。
    ///
    /// returns: bool 若查找到目标返回true，否则返回false。
    ///
    /// # Examples
    ///
    /// ```
    /// use fltk::{app, window};
    /// use fltk::button::Button;
    /// use fltk::group::Group;
    /// use fltk::prelude::{GroupExt, WidgetBase, WidgetExt, WindowExt};
    /// use fltkrs_richdisplay::rich_text::RichText;
    ///
    /// let app = app::App::default();
    /// let mut win = window::Window::default().with_size(1000, 1000).with_label("Search").center_screen();
    /// let group = Group::default_fill();
    /// let mut btn1 = Button::new(200, 0, 100, 30, "查找字符串1");
    /// let mut rich_text = RichText::new(100, 120, 800, 400, None);
    /// btn1.set_callback({
    ///     let mut rt = rich_text.clone();
    ///     move |_| {
    ///         rt.search_str(Some("程序".to_string()), false);
    ///     }
    /// });
    /// group.end();
    /// win.end();
    /// win.show();
    ///
    /// while app.wait() {
    ///     app::sleep(0.001);
    ///     app::awake();
    /// }
    /// ```
    pub fn search_str(&mut self, search_str: Option<String>, forward: bool) -> bool {
        let mut find_out = false;
        if let Ok(open_suc) = self.auto_open_reviewer() {
            if let Some(ref mut rr) = *self.reviewer.borrow_mut() {
                if let Some(search_str) = search_str {
                    if !search_str.is_empty() {
                        find_out = rr.search_str(search_str, forward);
                        if !open_suc {
                            // 如果回顾区早已打开，则强制刷新
                            rr.scroller.set_damage(true);
                        }
                    } else {
                        rr.clear_search_results();
                    }
                } else {
                    rr.clear_search_results();
                }
            }
        }

        #[cfg(target_os = "linux")]
        self.set_damage(true);

        find_out
    }

    fn new_offline(
        w: i32, h: i32, offscreen: Rc<RefCell<Offscreen>>,
        panel: &Widget,
        visible_lines: Rc<RefCell<HashMap<Rectangle, LinePiece>>>,
        clickable_data: Rc<RefCell<HashMap<Rectangle, usize>>>,
        bg_color: Color, data_buffer: Rc<RefCell<VecDeque<RichData>>>,
        blink_flag: Rc<Cell<BlinkState>>
        ) {
        if let Some(offs) = Offscreen::new(w, h) {
            offscreen.replace(offs);
            Self::draw_offline(offscreen.clone(), &panel, visible_lines.clone(), clickable_data, bg_color, data_buffer.clone(), blink_flag);
        }
    }

    fn draw_offline(
        offscreen: Rc<RefCell<Offscreen>>,
        panel: &Widget,
        visible_lines: Rc<RefCell<HashMap<Rectangle, LinePiece>>>,
        clickable_data: Rc<RefCell<HashMap<Rectangle, usize>>>,
        bg_color: Color, data_buffer: Rc<RefCell<VecDeque<RichData>>>,
        blink_flag: Rc<Cell<BlinkState>>) {

        offscreen.borrow().begin();
        let (panel_x, panel_y, window_width, window_height) = (panel.x(), panel.y(), panel.width(), panel.height());
        let mut offset_y = 0;

        let mut vl = visible_lines.borrow_mut();
        let mut cd = clickable_data.borrow_mut();
        vl.clear();
        cd.clear();

        // 填充背景
        draw_rect_fill(0, 0, window_width, window_height, bg_color);

        let mut need_blink = false;

        // 绘制数据内容
        let data = data_buffer.borrow();
        let mut set_offset_y = false;
        for (idx, rich_data) in data.iter().enumerate().rev() {
            let bottom_y = rich_data.v_bounds.get().1;
            if !set_offset_y && bottom_y > window_height {
                offset_y = bottom_y - window_height + PADDING.bottom;
                set_offset_y = true;
            }

            if bottom_y < offset_y {
                break;
            }

            // 暂存主体任意部分可见的数据行信息
            for piece in rich_data.line_pieces.iter() {
                let piece = &*piece.borrow();
                let y = piece.y - offset_y + panel_y;
                let rect = Rectangle::new(piece.x + panel_x, y, piece.w, piece.h);
                vl.insert(rect.clone(), piece.clone());

                // 暂存可操作数据信息
                if rich_data.clickable {
                    cd.insert(rect, idx);
                }
            }

            rich_data.draw(offset_y, blink_flag.get());

            if !need_blink && rich_data.blink {
                need_blink = true;
            }
        }

        // 填充顶部边界空白
        draw_rect_fill(0, 0, window_width, PADDING.top, bg_color);

        offscreen.borrow().end();

        // 更新闪烁标记
        if need_blink {
            let bs = blink_flag.get();
            blink_flag.set(bs.on());
        } else {
            let bs = blink_flag.get();
            if bs.is_on() {
                blink_flag.set(bs.off());
            }
        }
    }

    /// 设置面板背景色。
    ///
    /// # Arguments
    ///
    /// * `background_color`: 背景色。
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn set_background_color(&mut self, background_color: Color) {
        self.background_color.replace(background_color);
        if let Some(reviewer) = self.reviewer.borrow().as_ref() {
            reviewer.set_background_color(background_color);
        }
    }

    /// 设置数据缓存最大条数，并非行数。
    ///
    /// # Arguments
    ///
    /// * `max_lines`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn set_cache_size(&mut self, max_lines: usize) {
        self.buffer_max_lines = max_lines;
        if self.data_buffer.borrow().len() > self.buffer_max_lines {
            let r = 0..(self.data_buffer.borrow().len() - self.buffer_max_lines);
            self.data_buffer.borrow_mut().drain(r);
            self.data_buffer.borrow_mut().shrink_to_fit();
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
    pub fn set_notifier<F>(&mut self, cb: F) where F: FnMut(UserData) + 'static {
        let callback = Callback::new(Rc::new(RefCell::new(Box::new(cb))));
        self.notifier.replace(Some(callback));
    }

    pub(crate) fn draw_offline_2(&self) {
        Self::draw_offline(
            self.panel_screen.clone(),
            &self.panel,
            self.visible_lines.clone(),
            self.clickable_data.clone(),
            self.background_color.get(),
            self.data_buffer.clone(),
            self.blink_flag.clone()
        );
    }

    /// 更改数据属性。
    ///
    /// # Arguments
    ///
    /// * `options`: 调整属性。
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    /// use fltk::{app, window};
    /// use fltk::enums::Color;
    /// use fltk::prelude::{GroupExt, WidgetExt};
    /// use fltkrs_richdisplay::rich_text::RichText;
    /// use fltkrs_richdisplay::{RichDataOptions, UserData};
    ///
    /// pub enum GlobalMessage {
    ///     ContentData(UserData),
    ///     UpdateData(RichDataOptions),
    ///     DisableData(i64),
    /// }
    ///
    /// let app = app::App::default();
    /// let mut win = window::Window::default().with_size(1000, 600);
    /// let mut rich_text = RichText::new(100, 100, 800, 400, None);
    /// win.end();
    /// win.show();
    ///
    /// let (sender, mut receiver) = tokio::sync::mpsc::channel::<UserData>(100);
    /// let (global_sender, global_receiver) = app::channel::<GlobalMessage>();
    /// let (global_sender, global_receiver) = app::channel::<GlobalMessage>();
    ///
    /// let global_sender_rc = global_sender.clone();
    /// tokio::spawn(async move {
    ///     while let Some(data) = receiver.recv().await {
    ///         if data.text.starts_with("14") {
    ///             let toggle = !data.underline;
    ///             let update_options = RichDataOptions::new(data.id).underline(toggle);
    ///             global_sender_rc.send(GlobalMessage::UpdateData(update_options));
    ///         } else if data.text.starts_with("22") {
    ///             global_sender_rc.send(GlobalMessage::DisableData(data.id));
    ///         }
    ///     }
    /// });
    ///
    /// while app.wait() {
    ///     if let Some(msg) = global_receiver.recv() {
    ///         match msg {
    ///             GlobalMessage::ContentData(data) => {
    ///                 rich_text.append(data);
    ///             }
    ///             GlobalMessage::UpdateData(options) => {
    ///                 rich_text.update_data(options);
    ///             }
    ///             GlobalMessage::DisableData(id) => {
    ///                 rich_text.disable_data(id);
    ///             }
    ///         }
    ///     }
    ///
    ///     app::sleep(0.001);
    ///     app::awake();
    /// }
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
                update_data_properties(options.clone(), rd);
            }
            self.draw_offline_2();
        }

        if let Some(reviewer) = self.reviewer.borrow_mut().as_mut() {
            reviewer.update_data(options);
        }

        // self.inner.redraw();
        self.inner.set_damage(true);
    }

    /// 禁用数据片段的互动能力，同时伴随显示效果会有变化。
    /// 对于文本段会增加删除线，对于图像会进行灰度处理。
    ///
    /// # Arguments
    ///
    /// * `id`: 数据片段的ID。
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    /// use fltk::{app, window};
    /// use fltk::enums::Color;
    /// use fltk::prelude::{GroupExt, WidgetExt};
    /// use fltkrs_richdisplay::rich_text::RichText;
    /// use fltkrs_richdisplay::{RichDataOptions, UserData};
    ///
    /// pub enum GlobalMessage {
    ///     ContentData(UserData),
    ///     UpdateData(RichDataOptions),
    ///     DisableData(i64),
    /// }
    ///
    /// let app = app::App::default();
    /// let mut win = window::Window::default().with_size(1000, 600);
    /// let mut rich_text = RichText::new(100, 100, 800, 400, None);
    /// win.end();
    /// win.show();
    ///
    /// let (sender, mut receiver) = tokio::sync::mpsc::channel::<UserData>(100);
    /// let (global_sender, global_receiver) = app::channel::<GlobalMessage>();
    /// let (global_sender, global_receiver) = app::channel::<GlobalMessage>();
    ///
    /// let global_sender_rc = global_sender.clone();
    /// tokio::spawn(async move {
    ///     while let Some(data) = receiver.recv().await {
    ///         if data.text.starts_with("14") {
    ///             let toggle = !data.underline;
    ///             let update_options = RichDataOptions::new(data.id).underline(toggle);
    ///             global_sender_rc.send(GlobalMessage::UpdateData(update_options));
    ///         } else if data.text.starts_with("22") {
    ///             global_sender_rc.send(GlobalMessage::DisableData(data.id));
    ///         }
    ///     }
    /// });
    ///
    /// while app.wait() {
    ///     if let Some(msg) = global_receiver.recv() {
    ///         match msg {
    ///             GlobalMessage::ContentData(data) => {
    ///                 rich_text.append(data);
    ///             }
    ///             GlobalMessage::UpdateData(options) => {
    ///                 rich_text.update_data(options);
    ///             }
    ///             GlobalMessage::DisableData(id) => {
    ///                 rich_text.disable_data(id);
    ///             }
    ///         }
    ///     }
    ///
    ///     app::sleep(0.001);
    ///     app::awake();
    /// }
    /// ```
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

            self.draw_offline_2();
        }

        if let Some(reviewer) = self.reviewer.borrow_mut().as_mut() {
            reviewer.disable_data(id);
        }

        // self.inner.redraw();
        self.inner.set_damage(true);
    }

    /// 自动关闭回顾区的接口。当回顾区滚动条已抵达最底部时会关闭回顾区，否则不关闭也不产生额外干扰。
    ///
    /// 通常无需调用此方法，当回顾区的滚动条滚动到最底部时会自动关闭。
    /// 若希望响应PageDown按键关闭回顾区，需要自行在window上注册事件处理逻辑，并调用该接口。
    ///
    /// 该方法适合在调用者的事件处理器当中使用。
    ///
    /// returns: bool 当满足关闭条件时，返回 `true`，否则返回 `false`。对于事件处理器来说，当本方法返回 `true` 时，提示事件应被消耗，否则应忽略当前事件。
    ///
    /// # Examples
    ///
    /// ```
    /// use fltk::enums::{Event, Key};
    /// use fltk::{app, window};
    /// use fltk::prelude::{GroupExt, WidgetBase, WidgetExt};
    /// use fltkrs_richdisplay::rich_text::RichText;
    ///
    /// let app = app::App::default();
    /// let mut win = window::Window::default().with_size(1000, 600);
    /// let rich_text = RichText::new(100, 100, 800, 400, None);
    /// win.handle({
    ///     let rich_text_rc = rich_text.clone();
    ///     move |_, evt| {
    ///         let mut handled = false;
    ///         match evt {
    ///             Event::KeyDown => {
    ///                 if app::event_key_down(Key::PageDown) {
    ///                     handled = rich_text_rc.auto_close_reviewer();
    ///                 } else if app::event_key_down(Key::PageUp) {
    ///                     if let Ok(ret) = rich_text_rc.auto_open_reviewer() {
    ///                         handled = ret;
    ///                     }
    ///                 }
    ///
    ///             }
    ///             _ => {}
    ///         }
    ///         handled
    ///     }
    /// });
    /// win.end();
    /// win.show();
    /// app.run().unwrap();
    /// ```
    pub fn auto_close_reviewer(&self) -> bool {
        if self.reviewer.borrow().is_some() {
            if let Err(e) = app::handle_main(LocalEvent::DROP_REVIEWER_FROM_EXTERNAL) {
                error!("从外部发送关闭回顾区组件事件时出错: {:?}", e);
            }
        }
        false
    }

    /// 自动打开回顾区的接口。当没有显示回顾区时自动打开回顾区，否则不关闭也不产生额外干扰。该方法适合在调用者的事件处理器当中使用。
    ///
    /// 通常无需调用此方法，当鼠标滚轮向上滚动时会自动打开回顾区。
    /// 若希望响应PageUp按键打开回顾区，需要自行在window上注册事件处理逻辑，并调用该接口。
    ///
    /// returns: bool 当满足关闭条件时，返回 `true`，否则返回 `false`。对于事件处理器来说，当本方法返回 `true` 时，提示事件应被消耗，否则应忽略当前事件。
    ///
    /// # Examples
    ///
    /// ```
    /// use fltk::enums::{Event, Key};
    /// use fltk::{app, window};
    /// use fltk::prelude::{GroupExt, WidgetBase, WidgetExt};
    /// use fltkrs_richdisplay::rich_text::RichText;
    ///
    /// let app = app::App::default();
    /// let mut win = window::Window::default().with_size(1000, 600);
    /// let mut rich_text = RichText::new(100, 100, 800, 400, None);
    /// win.handle({
    ///     let mut rich_text_rc = rich_text.clone();
    ///     move |_, evt| {
    ///         let mut handled = false;
    ///         match evt {
    ///             Event::KeyDown => {
    ///                 if app::event_key_down(Key::PageDown) {
    ///                     handled = rich_text_rc.auto_close_reviewer();
    ///                 } else if app::event_key_down(Key::PageUp) {
    ///                     if let Ok(ret) = rich_text_rc.auto_open_reviewer() {
    ///                        handled = ret;
    ///                     }
    ///                 }
    ///
    ///             }
    ///             _ => {}
    ///         }
    ///         handled
    ///     }
    /// });
    /// win.end();
    /// win.show();
    /// app.run().unwrap();
    /// ```
    pub fn auto_open_reviewer(&self) -> Result<bool, FltkError> {
        return if !self.data_buffer.borrow().is_empty() && self.reviewer.borrow().is_none() {
            let handle_result = app::handle_main(LocalEvent::OPEN_REVIEWER_FROM_EXTERNAL);
            match handle_result {
                Ok(handled) => {Ok(handled)}
                Err(e) => {
                    error!("从外部发送打开回顾区组件事件时出错: {:?}", e);
                    Err(e)
                }
            }
        } else {
            Ok(false)
        }
    }

    /// 设置默认的字体，并与`fltk`的其他输入型组件同名接口方法保持兼容。
    ///
    /// # Arguments
    ///
    /// * `font`: 默认字体。
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn set_text_font(&mut self, font: Font) {
        // let old_font = self.text_font;
        self.text_font.set(font);

        // // 将缓存数据中的默认旧字体全部更换为新字体
        // self.data_buffer.borrow_mut().iter_mut().for_each(|rd| {
        //     if rd.font == old_font {
        //         rd.font = font;
        //     }
        // });
    }

    /// 获取默认的字体。
    pub fn text_font(&self) -> Font {
        self.text_font.get()
    }

    /// 设置默认的字体颜色，并与`fltk`的其他输入型组件同名接口方法保持兼容。
    ///
    /// # Arguments
    ///
    /// * `color`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn set_text_color(&mut self, color: Color) {
        // let old_cold = self.text_color;
        self.text_color.set(color);

        // // 将缓存数据中的默认旧字体颜色全部更换为新字体颜色
        // self.data_buffer.borrow_mut().iter_mut().for_each(|rd| {
        //     if rd.fg_color == old_cold {
        //         rd.fg_color = color;
        //     }
        // });
    }

    /// 获取默认的字体颜色。
    pub fn text_color(&self) -> Color {
        self.text_color.get()
    }

    /// 设置默认的字体尺寸，并与`fltk`的其他输入型组件同名接口方法保持兼容。
    ///
    /// # Arguments
    ///
    /// * `color`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn set_text_size(&mut self, size: i32) {
        // let old_size = self.text_size;
        self.text_size.set(size);

        // // 将缓存数据中的默认旧字体尺寸全部更换为新字体尺寸
        // self.data_buffer.borrow_mut().iter_mut().for_each(|rd| {
        //     if rd.font_size == old_size {
        //         rd.font_size = size
        //     }
        // });
    }

    /// 获取默认的字体尺寸。
    pub fn text_size(&self) -> i32 {
        self.text_size.get()
    }

    /// 设置单个数据被自动分割成适应行宽的片段之间的水平间距（像素数，自动缩放），默认为0。
    ///
    /// # Arguments
    ///
    /// * `spacing`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn set_piece_spacing(&mut self, spacing: i32) {
        self.piece_spacing.set(spacing);
    }


    /// 可以在app中使用的获取雪花流水号的工具方法。
    pub fn get_next_sn(&self) -> i64 {
        YitIdHelper::next_id()
    }
}
