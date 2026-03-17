use crate::ERROR_DOCS_BASE_URL;
use crate::diagnostic::{Diagnostic, DiagnosticLabel, DiagnosticTier};
use crate::token::Spanned;
use crate::token::type_hint::{TypeHint, TypeHintKind};
use crate::token::{RnsToken, Span};
use std::fmt::{Display, Formatter};
use std::vec;

#[derive(Debug, Eq, PartialEq, Clone)]
pub(super) enum ParserError {
    EmptyFile(Span),
    // TODO: the messages are total shit
    UnexpectedTokenInClassBody(RnsToken),
    // TODO: the messages are total shit
    UnexpectedTokenBeforeClassDefinition(RnsToken),
    TrailingTokens(usize, Vec<RnsToken>, TrailingTokensErrContext),
    IdentifierOrHintExpected(Span, RnsToken, OperandErrPosContext),
    // TODO: the messages are total shit
    MultipleSuperDefinitions(Vec<(Span, TypeHint)>),
    TypeHintExpectsI32Operand {
        int_type_hint_span: Span,
        found: String,
        found_span: Span,
        ctx: OperandErrPosContext,
    },
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub(super) enum OperandErrPosContext {
    ClassName,
    SuperName,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub(super) enum TrailingTokensErrContext {
    Class,
    Super,
    TypeHint(Spanned<TypeHintKind>),
}

impl Display for TrailingTokensErrContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TrailingTokensErrContext::Class => write!(f, "class definition"),
            TrailingTokensErrContext::Super => write!(f, "super class definition"),
            TrailingTokensErrContext::TypeHint(kind) => {
                write!(f, "type hint '{}'", kind.value.token_name())
            }
        }
    }
}

impl Display for OperandErrPosContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            OperandErrPosContext::ClassName => write!(f, "class name"),
            OperandErrPosContext::SuperName => write!(f, "super class name"),
        }
    }
}

impl OperandErrPosContext {
    fn expected_type_hint_kinds(&self) -> Vec<TypeHintKind> {
        match self {
            OperandErrPosContext::ClassName | OperandErrPosContext::SuperName => {
                vec![TypeHintKind::Class]
            }
        }
    }

    fn directive_name(&self) -> &'static str {
        match self {
            //TODO: use something like TokenKind::DotClass.name() to not hardcode here
            OperandErrPosContext::ClassName => ".class",
            OperandErrPosContext::SuperName => ".super",
        }
    }
}

impl ParserError {
    // TODO: put all codes as consts somewhere, and make sure they are unique across the whole codebase
    fn code(&self) -> &'static str {
        match self {
            ParserError::TypeHintExpectsI32Operand { .. } => "E-003",
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
            ParserError::TypeHintExpectsI32Operand { found, .. } => {
                format!("@int requires an i32 literal, found '{}'", found)
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
                let token_type = token.token_type();
                let msg = format!(
                    "attempted to set {} as {}, but it can't be resolved as either an identifier or a type hint",
                    token_type, ctx
                );
                vec![DiagnosticLabel::at(span.as_range(), msg)]
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
            ParserError::TypeHintExpectsI32Operand {
                int_type_hint_span,
                found,
                found_span,
                ..
            } => vec![
                DiagnosticLabel::context(
                    int_type_hint_span.as_range(),
                    "expects an 32-bit signed integer operand".to_string(),
                ),
                DiagnosticLabel::at(found_span.as_range(), format!("but found '{}'", found)),
            ],
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
            ParserError::IdentifierOrHintExpected(_, token, _ctx) =>
            /*match token {
            RnsToken::Integer(spanned) => Some(format!(
                "The constant pool requires each entry to have a specific kind.\n\
                 The parser can't determine whether you intended an identifier or an integer from the bare token alone.\n\n\
                 To treat it as an identifier (will resolve to the appropriate entry kind from context):\n\
                 #\"{}\"\n\n\
                 To explicitly store it as an {} constant pool entry:\n\
                 {} {}",
                spanned.value,
                TypeHintKind::Integer,
                TypeHintKind::Integer,
                spanned.value
            )),
            _ =>*/
            {
                unimplemented!()
            }
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
            ParserError::TypeHintExpectsI32Operand { found, ctx, .. } => Some(format!(
                "@int type hint is used to explicitly store numeric values in the\n\
                 constant pool. '{}' is an identifier, not a numeric literal.\n\
                 \n\
                 If you intended '{}' to be a {}, remove the @int hint:\n\
                 {} {}\n\
                 \n\
                 If you want to make it an @int, replace '{}' with a valid i32 literal",
                found,
                found,
                ctx,
                ctx.directive_name(),
                if found.contains(' ') {
                    format!("\"{}\"", found)
                } else {
                    found.to_string()
                },
                found
            )),
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
            ParserError::TypeHintExpectsI32Operand {
                int_type_hint_span, ..
            } => *int_type_hint_span,
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
