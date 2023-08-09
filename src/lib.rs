pub mod rich_text;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;
    use fltk::enums::Font;
    use crate::rich_text::{LinedData, LinePiece, Padding, RichData, UserData};
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }


    #[test]
    pub fn test_estimate() {
        // let rich_text = RichData::new_text("安全并且高效地处理并发编程是Rust的另一个主要目标。并发编程和并行编程这两种概念随着计算机设备的多核优化而变得越来越重要。并发编程允许程序中的不同部分相互独立地运行；并行编程则允许程序中不同部分同时执行。".to_string());
        let padding = Padding::new(5, 5, 5, 5);
        let mut rich_text: RichData = UserData::new_text("asdfh\nasdf\n".to_string()).into();
        let from_y = 5;
        let mut last_piece = LinePiece {
            line: "".to_string(),
            x: 5,
            y: from_y,
            w: 0,
            h: 0,
            spacing: 0,
            next_x: 5,
            next_y: 5,
            font_height: 1,
        };
        rich_text.estimate(&mut last_piece, 785);
        println!("last_line: {:?}", last_piece);
        let increased_height = rich_text.height();
        println!("increased_height: {}", increased_height);

        let mut rich_text: RichData = UserData::new_text("asdfh\nasdf\n".to_string()).set_font(Font::HelveticaBold, 32).into();
        rich_text.estimate(&mut last_piece, 785);
        println!("last_line: {:?}", last_piece);
        let increased_height = rich_text.height();
        println!("increased_height: {}", increased_height);

        let mut rich_text: RichData = UserData::new_text("asdfh\nasdf\n".to_string()).set_font(Font::HelveticaBold, 16).into();
        rich_text.estimate(&mut last_piece, 785);
        println!("last_line: {:?}", last_piece);
        let increased_height = rich_text.height();
        println!("increased_height: {}", increased_height);
    }

    #[test]
    pub fn test_str() {
        let text = "0123456789";
        let t1 = &text[0..4];
        println!("t1: {:?}", t1);

        let mut tw = 0;
        let chars = text.chars();
        let seek = 6;
        if let Ok(p) = (0..(text.len() - 1)).collect::<Vec<usize>>().binary_search_by({
            move |pos| {
                println!("pos: {:?}", pos);
                tw = *pos;
                if pos <= &seek && pos + 1 > seek {

                    Ordering::Equal
                } else if pos > &seek {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            }
        }) {
            println!("p: {}, tw: {}", p, tw);
        } else {
            println!("tw: {:?}", tw);
        }
    }
}
