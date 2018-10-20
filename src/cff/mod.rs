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

/// This module contains various utilities for parsing and using cff data in a font.

mod glyph_accessor;
mod standard_strings;
mod dictionary_deserializer;
mod index;

pub use self::glyph_accessor::*;
pub use self::standard_strings::*;
pub(crate) use self::dictionary_deserializer::DictionaryDeserializer;
pub use self::index::Index;
pub(crate) use self::index::parse_index;