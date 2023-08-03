pub mod rich_text;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use fltk::enums::Font;
    use crate::rich_text::{LineCoord, LinedData, Padding, RichData};
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }


    #[test]
    pub fn test_estimate() {
        // let rich_text = RichData::new_text("安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。".to_string());
        let rich_text = RichData::new_text("asdfh\nasdf\n".to_string());
        let from_y = 5;
        let mut below_line = LineCoord {
            x: 5,
            y: from_y,
            line_height: 0,
            line_spacing: 0,
            padding: Padding {left: 5, top: 5, right: 10, bottom: 5},
        };
        rich_text.estimate(&mut below_line, 785);
        println!("below_line: {:?}", below_line);
        let increased_height = from_y - below_line.y;
        println!("increased_height: {}", increased_height);

        let rich_text = RichData::new_text("asdfh\nasdf\n".to_string()).set_font(Font::HelveticaBold, 32);
        let from_y = below_line.y;
        rich_text.estimate(&mut below_line, 785);
        println!("below_line: {:?}", below_line);
        let increased_height = from_y - below_line.y;
        println!("increased_height: {}", increased_height);
    }
}
