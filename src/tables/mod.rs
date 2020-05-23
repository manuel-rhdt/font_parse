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

pub mod cff;
pub mod glyf;
pub mod head;
pub mod hhea;
pub mod loca;
pub mod maxp;

pub trait SfntTable<'a>: Sized {
    const TAG: &'static [u8; 4];

    type Context;
    type Err: 'static + ::std::error::Error;

    fn from_data(data: &'a [u8], context: Self::Context) -> Result<Self, Self::Err>;
}
