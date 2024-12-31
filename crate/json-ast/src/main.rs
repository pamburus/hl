// std imports
use std::{env, fs, process::ExitCode};

// external imports
use ariadne;
use logos::Logos;

// ---

use json_ast::{container::Container, token::Token};

fn main() -> ExitCode {
    let filename = env::args().nth(1).expect("Expected file argument");
    let src = fs::read_to_string(&filename).expect("Failed to read file");

    let mut lexer = Token::lexer(src.as_str());

    match Container::parse(&mut lexer) {
        Ok(value) => {
            println!("len: {}", value.nodes().len());
            println!("{:#?}", value);
        }
        Err((msg, span)) => {
            use ariadne::{ColorGenerator, Label, Report, ReportKind, Source};

            let mut colors = ColorGenerator::new();

            let a = colors.next();

            Report::build(ReportKind::Error, (&filename, 12..12))
                .with_message("Invalid JSON".to_string())
                .with_label(Label::new((&filename, span)).with_message(msg).with_color(a))
                .finish()
                .eprint((&filename, Source::from(&src)))
                .unwrap();

            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}