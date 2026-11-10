//! Transform > Convert to: which format the active tab (or its selection)
//! is in, worked out once per edit, and the Convert to and Read as menus.

use std::cell::RefCell;
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use super::EttinApp;
use super::menus::MenuModel;
use crate::actions::Action;
use crate::menu::MenuEntry;
use crate::tools::convert::{self, DataFormat};
use crate::tools::transform::TransformOp;

/// Texts up to this many characters are read straight away when a menu
/// opens; longer ones are read on a thread.
const READ_NOW_CHARS: usize = 2 * 1024 * 1024;
/// The toolbar only works out the format of texts up to this size, so
/// typing in a big file stays quick; it never starts a thread.
const TOOLBAR_CHARS: usize = 256 * 1024;
/// How long a menu waits for a reading thread before it shows "Reading…".
const WAIT_FOR_THREAD: Duration = Duration::from_millis(50);

/// What the text is, as far as Convert to knows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Source {
    /// The user picked it with Read as.
    Chosen(DataFormat),
    /// It looks like this format (`None`: like none of them).
    Detected(Option<DataFormat>),
    /// Still being worked out (a big text).
    Reading,
    /// Not worked out, because the text is big and nothing asked yet.
    Unknown,
}

impl Source {
    /// Whether converting to `to` can work: not to the format it is
    /// already in, and not when it is like none of them.
    pub fn allows(self, to: DataFormat) -> bool {
        match self {
            Source::Chosen(f) | Source::Detected(Some(f)) => f != to,
            Source::Detected(None) | Source::Reading => false,
            Source::Unknown => true,
        }
    }
}

enum State {
    Done(Option<DataFormat>),
    Running(Receiver<Option<DataFormat>>),
}

/// The last detection: what it was for, and its result.
#[derive(Default)]
pub(super) struct DetectCache {
    entry: RefCell<Option<(Key, State)>>,
}

/// What a detection depends on.
#[derive(Clone, PartialEq, Eq)]
struct Key {
    doc: u64,
    version: u64,
    target: std::ops::Range<usize>,
    syntax: String,
}

/// How far [`EttinApp::convert_source`] may go to find out.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Effort {
    /// A menu or the palette is open: read now, or on a thread for big texts.
    Menu,
    /// The toolbar: read small texts now, otherwise use what is known.
    Toolbar,
}

impl EttinApp {
    /// What Convert to reads the active tab's text (its first selection, or
    /// all of it) as.
    pub(super) fn convert_source(&self, effort: Effort) -> Source {
        let Some(doc) = self.docs.get(self.active) else {
            return Source::Detected(None);
        };
        if let Some(f) = self.read_as.get(&doc.id) {
            return Source::Chosen(*f);
        }
        if let Some(f) = DataFormat::from_language(&doc.syntax) {
            return Source::Detected(Some(f));
        }
        let editor = &doc.editor;
        let target = editor
            .tool_targets()
            .into_iter()
            .next()
            .unwrap_or(0..editor.buffer.len_chars());
        let key = Key {
            doc: doc.id,
            version: editor.buffer.version(),
            target: target.clone(),
            syntax: doc.syntax.clone(),
        };
        let mut entry = self.detect_cache.entry.borrow_mut();
        if let Some((k, state)) = entry.as_mut()
            && *k == key
        {
            match state {
                State::Done(found) => return Source::Detected(*found),
                State::Running(rx) => {
                    return match rx.try_recv() {
                        Ok(found) => {
                            *state = State::Done(found);
                            Source::Detected(found)
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => Source::Reading,
                        Err(_) => {
                            *state = State::Done(None);
                            Source::Detected(None)
                        }
                    };
                }
            }
        }
        let len = target.len();
        let limit = match effort {
            Effort::Menu => READ_NOW_CHARS,
            Effort::Toolbar => TOOLBAR_CHARS,
        };
        if len <= limit {
            let text = editor.buffer.slice_string(target);
            let found = convert::detect(&text, &doc.syntax);
            *entry = Some((key, State::Done(found)));
            return Source::Detected(found);
        }
        if effort == Effort::Toolbar {
            return Source::Unknown;
        }
        let text = editor.buffer.slice_string(target);
        let syntax = doc.syntax.clone();
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let _ = tx.send(convert::detect(&text, &syntax));
        });
        let state = match rx.recv_timeout(WAIT_FOR_THREAD) {
            Ok(found) => State::Done(found),
            Err(_) => State::Running(rx),
        };
        let source = match &state {
            State::Done(found) => Source::Detected(*found),
            State::Running(_) => Source::Reading,
        };
        *entry = Some((key, state));
        source
    }

    /// Transform > Convert to: a heading that says what the text is read
    /// as, the target formats, and Read as.
    pub(super) fn convert_menu(&self, m: &mut MenuModel) -> Vec<MenuEntry> {
        let source = self.convert_source(Effort::Menu);
        let mut out = Vec::new();
        match source {
            Source::Reading => {
                out.push(MenuEntry::Busy(t!("convert-reading")));
                // Check again soon for the thread's answer.
                m.ctx().request_repaint_after(Duration::from_millis(50));
            }
            Source::Chosen(f) | Source::Detected(Some(f)) => out.push(MenuEntry::Heading(t!(
                "convert-reading-as",
                format = f.name()
            ))),
            Source::Detected(None) => out.push(MenuEntry::Heading(t!("convert-not-data"))),
            Source::Unknown => {}
        }
        for to in DataFormat::TARGETS {
            let label = target_label(to);
            let name = t!("convert-to-format", format = label.clone());
            out.push(m.item_named(
                &label,
                &name,
                source.allows(to),
                Action::Transform(TransformOp::ConvertTo { to, from: None }),
            ));
        }
        out.push(MenuEntry::Separator);
        let chosen = match source {
            Source::Chosen(f) => Some(f),
            _ => None,
        };
        let auto = match source {
            Source::Detected(Some(f)) => t!("read-as-auto-format", format = f.name()),
            _ => t!("read-as-auto"),
        };
        let mut read_as = vec![m.toggle_named(
            &auto,
            &t!("read-as-named", format = t!("read-as-auto")),
            chosen.is_none(),
            Action::SetReadAs(None),
        )];
        for f in DataFormat::SOURCES {
            read_as.push(m.toggle_named(
                f.name(),
                &t!("read-as-named", format = f.name()),
                chosen == Some(f),
                Action::SetReadAs(Some(f)),
            ));
        }
        out.push(MenuEntry::Submenu {
            label: t!("menu-read-as"),
            entries: read_as,
        });
        out
    }

    /// Transform > Convert to > a format.
    pub(super) fn run_convert_to(
        &mut self,
        ctx: &egui::Context,
        to: DataFormat,
        from: Option<DataFormat>,
    ) {
        let doc = &self.docs[self.active];
        let whole = !doc.editor.selections.has_selection();
        let syntax = doc.syntax.clone();
        let from = from.or_else(|| self.read_as.get(&doc.id).copied());
        let unit = doc.editor.indent_unit();
        let what = t!("convert-to-format", format = target_label(to));
        let read = RefCell::new(None);
        let changed = self.transform(ctx, &what, |t| {
            let (out, from) = convert::convert_to(t, from, to, &syntax, &unit)?;
            read.borrow_mut().get_or_insert(from);
            Ok(out)
        });
        let Some(read) = read.into_inner() else {
            return;
        };
        if changed {
            if whole {
                let syntax = to.syntax();
                if self.languages.iter().any(|l| l == syntax) {
                    self.docs[self.active].syntax = syntax.to_owned();
                }
                // The text is in the new format now.
                let id = self.docs[self.active].id;
                self.read_as.remove(&id);
            }
            self.notify(
                ctx,
                t!("converted-to", to = target_label(to), from = read.name()),
                None,
            );
        }
    }

    /// Transform > Convert to > Read as.
    pub(super) fn set_read_as(&mut self, format: Option<DataFormat>) {
        let Some(doc) = self.docs.get(self.active) else {
            return;
        };
        match format {
            Some(f) => self.read_as.insert(doc.id, f),
            None => self.read_as.remove(&doc.id),
        };
    }
}

/// A target format's name in the menu: "Markdown table" for Markdown.
pub(super) fn target_label(f: DataFormat) -> String {
    match f {
        DataFormat::Markdown => t!("convert-markdown-table"),
        f => f.name().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sources_allow_the_right_targets() {
        let json = Source::Detected(Some(DataFormat::Json));
        assert!(!json.allows(DataFormat::Json));
        assert!(json.allows(DataFormat::Yaml));
        assert!(json.allows(DataFormat::Markdown));
        let none = Source::Detected(None);
        assert!(DataFormat::TARGETS.iter().all(|t| !none.allows(*t)));
        let chosen = Source::Chosen(DataFormat::Yaml);
        assert!(!chosen.allows(DataFormat::Yaml));
        assert!(chosen.allows(DataFormat::Json));
        assert!(!Source::Reading.allows(DataFormat::Json));
        assert!(Source::Unknown.allows(DataFormat::Json));
    }
}
