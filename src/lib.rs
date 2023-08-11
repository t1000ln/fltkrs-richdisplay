use std::cell::{RefCell};
use std::cmp::{max, min, Ordering};
use std::rc::{Rc, Weak};
use fltk::draw::{descent, draw_image, draw_line, draw_rounded_rectf, draw_text2, measure, set_draw_color, set_font};
use fltk::enums::{Align, Color, ColorDepth, Font};

pub mod rich_text;
pub mod rich_snapshot;

/// 默认内容边界到窗口之间的空白距离。
pub const PADDING: Padding = Padding { left: 5, top: 5, right: 5, bottom: 5 };

/// 图片与其他内容之间的垂直间距。
pub const IMAGE_PADDING_H: i32 = 2;

/// 图片与其他内容之间的水平间距。
pub const IMAGE_PADDING_V: i32 = 1;

#[derive(Debug, Clone)]
pub struct Coordinates(i32, i32, i32, i32);

/// 同一行内多个分片之间共享的信息。通过Rc<RefCell<ThroughLine>>进行链接。
#[derive(Debug, Clone)]
pub struct ThroughLine {
    pub max_h: i32,
    pub ys: RefCell<Vec<Weak<RefCell<i32>>>>,
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

    pub fn new(max_h: i32, exist_image: bool, y: Rc<RefCell<i32>>) -> Rc<RefCell<ThroughLine>> {
        Rc::new(RefCell::new(ThroughLine { max_h, exist_image, ys: RefCell::new(vec![Rc::downgrade(&y)]) }))
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
            // todo: 需要重新计算所有的y
            self.max_h = max_h;
        }
        self
    }

    pub fn set_exist_image(&mut self, exist_image: bool) -> &mut Self {
        self.exist_image = exist_image;
        self
    }

    pub fn set_y(&mut self, y: i32) -> &mut Self {
        // todo: implement
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
    pub fn get_or_create(x_ref: i32, start_x: i32, current_line_height: i32, last_piece: &mut LinePiece, image: bool, y: Rc<RefCell<i32>>) -> Rc<RefCell<ThroughLine>> {
        if start_x == x_ref {
            ThroughLine::new(current_line_height, image, y)
        } else {
            last_piece.through_line.borrow_mut().set_max_h(current_line_height);
            last_piece.through_line.clone()
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
    pub y: Rc<RefCell<i32>>,
    /// 分片宽度，小于等于行宽
    pub w: i32,
    /// 行高
    pub h: i32,
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
}

impl LinePiece {
    pub fn new(line: String, x: i32, y: Rc<RefCell<i32>>, w: i32, h: i32, spacing: i32, next_x: i32, next_y: i32, font_height: i32, through_line: Rc<RefCell<ThroughLine>>) -> Self {
        Self {
            line,
            x,
            y,
            w,
            h,
            spacing,
            next_x,
            next_y,
            font_height,
            through_line,
        }
    }

    pub fn init_piece() -> LinePiece {
        Self {
            line: "".to_string(),
            x: PADDING.left,
            y: Rc::new(RefCell::new(PADDING.top)),
            w: 0,
            h: 1,
            spacing: 0,
            next_x: PADDING.left,
            next_y: PADDING.top,
            font_height: 1,
            through_line: Rc::new(RefCell::new(Default::default())),
        }
    }

    // pub fn next_line(&mut self, padding: &Padding) {
    //     self.next_x = padding.left;
    //     self.next_y = self.y + self.h + self.spacing;
    // }
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
    fn draw(&mut self, offset_y: i32);

    fn estimate(&mut self, blow_line: &mut LinePiece, max_width: i32);

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
    text: String,
    pub font: Font,
    pub font_size: i32,
    fg_color: Color,
    bg_color: Option<Color>,
    underline: bool,
    clickable: bool,
    expired: bool,
    data_type: DataType,
    image: Option<Vec<u8>>,
    image_width: i32,
    image_height: i32,
}

impl Into<RichData> for UserData {
    fn into(self) -> RichData {
        match self.data_type {
            DataType::Text => {
                RichData {
                    text: self.text,
                    font: self.font,
                    font_size: self.font_size,
                    fg_color: self.fg_color,
                    bg_color: self.bg_color,
                    underline: self.underline,
                    clickable: self.clickable,
                    expired: self.expired,
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
                    text: self.text,
                    font: self.font,
                    font_size: self.font_size,
                    fg_color: self.fg_color,
                    bg_color: self.bg_color,
                    underline: self.underline,
                    clickable: self.clickable,
                    expired: self.expired,
                    line_height: 1,
                    v_bounds: None,
                    line_pieces: Vec::with_capacity(0),
                    data_type: DataType::Image,
                    image: self.image,
                    image_width: self.image_width,
                    image_height: self.image_height,
                }
            },
        }
    }
}

impl UserData {
    pub fn new_text(text: String) -> Self {
        Self {
            text,
            font: Font::Helvetica,
            font_size: 14,
            fg_color: Color::White,
            bg_color: None,
            underline: false,
            clickable: false,
            expired: false,
            data_type: DataType::Text,
            image: None,
            image_width: 0,
            image_height: 0,
        }
    }

    pub fn new_image(image: Vec<u8>, width: i32, height: i32) -> Self {
        Self {
            text: String::new(),
            font: Font::Helvetica,
            font_size: 14,
            fg_color: Color::White,
            bg_color: None,
            underline: false,
            clickable: false,
            expired: false,
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

    pub fn set_underline(mut self) -> Self {
        self.underline = true;
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

/// 绘制信息单元。
#[derive(Debug)]
pub(crate) struct RichData {
    text: String,
    pub font: Font,
    pub font_size: i32,
    fg_color: Color,
    bg_color: Option<Color>,
    underline: bool,
    clickable: bool,
    expired: bool,
    pub line_height: i32,
    /// 当前内容在面板垂直高度中的起始和截至y坐标，以及起始x坐标。
    v_bounds: Option<(i32, i32, i32)>,

    /// 对当前数据进行试算后，分割成适配单行宽度的分片保存起来。由于无需跨线程传输，因此也不考虑线程安全问题。
    pub(crate) line_pieces:Vec<LinePiece>,
    data_type: DataType,
    image: Option<Vec<u8>>,
    image_width: i32,
    image_height: i32,
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
    pub fn wrap_text_for_estimate(&mut self, text: &str, last_piece: &mut LinePiece, max_width: i32, measure_width: i32, font_height: i32) {
        // let mut last_piece = last_piece;
        // if let Some(lp) = self.line_pieces.last_mut() {
        //     last_piece = lp;
        // }
        let tw = Rc::new(RefCell::new(0));
        let text_len = text.len();
        if let Ok(stop_pos) = (0..text_len).collect::<Vec<usize>>().binary_search_by({
            let x = last_piece.next_x;
            let tw_rc = tw.clone();
            move |pos| {
                let (tw1, _) = measure(text.chars().take(*pos).collect::<String>().as_str(), false);
                if x + tw1 <= max_width {
                    if *pos == text_len - 1 {
                        if x + tw1 == max_width {
                            tw_rc.replace(tw1);
                            Ordering::Equal
                        } else {
                            Ordering::Less
                        }
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
            let max_h = if last_piece.line.ends_with('\n') {
                font_height
            } else {
                max(last_piece.h, font_height)
            };
            let next_y = last_piece.next_y + max_h + last_piece.spacing;

            let y = Rc::new(RefCell::new(last_piece.next_y));
            let through_line = ThroughLine::get_or_create(PADDING.left, last_piece.next_x, max_h, last_piece, false, y.clone());
            let mut new_piece = LinePiece::new(text.chars().take(stop_pos).collect::<String>(), last_piece.next_x, y, w, max_h, last_piece.spacing, next_x, next_y, font_height, through_line);
            self.line_pieces.push(new_piece.clone());

            new_piece.h = font_height;
            let last_piece = &mut new_piece;
            let rest_str = text.chars().skip(stop_pos).collect::<String>();
            let rest_width = measure_width - w;

            if rest_width > max_width {
                // 剩余部分的宽度仍然大于一整行宽度
                self.wrap_text_for_estimate(rest_str.as_str(), last_piece, max_width, rest_width, font_height);
            } else {
                let x = last_piece.next_x;
                let mut next_x = x + rest_width;
                let mut next_y = last_piece.next_y;
                if rest_str.ends_with("\n") {
                    next_x = PADDING.left;
                    next_y += last_piece.h + last_piece.spacing;
                }

                let y = Rc::new(RefCell::new(last_piece.next_y));
                let through_line = ThroughLine::get_or_create(PADDING.left, x, last_piece.h, last_piece, false, y.clone());
                let new_piece = LinePiece::new(rest_str, x, y, rest_width, last_piece.h, last_piece.spacing, next_x, next_y, font_height, through_line);
                self.line_pieces.push(new_piece);
            }
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

    fn draw(&mut self, offset_y: i32) {
        match self.data_type {
            DataType::Text => {
                let bg_offset = 1;
                let mut line_offset = 2;
                #[cfg(not(target_os = "windows"))]
                {
                    line_offset = 4;
                }

                set_font(self.font, self.font_size);
                for piece in self.line_pieces.iter_mut() {
                    let y = *piece.y.borrow();
                    if piece.line.starts_with("24") {
                        debug_assert!(true);
                    }
                    let draw_h = piece.through_line.borrow().max_h;
                    let (up, _) = if draw_h == piece.font_height { (0, 0) } else { calc_v_center_offset(draw_h, piece.font_height) };
                    // println!("up: {}, draw_h: {}, piece.font_height: {}, piece.h: {}", up, draw_h, piece.font_height, piece.h);
                    if let Some(bg_color) = &self.bg_color {
                        // 绘制背景色
                        set_draw_color(*bg_color);
                        draw_rounded_rectf(piece.x, y - offset_y - piece.spacing + up + bg_offset, piece.w, piece.font_height, 4);
                    }

                    set_draw_color(self.fg_color);
                    if self.underline {
                        // 绘制下划线
                        let line_y = y - offset_y + self.font_size + up + line_offset;
                        draw_line(piece.x, line_y, piece.x + piece.w, line_y);
                    }

                    // 绘制文本
                    draw_text2(piece.line.as_str(), piece.x, y - offset_y + bg_offset, piece.w, piece.h, Align::Left);
                }
            },
            DataType::Image => {
                if let Some(piece) = self.line_pieces.last_mut() {
                    if let Some(img) = &self.image {
                        if let Err(e) = draw_image(img.as_slice(), piece.x, *piece.y.borrow() - offset_y, piece.w, piece.h, ColorDepth::Rgb8) {
                            eprintln!("draw image error: {:?}", e);
                        }
                    }
                }
            },
        }
    }

    /// 试算当前内容绘制后所占高度信息。
    /// 试算逻辑考虑了窗口宽度自动换行的情形。
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
    fn estimate(&mut self, last_piece: &mut LinePiece, max_width: i32) {
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

                        if !last_piece.line.ends_with('\n') && current_line_height > last_piece.h {
                            last_piece.through_line.borrow_mut().set_max_h(current_line_height);
                            // last_piece.h = last_piece.through_line.borrow().max_h;
                        }

                        let mut next_x = last_piece.next_x + tw;
                        if next_x > max_width {
                            // 超出横向右边界
                            self.wrap_text_for_estimate(line, last_piece, max_width, tw, ref_font_height);
                        } else {
                            if let Some(lp) = self.line_pieces.last_mut() {
                                let mut next_y = lp.next_y;
                                // 最后一段可能带有换行符'\n'。
                                if line.ends_with("\n") {
                                    next_y += current_line_height;
                                    next_x = PADDING.left;
                                }
                                let y = Rc::new(RefCell::new(lp.next_y));
                                let through_line = ThroughLine::get_or_create(PADDING.left, lp.next_x, current_line_height, lp, false, y.clone());
                                let new_piece = LinePiece::new(line.to_string(), lp.next_x, y, tw, current_line_height, lp.spacing, next_x, next_y, ref_font_height, through_line);
                                self.line_pieces.push(new_piece);
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
                                let y = Rc::new(RefCell::new(last_piece.next_y));
                                let through_line = ThroughLine::get_or_create(PADDING.left, last_piece.next_x, current_line_height, last_piece, false, y.clone());
                                let new_piece = LinePiece::new(line.to_string(), last_piece.next_x, y, tw, current_line_height, last_piece.spacing, next_x, next_y, ref_font_height, through_line);
                                self.line_pieces.push(new_piece);
                            }
                        }
                    }

                } else {
                    let (_, th) = measure("A", false);
                    let mut current_line_height = max(ref_font_height, th);
                    self.line_height = current_line_height;

                    // 如果当前分片与上一个分片在同一行绘制，但是行高不同时，取最大的行高作为本行统一行高标准。
                    if !last_piece.line.ends_with("\n") && !last_piece.line.is_empty() {
                        current_line_height = max(last_piece.h, current_line_height);
                    }

                    if current_line_height > last_piece.h {
                        last_piece.through_line.borrow_mut().set_max_h(current_line_height);
                        // last_piece.h = last_piece.through_line.borrow().max_h;
                    }

                    let line = text.as_str();
                    let (tw, _) = measure(line, false);
                    if last_piece.next_x + tw > max_width {
                        // 超出横向右边界
                        self.wrap_text_for_estimate(line, last_piece, max_width, tw, ref_font_height);
                    } else {
                        let y = Rc::new(RefCell::new(top_y));
                        let through_line = ThroughLine::get_or_create(PADDING.left, start_x, current_line_height, last_piece, false, y.clone());
                        let new_piece = LinePiece::new(self.text.clone(), start_x, y, tw, current_line_height, current_line_spacing, start_x + tw, top_y, ref_font_height, through_line);
                        self.line_pieces.push(new_piece);
                    }
                }
            }
            DataType::Image => {
                let h = self.image_height + IMAGE_PADDING_V * 2;
                if start_x + self.image_width > max_width {
                    // 本行超宽，直接定位到下一行
                    let x = PADDING.left + IMAGE_PADDING_H;
                    let next_x = x + self.image_width + IMAGE_PADDING_H;
                    let next_y = top_y + last_piece.through_line.borrow().max_h + IMAGE_PADDING_V;
                    let y = Rc::new(RefCell::new(next_y));
                    let through_line = ThroughLine::new(self.image_height * IMAGE_PADDING_V * 2, true, y.clone());
                    let new_piece = LinePiece::new("".to_string(), x, y, self.image_width, self.image_height, last_piece.spacing, next_x, next_y, 1, through_line);
                    self.line_pieces.push(new_piece);
                } else {
                    let x = start_x + IMAGE_PADDING_H;
                    let next_x = start_x + self.image_width + IMAGE_PADDING_H * 2;
                    if last_piece.line.ends_with("\n") {
                        // 定位在行首
                        let y = Rc::new(RefCell::new(top_y + IMAGE_PADDING_V));
                        let through_line = ThroughLine::new(self.image_height * IMAGE_PADDING_V * 2, true, y.clone());
                        let new_piece = LinePiece::new("".to_string(), x, y, self.image_width, self.image_height, last_piece.spacing, next_x, top_y, 1, through_line);
                        self.line_pieces.push(new_piece);
                    } else {
                        // 在本行已有其他内容，需要与前一个片段协调行高
                        let current_line_height = max(last_piece.h, h);
                        let mut raw_y = top_y + IMAGE_PADDING_V;
                        if current_line_height > last_piece.h {
                            // 图形比前一个分片行高要高
                            last_piece.through_line.borrow_mut().set_max_h(current_line_height);
                            // last_piece.h = last_piece.through_line.borrow().max_h;
                        } else {
                            // 图形的高度小于等于前一个分片的行高，需要计算垂直居中位置
                            let (up, _) = calc_v_center_offset(current_line_height, h);
                            raw_y += up;
                        }
                        let y = Rc::new(RefCell::new(raw_y));
                        let through_line = ThroughLine::get_or_create(PADDING.left + IMAGE_PADDING_H, x, self.image_height * IMAGE_PADDING_V * 2, last_piece, true, y.clone());
                        let new_piece = LinePiece::new("".to_string(), x, y, self.image_width, self.image_height, last_piece.spacing, next_x, top_y + IMAGE_PADDING_V, 1, through_line);
                        self.line_pieces.push(new_piece);
                    }
                }
            }
        }

        let mut bottom_y = top_y;
        if let Some(last_piece) = self.line_pieces.last_mut() {
            bottom_y = *last_piece.y.borrow() + last_piece.through_line.borrow().max_h;
        }
        self.set_v_bounds(top_y, bottom_y, start_x);
    }

    fn erase(&mut self) {
        todo!()
    }

    fn truncate(&mut self, rtl: bool, length: Option<i32>) -> LinePiece {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;
    use fltk::enums::Font;
    use super::*;


    #[test]
    pub fn test_estimate() {
        // let rich_text = RichData::new_text("安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。".to_string());
        let padding = Padding::new(5, 5, 5, 5);
        let mut rich_text: RichData = UserData::new_text("asdfh\nasdf\n".to_string()).into();
        let from_y = Rc::new(RefCell::new(5));
        let mut last_piece = LinePiece {
            line: "".to_string(),
            x: 5,
            y: from_y,
            w: 0,
            h: 0,
            spacing: 0,
            next_x: 5,
            next_y: 5,
            font_height: 1,
            through_line: Rc::new(RefCell::new(Default::default())),
        };
        rich_text.estimate(&mut last_piece, 785);
        println!("last_line: {:?}", last_piece);
        let increased_height = rich_text.height();
        println!("increased_height: {}", increased_height);

        let mut rich_text: RichData = UserData::new_text("asdfh\nasdf\n".to_string()).set_font(Font::HelveticaBold, 32).into();
        rich_text.estimate(&mut last_piece, 785);
        println!("last_line: {:?}", last_piece);
        let increased_height = rich_text.height();
        println!("increased_height: {}", increased_height);

        let mut rich_text: RichData = UserData::new_text("asdfh\nasdf\n".to_string()).set_font(Font::HelveticaBold, 16).into();
        rich_text.estimate(&mut last_piece, 785);
        println!("last_line: {:?}", last_piece);
        let increased_height = rich_text.height();
        println!("increased_height: {}", increased_height);
    }

    #[test]
    pub fn test_str() {
        let text = "0123456789";
        let t1 = &text[0..4];
        println!("t1: {:?}", t1);

        let mut tw = 0;
        let chars = text.chars();
        let seek = 6;
        if let Ok(p) = (0..(text.len() - 1)).collect::<Vec<usize>>().binary_search_by({
            move |pos| {
                println!("pos: {:?}", pos);
                tw = *pos;
                if pos <= &seek && pos + 1 > seek {

                    Ordering::Equal
                } else if pos > &seek {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            }
        }) {
            println!("p: {}, tw: {}", p, tw);
        } else {
            println!("tw: {:?}", tw);
        }
    }
}
