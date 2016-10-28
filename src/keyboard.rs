use std::io;
use std::io::Read;

pub enum Command {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    MoveLeftWord,
    MoveRightWord,
    PageUp,
    PageDown,
    Save,
    Open,
    Find,
    Cut,
    Copy,
    Paste,
    GoHome,
    GoEnd,
    Backspace,
    BackspaceWord,
    BackspaceLine,
    Delete,
    Refresh,
    Quit,
    Char(char),
    Ignore,
}

pub struct CommandReader<R> {
    input: R,
}

impl <R: Read> CommandReader<R> {
    pub fn commands(reader: R) -> Self {
        CommandReader {
            input: reader,
        }
    }
}

impl <R: Read> Iterator for CommandReader<R> {
    type Item = Command;

    fn next(&mut self) -> Option<Self::Item> {
        let mut single_char = [0u8; 1];
        let mut seq = [0u8; 3];
        let nread = self.input.read(&mut single_char)
            .expect("failed to read from stdin");
        if nread == 0 {
            return Some(Command::Ignore)
        }

        loop {
            match single_char[0] {
                0x1B => {
                    // Escape
                    loop {
                        let nread = self.input.read(&mut seq[0..2])
                            .expect("failed to read from stdin");
                        if nread > 0 {
                            break;
                        }
                    }
                    match seq[0] {
                        0x5B => {
                            // Handle ESC [ sequences
                            if seq[1] >= 0x30 && seq[1] <= 0x39 {
                                // Extended escape, we need one more char
                                loop {
                                    let nread = self.input.read(&mut seq[2..3])
                                        .expect("failed to read from stdin");
                                    if nread > 0 {
                                        break;
                                    }
                                }
                                if seq[2] == 0x7E {
                                    match seq[1] {
                                        0x33 => return Some(Command::Backspace),
                                        0x35 => return Some(Command::PageUp),
                                        0x36 => return Some(Command::PageDown),
                                        c => {
                                            debug!("ignoring sequence ^[[{}~", c);
                                            return Some(Command::Ignore)
                                        }
                                    }
                                }
                            } else {
                                match seq[1] {
                                    0x41 => return Some(Command::MoveUp),
                                    0x42 => return Some(Command::MoveDown),
                                    0x43 => return Some(Command::MoveRight),
                                    0x44 => return Some(Command::MoveLeft),
                                    0x48 => return Some(Command::GoHome),
                                    0x46 => return Some(Command::GoEnd),
                                    c => {
                                        debug!("ignoring sequence ^[[{}", c);
                                        return Some(Command::Ignore)
                                    }
                                }
                            }
                        },
                        0x4F => {
                            // Handle ESC O sequences
                            match seq[1] {
                                0x48 => return Some(Command::GoHome),
                                0x46 => return Some(Command::GoEnd),
                                c => {
                                    debug!("ignoring sequence ^[O{}", c);
                                    return Some(Command::Ignore)
                                }
                            }
                        },
                        0x35 => {
                            // Handle ESC 5 sequences
                            match seq[1] {
                                0x44 => return Some(Command::MoveLeftWord),
                                0x43 => return Some(Command::MoveRightWord),
                                c => {
                                    debug!("ignoring sequence ^[5{}", c);
                                    return Some(Command::Ignore)
                                }
                            }
                        },
                        c => {
                            debug!("ignoring sequence starting with ^[{}", c);
                            return Some(Command::Ignore)
                        }
                    }
                },
                0x7F => {
                    return Some(Command::Backspace)
                },
                0x03 => {
                    return Some(Command::Copy)
                },
                0x16 => {
                    return Some(Command::Paste)
                },
                0x18 => {
                    return Some(Command::Cut)
                },
                0x11 => {
                    return Some(Command::Quit)
                },
                0x0C => {
                    return Some(Command::Refresh)
                },
                0x13 => {
                    return Some(Command::Save)
                },
                0x0F => {
                    return Some(Command::Open)
                },
                0x0D => {
                    return Some(Command::Char('\n'))
                },
                0x17 | 0x08 => {
                    return Some(Command::BackspaceWord)
                },
                0x15 => {
                    return Some(Command::BackspaceLine)
                },
                0x01 => {
                    return Some(Command::GoHome)
                },
                0x05 => {
                    return Some(Command::GoEnd)
                },
                ch => {
                    return Some(Command::Char(ch as char))
                }
            }
        }
    }
}
