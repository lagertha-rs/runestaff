use crate::lexer::{JasmLexer, LexerError};
use crate::token::JasmTokenKind;
use ariadne::{Color, Label, Report, ReportKind, Source};
use itertools::Itertools;

mod lexer;
mod token;

fn print_comprehensive_error(filename: &str, source_code: &str, err: &LexerError) {
    match err {
        LexerError::UnknownDirective(span, name) => {
            Report::build(ReportKind::Error, (filename, span.start..span.end))
                .with_message(err.message())
                .with_label(
                    Label::new((filename, span.as_range()))
                        .with_message(format!("The directive '{}' is not recognized", name))
                        .with_color(Color::Red),
                )
                .with_note(format!(
                    "Valid directives are {}",
                    JasmTokenKind::DIRECTIVES
                        .iter()
                        .map(ToString::to_string)
                        .join(", ")
                ))
                .finish()
                .eprint((filename, Source::from(source_code)))
                .unwrap();
        }
        _ => unimplemented!(),
    }
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
            print_comprehensive_error(&filename, &contents, &err);
            std::process::exit(1);
        }
    };

    println!("Tokens: {:?}", tokens);
}
