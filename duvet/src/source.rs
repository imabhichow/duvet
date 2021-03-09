use byteorder::BigEndian as BE;
use core::{
    fmt,
    ops::{self, Range},
};
use std::io::{self, BufRead};
use zerocopy::{AsBytes, FromBytes, Unaligned, U32};

pub struct Loader<'a, R> {
    pub contents: String,
    reader: &'a mut R,
}

impl<'a, R: BufRead> Loader<'a, R> {
    pub fn new(reader: &'a mut R) -> Self {
        Self {
            reader,
            contents: String::new(),
        }
    }
}

impl<'a, R: BufRead> Iterator for Loader<'a, R> {
    type Item = std::io::Result<LineInfo>;

    fn next(&mut self) -> Option<Self::Item> {
        macro_rules! unwrap {
            ($expr:expr) => {
                match $expr {
                    Ok(v) => v,
                    Err(err) => return Some(Err(err)),
                }
            };
        }

        let offset = self.contents.len();
        let mut len = unwrap!(self.reader.read_line(&mut self.contents));

        // EOF
        if len == 0 {
            return None;
        }

        let buf = unwrap!(self.reader.fill_buf());

        // handle carriage returns
        if buf.get(0).copied() == Some(b'\r') {
            self.contents.push('\r');
            self.reader.consume(1);
            len += 1;
        }

        Some(Ok(LineInfo {
            offset: U32::new(offset as _),
            len: U32::new(len as _),
        }))
    }
}

#[derive(Clone, Copy, Debug, AsBytes, FromBytes, Unaligned)]
#[repr(C)]
pub struct LineInfo {
    pub(crate) offset: U32<BE>,
    pub(crate) len: U32<BE>,
}

impl LineInfo {
    pub fn offset(self) -> u32 {
        self.offset.get()
    }

    pub fn len(self) -> u32 {
        self.len.get()
    }

    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    fn range(&self) -> Range<u32> {
        let offset = self.offset();
        let len = self.len();
        offset..(offset + len)
    }

    pub fn range_usize(&self) -> Range<usize> {
        let offset = self.offset() as usize;
        let len = self.len() as usize;
        offset..(offset + len)
    }
}

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
                offset: self.offset as _,
                line: self.line as _,
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
    lines: Vec<LineInfo>,
}

impl Source {
    pub fn read<R: BufRead>(reader: &mut R) -> io::Result<Self> {
        let mut loader = Loader::new(reader);
        let mut lines = vec![];

        while let Some(line) = loader.next() {
            lines.push(line?);
        }

        Ok(Self {
            contents: loader.contents,
            lines,
        })
    }

    pub fn line(&self, line: u32) -> Option<Line> {
        let map = self.get_line(line)?;

        let value = if cfg!(debug_assertions) {
            &self.contents[map.range_usize()]
        } else {
            unsafe { self.contents.get_unchecked(map.range_usize()) }
        };

        Some(Line {
            value,
            offset: Offset {
                offset: map.offset(),
                line,
            },
        })
    }

    fn get_line(&self, line: u32) -> Option<&LineInfo> {
        // lines start at 1
        let line = line.checked_sub(1)?;
        self.lines.get(line as usize)
    }
}

/// Checked offset
#[derive(Clone, Copy, Debug)]
pub struct Offset {
    offset: u32,
    /// Used to look the offset line number back up
    line: u32,
}

pub struct Line<'a> {
    value: &'a str,
    offset: Offset,
}

impl<'a> Line<'a> {
    pub fn offset(&self) -> u32 {
        self.offset.offset
    }

    pub fn range(&self) -> core::ops::Range<u32> {
        let offset = self.offset();
        offset..offset + self.value.len() as u32
    }

    pub fn line(&self) -> u32 {
        self.offset.line
    }

    pub fn trim_end(&self) -> Self {
        Self {
            value: self.value.trim_end_matches('\r').trim_end_matches('\n'),
            offset: self.offset,
        }
    }

    pub fn split_at_offset(&self, offset: u32) -> (Self, Self) {
        let line = self.offset.line;
        let start = self.offset();
        let index = offset - start;
        let (a, b) = self.value.split_at(index as usize);
        let a = Self {
            value: a,
            offset: Offset {
                offset: start,
                line,
            },
        };
        let b = Self {
            value: b,
            offset: Offset { offset, line },
        };
        (a, b)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    static CONTENTS: &str = include_str!("./source.rs");

    fn read_self() -> Source {
        Source::read(&mut Cursor::new(CONTENTS)).unwrap()
    }

    #[test]
    fn parse_self_test() {
        let source = read_self();

        bolero::check!()
            .with_generator((0..(source.lines.len() as u32 * 2), 0..2000))
            .cloned()
            .for_each(|(line, column)| {
                let _ = source.line(line);
            });
    }

    #[test]
    fn sanity_checks() {
        let source = read_self();

        assert_eq!(
            source.line(1).as_deref(),
            Some("use byteorder::BigEndian as BE;\n")
        );
    }
}
