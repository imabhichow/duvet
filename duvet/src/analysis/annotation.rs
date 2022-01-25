use crate::analysis::tokenizer::{Location, Token, Tokens};
use duvet_core::{
    diagnostics,
    fs::{ArcStr, Node, Substr},
    mapper,
};
use std::{ops::Deref, path::Path};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Citation {
    document: ArcStr,
    ty: citation_type::Id,
}

#[derive(Debug, PartialEq, Eq, Default)]
pub struct Citations(Vec<Citation>);

impl Deref for Citations {
    type Target = [Citation];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Analysis {
    pub patterns: Vec<mapper::Glob>,
    pub default_type: citation_type::Id,
}

impl Default for Analysis {
    fn default() -> Self {
        todo!()
        // Self { patterns: vec![] }
    }
}

impl Analysis {
    fn parse(&self, tokens: &[Token]) -> Citations {
        let mut parser = Parser::default();
        for token in tokens {
            parser.on_token(token);
        }
        Citations(parser.finish())
    }
}

impl mapper::Analyze for Analysis {
    type Mappers = (mapper::Dep<Tokens>,);
    type Reducers = ();
    type Output = Tokens;

    fn patterns(&self) -> Vec<mapper::Glob> {
        self.patterns.clone()
    }

    fn analyze(
        &self,
        (tokens,): (mapper::Dep<Tokens>,),
        _reducers: (),
        _path: &Path,
        _node: Node,
    ) -> (Option<Tokens>, diagnostics::List) {
        self.parse(&tokens);
        todo!()
    }
}

#[derive(Default)]
struct Parser {
    citation: Option<Citation>,
    citations: Vec<Citation>,
}

impl Parser {
    fn on_token(&mut self, token: &Token) {
        todo!()
    }

    fn flush(&mut self) {
        if let Some(citation) = self.citation.take() {
            self.citations.push(citation);
        }
    }

    fn finish(mut self) -> Vec<Citation> {
        self.flush();
        self.citations
    }
}
