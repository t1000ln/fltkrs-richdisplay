use fast_log::filter::ModuleFilter;
use fltk::{app, window};
use fltk::enums::{Color, Font};
use fltk::prelude::{GroupExt, WidgetExt, WindowExt};
use log::{debug, LevelFilter};
use fltkrs_richdisplay::rich_text::RichText;
use fltkrs_richdisplay::{DocEditType, UserData};

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

    let app = app::App::default().load_system_fonts();
    // app::get_font_names().iter().for_each(|font_name| { debug!("{:?}", font_name); });
    #[cfg(target_os = "windows")]
    {
        Font::set_font(Font::Screen, "Consolas");
        Font::set_font(Font::ScreenBold, "Consolas Bold");
    }

    let (kai_ti, kai_ti_bold) = if cfg!(target_os = "windows") {
        (" 楷体", "B楷体")
    } else {
        ("KaiTi", "KaiTi Bold")
    };

    let mut win = window::Window::default()
        .with_size(1220, 820)
        .with_label("rich-display newline test")
        .center_screen();
    win.make_resizable(true);

    let mut rich_text = RichText::new(10, 10, 1200, 800, None);
    // rich_text.set_text_font(Font::Helvetica);
    // rich_text.set_text_font(Font::by_name("KaiTi"));
    rich_text.set_text_size(10);

    win.end();
    win.show();

    let data = vec![
        UserData::new_text("0dev@DESKTOP-PCL7MBI:\t~$ ls\r\n0.1分片\r\n0.2分片".to_string()),
        UserData::new_text("0.5dev@DESKTOP-PCL7MBI:\t~$ ls\r\n".to_string()),
        UserData::new_text("0.6dev@DESKTOP-PCL7MBI:\t~$ ls\r\nls -al".to_string()),
        UserData::new_text("1未名谷\r\n".to_string()).set_font_and_size(Font::by_name(kai_ti), 28),
        UserData::new_text("2未名谷\r\n".to_string()).set_font_and_size(Font::by_name(kai_ti_bold), 28),
        UserData::new_text("3dev@DESKTOP-PCL7MBI:\t~$ ls\r\n速度".to_string()).set_bg_color(Some(Color::Green)),
        UserData::new_text("4dev@DESKTOP-PCL7MBII:\t~$ ls糊涂\r\n".to_string()).set_font_and_size(Font::Helvetica, 22),
        UserData::new_text("5dev@DESKTOP-PCL7MBI:\t~$ ls\r\n".to_string()),
        UserData::new_text("6dev@DESKTOP-PCL7MBI:~$ ls".to_string()).set_underline(true),
        UserData::new_text("7dev@DESKTOP-PCL7MBI:~$ ls\r\n".to_string()).set_underline(true),
        UserData::new_text("8│【食物】 264     / 300 @     [缺食    │【潜能】 3190                         │\x0d\x0a".to_string()).set_font_and_size(Font::by_name(kai_ti), 28).set_underline(false),
        UserData::new_text("9│【饮水】 228     / 300      [缺水    │@【经验】 270                          │\x0d\x0a".to_string()).set_font_and_size(Font::by_name(kai_ti), 16),
        UserData::new_text("10dev@DESKTOP-PCL7MBI:\t~$ ls".to_string()).set_font_and_size(Font::by_name(kai_ti), 20),
        UserData::new_text("11dev@DESKTOP-PCL7MBI:\t~$ ls\r\n".to_string()).set_font_and_size(Font::Helvetica, 20),
        UserData::new_text("12dev@DESKTOP-PCL7MBI:\t~$ ls\r\n".to_string()),
    ];

    let mut docs: Vec<DocEditType> = Vec::new();
    for ud in data {
        docs.push(DocEditType::Data(ud));
    }
    rich_text.append_batch(&mut docs);

    app.run().unwrap();

    if let Ok(w) = fast_log::flush() {
        // 等待日志刷出到磁盘上。
        w.wait();
    }
}