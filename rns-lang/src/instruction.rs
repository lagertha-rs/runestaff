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
    pub mnemonic: InstructionMnemonic,
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
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum InstructionMnemonic {
            $( $variant, )*
        }

        impl InstructionMnemonic {
            pub const fn as_str(self) -> &'static str {
                match self {
                    $( InstructionMnemonic::$variant => $name, )*
                }
            }
        }

        impl std::fmt::Display for InstructionMnemonic {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        pub static INSTRUCTION_SPECS: phf::Map<&'static str, InstructionSpec> = phf_map! {
            $(
                $name => InstructionSpec {
                    mnemonic: InstructionMnemonic::$variant,
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
    GetStatic => {
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
