use jclass::bytecode::Opcode;
use phf::phf_map;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionOperand {
    None,
    MethodRef,
    FieldRef,
    StringLiteral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstructionSpec {
    pub opcode: Opcode,
    pub operand: InstructionOperand,
}

macro_rules! define_instructions {
    (
        $(
            $variant:ident => {
                name: $name:literal,
                operand: $operand:ident,
            }
        ),* $(,)?
    ) => {
        pub static INSTRUCTION_SPECS: phf::Map<&'static str, InstructionSpec> = phf_map! {
            $(
                $name => InstructionSpec {
                    opcode: Opcode::$variant,
                    operand: InstructionOperand::$operand,
                },
            )*
        };
    };
}

define_instructions! {
    Aload0 => {
        name: "aload_0",
        operand: None,
    },
    InvokeSpecial => {
        name: "invokespecial",
        operand: MethodRef,
    },
    Return => {
        name: "return",
        operand: None,
    },
    Getstatic => {
        name: "getstatic",
        operand: FieldRef,
    },
    Ldc => {
        name: "ldc",
        operand: StringLiteral, // TODO: stub for hello world
    },
    InvokeVirtual => {
        name: "invokevirtual",
        operand: MethodRef,
    },
}
