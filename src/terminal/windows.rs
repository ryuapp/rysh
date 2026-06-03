use crate::terminal::LineRead;
use anyhow::{Result, bail};
use std::ffi::c_void;
use std::io::{self, Write};
use std::ptr;

type Bool = i32;
type Dword = u32;
type Handle = *mut c_void;

const STD_INPUT_HANDLE: Dword = -10i32 as Dword;
const ENABLE_PROCESSED_INPUT: Dword = 0x0001;
const ENABLE_LINE_INPUT: Dword = 0x0002;
const ENABLE_ECHO_INPUT: Dword = 0x0004;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetStdHandle(nStdHandle: Dword) -> Handle;
    fn GetConsoleMode(hConsoleHandle: Handle, lpMode: *mut Dword) -> Bool;
    fn SetConsoleMode(hConsoleHandle: Handle, dwMode: Dword) -> Bool;
    fn ReadConsoleW(
        hConsoleInput: Handle,
        lpBuffer: *mut c_void,
        nNumberOfCharsToRead: Dword,
        lpNumberOfCharsRead: *mut Dword,
        pInputControl: *mut c_void,
    ) -> Bool;
}

pub struct Terminal {
    input: Handle,
    original_mode: Dword,
    line: String,
}

impl Terminal {
    pub fn new() -> Result<Self> {
        let input = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
        if input.is_null() || input as isize == -1 {
            bail!("failed to get console input handle");
        }

        let mut original_mode = 0;
        if unsafe { GetConsoleMode(input, &mut original_mode) } == 0 {
            bail!("failed to read console mode");
        }

        let raw_mode =
            original_mode & !(ENABLE_PROCESSED_INPUT | ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT);
        if unsafe { SetConsoleMode(input, raw_mode) } == 0 {
            bail!("failed to set console mode");
        }

        Ok(Self {
            input,
            original_mode,
            line: String::new(),
        })
    }

    pub fn read_line(&mut self, prompt: &str) -> Result<LineRead> {
        print!("{prompt}");
        io::stdout().flush()?;

        self.line.clear();
        loop {
            let Some(ch) = self.read_char()? else {
                return Ok(LineRead::Eof);
            };

            match ch {
                '\u{3}' => {
                    println!("^C");
                    self.line.clear();
                    return Ok(LineRead::Interrupted);
                }
                '\u{1a}' if self.line.is_empty() => {
                    println!();
                    return Ok(LineRead::Eof);
                }
                '\r' | '\n' => {
                    println!();
                    return Ok(LineRead::Line(std::mem::take(&mut self.line)));
                }
                '\u{8}' => {
                    if self.line.pop().is_some() {
                        print!("\u{8} \u{8}");
                        io::stdout().flush()?;
                    }
                }
                ch if ch.is_control() => {}
                ch => {
                    self.line.push(ch);
                    print!("{ch}");
                    io::stdout().flush()?;
                }
            }
        }
    }

    fn read_char(&self) -> Result<Option<char>> {
        let mut buffer = [0u16; 1];
        let mut read = 0;
        if unsafe {
            ReadConsoleW(
                self.input,
                buffer.as_mut_ptr().cast(),
                1,
                &mut read,
                ptr::null_mut(),
            )
        } == 0
        {
            bail!("failed to read console input");
        }

        if read == 0 {
            return Ok(None);
        }

        Ok(char::from_u32(buffer[0] as u32))
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        unsafe {
            SetConsoleMode(self.input, self.original_mode);
        }
    }
}
