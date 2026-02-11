use crate::ast::JasmClass;
use crate::token::{JasmToken, JasmTokenKind, Span};
use common::descriptor::MethodDescriptor;
use std::iter::Peekable;
use std::ops::Range;
use std::vec::IntoIter;

#[derive(Debug, Eq, PartialEq, Clone)]
pub(crate) enum ParserError {
    ClassDirectiveExpected(Span, JasmTokenKind),
    TrailingTokens(Vec<JasmToken>, TrailingTokensContext),
    IdentifierExpected(Span, JasmTokenKind, IdentifierContext),

    MethodDescriptorExpected(Span, JasmTokenKind, MethodDescriptorContext),
    MethodDescriptorClosParenExpected(Span, JasmTokenKind, MethodDescriptorContext),

    UnexpectedCodeDirectiveArg(Span, JasmTokenKind),

    NonNegativeIntegerExpected(Span, JasmTokenKind, NonNegativeIntegerContext),

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
    ClassDirective,
    SuperDirective,
    MethodDirectiveName,
    MethodDirectiveDescriptorParams,
    MethodDirectiveDescriptorRet,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub(crate) enum MethodDescriptorContext {
    MethodDirective,
    Instruction,
}

impl ParserError {
    pub fn message(&self) -> Option<String> {
        match self {
            ParserError::ClassDirectiveExpected(_, token) => {
                Some(format!("unexpected {}", token.as_string_token_type()))
            }
            ParserError::TrailingTokens(_, _) => Some("trailing characters".to_string()),
            ParserError::IdentifierExpected(_, _, context) => Some(
                match context {
                    IdentifierContext::ClassDirective => "incomplete class definition",
                    IdentifierContext::SuperDirective => "incomplete .super directive",
                    IdentifierContext::MethodDirectiveName => "incomplete .method directive",
                    _ => unimplemented!(),
                }
                .to_string(),
            ),
            ParserError::EmptyFile(_) => Some("empty file".to_string()),
            ParserError::Internal(msg) => Some(format!("Internal parser error: {}", msg)),
            _ => unimplemented!(),
        }
    }

    pub fn label(&self) -> Option<String> {
        match self {
            ParserError::TrailingTokens(_, _) => {
                Some("Class headers must end after the name.".to_string())
            }
            ParserError::ClassDirectiveExpected(_, token) => Some(format!(
                "The '{}' {} cannot appear before a class is defined.",
                token,
                token.as_string_token_type()
            )),
            ParserError::IdentifierExpected(_, _, context) => {
                Some(
                    match context {
                        IdentifierContext::ClassDirective =>
                            "The .class directive requires a class name after the access flags.",
                        IdentifierContext::SuperDirective =>
                            "The .super directive requires a superclass name.",
                        IdentifierContext::MethodDirectiveName =>
                            "The .method directive requires a method name followed by parentheses and a method descriptor.",
                        _ => unimplemented!()
                    }.to_string())
            }
            ParserError::EmptyFile(_) => Some("The file contains no class definition.".to_string()),
            ParserError::Internal(_) => None,
                _ => unimplemented!()
        }
    }

    pub fn note(&self) -> Option<String> {
        match self {
            ParserError::ClassDirectiveExpected(_, _) => {
                //TODO: actually it is false. I guess the source file name or class file version could be added to the note when implemented
                Some("A Java assembly file must start with a '.class' definition.".to_string())
            }
            ParserError::TrailingTokens(tokens, context) => Some(format!(
                "The class definition should end after the class name.\n{}",
                match (tokens[0].kind.clone(), context) {
                    (JasmTokenKind::DotSuper, TrailingTokensContext::Class) =>
                        "Consider starting a new line for the '.super' directive.",
                    (JasmTokenKind::OpenParen, TrailingTokensContext::Class) =>
                        "If you're trying to define a method, use the '.method' directive instead.",
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
                (
                    JasmTokenKind::StringLiteral(_),
                    IdentifierContext::ClassDirective | IdentifierContext::SuperDirective,
                ) => Some("Consider removing the quotes around the class name".to_string()),
                (JasmTokenKind::StringLiteral(_), IdentifierContext::MethodDirectiveName) => {
                    Some("Consider removing the quotes around the method name".to_string())
                }
                _ => Some(
                    "The .class directive requires a name:\n.class [access_flags] <name>"
                        .to_string(),
                ),
            },
            ParserError::Internal(_) | ParserError::EmptyFile(_) => None,
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

    fn parse_super_directive(&mut self) -> Result<(), ParserError> {
        let dot_super = self.next_token()?; // consume .super token
        let super_name =
            self.expect_next_identifier(IdentifierContext::SuperDirective, dot_super.span.end)?;
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

    fn parse_code_directive(&mut self) -> Result<(), ParserError> {
        // TODO: Do I need "already defined" checks for stack and locals?
        let mut stack = None;
        let mut locals = None;
        let dot_code = self.next_token()?; // consume .code token

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

        unimplemented!("Parsing of code instructions is not implemented yet");
    }

    fn parse_method_descriptor(
        &mut self,
        context: MethodDescriptorContext,
    ) -> Result<(Option<String>, String), ParserError> {
        let open_paren_token = self.next_token()?;
        if !matches!(open_paren_token.kind, JasmTokenKind::OpenParen) {
            return Err(ParserError::MethodDescriptorExpected(
                open_paren_token.span,
                open_paren_token.kind,
                context,
            ));
        }

        let params_str = if matches!(self.peek_token_kind(), Some(JasmTokenKind::CloseParen)) {
            None
        } else {
            Some(self.expect_next_identifier(
                IdentifierContext::MethodDirectiveDescriptorParams, // TODO: hardcoded
                open_paren_token.span.end,
            )?)
        };

        let close_paren_token = self.next_token()?;
        if !matches!(close_paren_token.kind, JasmTokenKind::CloseParen) {
            return Err(ParserError::MethodDescriptorClosParenExpected(
                close_paren_token.span,
                close_paren_token.kind,
                context,
            ));
        }

        let ret_str = self.expect_next_identifier(
            IdentifierContext::MethodDirectiveDescriptorRet, // TODO: hardcoded
            close_paren_token.span.end,
        )?;

        Ok((params_str, ret_str))
    }

    fn parse_method(&mut self) -> Result<(), ParserError> {
        let dot_method = self.next_token()?; // consume .method token
        let _access_flags = self.parse_method_access_flags()?;
        let method_name = self
            .expect_next_identifier(IdentifierContext::MethodDirectiveName, dot_method.span.end)?;
        let method_descriptor =
            self.parse_method_descriptor(MethodDescriptorContext::MethodDirective)?;
        self.expect_no_trailing_tokens(TrailingTokensContext::Method)?;
        self.skip_newlines()?;
        self.parse_code_directive()?;
        todo!()
    }

    fn parse_class(&mut self) -> Result<JasmClass, ParserError> {
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
            self.expect_next_identifier(IdentifierContext::ClassDirective, self.last_span.end)?;

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

        todo!()
    }

    pub fn parse(tokens: Vec<JasmToken>) -> Result<JasmClass, ParserError> {
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
