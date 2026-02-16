use crate::diagnostic::JasmError;
use crate::lexer::JasmLexer;

mod diagnostic;
mod instruction;
mod lexer;
mod parser;
mod token;
mod utils;

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

    let (warnings, result) = parser::JasmParser::parse(tokens);

    for warning in &warnings {
        warning.print(&filename, &contents);
    }

    let class = match result {
        Ok(class) => class,
        Err(err) => {
            err.print(&filename, &contents);
            std::process::exit(1);
        }
    };

    let bytes = class.to_bytes();
    if let Err(err) = write_to_file(&filename.replace(".ja", ".class"), &bytes) {
        err.print(&filename, &contents);
        std::process::exit(1);
    }
}

// TODO: make proper fn, probably validate .ja name and class name in .class dir
fn write_to_file(filename: &str, bytes: &[u8]) -> Result<(), JasmError> {
    std::fs::write(filename, bytes).map_err(|err| {
        JasmError::Internal(format!("Failed to write to file {}: {}", filename, err))
    })
}
