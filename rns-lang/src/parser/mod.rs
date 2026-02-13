use crate::error::JasmError;
use crate::instruction::{INSTRUCTION_SPECS, InstructionArgKind};
use crate::parser::error::{
    IdentifierContext, MethodDescriptorContext, NonNegativeIntegerContext, ParserError,
    TrailingTokensContext,
};
use crate::token::{JasmToken, JasmTokenKind, Span};
use crate::warning::JasmWarning;
use std::iter::Peekable;
use std::vec::IntoIter;

mod error;
#[cfg(test)]
mod tests;

const JAVA_LANG_OBJECT: &str = "java/lang/Object";

pub struct JasmParser {
    tokens: Peekable<IntoIter<JasmToken>>,
    last_span: Span,
    warnings: Vec<JasmWarning>,

    super_name: Vec<SuperName>,
}

struct SuperName {
    pub name: String,
    pub directive_span: Span,
    pub identifier_span: Span,
}

impl JasmParser {
    fn set_super_name(&mut self, super_name: SuperName) {
        self.super_name.push(super_name);
    }

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
    ) -> Result<(String, Span), ParserError> {
        let token = self.next_token()?;
        match token.kind {
            JasmTokenKind::Identifier(name) => Ok((name, token.span)),
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
        let (super_name, super_name_span) =
            self.expect_next_identifier(IdentifierContext::SuperName, dot_super.span.end)?;
        self.set_super_name(SuperName {
            name: super_name,
            directive_span: dot_super.span,
            identifier_span: super_name_span,
        });
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
        let (instruction_name, _) =
            self.expect_next_identifier(IdentifierContext::InstructionName, self.last_span.end)?;
        let instruction_pos = self.last_span;
        let instruction_spec = INSTRUCTION_SPECS
            .get(instruction_name.as_str())
            .ok_or_else(|| {
                ParserError::UnknownInstruction(instruction_pos, instruction_name.clone())
            })?;
        for arg_spec in instruction_spec.args {
            match arg_spec {
                InstructionArgKind::ClassName => {
                    self.expect_next_identifier(
                        IdentifierContext::ClassNameInstructionArg,
                        instruction_pos.end,
                    )?;
                }
                InstructionArgKind::MethodName => {
                    self.expect_next_identifier(
                        IdentifierContext::MethodNameInstructionArg,
                        instruction_pos.end,
                    )?;
                }
                InstructionArgKind::MethodDescriptor => {
                    self.expect_next_method_descriptor(
                        MethodDescriptorContext::Instruction,
                        instruction_pos.end,
                    )?;
                }
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
                    };
                }
                InstructionArgKind::FieldName => {
                    self.expect_next_identifier(
                        IdentifierContext::FieldNameInstructionArg,
                        instruction_pos.end,
                    )?;
                }
                InstructionArgKind::FieldDescriptor => {
                    self.expect_next_identifier(
                        IdentifierContext::FieldDescriptorInstructionArg,
                        instruction_pos.end,
                    )?;
                }
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

        let (class_name, _) =
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

    pub fn parse(tokens: Vec<JasmToken>) -> Result<Vec<JasmWarning>, JasmError> {
        if !matches!(tokens.last().unwrap().kind, JasmTokenKind::Eof) {
            return Err(ParserError::Internal(
                "Token stream must end with an EOF token".to_string(),
            )
            .into());
        }

        let mut instance = Self {
            tokens: tokens.into_iter().peekable(),
            last_span: Span::new(0, 0),
            warnings: Vec::new(),
            super_name: Vec::new(),
        };

        instance.parse_class()?;
        instance.build_jasm_class()?;
        Ok(instance.warnings)
    }

    fn build_jasm_class(&mut self) -> Result<(), ParserError> {
        let super_name = {
            if self.super_name.is_empty() {
                JAVA_LANG_OBJECT.to_string()
            } else if self.super_name.len() == 1 {
                self.super_name[0].name.clone()
            } else {
                let definitions_count = self.super_name.len();
                let message = "Multiple .super directives found".to_string();
                let primary_location = self.super_name[0].directive_span.as_range();
                let taken_super_name = &self.super_name[definitions_count - 1];
                let labels = self
                    .super_name
                    .iter()
                    .map(|v| {
                        (
                            v.directive_span.start..v.identifier_span.end,
                            "Defined here".to_string(),
                        )
                    })
                    .collect::<Vec<_>>();

                let note = format!(
                    "The last .super directive will be used: '{}'",
                    taken_super_name.name
                );
                self.warnings
                    .push(JasmWarning::new(message, primary_location, labels, note));

                taken_super_name.name.clone()
            }
        };
        Ok(())
    }
}
