// Takes the contents of the services.lua file as a string, and returns
// the a lua-parseable code for all individual services as  a Vec<String>
// e.g.: Services = { { name = "test", port = 4000 }, { name = "test2", port = "4001" } }
// ---> [r#"{ name = "test", port = "4000"" }"#, r#"{ name = "test2", port = "4001" }"}#]
// this is useful so that we separate the lua code for each service and can create
// individual contexts for our attachables

use std::{str::{Chars, Bytes}, collections::HashMap};

use log::debug;

#[derive(Debug)]
enum TokenCategory {
    Ident(String),
    Integer(usize),
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
    
}

#[derive(Debug)]
struct Tokenizer {
    script: Vec<u8>,
    tokens: Vec<TokenCategory>,
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
            if self.position >= self.script.len() { break; } 
            let byte = self.script[self.position]; 
            println!("byte: {:x?}", byte); 
            
            // Handle double-quoted string
            if byte == 0x22 {
                self.position += 1;
                let token_bytes = self.read_until(|prev_byte, curr_byte|{ 
                    if let Some(prev) = prev_byte {
                        if prev == 0x5c && curr_byte == &0x22 {
                            return false; // Don't stop reading if it's a escaped quote
                        }
                    }

                    return curr_byte == &0x22;
                });

                let value = std::str::from_utf8(&token_bytes[..]).unwrap();
                self.tokens.push(TokenCategory::QuotedString(value.to_string()));
            }

            // Single quoted strings
            if byte == 0x27 {
                self.position += 1;
                let token_bytes = self.read_until(|prev_byte, curr_byte|{ 
                    if let Some(prev) = prev_byte {
                        if prev == 0x5c && curr_byte == &0x27 {
                            return false; // Don't stop reading if it's a escaped quote
                        }
                    }

                    return curr_byte == &0x27;
                });

                let value = std::str::from_utf8(&token_bytes[..]).unwrap();
                self.tokens.push(TokenCategory::QuotedString(value.to_string()));
            }
            


            if byte.is_ascii_alphanumeric() {

                // Read until the first non-alpha
                let token_bytes = self.read_until(|_, byte| {
                    !byte.is_ascii_alphanumeric() && byte.to_string() != "_"
                });

                let token_str = std::str::from_utf8(&token_bytes[..]).unwrap();
                
                if keywords.contains(&token_str) {
                    self.tokens.push(TokenCategory::Keyword(token_str.to_string()));
                } else {
                    self.tokens.push(TokenCategory::Ident(token_str.to_string()));
                }
            }

            match byte {
                b'-' => {
                    self.tokens.push(TokenCategory::Minus);
                },
                b'+' => {
                    self.tokens.push(TokenCategory::Plus);
                },
                b'*' => {
                    self.tokens.push(TokenCategory::Asterisk);
                },
                b'/' => {
                    self.tokens.push(TokenCategory::Slash);
                },
                b'{' => {
                    self.tokens.push(TokenCategory::OpenCurlyBracket);
                },
                b'}' => {
                    self.tokens.push(TokenCategory::CloseCurlyBracket);
                },
                b'(' => {
                    self.tokens.push(TokenCategory::OpenParen);
                },
                b')' => {
                    self.tokens.push(TokenCategory::CloseParen);
                },
                b'=' => {
                    self.tokens.push(TokenCategory::Equal);
                },

                _ => {}

            };

            if byte.is_ascii_whitespace() {
                self.position += 1;
                continue;
            }


            self.position += 1;
        }
    }



    fn read_until<F>(&mut self, mut pred: F) -> Vec<u8> where
    F: FnMut(Option<u8>, &u8) -> bool {
        let remaining_bytes = self.script.get(self.position..).unwrap();
        let mut collected: Vec<u8> = vec![];
        let mut prev_byte: Option<u8> = None;
        for byte in remaining_bytes {
            if pred(prev_byte, byte) {
                break;
            } else {
                collected.push(byte.to_owned());
                prev_byte = Some(byte.to_owned());
                self.position += 1;
            }
        }

        return collected;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tokenize_keyword() {
        let script = r#"
            local test = '\'test\''
        "#;
       let mut tokenizer = Tokenizer::new(script.to_string()); 
       tokenizer.tokenize();
       println!("TOKENS {:?}", tokenizer.tokens);
       assert!(1 == 1);
    }
}
