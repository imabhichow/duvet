use duvet_core::fs::{ArcStr, Substr};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    Meta {
        location: Location,
        key: Substr,
        value: Substr,
    },
    UnnamedMeta {
        location: Location,
        value: Substr,
    },
    Content {
        location: Location,
        value: Substr,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Location {
    pub line: u32,
    pub indent: u32,
}

impl Location {
    pub fn new(line: usize, indent: usize) -> Self {
        debug_assert!(line < u32::MAX as usize);
        debug_assert!(indent < u32::MAX as usize);
        Self {
            line: line as _,
            indent: indent as _,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Tokenizer<'a> {
    pub meta_prefix: &'a str,
    pub content_prefix: &'a str,
}

impl<'a> Tokenizer<'a> {
    pub fn tokenize(self, contents: &'a ArcStr) -> Iter {
        Iter::new(self, contents)
    }
}

pub struct Iter<'a> {
    tokenizer: Tokenizer<'a>,
    lines: core::str::Lines<'a>,
    contents: &'a ArcStr,
    lineno: usize,
}

impl<'a> Iter<'a> {
    fn new(tokenizer: Tokenizer<'a>, contents: &'a ArcStr) -> Self {
        Self {
            tokenizer,
            contents,
            lines: contents.lines(),
            lineno: 0,
        }
    }

    fn on_line(&mut self, line: &str) -> Option<Token> {
        self.lineno += 1;

        let total_len = line.len();
        let line = line.trim_start();
        if line.is_empty() {
            return None;
        }

        let indent = total_len - line.len();

        let location = Location::new(self.lineno, indent);

        if let Some(meta) = line.strip_prefix(&self.tokenizer.meta_prefix) {
            return Some(self.on_meta(meta, location));
        }

        if let Some(content) = line.strip_prefix(&self.tokenizer.content_prefix) {
            return Some(self.on_content(content, location));
        }

        None
    }

    fn on_content(&mut self, content: &str, location: Location) -> Token {
        let value = self.contents.substr_from(content);
        Token::Content { location, value }
    }

    fn on_meta(&mut self, meta: &str, location: Location) -> Token {
        let mut parts = meta.trim_start().splitn(2, '=');

        let key = parts.next().unwrap();
        let key = key.trim_end();
        let key = self.contents.substr_from(key);

        if let Some(value) = parts.next() {
            let value = value.trim_start();
            let value = self.contents.substr_from(value);
            Token::Meta {
                key,
                value,
                location,
            }
        } else {
            Token::UnnamedMeta {
                value: key,
                location,
            }
        }
    }
}

impl<'a> Iterator for Iter<'a> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use duvet_core::fs::arcstr;

    macro_rules! snapshot_test {
        ($name:ident, $input:expr) => {
            snapshot_test!(
                $name,
                $input,
                Tokenizer {
                    meta_prefix: "//=",
                    content_prefix: "//#",
                }
            );
        };
        ($name:ident, $input:expr, $config:expr) => {
            #[test]
            fn $name() {
                let parser = $config;
                insta::assert_debug_snapshot!(
                    stringify!($name),
                    parser
                        .tokenize(&arcstr::literal!($input))
                        .collect::<Vec<_>>()
                );
            }
        };
    }

    snapshot_test!(empty, "");
    snapshot_test!(
        basic,
        r#"
        //= thing goes here
        //= meta=foo
        //= meta2 = bar
        //# content goes
        //# here
        "#
    );
    snapshot_test!(
        only_unnamed,
        r#"
        //= this is meta
        //= this is other meta
        "#
    );
    snapshot_test!(
        duplicate_meta,
        r#"
        //= meta=1
        //= meta=2
        "#
    );
    snapshot_test!(
        configured,
        r#"
        /*
         *= meta=goes here
         *# content goes here
         */
        "#,
        Tokenizer {
            meta_prefix: "*=",
            content_prefix: "*#",
        }
    );
}
