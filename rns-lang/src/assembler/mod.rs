use crate::assembler::jvm_warning::JvmWarning;
use crate::diagnostic::Diagnostic;
use crate::token::type_hint::TypeHint;
use crate::token::{RnsFlag, Span};
use jclass::flags::ClassFlags;
use jclass::prelude::{ClassFileBuilder, ConstantPoolBuilder};
use jclass::ClassFile;
use std::collections::BTreeMap;

mod jvm_warning;

pub struct RnsModule {
    pub class_dir: ClassDirective,
    pub super_dir: Option<SuperDirective>,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct SuperDirective {
    pub dir_span: Option<Span>,
    pub name: TypeHint,
}

pub struct ClassDirective {
    pub dir_span: Span,
    pub name: Option<TypeHint>,
    // TODO: BTreeMap because I need it to be sorted for my snapshot test. investigate impact
    pub flags: BTreeMap<RnsFlag, Span>,
}

impl RnsModule {
    fn build_class_flags(&mut self) -> ClassFlags {
        let mut res = ClassFlags::new(0);
        for (flag, span) in &self.class_dir.flags {
            match flag {
                RnsFlag::Public => res.set_public(),
                RnsFlag::Final => res.set_final(),
                RnsFlag::Super => res.set_super(),
                RnsFlag::Interface => {
                    // TODO: put in method?
                    if !self.class_dir.flags.contains_key(&RnsFlag::Abstract) {
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
                                RnsFlag::Final | RnsFlag::Enum | RnsFlag::Module | RnsFlag::Super
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
                RnsFlag::Abstract => res.set_abstract(),
                RnsFlag::Enum => res.set_enum(),
                RnsFlag::Synthetic => res.set_synthetic(),
                RnsFlag::Annotation => res.set_annotation(),
                RnsFlag::Module => res.set_module(),
                _ => unimplemented!(),
            }
        }
        res
    }

    fn add_type_hint_to_cp(cp_builder: &mut ConstantPoolBuilder, hint: TypeHint) -> u16 {
        match hint {
            TypeHint::Class(_, class_name) => cp_builder.add_class(&class_name.value),
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
