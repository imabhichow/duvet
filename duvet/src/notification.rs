use crate::{
    marker::Markers,
    schema::{FileId, IdSet, IdSetExt, NotificationId},
};
use anyhow::Result;
use core::ops::Range;
use sled::Tree;
use std::{
    io,
    sync::{Arc, Mutex},
};
use v_htmlescape::escape as htmlescape;
use v_jsonescape::escape as jsonescape;

attribute!(pub const NOTIFICATION: NotificationId);

pub type Id = NotificationId;
pub type Ref = Arc<dyn Notification>;
pub type Entry = (Level, Ref);

pub trait Notification: 'static + Send + Sync {
    fn html(&self, out: &mut dyn io::Write) -> io::Result<()>;
    fn json(&self, out: &mut dyn io::Write) -> io::Result<()>;
    fn tty(&self, out: &mut dyn io::Write) -> io::Result<()>;
    fn text(&self, out: &mut dyn io::Write) -> io::Result<()>;
}

#[derive(Clone, Debug, Default)]
pub struct Simple {
    pub code: Option<String>,
    pub title: String,
    pub description: Option<String>,
}

/// A severity level for a notification
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum Level {
    Fatal,
    Error,
    Warning,
    Success,
    Info,
}

impl Default for Level {
    fn default() -> Self {
        Self::Info
    }
}

impl Notification for Simple {
    fn html(&self, out: &mut dyn io::Write) -> io::Result<()> {
        write!(out, "<div class=n-title>")?;
        write!(out, "{}", htmlescape(&self.title))?;
        write!(out, "</div>")?;

        if let Some(description) = self.description.as_ref() {
            write!(out, "<div class=n-desc>")?;
            write!(out, "{}", htmlescape(description))?;
            write!(out, "</div>")?;
        }

        Ok(())
    }

    fn json(&self, out: &mut dyn io::Write) -> io::Result<()> {
        write!(out, "{{")?;

        write!(out, "\"title\":\"{}\"", jsonescape(&self.title))?;

        if let Some(code) = self.code.as_ref() {
            write!(out, ",\"code\":\"{}\"", jsonescape(code))?;
        }

        if let Some(description) = self.description.as_ref() {
            write!(out, ",\"description\":\"{}\"", jsonescape(description))?;
        }

        write!(out, "}}")?;

        Ok(())
    }

    fn tty(&self, out: &mut dyn io::Write) -> io::Result<()> {
        Ok(())
    }

    fn text(&self, out: &mut dyn io::Write) -> io::Result<()> {
        Ok(())
    }
}

pub struct Notifications {
    notifications: Arc<Mutex<Vec<Entry>>>,
    markers: Markers,
    regions: Tree,
}

impl Notifications {
    pub(crate) fn new(markers: Markers, regions: Tree) -> Self {
        Self {
            notifications: Default::default(),
            markers,
            regions,
        }
    }

    pub fn create(&self, level: Level, notification: Ref) -> NotificationId {
        let mut notifications = self.notifications.lock().unwrap();
        let id = NotificationId::new(notifications.len() as _);
        notifications.push((level, notification));
        id
    }

    pub fn notify(&self, file: FileId, bytes: Range<u32>, id: NotificationId) -> Result<()> {
        self.markers.mark(file, bytes, id)
    }

    pub fn get(&self, id: NotificationId) -> Entry {
        self.notifications.lock().unwrap()[id.0.get() as usize].clone()
    }

    pub(crate) fn finish_file(&self, file: FileId) -> Result<()> {
        let regions = &self.regions;

        self.markers.for_each(file, |entry| {
            regions.insert((entry.file, entry.start, entry.end).join(), entry.buf)?;

            Ok(())
        })?;

        Ok(())
    }
}
