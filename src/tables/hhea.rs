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

use crate::error::ParserError;

#[derive(Debug)]
pub struct Hhea {
    major_version: u16,
    minor_version: u16,
}

impl<'a> SfntTable<'a> for &'a Hhea {
    const TAG: &'static [u8; 4] = b"hhea";
    type Context = ();
    type Err = ParserError;

    fn from_data(data: &'a [u8], _: ()) -> Result<Self, Self::Err> {
        unsafe { Ok(&*(data as *const [u8] as *const Hhea)) }
    }
}
