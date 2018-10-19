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

use tables::glyf::{self, Glyf, SimpleGlyph, CompositeGlyph, parse_simple_glyph, parse_composite_glyph, parse_header, GlyphPoint, GlyphPointIter};
use tables::loca::Loca;
use tables::head::Head;
use OpentypeTableAccess;
use error::ParserError;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum QuadraticPath {
    MoveTo(i32, i32),
    LineTo(i32, i32),
    CurveTo(i32, i32, i32, i32),
    Close,
}

#[derive(Debug, Copy, Clone)]
pub enum Glyph<'a> {
    Simple(SimpleGlyph<'a>),
    Composite(CompositeGlyph<'a>)
}

impl<'a> Glyph<'a> {
    pub fn header(&self) -> glyf::Header {
        match self {
            Glyph::Simple(g) => g.header,
            Glyph::Composite(g) => g.header
        }
    }

    pub fn contour_iter(&self) -> impl 'a + Iterator<Item=QuadraticPath> {
        match self {
            Glyph::Simple(g) => {
                let point_iter = g.point_iter();
                let end_pts_of_contours = g.end_pts_of_contours;
                ContourIterator { point_iter, end_pts_of_contours, last_pt: None, index: 0 }
            },
            Glyph::Composite(_) => unimplemented!(),
        }
    }
}

#[derive(Debug)]
struct ContourIterator<'a> {
    point_iter: GlyphPointIter<'a>,
    end_pts_of_contours: &'a [u8],
    last_pt: Option<GlyphPoint>,
    index: usize,
}

impl<'a> ContourIterator<'a> {
    fn get_last_contour_pt(&self) -> Option<u16> {
        Some((*self.end_pts_of_contours.get(0)? as u16) << 8 | *self.end_pts_of_contours.get(1)? as u16)
    }

    fn next_last_contour_pt(&mut self) {
        self.end_pts_of_contours = &self.end_pts_of_contours[2..];
    }
}

impl<'a> Iterator for ContourIterator<'a> {
    type Item = QuadraticPath;

    fn next(&mut self) -> Option<QuadraticPath> {
        if self.get_last_contour_pt().is_some() {
            if self.index == self.get_last_contour_pt().unwrap() as usize + 1 {
                self.next_last_contour_pt();
                self.last_pt = None;
                return Some(QuadraticPath::Close)
            }
        }
        
        let point = self.point_iter.next()?;

        info!("{:?}", point);
        let last_pt = match self.last_pt {
            Some(pt) => pt,
            None => { 
                self.last_pt = Some(point); 
                self.index += 1;
                return Some(QuadraticPath::MoveTo(point.x, point.y));
            }
        };

        self.last_pt = Some(point);
        self.index += 1;
        
        let curve = if point.on_curve && last_pt.on_curve {
            QuadraticPath::LineTo(point.x, point.y)
        } else if point.on_curve && !last_pt.on_curve {
            QuadraticPath::CurveTo(last_pt.x, last_pt.y, point.x, point.y)
        } else if !point.on_curve && last_pt.on_curve {
            // the maximal depth of recursion is 1
            return self.next()
        } else {
            let mid_point_x = (point.x + last_pt.x) / 2;
            let mid_point_y = (point.y + last_pt.y) / 2;
            QuadraticPath::CurveTo(last_pt.x, last_pt.y, mid_point_x, mid_point_y)
        };


        Some(curve)
    }
}

named!(parse_glyph<&[u8], Glyph>, 
    switch!(
        map!(peek!(parse_header), |header| header.number_of_contours < 0),
        true => map!(parse_composite_glyph, Glyph::Composite) |
        false => map!(parse_simple_glyph, Glyph::Simple)
    )
);

#[derive(Debug, Clone)]
pub struct GlyphAccessor<'font> {
    loca: Loca,
    glyf: Glyf<'font>,
}

impl<'font> GlyphAccessor<'font> {
    pub fn new(font: &'font impl OpentypeTableAccess) -> Result<Self, ParserError> {
        let head: Head = font.parse_table()?;
        let loca = font.parse_table_context(head.index_to_loc_format)?;
        let glyf = font.parse_table()?;
        Ok(GlyphAccessor { loca, glyf })
    }

    pub fn num_glyphs(&self) -> u32 {
        self.loca.num_entries().saturating_sub(1) as u32
    }

    pub fn index(&self, index: u16) -> Result<Option<Glyph>, ParserError> {
        if self.num_glyphs() <= index as u32 {
            return Ok(None);
        }
        let start = self.loca.offset(index);
        let end = self.loca.offset(index + 1);
        assert!(start <= end);

        if start == end {
            return Ok(None);
        }

        let glyph_data = self.glyf.at_offset(start as usize, end as usize);
        let (_, glyph) = parse_glyph(glyph_data).map_err(|err| ParserError::glyph_parse(index as u32, err.into()))?;
        Ok(Some(glyph))
    }
}



#[cfg(test)]
mod test {
    use super::*;
    use Font;

    #[test]
    fn test_single_glyph() {
        let data = include_bytes!("../tests/font_files/Inconsolata-Regular.ttf");
        let font = Font::from_bytes(data, 0).expect("Could not read font.");

        let glyph_accessor = GlyphAccessor::new(&font).unwrap();
        let glyph = glyph_accessor.index(16).unwrap();
        let header = glyph.map(|g| g.header());
        println!("{:?}", header);
    }

    #[test]
    fn test_all_glyphs() {
        let data = include_bytes!("../tests/font_files/Inconsolata-Regular.ttf");
        let font = Font::from_bytes(data, 0).expect("Could not read font.");

        let glyph_accessor = GlyphAccessor::new(&font).unwrap();
        let mut glyphs = vec![];
        for index in 0..glyph_accessor.num_glyphs() {
            glyphs.push(glyph_accessor.index(index as u16));
        }
    }
}
