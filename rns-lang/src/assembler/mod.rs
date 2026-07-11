use crate::assembler::error::AssemblerError;
use crate::assembler::jvm_warning::JvmWarning;
use crate::ast::flag::{RnsClassFlag, RnsInnerFlag, RnsMethodFlag};
use crate::ast::{InnerClassDirective, MethodDirective, RnsModule, RnsOperand};
use crate::diagnostic::{Diagnostic, DiagnosticTier};
use crate::instruction::InstructionNumericOperand;
use crate::token::type_hint::TypeHint;
use lvm_class::ClassFile;
use lvm_class::attribute::{AttributeKind, ClassAttribute, InnerClassEntry};
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

    fn build_method_flags(method_dir: &MethodDirective) -> u16 {
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

    fn build_inner_class_flags(flags: &HashMap<RnsInnerFlag, crate::token::Span>) -> u16 {
        let mut res = 0u16;
        for flag in flags.keys() {
            match flag {
                RnsInnerFlag::Public => res |= 0x0001,
                RnsInnerFlag::Private => res |= 0x0002,
                RnsInnerFlag::Protected => res |= 0x0004,
                RnsInnerFlag::Static => res |= 0x0008,
                RnsInnerFlag::Final => res |= 0x0010,
                RnsInnerFlag::Interface => res |= 0x0200,
                RnsInnerFlag::Abstract => res |= 0x0400,
                RnsInnerFlag::Synthetic => res |= 0x1000,
                RnsInnerFlag::Annotation => res |= 0x2000,
                RnsInnerFlag::Enum => res |= 0x4000,
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
        method_dir: &MethodDirective,
    ) -> MethodInfo {
        Self::build_method(cp_builder, method_dir, &mut self.diagnostics)
    }

    fn build_method(
        cp_builder: &mut ConstantPoolBuilder,
        method_dir: &MethodDirective,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> MethodInfo {
        let access_flags = MethodFlags::new(Self::build_method_flags(method_dir));
        let name_index = Self::add_type_hint_to_cp(cp_builder, method_dir.name.clone().unwrap());
        let descriptor_index =
            Self::add_type_hint_to_cp(cp_builder, method_dir.descriptor.clone().unwrap());

        let attributes = if let Some(code_dir) = &method_dir.code_dir {
            let mut code = Vec::new();
            for ins in &code_dir.instructions {
                let opcode = ins.spec.opcode;
                code.push(opcode as u8);
                match &ins.operand {
                    None => {}
                    Some(RnsOperand::CpRef(hint)) => {
                        let cp_index = Self::add_type_hint_to_cp(cp_builder, hint.clone());
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
                                diagnostics.push(
                                    AssemblerError::UndefinedLabel {
                                        label: label.clone(),
                                    }
                                    .into(),
                                );
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

    pub fn into_bytes(mut self) -> (Option<AssembledClasses>, Vec<Diagnostic>) {
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

        // Extract inner classes before consuming self
        let inner_classes = std::mem::take(&mut self.inner_classes);

        let (class_file, mut diagnostics) = self.into_class_file(&inner_classes);
        let bytes = class_file.map(|cf| cf.to_bytes());

        let assembled = bytes.map(|b| {
            let mut classes = HashMap::new();
            classes.insert(full_class_name.clone(), b);

            // Generate class files for inner classes
            for inner in &inner_classes {
                if let Some(inner_bytes) =
                    Self::build_inner_class_file(inner, &package, &class_name, &mut diagnostics)
                {
                    let inner_name = inner.name.as_ref().map(|n| n.value()).unwrap_or_default();
                    let mangled_name = if let Some(mangled) = &inner.mangled_name_dir {
                        mangled.value().to_string()
                    } else {
                        format!("{}${}", class_name, inner_name)
                    };
                    let full_inner_name = if package.is_empty() {
                        mangled_name
                    } else {
                        format!("{}/{}", package, mangled_name)
                    };
                    classes.insert(full_inner_name, inner_bytes);
                }
            }

            AssembledClasses { package, classes }
        });

        (assembled, diagnostics)
    }

    // TODO: why tuple but not Result?
    fn into_class_file(
        mut self,
        inner_classes: &[InnerClassDirective],
    ) -> (Option<ClassFile>, Vec<Diagnostic>) {
        let mut cp_builder = ConstantPoolBuilder::new();
        let super_cp_id = self.build_super_class(&mut cp_builder);
        let class_flags = self.build_class_flags();

        let package = self
            .package
            .as_ref()
            .map(|p| p.name.clone())
            .unwrap_or_default();

        let outer_class_name = self
            .class_dir
            .name
            .as_ref()
            .map(|n| n.value())
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

        for rns_method in &rns_methods {
            methods.push(self.build_method_directive(&mut cp_builder, rns_method));
        }

        let inner_classes_attr = self.handle_inner_classes(
            &mut cp_builder,
            this_cp_id.unwrap(),
            &package,
            &outer_class_name,
            inner_classes,
        );

        if self
            .diagnostics
            .iter()
            .any(|d| d.tier == DiagnosticTier::SyntaxError)
        {
            return (None, self.diagnostics);
        }

        let mut class_file = ClassFileBuilder::new(0, 69, cp_builder.build()) // TODO: allow specifying version in jasm
            .access_flags(class_flags) // TODO: set access flags based on parsed flags
            .this_class(this_cp_id)
            .super_class(super_cp_id)
            .methods(methods)
            .build();

        // Add InnerClasses attribute if present
        if let (Some(attr), Some(ref mut cf)) = (inner_classes_attr, class_file.as_mut()) {
            cf.attributes.push(attr);
        }

        if let Some(class_file) = &class_file {
            let findings = class_file.verify();
            for finding in findings {
                self.map_lvm_class_finding(finding);
            }
        }

        (class_file, self.diagnostics)
    }

    fn handle_inner_classes(
        &mut self,
        cp_builder: &mut ConstantPoolBuilder,
        this_cp_id: u16,
        package: &str,
        outer_class_name: &str,
        inner_classes: &[InnerClassDirective],
    ) -> Option<ClassAttribute> {
        let has_inner_classes = !inner_classes.is_empty();
        let has_explicit_attrs = !self.inner_classes_attrs.is_empty();

        if !has_inner_classes && !has_explicit_attrs {
            return None;
        }

        let attr_name_idx = cp_builder.add_attribute_utf8(AttributeKind::InnerClasses);
        let mut entries = Vec::new();

        // Auto-generate entries from .inner directives (like javac)
        for inner in inner_classes {
            if let Some(name) = &inner.name {
                // inner_name_index is the simple name (the .inner operand)
                let inner_simple_name = name.value();
                let inner_name_index = cp_builder.add_utf8(&inner_simple_name);

                // inner_class_info_index is the mangled name
                // Use .mangled_name if provided, otherwise build as {outer_full_name}${inner_name}
                let mangled_class_name = if let Some(mangled) = &inner.mangled_name_dir {
                    mangled.value().to_string()
                } else {
                    let outer_full_name = if package.is_empty() {
                        outer_class_name.to_string()
                    } else {
                        format!("{}/{}", package, outer_class_name)
                    };
                    format!("{}${}", outer_full_name, inner_simple_name)
                };
                let inner_class_info_index = cp_builder.add_class(&mangled_class_name);

                // .inner uses RnsClassFlag, but InnerClasses attr needs RnsInnerFlag-compatible flags
                // For auto-generated entries, we use 0 (no special flags) since .inner doesn't have
                // the attribute-level flags (private/protected/static etc.)
                let access_flags = 0u16;

                entries.push(InnerClassEntry {
                    inner_class_info_index,
                    outer_class_info_index: this_cp_id,
                    inner_name_index,
                    inner_class_access_flags: access_flags,
                });
            }
        }

        // Add explicit entries from .inner_classes_attr directives
        for attr in &self.inner_classes_attrs {
            let inner_class_info_index = attr
                .inner_class
                .as_ref()
                .map(|h| Self::add_type_hint_to_cp(cp_builder, h.clone()))
                .unwrap_or(0);

            let outer_class_info_index = attr
                .outer_class
                .as_ref()
                .map(|h| Self::add_type_hint_to_cp(cp_builder, h.clone()))
                .unwrap_or(0);

            let inner_name_index = attr
                .inner_name
                .as_ref()
                .map(|h| Self::add_type_hint_to_cp(cp_builder, h.clone()))
                .unwrap_or(0);

            let access_flags = Self::build_inner_class_flags(&attr.flags);

            entries.push(InnerClassEntry {
                inner_class_info_index,
                outer_class_info_index,
                inner_name_index,
                inner_class_access_flags: access_flags,
            });
        }

        Some(ClassAttribute::InnerClasses {
            attr_name_idx,
            classes: entries,
        })
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

    fn build_inner_class_file(
        inner: &InnerClassDirective,
        package: &str,
        outer_class_name: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<Vec<u8>> {
        let mut cp_builder = ConstantPoolBuilder::new();

        // Build super class
        let super_cp_id = if let Some(super_dir) = &inner.super_dir {
            Some(Self::add_type_hint_to_cp(
                &mut cp_builder,
                super_dir.name.clone(),
            ))
        } else {
            // Default to java/lang/Object
            Some(cp_builder.add_class("java/lang/Object"))
        };

        // Build class flags
        let mut class_flags = ClassFlags::new(0);
        for flag in inner.flags.keys() {
            match flag {
                RnsClassFlag::Public => class_flags.set_public(),
                RnsClassFlag::Final => class_flags.set_final(),
                RnsClassFlag::Super => class_flags.set_super(),
                RnsClassFlag::Interface => class_flags.set_interface(),
                RnsClassFlag::Abstract => class_flags.set_abstract(),
                RnsClassFlag::Enum => class_flags.set_enum(),
                RnsClassFlag::Synthetic => class_flags.set_synthetic(),
                RnsClassFlag::Annotation => class_flags.set_annotation(),
                RnsClassFlag::Module => class_flags.set_module(),
            }
        }

        // Build this class name (mangled)
        let inner_name = inner.name.as_ref().map(|n| n.value()).unwrap_or_default();
        let mangled_name = if let Some(mangled) = &inner.mangled_name_dir {
            mangled.value().to_string()
        } else {
            format!("{}${}", outer_class_name, inner_name)
        };
        let full_inner_name = if package.is_empty() {
            mangled_name.clone()
        } else {
            format!("{}/{}", package, mangled_name)
        };
        let this_cp_id = cp_builder.add_class(&full_inner_name);

        // Add outer class to constant pool
        let outer_full_name = if package.is_empty() {
            outer_class_name.to_string()
        } else {
            format!("{}/{}", package, outer_class_name)
        };
        let outer_cp_id = cp_builder.add_class(&outer_full_name);

        // Add inner name (simple name) to constant pool
        let inner_name_idx = cp_builder.add_utf8(&inner_name);

        // Build InnerClasses attribute for this inner class
        let attr_name_idx = cp_builder.add_attribute_utf8(AttributeKind::InnerClasses);
        let inner_classes_attr = ClassAttribute::InnerClasses {
            attr_name_idx,
            classes: vec![InnerClassEntry {
                inner_class_info_index: this_cp_id,
                outer_class_info_index: outer_cp_id,
                inner_name_index: inner_name_idx,
                inner_class_access_flags: 0, // TODO: derive from inner.flags if needed
            }],
        };

        // Build methods
        let mut methods = Vec::with_capacity(inner.methods.len());
        for method in &inner.methods {
            methods.push(Self::build_method(&mut cp_builder, method, diagnostics));
        }

        let mut class_file = ClassFileBuilder::new(0, 69, cp_builder.build())
            .access_flags(class_flags)
            .this_class(Some(this_cp_id))
            .super_class(super_cp_id)
            .methods(methods)
            .build();

        // Add InnerClasses attribute
        if let Some(ref mut cf) = class_file {
            cf.attributes.push(inner_classes_attr);
        }

        class_file.map(|cf| cf.to_bytes())
    }
}
