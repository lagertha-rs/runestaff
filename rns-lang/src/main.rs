use crate::error::{JasmDiagnostic, JasmError};
use crate::lexer::JasmLexer;
use crate::warning::JasmWarning;
use ariadne::{Color, Label, Report, ReportKind, Source};

mod ast;
mod error;
mod instruction;
mod lexer;
mod parser;
mod token;
mod utils;
mod warning;

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
            err.print(&filename, &contents);
            std::process::exit(1);
        }
    };

    let ast = match parser::JasmParser::parse(tokens) {
        Ok(warnings) => print_test_warning(&filename, &contents, warnings),
        Err(err) => {
            err.print(&filename, &contents);
            std::process::exit(1);
        }
    };
}

fn print_test_warning(filename: &str, source_code: &str, warnings: Vec<JasmWarning>) {
    for warning in warnings {
        let range = warning.primary_location().clone();
        let mut report = Report::build(ReportKind::Warning, (filename, range.clone()))
            .with_message(warning.message());

        for (label_range, label_msg) in warning.labels() {
            report = report.with_label(
                Label::new((filename, label_range.clone()))
                    .with_message(label_msg)
                    .with_color(Color::Yellow),
            );
        }

        report
            .with_note(warning.note())
            .finish()
            .eprint((filename, Source::from(source_code)))
            .unwrap();
    }
}
