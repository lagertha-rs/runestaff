use itertools::Itertools;
use std::ops::Range;

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum JasmTokenKind {
    DotClass,
    DotSuper,
    DotMethod,
    DotCode,
    DotEnd,
    DotLimit,

    Public,
    Static,
    Identifier(String),

    Integer(i32),
    StringLiteral(String),
    OpenParen,
    CloseParen,
    OpenBracket,
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
        JasmTokenKind::DotCode,
    ];

    pub fn from_directive(name: &str) -> Option<Self> {
        match name {
            "class" => Some(JasmTokenKind::DotClass),
            "super" => Some(JasmTokenKind::DotSuper),
            "method" => Some(JasmTokenKind::DotMethod),
            "end" => Some(JasmTokenKind::DotEnd),
            "limit" => Some(JasmTokenKind::DotLimit),
            "code" => Some(JasmTokenKind::DotCode),
            _ => None,
        }
    }

    pub fn from_identifier(name: String) -> Self {
        match name.as_str() {
            "public" => JasmTokenKind::Public,
            "static" => JasmTokenKind::Static,
            _ => JasmTokenKind::Identifier(name),
        }
    }

    pub fn list_directives() -> String {
        Self::DIRECTIVES.iter().map(ToString::to_string).join(", ")
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
            JasmTokenKind::DotCode => write!(f, ".code"),
            JasmTokenKind::Newline => write!(f, "newline"),
            JasmTokenKind::Eof => write!(f, "eof"),
            JasmTokenKind::Public => write!(f, "public"),
            JasmTokenKind::Static => write!(f, "static"),
            JasmTokenKind::Identifier(name) => write!(f, "identifier({})", name),
            JasmTokenKind::StringLiteral(value) => write!(f, "string_literal(\"{}\")", value),
            JasmTokenKind::OpenParen => write!(f, "("),
            JasmTokenKind::CloseParen => write!(f, ")"),
            JasmTokenKind::OpenBracket => write!(f, "["),
            JasmTokenKind::Integer(value) => write!(f, "integer({})", value),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Span {
    pub start: usize,
    pub end: usize, // is exclusive
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
