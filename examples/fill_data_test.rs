use std::time::Duration;
use fltk::{app, window};
use fltk::enums::{Color, Font};
use fltk::image::SharedImage;
use fltk::prelude::{GroupExt, ImageExt, WidgetExt, WindowExt};
use log::debug;
use fltkrs_richdisplay::rich_reviewer::RichReviewer;
use fltkrs_richdisplay::UserData;

pub enum GlobalMessage {
    FillData,
    Clear,
    AfterClear
}

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();
    let app = app::App::default();
    let mut win = window::Window::default()
        .with_size(1800, 1000)
        .with_label("rich-display fill data example")
        .center_screen();
    win.make_resizable(true);

    let mut reviewer = RichReviewer::new(100, 60, 1600, 800, None).history_mode();
    // reviewer.set_background_color(Color::Black);

    win.end();
    win.show();

    let mut data_buffer = Vec::<UserData>::new();
    let (global_sender, global_receiver) = app::channel::<GlobalMessage>();

    let img1 = SharedImage::load("res/1.jpg").unwrap();
    let (img1_width, img1_height, img1_data) = (img1.width(), img1.height(), img1.to_rgb_data());
    let img2 = SharedImage::load("res/2.jpg").unwrap();
    let (img2_width, img2_height, img2_data) = (img2.width(), img2.height(), img2.to_rgb_data());
    for i in 0..10 {
        let turn = i * 13;
        let mut data: Vec<UserData> = Vec::from([
            UserData::new_text(format!("{}安全并且高效地处理𝄞并发编程是Rust的另一个主要目标。💖并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程🐉允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 1)).set_underline(true).set_font(Font::Helvetica, 38).set_bg_color(Some(Color::DarkYellow)).set_clickable(true),
            UserData::new_text(format!("{}在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程。类似地，程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。", turn + 2)).set_font(Font::HelveticaItalic, 18).set_bg_color(Some(Color::Green)),
            UserData::new_image(img1_data.clone(), img1_width, img1_height),
            UserData::new_text(format!("{}由于多线程可以同时运行，🐉所以将计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 3)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)).set_underline(true),
            UserData::new_text(format!("{}由于多线程可以同时运行，所以将计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 4)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
            UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。\r\n", turn + 5)).set_font(Font::Helvetica, 9).set_underline(true).set_blink(true),
            UserData::new_text(format!("{}在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程b。类似地，𝄞程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。\r\n", turn + 6)).set_font(Font::Helvetica, 32),
            UserData::new_text(format!("{}由于多线程可以同时运行，所以将计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 7)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
            UserData::new_text(format!("{}由于多线程可以同时运行，所以将计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 8)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
            UserData::new_image(img1_data.clone(), img1_width, img1_height).set_clickable(true),
            UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 9)).set_fg_color(Color::Yellow).set_bg_color(Some(Color::DarkBlue)),
            UserData::new_text(format!("{}在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程。类似地，程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。\r\n", turn + 10)).set_font(Font::HelveticaBold, 32).set_bg_color(Some(Color::Magenta)).set_clickable(true),
            UserData::new_text(format!("{}由于多线程可以同时运行，所以将计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 11)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
            UserData::new_text(format!("{}由于多线程可以同时运行，所以将计算操作拆分至多个线程可以提高性能。", turn + 12)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)).set_clickable(true),
            UserData::new_text(format!("{}由于多线程可以同时运行，💖所以将计算操作拆分至多个线程可以提高性能。", turn + 13)).set_fg_color(Color::Cyan).set_font(Font::Courier, 18).set_clickable(true).set_blink(true),
            UserData::new_image(img2_data.clone(), img2_width, img2_height).set_clickable(true).set_blink(true),
        ]);
        data.reverse();
        while let Some(data_unit) = data.pop() {
            data_buffer.push(data_unit);
        }
    }

    // 模拟上层应用调用
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        global_sender.send(GlobalMessage::FillData);
        debug!("Sender closed");

        tokio::time::sleep(Duration::from_secs(5)).await;
        global_sender.send(GlobalMessage::Clear);
        global_sender.send(GlobalMessage::AfterClear);
    });

    while app.wait() {
        if let Some(msg) = global_receiver.recv() {
            match msg {
                GlobalMessage::FillData => {
                    // 更新数据段状态
                    reviewer.fill(&mut data_buffer);
                }
                GlobalMessage::Clear => {
                    // 清空数据段状态
                    reviewer.clear();
                }
                GlobalMessage::AfterClear => {
                    let mut ud = vec![UserData::new_text(format!("--已清屏--")).set_fg_color(Color::Light1).set_font(Font::Courier, 12)];
                    reviewer.fill(&mut ud);
                }
            }
        }

        app::sleep(0.001);
        app::awake();
    }
}