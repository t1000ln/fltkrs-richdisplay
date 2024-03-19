use std::cmp::{max};
use std::collections::{BTreeMap};
use std::sync::Arc;
use parking_lot::RwLock;
use crate::{LinedData, LinePiece, RichData};

/// 屏幕光标位置信息，以行、列的方式表示。
/// 参照`ANSI/CSI`的标准设计，行、列均从1开始。
#[derive(Debug, Clone)]
pub struct CursorPos {
    /// 行，起始值1。
    pub n: usize,
    /// 列，起始值1。
    pub m: usize,
    /// 当前最大行数。
    pub max_n: usize,
    /// 当前最大列数。
    pub max_m: usize,
}

// impl Default for CursorPos {
//     fn default() -> Self {
//         Self {n: 1, m: 1}
//     }
// }

impl CursorPos {
    pub fn new(n: usize, m: usize, max_n: usize, max_m: usize) -> Self {
        Self { n: max(n, 1), m: max(m, 1), max_n: max(max_n, 2), max_m: max(max_m, 2) }
    }

    // /// 设置最大行、列数。
    // ///
    // /// # Arguments
    // ///
    // /// * `max_n`: 最大行数。
    // /// * `max_m`: 最大列数。
    // ///
    // /// returns: ()
    // ///
    // /// # Examples
    // ///
    // /// ```
    // ///
    // /// ```
    // pub fn set_max(&mut self, max_n: usize, max_m: usize) {
    //     self.max_n = max(max_n, 2);
    //     self.max_m = max(max_m, 2);
    // }

    /// 直接设置光标位置。
    ///
    /// # Arguments
    ///
    /// * `n`: 第n行。
    /// * `m`: 第m列。
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn set(&mut self, n: usize, m: usize) {
        self.n = if n > self.max_n { self.max_n } else if n < 1 { 1 } else { n };
        self.m = if m > self.max_m { self.max_m } else if m < 1 { 1 } else { m };
    }

    /// 获取当前光标位置(行，列)。
    pub fn get(&self) -> (usize, usize) {
        (self.n, self.m)
    }

    // /// 设置光标位置到第n行。
    // ///
    // /// # Arguments
    // ///
    // /// * `n`:
    // ///
    // /// returns: ()
    // ///
    // /// # Examples
    // ///
    // /// ```
    // ///
    // /// ```
    // pub fn set_n(&mut self, n: usize) {
    //     self.n = if n > self.max_n { self.max_n } else if n < 1 { 1 } else { n };
    // }

    /// 设置光标位置到第m列。
    ///
    /// # Arguments
    ///
    /// * `m`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn set_m(&mut self, m: usize) {
        self.m = if m > self.max_m { self.max_m } else if m < 1 { 1 } else { m };
    }

    /// 设置光标位置上移n行。
    ///
    /// # Arguments
    ///
    /// * `n`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn sub_n(&mut self, n: usize) {
        self.n = max(self.n - n, 1);
    }

    /// 设置光标位置前移m列。
    ///
    /// # Arguments
    ///
    /// * `m`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn sub_m(&mut self, m: usize) {
        self.m = max(self.m - m, 1);
    }

    /// 设置光标位置下移n行。
    ///
    /// # Arguments
    ///
    /// * `n`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn add_n(&mut self, n: usize) {
        self.n += n;
    }

    /// 设置光标位置后移m列。
    ///
    /// # Arguments
    ///
    /// * `m`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn add_m(&mut self, m: usize) {
        self.m += m;
    }

    /// 获取坐标位置的状态报告。以`ESC[n;mR`形式返回。
    pub fn dsr(&self) -> String {
        format!("\x1b[{};{}R", self.n, self.m)
    }
}

/// 可反复擦写的光标定位显示板，用于CSI光标控制输出位置的场景。
#[derive(Debug)]
pub struct  ReWriteBoard {
    /// 行数
    pub max_rows: usize,
    /// 列数
    pub max_cols: usize,
    /// 显示板顶部相对y坐标。
    pub top_y: usize,
    /// 显示板底部相对y坐标。
    pub bottom_y: usize,
    /// 标准行高
    pub line_height: usize,
    /// 行间距
    pub line_space: usize,
    /// 数据行容器，key为行号，value为行数据。
    pub line_data_map: BTreeMap<usize, Vec<RichData>>,
    pub cursor_pos: CursorPos,
}

impl ReWriteBoard {
    pub fn new(max_rows: usize, max_cols: usize, top_y: usize, line_height: usize, line_space: usize) -> Self {
        let bottom_y = top_y + max_rows * line_height + line_space * (max_rows - 1);
        let line_data_map = BTreeMap::new();
        let cursor_pos = CursorPos::new(1, 1, max_rows, max_cols);

        Self {
            max_rows,
            max_cols,
            top_y,
            bottom_y,
            line_height,
            line_space,
            line_data_map,
            cursor_pos,
        }
    }


    pub fn resize(&mut self, rows: usize, cols: usize) {
        self.max_rows = rows;
        self.max_cols = cols;
        self.bottom_y = self.top_y + rows * self.line_height + self.line_space * (rows - 1);
    }

    /// 向面板中添加数据。
    ///
    /// # Arguments
    ///
    /// * `data`: 未试算的数据段。
    /// * `cursor_piece`: 当前虚拟光标信息。
    /// * `drawable_max_width`: 面板可绘制的最大宽度。
    /// * `basic_char`: 基本字符。
    ///
    /// returns: Option<Vec<RichData, Global>> 返回面板上所有的数据和超出面板的数据。
    /// 这些数据中的文本中已经去除了`"\r"`字符。
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn add_data(&mut self, data: RichData, cursor_piece: Arc<RwLock<LinePiece>>, drawable_max_width: i32, basic_char: char) -> Vec<RichData> {
        let mut exceed_board_data: Vec<RichData> = vec![];
        // {
        //     let (current_row, current_col) = self.cursor_pos.get();
        //     debug!("在起始光标位置 {current_row},{current_col} 处添加数据：{:?}", data.text);
        // }
        for line in data.text.split_inclusive("\r\n") {
            let (current_row, current_col) = self.cursor_pos.get();
            // debug!("在当前光标位置 {current_row},{current_col} 处添加数据：{:?}", line);
            // 将超出面板范围的行返回给上一级调用者
            let content = line.replace("\r", "");
            if current_row > self.max_rows {
                let mut rd = data.clone();
                rd.text = content;
                *cursor_piece.write() = rd.estimate(cursor_piece.clone(), drawable_max_width, basic_char).read().get_cursor();
                exceed_board_data.push(rd);
                // debug!("光标位置超出定位面板范围，即将退出定位面板。");
                continue;
            }

            // 将行数据存入对应行数据格子中。
            let mut rd = data.clone();
            rd.text = content.to_string();
            rd.rewrite_board_data = true;

            *cursor_piece.write() = rd.estimate(cursor_piece.clone(), drawable_max_width, basic_char).read().get_cursor();
            if !content.trim().is_empty() {
                let char_len = rd.text.chars().count();
                if let Some(line) = self.line_data_map.get_mut(&current_row) {
                    if current_col == 1 {
                        // 如果实在行首添加数据，则将本行数据清空后再添加。
                        // debug!("在行首添加数据：{:?}", content);
                        line.clear();
                    }
                    line.push(rd);
                } else {
                    self.line_data_map.insert(current_row, vec![rd]);
                }
                self.cursor_pos.add_m(char_len);
            }

            // 如果文本以换行符结尾，则将光标下移一行。
            if content.ends_with("\n") {
                self.cursor_pos.add_n(1);
                self.cursor_pos.set_m(1);
            }
        }

        let mut all = self.line_data_map.values().cloned().flatten().collect::<Vec<RichData>>();
        all.append(&mut exceed_board_data);
        all
    }

    pub fn erase_in_line(&mut self, erase_mode: u8) {
        let (row, col) = self.cursor_pos.get();
        // let col_idx = col - 1;
        match erase_mode {
            1 => {
                // 从光标位置擦除到行首。
                // debug!("在面板位置 {row},{col} 处开始擦除到行首");
                if let Some(rds) = self.line_data_map.get_mut(&row) {
                    let mut char_count_sum = 0;
                    for rd in rds.iter_mut() {
                        let chars = rd.text.chars();
                        let chars_len = chars.count();
                        if char_count_sum + chars_len > col && char_count_sum < col {
                            let sub_char_len = col - char_count_sum;
                            let sub_text_len = rd.text.chars().take(sub_char_len).collect::<String>().len();
                            rd.text.replace_range(..sub_text_len, " ".repeat(sub_char_len).as_str());
                            if let Some(fp) = rd.line_pieces.first_mut() {
                                fp.write().line = rd.text.clone();
                            }
                            break;
                        } else {
                            rd.text.replace_range(.., " ".repeat(chars_len).as_str());
                            if let Some(fp) = rd.line_pieces.first_mut() {
                                fp.write().line = rd.text.clone();
                            }
                            char_count_sum += chars_len;
                        }
                    }
                }
            }
            2 => {
                // 擦除整行。
                // debug!("在面板位置 {row},{col} 处开始擦除整行");
                // self.line_data_map.remove(&row);
                let empty_line_str = " ".repeat(self.max_cols);
                if let Some(rds) = self.line_data_map.get_mut(&row) {
                    if let Some(first) = rds.first_mut() {
                        first.text.replace_range(.., empty_line_str.as_str());
                        if let Some(fp) = first.line_pieces.first_mut() {
                            fp.write().line = first.text.clone();
                        }
                    }
                    rds.truncate(1);
                }
            }
            _ => {
                // 从光标位置擦除到行尾。
                // debug!("在面板位置 {row},{col} 处开始擦除到行尾");
                if let Some(rds) = self.line_data_map.get_mut(&row) {
                    if col == 1 {
                        rds.clear();
                    } else {
                        let (mut drain, mut idx) = (false, 0);
                        let mut char_count_sum = 0;
                        for (rd_idx, rd) in rds.iter_mut().enumerate() {
                            let chars = rd.text.chars();
                            let char_len = chars.count();
                            let text_len = rd.text.len();
                            // debug!("擦除到行尾时：col:{col}, char_count_sum:{char_count_sum}");

                            if char_count_sum + char_len > col {
                                if col >= char_count_sum {
                                    let sub_len = rd.text.chars().take(col - char_count_sum).collect::<String>().len();
                                    rd.text.replace_range(sub_len..text_len, " ".repeat(char_count_sum + char_len - col).as_str());
                                    if let Some(fp) = rd.line_pieces.first_mut() {
                                        fp.write().line = rd.text.clone();
                                    }
                                } else {
                                    drain = true;
                                    idx = rd_idx;
                                    break;
                                }
                            }
                            char_count_sum += char_len;
                        }
                        if drain {
                            rds.drain(idx..);
                        }
                    }
                }
            }
        }
    }

    pub fn erase_in_display(&mut self, erase_mode: u8) {
        // debug!("擦除屏幕 {erase_mode}");
        let (mut row, col) = self.cursor_pos.get();
        // let col_idx = col - 1;
        match erase_mode {
            1 => {
                // 从光标位置擦除到面板左上角。
                // debug!("擦除到面板左上角");
                if let Some(rds) = self.line_data_map.get_mut(&row) {
                    let mut char_count_sum = 0;
                    for rd in rds.iter_mut() {
                        let chars = rd.text.chars();
                        let chars_len = chars.count();
                        if char_count_sum + chars_len > col && char_count_sum < col {
                            let sub_char_len = col - char_count_sum;
                            let sub_text_len = rd.text.chars().take(sub_char_len).collect::<String>().len();
                            rd.text.replace_range(..sub_text_len, " ".repeat(sub_char_len).as_str());
                            if let Some(fp) = rd.line_pieces.first_mut() {
                                fp.write().line = rd.text.clone();
                            }
                            break;
                        } else {
                            rd.text.replace_range(.., " ".repeat(chars_len).as_str());
                            if let Some(fp) = rd.line_pieces.first_mut() {
                                fp.write().line = rd.text.clone();
                            }
                            char_count_sum += chars_len;
                        }
                    }
                }
                while row > 1 {
                    row -= 1;
                    let empty_line_str = " ".repeat(self.max_cols);
                    if let Some(rds) = self.line_data_map.get_mut(&row) {
                        if let Some(first) = rds.first_mut() {
                            first.text.replace_range(.., empty_line_str.as_str());
                            if let Some(fp) = first.line_pieces.first_mut() {
                                fp.write().line = first.text.clone();
                            }
                        }
                        rds.truncate(1);
                    }
                }
            }
            2 | 3 => {
                // 擦除整个面板。
                // debug!("面板全部擦除");
                let empty_line_str = " ".repeat(self.max_cols);
                for r in 1..=self.max_rows {
                    if let Some(rds) = self.line_data_map.get_mut(&r) {
                        if let Some(first) = rds.first_mut() {
                            first.text.replace_range(.., empty_line_str.as_str());
                            if let Some(fp) = first.line_pieces.first_mut() {
                                fp.write().line = first.text.clone();
                            }
                        }
                        rds.truncate(1);
                    }
                }
            }
            _ => {
                // 从光标位置擦除到面板末尾。
                // debug!("擦除到面板右下角");
                if let Some(rds) = self.line_data_map.get_mut(&row) {
                    let (mut drain, mut idx) = (false, 0);
                    let mut char_count_sum = 0;
                    for (rd_idx, rd) in rds.iter_mut().enumerate() {
                        let chars = rd.text.chars();
                        let char_len = chars.count();
                        let text_len = rd.text.len();
                        // debug!("擦除到行尾时：col:{col}, char_count_sum:{char_count_sum}");

                        if char_count_sum + char_len > col {
                            if col >= char_count_sum {
                                let sub_len = rd.text.chars().take(col - char_count_sum).collect::<String>().len();
                                rd.text.replace_range(sub_len..text_len, " ".repeat(char_count_sum + char_len - col).as_str());
                                if let Some(fp) = rd.line_pieces.first_mut() {
                                    fp.write().line = rd.text.clone();
                                }
                            } else {
                                drain = true;
                                idx = rd_idx;
                                break;
                            }
                        }
                        char_count_sum += char_len;
                    }
                    if drain {
                        rds.drain(idx..);
                    }
                }
                while row < self.max_rows {
                    row += 1;
                    let empty_line_str = " ".repeat(self.max_cols);
                    if let Some(rds) = self.line_data_map.get_mut(&row) {
                        if let Some(first) = rds.first_mut() {
                            first.text.replace_range(.., empty_line_str.as_str());
                            if let Some(fp) = first.line_pieces.first_mut() {
                                fp.write().line = first.text.clone();
                            }
                        }
                        rds.truncate(1);
                    }
                }
            }
        }
    }
}