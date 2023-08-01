//! 请描述文件用途。

use fltk::{app, window};
use fltk::button::Button;
use fltk::prelude::{GroupExt, WidgetBase, WidgetExt, WindowExt};
use fltk_flow::Flow;

fn main() {
    let app = app::App::default();
    let mut win = window::Window::default()
        .with_size(800, 400)
        .with_label("draw by notice")
        .center_screen();
    win.make_resizable(true);

    let mut flow = Flow::default_fill();

    let mut b1 = Button::new(0, 0, 100, 30, None);
    b1.set_label("123");

    let mut b2 = Button::new(0, 0, 100, 30, None);
    b2.set_label("456");
    flow.end();

    flow.rule(&b1, "^<");
    flow.rule(&b2, "v<");

    win.end();
    win.show();

    while app.wait() {
        app::sleep(0.016);
        app::awake();
    }
}
