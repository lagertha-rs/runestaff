use crate::token::{
    FLAG_ABSTRACT, FLAG_ANNOTATION, FLAG_ENUM, FLAG_FINAL, FLAG_INTERFACE, FLAG_MODULE,
    FLAG_PUBLIC, FLAG_STATIC, FLAG_SUPER, FLAG_SYNTHETIC,
};
use std::fmt::{Display, Formatter};

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
        }
    }

    pub fn token_name(&self) -> &'static str {
        self.name()
    }
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
