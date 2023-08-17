//! 富文本查看器组件。

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::OnceLock;

use fltk::draw::{draw_rect_fill};
use fltk::enums::{Color, ColorDepth, Cursor, Event};
use fltk::frame::Frame;
use fltk::prelude::{ImageExt, WidgetBase, WidgetExt};
use fltk::{app, draw, widget_extends};
use fltk::image::{RgbImage};
use crate::{Coordinates, DataType, LinedData, LinePiece, PADDING, RichData, RichDataOptions, UserData};

use idgenerator_thin::{IdGeneratorOptions, YitIdHelper};

static ID_GENERATOR_INIT: OnceLock<u8> = OnceLock::new();

pub struct RichText {
    panel: Frame,
    data_buffer: Rc<RefCell<VecDeque<RichData>>>,
    background_color: Rc<RefCell<Color>>,
    buffer_max_lines: usize,
    notifier: Rc<RefCell<Option<tokio::sync::mpsc::Sender<UserData>>>>
}
widget_extends!(RichText, Frame, panel);


impl RichText {
    pub fn new<T>(x: i32, y: i32, w: i32, h: i32, title: T) -> Self
        where T: Into<Option<&'static str>> + Clone {
        let _ = ID_GENERATOR_INIT.get_or_init(|| {
            // 初始化ID生成器。
            let options = IdGeneratorOptions::new(1);
            YitIdHelper::set_id_generator(options);
            0
        });

        let mut panel = Frame::new(x, y, w, h, title);

        let buffer_max_lines = 100;
        let data_buffer = Rc::new(RefCell::new(VecDeque::<RichData>::with_capacity(buffer_max_lines + 1)));
        let background_color = Rc::new(RefCell::new(Color::Black));
        let visible_lines = Rc::new(RefCell::new(HashMap::<Coordinates, usize>::new()));
        let notifier: Rc<RefCell<Option<tokio::sync::mpsc::Sender<UserData>>>> = Rc::new(RefCell::new(None));

        panel.draw({
            let data_buffer_rc = data_buffer.clone();
            let bg_rc = background_color.clone();
            let visible_lines_rc = visible_lines.clone();
            move |ctx| {
                let window_width = ctx.width();
                let window_height = ctx.height();
                let mut offset_y = 0;
                visible_lines_rc.borrow_mut().clear();

                // 填充背景
                draw_rect_fill(0, 0, window_width, window_height, *bg_rc.borrow());

                let data = data_buffer_rc.borrow();

                let mut set_offset_y = false;
                for (idx, rich_data) in data.iter().enumerate().rev() {
                    if let Some((_, bottom_y, _)) = rich_data.v_bounds {
                        if !set_offset_y && bottom_y > window_height {
                            offset_y = bottom_y - window_height + PADDING.bottom;
                            set_offset_y = true;
                        }

                        if bottom_y - offset_y < 0 {
                            break;
                        }
                        rich_data.draw(offset_y, idx, visible_lines_rc.clone());
                    }

                }
            }
        });

        /*
        处理窗口事件
         */
        panel.handle({
            let buffer_rc = data_buffer.clone();
            let last_window_size = Rc::new(RefCell::new((0, 0)));
            let visible_lines_rc = visible_lines.clone();
            let notifier_rc = notifier.clone();
            move |ctx, evt| {
                match evt {
                    Event::Resize => {
                        // 缩放窗口后重新计算分片绘制信息。
                        let (current_width, current_height) = (ctx.width(), ctx.height());
                        let (last_width, last_height) = *last_window_size.borrow();
                        if last_width != current_width || last_height != current_height {
                            last_window_size.replace((current_width, current_height));
                            let drawable_max_width = current_width - PADDING.left - PADDING.right;
                            let mut last_piece = LinePiece::init_piece();
                            for rich_data in buffer_rc.borrow_mut().iter_mut() {
                                rich_data.line_pieces.clear();
                                last_piece = rich_data.estimate(last_piece, drawable_max_width);
                            }
                        }
                    }
                    Event::Move => {
                        // 检测鼠标进入可互动区域，改变鼠标样式
                        let mut enter_piece = false;
                        for area in visible_lines_rc.borrow().keys() {
                            let (x, y, w, h) = area.to_rect();
                            if app::event_inside(x, y, w, h) {
                                enter_piece = true;
                                break;
                            }
                        }
                        if enter_piece {
                            draw::set_cursor(Cursor::Hand);
                        } else {
                            draw::set_cursor(Cursor::Default);
                        }
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
                                                eprintln!("send error: {:?}", e);
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
        });


        Self { panel, data_buffer, background_color, buffer_max_lines, notifier }
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

        self.panel.redraw();
    }

    pub fn set_background_color(&mut self, background_color: Color) {
        self.background_color.replace(background_color);
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
                self.panel.redraw();
            }
        }
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

                self.panel.redraw();
            }
        }
    }
}

pub enum GlobalMessage {
    UpdatePanel,
    ScrollToBottom,
    ContentData(UserData),
    UpdateData(RichDataOptions),
    DisableData(i64),
}


#[cfg(test)]
mod tests {
    use std::time::Duration;

    use fltk::{app, window};
    use fltk::enums::Font;
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

        let (global_sender, mut global_receiver) = app::channel::<GlobalMessage>();

        let (data_sender, data_receiver) = tokio::sync::mpsc::channel::<RichData>(64);

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
