use crate::assembler::{ClassDirective, RnsModule, SuperDirective};
use crate::diagnostic::Diagnostic;
use crate::parser::error::{OperandErrPosContext, ParserError, TrailingTokensErrContext};
use crate::parser::error_deprecated::{
    IdentifierContextDeprecated, NonNegativeIntegerContextDeprecated, ParserErrorDeprecated,
    TrailingTokensContextDeprecated,
};
use crate::parser::warning::ParserWarning;
use crate::token::type_hint::{TypeHint, TypeHintKind};
use crate::token::{RnsFlag, RnsToken, RnsTokenKind, Span, Spanned};
use std::collections::BTreeMap;
use std::iter::Peekable;
use std::vec::IntoIter;

mod error;
mod error_deprecated;
#[cfg(test)]
mod tests;
mod warning;

const JAVA_LANG_OBJECT: &str = "java/lang/Object";
const RECOVER_CLASS_NAME: &str = "Foo";

struct RnsParser {
    tokens: Peekable<IntoIter<RnsToken>>,
    eof_span: Span,
    last_span: Span,

    diagnostic: Vec<Diagnostic>,

    class_dir_span: Span,
    class_name: Option<TypeHint>,
    access_flags: BTreeMap<RnsFlag, Span>,

    super_directives: Vec<(Span, TypeHint)>,
}

impl RnsParser {
    fn peek_token(&mut self) -> Option<&RnsToken> {
        self.tokens.peek()
    }

    fn skip_newlines(&mut self) {
        while let Some(RnsToken::Newline(_)) = self.peek_token() {
            self.next_token();
        }
    }

    fn next_token(&mut self) -> RnsToken {
        match self.tokens.next() {
            Some(token) => {
                self.last_span = token.span();
                token
            }
            None => {
                self.last_span = self.eof_span;
                RnsToken::Eof(self.eof_span)
            }
        }
    }

    fn next_until_newline(&mut self) -> Vec<RnsToken> {
        let mut tokens = Vec::new();
        while let Some(token) = self.tokens.peek() {
            if matches!(token, RnsToken::Newline(_) | RnsToken::Eof(_)) {
                break;
            }
            tokens.push(self.next_token());
        }
        tokens
    }

    fn parse_class_access_flags(&mut self) -> BTreeMap<RnsFlag, Span> {
        let mut flags = BTreeMap::new();
        loop {
            match self.peek_token() {
                Some(token) if token.is_access_flag() => {
                    let next_token = self.next_token();
                    let next_token_span = next_token.span();
                    if let RnsToken::AccessFlag(spanned) = next_token {
                        flags
                            .entry(spanned.value)
                            .or_insert_with(Vec::new)
                            .push(next_token_span);
                    }
                }
                _ => break,
            }
        }
        flags
            .into_iter()
            .map(|(k, v)| {
                let first_span = v[0];
                if v.len() > 1 {
                    self.diagnostic
                        .push(ParserWarning::ClassDuplicateFlag { flag: k, spans: v }.into());
                }
                (k, first_span)
            })
            .collect()
    }

    //TODO: add all flags, handle orders, and check for duplicates
    fn parse_method_access_flags(&mut self) -> Result<u16, ParserErrorDeprecated> {
        todo!()
    }

    fn parse_operand(
        &mut self,
        err_ctx: OperandErrPosContext,
    ) -> Result<Spanned<String>, Diagnostic> {
        let prev_token_span = self.last_span;
        let token = self.next_token();
        let token_span = token.span();
        match token {
            RnsToken::Identifier(spanned) => Ok(spanned),
            RnsToken::DotClass(span)
            | RnsToken::DotSuper(span)
            | RnsToken::DotMethod(span)
            | RnsToken::DotCode(span)
            | RnsToken::DotEnd(span)
            | RnsToken::DotAnnotation(span) => {
                self.diagnostic
                    .push(ParserWarning::ReservedLikeIdentifierTodoName.into());
                Ok(Spanned::new(token.to_string(), span))
            }
            RnsToken::AccessFlag(spanned) => {
                self.diagnostic
                    .push(ParserWarning::ReservedLikeIdentifierTodoName.into());
                Ok(Spanned::new(spanned.value.to_string(), spanned.span))
            }
            RnsToken::TypeHint(ref spanned) => {
                self.diagnostic
                    .push(ParserWarning::ReservedLikeIdentifierTodoName.into());
                Ok(Spanned::new(token.to_string(), spanned.span))
            }
            RnsToken::Eof(_) | RnsToken::Newline(_) => {
                Err(ParserError::IdentifierOrHintExpected(prev_token_span, token, err_ctx).into())
            }
            // TODO: avoid _ everywhere and make sure to handle all cases properly
            _ => Err(ParserError::IdentifierOrHintExpected(token_span, token, err_ctx).into()),
        }
    }

    fn resolve_type_hint(
        &mut self,
        th: Spanned<TypeHintKind>,
        err_ctx: OperandErrPosContext,
    ) -> Result<TypeHint, Diagnostic> {
        let kind_span = th.span;
        let res = match th.value {
            TypeHintKind::Utf8 => Ok(TypeHint::Utf8(kind_span, self.parse_operand(err_ctx)?)),
            TypeHintKind::Integer => unimplemented!(),
            TypeHintKind::String => unimplemented!(),
            TypeHintKind::Class => unimplemented!(),
            TypeHintKind::Methodref => unimplemented!(),
            _ => unimplemented!(),
        };

        self.verify_trailing_tokens(TrailingTokensErrContext::TypeHint(th));
        res
    }

    fn parse_operand_or_type_hint<F>(
        &mut self,
        err_ctx: OperandErrPosContext,
        infer_hint: F,
    ) -> Result<TypeHint, Diagnostic>
    where
        F: FnOnce(Spanned<String>) -> TypeHint,
    {
        let token = self.peek_token();

        if let Some(RnsToken::TypeHint(th)) = token {
            let th = th.clone();
            self.next_token();
            self.resolve_type_hint(th, err_ctx)
        } else {
            Ok(infer_hint(self.parse_operand(err_ctx)?))
        }
    }

    fn expect_next_identifier_deprecated(
        &mut self,
        context: IdentifierContextDeprecated,
        prev_token_byte_end: usize,
        prev_token_col_end: usize,
    ) -> Result<(String, Span), ParserErrorDeprecated> {
        let token = self.next_token();
        let token_span = token.span();
        match token {
            RnsToken::Identifier(spanned) => Ok((spanned.value, token_span)),
            RnsToken::Eof(t) | RnsToken::Newline(t) => {
                Err(ParserErrorDeprecated::IdentifierExpected(
                    Span {
                        byte_start: prev_token_byte_end,
                        byte_end: prev_token_byte_end,
                        line: t.line,
                        col_start: prev_token_col_end,
                        col_end: prev_token_col_end,
                    },
                    token,
                    context,
                ))
            }
            _ => Err(ParserErrorDeprecated::IdentifierExpected(
                token.span(),
                token,
                context,
            )),
        }
    }

    // Doesn't need to recover, just check for trailing tokens and report error if they exist
    fn verify_trailing_tokens(&mut self, context: TrailingTokensErrContext) {
        let end_of_previous_token = self.last_span.byte_end - 1; // -1 to point to the last character
        let trailing_tokens = self.next_until_newline();
        if !trailing_tokens.is_empty() {
            self.diagnostic.push(
                ParserError::TrailingTokens(end_of_previous_token, trailing_tokens, context).into(),
            );
        }
    }

    fn expect_no_trailing_tokens_deprecated(
        &mut self,
        context: TrailingTokensContextDeprecated,
    ) -> Result<(), ParserErrorDeprecated> {
        let trailing_tokens = self.next_until_newline();
        if !trailing_tokens.is_empty() {
            return Err(ParserErrorDeprecated::TrailingTokens(
                trailing_tokens,
                context,
            ));
        }
        Ok(())
    }

    // TODO: this is wrong.. also need to review integer token.. it is i32
    // TODO: why I need here prev token info? I can just use current token info for error
    fn expect_next_non_negative_integer(
        &mut self,
        context: NonNegativeIntegerContextDeprecated,
        prev_token_byte_end: usize,
        prev_token_col_end: usize,
    ) -> Result<u32, ParserErrorDeprecated> {
        let token = self.next_token();
        match token {
            RnsToken::Integer(spanned) if spanned.value >= 0 => Ok(spanned.value as u32),
            RnsToken::Eof(t) | RnsToken::Newline(t) => {
                Err(ParserErrorDeprecated::NonNegativeIntegerExpected(
                    Span {
                        byte_start: prev_token_byte_end,
                        byte_end: prev_token_byte_end,
                        line: t.line,
                        col_start: prev_token_col_end,
                        col_end: prev_token_col_end,
                    },
                    token,
                    context,
                ))
            }
            _ => Err(ParserErrorDeprecated::NonNegativeIntegerExpected(
                token.span(),
                token,
                context,
            )),
        }
    }

    /*
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
     */

    fn parse_super_directive(&mut self) {
        let super_token = self.next_token(); // consume .super token

        match self.parse_operand_or_type_hint(OperandErrPosContext::SuperName, |spanned| {
            TypeHint::Class(None, spanned)
        }) {
            Ok(super_name) => self.super_directives.push((super_token.span(), super_name)),
            Err(e) => {
                self.diagnostic.push(e);
            }
        }
        self.verify_trailing_tokens(TrailingTokensErrContext::Super)
    }

    fn anchor_class_directive(&mut self) -> Result<Span, Diagnostic> {
        self.skip_newlines();
        let next_token = self.next_token();

        if matches!(next_token, RnsToken::Eof(_)) {
            return Err(ParserError::EmptyFile(next_token.span()).into());
        }

        if matches!(next_token, RnsToken::DotClass(_)) {
            return Ok(next_token.span());
        }

        let error: Diagnostic =
            ParserError::UnexpectedTokenBeforeClassDefinition(next_token).into();

        // first token is not `.class` — try to recover by finding the next `.class`
        if !self.anchor(&[RnsTokenKind::DotClass]) {
            // can't recover - fail here
            return Err(error);
        }

        // recovered - report error and continue parsing
        self.diagnostic.push(error);
        Ok(self.next_token().span())
    }

    fn parse_class(&mut self) -> Result<(), Diagnostic> {
        self.class_dir_span = self.anchor_class_directive()?;
        self.access_flags = self.parse_class_access_flags();

        match self.parse_operand_or_type_hint(OperandErrPosContext::ClassName, |spanned| {
            TypeHint::Class(None, spanned)
        }) {
            Ok(class_name) => {
                self.class_name = Some(class_name);
            }
            Err(e) => {
                self.diagnostic.push(e);
            }
        }

        self.verify_trailing_tokens(TrailingTokensErrContext::Class);

        while let Some(token) = self.tokens.peek() {
            match token {
                RnsToken::Newline(_) => {
                    self.next_token();
                }
                RnsToken::DotMethod(_) => {
                    unimplemented!("method parsing is not implemented yet")
                }
                RnsToken::DotSuper(_) => self.parse_super_directive(),
                RnsToken::DotEnd(_) => {
                    self.next_token(); // consume .end
                    if let Some(token) = self.tokens.peek() {
                        if let RnsToken::Identifier(s) = token {
                            if s.value == "class" {
                                self.next_token(); // consume "class"
                                break; // .end class - finish parsing
                            }
                        }
                    }
                }
                RnsToken::Eof(_) => break,
                _ => {
                    let unexpected_error =
                        ParserError::UnexpectedTokenInClassBody(self.next_token());
                    if self.anchor(&[RnsTokenKind::DotMethod, RnsTokenKind::DotSuper]) {
                        self.diagnostic.push(unexpected_error.into());
                    } else {
                        return Err(unexpected_error.into());
                    }
                }
            }
        }

        Ok(())
    }

    fn anchor(&mut self, recovery_token: &[RnsTokenKind]) -> bool {
        while let Some(token) = self.tokens.peek() {
            if recovery_token.iter().any(|kind| token.matches_kind(*kind)) {
                return true;
            }
            self.next_token();
        }
        false
    }

    fn take_super_directive(&mut self) -> Option<SuperDirective> {
        let super_directives = std::mem::take(&mut self.super_directives);
        match super_directives.len() {
            0 => {
                self.diagnostic.push(
                    ParserWarning::MissingSuperClass {
                        class_name: self.class_name.clone(),
                        class_dir_pos: self.class_dir_span,
                        default: JAVA_LANG_OBJECT,
                    }
                    .into(),
                );
                Some(SuperDirective {
                    dir_span: None,
                    name: TypeHint::Class(
                        None,
                        Spanned::new(JAVA_LANG_OBJECT.to_string(), Span::default()),
                    ),
                })
            }
            1 => {
                let (dir_span, name) = super_directives.into_iter().next().unwrap();
                Some(SuperDirective {
                    dir_span: Some(dir_span),
                    name,
                })
            }
            _ => {
                self.diagnostic
                    .push(ParserError::MultipleSuperDefinitions(super_directives).into());
                None
            }
        }
    }
}

pub fn parse(tokens: Vec<RnsToken>, eof_span: Span) -> Result<RnsModule, Vec<Diagnostic>> {
    let mut instance = RnsParser {
        tokens: tokens.into_iter().peekable(),
        eof_span,
        last_span: Span::default(),
        diagnostic: Vec::new(),

        class_dir_span: Span::default(),
        class_name: None,
        super_directives: Vec::new(),
        access_flags: Default::default(),
    };

    if let Err(e) = instance.parse_class() {
        instance.diagnostic.push(e);
        return Err(instance.diagnostic);
    }
    let super_dir = instance.take_super_directive();
    let class_dir = ClassDirective {
        dir_span: instance.class_dir_span,
        name: instance.class_name.take(),
        flags: instance.access_flags,
    };
    Ok(RnsModule {
        class_dir,
        super_dir,
        diagnostics: instance.diagnostic,
    })
}
