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
mod keyboard;

use std::env;
use std::io;
use std::io::{Read, Write};
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
    let filename = env::args().nth(1)
        .expect("Usage: mutxt <filename>");
    let (screen_cols, screen_rows) = terminal_size()
        .expect("Could not get the terminal size");
    let mut stdin = keyboard::CommandReader::commands(async_stdin());
    let mut stdout = io::stdout().into_raw_mode()
        .expect("Could not put stdout into raw mode");
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
                              signal::SigSet::empty()))
            .expect("failed to set signal handler");
    }

    let mut last_time_of_status = Instant::now();
    loop {
        if let Some(command) = stdin.next() {
            use keyboard::Command::*;
            match command {
                MoveUp => {
                    editor.move_cursor(editor::CursorDirection::Up);
                },
                MoveDown => {
                    editor.move_cursor(editor::CursorDirection::Down);
                },
                MoveLeft => {
                    editor.move_cursor(editor::CursorDirection::Left);
                },
                MoveRight => {
                    editor.move_cursor(editor::CursorDirection::Right);
                },
                MoveLeftWord => {
                    editor.cursor_to_left_word();
                },
                MoveRightWord => {
                    editor.cursor_to_right_word();
                },
                PageUp => {
                    editor.page_cursor(editor::CursorDirection::Up);
                },
                PageDown => {
                    editor.page_cursor(editor::CursorDirection::Down);
                },
                Save => {
                    let filename = editor.filename.as_ref().unwrap().clone();
                    let status_msg = match editor.save_file() {
                        Ok(_) => format!("Successfully written to {}", filename),
                        Err(_) => "Failed to save file".to_owned(),
                    };
                    editor.display_status(status_msg);
                    last_time_of_status = Instant::now();
                },
                Open => {
                    unimplemented!()
                },
                Find => {
                    unimplemented!()
                },
                Cut => {
                    unimplemented!()
                },
                Copy => {
                    unimplemented!()
                },
                Paste => {
                    editor.insert_str(clipbrd.get());
                },
                GoHome => {
                    editor.cursor_to_start_of_line();
                },
                GoEnd => {
                    editor.cursor_to_end_of_line();
                },
                BackspaceWord => {
                    editor.backspace_word();
                },
                BackspaceLine => {
                    editor.backspace_to_start_of_line();
                },
                Backspace => {
                    editor.backspace();
                },
                Refresh => {
                    let (screen_cols, screen_rows) = terminal_size()
                        .expect("Could not get the terminal size");
                    editor.set_screen_size(screen_rows as usize, screen_cols as usize);
                },
                Quit => break,
                Char('\n') => {
                    editor.newline();
                },
                Char(c) => {
                    editor.insert_char(c);
                },
                _ => {}
            }
        }
        if ShouldResizeWindow.compare_and_swap(true, false, Ordering::Relaxed) {
            let (screen_cols, screen_rows) = terminal_size()
                .expect("Could not get the terminal size");
            editor.set_screen_size(screen_rows as usize, screen_cols as usize);
        }

        thread::sleep(Duration::from_millis(50));
        if Instant::now() - last_time_of_status > status_gap {
            editor.empty_status();
        }
        render!(editor, stdout);
    }

    write!(stdout, "{}{}{}", termion::cursor::Goto(1, 1),
           termion::clear::All, termion::cursor::Show).unwrap();
}
