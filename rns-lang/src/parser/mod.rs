use crate::diagnostic::{Diagnostic, JasmError};
use crate::instruction::{InstructionOperand, INSTRUCTION_SPECS};
use crate::parser::error::{
    IdentifierContext, MultipleDefinitionContext, NonNegativeIntegerContext, ParserError,
    TrailingTokensContext,
};
use crate::parser::jasm_warning::ParserWarning;
use crate::token::{JasmToken, JasmTokenKind, Span};
use jclass::prelude::*;
use std::iter::Peekable;
use std::vec::IntoIter;

mod error;
mod jasm_warning;
mod jvm_warning;
#[cfg(test)]
mod tests;

const JAVA_LANG_OBJECT: &str = "java/lang/Object";

pub struct JasmParser {
    tokens: Peekable<IntoIter<JasmToken>>,
    last_span: Span,

    warnings: Vec<Box<dyn Diagnostic>>,

    cp_builder: ConstantPoolBuilder,
    methods: Vec<MethodInfo>,
    name: String,
    class_directive_pos: Span,
    class_flags: ClassFlags,
    super_name: Option<SuperDirective>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
struct ClassDirective {
    pub class_name: String,
    pub span: Span,
}

#[derive(Debug, Eq, PartialEq, Clone)]
struct SuperDirective {
    pub class_name: String,
    pub directive_span: Span,
    pub identifier_span: Span,
}

impl JasmParser {
    fn set_super_name(&mut self, dir: SuperDirective) -> Result<(), ParserError> {
        if let Some(exiting) = self.super_name.take() {
            if exiting.class_name == dir.class_name {
                self.warnings
                    .push(Box::new(ParserWarning::SameSuperDefinedMultipleTimes {
                        first_occurrence: exiting,
                        second_occurrence: dir.clone(),
                    }));
            } else {
                return Err(ParserError::MultipleDefinitions(
                    MultipleDefinitionContext::SuperClass {
                        first_definition: exiting,
                        second_definition: dir,
                    },
                ));
            }
        }
        self.super_name = Some(dir);
        Ok(())
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
    fn parse_class_access_flags(&mut self) -> Result<(), ParserError> {
        loop {
            match self.peek_token_kind() {
                Some(JasmTokenKind::Public) => {
                    self.class_flags.set_public();
                    self.next_token()?;
                }
                _ => break,
            }
        }
        Ok(())
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

    fn parse_super_directive(&mut self) -> Result<(), ParserError> {
        let dot_super = self.next_token()?; // consume .super token
        let (super_name, super_name_span) =
            self.expect_next_identifier(IdentifierContext::SuperName, dot_super.span.end)?;
        self.set_super_name(SuperDirective {
            class_name: super_name,
            directive_span: dot_super.span,
            identifier_span: super_name_span,
        })?;
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

    // TODO: this is wrong.. also need to review integer token.. it is i32
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

    fn parse_instruction(&mut self, code: &mut Vec<u8>) -> Result<(), ParserError> {
        let (instruction_name, _) =
            self.expect_next_identifier(IdentifierContext::InstructionName, self.last_span.end)?;
        let instruction_pos = self.last_span;
        let instruction_spec = INSTRUCTION_SPECS
            .get(instruction_name.as_str())
            .ok_or_else(|| {
                ParserError::UnknownInstruction(instruction_pos, instruction_name.clone())
            })?;
        code.push(instruction_spec.opcode as u8);
        match instruction_spec.operand {
            InstructionOperand::None => {}
            InstructionOperand::MethodRef => {
                let (class_name, _) = self.expect_next_identifier(
                    IdentifierContext::ClassNameInstructionArg,
                    instruction_pos.end,
                )?;
                let (method_name, _) = self.expect_next_identifier(
                    IdentifierContext::MethodNameInstructionArg,
                    self.last_span.end,
                )?;
                todo!("Method descriptor is deleted")
                /*
                let method_descriptor = self.expect_next_method_descriptor(
                    MethodDescriptorContext::Instruction,
                    self.last_span.end,
                )?;
                let idx =
                    self.cp_builder
                        .add_methodref(&class_name, &method_name, &method_descriptor);
                code.extend_from_slice(&idx.to_be_bytes());
                 */
            }
            InstructionOperand::FieldRef => {
                let (class_name, _) = self.expect_next_identifier(
                    IdentifierContext::ClassNameInstructionArg,
                    instruction_pos.end,
                )?;
                let (field_name, _) = self.expect_next_identifier(
                    IdentifierContext::FieldNameInstructionArg,
                    self.last_span.end,
                )?;
                let (field_descriptor, _) = self.expect_next_identifier(
                    IdentifierContext::FieldDescriptorInstructionArg,
                    self.last_span.end,
                )?;
                let idx = self
                    .cp_builder
                    .add_fieldref(&class_name, &field_name, &field_descriptor);
                code.extend_from_slice(&idx.to_be_bytes());
            }
            // TODO: it is still stub here for ldc, need to handle properly
            InstructionOperand::StringLiteral => {
                let token = self.next_token()?;
                let value = match token.kind {
                    JasmTokenKind::StringLiteral(value) => value,
                    _ => {
                        // TODO: error is wrong
                        return Err(ParserError::IdentifierExpected(
                            token.span,
                            token.kind,
                            IdentifierContext::InstructionName,
                        ));
                    }
                };
                let idx = self.cp_builder.add_string(&value);
                code.push(idx as u8);
            }
        }
        Ok(())
    }

    fn parse_code_directive(&mut self) -> Result<CodeAttribute, ParserError> {
        // TODO: Do I need "already defined" checks for stack and locals?
        let mut stack = None;
        let mut locals = None;
        self.next_token()?; // consume .code token
        let mut code = Vec::with_capacity(16); // TODO: find a better initial capacity

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
            self.parse_instruction(&mut code)?;
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

        Ok(CodeAttribute {
            max_stack: stack.unwrap() as u16, // TODO: proper error and u16 conversion check
            max_locals: locals.unwrap() as u16, // TODO: proper error and u16 conversion check
            code,
            exception_table: vec![],
            attributes: vec![],
        })
    }

    fn parse_method(&mut self) -> Result<MethodInfo, ParserError> {
        let dot_method = self.next_token()?; // consume .method token
        let access_flags = self.parse_method_access_flags()?;
        let (method_name, _) =
            self.expect_next_identifier(IdentifierContext::MethodName, dot_method.span.end)?;
        let (method_descriptor, _) =
            self.expect_next_identifier(IdentifierContext::MethodDescriptor, self.last_span.end)?;
        self.expect_no_trailing_tokens(TrailingTokensContext::Method)?;
        self.skip_newlines()?;
        let code_attr = self.parse_code_directive()?;
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
        Ok(MethodInfo {
            access_flags: MethodFlags::new(access_flags),
            name_index: self.cp_builder.add_utf8(&method_name),
            descriptor_index: self.cp_builder.add_utf8(&method_descriptor),
            attributes: vec![MethodAttribute::Code(code_attr)],
        })
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
        self.class_directive_pos = class_token.span;
        self.parse_class_access_flags()?;

        let (class_name, _) =
            self.expect_next_identifier(IdentifierContext::ClassName, self.last_span.end)?;
        self.name = class_name;

        // TODO: test EOF right after class name and check for correct span in error
        self.expect_no_trailing_tokens(TrailingTokensContext::Class)?;

        while let Some(token) = self.tokens.peek() {
            match token.kind {
                JasmTokenKind::Newline => {
                    self.next_token()?;
                }
                JasmTokenKind::DotMethod => {
                    let method = self.parse_method()?;
                    self.methods.push(method);
                }
                JasmTokenKind::DotSuper => self.parse_super_directive()?,
                JasmTokenKind::DotEnd => {
                    self.next_token()?; // consume .end
                    if let Some(token) = self.tokens.peek() {
                        if let JasmTokenKind::Identifier(ref s) = token.kind {
                            if s == "class" {
                                self.next_token()?; // consume "class"
                                break; // .end class - finish parsing
                            }
                        }
                    }
                }
                JasmTokenKind::Eof => break,
                _ => {
                    eprintln!("DEBUG: Unexpected token in class body: {:?}", token.kind);
                    todo!("Unexpected token in class body")
                }
            }
        }

        Ok(())
    }

    pub fn parse(
        tokens: Vec<JasmToken>,
    ) -> (Vec<Box<dyn Diagnostic>>, Result<ClassFile, JasmError>) {
        if !matches!(tokens.last().unwrap().kind, JasmTokenKind::Eof) {
            return (
                vec![],
                Err(
                    ParserError::Internal("Token stream must end with an EOF token".to_string())
                        .into(),
                ),
            );
        }

        let mut instance = Self {
            tokens: tokens.into_iter().peekable(),
            last_span: Span::new(0, 0),
            warnings: Vec::new(),
            super_name: None,
            name: String::new(),
            class_directive_pos: Span::new(0, 0),
            cp_builder: ConstantPoolBuilder::new(),
            methods: Vec::new(),
            class_flags: ClassFlags::new(0),
        };

        if let Err(e) = instance.parse_class() {
            return (instance.warnings, Err(e.into()));
        }

        match instance.build_jasm_class() {
            Ok(class) => (instance.warnings, Ok(class)),
            Err(e) => (instance.warnings, Err(e.into())),
        }
    }

    fn build_jasm_class(&mut self) -> Result<ClassFile, ParserError> {
        let super_name = match self.super_name.take() {
            Some(directive) => directive.class_name,
            None => {
                self.warnings
                    .push(Box::new(ParserWarning::MissingSuperClass {
                        class_name: self.name.clone(),
                        class_directive_pos: self.class_directive_pos,
                        default: JAVA_LANG_OBJECT,
                    }));
                JAVA_LANG_OBJECT.to_string()
            }
        };

        // registers attr name in cp, need to think about other attr and how to do it
        let mut attribute_names = AttributeNameMap::new();
        let code_name_idx = self.cp_builder.add_utf8(AttributeKind::Code.as_str());
        attribute_names.insert(AttributeKind::Code, code_name_idx);

        let this_cp_id = self.cp_builder.add_class(&self.name);
        let super_cp_id = self.cp_builder.add_class(&super_name);
        Ok(ClassFile {
            minor_version: 0,
            major_version: 69, // TODO: allow specifying version in jasm
            cp: std::mem::take(&mut self.cp_builder).build(),
            access_flags: self.class_flags, // TODO: set access flags based on parsed flags
            this_class: this_cp_id,
            super_class: super_cp_id,
            interfaces: vec![],
            fields: vec![],
            methods: std::mem::take(&mut self.methods),
            attributes: vec![],
            attribute_names,
        })
    }
}
