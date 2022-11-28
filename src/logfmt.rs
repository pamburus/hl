#[derive(PartialEq, Debug)]

pub struct Pair {
    pub key: String,
    pub val: Option<String>,
}

fn complete_pair(buf: String, pair: Option<Pair>) -> Pair {
    match pair {
        Some(Pair { key: k, val: _ }) => Pair {
            key: k,
            val: Some(buf),
        },
        None => Pair {
            key: buf,
            val: None,
        },
    }
}

pub fn parse(message: &str) -> Vec<Pair> {
    let mut pair: Option<Pair> = None;
    let mut pairs: Vec<Pair> = vec![];
    let mut buf = String::new();

    let mut escape = false;
    let mut garbage = false;
    let mut quoted = false;

    for c in message.chars() {
        match (quoted, c) {
            (false, ' ') => {
                if !buf.is_empty() {
                    if !garbage {
                        // the buffer that we just processed is either a value
                        // or a valueless key depending on the current state of
                        // `pair`
                        pairs.push(complete_pair(buf, pair));
                        pair = None;
                    }
                    buf = String::new();
                }
                garbage = false;
            }
            (false, '=') => {
                if !buf.is_empty() {
                    pair = Some(Pair {
                        key: buf,
                        val: None,
                    });
                    buf = String::new();
                } else {
                    garbage = true;
                }
            }
            (true, '\\') => {
                escape = true;
            }
            (_, '"') => {
                if escape {
                    buf.push(c);
                    escape = false;
                } else {
                    quoted = !quoted;
                }
            }
            _ => {
                // if the last character we read was an escape, but this
                // character was not a quote, then store the escape back into the
                // buffer
                if escape {
                    buf.push('\\');
                    escape = false;
                }
                buf.push(c);
            }
        }
    }

    // and process one final time at the end of the message to get the last
    // data point
    if !garbage {
        pairs.push(complete_pair(buf, pair));
    }

    pairs
}
