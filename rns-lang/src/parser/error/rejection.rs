use crate::token::{RnsToken, Spanned};
use std::num::IntErrorKind;
use std::str::FromStr;

// TODO: move from the error module since it has both parsing and error?

#[derive(Debug, Clone, PartialEq)]
pub(in crate::parser) enum NumericRejection {
    Missing(RnsToken),
    FloatingPoint(Spanned<String>),
    Overflow(Spanned<String>),
    NotNumeric(Spanned<String>),
}

pub(in crate::parser) trait ParseNumeric: Sized {
    fn parse_and_classify(raw: &str, spanned: &Spanned<String>) -> Result<Self, NumericRejection>;
}

macro_rules! impl_parse_numeric_int {
    ($($ty:ty),+) => {
        $(impl ParseNumeric for $ty {
            fn parse_and_classify(raw: &str, spanned: &Spanned<String>) -> Result<Self, NumericRejection> {
                match <$ty>::from_str(raw) {
                    Ok(value) => Ok(value),
                    Err(e) => {
                        if raw.contains('.') || looks_like_scientific_notation(raw) {
                            Err(NumericRejection::FloatingPoint(spanned.clone()))
                        } else if matches!(e.kind(), IntErrorKind::PosOverflow | IntErrorKind::NegOverflow) {
                            Err(NumericRejection::Overflow(spanned.clone()))
                        } else {
                            Err(NumericRejection::NotNumeric(spanned.clone()))
                        }
                    }
                }
            }
        })+
    };
}

impl_parse_numeric_int!(i32, u8, i64, u16);

macro_rules! impl_parse_numeric_float {
    ($($ty:ty),+) => {
        $(impl ParseNumeric for $ty {
            fn parse_and_classify(raw: &str, spanned: &Spanned<String>) -> Result<Self, NumericRejection> {
                match <$ty>::from_str(raw) {
                    Ok(value) => {
                        if value.is_infinite() {
                            Err(NumericRejection::Overflow(spanned.clone()))
                        } else {
                            Ok(value)
                        }
                    }
                    Err(_) => Err(NumericRejection::NotNumeric(spanned.clone())),
                }
            }
        })+
    };
}

impl_parse_numeric_float!(f32, f64);

fn looks_like_scientific_notation(s: &str) -> bool {
    let s = s
        .strip_prefix('-')
        .or_else(|| s.strip_prefix('+'))
        .unwrap_or(s);
    if let Some(e_pos) = s.find('e').or_else(|| s.find('E')) {
        e_pos > 0 && s[..e_pos].chars().all(|c| c.is_ascii_digit())
    } else {
        false
    }
}
