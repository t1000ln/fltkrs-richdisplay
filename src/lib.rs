use std::cell::{Cell, RefCell};
use std::cmp::{max, min, Ordering};
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{RangeInclusive};
use std::rc::{Rc, Weak};
use std::slice::Iter;
use fltk::{app, draw};
use fltk::draw::{descent, draw_image, draw_line, draw_rect_with_color, draw_rectf, draw_text2, measure, set_draw_color, set_font};
use fltk::enums::{Align, Color, ColorDepth, Cursor, Font};
use fltk::prelude::{FltkError, FltkErrorKind};
use fltk_sys::draw::Fl_draw_image_mono;

use idgenerator_thin::YitIdHelper;
use log::{debug, error};

pub mod rich_text;
pub mod rich_reviewer;

/// 默认内容边界到窗口之间的空白距离。
pub const PADDING: Padding = Padding { left: 5, top: 5, right: 5, bottom: 5 };

/// 图片与其他内容之间的垂直间距。
pub const IMAGE_PADDING_H: i32 = 2;

/// 图片与其他内容之间的水平间距。
pub const IMAGE_PADDING_V: i32 = 2;

/// 同一行内多个文字分片之间的水平间距。
pub const PIECE_SPACING: i32 = 2;

pub const BLINK_INTERVAL: f64 = 0.5;

pub const HIGHLIGHT_BACKGROUND_COLOR: Color = Color::from_rgb(0, 0, 255);
pub const HIGHLIGHT_RECT_COLOR: Color = Color::from_rgb(255, 145, 0);
pub const HIGHLIGHT_RECT_CONTRAST_COLOR: Color = Color::from_rgb(0, 110, 255);
pub const HIGHLIGHT_WHITE: Color = Color::from_rgb(255, 255, 255);

#[derive(Debug, Clone,Copy, PartialEq, Eq)]
pub enum BlinkDegree {
    Normal,
    Contrast,
}

#[derive(Debug, Clone,Copy, PartialEq, Eq)]
pub struct BlinkState {
    on: bool,
    next: BlinkDegree,
}

impl BlinkState {
    pub fn new() -> BlinkState {
        BlinkState {
            on: false,
            next: BlinkDegree::Normal,
        }
    }

    pub fn is_on(&self) -> bool {
        self.on
    }

    pub fn off(mut self) -> Self {
        self.on = false;
        self.next = BlinkDegree::Normal;
        self
    }

    pub fn on(mut self) -> Self {
        self.on = true;
        // self.next = BlinkDegree::Darker;
        self
    }

    pub fn toggle_when_on(mut self) -> (bool, Self) {
        if self.on {
            self.next = match self.next {
                BlinkDegree::Normal => BlinkDegree::Contrast,
                BlinkDegree::Contrast => BlinkDegree::Normal,
            };
            (true, self)
        } else {
            (false, self)
        }
    }
}

/// 自定义事件。
pub struct LocalEvent;
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
pub struct Rectangle(i32, i32, i32, i32);

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
    /// use fltkrs_richdisplay::Rectangle;
    ///
    /// let rect = Rectangle::new(20, 30, 25, 25);
    /// assert_eq!(rect.tup(), (20, 30, 25, 25));
    ///
    /// let rect = Rectangle::new(20, 30, -10, 25);
    /// assert_eq!(rect.tup(), (10, 30, 10, 25));
    ///
    /// let rect = Rectangle::new(20, 30, 10, -25);
    /// assert_eq!(rect.tup(), (20, 5, 10, 25));
    ///
    /// let rect = Rectangle::new(20, 30, -10, -25);
    /// assert_eq!(rect.tup(), (10, 5, 10, 25));
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

    /// 获得当前矩形左上角和右下角的坐标。
    pub fn corner(&self) -> (ClickPoint, ClickPoint) {
        (ClickPoint::new(self.0, self.1), ClickPoint::new(self.0 + self.2, self.1 + self.3))
    }

    /// 获得当前矩形左上角和右下角的空矩形。
    pub fn corner_rect(&self) -> (Self, Self) {
        (ClickPoint::new(self.0, self.1).as_rect(), ClickPoint::new(self.0 + self.2, self.1 + self.3).as_rect())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ClickPoint {
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
    /// use fltkrs_richdisplay::ClickPoint;
    /// let from_point = ClickPoint::new(120, 50);
    /// let to_point = ClickPoint::new(60, 150);
    /// let rect = from_point.to_rect(&to_point);
    /// let t = rect.tup();
    /// assert_eq!(t.0, 60);
    /// assert_eq!(t.1, 50);
    /// ```
    pub fn to_rect(&self, to_point: &Self) -> Rectangle {
        Rectangle::new(self.x, self.y, to_point.x - self.x, to_point.y - self.y)
    }
    pub fn towards_down(&self, to_point: &Self) -> bool {
        self.y < to_point.y
    }
}

/// 同一行内多个分片之间共享的信息。通过Rc<RefCell<ThroughLine>>进行链接。
#[derive(Debug, Clone)]
pub struct ThroughLine {
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

    pub fn set_exist_image(&mut self, exist_image: bool) -> &mut Self {
        if exist_image == true {
            self.exist_image = true;
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

#[derive(Debug, Clone, Default)]
pub struct Padding {
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) right: i32,
    pub(crate) bottom: i32,
}

/// 单行文本的渲染参数，由试算过程得到。
/// 一个大段文本在试算过程中，可能被拆分为多个适配当前窗口宽度的单行文本片段，用于简化绘制过程的运算。
#[derive(Debug, Clone)]
pub struct LinePiece {
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

    pub fn eq2(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.w == other.w && self.h == other.h && self.line.eq(&other.line)
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

    pub fn selection_text(&self) -> Option<String> {
        if let Some((from, to)) = self.selected_range.get() {
            Some(self.line.chars().skip(from).take(to - from).collect::<String>())
        } else {
            None
        }
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

pub trait LinedData {
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
    /// * `view_bounds`:
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
    /// * `suggested`: 建议的绘制参考信息，包括起始x,y位置，行高和行间距。
    ///
    /// returns: LineCoord 返回给下一个绘制单元的参考信息，包含本次绘制的结束位置x，y坐标和本单元的行高、行间距。
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn draw(&self, offset_y: i32, blink_state: BlinkState);

    fn estimate(&mut self, blow_line: Rc<RefCell<LinePiece>>, max_width: i32) -> Rc<RefCell<LinePiece>>;

}
#[derive(Clone, Debug, PartialEq)]
pub enum DataType {
    Text,
    Image,
}

/// 用户提供的数据单元。
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
        }
    }
}

impl UserData {
    pub fn new_text(text: String) -> Self {
        Self {
            id: 0,
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
        }
    }

    pub fn new_image(image: Vec<u8>, width: i32, height: i32) -> Self {
        Self {
            id: 0,
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
        }
    }

    pub fn set_font(mut self, font: Font, size: i32) -> Self {
        self.font = font;
        self.font_size = size;
        self
    }

    pub fn set_fg_color(mut self, fg_color: Color) -> Self {
        self.fg_color = fg_color;
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

#[derive(Debug)]
pub enum BlinkRangeError {
    Overlap(usize, usize, usize),
    Reverse(usize, usize, usize),
}

impl Display for BlinkRangeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BlinkRangeError::Overlap(i, from, to) => {
                write!(f, "第 {} 个闪烁片段的起始位置 {} 不应小于等于前一个闪烁片段的结束位置 {} ！", i, from, to)
            }
            BlinkRangeError::Reverse(i, from, to) => {
                write!(f, "第 {} 个闪烁片段的起始位置 {} 不应小于等于结束位置 {} ！", i, from, to)
            }
        }
    }
}

impl Error for BlinkRangeError {}

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
pub fn calc_v_center_offset(line_height: i32, font_height: i32) -> (i32, i32) {
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
pub fn mouse_enter(visible_lines: Rc<RefCell<HashMap<Rectangle, usize>>>) -> bool {
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
pub fn update_data_properties(options: RichDataOptions, rd: &mut RichData) {
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
pub fn disable_data(rd: &mut RichData) {
    rd.set_clickable(false);
    draw::set_cursor(Cursor::Default);

    match rd.data_type {
        DataType::Image => {
            // if let Some(image) = rd.image.as_mut() {
            //     if let Ok(mut ni) = RgbImage::new(image.as_slice(), rd.image_width, rd.image_height, ColorDepth::Rgb8) {
            //         ni.inactive();
            //         image.clear();
            //         image.append(&mut ni.to_rgb_data());
            //     }
            // }
            // 改为新的封装方法：draw_image_mono，这里不做额外处理。
        }
        DataType::Text => {
            rd.strike_through = true;
        }
    }
}

/// 绘制信息单元。
#[derive(Debug, Clone)]
pub struct RichData {
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
    image: Option<Vec<u8>>,
    image_width: i32,
    image_height: i32,

    pub(crate) search_result_positions: Option<Vec<(usize, usize)>>,
    pub(crate) search_highlight_pos: Option<usize>,
}

impl From<UserData> for RichData {
    fn from(data: UserData) -> Self {
        match data.data_type {
            DataType::Text => {
                RichData {
                    id: YitIdHelper::next_id(),
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
                    search_result_positions: None,
                    search_highlight_pos: None,
                }
            },
            DataType::Image => {
                RichData {
                    id: YitIdHelper::next_id(),
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
                    search_result_positions: None,
                    search_highlight_pos: None,
                }
            }
        }
    }
}

impl RichData {
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
            let x = last_piece.next_x + PIECE_SPACING;
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
                let mut rest_next_x = rest_x + rest_width + PIECE_SPACING;
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

    pub fn truncate(&mut self, from: Option<usize>) {
        if let Some(from) = from {
            self.text.truncate(from);
        } else {
            self.text.clear();
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

    fn draw(&self, offset_y: i32, blink_state: BlinkState) {
        match self.data_type {
            DataType::Text => {
                let bg_offset = 1;
                let mut processed_search_len = 0usize;
                set_font(self.font, self.font_size);
                for piece in self.line_pieces.iter() {
                    let piece = &*piece.borrow();
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
                            HIGHLIGHT_RECT_COLOR
                        } else {
                            HIGHLIGHT_RECT_CONTRAST_COLOR
                        };
                        let pl = piece.line.chars().count();
                        let range = processed_search_len..(processed_search_len + pl);
                        pos_vec.iter().enumerate().for_each(|(pos_i, (pos_from, pos_to))| {
                            if range.contains(pos_from) {
                                let start_index_of_piece = pos_from - processed_search_len;
                                let (skip_width, _) = measure(piece.line.chars().take(start_index_of_piece).collect::<String>().as_str(), false);
                                let (fill_width, _) = measure(piece.line.chars().skip(start_index_of_piece).take(pos_to - pos_from).collect::<String>().as_str(), false);

                                set_draw_color(HIGHLIGHT_BACKGROUND_COLOR);
                                #[cfg(target_os = "linux")]
                                {
                                    draw_rectf(piece.x + skip_width, y - piece.spacing + 2, fill_width, piece.font_height);
                                    if let Some(h_i) = self.search_highlight_pos {
                                        if h_i == pos_i {
                                            draw_rect_with_color(piece.x + skip_width, y - piece.spacing + 2, fill_width, piece.font_height, rect_color);
                                        }
                                    }
                                }

                                #[cfg(not(target_os = "linux"))]
                                {
                                    draw_rectf(piece.x + skip_width, y - piece.spacing, fill_width, piece.font_height);
                                    if let Some(h_i) = self.search_highlight_pos {
                                        if h_i == pos_i {
                                            draw_rect_with_color(piece.x + skip_width, y - piece.spacing, fill_width, piece.font_height, rect_color);
                                        }
                                    }
                                }

                            } else if range.contains(pos_to) {
                                let (fill_width, _) = measure(piece.line.chars().take(pos_to - processed_search_len).collect::<String>().as_str(), false);

                                set_draw_color(HIGHLIGHT_BACKGROUND_COLOR);
                                draw_rectf(piece.x, y - piece.spacing, fill_width, piece.font_height);
                                if let Some(h_i) = self.search_highlight_pos {
                                    if h_i == pos_i {
                                        draw_rect_with_color(piece.x, y - piece.spacing, fill_width, piece.font_height, rect_color);
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

                    // 绘制文本
                    draw_text2(piece.line.as_str(), piece.x, y + bg_offset, piece.w, piece.h, Align::Left);

                    if self.strike_through {
                        // 绘制删除线
                        let line_y = y + ((piece.font_height as f32 / 2f32).floor() as i32);
                        draw_line(piece.x, line_y, piece.x + piece.w - 4, line_y);
                    }
                }
            },
            DataType::Image => {
                if !self.blink || blink_state.next == BlinkDegree::Normal {
                    if let Some(piece) = self.line_pieces.last() {
                        let piece = &*piece.borrow();
                        if let Some(img) = &self.image {
                            if !self.disabled {
                                if let Err(e) = draw_image(img.as_slice(), piece.x, piece.y - offset_y, piece.w, piece.h, ColorDepth::Rgb8) {
                                    error!("draw image error: {:?}", e);
                                }
                            } else {
                                if let Err(e) = draw_image_mono(img.as_slice(), piece.x, piece.y - offset_y, piece.w, piece.h, ColorDepth::Rgb8) {
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
    /// * `last_line`: 给定一个参考位置。
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
                        let new_piece = LinePiece::new(self.text.clone(), start_x, y, tw, ref_font_height, top_y, current_line_spacing, start_x + tw + PIECE_SPACING, top_y, ref_font_height, font, font_size, through_line, self.v_bounds.clone());
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
                    let piece_top_y = y;
                    let through_line = ThroughLine::new(self.image_height * IMAGE_PADDING_V * 2, true);
                    let new_piece = LinePiece::new("".to_string(), x, y, self.image_width, self.image_height, piece_top_y, last_piece.spacing, next_x, next_y, 1, font, font_size, through_line, self.v_bounds.clone());
                    self.line_pieces.push(new_piece.clone());
                    ret = new_piece;
                } else {
                    let x = start_x + IMAGE_PADDING_H;
                    let next_x = start_x + self.image_width + IMAGE_PADDING_H * 2 + PIECE_SPACING;
                    if last_piece.line.ends_with("\n") {
                        // 定位在行首
                        let y = top_y + IMAGE_PADDING_V;
                        let piece_top_y = y;
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
                        let piece_top_y = y;
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
        let mut top_y = top_y;
        if let Some(first_piece) = self.line_pieces.first() {
            let fp = &*first_piece.borrow();
            top_y = fp.top_y;
            // pic_y = fp.y;
        }
        let mut bottom_y = top_y;
        if let Some(last_piece) = self.line_pieces.last() {
            let lp = &*last_piece.borrow();
            bottom_y = lp.top_y + lp.through_line.borrow().max_h;
            bound_end_x = lp.x + lp.w;
        }
        // debug!("estimated pic_y: {pic_y}, top_y: {}, bottom_y: {}, text: {}", top_y, bottom_y, self.text);
        self.set_v_bounds(top_y, bottom_y, bound_start_x, bound_end_x);
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
/// use fltkrs_richdisplay::{is_overlap, Rectangle};
/// let c_coord = Rectangle(40, 28, 40 + 21, 50);
/// let selection_area = Rectangle(40, 28, 80, 45);
/// if is_overlap(&c_coord, &selection_area) {
///     println!("选区包含了目标区域");
/// }
/// ```
pub fn is_overlap(target_area: &Rectangle, selection_area: &Rectangle) -> bool {
    target_area.0 < (selection_area.0 + selection_area.2) && (target_area.0 + target_area.2) > selection_area.0 && target_area.1 < (selection_area.1 + selection_area.3) && (target_area.1 + target_area.3) > selection_area.1
}

/// 拖选时，当前片段相对于拖选起点位置的方位。
#[derive(Debug)]
enum PosOfDrag {
    /// 上方，选区位于片段内
    UpInside,
    /// 上方，选区包裹片段
    UpCover,
    /// 左上方，与选区有交叉
    UpLeftInside,
    /// 左上方，与选区有交叉，起点在选区外
    UpLeftOutside,
    /// 左上方，选区外
    UpLeft,
    /// 右上方，与选区有交叉
    UpRightInside,
    /// 右上方，与选区有交叉，起点在选区外。
    UpRightOutside,
    /// 右上方，选区外
    UpRight,
    /// 下方，选区位于片段内
    DownInside,
    /// 下方，选区包裹片段
    DownCover,
    /// 左下方，与选区有交叉
    DownLeftInside,
    /// 左下方，选区外
    DownLeft,
    /// 右下方，与选区有交叉
    DownRightInside,
    /// 右下方，与选区有交叉，起点在外部。
    DownRightOutside,
    /// 左下方，与选区有交叉，起点在外部。
    DownLeftOutside,
    /// 右下方，选区外
    DownRight,
}

/// 水平方位
enum HorizonPosOfDrag {
    /// 选区位于片段内
    Inside,
    /// 选区位于左侧，有交叉
    LeftInside,
    /// 选区位于左侧，无交叉
    Left,
    /// 选区位于右侧，有交叉
    RightInside,
    /// 选区位于右侧，无交叉
    Right,
    /// 选区位于左侧，有交叉，起点位于外部
    RightOutside,
    /// 选区位于右侧，有交叉，起点位于外部
    LeftOutside,
    /// 选区包裹片段
    Cover,
}

fn _check_pos(pos: usize, border: i32, x: i32, text: &str) -> Ordering {
    if pos == 0 {
        Ordering::Equal
    } else {
        let (pw2, _) = measure(text.chars().take(pos).collect::<String>().as_str(), false);
        if x + pw2 <= border {
            Ordering::Equal
        } else {
            Ordering::Greater
        }
    }
}

/// 通过鼠标选择文本时，根据划选区域和方向，自动选择符合用户习惯的文本内容范围。
///
/// # Arguments
///
/// * `drag_area`: 拖选区域。
/// * `visible_lines`: 当前窗口内可见的内容行。
/// * `column_mode`: 是否列模式。普通操作时不选列模式。
/// * `push_from`: 鼠标拖选开始时的坐标。
/// * `panel_x`: 当前内容面板的x坐标。
///
/// returns: bool
///
/// # Examples
///
/// ```
///
/// ```
pub fn select_text(drag_area: &Rectangle, visible_lines: Rc<RefCell<HashMap<Rectangle, LinePiece>>>, column_mode: bool, push_from: (i32, i32), panel_x: i32, line_max_width: i32) -> bool {
    /*
    遍历可见行，检查每一行的矩形范围是否与选区有重叠，若有重叠则继续检测出现重叠的行中哪些文字片段与选区有重叠。
     */
    let mut selected = false;
    // 选中行的代表
    let mut selected_lines: BTreeMap<i32, LinePiece> = BTreeMap::new();
    for (line_rect, piece) in visible_lines.borrow_mut().iter_mut() {
        let extended_line_rect = Rectangle::new(line_rect.0, line_rect.1, line_max_width, line_rect.3);
        if is_overlap(&extended_line_rect, drag_area) {
            /*
            记录每一行位于选择区域中最左边那个片段，作为每一行的代表片段。
            由于同一行内的不同片段可能绘制高度不同，但是都具有的top_y属性可以表示虚拟行高的顶部位置。
             */
            if !selected_lines.contains_key(&piece.top_y) {
                selected_lines.insert(piece.top_y, piece.clone());
            } else {
                if let Some(old_piece) = selected_lines.get(&piece.top_y) {
                    if piece.x < old_piece.x {
                        selected_lines.insert(piece.top_y, piece.clone());
                    }
                }
            }

            set_font(piece.font, piece.font_size);

            /*
            采用二分法依次检索选区中字符串左边界和右边界的字符索引，并记录左右边界字符右侧水平坐标。
             */
            let (x, len)= (line_rect.0, piece.line.chars().count());
            let text = piece.line.as_str();
            let from_vec = (0..len).collect::<Vec<usize>>();
            if let Ok(from) = from_vec.binary_search_by({
                let left_border = drag_area.0;
                move |pos| {
                    let (pw1, _) = measure(text.chars().take(*pos + 1).collect::<String>().as_str(), false);
                    if x + pw1 < left_border {
                        Ordering::Less
                    } else {
                        _check_pos(*pos, left_border, x, text)
                    }
                }
            }) {
                if from == len - 1 {
                    piece.select_range(from, len);
                    selected = true;
                } else {
                    // 查找右边界字符位置
                    let to_vec = (from..len).collect::<Vec<usize>>();
                    if let Ok(to) = to_vec.binary_search_by({
                        let right_boarder = drag_area.0 + drag_area.2;
                        move |pos| {
                            let (pw1, _) = measure(text.chars().take(*pos + 1).collect::<String>().as_str(), false);
                            if x + pw1 < right_boarder {
                                if pos + 1 == len {
                                    Ordering::Equal
                                } else {
                                    Ordering::Less
                                }
                            } else {
                                _check_pos(*pos, right_boarder, x, text)
                            }
                        }
                    }) {
                        piece.select_range(from, to_vec[to] + 1);
                        selected = true;
                    } else {
                        error!("检测结尾时异常！")
                    }
                }
            }
        } else {
            piece.deselect();
        }
    }

    if selected && !column_mode {
        let selected_lines_clone = selected_lines.clone();
        if selected_lines.len() > 1 {
            // 选区有跨行情况
            // 首先处理(消费)最后一行
            if let Some((_, piece)) = selected_lines.pop_last() {
                let last_line = &piece.through_line.borrow_mut().ys;
                for p in last_line.borrow_mut().iter_mut() {
                    if let Some(p) = p.upgrade() {
                        let lp = &mut *p.borrow_mut();

                        /*
                        对位于代表片段左边的所有分片进行全选处理，而代表片段右边的片段不处理(选中或未选中都不变)。
                         */
                        if lp.eq2(&piece) {
                            if drag_area.0 >= panel_x + lp.x {
                                let mut new_to = 0;
                                if let Some((old_from, old_to)) = lp.selected_range.get() {
                                    if drag_area.1 < push_from.1 {
                                        if drag_area.0 < push_from.0 {
                                            // 选区向起始点左上方延伸，结束位不变。
                                            new_to = old_to;
                                        } else {
                                            // 选区向起始点右上方延伸，结束位是选区起始位。
                                            new_to = old_from + 1;
                                        }
                                    } else {
                                        if drag_area.0 < push_from.0 {
                                            // 选区向起始点左下方延伸，结束位是选区起始位。
                                            new_to = old_from + 1;
                                        } else {
                                            // 选区向起始点右下方延伸，结束位不变。
                                            new_to = old_to;
                                        }
                                    }
                                }
                                lp.select_to(new_to);
                            } else {
                                // 选区间隙，维持不变
                            }
                        } else {

                            if push_from.0 < panel_x + lp.x {
                                if drag_area.1 < push_from.1 {
                                    // 右侧片段，向上拖选时不选
                                    lp.deselect();
                                } else if lp.next_x > lp.x && drag_area.0 + drag_area.2 > panel_x + lp.next_x {
                                    lp.select_all();
                                }
                                // 右侧片段，向下拖选时维持选择状态
                            } else if push_from.0 >= panel_x + lp.next_x {
                                if drag_area.0 > panel_x + lp.x {
                                    // 左侧片段，全选
                                    lp.select_all();
                                } else {
                                    if drag_area.1 + drag_area.3 > push_from.1 {
                                        // 向左下拖选，选区片段左边界大于选区左边界，不选
                                        lp.deselect();
                                    } else {
                                        // 向左上拖选，当前片段维持原态
                                    }
                                }
                            } else {
                                // 拖选起点位于片段两端之间
                                if drag_area.0 < panel_x + lp.x && (drag_area.1 + drag_area.3 > push_from.1) {
                                    // 向左下拖选，左端大于选区左边界，不选择
                                    lp.deselect();
                                } else if push_from.1 == drag_area.1 && (drag_area.0 + drag_area.2 > panel_x + lp.next_x) {
                                    // 向下拖选，选区右边界超出片段右边界，全选
                                    lp.select_all();
                                } else {
                                    // 向左上拖选，维持原态
                                }
                            }
                        }
                    }
                }
            }

            // 接着处理(消费)第一行
            if let Some((_, piece)) = selected_lines.pop_first() {
                let first_line = &piece.through_line.borrow_mut().ys;
                for p in first_line.borrow_mut().iter_mut() {
                    if let Some(p) = p.upgrade() {
                        let lp = &*p.borrow_mut();
                        /*
                        根据当前分片相对于选区的方位，对选中内容进行调整，以符合用户操作习惯。
                        总计有20种情况需要判断处理。
                         */
                        let pos = find_position_of_piece(drag_area, push_from, panel_x, lp);
                        match pos {
                            PosOfDrag::UpInside => {
                                // 向上拖选，选区位于当前片段内
                                if drag_area.0 != push_from.0 {
                                    // 从起始点向左划选，从选区左边界向右扩选。
                                    extend_from_end(lp);
                                } else {
                                    // 从起始点向右划选，从选区右边界向右反选。
                                    reverse_to_extend_from_end(lp);
                                }
                            }
                            PosOfDrag::DownInside => {
                                // 向下拖选，选区位于当前片段内。
                                if drag_area.0 != push_from.0 {
                                    // 向左划选，以选区末尾字符为起始向右扩选内容。
                                    reverse_to_extend_from_end(lp);
                                } else {
                                    // 向右划选，扩选起点字符之后的内容
                                    extend_from_end(lp);
                                }
                            }
                            PosOfDrag::UpLeftInside | PosOfDrag::DownLeftOutside | PosOfDrag::DownLeft => {
                                // 向上拖选，片段位于选区左部有交叉，不选
                                lp.deselect();
                            }
                            PosOfDrag::UpLeft | PosOfDrag::UpLeftOutside | PosOfDrag::DownLeftInside => {
                                // 向上拖选，选区位于当前片段左侧外部，不选择。实际上维持不变。
                            }
                            PosOfDrag::UpRightInside | PosOfDrag::UpRight | PosOfDrag::DownRight | PosOfDrag::DownRightOutside => {
                                // 向上拖选，当前片段在选区右部有交叉，全选
                                lp.select_all();
                            }
                            // PosOfDrag::UpLeftOutside => {
                            //     // 向上拖选，当前片段在选区左部有交叉，维持不变
                            // }
                            // PosOfDrag::UpRight => {
                            //     // 向上拖选，选区位于当前片段右侧外部，全选。
                            //     lp.selected_range.set(Some((0, lp.line.chars().count())));
                            // }
                            PosOfDrag::UpRightOutside | PosOfDrag::DownRightInside => {
                                // 向上拖选，片段位于选区右部有交叉，向右反选
                                reverse_to_extend_from_end(lp);
                            }
                            PosOfDrag::DownCover => {
                                // 向下拖选，选区包括片段，不选择。
                                if drag_area.0 == push_from.0 {
                                    // 选区起点在片段左外侧，全选
                                    lp.select_all();
                                } else {
                                    // 选区起点在片段右外侧，不选
                                    lp.deselect();
                                }
                            }
                            PosOfDrag::UpCover => {
                                // 向上拖选，选区包括片段
                                if drag_area.0 != push_from.0 {
                                    // 向左划选，全选。
                                    lp.select_all();
                                } else {
                                    // 向右划选，不选
                                    lp.deselect();
                                }
                            }
                            // PosOfDrag::DownLeftInside => {
                            //     // 向下拖选，选区位于当前片段右部，有交叉，维持不变。
                            // }
                            // PosOfDrag::DownLeft => {
                            //     // 向下拖选，选区位于当前片段左侧外部，不选择。
                            //     lp.deselect();
                            // }
                            // PosOfDrag::DownRightInside => {
                            //     // 向下拖选，当前片段在选区右部有交叉，以选区末尾字符为起始向右扩选内容。
                            //     reverse_to_extend_from_end(lp);
                            // }
                            // PosOfDrag::DownRight => {
                            //     // 向下拖选，选区位于当前片段右侧外部，全选。
                            //     lp.selected_range.set(Some((0, lp.line.chars().count())));
                            // }
                            // PosOfDrag::DownRightOutside => {
                            //     // 向下拖选，选区位于当前片段右侧外部，全选。
                            //     lp.selected_range.set(Some((0, lp.line.chars().count())));
                            // }
                            // PosOfDrag::DownLeftOutside => {
                            //     // 向下拖选，选区位于当前片段右侧外部，不选。
                            //     lp.deselect(;)
                            // }
                        }
                    }
                }
            }
            // 最后处理中间行即剩余的行，进行全选处理。
            selected_lines.iter_mut().for_each({
                |(_, piece)| {
                    let tl = &piece.through_line.borrow_mut().ys;
                    tl.borrow_mut().iter_mut().for_each({
                        |p| {
                            if let Some(p) = p.upgrade() {
                                let lp = &mut *p.borrow_mut();
                                lp.select_all();
                            }
                        }
                    });
                }
            });
        }

        /*
        拷贝至剪贴板
         */
        let mut selection = String::new();
        for (_, piece) in selected_lines_clone.iter() {
            let tl = &piece.through_line.borrow().ys;
            copy_pieces(tl.borrow().iter(), &mut selection);
        }
        app::copy(selection.as_str());
    }
    selected
}

fn copy_pieces(it: Iter<Weak<RefCell<LinePiece>>>, selection: &mut String) {
    for p in it {
        if let Some(p) = p.upgrade() {
            let lp = &*p.borrow();
            lp.copy_selection(selection);
        }
    }
}

/// 检查拖选区与当前分片的方位关系。
///
/// # Arguments
///
/// * `drag_area`: 拖选区范围。
/// * `push_from`: 拖选起始坐标(x, y)。
/// * `panel_x`: 面板的x坐标。
/// * `piece`: 当前片段。
///
/// returns: PosOfDrag 返回对应的方位枚举。
///
/// # Examples
///
/// ```
///
/// ```
fn find_position_of_piece(drag_area: &Rectangle, push_from: (i32, i32), panel_x: i32, piece: &LinePiece) -> PosOfDrag {
    let to_down = drag_to_down(drag_area, push_from.1);
    match drag_from_piece(drag_area, panel_x, push_from.0, piece) {
        HorizonPosOfDrag::Inside => {
            if to_down {
                PosOfDrag::DownInside
            } else {
                PosOfDrag::UpInside
            }
        }
        HorizonPosOfDrag::Left => {
            if to_down {
                PosOfDrag::DownLeft
            } else {
                PosOfDrag::UpLeft
            }
        },
        HorizonPosOfDrag::Right => {
            if to_down {
                PosOfDrag::DownRight
            } else {
                PosOfDrag::UpRight
            }
        }
        HorizonPosOfDrag::LeftInside => {
            if to_down {
                PosOfDrag::DownLeftInside
            } else {
                PosOfDrag::UpLeftInside
            }
        }
        HorizonPosOfDrag::RightInside => {
            if to_down {
                PosOfDrag::DownRightInside
            } else {
                PosOfDrag::UpRightInside
            }
        }
        HorizonPosOfDrag::RightOutside => {
            if to_down {
                PosOfDrag::DownRightOutside
            } else {
                PosOfDrag::UpRightOutside
            }
        }
        HorizonPosOfDrag::LeftOutside => {
            if to_down {
                PosOfDrag::DownLeftOutside
            } else {
                PosOfDrag::UpLeftOutside
            }
        }
        HorizonPosOfDrag::Cover => {
            if to_down {
                PosOfDrag::DownCover
            } else {
                PosOfDrag::UpCover
            }
        }
    }
}

/// 检测拖拽起始位置是否位于片段内。
///
/// # Arguments
///
/// * `panel_x`: 面板x坐标。
/// * `piece`: 片段。
/// * `push_from_x`: 拖拽起始点x坐标。
///
/// returns: bool 若起始点位于片段内返回true，否则返回false。
///
/// # Examples
///
/// ```
///
/// ```
fn drag_from_piece(drag_area: &Rectangle, panel_x: i32, push_from_x: i32, piece: &LinePiece) -> HorizonPosOfDrag {
    if push_from_x >= panel_x + piece.x {
        // 拖拽起始点位于当前片段起始点右侧
        if piece.next_x > piece.x {
            // 非右边顶头的片段
            if push_from_x < panel_x + piece.next_x {
                // 拖选起始点位于当前片段内部
                if drag_area.0 >= panel_x + piece.x && drag_area.0 + drag_area.2 <= panel_x + piece.next_x {
                    // 拖选范围在当前片段内部
                    HorizonPosOfDrag::Inside
                } else if drag_area.0 < panel_x + piece.x {
                    // 拖选范围在起始点左侧，与当前片段有交叉
                    HorizonPosOfDrag::RightInside
                } else {
                    // 拖选范围在起始点右侧，与当前片段有交叉
                    HorizonPosOfDrag::LeftInside
                }
            } else {
                // 拖选起始点位于当前片段右边外侧
                if drag_area.0 < panel_x + piece.x {
                    // 拖选范围涵盖当前片段
                    HorizonPosOfDrag::Cover
                } else if drag_area.0 < panel_x + piece.next_x {
                    // 拖选范围在当前片段右侧，有交叉
                    HorizonPosOfDrag::LeftOutside
                } else {
                    HorizonPosOfDrag::Left
                }
            }
        } else {
            // 右边顶头的片段
            if drag_area.0 >= panel_x + piece.x {
                // 拖选范围在当前片段内部，或偏右
                HorizonPosOfDrag::Inside
            } else {
                // 拖选范围在当前片段左侧，有交叉
                HorizonPosOfDrag::RightInside
            }
        }
    } else {
        // 拖选起始点位于当前片段左侧外部
        if drag_area.0 + drag_area.2 > panel_x + piece.x {
            if piece.next_x > piece.x {
                // 非右边顶点片段
                if drag_area.0 + drag_area.2 > panel_x + piece.next_x {
                    // 拖选范围覆盖片段
                    HorizonPosOfDrag::Cover
                } else {
                    // 拖选范围在当前片段左侧，有交叉
                    HorizonPosOfDrag::RightOutside
                }
            } else {
                // 右边顶点片段
                HorizonPosOfDrag::RightOutside
            }

        } else {
            HorizonPosOfDrag::Right
        }
    }
}

/// 检查拖选垂直方向。
///
/// # Arguments
///
/// * `drag_area`: 拖选区域。
/// * `push_from_y`: 拖选起始点y坐标。
///
/// returns: bool 若向下返回true，否则返回false。
///
/// # Examples
///
/// ```
///
/// ```
fn drag_to_down(drag_area: &Rectangle, push_from_y: i32) -> bool {
    if drag_area.1 < push_from_y {
        false
    } else {
        true
    }
}

///
///
/// # Arguments
///
/// * `piece`:
///
/// returns: ()
///
/// # Examples
///
/// ```
///
/// ```
fn reverse_to_extend_from_end(piece: &LinePiece) {
    if let Some((_, old_to)) = piece.selected_range.get() {
        piece.select_from(old_to - 1);
    }
}

/// 从选区起始字符，向右扩选片段内容。
///
/// # Arguments
///
/// * `piece`:
///
/// returns: ()
///
/// # Examples
///
/// ```
///
/// ```
fn extend_from_end(piece: &LinePiece) {
    if let Some((old_from, _)) = piece.selected_range.get() {
        piece.select_from(old_from);
    }
}

pub fn locate_piece_at_point(visible_lines: Rc<RefCell<HashMap<Rectangle, LinePiece>>>, win_point: ClickPoint, offset_y: i32) -> Option<LinePiece> {
    for (_, piece) in visible_lines.borrow().iter() {
        if is_overlap(&piece.rect(offset_y), &win_point.as_rect()) {
            debug!("piece y: {}, offset_y: {}", piece.y, offset_y);
            return Some(piece.clone());
        }
    }
    None
}

pub(crate) fn clear_selected_pieces(selected_pieces: Rc<RefCell<Vec<Weak<RefCell<LinePiece>>>>>) {
    for piece in selected_pieces.borrow().iter() {
        if let Some(p) = piece.upgrade() {
            p.borrow().deselect();
        }
    }
    selected_pieces.borrow_mut().clear();
}

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

pub fn select_text2(from_point: &ClickPoint, to_point: ClickPoint, data_buffer: Rc<RefCell<Vec<RichData>>>, rd_range: RangeInclusive<usize>, selected_pieces: Rc<RefCell<Vec<Weak<RefCell<LinePiece>>>>>) {
    /*
    选择片段的原则：应选择起点右下方的第一行片段，结束点左上方的第一行片段，以及两点之间的中间行片段。
     */
    // debug!("传入的fp: {:?}, tp: {:?}", from_point, to_point);
    let drag_rect = from_point.to_rect(&to_point);
    let (lt, br) = drag_rect.corner_rect();
    let (mut lt_p, mut br_p) = (from_point, &to_point);
    if (br.0 == from_point.x && br.1 == from_point.y) || (lt.0 == from_point.x && br.1 == from_point.y) {
        // debug!("对换坐标点");
        lt_p = &to_point;
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
        if let Some(rd) = data_buffer.borrow().get(r_start) {
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
            if let Some(rd) = data_buffer.borrow().get(i) {
                for p in rd.line_pieces.iter() {
                    let piece = &*p.borrow();
                    piece.select_all();
                    piece_rcs.push(Rc::downgrade(p));
                }
            }
        }
        selected_pieces.borrow_mut().append(&mut piece_rcs);

        if let Some(rd) = data_buffer.borrow().get(r_end) {
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
        if let Some(rd) = data_buffer.borrow().get(r_start) {
            let across_pieces = t_p_i - f_p_i;
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

pub fn locate_target_rd(point: &mut ClickPoint, drag_rect: &Rectangle, panel_width: i32, data_buffer: Rc<RefCell<Vec<RichData>>>, index_vec: &Vec<usize>) -> Option<usize> {
    let point_rect = point.as_rect();
    // debug!("index_vec: {:?}", index_vec);
    if let Ok(idx) = index_vec.binary_search_by({
        let buffer_rc = data_buffer.clone();
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
            let rd = &(&*buffer_rc.borrow())[*row];
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
        let rd = &(&*data_buffer.borrow())[index_vec[idx]];
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

pub fn search_index_of_piece(piece: &LinePiece, point: &mut ClickPoint) {
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
        HIGHLIGHT_WHITE
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
    // let supported = draw::can_do_alpha_blending();
    // if supported {
    //      // 使用alpha blending算法降低色彩饱和度，效果不好。
    //     Color::from_rgba_tuple((r, g, b, 60u8))
    // } else {
    //     let total = r as u16 + g as u16 + b as u16;
    //     let max_c = max(r, max(g, b));
    //     if total >= 383 || max_c as u16 + 127 > 255u16 {
    //         let (cr, cg, cb) = (max(0i16, r as i16 - 127), max(0i16, g as i16 - 127), max(0i16, b as i16 - 127));
    //         Color::from_rgb(cr as u8, cg as u8, cb as u8)
    //          // 使用内置api降低亮度，效果不理想
    //         // color.darker()
    //     } else {
    //         let (cr, cg, cb) = (min(255i16, r as i16 + 127), min(255i16, g as i16 + 127), min(255i16, b as i16 + 127));
    //         Color::from_rgb(cr as u8, cg as u8, cb as u8)
    //          // 使用内置api提高亮度，效果不理想
    //         // color.lighter()
    //     }
    // }

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

pub fn draw_image_mono(
    data: &[u8],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    depth: ColorDepth,
) -> Result<(), FltkError> {
    let sz = (w * h * depth as i32) as usize;
    if sz > data.len() {
        return Err(FltkError::Internal(FltkErrorKind::ImageFormatError));
    }

    unsafe {
        Fl_draw_image_mono(data.as_ptr(), x, y, w, h, depth as i32, 0);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use fltk::enums::Color;
    use crate::{get_contrast_color, get_lighter_or_darker_color, HIGHLIGHT_WHITE, Rectangle};

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
            assert_ne!(get_contrast_color(Color::from_rgb(i, i, i)), HIGHLIGHT_WHITE);
        }
        for i in 116..=139 {
            assert_eq!(get_contrast_color(Color::from_rgb(i, i, i)), HIGHLIGHT_WHITE);
        }
        for i in 140..=255 {
            assert_ne!(get_contrast_color(Color::from_rgb(i, i, i)), HIGHLIGHT_WHITE);
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
