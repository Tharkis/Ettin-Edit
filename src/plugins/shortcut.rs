//! Keyboard shortcuts written as text by plugins, like `"Ctrl+Alt+U"`.

use egui::{Key, KeyboardShortcut, Modifiers};

/// Parses a shortcut such as `Ctrl+Shift+F12` or `Alt+1`. Ctrl means the
/// Command key on macOS, like Ettin Edit's own shortcuts.
pub fn parse(text: &str) -> Result<KeyboardShortcut, String> {
    let text = text.trim();
    // "Ctrl++" ends in the plus key.
    let (mods_part, key_name) = match text.strip_suffix("++") {
        Some(rest) => (rest, "+"),
        None => match text.rsplit_once('+') {
            Some((m, k)) => (m, k),
            None => ("", text),
        },
    };
    let mut mods = Modifiers::NONE;
    for part in mods_part
        .split('+')
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" | "cmd" | "command" | "cmdorctrl" | "commandorcontrol" => {
                mods |= Modifiers::COMMAND
            }
            "alt" | "option" => mods |= Modifiers::ALT,
            "shift" => mods |= Modifiers::SHIFT,
            other => return Err(t!("unknown-modifier", modifier = other, shortcut = text)),
        }
    }
    let key = key_from_name(key_name.trim())
        .ok_or_else(|| t!("unknown-key", key = key_name.trim(), shortcut = text))?;
    let function_key = key.name().starts_with('F') && key.name().len() > 1;
    if !mods.command && !mods.alt && !function_key {
        return Err(t!("shortcut-needs-modifier", shortcut = text));
    }
    // winit turns Ctrl with X, C or V into cut, copy and paste.
    if mods.command && matches!(key, Key::X | Key::C | Key::V) {
        return Err(t!("shortcut-reserved", shortcut = text));
    }
    Ok(KeyboardShortcut::new(mods, key))
}

/// Writes a shortcut the way [`parse`] reads it, such as `Ctrl+Shift+F12`.
pub fn to_text(s: &KeyboardShortcut) -> String {
    let m = s.modifiers;
    let mut text = String::new();
    if m.command || m.ctrl || m.mac_cmd {
        text += "Ctrl+";
    }
    if m.alt {
        text += "Alt+";
    }
    if m.shift {
        text += "Shift+";
    }
    // Newer egui versions name the plus key "Plus"; keep writing "+".
    text += match s.logical_key {
        Key::Plus => "+",
        key => key.name(),
    };
    text
}

/// A two-key chord (Ettin Edit 2.6): a shortcut, then a second key that
/// may be pressed on its own or with modifiers, like VS Code's `Ctrl+K S`.
pub type Chord = (KeyboardShortcut, KeyboardShortcut);

/// True when `text` is written as a chord: two shortcuts with a space.
pub fn is_chord(text: &str) -> bool {
    text.trim().contains(' ')
}

/// Parses a chord such as `Ctrl+K S` or `Ctrl+K Ctrl+0`.
pub fn parse_chord(text: &str) -> Result<Chord, String> {
    let mut parts = text.split_whitespace();
    let (Some(first), Some(second), None) = (parts.next(), parts.next(), parts.next()) else {
        return Err(t!("chord-two-keys", shortcut = text.trim()));
    };
    let first = parse(first)?;
    Ok((first, parse_second(second)?))
}

/// The second key of a chord: any key, with or without modifiers.
pub fn parse_second(text: &str) -> Result<KeyboardShortcut, String> {
    match parse(text) {
        Ok(s) => Ok(s),
        Err(e) if text.contains('+') && !text.ends_with("++") => Err(e),
        Err(_) => key_from_name(text.trim())
            .map(|k| KeyboardShortcut::new(Modifiers::NONE, k))
            .ok_or_else(|| t!("unknown-key", key = text.trim(), shortcut = text)),
    }
}

/// Writes a chord the way [`parse_chord`] reads it.
pub fn chord_to_text(c: &Chord) -> String {
    format!("{} {}", to_text(&c.0), to_text(&c.1))
}

/// A chord as menus show it, in the platform's style.
pub fn format_chord(ctx: &egui::Context, c: &Chord) -> String {
    format!(
        "{} {}",
        ctx.format_shortcut(&c.0),
        ctx.format_shortcut(&c.1)
    )
}

/// True when both chords are the same keys.
pub fn same_chord(a: &Chord, b: &Chord) -> bool {
    same(&a.0, &b.0) && same(&a.1, &b.1)
}

fn key_from_name(name: &str) -> Option<Key> {
    let alias = match name.to_ascii_lowercase().as_str() {
        "del" => "Delete",
        "ins" => "Insert",
        "pgup" => "PageUp",
        "pgdn" | "pgdown" => "PageDown",
        "bksp" => "Backspace",
        _ => name,
    };
    Key::from_name(alias).or_else(|| {
        // Accept any capitalisation: "pagedown", "ENTER", "f5".
        let lower = alias.to_ascii_lowercase();
        Key::ALL
            .iter()
            .copied()
            .find(|k| k.name().to_ascii_lowercase() == lower)
    })
}

/// How many modifiers a shortcut uses. Shortcuts with more are checked first,
/// because egui lets Shift and Alt be held for a shortcut without them.
pub fn modifier_count(s: &KeyboardShortcut) -> u8 {
    let m = s.modifiers;
    u8::from(m.command || m.ctrl) + u8::from(m.alt) + u8::from(m.shift)
}

/// True when both shortcuts are the same keys.
pub fn same(a: &KeyboardShortcut, b: &KeyboardShortcut) -> bool {
    a.logical_key == b.logical_key && modifier_bits(a) == modifier_bits(b)
}

fn modifier_bits(s: &KeyboardShortcut) -> (bool, bool, bool) {
    let m = s.modifiers;
    (m.command || m.ctrl, m.alt, m.shift)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chords() {
        let c = parse_chord("Ctrl+K S").unwrap();
        assert!(c.0.modifiers.command && c.0.logical_key == Key::K);
        assert!(c.1.modifiers.is_none() && c.1.logical_key == Key::S);
        assert_eq!(chord_to_text(&c), "Ctrl+K S");
        let c = parse_chord("ctrl+k ctrl+0").unwrap();
        assert_eq!(chord_to_text(&c), "Ctrl+K Ctrl+0");
        assert!(parse_chord("Ctrl+K").is_err());
        assert!(
            parse_chord("K S").is_err(),
            "the first key needs a modifier"
        );
        assert!(parse_chord("Ctrl+K Bogus").is_err());
        assert!(is_chord(" Ctrl+K  S ") && !is_chord("Ctrl+K"));
    }

    #[test]
    fn parses_common_shortcuts() {
        let s = parse("Ctrl+Alt+U").unwrap();
        assert_eq!(s.logical_key, Key::U);
        assert!(s.modifiers.command && s.modifiers.alt);
        assert!(!s.modifiers.shift);
        assert_eq!(parse("ctrl+shift+f12").unwrap().logical_key, Key::F12);
        assert_eq!(parse("F6").unwrap().logical_key, Key::F6);
        assert_eq!(parse("Alt+1").unwrap().logical_key, Key::Num1);
        assert_eq!(parse("Ctrl++").unwrap().logical_key, Key::Plus);
        assert_eq!(parse("Ctrl+PgDn").unwrap().logical_key, Key::PageDown);
        assert_eq!(parse("Alt+enter").unwrap().logical_key, Key::Enter);
    }

    #[test]
    fn rejects_bad_shortcuts() {
        assert!(parse("U").is_err());
        assert!(parse("Shift+U").is_err());
        assert!(parse("Ctrl+C").is_err());
        assert!(parse("Ctrl+Shift+V").is_err());
        assert!(parse("Hyper+U").is_err());
        assert!(parse("Ctrl+Banana").is_err());
    }

    #[test]
    fn writes_shortcuts_back_as_text() {
        for (text, written) in [
            ("Ctrl+Alt+U", "Ctrl+Alt+U"),
            ("Ctrl+Shift+F12", "Ctrl+Shift+F12"),
            ("F6", "F6"),
            ("Alt+1", "Alt+1"),
            ("Ctrl++", "Ctrl++"),
            ("Shift+F3", "Shift+F3"),
        ] {
            let s = parse(text).unwrap();
            assert_eq!(to_text(&s), written);
            assert!(same(&parse(&to_text(&s)).unwrap(), &s));
        }
    }

    #[test]
    fn compares_and_counts() {
        let a = parse("Ctrl+Shift+D").unwrap();
        let b = KeyboardShortcut::new(
            Modifiers {
                shift: true,
                ..Modifiers::COMMAND
            },
            Key::D,
        );
        assert!(same(&a, &b));
        assert!(!same(&a, &parse("Ctrl+D").unwrap()));
        assert_eq!(modifier_count(&a), 2);
        assert_eq!(modifier_count(&parse("F5").unwrap()), 0);
    }
}
