use crate::{
    db::Db,
    notification,
    schema::FileId,
    source::{Line, LinesIter},
};
use anyhow::Result;
use core::fmt;
use std::{
    collections::HashSet,
    fs,
    io::{BufWriter, Write},
    path::PathBuf,
};
use v_htmlescape::escape as htmlescape;

pub struct Config {
    pub outdir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            outdir: PathBuf::from("target/cargo-duvet/report"),
        }
    }
}

const TEMPLATE: &str = include_str!("./html/template.html");

fn template() -> (&'static str, &'static str) {
    let mut iter = TEMPLATE.split("CONTENT");
    let header = iter.next().unwrap();
    let footer = iter.next().unwrap();
    (header, footer)
}

impl Config {
    pub fn file(&self, db: &Db, file: FileId) -> Result<()> {
        let contents = db.fs().open(file)?;
        let path = db.fs().path(file)?;
        let mut notifications = db.notifications().for_file(file).peekable();

        // don't include anything missing notifications
        if notifications.peek().is_none() {
            return Ok(());
        }

        let mut used: HashSet<notification::Id> = Default::default();

        let mut path = self.outdir.join(&*path);
        let ext = path
            .extension()
            .map(|ext| format!("{}.html", ext.to_str().unwrap()))
            .unwrap_or_else(|| "html".to_string());
        path.set_extension(ext);

        std::fs::create_dir_all(path.parent().unwrap())?;

        let out = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        let mut out = BufWriter::new(out);

        let (header, footer) = template();

        let title = "TODO";
        write!(out, "{}", header.replace("TITLE", title))?;

        write!(out, "<div class=source>")?;
        for line in LinesIter::new(&contents) {
            write!(out, "<div id=L{}>", line.line())?;

            let line = line.trim_end();
            if line.is_empty() {
                write!(out, "<br/></div>")?;
                continue;
            }

            let content = line.trim_start();
            let whitespace = line.len() - content.len();
            if whitespace > 0 {
                write!(out, "{}", &line[..whitespace])?;
            }

            line_regions(content, &mut notifications, |region| {
                used.extend(region.ids);
                if let Some(notifications) = region.notifications(db) {
                    write!(
                        out,
                        "<span data-n={}>{}</span>",
                        notifications,
                        htmlescape(region.content)
                    )?;
                } else {
                    write!(out, "{}", htmlescape(region.content))?;
                }
                Ok(())
            })?;
            write!(out, "</div>")?;
        }
        write!(out, "</div>")?;

        write!(out, "<div class=notifications>")?;
        for id in used {
            let (level, notification) = db.notifications().get(id);
            write!(out, "<template id=n{}><div class={}>", id, level_id(level))?;
            notification.html(&mut out)?;
            write!(out, "</div></template>")?;
        }
        write!(out, "</div>")?;

        write!(out, "{}", footer)?;

        out.flush()?;
        drop(out);

        Ok(())
    }
}

fn line_regions<F: FnMut(Region) -> Result<()>>(
    mut line: Line,
    notifications: &mut core::iter::Peekable<notification::Iter>,
    mut f: F,
) -> Result<()> {
    use core::cmp::Ordering::*;

    while let Some(not) = notifications.peek() {
        if line.is_empty() {
            return Ok(());
        }

        let not = not.as_ref().unwrap();
        let line_range = line.range();
        let n_range = not.range();
        match (
            line_range.start.cmp(&n_range.start),
            line_range.end.cmp(&n_range.end),
        ) {
            // notifications don't come until later
            (Less, Less) => {
                break;
            }
            // notifications have passed
            (Greater, Greater) => {
                notifications.next().unwrap()?;
            }
            // notification comes later in the line
            (Less, Equal) | (Less, Greater) => {
                let end = line_range.end.min(n_range.start);

                f(Region {
                    content: &line[0..(end - line.offset()) as usize],
                    ids: &[],
                })?;

                let (_, l) = line.split_at_offset(end);
                line = l;
            }
            // overlap
            (Greater, Less) | (Equal, Less) => {
                let end = line_range.end.min(n_range.end);

                f(Region {
                    content: &line[0..(end - line.offset()) as usize],
                    ids: not.ids(),
                })?;

                let (_, l) = line.split_at_offset(end);
                line = l;
            }
            // overlap and next
            (_, Equal) | (_, Greater) => {
                let end = line_range.end.min(n_range.end);

                f(Region {
                    content: &line[0..(end - line.offset()) as usize],
                    ids: not.ids(),
                })?;

                let (_, l) = line.split_at_offset(end);
                line = l;
                notifications.next().unwrap()?;
            }
        }
    }

    if !line.is_empty() {
        f(Region {
            content: &*line,
            ids: &[],
        })?;
    }

    Ok(())
}

struct Region<'a> {
    pub content: &'a str,
    pub ids: &'a [notification::Id],
}

impl<'a> Region<'a> {
    fn notifications(&self, db: &Db) -> Option<NotificationList<'a>> {
        let notifications = db.notifications();
        let ids = self.ids;
        let level = ids
            .iter()
            .copied()
            .map(|id| notifications.get(id).0)
            .max()?;

        Some(NotificationList { level, ids })
    }
}

struct NotificationList<'a> {
    level: notification::Level,
    ids: &'a [notification::Id],
}

impl<'a> fmt::Display for NotificationList<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut iter = self.ids.iter();

        if let Some(id) = iter.next() {
            let level = level_id(self.level);
            write!(f, "{}-{}", level, id)?;
        }

        for id in iter {
            write!(f, ",{}", id)?;
        }

        Ok(())
    }
}

fn level_id(level: notification::Level) -> &'static str {
    match level {
        notification::Level::Fatal => "f",
        notification::Level::Error => "e",
        notification::Level::Warning => "w",
        notification::Level::Success => "s",
        notification::Level::Info => "i",
    }
}
