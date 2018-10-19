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

use cff_glyph_accessor::{GlyphAccessor as CffGlyphAccessor, Glyph as CffGlyph};
use ttf_glyph_accessor::{GlyphAccessor as TtfGlyphAccessor, Glyph as TtfGlyph};

use error::ParserError;

#[derive(Debug)]
pub enum Glyph<'font> {
    Cff(CffGlyph<'font>),
    Ttf(TtfGlyph<'font>)
}

#[derive(Debug, Clone)]
pub(crate) enum _GlyphAccessor<'font> {
    Cff(CffGlyphAccessor<'font>),
    Ttf(TtfGlyphAccessor<'font>)
}

#[derive(Debug, Clone, From)]
pub struct GlyphAccessor<'font>(pub(crate) _GlyphAccessor<'font>);

impl<'font> GlyphAccessor<'font> {
    pub fn num_glyphs(&self) -> u32 {
        match self.0 {
            _GlyphAccessor::Cff(ref accessor) => accessor.num_glyphs(),
            _GlyphAccessor::Ttf(ref accessor) => accessor.num_glyphs(),
        }
    }

    pub fn index(&mut self, index: u32) -> Result<Option<Glyph<'_>>, ParserError> {
        let glyph = match self.0 {
            _GlyphAccessor::Cff(ref mut accessor) => accessor.index(index).map(Glyph::Cff),
            _GlyphAccessor::Ttf(ref accessor) => accessor.index(index as u16)?.map(Glyph::Ttf),
        };
        Ok(glyph)
    }
}