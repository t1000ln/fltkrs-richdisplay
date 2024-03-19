use fltk::{enums::*, prelude::*, *};
use fltk::browser::HoldBrowser;
use fltk::button::Button;
use fltk::frame::Frame;
use fltk::group::{Flex, FlexType, Pack, PackType};
use fltk::text::{TextBuffer, TextEditor};

fn main() {
    let app = app::App::default();
    let mut window = window::Window::default().with_size(300, 300);
    window.set_frame(FrameType::NoBox);
    window.make_resizable(true);

    let dx = 20;
    let dy = dx; // border width of resizable() - see below
    let tile = group::Tile::default_fill();

    // create the symmetrical resize box with dx and dy pixels distance, resp.
    // from the borders of the Fl_Tile widget before all other children
    let r = frame::Frame::new(
        tile.x() + dx,
        tile.y() + dy,
        tile.w() - 2 * dx,
        tile.h() - 2 * dy,
        None,
    );
    tile.resizable(&r);

    // let mut box0 = frame::Frame::new(0, 0, 150, 150, "0");
    // box0.set_frame(FrameType::DownBox);
    // box0.set_color(Color::by_index(9));
    // box0.set_label_size(36);
    // box0.set_align(Align::Clip);

    // let mut lt_flex = Flex::new(0, 0, 150, 150, None);
    //
    // lt_flex.set_type(FlexType::Row);
    // lt_flex.end();
    let lt_browser = HoldBrowser::new(0, 0, 150, 120, None);

    let mut w1 = window::Window::new(150, 0, 150, 150, "1");
    w1.set_frame(FrameType::NoBox);
    let mut box1 = frame::Frame::new(0, 0, 150, 150, "1\nThis is a child window");
    box1.set_frame(FrameType::DownBox);
    box1.set_color(Color::by_index(19));
    box1.set_label_size(18);
    box1.set_align(Align::Clip | Align::Inside | Align::Wrap);
    w1.resizable(&box1);
    let mut flex = Flex::new(0, 0, 150, 30, None);
    flex.set_type(FlexType::Row);
    let mut p1 = Pack::new(0, 0, 120, 30, None);
    p1.set_type(PackType::Horizontal);
    let mut button = Button::new(0, 0, 60, 30, "按钮");
    button.set_frame(FrameType::ThinUpFrame);
    Frame::new(0, 0, 10, 30, None);
    let mut button2 = Button::new(0, 60, 60, 30, "按钮2");
    button2.set_frame(FrameType::ThinUpFrame);
    p1.end();
    let p2 = Pack::new(0, 120, 30, 30, None);
    flex.fixed(&p2, 30);
    flex.end();
    let mut editor = TextEditor::new(0, 30, 150, 120, None);
    editor.set_buffer(TextBuffer::default());
    w1.end();

    let mut box2a = frame::Frame::new(0, 120, 70, 180, "2a");
    box2a.set_frame(FrameType::DownBox);
    box2a.set_color(Color::by_index(12));
    box2a.set_label_size(36);
    box2a.set_align(Align::Clip);

    let mut box2b = frame::Frame::new(70, 120, 80, 180, "2b");
    box2b.set_frame(FrameType::DownBox);
    box2b.set_color(Color::by_index(13));
    box2b.set_label_size(36);
    box2b.set_align(Align::Clip);

    let mut box3a = frame::Frame::new(150, 150, 150, 70, "3a");
    box3a.set_frame(FrameType::DownBox);
    box3a.set_color(Color::by_index(12));
    box3a.set_label_size(36);
    box3a.set_align(Align::Clip);

    let mut box3b = frame::Frame::new(150, 150 + 70, 150, 80, "3b");
    box3b.set_frame(FrameType::DownBox);
    box3b.set_color(Color::by_index(13));
    box3b.set_label_size(36);
    box3b.set_align(Align::Clip);

    tile.end();
    window.end();

    w1.show();
    window.show();

    app.run().unwrap();
}
