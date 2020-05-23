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

use super::SfntTable;

use std::ops::RangeFrom;

use crate::error::ParserError;
use nom::types::CompleteByteSlice;
use nom::{need_more, AsBytes, AtEof, IResult, InputLength, Needed, Slice};

fn be_u16<T>(i: T) -> IResult<T, u16>
where
    T: AsBytes,
    T: InputLength + AtEof,
    T: Slice<RangeFrom<usize>>,
{
    if i.input_len() < 2 {
        need_more(i, Needed::Size(2))
    } else {
        let bytes = i.as_bytes();
        let res = ((bytes[0] as u16) << 8) + bytes[1] as u16;
        Ok((i.slice(2..), res))
    }
}

fn be_u32<T>(i: T) -> IResult<T, u32>
where
    T: AsBytes,
    T: InputLength + AtEof,
    T: Slice<RangeFrom<usize>>,
{
    if i.input_len() < 4 {
        need_more(i, Needed::Size(4))
    } else {
        let bytes = i.as_bytes();
        let res = ((bytes[0] as u32) << 24)
            + ((bytes[1] as u32) << 16)
            + ((bytes[2] as u32) << 8)
            + bytes[3] as u32;
        Ok((i.slice(4..), res))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum LocFormat {
    Short,
    Long,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loca(Vec<u32>);

impl<'a> SfntTable<'a> for Loca {
    const TAG: &'static [u8; 4] = b"loca";
    type Context = LocFormat;
    type Err = ParserError;

    fn from_data(data: &'a [u8], format: LocFormat) -> Result<Self, Self::Err> {
        let data = CompleteByteSlice(data);
        let loca = match format {
            LocFormat::Short => many0!(data, map!(be_u16, |x| x as u32 * 2))?.1,
            LocFormat::Long => many0!(data, be_u32)?.1,
        };
        Ok(Loca(loca))
    }
}

impl Loca {
    // TODO: Error handling
    pub fn offset(&self, index: u16) -> u32 {
        self.0[index as usize]
    }

    pub fn num_entries(&self) -> usize {
        self.0.len()
    }
}
