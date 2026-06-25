use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rnsc", about = "Java assembler and disassembler")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    /// Suppress AssemblerWarn and JvmSpecWarn diagnostics
    #[arg(short, long)]
    quiet: bool,
}

#[derive(Subcommand)]
enum Command {
    Asm {
        file: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Suppress AssemblerWarn and JvmSpecWarn diagnostics
        #[arg(short, long)]
        quiet: bool,
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
            quiet,
        }) => assemble(&file, output.as_ref(), quiet),
        Some(Command::Dis { file }) => disassemble(&file),
        None => {
            if let Some(file) = cli.file {
                assemble(&file, None, cli.quiet);
            } else {
                eprintln!("Usage: rnsc <file.rns> or rnsc asm <file.rns> or rnsc dis <file.class>");
                std::process::exit(1);
            }
        }
    }
}

fn assemble(path: &PathBuf, output: Option<&PathBuf>, quiet: bool) {
    let filename = path.to_string_lossy().to_string();
    let contents = std::fs::read_to_string(path).unwrap_or_else(|err| {
        eprintln!("Error reading file {}: {}", filename, err);
        std::process::exit(1);
    });

    let (bytes, diagnostics) = rns::assemble(&contents);

    for diag in diagnostics {
        if quiet
            && matches!(
                diag.tier,
                rns::diagnostic::DiagnosticTier::AssemblerWarn
                    | rns::diagnostic::DiagnosticTier::JvmSpecWarn
            )
        {
            continue;
        }
        diag.print(&filename, &contents);
    }

    match bytes {
        Some(bytes) => {
            let output_path = output
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| filename.replace(".rns", ".class"));
            std::fs::write(output_path, bytes).expect("Failed to write output file");
        }
        None => {
            std::process::exit(1);
        }
    }
}

fn disassemble(path: &PathBuf) {
    let bytes = std::fs::read(path).unwrap_or_else(|err| {
        eprintln!("Error reading file {}: {}", path.display(), err);
        std::process::exit(1);
    });

    let rns_text = rns::disassemble_bytes(bytes).unwrap_or_else(|err| {
        eprintln!("Error disassembling class file {}: {}", path.display(), err);
        std::process::exit(1);
    });
    print!("{}", rns_text);
}
