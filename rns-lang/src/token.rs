use itertools::Itertools;
use std::fmt::Display;
use std::ops::Range;

#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash)]
pub enum JasmAccessFlag {
    Public,
    Static,
    Final,
    Super,
    Interface,
    Abstract,
    Enum,
    Synthetic,
    Annotation,
    Module,
}

//TODO: is it worth to use &str instead of String to avoid unnecessary cloning?
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum JasmTokenKind {
    // directives
    DotClass,
    DotSuper,
    DotMethod,
    DotCode,
    DotEnd,
    DotAnnotation,

    AccessFlag(JasmAccessFlag),

    Identifier(String),
    Integer(i32),
    StringLiteral(String),
    Newline,
    Eof,
}

impl TryFrom<&str> for JasmAccessFlag {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "public" => Ok(JasmAccessFlag::Public),
            "static" => Ok(JasmAccessFlag::Static),
            "final" => Ok(JasmAccessFlag::Final),
            "super" => Ok(JasmAccessFlag::Super),
            "interface" => Ok(JasmAccessFlag::Interface),
            "abstract" => Ok(JasmAccessFlag::Abstract),
            "enum" => Ok(JasmAccessFlag::Enum),
            "synthetic" => Ok(JasmAccessFlag::Synthetic),
            "annotation" => Ok(JasmAccessFlag::Annotation),
            "module" => Ok(JasmAccessFlag::Module),
            _ => Err(()),
        }
    }
}

impl Display for JasmAccessFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JasmAccessFlag::Public => write!(f, "public"),
            JasmAccessFlag::Static => write!(f, "static"),
            JasmAccessFlag::Final => write!(f, "final"),
            JasmAccessFlag::Super => write!(f, "super"),
            JasmAccessFlag::Interface => write!(f, "interface"),
            JasmAccessFlag::Abstract => write!(f, "abstract"),
            JasmAccessFlag::Enum => write!(f, "enum"),
            JasmAccessFlag::Synthetic => write!(f, "synthetic"),
            JasmAccessFlag::Annotation => write!(f, "annotation"),
            JasmAccessFlag::Module => write!(f, "module"),
        }
    }
}

impl JasmTokenKind {
    pub const DIRECTIVES: &[Self] = &[
        JasmTokenKind::DotClass,
        JasmTokenKind::DotSuper,
        JasmTokenKind::DotMethod,
        JasmTokenKind::DotEnd,
        JasmTokenKind::DotCode,
        JasmTokenKind::DotAnnotation,
    ];

    // TODO: I don't want to search in DIRECTIVES, but this one should covered with tests to not miss any directive.
    pub fn is_directive(&self) -> bool {
        matches!(
            self,
            JasmTokenKind::DotClass
                | JasmTokenKind::DotSuper
                | JasmTokenKind::DotMethod
                | JasmTokenKind::DotEnd
                | JasmTokenKind::DotCode
                | JasmTokenKind::DotAnnotation
        )
    }

    pub fn is_class_nested_directive(&self) -> bool {
        matches!(
            self,
            JasmTokenKind::DotMethod | JasmTokenKind::DotAnnotation | JasmTokenKind::DotSuper
        )
    }

    pub fn is_method_nested_directive(&self) -> bool {
        matches!(self, JasmTokenKind::DotCode | JasmTokenKind::DotAnnotation)
    }

    pub fn is_access_flag(&self) -> bool {
        matches!(self, JasmTokenKind::AccessFlag(_))
    }

    pub fn from_directive(name: &str) -> Option<Self> {
        match name {
            "class" => Some(JasmTokenKind::DotClass),
            "super" => Some(JasmTokenKind::DotSuper),
            "method" => Some(JasmTokenKind::DotMethod),
            "end" => Some(JasmTokenKind::DotEnd),
            "code" => Some(JasmTokenKind::DotCode),
            "annotation" => Some(JasmTokenKind::DotAnnotation),
            _ => None,
        }
    }

    pub fn from_identifier(name: String) -> Self {
        if let Ok(access_flag) = JasmAccessFlag::try_from(name.as_str()) {
            JasmTokenKind::AccessFlag(access_flag)
        } else {
            JasmTokenKind::Identifier(name)
        }
    }

    pub fn list_directives() -> String {
        Self::DIRECTIVES.iter().map(ToString::to_string).join(", ")
    }

    pub fn as_string_token_type(&self) -> String {
        match self {
            JasmTokenKind::DotClass
            | JasmTokenKind::DotSuper
            | JasmTokenKind::DotMethod
            | JasmTokenKind::DotEnd
            | JasmTokenKind::DotAnnotation
            | JasmTokenKind::DotCode => "directive".to_string(),
            JasmTokenKind::AccessFlag(_) => "keyword".to_string(), // TODO: keywords or access flags?
            JasmTokenKind::Identifier(_) => "identifier".to_string(),
            JasmTokenKind::StringLiteral(_) => "string literal".to_string(),
            JasmTokenKind::Integer(_) => "integer".to_string(),
            JasmTokenKind::Newline => "newline".to_string(),
            JasmTokenKind::Eof => "eof".to_string(),
        }
    }
}

impl Display for JasmTokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JasmTokenKind::DotClass => write!(f, ".class"),
            JasmTokenKind::DotSuper => write!(f, ".super"),
            JasmTokenKind::DotMethod => write!(f, ".method"),
            JasmTokenKind::DotEnd => write!(f, ".end"),
            JasmTokenKind::DotCode => write!(f, ".code"),
            JasmTokenKind::DotAnnotation => write!(f, ".annotation"),
            JasmTokenKind::Newline => write!(f, "newline"),
            JasmTokenKind::Eof => write!(f, "eof"),
            JasmTokenKind::AccessFlag(flag) => write!(f, "{}", flag),
            JasmTokenKind::Identifier(name) => write!(f, "{}", name.escape_default()),
            JasmTokenKind::StringLiteral(value) => {
                write!(f, "{}", value.escape_default())
            }
            JasmTokenKind::Integer(value) => write!(f, "{}", value),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Default)]
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
