use crate::ast::JasmClass;
use crate::token::{JasmToken, JasmTokenKind, Span};
use std::iter::Peekable;
use std::ops::Range;
use std::vec::IntoIter;

#[derive(Debug, Eq, PartialEq, Clone)]
pub(crate) enum ParserError {
    ClassDirectiveExpected(Span, JasmTokenKind),
    ClassNameExpected(Span),
    StringLiteralAsClassName(Span),
    EmptyFile(Span),
    Internal(String),
}

impl ParserError {
    pub fn message(&self) -> Option<String> {
        match self {
            ParserError::ClassDirectiveExpected(_, token) => {
                Some(format!("unexpected {}", token.as_string_token_type()))
            }
            ParserError::StringLiteralAsClassName(_) => {
                Some("incorrect class definition".to_string())
            }
            ParserError::ClassNameExpected(_) => Some("incomplete class definition".to_string()),
            ParserError::EmptyFile(_) => Some("empty file".to_string()),
            ParserError::Internal(msg) => Some(format!("Internal parser error: {}", msg)),
        }
    }

    pub fn note(&self) -> Option<String> {
        match self {
            ParserError::ClassDirectiveExpected(_, _) => {
                //TODO: actually it is false. I guess the source file name or class file version could be added to the note when implemented
                Some("A Java assembly file must start with a '.class' definition.".to_string())
            }
            ParserError::ClassNameExpected(_) => Some(
                "The .class directive requires a name:\n.class [access_flags] <name>".to_string(),
            ),
            ParserError::StringLiteralAsClassName(_) => {
                Some("Consider removing the quotes around the value".to_string())
            }
            ParserError::Internal(_) | ParserError::EmptyFile(_) => None,
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
            ParserError::StringLiteralAsClassName(_) => Some(
                "Class names cannot be string literals. They should be identifiers (e.g., 'com/myapp/Main')."
                    .to_string(),
            ),
            ParserError::ClassNameExpected(_) => {
                Some("Expected a class identifier (e.g., 'com/myapp/Main')".to_string())
            }
            ParserError::EmptyFile(_) => Some("The file contains no class definition.".to_string()),
            ParserError::Internal(_) => None,
        }
    }

    fn span(&self) -> Option<&Span> {
        match self {
            ParserError::ClassDirectiveExpected(span, _)
            | ParserError::EmptyFile(span)
            | ParserError::StringLiteralAsClassName(span)
            | ParserError::ClassNameExpected(span) => Some(span),
            ParserError::Internal(_) => None,
        }
    }
}

pub struct JasmParser {
    tokens: Peekable<IntoIter<JasmToken>>,
    last_span: Span,
}

impl JasmParser {
    fn skip_newlines(&mut self) -> Result<(), ParserError> {
        while let Some(JasmToken {
            kind: JasmTokenKind::Newline,
            ..
        }) = self.tokens.peek()
        {
            self.next_token()?;
        }
        Ok(())
    }

    fn next_token(&mut self) -> Result<JasmToken, ParserError> {
        match self.tokens.next() {
            Some(token) => {
                self.last_span = token.span;
                Ok(token)
            }
            None => Err(ParserError::Internal(
                "Token stream ended before EOF token".to_string(),
            )),
        }
    }

    //TODO: add all flags, handle orders, and check for duplicates
    fn parse_class_access_flags(&mut self) -> Result<u16, ParserError> {
        let mut flags = 0u16;
        loop {
            match self.tokens.peek() {
                Some(JasmToken {
                    kind: JasmTokenKind::Public,
                    ..
                }) => {
                    flags |= 0x0001; // ACC_PUBLIC
                    self.next_token()?;
                }
                Some(JasmToken {
                    kind: JasmTokenKind::Static,
                    ..
                }) => {
                    flags |= 0x0008; // ACC_STATIC
                    self.next_token()?;
                }
                _ => break,
            }
        }
        Ok(flags)
    }

    fn parse_class(&mut self) -> Result<JasmClass, ParserError> {
        self.skip_newlines()?;
        let class_token = self.next_token()?;
        if matches!(class_token.kind, JasmTokenKind::Eof) {
            return Err(ParserError::EmptyFile(class_token.span));
        }
        if !matches!(class_token.kind, JasmTokenKind::DotClass) {
            return Err(ParserError::ClassDirectiveExpected(
                class_token.span,
                class_token.kind,
            ));
        }
        let _access_flags = self.parse_class_access_flags()?;
        let access_flag_end = self.last_span.end;
        let name_token = self.next_token()?;
        let class_name = if let JasmTokenKind::Identifier(name) = name_token.kind {
            name
        } else {
            if matches!(name_token.kind, JasmTokenKind::StringLiteral(_)) {
                return Err(ParserError::StringLiteralAsClassName(name_token.span));
            }
            let right_span = match name_token.kind {
                JasmTokenKind::Eof | JasmTokenKind::Newline => {
                    Span::new(access_flag_end, access_flag_end)
                }
                _ => name_token.span,
            };
            return Err(ParserError::ClassNameExpected(right_span));
        };

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
            last_span: Span::new(0, 0),
        };

        instance.parse_class()
    }
}

#[cfg(test)]
mod tests;
