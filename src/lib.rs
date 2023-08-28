use std::cell::{Cell, RefCell};
use std::cmp::{max, min, Ordering};
use std::collections::HashMap;
use std::fmt::{Debug};
use std::rc::{Rc, Weak};
use fltk::{app, draw};
use fltk::draw::{descent, draw_image, draw_line, draw_rounded_rectf, draw_text2, measure, set_draw_color, set_font};
use fltk::enums::{Align, Color, ColorDepth, Cursor, Font};
use fltk::image::RgbImage;
use fltk::prelude::ImageExt;

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

/// 自定义事件。
pub struct LocalEvent;
impl LocalEvent {

    /// 滚动事件。
    pub const SCROLL_TO: i32 = 100;

    /// 缩放事件。
    pub const RESIZE: i32 = 101;

    /// 清理回顾区组件事件。
    pub const DROP_REVIEWER: i32 = 102;

    /// 从rich-display容器外部发起关闭回顾区的事件。
    pub const DROP_REVIEWER_FROM_EXTERNAL: i32 = 103;

    /// 从rich-display容器外部发起打开回顾区的事件。
    pub const OPEN_REVIEWER_FROM_EXTERNAL: i32 = 104;

}

/// 矩形结构，x/y表示左上角坐标，w/h表示宽和高，w/h不为负值。
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct Rectangle(i32, i32, i32, i32);
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

    pub fn tup(&self) -> (i32, i32, i32, i32) {
        (self.0, self.1, self.2, self.3)
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
}

impl LinePiece {
    pub fn new(line: String, x: i32, y: i32, w: i32, h: i32, top_y: i32, spacing: i32, next_x: i32, next_y: i32, font_height: i32, through_line: Rc<RefCell<ThroughLine>>) -> Rc<RefCell<LinePiece>> {
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
        }));
        through_line.borrow_mut().add_piece(init_piece.clone());
        init_piece
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
    fn set_v_bounds(&mut self, top_y: i32, bottom_y: i32, start_x: i32);

    /// 获取绘制区域高度。
    fn height(&self) -> i32;

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
    fn draw(&self, offset_y: i32);

    fn estimate(&mut self, blow_line: Rc<RefCell<LinePiece>>, max_width: i32) -> Rc<RefCell<LinePiece>>;

    /// 擦除内容，但保留占位。
    fn erase(&mut self);

    /// 清除部分或全部内容，并释放已清除部分的占位。
    ///
    /// # Arguments
    ///
    /// * `rtl`: 清除方向，true表示从右向左清除，false表示从左向右清除。
    /// * `length`: 清除长度。对于文本内容是字符个数，对于图形内容是像素宽度。
    ///
    /// returns: LineCoord 返回释放出的空间的起始位置信息。
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn truncate(&mut self, rtl: bool, length: Option<i32>) -> LinePiece;
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
            if let Some(image) = rd.image.as_mut() {
                if let Ok(mut ni) = RgbImage::new(image.as_slice(), rd.image_width, rd.image_height, ColorDepth::Rgb8) {
                    ni.inactive();
                    image.clear();
                    image.append(&mut ni.to_rgb_data());
                }
            }
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
    pub strike_through: bool,
    pub line_height: i32,
    /// 当前内容在面板垂直高度中的起始和截至y坐标，以及起始x坐标。
    v_bounds: Option<(i32, i32, i32)>,

    /// 对当前数据进行试算后，分割成适配单行宽度的分片保存起来。由于无需跨线程传输，因此也不考虑线程安全问题。
    pub(crate) line_pieces: Vec<Rc<RefCell<LinePiece>>>,
    data_type: DataType,
    image: Option<Vec<u8>>,
    image_width: i32,
    image_height: i32,
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
                    strike_through: data.strike_through,
                    line_height: 1,
                    v_bounds: None,
                    line_pieces: vec![],
                    data_type: DataType::Text,
                    image: None,
                    image_width: 0,
                    image_height: 0,
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
                    strike_through: data.strike_through,
                    line_height: 1,
                    v_bounds: None,
                    line_pieces: Vec::with_capacity(0),
                    data_type: DataType::Image,
                    image: data.image,
                    image_width: data.image_width,
                    image_height: data.image_height,
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

            let new_piece = LinePiece::new(text.chars().take(stop_pos).collect::<String>(), last_piece.next_x, y, w, font_height, top_y, last_piece.spacing, next_x, next_y, font_height, through_line.clone());
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
                let new_piece = LinePiece::new(rest_str, rest_x, rest_y, rest_width, font_height, top_y, last_piece.spacing, rest_next_x, rest_next_y, font_height, through_line);
                self.line_pieces.push(new_piece.clone());
                new_piece
            }
        } else {
            // 从行首开始
            let through_line = ThroughLine::create_or_update(PADDING.left, PADDING.left, self.line_height, original.clone(), false);
            let y = last_piece.next_y + last_piece.through_line.borrow().max_h + last_piece.spacing;
            let new_piece = LinePiece::new(text.to_string(), PADDING.left, y, measure_width, self.line_height, y, last_piece.spacing, PADDING.left, y, font_height, through_line);
            self.wrap_text_for_estimate(text, new_piece, max_width, measure_width, font_height)
        }
    }
}


impl LinedData for RichData {
    fn set_v_bounds(&mut self, top_y: i32, bottom_y: i32, start_x: i32) {
        self.v_bounds = Some((top_y, bottom_y, start_x));
    }

    fn height(&self) -> i32 {
        if let Some(b) = &self.v_bounds {
            b.1 - b.0
        } else {
            0
        }
    }

    fn is_text_data(&self) -> bool {
        true
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
        if let Some(b) = &self.v_bounds {
            !(b.1 < top_y || b.0 > bottom_y)
        } else {
            true
        }
    }

    fn draw(&self, offset_y: i32) {
        match self.data_type {
            DataType::Text => {
                let bg_offset = 1;

                set_font(self.font, self.font_size);
                for piece in self.line_pieces.iter() {
                    let piece = &*piece.borrow();
                    let y = piece.y - offset_y;
                    // if self.clickable() {
                    //     let cmap = &mut *clickable_data_mapper.borrow_mut();
                    //     cmap.insert(Rectangle::new(piece.x + panel_x, y + panel_y, piece.w, piece.h), idx);
                    // }
                    //
                    // {
                    //     let vmap = &mut *visible_lines.borrow_mut();
                    //     vmap.insert(Rectangle::new(piece.x + panel_x, y + panel_y, piece.w, piece.h), piece.clone());
                    // }

                    if let Some(bg_color) = &self.bg_color {
                        // 绘制背景色
                        set_draw_color(*bg_color);

                        #[cfg(target_os = "linux")]
                        draw_rounded_rectf(piece.x, y - piece.spacing + 2, piece.w, piece.font_height, 4);

                        #[cfg(not(target_os = "linux"))]
                        draw_rounded_rectf(piece.x, y - piece.spacing, piece.w, piece.font_height, 4);
                    }

                    if let Some((from, to)) = piece.selected_range.get() {
                        // 绘制选中背景色
                        set_draw_color(Color::Selection);
                        let (skip_width, _) = measure(piece.line.chars().take(from).collect::<String>().as_str(), false);
                        let (fill_width, _) = measure(piece.line.chars().skip(from).take(to - from).collect::<String>().as_str(), false);

                        #[cfg(target_os = "linux")]
                        draw_rounded_rectf(piece.x + skip_width, y - piece.spacing + 2, fill_width, piece.font_height, 4);

                        #[cfg(not(target_os = "linux"))]
                        draw_rounded_rectf(piece.x + skip_width, y - piece.spacing, fill_width, piece.font_height, 4);
                    }

                    set_draw_color(self.fg_color);
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
                if let Some(piece) = self.line_pieces.last() {
                    let piece = &*piece.borrow();
                    // if self.clickable() {
                    //     let map = &mut *clickable_data_mapper.borrow_mut();
                    //     map.insert(Rectangle::new(piece.x + panel_x, piece.y - offset_y + panel_y, piece.w, piece.h), idx);
                    // }

                    if let Some(img) = &self.image {
                        if let Err(e) = draw_image(img.as_slice(), piece.x, piece.y - offset_y, piece.w, piece.h, ColorDepth::Rgb8) {
                            error!("draw image error: {:?}", e);
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
                                new_piece = LinePiece::new(line.to_string(), lp.next_x, y, tw, current_line_height, piece_top_y, lp.spacing, next_x, next_y, ref_font_height, through_line);

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
                                new_piece = LinePiece::new(line.to_string(), last_piece.next_x, y, tw, self.line_height, piece_top_y, last_piece.spacing, next_x, next_y, ref_font_height, through_line);
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
                        let new_piece = LinePiece::new(self.text.clone(), start_x, y, tw, ref_font_height, top_y, current_line_spacing, start_x + tw + PIECE_SPACING, top_y, ref_font_height, through_line);
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
                    let piece_top_y = top_y + IMAGE_PADDING_V;
                    let next_x = x + self.image_width + IMAGE_PADDING_H;
                    let next_y = top_y + last_piece.through_line.borrow().max_h + IMAGE_PADDING_V;
                    let through_line = ThroughLine::new(self.image_height * IMAGE_PADDING_V * 2, true);
                    let new_piece = LinePiece::new("".to_string(), x, next_y, self.image_width, self.image_height, piece_top_y, last_piece.spacing, next_x, next_y, 1, through_line);
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
                        let new_piece = LinePiece::new("".to_string(), x, y, self.image_width, self.image_height, piece_top_y, last_piece.spacing, next_x, top_y, 1, through_line);
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
                        let new_piece = LinePiece::new("".to_string(), x, y, self.image_width, self.image_height, piece_top_y, last_piece.spacing, next_x, top_y + IMAGE_PADDING_V, 1, through_line);
                        self.line_pieces.push(new_piece.clone());
                        ret = new_piece;
                    }
                }
            }
        }

        let mut to_be_updated: Vec<(Rc<RefCell<LinePiece>>, i32)> = Vec::new();
        for line_piece in self.line_pieces.iter() {
            let lp = &*line_piece.borrow();
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
        }

        let mut top_y = top_y;
        if let Some(first_piece) = self.line_pieces.first() {
            let fp = &*first_piece.borrow();
            top_y = fp.top_y;
        }
        let mut bottom_y = top_y;
        if let Some(last_piece) = self.line_pieces.last() {
            let lp = &*last_piece.borrow();
            // bottom_y = lp.y + lp.through_line.borrow().max_h;
            bottom_y = lp.top_y + lp.through_line.borrow().max_h;
        }
        self.set_v_bounds(top_y, bottom_y, start_x);
        ret
    }

    fn erase(&mut self) {
        todo!()
    }

    fn truncate(&mut self, rtl: bool, length: Option<i32>) -> LinePiece {
        todo!()
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

pub fn select_text(drag_area: &Rectangle, visible_lines: Rc<RefCell<HashMap<Rectangle, LinePiece>>>) -> bool {
    /*
    遍历可见行，检查每一行的矩形范围是否与选区有重叠，若有重叠则继续检测出现重叠的行中哪些文字片段与选区有重叠。
     */
    let mut selected = false;
    for (line_rect, piece) in visible_lines.borrow_mut().iter_mut() {
        if is_overlap(line_rect, drag_area) {
            // 当前行与选区有重叠，from:出现重叠的起始字符索引号，to:出现重叠的结束字符索引号。
            let (mut from, len) = (0, piece.line.chars().count());
            let mut to = len;
            let mut overlap = false;
            /*
            逐字检测重叠的文字片段。
            首先从首字符逐字累加试算其矩形范围，检测是否重叠，得出起始索引号。
            待得到起始索引号后，再试算剩余部分文字的矩形范围，并逐字向后缩减范围，得出非重叠字符的索引号时即刻终止循环。
             */
            for i in 0..len {
                if !overlap {
                    let (pre_half_width, pre_half_height) = measure(piece.line.chars().take(i + 1).collect::<String>().as_str(), false);
                    let part_rect = Rectangle::new(line_rect.0, line_rect.1, pre_half_width, pre_half_height);
                    if is_overlap(&part_rect, drag_area) {
                        from = i;
                        overlap = true;
                    }

                } else if overlap {
                    let (post_half_width, post_half_height) = measure(piece.line.chars().skip(i).collect::<String>().as_str(), false);
                    let part_rect = Rectangle::new(line_rect.0 + line_rect.2 - post_half_width, line_rect.1 + line_rect.3 - post_half_height, post_half_width, post_half_height);
                    if !is_overlap(&part_rect, drag_area) {
                        to = i;
                        break;
                    }

                }
            }
            debug!("from: {}, to: {}, len: {}", from, to, len);
            if overlap {
                piece.selected_range.set(Some((from, to)));
                selected = true;
            }
        }
    }
    selected
}

#[cfg(test)]
mod tests {
    use crate::Rectangle;

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
}
