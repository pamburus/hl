// workspace imports
use upstream::{Lex, Source, Span, Token};

// local imports
use crate::{EnabledFormat, EnabledFormatList, Error, ErrorKind};

// ---

#[derive(Clone, Debug)]
pub struct Lexer<'s> {
    input: &'s Source,
    enabled: EnabledFormatList,
    current: usize,
    candidate: Option<usize>,
    inner: LexerInner<'s>,
    checkpoint: usize,
    failing: bool,
}

impl<'s> Lexer<'s> {
    #[inline]
    pub(crate) fn new(input: &'s Source, enabled: EnabledFormatList) -> Self {
        // SAFETY: `enabled` is guaranteed to be non-empty by the `AutoFormat` constructor.
        let format = unsafe { *enabled.get_unchecked(0) };

        Self {
            input,
            enabled,
            current: 0,
            candidate: None,
            inner: LexerInner::new(input, format),
            checkpoint: 0,
            failing: false,
        }
    }

    #[cold]
    fn rotate(&mut self) -> bool {
        if self.failing || self.enabled.len() == 1 {
            return false;
        }

        let mut candidate = self.candidate.unwrap_or(0) + 1;
        if candidate == self.current {
            candidate += 1;
        }
        if candidate == self.enabled.len() {
            self.candidate = None;
            self.current = 0;
            self.failing = true;
            self.select(self.current);
            return false;
        }
        self.select(candidate);
        self.candidate = Some(candidate);

        true
    }

    fn select(&mut self, index: usize) {
        self.inner = LexerInner::new(self.input, self.enabled[index]);
        self.inner.bump(self.checkpoint);
    }
}

impl<'s> Lex<'s> for Lexer<'s> {
    type Error = Error;

    #[inline]
    fn span(&self) -> Span {
        match &self.inner {
            #[cfg(feature = "json")]
            LexerInner::Json(lexer) => lexer.span(),

            #[cfg(feature = "logfmt")]
            LexerInner::Logfmt(lexer) => lexer.span(),
        }
    }

    #[inline]
    fn bump(&mut self, n: usize) {
        match &mut self.inner {
            #[cfg(feature = "json")]
            LexerInner::Json(lexer) => lexer.bump(n),

            #[cfg(feature = "logfmt")]
            LexerInner::Logfmt(lexer) => lexer.bump(n),
        }
    }
}

impl<'s> Iterator for Lexer<'s> {
    type Item = Result<Token<'s>, Error>;

    #[inline]
    fn next(&mut self) -> Option<Result<Token<'s>, Error>> {
        loop {
            let next: Result<_, Error> = match &mut self.inner {
                #[cfg(feature = "json")]
                LexerInner::Json(lexer) => lexer.next()?.map_err(|e| e.into()),

                #[cfg(feature = "logfmt")]
                LexerInner::Logfmt(lexer) => lexer.next()?.map_err(|e| e.into()),
            };

            match next {
                Ok(Token::EntryBegin) => {
                    self.checkpoint = self.inner.span().start;
                    return Some(Ok(Token::EntryBegin));
                }
                Ok(Token::EntryEnd) => {
                    self.failing = false;
                    if let Some(candidate) = self.candidate {
                        self.current = candidate;
                    }
                    return Some(Ok(Token::EntryEnd));
                }
                Ok(token) => return Some(Ok(token)),
                Err(e) => {
                    if !self.rotate() {
                        if self.enabled.len() == 1 {
                            return Some(Err(e));
                        }
                        return Some(Err(Error {
                            kind: ErrorKind::CannotDetermineFormat(self.enabled.clone()),
                            span: e.span,
                        }));
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum LexerInner<'s> {
    #[cfg(feature = "json")]
    Json(log_format_json::Lexer<'s>),

    #[cfg(feature = "logfmt")]
    Logfmt(log_format_logfmt::Lexer<'s>),
}

impl<'s> LexerInner<'s> {
    #[inline]
    fn new(input: &'s Source, format: EnabledFormat) -> Self {
        match format {
            #[cfg(feature = "json")]
            EnabledFormat::Json => Self::Json(log_format_json::Lexer::from_source(input)),

            #[cfg(feature = "logfmt")]
            EnabledFormat::Logfmt => Self::Logfmt(log_format_logfmt::Lexer::from_source(input)),
        }
    }

    #[inline]
    fn span(&self) -> Span {
        match self {
            #[cfg(feature = "json")]
            Self::Json(lexer) => lexer.span(),

            #[cfg(feature = "logfmt")]
            Self::Logfmt(lexer) => lexer.span(),
        }
    }

    #[inline]
    fn bump(&mut self, n: usize) {
        match self {
            #[cfg(feature = "json")]
            Self::Json(lexer) => lexer.bump(n),

            #[cfg(feature = "logfmt")]
            Self::Logfmt(lexer) => lexer.bump(n),
        }
    }
}
