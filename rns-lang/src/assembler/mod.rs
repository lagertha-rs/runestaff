use crate::assembler::jvm_warning::JvmWarning;
use crate::ast::RnsModule;
use crate::ast::flag::RnsClassFlag;
use crate::diagnostic::Diagnostic;
use crate::token::type_hint::TypeHint;
use jclass::ClassFile;
use jclass::flags::ClassFlags;
use jclass::prelude::{ClassFileBuilder, ConstantPoolBuilder};

mod jvm_warning;

impl RnsModule {
    fn build_class_flags(&mut self) -> ClassFlags {
        let mut res = ClassFlags::new(0);
        for (flag, span) in &self.class_dir.flags {
            match flag {
                RnsClassFlag::Public => res.set_public(),
                RnsClassFlag::Final => res.set_final(),
                RnsClassFlag::Super => res.set_super(),
                RnsClassFlag::Interface => {
                    // TODO: put in method?
                    if !self.class_dir.flags.contains_key(&RnsClassFlag::Abstract) {
                        self.diagnostics.push(
                            JvmWarning::InterfaceFlagWithMissingAbstract {
                                interface_span: *span,
                            }
                            .into(),
                        )
                    }
                    let exclusive_flags = self
                        .class_dir
                        .flags
                        .iter()
                        .filter(|(f, _)| {
                            // TODO: add a method, something like "is_exclusive_with_interface" to RnsFlag
                            matches!(
                                f,
                                RnsClassFlag::Final
                                    | RnsClassFlag::Enum
                                    | RnsClassFlag::Module
                                    | RnsClassFlag::Super
                            )
                        })
                        .map(|(f, s)| (*f, *s))
                        .collect::<Vec<_>>();
                    if !exclusive_flags.is_empty() {
                        self.diagnostics.push(
                            JvmWarning::InterfaceMutuallyExclusive {
                                interface_span: *span,
                                exclusive_flags,
                            }
                            .into(),
                        )
                    }
                    res.set_interface()
                }
                RnsClassFlag::Abstract => res.set_abstract(),
                RnsClassFlag::Enum => res.set_enum(),
                RnsClassFlag::Synthetic => res.set_synthetic(),
                RnsClassFlag::Annotation => res.set_annotation(),
                RnsClassFlag::Module => res.set_module(),
                _ => unimplemented!(),
            }
        }
        res
    }

    fn add_type_hint_to_cp(cp_builder: &mut ConstantPoolBuilder, hint: TypeHint) -> u16 {
        match hint {
            TypeHint::Class(_, class_name) => cp_builder.add_class(&class_name.value),
            TypeHint::Integer(_, int_value) => cp_builder.add_integer(int_value.value),
            _ => unimplemented!(),
        }
    }

    fn build_super_class(&mut self, cp_builder: &mut ConstantPoolBuilder) -> Option<u16> {
        let name = self.super_dir.take()?.name;
        Some(Self::add_type_hint_to_cp(cp_builder, name))
    }

    // TODO: need to test that I build exactly same CP as javac, or not?
    pub fn into_class_file(mut self) -> (Option<ClassFile>, Vec<Diagnostic>) {
        let mut cp_builder = ConstantPoolBuilder::new();
        let super_cp_id = self.build_super_class(&mut cp_builder);
        let class_flags = self.build_class_flags();

        let this_cp_id = self
            .class_dir
            .name
            .map(|name| Self::add_type_hint_to_cp(&mut cp_builder, name));

        let class_file = ClassFileBuilder::new(0, 69, cp_builder.build()) // TODO: allow specifying version in jasm
            .access_flags(class_flags) // TODO: set access flags based on parsed flags
            .this_class(this_cp_id)
            .super_class(super_cp_id)
            .build();

        (class_file, self.diagnostics)
    }
}
