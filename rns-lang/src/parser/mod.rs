use crate::instruction::{INSTRUCTION_SPECS, InstructionArgKind};
use crate::token::{JasmToken, JasmTokenKind, Span};
use std::iter::Peekable;
use std::ops::Range;
use std::vec::IntoIter;

#[derive(Debug, Eq, PartialEq, Clone)]
pub(crate) enum ParserError {
    ClassDirectiveExpected(Span, JasmTokenKind),
    TrailingTokens(Vec<JasmToken>, TrailingTokensContext),
    IdentifierExpected(Span, JasmTokenKind, IdentifierContext),

    MethodDescriptorExpected(Span, JasmTokenKind, MethodDescriptorContext),

    UnexpectedCodeDirectiveArg(Span, JasmTokenKind),

    NonNegativeIntegerExpected(Span, JasmTokenKind, NonNegativeIntegerContext),

    UnknownInstruction(Span, String),

    EmptyFile(Span),
    Internal(String),
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub(crate) enum NonNegativeIntegerContext {
    CodeLocals,
    CodeStack,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub(crate) enum TrailingTokensContext {
    Class,
    Super,
    Method,
    Code,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub(crate) enum IdentifierContext {
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
pub(crate) enum MethodDescriptorContext {
    MethodDirective,
    Instruction,
}

impl ParserError {
    pub fn message(&self) -> Option<String> {
        match self {
            ParserError::ClassDirectiveExpected(_, token) => Some(format!(
                "Unexpected {} before class definition",
                token.as_string_token_type()
            )),
            ParserError::TrailingTokens(_, _) => Some("trailing characters".to_string()),
            ParserError::IdentifierExpected(_, token, context) => Some(match context {
                IdentifierContext::ClassName => format!(
                    "Expected class name but found {}",
                    token.as_string_token_type()
                ),
                IdentifierContext::SuperName => "incomplete .super directive".to_string(),
                IdentifierContext::MethodName => "incomplete .method directive".to_string(),
                _ => unimplemented!(),
            }),
            ParserError::EmptyFile(_) => Some("File contains no class definition".to_string()),
            ParserError::Internal(msg) => Some(format!("Internal parser error: {}", msg)),
            _ => unimplemented!(),
        }
    }

    pub fn label(&self) -> Option<String> {
        match self {
            ParserError::TrailingTokens(_, _) => {
                Some("Class headers must end after the name.".to_string())
            }
            ParserError::ClassDirectiveExpected(_, token) => match token {
                JasmTokenKind::DotMethod | JasmTokenKind::DotSuper => Some(format!(
                    "The '{}' directive is only allowed inside a class definition.",
                    token
                )),
                JasmTokenKind::DotCode => Some(format!(
                    "The '{}' directive is only allowed inside a method definition.",
                    token
                )),
                JasmTokenKind::DotEnd => Some(format!(
                    "The '{}' directive has no matching start directive.",
                    token
                )),
                _ => Some(format!(
                    "The '{}' {} must appear inside a class definition.",
                    token,
                    token.as_string_token_type()
                )),
            },
            ParserError::IdentifierExpected(_, token, context) => {
                Some(
                    match context {
                        IdentifierContext::ClassName =>
                            format!("Found '{}' instead of a class name. The .class directive requires a class name after optional access flags.", token),
                        IdentifierContext::SuperName =>
                            "The .super directive requires a superclass name.".to_string(),
                        IdentifierContext::MethodName =>
                            "The .method directive requires a method name followed by parentheses and a method descriptor.".to_string(),
                        _ => unimplemented!()
                    })
            }
            ParserError::Internal(_) => None,
            ParserError::EmptyFile(_) => Some("The file is empty or contains only comments.".to_string()),
                _ => unimplemented!()
        }
    }

    pub fn note(&self) -> Option<String> {
        match self {
            ParserError::ClassDirectiveExpected(_, token) => match token {
                JasmTokenKind::DotMethod | JasmTokenKind::DotSuper => {
                    Some("Define a class first using '.class [access_flags] <name>'.".to_string())
                }
                JasmTokenKind::DotCode => Some(
                    "Define a method first using '.method [access_flags] <name> <descriptor>'."
                        .to_string(),
                ),
                JasmTokenKind::DotEnd => Some(
                    "The '.end' directive must match a previous '.method', '.code', or '.class' directive.".to_string(),
                ),
                JasmTokenKind::Public | JasmTokenKind::Static => Some(
                    "Keywords like 'public' and 'static' are access modifiers that must appear within '.class' or '.method' directives.".to_string(),
                ),
                JasmTokenKind::Identifier(_) => Some(
                    "Identifiers (class, method, or field names) must be used within appropriate directives like '.class', '.method', or field references.".to_string(),
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
            ParserError::TrailingTokens(tokens, context) => Some(format!(
                "The class definition should end after the class name.\n{}",
                match (tokens[0].kind.clone(), context) {
                    (JasmTokenKind::DotSuper, TrailingTokensContext::Class) =>
                        "Consider starting a new line for the '.super' directive.",
                    (JasmTokenKind::DotMethod, TrailingTokensContext::Class) =>
                        "Consider starting a new line for the '.method' directive.",
                    (
                        JasmTokenKind::Public | JasmTokenKind::Static,
                        TrailingTokensContext::Class,
                    ) =>
                        "Access flags must appear before the class name:\n.class [access_flags] <name>",
                    _ =>
                        "Unexpected tokens after class name. Consider starting a new line for the next directive.",
                }
            )),
            ParserError::IdentifierExpected(_, kind, context) => match (kind, context) {
                // String literal cases
                (
                    JasmTokenKind::StringLiteral(_),
                    IdentifierContext::ClassName | IdentifierContext::SuperName,
                ) => Some("Consider removing the quotes around the class name".to_string()),
                (JasmTokenKind::StringLiteral(_), IdentifierContext::MethodName) => {
                    Some("Consider removing the quotes around the method name".to_string())
                }
                // Class name specific guidance
                (JasmTokenKind::DotClass | JasmTokenKind::DotMethod | JasmTokenKind::DotSuper | JasmTokenKind::DotCode | JasmTokenKind::DotEnd, IdentifierContext::ClassName) => {
                    Some(format!("Directives like '{}' cannot be used as class names.", kind))
                }
                (JasmTokenKind::Integer(_), IdentifierContext::ClassName) => {
                    Some("Integer literals cannot be used as class names.".to_string())
                }
                (JasmTokenKind::MethodDescriptor(_), IdentifierContext::ClassName) => {
                    Some("Method descriptors cannot be used as class names.".to_string())
                }
                (JasmTokenKind::Public | JasmTokenKind::Static, IdentifierContext::ClassName) => {
                    Some(format!("Keywords like '{}' are access flags and must appear before the class name.", kind))
                }
                (JasmTokenKind::Newline | JasmTokenKind::Eof, IdentifierContext::ClassName) => {
                    Some("Class name is missing after optional access flags.".to_string())
                }
                // Generic fallback
                (_, IdentifierContext::ClassName) => Some(
                    "The .class directive requires a valid Java class name:\n.class [access_flags] <name>"
                        .to_string(),
                ),
                // Keep existing for other contexts
                (_, IdentifierContext::SuperName) => Some(
                    "The .super directive requires a superclass name.".to_string(),
                ),
                (_, IdentifierContext::MethodName) => Some(
                    "The .method directive requires a method name followed by parentheses and a method descriptor.".to_string(),
                ),
                _ => unimplemented!(),
            },
            ParserError::EmptyFile(_) => Some("A Java assembly file must start with a '.class' directive.".to_string()),
            ParserError::Internal(_) => None,
            _ => unimplemented!(),
        }
    }

    pub fn as_range(&self) -> Option<Range<usize>> {
        self.span().map(|s| s.as_range())
    }

    fn span(&self) -> Option<Span> {
        match self {
            ParserError::ClassDirectiveExpected(span, _)
            | ParserError::EmptyFile(span)
            | ParserError::IdentifierExpected(span, _, _) => Some(*span),
            ParserError::TrailingTokens(tokens, _) => Some(Span::new(
                tokens[0].span.start,
                tokens.last().map(|v| v.span.end).unwrap_or(0),
            )),
            ParserError::Internal(_) => None,
            _ => unimplemented!(),
        }
    }
}

pub struct JasmParser {
    tokens: Peekable<IntoIter<JasmToken>>,
    last_span: Span,
}

impl JasmParser {
    fn peek_token_kind(&mut self) -> Option<&JasmTokenKind> {
        self.tokens.peek().map(|token| &token.kind)
    }

    fn skip_newlines(&mut self) -> Result<(), ParserError> {
        while let Some(JasmTokenKind::Newline) = self.peek_token_kind() {
            self.next_token()?;
        }
        Ok(())
    }

    fn next_token(&mut self) -> Result<JasmToken, ParserError> {
        match self.tokens.next() {
            Some(token) => {
                self.last_span = token.span;
                Ok(token)
            }
            None => Err(ParserError::Internal(
                "Token stream ended before EOF token".to_string(),
            )),
        }
    }

    fn next_until_newline(&mut self) -> Result<Vec<JasmToken>, ParserError> {
        let mut tokens = Vec::new();
        while let Some(token) = self.tokens.peek() {
            if matches!(token.kind, JasmTokenKind::Newline | JasmTokenKind::Eof) {
                break;
            }
            tokens.push(self.next_token()?);
        }
        Ok(tokens)
    }

    //TODO: add all flags, handle orders, and check for duplicates
    fn parse_class_access_flags(&mut self) -> Result<u16, ParserError> {
        let mut flags = 0u16;
        loop {
            match self.peek_token_kind() {
                Some(JasmTokenKind::Public) => {
                    flags |= 0x0001; // ACC_PUBLIC
                    self.next_token()?;
                }
                Some(JasmTokenKind::Static) => {
                    flags |= 0x0008; // ACC_STATIC
                    self.next_token()?;
                }
                _ => break,
            }
        }
        Ok(flags)
    }

    //TODO: add all flags, handle orders, and check for duplicates
    fn parse_method_access_flags(&mut self) -> Result<u16, ParserError> {
        let mut flags = 0u16;
        loop {
            match self.peek_token_kind() {
                Some(JasmTokenKind::Public) => {
                    flags |= 0x0001; // ACC_PUBLIC
                    self.next_token()?;
                }
                Some(JasmTokenKind::Static) => {
                    flags |= 0x0008; // ACC_STATIC
                    self.next_token()?;
                }
                _ => break,
            }
        }
        Ok(flags)
    }

    fn expect_next_identifier(
        &mut self,
        context: IdentifierContext,
        prev_token_end: usize,
    ) -> Result<String, ParserError> {
        let token = self.next_token()?;
        match token.kind {
            JasmTokenKind::Identifier(name) => Ok(name),
            JasmTokenKind::Eof | JasmTokenKind::Newline => Err(ParserError::IdentifierExpected(
                Span::new(prev_token_end, prev_token_end),
                token.kind,
                context,
            )),
            _ => Err(ParserError::IdentifierExpected(
                token.span, token.kind, context,
            )),
        }
    }

    fn expect_next_method_descriptor(
        &mut self,
        context: MethodDescriptorContext,
        prev_token_end: usize,
    ) -> Result<String, ParserError> {
        let token = self.next_token()?;
        match token.kind {
            JasmTokenKind::MethodDescriptor(name) => Ok(name),
            JasmTokenKind::Eof | JasmTokenKind::Newline => {
                Err(ParserError::MethodDescriptorExpected(
                    Span::new(prev_token_end, prev_token_end),
                    token.kind,
                    context,
                ))
            }
            _ => Err(ParserError::MethodDescriptorExpected(
                token.span, token.kind, context,
            )),
        }
    }

    fn parse_super_directive(&mut self) -> Result<(), ParserError> {
        let dot_super = self.next_token()?; // consume .super token
        let super_name =
            self.expect_next_identifier(IdentifierContext::SuperName, dot_super.span.end)?;
        self.expect_no_trailing_tokens(TrailingTokensContext::Super)
    }

    fn expect_no_trailing_tokens(
        &mut self,
        context: TrailingTokensContext,
    ) -> Result<(), ParserError> {
        let trailing_tokens = self.next_until_newline()?;
        if !trailing_tokens.is_empty() {
            return Err(ParserError::TrailingTokens(trailing_tokens, context));
        }
        Ok(())
    }

    fn expect_next_non_negative_integer(
        &mut self,
        context: NonNegativeIntegerContext,
        prev_token_end: usize,
    ) -> Result<u32, ParserError> {
        let token = self.next_token()?;
        match token.kind {
            JasmTokenKind::Integer(value) if value >= 0 => Ok(value as u32),
            JasmTokenKind::Eof | JasmTokenKind::Newline => {
                Err(ParserError::NonNegativeIntegerExpected(
                    Span::new(prev_token_end, prev_token_end),
                    token.kind,
                    context,
                ))
            }
            _ => Err(ParserError::NonNegativeIntegerExpected(
                token.span, token.kind, context,
            )),
        }
    }

    fn parse_instruction(&mut self) -> Result<(), ParserError> {
        let instruction_name =
            self.expect_next_identifier(IdentifierContext::InstructionName, self.last_span.end)?;
        let instruction_pos = self.last_span;
        let instruction_spec = INSTRUCTION_SPECS
            .get(instruction_name.as_str())
            .ok_or_else(|| {
                ParserError::UnknownInstruction(instruction_pos, instruction_name.clone())
            })?;
        for arg_spec in instruction_spec.args {
            match arg_spec {
                InstructionArgKind::ClassName => self.expect_next_identifier(
                    IdentifierContext::ClassNameInstructionArg,
                    instruction_pos.end,
                )?,
                InstructionArgKind::MethodName => self.expect_next_identifier(
                    IdentifierContext::MethodNameInstructionArg,
                    instruction_pos.end,
                )?,
                InstructionArgKind::MethodDescriptor => self.expect_next_method_descriptor(
                    MethodDescriptorContext::Instruction,
                    instruction_pos.end,
                )?,
                InstructionArgKind::StringLiteral => {
                    // TODO: stub for ldc
                    let token = self.next_token()?;
                    match token.kind {
                        JasmTokenKind::StringLiteral(value) => value,
                        _ => {
                            return Err(ParserError::IdentifierExpected(
                                token.span,
                                token.kind,
                                IdentifierContext::InstructionName,
                            ));
                        }
                    }
                }
                InstructionArgKind::FieldName => self.expect_next_identifier(
                    IdentifierContext::FieldNameInstructionArg,
                    instruction_pos.end,
                )?,
                InstructionArgKind::FieldDescriptor => self.expect_next_identifier(
                    IdentifierContext::FieldDescriptorInstructionArg,
                    instruction_pos.end,
                )?,
            };
        }
        Ok(())
    }

    fn parse_code_directive(&mut self) -> Result<(), ParserError> {
        // TODO: Do I need "already defined" checks for stack and locals?
        let mut stack = None;
        let mut locals = None;
        self.next_token()?; // consume .code token

        while let Some(JasmTokenKind::Identifier(_)) = self.peek_token_kind() {
            let identifier_token = self.next_token()?;
            match identifier_token.kind {
                JasmTokenKind::Identifier(ref name) if name == "stack" => {
                    stack = Some(self.expect_next_non_negative_integer(
                        NonNegativeIntegerContext::CodeStack,
                        identifier_token.span.end,
                    )?);
                }
                JasmTokenKind::Identifier(ref name) if name == "locals" => {
                    locals = Some(self.expect_next_non_negative_integer(
                        NonNegativeIntegerContext::CodeLocals,
                        identifier_token.span.end,
                    )?);
                }
                JasmTokenKind::Identifier(_) => Err(ParserError::UnexpectedCodeDirectiveArg(
                    identifier_token.span,
                    identifier_token.kind,
                ))?,
                _ => unreachable!(),
            }
        }

        self.expect_no_trailing_tokens(TrailingTokensContext::Code)?;
        self.skip_newlines()?;

        while let Some(token) = self.tokens.peek() {
            if matches!(token.kind, JasmTokenKind::DotEnd | JasmTokenKind::Eof) {
                break;
            }
            self.parse_instruction()?;
            self.skip_newlines()?
        }

        // TODO: move end check with new enum(with all possible end directives) to a separate function and use it for method and class end checks as well
        let next_token = self.next_token()?;
        if !matches!(next_token.kind, JasmTokenKind::DotEnd) {
            return Err(ParserError::Internal(format!(
                "Expected .end after code block, found {}",
                next_token.kind.as_string_token_type()
            )));
        }
        let next_token = self.next_token()?;
        if !matches!(next_token.kind, JasmTokenKind::Identifier(ref s) if s == "code") {
            return Err(ParserError::Internal(format!(
                "Expected .end code after code block, found {}",
                next_token.kind.as_string_token_type()
            )));
        }
        // TODO: assert no more tokens on the line after .end code
        self.skip_newlines()?;

        Ok(())
    }

    fn parse_method(&mut self) -> Result<(), ParserError> {
        let dot_method = self.next_token()?; // consume .method token
        let _access_flags = self.parse_method_access_flags()?;
        let method_name =
            self.expect_next_identifier(IdentifierContext::MethodName, dot_method.span.end)?;
        let method_descriptor = self.expect_next_method_descriptor(
            MethodDescriptorContext::MethodDirective,
            self.last_span.end,
        )?;
        self.expect_no_trailing_tokens(TrailingTokensContext::Method)?;
        self.skip_newlines()?;
        self.parse_code_directive()?;
        self.skip_newlines()?;

        // TODO: move end check with new enum(with all possible end directives) to a separate function and use it for method and class end checks as well
        let next_token = self.next_token()?;
        if !matches!(next_token.kind, JasmTokenKind::DotEnd) {
            return Err(ParserError::Internal(format!(
                "Expected .end after method body, found {}",
                next_token.kind.as_string_token_type()
            )));
        }

        let next_token = self.next_token()?;
        if !matches!(next_token.kind, JasmTokenKind::Identifier(ref s) if s == "method") {
            return Err(ParserError::Internal(format!(
                "Expected .end method after method body, found {}",
                next_token.kind.as_string_token_type()
            )));
        }

        // assert no more tokens on the line after .end method
        self.skip_newlines()?;
        Ok(())
    }

    fn parse_class(&mut self) -> Result<(), ParserError> {
        self.skip_newlines()?;
        let class_token = self.next_token()?;
        if matches!(class_token.kind, JasmTokenKind::Eof) {
            return Err(ParserError::EmptyFile(class_token.span));
        }
        if !matches!(class_token.kind, JasmTokenKind::DotClass) {
            return Err(ParserError::ClassDirectiveExpected(
                class_token.span,
                class_token.kind,
            ));
        }
        let _access_flags = self.parse_class_access_flags()?;

        let class_name =
            self.expect_next_identifier(IdentifierContext::ClassName, self.last_span.end)?;

        // TODO: test EOF right after class name and check for correct span in error
        self.expect_no_trailing_tokens(TrailingTokensContext::Class)?;

        while let Some(token) = self.tokens.peek() {
            match token.kind {
                JasmTokenKind::Newline => {
                    self.next_token()?;
                }
                JasmTokenKind::DotMethod => self.parse_method()?,
                JasmTokenKind::DotSuper => self.parse_super_directive()?,
                JasmTokenKind::DotEnd => todo!(), // TODO: check for .end class and break
                JasmTokenKind::Eof => break,
                _ => todo!("Unexpected token in class body"),
            }
        }

        Ok(())
    }

    pub fn parse(tokens: Vec<JasmToken>) -> Result<(), ParserError> {
        if !matches!(tokens.last().unwrap().kind, JasmTokenKind::Eof) {
            return Err(ParserError::Internal(
                "Token stream must end with an EOF token".to_string(),
            ));
        }

        let mut instance = Self {
            tokens: tokens.into_iter().peekable(),
            last_span: Span::new(0, 0),
        };

        instance.parse_class()
    }
}

#[cfg(test)]
mod tests;
