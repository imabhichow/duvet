use core::{
    fmt,
    ops::{self, Range},
};
use std::io::{self, BufRead};

#[derive(Clone, Copy, Debug)]
pub struct LinesIter<'a> {
    content: &'a str,
    line: usize,
    offset: usize,
}

impl<'a> LinesIter<'a> {
    pub fn new(content: &'a str) -> Self {
        Self {
            content,
            offset: 0,
            line: 1,
        }
    }
}

impl<'a> Iterator for LinesIter<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.content.len() {
            return None;
        }

        let content = &self.content[self.offset..];

        let value = if let Some(mut offset) = content.find('\n') {
            if content.as_bytes().get(offset + 1).copied() == Some(b'\r') {
                offset += 1;
            }
            &content[..=offset]
        } else {
            content
        };

        let value = Line {
            value,
            offset: Offset {
                offset: self.offset,
                line: self.line,
            },
        };

        self.offset += value.len();
        self.line += 1;
        Some(value)
    }
}

#[derive(Clone, Debug)]
pub struct Source {
    contents: String,
    lines: Vec<LineMap>,
}

impl Source {
    pub fn read<R: BufRead>(reader: &mut R) -> io::Result<Self> {
        let mut contents = String::new();
        let mut lines = vec![];

        loop {
            let offset = contents.len();
            let mut len = reader.read_line(&mut contents)?;

            // EOF
            if len == 0 {
                break;
            }

            let buf = reader.fill_buf()?;

            // handle carriage returns
            if buf.get(0).copied() == Some(b'\r') {
                contents.push('\r');
                reader.consume(1);
                len += 1;
            }

            lines.push(LineMap { offset, len });
        }

        Ok(Self { contents, lines })
    }

    pub fn line(&self, line: usize) -> Option<Line> {
        let map = self.get_line(line)?;

        let value = if cfg!(debug_assertions) {
            &self.contents[map.range()]
        } else {
            unsafe { self.contents.get_unchecked(map.range()) }
        };

        Some(Line {
            value,
            offset: Offset {
                offset: map.offset,
                line,
            },
        })
    }

    pub fn lincol_to_offset(&self, line: usize, column: usize) -> Option<Offset> {
        let offset = self.get_line(line)?.col_to_offset(column)?;
        Some(Offset { offset, line })
    }

    fn get_line(&self, line: usize) -> Option<&LineMap> {
        // lines start at 1
        let line = line.checked_sub(1)?;
        self.lines.get(line)
    }
}

/// Checked offset
pub struct Offset {
    offset: usize,
    /// Used to look the offset line number back up
    line: usize,
}

pub struct Line<'a> {
    value: &'a str,
    offset: Offset,
}

impl<'a> Line<'a> {
    pub fn offset(&self) -> usize {
        self.offset.offset
    }

    pub fn line(&self) -> usize {
        self.offset.line
    }
}

impl<'a> fmt::Debug for Line<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl<'a> fmt::Display for Line<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl<'a> ops::Deref for Line<'a> {
    type Target = str;

    fn deref(&self) -> &str {
        self.value
    }
}

#[derive(Clone, Copy, Debug)]
struct LineMap {
    offset: usize,
    len: usize,
}

impl LineMap {
    fn range(&self) -> Range<usize> {
        self.offset..(self.offset + self.len as usize)
    }

    fn col_to_offset(&self, column: usize) -> Option<usize> {
        if (self.len as usize) > column {
            Some(self.offset + column)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    static CONTENTS: &str = include_str!("./linemap.rs");

    fn read_self() -> Source {
        Source::read(&mut Cursor::new(CONTENTS)).unwrap()
    }

    #[test]
    fn parse_self_test() {
        let source = read_self();

        bolero::check!()
            .with_generator((0..(source.lines.len() * 2), 0..2000))
            .cloned()
            .for_each(|(line, column)| {
                let l = source.line(line);
                let offset = source.lincol_to_offset(line, column);
                if offset.is_some() {
                    assert!(l.is_some());
                }
            });
    }

    #[test]
    fn sanity_checks() {
        let source = read_self();

        assert_eq!(source.line(1).as_deref(), Some("use core::{\n"));
    }
}
