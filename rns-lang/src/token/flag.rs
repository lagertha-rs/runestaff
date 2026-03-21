use crate::token::{
    FLAG_ABSTRACT, FLAG_ANNOTATION, FLAG_BRIDGE, FLAG_ENUM, FLAG_FINAL, FLAG_INTERFACE,
    FLAG_MODULE, FLAG_NATIVE, FLAG_PRIVATE, FLAG_PROTECTED, FLAG_PUBLIC, FLAG_STATIC, FLAG_STRICT,
    FLAG_SUPER, FLAG_SYNCHRONIZED, FLAG_SYNTHETIC, FLAG_VARARGS,
};
use std::fmt::{Display, Formatter};

pub const JVMS_INTERFACE_FLAG_NAME: &str = "ACC_INTERFACE";
pub const JVMS_ABSTRACT_FLAG_NAME: &str = "ACC_ABSTRACT";
pub const JVMS_ENUM_FLAG_NAME: &str = "ACC_ENUM";
pub const JVMS_MODULE_FLAG_NAME: &str = "ACC_MODULE";
pub const JVMS_PUBLIC_FLAG_NAME: &str = "ACC_PUBLIC";
pub const JVMS_STATIC_FLAG_NAME: &str = "ACC_STATIC";
pub const JVMS_FINAL_FLAG_NAME: &str = "ACC_FINAL";
pub const JVMS_SUPER_FLAG_NAME: &str = "ACC_SUPER";
pub const JVMS_SYNTHETIC_FLAG_NAME: &str = "ACC_SYNTHETIC";
pub const JVMS_ANNOTATION_FLAG_NAME: &str = "ACC_ANNOTATION";
pub const JVMS_PRIVATE_FLAG_NAME: &str = "ACC_PRIVATE";
pub const JVMS_PROTECTED_FLAG_NAME: &str = "ACC_PROTECTED";
pub const JVMS_SYNCHRONIZED_FLAG_NAME: &str = "ACC_SYNCHRONIZED";
pub const JVMS_BRIDGE_FLAG_NAME: &str = "ACC_BRIDGE";
pub const JVMS_VARARGS_FLAG_NAME: &str = "ACC_VARARGS";
pub const JVMS_NATIVE_FLAG_NAME: &str = "ACC_NATIVE";
pub const JVMS_STRICT_FLAG_NAME: &str = "ACC_STRICT";

#[derive(Debug, Eq, PartialEq, Clone, Copy, Ord, PartialOrd)]
pub enum RnsFlag {
    Public,
    Private,
    Protected,
    Static,
    Final,
    Synchronized,
    Bridge,
    Varargs,
    Native,
    Super,
    Interface,
    Abstract,
    Strict,
    Enum,
    Synthetic,
    Annotation,
    Module,
}

impl RnsFlag {
    pub fn jvm_spec_name(&self) -> &'static str {
        match self {
            RnsFlag::Interface => JVMS_INTERFACE_FLAG_NAME,
            RnsFlag::Abstract => JVMS_ABSTRACT_FLAG_NAME,
            RnsFlag::Enum => JVMS_ENUM_FLAG_NAME,
            RnsFlag::Module => JVMS_MODULE_FLAG_NAME,
            RnsFlag::Public => JVMS_PUBLIC_FLAG_NAME,
            RnsFlag::Static => JVMS_STATIC_FLAG_NAME,
            RnsFlag::Final => JVMS_FINAL_FLAG_NAME,
            RnsFlag::Super => JVMS_SUPER_FLAG_NAME,
            RnsFlag::Synthetic => JVMS_SYNTHETIC_FLAG_NAME,
            RnsFlag::Annotation => JVMS_ANNOTATION_FLAG_NAME,
            RnsFlag::Private => JVMS_PRIVATE_FLAG_NAME,
            RnsFlag::Protected => JVMS_PROTECTED_FLAG_NAME,
            RnsFlag::Synchronized => JVMS_SYNCHRONIZED_FLAG_NAME,
            RnsFlag::Bridge => JVMS_BRIDGE_FLAG_NAME,
            RnsFlag::Varargs => JVMS_VARARGS_FLAG_NAME,
            RnsFlag::Native => JVMS_NATIVE_FLAG_NAME,
            RnsFlag::Strict => JVMS_STRICT_FLAG_NAME,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            RnsFlag::Interface => FLAG_INTERFACE,
            RnsFlag::Abstract => FLAG_ABSTRACT,
            RnsFlag::Enum => FLAG_ENUM,
            RnsFlag::Module => FLAG_MODULE,
            RnsFlag::Public => FLAG_PUBLIC,
            RnsFlag::Static => FLAG_STATIC,
            RnsFlag::Final => FLAG_FINAL,
            RnsFlag::Super => FLAG_SUPER,
            RnsFlag::Synthetic => FLAG_SYNTHETIC,
            RnsFlag::Annotation => FLAG_ANNOTATION,
            RnsFlag::Private => FLAG_PRIVATE,
            RnsFlag::Protected => FLAG_PROTECTED,
            RnsFlag::Synchronized => FLAG_SYNCHRONIZED,
            RnsFlag::Bridge => FLAG_BRIDGE,
            RnsFlag::Varargs => FLAG_VARARGS,
            RnsFlag::Native => FLAG_NATIVE,
            RnsFlag::Strict => FLAG_STRICT,
        }
    }

    pub fn token_name(&self) -> &'static str {
        self.name()
    }

    pub fn as_class_flag(&self) -> Option<RnsClassFlag> {
        match self {
            RnsFlag::Public => Some(RnsClassFlag::Public),
            RnsFlag::Static => Some(RnsClassFlag::Static),
            RnsFlag::Final => Some(RnsClassFlag::Final),
            RnsFlag::Super => Some(RnsClassFlag::Super),
            RnsFlag::Interface => Some(RnsClassFlag::Interface),
            RnsFlag::Abstract => Some(RnsClassFlag::Abstract),
            RnsFlag::Enum => Some(RnsClassFlag::Enum),
            RnsFlag::Synthetic => Some(RnsClassFlag::Synthetic),
            RnsFlag::Annotation => Some(RnsClassFlag::Annotation),
            RnsFlag::Module => Some(RnsClassFlag::Module),
            _ => None,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Ord, PartialOrd)]
pub enum RnsClassFlag {
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

impl RnsClassFlag {
    pub fn jvm_spec_name(&self) -> &'static str {
        match self {
            RnsClassFlag::Interface => JVMS_INTERFACE_FLAG_NAME,
            RnsClassFlag::Abstract => JVMS_ABSTRACT_FLAG_NAME,
            RnsClassFlag::Enum => JVMS_ENUM_FLAG_NAME,
            RnsClassFlag::Module => JVMS_MODULE_FLAG_NAME,
            RnsClassFlag::Public => JVMS_PUBLIC_FLAG_NAME,
            RnsClassFlag::Static => JVMS_STATIC_FLAG_NAME,
            RnsClassFlag::Final => JVMS_FINAL_FLAG_NAME,
            RnsClassFlag::Super => JVMS_SUPER_FLAG_NAME,
            RnsClassFlag::Synthetic => JVMS_SYNTHETIC_FLAG_NAME,
            RnsClassFlag::Annotation => JVMS_ANNOTATION_FLAG_NAME,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            RnsClassFlag::Interface => FLAG_INTERFACE,
            RnsClassFlag::Abstract => FLAG_ABSTRACT,
            RnsClassFlag::Enum => FLAG_ENUM,
            RnsClassFlag::Module => FLAG_MODULE,
            RnsClassFlag::Public => FLAG_PUBLIC,
            RnsClassFlag::Static => FLAG_STATIC,
            RnsClassFlag::Final => FLAG_FINAL,
            RnsClassFlag::Super => FLAG_SUPER,
            RnsClassFlag::Synthetic => FLAG_SYNTHETIC,
            RnsClassFlag::Annotation => FLAG_ANNOTATION,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Ord, PartialOrd)]
pub enum RnsMethodFlag {
    Public,
    Private,
    Protected,
    Static,
    Final,
    Synchronized,
    Bridge,
    Varargs,
    Native,
    Abstract,
    Strict,
    Synthetic,
}

impl Display for RnsFlag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.token_name())
    }
}

impl TryFrom<&str> for RnsFlag {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            FLAG_PUBLIC => Ok(RnsFlag::Public),
            FLAG_STATIC => Ok(RnsFlag::Static),
            FLAG_FINAL => Ok(RnsFlag::Final),
            FLAG_SUPER => Ok(RnsFlag::Super),
            FLAG_INTERFACE => Ok(RnsFlag::Interface),
            FLAG_ABSTRACT => Ok(RnsFlag::Abstract),
            FLAG_ENUM => Ok(RnsFlag::Enum),
            FLAG_SYNTHETIC => Ok(RnsFlag::Synthetic),
            FLAG_ANNOTATION => Ok(RnsFlag::Annotation),
            FLAG_MODULE => Ok(RnsFlag::Module),
            _ => Err(()),
        }
    }
}
