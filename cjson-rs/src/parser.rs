//! RFC 8259 parser.
//!
//! Consumes a UTF-8 byte slice and produces a `Value` tree. Enforces a
//! configurable maximum nesting depth to prevent stack overflow on
//! adversarial input (matches cJSON's `CJSON_NESTING_LIMIT` default of
//! 1000).

use crate::error::{Error, Position};
use crate::token::{PositionedToken, Token};
use crate::tokeniser::Tokeniser;
use crate::value::Value;

// The parser streams tokens straight from the tokeniser with a single-token
// lookahead, rather than materialising the whole token stream into a `Vec`.
// For a large document that removes a full-size intermediate allocation (and
// its reallocations) on the parse hot path.

/// Parser configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParserOptions {
    /// Maximum depth of nested arrays and objects before parsing is
    /// rejected. Default 1000, matching cJSON's `CJSON_NESTING_LIMIT`.
    pub nesting_limit: usize,
}

impl Default for ParserOptions {
    fn default() -> Self {
        Self {
            nesting_limit: 1000,
        }
    }
}

/// Parse a complete JSON document with default options.
pub fn parse(input: &[u8]) -> Result<Value, Error> {
    parse_with_options(input, &ParserOptions::default())
}

/// Parse a complete JSON document with caller-supplied options.
pub fn parse_with_options(input: &[u8], opts: &ParserOptions) -> Result<Value, Error> {
    let mut parser = Parser::new(input, opts.nesting_limit);
    let value = parser.parse_value()?;
    // Require the whole input to be a single value: anything after it (a
    // further token, or trailing bytes that don't even tokenise) is an error.
    match parser.peek()? {
        None => Ok(value),
        Some(_) => Err(Error::TrailingData {
            pos: parser.peek_pos(),
        }),
    }
}

/// Parse a single JSON value from the prefix of `input` and return both the
/// value and the byte offset immediately after its last consumed byte (i.e.
/// not counting any trailing whitespace or unparsed bytes). Trailing
/// whitespace, garbage, or extra values are allowed and ignored.
///
/// Backs cJSON's `cJSON_ParseWithOpts(..., require_null_terminated = false)`
/// — see `cjson-rs-ffi/src/lib.rs::cJSON_ParseWithOpts`.
pub fn parse_prefix(input: &[u8]) -> Result<(Value, usize), Error> {
    let mut parser = Parser::new(input, ParserOptions::default().nesting_limit);
    let value = parser.parse_value()?;
    // Stop at the end of the first value; trailing bytes (valid or not) are
    // ignored, matching cJSON's require_null_terminated = false mode.
    Ok((value, parser.last_end.offset))
}

struct Parser<'a> {
    tok: Tokeniser<'a>,
    /// One-token lookahead, lexed on demand. `None` means "not yet filled".
    peeked: Option<PositionedToken>,
    /// Set once the tokeniser has reported end of input.
    at_end: bool,
    /// Position and end of the most recently consumed token — used for EOF
    /// error reporting and for `parse_prefix`'s end offset.
    last_pos: Position,
    last_end: Position,
    depth: usize,
    limit: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a [u8], limit: usize) -> Self {
        Parser {
            tok: Tokeniser::new(input),
            peeked: None,
            at_end: false,
            last_pos: Position::START,
            last_end: Position::START,
            depth: 0,
            limit,
        }
    }

    /// Ensure the lookahead slot is filled (or end-of-input reached).
    /// Surfaces a tokenise error at the point the bad token is read.
    fn fill(&mut self) -> Result<(), Error> {
        if self.peeked.is_none() && !self.at_end {
            match self.tok.next_positioned() {
                None => self.at_end = true,
                Some(Ok(t)) => self.peeked = Some(t),
                Some(Err(e)) => return Err(e),
            }
        }
        Ok(())
    }

    fn peek(&mut self) -> Result<Option<&Token>, Error> {
        self.fill()?;
        Ok(self.peeked.as_ref().map(|pt| &pt.token))
    }

    /// Position to blame in an error message: the lookahead token's start if
    /// there is one, otherwise just past the last consumed token. Never
    /// propagates a tokenise error (it's only building a diagnostic).
    fn peek_pos(&mut self) -> Position {
        let _ = self.fill();
        self.peeked
            .as_ref()
            .map(|pt| pt.pos)
            .unwrap_or(self.last_pos)
    }

    fn advance(&mut self) {
        if let Some(pt) = self.peeked.take() {
            self.last_pos = pt.pos;
            self.last_end = pt.end;
        }
    }

    fn enter(&mut self, pos: Position) -> Result<(), Error> {
        if self.depth >= self.limit {
            return Err(Error::NestingLimitExceeded { pos });
        }
        self.depth += 1;
        Ok(())
    }

    fn leave(&mut self) {
        self.depth -= 1;
    }

    /// RFC 8259 §3: value = false / null / true / object / array / number / string
    fn parse_value(&mut self) -> Result<Value, Error> {
        let pos = self.peek_pos();
        let tok = self.peek()?.ok_or(Error::UnexpectedEof { pos })?;
        match tok {
            Token::Null => {
                self.advance();
                Ok(Value::Null)
            }
            Token::True => {
                self.advance();
                Ok(Value::Bool(true))
            }
            Token::False => {
                self.advance();
                Ok(Value::Bool(false))
            }
            Token::Number(n) => {
                let n = *n;
                self.advance();
                Ok(Value::Number(n))
            }
            Token::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(Value::String(s))
            }
            Token::BeginArray => self.parse_array(),
            Token::BeginObject => self.parse_object(),
            _ => Err(Error::UnexpectedToken { pos }),
        }
    }

    /// RFC 8259 §5: array = begin-array [ value *( value-separator value ) ] end-array
    fn parse_array(&mut self) -> Result<Value, Error> {
        let open_pos = self.peek_pos();
        self.enter(open_pos)?;
        self.advance(); // consume [

        let mut items = Vec::new();

        if matches!(self.peek()?, Some(Token::EndArray)) {
            self.advance();
            self.leave();
            return Ok(Value::Array(items));
        }

        loop {
            items.push(self.parse_value()?);
            match self.peek()? {
                Some(Token::ValueSeparator) => {
                    let comma_pos = self.peek_pos();
                    self.advance();
                    // Reject trailing comma: next token must start a value, not close.
                    if matches!(self.peek()?, Some(Token::EndArray)) {
                        return Err(Error::UnexpectedToken { pos: comma_pos });
                    }
                }
                Some(Token::EndArray) => {
                    self.advance();
                    self.leave();
                    return Ok(Value::Array(items));
                }
                Some(_) => {
                    return Err(Error::UnexpectedToken {
                        pos: self.peek_pos(),
                    });
                }
                None => {
                    return Err(Error::UnexpectedEof {
                        pos: self.peek_pos(),
                    });
                }
            }
        }
    }

    /// RFC 8259 §4: object = begin-object [ member *( value-separator member ) ] end-object
    ///              member = string name-separator value
    fn parse_object(&mut self) -> Result<Value, Error> {
        let open_pos = self.peek_pos();
        self.enter(open_pos)?;
        self.advance(); // consume {

        let mut members = Vec::new();

        if matches!(self.peek()?, Some(Token::EndObject)) {
            self.advance();
            self.leave();
            return Ok(Value::Object(members));
        }

        loop {
            // Expect string key.
            let key_pos = self.peek_pos();
            let key = match self.peek()? {
                Some(Token::String(s)) => {
                    let s = s.clone();
                    self.advance();
                    s
                }
                Some(_) => return Err(Error::UnexpectedToken { pos: key_pos }),
                None => return Err(Error::UnexpectedEof { pos: key_pos }),
            };

            // Expect name-separator (:).
            match self.peek()? {
                Some(Token::NameSeparator) => self.advance(),
                Some(_) => {
                    return Err(Error::UnexpectedToken {
                        pos: self.peek_pos(),
                    })
                }
                None => {
                    return Err(Error::UnexpectedEof {
                        pos: self.peek_pos(),
                    })
                }
            }

            // Parse value.
            let value = self.parse_value()?;
            members.push((key, value));

            // Expect value-separator (,) or end-object (}).
            match self.peek()? {
                Some(Token::ValueSeparator) => {
                    let comma_pos = self.peek_pos();
                    self.advance();
                    if matches!(self.peek()?, Some(Token::EndObject)) {
                        return Err(Error::UnexpectedToken { pos: comma_pos });
                    }
                }
                Some(Token::EndObject) => {
                    self.advance();
                    self.leave();
                    return Ok(Value::Object(members));
                }
                Some(_) => {
                    return Err(Error::UnexpectedToken {
                        pos: self.peek_pos(),
                    })
                }
                None => {
                    return Err(Error::UnexpectedEof {
                        pos: self.peek_pos(),
                    })
                }
            }
        }
    }
}
