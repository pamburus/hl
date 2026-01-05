use logos::Logos;

#[inline(always)]
pub fn looks_like_number(value: &[u8]) -> bool {
    if value.is_empty() || !is_number_first_byte(value[0]) {
        return false;
    }

    let mut lexer = Token::lexer(value);
    matches!(lexer.next(), Some(Ok(Token::Number))) && lexer.next().is_none()
}

#[inline(always)]
fn is_number_first_byte(byte: u8) -> bool {
    matches!(byte, b'0'..=b'9' | b'+' | b'-' | b'.')
}

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(utf8 = false)]
pub enum Token {
    #[regex(r"[+-]?(?:\d+\.\d*|\.\d+|\d+)(?:[eE][+-]?\d+)?")]
    Number,
}

#[cfg(test)]
mod tests;
