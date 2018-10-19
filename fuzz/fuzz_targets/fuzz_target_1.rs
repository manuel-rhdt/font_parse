#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate font_parse;

use font_parse::{Font, OpentypeTableAccess, Glyph};

fuzz_target!(|data: &[u8]| {
    // fuzzed code goes here
    if let Ok(font) = Font::from_bytes(data, 0) {
        let mut glyph_accessor = match font.glyphs() {
            Ok(x) => x,
            Err(_) => return,
        };

        for i in 0..glyph_accessor.num_glyphs() {
            if let Ok(Some(Glyph::Cff(mut glyph))) = glyph_accessor.index(i) {
                let _: Vec<_> = glyph.contour_iter().collect();
            }
        }
    }
});
