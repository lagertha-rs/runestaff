use crate::error::JasmError;
use crate::lexer::JasmLexer;

mod ast;
mod error;
mod lexer;
mod parser;
mod token;

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

    let tokens = match lexer.tokenize().map_err(JasmError::from) {
        Ok(tokens) => tokens,
        Err(err) => {
            err.print(&filename, &contents);
            std::process::exit(1);
        }
    };

    let ast = match parser::JasmParser::parse(tokens).map_err(JasmError::from) {
        Ok(ast) => ast,
        Err(err) => {
            err.print(&filename, &contents);
            std::process::exit(1);
        }
    };
}
