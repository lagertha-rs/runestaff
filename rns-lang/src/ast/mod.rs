pub mod flag;

use crate::ast::flag::RnsMethodFlag;
use crate::diagnostic::Diagnostic;
use crate::instruction::InstructionSpec;
use crate::token::Span;
use crate::token::type_hint::TypeHint;
use flag::RnsClassFlag;
use std::collections::BTreeMap;

pub struct RnsModule {
    pub class_dir: ClassDirective,
    pub super_dir: Option<SuperDirective>,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct SuperDirective {
    pub dir_span: Option<Span>,
    pub name: TypeHint,
}

pub struct ClassDirective {
    pub dir_span: Span,
    pub name: Option<TypeHint>,
    // TODO: BTreeMap because I need it to be sorted for my snapshot test. investigate impact
    pub flags: BTreeMap<RnsClassFlag, Span>,
}

pub struct MethodDirective {
    pub dir_span: Span,
    pub name: Option<TypeHint>,
    pub descriptor: Option<TypeHint>,
    pub flags: BTreeMap<RnsMethodFlag, Span>,
    pub code_dir: Option<CodeDirective>,
}

impl MethodDirective {
    pub fn new(dir_span: Span, flags: BTreeMap<RnsMethodFlag, Span>) -> Self {
        Self {
            dir_span,
            flags,
            name: None,
            descriptor: None,
            code_dir: None,
        }
    }
}

pub struct RnsInstruction {
    pub ins_span: Span,
    pub spec: InstructionSpec,
    pub operand: Option<TypeHint>,
}

pub struct CodeDirective {
    pub dir_span: Span,
}
