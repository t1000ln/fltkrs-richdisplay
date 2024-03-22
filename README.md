# fltkrs-richdisplay
![Static Badge](https://img.shields.io/badge/crates-0.4.1-blue) 
![Static Badge](https://img.shields.io/badge/build-passing-green)
![Static Badge](https://img.shields.io/badge/Readonly-gray)


这是一个基于`fltk-rs`的富文本展示组件，可用于展示文本和图片，不支持编辑操作。 该组件的设计目标是提供更多的文本样式展示，支持文、图混合，主要的展示形式以行为主，从上向下、从左到右的流式排列。 

组件支持的主要功能：
- 支持不同字体系列，粗体、斜体、颜色、背景色、下划线(禁用时自带删除线)，样式全面、自由组合。
- 同一行内，不同字体系列，不同字号，不同宽高的图片，随意组合，自动垂直居中。文本内容超宽时自动换行。
- 支持文字与图片混合展示。
- 支持数据（文字/图片）互动，可鼠标点击、选择。选中文本后自动复制到剪贴板。可自定义互动的回调函数。
- 主视图内容是单向流水式显示，回顾区视图为历史数据提供静态查看能力。
- 支持内容闪烁，图片灰度变换。
- 支持大数据量懒加载模式，按需加载/卸载分页化的数据。
- 兼容`fluid`设计器自动生成的代码。

目前版本主视图的最小高度为200px(跟随系统缩放比例)。


## 使用方法示例：
基本依赖：
```toml
[dependencies]
fltk = "1.4"
fltkrs-richdisplay = "0.3"
```

创建组件示例：
```rust
use fltk::{app, window};
use fltk::button::Button;
use fltk::enums::{Color, Event, Font, Key};
use fltk::group::Group;
use fltk::image::SharedImage;
use fltk::prelude::{GroupExt, ImageExt, WidgetBase, WidgetExt, WindowExt};

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();
    let app = app::App::default();
    let mut win = window::Window::default()
        .with_size(1000, 600)
        .with_label("rich-display example")
        .center_screen();
    win.make_resizable(true);

    let mut rich_text = RichText::new(100, 120, 800, 400, None);
    
    win.end();
    win.show();
    
    app.run().unwrap();
}
```

另一个略微复杂的演示代码，需要添加额外的依赖：
```toml
[dependencies]
fltk = { version = "1", features = ["use-ninja"] }
idgenerator-thin = "0"
log = "0"
parking_lot = "0"
tokio = {version = "1", features = ["rt-multi-thread", "macros", "time", "sync", "parking_lot"]}
serde = { version = "1", features = ["derive"] }
once_cell = "1"
debounce_fltk = "0"
regex = "1"

[dev-dependencies]
#simple_logger = "4"
rand = "0"
time = "0"
fast_log = "1"
```
示例代码：
```rust
use std::time::Duration;
use fast_log::consts::LogSize;
use fast_log::filter::ModuleFilter;
use fast_log::plugin::file_split::RollingType;
use fast_log::plugin::packer::LogPacker;
use fltk::{app, window};
use fltk::button::Button;
use fltk::enums::{Color, Event, Font, Key};
use fltk::group::Group;
use fltk::image::{RgbImage, SharedImage};
use fltk::prelude::{GroupExt, ImageExt, WidgetBase, WidgetExt, WindowExt};
use log::{debug, error, LevelFilter};
use rand::{Rng, thread_rng};
use fltkrs_richdisplay::rich_text::{RichText};
use fltkrs_richdisplay::{Action, ActionItem, CallbackData, DataType, DocEditType, image_to_rgb_data, RichDataOptions, UserData};

pub enum GlobalMessage {
    ContentData(UserData),
    UpdateData(RichDataOptions),
    DisableData(i64),
    UpdateBackgroundColor(Color),
    UpdateDefaultTextFont(Font),
    UpdateDefaultTextColor(Color),
    UpdateDefaultTextSize(i32),
    AppendBatchData(Vec<DocEditType>),
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
        .file_split("logs/test.log", LogSize::MB(10), RollingType::All, LogPacker {})
    ).unwrap();
}

#[tokio::main]
async fn main() {
    init_log();

    let app = app::App::default().load_system_fonts();
    let mut win = window::Window::default()
        .with_size(1800, 1000)
        .with_label("rich-display example")
        .center_screen();
    win.make_resizable(true);

    let group = Group::default_fill();

    let mut btn1 = Button::new(200, 0, 100, 30, "反向查找字符串");
    let mut btn11 = Button::new(500, 0, 100, 30, "清除查找目标");
    let mut btn12 = Button::new(350, 0, 100, 30, "正向查找字符串");
    let mut btn2 = Button::new(650, 0, 100, 30, "切换闪烁支持");



    let _ = Button::new(0, 200, 50, 30, "left");

    let mut rich_text = RichText::new(100, 60, 800, 400, None);
    // let mut rich_text = RichText::new(100, 60, 1600, 800, None);

    // 设置默认字体和颜色
    rich_text.set_text_font(Font::Courier);
    rich_text.set_text_color(Color::White);
    rich_text.set_text_size(20);
    // rich_text.set_enable_blink(false);
    // rich_text.set_search_focus_width(2);
    rich_text.set_search_focus_color(Color::White);
    // rich_text.set_search_focus_contrast(Color::Dark1);
    // rich_text.set_piece_spacing(20);
    rich_text.set_cache_size(200);
    rich_text.set_basic_char('A');

    // 应用层消息通道，该通道负责两个方向的消息传递：1将应用层产生的消息向下传递给fltk组件层通道，2将fltk组件层产生的事件消息向上传递给应用层。
    let (action_sender, action_receiver) = tokio::sync::mpsc::channel::<CallbackData>(100);
    // 自定义回调函数，当用户鼠标点击可互动的数据段时，组件会调用回调函数。
    let cb_fn = {
        let sender_rc = action_sender.clone();
        move |cb_data| {
            let sender = sender_rc.clone();
            tokio::spawn(async move {
                if let Err(e) = sender.send(cb_data).await {
                    error!("发送用户操作失败: {:?}", e);
                }
            });
        }
    };
    rich_text.set_notifier(cb_fn);


    let mut rich_text2 = RichText::new(980, 60, 800, 400, None);
    let mut rich_text3 = RichText::new(100, 560, 800, 300, None);
    let mut rich_text4 = RichText::new(980, 560, 400, 400, None);
    rich_text2.set_enable_blink(false);
    rich_text3.set_enable_blink(false);
    rich_text4.set_enable_blink(false);

    btn1.set_callback({
        let mut rt = rich_text.clone();
        move |_| {
            rt.search_str(Some("程序".to_string()), false);
        }
    });
    btn12.set_callback({
        let mut rt = rich_text.clone();
        move |_| {
            rt.search_str(Some("程序".to_string()), true);
        }
    });
    btn11.set_callback({
        let mut rt = rich_text.clone();
        move |_| {
            rt.search_str(None, false);
        }
    });

    btn2.set_callback({
        let mut rt = rich_text.clone();
        move |_| {
            rt.toggle_blink();
        }
    });

    let _ = Button::new(920, 200, 50, 50, "right");

    let mut btn4 = Button::new(200, 470, 150, 50, "删除最后一个数据段");
    btn4.set_callback({
        let mut rt = rich_text.clone();
        move |_| {
            rt.delete_last_data();
        }
    });

    // let mut btn5 = Button::new(400, 550, 100, 50, "测试");

    group.end();

    /*
    启用PageUp/PageDown快捷键打开和关闭回顾区的功能支持。
    使用鼠标滚轮进行打开/关闭回顾区的功能已经内置在模块包中，而PageUp/PageDown的快捷键无法被内置组件检测到，因此需要外层容器主动调用API实现。
    包里提供的两个API接口为此提供支持：`RichText::auto_open_reviewer(&self)`和`RichText::auto_close_reviewer(&self)`。
     */
    win.handle({
        let rich_text_rc = rich_text.clone();
        move |_, evt| {
            let mut handled = false;
            match evt {
                Event::KeyDown => {
                    if app::event_key_down(Key::PageDown) {
                        handled = rich_text_rc.auto_close_reviewer();
                    } else if app::event_key_down(Key::PageUp) {
                        if let Ok(ret) = rich_text_rc.auto_open_reviewer() {
                            handled = ret;
                        }
                    }
                }
                _ => {}
            }
            handled
        }
    });

    win.end();
    win.show();

    debug!("当前主视图的默认窗口尺寸：{:?}", rich_text.calc_default_window_size());

    // fltk组件层消息通道，该通道负责传递组件所需数据。
    let (global_sender, global_receiver) = app::channel::<GlobalMessage>();

    // 由于事先已经通过rich_text.set_notifier(cb_fn)设置回调函数，当可互动数据段产生事件时会发送出来，所以在这里可以监听互动事件并进行处理。
    handle_action(action_receiver, global_sender.clone());

    let mut action = Action::default();
    action.title = "测试提示信息".to_string();
    action.items = vec![ActionItem::new("hello", "hello"), ActionItem::new("world", "world")];

    // 注意！在linux环境下Image不能放在tokio::spawn(future)里面，因其会导致应用失去正常响应，无法关闭。目前原因未知。
    let img1 = SharedImage::load("res/1.jpg").unwrap().to_rgb().unwrap();
    let (img1_width, img1_height) = (img1.width(), img1.height());
    let img2 = SharedImage::load("res/2.jpg").unwrap().to_rgb().unwrap();
    let (img2_width, img2_height) = (img2.width(), img2.height());
    let img3 = SharedImage::load("res/test1.jpg").unwrap().to_rgb().unwrap();
    let (img3_width, img3_height) = (img3.width(), img3.height());
    let (blank_img_data, blank_img_depth, blank_img_width, blank_img_height) = image_to_rgb_data(&None, 500, 100);
    let blank_img = RgbImage::new(&blank_img_data.unwrap(), blank_img_width, blank_img_height, blank_img_depth).unwrap();
    // 异步生成模拟数据，将数据发送给fltk消息通道。
    tokio::spawn(async move {
        let mut last_ud_id = 0i64;
        for i in 0..30 {
            let turn = i * 16;
            let mut data: Vec<UserData> = Vec::from([
                UserData::new_text(format!("{}安全并且高效地处理𝄞并发编程是Rust的另一个主要目标。💖并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程🐉允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 0)).set_bg_color(Some(Color::DarkCyan)),
                UserData::new_text(format!("{}安全并且高效地处理𝄞并发编程是Rust的另一个主要目标。程序。💖并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程🐉允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 1)).set_underline(true).set_font_and_size(Font::Helvetica, 38).set_bg_color(Some(Color::DarkYellow)).set_clickable(true),
                UserData::new_text(format!("{}在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程。类似地，程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。", turn + 2)).set_font_and_size(Font::HelveticaItalic, 18).set_bg_color(Some(Color::Green)),
                UserData::new_image(img1.copy(), img1_width, img1_height, img1_width, img1_height, Some("res/1.jpg".to_string())).set_text("演示图片".to_string()).set_fg_color(Color::Light2).set_font_and_size(Font::HelveticaItalic, 22),
                UserData::new_text(format!("{}由于多线程可以同时运行，🐉所以将计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 3)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)).set_underline(true),
                UserData::new_text(format!("{}由于多线程可以同时运行，所以将计算操作拆分至多个线程可以提高性能。但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 4)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。\r\n", turn + 5)).set_font_and_size(Font::Helvetica, 9).set_underline(true).set_blink(true),
                // UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。\r\n", turn + 5)).set_font(Font::Helvetica, 9).set_underline(true),
                UserData::new_text(format!("{}在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程b。类似地，𝄞程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。\r\n", turn + 6)).set_font_and_size(Font::Helvetica, 32),
                UserData::new_text(format!("{}由于多线程可以同时运行，所以将计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 7)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                UserData::new_text(format!("{}由于多线程可以同时运行，所以将计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 8)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                UserData::new_image(img2.copy(), img2_width, img2_height, img2_width, img2_height, Some("res/2.jpg".to_string())).set_clickable(true),
                UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 9)).set_fg_color(Color::Yellow).set_bg_color(Some(Color::DarkBlue)),
                UserData::new_text(format!("{}在大部分现在操作系统中，执行程序的代码会运行在进程中，操作系统会同时管理多个进程。类似地，程序内部也可以拥有多个同时运行的独立部分，用来运行这些独立部分的就叫做线程。\r\n", turn + 10)).set_font_and_size(Font::HelveticaBold, 32).set_bg_color(Some(Color::Magenta)).set_clickable(true),
                UserData::new_text(format!("{}由于多线程可以同时运行，所以将计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 11)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                UserData::new_text(format!("{}由于多线程可以同时运行，所以将计算操作拆分至多个线程可以提高性能。", turn + 12)),
                UserData::new_text(format!("{}由于多线程可以同时运行，所以将计算操作拆分至多个线程可以提高性能。", turn + 13)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)).set_clickable(true),
                UserData::new_text(format!("{}由于多线程可以同时运行，💖所以将计算操作拆分至多个线程可以提高性能。", turn + 14)).set_fg_color(Color::Cyan).set_font_and_size(Font::Courier, 18).set_clickable(true).set_blink(true),
                // UserData::new_text(format!("{}由于多线程可以同时运行，💖所以将计算操作拆分至多个线程可以提高性能。", turn + 14)).set_fg_color(Color::Cyan).set_font(Font::Courier, 18).set_clickable(true),
                UserData::new_text(format!("{}由于多线程可以~!@#$%^&同时运行，💖所以将计算操作拆分至多个线程可以提高性能。", turn + 15)).set_action(action.clone()),
                UserData::new_image(blank_img.copy(), 500, 100, 500, 100, None).set_text("loading...".to_string()).set_clickable(true),
                UserData::new_text("\r\n这里有BUG吗？".to_string()),
                UserData::new_text(format!("{}由于多线程可以同时运行，所以将计算操作拆分至多个线程可以提高性能。a但是这也增加了程序的复杂度，因为不同线程的执行顺序是无法确定的。\r\n", turn + 17)).set_fg_color(Color::Red).set_bg_color(Some(Color::Green)),
                // UserData::new_image(img2_data.clone(), img2_width, img2_height).set_clickable(true),
            ]);
            // 用于测试行、列数计算的模拟数据。
            // let mut data: Vec<UserData> = Vec::from([
            //     UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。💖并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程🐉允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 0)).set_bg_color(Some(Color::DarkCyan)),
            //     UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。💖并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程🐉允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 1)).set_bg_color(Some(Color::DarkCyan)),
            //     UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。💖并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程🐉允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 2)).set_bg_color(Some(Color::DarkCyan)),
            //     UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。💖并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程🐉允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 3)).set_bg_color(Some(Color::DarkCyan)),
            //     UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。💖并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程🐉允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 4)).set_bg_color(Some(Color::DarkCyan)),
            //     UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。💖并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程🐉允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 5)).set_bg_color(Some(Color::DarkCyan)),
            //     UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。💖并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程🐉允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 6)).set_bg_color(Some(Color::DarkCyan)),
            //     UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。💖并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程🐉允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 7)).set_bg_color(Some(Color::DarkCyan)),
            //     UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。💖并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程🐉允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 8)).set_bg_color(Some(Color::DarkCyan)),
            //     UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。💖并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程🐉允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 9)).set_bg_color(Some(Color::DarkCyan)),
            //     UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。💖并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程🐉允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 10)).set_bg_color(Some(Color::DarkCyan)),
            //     UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。💖并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程🐉允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 11)).set_bg_color(Some(Color::DarkCyan)),
            //     UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。💖并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程🐉允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 12)).set_bg_color(Some(Color::DarkCyan)),
            //     UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。💖并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程🐉允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 13)).set_bg_color(Some(Color::DarkCyan)),
            //     UserData::new_text(format!("{}安全并且高效地处理并发编程是Rust的另一个主要目标。💖并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程🐉允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 14)).set_bg_color(Some(Color::DarkCyan)),
            // ]);

            let mut batch_data = vec![];
            data.reverse();
            while let Some(data_unit) = data.pop() {
                last_ud_id = data_unit.id;
                // global_sender.send(GlobalMessage::ContentData(data_unit));
                batch_data.push(DocEditType::Data(data_unit));
            }

            global_sender.send(GlobalMessage::AppendBatchData(batch_data));
        }
        tokio::time::sleep(Duration::from_secs(2)).await;

        let update_opt = RichDataOptions::new(last_ud_id - 2).image(Some(img3), 500, 100).text(String::new());
        global_sender.send(GlobalMessage::UpdateData(update_opt));

        debug!("Sender closed.");
    });

    let mut r = thread_rng();

    let mut has_recent_message = false;
    while app.wait() {
        // 从fltk消息通道接收数据，并发送给组件。
        if let Some(msg) = global_receiver.recv() {
            match msg {
                GlobalMessage::ContentData(data) => {
                    // 新增数据段，按近似比例发布到不同的窗口
                    if r.gen_bool(0.45f64) {
                        rich_text2.append(data.clone());
                    }
                    if r.gen_bool(0.1f64) {
                        rich_text3.append(data.clone());
                    }
                    if r.gen_bool(0.01f64) {
                        rich_text4.append(data.clone());
                    }
                    has_recent_message = true;
                    rich_text.append(data);
                    // debug!("新增消息");
                }
                GlobalMessage::AppendBatchData(mut batch) => {
                    if r.gen_bool(0.45f64) {
                        let mut batch2 = batch.clone();
                        rich_text2.append_batch(&mut batch2);
                    }
                    if r.gen_bool(0.1f64) {
                        let mut batch3 = batch.clone();
                        rich_text3.append_batch(&mut batch3);
                    }
                    if r.gen_bool(0.01f64) {
                        let mut batch4 = batch.clone();
                        rich_text4.append_batch(&mut batch4);
                    }
                    rich_text.append_batch(&mut batch);
                }
                GlobalMessage::UpdateData(options) => {
                    // 更新数据段状态
                    rich_text.update_data(options);
                }
                GlobalMessage::DisableData(id) => {
                    // 更新数据段状态为禁用
                    rich_text.disable_data(id);
                }
                GlobalMessage::UpdateBackgroundColor(color) => {
                    rich_text.set_background_color(color);
                }
                GlobalMessage::UpdateDefaultTextFont(font) => {
                    rich_text.set_text_font(font);
                }
                GlobalMessage::UpdateDefaultTextSize(size) => {
                    rich_text.set_text_size(size);
                }
                GlobalMessage::UpdateDefaultTextColor(color) => {
                    rich_text.set_text_color(color);
                }
            }
        } else {
            has_recent_message = false;
        }

        if !has_recent_message {
            app::sleep(0.001);
            app::awake();
        }
    }

    if let Ok(w) = fast_log::flush() {
        // 等待日志刷出到磁盘上。
        w.wait();
    }
}

pub fn handle_action(mut action_receiver: tokio::sync::mpsc::Receiver<CallbackData>, global_sender_rc: app::Sender<GlobalMessage>) {
    tokio::spawn(async move {
        while let Some(data) = action_receiver.recv().await {
            match data {
                CallbackData::Data(data) => {
                    // debug!("用户点击数据：{:?}", data);
                    if data.text.starts_with("10") {
                        let toggle = !data.blink;
                        let update_options = RichDataOptions::new(data.id).blink(toggle);
                        global_sender_rc.send(GlobalMessage::UpdateData(update_options));
                    } else if data.text.starts_with("13") {
                        let toggle = !data.blink;
                        let update_options = RichDataOptions::new(data.id).blink(toggle);
                        global_sender_rc.send(GlobalMessage::UpdateData(update_options));
                    } else if data.text.starts_with("14") {
                        let toggle = !data.underline;
                        let update_options = RichDataOptions::new(data.id).underline(toggle);
                        global_sender_rc.send(GlobalMessage::UpdateData(update_options));
                    } else if data.text.starts_with("22") {
                        global_sender_rc.send(GlobalMessage::DisableData(data.id));
                    } else if data.text.starts_with("23") {
                        let toggle = !data.strike_through;
                        let update_options = RichDataOptions::new(data.id).strike_through(toggle);
                        global_sender_rc.send(GlobalMessage::UpdateData(update_options));
                    } else if data.text.starts_with("25") {
                        let update_options = RichDataOptions::new(data.id).clickable(false).expired(true).bg_color(Color::DarkGreen);
                        global_sender_rc.send(GlobalMessage::UpdateData(update_options));
                    } else if data.data_type == DataType::Image {
                        let toggle = !data.disabled;
                        // let update_options = RichDataOptions::new(data.id).blink(toggle);
                        let update_options = RichDataOptions::new(data.id).disabled(toggle);
                        global_sender_rc.send(GlobalMessage::UpdateData(update_options));
                    }
                }
                CallbackData::Shape(data) => {
                    debug!("窗口尺寸发生变化，新：{},{},{},{}，旧：{},{}", data.new_width, data.new_height, data.new_cols, data.new_rows, data.old_width, data.old_height);
                }
                CallbackData::Image(image_event_data) => {
                    debug!("用户点击图片：{:?}", image_event_data);
                }
            }

        }
    });
}
```
*在`examples`目录中有更详细的示例代码。*

下图是目前已实现的图文混排效果预览图：

主内容预览
[![demo2](./res/demo2.png)](https://gitee.com/t1000ln/fltkrs-richdisplay/blob/main/res/demo2.png)

回顾区预览，包含文本选择、字符串查找
[![demo4](./res/demo4.png)](https://gitee.com/t1000ln/fltkrs-richdisplay/blob/main/res/demo4.png)

