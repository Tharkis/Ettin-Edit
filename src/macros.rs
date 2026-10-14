//! Macro recording and playback. A macro is the list of keys typed in the
//! editor and the editing commands run while recording.

use egui::{Event, Key, Modifiers};
use serde::{Deserialize, Serialize};

use crate::actions::Action;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MacroStep {
    /// Typed text.
    Text(String),
    /// A key press, by its egui name.
    Key {
        key: String,
        ctrl: bool,
        shift: bool,
        alt: bool,
    },
    Paste(String),
    Copy,
    Cut,
    Action(Action),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedMacro {
    pub name: String,
    pub steps: Vec<MacroStep>,
}

impl MacroStep {
    /// The step for an input event the editor handles, if it is one to record.
    pub fn from_event(event: &Event) -> Option<Self> {
        match event {
            Event::Text(t) if !t.is_empty() && !t.chars().any(char::is_control) => {
                Some(MacroStep::Text(t.clone()))
            }
            Event::Paste(t) => Some(MacroStep::Paste(t.clone())),
            Event::Copy => Some(MacroStep::Copy),
            Event::Cut => Some(MacroStep::Cut),
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => Some(MacroStep::Key {
                key: key.name().to_owned(),
                ctrl: modifiers.command || modifiers.ctrl,
                shift: modifiers.shift,
                alt: modifiers.alt,
            }),
            _ => None,
        }
    }

    /// The key and modifiers of a `Key` step.
    pub fn key(&self) -> Option<(Key, Modifiers)> {
        let MacroStep::Key {
            key,
            ctrl,
            shift,
            alt,
        } = self
        else {
            return None;
        };
        let mut m = Modifiers {
            alt: *alt,
            shift: *shift,
            ..Default::default()
        };
        if *ctrl {
            m.command = true;
            m.ctrl = !cfg!(target_os = "macos");
            m.mac_cmd = cfg!(target_os = "macos");
        }
        Some((Key::from_name(key)?, m))
    }
}

/// Recording state.
#[derive(Default)]
pub struct Recorder {
    pub recording: Option<Vec<MacroStep>>,
    /// The last recorded macro, ready to play.
    pub current: Vec<MacroStep>,
}

impl Recorder {
    pub fn is_recording(&self) -> bool {
        self.recording.is_some()
    }

    pub fn push(&mut self, step: MacroStep) {
        if let Some(steps) = &mut self.recording {
            // Merge consecutive typing into one step.
            if let (Some(MacroStep::Text(prev)), MacroStep::Text(t)) = (steps.last_mut(), &step) {
                prev.push_str(t);
                return;
            }
            steps.push(step);
        }
    }

    /// Starts recording, or stops and keeps what was recorded.
    pub fn toggle(&mut self) {
        match self.recording.take() {
            Some(steps) => self.current = steps,
            None => self.recording = Some(Vec::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_round_trip_through_json() {
        let steps = vec![
            MacroStep::Text("ab".into()),
            MacroStep::from_event(&Event::Key {
                key: Key::Home,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::SHIFT,
            })
            .unwrap(),
            MacroStep::Action(Action::Duplicate),
        ];
        let json = serde_json::to_string(&steps).unwrap();
        let back: Vec<MacroStep> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, steps);
        assert_eq!(back[1].key(), Some((Key::Home, Modifiers::SHIFT)));
    }

    #[test]
    fn typing_merges() {
        let mut r = Recorder::default();
        r.toggle();
        r.push(MacroStep::Text("a".into()));
        r.push(MacroStep::Text("b".into()));
        r.toggle();
        assert_eq!(r.current, vec![MacroStep::Text("ab".into())]);
    }
}
