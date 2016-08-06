extern crate termion;
extern crate nix;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate log;
extern crate env_logger;

mod syntax_highlight;
mod editor;

use std::env;
use std::io;
use std::io::{Write};
use std::sync::atomic::{AtomicBool, ATOMIC_BOOL_INIT, Ordering};

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::terminal_size;

use nix::sys::signal;

const HELP_MSG: &'static str = "HELP: CTRL-S to save the file, and CTRL-Q to quit.";
const BACKSPACE: char = 8 as char;
#[allow(non_upper_case_globals)]
static ShouldResizeWindow: AtomicBool = ATOMIC_BOOL_INIT;

macro_rules! render {
    ($editor:ident, $stdout:ident) => {
        $editor.render(&mut $stdout).expect("Failed to render");
        $stdout.flush().unwrap();
    }
}

extern fn signal_handler(signo: i32) {
    if signo == signal::SIGWINCH {
        ShouldResizeWindow.store(true, Ordering::Relaxed);
    }
}

fn main() {
    env_logger::init().expect("failed to initialize logging");

    let filename = env::args().nth(1).expect("Usage: mutxt <filename>");
    let (screen_cols, screen_rows) = terminal_size().expect("Could not get the terminal size");
    let stdin = io::stdin();
    let mut stdout = io::stdout().into_raw_mode().expect("Could not put stdout into raw mode");
    let mut editor = editor::Editor::new(screen_rows as usize, screen_cols as usize);
    editor.status_message = Some(HELP_MSG.to_owned());
    editor.open_file(&filename).expect("Could not open the file provided");
    render!(editor, stdout);

    unsafe {
        signal::sigaction(signal::SIGWINCH,
                          &signal::SigAction::new(
                              signal::SigHandler::Handler(signal_handler),
                              signal::SaFlags::empty(),
                              signal::SigSet::empty())).expect("failed to set signal handler");
    }

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
                editor.save_file().expect("failed to save file");
                render!(editor, stdout);
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
            Key::Ctrl('a') | Key::Home => {
                unimplemented!()
            },
            Key::Ctrl('e') | Key::End => {
                unimplemented!()
            },
            Key::Ctrl('w') | Key::Ctrl(BACKSPACE) => {
                unimplemented!()
            },
            Key::Backspace | Key::Ctrl('h') | Key::Delete => {
                editor.backspace();
                render!(editor, stdout);
            },
            Key::Ctrl('l') => {
                let (screen_cols, screen_rows) = terminal_size().expect("Could not get the terminal size");
                editor.set_screen_size(screen_rows as usize, screen_cols as usize);
                render!(editor, stdout);
            },
            Key::Ctrl('q') => break,
            Key::Char('\n') => {
                editor.newline();
                render!(editor, stdout);
            },
            Key::Char(c) => {
                editor.insert_char(c);
                render!(editor, stdout);
            },
            _ => {}
        }
        if ShouldResizeWindow.compare_and_swap(true, false, Ordering::Relaxed) {
            let (screen_cols, screen_rows) = terminal_size().expect("Could not get the terminal size");
            editor.set_screen_size(screen_rows as usize, screen_cols as usize);
            render!(editor, stdout);
        }
    }

    write!(stdout, "{}{}", termion::clear::All, termion::cursor::Show).unwrap();
}
