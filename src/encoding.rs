//! Text encodings: detecting them when a file is opened, decoding, and
//! encoding again on save. UTF-8 and UTF-16 are handled here; everything else
//! (Windows code pages, ISO-8859, KOI8, Shift_JIS...) goes through encoding_rs.

use encoding_rs::Encoding;

const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
const UTF16LE_BOM: &[u8] = &[0xFF, 0xFE];
const UTF16BE_BOM: &[u8] = &[0xFE, 0xFF];

/// How many bytes the charset detector looks at.
const DETECT_LIMIT: usize = 1 << 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextEncoding {
    Utf8 {
        bom: bool,
    },
    Utf16 {
        big_endian: bool,
        bom: bool,
    },
    /// A single- or multi-byte legacy encoding, never UTF-8 or UTF-16.
    Legacy(&'static Encoding),
}

impl Default for TextEncoding {
    fn default() -> Self {
        TextEncoding::Utf8 { bom: false }
    }
}

/// Named groups of legacy encodings for the Encoding menu: (group, [(label, encoding)]).
pub fn legacy_groups() -> Vec<(&'static str, Vec<(&'static str, &'static Encoding)>)> {
    use encoding_rs::*;
    vec![
        (
            "Western European",
            vec![
                ("Windows-1252 / ISO-8859-1", WINDOWS_1252),
                ("ISO-8859-15", ISO_8859_15),
                ("Macintosh (Mac Roman)", MACINTOSH),
                ("ISO-8859-14 (Celtic)", ISO_8859_14),
                ("ISO-8859-3 (South European)", ISO_8859_3),
            ],
        ),
        (
            "Central European",
            vec![("Windows-1250", WINDOWS_1250), ("ISO-8859-2", ISO_8859_2)],
        ),
        (
            "Cyrillic",
            vec![
                ("Windows-1251", WINDOWS_1251),
                ("ISO-8859-5", ISO_8859_5),
                ("KOI8-R", KOI8_R),
                ("KOI8-U", KOI8_U),
                ("IBM866 (OEM 866)", IBM866),
            ],
        ),
        (
            "Greek",
            vec![("Windows-1253", WINDOWS_1253), ("ISO-8859-7", ISO_8859_7)],
        ),
        ("Turkish", vec![("Windows-1254 / ISO-8859-9", WINDOWS_1254)]),
        (
            "Baltic",
            vec![
                ("Windows-1257", WINDOWS_1257),
                ("ISO-8859-4", ISO_8859_4),
                ("ISO-8859-13", ISO_8859_13),
            ],
        ),
        ("Nordic", vec![("ISO-8859-10", ISO_8859_10)]),
        (
            "Hebrew",
            vec![("Windows-1255", WINDOWS_1255), ("ISO-8859-8", ISO_8859_8)],
        ),
        (
            "Arabic",
            vec![("Windows-1256", WINDOWS_1256), ("ISO-8859-6", ISO_8859_6)],
        ),
        ("Vietnamese", vec![("Windows-1258", WINDOWS_1258)]),
        ("Thai", vec![("Windows-874", WINDOWS_874)]),
        ("Romanian", vec![("ISO-8859-16", ISO_8859_16)]),
        (
            "Chinese",
            vec![("GBK / GB2312", GBK), ("GB18030", GB18030), ("Big5", BIG5)],
        ),
        (
            "Japanese",
            vec![
                ("Shift_JIS", SHIFT_JIS),
                ("EUC-JP", EUC_JP),
                ("ISO-2022-JP", ISO_2022_JP),
            ],
        ),
        ("Korean", vec![("EUC-KR", EUC_KR)]),
    ]
}

impl TextEncoding {
    /// The four Unicode choices shown at the top of the Encoding menu.
    pub const UNICODE: [TextEncoding; 4] = [
        TextEncoding::Utf8 { bom: false },
        TextEncoding::Utf8 { bom: true },
        TextEncoding::Utf16 {
            big_endian: false,
            bom: true,
        },
        TextEncoding::Utf16 {
            big_endian: true,
            bom: true,
        },
    ];

    /// The name shown in menus and the status bar, also used to save the choice.
    pub fn name(self) -> String {
        match self {
            TextEncoding::Utf8 { bom: false } => "UTF-8".into(),
            TextEncoding::Utf8 { bom: true } => "UTF-8 BOM".into(),
            TextEncoding::Utf16 { big_endian, bom } => format!(
                "UTF-16 {}{}",
                if big_endian { "BE" } else { "LE" },
                if bom { " BOM" } else { "" }
            ),
            TextEncoding::Legacy(e) => legacy_groups()
                .into_iter()
                .flat_map(|(_, list)| list)
                .find(|(_, enc)| *enc == e)
                .map(|(label, _)| label.to_owned())
                .unwrap_or_else(|| e.name().to_owned()),
        }
    }

    /// The reverse of [`TextEncoding::name`]; also accepts any WHATWG label.
    pub fn from_name(name: &str) -> Option<Self> {
        if let Some(e) = Self::UNICODE.into_iter().find(|e| e.name() == name) {
            return Some(e);
        }
        match name {
            "UTF-16 LE" => {
                return Some(TextEncoding::Utf16 {
                    big_endian: false,
                    bom: false,
                });
            }
            "UTF-16 BE" => {
                return Some(TextEncoding::Utf16 {
                    big_endian: true,
                    bom: false,
                });
            }
            _ => {}
        }
        if let Some(e) = legacy_groups()
            .into_iter()
            .flat_map(|(_, list)| list)
            .find(|(label, _)| *label == name)
        {
            return Some(TextEncoding::Legacy(e.1));
        }
        Encoding::for_label(name.as_bytes()).map(Self::from_encoding)
    }

    fn from_encoding(e: &'static Encoding) -> Self {
        if e == encoding_rs::UTF_8 {
            TextEncoding::Utf8 { bom: false }
        } else if e == encoding_rs::UTF_16LE {
            TextEncoding::Utf16 {
                big_endian: false,
                bom: false,
            }
        } else if e == encoding_rs::UTF_16BE {
            TextEncoding::Utf16 {
                big_endian: true,
                bom: false,
            }
        } else {
            TextEncoding::Legacy(e)
        }
    }

    fn bom(self) -> &'static [u8] {
        match self {
            TextEncoding::Utf8 { bom: true } => UTF8_BOM,
            TextEncoding::Utf16 {
                big_endian: false,
                bom: true,
            } => UTF16LE_BOM,
            TextEncoding::Utf16 {
                big_endian: true,
                bom: true,
            } => UTF16BE_BOM,
            _ => &[],
        }
    }

    /// Works out how `bytes` are encoded. A byte-order mark wins, then valid
    /// UTF-8, then `hint` (an encoding the user picked before), then a guess.
    pub fn detect(bytes: &[u8], hint: Option<TextEncoding>) -> TextEncoding {
        if bytes.starts_with(UTF8_BOM) {
            return TextEncoding::Utf8 { bom: true };
        }
        if bytes.starts_with(UTF16LE_BOM) {
            return TextEncoding::Utf16 {
                big_endian: false,
                bom: true,
            };
        }
        if bytes.starts_with(UTF16BE_BOM) {
            return TextEncoding::Utf16 {
                big_endian: true,
                bom: true,
            };
        }
        if let Some(e) = utf16_without_bom(bytes) {
            return e;
        }
        if std::str::from_utf8(bytes).is_ok() {
            return TextEncoding::Utf8 { bom: false };
        }
        if let Some(h @ TextEncoding::Legacy(_)) = hint {
            return h;
        }
        let mut detector = chardetng::EncodingDetector::new(chardetng::Iso2022JpDetection::Allow);
        let sample = &bytes[..bytes.len().min(DETECT_LIMIT)];
        detector.feed(sample, sample.len() == bytes.len());
        // Not valid UTF-8 by now, so don't let the guess be UTF-8.
        Self::from_encoding(detector.guess(None, chardetng::Utf8Detection::Deny))
    }

    /// Decodes `bytes` (BOM included, if any). Returns the text and whether
    /// some bytes could not be decoded and were replaced with U+FFFD.
    pub fn decode(self, bytes: &[u8]) -> (String, bool) {
        let body = bytes.strip_prefix(self.bom()).unwrap_or(bytes);
        match self {
            TextEncoding::Utf8 { .. } => match std::str::from_utf8(body) {
                Ok(s) => (s.to_owned(), false),
                Err(_) => (String::from_utf8_lossy(body).into_owned(), true),
            },
            TextEncoding::Utf16 { big_endian, .. } => {
                let enc = if big_endian {
                    encoding_rs::UTF_16BE
                } else {
                    encoding_rs::UTF_16LE
                };
                let (text, errors) = enc.decode_without_bom_handling(body);
                (text.into_owned(), errors)
            }
            TextEncoding::Legacy(e) => {
                let (text, errors) = e.decode_without_bom_handling(body);
                (text.into_owned(), errors)
            }
        }
    }

    /// Encodes `text` (BOM included). Fails with the first character the
    /// encoding cannot represent.
    pub fn encode(self, text: &str) -> Result<Vec<u8>, char> {
        let mut out = self.bom().to_vec();
        match self {
            TextEncoding::Utf8 { .. } => out.extend_from_slice(text.as_bytes()),
            TextEncoding::Utf16 { big_endian, .. } => {
                out.reserve(text.len() * 2);
                for unit in text.encode_utf16() {
                    let b = if big_endian {
                        unit.to_be_bytes()
                    } else {
                        unit.to_le_bytes()
                    };
                    out.extend_from_slice(&b);
                }
            }
            TextEncoding::Legacy(e) => {
                let mut encoder = e.new_encoder();
                let cap = encoder
                    .max_buffer_length_from_utf8_without_replacement(text.len())
                    .unwrap_or(text.len() * 4);
                let mut buf = vec![0u8; cap];
                let (result, read, written) =
                    encoder.encode_from_utf8_without_replacement(text, &mut buf, true);
                match result {
                    encoding_rs::EncoderResult::InputEmpty => {
                        out.extend_from_slice(&buf[..written])
                    }
                    encoding_rs::EncoderResult::Unmappable(c) => return Err(c),
                    encoding_rs::EncoderResult::OutputFull => {
                        unreachable!("buffer sized for the worst case ({read} read)")
                    }
                }
            }
        }
        Ok(out)
    }

    pub fn is_utf8(self) -> bool {
        matches!(self, TextEncoding::Utf8 { .. })
    }
}

/// Recognises UTF-16 text without a byte-order mark: mostly-ASCII UTF-16 has
/// a zero in every other byte.
fn utf16_without_bom(bytes: &[u8]) -> Option<TextEncoding> {
    let sample = &bytes[..bytes.len().min(4096) & !1];
    if sample.len() < 4 {
        return None;
    }
    let pairs = sample.len() / 2;
    let even_zero = sample.iter().step_by(2).filter(|&&b| b == 0).count();
    let odd_zero = sample
        .iter()
        .skip(1)
        .step_by(2)
        .filter(|&&b| b == 0)
        .count();
    let mostly = |n: usize| n * 10 >= pairs * 7;
    let rarely = |n: usize| n * 20 <= pairs;
    if mostly(odd_zero) && rarely(even_zero) {
        Some(TextEncoding::Utf16 {
            big_endian: false,
            bom: false,
        })
    } else if mostly(even_zero) && rarely(odd_zero) {
        Some(TextEncoding::Utf16 {
            big_endian: true,
            bom: false,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_boms_and_utf8() {
        assert_eq!(
            TextEncoding::detect(b"\xEF\xBB\xBFhi", None),
            TextEncoding::Utf8 { bom: true }
        );
        assert_eq!(
            TextEncoding::detect(b"\xFF\xFEh\0i\0", None),
            TextEncoding::Utf16 {
                big_endian: false,
                bom: true
            }
        );
        assert_eq!(
            TextEncoding::detect("héllo".as_bytes(), None),
            TextEncoding::Utf8 { bom: false }
        );
        assert_eq!(
            TextEncoding::detect(b"\0h\0e\0l\0l\0o", None),
            TextEncoding::Utf16 {
                big_endian: true,
                bom: false
            }
        );
    }

    #[test]
    fn guesses_legacy_encodings() {
        // "Ça coûte très cher, déjà payé." in Windows-1252.
        let bytes = b"\xC7a co\xFBte tr\xE8s cher, d\xE9j\xE0 pay\xE9.";
        let e = TextEncoding::detect(bytes, None);
        assert_eq!(e, TextEncoding::Legacy(encoding_rs::WINDOWS_1252));
        assert_eq!(e.decode(bytes).0, "Ça coûte très cher, déjà payé.");
        // A hint the user chose wins over the guess.
        let koi = TextEncoding::Legacy(encoding_rs::KOI8_R);
        assert_eq!(TextEncoding::detect(bytes, Some(koi)), koi);
    }

    #[test]
    fn round_trips() {
        let text = "Grüße, мир! €";
        for e in [
            TextEncoding::Utf8 { bom: true },
            TextEncoding::Utf16 {
                big_endian: false,
                bom: true,
            },
            TextEncoding::Utf16 {
                big_endian: true,
                bom: false,
            },
        ] {
            let bytes = e.encode(text).unwrap();
            assert_eq!(e.decode(&bytes), (text.to_owned(), false), "{}", e.name());
        }
        let cp1252 = TextEncoding::Legacy(encoding_rs::WINDOWS_1252);
        assert_eq!(cp1252.encode("Grüße €").unwrap(), b"Gr\xFC\xDFe \x80");
        assert_eq!(cp1252.encode(text), Err('м'));
    }

    #[test]
    fn names_round_trip() {
        for e in TextEncoding::UNICODE {
            assert_eq!(TextEncoding::from_name(&e.name()), Some(e));
        }
        for (_, list) in legacy_groups() {
            for (label, enc) in list {
                assert_eq!(
                    TextEncoding::from_name(label),
                    Some(TextEncoding::Legacy(enc))
                );
            }
        }
        assert_eq!(
            TextEncoding::from_name("UTF-16 LE"),
            Some(TextEncoding::Utf16 {
                big_endian: false,
                bom: false
            })
        );
    }
}
