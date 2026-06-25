use crate::disassembler::DisasmResult;
use crate::disassembler::indent_write::Indented;
use crate::token::{
    DIRECTIVE_DOT_CLASS, FLAG_ABSTRACT, FLAG_ANNOTATION, FLAG_ENUM, FLAG_FINAL, FLAG_INTERFACE,
    FLAG_MODULE, FLAG_NATIVE, FLAG_PRIVATE, FLAG_PROTECTED, FLAG_PUBLIC, FLAG_STATIC, FLAG_STRICT,
    FLAG_SUPER, FLAG_SYNCHRONIZED,
};
use lvm_class::flags::{ClassFlags, MethodFlags};
use std::fmt::Write as _;

pub(crate) fn fmt_class_flags_rns(flags: &ClassFlags, ind: &mut Indented) -> DisasmResult<()> {
    write!(ind, "{} ", DIRECTIVE_DOT_CLASS)?;
    if flags.is_public() {
        write!(ind, "{} ", FLAG_PUBLIC)?;
    }
    if flags.is_module() {
        write!(ind, "{} ", FLAG_MODULE)?;
    }
    if flags.is_annotation() {
        write!(ind, "{} ", FLAG_ANNOTATION)?;
    }
    if flags.is_interface() {
        write!(ind, "{} ", FLAG_INTERFACE)?;
    }
    if flags.is_abstract() {
        write!(ind, "{} ", FLAG_ABSTRACT)?;
    }
    if flags.is_final() {
        write!(ind, "{} ", FLAG_FINAL)?;
    }
    if flags.is_enum() {
        write!(ind, "{} ", FLAG_ENUM)?;
    }
    if flags.is_super() {
        write!(ind, "{} ", FLAG_SUPER)?;
    }
    Ok(())
}

pub(crate) fn fmt_method_flags_rns(flags: &MethodFlags, ind: &mut Indented) -> DisasmResult<()> {
    if flags.is_public() {
        write!(ind, "{} ", FLAG_PUBLIC)?;
    } else if flags.is_protected() {
        write!(ind, "{} ", FLAG_PROTECTED)?;
    } else if flags.is_private() {
        write!(ind, "{} ", FLAG_PRIVATE)?;
    }

    if flags.is_static() {
        write!(ind, "{} ", FLAG_STATIC)?;
    }
    if flags.is_final() {
        write!(ind, "{} ", FLAG_FINAL)?;
    }
    if flags.is_synchronized() {
        write!(ind, "{} ", FLAG_SYNCHRONIZED)?;
    }
    if flags.is_native() {
        write!(ind, "{} ", FLAG_NATIVE)?;
    }
    if flags.is_abstract() {
        write!(ind, "{} ", FLAG_ABSTRACT)?;
    }
    if flags.is_strict() {
        write!(ind, "{} ", FLAG_STRICT)?;
    }
    Ok(())
}
