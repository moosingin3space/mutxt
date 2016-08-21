extern crate termion;
extern crate nix;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate clipboard;

mod syntax_highlight;
mod editor;
mod clip;

use std::env;
use std::io;
use std::io::{Write};
use std::sync::atomic::{AtomicBool, ATOMIC_BOOL_INIT, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{terminal_size, async_stdin};

use nix::sys::signal;

const HELP_MSG: &'static str = "HELP: CTRL-S to save the file, and CTRL-Q to quit.";
const EMPTY_STRING: &'static str = "";
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

    let status_gap = Duration::from_secs(10);
    let filename = env::args().nth(1).expect("Usage: mutxt <filename>");
    let (screen_cols, screen_rows) = terminal_size().expect("Could not get the terminal size");
    let mut stdin_keys = async_stdin().keys();
    let mut stdout = io::stdout().into_raw_mode().expect("Could not put stdout into raw mode");
    let mut editor = editor::Editor::new(screen_rows as usize, screen_cols as usize);
    let mut clipbrd = clip::Clipboard::new();
    editor.display_status(HELP_MSG);
    editor.open_file(&filename).expect("Could not open the file provided");
    render!(editor, stdout);

    unsafe {
        signal::sigaction(signal::SIGWINCH,
                          &signal::SigAction::new(
                              signal::SigHandler::Handler(signal_handler),
                              signal::SaFlags::empty(),
                              signal::SigSet::empty())).expect("failed to set signal handler");
    }

    let mut last_time_of_status = Instant::now();
    loop {
        if let Some(c) = stdin_keys.next() {
            match c.unwrap() {
                Key::Up => {
                    editor.move_cursor(editor::CursorDirection::Up);
                },
                Key::Down => {
                    editor.move_cursor(editor::CursorDirection::Down);
                },
                Key::Left => {
                    editor.move_cursor(editor::CursorDirection::Left);
                },
                Key::Right => {
                    editor.move_cursor(editor::CursorDirection::Right);
                },
                Key::PageUp => {
                    editor.page_cursor(editor::CursorDirection::Up);
                },
                Key::PageDown => {
                    editor.page_cursor(editor::CursorDirection::Down);
                },
                Key::Ctrl('s') => {
                    let filename = editor.filename.as_ref().unwrap().clone();
                    let status_msg = match editor.save_file() {
                        Ok(_) => format!("Successfully written to {}", filename),
                        Err(_) => "Failed to save file".to_owned(),
                    };
                    editor.display_status(status_msg);
                    last_time_of_status = Instant::now();
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
                Key::Ctrl('x') => {
                    unimplemented!()
                },
                Key::Ctrl('c') => {
                    unimplemented!()
                },
                Key::Ctrl('v') => {
                    editor.insert_str(clipbrd.get());
                },
                Key::Ctrl('a') | Key::Home => {
                    editor.cursor_to_start_of_line();
                },
                Key::Ctrl('e') | Key::End => {
                    editor.cursor_to_end_of_line();
                },
                Key::Ctrl('w') | Key::Ctrl('h') => {
                    editor.backspace_word();
                },
                Key::Ctrl('u') => {
                    editor.backspace_to_start_of_line();
                },
                Key::Backspace | Key::Delete => {
                    editor.backspace();
                },
                Key::Ctrl('l') => {
                    let (screen_cols, screen_rows) = terminal_size().expect("Could not get the terminal size");
                    editor.set_screen_size(screen_rows as usize, screen_cols as usize);
                },
                Key::Ctrl('q') => break,
                Key::Char('\n') => {
                    editor.newline();
                },
                Key::Char(c) => {
                    editor.insert_char(c);
                },
                _ => {}
            }
        }
        if ShouldResizeWindow.compare_and_swap(true, false, Ordering::Relaxed) {
            let (screen_cols, screen_rows) = terminal_size().expect("Could not get the terminal size");
            editor.set_screen_size(screen_rows as usize, screen_cols as usize);
        }

        thread::sleep(Duration::from_millis(50));
        if Instant::now() - last_time_of_status > status_gap {
            editor.display_status(EMPTY_STRING);
        }
        render!(editor, stdout);
    }

    write!(stdout, "{}{}", termion::clear::All, termion::cursor::Show).unwrap();
}
