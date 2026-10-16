//! Names for characters that show nothing (or look like a plain space), so
//! View > Show Symbol > Show Invisible Characters can draw them as small
//! labels: control characters, unusual spaces, zero-width and direction marks.

/// The short name drawn for `c`, or `None` for characters that show
/// themselves. Tabs, spaces and line breaks have their own symbols.
pub fn label(c: char) -> Option<&'static str> {
    const C0: [&str; 32] = [
        "NUL", "SOH", "STX", "ETX", "EOT", "ENQ", "ACK", "BEL", "BS", "HT", "LF", "VT", "FF", "CR",
        "SO", "SI", "DLE", "DC1", "DC2", "DC3", "DC4", "NAK", "SYN", "ETB", "CAN", "EM", "SUB",
        "ESC", "FS", "GS", "RS", "US",
    ];
    Some(match c {
        '\t' | '\n' => return None,
        '\0'..='\u{1F}' => C0[c as usize],
        '\u{7F}' => "DEL",
        '\u{80}'..='\u{9F}' => "C1",
        '\u{A0}' => "NBSP",
        '\u{AD}' => "SHY",
        '\u{34F}' => "CGJ",
        '\u{61C}' => "ALM",
        '\u{115F}' | '\u{1160}' | '\u{3164}' | '\u{FFA0}' => "HF",
        '\u{180E}' => "MVS",
        '\u{2000}'..='\u{200A}' => "SP",
        '\u{200B}' => "ZWSP",
        '\u{200C}' => "ZWNJ",
        '\u{200D}' => "ZWJ",
        '\u{200E}' => "LRM",
        '\u{200F}' => "RLM",
        '\u{2028}' => "LS",
        '\u{2029}' => "PS",
        '\u{202A}' => "LRE",
        '\u{202B}' => "RLE",
        '\u{202C}' => "PDF",
        '\u{202D}' => "LRO",
        '\u{202E}' => "RLO",
        '\u{202F}' => "NNBSP",
        '\u{205F}' => "MMSP",
        '\u{2060}' => "WJ",
        '\u{2061}'..='\u{2064}' => "INV",
        '\u{2066}' => "LRI",
        '\u{2067}' => "RLI",
        '\u{2068}' => "FSI",
        '\u{2069}' => "PDI",
        '\u{206A}'..='\u{206F}' => "DEPR",
        '\u{3000}' => "IDSP",
        '\u{FE00}'..='\u{FE0F}' => "VS",
        '\u{FEFF}' => "BOM",
        '\u{FFF9}'..='\u{FFFB}' => "IAC",
        '\u{E0001}' | '\u{E0020}'..='\u{E007F}' => "TAG",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::label;

    #[test]
    fn names_invisible_characters() {
        assert_eq!(label('\0'), Some("NUL"));
        assert_eq!(label('\u{1B}'), Some("ESC"));
        assert_eq!(label('\u{7F}'), Some("DEL"));
        assert_eq!(label('\u{A0}'), Some("NBSP"));
        assert_eq!(label('\u{200B}'), Some("ZWSP"));
        assert_eq!(label('\u{FEFF}'), Some("BOM"));
        assert_eq!(label('\u{202E}'), Some("RLO"));
    }

    #[test]
    fn leaves_ordinary_characters_alone() {
        for c in ['a', ' ', '\t', '\n', 'é', '中', '😀', '·'] {
            assert_eq!(label(c), None, "{c:?}");
        }
    }
}
