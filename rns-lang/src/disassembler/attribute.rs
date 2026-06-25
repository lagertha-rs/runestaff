use crate::disassembler::indent_write::Indented;
use crate::disassembler::instruction::fmt_instruction_rns;
use crate::disassembler::{DisasmError, DisasmResult};
use crate::token::{DIRECTIVE_DOT_CODE, DIRECTIVE_DOT_CODE_END};
use lvm_class::attribute::{CodeAttribute, MethodAttribute};
use lvm_class::bytecode::Instruction;
use lvm_class::constant_pool::ConstantPool;
use std::fmt::Write as _;

pub(crate) fn fmt_method_attribute_rns(
    attribute: &MethodAttribute,
    method_name: &str,
    ind: &mut Indented,
    cp: &ConstantPool,
) -> DisasmResult<()> {
    ind.with_indent(|ind| match attribute {
        MethodAttribute::Code(code) => fmt_code_attribute_rns(code, method_name, ind, cp),
        other => Err(DisasmError::UnsupportedMethodAttribute {
            method: method_name.to_string(),
            attribute: format!("{other:?}"),
        }),
    })
}

fn fmt_code_attribute_rns(
    code: &CodeAttribute,
    method_name: &str,
    ind: &mut Indented,
    cp: &ConstantPool,
) -> DisasmResult<()> {
    writeln!(
        ind,
        "{} stack {} locals {}",
        DIRECTIVE_DOT_CODE, code.max_stack, code.max_locals
    )?;
    ind.with_indent(|ind| {
        let mut pc = 0;
        while pc < code.code.len() {
            let inst = Instruction::new_at(&code.code, pc)?;
            fmt_instruction_rns(&inst, ind, cp)?;
            pc += inst.byte_size() as usize;
        }
        Ok(())
    })?;

    if !code.exception_table.is_empty() {
        return Err(DisasmError::UnsupportedExceptionTable {
            method: method_name.to_string(),
            handlers: code.exception_table.len(),
        });
    }

    if let Some(attribute) = code.attributes.first().map(code_attribute_name) {
        return Err(DisasmError::UnsupportedCodeAttribute {
            method: method_name.to_string(),
            attribute,
        });
    }

    writeln!(ind, "{}", DIRECTIVE_DOT_CODE_END)?;
    Ok(())
}

fn code_attribute_name(
    attribute: &lvm_class::attribute::method::CodeAttributeInfo,
) -> &'static str {
    match attribute {
        lvm_class::attribute::method::CodeAttributeInfo::LineNumberTable(_) => "LineNumberTable",
        lvm_class::attribute::method::CodeAttributeInfo::LocalVariableTable(_) => {
            "LocalVariableTable"
        }
        lvm_class::attribute::method::CodeAttributeInfo::StackMapTable(_) => "StackMapTable",
        lvm_class::attribute::method::CodeAttributeInfo::LocalVariableTypeTable(_) => {
            "LocalVariableTypeTable"
        }
        lvm_class::attribute::method::CodeAttributeInfo::RuntimeVisibleTypeAnnotations => {
            "RuntimeVisibleTypeAnnotations"
        }
        lvm_class::attribute::method::CodeAttributeInfo::RuntimeInvisibleTypeAnnotations => {
            "RuntimeInvisibleTypeAnnotations"
        }
    }
}
