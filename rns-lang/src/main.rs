use crate::lexer::{JasmLexer, LexerError};

mod cursor;
mod lexer;
mod token;

// TODO: put in separate module and make reusable
fn print_comprehensive_error(filename: &str, source: &str, err: &LexerError) {
    eprintln!("lexer error: {}", err.name());
    match err {
        LexerError::UnknownDirective(span, name) => {
            let line_content = source
                .lines()
                .nth(span.line - 1)
                .unwrap_or("<unable to fetch line>");
            let line_number_space = " ".repeat(span.line.to_string().len());
            eprintln!("{line_number_space} --> <{filename}>");
            eprintln!("{line_number_space} |");
            eprintln!("{} | {}", span.line, line_content);
            eprintln!(
                "{line_number_space} | {}{}",
                " ".repeat(span.start.checked_div(1).unwrap_or(0)),
                "^".repeat(name.len())
            );
            eprintln!("{line_number_space} = unknown directive: {}", name);
        }
        _ => todo!(), // Handle other error variants here
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
