use jclass::bytecode::Opcode;
use phf::phf_map;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionArgKind {
    ClassName,
    MethodName,
    MethodDescriptor,
    FieldName,
    FieldDescriptor,
    StringLiteral, // TODO: stub for ldc
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstructionSpec {
    pub opcode: Opcode,
    pub args: &'static [InstructionArgKind],
}

macro_rules! define_instructions {
    (
        $(
            $variant:ident => {
                name: $name:literal,
                args: [ $( $arg:ident ),* $(,)? ],
            }
        ),* $(,)?
    ) => {
        pub static INSTRUCTION_SPECS: phf::Map<&'static str, InstructionSpec> = phf_map! {
            $(
                $name => InstructionSpec {
                    opcode: Opcode::$variant,
                    args: &[ $( InstructionArgKind::$arg, )* ],
                },
            )*
        };
    };
}

define_instructions! {
    Aload0 => {
        name: "aload_0",
        args: [],
    },
    InvokeSpecial => {
        name: "invokespecial",
        args: [ClassName, MethodName, MethodDescriptor],
    },
    Return => {
        name: "return",
        args: [],
    },
    Getstatic => {
        name: "getstatic",
        args: [ClassName, FieldName, FieldDescriptor],
    },
    Ldc => {
        name: "ldc",
        args: [StringLiteral],
    },
    InvokeVirtual => {
        name: "invokevirtual",
        args: [ClassName, MethodName, MethodDescriptor],
    },
}
