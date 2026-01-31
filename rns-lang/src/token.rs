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

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    // TODO: support multi-line spans and multi-line syntax (e.g., multi-line strings) in the future
}

impl Span {
    pub fn new(start: usize, end: usize, line: usize) -> Self {
        Self { start, end, line }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct JasmToken {
    pub(crate) kind: JasmTokenKind,
    pub(crate) span: Span,
}
