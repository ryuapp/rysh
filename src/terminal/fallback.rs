use crate::terminal::LineRead;
use anyhow::Result;
use std::io::{self, ErrorKind, Write};

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
        match io::stdin().read_line(&mut self.line) {
            Ok(0) => {
                println!();
                return Ok(LineRead::Eof);
            }
            Ok(_) => {}
            Err(err) if err.kind() == ErrorKind::Interrupted => {
                println!("^C");
                return Ok(LineRead::Interrupted);
            }
            Err(err) => return Err(err.into()),
        }

        Ok(LineRead::Line(
            self.line.trim_end_matches(['\r', '\n']).to_string(),
        ))
    }
}
