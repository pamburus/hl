use super::{json, logfmt, Format, Parse, ParseOutput};
use crate::{error::Error, model::v2::ast};

// ---

pub const MAX_FORMATS: usize = 2;

// ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Json,
    Logfmt,
}
// ---

pub type Choices = heapless::Vec<Choice, MAX_FORMATS>;

// ---

pub struct AutoFormat {
    choices: Choices,
}

impl AutoFormat {
    pub fn new(choices: Choices) -> Self {
        if choices.is_empty() {
            panic!("at least one choice must be provided");
        }

        Self { choices }
    }
}

impl Default for AutoFormat {
    fn default() -> Self {
        Self {
            choices: Choices::from_slice(&[Choice::Json, Choice::Logfmt]).unwrap(),
        }
    }
}

impl super::Format for AutoFormat {
    type Lexer<'a> = Lexer<'a>;
    type Parser<'s> = Parser<'s>;

    fn new_lexer<'a>(&self, input: &'a [u8]) -> super::Result<Self::Lexer<'a>> {
        Ok(Lexer::new(self.choices[0], std::str::from_utf8(input)?))
    }

    fn new_parser_from_lexer<'s>(&self, lexer: Self::Lexer<'s>) -> Self::Parser<'s> {
        Parser::new(self.choices.clone(), lexer)
    }
}

// ---

pub struct Parser<'s> {
    choices: Choices,
    lexer: Lexer<'s>,
}

impl<'s> Parser<'s> {
    pub fn new(choices: Choices, lexer: Lexer<'s>) -> Self {
        Self { choices, lexer }
    }

    fn try_with<T: ast::Build<'s>>(lexer: &mut Lexer<'s>, target: T) -> Result<Option<ParseOutput>, Choice> {
        match lexer {
            Lexer::Json(lexer) => {
                if let Ok(result) = json::JsonFormat.parse_from_lexer(lexer, target) {
                    return Ok(result);
                }
                Err(Choice::Json)
            }
            Lexer::Logfmt(lexer) => {
                if let Ok(result) = logfmt::LogfmtFormat.parse_from_lexer(lexer, target) {
                    return Ok(result);
                }
                Err(Choice::Logfmt)
            }
        }
    }
}

impl<'s> Parse<'s> for Parser<'s> {
    type Lexer = Lexer<'s>;

    fn parse<T: ast::Build<'s>>(&mut self, target: T) -> super::Result<Option<ParseOutput>> {
        let checkpoint = self.lexer.clone();
        let mut end = self.lexer.span().end;

        match Self::try_with(&mut self.lexer, target) {
            Ok(output) => return Ok(output),
            Err(skip) => {
                end = end.max(self.lexer.span().end);
                for &choice in self.choices.iter().filter(|&&c| c != skip) {
                    let mut lexer = checkpoint.clone().morph(choice);
                    if let Ok(output) = Self::try_with(&mut lexer, target) {
                        self.lexer = lexer;
                        return Ok(output);
                    }
                    end = end.max(self.lexer.span().end);
                }
                return Err(Error::FailedToAutoDetectInputFormat {
                    start: checkpoint.span().start,
                    end,
                });
            }
        }
    }

    fn into_lexer(self) -> Self::Lexer {
        self.lexer
    }
}

// ---

#[derive(Clone)]
enum Lexer<'s> {
    Json(json::Lexer<'s>),
    Logfmt(logfmt::Lexer<'s>),
}

impl<'s> Lexer<'s> {
    fn new(choice: Choice, input: &'s str) -> Self {
        match choice {
            Choice::Json => Self::Json(json::Lexer::new(input)),
            Choice::Logfmt => Self::Logfmt(logfmt::Lexer::new(input)),
        }
    }

    fn morph(self, choice: Choice) -> Self {
        match (self, choice) {
            (Self::Json(lexer), Choice::Logfmt) => Self::Logfmt(lexer.morph()),
            (Self::Logfmt(lexer), Choice::Json) => Self::Json(lexer.morph()),
            (Self::Json(lexer), Choice::Json) => Self::Json(lexer),
            (Self::Logfmt(lexer), Choice::Logfmt) => Self::Logfmt(lexer),
        }
    }

    fn span(&self) -> std::ops::Range<usize> {
        match self {
            Self::Json(lexer) => lexer.span(),
            Self::Logfmt(lexer) => lexer.span(),
        }
    }

    fn matches(&self, choice: Choice) -> bool {
        match (self, choice) {
            (Self::Json(_), Choice::Json) => true,
            (Self::Logfmt(_), Choice::Logfmt) => true,
            _ => false,
        }
    }
}
