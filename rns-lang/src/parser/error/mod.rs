mod context;
mod rejection;

pub(super) use context::{OperandErrPosContext, TrailingTokensErrContext};
pub(super) use rejection::{FloatRejection, SignedIntRejection};

use crate::ERROR_DOCS_BASE_URL;
use crate::diagnostic::{Diagnostic, DiagnosticLabel, DiagnosticTier};
use crate::token::Spanned;
use crate::token::type_hint::{TypeHint, TypeHintKind, TypeHintOperandName};
use crate::token::{RnsToken, Span};
use std::vec;

#[derive(Debug, PartialEq, Clone)]
pub(super) enum ParserError {
    EmptyFile(Span),
    // TODO: the messages are total shit
    UnexpectedTokenInClassBody(RnsToken),
    // TODO: the messages are total shit
    UnexpectedTokenBeforeClassDefinition(RnsToken),
    TrailingTokens(usize, Vec<RnsToken>, TrailingTokensErrContext),
    IdentifierOrHintExpected(Span, RnsToken, OperandErrPosContext),
    MissingTypeHintOperand {
        type_hint: Spanned<TypeHintKind>,
        operand: TypeHintOperandName,
        after_span: Span,
    },
    // TODO: the messages are total shit
    MultipleSuperDefinitions(Vec<(Span, TypeHint)>),
    TypeHintExpectsIntegerOperand {
        type_hint: Spanned<TypeHintKind>,
        rejection: SignedIntRejection,
    },
    TypeHintExpectsFloatOperand {
        type_hint: Spanned<TypeHintKind>,
        rejection: FloatRejection,
    },
}

impl ParserError {
    // TODO: put all codes as consts somewhere, and make sure they are unique across the whole codebase
    fn code(&self) -> &'static str {
        match self {
            ParserError::TypeHintExpectsIntegerOperand { .. } => "E-003",
            ParserError::TypeHintExpectsFloatOperand { .. } => "E-003",
            ParserError::EmptyFile(_) => "E-006",
            ParserError::UnexpectedTokenInClassBody(_) => "E-007",
            ParserError::UnexpectedTokenBeforeClassDefinition(_) => "E-008",
            ParserError::IdentifierOrHintExpected(_, _, _) => "E-009",
            ParserError::TrailingTokens(_, _, ctx) => match ctx {
                TrailingTokensErrContext::Class => "E-010",
                TrailingTokensErrContext::Super => "E-012",
                TrailingTokensErrContext::TypeHint(_) => "E-013",
            },
            ParserError::MultipleSuperDefinitions(_) => "E-011",
            ParserError::MissingTypeHintOperand { .. } => "E-014",
        }
    }

    fn asm_msg(&self) -> String {
        match self {
            ParserError::EmptyFile(_) => "file contains no class definition".to_string(),
            ParserError::UnexpectedTokenInClassBody(token) => {
                format!("unexpected token in class body: '{token:?}'")
            }
            ParserError::UnexpectedTokenBeforeClassDefinition(unexpected) => {
                format!(
                    "unexpected {} before class definition",
                    unexpected.token_type()
                )
            }
            ParserError::IdentifierOrHintExpected(_, token, ctx) => {
                format!(
                    "unexpected {} where a {} is required",
                    token.token_type(),
                    ctx
                )
            }
            ParserError::TrailingTokens(_, _, ctx) => {
                format!("unexpected trailing tokens after {}", ctx)
            }
            ParserError::MultipleSuperDefinitions(_) => "multiple .super directives".to_string(),
            ParserError::TypeHintExpectsIntegerOperand {
                type_hint,
                rejection,
            } => {
                let bit_width = match type_hint.value {
                    TypeHintKind::Long => "64-bit",
                    _ => "32-bit",
                };
                match rejection {
                    SignedIntRejection::NotNumeric(spanned) => {
                        format!(
                            "'{}' requires a {} integer, but '{}' is not a number",
                            type_hint.value.token_name(),
                            bit_width,
                            spanned.value
                        )
                    }
                    SignedIntRejection::FloatingPoint(spanned) => {
                        format!(
                            "'{}' requires a whole number, but '{}' has a decimal point",
                            type_hint.value.token_name(),
                            spanned.value
                        )
                    }
                    SignedIntRejection::Overflow(spanned) => {
                        format!(
                            "'{}' requires a {} integer, but '{}' is out of range",
                            type_hint.value.token_name(),
                            bit_width,
                            spanned.value
                        )
                    }
                    SignedIntRejection::Missing(_) => {
                        unreachable!("Missing case handled by MissingTypeHintOperand")
                    }
                }
            }
            ParserError::MissingTypeHintOperand {
                type_hint, operand, ..
            } => {
                format!(
                    "'{}' type hint is missing its {}",
                    type_hint.value.token_name(),
                    operand
                )
            }
            ParserError::TypeHintExpectsFloatOperand {
                type_hint,
                rejection,
            } => {
                let bit_width = match type_hint.value {
                    TypeHintKind::Double => "64-bit",
                    _ => "32-bit",
                };
                match rejection {
                    FloatRejection::NotNumeric(spanned) => {
                        format!(
                            "'{}' requires a {} float, but '{}' is not a number",
                            type_hint.value.token_name(),
                            bit_width,
                            spanned.value
                        )
                    }
                    FloatRejection::Overflow(spanned) => {
                        format!(
                            "'{}' requires a {} float, but '{}' is out of range",
                            type_hint.value.token_name(),
                            bit_width,
                            spanned.value
                        )
                    }
                    FloatRejection::Missing(_) => {
                        unreachable!("Missing case handled by MissingTypeHintOperand")
                    }
                }
            }
        }
    }

    fn labels(&self) -> Vec<DiagnosticLabel> {
        match self {
            ParserError::EmptyFile(_) => {
                vec![DiagnosticLabel::at(0..0, "expected a '.class' directive")]
            }
            ParserError::UnexpectedTokenInClassBody(token) => {
                vec![DiagnosticLabel::at(
                    token.span().as_range(),
                    "unexpected token",
                )]
            }
            ParserError::UnexpectedTokenBeforeClassDefinition(unexpected) => {
                let valid_ctx = unexpected.can_appear_in();
                let msg = format!(
                    "valid context{} for {} {} {}: {}.",
                    if valid_ctx.len() == 1 { "" } else { "s" },
                    unexpected.as_identifier(),
                    unexpected.token_type(),
                    if valid_ctx.len() == 1 { "is" } else { "are" },
                    valid_ctx
                        .iter()
                        .map(|ctx| format!("'{}'", ctx))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                vec![DiagnosticLabel::at(unexpected.span().as_range(), msg)]
            }
            ParserError::IdentifierOrHintExpected(span, token, ctx) => {
                let context_label = DiagnosticLabel::context(
                    span.as_range(),
                    format!(
                        "the '{}' directive requires a {} as operand",
                        ctx.directive_name(),
                        ctx
                    ),
                );
                let error_label = if token.is_line_terminator() {
                    DiagnosticLabel::at(
                        span.byte_end..span.byte_end,
                        "but nothing was provided".to_string(),
                    )
                } else {
                    DiagnosticLabel::at(
                        token.span().as_range(),
                        format!(
                            "{} is not a valid identifier or type hint",
                            token.token_type()
                        ),
                    )
                };
                vec![context_label, error_label]
            }
            ParserError::TrailingTokens(last_valid_end, tokens, ctx) => {
                let context = match ctx {
                    TrailingTokensErrContext::TypeHint(th) => {
                        let operands_count = th.value.operands_count();
                        DiagnosticLabel::context(
                            th.span.as_range(),
                            format!(
                                "'{}' type hint takes {} operand{}",
                                th.value.token_name(),
                                operands_count,
                                if operands_count == 1 { "" } else { "s" }
                            ),
                        )
                    }
                    _ => DiagnosticLabel::context(
                        *last_valid_end..*last_valid_end,
                        format!("expected end of {} here", ctx),
                    ),
                };
                let msg = format!(
                    "but found {} trailing token{}",
                    tokens.len(),
                    if tokens.len() == 1 { "" } else { "s" }
                );
                vec![
                    context,
                    DiagnosticLabel::at(
                        tokens[0].span().byte_start..tokens.last().unwrap().span().byte_end,
                        msg,
                    ),
                ]
            }
            ParserError::MultipleSuperDefinitions(defs) => {
                let mut labels = Vec::with_capacity(defs.len());
                for (i, (span, hint)) in defs.iter().enumerate() {
                    let msg = if i == 0 {
                        format!(
                            "first .super directive defined here with super class '{}'",
                            hint.token_name_with_value()
                        )
                    } else {
                        "multiple .super directives are not allowed".to_string()
                    };
                    labels.push(DiagnosticLabel::at(span.as_range(), msg));
                }
                labels
            }
            ParserError::TypeHintExpectsIntegerOperand {
                type_hint,
                rejection,
            } => {
                let context_label = DiagnosticLabel::context(
                    type_hint.span.as_range(),
                    type_hint.value.context_label().to_string(),
                );
                let error_label = match rejection {
                    SignedIntRejection::NotNumeric(spanned) => DiagnosticLabel::at(
                        spanned.span.as_range(),
                        format!("'{}' is not a number", spanned.value),
                    ),
                    SignedIntRejection::FloatingPoint(spanned) => DiagnosticLabel::at(
                        spanned.span.as_range(),
                        format!("must be a whole number without a decimal point"),
                    ),
                    SignedIntRejection::Overflow(spanned) => {
                        let range_msg = match type_hint.value {
                            TypeHintKind::Long => {
                                format!("64-bit integer range is {} to {}", i64::MIN, i64::MAX)
                            }
                            _ => {
                                format!("32-bit integer range is {} to {}", i32::MIN, i32::MAX)
                            }
                        };
                        DiagnosticLabel::at(spanned.span.as_range(), range_msg)
                    }
                    SignedIntRejection::Missing(_) => {
                        unreachable!("Missing case handled by MissingTypeHintOperand")
                    }
                };
                vec![context_label, error_label]
            }
            ParserError::TypeHintExpectsFloatOperand {
                type_hint,
                rejection,
            } => {
                let context_label = DiagnosticLabel::context(
                    type_hint.span.as_range(),
                    type_hint.value.context_label().to_string(),
                );
                let error_label = match rejection {
                    FloatRejection::NotNumeric(spanned) => DiagnosticLabel::at(
                        spanned.span.as_range(),
                        format!("'{}' is not a number", spanned.value),
                    ),
                    FloatRejection::Overflow(spanned) => {
                        let range_msg = match type_hint.value {
                            TypeHintKind::Double => {
                                format!("64-bit float range is {} to {}", f64::MIN, f64::MAX)
                            }
                            _ => {
                                format!("32-bit float range is {} to {}", f32::MIN, f32::MAX)
                            }
                        };
                        DiagnosticLabel::at(spanned.span.as_range(), range_msg)
                    }
                    FloatRejection::Missing(_) => {
                        unreachable!("Missing case handled by MissingTypeHintOperand")
                    }
                };
                vec![context_label, error_label]
            }
            ParserError::MissingTypeHintOperand {
                type_hint,
                operand,
                after_span,
            } => {
                vec![
                    DiagnosticLabel::context(
                        type_hint.span.as_range(),
                        type_hint.value.context_label().to_string(),
                    ),
                    DiagnosticLabel::at(
                        after_span.byte_end..after_span.byte_end,
                        format!("but {} is missing", operand),
                    ),
                ]
            }
        }
    }

    fn note(&self) -> String {
        format!(
            "For more details see:\n{}{}",
            ERROR_DOCS_BASE_URL,
            self.code().to_ascii_lowercase()
        )
    }

    fn help(&self) -> Option<String> {
        match self {
            ParserError::EmptyFile(_) => {
                Some("Make sure the file contains a valid class definition.".to_string())
            }
            ParserError::UnexpectedTokenInClassBody(_) => Some(
                "Check the syntax of the class body and ensure all tokens are valid.".to_string(),
            ),
            ParserError::UnexpectedTokenBeforeClassDefinition(_) => {
                Some("Make sure the file starts with a '.class' directive.".to_string())
            }
            ParserError::IdentifierOrHintExpected(_, _token, ctx) => Some(format!(
                "Provide a {} after the '{}' directive, e.g.:\n\
                     {} MyClassName",
                ctx,
                ctx.directive_name(),
                ctx.directive_name()
            )),
            ParserError::TrailingTokens(_, tokens, _) => {
                if tokens[0].is_directive() {
                    Some(format!(
                        "If you are trying to declare a new '{}' directive, try to put it on a new line.",
                        tokens[0].token_name()
                    ))
                } else {
                    Some("Remove the trailing tokens or move them to a valid context.".to_string())
                }
            }
            ParserError::MultipleSuperDefinitions(_) => Some(
                "A class can only have one .super directive. Remove the duplicates.".to_string(),
            ),
            ParserError::TypeHintExpectsIntegerOperand {
                type_hint,
                rejection,
            } => {
                let operand_name = match type_hint.value {
                    TypeHintKind::Long => TypeHintOperandName::I64Literal,
                    _ => TypeHintOperandName::I32Literal,
                };
                let syntax = format!(
                    "{} {}",
                    type_hint.value.token_name(),
                    operand_name.placeholder()
                );
                let example = type_hint.value.example();
                match rejection {
                    SignedIntRejection::NotNumeric(spanned) => Some(format!(
                        "'{}' is not a valid integer literal.\n\n\
                         Syntax:\n\
                         {}\n\n\
                         For example:\n\
                         {}",
                        spanned.value, syntax, example
                    )),
                    SignedIntRejection::FloatingPoint(spanned) => {
                        let float_hint = match type_hint.value {
                            TypeHintKind::Long => "@double",
                            _ => "@float",
                        };
                        Some(format!(
                            "If you need a fractional value, use {} instead:\n\
                             {} {}",
                            float_hint, float_hint, spanned.value
                        ))
                    }
                    SignedIntRejection::Overflow(spanned) => match type_hint.value {
                        TypeHintKind::Long => Some(format!(
                            "The value '{}' exceeds the 64-bit integer range ({} to {}).",
                            spanned.value,
                            i64::MIN,
                            i64::MAX
                        )),
                        _ => Some(format!(
                            "If you need a larger integer, use @long instead:\n\
                             @long {}",
                            spanned.value
                        )),
                    },
                    SignedIntRejection::Missing(_) => {
                        unreachable!("Missing case handled by MissingTypeHintOperand")
                    }
                }
            }
            ParserError::TypeHintExpectsFloatOperand {
                type_hint,
                rejection,
            } => {
                let operand_name = match type_hint.value {
                    TypeHintKind::Double => TypeHintOperandName::F64Literal,
                    _ => TypeHintOperandName::F32Literal,
                };
                let syntax = format!(
                    "{} {}",
                    type_hint.value.token_name(),
                    operand_name.placeholder()
                );
                let example = type_hint.value.example();
                match rejection {
                    FloatRejection::NotNumeric(spanned) => Some(format!(
                        "'{}' is not a valid float literal.\n\n\
                         Syntax:\n\
                         {}\n\n\
                         For example:\n\
                         {}",
                        spanned.value, syntax, example
                    )),
                    FloatRejection::Overflow(spanned) => match type_hint.value {
                        TypeHintKind::Double => Some(format!(
                            "The value '{}' exceeds the 64-bit float range ({} to {}).",
                            spanned.value,
                            f64::MIN,
                            f64::MAX
                        )),
                        _ => Some(format!(
                            "If you need a larger float, use @double instead:\n\
                             @double {}",
                            spanned.value
                        )),
                    },
                    FloatRejection::Missing(_) => {
                        unreachable!("Missing case handled by MissingTypeHintOperand")
                    }
                }
            }
            ParserError::MissingTypeHintOperand { type_hint, .. } => {
                let syntax = type_hint
                    .value
                    .operand_names()
                    .iter()
                    .map(|op| op.placeholder())
                    .collect::<Vec<_>>()
                    .join(" ");
                Some(format!(
                    "Provide the value immediately after the hint:\n\
                     {} {}\n\n\
                     For example:\n\
                     {}",
                    type_hint.value.token_name(),
                    syntax,
                    type_hint.value.example()
                ))
            }
        }
    }

    fn primary_location(&self) -> Span {
        match self {
            ParserError::EmptyFile(span) | ParserError::IdentifierOrHintExpected(span, _, _) => {
                *span
            }
            ParserError::MultipleSuperDefinitions(defs) => defs[0].0,
            ParserError::TrailingTokens(_, tokens, _) => tokens[0].span(),
            ParserError::UnexpectedTokenInClassBody(token)
            | ParserError::UnexpectedTokenBeforeClassDefinition(token) => token.span(),
            ParserError::MissingTypeHintOperand { type_hint, .. }
            | ParserError::TypeHintExpectsIntegerOperand { type_hint, .. }
            | ParserError::TypeHintExpectsFloatOperand { type_hint, .. } => type_hint.span,
        }
    }

    fn lsp_message(&self) -> String {
        // TODO: stub
        self.asm_msg()
    }
}

impl From<ParserError> for Diagnostic {
    fn from(value: ParserError) -> Self {
        Diagnostic {
            asm_msg: value.asm_msg(),
            lsp_msg: value.lsp_message(),
            code: Some(value.code()),
            primary_location: value.primary_location(),
            note: Some(value.note()),
            help: value.help(),
            tier: DiagnosticTier::SyntaxError,
            labels: value.labels(),
        }
    }
}

impl From<ParserError> for Vec<Diagnostic> {
    fn from(value: ParserError) -> Self {
        vec![Diagnostic::from(value)]
    }
}
