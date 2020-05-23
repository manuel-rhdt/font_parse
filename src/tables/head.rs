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

use nom::{be_i16, be_i64, be_u16, be_u32};

use super::loca::LocFormat;
use super::SfntTable;
use crate::error::ParserError;

#[derive(Debug)]
pub struct Head {
    pub major_version: u16,
    pub minor_version: u16,
    pub font_revision: u32,
    pub check_sum_adjustment: u32,
    pub magic_number: u32,
    pub flags: u16,
    pub units_per_em: u16,
    pub created: i64,
    pub modified: i64,
    pub x_min: i16,
    pub y_min: i16,
    pub x_max: i16,
    pub y_max: i16,
    pub mac_style: u16,
    pub lowest_rec_ppem: u16,
    pub font_direction_hint: i16,
    pub index_to_loc_format: LocFormat,
    pub glyph_data_format: i16,
}

impl<'a> SfntTable<'a> for Head {
    const TAG: &'static [u8; 4] = b"head";
    type Context = ();
    type Err = ParserError;

    fn from_data(data: &'a [u8], _: ()) -> Result<Self, Self::Err> {
        parse_head(data)
            .map(|(_, result)| result)
            .map_err(|err| err.into())
    }
}

named!(parse_head<&[u8], Head>,
    do_parse!(
        major_version: be_u16 >>
        minor_version: be_u16 >>
        font_revision: be_u32 >>
        check_sum_adjustment: be_u32 >>
        magic_number: be_u32 >>
        flags: be_u16 >>
        units_per_em: be_u16 >>
        created: be_i64 >>
        modified: be_i64 >>
        x_min: be_i16 >>
        y_min: be_i16 >>
        x_max: be_i16 >>
        y_max: be_i16 >>
        mac_style: be_u16 >>
        lowest_rec_ppem: be_u16 >>
        font_direction_hint: be_i16 >>
        index_to_loc_format: be_i16 >>
        glyph_data_format: be_i16 >>
        (Head {
            major_version,
            minor_version,
            font_revision,
            check_sum_adjustment,
            magic_number,
            flags,
            units_per_em,
            created,
            modified,
            x_min,
            y_min,
            x_max,
            y_max,
            mac_style,
            lowest_rec_ppem,
            font_direction_hint,
            index_to_loc_format: match index_to_loc_format {
                0 => LocFormat::Short,
                _ => LocFormat::Long,   
            },
            glyph_data_format
        })
    )
);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_head_table() {
        let bytes = include_bytes!("../../tests/font_fragments/head_table");
        let head = Head::from_data(&*bytes, ()).unwrap();

        assert_eq!(head.x_min, 0x1234);
        assert_eq!(head.x_max, 0x1234);
        assert_eq!(head.y_min, 0x1234);
        assert_eq!(head.y_max, 0x1234);
        assert_eq!(head.index_to_loc_format, LocFormat::Long);
    }

    #[test]
    fn test_parse_head_table_2() {
        let bytes = include_bytes!("../../tests/font_fragments/head_table.2");
        let head = Head::from_data(&*bytes, ()).unwrap();

        assert_eq!(head.x_min, 0x1234);
        assert_eq!(head.x_max, 0x1234);
        assert_eq!(head.y_min, 0x1234);
        assert_eq!(head.y_max, 0x1234);
        assert_eq!(head.index_to_loc_format, LocFormat::Short);
    }

}
