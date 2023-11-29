//! 富文本查看器，支持图文混排，支持历史内容回顾。支持`fluid`设计器。
//!
//! 创建组件示例：
//! ```rust,no_run
//! use fltk::{app, window};
//! use fltk::enums::{Event, Key};
//! use fltk::prelude::{GroupExt, WidgetBase, WidgetExt, WindowExt};
//! use log::error;
//! use fltkrs_richdisplay::rich_text::RichText;
//! use fltkrs_richdisplay::{RichDataOptions, UserData};
//!
//! #[tokio::main]
//! async fn main() {
//!    let app = app::App::default();
//!    let mut win = window::Window::default().with_size(1000, 600).center_screen();
//!    let mut rich_text = RichText::new(100, 120, 800, 400, None);
//!    rich_text.set_cache_size(200);
//!    win.end();
//!    win.show();
//!    app.run().unwrap();
//! }
//! ```
//!
//! 另一个支持互动的复杂示例：
//! ```rust,no_run
//! use fltk::{app, window};
//! use fltk::enums::{Event, Key};
//! use fltk::prelude::{GroupExt, WidgetBase, WidgetExt, WindowExt};
//! use log::error;
//! use fltkrs_richdisplay::rich_text::RichText;
//! use fltkrs_richdisplay::{RichDataOptions, UserData};
//!
//! pub enum GlobalMessage {
//!     ContentData(UserData),
//!     UpdateData(RichDataOptions),
//!     DisableData(i64),
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!    let app = app::App::default();
//!    let mut win = window::Window::default().with_size(1000, 600).center_screen();
//!    let mut rich_text = RichText::new(100, 120, 800, 400, None);
//!    let (action_sender, mut action_receiver) = tokio::sync::mpsc::channel::<UserData>(100);
//!    // 自定义回调函数，当用户鼠标点击可互动的数据段时，组件会调用回调函数。
//!    let cb_fn = {
//!        let sender_rc = action_sender.clone();
//!        move |user_data| {
//!            let sender = sender_rc.clone();
//!            tokio::spawn(async move {
//!                if let Err(e) = sender.send(user_data).await {
//!                    error!("发送用户操作失败: {:?}", e);
//!                }
//!            });
//!        }
//!    };
//!    rich_text.set_notifier(cb_fn);
//!    rich_text.set_cache_size(1000);
//!
//!    /*
//!    启用PageUp/PageDown快捷键打开和关闭回顾区的功能支持。
//!    使用鼠标滚轮进行打开/关闭回顾区的功能已经内置在模块包中，而PageUp/PageDown的快捷键无法被内置组件检测到，因此需要外层容器主动调用API实现。
//!    包里提供的两个API接口为此提供支持：`RichText::auto_open_reviewer(&self)`和`RichText::auto_close_reviewer(&self)`。
//!     */
//!    win.handle({
//!        let rich_text_rc = rich_text.clone();
//!        move |_, evt| {
//!            let mut handled = false;
//!            match evt {
//!                Event::KeyDown => {
//!                    if app::event_key_down(Key::PageDown) {
//!                        handled = rich_text_rc.auto_close_reviewer();
//!                    } else if app::event_key_down(Key::PageUp) {
//!                        handled = rich_text_rc.auto_open_reviewer().unwrap();
//!                    }
//!                }
//!                _ => {}
//!            }
//!            handled
//!        }
//!    });
//!
//!    let (global_sender, global_receiver) = app::channel::<GlobalMessage>();
//!
//!    win.end();
//!    win.show();
//!
//!    let global_sender_rc = global_sender.clone();
//!    tokio::spawn(async move {
//!        while let Some(data) = action_receiver.recv().await {
//!            if data.text.starts_with("10") {
//!                let toggle = !data.blink;
//!                let update_options = RichDataOptions::new(data.id).blink(toggle);
//!                global_sender_rc.send(GlobalMessage::UpdateData(update_options));
//!            }
//!        }
//!    });
//!
//!    let mut has_recent_message = false;
//!    while app.wait() {
//!        if let Some(msg) = global_receiver.recv() {
//!            match msg {
//!                GlobalMessage::ContentData(data) => {
//!                    has_recent_message = true;
//!                    rich_text.append(data);
//!                }
//!                GlobalMessage::UpdateData(options) => {
//!                    rich_text.update_data(options);
//!                }
//!                GlobalMessage::DisableData(id) => {
//!                    rich_text.disable_data(id);
//!                }
//!            }
//!        } else {
//!            has_recent_message = false; 
//!        }
//!
//!        if !has_recent_message {
//!            app::sleep(0.001);
//!            app::awake();
//!        }
//!    }
//! }
//! ```
//!
//!

use std::cell::{Cell, RefCell};
use std::cmp::{max, min, Ordering};
use std::collections::{HashMap};
use std::fmt::{Debug, Formatter};
use std::ops::{RangeInclusive};
use std::rc::{Rc, Weak};
use std::slice::Iter;
use fltk::{app, draw};
use fltk::draw::{descent, draw_image, draw_line, draw_rectf, draw_rounded_rect, draw_rounded_rectf, draw_text_n, LineStyle, measure, set_draw_color, set_font, set_line_style};
use fltk::enums::{Color, ColorDepth, Cursor, Font};
use fltk::prelude::WidgetExt;
use fltk::widget::Widget;
use std::time::Duration;

use idgenerator_thin::YitIdHelper;
use log::{error};
use throttle_my_fn::throttle;

pub mod rich_text;
pub mod rich_reviewer;
mod utils;

/// 默认内容边界到窗口之间的空白距离。
pub(crate) const PADDING: Padding = Padding { left: 5, top: 5, right: 5, bottom: 5 };

/// 图片与其他内容之间的垂直间距。
pub const IMAGE_PADDING_H: i32 = 2;

/// 图片与其他内容之间的水平间距。
pub const IMAGE_PADDING_V: i32 = 2;

/// 闪烁强度切换间隔时间，目前使用固定频率。
pub const BLINK_INTERVAL: f64 = 0.5;

/// 高亮文本背景色，查询目标时所有匹配目标的背景色。
pub const HIGHLIGHT_BACKGROUND_COLOR: Color = Color::from_rgb(0, 0, 255);

/// 高亮文本焦点边框颜色，查询目标时当前正在聚焦的目标。
pub const HIGHLIGHT_RECT_COLOR: Color = Color::from_rgb(255, 145, 0);

/// 高亮文本焦点边框对比色，当查询目标时当前正在聚焦的目标在闪烁时切换的对比颜色。
pub const HIGHLIGHT_RECT_CONTRAST_COLOR: Color = Color::from_rgb(0, 110, 255);
/// 高亮文本焦点边框弧度参数。
pub const HIGHLIGHT_ROUNDED_RECT_RADIUS: i32 = 3;

/// 最亮的白色。
pub const WHITE: Color = Color::from_rgb(255, 255, 255);

/// 默认字体尺寸。
pub const DEFAULT_FONT_SIZE: i32 = 14;


/// 回调函数载体。
/// 当用户使用鼠标点击主视图或回顾区视图上的可互动数据段时，会执行该回调函数，并将点击目标处的数据作为参数传入回调函数。
/// 用户可自由定义回调函数的具体行为。
#[derive(Clone)]
pub struct Callback {
    /// 回调函数。
    notifier: Rc<RefCell<Box<dyn FnMut(UserData)>>>,
}

impl Callback {


    /// 构建新的回调结构体实例。
    ///
    /// # Arguments
    ///
    /// * `notifier`: 回调函数包装。
    ///
    /// returns: Callback
    ///
    /// # Examples
    ///
    /// ```
    /// use std::cell::RefCell;
    /// use std::rc::Rc;
    /// use log::error;
    /// use fltkrs_richdisplay::rich_text::RichText;
    /// use fltkrs_richdisplay::{Callback, UserData};
    ///
    /// let mut rich_text = RichText::new(100, 120, 800, 400, None);
    /// let (sender, mut receiver) = tokio::sync::mpsc::channel::<UserData>(100);
    /// let cb_fn = {
    ///     let sender_rc = sender.clone();
    ///     move |user_data| {
    ///         let sender = sender_rc.clone();
    ///         tokio::spawn(async move {
    ///             if let Err(e) = sender.send(user_data).await {
    ///                 error!("发送用户操作失败: {:?}", e);
    ///             }
    ///         });
    ///     }
    /// };
    /// let cb = Callback::new(Rc::new(RefCell::new(Box::new(cb_fn))));
    /// rich_text.set_notifier(cb);
    /// ```
    pub fn new(notifier: Rc<RefCell<Box<dyn FnMut(UserData)>>>) -> Callback {
        Callback { notifier }
    }

    /// 执行回调。
    ///
    /// # Arguments
    ///
    /// * `data`: 用户数据。
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn notify(&mut self, data: UserData) {
        let notify = &mut* self.notifier.borrow_mut();
        notify(data);
    }
}

impl Debug for Callback {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Callback count: {}", Rc::<RefCell<Box<(dyn FnMut(UserData) + 'static)>>>::strong_count(&self.notifier))
    }
}

/// 分页请求参数
#[derive(Debug, Clone)]
pub enum PageOptions {
    /// 下一页，附带当前页的最后一条记录的id。
    NextPage(i64),
    /// 上一页，附带当前页的第一条记录的id。
    PrevPage(i64),
}

/// 请求新页数据的回调函数载体。
/// 当视图滚动到页面底部或顶部时，通过鼠标滚轮或按键`PageDown`或`PageUp`时，会触发执行预定义的回调函数，
/// 若有更多可用的数据，用户应当在此时提供下一页或上一页数据。
#[derive(Clone)]
pub struct CallPage {
    /// 回调函数。
    notifier: Rc<RefCell<Box<dyn FnMut(PageOptions)>>>,
}

impl Debug for CallPage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "CallPage count: {}", Rc::<RefCell<Box<(dyn FnMut(PageOptions) + 'static)>>>::strong_count(&self.notifier))
    }
}

impl CallPage {
    /// 构建新的分页回调结构体实例。
    pub fn new(notifier: Rc<RefCell<Box<dyn FnMut(PageOptions)>>>) -> Self {
        Self { notifier }
    }

    fn notify(&mut self, opt: PageOptions) {
        // let notify = &mut* self.notifier.borrow_mut();
        let notify = &mut* self.notifier.borrow_mut();
        notify(opt);
    }
}

/// 闪烁强度状态。
#[derive(Debug, Clone,Copy, PartialEq, Eq)]
pub(crate) enum BlinkDegree {
    /// 正常，原色显示。
    Normal,
    /// 对比色或不显示。
    Contrast,
}

/// 可视区域闪烁开关标记和状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BlinkState {
    /// 可视区域是否存在闪烁内容。
    on: bool,
    /// 应闪烁内容在下一次刷新显示时的强度。
    next: BlinkDegree,

    /// 焦点目标的边框颜色。
    focus_boarder_color: Color,

    /// 焦点目标的边框对比色。
    focus_boarder_contrast_color: Color,

    /// 焦点目标的边框线条宽度。
    focus_boarder_width: i32,

    /// 焦点目标的背景颜色。
    focus_background_color: Color,
}

impl BlinkState {
    pub fn new() -> BlinkState {
        BlinkState {
            on: false,
            next: BlinkDegree::Normal,
            focus_boarder_color: HIGHLIGHT_RECT_COLOR,
            focus_boarder_contrast_color: HIGHLIGHT_RECT_CONTRAST_COLOR,
            focus_boarder_width: 2,
            focus_background_color: HIGHLIGHT_BACKGROUND_COLOR
        }
    }

    pub fn off(mut self) {
        self.on = false;
        self.next = BlinkDegree::Normal;
    }

    pub fn on(&mut self) {
        self.on = true;
    }

    pub fn toggle_when_on(&mut self) -> bool {
        if self.on {
            self.next = match self.next {
                BlinkDegree::Normal => BlinkDegree::Contrast,
                BlinkDegree::Contrast => BlinkDegree::Normal,
            };
            true
        } else {
            false
        }
    }

}

/// 自定义事件。
pub(crate) struct LocalEvent;
impl LocalEvent {

    /// 滚动事件。
    pub const SCROLL_TO: i32 = 100;

    /// 缩放事件。
    pub const RESIZE: i32 = 101;

    /// 从rich-display容器外部发起关闭回顾区的事件。
    pub const DROP_REVIEWER_FROM_EXTERNAL: i32 = 102;

    /// 从rich-display容器外部发起打开回顾区的事件。
    pub const OPEN_REVIEWER_FROM_EXTERNAL: i32 = 103;
}

/// 矩形结构，元素0/1代表x/y坐标，表示左上角坐标；元素2/3代表w/h宽和高，w/h不为负值。
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub(crate) struct Rectangle(i32, i32, i32, i32);

impl PartialOrd<Self> for Rectangle {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.1 > other.1 {
            Some(Ordering::Greater)
        } else if self.1 < other.1 {
            Some(Ordering::Less)
        } else {
            if self.0 > other.0 {
                Some(Ordering::Greater)
            } else if self.0 < other.0 {
                Some(Ordering::Less)
            } else {
                Some(Ordering::Equal)
            }
        }
    }
}

impl Ord for Rectangle {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.1 > other.1 {
            Ordering::Greater
        } else if self.1 < other.1 {
            Ordering::Less
        } else {
            if self.0 > other.0 {
                Ordering::Greater
            } else if self.0 < other.0 {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        }
    }
}

impl Rectangle {
    /// 构建新的矩形结构，并且保证构建出的矩形x/y在左上角，w/h大于等于0.
    ///
    /// # Arguments
    ///
    /// * `x`: 起始x坐标。
    /// * `y`: 起始y坐标。
    /// * `w`: 起始宽度，可以小于零。用于兼容鼠标向任意方向拖拽的场景。
    /// * `h`: 起始高度，可以小于零。用于兼容鼠标向任意方向拖拽的场景。
    ///
    /// returns: Rectangle
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        if w < 0 && h < 0 {
            Rectangle(x + w, y + h, -w, -h)
        } else if w < 0 {
            Rectangle(x + w, y, -w, h)
        } else if h < 0 {
            Rectangle(x, y + h, w, -h)
        } else {
            Rectangle(x, y, w, h)
        }
    }

    /// 构建一个空矩形。
    pub fn zero() -> Self {
        Rectangle::new(0, 0, 0, 0)
    }

    pub fn replace(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.0 = x;
        self.1 = y;
        self.2 = w;
        self.3 = h;
    }

    pub fn is_below(&self, another: &Self) -> bool {
        self.1 > another.1 + another.3
    }

    pub fn tup(&self) -> (i32, i32, i32, i32) {
        (self.0, self.1, self.2, self.3)
    }

    // /// 获得当前矩形左上角和右下角的坐标。
    // pub(crate) fn corner(&self) -> (ClickPoint, ClickPoint) {
    //     (ClickPoint::new(self.0, self.1), ClickPoint::new(self.0 + self.2, self.1 + self.3))
    // }

    /// 获得当前矩形左上角和右下角的空矩形。
    pub fn corner_rect(&self) -> (Self, Self) {
        (ClickPoint::new(self.0, self.1).as_rect(), ClickPoint::new(self.0 + self.2, self.1 + self.3).as_rect())
    }
}

/// 鼠标点击位置的坐标信息和数据片段索引信息。
#[derive(Debug, Clone, Copy)]
pub(crate) struct ClickPoint {
    pub x: i32,
    pub y: i32,
    /// 点所在分片在数据段中的索引号。
    pub p_i: usize,
    /// 点所在字符在分片中的索引号。
    pub c_i: usize,
}

impl ClickPoint {

    pub fn new(x: i32, y: i32) -> ClickPoint {
        Self {x, y, p_i: 0, c_i: 0}
    }

    /// 以一个点构建一个0宽高的举行结构。
    pub fn as_rect(&self) -> Rectangle {
        Rectangle::new(self.x, self.y, 0, 0)
    }

    /// 依据两个点构建新的矩形结构，无需考虑两点之间的相对位置，构建出的矩形x/y始终代表左上角坐标，w/h始终大于等于0。
    ///
    /// # Arguments
    ///
    /// * `to_point`: 另一个点。
    ///
    /// returns: Rectangle
    ///
    /// # Examples
    ///
    /// ```
    /// ```
    pub fn to_rect(&self, to_point: &Self) -> Rectangle {
        Rectangle::new(self.x, self.y, to_point.x - self.x, to_point.y - self.y)
    }
}

/// 同一行内多个分片之间共享的信息。通过Rc<RefCell<ThroughLine>>进行链接。
#[derive(Debug, Clone)]
pub(crate) struct ThroughLine {
    pub max_h: i32,
    pub ys: RefCell<Vec<Weak<RefCell<LinePiece>>>>,
    pub exist_image: bool,
}

impl Default for ThroughLine {
    fn default() -> Self {
        ThroughLine {
            max_h: 1,
            ys: RefCell::new(vec![]),
            exist_image: false,
        }
    }
}

impl ThroughLine {

    pub fn new(max_h: i32, exist_image: bool) -> Rc<RefCell<ThroughLine>> {
        Rc::new(RefCell::new(ThroughLine { max_h, exist_image, ys: RefCell::new(vec![]) }))
    }

    /// 如果传入的高度大于已记录高度，则替换为传入高度。
    ///
    /// # Arguments
    ///
    /// * `max_h`:
    ///
    /// returns: &mut ThroughLine
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn set_max_h(&mut self, max_h: i32) -> &mut Self {
        if max_h > self.max_h {
            self.max_h = max_h;
        }
        self
    }

    pub fn add_piece(&mut self, lp: Rc<RefCell<LinePiece>>) -> &mut Self {
        self.ys.borrow_mut().push(Rc::downgrade(&lp));
        self
    }

    /// 获取前一个分片的链接，或者创建新的链接。
    ///
    /// # Arguments
    ///
    /// * `x_ref`: 绘制起点的x坐标。
    /// * `start_x`: 当前分片的绘制起点。
    /// * `current_line_height`: 当前分片行高，或者图片高度。如果这个高度大于链接中记录的高度，则替换为新的高度。
    /// * `last_piece`: 前一个分片。
    /// * `image`: 当前分片是否为图形。
    ///
    /// returns: Rc<RefCell<ThroughLine>> 若当前分片是所在行的第一个分片则创新的链接，否则返回前一个分片的链接。
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn create_or_update(x_ref: i32, start_x: i32, current_line_height: i32, last_piece: Rc<RefCell<LinePiece>>, image: bool) -> Rc<RefCell<ThroughLine>> {
        if start_x == x_ref {
            ThroughLine::new(current_line_height, image)
        } else {
            if image {
                last_piece.borrow_mut().through_line.borrow_mut().exist_image = true;
            }
            last_piece.borrow().through_line.clone()
        }
    }
}

/// 可视内容在面板容器中的边界空白。
#[derive(Debug, Clone, Default)]
pub(crate) struct Padding {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// 单行文本的渲染参数，通过试算得到。
/// 一个大段文本在试算过程中，可能被拆分为多个适配当前窗口宽度的单行文本片段，用于简化绘制过程的运算。
#[derive(Debug, Clone)]
pub(crate) struct LinePiece {
    pub line: String,
    /// 起点x坐标
    pub x: i32,
    /// 起点y坐标
    pub y: i32,
    /// 分片宽度，小于等于行宽
    pub w: i32,
    /// 行高
    pub h: i32,
    /// 虚拟行高顶部y坐标
    pub top_y: i32,
    /// 额外的行间距
    pub spacing: i32,
    /// 建议下一个数据分片绘制起点x坐标
    pub next_x: i32,
    /// 建议下一个数据分片绘制起点y坐标
    pub next_y: i32,

    /// 字体渲染高度，小于等于行高。
    pub font_height: i32,

    /// 在同一行内有多个数据分片的情况下， 跟踪行高信息。每次新增行时，第一个分片需要创建新的对象；在同一行其他分片只需要引用第一个分片的对象即可。
    pub through_line: Rc<RefCell<ThroughLine>>,

    /// 选中文字相对于当前片段的起始到结束位置，位置索引是以`unicode`字符计算的。
    pub selected_range: Rc<Cell<Option<(usize, usize)>>>,

    pub font: Font,
    pub font_size: i32,

    /// 分片所在数据段的边界数据引用。
    pub rd_bounds: Rc<Cell<(i32, i32, i32, i32)>>,
}

impl LinePiece {
    pub fn new(line: String, x: i32, y: i32, w: i32, h: i32, top_y: i32, spacing: i32, next_x: i32, next_y: i32, font_height: i32, font: Font, font_size: i32, through_line: Rc<RefCell<ThroughLine>>, rd_bounds: Rc<Cell<(i32, i32, i32, i32)>>) -> Rc<RefCell<LinePiece>> {
        let new_piece = Rc::new(RefCell::new(Self {
            line,
            x,
            y,
            w,
            h,
            top_y,
            spacing,
            next_x,
            next_y,
            font_height,
            through_line: through_line.clone(),
            selected_range: Rc::new(Cell::new(None)),
            font,
            font_size,
            rd_bounds
        }));
        through_line.borrow_mut().add_piece(new_piece.clone());
        new_piece
    }

    pub fn init_piece() -> Rc<RefCell<LinePiece>> {
        let through_line = Rc::new(RefCell::new(Default::default()));
        let init_piece = Rc::new(RefCell::new(Self {
            line: "".to_string(),
            x: PADDING.left,
            y: PADDING.top,
            w: 0,
            h: 1,
            top_y: PADDING.top,
            spacing: 0,
            next_x: PADDING.left,
            next_y: PADDING.top,
            font_height: 1,
            through_line: through_line.clone(),
            selected_range: Rc::new(Cell::new(None)),
            font: Font::Helvetica,
            font_size: 12,
            rd_bounds: Rc::new(Cell::new((0, 0, 0, 0))),
        }));
        through_line.borrow_mut().add_piece(init_piece.clone());
        init_piece
    }

    pub fn select_from(&self, from: usize) {
        self.selected_range.set(Some((from, self.line.chars().count())));
    }

    pub fn select_to(&self, to: usize) {
        self.selected_range.set(Some((0, to)));
    }

    pub fn select_range(&self, from: usize, to: usize) {
        self.selected_range.set(Some((from, to)));
    }

    pub fn deselect(&self) {
        self.selected_range.set(None);
    }

    pub fn select_all(&self) {
        self.selected_range.set(Some((0, self.line.chars().count())));
    }

    pub fn copy_selection(&self, selection: &mut String) {
        if let Some((from, to)) = self.selected_range.get() {
            self.line.chars().skip(from).take(to - from).for_each(|c| {
                selection.push(c);
            });
        }
    }

    pub fn rect(&self, offset_y: i32) -> Rectangle {
        Rectangle::new(self.x, self.y - offset_y, self.w, self.h)
    }
}

pub(crate) trait LinedData {
    /// 设置绘制区域顶部和底部边界y坐标，以及起始x坐标。
    ///
    /// # Arguments
    ///
    /// * `start_point`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn set_v_bounds(&mut self, top_y: i32, bottom_y: i32, start_x: i32, end_x: i32);

    /// 表明是否纯文本数据段。
    fn is_text_data(&self) -> bool;

    /// 是否可点击互动。
    fn clickable(&self) -> bool;

    /// 设置可点击互动。
    fn set_clickable(&mut self, clickable: bool);

    /// 是否已失效
    fn is_expired(&self) -> bool;

    /// 设置失效状态。
    ///
    /// # Arguments
    ///
    /// * `expire`: 指定是否失效。
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn set_expire(&mut self, expire: bool);

    /// 设置文本数据。
    ///
    /// # Arguments
    ///
    /// * `text_data`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn set_text_data(&mut self, text_data: &str);

    /// 设置二
    ///
    /// # Arguments
    ///
    /// * `binary_data`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn set_binary_data(&mut self, binary_data: Vec<u8>);


    /// 检测是否位于可视窗口范围内。
    ///
    /// # Arguments
    ///
    /// * `top_y`: 面板顶部y轴偏移量。
    /// * `bottom_y`: 面板底部y轴偏移量。
    ///
    /// returns: bool
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn is_visible(&self, top_y: i32, bottom_y: i32) -> bool;


    /// 绘制内容。
    ///
    /// # Arguments
    ///
    /// * `offset_y`: 面板相对于数据的y轴偏移量。
    /// * `blink_state`: 面板范围内的闪烁状态。
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn draw(&self, offset_y: i32, blink_state: &BlinkState);

    /// 试算当前内容绘制后所占高度信息。
    /// 试算功能自动处理文本超宽时截断换行的逻辑。
    ///
    /// # Arguments
    ///
    /// * `last_piece`: 前一个数据片段，用于计算当前数据段的绘制坐标。每个数据段和数据片段都是按照缓存数据的顺序依次计算得到。
    /// * `max_width`: 可视区域最大宽度，不含padding宽度。
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn estimate(&mut self, last_piece: Rc<RefCell<LinePiece>>, max_width: i32) -> Rc<RefCell<LinePiece>>;

}

/// 数据段类型，当前支持文本和图片两种。
#[derive(Clone, Debug, PartialEq)]
pub enum DataType {
    Text,
    Image,
}

/// 用户提供的数据段结构。。
#[derive(Clone, Debug)]
pub struct UserData {
    /// 数据ID，在初始化新实例时可随意赋值。当源自RichData时，为RichData的ID值。
    pub id: i64,
    pub text: String,
    pub font: Font,
    pub font_size: i32,
    pub fg_color: Color,
    pub bg_color: Option<Color>,
    pub underline: bool,
    pub clickable: bool,
    pub expired: bool,
    pub blink: bool,
    pub disabled: bool,
    pub strike_through: bool,
    pub data_type: DataType,
    pub image: Option<Vec<u8>>,
    pub image_width: i32,
    pub image_height: i32,
    pub(crate) custom_font_text: bool,
    pub(crate) custom_font_color: bool,
}

impl From<&RichData> for UserData {
    fn from(data: &RichData) -> Self {
        Self {
            id: data.id,
            text: data.text.clone(),
            font: data.font,
            font_size: data.font_size,
            fg_color: data.fg_color,
            bg_color: data.bg_color.clone(),
            underline: data.underline,
            clickable: data.clickable,
            expired: data.expired,
            blink: data.blink,
            disabled: data.disabled,
            strike_through: data.strike_through,
            data_type: data.data_type.clone(),
            image: data.image.clone(),
            image_width: data.image_width,
            image_height: data.image_height,
            custom_font_text: false,
            custom_font_color: false,
        }
    }
}

impl UserData {
    pub fn new_text(text: String) -> Self {
        Self {
            id: YitIdHelper::next_id(),
            text,
            font: Font::Helvetica,
            font_size: 14,
            fg_color: Color::White,
            bg_color: None,
            underline: false,
            clickable: false,
            expired: false,
            blink: false,
            disabled: false,
            strike_through: false,
            data_type: DataType::Text,
            image: None,
            image_width: 0,
            image_height: 0,
            custom_font_text: false,
            custom_font_color: false,
        }
    }

    pub fn new_image(image: Vec<u8>, width: i32, height: i32) -> Self {
        Self {
            id: YitIdHelper::next_id(),
            text: String::new(),
            font: Font::Helvetica,
            font_size: 14,
            fg_color: Color::White,
            bg_color: None,
            underline: false,
            clickable: false,
            expired: false,
            blink: false,
            disabled: false,
            strike_through: false,
            data_type: DataType::Image,
            image: Some(image),
            image_width: width,
            image_height: height,
            custom_font_text: false,
            custom_font_color: false,
        }
    }

    pub fn set_font(mut self, font: Font, size: i32) -> Self {
        self.font = font;
        self.font_size = size;
        self.custom_font_text = true;
        self
    }

    pub fn set_fg_color(mut self, fg_color: Color) -> Self {
        self.fg_color = fg_color;
        self.custom_font_color = true;
        self
    }

    pub fn set_bg_color(mut self, bg_color: Option<Color>) -> Self {
        self.bg_color = bg_color;
        self
    }

    pub fn set_underline(mut self, u: bool) -> Self {
        self.underline = u;
        self
    }

    pub fn set_clickable(mut self, clickable: bool) -> Self {
        self.clickable = clickable;
        self
    }

    pub fn set_blink(mut self, blink: bool) -> Self {
        self.blink = blink;
        self
    }

    pub fn set_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}


/// 计算两个重叠垂线居中对齐后，短线相对于长线的上端和下端的偏移量。
///
/// # Arguments
///
/// * `line_height`:
/// * `font_height`:
///
/// returns: (i32, i32)
///
/// # Examples
///
/// ```
///
/// ```
pub(crate) fn calc_v_center_offset(line_height: i32, font_height: i32) -> (i32, i32) {
    let up = (line_height - font_height) / 2;
    let down = (line_height + font_height) / 2;
    (up, down)
}

/// 检测鼠标是否进入可交互的内容区域中。
///
/// # Arguments
///
/// * `visible_lines`:
///
/// returns: bool
///
/// # Examples
///
/// ```
///
/// ```
pub(crate) fn mouse_enter(visible_lines: Rc<RefCell<HashMap<Rectangle, usize>>>) -> bool {
    for area in visible_lines.borrow().keys() {
        let (x, y, w, h) = area.tup();
        if app::event_inside(x, y, w, h) {
            return true;
        }
    }
    return false;
}

/// 更新数据内容的属性。用于用户互动操作反馈。
///
/// # Arguments
///
/// * `options`:
/// * `rd`:
///
/// returns: ()
///
/// # Examples
///
/// ```
///
/// ```
pub(crate) fn update_data_properties(options: RichDataOptions, rd: &mut RichData) {
    if let Some(clickable) = options.clickable {
        rd.clickable = clickable;
        if !clickable {
            draw::set_cursor(Cursor::Default);
        }
    }
    if let Some(underline) = options.underline {
        rd.underline = underline;
    }
    if let Some(expired) = options.expired {
        rd.expired = expired;
    }
    if let Some(text) = options.text {
        rd.text = text;
    }
    if let Some(fg_color) = options.fg_color {
        rd.fg_color = fg_color;
    }
    if let Some(bg_color) = options.bg_color {
        rd.bg_color = Some(bg_color);
    }
    if let Some(strike_through) = options.strike_through {
        rd.strike_through = strike_through;
    }
    if let Some(blink) = options.blink {
        rd.blink = blink;
    }
    if let Some(disabled) = options.disabled {
        rd.disabled = disabled;

        if rd.data_type == DataType::Image {
            if rd.disabled && rd.image_inactive.is_none() {
                if let Some(ref image) = rd.image {
                    let new_image = gray_image(image);
                    rd.image_inactive = Some(new_image);
                }
            }
        }
    }
}

/// 禁用数据内容。
/// 当前的实现为：图形内容增加灰色遮罩层，文本内容增加删除线。
///
/// # Arguments
///
/// * `rd`:
///
/// returns: ()
///
/// # Examples
///
/// ```
///
/// ```
pub(crate) fn disable_data(rd: &mut RichData) {
    rd.set_clickable(false);
    draw::set_cursor(Cursor::Default);

    match rd.data_type {
        DataType::Image => {
            if rd.image_inactive.is_none() {
                if let Some(ref image) = rd.image {
                    let new_image = gray_image(image);
                    rd.image_inactive = Some(new_image);
                }
            }
        }
        DataType::Text => {
            rd.strike_through = true;
        }
    }
}

/// 将RGB8格式的图片内容转换为L8格式的灰度内容。
///
/// # Arguments
///
/// * `rgb_image`: RGB8格式的图片数据。
///
/// returns: Vec<u8, Global> 灰度计算后的图片数据，格式为L8。
///
/// # Examples
///
/// ```
///
/// ```
pub fn gray_image(rgb_image: &Vec<u8>) -> Vec<u8> {
    let gray_image_len = rgb_image.len() / 3;
    let mut gray_image: Vec<u8> = Vec::with_capacity(gray_image_len);
    let mut i = 0;
    while i < gray_image_len {
        let j = i * 3;
        let (r, g, b) = (rgb_image[j], rgb_image[j + 1], rgb_image[j + 2]);
        let gray = (306 * r as u32 + 601 * g as u32 + 116 * b as u32) >> 10;
        gray_image.push(gray as u8);
        i += 1;
    }
    gray_image
}

/// 组件内部使用的数据段结构。
#[derive(Debug, Clone)]
pub(crate) struct RichData {
    /// 数据ID。
    pub id: i64,
    pub text: String,
    pub font: Font,
    pub font_size: i32,
    pub fg_color: Color,
    pub bg_color: Option<Color>,
    underline: bool,
    clickable: bool,
    expired: bool,
    /// 闪烁片段列表
    blink: bool,
    disabled: bool,
    pub strike_through: bool,
    pub line_height: i32,
    /// 当前内容在面板垂直高度中的起始和截至y坐标，以及起始和结尾x坐标。
    v_bounds: Rc<Cell<(i32, i32, i32, i32)>>,

    /// 对当前数据进行试算后，分割成适配单行宽度的分片保存起来。由于无需跨线程传输，因此也不考虑线程安全问题。
    pub(crate) line_pieces: Vec<Rc<RefCell<LinePiece>>>,
    data_type: DataType,
    /// 格式为`ColorDepth::RGB8`的图片数据。
    image: Option<Vec<u8>>,
    image_width: i32,
    image_height: i32,
    /// 格式为`ColorDepth::L8`的灰度数据。
    image_inactive: Option<Vec<u8>>,
    /// 多行片段之间的水平空白距离。
    piece_spacing: i32,

    pub(crate) search_result_positions: Option<Vec<(usize, usize)>>,
    pub(crate) search_highlight_pos: Option<usize>,
}

impl From<UserData> for RichData {
    fn from(data: UserData) -> Self {
        match data.data_type {
            DataType::Text => {
                RichData {
                    id: data.id,
                    text: data.text,
                    font: data.font,
                    font_size: data.font_size,
                    fg_color: data.fg_color,
                    bg_color: data.bg_color,
                    underline: data.underline,
                    clickable: data.clickable,
                    expired: data.expired,
                    blink: data.blink,
                    disabled: false,
                    strike_through: data.strike_through,
                    line_height: 1,
                    v_bounds: Rc::new(Cell::new((0, 0, 0, 0))),
                    line_pieces: vec![],
                    data_type: DataType::Text,
                    image: None,
                    image_width: 0,
                    image_height: 0,
                    image_inactive: None,
                    piece_spacing: 0,
                    search_result_positions: None,
                    search_highlight_pos: None,
                }
            },
            DataType::Image => {
                let mut image_inactive: Option<Vec<u8>> = None;
                if let Some(ref image) = data.image {
                    let inactive_image = gray_image(image);
                    image_inactive.replace(inactive_image);
                }
                RichData {
                    id: data.id,
                    text: data.text,
                    font: data.font,
                    font_size: data.font_size,
                    fg_color: data.fg_color,
                    bg_color: data.bg_color,
                    underline: data.underline,
                    clickable: data.clickable,
                    expired: data.expired,
                    blink: data.blink,
                    disabled: false,
                    strike_through: data.strike_through,
                    line_height: 1,
                    v_bounds: Rc::new(Cell::new((0, 0, 0, 0))),
                    line_pieces: Vec::with_capacity(0),
                    data_type: DataType::Image,
                    image: data.image,
                    image_width: data.image_width,
                    image_height: data.image_height,
                    image_inactive,
                    piece_spacing: 0,
                    search_result_positions: None,
                    search_highlight_pos: None,
                }
            }
        }
    }
}

impl RichData {
    pub(crate) fn empty() -> Self {
        RichData {
            id: 0,
            text: String::new(),
            font: Font::Helvetica,
            font_size: 0,
            fg_color: Color::White,
            bg_color: None,
            underline: false,
            clickable: false,
            expired: false,
            blink: false,
            disabled: false,
            strike_through: false,
            line_height: 1,
            v_bounds: Rc::new(Cell::new((0, 0, 0, 0))),
            line_pieces: Vec::with_capacity(0),
            data_type: DataType::Text,
            image: None,
            image_width: 0,
            image_height: 0,
            image_inactive: None,
            piece_spacing: 0,
            search_result_positions: None,
            search_highlight_pos: None,
        }
    }

    pub(crate) fn set_piece_spacing(&mut self, piece_spacing: i32) {
        self.piece_spacing = piece_spacing;
    }
    
    /// 处理超宽的数据单元，自动换行。
    ///
    /// # Arguments
    ///
    /// * `text`:
    /// * `last_piece`:
    /// * `max_width`:
    /// * `padding`:
    /// * `measure_width`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn wrap_text_for_estimate(&mut self, text: &str, last_piece: Rc<RefCell<LinePiece>>, max_width: i32, measure_width: i32, font_height: i32) -> Rc<RefCell<LinePiece>> {
        let original = last_piece.clone();
        let last_piece = last_piece.borrow().clone();
        let tw = Rc::new(RefCell::new(0));
        let text_len = text.chars().count();
        let (font, font_size) = (self.font, self.font_size);
        if let Ok(stop_pos) = (0..text_len).collect::<Vec<usize>>().binary_search_by({
            let x = last_piece.next_x + self.piece_spacing;
            let tw_rc = tw.clone();
            move |pos| {
                let (tw1, _) = measure(text.chars().take(*pos).collect::<String>().as_str(), false);
                if x + tw1 <= max_width {
                    if *pos == text_len - 1 {
                        tw_rc.replace(tw1);
                        Ordering::Equal
                    } else {
                        let (tw2, _) = measure(text.chars().take(*pos + 1).collect::<String>().as_str(), false);
                        if x + tw2 > max_width {
                            tw_rc.replace(tw1);
                            Ordering::Equal
                        } else {
                            Ordering::Less
                        }
                    }
                } else {
                    Ordering::Greater
                }
            }
        }) {
            // 出现超宽
            let w = *tw.borrow();
            // 换行处理
            let next_x = PADDING.left;
            let through_line = ThroughLine::create_or_update(PADDING.left, last_piece.next_x, font_height, original.clone(), false);
            let line_max_h = through_line.borrow().max_h;
            let max_h = max(line_max_h, font_height);
            let mut next_y = last_piece.next_y + max_h + last_piece.spacing;
            if through_line.borrow().exist_image {
                next_y += IMAGE_PADDING_V * 2;
            }

            let y = last_piece.next_y;
            let top_y = last_piece.next_y;
            let new_piece = LinePiece::new(text.chars().take(stop_pos).collect::<String>(), last_piece.next_x, y, w, font_height, top_y, last_piece.spacing, next_x, next_y, font_height, font, font_size,  through_line.clone(), self.v_bounds.clone());
            self.line_pieces.push(new_piece.clone());

            let rest_str = text.chars().skip(stop_pos).collect::<String>();
            let rest_width = measure_width - w;

            if rest_width > max_width {
                // 剩余部分的宽度仍然大于一整行宽度
                self.wrap_text_for_estimate(rest_str.as_str(), new_piece.clone(), max_width, rest_width, font_height)
            } else {
                let rest_x = next_x;
                let rest_y = next_y;
                let top_y = next_y;
                let mut rest_next_x = rest_x + rest_width + self.piece_spacing;
                let mut rest_next_y = next_y;
                if rest_str.ends_with("\n") {
                    rest_next_x = PADDING.left;
                    rest_next_y += font_height + last_piece.spacing;
                }

                let through_line = ThroughLine::create_or_update(PADDING.left, rest_x, font_height, original.clone(), false);
                let new_piece = LinePiece::new(rest_str, rest_x, rest_y, rest_width, font_height, top_y, last_piece.spacing, rest_next_x, rest_next_y, font_height, font, font_size, through_line, self.v_bounds.clone());
                self.line_pieces.push(new_piece.clone());
                new_piece
            }
        } else {
            // 从行首开始
            let through_line = ThroughLine::create_or_update(PADDING.left, PADDING.left, self.line_height, original.clone(), false);
            let y = last_piece.next_y + last_piece.through_line.borrow().max_h + last_piece.spacing;
            let new_piece = LinePiece::new(text.to_string(), PADDING.left, y, measure_width, self.line_height, y, last_piece.spacing, PADDING.left, y, font_height, font, font_size, through_line, self.v_bounds.clone());
            self.wrap_text_for_estimate(text, new_piece, max_width, measure_width, font_height)
        }
    }

}


impl LinedData for RichData {
    fn set_v_bounds(&mut self, top_y: i32, bottom_y: i32, start_x: i32, end_x: i32,) {
        self.v_bounds.replace((top_y, bottom_y, start_x, end_x));
    }

    fn is_text_data(&self) -> bool {
        self.image.is_none()
    }

    fn clickable(&self) -> bool {
        self.clickable && !self.expired
    }

    fn set_clickable(&mut self, clickable: bool) {
        self.clickable = clickable;
    }

    fn is_expired(&self) -> bool {
        self.expired
    }

    fn set_expire(&mut self, expire: bool) {
        self.expired = expire;
    }

    fn set_text_data(&mut self, text_data: &str) {
        self.text.clear();
        self.text.push_str(text_data);
    }

    fn set_binary_data(&mut self, _: Vec<u8>) {}

    fn is_visible(&self, top_y: i32, bottom_y: i32) -> bool {
        let b = self.v_bounds.get();
        !(b.1 < top_y || b.0 > bottom_y)
    }

    fn draw(&self, offset_y: i32, blink_state: &BlinkState) {
        match self.data_type {
            DataType::Text => {
                let bg_offset = 1;
                let mut processed_search_len = 0usize;
                set_font(self.font, self.font_size);
                for piece in self.line_pieces.iter() {
                    let piece = &*piece.borrow();
                    let text = piece.line.trim_end_matches('\n');
                    if text.is_empty() {
                        continue;
                    }

                    let y = piece.y - offset_y;

                    if !self.blink || blink_state.next == BlinkDegree::Normal {
                        if let Some(bg_color) = &self.bg_color {
                            // 绘制文字背景色
                            set_draw_color(*bg_color);

                            #[cfg(target_os = "linux")]
                            draw_rectf(piece.x, y - piece.spacing + 2, piece.w, piece.font_height);

                            #[cfg(not(target_os = "linux"))]
                            draw_rectf(piece.x, y - piece.spacing, piece.w, piece.font_height);
                        }
                    }

                    if let Some((from, to)) = piece.selected_range.get() {
                        // 绘制选中背景色
                        let sel_color = if let Some(bg_color) = &self.bg_color {
                            if *bg_color == Color::Blue || *bg_color == Color::DarkBlue {
                                Color::DarkMagenta
                            } else {
                                Color::Selection
                            }
                        } else {
                            Color::Selection
                        };
                        set_draw_color(sel_color);
                        let (skip_width, _) = measure(piece.line.chars().take(from).collect::<String>().as_str(), false);
                        let (fill_width, _) = measure(piece.line.chars().skip(from).take(to - from).collect::<String>().as_str(), false);

                        #[cfg(target_os = "linux")]
                        draw_rectf(piece.x + skip_width, y - piece.spacing + 2, fill_width, piece.font_height);

                        #[cfg(not(target_os = "linux"))]
                        draw_rectf(piece.x + skip_width, y - piece.spacing, fill_width, piece.font_height);
                    }

                    // 绘制查找焦点框
                    if let Some(ref pos_vec) = self.search_result_positions {
                        let rect_color = if blink_state.next == BlinkDegree::Normal {
                            blink_state.focus_boarder_color
                        } else {
                            blink_state.focus_boarder_contrast_color
                        };
                        let pl = piece.line.chars().count();
                        let range = processed_search_len..(processed_search_len + pl);
                        pos_vec.iter().enumerate().for_each(|(pos_i, (pos_from, pos_to))| {
                            if range.contains(pos_from) {
                                let start_index_of_piece = pos_from - processed_search_len;
                                let (skip_width, _) = measure(piece.line.chars().take(start_index_of_piece).collect::<String>().as_str(), false);
                                let (fill_width, _) = measure(piece.line.chars().skip(start_index_of_piece).take(pos_to - pos_from).collect::<String>().as_str(), false);

                                set_draw_color(blink_state.focus_background_color);
                                #[cfg(not(target_os = "windows"))]
                                {
                                    // draw_rectf(piece.x + skip_width, y - piece.spacing + 2, fill_width, piece.font_height);
                                    draw_rounded_rectf(piece.x + skip_width, y - piece.spacing + 2, fill_width, piece.font_height, HIGHLIGHT_ROUNDED_RECT_RADIUS);
                                    if let Some(h_i) = self.search_highlight_pos {
                                        if h_i == pos_i {
                                            // debug!("blink1: {:?}", blink_state);
                                            set_draw_color(rect_color);
                                            set_line_style(LineStyle::Solid, blink_state.focus_boarder_width);
                                            // draw_rect_with_color(piece.x + skip_width, y - piece.spacing + 2, fill_width, piece.font_height, rect_color);
                                            draw_rounded_rect(piece.x + skip_width, y - piece.spacing + 2, fill_width, piece.font_height, HIGHLIGHT_ROUNDED_RECT_RADIUS);
                                            set_line_style(LineStyle::Solid, 0);
                                        }
                                    }
                                }

                                #[cfg(target_os = "windows")]
                                {
                                    // draw_rectf(piece.x + skip_width, y - piece.spacing, fill_width, piece.font_height);
                                    draw_rounded_rectf(piece.x + skip_width, y - piece.spacing, fill_width, piece.font_height, HIGHLIGHT_ROUNDED_RECT_RADIUS);
                                    if let Some(h_i) = self.search_highlight_pos {
                                        if h_i == pos_i {
                                            set_draw_color(rect_color);
                                            set_line_style(LineStyle::Solid, blink_state.focus_boarder_width);
                                            // draw_rect_with_color(piece.x + skip_width, y - piece.spacing, fill_width, piece.font_height, rect_color);
                                            draw_rounded_rect(piece.x + skip_width, y - piece.spacing, fill_width, piece.font_height, HIGHLIGHT_ROUNDED_RECT_RADIUS);
                                            set_line_style(LineStyle::Solid, 0);
                                        }
                                    }
                                }

                            } else if range.contains(pos_to) {
                                let (fill_width, _) = measure(piece.line.chars().take(pos_to - processed_search_len).collect::<String>().as_str(), false);

                                set_draw_color(blink_state.focus_background_color);
                                // draw_rectf(piece.x, y - piece.spacing, fill_width, piece.font_height);
                                draw_rounded_rectf(piece.x, y - piece.spacing, fill_width, piece.font_height, HIGHLIGHT_ROUNDED_RECT_RADIUS);
                                if let Some(h_i) = self.search_highlight_pos {
                                    if h_i == pos_i {
                                        set_draw_color(rect_color);
                                        set_line_style(LineStyle::Solid, blink_state.focus_boarder_width);
                                        // draw_rect_with_color(piece.x, y - piece.spacing, fill_width, piece.font_height, rect_color);
                                        draw_rounded_rect(piece.x, y - piece.spacing, fill_width, piece.font_height, HIGHLIGHT_ROUNDED_RECT_RADIUS);
                                        set_line_style(LineStyle::Solid, 0);
                                    }
                                }
                            }
                        });
                        processed_search_len += pl;
                    }

                    if self.blink && blink_state.next == BlinkDegree::Contrast {
                        set_draw_color(get_lighter_or_darker_color(self.fg_color));
                    } else {
                        set_draw_color(self.fg_color);
                    }

                    if self.underline {
                        // 绘制下划线
                        let line_y = y + piece.font_height - ((piece.font_height as f32 / 10f32).floor() as i32 + 1);
                        draw_line(piece.x, line_y, piece.x + piece.w - 4, line_y);
                    }

                    // 绘制文本，使用draw_text_n()函数可以正确渲染'@'字符而无需转义处理。
                    // draw_text2(piece.line.as_str(), piece.x, y + bg_offset, piece.w, piece.h, Align::Left);
                    draw_text_n(text, piece.x, y + bg_offset + self.font_size);

                    if self.strike_through {
                        // 绘制删除线
                        let line_y = y + ((piece.font_height as f32 / 2f32).floor() as i32);
                        draw_line(piece.x, line_y, piece.x + piece.w - 4, line_y);
                    }
                }
            },
            DataType::Image => {
                if let Some(piece) = self.line_pieces.last() {
                    let piece = &*piece.borrow();
                    if !self.disabled {
                        if !self.blink || blink_state.next == BlinkDegree::Normal {
                            if let Some(img) = &self.image {
                                if let Err(e) = draw_image(img.as_slice(), piece.x, piece.y - offset_y, piece.w, piece.h, ColorDepth::Rgb8) {
                                    error!("draw image error: {:?}", e);
                                }
                            }
                        }
                    } else {
                        if !self.blink || blink_state.next == BlinkDegree::Normal {
                            if let Some(img) = &self.image_inactive {
                                if let Err(e) = draw_image(img.as_slice(), piece.x, piece.y - offset_y, piece.w, piece.h, ColorDepth::L8) {
                                    error!("draw image error: {:?}", e);
                                }
                            }
                        }
                    }
                }

            },
        }
    }

    /// 试算当前内容绘制后所占高度信息。
    /// 试算功能自动处理文本超宽时截断换行的逻辑。
    ///
    /// # Arguments
    ///
    /// * `last_piece`: 前一个数据片段，用于计算当前数据段的绘制坐标。每个数据段和数据片段都是按照缓存数据的顺序依次计算得到。
    /// * `max_width`: 可视区域最大宽度，不含padding宽度。
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn estimate(&mut self, last_piece: Rc<RefCell<LinePiece>>, max_width: i32) -> Rc<RefCell<LinePiece>> {
        let mut ret = last_piece.clone();
        let last_piece = last_piece.borrow().clone();
        let (top_y, start_x) = (last_piece.next_y, last_piece.next_x);
        let (font, font_size) = (self.font, self.font_size);
        self.line_pieces.clear();
        match self.data_type {
            DataType::Text => {
                set_font(self.font, self.font_size);

                // 字体渲染高度，小于等于行高度。
                let ref_font_height = (self.font_size as f32 * 1.4).ceil() as i32;

                let current_line_spacing = min(last_piece.spacing, descent());

                /*
                对含有换行符和不含换行符的文本进行不同处理。
                 */
                let text = self.text.replace("\r", "");
                if text.contains('\n') {
                    // 以换行符为节点拆分成多段处理。
                    for line in text.split_inclusive("\n") {
                        let (tw, th) = measure(line, false);
                        let mut current_line_height = max(ref_font_height, th);
                        self.line_height = current_line_height;

                        let mut next_x = last_piece.next_x + tw;
                        if next_x > max_width {
                            // 超出横向右边界
                            ret = self.wrap_text_for_estimate(line, ret.clone(), max_width, tw, ref_font_height);
                        } else {
                            let new_piece: Rc<RefCell<LinePiece>>;
                            if let Some(lp) = self.line_pieces.last_mut() {
                                let lp = &mut *lp.borrow_mut();
                                let mut next_y = lp.next_y;
                                // 最后一段可能带有换行符'\n'。
                                if line.ends_with("\n") {
                                    next_y += current_line_height;
                                    next_x = PADDING.left;
                                }
                                let y = lp.next_y;
                                let piece_top_y = lp.next_y;
                                let through_line = ThroughLine::create_or_update(PADDING.left, lp.next_x, current_line_height, ret.clone(), false);
                                new_piece = LinePiece::new(line.to_string(), lp.next_x, y, tw, current_line_height, piece_top_y, lp.spacing, next_x, next_y, ref_font_height, font, font_size, through_line, self.v_bounds.clone());

                            } else {
                                let mut next_y = last_piece.next_y;
                                // 最后一段可能带有换行符'\n'。
                                if line.ends_with("\n") {
                                    if !last_piece.line.ends_with("\n") && !last_piece.line.is_empty() {
                                        current_line_height = max(current_line_height, last_piece.h);
                                    }
                                    next_y += current_line_height;
                                    next_x = PADDING.left;
                                }
                                let y = last_piece.next_y;
                                let piece_top_y = last_piece.next_y;
                                let through_line = ThroughLine::create_or_update(PADDING.left, last_piece.next_x, current_line_height, ret.clone(), false);
                                new_piece = LinePiece::new(line.to_string(), last_piece.next_x, y, tw, self.line_height, piece_top_y, last_piece.spacing, next_x, next_y, ref_font_height, font, font_size, through_line, self.v_bounds.clone());
                            }
                            self.line_pieces.push(new_piece.clone());
                            ret = new_piece;
                        }
                    }

                } else {
                    let (_, th) = measure("A", false);
                    self.line_height = max(ref_font_height, th);

                    let line = text.as_str();
                    let (tw, _) = measure(line, false);
                    if last_piece.next_x + tw > max_width {
                        // 超出横向右边界
                        ret = self.wrap_text_for_estimate(line, ret.clone(), max_width, tw, ref_font_height);
                    } else {
                        let y = top_y;
                        let through_line = ThroughLine::create_or_update(PADDING.left, start_x, ref_font_height, ret, false);
                        let new_piece = LinePiece::new(self.text.clone(), start_x, y, tw, ref_font_height, top_y, current_line_spacing, start_x + tw + self.piece_spacing, top_y, ref_font_height, font, font_size, through_line, self.v_bounds.clone());
                        self.line_pieces.push(new_piece.clone());
                        ret = new_piece;
                    }
                }
            }
            DataType::Image => {
                let h = self.image_height + IMAGE_PADDING_V * 2;
                if start_x + self.image_width > max_width {
                    // 本行超宽，直接定位到下一行
                    let x = PADDING.left + IMAGE_PADDING_H;
                    let y = top_y + last_piece.through_line.borrow().max_h + IMAGE_PADDING_V;
                    let next_x = x + self.image_width + IMAGE_PADDING_H;
                    let next_y = y - IMAGE_PADDING_V;
                    let piece_top_y = y - IMAGE_PADDING_V;
                    let through_line = ThroughLine::new(self.image_height * IMAGE_PADDING_V * 2, true);
                    let new_piece = LinePiece::new("".to_string(), x, y, self.image_width, self.image_height, piece_top_y, last_piece.spacing, next_x, next_y, 1, font, font_size, through_line, self.v_bounds.clone());
                    self.line_pieces.push(new_piece.clone());
                    ret = new_piece;
                } else {
                    let x = start_x + IMAGE_PADDING_H;
                    let next_x = start_x + self.image_width + IMAGE_PADDING_H * 2 + self.piece_spacing;
                    if last_piece.line.ends_with("\n") {
                        // 定位在行首
                        let y = top_y + IMAGE_PADDING_V;
                        let piece_top_y = y - IMAGE_PADDING_V;
                        let through_line = ThroughLine::new(self.image_height * IMAGE_PADDING_V * 2, true);
                        let new_piece = LinePiece::new("".to_string(), x, y, self.image_width, self.image_height, piece_top_y, last_piece.spacing, next_x, top_y, 1, font, font_size, through_line, self.v_bounds.clone());
                        self.line_pieces.push(new_piece.clone());
                        ret = new_piece;
                    } else {
                        // 在本行已有其他内容，需要与前一个片段协调行高
                        let current_line_height = max(last_piece.h, h);
                        let mut raw_y = top_y + IMAGE_PADDING_V;
                        if current_line_height > last_piece.h {
                            // 图形比前一个分片行高要高
                            last_piece.through_line.borrow_mut().set_max_h(current_line_height);
                        } else {
                            // 图形的高度小于等于前一个分片的行高，需要计算垂直居中位置
                            let (up, _) = calc_v_center_offset(current_line_height, h);
                            raw_y += up;
                        }
                        let y = raw_y;
                        let piece_top_y = y - IMAGE_PADDING_V;
                        let through_line = ThroughLine::create_or_update(PADDING.left + IMAGE_PADDING_H, x, self.image_height * IMAGE_PADDING_V * 2, ret, true);
                        let new_piece = LinePiece::new("".to_string(), x, y, self.image_width, self.image_height, piece_top_y, last_piece.spacing, next_x, top_y + IMAGE_PADDING_V, 1, font, font_size, through_line, self.v_bounds.clone());
                        self.line_pieces.push(new_piece.clone());
                        ret = new_piece;
                    }
                }
            }
        }

        let (mut _is_first_line, mut bound_start_x, mut bound_end_x) = (true, 0, 0);
        let mut to_be_updated: Vec<(Rc<RefCell<LinePiece>>, i32)> = Vec::new();
        for line_piece in self.line_pieces.iter() {
            let lp = &*line_piece.borrow();
            if _is_first_line {
                bound_start_x = lp.x;
                _is_first_line = false;
            }

            let tl = &mut *lp.through_line.borrow_mut();
            let ys = &*tl.ys.borrow_mut();
            let mut max_h = 1;
            // 找出最大的行高
            for l in ys.iter() {
                if let Some(l) = l.upgrade() {
                    if l.borrow().h > max_h {
                        max_h = l.borrow().h;
                    }
                }
            }
            tl.max_h = max_h;
            // 收集同一行内低于最大高度的分片。因为borrow作用域的问题，无法在一个for循环内直接处理，只能先收集再处理。
            for one_piece in ys.iter() {
                if let Some(p) = one_piece.upgrade() {
                    let lh = p.borrow().h;
                    if lh < max_h {
                        to_be_updated.push((p.clone(), max_h));
                    }
                }
            }
        }
        // 重新计算同一行内低于最大行高的片段的y坐标
        for (lp, max_h) in to_be_updated {
            let y = lp.borrow().y;
            let piece_top_y = lp.borrow().top_y;
            let h = lp.borrow().h;

            if lp.borrow().line.ends_with("\n") {
                let mut padding_v = 0;
                if lp.borrow().through_line.borrow().exist_image {
                    padding_v = IMAGE_PADDING_V;
                }
                lp.borrow_mut().next_y = y + max_h + padding_v;
            }

            let (up_offset, _) = calc_v_center_offset(max_h, h);
            lp.borrow_mut().y = piece_top_y + up_offset;
            let lpm = &mut*lp.borrow_mut();
            let mut vb = lpm.rd_bounds.get();
            vb.1 = piece_top_y + up_offset + lpm.h;
            lpm.rd_bounds.set(vb);
        }

        // let mut pic_y = 0;
        let v_b_top_y = if let Some(first_piece) = self.line_pieces.first() {
            let fp = &*first_piece.borrow();
            fp.top_y
            // pic_y = fp.y;
        } else {
            top_y
        };
        let v_b_bottom_y = if let Some(last_piece) = self.line_pieces.last() {
            let lp = &*last_piece.borrow();
            let bottom_y = lp.top_y + lp.through_line.borrow().max_h;
            bound_end_x = lp.x + lp.w;
            bottom_y
        } else {
            v_b_top_y
        };
        // debug!("estimated v_b_top_y: {v_b_top_y}, v_b_bottom_y: {v_b_bottom_y}, bound_start_x: {bound_start_x}, bound_end_x: {bound_end_x}, text: {}", self.text);
        self.set_v_bounds(v_b_top_y, v_b_bottom_y, bound_start_x, bound_end_x);
        ret
    }
}

/// 数据片段调整属性。
#[derive(Debug, Clone)]
pub struct RichDataOptions {
    pub id: i64,
    pub clickable: Option<bool>,
    pub underline: Option<bool>,
    pub expired: Option<bool>,
    pub text: Option<String>,
    pub fg_color: Option<Color>,
    pub bg_color: Option<Color>,
    pub strike_through: Option<bool>,
    pub blink: Option<bool>,
    pub disabled: Option<bool>
}

impl RichDataOptions {
    pub fn new(id: i64) -> RichDataOptions {
        RichDataOptions {
            id,
            clickable: None,
            underline: None,
            expired: None,
            text: None,
            fg_color: None,
            bg_color: None,
            strike_through: None,
            blink: None,
            disabled: None
        }
    }

    pub fn clickable(mut self, clickable: bool) -> RichDataOptions {
        self.clickable = Some(clickable);
        self
    }

    pub fn underline(mut self, underline: bool) -> RichDataOptions {
        self.underline = Some(underline);
        self
    }

    pub fn expired(mut self, expired: bool) -> RichDataOptions {
        self.expired = Some(expired);
        self
    }

    pub fn text(mut self, text: String) -> RichDataOptions {
        self.text = Some(text);
        self
    }

    pub fn fg_color(mut self, fg_color: Color) -> RichDataOptions {
        self.fg_color = Some(fg_color);
        self
    }

    pub fn bg_color(mut self, bg_color: Color) -> RichDataOptions {
        self.bg_color = Some(bg_color);
        self
    }

    pub fn strike_through(mut self, strike_through: bool) -> RichDataOptions {
        self.strike_through = Some(strike_through);
        self
    }

    pub fn blink(mut self, blink: bool) -> RichDataOptions {
        self.blink = Some(blink);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> RichDataOptions {
        self.disabled = Some(disabled);
        self
    }
}

/// 碰撞检测，检查两个矩形区域是否出现交叉。
///
/// # Arguments
///
/// * `a`: 目标区域。
/// * `b`: 当前区域，或者鼠标拖拽选择区域。
///
/// returns: bool
///
/// # Examples
///
/// ```
///
/// ```
pub(crate) fn is_overlap(target_area: &Rectangle, selection_area: &Rectangle) -> bool {
    target_area.0 < (selection_area.0 + selection_area.2) && (target_area.0 + target_area.2) > selection_area.0 && target_area.1 < (selection_area.1 + selection_area.3) && (target_area.1 + target_area.3) > selection_area.1
}


/// 复制选中片段的内容。
///
/// # Arguments
///
/// * `it`:
/// * `selection`:
///
/// returns: ()
///
/// # Examples
///
/// ```
///
/// ```
fn copy_pieces(it: Iter<Weak<RefCell<LinePiece>>>, selection: &mut String) {
    for p in it {
        if let Some(p) = p.upgrade() {
            let lp = &*p.borrow();
            lp.copy_selection(selection);
        }
    }
}

/// 清除数据片段的选中属性。
///
/// # Arguments
///
/// * `selected_pieces`:
///
/// returns: ()
///
/// # Examples
///
/// ```
///
/// ```
pub(crate) fn clear_selected_pieces(selected_pieces: Rc<RefCell<Vec<Weak<RefCell<LinePiece>>>>>) {
    for piece in selected_pieces.borrow().iter() {
        if let Some(p) = piece.upgrade() {
            p.borrow().deselect();
        }
    }
    selected_pieces.borrow_mut().clear();
}

/// 向前或向后选择数据片段。
///
/// # Arguments
///
/// * `rd`:
/// * `piece_index`:
/// * `pos`:
/// * `selected_pieces`:
/// * `from`:
///
/// returns: ()
///
/// # Examples
///
/// ```
///
/// ```
fn select_piece_from_or_to(rd: &RichData, piece_index: usize, pos: usize, selected_pieces: Rc<RefCell<Vec<Weak<RefCell<LinePiece>>>>>, from: bool) {
    if let Some(last_piece_rc) = rd.line_pieces.get(piece_index) {
        let piece = &*last_piece_rc.borrow();
        if from {
            piece.select_from(pos);
        } else {
            piece.select_to(pos);
        }
        selected_pieces.borrow_mut().push(Rc::downgrade(last_piece_rc));
    }
}

/// 计算选中文本所在数据片段的选中属性。
///
/// # Arguments
///
/// * `from_point`: 起始位置。
/// * `to_point`: 结束位置。
/// * `data_buffer`: 数据缓存。
/// * `rd_range`: 选中的数据段索引范围。
/// * `selected_pieces`: 选中数据片段临时记录容器。
///
/// returns: ()
///
/// # Examples
///
/// ```
///
/// ```
pub(crate) fn select_text(from_point: &ClickPoint, to_point: &ClickPoint, data_buffer: &[RichData], rd_range: RangeInclusive<usize>, selected_pieces: Rc<RefCell<Vec<Weak<RefCell<LinePiece>>>>>) {
    /*
    选择片段的原则：应选择起点右下方的第一行片段，结束点左上方的第一行片段，以及两点之间的中间行片段。
     */
    // debug!("传入的fp: {:?}, tp: {:?}", from_point, to_point);
    let drag_rect = from_point.to_rect(to_point);
    let (lt, br) = drag_rect.corner_rect();
    let (mut lt_p, mut br_p) = (from_point, to_point);
    if (br.0 == from_point.x && br.1 == from_point.y) || (lt.0 == from_point.x && br.1 == from_point.y) {
        // debug!("对换坐标点");
        lt_p = to_point;
        br_p = from_point;
    };
    let (f_p_i, t_p_i) = (lt_p.p_i, br_p.p_i);

    // debug!("f_p_i: {f_p_i}, t_p_i: {t_p_i}, rd_range: {:?}, drag_rect: {:?}, lt: {:?}, br: {:?}", rd_range, drag_rect, lt, br);
    // 清理上一次选择的区域
    clear_selected_pieces(selected_pieces.clone());

    let (r_start, r_end) = (*rd_range.start(), *rd_range.end());
    let across_rds = r_end - r_start;
    if across_rds > 0 {
        // 超过一行
        // debug!("选区超过一个数据段");
        if let Some(rd) = data_buffer.get(r_start) {
            // 选择第一个数据段起点之后的所有分片内容。
            select_piece_from_or_to(rd, f_p_i, lt_p.c_i, selected_pieces.clone(), true);
            for p in rd.line_pieces.iter().skip(f_p_i + 1) {
                let piece = &*p.borrow();
                piece.select_all();
                selected_pieces.borrow_mut().push(Rc::downgrade(p));
            }
        }

        // 如果中间有更多跨行数据段，则全选这些数据段。
        let mut piece_rcs = Vec::new();
        for i in r_start + 1..r_end {
            if let Some(rd) = data_buffer.get(i) {
                for p in rd.line_pieces.iter() {
                    let piece = &*p.borrow();
                    piece.select_all();
                    piece_rcs.push(Rc::downgrade(p));
                }
            }
        }
        selected_pieces.borrow_mut().append(&mut piece_rcs);

        if let Some(rd) = data_buffer.get(r_end) {
            // 选择最后一个数据段终点之前的所有内容。
            for p in rd.line_pieces.iter().take(t_p_i) {
                let piece = &*p.borrow();
                piece.select_all();
                selected_pieces.borrow_mut().push(Rc::downgrade(p));
            }
            select_piece_from_or_to(rd, t_p_i, br_p.c_i + 1, selected_pieces.clone(), false);
        }
    } else {
        // 只有一行
        // debug!("选区只有一个数据段");
        if let Some(rd) = data_buffer.get(r_start) {
            let across_pieces = if t_p_i > f_p_i {t_p_i - f_p_i} else {0};
            if across_pieces > 0 {
                // 超过一个分片
                // debug!("选区超过一个分片");
                select_piece_from_or_to(rd, f_p_i, lt_p.c_i, selected_pieces.clone(), true);

                // 超过两个分片
                // debug!("选区超过两个分片");
                let mut piece_rcs = Vec::new();
                for i in f_p_i + 1..t_p_i {
                    if let Some(piece_rc) = rd.line_pieces.get(i) {
                        let piece = &*piece_rc.borrow();
                        piece.select_all();
                        piece_rcs.push(Rc::downgrade(piece_rc));
                    }
                }
                selected_pieces.borrow_mut().append(&mut piece_rcs);

                select_piece_from_or_to(rd, t_p_i, br_p.c_i + 1, selected_pieces.clone(), false);
            } else {
                // 在同一个分片内
                if let Some(piece_rc) = rd.line_pieces.get(f_p_i) {
                    // debug!("selected range from: {} to: {}", lt_p.c_i, br_p.c_i + 1);
                    let (mut fci, mut tci) = (lt_p.c_i, br_p.c_i + 1);
                    if fci >= tci {
                        fci = br_p.c_i;
                        tci = lt_p.c_i + 1;
                    }
                    piece_rc.borrow().select_range(fci, tci);
                    selected_pieces.borrow_mut().push(Rc::downgrade(piece_rc));
                }
            }
        }
    }

    /*
    拷贝至剪贴板
     */
    let mut selection = String::new();
    copy_pieces(selected_pieces.borrow().iter(), &mut selection);
    app::copy(selection.as_str());
}

/// 检测拖选范围所涵盖的数据段。
///
/// # Arguments
///
/// * `point`: 起始点。
/// * `drag_rect`: 拖选矩形范围。
/// * `panel_width`: 容器面板宽度。
/// * `data_buffer`: 数据缓存。
/// * `index_vec`: 容器面板可见范围内数据的顺序位置索引。
///
/// returns: Option<usize> 返回拖选结束点的数据段索引。
///
/// # Examples
///
/// ```
///
/// ```
pub(crate) fn locate_target_rd(point: &mut ClickPoint, drag_rect: Rectangle, panel_width: i32, data_buffer: &[RichData], index_vec: Vec<usize>) -> Option<usize> {
    let point_rect = point.as_rect();
    // debug!("index_vec: {:?}", index_vec);
    if let Ok(idx) = index_vec.binary_search_by({
        let point_rect_rc = point_rect.clone();
        let point_rc = point.clone();
        move |row| {
            /*
            先将不规则的数据段外形扩展为顶宽的矩形，再检测划选区是否与之重叠；
            如果有重叠，则进一步检测其中每个分片是否与划选区有重叠，如果任意分片有重叠，说明划选区包含了该数据段的某些分片，
            还须再进一步确定选区起点位置所在的分片，最终返回等于，否则返回大于或小于；
            如果没有重叠，则判断其相对位置，并返回大于或小于。
             */
            let mut rd_extend_rect = Rectangle::zero();
            let rd = &data_buffer[*row];
            // debug!("检测行 {row} : {}", rd.text);
            let (rd_top_y, rd_bottom_y, _, _) = rd.v_bounds.get();
            rd_extend_rect.replace(0, rd_top_y, panel_width, rd_bottom_y - rd_top_y);
            // debug!("rd_top_y: {}, rd_bottom_y: {}, drag_rect: {:?}", rd_top_y, rd_bottom_y, drag_rect);

            // 粗略过滤到的数据段，还须进一步检测其中的分片是否包含划选区起点。
            if is_overlap(&rd_extend_rect, &drag_rect) {
                // debug!("行 {row} 与划选区有重叠");
                let mut ord = Ordering::Less;
                for piece_rc in rd.line_pieces.iter() {
                    let piece = &*piece_rc.borrow();
                    let piece_rect = piece.rect(0);
                    // debug!("piece_rect: {:?}, piece_top_y: {}, : {}", piece_rect, piece.top_y, piece.line);
                    if is_overlap(&piece_rect, &point_rect_rc) {
                        // 划选区起点位于分片内
                        // debug!("划选区起点位于分片内：{}", piece.line);
                        ord = Ordering::Equal;
                        break;
                    }
                }
                // 如果划选起点不在重叠数据段的任意分片内部，则还须判断当前数据段在起点的前面或后面，为查找算法提供判断依据。
                if ord != Ordering::Equal {
                    if let Some(first_piece_rc) = rd.line_pieces.first() {
                        let piece = &*first_piece_rc.borrow();
                        // debug!("piece: {:?}", piece);
                        if point_rc.x < piece.x && point_rc.y < piece.top_y + piece.through_line.borrow().max_h {
                            ord = Ordering::Greater;
                        } else {
                            ord = Ordering::Less;
                        }
                    }
                }
                // debug!("行 {row}: ord: {:?}", ord);
                ord
            } else {
                if rd_extend_rect.is_below(&drag_rect) {
                    // debug!("行 {row}: 大于");
                    Ordering::Greater
                } else {
                    // debug!("行 {row}: 小于");
                    Ordering::Less
                }
            }
        }
    }) {
        let rd = &data_buffer[index_vec[idx]];
        if rd.data_type != DataType::Image {
            // debug!("找到目标点所在数据段： {}", rd.text);
            for (p_i, piece_rc) in rd.line_pieces.iter().enumerate() {
                let piece = &*piece_rc.borrow();
                let piece_rect = piece.rect(0);
                // debug!("point_rect: {:?}, piece_rect: {:?}, line: {}", point_rect, piece_rect, piece.line);
                if is_overlap(&piece_rect, &point_rect) {
                    // 划选区起点位于分片内
                    point.p_i = p_i;
                    // debug!("目标点位于分片:{} 内: {}", p_i, piece.line);
                    search_index_of_piece(piece, point);
                    break;
                }
            }

            return Some(idx);
        } else {
            // 选择了图片，不予处理。
            // debug!("选择了图片")
        }

    } else {
        // debug!("没找到目标数据段！")
    }
    None
}

/// 更新拖选结束点所在数段的位置属性，用于拖选过程准实时检测结束点位置。
///
/// # Arguments
///
/// * `push_from_point`: 起始点。
/// * `select_from_row`: 起始点所在数据段的顺序索引号。
/// * `current_point`: 当前结束点。
/// * `data_buffer_slice`: 数据缓存。
/// * `selected_pieces`: 临时保存选中数据片段的容器。
/// * `panel`: 当前容器面板。
///
/// returns: bool
///
/// # Examples
///
/// ```
///
/// ```
#[throttle(1, Duration::from_millis(50))]
pub(crate) fn update_selection_when_drag(
    push_from_point: ClickPoint,
    select_from_row: usize,
    current_point: &mut ClickPoint,
    data_buffer_slice: &[RichData],
    selected_pieces: Rc<RefCell<Vec<Weak<RefCell<LinePiece>>>>>,
    panel: &mut Widget,) -> bool {

    let mut down = true;
    let index_vec = if current_point.y >= push_from_point.y {
        // 向下选择
        (select_from_row..data_buffer_slice.len()).collect::<Vec<usize>>()
    } else {
        // 向上选择
        down = false;
        (0..=select_from_row).collect::<Vec<usize>>()
    };
    // debug!("开始查找结束点所在数据段: {:?}", index_vec);
    if let Some(select_to_row) = locate_target_rd(current_point, current_point.as_rect(), panel.w(), data_buffer_slice, index_vec) {
        let rd_range = if down {
            select_from_row..=(select_from_row + select_to_row)
        } else {
            select_to_row..=select_from_row
        };
        select_text(&push_from_point, current_point, data_buffer_slice, rd_range, selected_pieces);
        // debug!("push_from: {:?}, current_point: {:?}", push_from_point, current_point);
        panel.set_damage(true);
        true
    } else {
        false
    }
}


/// 测量鼠标点击的片段内容字符索引位置。
///
/// # Arguments
///
/// * `piece`:
/// * `point`:
///
/// returns: ()
///
/// # Examples
///
/// ```
///
/// ```
pub(crate) fn search_index_of_piece(piece: &LinePiece, point: &mut ClickPoint) {
    let len = piece.line.chars().count();
    if let Ok(c_i) = (0..len).collect::<Vec<usize>>().binary_search_by({
        set_font(piece.font, piece.font_size);
        let text = piece.line.clone();
        let x = point.x;
        let start_x = piece.x;
        move |pos| {
            let (mut pw1, _) = measure(text.chars().take(*pos + 1).collect::<String>().as_str(), false);
            let (mut pw2, _) = measure(text.chars().take(*pos).collect::<String>().as_str(), false);
            pw1 += start_x;
            pw2 += start_x;
            if x > pw2 && x <= pw1 {
                Ordering::Equal
            } else if x <= pw2 {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        }
    }) {
        point.c_i = c_i;
        // debug!("目标字符：{}，位置：{}", piece.line.chars().nth(c_i).unwrap(), c_i);
    } else {
        // debug!("没找到目标字符！")
    }
}

/// 获取指定颜色的对比色。若指定颜色为中等灰色(R/G/B值相等且在116-139之间)，则返回白色。
///
/// # Arguments
///
/// * `color`: 指定颜色。
///
/// returns: Color 返回其对比色。
///
/// # Examples
///
/// ```
/// use fltk::enums::Color;
/// use fltkrs_richdisplay::get_contrast_color;
///
/// assert_eq!(get_contrast_color(Color::from_rgb(255, 255, 255)), Color::from_rgb(0, 0, 0));
/// assert_eq!(get_contrast_color(Color::from_rgb(0, 0, 0)), Color::from_rgb(255, 255, 255));
/// assert_eq!(get_contrast_color(Color::from_rgb(255, 0, 0)), Color::from_rgb(0, 255, 255));
/// assert_eq!(get_contrast_color(Color::from_rgb(120, 120, 120)), Color::from_rgb(255, 255, 255));
/// ```
pub fn get_contrast_color(color: Color) -> Color {
    let (r, g, b) = color.to_rgb();
    let (cr, cg, cb) = (255 - r, 255 - g, 255 - b);
    if (cr == cg && cg == cb) && ((cr as i16) - (r as i16)).abs() < 25 {
        WHITE
    } else {
        Color::from_rgb(cr, cg, cb)
    }
}

/// 获取指定颜色的亮色或暗色，若指定颜色的R/G/B值其中最大的超过128，则获取暗色，否则获取亮色。
///
/// # Arguments
///
/// * `color`: 指定颜色。
///
/// returns: Color 返回对应的亮色或暗色。
///
/// # Examples
///
/// ```
///
/// ```
pub fn get_lighter_or_darker_color(color: Color) -> Color {
    let (r, g, b) = color.to_rgb();

    let total = r as u16 + g as u16 + b as u16;
    let max_c = max(r, max(g, b));
    if total >= 383 || max_c as u16 + 127 > 255u16 {
        // 当三原色合计值超过最大合计值的一半时，或者某项原色值超过128，降低各原色数值。效果是变暗。
        let (cr, cg, cb) = (max(0i16, r as i16 - 127), max(0i16, g as i16 - 127), max(0i16, b as i16 - 127));
        Color::from_rgb(cr as u8, cg as u8, cb as u8)
    } else {
        // 当三原色合计值小于最大合计值的一半时，提高各原色数值。效果是变亮。
        let (cr, cg, cb) = (min(255i16, r as i16 + 127), min(255i16, g as i16 + 127), min(255i16, b as i16 + 127));
        Color::from_rgb(cr as u8, cg as u8, cb as u8)
    }
}

#[cfg(test)]
mod tests {
    use fltk::enums::Color;
    use crate::{get_contrast_color, get_lighter_or_darker_color, WHITE, Rectangle};

    #[test]
    pub fn make_rectangle_test() {
        let rect = Rectangle::new(20, 30, 25, 25);
        assert_eq!(rect.tup(), (20, 30, 25, 25));

        let rect = Rectangle::new(20, 30, -10, 25);
        assert_eq!(rect.tup(), (10, 30, 10, 25));

        let rect = Rectangle::new(20, 30, 10, -25);
        assert_eq!(rect.tup(), (20, 5, 10, 25));

        let rect = Rectangle::new(20, 30, -10, -25);
        assert_eq!(rect.tup(), (10, 5, 10, 25));
    }

    #[test]
    pub fn str_index_test() {
        let str = String::from("我爱中国");
        assert_eq!(str.find("中国"), Some(6));

        str.rmatch_indices("中国").for_each(|(i, _)| {
            let ni = str[0..i].chars().count();
            assert_eq!(ni, 2);
        })
    }

    #[test]
    pub fn test_contrast_color_test() {
        assert_eq!(get_contrast_color(Color::from_rgb(255, 255, 255)), Color::from_rgb(0, 0, 0));
        assert_eq!(get_contrast_color(Color::from_rgb(0, 0, 0)), Color::from_rgb(255, 255, 255));
        for i in 1..116 {
            assert_ne!(get_contrast_color(Color::from_rgb(i, i, i)), WHITE);
        }
        for i in 116..=139 {
            assert_eq!(get_contrast_color(Color::from_rgb(i, i, i)), WHITE);
        }
        for i in 140..=255 {
            assert_ne!(get_contrast_color(Color::from_rgb(i, i, i)), WHITE);
        }
    }

    #[test]
    pub fn get_lighter_color_test() {
        let lighter = get_lighter_or_darker_color(Color::DarkCyan);
        println!("{:?} -> {:?}", Color::DarkCyan.to_rgb(), lighter.to_rgb())
    }

    #[test]
    pub fn emoji_test() {
        let emoji = "😀";
        println!("{:?}", emoji.chars().nth(0).unwrap());
        assert_eq!(emoji.len(), 1);
    }
}
