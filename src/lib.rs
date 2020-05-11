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
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::Write;

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
use std::{
    fmt::{Debug, Display, Formatter},
    ops::Deref,
};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontRecord {
    pub version: u32,
    pub search_range: u16,
    pub entry_selector: u16,
    pub range_shift: u16,
    pub tables: BTreeMap<Tag, TableRecord>,
}

impl FontRecord {
    pub fn write_to<W: Write>(&self, mut sink: W) -> std::io::Result<()> {
        sink.write(&self.version.to_be_bytes())?;
        sink.write(&(self.tables.len() as u16).to_be_bytes())?;
        sink.write(&self.search_range.to_be_bytes())?;
        sink.write(&self.entry_selector.to_be_bytes())?;
        sink.write(&self.range_shift.to_be_bytes())?;
        for (_tag, table) in &self.tables {
            table.write_to(&mut sink)?;
        }
        Ok(())
    }
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

pub trait ParseTable {
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
        Tbl: tables::SfntTable<'b, Context = C>;
}

impl<T: OpentypeTableAccess> ParseTable for T {
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
}

#[derive(Debug)]
pub struct FontKitFont<'a> {
    pub inner: &'a font_kit::font::Font,
    table_data: RefCell<BTreeMap<Tag, Box<[u8]>>>,
}

impl<'a> FontKitFont<'a> {
    pub fn new(inner: &'a font_kit::font::Font) -> Self {
        FontKitFont {
            inner,
            table_data: Default::default(),
        }
    }

    // This method may be used safely because the returned reference will never be invalidated
    // while self is valid (i.e. we will not drop any of the boxes in `table_data` using a shared
    // reference to self).
    //
    // We have to ensure that there will never be deletions in table_data though!
    unsafe fn get_data_unsafe(&self, tag: Tag) -> Option<&[u8]> {
        let bla = self.table_data.try_borrow_unguarded().unwrap();
        bla.get(&tag).map(|x| x.deref() as &[u8])
    }
}

impl<'a> OpentypeTableAccess for FontKitFont<'a> {
    fn table_data(&self, tag: Tag) -> Option<&[u8]> {
        if let Some(_data) = self.table_data.borrow().get(&tag) {
            unsafe { self.get_data_unsafe(tag) }
        } else {
            let data = self.inner.load_font_table(u32::from_be_bytes(tag.0))?;
            self.table_data.borrow_mut().insert(tag, data);
            unsafe { self.get_data_unsafe(tag) }
        }
    }
}

fn int_log_base_2(mut val: u16) -> u16 {
    let mut r = 0;
    while {
        val >>= 1;
        val > 0
    } {
        r += 1
    }
    r
}

pub fn write_font(
    font: &dyn OpentypeTableAccess,
    version_tag: Tag,
    tables: &[Tag],
    sink: &mut dyn Write,
) -> std::io::Result<()> {
    use std::convert::TryFrom;
    const PADDING: u32 = std::mem::size_of::<u32>() as u32;

    let num_tables =
        u16::try_from(tables.len()).expect("A font can't contain more than 2^16 - 1 tables");
    // Log2(maximum power of 2 <= numTables)
    let entry_selector = int_log_base_2(num_tables);
    // (Maximum power of 2 <= numTables) x 16
    let search_range = (1u16 << entry_selector) * 16;
    let range_shift = num_tables * 16 - search_range;

    let first_table_offset = 16 + num_tables * 16;

    let mut offset = first_table_offset as u32;
    let mut table_records = BTreeMap::new();

    let mut head_data = vec![];

    let mut font_checksum: u32 = 0;
    for &tag in tables {
        let record = if tag == Tag(*b"head") {
            head_data = font
                .table_data(tag)
                .expect("did not find corresponding table")
                .to_vec();
            // set the checksum adjustment to zero
            (&mut head_data[8..12]).swap_with_slice(&mut [0, 0, 0, 0]);
            TableRecord {
                tag,
                offset,
                length: head_data.len() as u32,
                check_sum: compute_table_checksum(&head_data),
            }
        } else {
            let table_data = font
                .table_data(tag)
                .expect("did not find corresponding table");
            TableRecord {
                tag,
                offset,
                length: table_data.len() as u32,
                check_sum: compute_table_checksum(table_data),
            }
        };

        font_checksum = font_checksum.wrapping_add(record.check_sum);
        table_records.insert(tag, record);
        // add offset, including padding
        offset += record.length + (PADDING - record.length % PADDING) % PADDING;
    }

    let font_record = FontRecord {
        version: u32::from_be_bytes(version_tag.0),
        search_range,
        entry_selector,
        range_shift,
        tables: table_records,
    };
    let mut font_record_bytes = Vec::with_capacity(first_table_offset as usize);
    font_record.write_to(&mut font_record_bytes).unwrap();
    font_checksum = font_checksum.wrapping_add(compute_table_checksum(&font_record_bytes));

    let check_sum_adjustment =
        u32::from_be_bytes([0xB1, 0xB0, 0xAF, 0xBa]).wrapping_sub(font_checksum);
    (&mut head_data[8..12]).swap_with_slice(&mut check_sum_adjustment.to_be_bytes());

    // finally write the font file
    sink.write_all(&font_record_bytes)?;
    // padding
    sink.write_all(&[0; PADDING as usize])?;

    for &tag in tables {
        if tag == Tag(*b"head") {
            sink.write_all(&head_data)?;
        } else {
            sink.write_all(font.table_data(tag).unwrap())?;
        }
        // write padding bytes after every table
        let num_zero_bytes =
            ((PADDING - font_record.tables[&tag].length % PADDING) % PADDING) as usize;
        sink.write_all(&[0u8, 0, 0, 0, 0, 0, 0, 0][..num_zero_bytes])?;
    }

    Ok(())
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

    // pub fn from_tables(tables: &[Tag], record: FontRecord, provider: &dyn OpentypeTableAccess) -> Self {
    //     let num_tables = tables
    //         .iter()
    //         .filter(|&&tag| provider.has_table(tag))
    //         .count();

    //     let mut new_record = FontRecord {
    //         tables: vec![],
    //         ..record
    //     };

    //     todo!()
    // }

    pub fn write_to<W: Write>(&self, mut sink: W) -> std::io::Result<()> {
        let mut offset = 16 + self.record.tables.len() as u32 * 16;
        let mut record = self.record.clone();
        for (_tag, table) in &mut record.tables {
            table.offset = offset;
            offset += table.length;
            // alignment
            offset += (8 - table.length % 8) % 8;
        }
        record.write_to(&mut sink)?;
        // padding
        sink.write_all(&[0, 0, 0, 0])?;

        for table in record.tables.values() {
            sink.write_all(self.table_data(table.tag).unwrap())?;
            // write padding bytes after every table
            let num_zero_bytes = ((8 - table.length % 8) % 8) as usize;
            sink.write_all(&[0u8, 0, 0, 0, 0, 0, 0, 0][..num_zero_bytes])?;
        }

        Ok(())
    }
}

impl<'a> OpentypeTableAccess for Font<'a> {
    fn table_data(&self, tag: Tag) -> Option<&[u8]> {
        let record = self.record.tables.get(&tag);
        record.map(move |record| {
            &self.data[record.offset as usize..record.offset as usize + record.length as usize]
        })
    }
}

fn compute_table_checksum(mut table: &[u8]) -> u32 {
    let mut sum: u32 = 0;
    while table.len() >= 4 {
        let (first, second) = table.split_at(4);
        let first = [first[0], first[1], first[2], first[3]];
        sum = sum.wrapping_add(u32::from_be_bytes(first));
        table = second;
    }
    let mut final_bytes = [0; 4];
    for (index, val) in table.into_iter().enumerate() {
        final_bytes[index] = *val;
    }
    sum.wrapping_add(u32::from_be_bytes(final_bytes))
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct TableRecord {
    pub tag: Tag,
    pub check_sum: u32,
    pub offset: u32,
    pub length: u32,
}

impl TableRecord {
    pub fn write_to<W: Write>(&self, mut sink: W) -> std::io::Result<()> {
        sink.write(&self.tag.0)?;
        sink.write(&self.check_sum.to_be_bytes())?;
        sink.write(&self.offset.to_be_bytes())?;
        sink.write(&self.length.to_be_bytes())?;
        Ok(())
    }
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

fn to_btree_map(vec: Vec<TableRecord>) -> BTreeMap<Tag, TableRecord> {
    vec.into_iter().map(|record| (record.tag, record)).collect()
}

named!(parse_font<&[u8],FontRecord>,
    do_parse!(
        version: be_u32 >>
        num_tables: be_u16 >>
        search_range: be_u16 >>
        entry_selector: be_u16 >>
        range_shift: be_u16 >>
        // tables must be sorted for binary search
        tables: map!(count!(table_record, num_tables as usize), to_btree_map) >>
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

    #[test]
    fn test_write_font() {
        let data = include_bytes!("../tests/font_files/Inconsolata-Regular.ttf");
        let font = Font::from_bytes(data, 0).expect("Could not read font.");

        let mut data2 = vec![];
        font.write_to(&mut data2).unwrap();

        let font2 = Font::from_bytes(&data2, 0).unwrap();

        assert_eq!(
            font.table_data(Tag::new('G', 'D', 'E', 'F')),
            font2.table_data(Tag::new('G', 'D', 'E', 'F'))
        );
        assert_eq!(
            font.table_data(Tag::new('g', 'l', 'y', 'f')),
            font2.table_data(Tag::new('g', 'l', 'y', 'f'))
        );
    }

    #[test]
    fn test_write_font2() {
        let data = include_bytes!("../tests/font_files/Inconsolata-Regular.ttf");
        let font = Font::from_bytes(data, 0).expect("Could not read font.");

        let mut data2 = vec![];
        write_font(
            &font,
            Tag([0, 1, 0, 0]),
            &[Tag(*b"GDEF"), Tag(*b"glyf"), Tag(*b"head")],
            &mut data2,
        )
        .unwrap();

        assert!(data2.len() < data.len());

        let font2 = Font::from_bytes(&data2, 0).unwrap();

        assert_eq!(
            font.table_data(Tag::new('G', 'D', 'E', 'F')),
            font2.table_data(Tag::new('G', 'D', 'E', 'F'))
        );
        assert_eq!(
            font.table_data(Tag::new('g', 'l', 'y', 'f')),
            font2.table_data(Tag::new('g', 'l', 'y', 'f'))
        );
        assert!(font2.table_data(Tag(*b"hhea")).is_none())
    }

    #[test]
    fn checksum() {
        let data = include_bytes!("../tests/font_files/Inconsolata-Regular.ttf");
        let font = Font::from_bytes(data, 0).expect("Could not read font.");

        for (&tag, &record) in &font.record.tables {
            if &tag.0 == b"head" {
                // for head the checksum is computed differently
                continue;
            }
            let table_data = font.table_data(tag).unwrap();
            assert_eq!(
                compute_table_checksum(table_data),
                record.check_sum,
                "Checksum mismatch for {}",
                tag
            );
        }
    }
}
