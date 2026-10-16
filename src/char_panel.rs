//! The character panel: a table of characters with their codes (decimal,
//! hex, HTML entity) and Unicode names. Double-click one to insert it.

use egui::{Align, Layout, RichText, Sense, Ui};
use egui_extras::{Column, TableBuilder};

use crate::editor::invisible;
use crate::theme::Palette;

/// Unicode blocks offered in the panel's list (block names stay in English,
/// as Unicode names them).
pub const BLOCKS: &[(&str, u32, u32)] = &[
    ("ASCII", 0x00, 0x7F),
    ("Latin-1 Supplement", 0x80, 0xFF),
    ("Latin Extended-A", 0x100, 0x17F),
    ("Latin Extended-B", 0x180, 0x24F),
    ("Greek and Coptic", 0x370, 0x3FF),
    ("Cyrillic", 0x400, 0x4FF),
    ("Hebrew", 0x590, 0x5FF),
    ("Arabic", 0x600, 0x6FF),
    ("General Punctuation", 0x2000, 0x206F),
    ("Superscripts and Subscripts", 0x2070, 0x209F),
    ("Currency Symbols", 0x20A0, 0x20CF),
    ("Letterlike Symbols", 0x2100, 0x214F),
    ("Number Forms", 0x2150, 0x218F),
    ("Arrows", 0x2190, 0x21FF),
    ("Mathematical Operators", 0x2200, 0x22FF),
    ("Miscellaneous Technical", 0x2300, 0x23FF),
    ("Box Drawing", 0x2500, 0x257F),
    ("Block Elements", 0x2580, 0x259F),
    ("Geometric Shapes", 0x25A0, 0x25FF),
    ("Miscellaneous Symbols", 0x2600, 0x26FF),
    ("Dingbats", 0x2700, 0x27BF),
    ("CJK Symbols and Punctuation", 0x3000, 0x303F),
    ("Specials", 0xFFF0, 0xFFFF),
];

/// Searches look through characters up to here (the end of the emoji blocks).
const SEARCH_END: u32 = 0x1FAFF;
/// At most this many search results are listed.
const MAX_RESULTS: usize = 2000;

/// What a double-click inserts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InsertAs {
    #[default]
    Character,
    Html,
    Hex,
    Decimal,
}

impl InsertAs {
    const ALL: [InsertAs; 4] = [
        InsertAs::Character,
        InsertAs::Html,
        InsertAs::Hex,
        InsertAs::Decimal,
    ];

    fn label(self) -> String {
        match self {
            InsertAs::Character => t!("char-insert-character"),
            InsertAs::Html => t!("char-insert-html"),
            InsertAs::Hex => t!("char-insert-hex"),
            InsertAs::Decimal => t!("char-insert-decimal"),
        }
    }

    /// The text to insert for `c`.
    pub fn text(self, c: char) -> String {
        match self {
            InsertAs::Character => c.to_string(),
            InsertAs::Html => html_entity(c),
            InsertAs::Hex => format!("U+{:04X}", c as u32),
            InsertAs::Decimal => (c as u32).to_string(),
        }
    }
}

/// Named HTML entities for characters people commonly type; others use a
/// numeric reference.
const ENTITIES: &[(char, &str)] = &[
    ('"', "quot"),
    ('&', "amp"),
    ('\'', "apos"),
    ('<', "lt"),
    ('>', "gt"),
    ('\u{A0}', "nbsp"),
    ('¡', "iexcl"),
    ('¢', "cent"),
    ('£', "pound"),
    ('¤', "curren"),
    ('¥', "yen"),
    ('¦', "brvbar"),
    ('§', "sect"),
    ('¨', "uml"),
    ('©', "copy"),
    ('ª', "ordf"),
    ('«', "laquo"),
    ('¬', "not"),
    ('\u{AD}', "shy"),
    ('®', "reg"),
    ('¯', "macr"),
    ('°', "deg"),
    ('±', "plusmn"),
    ('²', "sup2"),
    ('³', "sup3"),
    ('´', "acute"),
    ('µ', "micro"),
    ('¶', "para"),
    ('·', "middot"),
    ('¸', "cedil"),
    ('¹', "sup1"),
    ('º', "ordm"),
    ('»', "raquo"),
    ('¼', "frac14"),
    ('½', "frac12"),
    ('¾', "frac34"),
    ('¿', "iquest"),
    ('À', "Agrave"),
    ('Á', "Aacute"),
    ('Â', "Acirc"),
    ('Ã', "Atilde"),
    ('Ä', "Auml"),
    ('Å', "Aring"),
    ('Æ', "AElig"),
    ('Ç', "Ccedil"),
    ('È', "Egrave"),
    ('É', "Eacute"),
    ('Ê', "Ecirc"),
    ('Ë', "Euml"),
    ('Ì', "Igrave"),
    ('Í', "Iacute"),
    ('Î', "Icirc"),
    ('Ï', "Iuml"),
    ('Ð', "ETH"),
    ('Ñ', "Ntilde"),
    ('Ò', "Ograve"),
    ('Ó', "Oacute"),
    ('Ô', "Ocirc"),
    ('Õ', "Otilde"),
    ('Ö', "Ouml"),
    ('×', "times"),
    ('Ø', "Oslash"),
    ('Ù', "Ugrave"),
    ('Ú', "Uacute"),
    ('Û', "Ucirc"),
    ('Ü', "Uuml"),
    ('Ý', "Yacute"),
    ('Þ', "THORN"),
    ('ß', "szlig"),
    ('à', "agrave"),
    ('á', "aacute"),
    ('â', "acirc"),
    ('ã', "atilde"),
    ('ä', "auml"),
    ('å', "aring"),
    ('æ', "aelig"),
    ('ç', "ccedil"),
    ('è', "egrave"),
    ('é', "eacute"),
    ('ê', "ecirc"),
    ('ë', "euml"),
    ('ì', "igrave"),
    ('í', "iacute"),
    ('î', "icirc"),
    ('ï', "iuml"),
    ('ð', "eth"),
    ('ñ', "ntilde"),
    ('ò', "ograve"),
    ('ó', "oacute"),
    ('ô', "ocirc"),
    ('õ', "otilde"),
    ('ö', "ouml"),
    ('÷', "divide"),
    ('ø', "oslash"),
    ('ù', "ugrave"),
    ('ú', "uacute"),
    ('û', "ucirc"),
    ('ü', "uuml"),
    ('ý', "yacute"),
    ('þ', "thorn"),
    ('ÿ', "yuml"),
    ('Œ', "OElig"),
    ('œ', "oelig"),
    ('Š', "Scaron"),
    ('š', "scaron"),
    ('Ÿ', "Yuml"),
    ('ƒ', "fnof"),
    ('ˆ', "circ"),
    ('˜', "tilde"),
    ('Α', "Alpha"),
    ('Β', "Beta"),
    ('Γ', "Gamma"),
    ('Δ', "Delta"),
    ('Ε', "Epsilon"),
    ('Ζ', "Zeta"),
    ('Η', "Eta"),
    ('Θ', "Theta"),
    ('Ι', "Iota"),
    ('Κ', "Kappa"),
    ('Λ', "Lambda"),
    ('Μ', "Mu"),
    ('Ν', "Nu"),
    ('Ξ', "Xi"),
    ('Ο', "Omicron"),
    ('Π', "Pi"),
    ('Ρ', "Rho"),
    ('Σ', "Sigma"),
    ('Τ', "Tau"),
    ('Υ', "Upsilon"),
    ('Φ', "Phi"),
    ('Χ', "Chi"),
    ('Ψ', "Psi"),
    ('Ω', "Omega"),
    ('α', "alpha"),
    ('β', "beta"),
    ('γ', "gamma"),
    ('δ', "delta"),
    ('ε', "epsilon"),
    ('ζ', "zeta"),
    ('η', "eta"),
    ('θ', "theta"),
    ('ι', "iota"),
    ('κ', "kappa"),
    ('λ', "lambda"),
    ('μ', "mu"),
    ('ν', "nu"),
    ('ξ', "xi"),
    ('ο', "omicron"),
    ('π', "pi"),
    ('ρ', "rho"),
    ('ς', "sigmaf"),
    ('σ', "sigma"),
    ('τ', "tau"),
    ('υ', "upsilon"),
    ('φ', "phi"),
    ('χ', "chi"),
    ('ψ', "psi"),
    ('ω', "omega"),
    ('\u{2002}', "ensp"),
    ('\u{2003}', "emsp"),
    ('\u{2009}', "thinsp"),
    ('\u{200C}', "zwnj"),
    ('\u{200D}', "zwj"),
    ('\u{200E}', "lrm"),
    ('\u{200F}', "rlm"),
    ('–', "ndash"),
    ('—', "mdash"),
    ('‘', "lsquo"),
    ('’', "rsquo"),
    ('‚', "sbquo"),
    ('“', "ldquo"),
    ('”', "rdquo"),
    ('„', "bdquo"),
    ('†', "dagger"),
    ('‡', "Dagger"),
    ('•', "bull"),
    ('…', "hellip"),
    ('‰', "permil"),
    ('′', "prime"),
    ('″', "Prime"),
    ('‹', "lsaquo"),
    ('›', "rsaquo"),
    ('‾', "oline"),
    ('⁄', "frasl"),
    ('€', "euro"),
    ('ℑ', "image"),
    ('℘', "weierp"),
    ('ℜ', "real"),
    ('™', "trade"),
    ('ℵ', "alefsym"),
    ('←', "larr"),
    ('↑', "uarr"),
    ('→', "rarr"),
    ('↓', "darr"),
    ('↔', "harr"),
    ('↵', "crarr"),
    ('⇐', "lArr"),
    ('⇑', "uArr"),
    ('⇒', "rArr"),
    ('⇓', "dArr"),
    ('⇔', "hArr"),
    ('∀', "forall"),
    ('∂', "part"),
    ('∃', "exist"),
    ('∅', "empty"),
    ('∇', "nabla"),
    ('∈', "isin"),
    ('∉', "notin"),
    ('∋', "ni"),
    ('∏', "prod"),
    ('∑', "sum"),
    ('−', "minus"),
    ('∗', "lowast"),
    ('√', "radic"),
    ('∝', "prop"),
    ('∞', "infin"),
    ('∠', "ang"),
    ('∧', "and"),
    ('∨', "or"),
    ('∩', "cap"),
    ('∪', "cup"),
    ('∫', "int"),
    ('∴', "there4"),
    ('∼', "sim"),
    ('≅', "cong"),
    ('≈', "asymp"),
    ('≠', "ne"),
    ('≡', "equiv"),
    ('≤', "le"),
    ('≥', "ge"),
    ('⊂', "sub"),
    ('⊃', "sup"),
    ('⊄', "nsub"),
    ('⊆', "sube"),
    ('⊇', "supe"),
    ('⊕', "oplus"),
    ('⊗', "otimes"),
    ('⊥', "perp"),
    ('⋅', "sdot"),
    ('◊', "loz"),
    ('♠', "spades"),
    ('♣', "clubs"),
    ('♥', "hearts"),
    ('♦', "diams"),
];

/// The HTML entity for `c`: a named one when there is one, otherwise `&#N;`.
pub fn html_entity(c: char) -> String {
    match ENTITIES.iter().find(|(e, _)| *e == c) {
        Some((_, name)) => format!("&{name};"),
        None => format!("&#{};", c as u32),
    }
}

/// The Unicode name of `c`, such as "LATIN SMALL LETTER E WITH ACUTE".
pub fn char_name(c: char) -> String {
    /// Control characters have no Unicode name, only these traditional ones.
    const C0: [&str; 32] = [
        "NULL",
        "START OF HEADING",
        "START OF TEXT",
        "END OF TEXT",
        "END OF TRANSMISSION",
        "ENQUIRY",
        "ACKNOWLEDGE",
        "BELL",
        "BACKSPACE",
        "CHARACTER TABULATION (TAB)",
        "LINE FEED (LF)",
        "LINE TABULATION",
        "FORM FEED (FF)",
        "CARRIAGE RETURN (CR)",
        "SHIFT OUT",
        "SHIFT IN",
        "DATA LINK ESCAPE",
        "DEVICE CONTROL ONE",
        "DEVICE CONTROL TWO",
        "DEVICE CONTROL THREE",
        "DEVICE CONTROL FOUR",
        "NEGATIVE ACKNOWLEDGE",
        "SYNCHRONOUS IDLE",
        "END OF TRANSMISSION BLOCK",
        "CANCEL",
        "END OF MEDIUM",
        "SUBSTITUTE",
        "ESCAPE",
        "INFORMATION SEPARATOR FOUR",
        "INFORMATION SEPARATOR THREE",
        "INFORMATION SEPARATOR TWO",
        "INFORMATION SEPARATOR ONE",
    ];
    if let Some(n) = unicode_names2::name(c) {
        return n.to_string();
    }
    match c as u32 {
        n @ 0..=0x1F => C0[n as usize].to_owned(),
        0x7F => "DELETE".to_owned(),
        _ if c.is_control() => "CONTROL".to_owned(),
        _ => String::new(),
    }
}

/// Characters matching a search: the character itself, a code (`U+00E9`,
/// `0xE9`, `e9` or `233`), or words of its name.
pub fn search(query: &str) -> Vec<char> {
    let q = query.trim();
    if q.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let push = |c: char, out: &mut Vec<char>| {
        if !out.contains(&c) {
            out.push(c);
        }
    };
    let mut chars = q.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        push(c, &mut out);
    }
    let lower = q.to_lowercase();
    let hex = lower
        .strip_prefix("u+")
        .or_else(|| lower.strip_prefix("0x"))
        .or_else(|| lower.strip_prefix("&#x").map(|s| s.trim_end_matches(';')))
        .unwrap_or(&lower);
    if let Some(c) = u32::from_str_radix(hex, 16).ok().and_then(char::from_u32) {
        push(c, &mut out);
    }
    let dec = lower
        .strip_prefix("&#")
        .map(|s| s.trim_end_matches(';'))
        .unwrap_or(&lower);
    if let Some(c) = dec.parse::<u32>().ok().and_then(char::from_u32) {
        push(c, &mut out);
    }
    if let Some(name) = lower.strip_prefix('&').map(|s| s.trim_end_matches(';'))
        && let Some((c, _)) = ENTITIES.iter().find(|(_, n)| *n == name)
    {
        push(*c, &mut out);
    }
    let words: Vec<String> = q.split_whitespace().map(str::to_uppercase).collect();
    if words.iter().any(|w| w.chars().any(|c| c.is_alphabetic())) {
        let phrase = words.join(" ");
        let mut found: Vec<(bool, usize, char)> = Vec::new();
        for code in 0..=SEARCH_END {
            if found.len() >= MAX_RESULTS {
                break;
            }
            let Some(c) = char::from_u32(code) else {
                continue;
            };
            let name = char_name(c);
            if words
                .iter()
                .all(|w| name.split([' ', '-']).any(|n| n.starts_with(w.as_str())))
            {
                found.push((!name.contains(&phrase), name.len(), c));
            }
        }
        // Names holding the words as typed come first, shortest first.
        found.sort_by_key(|&(apart, len, _)| (apart, len));
        for (_, _, c) in found {
            push(c, &mut out);
        }
    }
    out
}

/// What the panel asks the app to do.
pub enum CharAction {
    Insert(String),
    Close,
}

#[derive(Default)]
pub struct CharPanel {
    block: usize,
    query: String,
    /// The search the results are for, and the results.
    results: Option<(String, Vec<char>)>,
    insert_as: InsertAs,
    selected: Option<char>,
}

impl CharPanel {
    /// The characters listed: the search results, or the chosen block.
    fn listed(&mut self) -> Vec<char> {
        let q = self.query.trim().to_owned();
        if q.is_empty() {
            let (_, start, end) = BLOCKS[self.block.min(BLOCKS.len() - 1)];
            return (start..=end).filter_map(char::from_u32).collect();
        }
        if self.results.as_ref().is_none_or(|(r, _)| *r != q) {
            self.results = Some((q.clone(), search(&q)));
        }
        self.results
            .as_ref()
            .map(|(_, r)| r.clone())
            .unwrap_or_default()
    }

    pub fn ui(&mut self, ui: &mut Ui, palette: &Palette) -> Option<CharAction> {
        let mut action = None;
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("character-panel")).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(egui::Button::new("×").frame(false))
                    .on_hover_text(t!("close-panel"))
                    .clicked()
                {
                    action = Some(CharAction::Close);
                }
            });
        });
        ui.add(
            egui::TextEdit::singleline(&mut self.query)
                .hint_text(t!("char-search-hint"))
                .desired_width(f32::INFINITY),
        );
        ui.horizontal(|ui| {
            ui.add_enabled_ui(self.query.trim().is_empty(), |ui| {
                egui::ComboBox::from_id_salt("char-block")
                    .selected_text(BLOCKS[self.block].0)
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        for (i, (name, start, _)) in BLOCKS.iter().enumerate() {
                            ui.selectable_value(&mut self.block, i, *name)
                                .on_hover_text(format!("U+{start:04X}"));
                        }
                    });
            });
            egui::ComboBox::from_id_salt("char-insert-as")
                .selected_text(self.insert_as.label())
                .show_ui(ui, |ui| {
                    for a in InsertAs::ALL {
                        ui.selectable_value(&mut self.insert_as, a, a.label());
                    }
                });
        });
        ui.label(
            RichText::new(t!("char-panel-hint"))
                .size(11.5)
                .color(palette.text_dim),
        );
        ui.separator();
        let chars = self.listed();
        if chars.is_empty() {
            ui.label(RichText::new(t!("no-results")).color(palette.text_dim));
            return action;
        }
        let row_h = 20.0;
        let mono = egui::FontId::monospace(13.0);
        TableBuilder::new(ui)
            .id_salt(("char-table", self.block, self.query.trim()))
            .striped(true)
            .sense(Sense::click())
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::exact(34.0))
            .column(Column::exact(40.0))
            .column(Column::exact(44.0))
            .column(Column::exact(72.0))
            .column(Column::remainder().clip(true))
            .header(row_h, |mut header| {
                for title in [
                    t!("char-col-char"),
                    t!("char-col-dec"),
                    t!("char-col-hex"),
                    t!("char-col-html"),
                    t!("char-col-name"),
                ] {
                    header.col(|ui| {
                        ui.label(RichText::new(title).strong().size(12.0));
                    });
                }
            })
            .body(|body| {
                body.rows(row_h, chars.len(), |mut row| {
                    let c = chars[row.index()];
                    row.set_selected(self.selected == Some(c));
                    let label = match c {
                        '\t' => Some("HT"),
                        '\n' => Some("LF"),
                        ' ' => Some("SP"),
                        c => invisible::label(c),
                    };
                    let shown = match label {
                        Some(label) => RichText::new(label)
                            .size(10.0)
                            .color(palette.editor_fg)
                            .background_color(palette.whitespace.gamma_multiply(0.7)),
                        None => RichText::new(c.to_string()).font(mono.clone()),
                    };
                    row.col(|ui| {
                        ui.label(shown);
                    });
                    row.col(|ui| {
                        ui.label(RichText::new((c as u32).to_string()).size(12.0));
                    });
                    row.col(|ui| {
                        ui.label(RichText::new(format!("{:04X}", c as u32)).size(12.0));
                    });
                    row.col(|ui| {
                        ui.label(RichText::new(html_entity(c)).size(12.0));
                    });
                    row.col(|ui| {
                        ui.label(
                            RichText::new(char_name(c))
                                .size(11.5)
                                .color(palette.text_dim),
                        );
                    });
                    let response = row.response().on_hover_text(char_name(c));
                    if response.clicked() {
                        self.selected = Some(c);
                    }
                    if response.double_clicked() {
                        action = Some(CharAction::Insert(self.insert_as.text(c)));
                    }
                    response.context_menu(|ui| {
                        for (label, text) in [
                            (t!("char-copy-character"), c.to_string()),
                            (t!("char-copy-html"), html_entity(c)),
                            (t!("char-copy-code"), format!("U+{:04X}", c as u32)),
                        ] {
                            if ui.button(label).clicked() {
                                ui.ctx().copy_text(text);
                                ui.close();
                            }
                        }
                    });
                });
            });
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entities_and_names() {
        assert_eq!(html_entity('é'), "&eacute;");
        assert_eq!(html_entity('€'), "&euro;");
        assert_eq!(html_entity('Ā'), "&#256;");
        assert_eq!(char_name('é'), "LATIN SMALL LETTER E WITH ACUTE");
        assert_eq!(char_name('\u{1B}'), "ESCAPE");
        assert_eq!(char_name('\t'), "CHARACTER TABULATION (TAB)");
        assert_eq!(InsertAs::Hex.text('é'), "U+00E9");
        assert_eq!(InsertAs::Decimal.text('A'), "65");
    }

    #[test]
    fn searches_by_code_and_name() {
        assert!(search("U+00E9").contains(&'é'));
        assert!(search("0xe9").contains(&'é'));
        assert!(search("233").contains(&'é'));
        assert!(search("&eacute;").contains(&'é'));
        assert_eq!(search("é")[0], 'é');
        let found = search("e acute");
        assert!(found.contains(&'é') && found.contains(&'É'));
        assert_eq!(search("rightwards arrow")[0], '→');
        assert!(search("escape").contains(&'\u{1B}'));
        assert!(search("   ").is_empty());
    }

    #[test]
    fn every_block_is_a_valid_range() {
        for (name, start, end) in BLOCKS {
            assert!(start < end, "{name}");
        }
    }
}
