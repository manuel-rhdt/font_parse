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

use std::borrow::Cow;

use nom::{be_i16, be_i32, be_u16, be_u24, be_u32, be_u8};
use serde::de;
use serde::{Deserialize};

use error::{ParserError, DeserializerError};
use super::SfntTable;

const CFF_STANDARD_STRINGS: [&'static str; 391] = [
    ".notdef", "space", "exclam", "quotedbl", "numbersign", "dollar", "percent", "ampersand", "quoteright",
    "parenleft", "parenright", "asterisk", "plus", "comma", "hyphen", "period", "slash", "zero", "one", "two",
    "three", "four", "five", "six", "seven", "eight", "nine", "colon", "semicolon", "less", "equal", "greater",
    "question", "at", "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z", "bracketleft", "backslash", "bracketright", "asciicircum", "underscore",
    "quoteleft", "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s", "t",
    "u", "v", "w", "x", "y", "z", "braceleft", "bar", "braceright", "asciitilde", "exclamdown", "cent", "sterling",
    "fraction", "yen", "florin", "section", "currency", "quotesingle", "quotedblleft", "guillemotleft",
    "guilsinglleft", "guilsinglright", "fi", "fl", "endash", "dagger", "daggerdbl", "periodcentered", "paragraph",
    "bullet", "quotesinglbase", "quotedblbase", "quotedblright", "guillemotright", "ellipsis", "perthousand",
    "questiondown", "grave", "acute", "circumflex", "tilde", "macron", "breve", "dotaccent", "dieresis", "ring",
    "cedilla", "hungarumlaut", "ogonek", "caron", "emdash", "AE", "ordfeminine", "Lslash", "Oslash", "OE",
    "ordmasculine", "ae", "dotlessi", "lslash", "oslash", "oe", "germandbls", "onesuperior", "logicalnot", "mu",
    "trademark", "Eth", "onehalf", "plusminus", "Thorn", "onequarter", "divide", "brokenbar", "degree", "thorn",
    "threequarters", "twosuperior", "registered", "minus", "eth", "multiply", "threesuperior", "copyright",
    "Aacute", "Acircumflex", "Adieresis", "Agrave", "Aring", "Atilde", "Ccedilla", "Eacute", "Ecircumflex",
    "Edieresis", "Egrave", "Iacute", "Icircumflex", "Idieresis", "Igrave", "Ntilde", "Oacute", "Ocircumflex",
    "Odieresis", "Ograve", "Otilde", "Scaron", "Uacute", "Ucircumflex", "Udieresis", "Ugrave", "Yacute",
    "Ydieresis", "Zcaron", "aacute", "acircumflex", "adieresis", "agrave", "aring", "atilde", "ccedilla", "eacute",
    "ecircumflex", "edieresis", "egrave", "iacute", "icircumflex", "idieresis", "igrave", "ntilde", "oacute",
    "ocircumflex", "odieresis", "ograve", "otilde", "scaron", "uacute", "ucircumflex", "udieresis", "ugrave",
    "yacute", "ydieresis", "zcaron", "exclamsmall", "Hungarumlautsmall", "dollaroldstyle", "dollarsuperior",
    "ampersandsmall", "Acutesmall", "parenleftsuperior", "parenrightsuperior", "266 ff", "onedotenleader",
    "zerooldstyle", "oneoldstyle", "twooldstyle", "threeoldstyle", "fouroldstyle", "fiveoldstyle", "sixoldstyle",
    "sevenoldstyle", "eightoldstyle", "nineoldstyle", "commasuperior", "threequartersemdash", "periodsuperior",
    "questionsmall", "asuperior", "bsuperior", "centsuperior", "dsuperior", "esuperior", "isuperior", "lsuperior",
    "msuperior", "nsuperior", "osuperior", "rsuperior", "ssuperior", "tsuperior", "ff", "ffi", "ffl",
    "parenleftinferior", "parenrightinferior", "Circumflexsmall", "hyphensuperior", "Gravesmall", "Asmall",
    "Bsmall", "Csmall", "Dsmall", "Esmall", "Fsmall", "Gsmall", "Hsmall", "Ismall", "Jsmall", "Ksmall", "Lsmall",
    "Msmall", "Nsmall", "Osmall", "Psmall", "Qsmall", "Rsmall", "Ssmall", "Tsmall", "Usmall", "Vsmall", "Wsmall",
    "Xsmall", "Ysmall", "Zsmall", "colonmonetary", "onefitted", "rupiah", "Tildesmall", "exclamdownsmall",
    "centoldstyle", "Lslashsmall", "Scaronsmall", "Zcaronsmall", "Dieresissmall", "Brevesmall", "Caronsmall",
    "Dotaccentsmall", "Macronsmall", "figuredash", "hypheninferior", "Ogoneksmall", "Ringsmall", "Cedillasmall",
    "questiondownsmall", "oneeighth", "threeeighths", "fiveeighths", "seveneighths", "onethird", "twothirds",
    "zerosuperior", "foursuperior", "fivesuperior", "sixsuperior", "sevensuperior", "eightsuperior", "ninesuperior",
    "zeroinferior", "oneinferior", "twoinferior", "threeinferior", "fourinferior", "fiveinferior", "sixinferior",
    "seveninferior", "eightinferior", "nineinferior", "centinferior", "dollarinferior", "periodinferior",
    "commainferior", "Agravesmall", "Aacutesmall", "Acircumflexsmall", "Atildesmall", "Adieresissmall",
    "Aringsmall", "AEsmall", "Ccedillasmall", "Egravesmall", "Eacutesmall", "Ecircumflexsmall", "Edieresissmall",
    "Igravesmall", "Iacutesmall", "Icircumflexsmall", "Idieresissmall", "Ethsmall", "Ntildesmall", "Ogravesmall",
    "Oacutesmall", "Ocircumflexsmall", "Otildesmall", "Odieresissmall", "OEsmall", "Oslashsmall", "Ugravesmall",
    "Uacutesmall", "Ucircumflexsmall", "Udieresissmall", "Yacutesmall", "Thornsmall", "Ydieresissmall", "001.000",
    "001.001", "001.002", "001.003", "Black", "Bold", "Book", "Light", "Medium", "Regular", "Roman", "Semibold"];


#[derive(Debug, Clone)]
pub struct Cff<'a> {
    pub header: Header,
    pub name: &'a str,
    pub top_dict_data: TopDictData<'a>,
    pub(crate) private_dict_data: PrivateDictData,
    pub(crate) char_strings: Index<'a>,
    pub(crate) global_subrs: Index<'a>,
    pub(crate) local_subrs: Index<'a>
}

impl<'a> Cff<'a> {
    fn from_cffdata(cffdata: CffData<'a>, data: &'a [u8]) -> Result<Self, ParserError> {
        let name = cffdata.name_index.get(0).ok_or(ParserError::from_string("Expected name index.".to_string()))?;
        let name = ::std::str::from_utf8(name).map_err(|err| ParserError::from_err(err))?;

        let top_dict_index = cffdata.top_dict_index;
        let string_index = cffdata.string_index;

        let top_dict_data = top_dict_index.get(0).ok_or(ParserError::from_string("Expected top dict index.".to_string()))?;

        let mut dictionary_deserializer = DictionaryDeserializer::new(top_dict_data, &string_index);

        let top_dict_data= TopDictData::deserialize(&mut dictionary_deserializer)?;

        let char_strings = data.get(top_dict_data.char_strings..).ok_or(ParserError::from_string(format!("no char strings")))?;
        let char_strings = parse_index(char_strings)?.1;

        let p_data_start = top_dict_data.private.1;
        let p_data_end = top_dict_data.private.0 + p_data_start;
        let private_dict_data = data.get(p_data_start .. p_data_end).ok_or(ParserError::from_string(format!("no private dict")))?;
        let mut dictionary_deserializer = DictionaryDeserializer::new(private_dict_data, &string_index);
        let private_dict_data = PrivateDictData::deserialize(&mut dictionary_deserializer)?;

        let local_subrs = if private_dict_data.subrs != 0 {
            let subrs_start = p_data_start + private_dict_data.subrs;
            if let Some(subrs_data) = data.get(subrs_start..) {
                parse_index(subrs_data)?.1
            } else {
                Index::empty()
            }
        } else {
            Index::empty()
        };

        Ok(Cff {
            header: cffdata.header,
            name,
            top_dict_data,
            char_strings,
            private_dict_data,
            global_subrs: cffdata.global_subr_index,
            local_subrs
        })
    }

    pub fn num_glyphs(&self) -> u32 {
        self.char_strings.len() as u32
    }

    pub fn charstring(&self, glyph_index: u32) -> Option<&[u8]> {
        self.char_strings.get(glyph_index as usize)
    }
}

impl<'a> SfntTable<'a> for Cff<'a> {
    const TAG: &'static str = "CFF ";
    type Context = ();
    type Err = ParserError;

    fn from_data(data: &'a [u8], _: ()) -> Result<Self, Self::Err> {
        parse_cff_table(data)
            .map_err(|err| err.into())
            .and_then(|result| Cff::from_cffdata(result.1, data))
    }
}

#[derive(Debug, Clone)]
struct CffData<'a> {
    header: Header,
    name_index: Index<'a>,
    top_dict_index: Index<'a>,
    string_index: Index<'a>,
    global_subr_index: Index<'a>,
}

named!(parse_cff_table<&[u8], CffData>,
    do_parse!(
        header: parse_header >>
        name_index: parse_index >>
        top_dict_index: parse_index >>
        string_index: parse_index >>
        global_subr_index: parse_index >>
        (CffData { 
            header, 
            name_index,
            top_dict_index,
            string_index,
            global_subr_index
        })
    )
);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Header {
    pub major: u8,
    pub minor: u8,
    pub header_size: u8,
    pub offset_size: u8,
}

named!(parse_header<&[u8], Header>,
    do_parse!(
        major: be_u8 >>
        minor: be_u8 >>
        header_size: be_u8 >>
        offset_size: be_u8 >>
        take!(header_size.saturating_sub(4)) >>
        (Header {
            major,
            minor,
            header_size,
            offset_size
        })
    )
);

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Index<'a> {
    pub offsets: Vec<usize>,
    data: &'a [u8],
}

impl<'a> Index<'a> {
    pub fn empty() -> Index<'static> {
        Index {
            offsets: vec![],
            data: &[],
        }
    }

    pub fn get(&self, index: usize) -> Option<&'a [u8]> {
        let start = self.offsets.get(index)?.saturating_sub(1);
        let end = self.offsets.get(index + 1)?.saturating_sub(1);
        self.data.get(start..end)
    }

    pub fn len(&self) -> usize {
        self.offsets.len().saturating_sub(1)
    }
}

named_args!(parse_offset(offSize: u8)<&[u8], usize>, 
    switch!(
        value!(offSize), //< offset size
        1 => map!(be_u8, |x| x as usize) |
        2 => map!(be_u16, |x| x as usize) |
        3 => map!(be_u24, |x| x as usize) |
        4 => map!(be_u32, |x| x as usize)
    )
);

named_args!(parse_offset_list(num_offsets: usize)<&[u8], Vec<usize>>,
    map!(
        cond!(
            num_offsets > 0,
            do_parse!(
                offSize: be_u8 >>
                offsets: count!(apply!(parse_offset, offSize), num_offsets + 1) >>
                (offsets)
            )
        ),
        |vec| vec.unwrap_or_default()
    )
);

named!(parse_index<&[u8], Index>,
    do_parse!(
        num_offsets: map!(be_u16, |x| x as usize) >>
        offsets: apply!(parse_offset_list, num_offsets) >>
        data: take!(offsets.last().map(|&offset| offset.saturating_sub(1)).unwrap_or(0)) >>
        (Index { offsets, data })
    )
);

#[derive(Default, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "PascalCase")]
#[serde(default)]
pub struct TopDictData<'a> {
    #[serde(rename = "version")]
    pub version: u32,
    pub notice: &'a str,
    pub copyright: &'a str,
    pub full_name: &'a str,
    pub family_name: &'a str,
    pub weight: &'a str,
    char_strings: usize,
    // size and offset of private dict
    private: (usize, usize)
}

#[derive(Default, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "PascalCase")]
#[serde(default)]
pub(crate) struct PrivateDictData {
    pub(crate) subrs: usize,
    #[serde(rename = "defaultWidthX")]
    pub(crate) default_width_x: i32,
    #[serde(rename = "nominalWidthX")]
    pub(crate) nominal_width_x: i32,
}

#[derive(Debug, PartialEq)]
enum DictionaryParserState<'de> {
    Initial,
    ParseKey(Operator, &'de [u8]),
    ParseValue(Value, &'de [u8]),
    EndArray,
}

#[derive(Debug)]
struct DictionaryDeserializer<'a, 'de: 'a> {
    data: &'de [u8],
    string_index: &'a Index<'de>,
    state: DictionaryParserState<'de>,
}

impl<'a, 'de> DictionaryDeserializer<'a, 'de> {
    fn new(data: &'de [u8], string_index: &'a Index<'de>) -> Self {
        DictionaryDeserializer {
            data,
            string_index,
            state: DictionaryParserState::Initial
        }
    }

    fn get_cff_string(&self, index: usize) -> Cow<'de ,str> {
        if index <= 390 {
            Cow::Borrowed(CFF_STANDARD_STRINGS[index])
        } else {
            let bytes = self.string_index.get(index - 391).unwrap_or_default();
            String::from_utf8_lossy(bytes)
        }
    }
}

impl<'de, 'a, 'b> de::Deserializer<'de> for &'a mut DictionaryDeserializer<'b, 'de> {
    type Error = DeserializerError;

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char
        string bytes byte_buf option unit unit_struct newtype_struct
        tuple_struct map struct enum ignored_any
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.state {
            DictionaryParserState::Initial => {
                visitor.visit_map(self)
            },
            DictionaryParserState::ParseKey(operator, _) => {
                match operator {
                    Operator::Short(i) => visitor.visit_bytes(&[i]),
                    Operator::Long(i) => visitor.visit_bytes(&[12, i]),
                }
            }
            DictionaryParserState::ParseValue(value, _) => {
                match value {
                    Value::Integer(i) => visitor.visit_i32(i),
                    Value::Float(f) => visitor.visit_f32(f),
                }
            }
            _ => unreachable!()
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.state {
            DictionaryParserState::ParseValue(Value::Integer(i), _) => {
                let string = self.get_cff_string(i as usize);
                match string {
                    Cow::Borrowed(string) => visitor.visit_borrowed_str(string),
                    Cow::Owned(string) => visitor.visit_string(string),
                }
            },
            DictionaryParserState::ParseKey(_, _) => {
                self.deserialize_identifier(visitor)
            },
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.state {
            DictionaryParserState::ParseKey(operator, _) => {
                let string = operator.map_str();
                if string == "" {
                    self.deserialize_any(visitor)
                } else {
                    visitor.visit_str(string)
                }
            }
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.state {
            DictionaryParserState::ParseValue(_,_) => {
                visitor.visit_seq(self)
            },
            _ => self.deserialize_any(visitor)
        }
    }

    fn deserialize_tuple<V>(self, _: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.state {
            DictionaryParserState::ParseValue(_,_) => {
                visitor.visit_seq(self)
            },
            _ => self.deserialize_any(visitor)
        }
    }
}

impl<'de: 'a, 'a, 'b> de::MapAccess<'de> for &'a mut DictionaryDeserializer<'b, 'de> {
    type Error = DeserializerError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error> where K: de::DeserializeSeed<'de> {
        if self.data.len() == 0 {
            return Ok(None)
        }
        let parsing_result = recognize!(self.data, many0!(map!(parse_operand, |_| ())))?;
        let (operator_bytes, operands) = parsing_result;

        let (rem_bytes, operator) = parse_operator(operator_bytes)?;
        
        self.state = DictionaryParserState::ParseKey(operator, operands);
        self.data = rem_bytes;
        seed.deserialize(&mut **self).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error> where V: de::DeserializeSeed<'de> {
        match self.state {
            DictionaryParserState::ParseKey(_, operands) => {
                let (rem_bytes, value) = parse_operand(operands)?;
                self.state = DictionaryParserState::ParseValue(value, rem_bytes);
                seed.deserialize(&mut **self)
            },
            _ => panic!("Internal Inconsistency")
        }
    }
}

impl<'de, 'a, 'b> de::SeqAccess<'de> for &'a mut DictionaryDeserializer<'b, 'de> {
    type Error = DeserializerError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        if self.state == DictionaryParserState::EndArray {
            self.state = DictionaryParserState::Initial;
            return Ok(None)
        }

        let result = seed.deserialize(&mut **self).map(Some);

        let bytes = match self.state {
            DictionaryParserState::ParseValue(_, bytes) => bytes,
            _ => panic!("Internal Inconsistency")
        };
        if bytes.len() == 0 {
            self.state = DictionaryParserState::EndArray;
            return result
        }

        let (rem_bytes, value) = parse_operand(bytes)?;
        self.state = DictionaryParserState::ParseValue(value, rem_bytes);

        result
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
enum Operator {
    Short(u8),
    Long(u8),
}

impl Operator {
    fn map_str(&self) -> &'static str {
        // TODO: This list is not exhaustive
        match self {
            Operator::Short(0) => "version",
            Operator::Short(1) => "Notice",
            Operator::Long(0) => "Copyright",
            Operator::Short(2) => "FullName",
            Operator::Short(3) => "FamilyName",
            Operator::Short(4) => "Weight",
            Operator::Long(1) => "isFixedPitch",
            Operator::Long(2) => "ItalicAngle",
            Operator::Long(3) => "UnderlinePosition",
            Operator::Long(4) => "UnderlineThickness",
            Operator::Long(5) => "PaintType",
            Operator::Long(6) => "CharstringType",
            Operator::Long(7) => "FontMatrix",
            Operator::Short(13) => "UniqueID",
            Operator::Short(5) => "FontBBox",
            Operator::Long(8) => "StrokeWidth",
            Operator::Short(17) => "CharStrings",
            Operator::Short(18) => "Private",
            Operator::Short(19) => "Subrs",
            Operator::Short(20) => "defaultWidthX",
            Operator::Short(21) => "nominalWidthX",
            _ => "",
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum Value {
    Integer(i32),
    Float(f32),
}

named!(parse_operator<&[u8], Operator>,
    switch!(be_u8,
        12 => map!(be_u8, |x| Operator::Long(x)) |
        x @ 0...21 => value!(Operator::Short(x))
    )
);

named!(parse_operand<&[u8], Value>,
    switch!(
        be_u8,
        28 => map!(be_i16, |x| Value::Integer(x as i32)) |
        29 => map!(be_i32, |x| Value::Integer(x as i32)) |
        30 => map!(parse_float, |x| Value::Float(x)) |
        x @ 32...246 => value!(Value::Integer(x as i32 - 139)) |
        x @ 247...250 => map!(be_u8, |y| Value::Integer((x as i32 - 247) * 256 + y as i32 + 108)) |
        x @ 251...254 => map!(be_u8, |y| Value::Integer(-(x as i32 - 251) * 256 - y as i32 - 108))
    )
);

const LOOKUP_TABLE: [&'static [u8]; 16] = [
    b"0", b"1", b"2", b"3", b"4", b"5", b"6", b"7", b"8", b"9", b".", b"E", b"E-", b"", b"-", b"f",
];

// TODO: Maybe unnecessary heap allocations happen here...
named!(parse_float<&[u8], f32>,
    map!(bits!(many_till!(
        take_bits!(usize, 4), 
        tag_bits!(u8, 4, 0xf)
    )),
        |(vec, _)| {
            vec
                .into_iter()
                .flat_map(|idx| LOOKUP_TABLE[idx].iter().map(|&b| b as char))
                .collect::<String>()
                .parse()
                .unwrap_or_default()
        }
    )
);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_operator() {
        let data = [0x0a];
        assert_eq!(Operator::Short(0x0a), parse_operator(&data).unwrap().1);

        let data = [0x0c, 0x0a];
        assert_eq!(Operator::Long(0x0a), parse_operator(&data).unwrap().1);
    }

    #[test]
    fn test_parse_integer_operand() {
        let data = [0x8b];
        assert_eq!(Value::Integer(0), parse_operand(&data).unwrap().1);

        let data = [0xef];
        assert_eq!(Value::Integer(100), parse_operand(&data).unwrap().1);

        let data = [0x27];
        assert_eq!(Value::Integer(-100), parse_operand(&data).unwrap().1);

        let data = [0xfa, 0x7c];
        assert_eq!(Value::Integer(1000), parse_operand(&data).unwrap().1);

        let data = [0xfe, 0x7c];
        assert_eq!(Value::Integer(-1000), parse_operand(&data).unwrap().1);

        let data = [0x1c, 0x27, 0x10];
        assert_eq!(Value::Integer(10000), parse_operand(&data).unwrap().1);

        let data = [0x1c, 0xd8, 0xf0];
        assert_eq!(Value::Integer(-10000), parse_operand(&data).unwrap().1);

        let data = [0x1d, 0x00, 0x01, 0x86, 0xa0];
        assert_eq!(Value::Integer(100000), parse_operand(&data).unwrap().1);

        let data = [0x1d, 0xff, 0xfe, 0x79, 0x60];
        assert_eq!(Value::Integer(-100000), parse_operand(&data).unwrap().1);
    }

    #[test]
    fn test_parse_float_operand() {
        let data = [0x1e, 0xe2, 0xa2, 0x5f];
        assert_eq!(Value::Float(-2.25), parse_operand(&data).unwrap().1);

        let data = [0x1e, 0x0a, 0x14, 0x05, 0x41, 0xc3, 0xff];
        assert_eq!(Value::Float(0.140541E-3), parse_operand(&data).unwrap().1);
    }

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct MetaDict {
        #[serde(rename = "Copyright")]
        copyright: [i64; 2],
        version: i64,
    }

    #[test]
    fn test_deserialize_dict() {
        let data = [0x8b, 0x1c, 0xd8, 0xf0, 0x0c, 0x00, 0x1c, 0xd8, 0xf0, 0x00];

        let string_index = Index::default();
        let mut deserializer = DictionaryDeserializer::new(&data, &string_index);

        let result = MetaDict::deserialize(&mut deserializer).unwrap();

        assert_eq!(
            MetaDict {
                copyright: [0, -10000],
                version: -10000
            },
            result
        )
    }

    #[test]
    fn test_parse_offset() {
        let data = [0x12];
        assert_eq!(0x12, parse_offset(&data, 1).unwrap().1);

        let data = [0x12, 0x34];
        assert_eq!(0x1234, parse_offset(&data, 2).unwrap().1);

        let data = [0x12, 0x34, 0x56];
        assert_eq!(0x123456, parse_offset(&data, 3).unwrap().1);

        let data = [0x12, 0x34, 0x56, 0x78];
        assert_eq!(0x12345678, parse_offset(&data, 4).unwrap().1);
    }

    #[test]
    fn test_parse_index() {
        let data = [0x00, 0x00];
        assert_eq!(
            Index {
                offsets: vec![],
                data: &[]
            },
            parse_index(&data).unwrap().1
        );

        let data = [0x00, 0x01, 0x01, 0x01, 0x03, 0x0a, 0x0b];
        assert_eq!(
            Index {
                offsets: vec![0x01, 0x03],
                data: &[0x0a, 0x0b]
            },
            parse_index(&data).unwrap().1
        );
    }
}
