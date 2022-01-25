use crate::citation::types::{Type, TypeSet, Types};
use arcstr::{ArcStr, Substr};
use std::{collections::VecDeque, sync::Arc};

#[derive(Clone, Debug)]
pub enum Tree {
    Type(Type),
    Any(Arc<[Tree]>),
    All(Arc<[Tree]>),
    Xor(Arc<[Tree]>),
    Not(Arc<Tree>),
}

impl Tree {
    pub fn query<Q: Query>(&self, mut query: Q) -> Q::Value {
        enum Event<'a> {
            Enter(&'a Tree),
            Exit(&'a Tree),
        }

        let mut stack = VecDeque::new();
        let mut values = vec![];
        stack.push_back(Event::Enter(self));

        while let Some(event) = stack.pop_front() {
            match event {
                Event::Enter(tree) => match tree {
                    Tree::Type(ty) => values.push(query.eval(*ty)),
                    Tree::Any(args) | Tree::All(args) | Tree::Xor(args) => {
                        stack.push_front(Event::Exit(tree));
                        for arg in args.iter().rev() {
                            stack.push_front(Event::Enter(arg));
                        }
                    }
                    Tree::Not(arg) => {
                        stack.push_front(Event::Exit(tree));
                        stack.push_front(Event::Enter(arg));
                    }
                },
                Event::Exit(tree) => {
                    macro_rules! call {
                        ($name:ident, $args:expr) => {{
                            let index = values.len() - $args.len();
                            let value = query.$name(&values[index..]);
                            let _ = values.drain(index..);
                            values.push(value);
                        }};
                    }
                    match tree {
                        Tree::Type(_) => unreachable!(),
                        Tree::Any(args) => call!(any, args),
                        Tree::All(args) => call!(all, args),
                        Tree::Xor(args) => call!(xor, args),
                        Tree::Not(_) => {
                            let arg = values.pop().expect("invalid stack state");
                            let value = query.not(arg);
                            values.push(value);
                        }
                    }
                }
            }
        }

        debug_assert_eq!(values.len(), 1);

        values.pop().expect("invalid stack state")
    }
}

pub trait Query {
    type Value;

    fn eval(&mut self, ty: Type) -> Self::Value;
    fn all(&mut self, args: &[Self::Value]) -> Self::Value;
    fn any(&mut self, args: &[Self::Value]) -> Self::Value;
    fn xor(&mut self, args: &[Self::Value]) -> Self::Value;
    fn not(&mut self, arg: Self::Value) -> Self::Value;
}

impl Query for &'_ TypeSet {
    type Value = bool;

    fn eval(&mut self, ty: Type) -> Self::Value {
        self.get(ty)
    }

    fn all(&mut self, args: &[Self::Value]) -> Self::Value {
        args.iter().all(|v| *v)
    }

    fn any(&mut self, args: &[Self::Value]) -> Self::Value {
        args.iter().any(|v| *v)
    }

    fn xor(&mut self, args: &[Self::Value]) -> Self::Value {
        let mut value = false;
        for arg in args.iter().copied() {
            if arg && value {
                return false;
            } else if arg {
                value = true;
            }
        }
        value
    }

    fn not(&mut self, arg: Self::Value) -> Self::Value {
        !arg
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Op {
    Any,
    All,
    Xor,
    Not,
}

impl Op {
    fn close_parse(&self, mut args: Vec<Tree>) -> Tree {
        match self {
            Self::All => {
                assert!(!args.is_empty());
                if args.len() == 1 {
                    return args.pop().unwrap();
                }
                Tree::All(Arc::from(args))
            }
            Self::Any => {
                assert!(!args.is_empty());
                if args.len() == 1 {
                    return args.pop().unwrap();
                }
                Tree::Any(Arc::from(args))
            }
            Self::Xor => {
                assert!(!args.is_empty());
                if args.len() == 1 {
                    return args.pop().unwrap();
                }
                Tree::Xor(Arc::from(args))
            }
            Self::Not => {
                assert!(args.len() == 1);
                Tree::Not(Arc::new(args.pop().unwrap()))
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum Token {
    Any(Substr),
    All(Substr),
    Xor(Substr),
    Not(Substr),
    OpenParen(Substr),
    CloseParen(Substr),
    Type(Substr),
}

impl Token {
    pub fn iter(content: &ArcStr) -> TokenIter {
        TokenIter {
            content,
            cursor: &*content,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TokenIter<'a> {
    content: &'a ArcStr,
    cursor: &'a str,
}

impl TokenIter<'_> {
    pub fn types(&self) -> Types {
        self.clone()
            .filter_map(|token| {
                if let Token::Type(ty) = token {
                    Some(ty)
                } else {
                    None
                }
            })
            .collect()
    }
}

impl<'a> Iterator for TokenIter<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        let cursor = self.cursor.trim_start();

        if cursor.is_empty() {
            self.cursor = cursor;
            return None;
        }

        macro_rules! paren {
            ($pat:literal, $name:ident) => {
                if let Some(other) = cursor.strip_prefix($pat) {
                    self.cursor = other;
                    let token = &cursor[..1];
                    let token = self.content.substr_from(token);
                    let token = Token::$name(token);
                    return Some(token);
                }
            };
        }

        paren!("(", OpenParen);
        paren!(")", CloseParen);

        macro_rules! call {
            ($pat:ident, $name:ident) => {
                if let Some(other) = cursor.strip_prefix(stringify!($pat)) {
                    self.cursor = other;
                    let token = &cursor[..stringify!($pat).len()];
                    let token = self.content.substr_from(token);
                    let token = Token::$name(token);
                    return Some(token);
                }
            };
        }

        call!(ANY, Any);
        call!(ALL, All);
        call!(XOR, Xor);
        call!(NOT, Not);

        for (idx, ch) in cursor.char_indices() {
            if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                continue;
            }

            if idx == 0 {
                todo!("invalid {:?}", ch);
            }

            let (token, cursor) = cursor.split_at(idx);
            self.cursor = cursor;
            let token = self.content.substr_from(token);
            let token = Token::Type(token);
            return Some(token);
        }

        let token = cursor;
        self.cursor = &cursor[..0];
        let token = self.content.substr_from(token);
        let token = Token::Type(token);
        Some(token)
    }
}

fn parse(mut tokens: TokenIter, types: &Types) -> Result<Tree, Token> {
    let mut state = State::new();

    #[derive(Debug)]
    struct State {
        ty: Op,
        args: Vec<Tree>,
        stack: Vec<(Op, Vec<Tree>)>,
    }

    impl State {
        fn new() -> Self {
            Self {
                ty: Op::All,
                args: vec![],
                stack: vec![],
            }
        }

        fn push(&mut self, arg: Tree) {
            self.args.push(arg);
        }

        fn call(&mut self, ty: Op) {
            let prev_type = core::mem::replace(&mut self.ty, ty);
            let prev_args = core::mem::take(&mut self.args);
            self.stack.push((prev_type, prev_args));
        }

        fn open(&mut self) {
            // TODO
        }

        fn close(&mut self) {
            let (prev_type, prev_args) = self.stack.pop().unwrap();
            let current_type = core::mem::replace(&mut self.ty, prev_type);
            let current_args = core::mem::replace(&mut self.args, prev_args);
            self.args.push(current_type.close_parse(current_args));
        }

        fn finish(self) -> Tree {
            self.ty.close_parse(self.args)
        }
    }

    for token in tokens {
        match token {
            Token::Any(_) => state.call(Op::Any),
            Token::All(_) => state.call(Op::All),
            Token::Xor(_) => state.call(Op::Xor),
            Token::Not(_) => state.call(Op::Not),
            Token::OpenParen(_) => {
                state.open();
            }
            Token::CloseParen(_) => {
                state.close();
            }
            Token::Type(ty) => {
                let ty = types.resolve(&ty).unwrap();
                state.push(Tree::Type(ty));
            }
        }
    }

    Ok(state.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use arcstr::literal;

    fn tokenize(input: &'static str) -> Vec<Token> {
        Token::iter(&ArcStr::from(input)).collect()
    }

    fn parse(input: &'static str) -> Result<Tree, Token> {
        let input = ArcStr::from(input);
        let iter = Token::iter(&input);
        let types = iter.types();
        super::parse(iter, &types)
    }

    fn eval(expr: ArcStr, sets: &[(&[&'static str], bool)]) {
        let iter = Token::iter(&expr);
        let types = iter.types();
        let tree = super::parse(iter, &types).unwrap();

        for (set, result) in sets {
            let set: TypeSet = set
                .iter()
                .map(|item| types.resolve(item).expect("missing type"))
                .collect();

            assert_eq!(tree.query(&set), *result, "set = {:?}", set);
        }
    }

    macro_rules! test {
        ($name:ident, $input:expr, [$(($set:expr, $result:expr)),+ $(,)?]) => {
            #[test]
            fn $name() {
                insta::assert_debug_snapshot!(
                    concat!(stringify!($name), "__tokens"),
                    tokenize($input)
                );
                insta::assert_debug_snapshot!(
                    concat!(stringify!($name), "__tree"),
                    parse($input)
                );

                eval(literal!($input), &[
                    $(
                        (
                            &$set[..],
                            $result
                        ),
                    )+
                ][..]);
            }
        };
    }

    test!(simple, "citation", [([], false), (["citation"], true)]);
    test!(
        any,
        "ANY(citation test)",
        [
            ([], false),
            (["citation"], true),
            (["test"], true),
            (["citation", "test"], true),
        ]
    );
    test!(
        all,
        "ALL(citation test)",
        [
            ([], false),
            (["citation"], false),
            (["test"], false),
            (["citation", "test"], true),
        ]
    );
    test!(
        any_all,
        "ANY(ALL(citation test) exception)",
        [
            ([], false),
            (["citation"], false),
            (["test"], false),
            (["exception"], true),
            (["citation", "test"], true),
            (["citation", "test", "exception"], true),
        ]
    );
    test!(
        xor,
        "XOR(citation exception)",
        [
            ([], false),
            (["citation"], true),
            (["exception"], true),
            (["citation", "exception"], false),
        ]
    );
    test!(
        xor_any,
        "XOR(ANY(citation test) exception)",
        [
            ([], false),
            (["citation"], true),
            (["citation", "test"], true),
            (["exception"], true),
            (["citation", "exception"], false),
            (["test", "exception"], false),
        ]
    );
    test!(not, "NOT(citation)", [([], true), (["citation"], false)]);
}
