use std::fmt::Display;

pub struct Row(Vec<String>);

impl Row {
    pub fn new() -> Self {
        Row(Vec::new())
    }

    pub fn add_cell<S: Display>(mut self, value: S) -> Self {
        self.0.push(value.to_string());
        self
    }
}

pub enum FormatSpec {
    Left,
    Right,
    Literal(String),
}

fn parse_format_string(spec: &str) -> (Vec<FormatSpec>, usize) {
    use self::FormatSpec::*;

    let mut vec   = Vec::new();
    let mut count = 0;
    let mut buf   = String::new();

    let mut chars = spec.chars();

    while let Some(c) = chars.next() {
        if c == '%' {
            match chars.next() {
                Some('%') => buf.push('%'),

                Some('l') => {
                    if !buf.is_empty() {
                        vec.push(Literal(buf));
                        buf = String::new();
                    }
                    vec.push(Left);
                    count += 1;
                }

                Some('r') => {
                    if !buf.is_empty() {
                        vec.push(Literal(buf));
                        buf = String::new();
                    }
                    vec.push(Right);
                    count += 1;
                }

                Some(c) => panic!("parse_format_string: bad format spec ‘%{}’", c),

                None    => panic!("parse_format_string: string ends in single %"),
            }
        } else {
            buf.push(c);
        }
    }

    if !buf.is_empty() {
        vec.push(Literal(buf));
    }

    (vec, count)
}

pub struct TextTable {
    n_columns:     usize,
    format:        Vec<FormatSpec>,
    rows:          Vec<Row>,
    column_widths: Vec<usize>,
}

impl TextTable {
    pub fn new(format_string: &str) -> Self {
        let (format, n_columns) = parse_format_string(format_string);
        TextTable {
            n_columns,
            format,
            rows:           vec![],
            column_widths:  vec![0; n_columns]
        }
    }

    pub fn add_row(&mut self, row: Row) -> &mut Self {
        assert_eq!(row.0.len(), self.n_columns);

        for (width, s) in self.column_widths.iter_mut().zip(row.0.iter()) {
            *width = std::cmp::max(*width, s.len());
        }

        self.rows.push(row);
        self
    }
}

impl Display for TextTable {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use self::FormatSpec::*;

        for row in &self.rows {
            let mut fs_iter = self.format.iter();
            let mut cw_iter = self.column_widths.iter().cloned();
            let mut v_iter  = row.0.iter();

            while let Some(fs) = fs_iter.next() {
                match fs {
                    Left  => {
                        let cw = cw_iter.next().unwrap();
                        let v = match v_iter.next() {
                            Some(v) => v.to_owned(),
                            None    => "".to_owned(),
                        };
                        let len = cw - v.len();
                        f.write_str(&v)?;
                        for _ in 0 .. len {
                            f.write_str(" ")?;
                        }
                    }

                    Right => {
                        let cw = cw_iter.next().unwrap();
                        let v = match v_iter.next() {
                            Some(v) => v.to_owned(),
                            None    => "".to_owned(),
                        };
                        let len = cw - v.len();
                        for _ in 0 .. len {
                            f.write_str(" ")?;
                        }
                        f.write_str(&v)?;
                    }

                    Literal(s) => f.write_str(&s)?,
                }
            }
        }

        Ok(())
    }
}
