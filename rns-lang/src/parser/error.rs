use crate::diagnostic::{Diagnostic, DiagnosticLabel, DiagnosticTier};
use crate::suggestion;
use crate::token::{RnsToken, Span};

#[derive(Debug, Eq, PartialEq, Clone)]
pub(super) enum ParserError {
    ClassDirectiveExpected(Span, RnsToken),
    TrailingTokens(Vec<RnsToken>, TrailingTokensContext),
    IdentifierExpected(Span, RnsToken, IdentifierContext),

    UnexpectedCodeDirectiveArg(Span, RnsToken),

    NonNegativeIntegerExpected(Span, RnsToken, NonNegativeIntegerContext),

    UnknownInstruction(Span, String),

    EmptyFile(Span),
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
    fn asm_msg(&self) -> String {
        match self {
            ParserError::ClassDirectiveExpected(_, token) => format!(
                "unexpected {} before class definition",
                token.as_string_token_type()
            ),
            ParserError::TrailingTokens(tokens, context) => {
                let first_token_kind = &tokens[0];
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
                    RnsToken::Newline(_) | RnsToken::Eof(_) => {
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
        }
    }

    fn labels(&self) -> Vec<DiagnosticLabel> {
        match self {
            ParserError::TrailingTokens(tokens, context) => {
                let msg = match context {
                    TrailingTokensContext::Class => {
                        let first_token_kind = &tokens[0];
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
                            RnsToken::Integer(_) => {
                                "integer literals are not allowed here".to_string()
                            }
                            RnsToken::Identifier(_) => "not allowed here".to_string(),
                            RnsToken::StringLiteral(_) => {
                                "string literals are not allowed here".to_string()
                            }
                            _ => "not allowed here".to_string(),
                        }
                    }
                    _ => "not allowed here".to_string(),
                };
                vec![DiagnosticLabel::at(self.primary_location().as_range(), msg)]
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
                    RnsToken::DotEnd(_) => {
                        format!("'{}' has no matching start directive", token)
                    }
                    _ => format!(
                        "this {} must appear inside a class definition",
                        token.as_string_token_type()
                    ),
                };
                vec![DiagnosticLabel::at(self.primary_location().as_range(), msg)]
            }
            ParserError::IdentifierExpected(_, token, context) => {
                let msg = match context {
                    IdentifierContext::ClassName => match token {
                        RnsToken::Newline(_) | RnsToken::Eof(_) => {
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
                vec![DiagnosticLabel::at(self.primary_location().as_range(), msg)]
            }
            ParserError::UnexpectedCodeDirectiveArg(_, token) => {
                vec![DiagnosticLabel::at(
                    self.primary_location().as_range(),
                    format!("'{}' is not a valid argument for '.code'", token),
                )]
            }
            ParserError::NonNegativeIntegerExpected(_, token, _) => vec![DiagnosticLabel::at(
                self.primary_location().as_range(),
                format!("expected a non-negative integer, found '{}'", token),
            )],
            ParserError::UnknownInstruction(_, name) => {
                let msg = if let Some(s) = suggestion::closest_instruction(name) {
                    format!("did you mean '{}' ?", s)
                } else {
                    "unknown instruction".to_string()
                };
                vec![DiagnosticLabel::at(self.primary_location().as_range(), msg)]
            }
            ParserError::EmptyFile(_) => {
                vec![DiagnosticLabel::at(
                    self.primary_location().as_range(),
                    "the file is empty or contains only comments",
                )]
            }
        }
    }

    fn note(&self) -> Option<String> {
        match self {
            ParserError::ClassDirectiveExpected(_, token) => match token {
                _ if token.is_access_flag() => Some(
                    "Access flags must appear within a '.class' or '.method' directive.".to_string(),
                ),
                // TODO: is class nested instead?
                RnsToken::DotMethod(_) | RnsToken::DotSuper(_) => {
                    Some("Define a class first using '.class [access_flags] <name>'.".to_string())
                }
                RnsToken::DotCode(_) => Some(
                    "The '.code' directive is only valid inside a method definition. Define a method first using '.method [access_flags] <name> <descriptor>'."
                        .to_string(),
                ),
                RnsToken::DotEnd(_) => Some(
                    "The '.end' directive must match a previous '.method', '.code', or '.class' directive.".to_string(),
                ),
                RnsToken::Identifier(spanned) => Some(
                    format!("Found identifier '{}' before any class was defined. Did you forget to start the class? Try: '.class {}'", spanned.value, spanned.value),
                ),
                RnsToken::Integer(_) => Some(
                    "Integer literals are typically used as instruction arguments inside '.code' blocks.".to_string(),
                ),
                RnsToken::StringLiteral(_) => Some(
                    "String literals are constant values that must appear inside '.code' blocks as instruction arguments.".to_string(),
                ),
                RnsToken::DotAnnotation(_) => Some(
                    "The '.annotation' directive is only valid inside a class or method definition."
                        .to_string(),
                ),
                _ => {
                    Some("All assembly code must be placed inside a class definition starting with '.class'.".to_string())
                }
            },
            ParserError::TrailingTokens(tokens, context) => {
                let first_token_kind = &tokens[0];
                match context {
                    TrailingTokensContext::Class => match first_token_kind {
                        _ if first_token_kind.is_class_nested_directive() => {
                            Some(format!("Consider starting a new line for the '{}' directive.", first_token_kind))
                        }
                        _ if first_token_kind.is_access_flag() => {
                            // TODO: bad note, almost the same as the label
                            Some("Access flags must appear before the class name: '.class [access_flags] <name>'".to_string())
                        }
                        RnsToken::DotClass(_) => {
                            Some("The '.class' directive cannot be nested. Consider removing the second '.class' (todo when nested metada data is supported explain it).".to_string())
                        }
                        RnsToken::DotCode(_) => {
                            Some("The '.code' directive must be inside a method definition, not directly after the class name.".to_string())
                        }
                        RnsToken::DotEnd(_) => {
                            Some("The '.end' directive must match a previous '.method', '.code', or '.class' directive. It cannot appear directly after the class name.".to_string())
                        }
                        RnsToken::Integer(_) => {
                            Some("Integer literals belong inside '.code' blocks as instruction arguments.".to_string())
                        }
                        RnsToken::StringLiteral(_) => {
                            Some("String literals belong inside '.code' blocks as instruction arguments.".to_string())
                        }
                        RnsToken::Identifier(_) => {
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
                    RnsToken::StringLiteral(_),
                    IdentifierContext::ClassName | IdentifierContext::SuperName,
                ) => Some("Consider removing the quotes around the class name".to_string()),
                (RnsToken::StringLiteral(_), IdentifierContext::MethodName) => {
                    Some("Consider removing the quotes around the method name".to_string())
                }
                (RnsToken::DotClass(_) | RnsToken::DotMethod(_) | RnsToken::DotSuper(_) | RnsToken::DotCode(_) | RnsToken::DotEnd(_), IdentifierContext::ClassName) => {
                    Some("Directives are reserved keywords. If you meant to start a new directive, do so on a new line.".to_string())
                }
                (RnsToken::Integer(_), IdentifierContext::ClassName) => {
                    Some("Integer literals cannot be used as class names. Every class must have a valid identifier as its name.".to_string())
                }
                (RnsToken::Newline(_) | RnsToken::Eof(_), IdentifierContext::ClassName) => {
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
        }
    }

    fn primary_location(&self) -> Span {
        match self {
            ParserError::ClassDirectiveExpected(span, _)
            | ParserError::EmptyFile(span)
            | ParserError::IdentifierExpected(span, _, _)
            | ParserError::UnexpectedCodeDirectiveArg(span, _)
            | ParserError::NonNegativeIntegerExpected(span, _, _)
            | ParserError::UnknownInstruction(span, _) => *span,
            ParserError::TrailingTokens(tokens, _) => Span {
                byte_start: tokens[0].span().byte_start,
                byte_end: tokens.last().map(|v| v.span().byte_end).unwrap_or(0),
                line: tokens[0].span().line,
                col_start: tokens[0].span().col_start,
                col_end: tokens.last().map(|v| v.span().col_end).unwrap_or(0),
            },
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
            code: Some("PARSER-001"),
            primary_location: value.primary_location(),
            note: value.note(),
            help: None,
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
