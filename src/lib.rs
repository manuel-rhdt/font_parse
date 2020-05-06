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

#![deny(missing_debug_implementations)]

#[macro_use]
extern crate nom;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate log;
#[macro_use]
extern crate derive_more;

use nom::IResult;
use nom::{be_u16, be_u32, be_u8};

use std::borrow::Cow;

mod cff;
mod error;
mod glyph_accessor;
pub mod tables;
pub(crate) mod ttf_glyph_accessor;

use crate::error::{ErrorKind, ParserError};

use crate::cff::GlyphAccessor as CffGlyphAccessor;
pub use crate::cff::{Glyph as CffGlyph, PathInstruction};
use crate::glyph_accessor::_GlyphAccessor;
pub use crate::glyph_accessor::{Glyph, GlyphAccessor};
use crate::ttf_glyph_accessor::GlyphAccessor as TtfGlyphAccessor;
pub use crate::ttf_glyph_accessor::{Glyph as TtfGlyph, QuadraticPath};

pub type GlyphIndex = u16;

/// A type to represent 4-byte SFNT tags.
///
/// Tables, features, etc. in OpenType and many other font formats use SFNT tags
/// as identifiers. These are 4-bytes long and usually each byte represents an
/// ASCII value. `Tag` provides methods to create such identifiers from
/// individual `chars` or a `str` slice and to get the string representation of
/// a `Tag`.
#[derive(Copy, Clone, Hash, PartialEq, Eq, Ord, PartialOrd)]
pub struct Tag(pub [u8; 4]);

impl Tag {
    /// Create a `Tag` from its four-char textual representation.
    pub fn new(a: char, b: char, c: char, d: char) -> Self {
        Tag([
            (a as u32) as u8,
            (b as u32) as u8,
            (c as u32) as u8,
            (d as u32) as u8,
        ])
    }

    fn tag_to_string(&self) -> Cow<str> {
        String::from_utf8_lossy(&self.0)
    }
}

named!(parse_tag<&[u8],Tag>,
    do_parse!(
        array: count_fixed!(u8, be_u8, 4) >>
        (Tag(array))
    )
);

use std::fmt;
use std::fmt::{Debug, Display, Formatter};
impl Debug for Tag {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let string = self.tag_to_string().to_owned();
        let mut chars = string.chars();
        write!(
            f,
            "Tag({:?}, {:?}, {:?}, {:?})",
            chars.next().unwrap(),
            chars.next().unwrap(),
            chars.next().unwrap(),
            chars.next().unwrap()
        )
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.tag_to_string())
    }
}

#[derive(Debug, Clone)]
pub struct FontRecord {
    pub version: u32,
    pub search_range: u16,
    pub entry_selector: u16,
    pub range_shift: u16,
    pub tables: Vec<TableRecord>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OutlineType {
    TrueType,
    Cff,
    Cff2,
    Svg,
}

/// Types who can give accesss to SFNT Tables.
///
/// This trait can be implemented by types that represent OpenType fonts and are
/// capable of providing access to the raw SFNT tables present in a font.
pub trait OpentypeTableAccess {
    /// Returns a slice with the binary data of the font table whose tag is
    /// `tag`.
    ///
    /// Returns `None` if the font does not contain a table with the
    /// corresponding tag.
    ///
    /// The data returned from this function can not be expected to be sanitized
    /// at all. You must be very careful to not assume any well-formedness of
    /// the raw font table data.
    fn table_data(&self, tag: Tag) -> Option<&[u8]>;

    fn has_table(&self, tag: Tag) -> bool {
        self.table_data(tag).is_some()
    }

    /// Tries to parse a font table into the requested type.
    ///
    /// Examples
    /// --------
    ///
    /// ```
    /// use font_parse::{Font, OpentypeTableAccess, tables};
    ///
    /// let font_data = include_bytes!("../tests/font_files/Inconsolata-Regular.ttf");
    /// let font = Font::from_bytes(font_data, 0).unwrap();
    ///
    /// let loca_table: tables::head::Head = font.parse_table().unwrap();
    /// ```
    ///
    /// Panics
    /// ------
    /// The default implementation only panics if the implementation of `Tbl`
    /// contains an invalid Tag.
    fn parse_table<'b, Tbl>(&'b self) -> Result<Tbl, error::ParserError>
    where
        Tbl: tables::SfntTable<'b, Context = ()>,
    {
        self.parse_table_context(())
    }

    /// Tries to parse a font table into the requested type.
    ///
    /// Panics
    /// ------
    /// The default implementation only panics if the implementation of `Tbl`
    /// contains an invalid Tag.
    fn parse_table_context<'b, Tbl, C>(&'b self, context: C) -> Result<Tbl, error::ParserError>
    where
        Tbl: tables::SfntTable<'b, Context = C>,
    {
        let (_, tag) = parse_tag(Tbl::TAG.as_bytes()).expect("Invalid table tag.");
        let table_data = self
            .table_data(tag)
            .ok_or_else(|| ParserError::expected_table(tag))?;
        Tbl::from_data(table_data, context)
            .map_err(|err| error::ParserError::from_table_parse_err(tag, err))
    }

    // TODO: Needs Testing
    fn outline_type(&self) -> OutlineType {
        if self.has_table(Tag::new('S', 'V', 'G', ' ')) {
            OutlineType::Svg
        } else if self.has_table(Tag::new('C', 'F', 'F', ' ')) {
            OutlineType::Cff
        } else if self.has_table(Tag::new('C', 'F', 'F', '2')) {
            OutlineType::Cff2
        } else {
            OutlineType::TrueType
        }
    }

    /// Returns a `GlyphAccessor` providing access to individual glyphs of the font.
    fn glyphs(&self) -> Result<GlyphAccessor<'_>, ParserError>
    where
        Self: Sized,
    {
        match CffGlyphAccessor::new(self) {
            Err(err) => match err.kind() {
                ErrorKind::TableMissing(_) => {}
                _ => Err(err)?,
            },
            Ok(accessor) => return Ok(_GlyphAccessor::Cff(accessor).into()),
        }

        match TtfGlyphAccessor::new(self) {
            Err(err) => Err(err)?,
            Ok(accessor) => return Ok(_GlyphAccessor::Ttf(accessor).into()),
        }
    }
}

/// A type which reads a font file from bytes and implements `OpentypeTableAccess`.
///
/// It currently supports font files based on SFNT tables (TrueType and OpenType).
#[derive(Debug, Clone)]
pub struct Font<'a> {
    record: FontRecord,
    collection: Option<FontCollection>,
    data: Cow<'a, [u8]>,
}

impl<'a> Font<'a> {
    /// Create a `Font` from a slice of bytes and an index for selecting a font
    /// from an OpenType font collection.
    pub fn from_bytes(bytes: &'a [u8], index: u32) -> Result<Self, ParserError> {
        let (_, font_header) = parse_slice(bytes)?;
        let mut collection = None;
        let record = match font_header {
            FontFile::Single(record) => record,
            FontFile::Collection(c) => {
                let record = c
                    .fonts
                    .get(index as usize)
                    .ok_or_else(|| ParserError::font_not_found(index as usize))?
                    .clone();
                collection = Some(c);
                record
            }
        };
        Ok(Font {
            record,
            collection,
            data: Cow::Borrowed(bytes),
        })
    }
}

impl<'a> OpentypeTableAccess for Font<'a> {
    fn table_data(&self, tag: Tag) -> Option<&[u8]> {
        let index = self
            .record
            .tables
            .binary_search_by_key(&tag, |record| record.tag)
            .ok();
        let record: Option<TableRecord> = index.map(|index| self.record.tables[index]);
        record.map(move |record| {
            &self.data[record.offset as usize..record.offset as usize + record.length as usize]
        })
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct TableRecord {
    pub tag: Tag,
    pub check_sum: u32,
    pub offset: u32,
    pub length: u32,
}

#[derive(Debug, Clone)]
pub struct FontCollection {
    pub major_version: u16,
    pub minor_version: u16,
    pub fonts: Vec<FontRecord>,
    pub dsig_tag: u32,
    pub dsig_length: u32,
    pub dsig_offset: u32,
}

named!(parse_font<&[u8],FontRecord>,
    do_parse!(
        version: be_u32 >>
        num_tables: be_u16 >>
        search_range: be_u16 >>
        entry_selector: be_u16 >>
        range_shift: be_u16 >>
        // tables must be sorted for binary search
        tables: map!(count!(table_record, num_tables as usize), |mut tables| {tables.sort_unstable(); tables}) >>
        (FontRecord {
            version,
            search_range,
            entry_selector,
            range_shift,
            tables
        })
    )
);

named!(table_record<&[u8],TableRecord>,
    do_parse!(
        tag: parse_tag >>
        check_sum: be_u32 >>
        offset: be_u32 >>
        length: be_u32 >>
        (TableRecord { tag, check_sum, offset, length })
    )
);

named!(parse_font_collection<&[u8], FontCollection>,
    do_parse!(
        tag!("ttcf") >>
        major_version: be_u16 >>
        minor_version: be_u16 >>
        fonts: length_count!(verify!(be_u32, |val| val <= 10000), parse_font) >>
        sig: cond!(major_version >= 2, tuple!(be_u32, be_u32, be_u32)) >>
        (FontCollection {
            major_version,
            minor_version,
            fonts,
            dsig_tag: sig.map(|s| s.0).unwrap_or(0),
            dsig_length: sig.map(|s| s.1).unwrap_or(0),
            dsig_offset: sig.map(|s| s.2).unwrap_or(0),
        })
    )
);

fn parse_slice(input: &[u8]) -> IResult<&[u8], FontFile> {
    alt!(
        input,
        map!(parse_font_collection, FontFile::Collection) | map!(parse_font, FontFile::Single)
    )
}

#[derive(Debug)]
pub enum FontFile {
    Single(FontRecord),
    Collection(FontCollection),
}

pub fn parse(data: &[u8]) -> Result<FontFile, nom::Err<&[u8]>> {
    match parse_slice(data) {
        Ok((_, parsed)) => Ok(parsed),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn basic() {
        let data = include_bytes!("../tests/font_files/LinBiolinum_R.otf");
        let font = parse(data).unwrap();
        if let FontFile::Single(font) = font {
            println!("{:?}", font);
        }
    }

    #[test]
    fn font_collection() {
        let data = include_bytes!("../tests/font_files/01font-collection.otf");
        let font = parse(data).unwrap();
        match font {
            FontFile::Single(_) => panic!(),
            FontFile::Collection(collection) => {
                assert_eq!(collection.major_version, 1);
                assert_eq!(collection.minor_version, 0);
                assert_eq!(collection.fonts.len(), 0);
                assert_eq!(collection.dsig_length, 0);
                assert_eq!(collection.dsig_offset, 0);
                assert_eq!(collection.dsig_tag, 0);
            }
        }
    }

    #[test]
    fn tables() {
        let data = include_bytes!("../tests/font_files/Inconsolata-Regular.ttf");
        let font = Font::from_bytes(data, 0).expect("Could not read font.");
        assert!(font.table_data(Tag::new('G', 'D', 'E', 'F')).is_some());
        assert!(font.table_data(Tag::new('l', 'o', 'c', 'o')).is_none());
    }

    #[test]
    fn outline_type() {
        let data = include_bytes!("../tests/font_files/Inconsolata-Regular.ttf");
        let font = Font::from_bytes(data, 0).expect("Could not read font.");
        assert_eq!(font.outline_type(), OutlineType::TrueType);

        let data = include_bytes!("../tests/font_files/LinBiolinum_R.otf");
        let font = Font::from_bytes(data, 0).expect("Could not read font.");
        assert_eq!(font.outline_type(), OutlineType::Cff);
    }
}
