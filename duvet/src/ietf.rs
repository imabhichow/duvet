use arcstr::{ArcStr, Substr};
use core::fmt;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref SECTION_HEADER_RE: Regex = Regex::new(r"^(([A-Z]\.)?[0-9\.]+)\s+(.*)").unwrap();
    static ref APPENDIX_HEADER_RE: Regex = Regex::new(r"^Appendix ([A-Z]\.)\s+(.*)").unwrap();
}

struct Tokenizer<'a> {
    lines: core::str::Lines<'a>,
    contents: &'a ArcStr,
    lineno: usize,
}

impl<'a> Tokenizer<'a> {
    fn new(contents: &'a ArcStr) -> Self {
        Self {
            lines: contents.lines(),
            contents,
            lineno: 0,
        }
    }

    fn on_line(&mut self, line: &'a str) -> Option<Token> {
        let lineno = self.lineno;
        self.lineno += 1;

        if line.is_empty() {
            return Some(Token::Break { line: lineno });
        }

        if let Some(section) = SECTION_HEADER_RE.captures(line) {
            let id = section.get(1)?;
            let id = &line[id.range()].trim_end_matches('.');
            let id = self.contents.substr_from(id);

            let title = section.get(3)?;
            let title = &line[title.range()].trim();
            let title = self.contents.substr_from(title);

            return Some(Token::Section {
                id,
                title,
                line: lineno,
            });
        }

        if let Some(section) = APPENDIX_HEADER_RE.captures(line) {
            let id = section.get(1)?;
            let id = &line[id.range()].trim_end_matches('.');
            let id = self.contents.substr_from(id);

            let title = section.get(2)?;
            let title = &line[title.range()].trim();
            let title = self.contents.substr_from(title);

            return Some(Token::Section {
                id,
                title,
                line: lineno,
            });
        }

        let trimmed = line.trim_start();

        if trimmed.is_empty() {
            return Some(Token::Break { line: lineno });
        }

        if trimmed.len() != line.len() {
            return Some(Token::Content {
                value: self.contents.substr_from(line),
                line: lineno,
            });
        }

        None
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let line = self.lines.next()?;
            if let Some(token) = self.on_line(line) {
                return Some(token);
            }
        }
    }
}

#[derive(Clone)]
pub enum Token {
    Section {
        id: Substr,
        title: Substr,
        line: usize,
    },
    Break {
        line: usize,
    },
    Content {
        value: Substr,
        line: usize,
    },
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Section { id, title, line } => {
                write!(f, "SECTION#{}(id={}, title={})", line, id, title)
            }
            Self::Break { line } => write!(f, "  BREAK#{}", line),
            Self::Content { value, line } => write!(f, "CONTENT#{}({})", line, value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test {
        ($rfc:ident) => {
            #[test]
            fn $rfc() {
                let $rfc = ArcStr::from(include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../etc/",
                    stringify!($rfc),
                    ".txt"
                )));

                insta::assert_debug_snapshot!(
                    stringify!($rfc),
                    Tokenizer::new(&$rfc).collect::<Vec<_>>()
                );
            }
        };
    }

    test!(rfc2616);
    test!(rfc8446);
    test!(rfc9000);
    test!(rfc9001);
}
