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

    pub fn parse(chars: &[char]) -> (Option<AnsiEscape>, usize) {
        let mut i = 0;
        let mut tmp = String::new();

        loop {
            match chars[i] {
                'm' => break,
                _ => {
                    tmp.push(chars[i]);
                    i += 1;
                }
            }
        }

        let num = tmp.parse::<u8>().unwrap();
        match num {
            0 => (Some(AnsiEscape::Reset), i),
            1...8 => (None, 0),
            30...37 => (Some(AnsiEscape::Foreground(num - 30)), i),
            40...47 => (Some(AnsiEscape::Background(num - 40)), i),
            _ => (None, 0),
        }
    }
}
