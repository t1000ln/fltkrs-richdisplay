# fltkrs-richdisplay
![Static Badge](https://img.shields.io/badge/crates-0.2.7-blue) 
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

目前版本主视图的最小高度为200px(跟随系统缩放比例)。

## 性能参考
在`win10`环境下快速添加数据时，界面刷新速度依赖于CPU和GPU运算速度及视图尺寸。

| CPU   | GPU       | 数据量 | 最大缓存  | 新增数据间隔 | 起始内存  | 最大内存 | 平均CPU% | 平均GPU% | 视图尺寸     | 处理延迟     |
|---------|-------------|-------|--------------|------|-------|------|--------|--------|----------|----------|
| i7 12th | Nvidia 3070 | 1600条 | 1000条 | 30ms  | 3.4Mb | 61Mb | 2.8% | 10%    | 800x400  | &lt;30ms |
| i7 12th | Nvidia 3070 | 1600条 | 1000条 | 30ms  | 3.4Mb | 67Mb | 9%    | 27%    | 1600x800 | &lt;30ms |




## 使用方法示例：
基本依赖：
```toml
[dependencies]
fltk = "1.4"
fltkrs-richdisplay = "0.2.0"
```

由于下面的`examples`示例用到`tokio`框架进行异步交互，并且简单输出日志，所以需要额外添加依赖:
```toml
[dev-dependencies]
simple_logger = "4.2"
tokio = { version = "1.32", features = ["full"] }
```

示例代码：
```rust
use std::time::Duration;
use fltk::{app, window};
use fltk::button::Button;
use fltk::enums::{Color, Event, Font, Key};
use fltk::group::Group;
use fltk::image::SharedImage;
use fltk::prelude::{GroupExt, ImageExt, WidgetBase, WidgetExt, WindowExt};
use log::{debug, error};
use fltkrs_richdisplay::rich_text::{RichText};
use fltkrs_richdisplay::{DataType, RichDataOptions, UserData};

pub enum GlobalMessage {
    ContentData(UserData),
    UpdateData(RichDataOptions),
    DisableData(i64),
}

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();
    let app = app::App::default();
    let mut win = window::Window::default()
        .with_size(1000, 600)
        .with_label("rich-display example")
        .center_screen();
    win.make_resizable(true);

    let group = Group::default_fill();

    let mut btn1 = Button::new(200, 0, 100, 30, "反向查找字符串");
    let mut btn11 = Button::new(500, 0, 100, 30, "清除查找目标");
    let mut btn12 = Button::new(350, 0, 100, 30, "正向查找字符串");


    let _ = Button::new(0, 200, 50, 30, "left");

    let mut rich_text = RichText::new(100, 120, 800, 400, None);

    // 应用层消息通道，该通道负责两个方向的消息传递：1将应用层产生的消息向下传递给fltk组件层通道，2将fltk组件层产生的事件消息向上传递给应用层。
    let (action_sender, action_receiver) = tokio::sync::mpsc::channel::<UserData>(100);
    // 自定义回调函数，当用户鼠标点击可互动的数据段时，组件会调用回调函数。
    let cb_fn = {
        let sender_rc = action_sender.clone();
        move |user_data| {
            let sender = sender_rc.clone();
            tokio::spawn(async move {
                if let Err(e) = sender.send(user_data).await {
                    error!("发送用户操作失败: {:?}", e);
                }
            });
        }
    };
    rich_text.set_notifier(cb_fn);
    rich_text.set_buffer_max_lines(1000);

    btn1.set_callback({
        let mut rt = rich_text.clone();
        move |_| {
            rt.search_str(Some("程序".to_string()), false);
        }
    });
    btn12.set_callback({
        let mut rt = rich_text.clone();
        move |_| {
            rt.search_str(Some("高效".to_string()), true);
        }
    });
    btn11.set_callback({
        let mut rt = rich_text.clone();
        move |_| {
            rt.search_str(None, false);
        }
    });

    let _ = Button::new(950, 200, 50, 50, "right");

    let mut btn4 = Button::new(200, 550, 150, 50, "删除最后一个数据段");
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
                        handled = rich_text_rc.auto_open_reviewer().unwrap();
                    }

                }
                _ => {}
            }
            handled
        }
    });

    win.end();
    win.show();

    // fltk组件层消息通道，该通道负责传递组件所需数据。
    let (global_sender, global_receiver) = app::channel::<GlobalMessage>();

    // 由于事先已经通过rich_text.set_notifier(cb_fn)设置回调函数，当可互动数据段产生事件时会发送出来，所以在这里可以监听互动事件并进行处理。
    handle_action(action_receiver, global_sender.clone());


    // 注意！在linux环境下Image不能放在tokio::spawn(future)里面，因其会导致应用失去正常响应，无法关闭。目前原因未知。
    let img1 = SharedImage::load("res/1.jpg").unwrap();
    let (img1_width, img1_height, img1_data) = (img1.width(), img1.height(), img1.to_rgb_data());
    let img2 = SharedImage::load("res/2.jpg").unwrap();
    let (img2_width, img2_height, img2_data) = (img2.width(), img2.height(), img2.to_rgb_data());
    // 异步生成模拟数据，将数据发送给fltk消息通道。
    tokio::spawn(async move {
        for i in 0..1 {
            let turn = i * 13;
            let mut data: Vec<UserData> = Vec::from([
                UserData::new_text(format!("{}安全并且高效地处理𝄞并发编程是Rust的另一个@主要目标。💖并发编程和并行编程这两种概念随着计算机设备的多核a优化而变得越来越重要。并发编程🐉允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。", turn + 1)).set_underline(true).set_font(Font::Helvetica, 38).set_bg_color(Some(Color::DarkYellow)).set_clickable(true),
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
                global_sender.send(GlobalMessage::ContentData(data_unit));
                tokio::time::sleep(Duration::from_millis(30)).await;
            }
        }

        debug!("Sender closed");
    });

    while app.wait() {
        // 从fltk消息通道接收数据，并发送给组件。
        if let Some(msg) = global_receiver.recv() {
            match msg {
                GlobalMessage::ContentData(data) => {
                    // 新增数据段
                    rich_text.append(data);
                }
                GlobalMessage::UpdateData(options) => {
                    // 更新数据段状态
                    rich_text.update_data(options);
                }
                GlobalMessage::DisableData(id) => {
                    // 更新数据段状态为禁用
                    rich_text.disable_data(id);
                }
            }
        }

        app::sleep(0.001);
        app::awake();
    }
}

pub fn handle_action(mut action_receiver: tokio::sync::mpsc::Receiver<UserData>, global_sender_rc: app::Sender<GlobalMessage>) {
    tokio::spawn(async move {
        while let Some(data) = action_receiver.recv().await {
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
    });
}
```
示例代码中使用`tokio`发送异步消息，目的是演示组件的互动能力，但`richdisplay`包本身并不依赖`tokio`。

下图是目前已实现的图文混排效果预览图：

主内容预览
[![demo2](./res/demo2.png)](https://gitee.com/t1000ln/fltkrs-richdisplay/blob/main/res/demo2.png)

回顾区预览，包含文本选择、字符串查找
[![demo4](./res/demo4.png)](https://gitee.com/t1000ln/fltkrs-richdisplay/blob/main/res/demo4.png)

