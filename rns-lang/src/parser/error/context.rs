use crate::token::Spanned;
use crate::token::type_hint::TypeHintKind;
use std::fmt::{Display, Formatter};

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub(in crate::parser) enum AccessFlagContext {
    Class,
    Method,
}

impl Display for AccessFlagContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AccessFlagContext::Class => write!(f, "class"),
            AccessFlagContext::Method => write!(f, "method"),
        }
    }
}

impl AccessFlagContext {
    pub(in crate::parser) fn error_code(&self) -> &'static str {
        match self {
            AccessFlagContext::Class => "E-015",
            AccessFlagContext::Method => "E-016",
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub(in crate::parser) enum OperandErrPosContext {
    ClassName,
    SuperName,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub(in crate::parser) enum TrailingTokensErrContext {
    Class,
    Super,
    TypeHint(Spanned<TypeHintKind>),
}

impl Display for TrailingTokensErrContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TrailingTokensErrContext::Class => write!(f, "class definition"),
            TrailingTokensErrContext::Super => write!(f, "super class definition"),
            TrailingTokensErrContext::TypeHint(kind) => {
                write!(f, "type hint '{}'", kind.value.token_name())
            }
        }
    }
}

impl Display for OperandErrPosContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            OperandErrPosContext::ClassName => write!(f, "class name"),
            OperandErrPosContext::SuperName => write!(f, "super class name"),
        }
    }
}

impl OperandErrPosContext {
    pub(in crate::parser) fn expected_type_hint_kinds(&self) -> Vec<TypeHintKind> {
        match self {
            OperandErrPosContext::ClassName | OperandErrPosContext::SuperName => {
                vec![TypeHintKind::Class]
            }
        }
    }

    pub(in crate::parser) fn directive_name(&self) -> &'static str {
        match self {
            //TODO: use something like TokenKind::DotClass.name() to not hardcode here
            OperandErrPosContext::ClassName => ".class",
            OperandErrPosContext::SuperName => ".super",
        }
    }
}
