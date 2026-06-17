use jclass::bytecode::Opcode;
use phf::phf_map;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionNumericOperand {
    Byte,
    Short,
    Int,
}

impl InstructionNumericOperand {
    pub fn byte_size(self) -> u8 {
        match self {
            Self::Byte => 1,
            Self::Short => 2,
            Self::Int => 4,
        }
    }

    pub fn is_signed(self) -> bool {
        match self {
            Self::Byte => false,
            Self::Short | Self::Int => true,
        }
    }

    pub fn min_value(self) -> i64 {
        match self {
            Self::Byte => 0,
            Self::Short => i16::MIN as i64,
            Self::Int => i32::MIN as i64,
        }
    }

    pub fn max_value(self) -> i64 {
        match self {
            Self::Byte => u8::MAX as i64,
            Self::Short => i16::MAX as i64,
            Self::Int => i32::MAX as i64,
        }
    }

    pub fn byte_description(self) -> &'static str {
        match self {
            Self::Byte => "1 byte",
            Self::Short => "2 bytes",
            Self::Int => "4 bytes",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionOperand {
    None,
    // Constant pool references (parsed via type hints)
    MethodRef,
    FieldRef,
    TypeHint,
    // Raw values
    Numeric(InstructionNumericOperand),
    // Branch targets
    Label,
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
                operand: $operand:expr,
            }
        ),* $(,)?
    ) => {
        pub static INSTRUCTION_SPECS: phf::Map<&'static str, InstructionSpec> = phf_map! {
            $(
                $name => InstructionSpec {
                    opcode: Opcode::$variant,
                    operand: $operand,
                },
            )*
        };
    };
}

define_instructions! {
    // No operand
    Aload0 => {
        name: "aload_0",
        operand: InstructionOperand::None,
    },
    Return => {
        name: "return",
        operand: InstructionOperand::None,
    },
    Iload0 => {
        name: "iload_0",
        operand: InstructionOperand::None,
    },
    Iconst0 => {
        name: "iconst_0",
        operand: InstructionOperand::None,
    },

    // Constant pool references
    InvokeSpecial => {
        name: "invokespecial",
        operand: InstructionOperand::MethodRef,
    },
    InvokeVirtual => {
        name: "invokevirtual",
        operand: InstructionOperand::MethodRef,
    },
    InvokeStatic => {
        name: "invokestatic",
        operand: InstructionOperand::MethodRef,
    },
    Getstatic => {
        name: "getstatic",
        operand: InstructionOperand::FieldRef,
    },
    Ldc => {
        name: "ldc",
        operand: InstructionOperand::TypeHint,
    },

    // Numeric operand
    Bipush => {
        name: "bipush",
        operand: InstructionOperand::Numeric(InstructionNumericOperand::Byte),
    },

    // Label operand
    Goto => {
        name: "goto",
        operand: InstructionOperand::Label,
    },
    IfIcmpeq => {
        name: "if_icmpeq",
        operand: InstructionOperand::Label,
    },
}
