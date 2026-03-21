pub mod flag;

use crate::diagnostic::Diagnostic;
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

pub struct MethodDirective {}

pub struct CodeDirective {}
