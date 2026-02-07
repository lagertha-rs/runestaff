use crate::ast::JasmClass;
use crate::token::{JasmToken, JasmTokenKind, Span};
use std::iter::Peekable;
use std::ops::Range;
use std::vec::IntoIter;

#[derive(Debug, Eq, PartialEq, Clone)]
pub(crate) enum ParserError {
    ClassDirectiveExpected(Span, JasmTokenKind),
    Internal(String),
}

impl ParserError {
    pub fn message(&self) -> Option<String> {
        match self {
            ParserError::ClassDirectiveExpected(_, token) => {
                Some(format!("unexpected {}", token.as_string_token_type()))
            }
            ParserError::Internal(msg) => Some(format!("Internal parser error: {}", msg)),
        }
    }

    pub fn note(&self) -> Option<String> {
        match self {
            ParserError::ClassDirectiveExpected(_, _) => {
                //TODO: actually it is false. I guess the source file name or class file version could be added to the note when implemented
                Some("A Java assembly file must start with a '.class' definition.".to_string())
            }
            ParserError::Internal(_) => None,
        }
    }

    pub fn as_range(&self) -> Option<Range<usize>> {
        self.span().map(|s| s.as_range())
    }

    pub fn label(&self) -> Option<String> {
        match self {
            ParserError::ClassDirectiveExpected(_, token) => Some(format!(
                "The '{}' {} cannot appear before a class is defined.",
                token,
                token.as_string_token_type()
            )),
            ParserError::Internal(_) => None,
        }
    }

    fn span(&self) -> Option<&Span> {
        match self {
            ParserError::ClassDirectiveExpected(span, _) => Some(span),
            ParserError::Internal(_) => None,
        }
    }
}

pub struct JasmParser {
    tokens: Peekable<IntoIter<JasmToken>>,
}

impl JasmParser {
    fn skip_newlines(&mut self) {
        while let Some(JasmToken {
            kind: JasmTokenKind::Newline,
            ..
        }) = self.tokens.peek()
        {
            self.tokens.next();
        }
    }

    fn next(&mut self) -> Result<JasmToken, ParserError> {
        match self.tokens.next() {
            Some(token) => Ok(token),
            None => Err(ParserError::Internal(
                "Token stream ended before EOF token".to_string(),
            )),
        }
    }

    fn parse_class(&mut self) -> Result<JasmClass, ParserError> {
        self.skip_newlines();
        let class_token = self.next()?;
        if !matches!(class_token.kind, JasmTokenKind::DotClass) {
            return Err(ParserError::ClassDirectiveExpected(
                class_token.span,
                class_token.kind,
            ));
        }

        //self.expect(|t| matches!(t.kind, JasmTokenKind::DotClass), ".class")?;
        todo!()
    }

    pub fn parse(tokens: Vec<JasmToken>) -> Result<JasmClass, ParserError> {
        if !matches!(tokens.last().unwrap().kind, JasmTokenKind::Eof) {
            return Err(ParserError::Internal(
                "Token stream must end with an EOF token".to_string(),
            ));
        }

        let mut instance = Self {
            tokens: tokens.into_iter().peekable(),
        };

        instance.parse_class()
    }
}

#[cfg(test)]
mod tests;
