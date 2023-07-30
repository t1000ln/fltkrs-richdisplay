//! 富文本编辑器组件。



use std::cell::RefCell;
use std::cmp::{min, Ordering};
use std::collections::VecDeque;
use std::rc::Rc;
use fltk::draw::{descent, draw_line, draw_rect_fill, draw_rounded_rectf, draw_text2, measure, set_draw_color, set_font};
use fltk::enums::{Align, Color, Event, Font};
use fltk::frame::Frame;
use fltk::group::{Scroll, ScrollType};
use fltk::prelude::{GroupExt, WidgetBase, WidgetExt};
use fltk::widget_extends;

#[derive(Debug, Clone)]
pub struct Coordinates(i32, i32, i32, i32);



#[derive(Debug, Clone)]
pub struct Padding {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}
impl Default for Padding {
    fn default() -> Self {
        Self {
            left: 1,
            top: 1,
            right: 1,
            bottom: 1,
        }
    }
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
    pub padding: Padding,
    pub line_no: usize,
    pub line_count: usize,
}

impl LineCoord {
    /// 计算换行操作，增加行号计数。
    pub fn next_line(&mut self, total_height: Rc<RefCell<i32>>) {
        self.line_no += 1;
        self.next_line_for_wrap(total_height);

        // 恢复行高到默认值，使得下一个行可以应用自己的行高。
        self.line_height = 1;
    }

    /// 计算换行操作，用于文本数据超宽时的计算。
    pub fn next_line_for_wrap(&mut self, total_height: Rc<RefCell<i32>>) {
        self.x = self.padding.left;
        self.y += self.line_height;
        *total_height.borrow_mut() += self.line_height;
    }
}

pub trait LinedData: Ord + Clone {
    /// 获取当前数据段所在行号。
    ///
    /// # Arguments
    ///
    /// returns: u32
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn get_line_no(&self) -> usize;

    /// 设置当前数据段所在行号。
    ///
    /// # Arguments
    ///
    /// * `line_no` - 当前数据段所在行号。
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn set_line_no(&mut self, line_no: usize);

    fn get_col_no(&self) -> u16;

    fn set_col_no(&mut self, col_no: u16);

    /// 设置起始坐标原点。
    ///
    /// 注意：文字从坐标原点向右绘制，并从y点向上绘制。图形则是从y点向下绘制。
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
    fn set_start_point(&mut self, start_point: LineCoord);

    // /// 获取当前数据段结束位置右下角坐标。
    // fn get_end_point(&self) -> Coord<i32>;

    /// 指示当前数据绘制过程中是否溢出右边界，进行了换行处理。
    fn wrapped(&self) -> bool;

    /// 获取数据段的矩形占位，从左上角到右下角，若出现换行绘制，则可能有多个矩形区域。
    fn get_bounds(&self) -> &Vec<Coordinates>;

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
    fn is_visible(&self, view_bounds: Coordinates) -> bool;

    /// 滚动原点坐标。
    ///
    /// # Arguments
    ///
    /// * `offset_x`:
    /// * `offset_y`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn scroll(&mut self, offset_x: i32, offset_y: i32);

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
    fn draw(&mut self, suggested: &mut LineCoord, max_width: i32, max_height: i32, ctx: Rc<RefCell<i32>>);

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

#[derive(Clone, Debug)]
pub struct RichData {
    text: String,
    font: Font,
    font_size: i32,
    fg_color: Color,
    bg_color: Option<Color>,
    underline: bool,
    clickable: bool,
    expired: bool,
    line_no: usize,
    col_no: u16,
    start_point: Option<LineCoord>,
    bounds: Vec<Coordinates>,
    wrapped: bool,
    data_type: DataType,
    image: Option<Vec<u8>>,
    image_width: i32,
    image_height: i32,
}

impl RichData {
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
            line_no: 0,
            col_no: 0,
            start_point: None,
            bounds: vec![],
            wrapped: false,
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
            line_no: 0,
            col_no: 0,
            start_point: None,
            bounds: vec![],
            wrapped: false,
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
    pub fn wrap_text(&mut self, text: &str, suggested: &mut LineCoord, current_line_height: i32, current_line_spacing: i32, max_width: i32, max_height: i32, measure_width: i32, total_height: Rc<RefCell<i32>>) {
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
        let line_text: String = char_vec.iter().collect();

        if suggested.y + current_line_height > 0 && suggested.y - current_line_spacing < max_height {
            if let Some(bg_color) = &self.bg_color {
                // 绘制背景色
                set_draw_color(*bg_color);
                draw_rounded_rectf(suggested.x, suggested.y - current_line_spacing, measure_width, current_line_height, 4);
            }

            set_draw_color(self.fg_color);
            if self.underline {
                // 绘制下划线
                let line_y = suggested.y + self.font_size;
                draw_line(suggested.x, line_y, suggested.x + measure_width, line_y);
            }

            // 绘制文本
            draw_text2(line_text.as_str(), suggested.x, suggested.y, measure_width, self.font_size, Align::Left);
        }


        suggested.next_line_for_wrap(total_height.clone());

        /*
        处理剩余部分内容。
         */
        remaining_vec.reverse();
        let remaining_text: String = remaining_vec.iter().collect();
        let rt = remaining_text.as_str();
        let (rw, _) = measure(rt, false);
        if rw > max_width {
            // 余下部分内容仍超出整行宽度，继续进行换行处理
            self.wrap_text(rt, suggested, current_line_height, current_line_spacing, max_width, max_height, rw, total_height);
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
                self.line_no = suggested.line_no;
                suggested.next_line(total_height);
            } else {
                suggested.x += rw;
            }
        }
    }
}

impl Ord for RichData {
    fn cmp(&self, other: &Self) -> Ordering {
        let o = self.line_no.cmp(&other.line_no);
        if o != Ordering::Equal {
            o
        } else {
            self.col_no.cmp(&other.col_no)
        }
    }
}

impl Eq for RichData {}

impl PartialEq<RichData> for RichData {
    fn eq(&self, other: &Self) -> bool {
        self.line_no == other.line_no && self.col_no == other.col_no && self.text.eq(&other.text)
    }
}

impl PartialOrd<RichData> for RichData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let o = self.line_no.cmp(&other.line_no);
        if o != Ordering::Equal {
            Some(o)
        } else {
            let o2 = self.col_no.cmp(&other.col_no);
            if o2 != Ordering::Equal {
                Some(o2)
            } else {
                self.text.partial_cmp(&other.text)
            }
        }
    }
}


impl LinedData for RichData {
    fn get_line_no(&self) -> usize {
        self.line_no
    }

    fn set_line_no(&mut self, line_no: usize) {
        self.line_no = line_no;
    }

    fn get_col_no(&self) -> u16 {
        self.col_no
    }

    fn set_col_no(&mut self, col_no: u16) {
        self.col_no = col_no;
    }

    fn set_start_point(&mut self, start_point: LineCoord) {
        self.start_point = Some(start_point);
    }


    fn wrapped(&self) -> bool {
        self.wrapped
    }

    fn get_bounds(&self) -> &Vec<Coordinates> {
        &self.bounds
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

    fn is_visible(&self, view_bounds: Coordinates) -> bool {
        todo!()
    }

    fn scroll(&mut self, offset_x: i32, offset_y: i32) {
        if let Some(sp) = &mut self.start_point {
            sp.x += offset_x;
            sp.y += offset_y;
        }
    }

    fn draw(&mut self, suggested: &mut LineCoord, max_width: i32, max_height: i32, total_height: Rc<RefCell<i32>>) {
        set_font(self.font, self.font_size);
        let font_height = (self.font_size as f32 * 1.3).ceil() as i32;

        let (_, th) = measure(self.text.as_str(), false);
        let current_line_height = th;
        if current_line_height > suggested.line_height {
            suggested.line_height = current_line_height;
        }
        let current_line_spacing = min(suggested.line_spacing, descent());
        suggested.line_spacing = current_line_spacing;

        let text = self.text.replace("\r", "");
        text.split_inclusive("\n").for_each(|line| {
            let (tw, th) = measure(line, false);
            // let current_line_height = max(th, font_height);

            if suggested.x + tw > max_width {
                // 超出横向右边界
                self.wrapped = true;
                self.wrap_text(line, suggested, current_line_height, current_line_spacing, max_width, max_height, tw, total_height.clone());
            } else {
                if suggested.y + current_line_height > 0 && suggested.y - current_line_spacing < max_height {
                    if let Some(bg_color) = &self.bg_color {
                        // 绘制背景色
                        set_draw_color(*bg_color);
                        draw_rounded_rectf(suggested.x, suggested.y - current_line_spacing, tw, current_line_height, 4);
                    }

                    set_draw_color(self.fg_color);
                    if self.underline {
                        // 绘制下划线
                        let line_y = suggested.y + self.font_size;
                        draw_line(suggested.x, line_y, suggested.x + tw, line_y);
                    }

                    // 绘制文本
                    draw_text2(line, suggested.x, suggested.y, tw, self.font_size, Align::Left);
                }

                if line.ends_with("\n") {
                    /*
                    为当前处理的行数据设置行号。这个行号是对整体数据流而言，并非窗口上看到的行。因为窗口上的行会跟随窗口宽度调整而变化。
                    只有遇到数据中包含的换行符'\n'才会增加行标号。
                     */
                    self.set_line_no(suggested.line_no);
                    suggested.next_line(total_height.clone());
                } else {
                    suggested.x += tw;
                }
            }
        });
    }



    fn erase(&mut self) {
        todo!()
    }

    fn truncate(&mut self, rtl: bool, length: Option<i32>) -> LineCoord {
        todo!()
    }
}

pub struct RichText {
    inner: Scroll,
    panel: Frame,
    // data_buffer: Arc<Mutex<VecDeque<RichData>>>,
    data_buffer: Rc<RefCell<VecDeque<RichData>>>,
    background_color: Rc<RefCell<Color>>,
    padding: Rc<RefCell<Padding>>,
    total_height: Rc<RefCell<i32>>
}
widget_extends!(RichText, Scroll, inner);


impl RichText {
    pub const SCROLL_BAR_WIDTH: i32 = 10;
    pub const PANEL_MAX_HEIGHT: i32 = 10000;
    pub fn new<T>(x: i32, y: i32, w: i32, h: i32, title: T) -> Self
        where T: Into<Option<&'static str>> + Clone {
        let mut inner = Scroll::new(x, y, w, h, title);
        inner.set_type(ScrollType::Vertical);
        inner.set_scrollbar_size(Self::SCROLL_BAR_WIDTH);

        // let mut panel = Frame::new(x, y, w - Self::SCROLL_BAR_WIDTH, h - Self::SCROLL_BAR_WIDTH, None);
        let mut panel = Frame::default().size_of_parent().center_of_parent();

        inner.end();
        inner.resizable(&panel);
        inner.set_clip_children(true);

        let padding = Rc::new(RefCell::new(Padding::default()));

        let total_height = Rc::new(RefCell::new(0));
        // 缓存面板高度的当前值和历史值，用于辅助检测面板高度是否发生变化。
        // let panel_last_height = Rc::new(RefCell::new(0));
        // let panel_current_height = Rc::new(RefCell::new(0));

        let buffer: VecDeque<RichData> = VecDeque::with_capacity(200);
        // let data_buffer = Arc::new(Mutex::new(buffer));
        let data_buffer = Rc::new(RefCell::new(buffer));

        let background_color = Rc::new(RefCell::new(Color::Black));
        let bg_rc = background_color.clone();
        let data_buffer_rc = data_buffer.clone();

        panel.draw({
            let mut scroll_rc = inner.clone();
            let padding_rc = padding.clone();
            // let panel_current_height_rc = panel_current_height.clone();
            let total_height_rc = total_height.clone();
            move |ctx| {
                let base_y = scroll_rc.yposition();
                let window_width = scroll_rc.width();
                let window_height = scroll_rc.height();
                let drawable_max_width = window_width - padding_rc.borrow().left - padding_rc.borrow().right;
                let drawable_max_height = window_height - padding_rc.borrow().top - padding_rc.borrow().bottom;
                // 填充黑色背景
                draw_rect_fill(0, 0, window_width, window_height, *bg_rc.borrow());

                let mut data = data_buffer_rc.borrow_mut();
                let mut suggested = LineCoord {
                    x: padding_rc.borrow().left,
                    y: padding_rc.borrow().top - base_y,
                    line_height: 0,
                    line_spacing: 0,
                    padding: padding_rc.borrow().clone(),
                    line_no: 0,
                    line_count: 0
                };
                *total_height_rc.borrow_mut() = padding_rc.borrow().top;
                for (seq, rich_data) in data.iter_mut().enumerate() {
                    // rich_data.set_line_no(seq);
                    if seq == 0 {
                        set_font(rich_data.font, rich_data.font_size);
                        suggested.line_spacing = descent();
                        suggested.y += suggested.line_spacing;
                    }
                    rich_data.draw(&mut suggested, drawable_max_width, drawable_max_height, total_height_rc.clone());
                }

                let content_height = *total_height_rc.borrow() + padding_rc.borrow().bottom;
                if content_height > ctx.height() {
                    ctx.resize(ctx.x(), ctx.y(), ctx.width(), content_height);
                    // *panel_current_height_rc.borrow_mut() = content_height;
                }
            }
        });

        // panel.handle({
        //     let panel_current_height_rc = panel_current_height.clone();
        //     let total_height_rc = total_height.clone();
        //     move |ctx, evt| {
        //         match evt {
        //             Event::Resize => {
        //                 // *panel_current_height_rc.borrow_mut() = ctx.height();
        //                 println!("total height: {}, panel height: {}", *total_height_rc.borrow(), ctx.height());
        //             }
        //             _ => {}
        //         }
        //         false
        //     }
        // });

        /*
        跟随新增行自动滚动到最底部。
         */
        inner.handle({
            // let panel_last_height_rc = panel_last_height.clone();
            // let panel_current_height_rc = panel_current_height.clone();
            let total_height_rc = total_height.clone();
            let padding_rc = padding.clone();
            move |scroll, evt| {
                match evt {
                    Event::NoEvent => {
                        // let last_height = *panel_last_height_rc.borrow();
                        // let current_height = *panel_current_height_rc.borrow();
                        // if last_height != current_height {
                        //     scroll.scroll_to(0, *total_height_rc.borrow() - scroll.height());
                        //     *panel_last_height_rc.borrow_mut() = current_height;
                        // }
                        if *total_height_rc.borrow() > scroll.height() {
                            scroll.scroll_to(0, *total_height_rc.borrow() - scroll.height() + padding_rc.borrow().bottom);
                        }
                    }
                    _ => {}
                }
                false
            }
        });

        // panel.handle({
        //     let mut scroll_rc = inner.clone();
        //     move |ctx, evt| {
        //         match evt {
        //             // Event::Resize => {
        //             //     println!("resize, height {}", ctx.height());
        //             //     // scroll_rc.scroll_to(0, ctx.height());
        //             // }
        //             _ => {
        //                 println!("event {:?}", evt);
        //             }
        //         }
        //         false
        //     }
        // });

        Self { inner, panel, data_buffer, background_color, padding, total_height }
    }

    pub fn append(&mut self, rich_data: RichData) {
        // println!("append {:?}", rich_data);
        self.data_buffer.borrow_mut().push_back(rich_data);
        self.inner.redraw();
        // println!("current panel height {}", self.panel.height());
    }

    // pub fn set_message_receiver(&mut self, mut data_receiver: Receiver<RichData>, global_sender: channel::Sender<GlobalMessage>) {
    //     let mut buffer = self.data_buffer.clone();
    //     let mut panel = self.panel.clone();
    //     tokio::spawn(async move {
    //         while let Some(data) = data_receiver.recv().await {
    //             println!("Received data: {:?}", data.text);
    //             {
    //                 if let Ok(mut dvr) = buffer.lock() {
    //                     dvr.push_back(data);
    //                 }
    //             }
    //             global_sender.send(GlobalMessage::UpdatePanel);
    //         }
    //         println!("Receiver closed");
    //     });
    // }

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
        self.padding.replace(padding);

    }

    pub fn scroll_to_bottom(&mut self) {
        let mut y = self.panel.height();
        y = y - self.inner.height() + self.padding.borrow().bottom;
        println!("scroll to bottom, height {}, y {}", self.panel.height(), y);
        self.inner.scroll_to(0, y);
    }
}

pub enum GlobalMessage {
    UpdatePanel,
    ScrollToBottom,
    ContentData(RichData),
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
                let mut data: VecDeque<RichData> = VecDeque::from([
                    RichData::new_text("安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。\r\n".to_string()),
                    RichData::new_text("在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程。类似地，程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。\r\n".to_string()).set_font(Font::HelveticaItalic, 32),
                    RichData::new_text("由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                    RichData::new_text("由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                    RichData::new_text("安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。\r\n".to_string()),
                    RichData::new_text("在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程。类似地，程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。\r\n".to_string()).set_font(Font::HelveticaItalic, 32),
                    RichData::new_text("由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                    RichData::new_text("由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                    RichData::new_text("安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。\r\n".to_string()),
                    RichData::new_text("在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程。类似地，程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。\r\n".to_string()).set_font(Font::HelveticaItalic, 32),
                    RichData::new_text("由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                    RichData::new_text("由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
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
