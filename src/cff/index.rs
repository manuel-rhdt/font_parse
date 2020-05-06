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

use crate::error::ParserError;

use nom::{be_u16, be_u24, be_u32, be_u8};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Index<'data> {
    /// The offsets into the data that index the elements
    pub offsets: Vec<usize>,
    pub data: &'data [u8]
}

impl<'data> Index<'data> {
    pub fn empty() -> Self {
        Index {
            offsets: vec![],
            data: &[]
        }
    }

    pub fn parse_from(data: &'data [u8]) -> Result<Self, ParserError> {
        parse_index(data)
            .map(|(_, index)| Index {
                ..index
            })
            .map_err(ParserError::from)
    }

    pub fn get(&self, index: usize) -> Option<&'data [u8]> {
        // cff uses 1-based indexing
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

named!(pub(crate) parse_index<&[u8], Index>,
    do_parse!(
        num_offsets: map!(be_u16, |x| x as usize) >>
        offsets: apply!(parse_offset_list, num_offsets) >>
        data: take!(offsets.last().map(|&offset| offset.saturating_sub(1)).unwrap_or(0)) >>
        (Index { offsets, data })
    )
);

#[cfg(test)]
mod test {
    use super::*;

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
