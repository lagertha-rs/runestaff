use std::iter::Peekable;
use std::str::Chars;

pub(super) struct Cursor<'a> {
    data: Peekable<Chars<'a>>,
    cur_line: usize,
    cur_column: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(data: Peekable<Chars<'a>>) -> Self {
        Self {
            data,
            cur_column: 0,
            cur_line: 1,
        }
    }

    pub fn skip_whitespaces_and_comments(&mut self) {
        while let Some(&c) = self.data.peek() {
            match c {
                ' ' | '\t' | '\r' => {
                    self.next_char();
                }
                ';' => {
                    self.next_char();
                    while let Some(&c2) = self.data.peek() {
                        if c2 != '\n' {
                            self.next_char();
                        } else {
                            break;
                        }
                    }
                }
                _ => break,
            }
        }
    }

    pub fn next_char(&mut self) -> Option<char> {
        if let Some(n) = self.data.next() {
            self.cur_column += 1;
            if n == '\n' {
                self.cur_column = 0;
                self.cur_line += 1;
            }
            Some(n)
        } else {
            None
        }
    }

    pub fn next_string_while<F>(&mut self, condition: F) -> String
    where
        F: Fn(char) -> bool,
    {
        let mut result = String::new();
        while let Some(&c) = self.data.peek() {
            if condition(c) {
                result.push(c);
                self.next_char();
            } else {
                break;
            }
        }
        result
    }

    pub fn peek(&mut self) -> Option<char> {
        self.data.peek().cloned()
    }

    pub fn current_line_nbr(&self) -> usize {
        self.cur_line
    }

    pub fn current_column_nbr(&self) -> usize {
        self.cur_column
    }
}
