//! 请描述文件用途。

use std::collections::VecDeque;
use std::time::Duration;
use fltk::{app, window};
use fltk::enums::{Color, Font};
use fltk::image::SharedImage;
use fltk::prelude::{GroupExt, ImageExt, WidgetExt, WindowExt};
use fltkrs_richdisplay::rich_text::{GlobalMessage, RichText};
use fltkrs_richdisplay::UserData;


#[tokio::main]
async fn main() {
    let app = app::App::default();
    let mut win = window::Window::default()
        .with_size(800, 400)
        .with_label("draw by notice")
        .center_screen();
    win.make_resizable(true);

    let mut rich_text = RichText::new(0, 0, 800, 400, None).size_of_parent();

    let (global_sender, global_receiver) = app::channel::<GlobalMessage>();

    tokio::spawn(async move {
        let img1 = SharedImage::load("res/1.jpg").unwrap();
        let (img1_width, img1_height, mut img1_data) = (img1.width(), img1.height(), img1.to_rgb_data());
        let img2 = SharedImage::load("res/2.jpg").unwrap();
        let (img2_width, img2_height, mut img2_data) = (img2.width(), img2.height(), img2.to_rgb_data());

        img1_data.shrink_to_fit();
        img2_data.shrink_to_fit();

        for i in 0..1000 {
            let turn = i * 15;
            let mut data: VecDeque<UserData> = VecDeque::from([
                UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 1)).set_underline(true),
                UserData::new_text(format!("{}在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程。类似地，程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。", turn + 2)).set_font(Font::HelveticaItalic, 32),
                UserData::new_text(format!("{}由于多线程可以同时运行3，所有将城中计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 3)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)).set_underline(true),
                UserData::new_text(format!("{}由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 4)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。\r\n", turn + 5)),
                UserData::new_text(format!("{}在大部分现在操作系统中6，执行程序的代码会运行在进程中，操作系统会同时管理多个进程b。类似地，程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。\r\n", turn + 6)).set_font(Font::HelveticaItalic, 32),
                UserData::new_text(format!("{}由于多线程可以同时运行7，所有将城中计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 7)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                UserData::new_text(format!("{}由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 8)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                UserData::new_image(img1_data.clone(), img1_width, img1_height),
                UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。\r\n", turn + 9)),
                UserData::new_text(format!("{}在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程。类似地，程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。\r\n", turn + 10)).set_font(Font::HelveticaItalic, 32),
                UserData::new_text(format!("{}由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 11)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                UserData::new_text(format!("{}由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。", turn + 12)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                UserData::new_text(format!("{}由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。", turn + 13)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                UserData::new_image(img2_data.clone(), img2_width, img2_height),
            ]);
            while let Some(data_unit) = data.pop_front() {
                global_sender.send(GlobalMessage::ContentData(data_unit));
                tokio::time::sleep(Duration::from_millis(30)).await;
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
