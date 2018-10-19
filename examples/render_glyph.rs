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
#[macro_use]
extern crate log;
extern crate simple_logger;
extern crate svg;

use font_parse::{Font, Glyph, OpentypeTableAccess, PathInstruction, QuadraticPath};

use svg::node::element::path::Data;
use svg::node::element::Path;
use svg::Document;

use std::env;

fn main() {
    simple_logger::init_with_level(log::Level::Trace).unwrap();

    let glyph_id: u32 = env::args()
        .nth(1)
        .expect("Expected 1 argument")
        .parse()
        .expect("Expected number as first argument");

    // let data = include_bytes!("../tests/font_files/LinBiolinum_R.otf");
    let data = include_bytes!("../tests/font_files/Inconsolata-Regular.ttf");
    let font = Font::from_bytes(data, 0).expect("Could not read font.");

    let mut glyph_accessor = font.glyphs().unwrap();

    // path data
    let mut data = Data::new();
    match glyph_accessor.index(glyph_id) {
        Ok(Some(Glyph::Cff(mut glyph))) => {
            for instr in glyph.contour_iter() {
                match instr {
                    PathInstruction::MoveTo(x, y) => {
                        data = data.move_by((f32::from(x), f32::from(y)));
                    }
                    PathInstruction::LineTo(x, y) => {
                        data = data.line_by((f32::from(x), f32::from(y)));
                    }
                    PathInstruction::CurveTo(c1x, c1y, c2x, c2y, x, y) => {
                        let c1x = f32::from(c1x);
                        let c1y = f32::from(c1y);
                        let c2x = f32::from(c2x) + c1x;
                        let c2y = f32::from(c2y) + c1y;
                        let x = f32::from(x) + c2x;
                        let y = f32::from(y) + c2y;
                        data = data.cubic_curve_by((c1x, c1y, c2x, c2y, x, y));
                    }
                    PathInstruction::Close => {
                        data = data.close();
                    }
                }
            }
        }
        Ok(Some(Glyph::Ttf(glyph))) => {
            for path in glyph.contour_iter() {
                info!("{:?}", path);
                match path {
                    QuadraticPath::MoveTo(x, y) => data = data.move_to((x, y)),
                    QuadraticPath::LineTo(x, y) => data = data.line_to((x, y)),
                    QuadraticPath::CurveTo(cx, cy, x, y) => {
                        data = data.quadratic_curve_to((cx, cy, x, y))
                    }
                    QuadraticPath::Close => data = data.close(),
                }
            }
        }
        Ok(None) => panic!("Glyph not found"),
        Err(err) => panic!("{:?}", err),
    };

    let path = Path::new()
        .set("fill", "black")
        .set("fill-rule", "nonzero")
        .set("stroke-width", 0)
        .set("transform", "scale(1,-1)")
        .set("d", data);

    let document = Document::new()
        .set("viewBox", (0, -1800, 2000, 2000))
        .add(path);

    svg::save("image.svg", &document).unwrap();
}
