//! 请描述文件用途。

use std::collections::VecDeque;
use std::time::Duration;
use fltk::{app, window};
use fltk::enums::{Color, Font};
use fltk::prelude::{GroupExt, WidgetExt, WindowExt};
use fltkrs_richdisplay::rich_text::{GlobalMessage, Padding, RichData, RichText};


#[tokio::main]
async fn main() {
    let app = app::App::default();
    let mut win = window::Window::default()
        .with_size(800, 400)
        .with_label("draw by notice")
        .center_screen();
    win.make_resizable(true);

    let mut rich_text = RichText::new(0, 0, 800, 400, None).size_of_parent();
    rich_text.set_padding(Padding::new(5, 5, 10, 5));

    let (global_sender, mut global_receiver) = app::channel::<GlobalMessage>();

    // let (data_sender, mut data_receiver) = tokio::sync::mpsc::channel::<RichData>(64);
    // rich_text.set_message_receiver(data_receiver, global_sender);

    tokio::spawn(async move {

        for _ in 0..1 {
            let mut data: VecDeque<RichData> = VecDeque::from([
                RichData::new_text("安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。".to_string()).set_underline(true),
                RichData::new_text("在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程。类似地，程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。".to_string()).set_font(Font::HelveticaItalic, 32),
                RichData::new_text("由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)).set_underline(true),
                RichData::new_text("由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                RichData::new_text("安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。\r\n".to_string()),
                RichData::new_text("在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程b。类似地，程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。\r\n".to_string()).set_font(Font::HelveticaItalic, 32),
                RichData::new_text("由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                RichData::new_text("由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                RichData::new_text("安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。\r\n".to_string()),
                RichData::new_text("在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程。类似地，程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。\r\n".to_string()).set_font(Font::HelveticaItalic, 32),
                RichData::new_text("由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                RichData::new_text("由于多线程可以同时运行，所有将城中计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n".to_string()).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
            ]);
            while let Some(data_unit) = data.pop_front() {
                // if let Err(e) = data_sender.send(data_unit).await {
                //     eprintln!("Error sending data: {:?}", e);
                // }

                global_sender.send(GlobalMessage::ContentData(data_unit));
                tokio::time::sleep(Duration::from_millis(2000)).await;
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
