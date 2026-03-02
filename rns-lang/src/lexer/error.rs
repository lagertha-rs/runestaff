use crate::diagnostic::{Diagnostic, DiagnosticLabel, DiagnosticTier};
use crate::token::type_hint::TypeHintKind;
use crate::token::{RnsToken, Span};
use std::ops::Range;

//TODO: same error code for all lexer, try to categorize later if needed
const LEXER_ERROR_CODE: &str = "E001";
const IDENTIFIER_HELP_URL: &str =
    "https://rune.lagertha-vm.com/syntax/keywords-and-operands/#identifiers";
const INTEGER_HELP_URL: &str =
    "https://rune.lagertha-vm.com/syntax/keywords-and-operands/#identifiers";
const STRING_HELP_URL: &str =
    "https://rune.lagertha-vm.com/syntax/keywords-and-operands/#identifiers";

#[derive(Debug, Eq, PartialEq, Clone)]
pub(super) enum LexerError {
    UnexpectedChar(Span, char, Option<String>),
    UnknownDirective(Span, String),
    UnexpectedEof(Span),
    UnterminatedString(Span),
    InvalidEscape(Span, char),
    InvalidNumber(Span, String),
    UnexpectedHintOperand {
        hint_position: Span,
        operand_token: RnsToken,
        hint_kind: TypeHintKind,
        operand_order_nbr: usize, // TODO: confusing name, it is like position (first, second, etc)
    },
}

impl LexerError {
    fn get_message(&self) -> String {
        match self {
            LexerError::UnexpectedChar(_, _, _) => "unexpected character".to_string(),
            LexerError::UnknownDirective(_, _) => "unknown directive".to_string(),
            LexerError::UnexpectedEof(_) => "unexpected end of file".to_string(),
            LexerError::UnterminatedString(_) => "unterminated string literal".to_string(),
            LexerError::InvalidEscape(_, _) => "invalid escape sequence".to_string(),
            LexerError::InvalidNumber(_, _) => "invalid integer".to_string(),
            LexerError::UnexpectedHintOperand { hint_kind, .. } => {
                format!("unexpected operand for '{}' type hint", hint_kind)
            }
        }
    }

    fn get_note(&self) -> Option<String> {
        let note = match self {
            LexerError::UnexpectedHintOperand { .. } => format!("note msg"),
            LexerError::UnexpectedEof(_) => format!(
                "Expected one of the directives: {}",
                RnsToken::list_directives()
            ),
            LexerError::UnexpectedChar(_, _, context) => return context.clone(),

            LexerError::UnterminatedString(_) => {
                "String literal is not terminated before the end of the line or file.".to_string()
            }
            LexerError::InvalidEscape(_, c) => {
                format!("The character '\\{}' is not a valid escape sequence.", c)
            }
            LexerError::UnknownDirective(_, _) => {
                format!("Valid directives are {}", RnsToken::list_directives())
            }
            LexerError::InvalidNumber(_, value) => {
                if value.starts_with("0x") || value.starts_with("0X") {
                    "Hexadecimal numbers are not supported yet, but are planned for the future."
                        .to_string()
                } else {
                    "Integers must be between -2147483648 and 2147483647".to_string()
                }
            }
        };
        Some(note)
    }

    // TODO: do better
    fn conjugate_ordinal(n: usize) -> &'static str {
        match n {
            0 => "first",
            1 => "second",
            2 => "third",
            _ => "nth",
        }
    }

    fn get_labels(&self) -> Vec<DiagnosticLabel> {
        match self {
            LexerError::UnexpectedHintOperand {
                hint_position,
                operand_token,
                hint_kind,
                operand_order_nbr: operand_nbr,
            } => {
                let ordinal = Self::conjugate_ordinal(*operand_nbr);
                vec![
                    DiagnosticLabel::context(
                        hint_position.as_range(),
                        format!(
                            "the '{}' {} operand is expected to be {}",
                            hint_kind,
                            ordinal,
                            hint_kind.expected_argument_types()[*operand_nbr]
                        ),
                    ),
                    DiagnosticLabel::at(
                        operand_token.span().as_range(),
                        format!("but {} found instead", operand_token.as_string_token_type()),
                    ),
                ]
            }
            LexerError::UnexpectedChar(_, c, _) => {
                vec![DiagnosticLabel::at(
                    self.get_primary_location(),
                    format!("found '{}' here", c.escape_default()),
                )]
            }
            LexerError::UnknownDirective(_, name) => {
                let mut closest = None;
                let mut min_dist = usize::MAX;
                for directive in RnsToken::DIRECTIVES {
                    let d_str = directive.to_string();
                    let dist = crate::utils::levenshtein_distance(name, &d_str);
                    if dist < min_dist && dist <= 2 {
                        min_dist = dist;
                        closest = Some(d_str);
                    }
                }

                let msg = if let Some(suggestion) = closest {
                    format!("did you mean '{}' ?", suggestion)
                } else {
                    "unknown directive".to_string()
                };
                vec![DiagnosticLabel::at(self.get_primary_location(), msg)]
            }
            LexerError::UnexpectedEof(_) => vec![DiagnosticLabel::at(
                self.get_primary_location(),
                "unexpected end of file".to_string(),
            )],
            LexerError::UnterminatedString(_) => {
                vec![DiagnosticLabel::at(
                    self.get_primary_location(),
                    "this string literal is not terminated".to_string(),
                )]
            }
            LexerError::InvalidEscape(_, c) => vec![DiagnosticLabel::at(
                self.get_primary_location(),
                format!("invalid escape sequence '\\{}'", c),
            )],
            LexerError::InvalidNumber(_, value) => {
                let msg = if value.parse::<i128>().is_ok() {
                    format!(
                        "integer '{}' is too large for a 32-bit signed integer",
                        value
                    )
                } else if value.chars().any(|c| !c.is_digit(10) && c != '-') {
                    format!("'{}' contains invalid characters", value)
                } else {
                    format!("'{}' is not a valid integer", value)
                };
                vec![DiagnosticLabel::at(self.get_primary_location(), msg)]
            }
        }
    }

    fn get_primary_location(&self) -> Range<usize> {
        match self {
            LexerError::UnexpectedChar(span, _, _)
            | LexerError::UnknownDirective(span, _)
            | LexerError::UnexpectedEof(span)
            | LexerError::UnterminatedString(span)
            | LexerError::InvalidEscape(span, _)
            | LexerError::InvalidNumber(span, _) => span.as_range(),
            LexerError::UnexpectedHintOperand { hint_position, .. } => hint_position.as_range(),
        }
    }
}

impl From<LexerError> for Diagnostic {
    fn from(value: LexerError) -> Self {
        Diagnostic {
            message: value.get_message(),
            code: LEXER_ERROR_CODE,
            primary_location: value.get_primary_location(),
            note: value.get_note(),
            help: None,
            tier: DiagnosticTier::SyntaxError,
            labels: value.get_labels(),
        }
    }
}
