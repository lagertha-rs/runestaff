use crate::diagnostic::{Diagnostic, DiagnosticLabel, JasmError, Severity};
use crate::instruction::INSTRUCTION_SPECS;
use crate::parser::SuperDirective;
use crate::token::{JasmToken, JasmTokenKind, Span};
use std::ops::Range;

#[derive(Debug, Eq, PartialEq, Clone)]
pub(super) enum ParserError {
    ClassDirectiveExpected(Span, JasmTokenKind),
    TrailingTokens(Vec<JasmToken>, TrailingTokensContext),
    IdentifierExpected(Span, JasmTokenKind, IdentifierContext),

    MethodDescriptorExpected(Span, JasmTokenKind, MethodDescriptorContext),

    UnexpectedCodeDirectiveArg(Span, JasmTokenKind),

    NonNegativeIntegerExpected(Span, JasmTokenKind, NonNegativeIntegerContext),

    UnknownInstruction(Span, String),

    MultipleDefinitions(MultipleDefinitionContext),

    EmptyFile(Span),
    Internal(String),
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub(super) enum MultipleDefinitionContext {
    SuperClass {
        first_definition: SuperDirective,
        second_definition: SuperDirective,
    },
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
    InstructionName,
    ClassNameInstructionArg,
    MethodNameInstructionArg,
    FieldNameInstructionArg,
    FieldDescriptorInstructionArg,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub(super) enum MethodDescriptorContext {
    MethodDirective,
    Instruction,
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
                    JasmTokenKind::Newline | JasmTokenKind::Eof => {
                        "missing class name in '.class' directive".to_string()
                    }
                    JasmTokenKind::DotClass
                    | JasmTokenKind::DotSuper
                    | JasmTokenKind::DotMethod
                    | JasmTokenKind::DotCode
                    | JasmTokenKind::DotEnd => {
                        format!("cannot use directive '{}' as a class name", token)
                    }
                    _ => "expected class name".to_string(),
                },
                IdentifierContext::SuperName => "incomplete '.super' directive".to_string(),
                IdentifierContext::MethodName => "incomplete '.method' directive".to_string(),
                IdentifierContext::InstructionName => "expected instruction".to_string(),
                IdentifierContext::ClassNameInstructionArg => "missing class name".to_string(),
                IdentifierContext::MethodNameInstructionArg => "missing method name".to_string(),
                IdentifierContext::FieldNameInstructionArg => "missing field name".to_string(),
                IdentifierContext::FieldDescriptorInstructionArg => {
                    "missing field descriptor".to_string()
                }
            },
            ParserError::MethodDescriptorExpected(_, token, _) => format!(
                "expected method descriptor but found {}",
                token.as_string_token_type()
            ),
            ParserError::UnexpectedCodeDirectiveArg(_, token) => format!(
                "unexpected argument in '.code' directive: {}",
                token.as_string_token_type()
            ),
            ParserError::NonNegativeIntegerExpected(_, token, context) => {
                let context_name = match context {
                    NonNegativeIntegerContext::CodeLocals => "locals limit",
                    NonNegativeIntegerContext::CodeStack => "stack limit",
                };
                format!("expected non-negative integer for {}", context_name)
            }
            ParserError::UnknownInstruction(_, name) => {
                format!("unknown instruction '{}'", name)
            }
            ParserError::MultipleDefinitions(context) => format!(
                "multiple {} definitions",
                match context {
                    MultipleDefinitionContext::SuperClass { .. } => "superclass",
                }
            ),
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
                            JasmTokenKind::DotSuper | JasmTokenKind::DotMethod => {
                                "must start on a new line".to_string()
                            }
                            JasmTokenKind::DotClass
                            | JasmTokenKind::DotCode
                            | JasmTokenKind::DotEnd => {
                                format!("directive '{}' is not allowed here", first_token_kind)
                            }
                            JasmTokenKind::Public | JasmTokenKind::Static => {
                                "access flags must appear before the class name".to_string()
                            }
                            JasmTokenKind::Integer(_) => {
                                "integer literals are not allowed here".to_string()
                            }
                            JasmTokenKind::Identifier(_) => "not allowed here".to_string(),
                            JasmTokenKind::StringLiteral(_) => {
                                "string literals are not allowed here".to_string()
                            }
                            JasmTokenKind::MethodDescriptor(_) => {
                                "method descriptors are not allowed here".to_string()
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
                    JasmTokenKind::DotMethod | JasmTokenKind::DotSuper => {
                        format!("'{}' is only allowed inside a class definition", token)
                    }
                    JasmTokenKind::DotCode => {
                        format!("'{}' is only allowed inside a method definition", token)
                    }
                    JasmTokenKind::DotEnd => {
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
                        JasmTokenKind::Newline | JasmTokenKind::Eof => {
                            "expected a class name here".to_string()
                        }
                        JasmTokenKind::DotClass
                        | JasmTokenKind::DotSuper
                        | JasmTokenKind::DotMethod
                        | JasmTokenKind::DotCode
                        | JasmTokenKind::DotEnd => "directives cannot be used as names".to_string(),
                        _ => format!("found '{}' instead", token),
                    },
                    IdentifierContext::SuperName => "expected a superclass name".to_string(),
                    IdentifierContext::MethodName => "expected a method name".to_string(),
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
            ParserError::MethodDescriptorExpected(_, token, _) => vec![DiagnosticLabel::at(
                self.get_primary_location(),
                format!("expected a method descriptor, but found '{}'", token),
            )],
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
            ParserError::MultipleDefinitions(context) => match context {
                MultipleDefinitionContext::SuperClass {
                    first_definition,
                    second_definition,
                } => vec![
                    DiagnosticLabel::context(
                        first_definition.identifier_span.as_range(),
                        format!(
                            "superclass was first defined as '{}'",
                            first_definition.class_name
                        ),
                    ),
                    DiagnosticLabel::at(
                        second_definition.directive_span.start
                            ..second_definition.identifier_span.end,
                        format!(
                            "attempted to redefine as '{}'",
                            second_definition.class_name
                        ),
                    ),
                ],
            },
        }
    }

    fn get_note(&self) -> Option<String> {
        match self {
            ParserError::ClassDirectiveExpected(_, token) => match token {
                JasmTokenKind::DotMethod | JasmTokenKind::DotSuper => {
                    Some("Define a class first using '.class [access_flags] <name>'.".to_string())
                }
                JasmTokenKind::DotCode => Some(
                    "The '.code' directive is only valid inside a method definition. Define a method first using '.method [access_flags] <name> <descriptor>'."
                        .to_string(),
                ),
                JasmTokenKind::DotEnd => Some(
                    "The '.end' directive must match a previous '.method', '.code', or '.class' directive.".to_string(),
                ),
                JasmTokenKind::Public | JasmTokenKind::Static => Some(
                    "Access flags like 'public' and 'static' must appear within a '.class' or '.method' directive.".to_string(),
                ),
                JasmTokenKind::Identifier(name) => Some(
                    format!("Found identifier '{}' before any class was defined. Did you forget to start the class? Try: '.class {}'", name, name),
                ),
                JasmTokenKind::Integer(_) => Some(
                    "Integer literals are typically used as instruction arguments inside '.code' blocks.".to_string(),
                ),
                JasmTokenKind::StringLiteral(_) => Some(
                    "String literals are constant values that must appear inside '.code' blocks as instruction arguments.".to_string(),
                ),
                JasmTokenKind::MethodDescriptor(_) => Some(
                    "Method descriptors specify method signatures and must appear after method names in '.method' directives or as instruction arguments.".to_string(),
                ),
                _ => {
                    Some("All assembly code must be placed inside a class definition starting with '.class'.".to_string())
                }
            },
            ParserError::TrailingTokens(tokens, context) => {
                let first_token_kind = &tokens[0].kind;
                match context {
                    TrailingTokensContext::Class => match first_token_kind {
                        JasmTokenKind::DotSuper | JasmTokenKind::DotMethod => {
                            Some(format!("Consider starting a new line for the '{}' directive.", first_token_kind))
                        }
                        JasmTokenKind::DotClass => {
                            Some("The '.class' directive cannot be nested. Consider removing the second '.class' (todo when nested metada data is supported explain it).".to_string())
                        }
                        JasmTokenKind::DotCode => {
                            Some("The '.code' directive must be inside a method definition, not directly after the class name.".to_string())
                        }
                        JasmTokenKind::DotEnd => {
                            Some("The '.end' directive must match a previous '.method', '.code', or '.class' directive. It cannot appear directly after the class name.".to_string())
                        }
                        JasmTokenKind::Public | JasmTokenKind::Static => {
                            Some("Access flags must appear before the class name: '.class [access_flags] <name>'".to_string())
                        }
                        JasmTokenKind::Integer(_) => {
                            Some("Integer literals belong inside '.code' blocks as instruction arguments.".to_string())
                        }
                        JasmTokenKind::StringLiteral(_) => {
                            Some("String literals belong inside '.code' blocks as instruction arguments.".to_string())
                        }
                        JasmTokenKind::Identifier(_) => {
                            Some("The class header should end by the class name. Use directives like '.method' or '.field' on the new line for other members.".to_string())
                        }
                        JasmTokenKind::MethodDescriptor(_) => {
                            Some("Method descriptors must follow method names in '.method' directives.".to_string())
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
                    JasmTokenKind::StringLiteral(_),
                    IdentifierContext::ClassName | IdentifierContext::SuperName,
                ) => Some("Consider removing the quotes around the class name".to_string()),
                (JasmTokenKind::StringLiteral(_), IdentifierContext::MethodName) => {
                    Some("Consider removing the quotes around the method name".to_string())
                }
                (JasmTokenKind::DotClass | JasmTokenKind::DotMethod | JasmTokenKind::DotSuper | JasmTokenKind::DotCode | JasmTokenKind::DotEnd, IdentifierContext::ClassName) => {
                    Some("Directives are reserved keywords. If you meant to start a new directive, do so on a new line.".to_string())
                }
                (JasmTokenKind::Integer(_), IdentifierContext::ClassName) => {
                    Some("Integer literals cannot be used as class names. Every class must have a valid identifier as its name.".to_string())
                }
                (JasmTokenKind::MethodDescriptor(_), IdentifierContext::ClassName) => {
                    Some("Method descriptors cannot be used as class names. Please provide a class name like 'com/example/MyClass'.".to_string())
                }
                (JasmTokenKind::Public | JasmTokenKind::Static, IdentifierContext::ClassName) => {
                    Some(format!("Access flags like '{}' must appear before the class name. Example: '.class {} MyClass'", kind, kind))
                }
                (JasmTokenKind::Newline | JasmTokenKind::Eof, IdentifierContext::ClassName) => {
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
            },
            ParserError::MethodDescriptorExpected(_, _, context) => Some(match context {
                MethodDescriptorContext::MethodDirective => {
                    "Method descriptors specify parameter types and return type. Example: '(II)V' for a method taking two ints and returning void."
                        .to_string()
                }
                MethodDescriptorContext::Instruction => {
                    "This instruction requires a method descriptor to identify the target method signature."
                        .to_string()
                }
            }),
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
            ParserError::MultipleDefinitions(context) => match context {
                MultipleDefinitionContext::SuperClass { .. } => Some(
                    "Java classes do not support multiple inheritance.\nA class can only have one parent.".to_string(),
                ),
            },
        }
    }

    fn get_primary_location(&self) -> Range<usize> {
        match self {
            ParserError::ClassDirectiveExpected(span, _)
            | ParserError::EmptyFile(span)
            | ParserError::IdentifierExpected(span, _, _)
            | ParserError::MethodDescriptorExpected(span, _, _)
            | ParserError::UnexpectedCodeDirectiveArg(span, _)
            | ParserError::NonNegativeIntegerExpected(span, _, _)
            | ParserError::UnknownInstruction(span, _) => span.as_range(),
            ParserError::TrailingTokens(tokens, _) => {
                tokens[0].span.start..tokens.last().map(|v| v.span.end).unwrap_or(0)
            }
            ParserError::MultipleDefinitions(context) => match context {
                MultipleDefinitionContext::SuperClass {
                    second_definition, ..
                } => second_definition.directive_span.as_range(),
            },
            ParserError::Internal(_) => 0..0,
        }
    }
}

impl Diagnostic for ParserError {
    fn message(&self) -> String {
        self.get_message()
    }

    fn primary_location(&self) -> Range<usize> {
        self.get_primary_location()
    }

    fn labels(&self) -> Vec<DiagnosticLabel> {
        self.get_labels()
    }

    fn note(&self) -> Option<String> {
        self.get_note()
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }
}

impl From<ParserError> for JasmError {
    fn from(err: ParserError) -> Self {
        match err {
            ParserError::Internal(msg) => JasmError::Internal(msg),
            _ => JasmError::Diagnostic(Box::new(err)),
        }
    }
}
