use logos::Logos;

pub fn looks_like_number(value: &[u8]) -> bool {
    if value.is_empty() {
        return false;
    }

    let mut lexer = Token::lexer(value);
    matches!(lexer.next(), Some(Ok(Token::Number))) && lexer.next().is_none()
}

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(source = [u8])]
pub enum Token {
    #[regex(r"[+-]?(?:\d+\.\d*|\.\d+|\d+)(?:[eE][+-]?\d+)?")]
    Number,
}

#[cfg(test)]
mod tests;
