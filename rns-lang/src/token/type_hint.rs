use crate::token::span::Spanned;

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum TypeHint {
    Utf8(Spanned<String>),
    Integer(Spanned<i32>),
    String(Spanned<String>),
    Class(Spanned<String>),
    Methodref(Spanned<String>, Spanned<String>, Spanned<String>),
    Fieldref,
    InterfaceMethodref,
    Float,
    Long,
    Double,
    NameAndType,
    MethodHandle,
    MethodType,
    Dynamic,
    InvokeDynamic,
    Module,
    Package,
}
