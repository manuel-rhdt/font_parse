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

//! Structures describing the `glyf` table which contains glyph data for ttf fonts.

use nom::{self, be_i16, be_u16, rest};

use crate::error::ParserError;
use crate::tables::SfntTable;

#[derive(Debug, Copy, Clone)]
pub struct Glyf<'a> {
    data: &'a [u8],
}

impl<'a> SfntTable<'a> for Glyf<'a> {
    const TAG: &'static [u8; 4] = b"glyf";
    type Context = ();
    type Err = ParserError;

    fn from_data(data: &'a [u8], _: ()) -> Result<Self, Self::Err> {
        Ok(Glyf { data })
    }
}

impl<'a> Glyf<'a> {
    pub fn at_offset(&self, start: usize, end: usize) -> &[u8] {
        &self.data[start..end]
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct Header {
    pub number_of_contours: i16,
    pub xmin: i16,
    pub ymin: i16,
    pub xmax: i16,
    pub ymax: i16,
}

named!(pub parse_header<&[u8], Header>,
    do_parse!(
        number_of_contours: be_i16 >>
        xmin: be_i16 >>
        ymin: be_i16 >>
        xmax: be_i16 >>
        ymax: be_i16 >>
        (Header {
            number_of_contours,
            xmin,
            ymin,
            xmax,
            ymax
        })
    )
);

#[derive(Debug, Copy, Clone)]
pub struct SimpleGlyph<'a> {
    pub header: Header,
    pub(crate) end_pts_of_contours: &'a [u8],
    pub(crate) instructions: &'a [u8],
    pub(crate) outline: &'a [u8],
}

impl<'a> SimpleGlyph<'a> {
    pub fn num_points(self) -> u16 {
        let num_contours = self.end_pts_of_contours.len() / 2;
        if num_contours < 1 {
            0
        } else {
            let high = self.end_pts_of_contours[2 * num_contours - 2] as u16;
            let low = self.end_pts_of_contours[2 * num_contours - 1] as u16;
            (high << 8 | low) + 1
        }
    }

    pub fn flags_iter(self) -> FlagsIter<'a> {
        FlagsIter {
            bytes: self.outline,
            repeat: (0, 0),
        }
    }

    pub fn point_iter(&self) -> GlyphPointIter<'a> {
        info!("{:02x?}", self.outline);

        let mut remaining_points = self.num_points();
        let mut x_size = 0;
        let mut flags = self.outline.iter();

        while remaining_points > 0 {
            let byte = *flags.next().unwrap();

            let delta_x_size = if byte & 0x02 > 0 {
                // 0x02 flag indicates a single byte x vector
                1
            } else {
                if byte & 0x10 > 0 {
                    // 0x10 flag means same x value as before
                    0
                } else {
                    // x vector is a two byte integer
                    2
                }
            };
            x_size += delta_x_size;
            remaining_points -= 1;

            let repeat_count = if (byte & 0x08) > 0 {
                *flags.next().unwrap()
            } else {
                0
            };

            x_size += delta_x_size * repeat_count;
            remaining_points -= repeat_count as u16;
        }

        let flags_size = self.outline.len() - flags.len();
        let x_size = x_size as usize;

        GlyphPointIter {
            flags: FlagsIter {
                bytes: &self.outline[0..flags_size],
                repeat: (0, 0),
            },
            x_coordinates: &self.outline[flags_size..flags_size + x_size],
            y_coordinates: &self.outline[flags_size + x_size..],
            cursor: (0, 0),
        }
    }
}

/// A struct that represents a Point of a TrueType Outline.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct GlyphPoint {
    /// The absolute horizontal position.
    pub x: i32,
    /// The absolute vertical position.
    pub y: i32,
    /// `true` if this point is on the curve.
    pub on_curve: bool,
}

impl GlyphPoint {
    pub fn new(x: i32, y: i32, on_curve: bool) -> GlyphPoint {
        GlyphPoint { x, y, on_curve }
    }
}

/// An iterator over the glyph points on a TrueType outline.
#[derive(Debug, Clone)]
pub struct GlyphPointIter<'a> {
    flags: FlagsIter<'a>,
    x_coordinates: &'a [u8],
    y_coordinates: &'a [u8],
    cursor: (i32, i32),
}

impl<'a> Iterator for GlyphPointIter<'a> {
    type Item = GlyphPoint;

    fn next(&mut self) -> Option<GlyphPoint> {
        let flag = self.flags.next()?;

        let x = if flag & 0x02 > 0 {
            let (&x, remaining_x_coordinates) = self.x_coordinates.split_first()?;
            self.x_coordinates = remaining_x_coordinates;
            if flag & 0x10 > 0 {
                x as i16
            } else {
                -(x as i16)
            }
        } else {
            if flag & 0x10 > 0 {
                0
            } else {
                let (x, remaining_x_coordinates) = self.x_coordinates.split_at(2);
                self.x_coordinates = remaining_x_coordinates;
                (*x.get(0)? as i16) << 8 | *x.get(1)? as i16
            }
        };

        let y = if flag & 0x04 > 0 {
            let (&y, remaining_y_coordinates) = self.y_coordinates.split_first()?;
            self.y_coordinates = remaining_y_coordinates;
            if flag & 0x20 > 0 {
                y as i16
            } else {
                -(y as i16)
            }
        } else {
            if flag & 0x20 > 0 {
                0
            } else {
                let (y, remaining_y_coordinates) = self.y_coordinates.split_at(2);
                self.y_coordinates = remaining_y_coordinates;
                (*y.get(0)? as i16) << 8 | *y.get(1)? as i16
            }
        };

        self.cursor.0 = self.cursor.0.wrapping_add(x as i32);
        self.cursor.1 = self.cursor.1.wrapping_add(y as i32);

        Some(GlyphPoint::new(
            self.cursor.0,
            self.cursor.1,
            flag & 0x01 > 0,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct FlagsIter<'a> {
    bytes: &'a [u8],
    repeat: (u8, u8),
}

impl<'a> Iterator for FlagsIter<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        let (byte, repeat_count) = self.repeat;
        if repeat_count > 0 {
            self.repeat.1 -= 1;
            return Some(byte);
        }

        let (&byte, remaining_bytes) = self.bytes.split_first()?;
        self.bytes = remaining_bytes;
        if (byte & 0x08) > 0 {
            let repeat_count = *self.bytes.get(0)?;
            self.repeat = (byte, repeat_count);
        }

        Some(byte)
    }
}

named!(pub parse_simple_glyph<&[u8], SimpleGlyph>,
    dbg_dmp!(do_parse!(
        header: verify!(parse_header, |Header { number_of_contours, .. }| number_of_contours >= 0) >>
        end_pts_of_contours: return_error!(nom::ErrorKind::Custom(0), complete!(take!(header.number_of_contours as u16 * 2))) >>
        instructions: return_error!(nom::ErrorKind::Custom(1), complete!(length_data!(be_u16))) >>
        outline: rest >>
        (SimpleGlyph {
            header,
            end_pts_of_contours,
            instructions,
            outline
        })
    ))
);

#[derive(Debug, Copy, Clone)]
pub struct CompositeGlyph<'a> {
    pub header: Header,
    data: &'a [u8],
}

named!(pub parse_composite_glyph<&[u8], CompositeGlyph>,
    do_parse!(
        header: verify!(parse_header, |Header { number_of_contours, .. }| number_of_contours < 0) >>
        data: rest >>
        (CompositeGlyph {
            header,
            data,
        })
    )
);

#[cfg(test)]
mod test {
    use super::*;

    // We define a glyph with 1 contour and 3 points.
    const HEADER: &'static [u8] = &[0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00, 0x05];
    const CONTOUR_END_PTS: &'static [u8] = &[0x00, 0x02];
    const INSTRUCTIONS: &'static [u8] = &[0x00, 0x00];

    #[test]
    fn test_simple_glyph_header() {
        let (_, header) = parse_header(HEADER).unwrap();
        assert_eq!(
            header,
            Header {
                number_of_contours: 1,
                xmin: 2,
                ymin: 3,
                xmax: 4,
                ymax: 5
            }
        )
    }

    #[test]
    fn test_simple_glyph_empty() {
        const HEADER_0: &'static [u8] =
            &[0x00, 0x00, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00, 0x05];

        let mut glyph_data = vec![];
        glyph_data.extend(HEADER_0);
        glyph_data.extend(INSTRUCTIONS);
        let (_, glyph) = parse_simple_glyph(&glyph_data).unwrap();

        assert_eq!(glyph.num_points(), 0);
    }

    #[test]
    fn test_simple_glyph() {
        // we have 3 positive short vectors
        const FLAGS: &'static [u8] = &[
            0x02 | 0x04 | 0x10 | 0x20,
            0x01 | 0x02 | 0x04 | 0x10 | 0x20,
            0x02 | 0x04 | 0x10 | 0x20,
        ];
        const X_VALUES: &'static [u8] = &[0x01, 0x02, 0x03];
        const Y_VALUES: &'static [u8] = &[0x04, 0x05, 0x06];

        let mut glyph_data = vec![];
        glyph_data.extend(HEADER);
        glyph_data.extend(CONTOUR_END_PTS);
        glyph_data.extend(INSTRUCTIONS);
        glyph_data.extend(FLAGS);
        glyph_data.extend(X_VALUES);
        glyph_data.extend(Y_VALUES);
        let (_, glyph) = parse_simple_glyph(&glyph_data).unwrap();

        assert_eq!(glyph.num_points(), 3);

        let mut iter = glyph.point_iter();

        assert_eq!(iter.next().unwrap(), GlyphPoint::new(1, 4, false));
        assert_eq!(iter.next().unwrap(), GlyphPoint::new(3, 9, true));
        assert_eq!(iter.next().unwrap(), GlyphPoint::new(6, 15, false));
        assert_eq!(iter.next(), None);
    }
}
