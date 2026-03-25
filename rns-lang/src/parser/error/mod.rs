mod context;
mod rejection;

pub(super) use context::{
    AccessFlagContext, OperandErrPosContext, TrailingTokensErrContext, UnexpectedTokenContext,
};
pub(super) use rejection::{NumericRejection, ParseNumeric};

use crate::ERROR_DOCS_BASE_URL;
use crate::diagnostic::{Diagnostic, DiagnosticLabel, DiagnosticTier};
use crate::token::type_hint::{TypeHint, TypeHintKind, TypeHintOperandName};
use crate::token::{RnsFlag, Spanned};
use crate::token::{RnsToken, Span};
use std::vec;

// TODO: should actually take ownership when converting to Diagnostic

#[derive(Debug, PartialEq, Clone)]
pub(super) enum ParserError {
    EmptyFile(Span),
    UnexpectedBodyToken(UnexpectedTokenContext, RnsToken),
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
    TypeHintExpectsNumericOperand {
        type_hint: Spanned<TypeHintKind>,
        rejection: NumericRejection,
    },
    InvalidAccessFlag(AccessFlagContext, Spanned<RnsFlag>),
    MissingImplicitTypeHintOperand {
        err_ctx: OperandErrPosContext,
        implicit_kind: TypeHintKind,
        operand: TypeHintOperandName,
        after_span: Span,
    },
    MultipleCodeBlocks {
        method_name: Option<TypeHint>,
        method_span: Span,
        first_code_span: Span,
        duplicate: Span,
    },
    UnknownInstruction(Spanned<String>),
}

impl ParserError {
    // TODO: put all codes as consts somewhere, and make sure they are unique across the whole codebase
    fn code(&self) -> &'static str {
        match self {
            ParserError::TypeHintExpectsNumericOperand { .. } => "E-003",
            ParserError::EmptyFile(_) => "E-006",
            ParserError::UnexpectedBodyToken(ctx, _) => ctx.error_code(),
            ParserError::UnexpectedTokenBeforeClassDefinition(_) => "E-008",
            ParserError::IdentifierOrHintExpected(_, _, _) => "E-009",
            ParserError::TrailingTokens(_, _, ctx) => match ctx {
                TrailingTokensErrContext::Class => "E-010",
                TrailingTokensErrContext::Super => "E-012",
                TrailingTokensErrContext::TypeHint(_) => "E-013",
                TrailingTokensErrContext::Method => "E-018",
            },
            ParserError::MultipleSuperDefinitions(_) => "E-011",
            ParserError::MissingTypeHintOperand { .. } => "E-014",
            ParserError::MissingImplicitTypeHintOperand { .. } => "E-020",
            ParserError::MultipleCodeBlocks { .. } => "E-019",
            ParserError::UnknownInstruction { .. } => "E-021",
            ParserError::InvalidAccessFlag(ctx, _) => ctx.error_code(),
        }
    }

    fn asm_msg(&self) -> String {
        match self {
            ParserError::UnknownInstruction(instruction) => {
                format!("invalid instruction '{}'", instruction.value)
            }
            ParserError::EmptyFile(_) => "file contains no class definition".to_string(),
            ParserError::UnexpectedBodyToken(ctx, token) => {
                format!("unexpected token in {}: '{token:?}'", ctx)
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
            ParserError::TypeHintExpectsNumericOperand {
                type_hint,
                rejection,
            } => {
                let numeric_kind = type_hint.value.numeric_kind();
                let bit_width = type_hint.value.bit_width();
                match rejection {
                    NumericRejection::NotNumeric(spanned) => {
                        format!(
                            "'{}' requires a {} {}, but '{}' is not a number",
                            type_hint.value.token_name(),
                            bit_width,
                            numeric_kind,
                            spanned.value
                        )
                    }
                    NumericRejection::FloatingPoint(spanned) => {
                        format!(
                            "'{}' requires a whole number, but '{}' has a decimal point",
                            type_hint.value.token_name(),
                            spanned.value
                        )
                    }
                    NumericRejection::Overflow(spanned) => {
                        format!(
                            "'{}' requires a {} {}, but '{}' is out of range",
                            type_hint.value.token_name(),
                            bit_width,
                            numeric_kind,
                            spanned.value
                        )
                    }
                    NumericRejection::Missing(_) => {
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
            ParserError::InvalidAccessFlag(ctx, flag) => {
                format!("invalid {} access flag '{}'", ctx, flag.value.token_name())
            }
            ParserError::MissingImplicitTypeHintOperand {
                err_ctx, operand, ..
            } => {
                format!("missing {} after '{}'", operand, err_ctx.directive_name(),)
            }
            ParserError::MultipleCodeBlocks { method_name, .. } => {
                let method = if let Some(name_hint) = method_name {
                    format!("method '{}'", name_hint.value())
                } else {
                    "method".to_string()
                };
                format!("multiple code blocks defined for {}", method,)
            }
        }
    }

    fn labels(&self) -> Vec<DiagnosticLabel> {
        match self {
            ParserError::EmptyFile(_) => {
                vec![DiagnosticLabel::at(0..0, "expected a '.class' directive")]
            }
            ParserError::UnexpectedBodyToken(_, token) => {
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
                labels.push(DiagnosticLabel::context(
                    defs[0].0.as_range(),
                    format!(
                        "first .super directive defined here with super class '{}'",
                        defs[0].1.value()
                    ),
                ));

                for (span, hint) in defs.iter().skip(1) {
                    labels.push(DiagnosticLabel::at(
                        span.as_range(),
                        format!(
                            "but another .super directive defined here with super class '{}'",
                            hint.value()
                        ),
                    ));
                }
                labels
            }
            ParserError::TypeHintExpectsNumericOperand {
                type_hint,
                rejection,
            } => {
                let context_label = DiagnosticLabel::context(
                    type_hint.span.as_range(),
                    type_hint.value.context_label().to_string(),
                );
                let error_label = match rejection {
                    NumericRejection::NotNumeric(spanned) => DiagnosticLabel::at(
                        spanned.span.as_range(),
                        format!("'{}' is not a number", spanned.value),
                    ),
                    NumericRejection::FloatingPoint(spanned) => DiagnosticLabel::at(
                        spanned.span.as_range(),
                        "must be a whole number without a decimal point".to_string(),
                    ),
                    NumericRejection::Overflow(spanned) => DiagnosticLabel::at(
                        spanned.span.as_range(),
                        type_hint.value.range_description(),
                    ),
                    NumericRejection::Missing(_) => {
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
            ParserError::MissingImplicitTypeHintOperand {
                err_ctx,
                operand,
                after_span,
                ..
            } => {
                vec![
                    DiagnosticLabel::context(
                        after_span.as_range(),
                        format!(
                            "the '{}' directive requires a {} as operand",
                            err_ctx.directive_name(),
                            err_ctx,
                        ),
                    ),
                    DiagnosticLabel::at(
                        after_span.byte_end..after_span.byte_end,
                        format!("but {} is missing", operand),
                    ),
                ]
            }
            ParserError::InvalidAccessFlag(ctx, flag) => {
                vec![DiagnosticLabel::at(
                    flag.span.as_range(),
                    format!(
                        "'{}' is not a valid {} access flag",
                        flag.value.token_name(),
                        ctx
                    ),
                )]
            }
            ParserError::MultipleCodeBlocks {
                method_span,
                first_code_span,
                duplicate,
                method_name,
            } => {
                let method = if let Some(name_hint) = method_name {
                    format!("method '{}'", name_hint.value())
                } else {
                    "method".to_string()
                };
                vec![
                    DiagnosticLabel::context(
                        method_span.as_range(),
                        format!("{} defined here has multiple code blocks", method),
                    ),
                    DiagnosticLabel::context(
                        first_code_span.as_range(),
                        "first code block defined here".to_string(),
                    ),
                    DiagnosticLabel::at(
                        duplicate.as_range(),
                        "but another code block defined here".to_string(),
                    ),
                ]
            }
            ParserError::UnknownInstruction(instruction) => {
                vec![DiagnosticLabel::at(
                    instruction.span.as_range(),
                    format!("'{}' is not a valid instruction", instruction.value),
                )]
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
            ParserError::UnexpectedBodyToken(ctx, _) => Some(format!(
                "Check the syntax of the {} and ensure all tokens are valid.",
                ctx
            )),
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
            ParserError::TypeHintExpectsNumericOperand {
                type_hint,
                rejection,
            } => {
                let operand_name = type_hint.value.operand_names()[0];
                let syntax = format!(
                    "{} {}",
                    type_hint.value.token_name(),
                    operand_name.placeholder()
                );
                let example = type_hint.value.example();
                let numeric_kind = type_hint.value.numeric_kind();
                match rejection {
                    NumericRejection::NotNumeric(spanned) => Some(format!(
                        "'{}' is not a valid {} literal.\n\n\
                         Syntax:\n\
                         {}\n\n\
                         For example:\n\
                         {}",
                        spanned.value, numeric_kind, syntax, example
                    )),
                    NumericRejection::FloatingPoint(spanned) => {
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
                    NumericRejection::Overflow(spanned) => match type_hint.value {
                        TypeHintKind::Long | TypeHintKind::Double => Some(format!(
                            "The value '{}' exceeds the {}.",
                            spanned.value,
                            type_hint.value.range_description(),
                        )),
                        TypeHintKind::Integer => Some(format!(
                            "If you need a larger integer, use @long instead:\n\
                             @long {}",
                            spanned.value
                        )),
                        TypeHintKind::Float => Some(format!(
                            "If you need a larger float, use @double instead:\n\
                             @double {}",
                            spanned.value
                        )),
                        _ => unreachable!("Overflow on non-numeric type hint"),
                    },
                    NumericRejection::Missing(_) => {
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
            ParserError::InvalidAccessFlag(_, _) => None,
            ParserError::MissingImplicitTypeHintOperand {
                err_ctx,
                implicit_kind,
                ..
            } => {
                let syntax = implicit_kind
                    .operand_names()
                    .iter()
                    .map(|op| op.placeholder())
                    .collect::<Vec<_>>()
                    .join(" ");
                Some(format!(
                    "Provide a {} after the '{}' directive, e.g.:\n\
                     {} {}",
                    err_ctx,
                    err_ctx.directive_name(),
                    err_ctx.directive_name(),
                    syntax,
                ))
            }
            ParserError::MultipleCodeBlocks { .. } => {
                Some("Each method can only have one code block. Remove the duplicates or merge them into one.".to_string())
            }
            ParserError::UnknownInstruction { .. } => None,
        }
    }

    fn primary_location(&self) -> Span {
        match self {
            ParserError::EmptyFile(span) | ParserError::IdentifierOrHintExpected(span, _, _) => {
                *span
            }
            ParserError::MultipleSuperDefinitions(defs) => defs[1].0,
            ParserError::TrailingTokens(_, tokens, _) => tokens[0].span(),
            ParserError::MultipleCodeBlocks { duplicate, .. } => *duplicate,
            ParserError::UnexpectedBodyToken(_, token)
            | ParserError::UnexpectedTokenBeforeClassDefinition(token) => token.span(),
            ParserError::MissingTypeHintOperand { type_hint, .. }
            | ParserError::TypeHintExpectsNumericOperand { type_hint, .. } => type_hint.span,
            ParserError::InvalidAccessFlag(_, flag) => flag.span,
            ParserError::MissingImplicitTypeHintOperand { after_span, .. } => *after_span,
            ParserError::UnknownInstruction(instruction) => instruction.span,
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
