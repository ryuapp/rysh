use crate::terminal::LineRead;
use anyhow::Result;
use std::io::{self, Write};

pub struct Terminal {
    line: String,
}

impl Terminal {
    pub fn new() -> Result<Self> {
        Ok(Self {
            line: String::new(),
        })
    }

    pub fn read_line(&mut self, prompt: &str) -> Result<LineRead> {
        print!("{prompt}");
        io::stdout().flush()?;

        self.line.clear();
        if io::stdin().read_line(&mut self.line)? == 0 {
            println!();
            return Ok(LineRead::Eof);
        }

        Ok(LineRead::Line(
            self.line.trim_end_matches(['\r', '\n']).to_string(),
        ))
    }
}
