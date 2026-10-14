//! Keyboard shortcuts written as text by plugins, like `"Ctrl+Alt+U"`.

use egui::{Key, KeyboardShortcut, Modifiers};

/// Parses a shortcut such as `Ctrl+Shift+F12` or `Alt+1`. Ctrl means the
/// Command key on macOS, like NixPad's own shortcuts.
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
            other => return Err(format!("Unknown modifier \"{other}\" in \"{text}\"")),
        }
    }
    let key = key_from_name(key_name.trim())
        .ok_or_else(|| format!("Unknown key \"{}\" in \"{text}\"", key_name.trim()))?;
    let function_key = key.name().starts_with('F') && key.name().len() > 1;
    if !mods.command && !mods.alt && !function_key {
        return Err(format!(
            "\"{text}\" needs Ctrl or Alt, or it would get in the way of typing"
        ));
    }
    // winit turns Ctrl with X, C or V into cut, copy and paste.
    if mods.command && matches!(key, Key::X | Key::C | Key::V) {
        return Err(format!("\"{text}\" is kept for cut, copy and paste"));
    }
    Ok(KeyboardShortcut::new(mods, key))
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
