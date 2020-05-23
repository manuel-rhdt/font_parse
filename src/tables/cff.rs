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

use nom::be_u8;
use serde::Deserialize;

use super::SfntTable;
use crate::cff;
use crate::cff::parse_index;
use crate::error::ParserError;

#[derive(Debug, Clone)]
pub struct Cff<'font> {
    pub header: Header,
    pub name: String,
    pub top_dict_data: TopDictData,
    pub(crate) private_dict_data: PrivateDictData,
    pub(crate) char_strings: cff::Index<'font>,
    pub(crate) global_subrs: cff::Index<'font>,
    pub(crate) local_subrs: cff::Index<'font>,
}

impl<'font> Cff<'font> {
    fn from_cffdata(cffdata: CffData<'font>, data: &'font [u8]) -> Result<Self, ParserError> {
        let name = cffdata
            .name_index
            .get(0)
            .ok_or(ParserError::from_string("Expected name index.".to_string()))?;
        let name = String::from_utf8_lossy(name).into_owned();

        let top_dict_index = cffdata.top_dict_index;
        let string_index = cffdata.string_index;

        let top_dict_data = top_dict_index.get(0).ok_or(ParserError::from_string(
            "Expected top dict index.".to_string(),
        ))?;

        let mut dictionary_deserializer =
            cff::DictionaryDeserializer::new(top_dict_data, &string_index);

        let top_dict_data = TopDictData::deserialize(&mut dictionary_deserializer)?;

        let char_strings = data
            .get(top_dict_data.char_strings..)
            .ok_or(ParserError::from_string(format!("no char strings")))?;
        let char_strings = cff::Index::parse_from(char_strings)?;

        let p_data_start = top_dict_data.private.1;
        let p_data_end = top_dict_data.private.0 + p_data_start;
        let private_dict_data = data
            .get(p_data_start..p_data_end)
            .ok_or(ParserError::from_string(format!("no private dict")))?;
        let mut dictionary_deserializer =
            cff::DictionaryDeserializer::new(private_dict_data, &string_index);
        let private_dict_data = PrivateDictData::deserialize(&mut dictionary_deserializer)?;

        let local_subrs = if private_dict_data.subrs != 0 {
            let subrs_start = p_data_start + private_dict_data.subrs;
            if let Some(subrs_data) = data.get(subrs_start..) {
                cff::Index::parse_from(subrs_data)?
            } else {
                cff::Index::empty()
            }
        } else {
            cff::Index::empty()
        };

        Ok(Cff {
            header: cffdata.header,
            name,
            top_dict_data,
            char_strings,
            private_dict_data,
            global_subrs: cffdata.global_subr_index,
            local_subrs,
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
    const TAG: &'static [u8; 4] = b"CFF ";
    type Context = ();
    type Err = ParserError;

    fn from_data(data: &'a [u8], _: ()) -> Result<Self, Self::Err> {
        parse_cff_table(data)
            .map_err(|err| err.into())
            .and_then(|result| Cff::from_cffdata(result.1, data))
    }
}

#[derive(Debug, Clone)]
struct CffData<'data> {
    header: Header,
    name_index: cff::Index<'data>,
    top_dict_index: cff::Index<'data>,
    string_index: cff::Index<'data>,
    global_subr_index: cff::Index<'data>,
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

#[derive(Default, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
#[serde(default)]
pub struct TopDictData {
    #[serde(rename = "version")]
    pub version: u32,
    pub notice: String,
    pub copyright: String,
    pub full_name: String,
    pub family_name: String,
    pub weight: String,
    char_strings: usize,
    // size and offset of private dict
    private: (usize, usize),
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
