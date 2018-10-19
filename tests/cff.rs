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

extern crate font_parse;
extern crate simple_logger;

use font_parse::tables::cff;
use font_parse::{Font, OpentypeTableAccess, Tag, Glyph};

fn get_cff_font() -> Font<'static> {
    let data = include_bytes!("font_files/LinBiolinum_R.otf");
    Font::from_bytes(data, 0).expect("Could not read font.")
}

#[test]
fn test_cff_font() {
    let font = get_cff_font();
    assert!(font.table_data(Tag::new('C', 'F', 'F', ' ')).is_some());
}

#[test]
fn test_parse_cff_table() {
    let font = get_cff_font();
    let cff: cff::Cff = font.parse_table().unwrap();
    assert_eq!("LinBiolinumO", cff.name);
}

#[test]
fn test_parse_glyphs_cff() {
    simple_logger::init().unwrap();

    let data = include_bytes!("../tests/font_files/LinBiolinum_R.otf");
    let font = Font::from_bytes(data, 0).expect("Could not read font.");

    let mut glyph_accessor = font.glyphs().unwrap();

    let num_glyphs = glyph_accessor.num_glyphs();
    for index in 0..num_glyphs {
        if let Ok(Some(Glyph::Cff(mut glyph))) = glyph_accessor.index(index) {
            let _: Vec<_> = glyph.contour_iter().collect();
        } else {
            panic!()
        }
    }
}