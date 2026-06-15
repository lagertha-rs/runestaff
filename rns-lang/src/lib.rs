mod assembler;
pub mod ast;
pub mod diagnostic;
mod disassembler;
mod instruction;
mod lexer;
mod parser;
pub(crate) mod suggestion;
pub(crate) mod token;

use crate::diagnostic::{Diagnostic, DiagnosticTier};

pub use disassembler::{DisasmError, disassemble_bytes};

pub const ERROR_DOCS_BASE_URL: &str = "https://rune.lagertha-vm.com/errors/";

pub fn assemble(source: &str) -> (Option<Vec<u8>>, Vec<Diagnostic>) {
    let mut all_diagnostics = Vec::new();
    let mut has_error = false;

    let (tokens, lexer_diags, eof_span) = lexer::tokenize(source);
    for diag in lexer_diags {
        if diag.tier == DiagnosticTier::SyntaxError {
            has_error = true;
        }
        all_diagnostics.push(diag);
    }
    if has_error {
        return (None, all_diagnostics);
    }

    let module = match parser::parse(tokens, eof_span) {
        Ok(mut module) => {
            let module_diags = std::mem::take(&mut module.diagnostics);
            for diag in module_diags {
                if diag.tier == DiagnosticTier::SyntaxError {
                    has_error = true;
                }
                all_diagnostics.push(diag);
            }
            module
        }
        Err(errors) => {
            for err in errors {
                all_diagnostics.push(err);
            }
            return (None, all_diagnostics);
        }
    };

    if has_error {
        return (None, all_diagnostics);
    }

    let (bytes, asm_diags) = module.into_bytes();
    for diag in asm_diags {
        if diag.tier == DiagnosticTier::SyntaxError {
            has_error = true;
        }
        all_diagnostics.push(diag);
    }

    if has_error {
        return (None, all_diagnostics);
    }

    (bytes, all_diagnostics)
}

pub fn lex(source: &str) -> Vec<Diagnostic> {
    let (_, diagnostics, _) = lexer::tokenize(source);
    diagnostics
}
