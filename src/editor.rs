use std::io;
use std::io::{Write};
use std::iter;
use std::io::{BufRead, BufReader};
use std::fs::{File, OpenOptions};
use syntax_highlight::{HighlightParams, HighlightType};
use termion::{cursor, clear, color, style};
use termion::raw::IntoRawMode;

const TAB: char = '\t';
const SPACES_PER_TAB: usize = 4;
const VERSION : &'static str = env!("CARGO_PKG_VERSION");

/// A type defining elements of syntax
pub struct SyntaxHighlightRule {
    /// The keywords in the file
    keywords: Vec<String>,
    single_line_comment_start: Vec<String>,
    multi_line_comment_start: String,
    multi_line_comment_end: String,
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
    filename: Option<String>,
    /// A message to display on the screen
    pub status_message: Option<String>,
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
            screen_rows: screen_rows-2,
            screen_cols: screen_cols,
            rows: vec![],
            modified: false,
            filename: None,
            status_message: None,
            syntax_highlight: None,
        }
    }

    pub fn set_screen_size(&mut self, screen_rows: usize, screen_cols: usize) {
        self.screen_rows = screen_rows - 2;
        self.screen_cols = screen_cols;
    }

    pub fn render<W: Write>(&self, out: &mut W) -> io::Result<()> {
        try!(write!(out, "{}{}{}", cursor::Hide, cursor::Goto(1, 1), clear::All));
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
                    try!(write!(out, "~\r\n"));
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
                    use syntax_highlight::HighlightType::*;
                    match hl {
                        NonPrint => {
                            unimplemented!();
                        },
                        Normal => {
                            try!(write!(out, "{}{}", color::Fg(color::White), c));
                        },
                        _ => {
                            // TODO get colors and use them
                            unimplemented!();
                        }
                    }
                }
            }
            try!(write!(out, "{}\r\n", color::Fg(color::White)));
        }
        // Render status bar
        try!(write!(out, "{}", style::Invert));
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
        try!(write!(out, "{0}{1: >2$}", lhs_status, rhs_status, padding));
        try!(write!(out, "{}\r\n", style::Reset));
        match self.status_message {
            Some(ref msg) => {
                try!(write!(out, "{}\r\n", msg));
            },
            None => {}
        };

        // Put the cursor in the right spot.
        try!(write!(out, "{}{}", cursor::Goto((self.cursor_y+1) as u16, self.cursor_x as u16), cursor::Show));
        Ok(())
    }

    pub fn open_file(&mut self, filename: &str) -> io::Result<()> {
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
                        if !self.top_edge() {
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
                if self.top_edge() && self.row_offset > 0 {
                    self.row_offset -= 1;
                } else if !self.top_edge() {
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
}
