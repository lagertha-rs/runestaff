use crate::diagnostic::DiagnosticTier;
use crate::lexer::RnsLexer;
use crate::parser::RnsParser;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod assembler;
mod diagnostic;
mod instruction;
mod lexer;
mod parser;
mod token;
mod utils;

#[derive(Parser)]
#[command(name = "rns", about = "Java assembler and disassembler")]
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
                eprintln!("Usage: rns <file.ja> or rns asm <file.ja> or rns dis <file.class>");
                std::process::exit(1);
            }
        }
    }
}

fn assemble(path: &PathBuf, output: Option<&PathBuf>, warn_asm: bool, warn_error: bool) {
    let filename = path.to_string_lossy().to_string();
    let contents = std::fs::read_to_string(path).unwrap_or_else(|err| {
        eprintln!("Error reading file {}: {}", filename, err);
        std::process::exit(1);
    });

    let mut lexer = RnsLexer::new(&contents);

    let tokens = match lexer.tokenize() {
        Ok(tokens) => tokens,
        Err(err) => {
            err.print(&filename, &contents);
            std::process::exit(1);
        }
    };

    let rns_module = match RnsParser::parse(tokens) {
        Ok(module) => module,
        Err(errors) => {
            for err in errors {
                err.print(&filename, &contents);
            }
            std::process::exit(1);
        }
    };

    let (class, diagnostics) = rns_module.into_class_file();

    let mut has_error = false;
    for diag in diagnostics {
        match (diag.tier, warn_asm, warn_error) {
            (DiagnosticTier::SyntaxError, _, _) => has_error = true,
            (DiagnosticTier::JvmSpecWarn, true, _) => has_error = true,
            (DiagnosticTier::AssemblerWarn, _, true) => has_error = true,
            _ => {}
        }
        diag.print(&filename, &contents);
    }

    if has_error {
        std::process::exit(1);
    }

    let bytes = class.to_bytes();
    let output_path = output
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| filename.replace(".ja", ".class"));
    std::fs::write(output_path, bytes).expect("Failed to write output file");
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

    let ja_text = class_file.fmt_rns();
    print!("{}", ja_text.unwrap());
}
