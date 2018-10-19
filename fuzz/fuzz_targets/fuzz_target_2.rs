#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate font_parse;

use font_parse::{Glyph, OpentypeTableAccess, Tag};

struct FuzzFont<'a> {
    data: &'a [u8],
}

impl<'a> OpentypeTableAccess for FuzzFont<'a> {
    fn table_data(&self, tag: Tag) -> Option<&[u8]> {
        if tag == Tag::new('C', 'F', 'F', ' ') {
            Some(self.data)
        } else {
            None
        }
    }

    fn all_tables(&self) -> Vec<Tag> {
        vec![Tag::new('C', 'F', 'F', ' ')]
    }
}

fuzz_target!(|data: &[u8]| {
    // fuzzed code goes here
    let font = FuzzFont { data };
    let mut glyph_accessor = match font.glyphs() {
        Ok(x) => x,
        Err(_) => return,
    };

    for i in 0..glyph_accessor.num_glyphs() {
        if let Ok(Some(Glyph::Cff(mut glyph))) = glyph_accessor.index(i) {
            let _: Vec<_> = glyph.contour_iter().collect();
        }
    }
});
