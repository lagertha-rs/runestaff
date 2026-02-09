use crate::ast::JasmClass;
use crate::token::{JasmToken, JasmTokenKind, Span};
use std::iter::Peekable;
use std::ops::Range;
use std::vec::IntoIter;

#[derive(Debug, Eq, PartialEq, Clone)]
pub(crate) enum ParserError {
    ClassDirectiveExpected(Span, JasmTokenKind),
    ClassDirectiveTrailingTokens(Vec<JasmToken>, String),
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
            ParserError::ClassDirectiveTrailingTokens(_, class_name) => {
                Some(format!("trailing characters after '{class_name}'"))
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
            ParserError::ClassDirectiveTrailingTokens(tokens, class_name) => Some(format!(
                "The class definition should end after the class name '{class_name}'.\n{}",
                match tokens[0].kind {
                    JasmTokenKind::DotSuper =>
                        "Consider starting a new line for the '.super' directive.",
                    JasmTokenKind::OpenParen =>
                        "If you're trying to define a method, use the '.method' directive instead.",
                    JasmTokenKind::DotMethod =>
                        "Consider starting a new line for the '.method' directive.",
                    JasmTokenKind::Public | JasmTokenKind::Static =>
                        "Access flags must appear before the class name:\n.class [access_flags] <name>",
                    _ =>
                        "Unexpected tokens after class name. Consider starting a new line for the next directive.",
                }
            )),
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
            ParserError::ClassDirectiveTrailingTokens(_, _) => {
                Some("Class headers must end after the name.".to_string())
            },
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

    fn span(&self) -> Option<Span> {
        match self {
            ParserError::ClassDirectiveExpected(span, _)
            | ParserError::EmptyFile(span)
            | ParserError::StringLiteralAsClassName(span)
            | ParserError::ClassNameExpected(span) => Some(*span),
            ParserError::ClassDirectiveTrailingTokens(tokens, _) => Some(Span::new(
                tokens[0].span.start,
                tokens.last().map(|v| v.span.end).unwrap_or(0),
            )),
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

    fn next_until_newline(&mut self) -> Result<Vec<JasmToken>, ParserError> {
        let mut tokens = Vec::new();
        while let Some(token) = self.tokens.peek() {
            if matches!(token.kind, JasmTokenKind::Newline | JasmTokenKind::Eof) {
                break;
            }
            tokens.push(self.next_token()?);
        }
        Ok(tokens)
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

        // TODO: make a reusable function
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

        // TODO: test EOF right after class name and check for correct span in error
        let trailing_tokens = self.next_until_newline()?;
        if !trailing_tokens.is_empty() {
            return Err(ParserError::ClassDirectiveTrailingTokens(
                trailing_tokens,
                class_name,
            ));
        }

        while let Some(token) = self.tokens.peek() {
            match token.kind {
                JasmTokenKind::Newline => {
                    self.next_token()?;
                }
                JasmTokenKind::DotMethod => unimplemented!(),
                JasmTokenKind::DotSuper => unimplemented!(),
                JasmTokenKind::DotEnd => todo!(), // TODO: check for .end class and break
                JasmTokenKind::Eof => break,
                _ => todo!("Unexpected token in class body"),
            }
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
            last_span: Span::new(0, 0),
        };

        instance.parse_class()
    }
}

#[cfg(test)]
mod tests;
