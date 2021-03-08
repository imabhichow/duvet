use crate::{attribute::Value, db::Db, fs, schema::FileId, source::LinesIter};
use anyhow::Result;
use core::mem::size_of;
use sled::IVec;
use std::path::Path;
use syntect::parsing::{ParseState, Scope, ScopeStack, SyntaxReference, SyntaxSet};

lazy_static::lazy_static! {
    static ref SYNTAX: SyntaxSet = SyntaxSet::load_defaults_newlines();
}

pub fn highlight_all(db: &Db) -> Iter {
    highlight_all_with_syntax(db, &SYNTAX)
}

pub fn highlight_all_with_syntax<'a>(db: &'a Db, set: &'a SyntaxSet) -> Iter<'a> {
    Iter {
        db,
        fs: db.fs().iter(),
        set,
    }
}

pub struct Iter<'a> {
    db: &'a Db,
    fs: fs::Iter,
    set: &'a SyntaxSet,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Result<()>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(match self.fs.next()? {
            Ok((id, filename)) => highlight_file(self.db, self.set, id, &filename),
            Err(err) => Err(err),
        })
    }
}

pub fn highlight_file(db: &Db, set: &SyntaxSet, file: FileId, filename: &str) -> Result<()> {
    let content = db.fs().open(file)?;
    let lines = LinesIter::new(&content);
    let syntax = if let Some(s) = get_syntax(set, filename, &content) {
        s
    } else {
        return Ok(());
    };
    let mut stack = ScopeStack::new();
    let mut state = ParseState::new(syntax);

    for line in lines {
        let line_offset = line.offset();
        let mut start = line_offset;
        let ops = state.parse_line(&line, set);

        let mut idx = 0;
        while let Some((offset, op)) = ops.get(idx) {
            stack.apply(&op);
            idx += 1;

            // peek the next ops and check if they have the
            // same offset
            while let Some((next_offset, op)) = ops.get(idx) {
                if offset != next_offset {
                    break;
                }
                stack.apply(&op);
                idx += 1;
            }

            let scopes = stack.as_slice();
            if scopes.is_empty() {
                start = line_offset + *offset as u32;
                continue;
            }
            // TODO match scopes to generic theme that can be swapped out

            //let entity = db.entities().create()?;
            //db.entities().set_attribute(entity, &SCOPE, scopes)?;

            //let bytes = start..line_offset + (*offset as u32);

            //db.regions().insert(file, entity, bytes)?;

            start = line_offset + *offset as u32;
        }
    }

    Ok(())
}

fn get_syntax<'a>(set: &'a SyntaxSet, path: &str, content: &str) -> Option<&'a SyntaxReference> {
    let path = Path::new(path);
    let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
    if let Some(syntax) = set.find_syntax_by_extension(filename) {
        return Some(syntax);
    }

    let extension = path.extension().and_then(|f| f.to_str()).unwrap_or("");
    if let Some(syntax) = set.find_syntax_by_extension(extension) {
        return Some(syntax);
    }

    let line = LinesIter::new(content).next()?;
    set.find_syntax_by_first_line(&line)
}
