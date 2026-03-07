pub(crate) use crate::token::span::{Span, Spanned};
use crate::token::type_hint::TypeHintKind;
use std::fmt::Display;

pub(crate) mod span;
pub(crate) mod type_hint;

#[derive(Debug, Eq, PartialEq, Clone, Copy, Ord, PartialOrd)]
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
// Since the source code is lives the whole time of the parsing we can use &str, but it will require some lifetime annotations
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum RnsToken {
    // directives
    DotClass(Span),
    DotSuper(Span),
    DotMethod(Span),
    DotCode(Span),
    DotEnd(Span),
    DotAnnotation(Span),

    AccessFlag(Spanned<RnsFlag>),
    TypeHint(Spanned<TypeHintKind>),

    Identifier(Spanned<String>),
    Integer(Spanned<i32>),
    StringLiteral(Spanned<String>),
    Newline(Span),
    Eof(Span),
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

impl RnsToken {
    // TODO: I don't want to search in DIRECTIVES, but this one should covered with tests to not miss any directive.
    pub fn is_directive(&self) -> bool {
        matches!(
            self,
            RnsToken::DotClass(_)
                | RnsToken::DotSuper(_)
                | RnsToken::DotMethod(_)
                | RnsToken::DotEnd(_)
                | RnsToken::DotCode(_)
                | RnsToken::DotAnnotation(_)
        )
    }

    pub fn is_class_nested_directive(&self) -> bool {
        matches!(
            self,
            RnsToken::DotMethod(_) | RnsToken::DotAnnotation(_) | RnsToken::DotSuper(_)
        )
    }

    pub fn is_method_nested_directive(&self) -> bool {
        matches!(self, RnsToken::DotCode(_) | RnsToken::DotAnnotation(_))
    }

    pub fn is_access_flag(&self) -> bool {
        matches!(self, RnsToken::AccessFlag(_))
    }

    pub fn from_directive(name: &str, span: Span) -> Option<Self> {
        match name {
            "class" => Some(RnsToken::DotClass(span)),
            "super" => Some(RnsToken::DotSuper(span)),
            "method" => Some(RnsToken::DotMethod(span)),
            "end" => Some(RnsToken::DotEnd(span)),
            "code" => Some(RnsToken::DotCode(span)),
            "annotation" => Some(RnsToken::DotAnnotation(span)),
            _ => None,
        }
    }

    pub fn from_identifier(name: String, span: Span) -> Self {
        if let Ok(access_flag) = RnsFlag::try_from(name.as_str()) {
            RnsToken::AccessFlag(Spanned::new(access_flag, span))
        } else {
            RnsToken::Identifier(Spanned::new(name, span))
        }
    }

    pub fn span(&self) -> Span {
        match self {
            RnsToken::DotClass(span)
            | RnsToken::DotSuper(span)
            | RnsToken::DotMethod(span)
            | RnsToken::DotEnd(span)
            | RnsToken::DotCode(span)
            | RnsToken::DotAnnotation(span)
            | RnsToken::Newline(span)
            | RnsToken::Eof(span) => *span,
            RnsToken::AccessFlag(spanned) => spanned.span,
            RnsToken::Identifier(spanned) => spanned.span,
            RnsToken::StringLiteral(spanned) => spanned.span,
            RnsToken::Integer(spanned) => spanned.span,
            RnsToken::TypeHint(spanned) => spanned.span,
        }
    }

    pub fn as_string_token_type(&self) -> String {
        match self {
            RnsToken::DotClass(_)
            | RnsToken::DotSuper(_)
            | RnsToken::DotMethod(_)
            | RnsToken::DotEnd(_)
            | RnsToken::DotAnnotation(_)
            | RnsToken::DotCode(_) => "directive".to_string(),
            RnsToken::AccessFlag(_) => "keyword".to_string(), // TODO: keywords or access flags?
            RnsToken::Identifier(_) => "identifier".to_string(),
            RnsToken::StringLiteral(_) => "string literal".to_string(),
            RnsToken::Integer(_) => "integer".to_string(),
            RnsToken::Newline(_) => "newline".to_string(),
            RnsToken::Eof(_) => "eof".to_string(),
            RnsToken::TypeHint(_) => "type hint".to_string(),
        }
    }
}

impl Display for RnsToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RnsToken::DotClass(_) => write!(f, ".class"),
            RnsToken::DotSuper(_) => write!(f, ".super"),
            RnsToken::DotMethod(_) => write!(f, ".method"),
            RnsToken::DotEnd(_) => write!(f, ".end"),
            RnsToken::DotCode(_) => write!(f, ".code"),
            RnsToken::DotAnnotation(_) => write!(f, ".annotation"),
            RnsToken::Newline(_) => write!(f, "newline"),
            RnsToken::Eof(_) => write!(f, "eof"),
            RnsToken::AccessFlag(spanned) => write!(f, "{}", spanned.value),
            RnsToken::Identifier(spanned) => write!(f, "{}", spanned.value.escape_default()),
            RnsToken::StringLiteral(spanned) => {
                write!(f, "{}", spanned.value.escape_default())
            }
            RnsToken::Integer(spanned) => write!(f, "{}", spanned.value),
            RnsToken::TypeHint(spanned) => write!(f, "{}", spanned.value),
        }
    }
}
