use crate::ast::flag::{RnsClassFlag, RnsInnerFlag, RnsMethodFlag};
use crate::ast::{
    ClassDirective, CodeDirective, InnerClassDirective, MethodDirective, PackageDirective,
    RnsInstruction, RnsModule, RnsOperand, SuperDirective,
};
use crate::diagnostic::Diagnostic;
use crate::instruction::{INSTRUCTION_SPECS, InstructionOperand};
use crate::parser::error::{
    AccessFlagContext, NumericRejection, OperandErrPosContext, ParseNumeric, ParserError,
    TrailingTokensErrContext, UnexpectedTokenContext,
};
use crate::parser::warning::ParserWarning;
use crate::token::type_hint::{RefTypeHint, TypeHint, TypeHintKind, TypeHintOperandName};
use crate::token::{RnsFlag, RnsToken, RnsTokenKind, Span, Spanned};
use std::collections::HashMap;
use std::hash::Hash;
use std::iter::Peekable;
use std::vec::IntoIter;

mod error;
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
    class_dir: ParsedClassDirective,
}

#[derive(Default)]
struct ParsedClassDirective {
    reported_errs: ReportedErrs,
    class_dir_span: Span,
    class_name: Option<TypeHint>,
    access_flags: HashMap<RnsClassFlag, Span>,
    method_dirs: Vec<MethodDirective>,

    super_directives: Vec<(Span, TypeHint)>,
    package_directives: Vec<(Span, String)>,

    inner_classes: Vec<ParsedInnerDirective>,
}

#[derive(Default)]
struct ParsedInnerDirective {
    reported_errs: ReportedErrs,
    dir_span: Span,
    name: Option<TypeHint>,
    flags: HashMap<RnsInnerFlag, Span>,
    super_directives: Vec<(Span, TypeHint)>,
    mangled_name_dirs: Vec<(Span, TypeHint)>,
}

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum ErrKind {
    Super = 0,
}

#[derive(Default, Clone, Copy)]
struct ReportedErrs(u32);

impl ReportedErrs {
    fn insert(&mut self, kind: ErrKind) {
        self.0 |= 1 << kind as u8;
    }

    fn contains(self, kind: ErrKind) -> bool {
        self.0 & (1 << kind as u8) != 0
    }
}

impl RnsParser {
    fn from_tokens(tokens: Peekable<IntoIter<RnsToken>>, eof_span: Span) -> Self {
        Self {
            tokens,
            eof_span,
            last_span: Span::default(),
            last_kind: RnsTokenKind::Eof,
            diagnostic: Vec::new(),
            class_dir: ParsedClassDirective::default(),
        }
    }

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

    fn parse_access_flags<F: Eq + Hash + Ord>(
        &mut self,
        ctx: AccessFlagContext,
        convert: fn(&RnsFlag) -> Option<F>,
    ) -> HashMap<F, Span> {
        let mut flags: HashMap<F, (RnsFlag, Vec<Span>)> = HashMap::new();
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
        let mut sorted: Vec<_> = flags.into_iter().collect();
        sorted.sort_by(|(a, _), (b, _)| a.cmp(b));
        sorted
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

    fn parse_class_access_flags(&mut self) -> HashMap<RnsClassFlag, Span> {
        self.parse_access_flags(AccessFlagContext::Class, |f| f.as_class_flag())
    }

    fn parse_inner_access_flags(&mut self) -> HashMap<RnsInnerFlag, Span> {
        self.parse_access_flags(AccessFlagContext::Inner, |f| f.as_inner_flag())
    }

    fn parse_method_access_flags(&mut self) -> HashMap<RnsMethodFlag, Span> {
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
    ) -> Result<Spanned<String>, Box<Diagnostic>> {
        let prev_token_span = self.last_span;
        self.try_next_identifier().map_err(|token| {
            ParserError::IdentifierOrHintExpected(prev_token_span, token, err_ctx).into()
        })
    }

    fn parse_hint_identifier(
        &mut self,
        origin: &TypeHintOrigin,
        operand: TypeHintOperandName,
    ) -> Result<Spanned<String>, Box<ParserError>> {
        let after_span = self.last_span;
        self.try_next_identifier().map_err(|_| {
            Box::new(match origin {
                TypeHintOrigin::Explicit(th) => ParserError::MissingTypeHintOperand {
                    type_hint: th.clone(),
                    operand,
                    after_span,
                },
                TypeHintOrigin::Implicit(ctx, kind) => {
                    ParserError::MissingImplicitTypeHintOperand {
                        err_ctx: ctx.clone(),
                        implicit_kind: *kind,
                        operand,
                        after_span,
                    }
                }
            })
        })
    }

    fn parse_type_hint_u16(
        &mut self,
        type_hint: Spanned<TypeHintKind>,
    ) -> Result<Spanned<u16>, Box<ParserError>> {
        let after_span = self.last_span;
        self.try_next_numeric::<u16>().map_err(|rejection| {
            Box::new(match rejection {
                NumericRejection::Missing(_) => ParserError::MissingTypeHintOperand {
                    type_hint,
                    operand: TypeHintOperandName::U16Literal,
                    after_span,
                },
                other => ParserError::TypeHintExpectsNumericOperand {
                    type_hint,
                    rejection: other,
                },
            })
        })
    }

    fn parse_type_hint_i32(
        &mut self,
        type_hint: Spanned<TypeHintKind>,
    ) -> Result<Spanned<i32>, Box<ParserError>> {
        let after_span = self.last_span;
        self.try_next_numeric::<i32>().map_err(|rejection| {
            Box::new(match rejection {
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
        })
    }

    fn parse_type_hint_i64(
        &mut self,
        type_hint: Spanned<TypeHintKind>,
    ) -> Result<Spanned<i64>, Box<ParserError>> {
        let after_span = self.last_span;
        self.try_next_numeric::<i64>().map_err(|rejection| {
            Box::new(match rejection {
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
        })
    }

    fn parse_type_hint_f32(
        &mut self,
        type_hint: Spanned<TypeHintKind>,
    ) -> Result<Spanned<f32>, Box<ParserError>> {
        let after_span = self.last_span;
        self.try_next_numeric::<f32>().map_err(|rejection| {
            Box::new(match rejection {
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
        })
    }

    fn parse_type_hint_f64(
        &mut self,
        type_hint: Spanned<TypeHintKind>,
    ) -> Result<Spanned<f64>, Box<ParserError>> {
        let after_span = self.last_span;
        self.try_next_numeric::<f64>().map_err(|rejection| {
            Box::new(match rejection {
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
        })
    }

    fn resolve_identifier_type_hint(
        &mut self,
        origin: &TypeHintOrigin,
    ) -> Result<TypeHint, Box<Diagnostic>> {
        let kind = origin.kind();
        let kind_span = origin.kind_span();
        match kind {
            TypeHintKind::Utf8 => {
                let operand_name = match origin {
                    TypeHintOrigin::Implicit(OperandErrPosContext::MethodName, _) => {
                        TypeHintOperandName::MethodName
                    }
                    TypeHintOrigin::Implicit(OperandErrPosContext::MethodDescriptor, _) => {
                        TypeHintOperandName::MethodDescriptor
                    }
                    TypeHintOrigin::Implicit(OperandErrPosContext::ClassName, _) => {
                        TypeHintOperandName::ClassName
                    }
                    TypeHintOrigin::Implicit(OperandErrPosContext::SuperName, _) => {
                        TypeHintOperandName::ClassName
                    }
                    _ => TypeHintOperandName::Utf8Entry,
                };
                Ok(TypeHint::Utf8(
                    kind_span,
                    self.parse_hint_identifier(origin, operand_name)?,
                ))
            }
            TypeHintKind::String => Ok(TypeHint::String(
                kind_span,
                self.parse_hint_identifier(origin, TypeHintOperandName::StringLiteral)?,
            )),
            TypeHintKind::Class => Ok(TypeHint::Class(
                kind_span,
                self.parse_hint_identifier(origin, TypeHintOperandName::ClassName)?,
            )),
            TypeHintKind::Methodref => {
                let class = self.parse_hint_identifier(origin, TypeHintOperandName::ClassName)?;
                let name = self.parse_hint_identifier(origin, TypeHintOperandName::MethodName)?;
                let descriptor =
                    self.parse_hint_identifier(origin, TypeHintOperandName::MethodDescriptor)?;
                Ok(TypeHint::Methodref(Box::new(RefTypeHint {
                    hint_span: kind_span,
                    class,
                    name,
                    descriptor,
                })))
            }
            TypeHintKind::Fieldref => {
                let class = self.parse_hint_identifier(origin, TypeHintOperandName::ClassName)?;
                let name = self.parse_hint_identifier(origin, TypeHintOperandName::FieldName)?;
                let descriptor =
                    self.parse_hint_identifier(origin, TypeHintOperandName::FieldDescriptor)?;
                Ok(TypeHint::Fieldref(Box::new(RefTypeHint {
                    hint_span: kind_span,
                    class,
                    name,
                    descriptor,
                })))
            }
            _ => unimplemented!(),
        }
    }

    fn resolve_explicit_type_hint(
        &mut self,
        th: Spanned<TypeHintKind>,
    ) -> Result<TypeHint, Box<Diagnostic>> {
        let th_for_trailing = th.clone();
        let res = match th.value {
            TypeHintKind::CpIndex => Ok(TypeHint::CpIndex(th.span, self.parse_type_hint_u16(th)?)),
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
    ) -> Result<TypeHint, Box<Diagnostic>> {
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

    fn parse_instruction(&mut self) -> Result<RnsInstruction, Box<Diagnostic>> {
        let raw_instruction = self.parse_identifier(OperandErrPosContext::InstructionName)?;
        let instruction_spec = *INSTRUCTION_SPECS
            .get(raw_instruction.value.as_str())
            .ok_or_else(|| ParserError::UnknownInstruction(raw_instruction.clone()))?;
        let instruction = match instruction_spec.operand {
            InstructionOperand::None => {
                RnsInstruction::new_without_operand(raw_instruction.span, instruction_spec)
            }
            InstructionOperand::MethodRef => {
                let operand = self.parse_operand_or_type_hint(
                    OperandErrPosContext::InstructionOperand(instruction_spec),
                    TypeHintKind::Methodref,
                )?;
                RnsInstruction::new(
                    raw_instruction.span,
                    instruction_spec,
                    RnsOperand::CpRef(operand),
                )
            }
            InstructionOperand::FieldRef => {
                let operand = self.parse_operand_or_type_hint(
                    OperandErrPosContext::InstructionOperand(instruction_spec),
                    TypeHintKind::Fieldref,
                )?;
                RnsInstruction::new(
                    raw_instruction.span,
                    instruction_spec,
                    RnsOperand::CpRef(operand),
                )
            }
            InstructionOperand::TypeHint => match self.next_token() {
                RnsToken::TypeHint(th) => {
                    let operand = self.resolve_explicit_type_hint(th)?;
                    RnsInstruction::new(
                        raw_instruction.span,
                        instruction_spec,
                        RnsOperand::CpRef(operand),
                    )
                }
                found => {
                    return Err(ParserError::InstructionRequiresExplicitTypeHint {
                        raw_instruction,
                        found,
                    }
                    .into());
                }
            },
            InstructionOperand::Numeric(numeric_kind) => {
                let value = match self.try_next_numeric::<i64>() {
                    Ok(spanned) => {
                        if spanned.value < numeric_kind.min_value()
                            || spanned.value > numeric_kind.max_value()
                        {
                            return Err(ParserError::InstructionOperandNumericError {
                                instruction: Spanned::new(
                                    raw_instruction.value.clone(),
                                    raw_instruction.span,
                                ),
                                rejection: NumericRejection::Overflow(Spanned::new(
                                    spanned.value.to_string(),
                                    spanned.span,
                                )),
                                numeric_kind,
                            }
                            .into());
                        }
                        spanned
                    }
                    Err(rejection) => {
                        return Err(ParserError::InstructionOperandNumericError {
                            instruction: Spanned::new(
                                raw_instruction.value.clone(),
                                raw_instruction.span,
                            ),
                            rejection,
                            numeric_kind,
                        }
                        .into());
                    }
                };
                RnsInstruction::new(
                    raw_instruction.span,
                    instruction_spec,
                    RnsOperand::Numeric(numeric_kind, value),
                )
            }
            InstructionOperand::Label => {
                let label = self
                    .parse_identifier(OperandErrPosContext::InstructionOperand(instruction_spec))?;
                RnsInstruction::new(
                    raw_instruction.span,
                    instruction_spec,
                    RnsOperand::Label(label),
                )
            }
        };

        self.verify_trailing_tokens(TrailingTokensErrContext::Instruction(raw_instruction));
        Ok(instruction)
    }

    fn parse_code_header(&mut self) -> (u16, u16) {
        let dir_span = self.last_span;
        let after_code = Span {
            byte_start: dir_span.byte_end,
            byte_end: dir_span.byte_end,
            line: dir_span.line,
            col_start: dir_span.col_end,
            col_end: dir_span.col_end,
        };
        let mut stack = None;
        let mut locals = None;

        while let Some(RnsToken::Identifier(_)) = self.peek_token() {
            let identifier_token = self.next_token();
            match identifier_token {
                RnsToken::Identifier(ref name) if name.value == "stack" => {
                    stack = self.try_next_numeric().ok().map(|s| s.value);
                }
                RnsToken::Identifier(ref name) if name.value == "locals" => {
                    locals = self.try_next_numeric().ok().map(|s| s.value);
                }
                other => self
                    .diagnostic
                    .push(ParserError::UnknownCodeDirectiveAttribute(other).into()),
            }
        }

        if !matches!(self.peek_token(), Some(RnsToken::Newline(_))) {
            self.diagnostic.push(
                ParserError::NotYetImplemented {
                    msg: "unexpected tokens after code header".into(),
                    label_msg: "unexpected tokens after code header".into(),
                    span: after_code,
                }
                .into(),
            );
        }

        let stack = stack.unwrap_or_else(|| {
            self.diagnostic.push(
                ParserError::NotYetImplemented {
                    msg: "missing stack operand for code directive".into(),
                    label_msg: "missing 'stack' operand".into(),
                    span: after_code,
                }
                .into(),
            );
            0
        });
        let locals = locals.unwrap_or_else(|| {
            self.diagnostic.push(
                ParserError::NotYetImplemented {
                    msg: "missing locals operand for code directive".into(),
                    label_msg: "missing 'locals' operand".into(),
                    span: after_code,
                }
                .into(),
            );
            0
        });
        (stack, locals)
    }

    fn parse_code_directive(&mut self) -> Option<CodeDirective> {
        let dir_span = self.next_token().span(); // consume .code token
        let mut labels: HashMap<String, u32> = HashMap::new();
        let mut instructions = Vec::new();
        let mut cur_pc = 0u32;

        let (stack, locals) = self.parse_code_header();

        while let Some(token) = self.peek_token() {
            match token {
                RnsToken::Newline(_) => {
                    self.next_token();
                }
                RnsToken::Label(_) => {
                    if let RnsToken::Label(label) = self.next_token() {
                        // TODO: check trailing tokens after label
                        labels.insert(label.value, cur_pc);
                        self.skip_newlines();
                        if matches!(self.peek_token(), Some(RnsToken::DotCodeEnd(_))) {
                            todo!("Special error for label should be followed by instruction")
                        }
                        let instruction = self.parse_instruction().unwrap();
                        cur_pc += instruction.spec.opcode.pc_size().unwrap() as u32;
                        instructions.push(instruction);
                    }
                }
                RnsToken::DotCodeEnd(_) | RnsToken::Eof(_) => {
                    self.next_token(); // consume .code_end or EOF
                    break;
                }
                _ => match self.parse_instruction() {
                    Ok(instruction) => {
                        cur_pc += instruction.spec.opcode.pc_size().unwrap() as u32;
                        instructions.push(instruction);
                    }
                    Err(e) => {
                        self.anchor(&[RnsTokenKind::Newline]);
                        self.diagnostic.push(*e);
                    }
                },
            }
        }

        Some(CodeDirective {
            dir_span,
            instructions,
            labels,
            max_stack: stack,
            max_locals: locals,
        })
    }

    fn parse_method(&mut self) -> Result<MethodDirective, Box<Diagnostic>> {
        let dot_method = self.next_token(); // consume .method token
        let access_flags = self.parse_method_access_flags();
        let mut method = MethodDirective::new(dot_method.span(), access_flags);
        method.name = self
            .parse_operand_or_type_hint(OperandErrPosContext::MethodName, TypeHintKind::Utf8)
            .map_err(|e| self.diagnostic.push(*e))
            .ok();
        // when there is no method name, it doesn't make sense to expect anything else
        if method.name.is_some() {
            method.descriptor = self
                .parse_operand_or_type_hint(
                    OperandErrPosContext::MethodDescriptor,
                    TypeHintKind::Utf8,
                )
                .map_err(|e| self.diagnostic.push(*e))
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
                    break;
                }
                // TODO: decide strategy, allow not closed?
                RnsToken::Eof(_) => break,
                _ => {
                    let unexpected_error = ParserError::UnexpectedToken(
                        UnexpectedTokenContext::MethodBody,
                        self.next_token(),
                    );
                    self.diagnostic.push(unexpected_error.into());
                    // TODO: test unknown_token .super/.method etc.
                    self.anchor(&[RnsTokenKind::DotCode]);
                }
            }
        }

        Ok(MethodDirective {
            dir_span: method.dir_span,
            name: method.name,
            descriptor: method.descriptor,
            flags: method.flags,
            code_dir: method.code_dir,
        })
    }

    // TODO: I don't like the ret type
    fn parse_super_directive(&mut self) -> Option<(Span, TypeHint)> {
        let super_token = self.next_token(); // consume .super token

        let ret = match self
            .parse_operand_or_type_hint(OperandErrPosContext::SuperName, TypeHintKind::Class)
        {
            Ok(super_name) => Some((super_token.span(), super_name)),
            Err(e) => {
                self.diagnostic.push(*e);
                None
            }
        };
        self.verify_trailing_tokens(TrailingTokensErrContext::Super);
        ret
    }

    fn parse_package_directive(&mut self) {
        let package_token = self.next_token(); // consume .package token

        match self.try_next_identifier() {
            Ok(package_name) => {
                let name_span = package_name.span;
                let name_value = package_name.value;

                // Warn if package name contains '.'
                if name_value.contains('.') {
                    self.diagnostic.push(
                        ParserWarning::PackageContainsDot {
                            package_name: name_value.clone(),
                            package_span: name_span,
                        }
                        .into(),
                    );
                }

                self.class_dir
                    .package_directives
                    .push((package_token.span(), name_value));
            }
            Err(token) => {
                self.diagnostic.push(
                    ParserError::IdentifierOrHintExpected(
                        self.last_span,
                        token,
                        OperandErrPosContext::PackageName,
                    )
                    .into(),
                );
            }
        }
        self.verify_trailing_tokens(TrailingTokensErrContext::Package);
    }

    fn anchor_class_directive(&mut self) -> Result<Span, Box<Diagnostic>> {
        self.skip_newlines();
        let next_token = self.next_token();

        if matches!(next_token, RnsToken::Eof(_)) {
            return Err(ParserError::EmptyFile(next_token.span()).into());
        }

        if matches!(next_token, RnsToken::DotClass(_)) {
            return Ok(next_token.span());
        }

        let error: Diagnostic =
            ParserError::UnexpectedToken(UnexpectedTokenContext::BeforeClassDefinition, next_token)
                .into();

        // first token is not `.class` — try to recover by finding the next `.class`
        if !self.anchor(&[RnsTokenKind::DotClass]) {
            // can't recover - fail here
            return Err(Box::new(error));
        }

        // recovered - report error and continue parsing
        self.diagnostic.push(error);
        Ok(self.next_token().span())
    }

    fn parse_inner(&mut self) -> ParsedInnerDirective {
        let inner_token = self.next_token(); // consume .inner token

        let flags = self.parse_inner_access_flags();
        let mut reported_errs = ReportedErrs::default();
        let class_name = self
            .parse_operand_or_type_hint(OperandErrPosContext::InnerName, TypeHintKind::Class)
            .map_err(|e| self.diagnostic.push(*e))
            .ok();
        let mut super_directives = Vec::new();
        let mut mangled_name_dirs = Vec::new();

        self.verify_trailing_tokens(TrailingTokensErrContext::Inner);

        while let Some(token) = self.tokens.peek() {
            match token {
                RnsToken::Newline(_) => {
                    self.next_token();
                }
                RnsToken::DotMethod(_) => {
                    // TODO: I shouldn't fail fast here, need try to anchor
                    let method_dir = self.parse_method().unwrap();
                    self.class_dir.method_dirs.push(method_dir);
                }
                RnsToken::DotInnerEnd(_) => {
                    self.next_token(); // consume .class_end
                    self.verify_trailing_tokens(TrailingTokensErrContext::InnerEnd);
                    break;
                }
                RnsToken::DotSuper(_) => {
                    if let Some(super_dir) = self.parse_super_directive() {
                        super_directives.push(super_dir);
                    } else {
                        reported_errs.insert(ErrKind::Super);
                    }
                }
                RnsToken::DotMangledName(_) => {
                    let mangled_name_token = self.next_token(); // consume .inner token
                    let mangled_name = self
                        .parse_operand_or_type_hint(
                            OperandErrPosContext::MangledName,
                            TypeHintKind::Utf8,
                        )
                        .map_err(|e| self.diagnostic.push(*e))
                        .ok();

                    if let Some(mangled_name) = mangled_name {
                        mangled_name_dirs.push((mangled_name_token.span(), mangled_name));
                    }
                }
                RnsToken::DotInner(_) => unimplemented!("Not supported yet"),
                RnsToken::Eof(_) => break,
                _ => {
                    let unexpected_error = ParserError::UnexpectedToken(
                        UnexpectedTokenContext::InnerBody,
                        self.next_token(),
                    );
                    self.diagnostic.push(unexpected_error.into());
                    // TODO: test unknown_token .super/.method etc.
                    self.anchor(&[RnsTokenKind::DotMethod, RnsTokenKind::DotSuper]);
                }
            }
        }

        ParsedInnerDirective {
            dir_span: inner_token.span(),
            name: class_name,
            reported_errs,
            flags,
            super_directives,
            mangled_name_dirs,
        }
    }

    fn parse_class(&mut self) -> Result<(), Box<Diagnostic>> {
        self.class_dir.class_dir_span = self.anchor_class_directive()?;
        self.class_dir.access_flags = self.parse_class_access_flags();

        self.class_dir.class_name = self
            .parse_operand_or_type_hint(OperandErrPosContext::ClassName, TypeHintKind::Class)
            .map_err(|e| self.diagnostic.push(*e))
            .ok();

        self.verify_trailing_tokens(TrailingTokensErrContext::Class);

        while let Some(token) = self.tokens.peek() {
            match token {
                RnsToken::Newline(_) => {
                    self.next_token();
                }
                RnsToken::DotMethod(_) => {
                    // TODO: I shouldn't fail fast here, need try to anchor
                    let method_dir = self.parse_method()?;
                    self.class_dir.method_dirs.push(method_dir);
                }
                RnsToken::DotSuper(_) => {
                    if let Some(super_dir) = self.parse_super_directive() {
                        self.class_dir.super_directives.push(super_dir);
                    } else {
                        self.class_dir.reported_errs.insert(ErrKind::Super);
                    }
                }
                RnsToken::DotClassEnd(_) => {
                    self.next_token(); // consume .class_end
                    self.verify_trailing_tokens(TrailingTokensErrContext::ClassEnd);
                    break;
                }
                RnsToken::DotPackage(_) => self.parse_package_directive(),
                RnsToken::DotInner(_) => {
                    let inner = self.parse_inner();
                    self.class_dir.inner_classes.push(inner);
                }
                RnsToken::Eof(_) => break,
                _ => {
                    let unexpected_error = ParserError::UnexpectedToken(
                        UnexpectedTokenContext::ClassBody,
                        self.next_token(),
                    );
                    self.diagnostic.push(unexpected_error.into());
                    // TODO: test unknown_token .super/.method etc.
                    // TODO:
                    self.anchor(&[RnsTokenKind::DotMethod, RnsTokenKind::DotSuper]);
                }
            }
        }

        self.skip_newlines();
        let token_after_class = self.next_token();
        if !matches!(token_after_class, RnsToken::Eof(_)) {
            self.diagnostic.push(
                ParserError::UnexpectedToken(
                    UnexpectedTokenContext::AfterClassDefinition,
                    token_after_class,
                )
                .into(),
            );
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
        let mut super_directives = std::mem::take(&mut self.class_dir.super_directives);
        match super_directives.len() {
            0 => {
                // If no class name is defined, doesn't make sense to report missing superclass
                // If problem with super is already reported, don't report missing superclass to avoid duplicate errors
                if self.class_dir.class_name.is_some()
                    && !self.class_dir.reported_errs.contains(ErrKind::Super)
                {
                    self.diagnostic.push(
                        ParserWarning::MissingSuperClass {
                            class_name: self.class_dir.class_name.clone(),
                            class_dir_pos: self.class_dir.class_dir_span,
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
                let (dir_span, name) = super_directives.swap_remove(0);
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

    // TODO: clean up, very similar to take_super_directive
    fn take_inner_super_directive(
        &mut self,
        inner_dir: &mut ParsedInnerDirective,
    ) -> Option<SuperDirective> {
        let mut super_directives = std::mem::take(&mut inner_dir.super_directives);
        match super_directives.len() {
            0 => {
                // If no class name is defined, doesn't make sense to report missing superclass
                // If problem with super is already reported, don't report missing superclass to avoid duplicate errors
                if inner_dir.name.is_some() && !inner_dir.reported_errs.contains(ErrKind::Super) {
                    self.diagnostic.push(
                        // TODO: error should mention it is error in inner?
                        ParserWarning::MissingSuperClass {
                            class_name: inner_dir.name.clone(),
                            class_dir_pos: inner_dir.dir_span,
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
                let (dir_span, name) = super_directives.swap_remove(0);
                Some(SuperDirective {
                    dir_span: Some(dir_span),
                    name,
                })
            }
            _ => {
                // TODO: error should mention it is error in inner?
                self.diagnostic
                    .push(ParserError::MultipleSuperDefinitions(super_directives).into());
                None
            }
        }
    }

    fn take_package_directive(&mut self) -> Option<PackageDirective> {
        let mut package_directives = std::mem::take(&mut self.class_dir.package_directives);
        match package_directives.len() {
            0 => None,
            1 => {
                let (dir_span, name) = package_directives.swap_remove(0);
                Some(PackageDirective {
                    dir_span: Some(dir_span),
                    name,
                })
            }
            _ => {
                self.diagnostic
                    .push(ParserError::MultiplePackageDefinitions(package_directives).into());
                None
            }
        }
    }
}

// TODO: clean up
fn map_inner(
    instance: &mut RnsParser,
    mut parsed_inner_directive: ParsedInnerDirective,
) -> InnerClassDirective {
    let super_dir = instance.take_inner_super_directive(&mut parsed_inner_directive);
    let mangled_name_dir = {
        let mut mangled_vec = std::mem::take(&mut parsed_inner_directive.mangled_name_dirs);

        match mangled_vec.len() {
            0 => None,
            1 => Some(mangled_vec.swap_remove(0).1),
            _ => {
                instance
                    .diagnostic
                    .push(ParserError::MultipleMangledNames(mangled_vec).into());
                None
            }
        }
    };

    InnerClassDirective {
        dir_span: parsed_inner_directive.dir_span,
        name: parsed_inner_directive.name,
        super_dir,
        mangled_name_dir,
        flags: parsed_inner_directive.flags,
    }
}

pub fn parse(tokens: Vec<RnsToken>, eof_span: Span) -> Result<RnsModule, Vec<Diagnostic>> {
    let mut instance = RnsParser::from_tokens(tokens.into_iter().peekable(), eof_span);

    if let Err(e) = instance.parse_class() {
        instance.diagnostic.push(*e);
        return Err(instance.diagnostic);
    }

    let parsed_inner_classes = std::mem::take(&mut instance.class_dir.inner_classes);
    let inner_classes = parsed_inner_classes
        .into_iter()
        .map(|c| map_inner(&mut instance, c))
        .collect();

    let package = instance.take_package_directive();
    let super_dir = instance.take_super_directive();
    let class_dir = ClassDirective {
        dir_span: instance.class_dir.class_dir_span,
        name: instance.class_dir.class_name.take(),
        flags: instance.class_dir.access_flags,
    };

    Ok(RnsModule {
        package,
        class_dir,
        super_dir,
        diagnostics: instance.diagnostic,
        methods: instance.class_dir.method_dirs,
        inner_classes,
    })
}
