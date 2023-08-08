//! 富文本编辑器组件。



use std::cell::RefCell;
use std::cmp::{max, min, Ordering};
use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use fltk::draw::{descent, draw_line, draw_rect_fill, draw_rounded_rectf, draw_text2, measure, set_draw_color, set_font};
use fltk::enums::{Align, Color, Event, Font};
use fltk::frame::Frame;
use fltk::group::{Scroll, ScrollType};
use fltk::prelude::{GroupExt, ValuatorExt, WidgetBase, WidgetExt};
use fltk::{app, widget_extends};

#[derive(Debug, Clone)]
pub struct Coordinates(i32, i32, i32, i32);


#[derive(Debug, Clone, Default)]
pub struct Padding {
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) right: i32,
    pub(crate) bottom: i32,
}

impl Padding {
    pub fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LineCoord {
    pub x: i32,
    pub y: i32,
    pub line_height: i32,
    pub line_spacing: i32,
    pub padding: Arc<Padding>,
}

impl LineCoord {
    /// 计算换行操作，增加行号计数。
    pub fn next_line(&mut self) {
        self.x = self.padding.left;
        self.y += self.line_height + self.line_spacing;
    }

    /// 将行高恢复到最小值。
    pub fn shrink_line_height(&mut self) {
        self.line_height = 1;
    }

    pub fn previous_line(&mut self) {
        self.x = self.padding.left;
        self.y -= self.line_height - self.line_spacing;
    }
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
    /// 额外的行间距
    pub spacing: i32,
    /// 建议下一个数据分片绘制起点x坐标
    pub next_x: i32,
    /// 建议下一个数据分片绘制起点y坐标
    pub next_y: i32,

    /// 字体渲染高度，不同于行高。
    pub font_height: i32,
}

impl LinePiece {
    pub fn new(line: String, x: i32, y: i32, w: i32, h: i32, spacing: i32, next_x: i32, next_y: i32, font_height: i32) -> Self {
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
        }
    }

    pub fn next_line(&mut self, padding: &Padding) {
        self.next_x = padding.left;
        self.next_y = self.y + self.h + self.spacing;
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

    // /// 获取当前数据段结束位置右下角坐标。
    // fn get_end_point(&self) -> Coord<i32>;

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

    fn estimate(&mut self, blow_line: &mut LinePiece, max_width: i32, padding: &Padding);

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
    fn truncate(&mut self, rtl: bool, length: Option<i32>) -> LineCoord;
}

#[derive(Clone, Debug)]
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

    pub fn set_underline(mut self, underline: bool) -> Self {
        self.underline = underline;
        self
    }

    pub fn set_clickable(mut self, clickable: bool) -> Self {
        self.clickable = clickable;
        self
    }
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
    line_pieces:Vec<LinePiece>,
    data_type: DataType,
    image: Option<Vec<u8>>,
    image_width: i32,
    image_height: i32,
}

impl RichData {


    /// 处理超出窗口宽度的行数据。
    ///
    /// # Arguments
    ///
    /// * `text`:
    /// * `suggested`:
    /// * `current_line_height`:
    /// * `current_line_spacing`:
    /// * `max_width`:
    /// * `max_height`:
    /// * `measure_width`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn wrap_text(&mut self, text: &str, suggested: &mut LineCoord, current_line_height: i32, current_line_spacing: i32, max_width: i32, max_height: i32, measure_width: i32) {
        /*
        先对传入内容进行计算，得出可以绘制在边界内的部分内容，将超出边界的部分内容放入缓存中再进行下一步处理。
         */
        let chars = text.chars();
        let mut char_vec: Vec<char> = chars.collect();
        let mut remaining_vec: Vec<char> = Vec::new();
        while let Some(c) = char_vec.pop() {
            remaining_vec.push(c);
            let (tw, _) = measure(char_vec.iter().collect::<String>().as_str(), false);
            if suggested.x + tw <= max_width {
                break;
            }
        }

        // 本行内顶宽文本
        let draw_width = min(max_width - suggested.x, measure_width);
        let line_text: String = char_vec.iter().collect();
        if suggested.y + current_line_height > 0 && suggested.y - current_line_spacing < max_height {
            if let Some(bg_color) = &self.bg_color {
                // 绘制背景色
                set_draw_color(*bg_color);
                draw_rounded_rectf(suggested.x, suggested.y - current_line_spacing, draw_width, current_line_height, 4);
            }

            set_draw_color(self.fg_color);
            if self.underline {
                // 绘制下划线
                let line_y = suggested.y + self.font_size;
                draw_line(suggested.x, line_y, suggested.x + draw_width, line_y);
            }

            // 绘制文本
            draw_text2(line_text.as_str(), suggested.x, suggested.y, draw_width, self.font_size, Align::Left);
        }

        suggested.next_line();

        /*
        处理剩余部分内容。
         */
        remaining_vec.reverse();
        let remaining_text: String = remaining_vec.iter().collect();
        let rt = remaining_text.as_str();
        let (rw, _) = measure(rt, false);
        if rw > max_width {
            // 余下部分内容仍超出整行宽度，继续进行换行处理
            self.wrap_text(rt, suggested, current_line_height, current_line_spacing, max_width, max_height, rw);
        } else {
            // 余下部分未超宽
            if suggested.y + current_line_height > 0 && suggested.y - current_line_spacing < max_height {
                if let Some(bg_color) = &self.bg_color {
                    // 绘制背景色
                    set_draw_color(*bg_color);
                    draw_rounded_rectf(suggested.x, suggested.y - current_line_spacing, rw, current_line_height, 4);
                }

                set_draw_color(self.fg_color);
                if self.underline {
                    // 绘制下划线
                    let line_y = suggested.y + self.font_size;
                    draw_line(suggested.x, line_y, suggested.x + rw, line_y);
                }

                // 绘制文本
                draw_text2(rt, suggested.x, suggested.y, rw, self.font_size, Align::Left);
            }

            if rt.ends_with("\n") {
                suggested.next_line();
            } else {
                suggested.x += rw;
            }
        }
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
    pub fn wrap_text_for_estimate(&mut self, text: &str, last_piece: &mut LinePiece, max_width: i32, padding: &Padding, measure_width: i32, font_height: i32) {
        let mut last_piece = last_piece;
        if let Some(lp) = self.line_pieces.last_mut() {
            last_piece = lp;
        }
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
            let next_x = padding.left;
            let max_h = if last_piece.line.ends_with('\n') {
                font_height
            } else {
                max(last_piece.h, font_height)
            };
            let next_y = last_piece.next_y + max_h + last_piece.spacing;
            let mut new_piece = LinePiece::new(text.chars().take(stop_pos).collect::<String>(), last_piece.next_x, last_piece.next_y, w, max_h, last_piece.spacing, next_x, next_y, font_height);
            self.line_pieces.push(new_piece.clone());

            new_piece.h = font_height;
            let last_piece = &mut new_piece;
            let rest_str = text.chars().skip(stop_pos).collect::<String>();
            let rest_width = measure_width - w;

            if rest_width > max_width {
                // 剩余部分的宽度仍然大于一整行宽度
                self.wrap_text_for_estimate(rest_str.as_str(), last_piece, max_width, padding, rest_width, font_height);
            } else {
                let x = last_piece.next_x;
                let y = last_piece.next_y;
                let mut next_x = x + rest_width;
                let mut next_y = y;
                if rest_str.ends_with("\n") {
                    next_x = padding.left;
                    next_y += last_piece.h + last_piece.spacing;
                }
                let new_piece = LinePiece::new(rest_str, x, y, rest_width, last_piece.h, last_piece.spacing, next_x, next_y, font_height);
                self.line_pieces.push(new_piece);
            }
        }
    }
}

// impl Ord for RichData {
//     fn cmp(&self, other: &Self) -> Ordering {
//         let o = self.line_no.cmp(&other.line_no);
//         if o != Ordering::Equal {
//             o
//         } else {
//             self.col_no.cmp(&other.col_no)
//         }
//     }
// }
//
// impl Eq for RichData {}
//
// impl PartialEq<RichData> for RichData {
//     fn eq(&self, other: &Self) -> bool {
//         self.line_no == other.line_no && self.col_no == other.col_no && self.text.eq(&other.text)
//     }
// }
//
// impl PartialOrd<RichData> for RichData {
//     fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
//         let o = self.line_no.cmp(&other.line_no);
//         if o != Ordering::Equal {
//             Some(o)
//         } else {
//             let o2 = self.col_no.cmp(&other.col_no);
//             if o2 != Ordering::Equal {
//                 Some(o2)
//             } else {
//                 self.text.partial_cmp(&other.text)
//             }
//         }
//     }
// }


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
        set_font(self.font, self.font_size);
        for piece in self.line_pieces.iter_mut() {
            let (up, down) = calc_v_center_offset(piece.h, piece.font_height);
            if let Some(bg_color) = &self.bg_color {
                // 绘制背景色
                set_draw_color(*bg_color);
                draw_rounded_rectf(piece.x, piece.y - offset_y - piece.spacing + up, piece.w, piece.font_height, 4);
            }

            set_draw_color(self.fg_color);
            if self.underline {
                // 绘制下划线
                let line_y = piece.y - offset_y + self.font_size + up + 2;
                draw_line(piece.x, line_y, piece.x + piece.w, line_y);
            }

            // 绘制文本
            draw_text2(piece.line.as_str(), piece.x, piece.y - offset_y, piece.w, piece.h, Align::Left);
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
    fn estimate(&mut self, last_piece: &mut LinePiece, max_width: i32, padding: &Padding) {
        let (top_y, start_x) = (last_piece.next_y, last_piece.next_x);
        set_font(self.font, self.font_size);

        // 字体渲染高度，小于等于行高度。
        let ref_font_height = (self.font_size as f32 * 1.4).ceil() as i32;

        let current_line_spacing = min(last_piece.spacing, descent());
        // last_piece.spacing = current_line_spacing;

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
                    last_piece.h = current_line_height;
                }

                let mut next_x = last_piece.next_x + tw;
                if next_x > max_width {
                    // 超出横向右边界
                    self.wrap_text_for_estimate(line, last_piece, max_width, padding, tw, ref_font_height);
                } else {
                    if let Some(lp) = self.line_pieces.last_mut() {
                        let mut next_y = lp.next_y;
                        // 最后一段可能带有换行符'\n'。
                        if line.ends_with("\n") {
                            next_y += current_line_height;
                            next_x = padding.left;
                        }
                        let new_piece = LinePiece::new(line.to_string(), lp.next_x, lp.next_y, tw, current_line_height, lp.spacing, next_x, next_y, ref_font_height);
                        self.line_pieces.push(new_piece);
                    } else {
                        let mut next_y = last_piece.next_y;
                        // 最后一段可能带有换行符'\n'。
                        if line.ends_with("\n") {
                            if !last_piece.line.ends_with("\n") {
                                current_line_height = max(current_line_height, last_piece.h);
                            }
                            next_y += current_line_height;
                            next_x = padding.left;
                        }
                        let new_piece = LinePiece::new(line.to_string(), last_piece.next_x, last_piece.next_y, tw, current_line_height, last_piece.spacing, next_x, next_y, ref_font_height);
                        self.line_pieces.push(new_piece);
                    }
                }
            }

        } else {
            let (_, th) = measure("A", false);
            let mut current_line_height = max(ref_font_height, th);
            self.line_height = current_line_height;

            // 如果当前分片与上一个分片在同一行绘制，但是行高不同时，取最大的行高作为本行统一行高标准。
            if !last_piece.line.ends_with("\n") {
                current_line_height = max(last_piece.h, current_line_height);
            }
            last_piece.h = current_line_height;

            let line = text.as_str();
            let (tw, _) = measure(line, false);
            if last_piece.next_x + tw > max_width {
                // 超出横向右边界
                self.wrap_text_for_estimate(line, last_piece, max_width, padding, tw, ref_font_height);
            } else {
                self.line_pieces.push(LinePiece::new(self.text.clone(), start_x, top_y, tw, current_line_height, current_line_spacing, start_x + tw, top_y, ref_font_height));
            }
        }

        let mut bottom_y = top_y;
        if let Some(last_piece) = self.line_pieces.last_mut() {
            bottom_y = last_piece.y + last_piece.h;
        }
        self.set_v_bounds(top_y, bottom_y, start_x);
    }

    fn erase(&mut self) {
        todo!()
    }

    fn truncate(&mut self, rtl: bool, length: Option<i32>) -> LineCoord {
        todo!()
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

pub struct RichText {
    scroller: Scroll,
    panel: Frame,
    data_buffer: Rc<RefCell<VecDeque<RichData>>>,
    background_color: Rc<RefCell<Color>>,
    padding: Rc<RefCell<Padding>>,

    /// 可视内容总高度，不含padding高度。
    total_height: Arc<Mutex<i32>>,

    /// 自动滚动到底部的标记
    auto_scroll: Rc<RefCell<bool>>,
}
widget_extends!(RichText, Scroll, scroller);


impl RichText {
    pub const SCROLL_BAR_WIDTH: i32 = 10;
    pub const PANEL_MAX_HEIGHT: i32 = 10;
    pub fn new<T>(x: i32, y: i32, w: i32, h: i32, title: T) -> Self
        where T: Into<Option<&'static str>> + Clone {
        let mut scroller = Scroll::new(x, y, w, h, title);
        scroller.set_type(ScrollType::Vertical);
        scroller.set_scrollbar_size(Self::SCROLL_BAR_WIDTH);

        // let mut panel = Frame::new(x, y, w - Self::SCROLL_BAR_WIDTH, h - Self::SCROLL_BAR_WIDTH, None);
        let mut panel = Frame::default().size_of_parent().center_of_parent();

        scroller.end();
        scroller.resizable(&panel);
        scroller.set_clip_children(true);

        let auto_scroll = Rc::new(RefCell::new(false));

        let padding = Rc::new(RefCell::new(Padding::default()));

        let total_height = Arc::new(Mutex::new(0));
        let data_buffer = Rc::new(RefCell::new(VecDeque::<RichData>::with_capacity(200)));
        let background_color = Rc::new(RefCell::new(Color::Black));

        /*
        利用tokio::spawn的协程特性，在app主线程中异步执行滚动操作。
        在非主线程的其他独立线程中，是无法执行滚动操作的。
         */
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<()>(10);
        let total_height_rc = total_height.clone();
        let mut scroll_rc = scroller.clone();
        tokio::spawn(async move {
            while receiver.recv().await.is_some() {
                let mut current_height = 0;
                if let Ok(height) = total_height_rc.lock() {
                    current_height = *height.deref();
                }
                let scroll_y = current_height - scroll_rc.height();
                scroll_rc.scroll_to(0, scroll_y);
            }
        });

        panel.draw({
            let data_buffer_rc = data_buffer.clone();
            let mut scroll_rc = scroller.clone();
            let padding_rc = padding.clone();
            let bg_rc = background_color.clone();
            let auto_scroll_rc = auto_scroll.clone();
            move |ctx| {
                // 滚动条滚动的高度在0到(panel.height - scroll.height)之间。
                let base_y = scroll_rc.yposition();
                let window_width = scroll_rc.width();
                let window_height = scroll_rc.height();
                let padding = padding_rc.borrow();
                let padding = padding.deref();
                let panel_height = ctx.height();

                let (mut top_y, mut bottom_y) = (base_y, base_y + window_height);

                if auto_scroll_rc.replace(false) {
                    bottom_y = panel_height;
                    top_y = panel_height - window_height;
                    let sender = sender.clone();
                    tokio::spawn(async move {
                        if let Err(e) = sender.send(()).await {
                            println!("发送滚动信号失败！{:?}", e);
                        }
                    });
                }
                let offset_y = top_y - padding.top;

                // 填充黑色背景
                draw_rect_fill(0, 0, window_width, window_height, *bg_rc.borrow());

                let mut data = data_buffer_rc.borrow_mut();

                /*
                先试算出可显示的行，再真正绘制可显示的行。
                试算从数据队列的尾部向头部取数，试算位置从窗口底部向顶部堆积。
                 */
                let (mut from_index, mut to_index, total_len) = (0, data.len(), data.len());
                let mut set_to_index = false;
                let mut begin_check_from_index = false;
                for (seq, rich_data) in data.iter_mut().rev().enumerate() {
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

                for rich_data in data.range_mut(from_index..to_index) {
                    rich_data.draw(offset_y);
                }
            }
        });

        /*
        跟随新增行自动滚动到最底部。
         */
        scroller.handle({
            let mut panel_rc = panel.clone();
            let total_height_rc = total_height.clone();
            let padding_rc = padding.clone();
            let buffer_rc = data_buffer.clone();
            let last_window_size = Rc::new(RefCell::new((0, 0)));
            move |scroller, evt| {
                match evt {
                    Event::Resize => {
                        let (current_width, current_height) = (scroller.width(), scroller.height());
                        let (last_width, last_height) = *last_window_size.borrow();
                        if last_width != current_width || last_height != current_height {
                            last_window_size.replace((current_width, current_height));
                            let window_width = scroller.width();
                            let window_height = scroller.height();
                            let padding = padding_rc.borrow();
                            let padding = padding.deref();
                            let drawable_max_width = window_width - padding.left - padding.right;
                            let mut init_piece = LinePiece {
                                line: "".to_string(),
                                x: padding.left,
                                y: padding.top,
                                w: 0,
                                h: 1,
                                spacing: 0,
                                next_x: padding.left,
                                next_y: padding.top,
                                font_height: 1,
                            };
                            let mut last_piece = &mut init_piece;
                            for rich_data in buffer_rc.borrow_mut().iter_mut() {
                                rich_data.line_pieces.clear();
                                rich_data.estimate(last_piece, drawable_max_width, padding);
                                if let Some(piece) = rich_data.line_pieces.iter_mut().last() {
                                    last_piece = piece;
                                }
                            }

                            if let Some(rich_data) = buffer_rc.borrow().iter().last() {
                                if let Some(piece) = rich_data.line_pieces.last() {
                                    let new_content_height = piece.y + piece.h + piece.spacing;
                                    // total_height_rc.replace(piece.y + piece.h + piece.spacing);
                                    let new_total_height = new_content_height + padding.top + padding.bottom;

                                    if new_total_height > window_height {
                                        panel_rc.resize(panel_rc.x(), scroller.y(), window_width, new_total_height);
                                    } else {
                                        panel_rc.resize(panel_rc.x(), scroller.y(), window_width, window_height);
                                    }

                                    // let yp = scroller.yposition();
                                    // println!("yposition = {}", yp);
                                    // if yp > new_total_height - window_height {
                                    //     let scroll_y = if new_total_height > window_height {
                                    //         new_total_height - window_height
                                    //     } else {
                                    //         0
                                    //     };
                                    //     scroller.scroll_to(0, scroll_y);
                                    // } else if yp < 0 {
                                    //     scroller.scroll_to(0, 0);
                                    // }
                                    // scroller.redraw();

                                    if let Ok(mut height) = total_height_rc.lock() {
                                        *height.deref_mut() = new_content_height;
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


        Self { scroller, panel, data_buffer, background_color, padding, total_height, auto_scroll, }
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
        let window_width = self.scroller.width();
        let window_height = self.scroller.height();
        let drawable_max_width = window_width - self.padding.borrow().left - self.padding.borrow().right;

        let padding = self.padding.borrow();
        let padding = padding.deref();

        /*
        试算单元绘制信息
         */
        if !self.data_buffer.borrow().is_empty() {
            if let Some(rd) = self.data_buffer.borrow_mut().iter_mut().last() {
                if let Some(last_piece) = rd.line_pieces.iter_mut().last() {
                    rich_data.estimate(last_piece, drawable_max_width, padding);
                }
            }
        } else {
            // 首次添加数据
            let mut last_piece = LinePiece {
                line: "".to_string(),
                x: padding.left,
                y: padding.top,
                w: 0,
                h: 1,
                spacing: 0,
                next_x: padding.left,
                next_y: padding.top,
                font_height: 1,
            };
            rich_data.estimate(&mut last_piece, drawable_max_width, padding);
        }

        if let Some(piece) = rich_data.line_pieces.last() {
            let new_content_height = piece.y + piece.h + piece.spacing;
            let new_total_height = new_content_height + padding.top + padding.bottom;
            if new_total_height > window_height {
                self.panel.resize(self.panel.x(), self.y(), self.panel.width(), new_total_height);
                self.auto_scroll.replace(true);
                // let scroll_y = new_height - window_height;
                // self.scroller.scrollbar().set_value(new_height as f64);
                // self.scroller.scroll_to(0, scroll_y);

                if let Ok(mut height) = self.total_height.lock() {
                    *height.deref_mut() = new_content_height;
                }
            }
        }

        self.data_buffer.borrow_mut().push_back(rich_data);

        self.scroller.redraw();
    }

    pub fn scroll_to_bottom(&mut self) {

        // let window_height = self.scroller.height();
        // let new_height = *self.total_height.borrow() + self.padding.borrow().bottom + self.padding.borrow().top;
        // if new_height > window_height {
        //
        //     let scroll_y = new_height - window_height;
        //     println!("滚动 {}", scroll_y);
        //     self.scroller.scroll_to(0, scroll_y);
        //     // self.scroller.redraw();
        // }
    }

    pub fn set_background_color(&mut self, background_color: Color) {
        self.background_color.replace(background_color);
    }

    /// 设置面板内侧边空白。
    ///
    /// # Arguments
    ///
    /// * `padding`: 左、上、右、下。
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn set_padding(&mut self, padding: Padding) {
        let right = padding.right;
        self.padding.replace(padding);
        self.scroller.set_scrollbar_size(right);
    }

}

pub enum GlobalMessage {
    UpdatePanel,
    ScrollToBottom,
    ContentData(UserData),
}


#[cfg(test)]
mod tests {
    use std::time::Duration;

    use fltk::{app, window};
    use fltk::prelude::{GroupExt, WidgetExt, WindowExt};

    use super::*;

    #[tokio::test]
    pub async fn test_run_app() {
        let app = app::App::default();
        let mut win = window::Window::default()
            .with_size(800, 400)
            .with_label("draw by notice")
            .center_screen();
        win.make_resizable(true);

        let mut rich_text = RichText::new(0, 0, 800, 400, None).size_of_parent();
        rich_text.set_padding(Padding::new(5, 5, 5, 5));

        let (global_sender, mut global_receiver) = app::channel::<GlobalMessage>();

        let (data_sender, mut data_receiver) = tokio::sync::mpsc::channel::<RichData>(64);
        // rich_text.set_message_receiver(data_receiver, global_sender);

        tokio::spawn(async move {
            for _ in 0..1000 {
                let mut data: VecDeque<UserData> = VecDeque::from([
                    UserData::new_text("安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。\r\n".to_string()),
                    UserData::new_text("在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程。类似地，程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。\r\n".to_string()).set_font(Font::HelveticaItalic, 32),
                    UserData::new_text("由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                    UserData::new_text("由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                    UserData::new_text("安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。\r\n".to_string()),
                    UserData::new_text("在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程。类似地，程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。\r\n".to_string()).set_font(Font::HelveticaItalic, 32),
                    UserData::new_text("由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                    UserData::new_text("由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                    UserData::new_text("安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。\r\n".to_string()),
                    UserData::new_text("在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程。类似地，程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。\r\n".to_string()).set_font(Font::HelveticaItalic, 32),
                    UserData::new_text("由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                    UserData::new_text("由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                ]);
                while let Some(data_unit) = data.pop_front() {
                    // if let Err(e) = data_sender.send(data_unit).await {
                    //     eprintln!("Error sending data: {:?}", e);
                    // }

                    global_sender.send(GlobalMessage::ContentData(data_unit));
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }

            println!("Sender closed");
        });


        win.end();
        win.show();

        while app.wait() {
            if let Some(msg) = global_receiver.recv() {
                match msg {
                    GlobalMessage::UpdatePanel => {
                        rich_text.redraw();
                        // rich_text.scroll_to_bottom();
                    }
                    GlobalMessage::ContentData(data) => {
                        rich_text.append(data);
                    }
                    _ => {}
                }
            }
            app::sleep(0.016);
            app::awake();
        }
    }


}
