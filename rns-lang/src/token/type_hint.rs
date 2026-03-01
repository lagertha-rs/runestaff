use crate::token::span::{SpannedInteger, SpannedString};

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum TypeHint {
    Utf8(SpannedString),
    Integer(SpannedInteger),
    String(SpannedString),
    Class(SpannedString),
    Methodref(SpannedString, SpannedString, SpannedString),
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
