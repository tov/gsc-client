use std::fmt::{Display, Formatter, Result};
use textwrap;

pub struct Percentage(pub f64);

impl Display for Percentage {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{:.1}%", 100.0 * self.0)
    }
}

const HANGING_INDENT: &str = "    ";

pub fn hanging(text: &str) -> String {
    let width = textwrap::termwidth() - HANGING_INDENT.len();
    textwrap::indent(&textwrap::fill(text, width), HANGING_INDENT)
}
