use std::ops::Range;

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum JasmTokenKind {
    DotClass,
    DotSuper,
    DotMethod,
    DotEnd,
    DotLimit,

    Public,
    Static,
    Identifier(String),

    Integer(i32),
    StringLiteral(String),

    Newline,
    Eof,
}

impl JasmTokenKind {
    pub const DIRECTIVES: &[Self] = &[
        JasmTokenKind::DotClass,
        JasmTokenKind::DotSuper,
        JasmTokenKind::DotMethod,
        JasmTokenKind::DotEnd,
        JasmTokenKind::DotLimit,
    ];

    pub fn try_directive(name: &str) -> Option<Self> {
        match name {
            "class" => Some(JasmTokenKind::DotClass),
            "super" => Some(JasmTokenKind::DotSuper),
            "method" => Some(JasmTokenKind::DotMethod),
            "end" => Some(JasmTokenKind::DotEnd),
            "limit" => Some(JasmTokenKind::DotLimit),
            _ => None,
        }
    }
}

impl std::fmt::Display for JasmTokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JasmTokenKind::DotClass => write!(f, ".class"),
            JasmTokenKind::DotSuper => write!(f, ".super"),
            JasmTokenKind::DotMethod => write!(f, ".method"),
            JasmTokenKind::DotEnd => write!(f, ".end"),
            JasmTokenKind::DotLimit => write!(f, ".limit"),
            JasmTokenKind::Public => write!(f, "public"),
            JasmTokenKind::Static => write!(f, "static"),
            JasmTokenKind::Identifier(name) => write!(f, "identifier({})", name),
            JasmTokenKind::Integer(value) => write!(f, "integer({})", value),
            JasmTokenKind::StringLiteral(value) => write!(f, "string_literal({})", value),
            JasmTokenKind::Newline => write!(f, "newline"),
            JasmTokenKind::Eof => write!(f, "eof"),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub(crate) fn as_range(&self) -> Range<usize> {
        self.start..self.end
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct JasmToken {
    pub(crate) kind: JasmTokenKind,
    pub(crate) span: Span,
}
