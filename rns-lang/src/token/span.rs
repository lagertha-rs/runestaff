use crate::token::RnsFlag;
use std::ops::Range;

#[derive(Debug, Eq, PartialEq, Clone, Copy, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize, // is exclusive
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub(crate) fn as_range(&self) -> Range<usize> {
        self.start..self.end
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct SpannedString {
    pub value: String,
    pub span: Span,
}

impl SpannedString {
    pub fn new(value: String, span: Span) -> Self {
        Self { value, span }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct SpannedInteger {
    pub value: i32,
    pub span: Span,
}

impl SpannedInteger {
    pub fn new(value: i32, span: Span) -> Self {
        Self { value, span }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct SpannedFlag {
    pub value: RnsFlag,
    pub span: Span,
}

impl SpannedFlag {
    pub fn new(value: RnsFlag, span: Span) -> Self {
        Self { value, span }
    }
}
