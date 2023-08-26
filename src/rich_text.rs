//! 富文本查看器组件。

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::OnceLock;

use fltk::draw::{draw_rect_fill, Offscreen};
use fltk::enums::{Color, Cursor, Event};
use fltk::frame::Frame;
use fltk::prelude::{GroupExt, WidgetBase, WidgetExt};
use fltk::{app, draw, widget_extends};
use fltk::app::MouseWheel;
use fltk::group::{Flex, Scroll};
use crate::{Coordinates, disable_data, LinedData, LinePiece, LocalEvent, mouse_enter, PADDING, RichData, RichDataOptions, update_data_properties, UserData};

use idgenerator_thin::{IdGeneratorOptions, YitIdHelper};
use log::{error};
use crate::rich_reviewer::RichReviewer;

static ID_GENERATOR_INIT: OnceLock<u8> = OnceLock::new();

pub const MAIN_PANEL_FIX_HEIGHT: i32 = 200;
pub const PANEL_PADDING: i32 = 8;

#[derive(Debug, Clone)]
pub struct RichText {
    panel: Frame,
    data_buffer: Rc<RefCell<VecDeque<RichData>>>,
    background_color: Rc<Cell<Color>>,
    buffer_max_lines: usize,
    notifier: Rc<RefCell<Option<tokio::sync::mpsc::Sender<UserData>>>>,
    inner: Flex,
    reviewer: Rc<RefCell<Option<RichReviewer>>>,
    panel_screen: Rc<RefCell<Offscreen>>,
    visible_lines: Rc<RefCell<HashMap<Coordinates, usize>>>,
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

        let background_color = Rc::new(Cell::new(Color::Black));
        let reviewer = Rc::new(RefCell::new(None::<RichReviewer>));

        let mut inner = Flex::new(x, y, w, h, title).column();
        inner.set_pad(0);
        inner.end();


        let mut panel = Frame::new(x, y, w, h, None);
        inner.add(&panel);

        let panel_screen = Rc::new(RefCell::new(Offscreen::new(w, h).unwrap()));

        let buffer_max_lines = 100;
        let data_buffer = Rc::new(RefCell::new(VecDeque::<RichData>::with_capacity(buffer_max_lines + 1)));

        let visible_lines = Rc::new(RefCell::new(HashMap::<Coordinates, usize>::new()));
        let notifier: Rc<RefCell<Option<tokio::sync::mpsc::Sender<UserData>>>> = Rc::new(RefCell::new(None));
        let to_be_dropped_reviewer = Rc::new(Cell::new(None::<(Scroll, Frame)>));

        panel.draw({
            let screen_rc = panel_screen.clone();
            move |ctx| {
                let (x, y, window_width, window_height) = (ctx.x(), ctx.y(), ctx.width(), ctx.height());
                screen_rc.borrow().copy(x, y, window_width, window_height, 0, 0);
            }
        });

        inner.handle({
            let last_window_size = Rc::new(Cell::new((0, 0)));
            let panel_rc = panel.clone();
            let reviewer_rc = reviewer.clone();
            let buffer_rc = data_buffer.clone();
            let bg_rc = background_color.clone();
            let visible_lines_rc = visible_lines.clone();
            let screen_rc = panel_screen.clone();
            let to_be_dropped_reviewer_rc = to_be_dropped_reviewer.clone();
            let notifier_rc = notifier.clone();
            move |flex, evt| {
                if evt == LocalEvent::DROP_REVIEWER_FROM_EXTERNAL.into() {
                    // 隐藏回顾区
                    let (should_remove, drop_target) = Self::should_hide_reviewer(
                        reviewer_rc.clone(),
                        flex,
                        &panel_rc,
                        screen_rc.clone(),
                        bg_rc.clone(),
                        visible_lines_rc.clone(),
                        buffer_rc.clone()
                    );
                    return if should_remove {
                        reviewer_rc.replace(None);
                        to_be_dropped_reviewer_rc.replace(drop_target);
                        if let Err(e) = app::handle_main(LocalEvent::DROP_REVIEWER) {
                            error!("发送删除回顾事件时发生错误：{}", e);
                        }
                        true
                    } else {
                        false
                    }
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
                    Self::new_offline(
                        flex.width(),
                        MAIN_PANEL_FIX_HEIGHT,
                        screen_rc.clone(),
                        &panel_rc,
                        visible_lines_rc.clone(),
                        bg_rc.get(),
                        buffer_rc.clone()
                    );

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
                                if let Some(notifier_rc) = notifier_rc.borrow().as_ref() {
                                    reviewer.set_notifier(notifier_rc.clone());
                                }
                                let snapshot = Vec::from(buffer_rc.borrow().clone());
                                reviewer.set_data(snapshot);
                                flex.insert(&reviewer.scroller, 0);
                                flex.fixed(&panel_rc, MAIN_PANEL_FIX_HEIGHT);
                                flex.recalc();

                                // 替换新的离线绘制板
                                Self::new_offline(
                                    flex.width(),
                                    MAIN_PANEL_FIX_HEIGHT,
                                    screen_rc.clone(),
                                    &panel_rc,
                                    visible_lines_rc.clone(),
                                    bg_rc.get(),
                                    buffer_rc.clone()
                                );

                                reviewer.scroll_to_bottom();
                                reviewer_rc.replace(Some(reviewer));

                            } else if app::event_dy() == MouseWheel::Up && reviewer_rc.borrow().is_some() {
                                // 隐藏回顾区
                                let (should_remove, drop_target) = Self::should_hide_reviewer(
                                    reviewer_rc.clone(),
                                    flex,
                                    &panel_rc,
                                    screen_rc.clone(),
                                    bg_rc.clone(),
                                    visible_lines_rc.clone(),
                                    buffer_rc.clone()
                                );
                                if should_remove {
                                    reviewer_rc.replace(None);
                                    to_be_dropped_reviewer_rc.replace(drop_target);
                                    if let Err(e) = app::handle_main(LocalEvent::DROP_REVIEWER) {
                                        error!("发送删除回顾事件时发生错误：{}", e);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                    false
                }
            }
        });

        /*
        处理窗口事件
         */
        panel.handle({
            let buffer_rc = data_buffer.clone();
            let last_window_size = Rc::new(Cell::new((0, 0)));
            let visible_lines_rc = visible_lines.clone();
            let notifier_rc = notifier.clone();
            let screen_rc = panel_screen.clone();
            let bg_rc = background_color.clone();
            let to_be_dropped_reviewer_rc = to_be_dropped_reviewer.clone();
            move |ctx, evt| {
                if evt == LocalEvent::DROP_REVIEWER.into() {
                    // 销毁组件，回收内存，否则会有内存泄漏。
                    let target = to_be_dropped_reviewer_rc.replace(None);
                    if let Some((scroller, panel)) = target {
                        app::delete_widget(scroller);
                        app::delete_widget(panel);
                    }
                    true
                } else {
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
                                Self::new_offline(
                                    current_width,
                                    current_height,
                                    screen_rc.clone(),
                                    &ctx,
                                    visible_lines_rc.clone(),
                                    bg_rc.get(),
                                    buffer_rc.clone()
                                );
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
                                                    error!("send error: {:?}", e);
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

            }
        });

        Self { panel, data_buffer, background_color, buffer_max_lines, notifier, inner, reviewer, panel_screen, visible_lines }
    }

    fn should_hide_reviewer(
        reviewer_rc: Rc<RefCell<Option<RichReviewer>>>,
        flex: &mut Flex,
        panel_rc: &Frame,
        screen_rc: Rc<RefCell<Offscreen>>,
        bg_rc: Rc<Cell<Color>>,
        visible_lines_rc: Rc<RefCell<HashMap<Coordinates, usize>>>,
        buffer_rc: Rc<RefCell<VecDeque<RichData>>>
    ) -> (bool, Option<(Scroll, Frame)>){
        if let Some(reviewer) = &*reviewer_rc.borrow() {
            let dy = reviewer.scroller.yposition();
            if dy == reviewer.panel.height() - reviewer.scroller.height() {
                let full_height = flex.height();
                flex.remove(&reviewer.scroller);
                flex.fixed(panel_rc, flex.height());
                flex.recalc();

                // 替换新的离线绘制板
                Self::new_offline(
                    flex.width(),
                    full_height,
                    screen_rc.clone(),
                    panel_rc,
                    visible_lines_rc.clone(),
                    bg_rc.get(),
                    buffer_rc.clone()
                );

                return (true, Some((reviewer.scroller.clone(), reviewer.panel.clone())));
            }
        }
        return (false, None);
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

        Self::draw_offline(self.panel_screen.clone(), &self.panel, self.visible_lines.clone(), self.background_color.get(), self.data_buffer.clone());

        self.panel.redraw();
    }

    pub fn new_offline(w: i32, h: i32, offscreen: Rc<RefCell<Offscreen>>, panel: &Frame, visible_lines: Rc<RefCell<HashMap<Coordinates, usize>>>, bg_color: Color, data_buffer: Rc<RefCell<VecDeque<RichData>>>) {
        if let Some(offs) = Offscreen::new(w, h) {
            offscreen.replace(offs);
            Self::draw_offline(offscreen.clone(), &panel, visible_lines.clone(), bg_color, data_buffer.clone());
        }
    }

    pub fn draw_offline(offscreen: Rc<RefCell<Offscreen>>, panel: &Frame, visible_lines: Rc<RefCell<HashMap<Coordinates, usize>>>, bg_color: Color, data_buffer: Rc<RefCell<VecDeque<RichData>>>) {
        offscreen.borrow().begin();
        let (panel_x, panel_y, window_width, window_height) = (panel.x(), panel.y(), panel.width(), panel.height());
        let mut offset_y = 0;
        visible_lines.borrow_mut().clear();

        // 填充背景
        draw_rect_fill(0, 0, window_width, window_height, bg_color);

        let data = data_buffer.borrow();

        let mut set_offset_y = false;
        for (idx, rich_data) in data.iter().enumerate().rev() {
            if let Some((_, bottom_y, _)) = rich_data.v_bounds {
                if !set_offset_y && bottom_y > window_height {
                    offset_y = bottom_y - window_height + PADDING.bottom;
                    set_offset_y = true;
                }

                if bottom_y < offset_y {
                    break;
                }
                rich_data.draw(offset_y, idx, visible_lines.clone(), panel_x, panel_y);
            }

        }

        // 填充顶部边界空白
        draw_rect_fill(0, 0, window_width, PADDING.top, bg_color);

        offscreen.borrow().end();
    }

    pub fn set_background_color(&mut self, background_color: Color) {
        self.background_color.replace(background_color);
        if let Some(reviewer) = self.reviewer.borrow().as_ref() {
            reviewer.set_background_color(background_color);
        }
    }

    /// 设置缓冲区最大数据条数，并非行数。
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
    pub fn set_buffer_max_lines(&mut self, max_lines: usize) {
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
    pub fn set_notifier(&mut self, notifier: tokio::sync::mpsc::Sender<UserData>) {
        self.notifier.replace(Some(notifier));
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
                update_data_properties(options.clone(), rd);
            }
            Self::draw_offline(self.panel_screen.clone(), &self.panel, self.visible_lines.clone(), self.background_color.get(), self.data_buffer.clone());
        }

        if let Some(reviewer) = self.reviewer.borrow_mut().as_mut() {
            reviewer.update_data(options);
        }

        self.inner.redraw();
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

            Self::draw_offline(self.panel_screen.clone(), &self.panel, self.visible_lines.clone(), self.background_color.get(), self.data_buffer.clone());
        }

        if let Some(reviewer) = self.reviewer.borrow_mut().as_mut() {
            reviewer.disable_data(id);
        }

        self.inner.redraw();
    }

    pub fn auto_close_reviewer(&mut self) -> bool {
        return if self.reviewer.borrow().is_some() {
            let handle_result = app::handle_main(LocalEvent::DROP_REVIEWER_FROM_EXTERNAL);
            match handle_result {
                Ok(handled) => {handled}
                Err(e) => {
                    error!("从外部发送关闭回顾区组件事件时出错: {:?}", e);
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn auto_open_reviewer(&mut self) -> bool {
        return if !self.data_buffer.borrow().is_empty() && self.reviewer.borrow().is_none() {
            let handle_result = app::handle_main(LocalEvent::OPEN_REVIEWER_FROM_EXTERNAL);
            match handle_result {
                Ok(handled) => {handled}
                Err(e) => {
                    error!("从外部发送打开回顾区组件事件时出错: {:?}", e);
                    false
                }
            }
        } else {
            false
        }
    }
}
