use crate::disassembler::DisasmResult;
use crate::disassembler::attribute::fmt_method_attribute_rns;
use crate::disassembler::constant_pool::fmt_cp_entry_rns;
use crate::disassembler::flags::fmt_method_flags_rns;
use crate::disassembler::indent_write::Indented;
use crate::token::{DIRECTIVE_DOT_METHOD, DIRECTIVE_DOT_METHOD_END};
use jclass::constant_pool::ConstantPool;
use jclass::member::MethodInfo;
use std::fmt::Write as _;

fn fmt_signature(method: &MethodInfo, ind: &mut Indented, cp: &ConstantPool) -> DisasmResult<()> {
    write!(ind, "{} ", DIRECTIVE_DOT_METHOD)?;
    fmt_method_flags_rns(&method.access_flags, ind)?;
    fmt_cp_entry_rns(ind, cp, method.name_index)?;
    write!(ind, " ")?;
    fmt_cp_entry_rns(ind, cp, method.descriptor_index)?;
    writeln!(ind)?;
    Ok(())
}

fn method_name(method: &MethodInfo, cp: &ConstantPool) -> DisasmResult<String> {
    Ok(cp.get_utf8(&method.name_index)?.to_string())
}

pub(crate) fn fmt_method_rns(
    method: &MethodInfo,
    ind: &mut Indented,
    cp: &ConstantPool,
) -> DisasmResult<()> {
    let method_name = method_name(method, cp)?;
    fmt_signature(method, ind, cp)?;
    ind.with_indent(|ind| {
        for attr in &method.attributes {
            fmt_method_attribute_rns(attr, &method_name, ind, cp)?;
        }
        Ok(())
    })?;
    writeln!(ind, "{}", DIRECTIVE_DOT_METHOD_END)?;
    Ok(())
}
