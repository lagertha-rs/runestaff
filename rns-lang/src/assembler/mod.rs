use crate::diagnostic::Diagnostic;
use crate::token::{RnsAccessFlag, Span};
use jclass::ClassFile;
use jclass::flags::ClassFlags;
use jclass::prelude::{AttributeNameMap, ConstantPoolBuilder};
use std::collections::BTreeMap;

mod jvm_warning;

pub struct RnsModule {
    pub class_directive: ClassDirective,
    pub super_directives: Vec<SuperDirective>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Default)]
pub struct ClassDirective {
    pub directive_span: Span,
    pub name: String,
    pub name_span: Span,
    // TODO: BTreeMap because I need it to be sorted for my snapshot test. investigate impact
    pub access_flags: BTreeMap<RnsAccessFlag, Span>,
}

pub struct SuperDirective {
    pub name: Option<String>,
    pub identifier_span: Option<Span>,
    pub directive_span: Span,
}

impl RnsModule {
    fn build_class_flags(&mut self) -> ClassFlags {
        let mut res = ClassFlags::new(0);
        for (flag, span) in &self.class_directive.access_flags {
            match flag {
                RnsAccessFlag::Public => res.set_public(),
                RnsAccessFlag::Final => res.set_final(),
                RnsAccessFlag::Super => res.set_super(),
                RnsAccessFlag::Interface => res.set_interface(),
                RnsAccessFlag::Abstract => res.set_abstract(),
                RnsAccessFlag::Enum => res.set_enum(),
                RnsAccessFlag::Synthetic => res.set_synthetic(),
                RnsAccessFlag::Annotation => res.set_annotation(),

                _ => unimplemented!(),
            }
        }
        res
    }

    // TODO: need to test that I build exactly same CP as javac
    pub fn into_class_file(mut self) -> (ClassFile, Vec<Diagnostic>) {
        let mut cp_builder = ConstantPoolBuilder::new();
        let super_name = self.super_directives[0].name.clone().unwrap();
        let class_flags = self.build_class_flags();

        let this_cp_id = cp_builder.add_class(&self.class_directive.name);
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
