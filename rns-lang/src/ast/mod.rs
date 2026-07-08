pub mod flag;

use crate::ast::flag::{RnsInnerFlag, RnsMethodFlag};
use crate::diagnostic::Diagnostic;
use crate::instruction::{InstructionNumericOperand, InstructionSpec};
use crate::token::type_hint::TypeHint;
use crate::token::{Span, Spanned};
use flag::RnsClassFlag;
use std::collections::HashMap;

pub struct RnsModule {
    pub package: Option<PackageDirective>,
    pub class_dir: ClassDirective,
    pub super_dir: Option<SuperDirective>,
    pub diagnostics: Vec<Diagnostic>,
    pub methods: Vec<MethodDirective>,
    pub inner_classes: Vec<InnerClassDirective>,
}

pub struct PackageDirective {
    pub dir_span: Option<Span>,
    pub name: String,
}

pub struct SuperDirective {
    pub dir_span: Option<Span>,
    pub name: TypeHint,
}

pub struct ClassDirective {
    pub dir_span: Span,
    pub name: Option<TypeHint>,
    pub flags: HashMap<RnsClassFlag, Span>,
}

pub struct InnerClassDirective {
    pub dir_span: Span,
    pub name: Option<TypeHint>,
    pub flags: HashMap<RnsInnerFlag, Span>,
}

pub struct MethodDirective {
    pub dir_span: Span,
    pub name: Option<TypeHint>,
    pub descriptor: Option<TypeHint>,
    pub flags: HashMap<RnsMethodFlag, Span>,
    pub code_dir: Option<CodeDirective>,
}

impl MethodDirective {
    pub fn new(dir_span: Span, flags: HashMap<RnsMethodFlag, Span>) -> Self {
        Self {
            dir_span,
            flags,
            name: None,
            descriptor: None,
            code_dir: None,
        }
    }
}

pub enum RnsOperand {
    CpRef(TypeHint),
    Numeric(InstructionNumericOperand, Spanned<i64>),
    Label(Spanned<String>),
}

pub struct RnsInstruction {
    pub ins_span: Span,
    pub spec: InstructionSpec,
    pub operand: Option<RnsOperand>,
}

impl RnsInstruction {
    pub fn new(ins_span: Span, spec: InstructionSpec, operand: RnsOperand) -> Self {
        Self {
            ins_span,
            spec,
            operand: Some(operand),
        }
    }

    pub fn new_without_operand(ins_span: Span, spec: InstructionSpec) -> Self {
        Self {
            ins_span,
            spec,
            operand: None,
        }
    }
}

pub struct CodeDirective {
    pub dir_span: Span,
    pub instructions: Vec<RnsInstruction>,
    pub labels: HashMap<String, u32>,
    pub max_stack: u16,
    pub max_locals: u16,
}
