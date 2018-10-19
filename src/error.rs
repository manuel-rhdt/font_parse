//    Copyright 2018 Manuel Reinhardt
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
//    You may obtain a copy of the License at
//
//        http://www.apache.org/licenses/LICENSE-2.0
//
//    Unless required by applicable law or agreed to in writing, software
//    distributed under the License is distributed on an "AS IS" BASIS,
//    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//    See the License for the specific language governing permissions and
//    limitations under the License.

use nom;
use serde::de;

use std;
use std::convert::From;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use Tag;

#[derive(Debug, Display)]
pub struct ParserError {
    kind: Box<ErrorKind>,
}

impl ParserError {
    pub fn new(kind: ErrorKind) -> Self {
        ParserError {
            kind: Box::new(kind),
        }
    }

    pub fn from_string(s: String) -> Self {
        ParserError::new(ErrorKind::Other(s))
    }

    pub fn from_err(e: impl 'static + Error) -> Self {
        ParserError::new(ErrorKind::ForeignError(Box::new(e)))
    }

    pub(crate) fn from_table_parse_err(tag: Tag, err: impl 'static + Error) -> Self {
        let foreign_err = ParserError::from_err(err);
        ParserError::new(ErrorKind::TableParse(tag, Some(foreign_err)))
    }

    pub fn expected_table(tag: Tag) -> Self {
        ParserError::new(ErrorKind::TableMissing(tag))
    }

    pub fn font_not_found(index: usize) -> Self {
        ParserError::new(ErrorKind::FontNotFound(index))
    }

    pub fn glyph_parse(index: u32, error: ParserError) -> Self {
        ParserError::new(ErrorKind::GlyphParse {
            index,
            cause: error,
        })
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

impl<'a> From<nom::Err<&'a [u8]>> for ParserError {
    fn from(nom_err: nom::Err<&'a [u8]>) -> ParserError {
        let error_context = match nom_err {
            nom::Err::Incomplete(_) => return ParserError::new(ErrorKind::UnexpectedEndOfData),
            nom::Err::Error(context) => context,
            nom::Err::Failure(context) => context,
        };

        let v: Vec<(&'a [u8], nom::ErrorKind)> = nom::error_to_list(&error_context);
        for (_, kind) in &v {
            match kind {
                nom::ErrorKind::Custom(0) => return ParserError::from_string(format!("Problem 0")),
                _ => {}
            }
        }

        ParserError::from_string(format!("{:?}", v))
    }
}

impl<'a> From<nom::Err<nom::types::CompleteByteSlice<'a>>> for ParserError {
    fn from(nom_err: nom::Err<nom::types::CompleteByteSlice<'a>>) -> ParserError {
        let error_context = match nom_err {
            nom::Err::Incomplete(_) => return ParserError::new(ErrorKind::UnexpectedEndOfData),
            nom::Err::Error(context) => context,
            nom::Err::Failure(context) => context,
        };

        let v: Vec<(nom::types::CompleteByteSlice<'a>, nom::ErrorKind)> =
            nom::error_to_list(&error_context);
        for (_, kind) in &v {
            match kind {
                nom::ErrorKind::Custom(0) => return ParserError::from_string(format!("Problem 0")),
                _ => {}
            }
        }

        ParserError::from_string(format!("{:?}", v))
    }
}

impl From<DeserializerError> for ParserError {
    fn from(err: DeserializerError) -> ParserError {
        ParserError::new(ErrorKind::CffDictionaryDeserialize(err))
    }
}

impl Error for ParserError {
    fn cause(&self) -> Option<&Error> {
        match *self.kind {
            ErrorKind::TableParse(_, Some(ref cause)) => Some(cause),
            ErrorKind::CffDictionaryDeserialize(ref err) => Some(err),
            ErrorKind::ForeignError(ref err) => Some(err.as_ref()),
            ErrorKind::GlyphParse { ref cause, .. } => Some(cause),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    GlyphParse { index: u32, cause: ParserError },
    UnexpectedEndOfData,
    FontNotFound(usize),
    TableMissing(Tag),
    TableParse(Tag, Option<ParserError>),
    CffDictionaryDeserialize(DeserializerError),
    Other(String),
    ForeignError(Box<dyn Error>),
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            ErrorKind::Other(s) => f.write_str(&s),
            ErrorKind::TableParse(tag, _) => write!(f, "{} table could not be parsed.", tag),
            ErrorKind::GlyphParse { index, .. } => {
                write!(f, "Glyph at index {} could not be parsed.", index)
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Display)]
pub struct DeserializerError(ParserError);

impl std::error::Error for DeserializerError {}

impl de::Error for DeserializerError {
    fn custom<T: Display>(msg: T) -> Self {
        DeserializerError(ParserError::from_string(msg.to_string()))
    }
}

impl<'a> From<nom::Err<&'a [u8]>> for DeserializerError {
    fn from(nom_err: nom::Err<&'a [u8]>) -> DeserializerError {
        DeserializerError(nom_err.into())
    }
}
