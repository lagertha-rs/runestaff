use crate::token::{RnsToken, Spanned};

#[derive(Debug, Clone, PartialEq)]
pub(in crate::parser) enum SignedIntRejection {
    Missing(RnsToken),
    FloatingPoint(Spanned<String>),
    Overflow(Spanned<String>),
    NotNumeric(Spanned<String>),
}
