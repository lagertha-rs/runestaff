use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rnsc", about = "Java assembler and disassembler", version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    /// Suppress AssemblerWarn and JvmSpecWarn diagnostics
    #[arg(short, long)]
    quiet: bool,

    /// Output directory for class files
    #[arg(short = 'd', long)]
    output_dir: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Command {
    Asm {
        file: PathBuf,
        /// Suppress AssemblerWarn and JvmSpecWarn diagnostics
        #[arg(short, long)]
        quiet: bool,
        /// Output directory for class files
        #[arg(short = 'd', long)]
        output_dir: Option<PathBuf>,
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
            quiet,
            output_dir,
        }) => assemble(&file, output_dir, quiet),
        Some(Command::Dis { file }) => disassemble(&file),
        None => {
            if let Some(file) = cli.file {
                assemble(&file, cli.output_dir, cli.quiet);
            } else {
                eprintln!("Usage: rnsc <file.rns> or rnsc asm <file.rns> or rnsc dis <file.class>");
                std::process::exit(1);
            }
        }
    }
}

fn assemble(path: &PathBuf, output_dir: Option<PathBuf>, quiet: bool) {
    let filename = path.to_string_lossy().to_string();
    let contents = std::fs::read_to_string(path).unwrap_or_else(|err| {
        eprintln!("Error reading file {}: {}", filename, err);
        std::process::exit(1);
    });

    let (assembled, diagnostics) = rns::assemble(&contents);

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

    match assembled {
        Some(assembled) => {
            let base_dir = output_dir.unwrap_or_else(|| PathBuf::from("."));

            for (class_name, bytes) in &assembled.classes {
                let output_path = base_dir.join(format!("{}.class", class_name));

                if let Some(parent) = output_path.parent() {
                    std::fs::create_dir_all(parent).unwrap_or_else(|err| {
                        eprintln!("Error creating directory {}: {}", parent.display(), err);
                        std::process::exit(1);
                    });
                }

                std::fs::write(&output_path, bytes).unwrap_or_else(|err| {
                    eprintln!("Error writing file {}: {}", output_path.display(), err);
                    std::process::exit(1);
                });
            }
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
