use crate::diagnostic::{
    ERR_CODE_CLASS_DEF_TRAILING_TOK, ERR_CODE_CLASS_END_TRAILING_TOK, ERR_CODE_FLAGS_TRAILING_TOK,
    ERR_CODE_INNER_CLASS_TRAILING_TOK, ERR_CODE_INNER_CLASSES_ATTR_END_TRAILING_TOK,
    ERR_CODE_INNER_END_TRAILING_TOK, ERR_CODE_INNER_NAME_TRAILING_TOK, ERR_CODE_INNER_TRAILING_TOK,
    ERR_CODE_INSTR_TRAILING_TOK, ERR_CODE_INVALID_CLASS_FLAG, ERR_CODE_INVALID_INNER_FLAG,
    ERR_CODE_INVALID_METHOD_FLAG, ERR_CODE_METHOD_TRAILING_TOK, ERR_CODE_OUTER_CLASS_TRAILING_TOK,
    ERR_CODE_PACKAGE_TRAILING_TOK, ERR_CODE_SUPER_TRAILING_TOK, ERR_CODE_TH_TRAILING_TOK,
    ERR_CODE_TOKEN_OUTSIDE_CLASS, ERR_CODE_UNEXPECTED_TOKEN_IN_CLASS,
    ERR_CODE_UNEXPECTED_TOKEN_IN_INNER, ERR_CODE_UNEXPECTED_TOKEN_IN_INNER_CLASSES_ATTR,
    ERR_CODE_UNEXPECTED_TOKEN_IN_METHOD,
};
use crate::instruction::InstructionSpec;
use crate::token::Spanned;
use crate::token::type_hint::TypeHintKind;
use std::fmt::{Display, Formatter};

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub(in crate::parser) enum AccessFlagContext {
    Class,
    InnerClassesAttr,
    Method,
}

impl Display for AccessFlagContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AccessFlagContext::Class => write!(f, "class"),
            AccessFlagContext::Method => write!(f, "method"),
            AccessFlagContext::InnerClassesAttr => write!(f, "inner_classes_attr flags"),
        }
    }
}

impl AccessFlagContext {
    pub(in crate::parser) fn error_code(&self) -> &'static str {
        match self {
            AccessFlagContext::Class => ERR_CODE_INVALID_CLASS_FLAG,
            AccessFlagContext::Method => ERR_CODE_INVALID_METHOD_FLAG,
            AccessFlagContext::InnerClassesAttr => ERR_CODE_INVALID_INNER_FLAG,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub(in crate::parser) enum UnexpectedTokenContext {
    BeforeClassDefinition,
    AfterClassDefinition,
    ClassBody,
    InnerBody,
    InnerClassesAttrBody,
    MethodBody,
}

impl Display for UnexpectedTokenContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UnexpectedTokenContext::BeforeClassDefinition => {
                write!(f, "before class definition")
            }
            UnexpectedTokenContext::AfterClassDefinition => {
                write!(f, "after class definition")
            }
            UnexpectedTokenContext::ClassBody => write!(f, "class body"),
            UnexpectedTokenContext::InnerBody => write!(f, "inner class body"),
            UnexpectedTokenContext::InnerClassesAttrBody => {
                write!(f, "inner_classes_attr body")
            }
            UnexpectedTokenContext::MethodBody => write!(f, "method body"),
        }
    }
}

impl UnexpectedTokenContext {
    pub(in crate::parser) fn error_code(&self) -> &'static str {
        match self {
            UnexpectedTokenContext::BeforeClassDefinition
            | UnexpectedTokenContext::AfterClassDefinition => ERR_CODE_TOKEN_OUTSIDE_CLASS,
            UnexpectedTokenContext::ClassBody => ERR_CODE_UNEXPECTED_TOKEN_IN_CLASS,
            UnexpectedTokenContext::InnerBody => ERR_CODE_UNEXPECTED_TOKEN_IN_INNER,
            UnexpectedTokenContext::InnerClassesAttrBody => {
                ERR_CODE_UNEXPECTED_TOKEN_IN_INNER_CLASSES_ATTR
            }
            UnexpectedTokenContext::MethodBody => ERR_CODE_UNEXPECTED_TOKEN_IN_METHOD,
        }
    }
}

// TODO: Rename because not only about operands, for example instruction name...
#[derive(Debug, Eq, PartialEq, Clone)]
pub(in crate::parser) enum OperandErrPosContext {
    ClassName,
    InnerName,
    SuperName,
    PackageName,
    MethodName,
    MangledName,
    InnerClassRef,
    OuterClassRef,
    InnerAttrName,
    MethodDescriptor,
    InstructionName,
    InstructionOperand(InstructionSpec),
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub(in crate::parser) enum TrailingTokensErrContext {
    Class,
    Inner,
    InnerEnd,
    InnerClassesAttrEnd,
    InnerClassRef,
    OuterClassRef,
    InnerName,
    Flags,
    Super,
    Method,
    TypeHint(Spanned<TypeHintKind>),
    ClassEnd,
    Instruction(Spanned<String>),
    Package,
}

impl Display for TrailingTokensErrContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TrailingTokensErrContext::Class => write!(f, "class definition"),
            TrailingTokensErrContext::Inner => write!(f, "inner class definition"),
            TrailingTokensErrContext::Super => write!(f, "super class definition"),
            TrailingTokensErrContext::Method => write!(f, "method definition"),
            TrailingTokensErrContext::Package => write!(f, "package definition"),
            TrailingTokensErrContext::TypeHint(kind) => {
                write!(f, "type hint '{}'", kind.value.token_name())
            }
            TrailingTokensErrContext::ClassEnd => write!(f, "class definition end"),
            TrailingTokensErrContext::InnerEnd => write!(f, "inner class definition end"),
            TrailingTokensErrContext::InnerClassesAttrEnd => {
                write!(f, "inner_classes_attr definition end")
            }
            TrailingTokensErrContext::InnerClassRef => write!(f, ".inner_class directive"),
            TrailingTokensErrContext::OuterClassRef => write!(f, ".outer_class directive"),
            TrailingTokensErrContext::InnerName => write!(f, ".inner_name directive"),
            TrailingTokensErrContext::Flags => write!(f, ".flags directive"),
            TrailingTokensErrContext::Instruction(name) => {
                write!(f, "instruction '{}'", name.value)
            }
        }
    }
}

impl TrailingTokensErrContext {
    pub(in crate::parser) fn error_code(&self) -> &'static str {
        match self {
            TrailingTokensErrContext::Class => ERR_CODE_CLASS_DEF_TRAILING_TOK,
            TrailingTokensErrContext::Super => ERR_CODE_SUPER_TRAILING_TOK,
            TrailingTokensErrContext::Package => ERR_CODE_PACKAGE_TRAILING_TOK,
            TrailingTokensErrContext::TypeHint(_) => ERR_CODE_TH_TRAILING_TOK,
            TrailingTokensErrContext::Method => ERR_CODE_METHOD_TRAILING_TOK,
            TrailingTokensErrContext::ClassEnd => ERR_CODE_CLASS_END_TRAILING_TOK,
            TrailingTokensErrContext::Instruction(_) => ERR_CODE_INSTR_TRAILING_TOK,
            TrailingTokensErrContext::Inner => ERR_CODE_INNER_TRAILING_TOK,
            TrailingTokensErrContext::InnerEnd => ERR_CODE_INNER_END_TRAILING_TOK,
            TrailingTokensErrContext::InnerClassesAttrEnd => {
                ERR_CODE_INNER_CLASSES_ATTR_END_TRAILING_TOK
            }
            TrailingTokensErrContext::InnerClassRef => ERR_CODE_INNER_CLASS_TRAILING_TOK,
            TrailingTokensErrContext::OuterClassRef => ERR_CODE_OUTER_CLASS_TRAILING_TOK,
            TrailingTokensErrContext::InnerName => ERR_CODE_INNER_NAME_TRAILING_TOK,
            TrailingTokensErrContext::Flags => ERR_CODE_FLAGS_TRAILING_TOK,
        }
    }
}

impl Display for OperandErrPosContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            OperandErrPosContext::ClassName => write!(f, "class name"),
            OperandErrPosContext::InnerName => write!(f, "inner name"),
            OperandErrPosContext::SuperName => write!(f, "super class name"),
            OperandErrPosContext::PackageName => write!(f, "package name"),
            OperandErrPosContext::MethodName => write!(f, "method name"),
            OperandErrPosContext::MethodDescriptor => write!(f, "method descriptor"),
            OperandErrPosContext::InstructionName => write!(f, "instruction name"),
            OperandErrPosContext::MangledName => write!(f, "mangled name"),
            OperandErrPosContext::InnerClassRef => write!(f, "inner class reference"),
            OperandErrPosContext::OuterClassRef => write!(f, "outer class reference"),
            OperandErrPosContext::InnerAttrName => write!(f, "inner attribute name"),
            OperandErrPosContext::InstructionOperand(spec) => {
                write!(f, "instruction '{}' operand", spec.opcode)
            }
        }
    }
}

impl OperandErrPosContext {
    pub(in crate::parser) fn directive_name(&self) -> &'static str {
        match self {
            //TODO: use something like TokenKind::DotClass.name() to not hardcode here
            OperandErrPosContext::ClassName => ".class",
            OperandErrPosContext::InnerName => ".inner",
            OperandErrPosContext::SuperName => ".super",
            OperandErrPosContext::PackageName => ".package",
            OperandErrPosContext::MethodName | OperandErrPosContext::MethodDescriptor => ".method",
            OperandErrPosContext::InstructionName => "instruction",
            OperandErrPosContext::MangledName => ".mangled_name",
            OperandErrPosContext::InnerClassRef => ".inner_class",
            OperandErrPosContext::OuterClassRef => ".outer_class",
            OperandErrPosContext::InnerAttrName => ".inner_name",
            OperandErrPosContext::InstructionOperand(spec) => spec.opcode.as_str(),
        }
    }
}
