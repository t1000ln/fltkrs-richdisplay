use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;
use fast_log::filter::ModuleFilter;
use fltk::{app, window};
use fltk::button::Button;
use fltk::enums::{Color, Font};
use fltk::image::SharedImage;
use fltk::prelude::{GroupExt, ImageExt, WidgetBase, WidgetExt, WindowExt};
use log::{LevelFilter, warn};
use parking_lot::RwLock;
use fltkrs_richdisplay::rich_reviewer::RichReviewer;
use fltkrs_richdisplay::{PageOptions, UserData};

pub enum GlobalMessage {
    Clear,
    AfterClear
}

fn init_log() {
    let filter = ModuleFilter::new();
    // filter.modules.push("mobc".to_string());
    // filter.modules.push("reqwest".to_string());

    fast_log::init(fast_log::Config::new()
        .console()
        .chan_len(Some(100000))
        .level(LevelFilter::Debug)
        .add_filter(filter)
    ).unwrap();
}

#[tokio::main]
async fn main() {
    init_log();

    let app = app::App::default();
    let mut win = window::Window::default()
        .with_size(1800, 1000)
        .with_label("rich-display fill data example")
        .center_screen();
    win.make_resizable(true);

    let page_size = Arc::new(AtomicUsize::new(10));
    let mut btn1 = Button::new(120, 10, 100, 30, "page_size - 10");
    let mut btn2 = Button::new(240, 10, 100, 30, "page_size + 10");

    let mut reviewer = RichReviewer::new(100, 60, 1600, 800, None).lazy_page_mode();
    // reviewer.set_background_color(Color::Dark1);
    reviewer.set_page_size(page_size.load(Relaxed));
    reviewer.set_piece_spacing(5);

    // 设置默认字体和颜色
    reviewer.set_text_font(Font::Times);
    reviewer.set_text_color(Color::Light1);
    reviewer.set_text_size(12);

    btn1.set_callback({
        let page_size_rc = page_size.clone();
        let mut reviewer_rc = reviewer.clone();
        move |_| {
            if page_size_rc.load(Relaxed) >= 10 {
                let new_page_size = page_size_rc.load(Relaxed) - 10;
                page_size_rc.store(new_page_size, Relaxed);
                reviewer_rc.set_page_size(new_page_size);
            }
        }
    });
    btn2.set_callback({
        let page_size_rc = page_size.clone();
        let mut reviewer_rc = reviewer.clone();
        move |_| {
            if page_size_rc.load(Relaxed) <= 100 {
                let new_page_size = page_size_rc.load(Relaxed) + 10;
                page_size_rc.store(new_page_size, Relaxed);
                reviewer_rc.set_page_size(new_page_size);
            }
        }
    });

    win.end();
    win.show();


    let data_buffer = Arc::new(RwLock::new(Vec::<UserData>::new()));

    let img1 = SharedImage::load("res/1.jpg").unwrap().to_rgb().unwrap();
    let (img1_width, img1_height) = (img1.width(), img1.height());
    let img2 = SharedImage::load("res/2.jpg").unwrap().to_rgb().unwrap();
    let (img2_width, img2_height) = (img2.width(), img2.height());

    let mut reversed_buffer: Vec<UserData> = vec![];
    for i in 0..100 {
        let turn = i * 14;
        let mut data: Vec<UserData> = Vec::from([
            UserData::new_text(format!("{}安全并且高效地处理𝄞并发编程是Rust的另一个@主要目标。💖并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程🐉允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 0)).set_bg_color(Some(Color::DarkCyan)),
            UserData::new_text(format!("{}安全并且高效地处理𝄞并发编程是Rust的另一个主要目标。💖并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程🐉允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 1)).set_underline(true).set_font_and_size(Font::Helvetica, 38).set_bg_color(Some(Color::DarkYellow)).set_clickable(true),
            UserData::new_text(format!("{}在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程。类似地，程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。", turn + 2)).set_font_and_size(Font::HelveticaItalic, 18).set_bg_color(Some(Color::Green)),
            UserData::new_image(img1.copy(), img1_width, img1_height, img1_width, img1_height, Some("res/1.jpg".to_string())),
            UserData::new_text(format!("{}由于多线程可以同时运行，🐉所以将计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 3)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)).set_underline(true),
            UserData::new_text(format!("{}由于多线程可以同时运行，所以将计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 4)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
            UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。\r\n", turn + 5)).set_font_and_size(Font::Helvetica, 9).set_underline(true).set_blink(true),
            UserData::new_text(format!("{}在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程b。类似地，𝄞程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。\r\n", turn + 6)).set_font_and_size(Font::Helvetica, 32),
            UserData::new_text(format!("{}由于多线程可以同时运行，所以将计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 7)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
            UserData::new_text(format!("{}由于多线程可以同时运行，所以将计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 8)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
            UserData::new_image(img1.copy(), img1_width, img1_height, img1_width, img1_height, Some("res/1.jpg".to_string())).set_clickable(true),
            UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 9)).set_fg_color(Color::Yellow).set_bg_color(Some(Color::DarkBlue)),
            UserData::new_text(format!("{}在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程。类似地，程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。\r\n", turn + 10)).set_font_and_size(Font::HelveticaBold, 32).set_bg_color(Some(Color::Magenta)).set_clickable(true),
            UserData::new_text(format!("{}由于多线程可以同时运行，所以将计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 11)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
            UserData::new_text(format!("{}由于多线程可以同时运行，所以将计算操作拆分至多个线程可以提高性能。", turn + 12)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)).set_clickable(true),
            UserData::new_text(format!("{}由于多线程可以同时运行，💖所以将计算操作拆分至多个线程可以提高性能。", turn + 13)).set_fg_color(Color::Cyan).set_font_and_size(Font::Courier, 18).set_clickable(true).set_blink(true),
            UserData::new_image(img2.copy(), img2_width, img2_height, img2_width, img2_height, Some("res/2.jpg".to_string())).set_clickable(true).set_blink(true),
        ]);
        data.reverse();
        while let Some(data_unit) = data.pop() {
            reversed_buffer.push(data_unit);
        }
    }
    data_buffer.write().append(&mut reversed_buffer);

    let fetch_page_fn = {
        let data_buffer_rc = data_buffer.clone();
        let mut reviewer_rc = reviewer.clone();
        let page_size_rc = page_size.clone();
        move |opt| {
            let ps = page_size_rc.load(Relaxed);
            match opt {
                PageOptions::NextPage(last_uid) => {
                    if let Ok(last_pos) = data_buffer_rc.read().binary_search_by_key(&last_uid, |d| d.id) {
                        // debug!("找到当前页最后一条数据的索引位置: {}, {}", last_pos, auto_extend);
                        if data_buffer_rc.read().len() > last_pos + 1 {
                            let mut page_data = Vec::<UserData>::with_capacity(ps);
                            for ud in data_buffer_rc.read()[(last_pos + 1)..].iter().take(ps) {
                                page_data.push(ud.clone());
                            }
                            // debug!("载入下一页数据");
                            reviewer_rc.load_page_now(page_data, opt);
                        }
                    } else {
                        warn!("未找到目标数据: {}", last_uid);
                    }
                }
                PageOptions::PrevPage(first_uid) => {
                    if let Ok(first_pos) = data_buffer_rc.read().binary_search_by_key(&first_uid, |d| d.id) {
                        // debug!("找到当前页第一条数据的索引位置: {}", first_pos);
                        if first_pos > 0 {
                            let mut page_data = Vec::<UserData>::with_capacity(ps);
                            let from = if first_pos >= ps {
                                first_pos - ps
                            } else {
                                0
                            };
                            let to = from + ps;
                            for ud in data_buffer_rc.read()[from..to].iter().take(ps) {
                                page_data.push(ud.clone());
                            }
                            // debug!("载入上一页数据");
                            reviewer_rc.load_page_now(page_data, opt);
                        }
                    } else {
                        warn!("未找到目标数据: {}", first_uid);
                    }
                }
            }
        }
    };
    reviewer.set_page_notifier(fetch_page_fn);

    let mut page_data = Vec::<UserData>::with_capacity(page_size.load(Relaxed));
    for ud in data_buffer.read().iter().take(page_size.load(Relaxed)) {
        page_data.push(ud.clone());
    }
    reviewer.load_page_now(page_data, PageOptions::NextPage(0));

    app.run().unwrap();

    if let Ok(w) = fast_log::flush() {
        // 等待日志刷出到磁盘上。
        w.wait();
    }
}