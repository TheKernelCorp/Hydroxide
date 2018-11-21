use alloc::prelude::*;
use core::slice;

pub enum ColorPlace {}

#[derive(Debug)]
pub enum AnsiEscape {
    Foreground(u8),
    Background(u8),
    Reset,
}

pub struct Ansi;

pub static COLORS: &'static [u8] = include_bytes!("colors.bin");

impl Ansi {
    pub fn color(color: u8) -> u32 {
        let colors: &'static [u32] =
            unsafe { slice::from_raw_parts(COLORS.as_ptr() as *const _, COLORS.len()) };
        colors[color as usize]
    }

    pub fn parse(chars: &[char]) -> (Vec<Option<AnsiEscape>>, usize) {
        let mut i = 0;
        let mut skip = 0;
        let mut vec: Vec<Option<AnsiEscape>> = Vec::new();

        'outer: loop {
            let mut end = false;
            let mut tmp = String::new();
            'inner: loop {
                match chars[i] {
                    'm' => {
                        end = true;
                        break 'inner;
                    }
                    ';' => {
                        skip += 1;
                        i += 1;
                        break 'inner;
                    }
                    _ => {
                        tmp.push(chars[i]);
                        skip += 1;
                        i += 1;
                    }
                }
            }
            let num = tmp.parse::<u8>().unwrap();
            vec.push(match num {
                0 => Some(AnsiEscape::Reset),
                1...8 => None,
                30...37 => Some(AnsiEscape::Foreground(num - 30)),
                40...47 => Some(AnsiEscape::Background(num - 40)),
                _ => None,
            });
            if end {
                break 'outer;
            }
        }

        (vec, skip)
    }
}
