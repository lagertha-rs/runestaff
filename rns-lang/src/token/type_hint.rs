use crate::token::span::Spanned;
use std::fmt::Display;

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
            "utf8" => Some(Self::Utf8),
            "int" => Some(Self::Integer),
            "string" => Some(Self::String),
            "class" => Some(Self::Class),
            "methodref" => Some(Self::Methodref),
            "fieldref" => Some(Self::Fieldref),
            "interfaceMethodref" => Some(Self::InterfaceMethodref),
            "float" => Some(Self::Float),
            "long" => Some(Self::Long),
            "double" => Some(Self::Double),
            "nameAndType" => Some(Self::NameAndType),
            "methodHandle" => Some(Self::MethodHandle),
            "methodType" => Some(Self::MethodType),
            "dynamic" => Some(Self::Dynamic),
            "invokeDynamic" => Some(Self::InvokeDynamic),
            "module" => Some(Self::Module),
            "package" => Some(Self::Package),
            _ => None,
        }
    }

    pub fn argument_count(&self) -> usize {
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
}

impl Display for TypeHintKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Utf8 => "@utf8",
            Self::Integer => "@integer",
            Self::String => "@string",
            Self::Class => "@class",
            Self::Methodref => "@methodref",
            Self::Fieldref => "@fieldref",
            Self::InterfaceMethodref => "@interfaceMethodref",
            Self::Float => "@float",
            Self::Long => "@long",
            Self::Double => "@double",
            Self::NameAndType => "@nameAndType",
            Self::MethodHandle => "@methodHandle",
            Self::MethodType => "@methodType",
            Self::Dynamic => "@dynamic",
            Self::InvokeDynamic => "@invokeDynamic",
            Self::Module => "@module",
            Self::Package => "@package",
        };
        write!(f, "{s}")
    }
}

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

impl Display for TypeHint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Utf8(s) => write!(f, "@utf8 {}", s.value),
            Self::Integer(n) => write!(f, "@int {}", n.value),
            Self::String(s) => write!(f, "@string \"{}\"", s.value),
            Self::Class(s) => write!(f, "@class {}", s.value),
            Self::Methodref(class, name, descriptor) => write!(
                f,
                "@methodref {} {} {}",
                class.value, name.value, descriptor.value
            ),
            _ => unimplemented!(),
        }
    }
}
