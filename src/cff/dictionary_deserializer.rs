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

use cff;
use error::DeserializerError;

use nom::*;
use serde::de;

use std::borrow::Cow;

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

#[derive(Debug, PartialEq)]
enum DictionaryParserState<'de> {
    Initial,
    ParseKey(Operator, &'de [u8]),
    ParseValue(Value, &'de [u8]),
    EndArray,
}

#[derive(Debug)]
pub(crate) struct DictionaryDeserializer<'a, 'de: 'a> {
    data: &'de [u8],
    string_index: &'a cff::Index<'de>,
    state: DictionaryParserState<'de>,
}

impl<'a, 'de> DictionaryDeserializer<'a, 'de> {
    pub(crate) fn new(data: &'de [u8], string_index: &'a cff::Index<'de>) -> Self {
        DictionaryDeserializer {
            data,
            string_index,
            state: DictionaryParserState::Initial,
        }
    }

    pub(crate) fn get_cff_string(&self, index: usize) -> Cow<'de, str> {
        if index <= 390 {
            Cow::Borrowed(super::STANDARD_STRINGS[index])
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
            DictionaryParserState::Initial => visitor.visit_map(self),
            DictionaryParserState::ParseKey(operator, _) => match operator {
                Operator::Short(i) => visitor.visit_bytes(&[i]),
                Operator::Long(i) => visitor.visit_bytes(&[12, i]),
            },
            DictionaryParserState::ParseValue(value, _) => match value {
                Value::Integer(i) => visitor.visit_i32(i),
                Value::Float(f) => visitor.visit_f32(f),
            },
            _ => unreachable!(),
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
            }
            DictionaryParserState::ParseKey(_, _) => self.deserialize_identifier(visitor),
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
            DictionaryParserState::ParseValue(_, _) => visitor.visit_seq(self),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_tuple<V>(self, _: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.state {
            DictionaryParserState::ParseValue(_, _) => visitor.visit_seq(self),
            _ => self.deserialize_any(visitor),
        }
    }
}

impl<'de: 'a, 'a, 'b> de::MapAccess<'de> for &'a mut DictionaryDeserializer<'b, 'de> {
    type Error = DeserializerError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if self.data.len() == 0 {
            return Ok(None);
        }
        let parsing_result = recognize!(self.data, many0!(map!(parse_operand, |_| ())))?;
        let (operator_bytes, operands) = parsing_result;

        let (rem_bytes, operator) = parse_operator(operator_bytes)?;

        self.state = DictionaryParserState::ParseKey(operator, operands);
        self.data = rem_bytes;
        seed.deserialize(&mut **self).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        match self.state {
            DictionaryParserState::ParseKey(_, operands) => {
                let (rem_bytes, value) = parse_operand(operands)?;
                self.state = DictionaryParserState::ParseValue(value, rem_bytes);
                seed.deserialize(&mut **self)
            }
            _ => panic!("Internal Inconsistency"),
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
            return Ok(None);
        }

        let result = seed.deserialize(&mut **self).map(Some);

        let bytes = match self.state {
            DictionaryParserState::ParseValue(_, bytes) => bytes,
            _ => panic!("Internal Inconsistency"),
        };
        if bytes.len() == 0 {
            self.state = DictionaryParserState::EndArray;
            return result;
        }

        let (rem_bytes, value) = parse_operand(bytes)?;
        self.state = DictionaryParserState::ParseValue(value, rem_bytes);

        result
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use serde::Deserialize;

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

        let string_index = cff::Index::default();
        let mut deserializer = cff::DictionaryDeserializer::new(&data, &string_index);

        let result = MetaDict::deserialize(&mut deserializer).unwrap();

        assert_eq!(
            MetaDict {
                copyright: [0, -10000],
                version: -10000
            },
            result
        )
    }
}