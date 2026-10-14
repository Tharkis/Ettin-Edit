//! MIME Tools: Base64, URL (percent) encoding, quoted-printable, HTML
//! entities and SAML message decoding.

use serde::{Deserialize, Serialize};

use super::{ToolError, ToolResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MimeOp {
    Base64Encode,
    /// Base64 with a line break every 76 characters, as in email.
    Base64EncodeWrapped,
    /// The URL and file name safe alphabet (`-` and `_`), without padding.
    Base64UrlEncode,
    /// Either alphabet, with or without padding and line breaks.
    Base64Decode,
    /// Encodes everything but letters, digits and `-_.~` (for one URL part).
    UrlEncodeComponent,
    /// Keeps the characters that give a whole URL its structure.
    UrlEncode,
    UrlDecode,
    QpEncode,
    QpDecode,
    /// `& < > " '` only.
    HtmlEncode,
    /// Also every non-ASCII character, by name where it has one.
    HtmlEncodeAll,
    HtmlDecode,
    SamlDecode,
}

impl MimeOp {
    pub fn run(self, text: &str) -> ToolResult {
        match self {
            MimeOp::Base64Encode => Ok(base64_encode(text.as_bytes(), STANDARD, true, None)),
            MimeOp::Base64EncodeWrapped => {
                Ok(base64_encode(text.as_bytes(), STANDARD, true, Some(76)))
            }
            MimeOp::Base64UrlEncode => Ok(base64_encode(text.as_bytes(), URL_SAFE, false, None)),
            MimeOp::Base64Decode => utf8(base64_decode(text)?),
            MimeOp::UrlEncodeComponent => Ok(url_encode(text, false)),
            MimeOp::UrlEncode => Ok(url_encode(text, true)),
            MimeOp::UrlDecode => utf8(url_decode(text)),
            MimeOp::QpEncode => Ok(qp_encode(text)),
            MimeOp::QpDecode => utf8(qp_decode(text)),
            MimeOp::HtmlEncode => Ok(html_encode(text, false)),
            MimeOp::HtmlEncodeAll => Ok(html_encode(text, true)),
            MimeOp::HtmlDecode => Ok(html_decode(text)),
            MimeOp::SamlDecode => saml_decode(text),
        }
    }
}

fn utf8(bytes: Vec<u8>) -> ToolResult {
    String::from_utf8(bytes).map_err(|e| {
        ToolError::new(format!(
            "The decoded data isn't UTF-8 text (invalid byte at position {}).",
            e.utf8_error().valid_up_to()
        ))
    })
}

// ----------------------------------------------------------------------
// Base64

const STANDARD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL_SAFE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

pub fn base64_encode(data: &[u8], alphabet: &[u8; 64], pad: bool, wrap: Option<usize>) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let n = chunk.len();
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let v = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= n {
                out.push(alphabet[(v >> (18 - 6 * i) & 63) as usize] as char);
            } else if pad {
                out.push('=');
            }
        }
    }
    match wrap {
        Some(width) if out.len() > width => {
            let mut wrapped = String::with_capacity(out.len() + out.len() / width);
            for (i, line) in out.as_bytes().chunks(width).enumerate() {
                if i > 0 {
                    wrapped.push('\n');
                }
                // The output is ASCII, so any split is on a char boundary.
                wrapped.push_str(std::str::from_utf8(line).unwrap_or_default());
            }
            wrapped
        }
        _ => out,
    }
}

fn base64_value(c: u8) -> Option<u32> {
    Some(match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' | b'-' => 62,
        b'/' | b'_' => 63,
        _ => return None,
    } as u32)
}

/// Decodes either alphabet; whitespace is skipped and padding is optional.
pub fn base64_decode(text: &str) -> Result<Vec<u8>, ToolError> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let mut acc = 0u32;
    let mut bits = 0;
    let mut padding_at = None;
    for (i, c) in text.bytes().enumerate() {
        if c.is_ascii_whitespace() {
            continue;
        }
        if c == b'=' {
            padding_at.get_or_insert(i);
            continue;
        }
        let Some(v) = base64_value(c) else {
            return Err(ToolError::at(
                format!("'{}' isn't a Base64 character.", char_at(text, i)),
                i,
            ));
        };
        if let Some(p) = padding_at {
            return Err(ToolError::at("Base64 padding ('=') must come last.", p));
        }
        acc = acc << 6 | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    // Six leftover bits can't make a byte: one character too many or too few.
    if bits == 6 {
        return Err(ToolError::new(
            "The Base64 text is cut short: its length doesn't fit.",
        ));
    }
    Ok(out)
}

fn char_at(text: &str, i: usize) -> char {
    text[super::floor_char_boundary(text, i)..]
        .chars()
        .next()
        .unwrap_or('?')
}

// ----------------------------------------------------------------------
// URL encoding

fn is_unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"-_.~".contains(&b)
}

/// Percent-encodes `text` as UTF-8. With `whole_url`, the characters that
/// separate a URL's parts stay as they are.
pub fn url_encode(text: &str, whole_url: bool) -> String {
    let mut out = String::with_capacity(text.len());
    for &b in text.as_bytes() {
        if is_unreserved(b) || (whole_url && b";,/?:@&=+$!*'()#".contains(&b)) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn hex_value(b: u8) -> Option<u8> {
    (b as char).to_digit(16).map(|d| d as u8)
}

/// Decodes `%XX` sequences. A `%` not followed by two hex digits stays as it is.
pub fn url_decode(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h), Some(l)) = (hex_value(bytes[i + 1]), hex_value(bytes[i + 2]))
        {
            out.push(h << 4 | l);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

// ----------------------------------------------------------------------
// Quoted-printable

const QP_LINE: usize = 76;

/// Encodes text as quoted-printable (RFC 2045), keeping its line breaks and
/// wrapping long lines with soft breaks.
pub fn qp_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 11 / 10);
    for (n, line) in text.split('\n').enumerate() {
        if n > 0 {
            out.push('\n');
        }
        let line = line.strip_suffix('\r').unwrap_or(line);
        let bytes = line.as_bytes();
        let mut width = 0;
        for (i, &b) in bytes.iter().enumerate() {
            let last = i + 1 == bytes.len();
            let token = match b {
                b' ' | b'\t' if last => format!("={b:02X}"),
                b'=' => "=3D".to_owned(),
                33..=126 | b' ' | b'\t' => (b as char).to_string(),
                _ => format!("={b:02X}"),
            };
            // Leave room for the '=' of a soft break, unless this ends the line.
            let limit = if last { QP_LINE } else { QP_LINE - 1 };
            if width + token.len() > limit {
                out.push_str("=\n");
                width = 0;
            }
            width += token.len();
            out.push_str(&token);
        }
    }
    out
}

pub fn qp_decode(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    let lines: Vec<&str> = text.split('\n').collect();
    for (n, line) in lines.iter().enumerate() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        // Trailing whitespace is padding added in transit.
        let line = line.trim_end_matches([' ', '\t']);
        let (body, soft) = match line.strip_suffix('=') {
            Some(b) => (b, true),
            None => (line, false),
        };
        let bytes = body.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'='
                && let (Some(h), Some(l)) = (
                    bytes.get(i + 1).copied().and_then(hex_value),
                    bytes.get(i + 2).copied().and_then(hex_value),
                )
            {
                out.push(h << 4 | l);
                i += 3;
            } else {
                out.push(bytes[i]);
                i += 1;
            }
        }
        if !soft && n + 1 < lines.len() {
            out.push(b'\n');
        }
    }
    out
}

// ----------------------------------------------------------------------
// HTML entities

/// Names for U+00A0 to U+00FF, in order.
const LATIN1: [&str; 96] = [
    "nbsp", "iexcl", "cent", "pound", "curren", "yen", "brvbar", "sect", "uml", "copy", "ordf",
    "laquo", "not", "shy", "reg", "macr", "deg", "plusmn", "sup2", "sup3", "acute", "micro",
    "para", "middot", "cedil", "sup1", "ordm", "raquo", "frac14", "frac12", "frac34", "iquest",
    "Agrave", "Aacute", "Acirc", "Atilde", "Auml", "Aring", "AElig", "Ccedil", "Egrave", "Eacute",
    "Ecirc", "Euml", "Igrave", "Iacute", "Icirc", "Iuml", "ETH", "Ntilde", "Ograve", "Oacute",
    "Ocirc", "Otilde", "Ouml", "times", "Oslash", "Ugrave", "Uacute", "Ucirc", "Uuml", "Yacute",
    "THORN", "szlig", "agrave", "aacute", "acirc", "atilde", "auml", "aring", "aelig", "ccedil",
    "egrave", "eacute", "ecirc", "euml", "igrave", "iacute", "icirc", "iuml", "eth", "ntilde",
    "ograve", "oacute", "ocirc", "otilde", "ouml", "divide", "oslash", "ugrave", "uacute", "ucirc",
    "uuml", "yacute", "thorn", "yuml",
];

/// Greek capitals from U+0391, then small letters from U+03B1 (U+03A2 has no letter).
const GREEK: [&str; 25] = [
    "Alpha", "Beta", "Gamma", "Delta", "Epsilon", "Zeta", "Eta", "Theta", "Iota", "Kappa",
    "Lambda", "Mu", "Nu", "Xi", "Omicron", "Pi", "Rho", "", "Sigma", "Tau", "Upsilon", "Phi",
    "Chi", "Psi", "Omega",
];
const GREEK_SMALL: [&str; 25] = [
    "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta", "iota", "kappa",
    "lambda", "mu", "nu", "xi", "omicron", "pi", "rho", "sigmaf", "sigma", "tau", "upsilon", "phi",
    "chi", "psi", "omega",
];

/// The other named entities of HTML 4, plus `apos`.
const OTHER: &[(&str, u32)] = &[
    ("quot", 34),
    ("amp", 38),
    ("apos", 39),
    ("lt", 60),
    ("gt", 62),
    ("OElig", 338),
    ("oelig", 339),
    ("Scaron", 352),
    ("scaron", 353),
    ("Yuml", 376),
    ("fnof", 402),
    ("circ", 710),
    ("tilde", 732),
    ("thetasym", 977),
    ("upsih", 978),
    ("piv", 982),
    ("ensp", 8194),
    ("emsp", 8195),
    ("thinsp", 8201),
    ("zwnj", 8204),
    ("zwj", 8205),
    ("lrm", 8206),
    ("rlm", 8207),
    ("ndash", 8211),
    ("mdash", 8212),
    ("lsquo", 8216),
    ("rsquo", 8217),
    ("sbquo", 8218),
    ("ldquo", 8220),
    ("rdquo", 8221),
    ("bdquo", 8222),
    ("dagger", 8224),
    ("Dagger", 8225),
    ("bull", 8226),
    ("hellip", 8230),
    ("permil", 8240),
    ("prime", 8242),
    ("Prime", 8243),
    ("lsaquo", 8249),
    ("rsaquo", 8250),
    ("oline", 8254),
    ("frasl", 8260),
    ("euro", 8364),
    ("image", 8465),
    ("weierp", 8472),
    ("real", 8476),
    ("trade", 8482),
    ("alefsym", 8501),
    ("larr", 8592),
    ("uarr", 8593),
    ("rarr", 8594),
    ("darr", 8595),
    ("harr", 8596),
    ("crarr", 8629),
    ("lArr", 8656),
    ("uArr", 8657),
    ("rArr", 8658),
    ("dArr", 8659),
    ("hArr", 8660),
    ("forall", 8704),
    ("part", 8706),
    ("exist", 8707),
    ("empty", 8709),
    ("nabla", 8711),
    ("isin", 8712),
    ("notin", 8713),
    ("ni", 8715),
    ("prod", 8719),
    ("sum", 8721),
    ("minus", 8722),
    ("lowast", 8727),
    ("radic", 8730),
    ("prop", 8733),
    ("infin", 8734),
    ("ang", 8736),
    ("and", 8743),
    ("or", 8744),
    ("cap", 8745),
    ("cup", 8746),
    ("int", 8747),
    ("there4", 8756),
    ("sim", 8764),
    ("cong", 8773),
    ("asymp", 8776),
    ("ne", 8800),
    ("equiv", 8801),
    ("le", 8804),
    ("ge", 8805),
    ("sub", 8834),
    ("sup", 8835),
    ("nsub", 8836),
    ("sube", 8838),
    ("supe", 8839),
    ("oplus", 8853),
    ("otimes", 8855),
    ("perp", 8869),
    ("sdot", 8901),
    ("lceil", 8968),
    ("rceil", 8969),
    ("lfloor", 8970),
    ("rfloor", 8971),
    ("lang", 9001),
    ("rang", 9002),
    ("loz", 9674),
    ("spades", 9824),
    ("clubs", 9827),
    ("hearts", 9829),
    ("diams", 9830),
];

fn entity_name(c: char) -> Option<&'static str> {
    let cp = c as u32;
    match cp {
        0xA0..=0xFF => Some(LATIN1[(cp - 0xA0) as usize]),
        0x391..=0x3A9 => Some(GREEK[(cp - 0x391) as usize]).filter(|n| !n.is_empty()),
        0x3B1..=0x3C9 => Some(GREEK_SMALL[(cp - 0x3B1) as usize]),
        _ => OTHER.iter().find(|&&(_, v)| v == cp).map(|&(n, _)| n),
    }
}

fn entity_char(name: &str) -> Option<char> {
    if let Some(i) = LATIN1.iter().position(|&n| n == name) {
        return char::from_u32(0xA0 + i as u32);
    }
    if let Some(i) = GREEK.iter().position(|&n| n == name && !n.is_empty()) {
        return char::from_u32(0x391 + i as u32);
    }
    if let Some(i) = GREEK_SMALL.iter().position(|&n| n == name) {
        return char::from_u32(0x3B1 + i as u32);
    }
    OTHER
        .iter()
        .find(|&&(n, _)| n == name)
        .and_then(|&(_, v)| char::from_u32(v))
}

pub fn html_encode(text: &str, all: bool) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c if all && !c.is_ascii() => match entity_name(c) {
                Some(name) => {
                    out.push('&');
                    out.push_str(name);
                    out.push(';');
                }
                None => out.push_str(&format!("&#{};", c as u32)),
            },
            c => out.push(c),
        }
    }
    out
}

/// Decodes named and numeric character references. Unknown ones stay as they are.
pub fn html_decode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let decoded = rest[1..]
            .find(';')
            .filter(|&semi| semi > 0 && semi <= 32)
            .and_then(|semi| {
                let name = &rest[1..1 + semi];
                let c = if let Some(num) = name.strip_prefix('#') {
                    let n = match num.strip_prefix(['x', 'X']) {
                        Some(hex) => u32::from_str_radix(hex, 16).ok(),
                        None => num.parse().ok(),
                    };
                    n.and_then(char::from_u32)
                } else {
                    entity_char(name)
                };
                c.map(|c| (c, semi + 2))
            });
        match decoded {
            Some((c, len)) => {
                out.push(c);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

// ----------------------------------------------------------------------
// SAML

/// Decodes a SAML request or response: from a whole redirect URL, a query
/// parameter value, or the Base64 text of a POST, deflated or not. The XML
/// comes back pretty-printed.
pub fn saml_decode(text: &str) -> ToolResult {
    let mut value = text.trim();
    for key in ["SAMLRequest=", "SAMLResponse="] {
        if let Some(i) = value.find(key) {
            value = &value[i + key.len()..];
            value = value.split('&').next().unwrap_or(value);
            break;
        }
    }
    let unescaped;
    if value.contains('%') {
        unescaped = String::from_utf8_lossy(&url_decode(value)).into_owned();
        value = &unescaped;
    }
    let bytes = base64_decode(value).map_err(|e| ToolError {
        offset: None,
        message: format!("Not a SAML message: {}", e.message),
    })?;
    let xml = miniz_oxide::inflate::decompress_to_vec(&bytes)
        .ok()
        .and_then(|inflated| String::from_utf8(inflated).ok())
        .or_else(|| String::from_utf8(bytes).ok())
        .filter(|s| s.trim_start().starts_with('<'))
        .ok_or_else(|| ToolError::new("The decoded data isn't SAML XML."))?;
    Ok(super::xml::pretty_print(&xml, "  ", false).unwrap_or(xml))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trips() {
        for s in [
            "",
            "f",
            "fo",
            "foo",
            "foob",
            "fooba",
            "foobar",
            "héllo wörld ✓",
        ] {
            let enc = MimeOp::Base64Encode.run(s).unwrap();
            assert_eq!(MimeOp::Base64Decode.run(&enc).unwrap(), s);
            let url = MimeOp::Base64UrlEncode.run(s).unwrap();
            assert!(!url.contains('='));
            assert_eq!(MimeOp::Base64Decode.run(&url).unwrap(), s);
        }
        assert_eq!(MimeOp::Base64Encode.run("foobar").unwrap(), "Zm9vYmFy");
        assert_eq!(MimeOp::Base64Encode.run("fo").unwrap(), "Zm8=");
        assert_eq!(
            MimeOp::Base64UrlEncode.run("\u{fb}\u{ff}").unwrap(),
            "w7vDvw"
        );
        assert_eq!(MimeOp::Base64Decode.run("Zm9v\n YmFy").unwrap(), "foobar");
        let wrapped = MimeOp::Base64EncodeWrapped.run(&"x".repeat(100)).unwrap();
        assert_eq!(wrapped.lines().next().unwrap().len(), 76);
    }

    #[test]
    fn base64_errors_point_at_the_problem() {
        let e = MimeOp::Base64Decode.run("Zm9v!YmFy").unwrap_err();
        assert_eq!(e.offset, Some(4));
        assert!(MimeOp::Base64Decode.run("Zm9vY").is_err());
        assert!(MimeOp::Base64Decode.run("Zm=9v").is_err());
        assert!(
            MimeOp::Base64Decode.run("/w==").is_err(),
            "0xFF isn't UTF-8"
        );
    }

    #[test]
    fn url_encoding() {
        assert_eq!(
            MimeOp::UrlEncodeComponent.run("a b/c?d=é").unwrap(),
            "a%20b%2Fc%3Fd%3D%C3%A9"
        );
        assert_eq!(
            MimeOp::UrlEncode
                .run("https://x.org/a b?q=1&r=é#top")
                .unwrap(),
            "https://x.org/a%20b?q=1&r=%C3%A9#top"
        );
        assert_eq!(
            MimeOp::UrlDecode.run("a%20b%2fc%C3%A9%zz%4").unwrap(),
            "a b/cé%zz%4"
        );
    }

    #[test]
    fn quoted_printable() {
        let text = "Café = good \nnext";
        let enc = MimeOp::QpEncode.run(text).unwrap();
        assert_eq!(enc, "Caf=C3=A9 =3D good=20\nnext");
        assert_eq!(MimeOp::QpDecode.run(&enc).unwrap(), text);
        let long = "a".repeat(200);
        let enc = MimeOp::QpEncode.run(&long).unwrap();
        assert!(enc.lines().all(|l| l.len() <= 76));
        assert_eq!(MimeOp::QpDecode.run(&enc).unwrap(), long);
        assert_eq!(MimeOp::QpDecode.run("a=\r\nb=3d").unwrap(), "ab=");
    }

    #[test]
    fn html_entities() {
        assert_eq!(
            MimeOp::HtmlEncode
                .run("<a href=\"x\">Tom & 'Jé'</a>")
                .unwrap(),
            "&lt;a href=&quot;x&quot;&gt;Tom &amp; &#39;Jé&#39;&lt;/a&gt;"
        );
        assert_eq!(
            MimeOp::HtmlEncodeAll.run("é € Ω ✓").unwrap(),
            "&eacute; &euro; &Omega; &#10003;"
        );
        assert_eq!(
            MimeOp::HtmlDecode
                .run("&lt;p&gt; &eacute;&#233;&#xE9; &nbsp;&bogus; & x &amp;amp;")
                .unwrap(),
            "<p> ééé \u{a0}&bogus; & x &amp;"
        );
        for c in ['\u{a0}', 'ÿ', 'Α', 'ω', 'ς', '♦', '€'] {
            assert_eq!(entity_char(entity_name(c).unwrap()), Some(c));
        }
    }

    #[test]
    fn saml_redirect_and_post() {
        let xml = "<samlp:AuthnRequest xmlns:samlp=\"urn:x\" ID=\"a1\"><Issuer>me</Issuer></samlp:AuthnRequest>";
        let pretty = "<samlp:AuthnRequest xmlns:samlp=\"urn:x\" ID=\"a1\">\n  <Issuer>me</Issuer>\n</samlp:AuthnRequest>";
        // POST binding: plain Base64.
        let post = base64_encode(xml.as_bytes(), STANDARD, true, None);
        assert_eq!(saml_decode(&post).unwrap(), pretty);
        // Redirect binding: deflated, Base64, then URL-encoded in a query string.
        let deflated = miniz_oxide::deflate::compress_to_vec(xml.as_bytes(), 6);
        let b64 = base64_encode(&deflated, STANDARD, true, None);
        let url = format!(
            "https://idp.example/sso?SAMLRequest={}&RelayState=x",
            url_encode(&b64, false)
        );
        assert_eq!(saml_decode(&url).unwrap(), pretty);
        assert!(saml_decode("hello").is_err());
    }
}
