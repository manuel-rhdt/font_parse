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

use nom::{be_u16, be_u32};

use super::SfntTable;
use crate::error::ParserError;

#[derive(Debug)]
pub struct Maxp<'a> {
    pub version: u32,
    pub num_glyphs: u16,
    remainder: &'a [u8],
}

impl<'a> SfntTable<'a> for Maxp<'a> {
    const TAG: &'static [u8; 4] = b"maxp";
    type Context = ();
    type Err = ParserError;

    fn from_data(data: &'a [u8], _: ()) -> Result<Self, Self::Err> {
        parse_maxp(data)
            .map(|result| result.1)
            .map_err(|err| err.into())
    }
}

named!(pub parse_maxp<&[u8], Maxp>,
    do_parse!(
        version: be_u32 >>
        num_glyphs: be_u16 >>
        remainder: cond!(version >= 0x10000000, take!(26)) >>
        (Maxp {
            version,
            num_glyphs,
            remainder: remainder.unwrap_or(&[]),
        })
    )
);
