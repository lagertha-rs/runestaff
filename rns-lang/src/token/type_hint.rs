use crate::token::Span;
use crate::token::span::Spanned;
use std::fmt::{Display, Formatter};

pub const TYPE_HINT_AT_UTF8: &str = "@utf8";
pub const TYPE_HINT_AT_INTEGER: &str = "@int";
pub const TYPE_HINT_AT_STRING: &str = "@string";
pub const TYPE_HINT_AT_CLASS: &str = "@class";
pub const TYPE_HINT_AT_METHODREF: &str = "@methodref";
pub const TYPE_HINT_AT_FIELDREF: &str = "@fieldref";
pub const TYPE_HINT_AT_INTERFACE_METHODREF: &str = "@interfaceMethodref";
pub const TYPE_HINT_AT_FLOAT: &str = "@float";
pub const TYPE_HINT_AT_LONG: &str = "@long";
pub const TYPE_HINT_AT_DOUBLE: &str = "@double";
pub const TYPE_HINT_AT_NAME_AND_TYPE: &str = "@nameAndType";
pub const TYPE_HINT_AT_METHOD_HANDLE: &str = "@methodHandle";
pub const TYPE_HINT_AT_METHOD_TYPE: &str = "@methodType";
pub const TYPE_HINT_AT_DYNAMIC: &str = "@dynamic";
pub const TYPE_HINT_AT_INVOKE_DYNAMIC: &str = "@invokeDynamic";
pub const TYPE_HINT_AT_MODULE: &str = "@module";
pub const TYPE_HINT_AT_PACKAGE: &str = "@package";

pub const TYPE_HINT_UTF8: &str = "utf8";
pub const TYPE_HINT_INTEGER: &str = "int";
pub const TYPE_HINT_STRING: &str = "string";
pub const TYPE_HINT_CLASS: &str = "class";
pub const TYPE_HINT_METHODREF: &str = "methodref";
pub const TYPE_HINT_FIELDREF: &str = "fieldref";
pub const TYPE_HINT_INTERFACE_METHODREF: &str = "interfaceMethodref";
pub const TYPE_HINT_FLOAT: &str = "float";
pub const TYPE_HINT_LONG: &str = "long";
pub const TYPE_HINT_DOUBLE: &str = "double";
pub const TYPE_HINT_NAME_AND_TYPE: &str = "nameAndType";
pub const TYPE_HINT_METHOD_HANDLE: &str = "methodHandle";
pub const TYPE_HINT_METHOD_TYPE: &str = "methodType";
pub const TYPE_HINT_DYNAMIC: &str = "dynamic";
pub const TYPE_HINT_INVOKE_DYNAMIC: &str = "invokeDynamic";
pub const TYPE_HINT_MODULE: &str = "module";
pub const TYPE_HINT_PACKAGE: &str = "package";

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum TypeHintKind {
    Utf8,
    Integer,
    String,
    Class,
    Methodref,
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

impl TypeHintKind {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            TYPE_HINT_UTF8 => Some(Self::Utf8),
            TYPE_HINT_INTEGER => Some(Self::Integer),
            TYPE_HINT_STRING => Some(Self::String),
            TYPE_HINT_CLASS => Some(Self::Class),
            TYPE_HINT_METHODREF => Some(Self::Methodref),
            TYPE_HINT_FIELDREF => Some(Self::Fieldref),
            TYPE_HINT_INTERFACE_METHODREF => Some(Self::InterfaceMethodref),
            TYPE_HINT_FLOAT => Some(Self::Float),
            TYPE_HINT_LONG => Some(Self::Long),
            TYPE_HINT_DOUBLE => Some(Self::Double),
            TYPE_HINT_NAME_AND_TYPE => Some(Self::NameAndType),
            TYPE_HINT_METHOD_HANDLE => Some(Self::MethodHandle),
            TYPE_HINT_METHOD_TYPE => Some(Self::MethodType),
            TYPE_HINT_DYNAMIC => Some(Self::Dynamic),
            TYPE_HINT_INVOKE_DYNAMIC => Some(Self::InvokeDynamic),
            TYPE_HINT_MODULE => Some(Self::Module),
            TYPE_HINT_PACKAGE => Some(Self::Package),
            _ => None,
        }
    }

    pub fn operands_count(&self) -> usize {
        match self {
            Self::Utf8 | Self::Integer | Self::String | Self::Class => 1,
            Self::Methodref => 3,
            _ => unimplemented!(),
        }
    }

    pub fn expected_argument_types(&self) -> &'static [&'static str] {
        match self {
            Self::Utf8 => &["identifier"],
            Self::Integer => &["integer"],
            Self::String => &["string literal"],
            Self::Class => &["identifier"],
            Self::Methodref => &[
                "identifier (class name)",
                "identifier (method name)",
                "identifier (method descriptor)",
            ],
            _ => unimplemented!(),
        }
    }

    /// Returns the source-code form of this type hint (e.g., `"@int"`, `"@utf8"`, `"@class"`).
    pub fn token_name(&self) -> &'static str {
        match self {
            Self::Utf8 => TYPE_HINT_AT_UTF8,
            Self::Integer => TYPE_HINT_AT_INTEGER,
            Self::String => TYPE_HINT_AT_STRING,
            Self::Class => TYPE_HINT_AT_CLASS,
            Self::Methodref => TYPE_HINT_AT_METHODREF,
            Self::Fieldref => TYPE_HINT_AT_FIELDREF,
            Self::InterfaceMethodref => TYPE_HINT_AT_INTERFACE_METHODREF,
            Self::Float => TYPE_HINT_AT_FLOAT,
            Self::Long => TYPE_HINT_AT_LONG,
            Self::Double => TYPE_HINT_AT_DOUBLE,
            Self::NameAndType => TYPE_HINT_AT_NAME_AND_TYPE,
            Self::MethodHandle => TYPE_HINT_AT_METHOD_HANDLE,
            Self::MethodType => TYPE_HINT_AT_METHOD_TYPE,
            Self::Dynamic => TYPE_HINT_AT_DYNAMIC,
            Self::InvokeDynamic => TYPE_HINT_AT_INVOKE_DYNAMIC,
            Self::Module => TYPE_HINT_AT_MODULE,
            Self::Package => TYPE_HINT_AT_PACKAGE,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Utf8 => TYPE_HINT_UTF8,
            Self::Integer => TYPE_HINT_INTEGER,
            Self::String => TYPE_HINT_STRING,
            Self::Class => TYPE_HINT_CLASS,
            Self::Methodref => TYPE_HINT_METHODREF,
            Self::Fieldref => TYPE_HINT_FIELDREF,
            Self::InterfaceMethodref => TYPE_HINT_INTERFACE_METHODREF,
            Self::Float => TYPE_HINT_FLOAT,
            Self::Long => TYPE_HINT_LONG,
            Self::Double => TYPE_HINT_DOUBLE,
            Self::NameAndType => TYPE_HINT_NAME_AND_TYPE,
            Self::MethodHandle => TYPE_HINT_METHOD_HANDLE,
            Self::MethodType => TYPE_HINT_METHOD_TYPE,
            Self::Dynamic => TYPE_HINT_DYNAMIC,
            Self::InvokeDynamic => TYPE_HINT_INVOKE_DYNAMIC,
            Self::Module => TYPE_HINT_MODULE,
            Self::Package => TYPE_HINT_PACKAGE,
        }
    }
}

impl Display for TypeHintKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.token_name())
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum TypeHint {
    ZeroIndex(Span),
    Utf8(Span, Spanned<String>),
    Integer(Span, Spanned<i32>),
    String(Span, Spanned<String>),
    Class(Option<Span>, Spanned<String>),
    Methodref(Span, Spanned<String>, Spanned<String>, Spanned<String>),
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

impl TypeHint {
    pub fn token_name_with_value(&self) -> String {
        match self {
            Self::Utf8(_, value) => format!("{} ({})", TYPE_HINT_AT_UTF8, value.value),
            Self::Integer(_, value) => format!("{} ({})", TYPE_HINT_AT_INTEGER, value.value),
            Self::String(_, value) => format!("{} ({})", TYPE_HINT_AT_STRING, value.value),
            Self::Class(_, value) => format!("{} ({})", TYPE_HINT_AT_CLASS, value.value),
            Self::Methodref(_, class, name, descriptor) => format!(
                "{} (class: {}, name: {}, descriptor: {})",
                TYPE_HINT_AT_METHODREF, class.value, name.value, descriptor.value
            ),
            _ => unimplemented!(),
        }
    }
}
