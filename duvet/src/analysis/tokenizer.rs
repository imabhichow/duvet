use duvet_core::{
    diagnostics,
    fs::{ArcStr, Node, Substr},
    mapper,
};
use std::{ops::Deref, path::Path};

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

#[derive(Debug, PartialEq, Eq, Default)]
pub struct Tokens(Vec<Token>);

impl Deref for Tokens {
    type Target = [Token];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Analysis {
    pub patterns: Vec<mapper::Glob>,
    pub meta_prefix: String,
    pub content_prefix: String,
}

impl Default for Analysis {
    fn default() -> Self {
        Self {
            patterns: vec![],
            meta_prefix: "//=".to_string(),
            content_prefix: "//#".to_string(),
        }
    }
}

impl Analysis {
    fn parse(&self, contents: &ArcStr) -> Tokens {
        let mut parser = Parser::new(&self.meta_prefix, &self.content_prefix, contents);
        for (lineno, line) in contents.lines().enumerate() {
            parser.on_line(lineno, line);
        }
        let refs = parser.finish();
        Tokens(refs)
    }
}

impl mapper::Analyze for Analysis {
    type Reducers = ();
    type Mappers = ();
    type Output = Tokens;

    fn patterns(&self) -> Vec<mapper::Glob> {
        self.patterns.clone()
    }

    fn analyze(
        &self,
        _mappers: (),
        _reducers: (),
        _path: &Path,
        node: Node,
    ) -> (Option<Tokens>, diagnostics::List) {
        match node.as_str() {
            Ok(contents) => (Some(self.parse(contents)), Default::default()),
            Err(_err) => {
                // TODO emit error
                (None, Default::default())
            }
        }
    }
}

struct Parser<'a> {
    contents: &'a ArcStr,
    meta_prefix: &'a str,
    content_prefix: &'a str,
    references: Vec<Token>,
}

impl<'a> Parser<'a> {
    fn new(meta_prefix: &'a str, content_prefix: &'a str, contents: &'a ArcStr) -> Self {
        Self {
            contents,
            meta_prefix,
            content_prefix,
            references: vec![],
        }
    }

    fn on_line(&mut self, lineno: usize, line: &str) {
        let total_len = line.len();
        let line = line.trim_start();
        if line.is_empty() {
            return;
        }

        let indent = total_len - line.len();

        let location = Location::new(lineno, indent);

        if let Some(meta) = line.strip_prefix(&self.meta_prefix) {
            self.on_meta(meta, location);
            return;
        }

        if let Some(content) = line.strip_prefix(&self.content_prefix) {
            self.on_content(content, location);
            return;
        }
    }

    fn on_content(&mut self, content: &str, location: Location) {
        let value = self.contents.substr_from(content);
        self.references.push(Token::Content { location, value });
    }

    fn on_meta(&mut self, meta: &str, location: Location) {
        let mut parts = meta.trim_start().splitn(2, '=');

        let key = parts.next().unwrap();
        let key = key.trim_end();
        let key = self.contents.substr_from(key);

        if let Some(value) = parts.next() {
            let value = value.trim_start();
            let value = self.contents.substr_from(value);
            self.references.push(Token::Meta {
                key,
                value,
                location,
            })
        } else {
            self.references.push(Token::UnnamedMeta {
                value: key,
                location,
            });
        }
    }

    fn finish(self) -> Vec<Token> {
        self.references
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
                Analysis {
                    patterns: vec![],
                    meta_prefix: "//=".to_string(),
                    content_prefix: "//#".to_string(),
                }
            );
        };
        ($name:ident, $input:expr, $config:expr) => {
            #[test]
            fn $name() {
                let parser = $config;
                insta::assert_debug_snapshot!(
                    stringify!($name),
                    parser.parse(&arcstr::literal!($input))
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
        Analysis {
            meta_prefix: "*=".to_string(),
            content_prefix: "*#".to_string(),
            ..Default::default()
        }
    );
}
