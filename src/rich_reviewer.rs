//! 展示缓存数据的组件，数据可来自主视图(主视图+回顾区配合使用)的快照，也可直接填充外部数据，可滚动浏览。
//! 当以历史模式(即脱离主视图单独使用)展示数据时，不应修改数据。
//!
//! 大量数据懒加载模式用法示例：
//! ```rust
//! use std::sync::Arc;
//! use std::sync::atomic::AtomicUsize;
//! use std::sync::atomic::Ordering::Relaxed;
//! use fast_log::filter::ModuleFilter;
//! use fltk::{app, window};
//! use fltk::button::Button;
//! use fltk::enums::{Color, Font};
//! use fltk::image::SharedImage;
//! use fltk::prelude::{GroupExt, ImageExt, WidgetBase, WidgetExt, WindowExt};
//! use log::{LevelFilter, warn};
//! use parking_lot::RwLock;
//! use fltkrs_richdisplay::rich_reviewer::RichReviewer;
//! use fltkrs_richdisplay::{PageOptions, UserData};
//!
//! pub enum GlobalMessage {
//!     Clear,
//!     AfterClear
//! }
//!
//! fn init_log() {
//!     let filter = ModuleFilter::new();
//!     // filter.modules.push("mobc".to_string());
//!     // filter.modules.push("reqwest".to_string());
//!
//!     fast_log::init(fast_log::Config::new()
//!         .console()
//!         .chan_len(Some(100000))
//!         .level(LevelFilter::Debug)
//!         .add_filter(filter)
//!     ).unwrap();
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     init_log();
//!
//!     let app = app::App::default();
//!     let mut win = window::Window::default()
//!         .with_size(1800, 1000)
//!         .with_label("rich-display fill data example")
//!         .center_screen();
//!     win.make_resizable(true);
//!
//!     let page_size = Arc::new(AtomicUsize::new(10));
//!     let mut btn1 = Button::new(120, 10, 100, 30, "page_size - 10");
//!     let mut btn2 = Button::new(240, 10, 100, 30, "page_size + 10");
//!
//!     let mut reviewer = RichReviewer::new(100, 60, 1600, 800, None).lazy_page_mode();
//!     // reviewer.set_background_color(Color::Dark1);
//!     reviewer.set_page_size(page_size.load(Relaxed));
//!     reviewer.set_piece_spacing(5);
//!
//!     // 设置默认字体和颜色
//!     reviewer.set_text_font(Font::Times);
//!     reviewer.set_text_color(Color::Light1);
//!     reviewer.set_text_size(12);
//!
//!     btn1.set_callback({
//!         let page_size_rc = page_size.clone();
//!         let mut reviewer_rc = reviewer.clone();
//!         move |_| {
//!             if page_size_rc.load(Relaxed) >= 10 {
//!                 let new_page_size = page_size_rc.load(Relaxed) - 10;
//!                 page_size_rc.store(new_page_size, Relaxed);
//!                 reviewer_rc.set_page_size(new_page_size);
//!             }
//!         }
//!     });
//!     btn2.set_callback({
//!         let page_size_rc = page_size.clone();
//!         let mut reviewer_rc = reviewer.clone();
//!         move |_| {
//!             if page_size_rc.load(Relaxed) <= 100 {
//!                 let new_page_size = page_size_rc.load(Relaxed) + 10;
//!                 page_size_rc.store(new_page_size, Relaxed);
//!                 reviewer_rc.set_page_size(new_page_size);
//!             }
//!         }
//!     });
//!
//!     win.end();
//!     win.show();
//!
//!
//!     let data_buffer = Arc::new(RwLock::new(Vec::<UserData>::new()));
//!
//!     let img1 = SharedImage::load("res/1.jpg").unwrap().to_rgb().unwrap();
//!     let (img1_width, img1_height) = (img1.width(), img1.height());
//!     let img2 = SharedImage::load("res/2.jpg").unwrap().to_rgb().unwrap();
//!     let (img2_width, img2_height) = (img2.width(), img2.height());
//!
//!     let mut reversed_buffer: Vec<UserData> = vec![];
//!     for i in 0..100 {
//!         let turn = i * 14;
//!         let mut data: Vec<UserData> = Vec::from([
//!             UserData::new_text(format!("{}安全并且高效地处理𝄞并发编程是Rust的另一个@主要目标。💖并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程🐉允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 0)).set_bg_color(Some(Color::DarkCyan)),
//!             UserData::new_text(format!("{}安全并且高效地处理𝄞并发编程是Rust的另一个主要目标。💖并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程🐉允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 1)).set_underline(true).set_font_and_size(Font::Helvetica, 38).set_bg_color(Some(Color::DarkYellow)).set_clickable(true),
//!             UserData::new_text(format!("{}在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程。类似地，程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。", turn + 2)).set_font_and_size(Font::HelveticaItalic, 18).set_bg_color(Some(Color::Green)),
//!             UserData::new_image(img1.copy(), img1_width, img1_height, img1_width, img1_height, Some("res/1.jpg".to_string())),
//!             UserData::new_text(format!("{}由于多线程可以同时运行，🐉所以将计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 3)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)).set_underline(true),
//!             UserData::new_text(format!("{}由于多线程可以同时运行，所以将计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 4)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
//!             UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。\r\n", turn + 5)).set_font_and_size(Font::Helvetica, 9).set_underline(true).set_blink(true),
//!             UserData::new_text(format!("{}在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程b。类似地，𝄞程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。\r\n", turn + 6)).set_font_and_size(Font::Helvetica, 32),
//!             UserData::new_text(format!("{}由于多线程可以同时运行，所以将计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 7)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
//!             UserData::new_text(format!("{}由于多线程可以同时运行，所以将计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 8)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
//!             UserData::new_image(img1.copy(), img1_width, img1_height, img1_width, img1_height, Some("res/1.jpg".to_string())).set_clickable(true),
//!             UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 9)).set_fg_color(Color::Yellow).set_bg_color(Some(Color::DarkBlue)),
//!             UserData::new_text(format!("{}在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程。类似地，程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。\r\n", turn + 10)).set_font_and_size(Font::HelveticaBold, 32).set_bg_color(Some(Color::Magenta)).set_clickable(true),
//!             UserData::new_text(format!("{}由于多线程可以同时运行，所以将计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 11)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
//!             UserData::new_text(format!("{}由于多线程可以同时运行，所以将计算操作拆分至多个线程可以提高性能。", turn + 12)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)).set_clickable(true),
//!             UserData::new_text(format!("{}由于多线程可以同时运行，💖所以将计算操作拆分至多个线程可以提高性能。", turn + 13)).set_fg_color(Color::Cyan).set_font_and_size(Font::Courier, 18).set_clickable(true).set_blink(true),
//!             UserData::new_image(img2.copy(), img2_width, img2_height, img2_width, img2_height, Some("res/2.jpg".to_string())).set_clickable(true).set_blink(true),
//!         ]);
//!         data.reverse();
//!         while let Some(data_unit) = data.pop() {
//!             reversed_buffer.push(data_unit);
//!         }
//!     }
//!     data_buffer.write().append(&mut reversed_buffer);
//!
//!     let fetch_page_fn = {
//!         let data_buffer_rc = data_buffer.clone();
//!         let mut reviewer_rc = reviewer.clone();
//!         let page_size_rc = page_size.clone();
//!         move |opt| {
//!             let ps = page_size_rc.load(Relaxed);
//!             match opt {
//!                 PageOptions::NextPage(last_uid) => {
//!                     if let Ok(last_pos) = data_buffer_rc.read().binary_search_by_key(&last_uid, |d| d.id) {
//!                         // debug!("找到当前页最后一条数据的索引位置: {}, {}", last_pos, auto_extend);
//!                         if data_buffer_rc.read().len() > last_pos + 1 {
//!                             let mut page_data = Vec::<UserData>::with_capacity(ps);
//!                             for ud in data_buffer_rc.read()[(last_pos + 1)..].iter().take(ps) {
//!                                 page_data.push(ud.clone());
//!                             }
//!                             // debug!("载入下一页数据");
//!                             reviewer_rc.load_page_now(page_data, opt);
//!                         }
//!                     } else {
//!                         warn!("未找到目标数据: {}", last_uid);
//!                     }
//!                 }
//!                 PageOptions::PrevPage(first_uid) => {
//!                     if let Ok(first_pos) = data_buffer_rc.read().binary_search_by_key(&first_uid, |d| d.id) {
//!                         // debug!("找到当前页第一条数据的索引位置: {}", first_pos);
//!                         if first_pos > 0 {
//!                             let mut page_data = Vec::<UserData>::with_capacity(ps);
//!                             let from = if first_pos >= ps {
//!                                 first_pos - ps
//!                             } else {
//!                                 0
//!                             };
//!                             let to = from + ps;
//!                             for ud in data_buffer_rc.read()[from..to].iter().take(ps) {
//!                                 page_data.push(ud.clone());
//!                             }
//!                             // debug!("载入上一页数据");
//!                             reviewer_rc.load_page_now(page_data, opt);
//!                         }
//!                     } else {
//!                         warn!("未找到目标数据: {}", first_uid);
//!                     }
//!                 }
//!             }
//!         }
//!     };
//!     reviewer.set_page_notifier(fetch_page_fn);
//!
//!     let mut page_data = Vec::<UserData>::with_capacity(page_size.load(Relaxed));
//!     for ud in data_buffer.read().iter().take(page_size.load(Relaxed)) {
//!         page_data.push(ud.clone());
//!     }
//!     reviewer.load_page_now(page_data, PageOptions::NextPage(0));
//!
//!     app.run().unwrap();
//!
//!     if let Ok(w) = fast_log::flush() {
//!         // 等待日志刷出到磁盘上。
//!         w.wait();
//!     }
//! }
//! ```

use std::cell::{Cell};
use std::cmp::{max, min, Ordering};
use std::collections::{HashMap};
use std::rc::Rc;
use std::sync::{Arc, OnceLock, Weak};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize};
use std::sync::atomic::Ordering::Relaxed;
use std::time::{Duration};
use debounce_fltk::throttle_check;
use fltk::draw::{draw_rect_fill, draw_xyline, LineStyle, Offscreen, set_draw_color, set_line_style};
use fltk::enums::{Align, Color, Cursor, Event, Font};
use fltk::group::{Scroll, ScrollType};
use fltk::prelude::{GroupExt, MenuExt, WidgetBase, WidgetExt};
use fltk::{app, draw, widget_extends};
use fltk::app::{awake_callback, MouseButton, MouseWheel};
use fltk::menu::{MenuButton, MenuButtonType};
use fltk::widget::Widget;
use idgenerator_thin::YitIdHelper;
use log::{error};
use parking_lot::RwLock;
use crate::{Rectangle, disable_data, LinedData, LinePiece, LocalEvent, mouse_enter, PADDING, RichData, RichDataOptions, update_data_properties, UserData, ClickPoint, clear_selected_pieces, BlinkState, BLINK_INTERVAL, Callback, CallPage, PageOptions, DEFAULT_FONT_SIZE, WHITE, locate_target_rd, update_selection_when_drag, CallbackData, BASIC_UNIT_CHAR, DataType, ImageEventData, IMAGE_PADDING_V, expire_data, select_paragraph};
use crate::rich_text::{PANEL_PADDING};

static LOAD_PAGE_TASK_ID: OnceLock<i64> = OnceLock::new();

#[derive(Clone, Debug)]
pub struct RichReviewer {
    pub(crate) scroller: Scroll,
    pub(crate) panel: Widget,
    pub(crate) data_buffer: Arc<RwLock<Vec<RichData>>>,
    background_color: Arc<RwLock<Color>>,
    visible_lines: Arc<RwLock<HashMap<Rectangle, LinePiece>>>,
    clickable_data: Arc<RwLock<HashMap<Rectangle, usize>>>,
    reviewer_screen: Arc<RwLock<Offscreen>>,
    notifier: Arc<RwLock<Option<Callback>>>,
    page_notifier: Arc<RwLock<Option<CallPage>>>,
    search_string: Arc<RwLock<Option<String>>>,
    /// 查找结果，保存查询到的目标数据段在data_buffer中的索引编号。
    search_results: Arc<RwLock<Vec<usize>>>,
    current_highlight_focus: Arc<RwLock<Option<(usize, usize)>>>,
    blink_flag: Arc<RwLock<BlinkState>>,
    /// true表示历史记录模式，默认false表示在线回顾模式。
    history_mode: Arc<AtomicBool>,
    /// 历史模式下，分页数据大小。
    page_size: Arc<AtomicUsize>,
    text_font: Arc<RwLock<Font>>,
    text_color: Arc<RwLock<Color>>,
    text_size: Arc<AtomicI32>,
    piece_spacing: Arc<AtomicI32>,
    enable_blink: Arc<AtomicBool>,
    basic_char: Arc<RwLock<char>>,
}
widget_extends!(RichReviewer, Scroll, scroller);

impl RichReviewer {
    pub const SCROLL_BAR_WIDTH: i32 = 10;
    // pub const PANEL_MAX_HEIGHT: i32 = 10;

    pub fn new<T>(x: i32, y: i32, w: i32, h: i32, title: T) -> Self
        where T: Into<Option<&'static str>> + Clone {

        let mut scroller = Scroll::new(x, y, w, h, title);
        scroller.set_type(ScrollType::Vertical);
        scroller.set_scrollbar_size(Self::SCROLL_BAR_WIDTH);
        scroller.set_align(Align::Bottom);
        scroller.end();

        let text_font = Arc::new(RwLock::new(Font::Helvetica));
        let text_color = Arc::new(RwLock::new(WHITE));
        let text_size = Arc::new(AtomicI32::new(DEFAULT_FONT_SIZE));

        let mut panel = Widget::new(x, y, w, h, None);
        scroller.add_resizable(&panel);

        let data_buffer: Arc<RwLock<Vec<RichData>>> = Arc::new(RwLock::new(vec![]));
        let background_color = Arc::new(RwLock::new(Color::Black));
        let visible_lines = Arc::new(RwLock::new(HashMap::<Rectangle, LinePiece>::new()));
        let clickable_data = Arc::new(RwLock::new(HashMap::<Rectangle, usize>::new()));
        let notifier: Arc<RwLock<Option<Callback>>> = Arc::new(RwLock::new(None));
        let page_notifier: Arc<RwLock<Option<CallPage>>> = Arc::new(RwLock::new(None));
        let reviewer_screen = Arc::new(RwLock::new(Offscreen::new(w, h).unwrap()));
        let scroll_panel_to_y_after_resize = Arc::new(AtomicI32::new(0));
        let resize_panel_after_resize = Arc::new(RwLock::new((0, 0, 0, 0)));
        let history_mode = Arc::new(AtomicBool::new(false));
        let page_size = Arc::new(AtomicUsize::new(10));
        let piece_spacing = Arc::new(AtomicI32::new(0));
        let enable_blink = Arc::new(AtomicBool::new(false));

        let search_results = Arc::new(RwLock::new(Vec::<usize>::new()));
        let search_str = Arc::new(RwLock::new(None::<String>));
        let current_highlight_focus = Arc::new(RwLock::new(None::<(usize, usize)>));
        let basic_char = Arc::new(RwLock::new(BASIC_UNIT_CHAR));

        let blink_flag = Arc::new(RwLock::new(BlinkState::new()));
        let blink_handler = {
            let blink_flag_rc = blink_flag.clone();
            let enable_blink_rc = enable_blink.clone();

            #[cfg(target_os = "linux")]
            let scroller_rc = scroller.clone();

            #[cfg(not(target_os = "linux"))]
            let mut scroller_rc = scroller.clone();

            move |handler| {
                if !scroller_rc.was_deleted() {
                    if enable_blink_rc.load(Relaxed) {
                        let should_toggle = blink_flag_rc.write().toggle_when_on();
                        if should_toggle {
                            // blink_flag_rc.set(bs);
                            // debug!("from reviewer blink flag: {:?}", blink_flag_rc.get());

                            #[cfg(target_os = "linux")]
                            if let Some(mut parent) = scroller_rc.parent() {
                                parent.set_damage(true);
                            }

                            #[cfg(not(target_os = "linux"))]
                            scroller_rc.set_damage(true);
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
            let data_buffer_rc = data_buffer.clone();
            let scroll_rc = scroller.clone();
            let visible_lines_rc = visible_lines.clone();
            let clickable_data_rc = clickable_data.clone();
            let bg_rc = background_color.clone();
            let screen_rc = reviewer_screen.clone();
            let blink_flag_rc = blink_flag.clone();
            let history_mode_rc = history_mode.clone();
            move |_| {
                /*
                先离线绘制内容面板，再根据面板大小复制所需区域内容。这样做是为了避免在线绘制时，会出现绘制内容超出面板边界的问题。
                 */
                Self::draw_offline(screen_rc.clone(), &scroll_rc, visible_lines_rc.clone(), clickable_data_rc.clone(), data_buffer_rc.clone(), *bg_rc.read(), blink_flag_rc.clone(), history_mode_rc.load(Relaxed));

                screen_rc.read().copy(scroll_rc.x(), scroll_rc.y(), scroll_rc.width(), scroll_rc.height(), 0, 0);
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
                    let (x, y, w, h) = &*resize_panel_after_resize_rc.read();
                    // 强制滚动到最顶部，避免scroll.yposition()缓存，在窗口不需要滚动条时仍出现滚动条的问题。
                    // debug!("resize panel to ({}, {}, {}, {})", x, y, w, h);
                    scroller_rc.scroll_to(0, 0);
                    ctx.resize(*x, *y, *w, *h);
                    true
                } else if evt == LocalEvent::SCROLL_TO.into() {
                    scroller_rc.scroll_to(0, new_scroll_y_rc.load(Relaxed));
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
            let page_notifier_rc = page_notifier.clone();
            let screen_rc = reviewer_screen.clone();
            let panel_rc = panel.clone();
            let new_scroll_y_rc = scroll_panel_to_y_after_resize.clone();
            let resize_panel_after_resize_rc = resize_panel_after_resize.clone();
            let clickable_data_rc = clickable_data.clone();
            let mut push_from_point = ClickPoint::new(0, 0);
            let mut select_from_row = 0;
            let selected_pieces = Arc::new(RwLock::new(Vec::<Weak<RwLock<LinePiece>>>::new()));
            let basic_char_rc = basic_char.clone();
            let text_size_rc = text_size.clone();
            move |scroller, evt| {
                match evt {
                    // Event::Close => {
                    //     debug!("Closing");
                    // }
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
                                let mut last_piece = LinePiece::init_piece(text_size_rc.load(Relaxed));
                                for rich_data in buffer_rc.write().iter_mut() {
                                    rich_data.line_pieces.clear();
                                    last_piece = rich_data.estimate(last_piece, drawable_max_width, *basic_char_rc.read());
                                }

                                new_panel_height = Self::calc_panel_height(buffer_rc.clone(), current_height);

                                // 同步缩放回顾内容面板
                                *resize_panel_after_resize_rc.write() = (scroller.x(), scroller.y(), current_width, new_panel_height);
                                if let Err(e) = app::handle_main(LocalEvent::RESIZE) {
                                    error!("发送缩放信号失败:{e}");
                                }
                            }

                            // 按照新的窗口大小重新生成绘图板
                            if let Some(offs) = Offscreen::new(current_width, current_height) {
                                *screen_rc.write() = offs;
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
                                new_scroll_y_rc.store(new_scroll_y, Relaxed);
                                if let Err(e) = app::handle_main(LocalEvent::SCROLL_TO) {
                                    error!("发送滚动信号失败:{e}");
                                }
                            }
                        }
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
                                    target_rd_v_bounds.replace(*rd.v_bounds.read());
                                    let sd: UserData = rd.into();
                                    target_opt.replace(sd);
                                    // if let Some(cb) = &mut *notifier_rc.borrow_mut() {
                                    //     cb.notify(CallbackData::Data(sd));
                                    // }
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
                                                let mut offset_y = scroller.yposition() - PANEL_PADDING;
                                                // 处理数据相对位移
                                                if let Some(first) = buffer_rc.read().first() {
                                                    offset_y += first.v_bounds.read().0;
                                                }
                                                if offset_y < 0 {offset_y = 0;}
                                                let click_at_x = app_x - scroller.x() - v_bounds.2;
                                                let click_at_y = app_y - scroller.y() + offset_y - v_bounds.0 + IMAGE_PADDING_V;
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
                                scroller.set_damage(true);
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
                        clear_selected_pieces(selected_pieces.clone());
                        scroller.set_damage(true);
                        select_from_row = 0;

                        let (p_offset_x, p_offset_y) = (scroller.x(), scroller.y());
                        let mut offset_y = scroller.yposition() - PANEL_PADDING;
                        // 处理数据相对位移
                        if let Some(first) = buffer_rc.read().first() {
                            offset_y += first.v_bounds.read().0;
                        }
                        push_from_point.x = push_from_x - p_offset_x;
                        push_from_point.y = push_from_y + offset_y - p_offset_y;
                        push_from_point.align(scroller.width(), scroller.height(), offset_y);

                        // 尝试检测起始点击位置是否位于某个数据段内，可减少后续划选过程中的检测目标范围
                        let index_vec = (0..buffer_rc.read().len()).collect::<Vec<usize>>();
                        let rect = push_from_point.as_rect();
                        if let Some(tr) = locate_target_rd(&mut push_from_point, rect, scroller.w(), buffer_rc.read().as_slice(), index_vec) {
                            select_from_row = tr.row;
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
                        if let Some(first) = buffer_rc.read().first() {
                            offset_y += first.v_bounds.read().0;
                        }
                        if offset_y < 0 {offset_y = 0;}
                        let data_buffer_ref = buffer_rc.read();
                        let data_buffer_slice = data_buffer_ref.as_slice();
                        let mut current_point = ClickPoint::new(
                            current_x - p_offset_x,
                            current_y + offset_y - p_offset_y
                        );
                        current_point.align(scroller.w(), scroller.h(), offset_y);
                        update_selection_when_drag(
                            push_from_point,
                            select_from_row,
                            &mut current_point,
                            data_buffer_slice,
                            selected_pieces.clone(),
                            &mut scroller.as_base_widget(),
                        );

                        // selected = !selected_pieces.read().is_empty();
                        // debug!("拖选结果：{selected}");
                        #[cfg(target_os = "linux")]
                        if let Some(mut parent) = scroller.parent() {
                            parent.set_damage(true);
                        }
                        return true;
                    }
                    Event::MouseWheel => {
                        if app::event_inside_widget(scroller) {
                            let mut id = 0i64;
                            if app::event_dy() == MouseWheel::Down {
                                // 向上滚动
                                if scroller.yposition() < (scroller.h() / 4) {
                                    // debug!("请求前一页");
                                    // 获取id与执行回调之间分开处理，避免buffer_rc的嵌套借用出现问题
                                    if let Some(rd) = buffer_rc.read().first() {
                                        id = rd.id;
                                    }

                                    if id != 0 {
                                        if let Some(cb) = &mut *page_notifier_rc.write() {
                                            // cb.notify(PageOptions::PrevPage(id));
                                            Self::load_page(cb, PageOptions::PrevPage(id));
                                        };
                                    };
                                }
                            } else if app::event_dy() == MouseWheel::Up {
                                // 向下滚动
                                if !Self::should_hide(scroller, &panel_rc) {
                                    if scroller.yposition() > panel_rc.height() - scroller.h() - (scroller.h() / 4) {
                                        // debug!("请求后一页");
                                        // 获取id与执行回调之间分开处理，避免buffer_rc的嵌套借用出现问题
                                        if let Some(rd) = buffer_rc.read().last() {
                                            id = rd.id;
                                        }

                                        if id != 0 {
                                            if let Some(cb) = &mut *page_notifier_rc.write() {
                                                // cb.notify(PageOptions::NextPage(id, false));
                                                Self::load_page(cb, PageOptions::NextPage(id));
                                            };
                                        }
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

        Self {
            scroller, panel, data_buffer, background_color, visible_lines, clickable_data,
            reviewer_screen, notifier, page_notifier, search_string: search_str, search_results,
            current_highlight_focus, blink_flag, history_mode, page_size, text_font, text_color,
            text_size, piece_spacing, enable_blink, basic_char }
    }

    fn should_hide(scroller: &Scroll, panel: &Widget) -> bool {
        scroller.yposition() == panel.height() - scroller.height()
    }

    pub fn set_background_color(&self, color: Color) {
        *self.background_color.write() = color;
    }

    /// 设置回顾区数据。
    ///
    /// # Arguments
    ///
    /// * `data`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub(crate) fn set_data(&mut self, mut data: Vec<RichData>) {
        // 更新回看数据
        self.data_buffer.write().clear();
        self.data_buffer.write().append(&mut data);

        let (scroller_width, scroller_height) = (self.panel.width(), self.scroller.height());

        // 设置新的窗口尺寸
        let panel_height = Self::calc_panel_height(self.data_buffer.clone(), scroller_height);
        self.panel.resize(self.panel.x(), self.panel.y(), scroller_width, panel_height);
    }


    pub fn scroll_to_bottom(&mut self) {
        self.scroller.scroll_to(0, self.panel.height() - self.scroller.height());
    }


    fn draw_offline(
        screen: Arc<RwLock<Offscreen>>,
        scroller: &Scroll,
        visible_lines: Arc<RwLock<HashMap<Rectangle, LinePiece>>>,
        clickable_data: Arc<RwLock<HashMap<Rectangle, usize>>>,
        data_buffer: Arc<RwLock<Vec<RichData>>>,
        background_color: Color,
        blink_flag: Arc<RwLock<BlinkState>>,
        history_mode: bool
        ) {

        screen.read().begin();
        let (scroller_x, scroller_y, window_width, window_height) = (scroller.x(), scroller.y(), scroller.width(), scroller.height());
        let drawable_height = window_height - PANEL_PADDING;

        let mut vl = visible_lines.write();
        let mut cd = clickable_data.write();
        vl.clear();
        cd.clear();

        // 滚动条滚动的高度在0到(panel.height - scroll.height)之间。
        let mut base_y = scroller.yposition();
        if base_y < 0 {
            base_y = 0;
        }

        let (mut top_y, mut bottom_y) = (base_y, base_y + drawable_height);

        // 处理数据相对位移
        if let Some(first) = data_buffer.read().first() {
            let y = first.v_bounds.read().0;
            top_y += y;
            bottom_y += y;
        }

        let offset_y = top_y - PADDING.top;

        // 填充背景色
        draw_rect_fill(0, 0, window_width, window_height, background_color);

        let data = &*data_buffer.read();

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
        // debug!("离线绘制， from_index:{from_index}, to_index:{to_index}");
        let mut need_blink = false;
        for (idx, rich_data) in data[from_index..to_index].iter().enumerate() {
            // debug!("回顾区离线绘制， idx:{idx}, type: {:?}, rich_data:{:?}", rich_data.data_type, rich_data.text);
            rich_data.draw(offset_y, &*blink_flag.read());

            if !need_blink && (rich_data.blink || rich_data.search_highlight_pos.is_some()) {
                // debug!("需要闪烁");
                need_blink = true;
            }

            for piece in rich_data.line_pieces.iter() {
                let piece = &*piece.read();
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
        if !history_mode {
            draw_rect_fill(0, drawable_height, window_width, PANEL_PADDING, background_color);
            set_draw_color(Color::White);
            set_line_style(LineStyle::DashDotDot, (PANEL_PADDING as f32 / 3f32).floor() as i32);
            draw_xyline(0, drawable_height + (PANEL_PADDING / 2), scroller_x + window_width);
            set_line_style(LineStyle::Solid, 1);
        } else {
            draw_rect_fill(0, scroller.h() - PADDING.bottom, window_width, PADDING.bottom, background_color);
        }

        // 填充顶部边界空白
        draw_rect_fill(0, 0, window_width, PADDING.top, background_color);

        screen.read().end();

        // 更新闪烁标记
        if need_blink {
            blink_flag.write().on();
        } else {
            blink_flag.write().off();
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
    pub fn set_notifier(&mut self, notifier: Callback) {
        self.notifier.write().replace(notifier);
    }

    /// 设置分页请求回调函数。
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
    pub fn set_page_notifier<F>(&mut self, cb: F) where F: FnMut(PageOptions) + Sync + Send + 'static {
        let call_page = CallPage::new(Arc::new(RwLock::new(Box::new(cb))));
        self.page_notifier.write().replace(call_page);
    }

    fn draw_offline2(&self) {
        Self::draw_offline(
            self.reviewer_screen.clone(),
            &self.scroller,
            self.visible_lines.clone(),
            self.clickable_data.clone(),
            self.data_buffer.clone(),
            *self.background_color.read(),
            self.blink_flag.clone(),
            self.history_mode.load(Relaxed)
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
        if self.history_mode.load(Relaxed) {
            return;
        }

        let mut find_out = false;
        let mut target_idx = 0;
        if let Ok(idx) = self.data_buffer.read().binary_search_by_key(&options.id, |rd| rd.id) {
            target_idx = idx;
            find_out = true;
        }

        if find_out {
            if let Some(rd) = self.data_buffer.write().get_mut(target_idx) {
                update_data_properties(options, rd);
            }
            self.draw_offline2();
        }
    }

    pub fn disable_data(&mut self, id: i64) {
        if self.history_mode.load(Relaxed) {
            return;
        }

        let mut find_out = false;
        let mut target_idx = 0;
        if let Ok(idx) = self.data_buffer.read().binary_search_by_key(&id, |rd| rd.id) {
            target_idx = idx;
            find_out = true;
        }

        if find_out {
            if let Some(rd) = self.data_buffer.write().get_mut(target_idx) {
                disable_data(rd);
            }

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
        let old_str_opt = self.search_string.read().as_ref().map(|s| s.clone());
        let find_out = if let Some(old) = old_str_opt {
            if old.eq(&search_str) {
                // 查询字符串未发生变化，则尝试定位到下一个目标
                !self.search_results.read().is_empty()
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
        let oo = *self.current_highlight_focus.read();
        if let Some((old_rd_idx, old_result_idx)) = oo {
            // debug!("上一次定位的数据段索引：{}，目标编号：{}", old_rd_idx, old_result_idx);
            let (mut scroll_to_next, mut next_rd_pos) = (false, 0);
            if let Some(rd) = self.data_buffer.write().get_mut(old_rd_idx) {
                if let Some(ref result_pos_vec) = rd.search_result_positions {
                    let next_result_idx = old_result_idx + 1;
                    if result_pos_vec.get(next_result_idx).is_some() {
                        // 在当前数据段中定位到下一个目标位置
                        // debug!("在当前数据段中定位到下一个目标位置");
                        self.current_highlight_focus.write().replace((old_rd_idx, next_result_idx));
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
                        if let Some(next_rd_idx) = self.search_results.read().get(next_idx) {
                            // debug!("下一个数据段索引：{}，目标序号：{}", next_rd_idx, next_idx);
                            next_rd_pos = *next_rd_idx;
                        } else {
                            if let Some(next_rd_idx) = self.search_results.read().first() {
                                next_rd_pos = *next_rd_idx;
                            }
                            // debug!("回归到循环开始位置，下一个数据段索引：{}， 目标序号：0", next_rd_pos);
                        }

                        rd.search_highlight_pos = None;
                    }
                }
            }

            if scroll_to_next {
                self.current_highlight_focus.write().replace((next_rd_pos, 0));
                if let Some(rd) = self.data_buffer.write().get_mut(next_rd_pos) {
                    rd.search_highlight_pos = Some(0);
                }
            }
        } else {
            if let Some(rd_idx) = self.search_results.read().first() {
                if let Some(rd) = self.data_buffer.write().get_mut(*rd_idx) {
                    if rd.search_result_positions.is_some() {
                        // debug!("首次定位到第一个目标");
                        self.current_highlight_focus.write().replace((*rd_idx, 0));
                        rd.search_highlight_pos = Some(0);
                    }
                }
            }
        }
    }

    /// 顺序(从上向下，从左到右)查找高亮下一个目标。
    fn highlight_next(&mut self) {
        let oo = *self.current_highlight_focus.read();
        if let Some((old_rd_idx, old_result_idx)) = oo {
            // debug!("上一次定位的数据段索引：{}，目标编号：{}", old_rd_idx, old_result_idx);
            let (mut scroll_to_next, mut next_rd_pos) = (false, 0);
            if let Some(rd) = self.data_buffer.write().get_mut(old_rd_idx) {
                if old_result_idx >= 1 {
                    // 在当前数据段中定位到下一个目标位置
                    self.current_highlight_focus.write().replace((old_rd_idx, old_result_idx - 1));
                    rd.search_highlight_pos.replace(old_result_idx - 1);
                } else {
                    // 在当前数据段中已经没有更多目标，则跳到下一个数据段；如果没有更多数据段则跳到第一个数据段。
                    let next_idx  = if let Ok(old_idx) = self.binary_search_with_desc_order(old_rd_idx) {
                        if old_idx >= 1 {
                            old_idx - 1
                        } else {
                            self.search_results.read().len() - 1
                        }
                    } else {
                        self.search_results.read().len() - 1
                    };
                    scroll_to_next = true;
                    if let Some(next_rd_idx) = self.search_results.read().get(next_idx) {
                        // debug!("下一个数据段索引：{}，目标序号：{}", next_rd_idx, next_idx);
                        next_rd_pos = *next_rd_idx;
                    } else if let Some(next_rd_idx) = self.search_results.read().last() {
                        next_rd_pos = *next_rd_idx;
                        // debug!("回归到循环开始位置，下一个数据段索引：{}， 目标序号：0", next_rd_pos);
                    }

                    rd.search_highlight_pos = None;
                }
            }

            if scroll_to_next {
                if let Some(rd) = self.data_buffer.write().get_mut(next_rd_pos) {
                    if let Some(ref pos_vec) = rd.search_result_positions {
                        self.current_highlight_focus.write().replace((next_rd_pos, pos_vec.len() - 1));
                        rd.search_highlight_pos = Some(pos_vec.len() - 1);
                    }
                }
            }

        } else {
            if let Some(rd_idx) = self.search_results.read().last() {
                if let Some(rd) = self.data_buffer.write().get_mut(*rd_idx) {
                    if let Some(ref srp) = rd.search_result_positions {
                        let len = srp.len();
                        // debug!("首次定位到第一个目标");
                        self.current_highlight_focus.write().replace((*rd_idx, len - 1));
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
        self.search_results.read().binary_search_by(|&a| {
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
        let s = search_str.as_str();

        let len = s.chars().count();
        {
            let sr = &mut *self.search_results.write();
            for (idx, rd) in self.data_buffer.write().iter_mut().enumerate() {
                if rd.text.contains(s) {
                    find_out = true;
                    sr.push(idx);
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
        }

        self.search_string.write().replace(search_str);

        if find_out {
            self.search_results.write().reverse();
        }
        find_out
    }

    /// 清除上一次查询的缓存记录。
    fn _clear_search_results(&mut self) {
        self.search_results.read().iter().for_each(|idx| {
            if let Some(rd) = self.data_buffer.write().get_mut(*idx) {
                rd.search_result_positions = None;
                rd.search_highlight_pos = None;
            }
        });
        self.search_results.write().clear();
        *self.current_highlight_focus.write() = None;
    }

    /// 清除查询缓存，并刷新界面。
    pub(crate) fn clear_search_results(&mut self) {
        self._clear_search_results();
        *self.search_string.write() = None;
        self.scroller.set_damage(true);
    }

    /// 定位到下一个查询目标并显示在可见区域。
    fn show_search_results(&mut self) {
        let rr = *self.current_highlight_focus.read();
        if let Some((rd_idx, result_idx)) = rr {
            let mut piece_idx = 0;
            if let Some(rd) = self.data_buffer.read().get(rd_idx) {
                if let Some(s) = self.search_string.read().as_ref() {
                    // debug!("正向定位到第{}个目标", result_idx);
                    if let Some((pos, _)) =  rd.text.rmatch_indices(s).nth(result_idx) {
                        let mut processed_len = 0usize;
                        for (i, piece_rc) in rd.line_pieces.iter().enumerate() {
                            let piece = &*piece_rc.read();
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
        let mut offset_y = 0;
        if let Some(rd) = self.data_buffer.read().first() {
            offset_y = rd.v_bounds.read().0;
        }

        if let Some(rd) = self.data_buffer.read().get(rd_idx) {
            if piece_idx < rd.line_pieces.len() {
                if let Some(piece_rc) = rd.line_pieces.get(piece_idx) {
                    let piece = &*piece_rc.read();
                    // debug!("piece.top_y: {}, panel_height: {}, scroller.yposition: {}, piece.line: {}", piece.top_y, self.panel.h(), self.scroller.yposition(), piece.line);
                    let scroller_y = self.scroller.yposition();
                    if piece.y < scroller_y || piece.y + piece.h >= scroller_y + self.scroller.h() {
                        let mut scroll_to_y = piece.y - self.scroller.h() + piece.h * 2 + PADDING.top + 3 - offset_y;
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

    /// 大数据量懒加载模式，也可称为历史模式。
    pub fn lazy_page_mode(self) -> Self {
        self.history_mode.store(true, Relaxed);
        self
    }



    /// 立即加载页数据。
    ///
    /// # Arguments
    ///
    /// * `user_data_page`:
    /// * `direction`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    /// use fltkrs_richdisplay::{PageOptions, UserData};
    /// use fltkrs_richdisplay::rich_reviewer::RichReviewer;
    ///
    /// let mut reviewer = RichReviewer::new(100, 60, 1600, 800, None).lazy_page_mode();    ///
    ///
    /// let mut reviewer_rc = reviewer.clone();
    /// let mut page_data: Vec<UserData> = vec![
    ///     UserData::new_text("由于多线程可以同时运行，所以将计算操作拆分至多个线程可以提高性能。".to_string()),
    ///     UserData::new_text("由于多线程可以同时运行，所以将计算操作拆分至多个线程可以提高性能。".to_string()),
    /// ];
    /// let opt = PageOptions::NextPage(1);
    /// reviewer_rc.load_page_now(page_data, opt);
    /// ```
    pub fn load_page_now(&mut self, user_data_page: Vec<UserData>, direction: PageOptions) {
        // debug!("已载入页数据");
        let window_width = self.panel.width();
        let drawable_max_width = window_width - PADDING.left - PADDING.right;

        let mut page_buffer = Vec::<RichData>::new();
        for ud in user_data_page {
            let default_font_text = !ud.custom_font_text;
            let default_font_color = !ud.custom_font_color;
            let mut rich_data: RichData = ud.into();
            rich_data.set_piece_spacing(self.piece_spacing.load(Relaxed));
            if default_font_text {
                rich_data.font = *self.text_font.read();
                rich_data.font_size = self.text_size.load(Relaxed);
            }
            if default_font_color {
                rich_data.fg_color = *self.text_color.read();
            }
            page_buffer.push(rich_data);
        }

        // 在尾部或头部添加页数据
        match direction {
            PageOptions::NextPage(_) => {
                self.data_buffer.write().append(&mut page_buffer);
            }
            PageOptions::PrevPage(_) => {
                let mut buffer = self.data_buffer.write();
                buffer.reverse();
                page_buffer.reverse();
                buffer.append(&mut page_buffer);
                buffer.reverse();
            }
        }
        // debug!("缓存数据已变化");

        // 重新计算数据绘制坐标，并检测是否需要继续补充页数据。
        let (need_more, panel_height) = Self::recalculate_data_buffer_position(
            self.data_buffer.clone(),
            drawable_max_width,
            &mut self.panel,
            self.scroller.clone(),
            self.basic_char.clone(),
            self.text_size.clone());
        if need_more {
            // debug!("需要更多数据");
            let load_more_fn = {
                let buffer_rc = self.data_buffer.clone();
                let page_notifier_rc = self.page_notifier.clone();
                let dir = direction.clone();
                move || {
                    let mut id = 0i64;
                    if let Some(rd) = buffer_rc.read().last() {
                        id = rd.id;
                    }
                    if id != 0 {
                        // debug!("执行回调");
                        if let Some(cp) = &mut *page_notifier_rc.write() {
                            match dir {
                                PageOptions::NextPage(_) => {
                                    cp.notify(PageOptions::NextPage(id));
                                }
                                PageOptions::PrevPage(_) => {
                                    cp.notify(PageOptions::PrevPage(id));
                                }
                            }
                            // debug!("补充数据完成！");
                        }
                    }
                }
            };
            // debug!("准备在下一个循环中补充数据...");
            awake_callback(load_more_fn);
        } else {
            // debug!("刷新页面");
            match direction {
                PageOptions::NextPage(_) => {
                    if self.scroller.yposition() as f32 / self.scroller.h() as f32 > 4.0 {
                        // debug!("当前前进位置超过4倍，触发移除远端数据操作...");
                        awake_callback({
                            let buffer_rc = self.data_buffer.clone();
                            let page_size = self.page_size.load(Relaxed);
                            let scroll_rc = self.scroller.clone();
                            let mut panel_rc = self.panel.clone();
                            let basic_char_rc = self.basic_char.clone();
                            let text_size_rc = self.text_size.clone();
                            move || {
                                let mut last_height = 0;
                                {
                                    let len = buffer_rc.read().len();
                                    let mut buffer = buffer_rc.write();
                                    if let Some(rd) = buffer.get(page_size - 1) {
                                        last_height = rd.v_bounds.read().1
                                    }
                                    buffer.reverse();
                                    buffer.truncate(len - page_size);
                                    buffer.reverse();
                                }

                                Self::recalculate_data_buffer_position(buffer_rc.clone(), drawable_max_width, &mut panel_rc, scroll_rc.clone(), basic_char_rc.clone(), text_size_rc.clone());
                                panel_rc.set_damage(true);
                                // debug!("清除远端数据完成！");

                                Self::scroll_page(panel_rc.clone(), scroll_rc.clone(), (true, last_height));
                            }
                        })
                    } else {
                        Self::scroll_page(self.panel.clone(), self.scroller.clone(), (false, 0));
                        self.panel.set_damage(true);
                    }
                }
                PageOptions::PrevPage(_) => {
                    if self.scroller.yposition() > 0 && panel_height as f32 / self.scroller.h() as f32 > 4.0 {
                        // debug!("当前后退位置超过4倍，触发移除远端数据操作...");
                        awake_callback({
                            let buffer_rc = self.data_buffer.clone();
                            let page_size = self.page_size.load(Relaxed);
                            let scroll_rc = self.scroller.clone();
                            let mut panel_rc = self.panel.clone();
                            let basic_char_rc = self.basic_char.clone();
                            let text_size_rc = self.text_size.clone();
                            move || {
                                let mut last_height = 0;
                                {
                                    let len = buffer_rc.read().len();
                                    let mut buffer = buffer_rc.write();
                                    if let Some(rd) = buffer.get(page_size - 1) {
                                        last_height = rd.v_bounds.read().1
                                    }
                                    // buffer.reverse();
                                    buffer.truncate(len - page_size);
                                    // buffer.reverse();
                                }

                                Self::recalculate_data_buffer_position(buffer_rc.clone(), drawable_max_width, &mut panel_rc, scroll_rc.clone(), basic_char_rc.clone(), text_size_rc.clone());
                                panel_rc.set_damage(true);
                                // debug!("清除远端数据完成！");

                                Self::scroll_page(panel_rc.clone(), scroll_rc.clone(), (true, -last_height));
                            }
                        })
                    } else {
                        Self::scroll_page(self.panel.clone(), self.scroller.clone(), (false, 0));
                        self.panel.set_damage(true);
                    }
                }
            }

        }
    }


    pub fn clear(&mut self) {
        self.data_buffer.write().clear();
        self.panel.resize(self.scroller.x(), self.scroller.y(), self.panel.w(), self.scroller.h());
        self.scroller.set_damage(true);
    }

    /// 设置历史模式下的分页大小。这个数值作为页面可见数据量的参考值，并不一定只显示这么多数据，可能会显示最多分页大小2倍的数据。
    ///
    /// # Arguments
    ///
    /// * `new_size`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn set_page_size(&mut self, new_size: usize) {
        self.page_size.store(new_size, Relaxed);
    }


    fn load_page(callpage: &mut CallPage, opt: PageOptions) {
        let task_id = LOAD_PAGE_TASK_ID.get_or_init(|| YitIdHelper::next_id());
        if !throttle_check(*task_id, Duration::from_millis(500)) {
            return;
        }
        callpage.notify(opt);
    }

    fn recalculate_data_buffer_position(
        data_buffer: Arc<RwLock<Vec<RichData>>>,
        drawable_max_width: i32,
        panel: &mut Widget,
        scroller: Scroll,
        basic_char: Arc<RwLock<char>>,
        text_size: Arc<AtomicI32>) -> (bool, i32) {
        let _empty = RichData::empty();
        let mut last_rd = &_empty;
        let mut is_first_data = true;

        {
            let mut buffer = data_buffer.write();
            for rd in buffer.iter_mut() {
                let last_piece = if is_first_data {
                    is_first_data = false;
                    LinePiece::init_piece(text_size.load(Relaxed))
                } else {
                    last_rd.line_pieces.last().unwrap().clone()
                };
                rd.estimate(last_piece, drawable_max_width, *basic_char.read());
                // debug!("rd.text: {}, rd.v_bounds: {:?}", rd.text, rd.v_bounds);
                last_rd = rd;
            }
        }

        // 设置新的窗口尺寸
        let (scroller_width, scroller_height) = (panel.width(), scroller.height());
        let panel_height = Self::calc_panel_height(data_buffer.clone(), scroller_height);
        panel.resize(panel.x(), panel.y(), scroller_width, panel_height);
        // debug!("panel_height: {}, scroller_height: {}", panel_height, scroller_height);
        if let Some(rd) = data_buffer.read().last() {
            // debug!("panel_height: {}, data bottom y: {}, scroller_height: {}", panel_height, rd.v_bounds.get().1, scroller_height);
            (rd.v_bounds.read().1 <= scroller_height, panel_height)
        } else {
            (false, 0)
        }
    }

    fn calc_panel_height(buffer_rc: Arc<RwLock<Vec<RichData>>>, scroller_height: i32) -> i32 {
        let buffer = &*buffer_rc.read();
        let (mut top, mut bottom) = (0, 0);
        if let Some(first) = buffer.first() {
            top = first.v_bounds.read().0;
        }
        if let Some(last) = buffer.last() {
            bottom = last.v_bounds.read().1;
        }
        let content_height = bottom - top + PADDING.bottom + PADDING.top;
        if content_height > scroller_height {
            content_height
        } else {
            scroller_height
        }
    }

    fn scroll_page(panel: Widget, mut scroller: Scroll, offset: (bool, i32)) {
        // debug!("yposition: {}, diff: {}", self.scroller.yposition(), self.panel.h() - self.scroller.h());
        let height_diff = panel.h() - scroller.h();
        let yposition = scroller.yposition();
        if yposition > height_diff {
            // scroller.scroll_to(0, height_diff);
            awake_callback({
                let mut scroller_rc = scroller.clone();
                move || {
                    // debug!("滚动到1: {}", height_diff);
                    scroller_rc.scroll_to(0, height_diff);
                    scroller_rc.set_damage(true);
                }
            });
        } else if offset.0 {
            // debug!("滚动到2: {}", yposition - offset.1);
            scroller.scroll_to(0, max(0, yposition - offset.1));
            scroller.set_damage(true);
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
        *self.text_font.write() = font;
    }

    /// 获取默认的字体。
    pub fn text_font(&self) -> Font {
        *self.text_font.read()
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
        self.text_size.store(size, Relaxed);
    }

    /// 获取默认的字体尺寸。
    pub fn text_size(&self) -> i32 {
        self.text_size.load(Relaxed)
    }

    /// 设置单个数据被自动分割成适应行宽的片段之间的水平间距（像素数，自动缩放），默认为0。仅在懒加载模式/历史模式有效。
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
        self.piece_spacing.store(spacing, Relaxed);
    }


    /// 替换闪烁状态对象。
    ///
    /// # Arguments
    ///
    /// * `state`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub(crate) fn set_blink_state(&mut self, state: BlinkState) {
        *self.blink_flag.write() = state;
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
        self.enable_blink.store(enable, Relaxed);
    }

    pub fn set_search_focus_color(&mut self, color: Color) {
        self.blink_flag.write().focus_boarder_color = color;
    }

    pub fn set_search_focus_contrast(&mut self, contrast: Color) {
        self.blink_flag.write().focus_boarder_contrast_color = contrast;
    }

    pub fn set_search_focus_color_and_contrast(&mut self, color: Color, contrast: Color) {
        let mut bf = self.blink_flag.write();
        bf.focus_boarder_color = color;
        bf.focus_boarder_contrast_color = contrast;
    }

    pub fn set_search_focus_width(&mut self, width: u8) {
        self.blink_flag.write().focus_boarder_width = width as i32;
    }

    pub fn set_search_focus_background(&mut self, background: Color) {
        self.blink_flag.write().focus_background_color = background;
    }

    /// 设置用于计算字符宽度的标准字符。
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
    pub(crate) fn expire_review_data(&mut self, target: &String) {
        expire_data(self.data_buffer.clone(), target);
        self.panel.set_damage(true);
    }
}