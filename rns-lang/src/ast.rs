#[derive(Debug)]
pub(crate) struct JasmClass {
    pub name: String,
    pub super_name: String,
    pub access_flags: Vec<JasmAccessFlag>,
    pub methods: Vec<JasmMethod>,
}

#[derive(Debug)]
pub struct JasmMethod {
    pub access_flags: Vec<JasmAccessFlag>,
    pub name: String,
    pub descriptor: String,
    pub code: JasmCodeAttribute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JasmAccessFlag {
    Public,
    Static,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JasmCodeAttribute {
    pub max_stack: Option<u16>,
    pub max_locals: Option<u16>,
    pub instructions: Vec<JasmInstruction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JasmInstruction {
    Aload {
        index: u16,
    },
    GetStatic {
        class: String,
        name: String,
        descriptor: String,
    },
    Ldc {
        value: String,
    },
    //TODO: substructure for class, name, descriptor since it's repeated ?
    InvokeSpecial {
        class: String,
        name: String,
        descriptor: String,
    },
    InvokeVirtual {
        class: String,
        name: String,
        descriptor: String,
    },
    Return,
}
