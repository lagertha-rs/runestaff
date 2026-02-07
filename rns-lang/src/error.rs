use std::ops::Range;

use crate::lexer::LexerError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JasmError {
    message: String,
    range: Range<usize>,
    note: String,
    label: String,
}

impl JasmError {
    pub fn new(
        message: impl Into<String>,
        range: Range<usize>,
        note: impl Into<String>,
        label: impl Into<String>,
    ) -> Self {
        Self {
            message: message.into(),
            range,
            note: note.into(),
            label: label.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn range(&self) -> &Range<usize> {
        &self.range
    }

    pub fn note(&self) -> &str {
        &self.note
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

impl From<LexerError> for JasmError {
    fn from(err: LexerError) -> Self {
        JasmError::new("lexing error", err.as_range(), err.note(), err.label())
    }
}
