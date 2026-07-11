use crate::token::flag::RnsFlag;
use crate::token::type_hint::TypeHintKind;
use crate::token::{
    DIRECTIVE_DOT_ANNOTATION, DIRECTIVE_DOT_CLASS, DIRECTIVE_DOT_CLASS_END, DIRECTIVE_DOT_CODE,
    DIRECTIVE_DOT_CODE_END, DIRECTIVE_DOT_FLAGS, DIRECTIVE_DOT_INNER, DIRECTIVE_DOT_INNER_CLASS,
    DIRECTIVE_DOT_INNER_CLASSES_ATTR, DIRECTIVE_DOT_INNER_CLASSES_ATTR_END,
    DIRECTIVE_DOT_INNER_END, DIRECTIVE_DOT_INNER_NAME, DIRECTIVE_DOT_MANGLED_NAME,
    DIRECTIVE_DOT_METHOD, DIRECTIVE_DOT_METHOD_END, DIRECTIVE_DOT_OUTER_CLASS,
    DIRECTIVE_DOT_PACKAGE, DIRECTIVE_DOT_SUPER, TOKEN_TYPE_EOF, TOKEN_TYPE_IDENTIFIER,
    TOKEN_TYPE_LABEL, TOKEN_TYPE_NEWLINE,
};
use std::fmt::{Display, Formatter};

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum RnsTokenKind {
    DotClass,
    DotClassEnd,
    DotInner,
    DotInnerEnd,
    DotInnerClassesAttr,
    DotInnerClassesAttrEnd,
    DotInnerClass,
    DotOuterClass,
    DotFlags,
    DotInnerName,
    DotSuper,
    DotMethod,
    DotMethodEnd,
    DotPackage,
    DotCode,
    DotCodeEnd,
    DotAnnotation,
    DotMangledName,
    AccessFlag(RnsFlag),
    TypeHint(TypeHintKind),
    Label,
    Identifier,
    Newline,
    Eof,
}

impl RnsTokenKind {
    pub fn token_name(&self) -> &'static str {
        match self {
            RnsTokenKind::DotClass => DIRECTIVE_DOT_CLASS,
            RnsTokenKind::DotClassEnd => DIRECTIVE_DOT_CLASS_END,
            RnsTokenKind::DotInner => DIRECTIVE_DOT_INNER,
            RnsTokenKind::DotInnerEnd => DIRECTIVE_DOT_INNER_END,
            RnsTokenKind::DotInnerClassesAttr => DIRECTIVE_DOT_INNER_CLASSES_ATTR,
            RnsTokenKind::DotInnerClassesAttrEnd => DIRECTIVE_DOT_INNER_CLASSES_ATTR_END,
            RnsTokenKind::DotInnerClass => DIRECTIVE_DOT_INNER_CLASS,
            RnsTokenKind::DotOuterClass => DIRECTIVE_DOT_OUTER_CLASS,
            RnsTokenKind::DotFlags => DIRECTIVE_DOT_FLAGS,
            RnsTokenKind::DotInnerName => DIRECTIVE_DOT_INNER_NAME,
            RnsTokenKind::DotSuper => DIRECTIVE_DOT_SUPER,
            RnsTokenKind::DotPackage => DIRECTIVE_DOT_PACKAGE,
            RnsTokenKind::DotMethod => DIRECTIVE_DOT_METHOD,
            RnsTokenKind::DotMethodEnd => DIRECTIVE_DOT_METHOD_END,
            RnsTokenKind::DotCode => DIRECTIVE_DOT_CODE,
            RnsTokenKind::DotCodeEnd => DIRECTIVE_DOT_CODE_END,
            RnsTokenKind::DotMangledName => DIRECTIVE_DOT_MANGLED_NAME,
            RnsTokenKind::DotAnnotation => DIRECTIVE_DOT_ANNOTATION,
            RnsTokenKind::AccessFlag(flag) => flag.token_name(),
            RnsTokenKind::TypeHint(type_hint) => type_hint.token_name(),
            RnsTokenKind::Identifier => TOKEN_TYPE_IDENTIFIER,
            RnsTokenKind::Label => TOKEN_TYPE_LABEL,
            RnsTokenKind::Newline => TOKEN_TYPE_NEWLINE,
            RnsTokenKind::Eof => TOKEN_TYPE_EOF,
        }
    }
}

impl Display for RnsTokenKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.token_name())
    }
}
