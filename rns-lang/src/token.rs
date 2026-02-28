use itertools::Itertools;
use std::fmt::Display;
use std::ops::Range;

#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash, Ord, PartialOrd)]
pub enum RnsFlag {
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
pub enum RnsTokenKind {
    // directives
    DotClass,
    DotSuper,
    DotMethod,
    DotCode,
    DotEnd,
    DotAnnotation,

    AccessFlag(RnsFlag),

    Identifier(String),
    Integer(i32),
    StringLiteral(String),
    Newline,
    Eof,
}

impl TryFrom<&str> for RnsFlag {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "public" => Ok(RnsFlag::Public),
            "static" => Ok(RnsFlag::Static),
            "final" => Ok(RnsFlag::Final),
            "super" => Ok(RnsFlag::Super),
            "interface" => Ok(RnsFlag::Interface),
            "abstract" => Ok(RnsFlag::Abstract),
            "enum" => Ok(RnsFlag::Enum),
            "synthetic" => Ok(RnsFlag::Synthetic),
            "annotation" => Ok(RnsFlag::Annotation),
            "module" => Ok(RnsFlag::Module),
            _ => Err(()),
        }
    }
}

impl Display for RnsFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RnsFlag::Public => write!(f, "public"),
            RnsFlag::Static => write!(f, "static"),
            RnsFlag::Final => write!(f, "final"),
            RnsFlag::Super => write!(f, "super"),
            RnsFlag::Interface => write!(f, "interface"),
            RnsFlag::Abstract => write!(f, "abstract"),
            RnsFlag::Enum => write!(f, "enum"),
            RnsFlag::Synthetic => write!(f, "synthetic"),
            RnsFlag::Annotation => write!(f, "annotation"),
            RnsFlag::Module => write!(f, "module"),
        }
    }
}

impl RnsFlag {
    pub fn jvm_spec_name(&self) -> &'static str {
        match self {
            RnsFlag::Interface => "ACC_INTERFACE",
            RnsFlag::Abstract => "ACC_ABSTRACT",
            RnsFlag::Enum => "ACC_ENUM",
            RnsFlag::Module => "ACC_MODULE",
            RnsFlag::Public => "ACC_PUBLIC",
            RnsFlag::Static => "ACC_STATIC",
            RnsFlag::Final => "ACC_FINAL",
            RnsFlag::Super => "ACC_SUPER",
            RnsFlag::Synthetic => "ACC_SYNTHETIC",
            RnsFlag::Annotation => "ACC_ANNOTATION",
        }
    }
}

impl RnsTokenKind {
    pub const DIRECTIVES: &[Self] = &[
        RnsTokenKind::DotClass,
        RnsTokenKind::DotSuper,
        RnsTokenKind::DotMethod,
        RnsTokenKind::DotEnd,
        RnsTokenKind::DotCode,
        RnsTokenKind::DotAnnotation,
    ];

    // TODO: I don't want to search in DIRECTIVES, but this one should covered with tests to not miss any directive.
    pub fn is_directive(&self) -> bool {
        matches!(
            self,
            RnsTokenKind::DotClass
                | RnsTokenKind::DotSuper
                | RnsTokenKind::DotMethod
                | RnsTokenKind::DotEnd
                | RnsTokenKind::DotCode
                | RnsTokenKind::DotAnnotation
        )
    }

    pub fn is_class_nested_directive(&self) -> bool {
        matches!(
            self,
            RnsTokenKind::DotMethod | RnsTokenKind::DotAnnotation | RnsTokenKind::DotSuper
        )
    }

    pub fn is_method_nested_directive(&self) -> bool {
        matches!(self, RnsTokenKind::DotCode | RnsTokenKind::DotAnnotation)
    }

    pub fn is_access_flag(&self) -> bool {
        matches!(self, RnsTokenKind::AccessFlag(_))
    }

    pub fn from_directive(name: &str) -> Option<Self> {
        match name {
            "class" => Some(RnsTokenKind::DotClass),
            "super" => Some(RnsTokenKind::DotSuper),
            "method" => Some(RnsTokenKind::DotMethod),
            "end" => Some(RnsTokenKind::DotEnd),
            "code" => Some(RnsTokenKind::DotCode),
            "annotation" => Some(RnsTokenKind::DotAnnotation),
            _ => None,
        }
    }

    pub fn from_identifier(name: String) -> Self {
        if let Ok(access_flag) = RnsFlag::try_from(name.as_str()) {
            RnsTokenKind::AccessFlag(access_flag)
        } else {
            RnsTokenKind::Identifier(name)
        }
    }

    pub fn list_directives() -> String {
        Self::DIRECTIVES.iter().map(ToString::to_string).join(", ")
    }

    pub fn as_string_token_type(&self) -> String {
        match self {
            RnsTokenKind::DotClass
            | RnsTokenKind::DotSuper
            | RnsTokenKind::DotMethod
            | RnsTokenKind::DotEnd
            | RnsTokenKind::DotAnnotation
            | RnsTokenKind::DotCode => "directive".to_string(),
            RnsTokenKind::AccessFlag(_) => "keyword".to_string(), // TODO: keywords or access flags?
            RnsTokenKind::Identifier(_) => "identifier".to_string(),
            RnsTokenKind::StringLiteral(_) => "string literal".to_string(),
            RnsTokenKind::Integer(_) => "integer".to_string(),
            RnsTokenKind::Newline => "newline".to_string(),
            RnsTokenKind::Eof => "eof".to_string(),
        }
    }
}

impl Display for RnsTokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RnsTokenKind::DotClass => write!(f, ".class"),
            RnsTokenKind::DotSuper => write!(f, ".super"),
            RnsTokenKind::DotMethod => write!(f, ".method"),
            RnsTokenKind::DotEnd => write!(f, ".end"),
            RnsTokenKind::DotCode => write!(f, ".code"),
            RnsTokenKind::DotAnnotation => write!(f, ".annotation"),
            RnsTokenKind::Newline => write!(f, "newline"),
            RnsTokenKind::Eof => write!(f, "eof"),
            RnsTokenKind::AccessFlag(flag) => write!(f, "{}", flag),
            RnsTokenKind::Identifier(name) => write!(f, "{}", name.escape_default()),
            RnsTokenKind::StringLiteral(value) => {
                write!(f, "{}", value.escape_default())
            }
            RnsTokenKind::Integer(value) => write!(f, "{}", value),
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
pub struct RnsToken {
    pub(crate) kind: RnsTokenKind,
    pub(crate) span: Span,
}
