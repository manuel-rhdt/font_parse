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

use error::ParserError;
use tables::cff;
use OpentypeTableAccess;

use nom::{be_i16, be_i32, Err, IResult};

use std::collections::VecDeque;
use std::fmt::{Debug, Error, Formatter};

const SUBROUTINE_EVAL_MAX_DEPTH: usize = 64;

/// A fixed-point number with a 16 bit integral component and a 16 bit fractional component.

#[derive(
    Default,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Add,
    AddAssign,
    Mul,
    MulAssign,
    Sub,
    SubAssign,
    Div,
    DivAssign,
    Neg,
    Hash,
)]
pub struct Fixed16_16(i32);

impl Debug for Fixed16_16 {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        if self.frac() == 0 {
            self.int().fmt(f)
        } else {
            f32::from(*self).fmt(f)
        }
    }
}

impl Fixed16_16 {
    /// Return the integer component, discarding the fractional part.
    pub fn int(self) -> i16 {
        (self.0 >> 16) as i16
    }

    /// Return the fractional part.
    pub fn frac(self) -> i16 {
        self.0 as i16
    }

    /// Checked addition. Computes `self + rhs`, returning `None` if overflow occurred.
    pub fn checked_add(self, rhs: Fixed16_16) -> Option<Fixed16_16> {
        self.0.checked_add(rhs.0).map(Fixed16_16)
    }
}

impl From<i16> for Fixed16_16 {
    fn from(val: i16) -> Fixed16_16 {
        Fixed16_16((val as i32) << 16)
    }
}

impl From<f32> for Fixed16_16 {
    fn from(val: f32) -> Fixed16_16 {
        Fixed16_16((val * (1 << 16) as f32).round() as i32)
    }
}

impl From<Fixed16_16> for f32 {
    fn from(val: Fixed16_16) -> f32 {
        val.0 as f32 / (1 << 16) as f32
    }
}

#[derive(Debug)]
pub struct Glyph<'font> {
    parser: CffCharstringParser<'font>,
}

impl<'font> Glyph<'font> {
    pub fn contour_iter(&mut self) -> &mut CffCharstringParser<'font> {
        &mut self.parser
    }
}

#[derive(Debug, Clone)]
pub struct GlyphAccessor<'font> {
    cff: cff::Cff<'font>,
    parser_stack: VecDeque<Fixed16_16>,
}

impl<'font> GlyphAccessor<'font> {
    pub fn new(font: &'font impl OpentypeTableAccess) -> Result<Self, ParserError> {
        let cff = font.parse_table()?;
        Ok(GlyphAccessor {
            cff,
            parser_stack: Default::default(),
        })
    }

    pub fn num_glyphs(&self) -> u32 {
        self.cff.num_glyphs()
    }

    pub fn index(&mut self, index: u32) -> Option<Glyph<'_>> {
        let charstring = self.cff.charstring(index)?;
        let parser = CffCharstringParser::new(
            index,
            charstring,
            &mut self.parser_stack,
            Some(&self.cff.global_subrs),
            Some(&self.cff.local_subrs),
            self.cff.private_dict_data.nominal_width_x,
        );
        Some(Glyph { parser })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PathInstruction {
    MoveTo(Fixed16_16, Fixed16_16),
    LineTo(Fixed16_16, Fixed16_16),
    CurveTo(
        Fixed16_16,
        Fixed16_16,
        Fixed16_16,
        Fixed16_16,
        Fixed16_16,
        Fixed16_16,
    ),
    Close,
}

#[derive(Debug)]
pub struct CffCharstringParser<'a> {
    // used for logging
    glyph_id: u32,

    // references to subroutine indexes
    local_subr: Option<&'a cff::Index<'a>>,
    global_subr: Option<&'a cff::Index<'a>>,

    // stack of parsing data (needed for subroutine calls)
    code: Vec<&'a [u8]>,

    // queue of operands
    stack: &'a mut VecDeque<Fixed16_16>,

    c1x: Fixed16_16,
    c1y: Fixed16_16,
    c2x: Fixed16_16,
    c2y: Fixed16_16,
    x: Fixed16_16,
    y: Fixed16_16,

    width: Option<Fixed16_16>,
    nominal_width_x: Fixed16_16,
    nstems: usize,
    open: bool,

    current_op: u8,
    repeat_c: i32,
    should_repeat: bool,

    next_instr: Option<PathInstruction>,
}

impl<'a> CffCharstringParser<'a> {
    fn new(
        glyph_id: u32,
        bytes: &'a [u8],
        stack: &'a mut VecDeque<Fixed16_16>,
        global_subr: Option<&'a cff::Index<'a>>,
        local_subr: Option<&'a cff::Index<'a>>,
        nominal_width_x: i32,
    ) -> Self {
        stack.clear();
        CffCharstringParser {
            glyph_id,
            global_subr,
            local_subr,
            code: vec![bytes],
            stack,
            nominal_width_x: (nominal_width_x as i16).into(),
            c1x: Default::default(),
            c1y: Default::default(),
            c2x: Default::default(),
            c2y: Default::default(),
            x: Default::default(),
            y: Default::default(),
            current_op: Default::default(),
            repeat_c: Default::default(),
            should_repeat: Default::default(),
            width: Default::default(),
            open: Default::default(),
            nstems: Default::default(),
            next_instr: Default::default(),
        }
    }

    fn cff_subroutine_bias(subr: &cff::Index) -> i32 {
        if subr.len() < 1240 {
            107
        } else if subr.len() < 33900 {
            1131
        } else {
            32768
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(&byte) = self.code.last()?.get(0) {
            *self.code.last_mut()? = &self.code.last()?[1..];
            Some(byte)
        } else {
            self.code.pop()?;
            None
        }
    }

    fn parse_nom<T>(&mut self, f: impl Fn(&[u8]) -> IResult<&[u8], T>) -> Option<T> {
        let result = f(self.code.last()?);
        match result {
            Result::Err(Err::Incomplete(_)) => {
                // we reached the end of input
                warn!("Unexpected enf of input in cff charstring parser");
                self.code = vec![];
                None
            }
            Result::Err(err) => {
                // other error
                warn!("Error while parsing cff charstring: {}", err);
                None
            }
            Result::Ok((rem_bytes, t)) => {
                *self.code.last_mut()? = rem_bytes;
                Some(t)
            }
        }
    }

    fn repeat(&mut self) {
        if !self.stack.is_empty() {
            self.should_repeat = true;
        }
    }

    fn clear_vars(&mut self) {
        self.x = 0.into();
        self.y = 0.into();
        self.c1x = 0.into();
        self.c1y = 0.into();
        self.c2x = 0.into();
        self.c2y = 0.into();
    }

    fn move_to(&mut self) -> PathInstruction {
        let move_instr = PathInstruction::MoveTo(self.x, self.y);
        if self.open {
            self.next_instr = Some(move_instr);
            PathInstruction::Close
        } else {
            move_instr
        }
    }

    fn line_to(&self) -> PathInstruction {
        PathInstruction::LineTo(self.x, self.y)
    }

    fn curve_to(&self) -> PathInstruction {
        PathInstruction::CurveTo(self.c1x, self.c1y, self.c2x, self.c2y, self.x, self.y)
    }

    fn evaluate_subroutine(&mut self, subr: &'a [u8]) {
        if self.code.len() > SUBROUTINE_EVAL_MAX_DEPTH {
            self.code = vec![];
        } else {
            self.code.push(subr);
        }
    }

    fn parse_stems(&mut self) -> Option<()> {
        let has_width_arg = self.stack.len() % 2 != 0;
        if has_width_arg && self.width.is_none() {
            self.width = Some(self.stack.pop_back()? + self.nominal_width_x);
        }

        self.nstems += self.stack.len() >> 1;
        self.stack.clear();
        Some(())
    }

    // This function is heavily inspired on the cff.js file of the opentype.js
    // project.
    fn parse_byte(&mut self) -> Option<PathInstruction> {
        if let Some(pi) = self.next_instr.take() {
            return Some(pi);
        }

        // clears all path variables (i.e. x, y, c1x, ...)
        self.clear_vars();

        self.current_op = if self.should_repeat {
            self.should_repeat = false;
            self.repeat_c += 1;
            self.current_op
        } else {
            self.repeat_c = 0;
            self.next_byte()?
        };
        match self.current_op {
            // hstem | vstem
            1 | 3 => {
                trace!("{:?} h/vstem", self.stack);
                self.parse_stems()?;
                None
            }
            // vmoveto
            4 => {
                trace!("{:?} vmoveto", self.stack);
                if self.stack.len() > 1 && self.width.is_none() {
                    self.width = Some(self.nominal_width_x.checked_add(self.stack.pop_front()?)?);
                }

                self.y = self.stack.pop_front()?;
                Some(self.move_to())
            }
            // rlineto
            5 => {
                if self.repeat_c == 0 {
                    trace!("{:?} rlineto", self.stack);
                }
                self.x = self.stack.pop_front()?;
                self.y = self.stack.pop_front()?;
                self.repeat();
                Some(self.line_to())
            }
            // hlineto
            6 => {
                if self.repeat_c == 0 {
                    trace!("{:?} hlineto", self.stack);
                }
                if self.repeat_c % 2 == 0 {
                    self.x = self.stack.pop_front()?;
                    self.repeat();
                    Some(self.line_to())
                } else {
                    self.y = self.stack.pop_front()?;
                    self.repeat();
                    Some(self.line_to())
                }
            }
            // vlineto
            7 => {
                if self.repeat_c % 2 == 0 {
                    self.y = self.stack.pop_front()?;
                    self.repeat();
                    Some(self.line_to())
                } else {
                    self.x = self.stack.pop_front()?;
                    self.repeat();
                    Some(self.line_to())
                }
            }
            // rrcurveto
            8 => {
                if self.repeat_c == 0 {
                    trace!("{:?} rrcurveto", self.stack);
                }
                self.c1x = self.stack.pop_front()?;
                self.c1y = self.stack.pop_front()?;
                self.c2x = self.stack.pop_front()?;
                self.c2y = self.stack.pop_front()?;
                self.x = self.stack.pop_front()?;
                self.y = self.stack.pop_front()?;
                self.repeat();

                Some(self.curve_to())
            }
            // callsubr
            10 => {
                trace!("{:?} callsubr", self.stack);
                let code_index = (self.stack.pop_back()?.int() as i32)
                    .checked_add(Self::cff_subroutine_bias(self.local_subr?))?;
                let subr_code = self.local_subr?.get(code_index as usize)?;
                trace!("subroutine {}:", code_index);
                self.evaluate_subroutine(subr_code); 
                None
            }
            // return
            11 => {
                trace!("return");
                self.code.pop()?;
                None
            }
            // endchar
            14 => {
                if self.stack.len() > 0 && self.width.is_none() {
                    self.width = Some(self.nominal_width_x.checked_add(self.stack.pop_front()?)?);
                }

                self.code = vec![];
                if self.open {
                    self.open = false;
                    Some(PathInstruction::Close)
                } else {
                    None
                }
            }
            // hstemh
            18 => {
                self.parse_stems()?;
                None
            }
            // hintmask | cntrmask
            19 | 20 => {
                self.parse_stems()?;
                let nstems = self.nstems;
                self.parse_nom(|b| map!(b, take!((nstems + 7) >> 3), |_| ()));
                None
            }
            // rmoveto
            21 => {
                trace!("{:?} rmoveto", self.stack);
                if self.stack.len() > 2 && self.width.is_none() {
                    self.width = Some(self.nominal_width_x.checked_add(self.stack.pop_front()?)?);
                }

                self.x = self.stack.pop_front()?;
                self.y = self.stack.pop_front()?;
                Some(self.move_to())
            }
            // hmoveto
            22 => {
                if self.stack.len() > 1 && self.width.is_none() {
                    self.width = Some(self.nominal_width_x.checked_add(self.stack.pop_front()?)?);
                }

                self.x = self.stack.pop_front()?;
                Some(self.move_to())
            }
            // vstemh
            23 => {
                self.parse_stems()?;
                None
            }
            // vvcurveto
            26 => {
                if self.stack.len() % 2 > 0 {
                    self.x = self.stack.pop_front()?;
                }
                self.c1y = self.stack.pop_front()?;
                self.c2x = self.stack.pop_front()?;
                self.c2y = self.stack.pop_front()?;
                self.y = self.stack.pop_front()?;

                self.repeat();

                Some(self.curve_to())
            }
            // hhcurveto
            27 => {
                if self.stack.len() % 2 > 0 {
                    self.y = self.stack.pop_front()?;
                }
                self.c1x = self.stack.pop_front()?;
                self.c2x = self.stack.pop_front()?;
                self.c2y = self.stack.pop_front()?;
                self.x = self.stack.pop_front()?;

                self.repeat();

                Some(self.curve_to())
            }
            // shortint
            28 => {
                let val = self.parse_nom(be_i16)?;
                self.stack.push_back(val.into());
                None
            }
            // callgsubr
            29 => {
                let code_index = (self.stack.pop_back()?.int() as i32)
                    .checked_add(Self::cff_subroutine_bias(self.global_subr?))?;
                let subr_code = self.global_subr?.get(code_index as usize)?;
                self.evaluate_subroutine(subr_code);
                None
            }
            // vhcurveto
            30 => {
                if self.repeat_c % 2 == 0 {
                    self.c1y = self.stack.pop_front()?;
                    self.c2x = self.stack.pop_front()?;
                    self.c2y = self.stack.pop_front()?;
                    self.x = self.stack.pop_front()?;
                } else {
                    self.c1x = self.stack.pop_front()?;
                    self.c2x = self.stack.pop_front()?;
                    self.c2y = self.stack.pop_front()?;
                    self.y = self.stack.pop_front()?;
                }

                if self.stack.len() == 1 {
                    if self.repeat_c % 2 == 0 {
                        self.y = self.stack.pop_front()?;
                    } else {
                        self.x = self.stack.pop_front()?;
                    }
                }

                self.repeat();
                Some(self.curve_to())
            }
            // hvcurveto
            31 => {
                if self.repeat_c % 2 == 0 {
                    self.c1x = self.stack.pop_front()?;
                    self.c2x = self.stack.pop_front()?;
                    self.c2y = self.stack.pop_front()?;
                    self.y = self.stack.pop_front()?;
                } else {
                    self.c1y = self.stack.pop_front()?;
                    self.c2x = self.stack.pop_front()?;
                    self.c2y = self.stack.pop_front()?;
                    self.x = self.stack.pop_front()?;
                }

                if self.stack.len() == 1 {
                    if self.repeat_c % 2 == 0 {
                        self.x = self.stack.pop_front()?;
                    } else {
                        self.y = self.stack.pop_front()?;
                    }
                }

                self.repeat();
                Some(self.curve_to())
            }
            x @ 32...246 => {
                self.stack.push_back((x as i16 - 139).into());
                None
            }
            x @ 247...250 => {
                let w = self.next_byte()?;
                self.stack
                    .push_back(((x as i16 - 247) * 256 + w as i16 + 108).into());
                None
            }
            x @ 251...254 => {
                let w = self.next_byte()?;
                self.stack
                    .push_back((-(x as i16 - 251) * 256 - w as i16 - 108).into());
                None
            }
            255 => {
                let val = self.parse_nom(be_i32)?;
                self.stack.push_back(Fixed16_16(val));
                None
            }
            x @ 0...31 => {
                warn!(
                    "Unknown operator in cff charstring (glyph id={}): {}",
                    self.glyph_id, x
                );
                self.stack.clear();
                None
            }
            _ => unreachable!(),
        }
    }
}

impl<'a> Iterator for CffCharstringParser<'a> {
    type Item = PathInstruction;

    fn next(&mut self) -> Option<PathInstruction> {
        loop {
            if self.code.is_empty() {
                if self.stack.len() != 0 {
                    warn!(
                        "CFF charstring parser finished with non-empty stack: {:?}",
                        self.stack
                    );
                }
                break None;
            }
            if let Some(instr) = self.parse_byte() {
                break Some(instr);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_fixed_frac_conv() {
        assert_eq!(10.0f32, Fixed16_16::from(10).into());
        assert_eq!(0.5f32, (Fixed16_16::from(1) / 2).into())
    }

    #[test]
    fn test_cff_number_decoding() {
        let data = &[32, 246, 247, 10, 248, 10, 251, 10, 252, 10];
        let mut stack = VecDeque::new();
        {
            let parser = CffCharstringParser::new(0, data, &mut stack, None, None, 0);
            parser.for_each(|_| {});
        }

        assert_eq!(32 - 139, stack[0].int());
        assert_eq!(246 - 139, stack[1].int());
        assert_eq!(0 * 256 + 10 + 108, stack[2].int());
        assert_eq!(1 * 256 + 10 + 108, stack[3].int());
        assert_eq!(-0 * 256 - 10 - 108, stack[4].int());
        assert_eq!(-1 * 256 - 10 - 108, stack[5].int());
    }

    #[test]
    fn test_cff_charstring_rmoveto() {
        let data = &[10 + 139, 20 + 139, 21, 10 + 139, 20 + 139, 21];
        let mut stack = VecDeque::new();
        {
            let mut parser = CffCharstringParser::new(0, data, &mut stack, None, None, 0);
            let moveto = parser.next().unwrap();
            assert_eq!(moveto, PathInstruction::MoveTo(10.into(), 20.into()));
            let moveto = parser.next().unwrap();
            assert_eq!(moveto, PathInstruction::MoveTo(10.into(), 20.into()));
            assert_eq!(parser.next(), None);
        }

        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_cff_charstring_hmoveto() {
        let data = &[10 + 139, 22];
        let mut stack = VecDeque::new();
        {
            let mut parser = CffCharstringParser::new(0, data, &mut stack, None, None, 0);
            let moveto = parser.next().unwrap();
            assert_eq!(moveto, PathInstruction::MoveTo(10.into(), 0.into()));
        }

        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_cff_charstring_vmoveto() {
        let data = &[10 + 139, 4];
        let mut stack = VecDeque::new();
        {
            let mut parser = CffCharstringParser::new(0, data, &mut stack, None, None, 0);
            let moveto = parser.next().unwrap();
            assert_eq!(moveto, PathInstruction::MoveTo(0.into(), 10.into()));
        }

        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_cff_charstring_rlineto() {
        let data = &[10 + 139, 20 + 139, 10 + 139, 20 + 139, 5];
        let mut stack = VecDeque::new();
        {
            let mut parser = CffCharstringParser::new(0, data, &mut stack, None, None, 0);
            let line = parser.next().unwrap();
            assert_eq!(PathInstruction::LineTo(10.into(), 20.into()), line);
            let line = parser.next().unwrap();
            assert_eq!(PathInstruction::LineTo(10.into(), 20.into()), line);
        }

        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_cff_charstring_rrcurveto() {
        let data = &[
            10 + 139,
            20 + 139,
            10 + 139,
            20 + 139,
            10 + 139,
            20 + 139,
            8,
        ];
        let mut stack = VecDeque::new();
        {
            let mut parser = CffCharstringParser::new(0, data, &mut stack, None, None, 0);
            let line = parser.next().unwrap();
            assert_eq!(
                PathInstruction::CurveTo(
                    10.into(),
                    20.into(),
                    10.into(),
                    20.into(),
                    10.into(),
                    20.into()
                ),
                line
            );
        }

        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_cff_charstring_hvcurveto() {
        let data = &[10 + 139, 20 + 139, 10 + 139, 20 + 139, 31];
        let mut stack = VecDeque::new();
        {
            let mut parser = CffCharstringParser::new(0, data, &mut stack, None, None, 0);
            let line = parser.next().unwrap();
            assert_eq!(
                PathInstruction::CurveTo(
                    10.into(),
                    0.into(),
                    20.into(),
                    10.into(),
                    0.into(),
                    20.into()
                ),
                line
            );
        }

        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_cff_charstring_vhcurveto() {
        let data = &[
            10 + 139,
            20 + 139,
            10 + 139,
            20 + 139,
            10 + 139,
            20 + 139,
            10 + 139,
            20 + 139,
            10 + 139,
            20 + 139,
            10 + 139,
            20 + 139,
            30, //< vhcurveto
        ];
        let mut stack = VecDeque::new();
        {
            let mut parser = CffCharstringParser::new(0, data, &mut stack, None, None, 0);
            let line = parser.next().unwrap();
            assert_eq!(
                PathInstruction::CurveTo(
                    0.into(),
                    10.into(),
                    20.into(),
                    10.into(),
                    20.into(),
                    0.into()
                ),
                line
            );
            let line = parser.next().unwrap();
            assert_eq!(
                PathInstruction::CurveTo(
                    10.into(),
                    0.into(),
                    20.into(),
                    10.into(),
                    0.into(),
                    20.into()
                ),
                line
            );
            let line = parser.next().unwrap();
            assert_eq!(
                PathInstruction::CurveTo(
                    0.into(),
                    10.into(),
                    20.into(),
                    10.into(),
                    20.into(),
                    0.into()
                ),
                line
            );
            assert_eq!(parser.next(), None);
        }

        assert_eq!(stack.len(), 0);
    }
}
