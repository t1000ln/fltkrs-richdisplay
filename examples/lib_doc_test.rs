// use fltk::{app, window};
// use fltk::prelude::{GroupExt, WidgetBase, WidgetExt, WindowExt};
// use fltkrs_richdisplay::rich_text::RichText;
// use fltkrs_richdisplay::UserData;
//
// #[tokio::main]
// async fn main() {
//     let app = app::App::default();
//     let mut win = window::Window::default().with_size(1000, 600).center_screen();
//     let mut rich_text = RichText::new(100, 120, 800, 400, None);
//     rich_text.set_cache_size(200);
//     win.end();
//     win.show();
//
//     let ud = UserData::new_text("dev@DESKTOP-PCL7MBI:\t~$ ls\r\n".to_string());
//     rich_text.append(ud);
//     app.run().unwrap();
// }


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