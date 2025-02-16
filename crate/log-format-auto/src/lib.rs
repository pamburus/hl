// workspace imports
use upstream::{ast, Format, Span};

// conditional imports
#[cfg(feature = "json")]
use log_format_json::JsonFormat;
#[cfg(feature = "logfmt")]
use log_format_logfmt::LogfmtFormat;

pub mod error;
pub mod format;
pub mod lexer;

pub use error::{Error, ErrorKind};
pub use format::*;
pub use lexer::*;

// ---

pub struct AutoFormat {
    enabled: EnabledFormatList,
    current: usize,
}

impl AutoFormat {
    pub fn new<E: IntoEnabledFormatList>(enabled: E) -> Self {
        let enabled = enabled.into_enabled_format_list();
        if enabled.is_empty() {
            panic!("at least one format must be enabled");
        }

        Self { enabled, current: 0 }
    }
}

impl Default for AutoFormat {
    fn default() -> Self {
        let mut enabled = Vec::new();

        #[cfg(feature = "json")]
        enabled.push(EnabledFormat::Json);

        #[cfg(feature = "logfmt")]
        enabled.push(EnabledFormat::Logfmt);

        Self::new(enabled)
    }
}

impl Format for AutoFormat {
    type Error = Error;
    type Lexer<'s> = Lexer<'s>;

    #[inline]
    fn lexer<'s>(&self, input: &'s [u8]) -> Lexer<'s> {
        Lexer::new(input, self.enabled.clone())
    }

    fn parse<'s, B>(&mut self, s: &'s [u8], mut target: B) -> Result<(Option<Span>, B), (Self::Error, B)>
    where
        B: ast::Build,
    {
        let initial = self.current;
        let checkpoint = target.checkpoint();

        loop {
            let result = match self.enabled[self.current] {
                #[cfg(feature = "json")]
                EnabledFormat::Json => JsonFormat.parse(s, target).map_err(|(e, target)| (e.into(), target)),
                #[cfg(feature = "logfmt")]
                EnabledFormat::Logfmt => LogfmtFormat.parse(s, target).map_err(|(e, target)| (e.into(), target)),
            };

            match result {
                Ok(output) => {
                    return Ok(output);
                }
                Err((e, t)) => {
                    target = t;

                    if self.enabled.len() == 1 {
                        return Err((e, target));
                    }

                    if self.current == initial {
                        self.current = 0;
                    }

                    self.current += 1;
                    if self.current == initial {
                        self.current += 1;
                    }

                    target.rollback(&checkpoint);

                    if self.current == self.enabled.len() {
                        self.current = 0;
                        return Err((
                            Error {
                                kind: ErrorKind::CannotDetermineFormat(self.enabled.clone()),
                                span: e.span,
                            },
                            target,
                        ));
                    }
                }
            }
        }
    }
}
