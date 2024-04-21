# fltkrs-richdisplay
![Static Badge](https://img.shields.io/badge/crates-1.0.2-blue) 
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
fltk = "1"
fltkrs-richdisplay = "1"
```

创建组件示例：
```rust
use fltk::{app, window};
use fltk::prelude::{GroupExt, WidgetBase, WidgetExt, WindowExt};
use fltkrs_richdisplay::rich_text::RichText;
use fltkrs_richdisplay::UserData;

#[tokio::main]
async fn main() {
    let app = app::App::default();
    let mut win = window::Window::default().with_size(1000, 600).center_screen();
    let mut rich_text = RichText::new(100, 120, 800, 400, None);
    rich_text.set_cache_size(200);
    win.end();
    win.show();

    let ud = UserData::new_text("dev@DESKTOP-PCL7MBI:\t~$ ls\r\n".to_string());
    rich_text.append(ud);
    app.run().unwrap();
}
```

另一个略微复杂的演示代码，需要添加额外的依赖：
```toml
[dependencies]
fltk = "1"
fltkrs-richdisplay = "1"
tokio = {version = "1", features = ["rt-multi-thread", "macros", "time", "sync", "parking_lot"]}
```
示例代码：
```rust
use fltk::{app, window};
use fltk::enums::{Color, Event, Font, Key};
use fltk::prelude::{GroupExt, WidgetBase, WidgetExt, WindowExt};
use log::error;
use fltkrs_richdisplay::rich_text::RichText;
use fltkrs_richdisplay::{RichDataOptions, UserData, CallbackData, DocEditType};

pub enum GlobalMessage {
    ContentData(UserData),
    UpdateData(RichDataOptions),
    DisableData(i64),
}

#[tokio::main]
async fn main() {
    let app = app::App::default();
    let mut win = window::Window::default().with_size(1000, 600).center_screen();
    win.make_resizable(true);

    let mut rich_text = RichText::new(100, 120, 800, 400, None);

    // 互动消息通道
    let (action_sender, mut action_receiver) = tokio::sync::mpsc::channel::<CallbackData>(100);

    // 自定义回调函数，当用户鼠标点击可互动的数据段时，组件会调用回调函数。
    let cb_fn = {
        let action_sender_rc = action_sender.clone();
        move |user_data| {
            let sender = action_sender_rc.clone();
            tokio::spawn(async move {
                if let Err(e) = sender.send(user_data).await {
                    error!("发送用户操作失败: {:?}", e);
                }
            });
        }
    };
    rich_text.set_notifier(cb_fn);
    rich_text.set_cache_size(1000);

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

    // App全局消息唯一通道
    let (global_sender, global_receiver) = app::channel::<GlobalMessage>();

    win.end();
    win.show();

    let global_sender_rc = global_sender.clone();

    // 互动消息处理器
    tokio::spawn(async move {
        while let Some(cb_data) = action_receiver.recv().await {
            if let CallbackData::Data(data) = cb_data {
                if data.text.starts_with("10") {
                    let toggle = !data.blink;
                    let update_options = RichDataOptions::new(data.id).blink(toggle);
                    global_sender_rc.send(GlobalMessage::UpdateData(update_options));
                }
            }
        }
    });

    let data = vec![
        UserData::new_text("0dev@DESKTOP-PCL7MBI:\t~$ ls\r\n1分片\r\n2分片".to_string()),
        UserData::new_text("3dev@DESKTOP-PCL7MBI:\t~$ ls\r\n".to_string()),
        UserData::new_text("4dev@DESKTOP-PCL7MBI:\t~$ ls\r\nls -al".to_string()),
        UserData::new_text("5dev@DESKTOP-PCL7MBI:\t~$ ls\r\n速度".to_string()).set_bg_color(Some(Color::Green)),
        UserData::new_text("6dev@DESKTOP-PCL7MBII:\t~$ ls Downloads\r\n".to_string()).set_font_and_size(Font::Helvetica, 22),
        UserData::new_text("7dev@DESKTOP-PCL7MBI:\t~$ ls\r\n".to_string()),
        UserData::new_text("8dev@DESKTOP-PCL7MBI:~$ ls".to_string()).set_underline(true),
        UserData::new_text("9dev@DESKTOP-PCL7MBI:~$ ls\r\n".to_string()).set_underline(true),
        UserData::new_text("10 Right click me! 鼠标右键点击！\r\n".to_string()).set_font_and_size(Font::Helvetica, 20).set_clickable(true).set_blink(true),
        UserData::new_text("11dev@DESKTOP-PCL7MBI:\t~$ ls\r\n".to_string()),
    ];

    let mut docs: Vec<DocEditType> = Vec::new();
    for ud in data {
        docs.push(DocEditType::Data(ud));
    }
    rich_text.append_batch(&mut docs);

    let mut has_recent_message = false;
    while app.wait() {
        if let Some(msg) = global_receiver.recv() {
            match msg {
                GlobalMessage::ContentData(data) => {
                    has_recent_message = true;
                    rich_text.append(data);
                }
                GlobalMessage::UpdateData(options) => {
                    rich_text.update_data(options);
                }
                GlobalMessage::DisableData(id) => {
                    rich_text.disable_data(id);
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
}
```
*在`examples`目录中有更详细的示例代码。*

`1.0.2`更新：
1. 增加`RichText.fix_scale`方法，用于解决在`Windows`环境下屏幕缩放比例为`100%`时可能出现回顾区渲染异常的问题。
2. 增加`RichText::default`和`RichText::default_fill`函数。

下图是目前已实现的图文混排效果预览图：

主内容预览
[![demo2](./res/demo2.png)](https://gitee.com/t1000ln/fltkrs-richdisplay/blob/main/res/demo2.png)

回顾区预览，包含文本选择、字符串查找
[![demo4](./res/demo4.png)](https://gitee.com/t1000ln/fltkrs-richdisplay/blob/main/res/demo4.png)

