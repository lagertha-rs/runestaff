use crate::assembler::error::AssemblerError;
use crate::assembler::jvm_warning::JvmWarning;
use crate::ast::flag::{RnsClassFlag, RnsMethodFlag};
use crate::ast::{MethodDirective, RnsModule, RnsOperand};
use crate::diagnostic::{Diagnostic, DiagnosticTier};
use crate::instruction::InstructionNumericOperand;
use crate::token::type_hint::TypeHint;
use lvm_class::ClassFile;
use lvm_class::attribute::AttributeKind;
use lvm_class::flags::ClassFlags;
use lvm_class::prelude::{
    ClassFileBuilder, CodeAttribute, ConstantPoolBuilder, MethodAttribute, MethodFlags, MethodInfo,
};
use lvm_class::verify::{ClassFlagsFinding, Finding};
use std::collections::HashMap;

mod error;
mod jvm_warning;

pub struct AssembledClasses {
    pub package: String,
    pub classes: HashMap<String, Vec<u8>>,
}

impl RnsModule {
    fn build_class_flags(&mut self) -> ClassFlags {
        let mut res = ClassFlags::new(0);
        for flag in self.class_dir.flags.keys() {
            match flag {
                RnsClassFlag::Public => res.set_public(),
                RnsClassFlag::Final => res.set_final(),
                RnsClassFlag::Super => res.set_super(),
                RnsClassFlag::Interface => res.set_interface(),
                RnsClassFlag::Abstract => res.set_abstract(),
                RnsClassFlag::Enum => res.set_enum(),
                RnsClassFlag::Synthetic => res.set_synthetic(),
                RnsClassFlag::Annotation => res.set_annotation(),
                RnsClassFlag::Module => res.set_module(),
            }
        }
        res
    }

    fn build_method_flags(&mut self, method_dir: &MethodDirective) -> u16 {
        let mut res = 0;
        for flag in method_dir.flags.keys() {
            match flag {
                RnsMethodFlag::Public => res |= 0x0001,
                RnsMethodFlag::Private => res |= 0x0002,
                RnsMethodFlag::Protected => res |= 0x0004,
                RnsMethodFlag::Static => res |= 0x0008,
                RnsMethodFlag::Final => res |= 0x0010,
                RnsMethodFlag::Synchronized => res |= 0x0020,
                RnsMethodFlag::Bridge => res |= 0x0040,
                RnsMethodFlag::Varargs => res |= 0x0080,
                RnsMethodFlag::Native => res |= 0x0100,
                RnsMethodFlag::Abstract => res |= 0x0400,
                RnsMethodFlag::Strict => res |= 0x0800,
                RnsMethodFlag::Synthetic => res |= 0x1000,
            }
        }
        res
    }

    fn add_type_hint_to_cp(cp_builder: &mut ConstantPoolBuilder, hint: TypeHint) -> u16 {
        match hint {
            TypeHint::Utf8(_, utf8_value) => cp_builder.add_utf8(&utf8_value.value),
            TypeHint::Class(_, class_name) => cp_builder.add_class(&class_name.value),
            TypeHint::Integer(_, int_value) => cp_builder.add_integer(int_value.value),
            TypeHint::Methodref(r) => {
                cp_builder.add_methodref(&r.class.value, &r.name.value, &r.descriptor.value)
            }
            TypeHint::Fieldref(r) => {
                cp_builder.add_fieldref(&r.class.value, &r.name.value, &r.descriptor.value)
            }
            TypeHint::String(_, string_value) => cp_builder.add_string(&string_value.value),
            TypeHint::CpIndex(_, explicit_idx) => explicit_idx.value, // TODO: warn somewhere?
            _ => unimplemented!(),
        }
    }

    fn build_super_class(&mut self, cp_builder: &mut ConstantPoolBuilder) -> Option<u16> {
        let name = self.super_dir.take()?.name;
        Some(Self::add_type_hint_to_cp(cp_builder, name))
    }

    fn build_method_directive(
        &mut self,
        cp_builder: &mut ConstantPoolBuilder,
        method_dir: MethodDirective,
    ) -> MethodInfo {
        let access_flags = MethodFlags::new(self.build_method_flags(&method_dir));
        let name_index = Self::add_type_hint_to_cp(cp_builder, method_dir.name.unwrap());
        let descriptor_index =
            Self::add_type_hint_to_cp(cp_builder, method_dir.descriptor.unwrap());

        let attributes = if let Some(code_dir) = method_dir.code_dir {
            let mut code = Vec::new();
            for ins in code_dir.instructions {
                let opcode = ins.spec.opcode;
                code.push(opcode as u8);
                match ins.operand {
                    None => {}
                    Some(RnsOperand::CpRef(hint)) => {
                        let cp_index = Self::add_type_hint_to_cp(cp_builder, hint);
                        match opcode.operand_size() {
                            1 => code.push(cp_index as u8),
                            2 => code.extend(cp_index.to_be_bytes()),
                            _ => unimplemented!(),
                        }
                    }
                    Some(RnsOperand::Numeric(numeric_kind, v)) => match numeric_kind {
                        InstructionNumericOperand::Byte => code.push(v.value as u8),
                        InstructionNumericOperand::Short => {
                            code.extend((v.value as i16).to_be_bytes())
                        }
                        InstructionNumericOperand::Int => {
                            code.extend((v.value as i32).to_be_bytes())
                        }
                    },
                    Some(RnsOperand::Label(label)) => {
                        let target_pc = match code_dir.labels.get(&label.value) {
                            Some(pc) => *pc,
                            None => {
                                self.diagnostics
                                    .push(AssemblerError::UndefinedLabel { label }.into());
                                0
                            }
                        };
                        let current_pc = (code.len() - 1) as u32; // pc of opcode
                        let offset = (target_pc as i32) - (current_pc as i32);
                        match opcode.operand_size() {
                            2 => code.extend((offset as i16).to_be_bytes()),
                            4 => code.extend(offset.to_be_bytes()),
                            _ => unimplemented!(),
                        }
                    }
                }
            }
            let attr_name_idx = cp_builder.add_attribute_utf8(AttributeKind::Code);
            let code_attribute = MethodAttribute::Code {
                attr_name_idx,
                code_attr: CodeAttribute {
                    max_stack: code_dir.max_stack,
                    max_locals: code_dir.max_locals,
                    code,
                    exception_table: vec![],
                    attributes: vec![],
                },
            };
            vec![code_attribute]
        } else {
            // abstract and native methods don't need code
            vec![]
        };

        MethodInfo {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        }
    }

    pub fn into_bytes(self) -> (Option<AssembledClasses>, Vec<Diagnostic>) {
        let package = self
            .package
            .as_ref()
            .map(|p| p.name.clone())
            .unwrap_or_default();

        let class_name = self
            .class_dir
            .name
            .as_ref()
            .map(|n| n.value())
            .unwrap_or_default();

        let full_class_name = if package.is_empty() {
            class_name.to_string()
        } else {
            format!("{}/{}", package, class_name)
        };

        let (class_file, diagnostics) = self.into_class_file();
        let bytes = class_file.map(|cf| cf.to_bytes());

        let assembled = bytes.map(|b| {
            let mut classes = HashMap::new();
            classes.insert(full_class_name, b);
            AssembledClasses { package, classes }
        });

        (assembled, diagnostics)
    }

    // TODO: why tuple but not Result?
    fn into_class_file(mut self) -> (Option<ClassFile>, Vec<Diagnostic>) {
        let mut cp_builder = ConstantPoolBuilder::new();
        let super_cp_id = self.build_super_class(&mut cp_builder);
        let class_flags = self.build_class_flags();

        let package = self
            .package
            .as_ref()
            .map(|p| p.name.clone())
            .unwrap_or_default();

        let this_cp_id = self.class_dir.name.take().map(|name| {
            let class_name = name.value();
            let full_name = if package.is_empty() {
                class_name.to_string()
            } else {
                format!("{}/{}", package, class_name)
            };
            cp_builder.add_class(&full_name)
        });

        let rns_methods = std::mem::take(&mut self.methods);
        let mut methods = Vec::with_capacity(rns_methods.len());

        for rns_method in rns_methods {
            methods.push(self.build_method_directive(&mut cp_builder, rns_method));
        }

        self.handle_inner_classes(&mut cp_builder, this_cp_id.unwrap());

        if self
            .diagnostics
            .iter()
            .any(|d| d.tier == DiagnosticTier::SyntaxError)
        {
            return (None, self.diagnostics);
        }

        let class_file = ClassFileBuilder::new(0, 69, cp_builder.build()) // TODO: allow specifying version in jasm
            .access_flags(class_flags) // TODO: set access flags based on parsed flags
            .this_class(this_cp_id)
            .super_class(super_cp_id)
            .methods(methods)
            .build();

        if let Some(class_file) = &class_file {
            let findings = class_file.verify();
            for finding in findings {
                self.map_lvm_class_finding(finding);
            }
        }

        (class_file, self.diagnostics)
    }

    fn handle_inner_classes(&mut self, cp_builder: &mut ConstantPoolBuilder, _this_cp_id: u16) {
        if self.inner_classes.is_empty() {
            return;
        }

        let _nest_members_attr_idx = cp_builder.add_attribute_utf8(AttributeKind::NestMembers);
        let _inner_classes_attr_idx = cp_builder.add_attribute_utf8(AttributeKind::InnerClasses);

        /*
        let mut nest_members = Vec::new();
        let mut inner_classes = Vec::new();

        for inner in self.inner_classes {
            if let Some(name) = inner.name {
                let idx = Self::add_type_hint_to_cp(cp_builder, name);
                nest_members.push(idx);
                InnerClassEntry {
                    inner_class_info_index: idx,
                    outer_class_info_index: this_cp_id,
                    inner_name_index: 0,       // TODO: handle inner name
                    inner_class_access_flags: 0, // TODO: handle access flags
                };
            }
        }

         */
    }

    fn map_lvm_class_finding(&mut self, finding: Finding) {
        match finding {
            Finding::ClassFlag(class_flag_finding) => match class_flag_finding {
                ClassFlagsFinding::InterfaceWithoutAbstract(_) => {
                    let interface_span = self.class_dir.flags[&RnsClassFlag::Interface];
                    self.diagnostics.push(
                        JvmWarning::InterfaceFlagWithMissingAbstract { interface_span }.into(),
                    );
                }
                ClassFlagsFinding::InterfaceIncompatibleFlags(incompatible) => {
                    let interface_span = self.class_dir.flags[&RnsClassFlag::Interface];
                    let mut exclusive_flags: Vec<_> = self
                        .class_dir
                        .flags
                        .iter()
                        .filter(|(f, _)| Self::rns_flag_in_mask(**f, incompatible))
                        .map(|(f, s)| (*f, *s))
                        .collect();
                    exclusive_flags.sort_by_key(|(f, _)| *f);
                    self.diagnostics.push(
                        JvmWarning::InterfaceMutuallyExclusive {
                            interface_span,
                            exclusive_flags,
                        }
                        .into(),
                    );
                }
            },
        }
    }

    fn rns_flag_in_mask(f: RnsClassFlag, mask: ClassFlags) -> bool {
        match f {
            RnsClassFlag::Final => mask.is_final(),
            RnsClassFlag::Super => mask.is_super(),
            RnsClassFlag::Enum => mask.is_enum(),
            RnsClassFlag::Module => mask.is_module(),
            _ => false,
        }
    }
}
