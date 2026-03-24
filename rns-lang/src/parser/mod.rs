use crate::ast::flag::{RnsClassFlag, RnsMethodFlag};
use crate::ast::{
    ClassDirective, CodeDirective, MethodDirective, RnsInstruction, RnsModule, SuperDirective,
};
use crate::diagnostic::Diagnostic;
use crate::parser::error::{
    AccessFlagContext, NumericRejection, OperandErrPosContext, ParseNumeric, ParserError,
    TrailingTokensErrContext, UnexpectedTokenContext,
};
use crate::parser::error_deprecated::{NonNegativeIntegerContextDeprecated, ParserErrorDeprecated};
use crate::parser::warning::ParserWarning;
use crate::token::type_hint::{TypeHint, TypeHintKind, TypeHintOperandName};
use crate::token::{RnsToken, RnsTokenKind, Span, Spanned};
use std::collections::BTreeMap;
use std::iter::Peekable;
use std::str::FromStr;
use std::vec::IntoIter;

mod error;
mod error_deprecated;
#[cfg(test)]
mod tests;
mod warning;

const JAVA_LANG_OBJECT: &str = "java/lang/Object";

enum TypeHintOrigin {
    Explicit(Spanned<TypeHintKind>),
    Implicit(OperandErrPosContext, TypeHintKind),
}

impl TypeHintOrigin {
    fn kind(&self) -> TypeHintKind {
        match self {
            TypeHintOrigin::Explicit(th) => th.value,
            TypeHintOrigin::Implicit(_, kind) => *kind,
        }
    }

    fn kind_span(&self) -> Option<Span> {
        match self {
            TypeHintOrigin::Explicit(th) => Some(th.span),
            TypeHintOrigin::Implicit(_, _) => None,
        }
    }
}

struct RnsParser {
    tokens: Peekable<IntoIter<RnsToken>>,
    eof_span: Span,
    last_kind: RnsTokenKind,
    last_span: Span,

    diagnostic: Vec<Diagnostic>,

    class_dir_span: Span,
    class_name: Option<TypeHint>,
    access_flags: BTreeMap<RnsClassFlag, Span>,
    method_dirs: Vec<MethodDirective>,

    super_directives: Vec<(Span, TypeHint)>,
    super_err_present: bool,
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
                self.last_kind = token.kind();
                token
            }
            None => {
                self.last_span = self.eof_span;
                self.last_kind = RnsTokenKind::Eof;
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

    fn parse_access_flags<F: Ord>(
        &mut self,
        ctx: AccessFlagContext,
        convert: fn(&crate::token::flag::RnsFlag) -> Option<F>,
    ) -> BTreeMap<F, Span> {
        let mut flags: BTreeMap<F, (crate::token::flag::RnsFlag, Vec<Span>)> = BTreeMap::new();
        loop {
            match self.peek_token() {
                Some(token) if token.is_access_flag() => {
                    let next_token = self.next_token();
                    let next_token_span = next_token.span();
                    if let RnsToken::AccessFlag(spanned) = next_token {
                        if let Some(flag) = convert(&spanned.value) {
                            flags
                                .entry(flag)
                                .or_insert_with(|| (spanned.value, Vec::new()))
                                .1
                                .push(next_token_span);
                        } else {
                            self.diagnostic
                                .push(ParserError::InvalidAccessFlag(ctx, spanned).into())
                        }
                    }
                }
                _ => break,
            }
        }
        flags
            .into_iter()
            .map(|(k, (rns_flag, spans))| {
                let first_span = spans[0];
                if spans.len() > 1 {
                    self.diagnostic.push(
                        ParserWarning::DuplicateAccessFlag {
                            ctx,
                            flag: rns_flag,
                            spans,
                        }
                        .into(),
                    );
                }
                (k, first_span)
            })
            .collect()
    }

    fn parse_class_access_flags(&mut self) -> BTreeMap<RnsClassFlag, Span> {
        self.parse_access_flags(AccessFlagContext::Class, |f| f.as_class_flag())
    }

    fn parse_method_access_flags(&mut self) -> BTreeMap<RnsMethodFlag, Span> {
        self.parse_access_flags(AccessFlagContext::Method, |f| f.as_method_flag())
    }

    fn try_next_identifier(&mut self) -> Result<Spanned<String>, RnsToken> {
        // don't consume to not break trailing tokens check
        if let Some(next) = self.peek_token()
            && matches!(next, RnsToken::Eof(_) | RnsToken::Newline(_))
        {
            return Err(next.clone());
        }
        let token = self.next_token();
        match token {
            RnsToken::Identifier(spanned) => Ok(spanned),
            RnsToken::Eof(_) | RnsToken::Newline(_) => Err(token),
            identifier_like => {
                self.diagnostic
                    .push(ParserWarning::ReservedLikeIdentifierTodoName.into());
                Ok(Spanned::new(
                    identifier_like.token_name().to_string(),
                    identifier_like.span(),
                ))
            }
        }
    }

    fn try_next_numeric<T: ParseNumeric>(&mut self) -> Result<Spanned<T>, NumericRejection> {
        // Don't consume EOF/newline to not break trailing tokens check
        if let Some(next) = self.peek_token()
            && matches!(next, RnsToken::Eof(_) | RnsToken::Newline(_))
        {
            return Err(NumericRejection::Missing(next.clone()));
        }
        let token = self.next_token();
        match token {
            RnsToken::Identifier(ref spanned) => {
                let raw = &spanned.value;
                T::parse_and_classify(raw, spanned).map(|value| Spanned::new(value, spanned.span))
            }
            RnsToken::Eof(_) | RnsToken::Newline(_) => Err(NumericRejection::Missing(token)),
            _ => {
                let spanned = Spanned::new(token.as_identifier().to_string(), token.span());
                Err(NumericRejection::NotNumeric(spanned))
            }
        }
    }

    fn parse_identifier(
        &mut self,
        err_ctx: OperandErrPosContext,
    ) -> Result<Spanned<String>, Diagnostic> {
        let prev_token_span = self.last_span;
        self.try_next_identifier().map_err(|token| {
            ParserError::IdentifierOrHintExpected(prev_token_span, token, err_ctx).into()
        })
    }

    fn parse_hint_identifier(
        &mut self,
        origin: &TypeHintOrigin,
        operand: TypeHintOperandName,
    ) -> Result<Spanned<String>, ParserError> {
        let after_span = self.last_span;
        self.try_next_identifier().map_err(|_| match origin {
            TypeHintOrigin::Explicit(th) => ParserError::MissingTypeHintOperand {
                type_hint: th.clone(),
                operand,
                after_span,
            },
            TypeHintOrigin::Implicit(ctx, kind) => ParserError::MissingImplicitTypeHintOperand {
                err_ctx: ctx.clone(),
                implicit_kind: *kind,
                operand,
                after_span,
            },
        })
    }

    fn parse_type_hint_i32(
        &mut self,
        type_hint: Spanned<TypeHintKind>,
    ) -> Result<Spanned<i32>, ParserError> {
        let after_span = self.last_span;
        self.try_next_numeric::<i32>()
            .map_err(|rejection| match rejection {
                NumericRejection::Missing(_) => ParserError::MissingTypeHintOperand {
                    type_hint,
                    operand: TypeHintOperandName::I32Literal,
                    after_span,
                },
                other => ParserError::TypeHintExpectsNumericOperand {
                    type_hint,
                    rejection: other,
                },
            })
    }

    fn parse_type_hint_i64(
        &mut self,
        type_hint: Spanned<TypeHintKind>,
    ) -> Result<Spanned<i64>, ParserError> {
        let after_span = self.last_span;
        self.try_next_numeric::<i64>()
            .map_err(|rejection| match rejection {
                NumericRejection::Missing(_) => ParserError::MissingTypeHintOperand {
                    type_hint,
                    operand: TypeHintOperandName::I64Literal,
                    after_span,
                },
                other => ParserError::TypeHintExpectsNumericOperand {
                    type_hint,
                    rejection: other,
                },
            })
    }

    fn parse_type_hint_f32(
        &mut self,
        type_hint: Spanned<TypeHintKind>,
    ) -> Result<Spanned<f32>, ParserError> {
        let after_span = self.last_span;
        self.try_next_numeric::<f32>()
            .map_err(|rejection| match rejection {
                NumericRejection::Missing(_) => ParserError::MissingTypeHintOperand {
                    type_hint,
                    operand: TypeHintOperandName::F32Literal,
                    after_span,
                },
                other => ParserError::TypeHintExpectsNumericOperand {
                    type_hint,
                    rejection: other,
                },
            })
    }

    fn parse_type_hint_f64(
        &mut self,
        type_hint: Spanned<TypeHintKind>,
    ) -> Result<Spanned<f64>, ParserError> {
        let after_span = self.last_span;
        self.try_next_numeric::<f64>()
            .map_err(|rejection| match rejection {
                NumericRejection::Missing(_) => ParserError::MissingTypeHintOperand {
                    type_hint,
                    operand: TypeHintOperandName::F64Literal,
                    after_span,
                },
                other => ParserError::TypeHintExpectsNumericOperand {
                    type_hint,
                    rejection: other,
                },
            })
    }

    fn resolve_identifier_type_hint(
        &mut self,
        origin: &TypeHintOrigin,
    ) -> Result<TypeHint, Diagnostic> {
        let kind = origin.kind();
        let kind_span = origin.kind_span();
        match kind {
            TypeHintKind::Utf8 => Ok(TypeHint::Utf8(
                kind_span,
                self.parse_hint_identifier(origin, TypeHintOperandName::Utf8Entry)?,
            )),
            TypeHintKind::String => Ok(TypeHint::String(
                kind_span,
                self.parse_hint_identifier(origin, TypeHintOperandName::StringLiteral)?,
            )),
            TypeHintKind::Class => Ok(TypeHint::Class(
                kind_span,
                self.parse_hint_identifier(origin, TypeHintOperandName::ClassName)?,
            )),
            TypeHintKind::Methodref => {
                let class_name =
                    self.parse_hint_identifier(origin, TypeHintOperandName::ClassName)?;
                let method_name =
                    self.parse_hint_identifier(origin, TypeHintOperandName::MethodName)?;
                let descriptor =
                    self.parse_hint_identifier(origin, TypeHintOperandName::MethodDescriptor)?;
                Ok(TypeHint::Methodref(
                    kind_span,
                    class_name,
                    method_name,
                    descriptor,
                ))
            }
            TypeHintKind::Fieldref => {
                let class_name =
                    self.parse_hint_identifier(origin, TypeHintOperandName::ClassName)?;
                let field_name =
                    self.parse_hint_identifier(origin, TypeHintOperandName::FieldName)?;
                let descriptor =
                    self.parse_hint_identifier(origin, TypeHintOperandName::FieldDescriptor)?;
                Ok(TypeHint::Fieldref(
                    kind_span, class_name, field_name, descriptor,
                ))
            }
            _ => unimplemented!(),
        }
    }

    fn resolve_explicit_type_hint(
        &mut self,
        th: Spanned<TypeHintKind>,
    ) -> Result<TypeHint, Diagnostic> {
        let th_for_trailing = th.clone();
        let res = match th.value {
            TypeHintKind::ZeroIndex => Ok(TypeHint::ZeroIndex(th.span)),
            TypeHintKind::Integer => Ok(TypeHint::Integer(
                Some(th.span),
                self.parse_type_hint_i32(th)?,
            )),
            TypeHintKind::Long => Ok(TypeHint::Long(Some(th.span), self.parse_type_hint_i64(th)?)),
            TypeHintKind::Float => Ok(TypeHint::Float(
                Some(th.span),
                self.parse_type_hint_f32(th)?,
            )),
            TypeHintKind::Double => Ok(TypeHint::Double(
                Some(th.span),
                self.parse_type_hint_f64(th)?,
            )),
            _ => {
                let origin = TypeHintOrigin::Explicit(th);
                self.resolve_identifier_type_hint(&origin)
            }
        };
        self.verify_trailing_tokens(TrailingTokensErrContext::TypeHint(th_for_trailing));
        res
    }

    fn parse_operand_or_type_hint(
        &mut self,
        err_ctx: OperandErrPosContext,
        implicit_kind: TypeHintKind,
    ) -> Result<TypeHint, Diagnostic> {
        if let Some(RnsToken::TypeHint(th)) = self.peek_token() {
            let th = th.clone();
            self.next_token();
            self.resolve_explicit_type_hint(th)
        } else {
            let origin = TypeHintOrigin::Implicit(err_ctx, implicit_kind);
            self.resolve_identifier_type_hint(&origin)
        }
    }

    // Doesn't need to recover, just check for trailing tokens and report error if they exist
    fn verify_trailing_tokens(&mut self, context: TrailingTokensErrContext) {
        let end_of_previous_token = self.last_span.byte_end;
        let trailing_tokens = self.next_until_newline();
        if !trailing_tokens.is_empty() {
            self.diagnostic.push(
                ParserError::TrailingTokens(end_of_previous_token, trailing_tokens, context).into(),
            );
        }
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
            RnsToken::Identifier(spanned) => Ok(u32::from_str(&spanned.value).unwrap()),
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

    fn parse_instruction(&mut self) -> Result<RnsInstruction, Diagnostic> {
        /*
        let raw_instruction = self.parse_identifier(
            OperandErrPosContext::InstructionName
        )?;
        let instruction_spec = *INSTRUCTION_SPECS
            .get(raw_instruction.value.as_str())
            .ok_or_else(|| { todo!() })?;
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
         */
        todo!()
    }

    /* TODO: doesn't handle errors. In the closest future I want to calculate stack and locals and make it optional.
       TODO: When it will be explicitly specified but don't match my calculations, it should warn.
    */
    fn parse_code_header(&mut self) -> (u16, u16) {
        let mut stack = None;
        let mut locals = None;

        while let Some(RnsToken::Identifier(_)) = self.peek_token() {
            let identifier_token = self.next_token();
            match identifier_token {
                RnsToken::Identifier(ref name) if name.value == "stack" => {
                    stack = Some(self.try_next_numeric().unwrap().value);
                }
                RnsToken::Identifier(ref name) if name.value == "locals" => {
                    locals = Some(self.try_next_numeric().unwrap().value);
                }
                other => panic!("Unexpected code directive argument {other:?}"),
            }
        }

        if !matches!(self.peek_token(), Some(RnsToken::Newline(_))) {
            panic!("Unexpected tokens after code directive header");
        }

        (stack.unwrap(), locals.unwrap())
    }

    fn parse_code_directive(&mut self) -> Option<CodeDirective> {
        /*
        self.next_token(); // consume .code token
        let mut has_fatal_error = false;
        let mut labels = HashMap::new();
        let mut instructions = Vec::new();

        let (stack, locals) = self.parse_code_header();
        self.skip_newlines();

        while let Some(token) = self.tokens.peek() {
            match token {
                RnsToken::Eof(_) |
            }
            if matches!(token, RnsToken::DotEnd(_) | RnsToken::Eof(_)) {
                break;
            }
            self.skip_newlines();
            // TODO: ANTON, DON'T FORGET TO ANCHOR THE NEWLINE TOMORROW
            self.parse_instruction(&mut code)?;
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
         */
        todo!()
    }

    fn parse_method(&mut self) -> Result<MethodDirective, Diagnostic> {
        let dot_method = self.next_token(); // consume .method token
        let access_flags = self.parse_method_access_flags();
        let mut method = MethodDirective::new(dot_method.span(), access_flags);
        method.name = self
            .parse_operand_or_type_hint(OperandErrPosContext::MethodName, TypeHintKind::Utf8)
            .map_err(|e| self.diagnostic.push(e))
            .ok();
        // when there is no method name, it doesn't make sense to expect anything else
        if method.name.is_some() {
            method.descriptor = self
                .parse_operand_or_type_hint(
                    OperandErrPosContext::MethodDescriptor,
                    TypeHintKind::Utf8,
                )
                .map_err(|e| self.diagnostic.push(e))
                .ok();
        }
        // trailing tokens can be only when descriptor is present
        if method.descriptor.is_some() {
            self.verify_trailing_tokens(TrailingTokensErrContext::Method);
        }

        while let Some(token) = self.tokens.peek() {
            match token {
                RnsToken::Newline(_) => {
                    self.next_token();
                }
                RnsToken::DotCode(_) => {
                    if let Some(code_dir) = &method.code_dir {
                        self.diagnostic.push(
                            ParserError::MultipleCodeBlocks {
                                method_name: method.name.clone(),
                                method_span: method.dir_span,
                                first_code_span: code_dir.dir_span,
                                duplicate: token.span(),
                            }
                            .into(),
                        );
                        // TODO: decide final strategy
                        self.anchor(&[RnsTokenKind::DotCodeEnd]);
                    } else {
                        method.code_dir = self.parse_code_directive();
                    }
                }
                RnsToken::DotMethodEnd(_) => {
                    self.next_token(); // consume .method_end
                }
                // TODO: decide strategy, allow not closed?
                RnsToken::Eof(_) => break,
                _ => {
                    let unexpected_error = ParserError::UnexpectedBodyToken(
                        UnexpectedTokenContext::MethodBody,
                        self.next_token(),
                    );
                    self.diagnostic.push(unexpected_error.into());
                    // TODO: test unknown_token .super/.method etc.
                    self.anchor(&[RnsTokenKind::DotCode]);
                }
            }
        }

        todo!()
    }

    fn parse_super_directive(&mut self) {
        let super_token = self.next_token(); // consume .super token

        match self.parse_operand_or_type_hint(OperandErrPosContext::SuperName, TypeHintKind::Class)
        {
            Ok(super_name) => self.super_directives.push((super_token.span(), super_name)),
            Err(e) => {
                self.super_err_present = true;
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

        self.class_name = self
            .parse_operand_or_type_hint(OperandErrPosContext::ClassName, TypeHintKind::Class)
            .map_err(|e| self.diagnostic.push(e))
            .ok();

        self.verify_trailing_tokens(TrailingTokensErrContext::Class);

        while let Some(token) = self.tokens.peek() {
            match token {
                RnsToken::Newline(_) => {
                    self.next_token();
                }
                RnsToken::DotMethod(_) => {
                    let method_dir = self.parse_method()?;
                    self.method_dirs.push(method_dir);
                }
                RnsToken::DotSuper(_) => self.parse_super_directive(),
                RnsToken::DotClassEnd(_) => {
                    self.next_token(); // consume .class_end
                    break;
                }
                RnsToken::Eof(_) => break,
                _ => {
                    let unexpected_error = ParserError::UnexpectedBodyToken(
                        UnexpectedTokenContext::ClassBody,
                        self.next_token(),
                    );
                    self.diagnostic.push(unexpected_error.into());
                    // TODO: test unknown_token .super/.method etc.
                    self.anchor(&[RnsTokenKind::DotMethod, RnsTokenKind::DotSuper]);
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
                // If no class name is defined, doesn't make sense to report missing superclass
                // If problem with super is already report, don't report missing superclass to avoid duplicate errors
                if self.class_name.is_some() && !self.super_err_present {
                    self.diagnostic.push(
                        ParserWarning::MissingSuperClass {
                            class_name: self.class_name.clone(),
                            class_dir_pos: self.class_dir_span,
                            default: JAVA_LANG_OBJECT,
                        }
                        .into(),
                    );
                }
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
        last_kind: RnsTokenKind::Eof,
        diagnostic: Vec::new(),

        class_dir_span: Span::default(),
        class_name: None,
        method_dirs: Vec::new(),
        super_directives: Vec::new(),
        super_err_present: false,
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
