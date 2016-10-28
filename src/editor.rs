use std::io;
use std::io::{Write};
use std::iter;
use std::io::{BufRead, BufReader};
use std::fs::{File, OpenOptions};
use std::collections::{HashSet};
use termion::{cursor, clear, color, style};

const TAB: char = '\t';
const SPACES_PER_TAB: usize = 4;
const VERSION : &'static str = env!("CARGO_PKG_VERSION");

bitflags! {
    pub flags HighlightParams: u8 {
        const HighlightStrings = (1 << 0),
        const HighlightNumbers = (1 << 1),
    }
}

#[derive(Copy, Clone)]
pub enum HighlightType {
    Normal,
    NonPrint,
    Comment,
    Keyword,
    String,
    Number,
    Selection,
}

/// A type defining elements of syntax
pub struct SyntaxHighlightRule {
    /// The keywords of the language
    keywords: HashSet<String>,
    /// The start character sequence for a single-line comment
    single_line_comment_start: HashSet<String>,
    /// The start character sequence for a multi-line comment
    multi_line_comment_start: String,
    /// The end character sequence for a multi-line comment
    multi_line_comment_end: String,
    /// Flags specifying what elements of syntax should be highlighted
    params: HighlightParams,
}

pub struct Row {
    index_in_file: usize,
    content: String,
}

pub type RenderedRow = Vec<(char, HighlightType)>;

impl Row {
    pub fn new(at: usize, content: &str) -> Self {
        Row {
            index_in_file: at,
            content: content.to_owned(),
        }
    }

    pub fn with_content(content: &str) -> Self {
        Row {
            index_in_file: 0,
            content: content.to_owned(),
        }
    }

    pub fn empty() -> Self {
        Row {
            index_in_file: 0,
            content: String::new(),
        }
    }

    pub fn render(&self) -> RenderedRow {
        self.content.chars()
            .flat_map(|ch| {
                if ch == TAB {
                    iter::repeat((' ', HighlightType::Normal)).take(SPACES_PER_TAB)
                } else {
                    iter::repeat((ch, HighlightType::Normal)).take(1)
                }
            }).collect()
    }

    pub fn push_str(&mut self, s: String) {
        self.content.push_str(&s);
    }

    pub fn backspace(&mut self, at: usize) {
        self.content.remove(at-1);
    }

    pub fn insert_char(&mut self, at: usize, c: char) {
        self.content.insert(at, c);
    }

    #[inline(always)]
    pub fn column_exists(&self, col_idx: usize) -> bool {
        col_idx < self.content.len()
    }

    #[inline(always)]
    pub fn is_end(&self, col_idx: usize) -> bool {
        col_idx == self.content.len()
    }
}

/// A type defining the editor's state.
pub struct Editor {
    /// The cursor's x-position in characters.
    cursor_x: usize,
    /// The cursor's y-position in characters.
    cursor_y: usize,
    /// The row offset into the file.
    row_offset: usize,
    /// The column offset into the file.
    col_offset: usize,
    /// The number of rows able to be displayed on screen
    screen_rows: usize,
    /// The number of columns able to be displayed on screen
    screen_cols: usize,
    /// The editor rows
    rows: Vec<Row>,
    /// Whether the file has been modified but not saved
    modified: bool,
    /// The current open file
    pub filename: Option<String>,
    /// A message to display on the screen
    status_message: Option<String>,
    /// The syntax highlighting rule configured.
    syntax_highlight: Option<SyntaxHighlightRule>,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum CursorDirection {
    Up,
    Down,
    Left,
    Right,
}

impl Editor {
    pub fn new(screen_rows: usize, screen_cols: usize) -> Self {
        Editor {
            cursor_x: 0,
            cursor_y: 0,
            row_offset: 0,
            col_offset: 0,
            screen_rows: screen_rows-3,
            screen_cols: screen_cols,
            rows: vec![],
            modified: false,
            filename: None,
            status_message: None,
            syntax_highlight: None,
        }
    }

    pub fn set_screen_size(&mut self, screen_rows: usize, screen_cols: usize) {
        self.screen_rows = screen_rows - 3;
        self.screen_cols = screen_cols;
    }

    pub fn render<W: Write>(&self, out: &mut W) -> io::Result<()> {
        try!(write!(out, "{}{}", cursor::Hide, cursor::Goto(1, 1)));
        // now we render the text, line by line
        for y in 0..self.screen_rows {
            let file_row = self.row_offset + y;

            if file_row >= self.rows.len() {
                if self.rows.len() == 0 && y == self.screen_rows / 3 {
                    let title = format!("mutxt version {}{}{}\r\n", style::Bold, VERSION, style::Reset);
                    let padding = (self.screen_cols - title.len())/2;
                    let padding_str: String = iter::repeat(' ').take(padding-1).collect();
                    try!(write!(out, "~{}{}", padding_str, title));
                } else {
                    try!(write!(out, "~{}\r\n", clear::AfterCursor));
                }
                continue;
            }

            let rendered_row = self.rows[file_row].render();
            let mut len = rendered_row.len() - self.col_offset;
            if len > 0 {
                if len > self.screen_cols {
                    len = self.screen_cols;
                }
                for (c, hl) in rendered_row.into_iter().skip(self.col_offset).take(len) {
                    // TODO use configurable colors
                    use self::HighlightType::*;
                    match hl {
                        NonPrint => {
                            try!(write!(out, "{}{}{}", color::Fg(color::Reset),
                                        color::Bg(color::Reset), c));
                        },
                        Normal => {
                            try!(write!(out, "{}{}", color::Fg(color::White), c));
                        },
                        Comment => {
                            try!(write!(out, "{}{}", color::Fg(color::Cyan), c));
                        },
                        Keyword => {
                            try!(write!(out, "{}{}", color::Fg(color::Magenta), c));
                        },
                        String => {
                            try!(write!(out, "{}{}", color::Fg(color::Green), c));
                        },
                        Number => {
                            try!(write!(out, "{}{}", color::Fg(color::Blue), c));
                        },
                        Selection => {
                            try!(write!(out, "{}{}{}", color::Bg(color::LightBlack),
                                        color::Fg(color::White), c));
                        }
                    }
                }
            }
            try!(write!(out, "{}{}\r\n", color::Fg(color::White), clear::AfterCursor));
        }
        // Render status bar
        try!(write!(out, "{}{}", clear::AfterCursor, style::Invert));
        let modified_str = if self.modified {
            " (modified)"
        } else {
            ""
        };
        let filename_str = match self.filename {
            Some(ref f) => &f,
            None => "(no file)"
        };
        let lhs_status = format!("{}{} - {} lines", filename_str, modified_str, self.rows.len());
        let rhs_status = format!("{}/{}", self.row_offset+self.cursor_y+1, self.rows.len());
        let padding = self.screen_cols - lhs_status.len();
        try!(write!(out, "{0}{1}{2: >3$}", clear::AfterCursor, lhs_status, rhs_status, padding));
        try!(write!(out, "{}\r\n", style::Reset));
        match self.status_message {
            Some(ref msg) => {
                try!(write!(out, "{}\r\n", msg));
            },
            None => {}
        };

        // Put the cursor in the right spot.
        let mut cx = 1;
        let file_row = self.row_offset + self.cursor_y;
        if file_row < self.rows.len() {
            for j in self.col_offset..(self.cursor_x + self.col_offset) {
                if let Some(ch) = self.rows[file_row].content.chars().nth(j) {
                    if ch == TAB {
                        cx += 3-((cx)%4);
                    }
                    cx += 1;
                }
            }
        }
        try!(write!(out, "{}{}", cursor::Goto(cx as u16, (self.cursor_y+1) as u16), cursor::Show));
        Ok(())
    }

    pub fn open_file(&mut self, filename: &str) -> io::Result<()> {
        debug!("open_file {}", filename);
        self.modified = false;
        self.filename = Some(filename.to_owned());
        let file = try!(OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .open(filename));
        let input = BufReader::new(&file);
        self.rows.clear();
        for (at, line) in input.lines().enumerate() {
            let l = try!(line);
            self.rows.push(Row::new(at, &l));
        }
        Ok(())
    }

    pub fn save_file(&mut self) -> io::Result<()> {
        if !self.modified {
            return Ok(());
        }
        debug!("save_file - {} rows", self.rows.len());

        let filename = self.filename.as_ref().expect("(not possible in current version) filename not specified");

        let mut file = try!(OpenOptions::new()
                            .write(true)
                            .truncate(true)
                            .create(true)
                            .open(filename));
        for row in &self.rows {
            try!(writeln!(file, "{}", row.content));
        }
        self.modified = false;
        Ok(())
    }


    #[inline(always)]
    fn left_edge(&self) -> bool {
        self.cursor_x == 0
    }

    #[inline(always)]
    fn right_edge(&self) -> bool {
        self.cursor_x == self.screen_cols - 1
    }

    #[inline(always)]
    fn top_edge(&self) -> bool {
        self.cursor_y == 0
    }

    #[inline(always)]
    fn bottom_edge(&self) -> bool {
        self.cursor_y == self.screen_rows - 1
    }

    pub fn move_cursor(&mut self, dir: CursorDirection) {
        use self::CursorDirection::*;
        let file_row = self.row_offset + self.cursor_y;
        let file_col = self.col_offset + self.cursor_x;
        let row_exists = file_row < self.rows.len();

        match dir {
            Left => {
                if self.left_edge() {
                    // We're on the left edge of the terminal window
                    if self.col_offset > 0 {
                        // Shift the frame over
                        self.col_offset -= 1;
                    } else {
                        // Move to the end of the previous line
                        if file_row > 0 {
                            self.cursor_y -= 1;
                            self.cursor_x = self.rows[file_row-1].content.len();
                            if self.cursor_x > self.screen_cols - 1 {
                                self.col_offset = self.cursor_x - self.screen_cols + 1;
                                self.cursor_x = self.screen_cols - 1;
                            }
                        }
                    }
                } else {
                    // We aren't on the left edge, just move the cursor one to the left
                    self.cursor_x -= 1;
                }
            },
            Right => {
                if row_exists && self.rows[file_row].column_exists(file_col) {
                    if self.right_edge() {
                        self.col_offset += 1;
                    } else {
                        self.cursor_x += 1;
                    }
                } else if row_exists && self.rows[file_row].is_end(file_col) {
                    self.cursor_x = 0;
                    self.col_offset = 0;
                    if self.bottom_edge() {
                        self.row_offset += 1;
                    } else {
                        self.cursor_y += 1;
                    }
                }
            },
            Up => {
                if self.top_edge() {
                    if self.row_offset > 0 {
                        self.row_offset -= 1;
                    }
                } else {
                    self.cursor_y -= 1;
                }
            },
            Down => {
                if file_row < self.rows.len() {
                    if self.bottom_edge() {
                        self.row_offset += 1;
                    } else {
                        self.cursor_y += 1;
                    }
                }
            }
        }

        // recalculate position
        let new_file_row = self.row_offset + self.cursor_y;
        let new_file_col = self.col_offset + self.cursor_x;
        if let Some(row) = self.rows.get(new_file_row) {
            if new_file_col > row.content.len() {
                let cx = (self.cursor_x as isize) - ((new_file_col as isize) - (row.content.len() as isize));
                if cx < 0 {
                    self.col_offset = ((self.col_offset as isize) + cx) as usize;
                    self.cursor_x = 0;
                } else {
                    self.cursor_x = cx as usize;
                }
            }
        }
    }

    pub fn cursor_to_start_of_line(&mut self) {
        self.col_offset = 0;
        self.cursor_x = 0;
    }

    pub fn cursor_to_end_of_line(&mut self) {
        let file_row = self.row_offset + self.cursor_y;
        let file_col = self.col_offset + self.cursor_x;
        let row_exists = file_row < self.rows.len();

        if row_exists {
            for _ in file_col..self.rows[file_row].content.len() {
                self.move_cursor(CursorDirection::Right)
            }
        }
    }

    fn find_previous_non_alphanumeric_char_index(&self) -> Option<usize> {
        // Search backwards for the previous non-alphanumeric character
        let file_row = self.row_offset + self.cursor_y;
        let file_col = self.col_offset + self.cursor_x;

        if file_row < self.rows.len() {
            Some({
                let row = &self.rows[file_row];
                let len = row.content.len();
                // Find the index of the previous non-alphanumeric character
                row.content.chars()
                    .rev()
                    .skip(len - file_col)
                    .take_while(|c| c.is_alphanumeric())
                    .count()
            })
        } else {
            None
        }
    }

    pub fn cursor_to_left_word(&mut self) {
        // Search backwards for the previous non-alphanumeric character
        if let Some(num) = self.find_previous_non_alphanumeric_char_index() {
            for _ in 0..(num+1) {
                self.move_cursor(CursorDirection::Left);
            }
        }
    }

    pub fn cursor_to_right_word(&mut self) {
        // Search forwards for the next non-alphanumeric character
        let file_row = self.row_offset + self.cursor_y;
        let file_col = self.col_offset + self.cursor_x;

        if file_row < self.rows.len() {
            let num = {
                let row = &self.rows[file_row];
                // Find the next index
                row.content.chars()
                    .skip(file_col+1)
                    .take_while(|c| c.is_alphanumeric())
                    .count()+1
            };
            for _ in 0..num {
                self.move_cursor(CursorDirection::Right);
            }
        }
    }

    pub fn page_cursor(&mut self, dir: CursorDirection) {
        if dir == CursorDirection::Up && !self.top_edge() {
            self.cursor_y = 0;
        } else if dir == CursorDirection::Down && !self.bottom_edge() {
            self.cursor_y = self.screen_rows - 1;
        }
        for _ in 0..self.screen_rows {
            self.move_cursor(dir);
        }
    }

    pub fn backspace(&mut self) {
        debug!("backspace");
        let file_row = self.row_offset + self.cursor_y;
        let file_col = self.col_offset + self.cursor_x;
        if file_row >= self.rows.len() || (file_col == 0 && file_row == 0) {
            return;
        }
        if file_col == 0 {
            // Append to the prior row, then delete the current row
            let content = self.rows[file_row].content.clone();
            let prior_row_len = self.rows[file_row-1].content.len();
            self.rows[file_row-1].push_str(content);
            self.rows.remove(file_row);
            if self.cursor_y == 0 {
                self.row_offset -= 1;
            } else {
                self.cursor_y -= 1;
            }
            self.cursor_x = prior_row_len;
            if self.cursor_x >= self.screen_cols {
                let shift = (self.screen_cols - self.cursor_x) + 1;
                self.cursor_x -= shift;
                self.col_offset += shift;
            }
        } else {
            // Just drop the char from the row
            self.rows[file_row].backspace(file_col);
            if self.left_edge() && self.col_offset > 0 {
                self.col_offset -= 1;
            } else if !self.left_edge() {
                self.cursor_x -= 1;
            }
        }

        self.modified = true;
        debug!("number of rows: {}", self.rows.len());
    }

    pub fn backspace_word(&mut self) {
        if let Some(num_backspaces) = self.find_previous_non_alphanumeric_char_index() {
            for _ in 0..num_backspaces {
                self.backspace();
            }
        }
    }

    pub fn backspace_to_start_of_line(&mut self) {
        // Simply delete from here to the beginning of the line
        let file_row = self.row_offset + self.cursor_y;
        let file_col = self.col_offset + self.cursor_x;

        if file_row < self.rows.len() {
            for _ in 0..file_col {
                self.backspace()
            }
        }
    }

    pub fn empty_status(&mut self) {
        self.status_message = None
    }

    pub fn display_status<S: AsRef<str>>(&mut self, status: S) {
        self.status_message = Some(status.as_ref().to_owned());
    }

    pub fn insert_str<S: AsRef<str>>(&mut self, text: S) {
        for c in text.as_ref().chars() {
            self.insert_char(c);
        }
    }

    pub fn insert_char(&mut self, c: char) {
        let file_row = self.row_offset + self.cursor_y;
        let file_col = self.col_offset + self.cursor_x;

        if file_row >= self.rows.len() {
            while self.rows.len() <= file_row {
                self.rows.push(Row::empty());
            }
        }

        self.rows[file_row].insert_char(file_col, c);
        if self.right_edge() {
            self.col_offset += 1;
        } else {
            self.cursor_x += 1;
        }

        self.modified = true;
    }

    pub fn newline(&mut self) {
        debug!("newline");
        let file_row = self.row_offset + self.cursor_y;
        let mut file_col = self.col_offset + self.cursor_x;

        use std::cmp::Ordering::*;
        match file_row.cmp(&self.rows.len()) {
            Greater => return,
            Equal => {
                self.rows.push(Row::empty());
            },
            Less => {
                if file_col >= self.rows[file_row].content.len() {
                    file_col = self.rows[file_row].content.len();
                }

                if file_col == 0 {
                    self.rows.insert(file_row, Row::empty());
                } else {
                    // Split the current row in TWO!
                    let content = self.rows[file_row].content.clone();
                    let (prior_row, new_row) = content.split_at(file_col);
                    self.rows.insert(file_row+1, Row::with_content(new_row));
                    self.rows[file_row].content = prior_row.to_owned();
                }
            }
        }
        // Fix the cursor position
        if self.bottom_edge() {
            self.row_offset += 1;
        } else {
            self.cursor_y += 1;
        }

        self.cursor_x = 0;
        self.col_offset = 0;
        self.modified = true;
        debug!("number of rows: {}", self.rows.len());
    }
}
