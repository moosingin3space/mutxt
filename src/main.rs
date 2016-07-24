extern crate termion;
extern crate nix;
#[macro_use]
extern crate bitflags;

mod syntax_highlight;
mod editor;

use std::env;
use std::io;
use std::io::{Write};

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::terminal_size;

const HELP_MSG: &'static str = "HELP: CTRL-O to open a file, CTRL-S to save the current file, and CTRL-Q to quit.";

macro_rules! render {
    ($editor:ident, $stdout:ident) => {
        $editor.render(&mut $stdout).expect("Failed to render");
        $stdout.flush().unwrap();
    }
}

fn main() {
    let filename = env::args().nth(1).expect("Usage: mutxt <filename>");
    let (screen_cols, screen_rows) = terminal_size().expect("Could not get the terminal size");
    let stdin = io::stdin();
    let mut stdout = io::stdout().into_raw_mode().expect("Could not put stdout into raw mode");
    let mut editor = editor::Editor::new(screen_rows as usize, screen_cols as usize);
    editor.status_message = Some(HELP_MSG.to_owned());
    editor.open_file(&filename).expect("Could not open the file provided");
    render!(editor, stdout);

    for c in stdin.keys() {
        match c.unwrap() {
            Key::Up => {
                editor.move_cursor(editor::CursorDirection::Up);
                render!(editor, stdout);
            },
            Key::Down => {
                editor.move_cursor(editor::CursorDirection::Down);
                render!(editor, stdout);
            },
            Key::Left => {
                editor.move_cursor(editor::CursorDirection::Left);
                render!(editor, stdout);
            },
            Key::Right => {
                editor.move_cursor(editor::CursorDirection::Right);
                render!(editor, stdout);
            },
            Key::PageUp => {
                editor.page_cursor(editor::CursorDirection::Up);
                render!(editor, stdout);
            },
            Key::PageDown => {
                editor.page_cursor(editor::CursorDirection::Down);
                render!(editor, stdout);
            },
            Key::Ctrl('s') => {
                unimplemented!()
            },
            Key::Ctrl('o') => {
                unimplemented!()
            },
            Key::Ctrl('p') => {
                unimplemented!()
            },
            Key::Ctrl('f') => {
                unimplemented!()
            },
            Key::Backspace | Key::Ctrl('h') | Key::Delete => {
                // TODO delete char
            },
            Key::Ctrl('l') => {
                let (screen_cols, screen_rows) = terminal_size().expect("Could not get the terminal size");
                editor.set_screen_size(screen_rows as usize, screen_cols as usize);
                render!(editor, stdout);
            },
            Key::Ctrl('q') => break,
            _ => {
                // TODO insert the char
            }
        }
    }

    write!(stdout, "{}{}", termion::clear::All, termion::cursor::Show).unwrap();
}