use crate::assembler::jvm_warning::JvmWarning;
use crate::diagnostic::Diagnostic;
use crate::token::type_hint::TypeHint;
use crate::token::{RnsFlag, Span};
use jclass::flags::ClassFlags;
use jclass::prelude::{AttributeNameMap, ConstantPoolBuilder};
use jclass::ClassFile;
use std::collections::BTreeMap;

mod jvm_warning;

pub struct RnsModule {
    pub class_dir: ClassDirective,
    pub super_directives: Vec<SuperDirective>,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct ClassDirective {
    pub directive_span: Span,
    pub name: TypeHint,
    // TODO: BTreeMap because I need it to be sorted for my snapshot test. investigate impact
    pub flags: BTreeMap<RnsFlag, Span>,
}

pub struct SuperDirective {
    pub name: Option<String>,
    pub identifier_span: Option<Span>,
    pub directive_span: Span,
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
            TypeHint::Class(class_name) => cp_builder.add_class(&class_name.value),
            _ => unimplemented!(),
        }
    }

    // TODO: need to test that I build exactly same CP as javac
    pub fn into_class_file(mut self) -> (ClassFile, Vec<Diagnostic>) {
        let mut cp_builder = ConstantPoolBuilder::new();
        let super_name = self.super_directives[0].name.clone().unwrap();
        let class_flags = self.build_class_flags();

        let this_cp_id = Self::add_type_hint_to_cp(&mut cp_builder, self.class_dir.name);
        let super_cp_id = cp_builder.add_class(&super_name);

        (
            ClassFile {
                minor_version: 0,
                major_version: 69, // TODO: allow specifying version in jasm
                cp: cp_builder.build(),
                access_flags: class_flags, // TODO: set access flags based on parsed flags
                this_class: this_cp_id,
                super_class: super_cp_id,
                interfaces: vec![],
                fields: vec![],
                //methods: std::mem::take(&mut self.methods),
                methods: vec![],
                attributes: vec![],
                attribute_names: AttributeNameMap::new(),
            },
            self.diagnostics,
        )
    }
}
