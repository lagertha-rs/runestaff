use crate::diagnostic::JasmError;
use crate::lexer::JasmLexer;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod diagnostic;
mod instruction;
mod lexer;
mod parser;
mod token;
mod utils;

#[derive(Parser)]
#[command(name = "jasm", about = "Java assembler and disassembler")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Command {
    Asm {
        file: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        #[allow(non_snake_case)]
        Wasm: bool,
        #[arg(long)]
        #[allow(non_snake_case)]
        Werror: bool,
    },
    Dis {
        file: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Asm {
            file,
            output,
            Wasm,
            Werror,
        }) => assemble(&file, output.as_ref(), Wasm, Werror),
        Some(Command::Dis { file }) => disassemble(&file),
        None => {
            if let Some(file) = cli.file {
                assemble(&file, None, false, false);
            } else {
                eprintln!("Usage: jasm <file.ja> or jasm asm <file.ja> or jasm dis <file.class>");
                std::process::exit(1);
            }
        }
    }
}

fn assemble(path: &PathBuf, output: Option<&PathBuf>, _warn_asm: bool, _warn_error: bool) {
    let filename = path.to_string_lossy().to_string();
    let contents = std::fs::read_to_string(path).unwrap_or_else(|err| {
        eprintln!("Error reading file {}: {}", filename, err);
        std::process::exit(1);
    });

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
    let output_path = output
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| filename.replace(".ja", ".class"));
    if let Err(err) = write_to_file(&output_path, &bytes) {
        err.print(&filename, &contents);
        std::process::exit(1);
    }
}

fn disassemble(path: &PathBuf) {
    let bytes = std::fs::read(path).unwrap_or_else(|err| {
        eprintln!("Error reading file {}: {}", path.display(), err);
        std::process::exit(1);
    });

    let class_file = jclass::ClassFile::try_from(bytes).unwrap_or_else(|err| {
        eprintln!("Error parsing class file {}: {}", path.display(), err);
        std::process::exit(1);
    });

    let ja_text = class_file.fmt_jasm();
    print!("{}", ja_text.unwrap());
}

// TODO: make proper fn, probably validate .ja name and class name in .class dir
fn write_to_file(filename: &str, bytes: &[u8]) -> Result<(), JasmError> {
    std::fs::write(filename, bytes).map_err(|err| {
        JasmError::Internal(format!("Failed to write to file {}: {}", filename, err))
    })
}
