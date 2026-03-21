pub(crate) use crate::token::flag::RnsFlag;
pub(crate) use crate::token::kind::RnsTokenKind;
pub(crate) use crate::token::span::{Span, Spanned};
use crate::token::type_hint::TypeHintKind;
use std::fmt::{Display, Formatter};

pub mod flag;
mod kind;
pub(crate) mod span;
pub mod type_hint;

pub const DIRECTIVE_DOT_CLASS: &str = ".class";
pub const DIRECTIVE_DOT_SUPER: &str = ".super";
pub const DIRECTIVE_DOT_METHOD: &str = ".method";
pub const DIRECTIVE_DOT_END: &str = ".end";
pub const DIRECTIVE_DOT_CODE: &str = ".code";
pub const DIRECTIVE_DOT_ANNOTATION: &str = ".annotation";

pub const DIRECTIVE_CLASS: &str = "class";
pub const DIRECTIVE_SUPER: &str = "super";
pub const DIRECTIVE_METHOD: &str = "method";
pub const DIRECTIVE_END: &str = "end";
pub const DIRECTIVE_CODE: &str = "code";
pub const DIRECTIVE_ANNOTATION: &str = "annotation";

pub const FLAG_PUBLIC: &str = "public";
pub const FLAG_STATIC: &str = "static";
pub const FLAG_FINAL: &str = "final";
pub const FLAG_SUPER: &str = "super";
pub const FLAG_INTERFACE: &str = "interface";
pub const FLAG_ABSTRACT: &str = "abstract";
pub const FLAG_ENUM: &str = "enum";
pub const FLAG_SYNTHETIC: &str = "synthetic";
pub const FLAG_ANNOTATION: &str = "annotation";
pub const FLAG_MODULE: &str = "module";
pub const FLAG_PRIVATE: &str = "private";
pub const FLAG_PROTECTED: &str = "protected";
pub const FLAG_SYNCHRONIZED: &str = "synchronized";
pub const FLAG_BRIDGE: &str = "bridge";
pub const FLAG_VARARGS: &str = "varargs";
pub const FLAG_NATIVE: &str = "native";
pub const FLAG_STRICT: &str = "strict";

pub const TOKEN_TYPE_DIRECTIVE: &str = "directive";
pub const TOKEN_TYPE_ACCESS_FLAG: &str = "access flag";
pub const TOKEN_TYPE_IDENTIFIER: &str = "identifier";
pub const TOKEN_TYPE_NEWLINE: &str = "newline";
pub const TOKEN_TYPE_EOF: &str = "eof";
pub const TOKEN_TYPE_TYPE_HINT: &str = "type hint";

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum RnsTokenContext {
    ClassDefinition,
    ClassBody,
    MethodDefinition,
    MethodBody,
    CodeBody,
    Operand,
    TopLevel,
    Contextless,
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
    Newline(Span),
    Eof(Span),
}

impl Display for RnsTokenContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RnsTokenContext::ClassDefinition => write!(f, "class definition"),
            RnsTokenContext::ClassBody => write!(f, "class body"),
            RnsTokenContext::MethodDefinition => write!(f, "method definition"),
            RnsTokenContext::MethodBody => write!(f, "method body"),
            RnsTokenContext::CodeBody => write!(f, "code body"),
            RnsTokenContext::Operand => write!(f, "operand"),
            RnsTokenContext::TopLevel => write!(f, "file top level"),
            RnsTokenContext::Contextless => write!(f, "everywhere"),
        }
    }
}

impl RnsToken {
    pub fn token_name(&self) -> &'static str {
        match self {
            RnsToken::DotClass(_) => DIRECTIVE_DOT_CLASS,
            RnsToken::DotSuper(_) => DIRECTIVE_DOT_SUPER,
            RnsToken::DotMethod(_) => DIRECTIVE_DOT_METHOD,
            RnsToken::DotEnd(_) => DIRECTIVE_DOT_END,
            RnsToken::DotCode(_) => DIRECTIVE_DOT_CODE,
            RnsToken::DotAnnotation(_) => DIRECTIVE_DOT_ANNOTATION,
            RnsToken::AccessFlag(spanned) => spanned.value.token_name(),
            RnsToken::TypeHint(spanned) => spanned.value.token_name(),
            RnsToken::Identifier(_) => TOKEN_TYPE_IDENTIFIER,
            RnsToken::Newline(_) => TOKEN_TYPE_NEWLINE,
            RnsToken::Eof(_) => TOKEN_TYPE_EOF,
        }
    }
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
                RnsTokenContext::ClassDefinition,
                RnsTokenContext::MethodDefinition,
            ],
            RnsToken::Newline(_) | RnsToken::Eof(_) => &[RnsTokenContext::Contextless],
            RnsToken::TypeHint(_) | RnsToken::Identifier(_) => &[RnsTokenContext::Operand],
        }
    }

    pub fn kind(&self) -> RnsTokenKind {
        match self {
            RnsToken::DotClass(_) => RnsTokenKind::DotClass,
            RnsToken::DotSuper(_) => RnsTokenKind::DotSuper,
            RnsToken::DotMethod(_) => RnsTokenKind::DotMethod,
            RnsToken::DotEnd(_) => RnsTokenKind::DotEnd,
            RnsToken::DotCode(_) => RnsTokenKind::DotCode,
            RnsToken::DotAnnotation(_) => RnsTokenKind::DotAnnotation,
            RnsToken::AccessFlag(spanned) => RnsTokenKind::AccessFlag(spanned.value),
            RnsToken::TypeHint(spanned) => RnsTokenKind::TypeHint(spanned.value),
            RnsToken::Identifier(_) => RnsTokenKind::Identifier,
            RnsToken::Newline(_) => RnsTokenKind::Newline,
            RnsToken::Eof(_) => RnsTokenKind::Eof,
        }
    }

    pub fn is_line_terminator(&self) -> bool {
        matches!(self, RnsToken::Newline(_) | RnsToken::Eof(_))
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
            DIRECTIVE_CLASS => Some(RnsToken::DotClass(span)),
            DIRECTIVE_SUPER => Some(RnsToken::DotSuper(span)),
            DIRECTIVE_METHOD => Some(RnsToken::DotMethod(span)),
            DIRECTIVE_END => Some(RnsToken::DotEnd(span)),
            DIRECTIVE_CODE => Some(RnsToken::DotCode(span)),
            DIRECTIVE_ANNOTATION => Some(RnsToken::DotAnnotation(span)),
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
            | RnsToken::DotCode(_) => TOKEN_TYPE_DIRECTIVE,
            RnsToken::AccessFlag(_) => TOKEN_TYPE_ACCESS_FLAG,
            RnsToken::Identifier(_) => TOKEN_TYPE_IDENTIFIER,
            RnsToken::Newline(_) => TOKEN_TYPE_NEWLINE,
            RnsToken::Eof(_) => TOKEN_TYPE_EOF,
            RnsToken::TypeHint(_) => TOKEN_TYPE_TYPE_HINT,
        }
    }

    pub fn as_identifier(&self) -> &str {
        match self {
            RnsToken::Identifier(spanned) => &spanned.value,
            _ => self.token_name(),
        }
    }
}

impl Display for RnsToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.token_name())
    }
}
