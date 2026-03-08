pub(crate) use crate::token::span::{Span, Spanned};
use crate::token::type_hint::TypeHintKind;
use std::fmt::{Display, Formatter};

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

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum RnsTokenContext {
    ClassDeclaration,
    ClassBody,
    MethodDeclaration,
    MethodBody,
    CodeBody,
    Operand,
    TopLevel,
    Contextless,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum RnsTokenKind {
    DotClass,
    DotSuper,
    DotMethod,
    DotEnd,
    DotCode,
    DotAnnotation,
    AccessFlag(RnsFlag),
    TypeHint(TypeHintKind),
    Identifier,
    Integer,
    StringLiteral,
    Newline,
    Eof,
}

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

impl Display for RnsTokenContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RnsTokenContext::ClassDeclaration => write!(f, "class declaration"),
            RnsTokenContext::ClassBody => write!(f, "class body"),
            RnsTokenContext::MethodDeclaration => write!(f, "method declaration"),
            RnsTokenContext::MethodBody => write!(f, "method body"),
            RnsTokenContext::CodeBody => write!(f, "code body"),
            RnsTokenContext::Operand => write!(f, "operand"),
            RnsTokenContext::TopLevel => write!(f, "file top level"),
            RnsTokenContext::Contextless => write!(f, "everywhere"),
        }
    }
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
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
    pub fn can_appear_in(&self) -> &[RnsTokenContext] {
        match self {
            RnsToken::DotClass(_) => &[RnsTokenContext::TopLevel],
            RnsToken::DotSuper(_) => &[RnsTokenContext::ClassBody],
            RnsToken::DotMethod(_) => &[RnsTokenContext::ClassBody],
            RnsToken::DotEnd(_) => &[
                RnsTokenContext::ClassBody,
                RnsTokenContext::MethodBody,
                RnsTokenContext::CodeBody,
            ],
            RnsToken::DotCode(_) => &[RnsTokenContext::MethodBody],
            RnsToken::DotAnnotation(_) => {
                &[RnsTokenContext::ClassBody, RnsTokenContext::MethodBody]
            }
            RnsToken::AccessFlag(_) => &[
                RnsTokenContext::ClassDeclaration,
                RnsTokenContext::MethodDeclaration,
            ],
            RnsToken::Newline(_) | RnsToken::Eof(_) => &[RnsTokenContext::Contextless],
            RnsToken::TypeHint(_)
            | RnsToken::Identifier(_)
            | RnsToken::Integer(_)
            | RnsToken::StringLiteral(_) => &[RnsTokenContext::Operand],
        }
    }

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

    pub fn matches_kind(&self, kind: RnsTokenKind) -> bool {
        match (self, kind) {
            (RnsToken::DotClass(_), RnsTokenKind::DotClass)
            | (RnsToken::DotSuper(_), RnsTokenKind::DotSuper)
            | (RnsToken::DotMethod(_), RnsTokenKind::DotMethod)
            | (RnsToken::DotEnd(_), RnsTokenKind::DotEnd)
            | (RnsToken::DotCode(_), RnsTokenKind::DotCode)
            | (RnsToken::DotAnnotation(_), RnsTokenKind::DotAnnotation) => true,
            (RnsToken::AccessFlag(spanned), RnsTokenKind::AccessFlag(expected_flag)) => {
                spanned.value == expected_flag
            }
            (RnsToken::TypeHint(spanned), RnsTokenKind::TypeHint(expected_hint)) => {
                spanned.value == expected_hint
            }
            (RnsToken::Identifier(_), RnsTokenKind::Identifier) => true,
            (RnsToken::Integer(_), RnsTokenKind::Integer) => true,
            (RnsToken::StringLiteral(_), RnsTokenKind::StringLiteral) => true,
            (RnsToken::Newline(_), RnsTokenKind::Newline) => true,
            (RnsToken::Eof(_), RnsTokenKind::Eof) => true,
            _ => false,
        }
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

    pub fn token_type(&self) -> &'static str {
        match self {
            RnsToken::DotClass(_)
            | RnsToken::DotSuper(_)
            | RnsToken::DotMethod(_)
            | RnsToken::DotEnd(_)
            | RnsToken::DotAnnotation(_)
            | RnsToken::DotCode(_) => "directive",
            RnsToken::AccessFlag(_) => "access flag",
            RnsToken::Identifier(_) => "identifier",
            RnsToken::StringLiteral(_) => "string literal",
            RnsToken::Integer(_) => "integer",
            RnsToken::Newline(_) => "newline",
            RnsToken::Eof(_) => "eof",
            RnsToken::TypeHint(_) => "type hint",
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
