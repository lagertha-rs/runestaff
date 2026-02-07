use crate::lexer::JasmLexer;
use ariadne::{Color, Label, Report, ReportKind, Source};

mod ast;
mod error;
mod lexer;
mod parser;
mod token;

use crate::error::JasmError;

fn print_comprehensive_error(filename: &str, source_code: &str, err: JasmError) {
    let range = err.range().clone();
    Report::build(ReportKind::Error, (filename, range.clone()))
        .with_message(err.message())
        .with_note(err.note())
        .with_label(
            Label::new((filename, range))
                .with_message(err.label())
                .with_color(Color::Red),
        )
        .finish()
        .eprint((filename, Source::from(source_code)))
        .unwrap();
}

fn get_filename_and_contents_from_arg() -> (String, String) {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: jasm <filename>");
        std::process::exit(1);
    }
    let filename = &args[1];
    let content = std::fs::read_to_string(filename).unwrap_or_else(|err| {
        eprintln!("Error reading file {}: {}", filename, err);
        std::process::exit(1);
    });
    (filename.clone(), content)
}

fn main() {
    let (filename, contents) = get_filename_and_contents_from_arg();
    let mut lexer = JasmLexer::new(&contents);

    let tokens = match lexer.tokenize() {
        Ok(tokens) => tokens,
        Err(err) => {
            print_comprehensive_error(&filename, &contents, err.into());
            std::process::exit(1);
        }
    };

    tokens.iter().for_each(|v| println!("{v:?}"));
}
