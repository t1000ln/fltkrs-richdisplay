use fltk::{app, window};
use fltk::app::{get_font_names, set_fonts};
use fltk::button::Button;
use fltk::draw::show_colormap;
use fltk::enums::{Color, Font};
use fltk::prelude::{GroupExt, WidgetBase, WidgetExt, WindowExt};
use log::debug;
use fltkrs_richdisplay::rich_text::RichText;
use fltkrs_richdisplay::{DocEditType, UserData, Action, ActionItem};

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();
    let app = app::App::default().load_system_fonts();
    let mut win = window::Window::default()
        .with_size(1620, 820)
        .with_label("rich-display newline test")
        .center_screen();
    win.make_resizable(true);

    // app::fonts().iter().for_each(|font| {debug!("{}", font)});

    let mut rich_text = RichText::new(10, 10, 1400, 800, None);
    #[cfg(target_os = "windows")]
    let song = Font::by_name(" 楷体");

    #[cfg(target_os = "linux")]
    // let song = Font::by_name("SimSun");
    // let song = Font::by_name("NSimSun");
    // let song = Font::by_name("SimHei");
    // let song = Font::by_name("KaiTi");
    // let song = Font::by_name("FangSong");
    let song = Font::by_name("Noto Sans Mono CJK SC");

    rich_text.set_text_font(song);
    // rich_text.set_text_font(Font::by_index(13));
    rich_text.set_text_size(16);
    rich_text.set_cache_size(1000);

    let mut btn = Button::new(1450, 10, 100, 30, "色彩");
    btn.set_callback(|ctx| {
        let ret_color = show_colormap(Color::DarkCyan);

        debug!("ret_color: {:?}, bits: {}", ret_color, ret_color.bits());
    });

    // set_fonts("");


    win.end();
    win.show();

    let mut action = Action::default();
    action.title = "小河".to_string();
    let mut data = vec![
        // // UserData::new_text("∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))).set_font(Font::by_name("B宋体"), 14),
        UserData::new_text("连接 mud.pkuxkx.net:8081 ...".to_string()),
        UserData::new_text(" ".to_string()),
        UserData::new_text("成功！".to_string()).set_fg_color(Color::Green),
        UserData::new_text("\r\n".to_string()),
        UserData::new_text("\r\n                  ☆☆☆☆☆☆☆☆☆☆☆☆☆☆☆☆\r\n                ☆ 飞雪连天射白鹿，笑书神侠倚碧鸳 ☆\r\n      \t          ☆☆☆☆☆☆☆☆☆☆☆☆☆☆☆☆\r\n                   本游戏参考金庸武侠系列小说编写     \r\n\r\n              ".to_string()),
        UserData::new_text("----====   北  大  侠  客  行  ====----".to_string()),
        UserData::new_text("\r\n\r\n    ".to_string()),
        UserData::new_text("∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\r\n    ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\r\n    ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\u{3000}     释卷掩灯，抚剑品香茗。总是一番风云尽，坐听虚空。   ".to_string()).set_fg_color(Color::from_rgb(229, 229, 229)),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\r\n    ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\u{3000} 信马扬州由何处？廿载北大侠客行。                       ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\r\n    ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\u{3000}\u{3000}                                                  \u{3000}\u{3000}".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\r\n    ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\u{3000}\u{3000}   你可曾怀念误入泥潭".to_string()),
        UserData::new_text("的新奇，你可曾追忆白马轻裘的香   ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\r\n    ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\u{3000} 风，你可曾牵挂执手红颜的如花笑靨，你可曾想念纵酒放歌   ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\r\n    ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\u{3000} 的生死弟兄？归去来兮！北大侠客行！你可以重温扬州的华\u{3000} ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\r\n    ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\u{3000} 丽，天山的冰冷，丐帮的刚烈，桃花的自由！你可以纵横叱   ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\r\n    ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\u{3000} 咤御敌于襄阳关口显英雄本色，也可以齐肩并辔伴美看风景   ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\r\n    ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\u{3000} 丽秀传千古佳谣。\u{3000}                                     ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\r\n    ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\u{3000}\u{3000}                                                  \u{3000}\u{3000}".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\r\n    ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\u{3000}\u{3000}   抑或，你什么都不为，只为了走一走，那久违了的，坚   ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\r\n    ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\u{3000} 硬冰冷，却又带着那么一点温情的青石大道？               ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\r\n    ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\u{3000}\u{3000}                                                    \u{3000}".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\r\n    ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\u{3000}\u{3000}\u{3000} \u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}  ".to_string()),
        UserData::new_text("-- ".to_string()),
        UserData::new_text("by guodalu@pkuxkx".to_string()).set_fg_color(Color::Red),
        UserData::new_text("\u{3000} ".to_string()),
        UserData::new_text("∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("\r\n    ".to_string()),
        UserData::new_text("∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷∷".to_string()).set_bg_color(Some(Color::from_rgb(0, 0, 205))),
        UserData::new_text("      \r\n\r\n           ".to_string()),
        UserData::new_text("帐号说明".to_string()),
        UserData::new_text("\t".to_string()),
        UserData::new_text("https://pkuxkx.net/wiki/pkuxkx/account\r\n".to_string()),
        UserData::new_text("           ".to_string()),
        UserData::new_text("入门导读".to_string()),
        UserData::new_text("\t".to_string()),
        UserData::new_text("https://pkuxkx.net/wiki/pkuxkx/guide\r\n".to_string()),
        UserData::new_text("           ".to_string()),
        UserData::new_text("游戏地址".to_string()),
        UserData::new_text("\t".to_string()),
        UserData::new_text("mud.pkuxkx.net 8080\r\n".to_string()),
        UserData::new_text("           ".to_string()),
        UserData::new_text("北侠主页".to_string()),
        UserData::new_text("\t".to_string()),
        UserData::new_text("https://pkuxkx.net\r\n".to_string()),
        UserData::new_text("           ".to_string()),
        UserData::new_text("北侠QQ群".to_string()),
        UserData::new_text("\t".to_string()),
        UserData::new_text("770985873\r\n\r\n".to_string()),
        UserData::new_text("北大侠客行已经执行了".to_string()),
        UserData::new_text("五小时五十四分二十七秒".to_string()),
        UserData::new_text("。\r\n目前共有 631 位玩家在线上。\r\n".to_string()),
        UserData::new_text("Input 1 for GBK, 2 for UTF8, 3 for BIG5\r\n".to_string()),
        UserData::new_text("1;31m".to_string()),
        UserData::new_text("由于MushClient对Unicode输出支持有限，使用MushClient时请连接8080端口，并且不要选择编码！\r\n如已登录，并且再次登录8080端口时出现档案问题，请联系巫师处理。\r\n".to_string()),
        UserData::new_text("您的英文名字（要注册新人物请输入new。）：".to_string()),
        UserData::new_text("此ID档案已存在，请输入密码：".to_string()),
        UserData::new_text("\r\n\r\n目前权限：(player)\r\n未明谷".to_string()),
        UserData::new_text(" - \r\n                    树林----".to_string()),
        UserData::new_text("未明谷".to_string()),
        UserData::new_text("----乱石阵    \r\n                              ｜     \r\n                           青石桥头             \r\n    山谷中绿树成荫，却不见有多么明媚的花开于此，但你仍能闻见了远远飘来的花香。耳边听到了溪\r\n水叮咚的声音，原来不远处有一条蜿蜒的小溪".to_string()),

        UserData::new_text("(river)".to_string()).set_action(action),

        UserData::new_text("，岸边似乎散落了一些物什。在山谷的北侧有条陡\r\n".to_string()),
        UserData::new_text("峭的山坡".to_string()),
        UserData::new_text("(path)".to_string()),
        UserData::new_text("隐隐可以通向外界。".to_string()),
        UserData::new_text("\r\n    「寒冬」: ".to_string()),
        UserData::new_text("东方的天空渐渐的发白了".to_string()),
        UserData::new_text("。\r\n\r\n    ".to_string()),
        UserData::new_text("这里明显的方向有 south、west 和 east。".to_string()),
        UserData::new_text("\r\n\r\n    二枚".to_string()),
        UserData::new_text("野果".to_string()),
        UserData::new_text("(Ye guo".to_string()),
        UserData::new_text(")\r\n    ".to_string()),
        UserData::new_text("葫芦".to_string()),
        UserData::new_text("(Hu lu".to_string()),
        UserData::new_text(")\r\n终端机型态设定为 ansi 。\r\n".to_string()),
        UserData::new_text("你还没有看过help newbie，那里面有大部分新手常问的问题如吃喝等的解答\r\n以及很多很好的建议，希望你能尽快仔细阅读".to_string()),
        UserData::new_text("它。\r\n".to_string()),
        UserData::new_text("即将开始检测你的客户端对MXP的支持程度。如果5s中之内没有反应，代表你使用的客户端完全不支持MXP,请按回车进入普通模式。\r\n推荐使用Mudlet,MushClient和Zmud。请访问http://pkuxkx.net/forum/thread-9684-1-1.html学习如何配置MXP客户端。\r\n".to_string()),
        UserData::new_text("你的客户端不支持MXP功能，使用普通文本模式。\r\n> ".to_string()),
        UserData::new_text("【江湖】听说独孤一明(Duguyiming)决定弃文从武，投身江湖，🐉新一代大侠可能就此诞生了！".to_string()),
        UserData::new_text(" \r\n".to_string()),
        UserData::new_text("                                                         ".to_string()),
        UserData::new_text("                \r\n".to_string()),
        UserData::new_text("               ".to_string()),
        UserData::new_text("┌─┬─┬─┬─┬─┬─┬─┬─┬─┬─┐".to_string()),
        UserData::new_text("                \r\n".to_string()),
        UserData::new_text("               ".to_string()),
        UserData::new_text("│\u{3000}│\u{3000}│此│相│寒│落│秋│秋│\u{3000}│\u{3000}│".to_string()),
        UserData::new_text("                \r\n".to_string()),
        UserData::new_text("               ".to_string()),
        UserData::new_text("│\u{3000}│\u{3000}│时│思│鸦│叶│月│风│\u{3000}│\u{3000}│".to_string()),
        UserData::new_text("                \r\n".to_string()),
        UserData::new_text("               ".to_string()),
        UserData::new_text("│\u{3000}│\u{3000}│此│相│栖│聚│明│清│\u{3000}│\u{3000}│".to_string()),
        UserData::new_text("                \r\n".to_string()),
        UserData::new_text("               ".to_string()),
        UserData::new_text("│\u{3000}│\u{3000}│夜│见│复│还│  │  │\u{3000}│\u{3000}│".to_string()),
        UserData::new_text("                \r\n".to_string()),
        UserData::new_text("               ".to_string()),
        UserData::new_text("│\u{3000}│\u{3000}│难│知│惊│散│\u{3000}│\u{3000}│\u{3000}│\u{3000}│".to_string()),
        UserData::new_text("                \r\n".to_string()),
        UserData::new_text("               ".to_string()),
        UserData::new_text("│\u{3000}│\u{3000}│为│何│  │  │\u{3000}│\u{3000}│\u{3000}│\u{3000}│".to_string()),
        UserData::new_text("                \r\n".to_string()),
        UserData::new_text("               ".to_string()),
        UserData::new_text("│\u{3000}│\u{3000}│情│日│\u{3000}│\u{3000}│\u{3000}│\u{3000}│\u{3000}│\u{3000}│".to_string()),
        UserData::new_text("                \r\n".to_string()),
        UserData::new_text("6;1m               ".to_string()),
        UserData::new_text("└─┴─┴─┴─┴─┴─┴─┴─┴─┴─┘".to_string()),
        UserData::new_text("                \r\n".to_string()),
        UserData::new_text("                                                         ".to_string()),
        UserData::new_text("                \r\n".to_string()),
        UserData::new_text("\r\n".to_string()),
    ];

    // for ud in data {
    //     let text = ud.text.clone();
    //     debug!("text: {}", text);
    //     rich_text.append(ud);
    // }

    let mut actions: Vec<DocEditType> = vec![];
    for rd in data {
        actions.push(DocEditType::Data(rd));
    }
    rich_text.append_batch(&mut actions);

    app.run().unwrap();
}