use std::env;
use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

use crate::error::*;

pub type OutputStream = Box<dyn Write + Send + Sync>;

pub struct Pager {
    process: Child,
}

impl Pager {
    pub fn new() -> Result<Self> {
        let pager = match env::var("PAGER") {
            Ok(pager) => pager,
            _ => "less".into(),
        };

        let pager = shellwords::split(&pager).unwrap_or(vec![pager]);
        let (pager, args) = match pager.split_first() {
            Some((pager, args)) => (pager, args),
            None => (&pager[0], &pager[0..0]),
        };
        let pager = PathBuf::from(pager);
        let mut command = Command::new(&pager);
        for arg in args {
            command.arg(arg);
        }
        if pager.file_stem() == Some(&OsString::from("less")) {
            command.arg("-R");
            command.env("LESSCHARSET", "UTF-8");
        }

        let process = command.stdin(Stdio::piped()).spawn()?;

        Ok(Self { process })
    }
}

impl Drop for Pager {
    fn drop(&mut self) {
        self.process.wait().ok();
    }
}

impl Write for Pager {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.process.stdin.as_mut().unwrap().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.process.stdin.as_mut().unwrap().flush()
    }
}
