use crate::diagnostic::{Diagnostic, DiagnosticLabel, DiagnosticTier};
use crate::instruction::INSTRUCTION_SPECS;
use crate::token::{RnsFlag, RnsToken, RnsTokenKind, Span};
use std::ops::Range;

#[derive(Debug, Eq, PartialEq, Clone)]
pub(super) enum ParserError {
    ClassDirectiveExpected(Span, RnsTokenKind),
    TrailingTokens(Vec<RnsToken>, TrailingTokensContext),
    IdentifierExpected(Span, RnsTokenKind, IdentifierContext),

    UnexpectedCodeDirectiveArg(Span, RnsTokenKind),

    NonNegativeIntegerExpected(Span, RnsTokenKind, NonNegativeIntegerContext),

    UnknownInstruction(Span, String),

    EmptyFile(Span),
    //TODO: add a specific handling
    Internal(String),
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub(super) enum NonNegativeIntegerContext {
    CodeLocals,
    CodeStack,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub(super) enum TrailingTokensContext {
    Class,
    Super,
    Method,
    Code,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub(super) enum IdentifierContext {
    ClassName,
    SuperName,
    MethodName,
    MethodDescriptor,
    InstructionName,
    ClassNameInstructionArg,
    MethodNameInstructionArg,
    FieldNameInstructionArg,
    FieldDescriptorInstructionArg,
}

impl ParserError {
    fn get_message(&self) -> String {
        match self {
            ParserError::ClassDirectiveExpected(_, token) => format!(
                "unexpected {} before class definition",
                token.as_string_token_type()
            ),
            ParserError::TrailingTokens(tokens, context) => {
                let first_token_kind = &tokens[0].kind;
                match context {
                    TrailingTokensContext::Class => format!(
                        "unexpected {} after class name",
                        first_token_kind.as_string_token_type()
                    ),
                    TrailingTokensContext::Super => format!(
                        "unexpected {} after superclass name",
                        first_token_kind.as_string_token_type()
                    ),
                    TrailingTokensContext::Method => format!(
                        "unexpected {} after method signature",
                        first_token_kind.as_string_token_type()
                    ),
                    TrailingTokensContext::Code => format!(
                        "unexpected {} after '.code' directive",
                        first_token_kind.as_string_token_type()
                    ),
                }
            }
            ParserError::IdentifierExpected(_, token, context) => match context {
                IdentifierContext::ClassName => match token {
                    RnsTokenKind::Newline | RnsTokenKind::Eof => {
                        "missing class name in '.class' directive".to_string()
                    }
                    _ if token.is_directive() => {
                        format!("cannot use directive '{}' as a class name", token)
                    }
                    _ => "expected class name".to_string(),
                },
                IdentifierContext::SuperName => "incomplete '.super' directive".to_string(),
                IdentifierContext::MethodName => "incomplete '.method' directive".to_string(),
                IdentifierContext::MethodDescriptor => "missing method descriptor".to_string(),
                IdentifierContext::InstructionName => "expected instruction".to_string(),
                IdentifierContext::ClassNameInstructionArg => "missing class name".to_string(),
                IdentifierContext::MethodNameInstructionArg => "missing method name".to_string(),
                IdentifierContext::FieldNameInstructionArg => "missing field name".to_string(),
                IdentifierContext::FieldDescriptorInstructionArg => {
                    "missing field descriptor".to_string()
                }
            },
            ParserError::UnexpectedCodeDirectiveArg(_, token) => format!(
                "unexpected argument in '.code' directive: {}",
                token.as_string_token_type()
            ),
            ParserError::NonNegativeIntegerExpected(_, _token, context) => {
                let context_name = match context {
                    NonNegativeIntegerContext::CodeLocals => "locals limit",
                    NonNegativeIntegerContext::CodeStack => "stack limit",
                };
                format!("expected non-negative integer for {}", context_name)
            }
            ParserError::UnknownInstruction(_, name) => {
                format!("unknown instruction '{}'", name)
            }
            ParserError::EmptyFile(_) => "file contains no class definition".to_string(),
            ParserError::Internal(msg) => format!("Internal parser error: {}", msg),
        }
    }

    fn get_labels(&self) -> Vec<DiagnosticLabel> {
        match self {
            ParserError::TrailingTokens(tokens, context) => {
                let msg = match context {
                    TrailingTokensContext::Class => {
                        let first_token_kind = &tokens[0].kind;
                        match first_token_kind {
                            _ if first_token_kind.is_class_nested_directive() => {
                                "must start on a new line".to_string()
                            }
                            _ if first_token_kind.is_directive() => {
                                format!("directive '{}' is not allowed here", first_token_kind)
                            }
                            _ if first_token_kind.is_access_flag() => {
                                "access flags must appear before the class name".to_string()
                            }
                            RnsTokenKind::Integer(_) => {
                                "integer literals are not allowed here".to_string()
                            }
                            RnsTokenKind::Identifier(_) => "not allowed here".to_string(),
                            RnsTokenKind::StringLiteral(_) => {
                                "string literals are not allowed here".to_string()
                            }
                            _ => "not allowed here".to_string(),
                        }
                    }
                    _ => "not allowed here".to_string(),
                };
                vec![DiagnosticLabel::at(self.get_primary_location(), msg)]
            }
            ParserError::ClassDirectiveExpected(_, token) => {
                let msg = match token {
                    _ if token.is_class_nested_directive()
                        && token.is_method_nested_directive() =>
                    {
                        format!(
                            "'{}' is only allowed inside a class or method definition",
                            token
                        )
                    }
                    _ if token.is_class_nested_directive() => {
                        format!("'{}' is only allowed inside a class definition", token)
                    }
                    _ if token.is_method_nested_directive() => {
                        format!("'{}' is only allowed inside a method definition", token)
                    }
                    RnsTokenKind::DotEnd => {
                        format!("'{}' has no matching start directive", token)
                    }
                    _ => format!(
                        "this {} must appear inside a class definition",
                        token.as_string_token_type()
                    ),
                };
                vec![DiagnosticLabel::at(self.get_primary_location(), msg)]
            }
            ParserError::IdentifierExpected(_, token, context) => {
                let msg = match context {
                    IdentifierContext::ClassName => match token {
                        RnsTokenKind::Newline | RnsTokenKind::Eof => {
                            "expected a class name here".to_string()
                        }
                        _ if token.is_directive() => {
                            "directives cannot be used as names".to_string()
                        }
                        _ => format!("found '{}' instead", token),
                    },
                    IdentifierContext::SuperName => "expected a superclass name".to_string(),
                    IdentifierContext::MethodName => "expected a method name".to_string(),
                    IdentifierContext::MethodDescriptor => {
                        "expected a method descriptor (e.g., '(I)V')".to_string()
                    }
                    IdentifierContext::InstructionName => {
                        "expected an instruction mnemonic".to_string()
                    }
                    IdentifierContext::ClassNameInstructionArg => {
                        "expected a class name".to_string()
                    }
                    IdentifierContext::MethodNameInstructionArg => {
                        "expected a method name".to_string()
                    }
                    IdentifierContext::FieldNameInstructionArg => {
                        "expected a field name".to_string()
                    }
                    IdentifierContext::FieldDescriptorInstructionArg => {
                        "expected a field descriptor".to_string()
                    }
                };
                vec![DiagnosticLabel::at(self.get_primary_location(), msg)]
            }
            ParserError::UnexpectedCodeDirectiveArg(_, token) => {
                vec![DiagnosticLabel::at(
                    self.get_primary_location(),
                    format!("'{}' is not a valid argument for '.code'", token),
                )]
            }
            ParserError::NonNegativeIntegerExpected(_, token, _) => vec![DiagnosticLabel::at(
                self.get_primary_location(),
                format!("expected a non-negative integer, found '{}'", token),
            )],
            ParserError::UnknownInstruction(_, name) => {
                let mut closest = None;
                let mut min_dist = usize::MAX;
                for (mnemonic, _) in INSTRUCTION_SPECS.entries() {
                    let dist = crate::utils::levenshtein_distance(name, mnemonic);
                    if dist < min_dist && dist <= 2 {
                        min_dist = dist;
                        closest = Some(mnemonic);
                    }
                }

                let msg = if let Some(suggestion) = closest {
                    format!("did you mean '{}' ?", suggestion)
                } else {
                    "unknown instruction".to_string()
                };
                vec![DiagnosticLabel::at(self.get_primary_location(), msg)]
            }
            ParserError::Internal(_) => vec![],
            ParserError::EmptyFile(_) => {
                vec![DiagnosticLabel::at(
                    self.get_primary_location(),
                    "the file is empty or contains only comments",
                )]
            }
        }
    }

    fn get_note(&self) -> Option<String> {
        match self {
            ParserError::ClassDirectiveExpected(_, token) => match token {
                _ if token.is_access_flag() => Some(
                    "Access flags must appear within a '.class' or '.method' directive.".to_string(),
                ),
                // TODO: is class nested instead?
                RnsTokenKind::DotMethod | RnsTokenKind::DotSuper => {
                    Some("Define a class first using '.class [access_flags] <name>'.".to_string())
                }
                RnsTokenKind::DotCode => Some(
                    "The '.code' directive is only valid inside a method definition. Define a method first using '.method [access_flags] <name> <descriptor>'."
                        .to_string(),
                ),
                RnsTokenKind::DotEnd => Some(
                    "The '.end' directive must match a previous '.method', '.code', or '.class' directive.".to_string(),
                ),
                RnsTokenKind::Identifier(name) => Some(
                    format!("Found identifier '{}' before any class was defined. Did you forget to start the class? Try: '.class {}'", name, name),
                ),
                RnsTokenKind::Integer(_) => Some(
                    "Integer literals are typically used as instruction arguments inside '.code' blocks.".to_string(),
                ),
                RnsTokenKind::StringLiteral(_) => Some(
                    "String literals are constant values that must appear inside '.code' blocks as instruction arguments.".to_string(),
                ),
                RnsTokenKind::DotAnnotation => Some(
                    "The '.annotation' directive is only valid inside a class or method definition."
                        .to_string(),
                ),
                _ => {
                    Some("All assembly code must be placed inside a class definition starting with '.class'.".to_string())
                }
            },
            ParserError::TrailingTokens(tokens, context) => {
                let first_token_kind = &tokens[0].kind;
                match context {
                    TrailingTokensContext::Class => match first_token_kind {
                        _ if first_token_kind.is_class_nested_directive() => {
                            Some(format!("Consider starting a new line for the '{}' directive.", first_token_kind))
                        }
                        _ if first_token_kind.is_access_flag() => {
                            // TODO: bad note, almost the same as the label
                            Some("Access flags must appear before the class name: '.class [access_flags] <name>'".to_string())
                        }
                        RnsTokenKind::DotClass => {
                            Some("The '.class' directive cannot be nested. Consider removing the second '.class' (todo when nested metada data is supported explain it).".to_string())
                        }
                        RnsTokenKind::DotCode => {
                            Some("The '.code' directive must be inside a method definition, not directly after the class name.".to_string())
                        }
                        RnsTokenKind::DotEnd => {
                            Some("The '.end' directive must match a previous '.method', '.code', or '.class' directive. It cannot appear directly after the class name.".to_string())
                        }
                        RnsTokenKind::Integer(_) => {
                            Some("Integer literals belong inside '.code' blocks as instruction arguments.".to_string())
                        }
                        RnsTokenKind::StringLiteral(_) => {
                            Some("String literals belong inside '.code' blocks as instruction arguments.".to_string())
                        }
                        RnsTokenKind::Identifier(_) => {
                            Some("The class header should end by the class name. Use directives like '.method' or '.field' on the new line for other members.".to_string())
                        }
                        _ => Some("The class definition should end after the class name.".to_string()),
                    },
                    TrailingTokensContext::Super => Some(
                        "The .super directive must end after the superclass name.\nConsider starting a new line for the next directive.".to_string(),
                    ),
                    TrailingTokensContext::Method => Some(
                        "The .method directive must end after the method signature.\nConsider starting a new line for the next directive.".to_string(),
                    ),
                    TrailingTokensContext::Code => Some(
                        "The .code directive must end after stack/local arguments.\nConsider starting a new line for the next directive.".to_string(),
                    ),
                }
            }
            ParserError::IdentifierExpected(_, kind, context) => match (kind, context) {
                (
                    RnsTokenKind::StringLiteral(_),
                    IdentifierContext::ClassName | IdentifierContext::SuperName,
                ) => Some("Consider removing the quotes around the class name".to_string()),
                (RnsTokenKind::StringLiteral(_), IdentifierContext::MethodName) => {
                    Some("Consider removing the quotes around the method name".to_string())
                }
                (RnsTokenKind::DotClass | RnsTokenKind::DotMethod | RnsTokenKind::DotSuper | RnsTokenKind::DotCode | RnsTokenKind::DotEnd, IdentifierContext::ClassName) => {
                    Some("Directives are reserved keywords. If you meant to start a new directive, do so on a new line.".to_string())
                }
                (RnsTokenKind::Integer(_), IdentifierContext::ClassName) => {
                    Some("Integer literals cannot be used as class names. Every class must have a valid identifier as its name.".to_string())
                }
                (RnsTokenKind::AccessFlag(RnsFlag::Public) | RnsTokenKind::AccessFlag(RnsFlag::Static), IdentifierContext::ClassName) => {
                    Some(format!("Access flags like '{}' must appear before the class name. Example: '.class {} MyClass'", kind, kind))
                }
                (RnsTokenKind::Newline | RnsTokenKind::Eof, IdentifierContext::ClassName) => {
                    Some("Every class definition needs a name. Example: '.class public MyClass'".to_string())
                }
                (_, IdentifierContext::ClassName) => Some(
                    "The .class directive requires a valid Java class name:\n.class [access_flags] <name>"
                        .to_string(),
                ),
                (_, IdentifierContext::SuperName) => Some(
                    "The .super directive requires a superclass name.".to_string(),
                ),
                (_, IdentifierContext::MethodName) => Some(
                    "The .method directive requires a method name followed by parentheses and a method descriptor.".to_string(),
                ),
                (_, IdentifierContext::InstructionName) => Some(
                    "Instructions must appear inside a '.code' block.".to_string(),
                ),
                (_, IdentifierContext::ClassNameInstructionArg) => Some(
                    "This instruction requires a class name as its first argument.".to_string(),
                ),
                (_, IdentifierContext::MethodNameInstructionArg) => Some(
                    "This instruction requires a method name as an argument.".to_string(),
                ),
                (_, IdentifierContext::FieldNameInstructionArg) => Some(
                    "This instruction requires a field name as an argument.".to_string(),
                ),
                (_, IdentifierContext::FieldDescriptorInstructionArg) => Some(
                    "This instruction requires a field descriptor (e.g., 'I', 'Ljava/lang/String;') as an argument.".to_string(),
                ),
                (_, IdentifierContext::MethodDescriptor) => Some(
                    "Method descriptors describe the parameter and return types of a method. Example: '(I)V' for a method that takes an int and returns void.".to_string(),
                ),
            },
            ParserError::UnexpectedCodeDirectiveArg(_, _) => Some(
                "The .code directive only accepts two non-negative integers: stack limit and locals limit.\nExample: '.code 2 1'".to_string(),
            ),
            ParserError::NonNegativeIntegerExpected(_, _, context) => Some(match context {
                NonNegativeIntegerContext::CodeStack => {
                    "The first argument to '.code' is the maximum stack depth.".to_string()
                }
                NonNegativeIntegerContext::CodeLocals => {
                    "The second argument to '.code' is the number of local variable slots.".to_string()
                }
            }),
            ParserError::UnknownInstruction(_, _) => Some(
                "Check the Java Virtual Machine specification for a list of valid opcodes.".to_string(),
            ),
            ParserError::EmptyFile(_) => Some("A Java assembly file must start with a '.class' directive.".to_string()),
            ParserError::Internal(_) => None,
        }
    }

    fn get_primary_location(&self) -> Range<usize> {
        match self {
            ParserError::ClassDirectiveExpected(span, _)
            | ParserError::EmptyFile(span)
            | ParserError::IdentifierExpected(span, _, _)
            | ParserError::UnexpectedCodeDirectiveArg(span, _)
            | ParserError::NonNegativeIntegerExpected(span, _, _)
            | ParserError::UnknownInstruction(span, _) => span.as_range(),
            ParserError::TrailingTokens(tokens, _) => {
                tokens[0].span.start..tokens.last().map(|v| v.span.end).unwrap_or(0)
            }
            ParserError::Internal(_) => 0..0,
        }
    }
}

impl From<ParserError> for Diagnostic {
    fn from(value: ParserError) -> Self {
        Diagnostic {
            message: value.get_message(),
            code: "PARSER-001",
            primary_location: value.get_primary_location(),
            note: value.get_note(),
            help: None,
            tier: DiagnosticTier::SyntaxError,
            labels: value.get_labels(),
        }
    }
}

impl From<ParserError> for Vec<Diagnostic> {
    fn from(value: ParserError) -> Self {
        vec![Diagnostic::from(value)]
    }
}
