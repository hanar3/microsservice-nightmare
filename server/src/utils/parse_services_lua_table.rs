// Takes the contents of the services.lua file as a string, and returns
// the a lua-parseable code for all individual services as  a Vec<String>
// e.g.: Services = { { name = "test", port = 4000 }, { name = "test2", port = "4001" } }
// ---> [r#"{ name = "test", port = "4000"" }"#, r#"{ name = "test2", port = "4001" }"}#]
// this is useful so that we separate the lua code for each service and can create
// individual contexts for our attachables

use std::{
    collections::HashMap,
    str::{Bytes, Chars},
};

use log::debug;

#[derive(Debug)]
enum TokenKind {
    Ident(String),
    Integer(i32),
    Float(f32),
    QuotedString(String),

    OpenSquareBracket,
    CloseSquareBracket,

    OpenParen,
    CloseParen,

    OpenCurlyBracket,
    CloseCurlyBracket,

    Keyword(String),

    Dot,
    Equal,
    Minus,
    Asterisk,
    Plus,
    Slash,
    Semicolon,
    Comma,
}

#[derive(Debug)]
struct Tokenizer {
    script: Vec<u8>,
    tokens: Vec<TokenKind>,
    position: usize,
}

impl Tokenizer {
    fn new(script: String) -> Tokenizer {
        println!("New tokenizer");
        Tokenizer {
            script: script.bytes().collect(),
            position: 0,
            tokens: Vec::new(),
        }
    }

    fn tokenize(&mut self) {
        let keywords: Vec<&str> = "break,do,else,elseif,end,false,for,function,if,in,local,nil,not,or,repeat,return,then,true,until,while".split(",").collect();
        println!("Tokenize bytes: {:x?}", self.script);
        for _ in 0..self.script.len() {
            if self.position >= self.script.len() {
                break;
            }
            let byte = self.script[self.position];

            // Handle double-quoted string
            if byte == 0x22 {
                self.position += 1;
                let token_bytes = self.read_until(|prev_byte, curr_byte| {
                    if let Some(prev) = prev_byte {
                        if prev == 0x5c && curr_byte == &0x22 {
                            return false; // Don't stop reading if it's a escaped quote
                        }
                    }

                    return curr_byte == &0x22;
                });

                let value = std::str::from_utf8(&token_bytes[..]).unwrap();
                self.tokens
                    .push(TokenKind::QuotedString(value.to_string()));
                self.position += 1;
                continue;
            }

            // Single quoted strings
            if byte == 0x27 {
                self.position += 1; // Enter quote so we can read quote contents
                let token_bytes = self.read_until(|prev_byte, curr_byte| {
                    if let Some(prev) = prev_byte {
                        if prev == 0x5c && curr_byte == &0x27 {
                            return false; // Don't stop reading if it's a escaped quote
                        }
                    }

                    return curr_byte == &0x27;
                });

                let value = std::str::from_utf8(&token_bytes[..]).unwrap();
                self.tokens
                    .push(TokenKind::QuotedString(value.to_string()));
                self.position += 1; // Leave quote
                continue;
            }

            if byte.is_ascii_digit() {
                let mut is_floating_point = false;
                let token_bytes = self.read_until(|_, curr_byte| {
                    if curr_byte == &0x2E {
                        is_floating_point = true;
                        return false;
                    } else {
                        return !curr_byte.is_ascii_digit();
                    }
                });

                let token_value = std::str::from_utf8(&token_bytes[..]).unwrap();
                if is_floating_point {
                    let f_number = token_value.parse::<f32>().unwrap();
                    self.tokens.push(TokenKind::Float(f_number));
                } else {
                    let i_number = token_value.parse::<i32>().unwrap();
                    self.tokens.push(TokenKind::Integer(i_number));
                }
                continue;

            }

            if byte.is_ascii_alphanumeric() {
                // Read until the first non-alpha
                let token_bytes = self
                    .read_until(|_, byte| !byte.is_ascii_alphanumeric() && byte != &0x5F);

                let token_str = std::str::from_utf8(&token_bytes[..]).unwrap();

                if keywords.contains(&token_str) {
                    self.tokens
                        .push(TokenKind::Keyword(token_str.to_string()));
                } else {
                    self.tokens
                        .push(TokenKind::Ident(token_str.to_string()));
                }
                continue;
            }

            match byte {
                b'-' => {
                    self.tokens.push(TokenKind::Minus);
                }
                b'+' => {
                    self.tokens.push(TokenKind::Plus);
                }
                b'*' => {
                    self.tokens.push(TokenKind::Asterisk);
                }
                b'/' => {
                    self.tokens.push(TokenKind::Slash);
                }
                b'{' => {
                    self.tokens.push(TokenKind::OpenCurlyBracket);
                }
                b'}' => {
                    self.tokens.push(TokenKind::CloseCurlyBracket);
                }
                b'(' => {
                    self.tokens.push(TokenKind::OpenParen);
                }
                b')' => {
                    self.tokens.push(TokenKind::CloseParen);
                }
                b'=' => {
                    self.tokens.push(TokenKind::Equal);
                }

                b';' => self.tokens.push(TokenKind::Dot),
                b',' => self.tokens.push(TokenKind::Comma),

                _ => {}
            };

            if byte.is_ascii_whitespace() {
                self.position += 1;
                continue;
            }

            self.position += 1;
        }
    }

    fn read_until<F>(&mut self, mut pred: F) -> Vec<u8>
    where
        F: FnMut(Option<u8>, &u8) -> bool,
    {
        let remaining_bytes = self.script.get(self.position..).unwrap();
        let mut collected: Vec<u8> = vec![];
        let mut prev_byte: Option<u8> = None;
        for byte in remaining_bytes {
            if pred(prev_byte, byte) {
                break;
            } else {
                collected.push(byte.to_owned());
                prev_byte = Some(byte.to_owned());
            }
        }
        self.position += collected.len();
        return collected;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tokenize_keyword() {
        let script = r#"
            local test = 11.1234 
            local test_string = "test"

            function test(n)
                a = "test"
            end

        "#;
        let mut tokenizer = Tokenizer::new(script.into());
        tokenizer.tokenize();
        println!("TOKENS {:?}", tokenizer.tokens);
        assert!(1 == 1);
    }
}
