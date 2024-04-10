//! 富文本查看器组件。

use std::cmp::{max};
use std::collections::{HashMap};
use std::fmt::{Debug};
use std::rc::{Rc};
use std::sync::{Arc, Weak};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicUsize, Ordering};
use std::time::{Duration};
use debounce_fltk::TokioDebounce;

use fltk::draw::{draw_line, draw_rect_fill, measure, Offscreen, set_draw_color};
use fltk::enums::{Color, Cursor, Event, Font};
use fltk::prelude::{FltkError, GroupExt, MenuExt, WidgetBase, WidgetExt};
use fltk::{app, draw, widget_extends};
use fltk::app::{MouseButton, MouseWheel};
use fltk::frame::Frame;
use fltk::group::{Flex};
use fltk::menu::{MenuButton, MenuButtonType};
use crate::{Rectangle, disable_data, LinedData, LinePiece, LocalEvent, mouse_enter, PADDING, RichData, RichDataOptions, update_data_properties, UserData, BLINK_INTERVAL, BlinkState, Callback, DEFAULT_FONT_SIZE, WHITE, clear_selected_pieces, ClickPoint, locate_target_rd, update_selection_when_drag, CallbackData, ShapeData, LINE_HEIGHT_FACTOR, BASIC_UNIT_CHAR, DEFAULT_TAB_WIDTH, DocEditType, BlinkDegree, DataType, ImageEventData, IMAGE_PADDING_V, expire_data, select_paragraph};

use log::{debug, error};
use parking_lot::RwLock;
use crate::rewrite_board::ReWriteBoard;
use crate::rich_reviewer::RichReviewer;


pub const MAIN_PANEL_FIX_HEIGHT: i32 = 200;
pub const PANEL_PADDING: i32 = 8;

pub const MAX_SIZE_OF_TEMP_BUFFER: usize = 1024 * 1024 * 10;

// static FULL_DRAW: Lazy<Arc<AtomicBool>> = Lazy::new(|| Arc::new(AtomicBool::new(false)));

// #[derive(Debug, Clone)]
// struct ThrottleHolder {
//     pub last_rid: i64,
//     pub current_rid: i64,
// }


/// rich-display主面板结构。
#[derive(Debug, Clone)]
pub struct RichText {
    panel: Frame,
    data_buffer: Arc<RwLock<Option<Vec<RichData>>>>,
    // temp_buffer: Arc<RwLock<Option<Vec<RichData>>>>,
    current_buffer: Arc<RwLock<Vec<RichData>>>,
    background_color: Arc<RwLock<Color>>,
    buffer_max_lines: Arc<AtomicUsize>,
    notifier: Arc<RwLock<Option<Callback>>>,
    inner: Flex,
    reviewer: Arc<RwLock<Option<RichReviewer>>>,
    // panel_screen: Arc<RwLock<Offscreen>>,
    // clickable_data: Arc<RwLock<HashMap<Rectangle, usize>>>,
    // /// 主面板上可见行片段的集合容器，在每次离线绘制时被清空和填充。
    // visible_lines: Arc<RwLock<HashMap<Rectangle, LinePiece>>>,
    blink_flag: Arc<RwLock<BlinkState>>,
    /// 默认字体。
    text_font: Arc<RwLock<Font>>,
    /// 默认字体颜色。
    text_color: Arc<RwLock<Color>>,
    text_size: Arc<AtomicI32>,
    piece_spacing: Arc<AtomicI32>,
    // throttle_holder: Arc<RwLock<ThrottleHolder>>,
    enable_blink: Arc<AtomicBool>,
    basic_char: Arc<RwLock<char>>,
    tab_width: Arc<AtomicU8>,
    /// 虚拟光标，零宽度。
    cursor_piece: Arc<RwLock<LinePiece>>,
    show_cursor: Arc<AtomicBool>,
    /// 远程流控制状态。对应`RFC1080`和`RFC1372`协议所述。
    /// 在本地实现为光标位置控制方式，当为`true`时按照本地顺序流单向移动光标位置，为当`false`时按照服务端发送过来的光标控制信息全屏移动光标位置。
    remote_flow_control: Arc<AtomicBool>,
    rewrite_board: Arc<RwLock<Option<ReWriteBoard>>>,
    max_rows: Arc<AtomicUsize>,
    max_cols: Arc<AtomicUsize>,
    update_panel_fn: Arc<RwLock<TokioDebounce<bool>>>
}
widget_extends!(RichText, Flex, inner);



impl RichText {
    pub fn new<T>(x: i32, y: i32, w: i32, h: i32, title: T) -> Self
        where T: Into<Option<&'static str>> + Clone {

        let text_font = Arc::new(RwLock::new(Font::Helvetica));
        let text_color = Arc::new(RwLock::new(WHITE));
        let text_size = Arc::new(AtomicI32::new(DEFAULT_FONT_SIZE));
        let piece_spacing = Arc::new(AtomicI32::new(0));

        let background_color = Arc::new(RwLock::new(Color::Black));
        let reviewer = Arc::new(RwLock::new(None::<RichReviewer>));

        // let mut inner = Flex::new(x, y, w, h, title).column(); // fltk 1.4.15变更为私有函数
        let mut inner = <Flex as WidgetBase>::new(x, y, w, h, title).column();
        inner.set_pad(0);
        inner.set_margin(0);
        inner.end();

        // let mut panel = Widget::new(x, y, w, h, None);
        let mut panel = Frame::new(x, y, w, h, None);

        inner.add(&panel);

        // let mut popup_menu = MenuButton::default();
        // popup_menu.set_type(MenuButtonType::Popup1);
        // popup_menu.hide();
        // inner.add(&popup_menu);

        let panel_screen = Arc::new(RwLock::new(Offscreen::new(w, h).unwrap()));

        let buffer_max_lines = 100;
        let data_buffer = Arc::new(RwLock::new(Some(Vec::<RichData>::with_capacity(buffer_max_lines + 1))));

        let visible_lines = Arc::new(RwLock::new(HashMap::<Rectangle, LinePiece>::new()));
        let clickable_data = Arc::new(RwLock::new(HashMap::<Rectangle, usize>::new()));
        let notifier: Arc<RwLock<Option<Callback>>> = Arc::new(RwLock::new(None));
        let selected = Arc::new(AtomicBool::new(false));
        let should_resize_content = Arc::new(AtomicI32::new(0));
        let enable_blink = Arc::new(AtomicBool::new(true));
        let basic_char = Arc::new(RwLock::new(BASIC_UNIT_CHAR));
        let tab_width = Arc::new(AtomicU8::new(DEFAULT_TAB_WIDTH));
        let cursor_piece = LinePiece::init_piece(DEFAULT_FONT_SIZE);
        let show_cursor = Arc::new(AtomicBool::new(false));
        let remote_flow_control = Arc::new(AtomicBool::new(true));
        // let temp_buffer = Arc::new(RwLock::new(Some(Vec::new())));
        let current_buffer = Arc::new(RwLock::new(Vec::new()));
        let rewrite_board: Arc<RwLock<Option<ReWriteBoard>>> = Arc::new(RwLock::new(None));
        let max_rows = Arc::new(AtomicUsize::new(1usize));
        let max_cols = Arc::new(AtomicUsize::new(1usize));

        let _ = Self::update_window_size(
            text_font.clone(),
            text_size.clone(),
            basic_char.clone(),
            w,
            h,
            max_rows.clone(),
            max_cols.clone(),
            rewrite_board.clone(),
        );

        // 数据段闪烁控制器
        let blink_flag = Arc::new(RwLock::new(BlinkState::new()));

        let update_panel_fn = Arc::new(RwLock::new(TokioDebounce::new_debounce({
            let mut panel_rc = panel.clone();
            let screen_rc = panel_screen.clone();
            let visible_lines_rc = visible_lines.clone();
            let clickable_data_rc = clickable_data.clone();
            let bg_rc = background_color.clone();
            let buffer_rc = current_buffer.clone();
            let blink_flag_rc = blink_flag.clone();
            let show_cursor_rc = show_cursor.clone();
            let cursor_piece_rc = cursor_piece.clone();
            move |redraw: bool| {
                let enable_cursor = if show_cursor_rc.load(Ordering::Relaxed) {
                    Some(cursor_piece_rc.clone())
                } else {
                    None
                };
                // debug!("update_panel_fn");
                Self::draw_offline(
                    screen_rc.clone(),
                    &mut panel_rc,
                    visible_lines_rc.clone(),
                    clickable_data_rc.clone(),
                    *bg_rc.read(),
                    buffer_rc.clone(),
                    blink_flag_rc.clone(),
                    enable_cursor,
               );
                if redraw {
                    panel_rc.redraw();
                }
               // panel_rc.set_damage(true);
           }
        }, Duration::from_millis(20), true)));

        let mut create_reviewer_fn = TokioDebounce::new_throttle({
            let mut flex = inner.clone();
            let panel_rc = panel.clone();
            let buffer_rc = current_buffer.clone();
            let main_buffer = data_buffer.clone();
            let selected_rc = selected.clone();
            let enable_blink_rc = enable_blink.clone();
            let blink_flag_rc = blink_flag.clone();
            let basic_char_rc = basic_char.clone();
            let bg_rc = background_color.clone();
            let notifier_rc = notifier.clone();
            let remote_flow_control_rc = remote_flow_control.clone();
            let reviewer_rc = reviewer.clone();
            let update_panel_fn = update_panel_fn.clone();
            let should_resize = should_resize_content.clone();
            move |()| {
                // 显示回顾区
                let mut reviewer = RichReviewer::new(0, 0, flex.width(), flex.height() - MAIN_PANEL_FIX_HEIGHT, None);
                reviewer.set_enable_blink(enable_blink_rc.load(Ordering::Relaxed));
                reviewer.set_blink_state(blink_flag_rc.read().clone());
                reviewer.set_background_color(*bg_rc.read());
                reviewer.set_basic_char(*basic_char_rc.read());
                if let Some(notifier_rc_ref) = notifier_rc.write().as_mut() {
                    let cb = notifier_rc_ref.clone();
                    reviewer.set_notifier(cb);
                }
                // let drawable_max_width = flex.w() - PADDING.left - PADDING.right;
                // let mut snapshot = Self::create_snapshot(buffer_rc.clone());
                let mut snapshot = if remote_flow_control_rc.load(Ordering::SeqCst) {
                    // 当前缓存就是主缓存
                    buffer_rc.read().clone()
                } else {
                    // 当前缓存是临时缓存，主缓存位于data_buffer中。
                    if let Some(mb) = main_buffer.read().as_ref() {
                        mb.clone()
                    } else {
                        vec![]
                    }
                };
                if selected_rc.load(Ordering::Relaxed) {
                    snapshot.iter_mut().for_each(|rd| {
                        rd.line_pieces.iter_mut().for_each(|piece| {
                            piece.read().deselect();
                        })
                    });
                }

                // debug!("历史数据长度：{}", snapshot.len());

                reviewer.set_data(snapshot);
                flex.insert(&reviewer.scroller, 0);
                // flex.resizable(&reviewer.scroller);
                flex.fixed(&panel_rc, MAIN_PANEL_FIX_HEIGHT);
                flex.recalc();

                should_resize.store(MAIN_PANEL_FIX_HEIGHT, Ordering::Relaxed);

                reviewer.scroll_to_bottom();
                reviewer_rc.write().replace(reviewer);
                update_panel_fn.write().update_param(false);
                // debug!("打开回顾区");
                flex.set_damage(true);

                false
            }
        }, Duration::from_millis(100), true);

        let blink_handler = {
            let blink_flag_rc = blink_flag.clone();
            let panel_rc = panel.clone();
            let enable_blink_rc = enable_blink.clone();
            let show_cursor_rc = show_cursor.clone();
            let update_panel_fn = update_panel_fn.clone();
            move |handler| {
                if !panel_rc.was_deleted() {
                    if enable_blink_rc.load(Ordering::Relaxed) {
                        if show_cursor_rc.load(Ordering::Relaxed) {
                            blink_flag_rc.write().on();
                        }
                        let should_toggle = blink_flag_rc.write().toggle_when_on();
                        if should_toggle {
                            // FULL_DRAW.store(false, Ordering::Relaxed);
                            update_panel_fn.write().update_param(false);
                        }
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
            let buffer_rc = current_buffer.clone();
            let blink_flag_rc = blink_flag.clone();
            let show_cursor_rc = show_cursor.clone();
            let cursor_piece_rc = cursor_piece.clone();
            move |ctx| {
                // debug!("绘制主面板");
                let h = resize_to.fetch_add(0, Ordering::Relaxed);
                if h != 0 {
                    let enable_cursor = if show_cursor_rc.load(Ordering::Relaxed) {
                        Some(cursor_piece_rc.clone())
                    } else {
                        None
                    };
                    Self::new_offline(
                        flex.width(),
                        h,
                        screen_rc.clone(),
                        ctx,
                        visible_lines_rc.clone(),
                        clickable_data_rc.clone(),
                        *bg_rc.read(),
                        buffer_rc.clone(),
                        blink_flag_rc.clone(),
                        enable_cursor,
                    );
                }
                screen_rc.read().copy(ctx.x(), ctx.y(), ctx.width(), ctx.height(), 0, 0);
            }
        });

        /*
        处理主面板容器的动作事件，打开或关闭回顾区。
         */
        inner.handle({
            let last_window_size = Arc::new(RwLock::new((0, 0)));
            let panel_rc = panel.clone();
            let reviewer_rc = reviewer.clone();
            let main_buffer = data_buffer.clone();
            let buffer_rc = current_buffer.clone();
            let bg_rc = background_color.clone();
            let notifier_rc = notifier.clone();
            let should_resize = should_resize_content.clone();
            let enable_blink_rc = enable_blink.clone();
            let blink_flag_rc = blink_flag.clone();
            let basic_char_rc = basic_char.clone();
            let remote_flow_control_rc = remote_flow_control.clone();
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
                    reviewer.set_enable_blink(enable_blink_rc.load(Ordering::Relaxed));
                    reviewer.set_blink_state(blink_flag_rc.read().clone());
                    reviewer.set_background_color(*bg_rc.read());
                    reviewer.set_basic_char(*basic_char_rc.read());
                    if let Some(notifier_rc) = notifier_rc.read().as_ref() {
                        reviewer.set_notifier(notifier_rc.clone());
                    }
                    // let drawable_max_width = flex.w() - PADDING.left - PADDING.right;
                    // let snapshot = Self::create_snapshot(buffer_rc.clone());
                    let snapshot = if remote_flow_control_rc.load(Ordering::SeqCst) {
                        // 当前缓存就是主缓存
                        buffer_rc.read().clone()
                    } else {
                        // 当前缓存是临时缓存，主缓存位于data_buffer中。
                        if let Some(mb) = main_buffer.read().as_ref() {
                            mb.clone()
                        } else {
                            vec![]
                        }
                    };
                    reviewer.set_data(snapshot);
                    flex.insert(&reviewer.scroller, 0);
                    flex.fixed(&panel_rc, MAIN_PANEL_FIX_HEIGHT);
                    flex.recalc();

                    // 替换新的离线绘制板
                    should_resize.store(MAIN_PANEL_FIX_HEIGHT, Ordering::Relaxed);

                    reviewer.scroll_to_bottom();
                    reviewer_rc.write().replace(reviewer);
                    true
                } else {
                    match evt {
                        Event::Resize => {
                            let (current_width, current_height) = (flex.width(), flex.height());
                            let (last_width, last_height) = (*last_window_size.read()).clone();
                            if last_width != current_width || last_height != current_height {
                                {
                                    let mut lws = last_window_size.write();
                                    lws.0 = current_width;
                                    lws.1 = current_height;
                                }
                                let panel_height = if reviewer_rc.read().is_some() {
                                    MAIN_PANEL_FIX_HEIGHT
                                } else {
                                    current_height
                                };
                                flex.fixed(&panel_rc, panel_height);
                                if panel_height != current_height {
                                    // 包含有回顾区，在fltk-rs 1.4.12版本中，需要手动设置其尺寸
                                    if let Some(rv) = &*reviewer_rc.read() {
                                        flex.fixed(&rv.scroller, current_height - panel_height);
                                    }
                                }
                                // flex.recalc();
                            }
                            // debug!("容器面板缩放");
                        }
                        Event::MouseWheel => {
                            /*
                            显示或隐藏回顾区。
                             */
                            if app::event_inside_widget(flex) {
                                if app::event_dy() == MouseWheel::Down && reviewer_rc.read().is_none() {
                                    create_reviewer_fn.update_param(());
                                } else if app::event_dy() == MouseWheel::Up && reviewer_rc.read().is_some() {
                                    // 隐藏回顾区
                                    Self::should_hide_reviewer(
                                        reviewer_rc.clone(),
                                        flex,
                                        &panel_rc,
                                        should_resize.clone()
                                    );
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
        处理主面板缩放及鼠标操作事件。
         */
        panel.handle({
            let buffer_rc = current_buffer.clone();
            let last_window_size = Arc::new(RwLock::new((0, 0)));
            let clickable_data_rc = clickable_data.clone();
            let notifier_rc = notifier.clone();
            let selected = selected.clone();
            let mut select_from_row = 0;
            let mut push_from_point = ClickPoint::new(0, 0);
            let selected_pieces = Arc::new(RwLock::new(Vec::<Weak<RwLock<LinePiece>>>::new()));
            let should_resize = should_resize_content.clone();
            let text_font_rc = text_font.clone();
            let text_size_rc = text_size.clone();
            let basic_char_rc = basic_char.clone();
            let rewrite_board_rc = rewrite_board.clone();
            let max_rows_rc = max_rows.clone();
            let max_cols_rc = max_cols.clone();
            let update_panel_fn = update_panel_fn.clone();
            move |ctx, evt| {
                // let enable_cursor = if show_cursor_rc.load(Ordering::Relaxed) {
                //     Some(cursor_piece_rc.clone())
                // } else {
                //     None
                // };
                match evt {
                    Event::Resize => {
                        // 缩放窗口后重新计算分片绘制信息。
                        let (current_width, current_height) = (ctx.width(), ctx.height());
                        let (last_width, last_height) = (*last_window_size.read()).clone();
                        if last_width != current_width || last_height != current_height {
                            {
                                let mut lws = last_window_size.write();
                                lws.0 = current_width;
                                lws.1 = current_height;
                            }
                            if last_width != current_width {
                                // 当窗口宽度发生变化时，需要重新计算数据分片坐标信息。
                                let drawable_max_width = current_width - PADDING.left - PADDING.right;
                                let mut last_piece = LinePiece::init_piece(text_size_rc.load(Ordering::Relaxed));
                                for rich_data in buffer_rc.write().iter_mut() {
                                    rich_data.line_pieces.clear();
                                    last_piece = rich_data.estimate(last_piece, drawable_max_width, *basic_char_rc.read());
                                }
                            }

                            if current_width > 0 || current_height > 0 {
                                let (new_rows, new_cols) = Self::update_window_size(
                                    text_font_rc.clone(),
                                    text_size_rc.clone(),
                                    basic_char_rc.clone(),
                                    current_width,
                                    current_height,
                                    max_rows_rc.clone(),
                                    max_cols_rc.clone(),
                                    rewrite_board_rc.clone(),
                                );

                                if let Some(cb) = notifier_rc.write().as_mut() {
                                    cb.notify(CallbackData::Shape(ShapeData::new(last_width, last_height, current_width, current_height, new_cols, new_rows)));
                                }
                            }

                            // 替换新的离线绘制板
                            should_resize.store(current_height, Ordering::Relaxed);
                        }
                        update_panel_fn.write().update_param(false);
                        // debug!("主面板缩放");
                    }
                    Event::Move => {
                        // 检测鼠标进入可互动区域，改变鼠标样式
                        let (entered, _idx) = mouse_enter(clickable_data_rc.clone());
                        if entered {
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
                        let mut target_opt: Option<UserData> = None;
                        let mut target_rd_v_bounds: Option<(i32, i32, i32, i32)> = None;
                        for (area, idx) in clickable_data_rc.read().iter() {
                            let (x, y, w, h) = area.tup();
                            if app::event_inside(x, y, w, h) {
                                if let Some(rd) = buffer_rc.read().get(*idx) {
                                    target_rd_v_bounds.replace(rd.v_bounds.read().clone());
                                    let sd: UserData = rd.into();
                                    target_opt.replace(sd);
                                }
                                break;
                            }
                        }
                        if app::event_mouse_button() == MouseButton::Right {
                            if let Some(ud) = target_opt {
                                if ud.action.is_some() {
                                    // 右键弹出互动菜单
                                    let ud_rc = Rc::new(ud);
                                    if let Some(action) = &ud_rc.action {
                                        let mut popup_menu_rc = MenuButton::new(0, 0, 0, 0, None);
                                        // popup_menu_rc.clear();
                                        popup_menu_rc.set_type(MenuButtonType::Popup1);
                                        popup_menu_rc.set_color(Color::by_index(214));
                                        popup_menu_rc.set_label_font(Font::Screen);
                                        if !action.title.trim().is_empty() {
                                            // 处理提示信息，添加换行，避免单行过宽。
                                            let new_hint = action.title.chars().fold("".to_string(), |mut s, c| {
                                                s.push(c);
                                                if s.ends_with(". ")
                                                    || s.ends_with("。")
                                                    || s.ends_with("?")
                                                    || s.ends_with("？")
                                                    || s.ends_with("!")
                                                    || s.ends_with("！") {
                                                    s.push('\n');
                                                }
                                                s
                                            });
                                            popup_menu_rc.set_label(new_hint.as_str());
                                        }
                                        for item in action.items.iter() {
                                            popup_menu_rc.add_choice(item.desc.as_str());
                                        }
                                        // 用户选中的菜单项后将其附带到目标数据段中回传到上层应用。
                                        if ud_rc.data_type == DataType::Text {
                                            // 文字类型
                                            popup_menu_rc.set_callback({
                                                let ud_rc_2 = ud_rc.clone();
                                                let notifier_rc = notifier_rc.clone();
                                                move |menu| {
                                                    let selected_idx = menu.value();
                                                    if selected_idx >= 0 {
                                                        let mut ud = ud_rc_2.as_ref().clone();
                                                        if let Some(action) = &mut ud.action {
                                                            if let Some(item) = action.items.get(selected_idx as usize) {
                                                                if let Some(cb) = notifier_rc.write().as_mut() {
                                                                    action.active.replace(item.cmd.clone());
                                                                    cb.notify(CallbackData::Data(ud));
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            });
                                        } else {
                                            // 图片类型
                                            let click_point = if let Some(v_bounds) = target_rd_v_bounds {
                                                let (app_x, app_y) = app::event_coords();
                                                // debug!("rd v_bounds: {:?}, app_coords: {}, {}", v_bounds, app_x, app_y);
                                                let scroll_y = Self::calc_scroll_height(buffer_rc.clone(), ctx.height());
                                                let click_at_x = app_x - ctx.x() - v_bounds.2;
                                                let click_at_y = app_y - ctx.y() + scroll_y - v_bounds.0 - IMAGE_PADDING_V;
                                                // debug!("click_at_x: {}, click_at_y: {}", click_at_x, click_at_y);
                                                (click_at_x, click_at_y)
                                            } else {
                                                (0, 0)
                                            };

                                            popup_menu_rc.set_callback({
                                                let ud_rc_2 = ud_rc.clone();
                                                let notifier_rc = notifier_rc.clone();
                                                move |menu| {
                                                    let selected_idx = menu.value();
                                                    if selected_idx >= 0 {
                                                        let mut ud = ud_rc_2.as_ref().clone();
                                                        if let Some(action) = &mut ud.action {
                                                            if let Some(item) = action.items.get(selected_idx as usize) {
                                                                if let Some(cb) = notifier_rc.write().as_mut() {
                                                                    cb.notify(CallbackData::Image(ImageEventData::new(click_point, ud.image_src_url, ud.id, item.cmd.clone(), ud.image_file_path.clone(), (ud.image_target_width, ud.image_target_height))));
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            });
                                        }

                                        popup_menu_rc.popup();
                                    }
                                } else if let Some(cb) = notifier_rc.write().as_mut() {
                                    // 直接返回当前目标数据
                                    cb.notify(CallbackData::Data(ud));
                                }
                            }
                        } else if app::event_mouse_button() == MouseButton::Left {
                            if app::event_clicks() {
                                // debug!("双击");
                                select_paragraph(select_from_row, &mut push_from_point, buffer_rc.read().as_slice(), selected_pieces.clone());
                                ctx.set_damage(true);
                            } else if let Some(ud) = target_opt {
                                // 左键弹出提示信息
                                // debug!("左键点击：{:?}", ud);
                                if let Some(action) = &ud.action {
                                    let mut popup_menu_rc = MenuButton::new(0, 0, 0, 0, None);
                                    popup_menu_rc.set_type(MenuButtonType::Popup1);
                                    if !action.items.is_empty() {
                                        popup_menu_rc.set_label("右键列出可选操作");
                                    }
                                    popup_menu_rc.set_color(Color::by_index(215));
                                    if !action.title.is_empty() {
                                        let new_hint = action.title.chars().fold("".to_string(), |mut s, c| {
                                            s.push(c);
                                            if s.ends_with(". ")
                                                || s.ends_with("。")
                                                || s.ends_with("?")
                                                || s.ends_with("？")
                                                || s.ends_with("!")
                                                || s.ends_with("！") {
                                                s.push('\n');
                                            }
                                            s
                                        });
                                        popup_menu_rc.add_choice(new_hint.as_str());
                                    } else {
                                        popup_menu_rc.add_choice("暂无描述");
                                    }
                                    popup_menu_rc.popup();
                                }
                            }
                        }

                    }
                    Event::Push => {
                        let (push_from_x, push_from_y) = app::event_coords();
                        // debug!("清除选区");
                        selected.store(false, Ordering::Relaxed);
                        clear_selected_pieces(selected_pieces.clone());
                        update_panel_fn.write().update_param(true);
                        // ctx.set_damage(true);
                        select_from_row = 0;

                        let (p_offset_x, p_offset_y) = (ctx.x(), ctx.y());
                        let scroll_y = Self::calc_scroll_height(buffer_rc.clone(), ctx.height());
                        push_from_point.x = push_from_x - p_offset_x;
                        push_from_point.y = push_from_y - p_offset_y + scroll_y;
                        // debug!("scroll_y: {scroll_y}, push_from: {:?}", push_from_point);
                        push_from_point.align(ctx.width(), ctx.height(), scroll_y);

                        // 尝试检测起始点击位置是否位于某个数据段内，可减少后续划选过程中的检测目标范围
                        let index_vec = (0..buffer_rc.read().len()).collect::<Vec<usize>>();
                        let rect = push_from_point.as_rect();
                        if let Some(tr) = locate_target_rd(&mut push_from_point, rect, ctx.w(), buffer_rc.read().as_slice(), index_vec) {
                            select_from_row = tr.row;
                            // debug!("选择行 {row}");
                        }

                        return true;
                    }
                    Event::Drag => {
                        let (current_x, current_y) = app::event_coords();
                        let (p_offset_x, p_offset_y) = (ctx.x(), ctx.y());
                        let scroll_y = Self::calc_scroll_height(buffer_rc.clone(), ctx.height());
                        let mut current_point = ClickPoint::new(current_x - p_offset_x, current_y - p_offset_y + scroll_y);
                        current_point.align(ctx.width(), ctx.height(), scroll_y);
                        update_selection_when_drag(
                            push_from_point,
                            select_from_row,
                            &mut current_point,
                            buffer_rc.read().as_slice(),
                            selected_pieces.clone(),
                            ctx
                        );
                        // selected.set(ret);
                        let need_redraw = !selected_pieces.read().is_empty();
                        selected.store(need_redraw, Ordering::Relaxed);
                        if need_redraw {
                            // debug!("{need_redraw}");
                            update_panel_fn.write().update_param(true);
                            // ctx.set_damage(true);
                        }
                        return true;
                    }
                    _ => {}
                }
                false
            }
        });

        Self {
            panel, data_buffer,
            current_buffer,
            background_color, buffer_max_lines: Arc::new(AtomicUsize::new(buffer_max_lines)), notifier, inner, reviewer,
            blink_flag, text_font, text_color,
            text_size, piece_spacing, enable_blink, basic_char, tab_width,
            cursor_piece, show_cursor, remote_flow_control, rewrite_board, max_rows, max_cols,
            update_panel_fn,
        }
    }

    fn update_window_size(
        text_font_rc: Arc<RwLock<Font>>,
        text_size_rc: Arc<AtomicI32>,
        basic_char_rc: Arc<RwLock<char>>,
        panel_width: i32,
        panel_height: i32,
        max_rows_rc: Arc<AtomicUsize>,
        max_cols_rc: Arc<AtomicUsize>,
        rewrite_board_rc: Arc<RwLock<Option<ReWriteBoard>>>,
    ) -> (i32, i32) {
        draw::set_font(*text_font_rc.read(), text_size_rc.load(Ordering::Relaxed));
        let (char_width, _) = draw::measure(&basic_char_rc.read().to_string(), false);
        let new_cols = ((panel_width - PADDING.left - PADDING.right) as f32 / char_width as f32).floor() as i32;
        let new_rows = ((panel_height - PADDING.top - PADDING.bottom) as f32 / (text_size_rc.load(Ordering::Relaxed) as f32 * LINE_HEIGHT_FACTOR).ceil()).floor() as i32;
        max_rows_rc.store(max(new_rows, 1) as usize, Ordering::Relaxed);
        max_cols_rc.store(max(new_cols, 1) as usize, Ordering::Relaxed);
        if let Some(board) = rewrite_board_rc.write().as_mut() {
            board.resize(max(new_rows, 2) as usize, max(new_cols, 2) as usize);
        }
        // debug!("窗口行列数: {} x {}", new_rows, new_cols);
        (new_rows, new_cols)
    }

    /// 计算当前数据缓存的高度超出目标面板的高度差。
    ///
    /// # Arguments
    ///
    /// * `buffer_rc`: 数据缓存。
    /// * `panel_height`: 目标面板。在当前场景中是主视图面板。
    ///
    /// returns: i32 返回高度差，如果数据高度小于面板高度则返回0。
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn calc_scroll_height(buffer_rc: Arc<RwLock<Vec<RichData>>>, panel_height: i32) -> i32 {
        if let Some(last_rd) = buffer_rc.read().iter().last() {
            let last_rd_bottom = last_rd.v_bounds.read().1;
            if last_rd_bottom + PADDING.bottom > panel_height {
                last_rd_bottom - panel_height + PADDING.bottom
            } else {
                0
            }
        } else {
            0
        }
    }

    /// 检查是否应该关闭回顾区，若满足关闭条件则关闭回顾区并记录待销毁的回顾区组件。
    fn should_hide_reviewer(
        reviewer_rc: Arc<RwLock<Option<RichReviewer>>>,
        flex: &mut Flex,
        panel_rc: &impl WidgetBase,
        should_resize: Arc<AtomicI32>
    ) {
        let mut should_remove = false;
        if let Some(reviewer) = &*reviewer_rc.read() {
            let dy = reviewer.scroller.yposition();
            if dy == reviewer.panel.height() - reviewer.scroller.height() {
                let h = flex.h();
                flex.remove(&reviewer.scroller);
                flex.fixed(panel_rc, h);
                flex.recalc();

                // 替换新的离线绘制板
                should_resize.store(h, Ordering::Relaxed);
                flex.set_damage(true);
                should_remove = true;
            }
        }

        if should_remove {
            if let Some(mut rv) = reviewer_rc.write().take() {
                // println!("父窗口删除回顾区");
                rv.hide();
                app::awake_callback({
                    move || {
                        app::delete_widget(rv.scroller.clone());
                    }
                });
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
        self._append(user_data);

        self.update_panel_fn.write().update_param(false);
    }

    /// 向缓冲区批量添加数据或操作。
    ///
    /// # Arguments
    ///
    /// * `batch`: 批次数据将被消费。
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn append_batch(&mut self, batch: &mut Vec<DocEditType>) {

        batch.reverse();
        while let Some(at) = batch.pop() {
            // debug!("append_batch: {:?}", at);
            match at {
                DocEditType::Data(user_data) => {
                    // debug!("添加数据: {:?}", user_data.text);
                    // let now = Instant::now();
                    self._append(user_data);
                    // debug!("添加数据耗时: {:?}", now.elapsed());
                }
                DocEditType::ToggleCursor(_param, show) => {
                    // debug!("{}光标: {}", if show {"显示"} else {"关闭"}, _param);
                    self.toggle_cursor(show);
                }
                DocEditType::EraseInLine(mode) => {
                    // debug!("行内删除: {:?}", mode);
                    self.erase_in_line(mode);
                }
                DocEditType::EraseInDisplay(mode) => {
                    // debug!("屏内删除: {:?}", mode);
                    self.erase_in_display(mode);
                }
                DocEditType::CursorAbsolute(n, m) => {
                    // debug!("移动光标: ({}, {})", n, m);
                    // let now = Instant::now();
                    self.move_cursor(n, m);
                    // debug!("移动光标耗时: {:?}", now.elapsed());
                }
                DocEditType::CursorUp(n) => {
                    debug!("上移光标: {}", n);
                    self.cursor_up(n);
                }
                DocEditType::CursorDown(n) => {
                    debug!("下移光标: {}", n);
                    self.cursor_down(n);
                }
                DocEditType::CursorBack(n) => {
                    debug!("左移光标: {}", n);
                    self.cursor_back(n);
                }
                DocEditType::CursorForward(n) => {
                    debug!("右移光标: {}", n);
                    self.cursor_forward(n);
                }
                DocEditType::Expire(target) => {
                    // debug!("使缓存目标过期：{}", target);
                    self.expire_main_data(target);
                }
                DocEditType::RemoteFlowControl(_code) => {
                    // IAC流控协议回调入口。目前忽略。
                    // 流控开始由光标移动到左上角触发，流控结束由特定的CSI串"\x1b[n;1H\x1b[K"触发。
                    // debug!("切换光标控制模式: {:?}", code);
                    // self.switch_mode(code);
                }
                DocEditType::CursorPosReport(cb) => {
                    // debug!("汇报光标位置");
                    if let Some(pos_str) = self.get_cursor_pos_dsr() {
                        cb.report.write()(pos_str);
                    }
                }
                DocEditType::PanelFlowEnd => {
                    debug!("面板流结束，切换到本地光标控制模式");
                    self.switch_mode(1);
                }
                DocEditType::CursorNextLine(_) => {}
                DocEditType::CursorPreviousLine(_) => {}
                DocEditType::CursorHorizontalAbsolute(_) => {}
            }
        }

        self.update_panel_fn.write().update_param(false);

        // debug!("append_batch: {:?}", now.elapsed());
    }

    /// 向缓冲区添加数据，并计算数据片段的绘制坐标。
    ///
    /// # Arguments
    ///
    /// * `user_data`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn _append(&mut self, user_data: UserData) {
        let default_font_text = !user_data.custom_font_text;
        let default_font_color = !user_data.custom_font_color;
        let mut rich_data: RichData = user_data.into();
        rich_data.piece_spacing = self.piece_spacing.load(Ordering::Relaxed);

        rich_data.text =  rich_data.text.replace('\t', &" ".repeat(self.tab_width.load(Ordering::Relaxed) as usize));

        if default_font_text {
            rich_data.font = *self.text_font.read();
            rich_data.font_size = self.text_size.load(Ordering::Relaxed);
        }
        if default_font_color {
            rich_data.fg_color = *self.text_color.read();
        }
        let window_width = self.panel.width();
        let drawable_max_width = window_width - PADDING.left - PADDING.right;

        if rich_data.bg_color.is_none() {
            rich_data.bg_color.replace(*self.background_color.read());
        }

        /*
        对文档结束符进行特殊处理：当作光标移动到行首的操作，不作为可见数据添加。
         */
        match rich_data.data_type {
            DataType::Text => {
                // debug!("接收到 rich_data.text: {:?}", rich_data.text);
                if self.rewrite_board.read().is_some() {
                    // 从当前缓存中清理上一批面板流数据
                    // self.clear_board_data();
                    {
                        self.current_buffer.write().clear();
                    }

                    if let Some(board) = self.rewrite_board.write().as_mut() {
                        // debug!("在面板流中添加数据：{:?}", rich_data.text);
                        let mut board_data = board.add_data(rich_data, self.cursor_piece.clone(), drawable_max_width, *self.basic_char.read());
                        // debug!("面板流有 {} 条数据", board_data.len());
                        self.current_buffer.write().append(&mut board_data);
                    }
                } else {
                    // debug!("在常规流中添加数据：{:?}", rich_data.text);
                    rich_data.text = rich_data.text.replace("\r", "");
                    let last_piece = rich_data.estimate(self.cursor_piece.clone(), drawable_max_width, *self.basic_char.read());
                    *self.cursor_piece.write() = last_piece.read().get_cursor();
                    self.current_buffer.write().push(rich_data);

                    if self.current_buffer.read().len() > self.buffer_max_lines.load(Ordering::Relaxed) {
                        self.current_buffer.write().reverse();
                        self.current_buffer.write().pop();
                        self.current_buffer.write().reverse();
                    }
                }

            }
            DataType::Image => {
                let last_piece = rich_data.estimate(self.cursor_piece.clone(), drawable_max_width, *self.basic_char.read());
                *self.cursor_piece.write() = last_piece.read().get_cursor();
                // self.throttle_holder.write().current_rid = rich_data.id;
                // self.add_data(rich_data);
                self.current_buffer.write().push(rich_data);
            }
        }
    }

    /// 删除最后一个数据段。
    pub fn delete_last_data(&mut self) {
        if let Some(_rich_data) = self.current_buffer.write().pop() {
            self.update_panel_fn.write().update_param(false);
        }
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
        if search_str.is_none() {
            if let Some(rr) = &mut *self.reviewer.write() {
                rr.clear_search_results();
            }
        } else if let Ok(open_suc) = self.auto_open_reviewer() {
            if let Some(ref mut rr) = *self.reviewer.write() {
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
        w: i32, h: i32, offscreen: Arc<RwLock<Offscreen>>,
        panel: &mut impl WidgetBase,
        visible_lines: Arc<RwLock<HashMap<Rectangle, LinePiece>>>,
        clickable_data: Arc<RwLock<HashMap<Rectangle, usize>>>,
        bg_color: Color,
        temp_buffer: Arc<RwLock<Vec<RichData>>>,
        blink_flag: Arc<RwLock<BlinkState>>,
        cursor: Option<Arc<RwLock<LinePiece>>>,
        ) {
        if let Some(offs) = Offscreen::new(w, h) {
            *offscreen.write() = offs;
            Self::draw_offline(offscreen.clone(), panel, visible_lines.clone(), clickable_data, bg_color, temp_buffer.clone(), blink_flag, cursor);
        }
    }

    fn draw_offline(
        offscreen: Arc<RwLock<Offscreen>>,
        panel: &mut impl WidgetBase,
        visible_lines: Arc<RwLock<HashMap<Rectangle, LinePiece>>>,
        clickable_data: Arc<RwLock<HashMap<Rectangle, usize>>>,
        bg_color: Color,
        current_buffer: Arc<RwLock<Vec<RichData>>>,
        blink_flag: Arc<RwLock<BlinkState>>,
        cursor: Option<Arc<RwLock<LinePiece>>>,) {
        // debug!("开始离线绘制");
        // let mut damage_area = (0, 0, 0, 0);
        offscreen.read().begin();

        let (panel_x, panel_y, window_width, window_height) = (panel.x(), panel.y(), panel.width(), panel.height());
        let mut offset_y = 0;

        let vl = &mut *visible_lines.write();
        let cd = &mut *clickable_data.write();
        vl.clear();
        cd.clear();

        // 填充背景
        draw_rect_fill(0, 0, window_width, window_height, bg_color);
        // damage_area = (0, 0, window_width, window_height);

        let mut need_blink = false;

        // 绘制数据内容
        let data = current_buffer.read();
        let mut set_offset_y = false;
        let mut drawable_vec: Vec<&RichData> = vec![];
        for (idx, rich_data) in data.iter().enumerate().rev() {
            let bottom_y = rich_data.v_bounds.read().1;
            if !set_offset_y && bottom_y > window_height {
                offset_y = bottom_y - window_height + PADDING.bottom;
                set_offset_y = true;
            }

            if bottom_y < offset_y {
                break;
            }

            // 暂存主体任意部分可见的数据行信息
            for piece in rich_data.line_pieces.iter() {
                let piece = &*piece.read();
                let y = piece.y - offset_y + panel_y;
                let rect = Rectangle::new(piece.x + panel_x, y, piece.w, piece.h);
                vl.insert(rect.clone(), piece.clone());

                // 暂存可操作数据信息
                if rich_data.clickable {
                    cd.insert(rect, idx);
                }
            }

            // rich_data.draw(offset_y, &*blink_flag.borrow());
            // 倒序暂存
            drawable_vec.push(rich_data);

            if !need_blink && rich_data.blink {
                need_blink = true;
            }
        }

        // 顺序绘制
        {
            // debug!("本次绘制数据段：{:?}", drawable_vec.len());
            let bf = &*blink_flag.read();
            while let Some(rd) = drawable_vec.pop() {
                // debug!("绘制数据段: {:?}", rd.text);
                rd.draw(offset_y, bf);
            }
        }

        // 填充顶部边界空白
        draw_rect_fill(0, 0, window_width, PADDING.top, bg_color);

        if let Some(cursor) = cursor {
            // 绘制光标
            blink_flag.write().on();
            let cursor_piece = &*cursor.read();
            // debug!("开始离线绘制光标: {:?}", cursor_piece);
            let cursor_width = max(cursor_piece.font_size / 2, 4);
            let y = cursor_piece.y - offset_y;
            let bs = &*blink_flag.read();
            let line_y = y + cursor_piece.font_height - ((cursor_piece.font_height as f32 / 10f32).floor() as i32 + 1);
            match bs.next {
                BlinkDegree::Normal => {
                    // draw_rect_fill(cursor_piece.x, cursor_piece.y, cursor_width, cursor_piece.font_size, Color::White);
                    set_draw_color(Color::White);
                    // debug!("绘制白色光标");
                    draw_line(cursor_piece.x, line_y, cursor_piece.x + cursor_width, line_y);
                }
                BlinkDegree::Contrast => {
                    set_draw_color(bg_color);
                    // debug!("绘制黑色光标");
                    draw_line(cursor_piece.x, line_y, cursor_piece.x + cursor_width, line_y);
                }
            }

            // damage_area = (cursor_piece.x, line_y - 1, cursor_width, 3);
        }

        offscreen.read().end();

        // 更新闪烁标记
        if need_blink {
            blink_flag.write().on();
        } else {
            blink_flag.write().off();
        }

        // debug!("待刷新区域: {:?}", damage_area);
        // panel.set_damage_area(Damage::All, damage_area.0, damage_area.1, damage_area.2, damage_area.3);
        panel.set_damage(true);
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
        *self.background_color.write() = background_color;
        if let Some(reviewer) = self.reviewer.read().as_ref() {
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
        self.buffer_max_lines.store(max_lines, Ordering::Relaxed);
        if self.current_buffer.read().len() > self.buffer_max_lines.load(Ordering::Relaxed) {
            let r = 0..(self.current_buffer.read().len() - self.buffer_max_lines.load(Ordering::Relaxed));
            self.current_buffer.write().drain(r);
            self.current_buffer.write().shrink_to_fit();
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
    pub fn set_notifier<F>(&mut self, cb: F) where F: FnMut(CallbackData) + Send + Sync +'static {
        let callback = Callback::new(Arc::new(RwLock::new(Box::new(cb))));
        self.notifier.write().replace(callback);
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
        if let Ok(idx) = self.current_buffer.read().binary_search_by_key(&options.id, |rd| rd.id) {
            target_idx = idx;
            find_out = true;
        }

        if find_out {
            if let Some(rd) = self.current_buffer.write().get_mut(target_idx) {
                update_data_properties(options.clone(), rd);
            }
            self.update_panel_fn.write().update_param(false);
        }

        if let Some(reviewer) = self.reviewer.write().as_mut() {
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
        if let Ok(idx) = self.current_buffer.read().binary_search_by_key(&id, |rd| rd.id) {
            target_idx = idx;
            find_out = true;
        }

        if find_out {
            if let Some(rd) = self.current_buffer.write().get_mut(target_idx) {
                disable_data(rd);
            }

            self.update_panel_fn.write().update_param(false);
        }

        if let Some(reviewer) = self.reviewer.write().as_mut() {
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
        if self.reviewer.read().is_some() {
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
        return if !self.current_buffer.read().is_empty() && self.reviewer.read().is_none() {
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
        *self.text_font.write() = font;

    }

    /// 获取默认的字体。
    pub fn text_font(&self) -> Font {
        *self.text_font.read()
    }

    pub fn text_font_ref(&self) -> Arc<RwLock<Font>> {
        self.text_font.clone()
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
        *self.text_color.write() = color;

    }

    /// 获取默认的字体颜色。
    pub fn text_color(&self) -> Color {
        *self.text_color.read()
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
        self.text_size.store(size, Ordering::Relaxed);
        if self.current_buffer.read().is_empty() {
            // 更新虚拟光标高度
            let cursor = &mut *self.cursor_piece.write();
            cursor.h = (size as f32 * LINE_HEIGHT_FACTOR).ceil() as i32;
            cursor.font_size = size;
            *cursor.rd_bounds.write() = (PADDING.top, PADDING.top + (size as f32 * LINE_HEIGHT_FACTOR).ceil() as i32, PADDING.left, PADDING.left);
        }
    }

    /// 获取默认的字体尺寸。
    pub fn text_size(&self) -> i32 {
        self.text_size.load(Ordering::Relaxed)
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
        self.piece_spacing.store(spacing, Ordering::Relaxed);
    }


    /// 设置启用或禁用闪烁支持。
    ///
    /// # Arguments
    ///
    /// * `enable`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn set_enable_blink(&mut self, enable: bool) {
        self.enable_blink.store(enable, Ordering::Relaxed);
        if let Some(reviewer) = self.reviewer.write().as_mut() {
            reviewer.set_enable_blink(enable);
        }
    }

    /// 启用或禁用闪烁，切换状态。
    pub fn toggle_blink(&mut self) {
        let toggle = !self.enable_blink.load(Ordering::Relaxed);
        self.enable_blink.store(toggle, Ordering::Relaxed);
        if let Some(reviewer) = self.reviewer.write().as_mut() {
            reviewer.set_enable_blink(toggle);
        }
    }

    pub fn set_search_focus_color(&mut self, color: Color) {
        self.blink_flag.write().focus_boarder_color = color;
        if let Some(reviewer) = &mut *self.reviewer.write() {
            reviewer.set_search_focus_color(color);
        }
    }

    pub fn set_search_focus_contrast(&mut self, contrast: Color) {
        self.blink_flag.write().focus_boarder_contrast_color = contrast;
        if let Some(reviewer) = &mut *self.reviewer.write() {
            reviewer.set_search_focus_contrast(contrast);
        }
    }

    pub fn set_search_focus_color_and_contrast(&mut self, color: Color, contrast: Color) {
        let mut bf = self.blink_flag.write();
        bf.focus_boarder_color = color;
        bf.focus_boarder_contrast_color = contrast;

        if let Some(reviewer) = &mut *self.reviewer.write() {
            reviewer.set_search_focus_color(color);
            reviewer.set_search_focus_contrast(contrast);
        }
    }

    pub fn set_search_focus_width(&mut self, width: u8) {
        self.blink_flag.write().focus_boarder_width = width as i32;
        if let Some(reviewer) = &mut *self.reviewer.write() {
            reviewer.set_search_focus_width(width);
        }
    }

    pub fn set_search_focus_background_color(&mut self, background: Color) {
        self.blink_flag.write().focus_background_color = background;
        if let Some(reviewer) = &mut *self.reviewer.write() {
            reviewer.set_search_focus_background(background);
        }
    }

    /// 计算当前主视图以默认字体大小可以完整显示的(列数，行数)。实际可见的行数可能大于计算返回的行数。
    /// 若应用对窗口尺寸敏感，则建议使用等宽字体作为默认字体。`fltk`中`Font::Screen`代表等宽字体。
    pub fn calc_default_window_size(&self) -> (i32, i32) {
        draw::set_font(*self.text_font.read(), self.text_size.load(Ordering::Relaxed));
        let (char_width, _) = draw::measure(&self.basic_char.read().to_string(), false);
        let new_cols = ((self.panel.w() - PADDING.left - PADDING.right) as f32 / char_width as f32).floor() as i32;
        let new_rows = ((self.panel.h() - PADDING.top - PADDING.bottom) as f32 / (self.text_size.load(Ordering::Relaxed) as f32 * LINE_HEIGHT_FACTOR).ceil()).floor() as i32;
        (new_cols, new_rows)
    }

    /// 设置用于衡量窗口尺寸的基本字符。对于非ASCII字符，可能计算出的尺寸要小于ASCII字符的，因为非ASCII字符可能需要占用更多的空间。
    /// 例如以非等宽字体作为默认字体时，将`'a'`当作基本衡量单位计算出来的窗口尺寸，就要大于以`'中'`为基本衡量单位计算的结果。
    /// 若应用对窗口尺寸敏感，则建议使用等宽字体作为默认字体。`fltk`中`Font::Screen`代表等宽字体。
    ///
    /// # Arguments
    ///
    /// * `basic_char`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn set_basic_char(&mut self, basic_char: char) {
        *self.basic_char.write() = basic_char;
        if let Some(reviewer) = &mut *self.reviewer.write() {
            reviewer.set_basic_char(basic_char);
        }
    }


    /// 设置'\t'所占的空格数。文本内容中的'\t'将被替换为`tab_width`个空格。
    ///
    /// # Arguments
    ///
    /// * `tab_width`: 一个`'\t'`所占的空格数。
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn set_tab_width(&mut self, tab_width: u8) {
        self.tab_width.store(tab_width, Ordering::Relaxed);
    }

    /// 显示或关闭光标。
    ///
    /// # Arguments
    ///
    /// * `show`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn toggle_cursor(&mut self, show: bool) {
        self.show_cursor.store(show, Ordering::Relaxed);
    }

    /// 获取当前坐标信息，以行、列的方式表示。
    pub fn get_cursor_pos_dsr(&self) -> Option<String> {
        if let Some(board) = self.rewrite_board.read().as_ref() {
            Some(board.cursor_pos.dsr())
        } else {
            None
        }
    }

    /// 切换光标控制模式。
    ///
    /// # Arguments
    ///
    /// * `code`: 0服务器控制，1本地控制。
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn switch_mode(&mut self, code: u8) {
        let local_mode = code == 1;
        self.remote_flow_control.store(local_mode, Ordering::SeqCst);
        if local_mode {
            // 切换到主缓存，将主缓存内的数据移动到当前缓存中。
            debug!("切换到主缓存");
            self.current_buffer.write().clear();
            if let Some(main_buffer) = self.data_buffer.write().as_mut() {
                self.current_buffer.write().append(main_buffer);
            }
            // debug!("面板流已结束1");
            // self.clear_board_data();
            self.rewrite_board.write().take();
        } else {
            // 切换到临时缓存，将当前缓存中的数据移动到主缓存中。
            debug!("切换到临时缓存");
            if let Some(main_buffer) = self.data_buffer.write().as_mut() {
                main_buffer.append(&mut *self.current_buffer.write());
            }
            self.show_cursor.store(true, Ordering::Relaxed);
        }
    }

    fn get_default_line_height(&self) -> i32 {
        let ref_font_height = (self.text_size.load(Ordering::Relaxed) as f32 * LINE_HEIGHT_FACTOR).ceil() as i32;
        let (_, th) = measure(" ", false);
        max(ref_font_height, th)
    }

    /// 移动光标到n行m列。
    ///
    /// # Arguments
    ///
    /// * `n`: 第n行，从1开始。
    /// * `m`: 第m列，从1开始。
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn move_cursor(&mut self, n: usize, m: usize) {
        let n = if n == 0 { 1 } else { n };
        let m = if m == 0 { 1 } else { m };
        let offset_y = self.get_offset_y();
        // debug!("移动光标到第{}行第{}列", n, m);

        let default_line_height = self.get_default_line_height();


        if n == 1 && m == 1 && self.rewrite_board.read().is_none() {
            debug!("创建新的定位面板，尺寸：{}/{}", self.max_rows.load(Ordering::Relaxed), self.max_cols.load(Ordering::Relaxed));
            self.switch_mode(0);
            self.rewrite_board.write().replace(ReWriteBoard::new(self.max_rows.load(Ordering::Relaxed), self.max_cols.load(Ordering::Relaxed), offset_y as usize, default_line_height as usize, 0));
        }

        let mut need_insert_empty = false;
        if let Some(board) = self.rewrite_board.write().as_mut() {
            board.cursor_pos.set(n, m);

            draw::set_font(*self.text_font.read(), self.text_size.load(Ordering::Relaxed));

            if let Some(rds) = board.line_data_map.get(&n) {
                let mut total_char_len = 0;
                // if n == 1 && m == 1 {
                //     debug!("移动光标到左上角");
                // }
                for rd in rds {
                    let char_len = rd.text.chars().count();
                    if total_char_len < m && total_char_len + char_len > m {
                        let char_pos = m - total_char_len - 1;
                        let sub_str = rd.text.chars().take(char_pos).collect::<String>();
                        let (char_width, _) = draw::measure(&sub_str, false);
                        if let Some(fp) = rd.line_pieces.first() {
                            let fpb = fp.read();
                            let new_x = fpb.x + char_width;
                            let new_y = fpb.top_y;
                            self.cursor_piece.write().move_cursor_to(new_x, new_y);
                        }
                        break;
                    }
                    total_char_len += char_len;
                }
            } else {
                let (char_width, _) = draw::measure(&self.basic_char.read().to_string(), false);

                let new_y = PADDING.top + (default_line_height * (n as i32 - 1)) + offset_y;
                let new_x = PADDING.left + char_width * (m as i32 - 1);
                self.cursor_piece.write().move_cursor_to(new_x, new_y);
            }

        } else {
            let (char_width, _) = draw::measure(&self.basic_char.read().to_string(), false);

            let new_y = PADDING.top + (default_line_height * (n as i32 - 1)) + offset_y;
            let new_x = PADDING.left + char_width * (m as i32 - 1);
            self.cursor_piece.write().move_cursor_to(new_x, new_y);
            need_insert_empty = true;
        }

        if need_insert_empty {
            self.append(UserData::new_text("".to_string()));
        }

        // debug!("虚拟光标位置: {:?}", self.cursor_piece.read().rect(0, 0));
    }

    // fn clear_board_data(&mut self) {
    //     let last_normal_data_pos = self.current_buffer.read().iter().rposition(|rd| !rd.rewrite_board_data);
    //     let current_len = self.current_buffer.read().len();
    //     if let Some(pos) = last_normal_data_pos {
    //         // 从当前缓存中移除面板数据
    //         if pos < current_len - 1 {
    //             // debug!("清空上一批定位面板流的数据");
    //             self.current_buffer.write().truncate(pos + 1);
    //         }
    //     }
    // }

    /// 光标上移n行。
    ///
    /// # Arguments
    ///
    /// * `n`: n大于等于1。
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn cursor_up(&mut self, mut n: usize) {
        if n == 0 { n = 1; }

        let cursor_piece = &mut *self.cursor_piece.write();
        cursor_piece.y -= cursor_piece.h * n as i32;
        if cursor_piece.y < PADDING.top {
            cursor_piece.y = PADDING.top;
        }
        cursor_piece.next_y = cursor_piece.y;
        let mut rd_bounds = *cursor_piece.rd_bounds.write();
        rd_bounds.0 = cursor_piece.y;
        rd_bounds.1 = cursor_piece.y + cursor_piece.h;
        // *cursor_piece.rd_bounds.write() = rd_bounds;
        if self.rewrite_board.read().is_none() {
            let default_line_height = self.get_default_line_height();
            self.rewrite_board.write().replace(ReWriteBoard::new(self.max_rows.load(Ordering::Relaxed), self.max_cols.load(Ordering::Relaxed), self.get_offset_y() as usize, default_line_height as usize, 0));
        }
        self.rewrite_board.write().as_mut().unwrap().cursor_pos.sub_n(n);
    }

    /// 光标下移n行。
    ///
    /// # Arguments
    ///
    /// * `n`: n大于等于1。
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn cursor_down(&mut self, mut n: usize) {
        if n == 0 { n = 1; }

        let cursor_piece = &mut *self.cursor_piece.write();
        cursor_piece.y += cursor_piece.h * n as i32;
        cursor_piece.next_y = cursor_piece.y;
        let mut rd_bounds = *cursor_piece.rd_bounds.write();
        rd_bounds.0 = cursor_piece.y;
        rd_bounds.1 = cursor_piece.y + cursor_piece.h;
        // cursor_piece.rd_bounds.set(rd_bounds);
        if self.rewrite_board.read().is_none() {
            let default_line_height = self.get_default_line_height();
            self.rewrite_board.write().replace(ReWriteBoard::new(self.max_rows.load(Ordering::Relaxed), self.max_cols.load(Ordering::Relaxed), self.get_offset_y() as usize, default_line_height as usize, 0));
        }
        self.rewrite_board.write().as_mut().unwrap().cursor_pos.add_n(n);
    }

    /// 光标左移m列。
    ///
    /// # Arguments
    ///
    /// * `m`: m大于等于1。
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn cursor_back(&mut self, mut m: usize) {
        if m == 0 { m = 1; }
        let cursor_piece = &mut *self.cursor_piece.write();

        draw::set_font(*self.text_font.read(), self.text_size.load(Ordering::Relaxed));
        let (char_width, _) = draw::measure(&self.basic_char.read().to_string(), false);

        cursor_piece.x -= char_width * m as i32;
        if cursor_piece.x < PADDING.left {
            cursor_piece.x = PADDING.left;
        }
        cursor_piece.next_x = cursor_piece.x;
        let mut rd_bounds = *cursor_piece.rd_bounds.write();
        rd_bounds.2 = cursor_piece.x;
        rd_bounds.3 = cursor_piece.x;
        // cursor_piece.rd_bounds.set(rd_bounds);
        if self.rewrite_board.read().is_none() {
            let default_line_height = self.get_default_line_height();
            self.rewrite_board.write().replace(ReWriteBoard::new(self.max_rows.load(Ordering::Relaxed), self.max_cols.load(Ordering::Relaxed), self.get_offset_y() as usize, default_line_height as usize, 0));
        }
        self.rewrite_board.write().as_mut().unwrap().cursor_pos.sub_m(m);
    }

    /// 光标右移m列。
    ///
    /// # Arguments
    ///
    /// * `m`: m大于等于1。
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn cursor_forward(&mut self, mut m: usize) {
        if m == 0 { m = 1; }

        let cursor_piece = &mut *self.cursor_piece.write();

        draw::set_font(*self.text_font.read(), self.text_size.load(Ordering::Relaxed));
        let (char_width, _) = draw::measure(&self.basic_char.read().to_string(), false);

        cursor_piece.x += char_width * m as i32;
        let max_width = self.panel.w() - PADDING.right;
        if cursor_piece.x > max_width {
            cursor_piece.x = max_width;
        }
        cursor_piece.next_x = cursor_piece.x;
        let mut rd_bounds = *cursor_piece.rd_bounds.write();
        rd_bounds.2 = cursor_piece.x;
        rd_bounds.3 = cursor_piece.x;
        // cursor_piece.rd_bounds.set(rd_bounds);
        if self.rewrite_board.read().is_none() {
            let default_line_height = self.get_default_line_height();
            self.rewrite_board.write().replace(ReWriteBoard::new(self.max_rows.load(Ordering::Relaxed), self.max_cols.load(Ordering::Relaxed), self.get_offset_y() as usize, default_line_height as usize, 0));
        }
        self.rewrite_board.write().as_mut().unwrap().cursor_pos.add_m(m);
    }

    /// 从当前光标处擦除行内数据，光标位置不变。。
    ///
    /// # Arguments
    ///
    /// * `mode`: 擦除模式：0从光标位置擦除到行尾，1从光标位置擦除到行首，2擦除整行。
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn erase_in_line(&mut self, erase_mode: u8) {
        // debug!("erase_in_line: {erase_mode}");
        if let Some(board) = self.rewrite_board.write().as_mut() {
            board.erase_in_line(erase_mode);
        }
        // else {
        //     let mut cursor_rect = (*self.cursor_piece.read()).rect(0, 0);
        //     // debug!("cursor_rect: {:?}", cursor_rect);
        //     match erase_mode {
        //         1 => {
        //             // 从光标位置擦除到行首。水平向左拉伸虚拟光标矩形边界。
        //             cursor_rect.stretch_to_left(cursor_rect.0 - PADDING.right);
        //             debug!("擦除到行首: {:?}", cursor_rect);
        //         }
        //         2 => {
        //             // 擦除整行。将虚拟光标矩形边界水平扩张到左右边界。
        //             cursor_rect.0 = PADDING.left;
        //             cursor_rect.2 = self.panel.w() - PADDING.left - PADDING.right;
        //             debug!("擦除整行: {:?}", cursor_rect);
        //         }
        //         _ => {
        //             // 从光标位置擦除到行尾。水平向右拉伸虚拟光标矩形边界。
        //             cursor_rect.2 = self.panel.w() - cursor_rect.0 - PADDING.right ;
        //             debug!("擦除到行尾: {:?}", cursor_rect);
        //         }
        //     }
        //     // debug!("cursor_rect: {:?}", cursor_rect);
        //     let temp_vec = &mut *self.current_buffer.write();
        //     if let Some(rd) = temp_vec.iter().find(|rd| {
        //         rd.text.contains("\r")
        //     }) {
        //         warn!("EraseInLine: 出现未处理的换行符！{:?}", rd.text);
        //     }
        //
        //     for (rd_idx, rd) in temp_vec.iter_mut().enumerate().rev()  {
        //         // debug!("rd vbounds: {:?}, text: {:?}", rd.v_bounds, rd.text);
        //         let rd_v_bounds = *rd.v_bounds.read();
        //         if rd_v_bounds.0 >= cursor_rect.1 + cursor_rect.3 {
        //             // 在光标下面的行
        //             continue;
        //         }
        //         if rd_v_bounds.1 <= cursor_rect.1 {
        //             // 在光标上面的行
        //             break;
        //         }
        //
        //         // debug!("rd与cursor可能有交会");
        //         let mut to_be_erased_lp: Option<usize> = None;
        //         for (lp_idx, lp) in rd.line_pieces.iter().enumerate() {
        //             let lp_rect = lp.read().rect(0, 0);
        //             // debug!("lp_rect: {:?}", lp_rect);
        //             if cursor_rect.intersects(&lp_rect) {
        //                 debug!("找到光标{:?}行上的数据片段：{:?}, {:?}", cursor_rect, lp_rect, lp.read().line);
        //                 to_be_erased_lp.replace(lp_idx);
        //                 break;
        //             }
        //         }
        //         if let Some(lp_idx) = to_be_erased_lp {
        //             let removed_piece = &rd.line_pieces.remove(lp_idx);
        //             let piece_str = &removed_piece.read().line;
        //             debug!("删除的数据片段：{:?}", *piece_str);
        //             let mut piece_from = 0;
        //             for i in 0..lp_idx {
        //                 if let Some(previous_lp) = rd.line_pieces.get(i) {
        //                     piece_from += previous_lp.read().line.len();
        //                 }
        //             }
        //             let piece_to = piece_from + piece_str.len();
        //             debug!("replace range: {:?} - {:?}", piece_from, piece_to);
        //             rd.text.replace_range(piece_from..piece_to, "");
        //             // debug!("删除后rd text: {:?}", rd.text);
        //             if rd.text.is_empty() {
        //                 // temp_vec.remove(1);
        //                 debug!("删除片段后rd({})为空", rd_idx);
        //             } else {
        //                 debug!("删除片段后rd({})不为空: {:?}", rd_idx, rd.text);
        //             }
        //
        //
        //             // todo(): 缩小面板后若出现自动换行处理，则会出现换行后的内容不能正确显示的问题。
        //         }
        //     }
        // }
    }

    /// 计算y轴偏移量。
    fn get_offset_y(&self) -> i32 {
        let (mut offset_y, window_height) = (0, self.panel.h());
        let bottom_y = if let Some(rd) = self.current_buffer.read().iter().last() {
            rd.v_bounds.read().1
        } else {
            0
        };
        if bottom_y > window_height {
            offset_y = bottom_y - window_height + PADDING.bottom;
        }
        offset_y
    }

    fn erase_in_display(&mut self, erase_mode: u8) {
        // debug!("erase in display: {erase_mode}");
        if let Some(board) = self.rewrite_board.write().as_mut() {
            board.erase_in_display(erase_mode);
        } else {
            // 计算y轴偏移量
            let offset_y = self.get_offset_y();
            let binding = self.cursor_piece.clone();
            let cursor_piece = &*binding.read();
            // 待擦除的矩形区域
            let mut expand_rect = cursor_piece.rect(0, 0);
            let mut current_line_rect: Option<Rectangle> = None;
            match erase_mode {
                1 => {
                    // 从光标位置擦除到面板左上角所有的行。
                    debug!("擦除到左上角");
                    let old_top = expand_rect.1 - offset_y;
                    expand_rect.stretch_to_left(PADDING.left - expand_rect.0);
                    current_line_rect.replace(expand_rect.clone());

                    expand_rect.0 = PADDING.left;
                    expand_rect.1 = PADDING.top - offset_y;
                    expand_rect.2 = self.panel.w() - PADDING.left - PADDING.right;
                    expand_rect.3 = self.panel.h() - PADDING.top - PADDING.bottom - old_top - 1;
                    // 待完善此场景
                }
                2 | 3 => {
                    // 擦除整个面板。
                    debug!("全部擦除");
                    expand_rect.0 = PADDING.left;
                    expand_rect.1 = PADDING.top - offset_y;
                    expand_rect.2 = self.panel.w() - PADDING.left - PADDING.right;
                    expand_rect.3 = self.panel.h() - PADDING.top - PADDING.bottom;
                }
                _ => {
                    // 从光标位置擦除到面板右下角所有的行。
                    debug!("擦除到右下角");
                    expand_rect.2 = self.panel.w() - PADDING.left - PADDING.right - expand_rect.0;
                    current_line_rect.replace(expand_rect.clone());

                    expand_rect.0 = PADDING.left;
                    expand_rect.1 = cursor_piece.y + cursor_piece.h + 1;
                    expand_rect.2 = self.panel.w() - PADDING.left - PADDING.right;
                    expand_rect.3 = self.panel.h() - (expand_rect.1 - offset_y) - PADDING.bottom;
                }
            }

            for (rd_idx, rd) in self.current_buffer.write().iter_mut().enumerate().rev()  {
                // debug!("rd vbounds: {:?}, text: {:?}", rd.v_bounds, rd.text);
                let rd_v_bounds = *rd.v_bounds.read();
                match erase_mode {
                    1 => {
                        if let Some(current_line_rect) = &current_line_rect {
                            // 检查目标是否在光标行以下
                            if rd_v_bounds.0 > current_line_rect.1 + current_line_rect.3 {
                                // debug!("跳过 rd v_bounds: {:?}, text: {:?}", rd_v_bounds, rd.text);
                                continue;
                            }
                        } else {
                            // 检查目标是否在扩展框以上
                            if rd_v_bounds.1 < expand_rect.1 {
                                // debug!("截至 rd v_bounds: {:?}, text: {:?}", rd_v_bounds, rd.text);
                                break;
                            }
                        }
                    }
                    2 | 3 => {
                        if rd_v_bounds.0 > expand_rect.1 + expand_rect.3 {
                            // debug!("跳过 rd v_bounds: {:?}, text: {:?}", rd_v_bounds, rd.text);
                            continue;
                        } else if rd_v_bounds.1 < expand_rect.1 {
                            // debug!("截至 rd v_bounds: {:?}, text: {:?}", rd_v_bounds, rd.text);
                            break;
                        }
                    }
                    _ => {
                        if let Some(current_line_rect) = &current_line_rect {
                            if rd_v_bounds.1 < current_line_rect.1 {
                                debug!("截至 rd v_bounds: {:?}, text: {:?}", rd_v_bounds, rd.text);
                                break;
                            }
                        } else {
                            if rd_v_bounds.0 > expand_rect.1 + expand_rect.3 {
                                // debug!("跳过 rd v_bounds: {:?}, text: {:?}", rd_v_bounds, rd.text);
                                continue;
                            }
                        }
                    }
                }

                // debug!("rd: {} 与cursor可能有交会: {}", rd.id, rd.text);
                let mut to_be_erased_lp: Vec<usize> = vec![];
                for (lp_idx, lp) in rd.line_pieces.iter().enumerate() {
                    let lp_rect = lp.read().rect(0, 0);
                    // debug!("lp_rect: {:?}", lp_rect);
                    if expand_rect.intersects(&lp_rect) {
                        // debug!("找到扩展框中的数据片段：{:?}", lp.borrow().line);
                        to_be_erased_lp.push(lp_idx);
                    }
                    if let Some(current_line_rect) = &current_line_rect {
                        if lp_rect.intersects(&current_line_rect) {
                            // debug!("找到光标行上的数据片段：{:?}", lp.borrow().line);
                            to_be_erased_lp.push(lp_idx);
                        }
                    }
                }
                if !to_be_erased_lp.is_empty() {
                    // 倒序并去重
                    to_be_erased_lp.sort_by(|a, b| (*b).cmp(a));
                    to_be_erased_lp.dedup();
                    // debug!("to_be_erased_lp {:?}", to_be_erased_lp);

                    let (mut erase_from, mut erase_len) = (0, 0);
                    for lp_idx in &to_be_erased_lp {
                        let removed_piece = &rd.line_pieces.remove(*lp_idx);
                        let piece_str = &removed_piece.read().line;
                        // debug!("删除的数据片段：{:?}", piece_str);
                        erase_len += piece_str.len();
                    }
                    if let Some(min) = to_be_erased_lp.last() {
                        for previous_lp in rd.line_pieces.iter().take(*min) {
                            erase_from += previous_lp.read().line.len();
                        }
                    }
                    rd.text.replace_range(erase_from..(erase_from + erase_len), "");
                    if rd.text.is_empty() {
                        // temp_vec.remove(1);
                        debug!("清屏时删除片段后rd({})为空", rd_idx);
                    } else {
                        debug!("清屏时删除片段后rd({})不为空: {:?}", rd_idx, rd.text);
                    }
                } else {
                    // debug!("没有需要擦除的数据片段: {}", rd.id);
                }
            }
        }
    }


    /// 使符合过滤条件的目标数据段过期、禁用。
    ///
    /// # Arguments
    ///
    /// * `target`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub(crate) fn expire_main_data(&mut self, target: String) {
        // expire_data(self.temp_buffer.clone(), &target);
        expire_data(self.current_buffer.clone(), &target);
        self.panel.set_damage(true);
        if let Some(reviewer) = &mut *self.reviewer.write() {
            reviewer.expire_review_data(&target);
        }
    }

    /// 获取远程流控制状态。
    pub fn get_remote_flow_control(&self) -> Arc<AtomicBool> {
        self.remote_flow_control.clone()
    }
}
