//! Date patterns like `DD/MM/YYYY` or `YYYYMMDD HHmm`, for reading dates
//! written the way another system writes them.
//!
//! | Token | Means |
//! | --- | --- |
//! | `YYYY` / `YY` | year, four or two digits (`YY` 00–49 is 20xx) |
//! | `MM` / `M` | month number |
//! | `MMM` / `MMMM` | month name, short or long (English) |
//! | `DD` / `D` | day of the month |
//! | `HH` / `H` | hour, 0–23 |
//! | `hh` / `h` | hour, 1–12, with `A` (AM/PM) |
//! | `mm` | minutes |
//! | `ss` | seconds |
//! | `SSS` | fractions of a second (read and ignored) |
//! | `A` | AM or PM |
//!
//! Anything else must appear as it is. Numbers may have fewer digits than
//! their token when a separator follows (`D/M/YYYY` reads `2/3/1984`).

use chrono::{NaiveDate, NaiveDateTime};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Token {
    Year4,
    Year2,
    Month,
    MonthShort,
    MonthLong,
    Day,
    Hour24,
    Hour12,
    Minute,
    Second,
    Fraction,
    AmPm,
    Lit(char),
}

impl Token {
    /// The most digits the token reads, for numeric tokens.
    fn width(self) -> Option<usize> {
        Some(match self {
            Token::Year4 => 4,
            Token::Year2 | Token::Month | Token::Day => 2,
            Token::Hour24 | Token::Hour12 | Token::Minute | Token::Second => 2,
            Token::Fraction => 9,
            _ => return None,
        })
    }
}

const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

/// A parsed date pattern.
#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    tokens: Vec<Token>,
}

impl Pattern {
    pub fn new(pattern: &str) -> Result<Self, String> {
        let chars: Vec<char> = pattern.chars().collect();
        let mut tokens = Vec::new();
        let mut i = 0;
        let run = |i: usize, c: char| chars[i..].iter().take_while(|&&x| x == c).count();
        while i < chars.len() {
            let c = chars[i];
            let n = run(i, c);
            let (token, used) = match (c, n) {
                ('Y', 4..) => (Token::Year4, 4),
                ('Y', 2..) => (Token::Year2, 2),
                ('M', 4..) => (Token::MonthLong, 4),
                ('M', 3) => (Token::MonthShort, 3),
                ('M', _) => (Token::Month, n.min(2)),
                ('D', _) => (Token::Day, n.min(2)),
                ('H', _) => (Token::Hour24, n.min(2)),
                ('h', _) => (Token::Hour12, n.min(2)),
                ('m', _) => (Token::Minute, n.min(2)),
                ('s', _) => (Token::Second, n.min(2)),
                ('S', _) => (Token::Fraction, n),
                ('A' | 'a', _) => (Token::AmPm, 1),
                _ => (Token::Lit(c), 1),
            };
            tokens.push(token);
            i += used;
        }
        let has = |t: Token| tokens.contains(&t);
        let date = (has(Token::Year4) || has(Token::Year2))
            && (has(Token::Month) || has(Token::MonthShort) || has(Token::MonthLong))
            && has(Token::Day);
        if !date {
            return Err(t!("table-pattern-needs-date"));
        }
        if has(Token::Hour12) != has(Token::AmPm) {
            return Err(t!("table-pattern-ampm"));
        }
        Ok(Self { tokens })
    }

    /// Whether the pattern has a time of day.
    pub fn has_time(&self) -> bool {
        self.tokens
            .iter()
            .any(|t| matches!(t, Token::Hour24 | Token::Hour12))
    }

    /// Reads `text` (surrounding spaces ignored); `None` when it doesn't
    /// match or isn't a real date.
    pub fn parse(&self, text: &str) -> Option<NaiveDateTime> {
        let s: Vec<char> = text.trim().chars().collect();
        let mut at = 0;
        let (mut year, mut month, mut day) = (None, None, None);
        let (mut hour, mut minute, mut second, mut pm) = (0u32, 0u32, 0u32, None);
        for (k, token) in self.tokens.iter().enumerate() {
            match *token {
                Token::Lit(c) => {
                    if s.get(at) != Some(&c) {
                        return None;
                    }
                    at += 1;
                }
                Token::MonthShort | Token::MonthLong => {
                    let rest: String = s[at..].iter().collect::<String>().to_lowercase();
                    let (m, len) = MONTHS.iter().enumerate().find_map(|(i, name)| {
                        if *token == Token::MonthLong {
                            rest.starts_with(name).then_some((i, name.len()))
                        } else {
                            rest.starts_with(&name[..3]).then_some((i, 3))
                        }
                    })?;
                    month = Some(m as u32 + 1);
                    at += len;
                }
                Token::AmPm => {
                    let two: String = s.get(at..at + 2)?.iter().collect::<String>().to_lowercase();
                    pm = Some(match two.as_str() {
                        "am" => false,
                        "pm" => true,
                        _ => return None,
                    });
                    at += 2;
                }
                numeric => {
                    let width = numeric.width().unwrap_or(2);
                    // Next to another number, a token takes all its digits.
                    let numeric_at = |i: Option<usize>| {
                        i.and_then(|i| self.tokens.get(i))
                            .is_some_and(|t| t.width().is_some())
                    };
                    let next_numeric = numeric_at(Some(k + 1)) || numeric_at(k.checked_sub(1));
                    let mut len = 0;
                    while len < width && s.get(at + len).is_some_and(|c| c.is_ascii_digit()) {
                        len += 1;
                    }
                    if len == 0 || (next_numeric && len < width && numeric != Token::Fraction) {
                        return None;
                    }
                    let digits: String = s[at..at + len].iter().collect();
                    at += len;
                    if numeric == Token::Fraction {
                        continue;
                    }
                    let v: u32 = digits.parse().ok()?;
                    match numeric {
                        Token::Year4 => year = Some(v as i32),
                        Token::Year2 => {
                            year = Some(if v < 50 { 2000 + v as i32 } else { 1900 + v as i32 })
                        }
                        Token::Month => month = Some(v),
                        Token::Day => day = Some(v),
                        Token::Hour24 | Token::Hour12 => hour = v,
                        Token::Minute => minute = v,
                        Token::Second => second = v,
                        _ => {}
                    }
                }
            }
        }
        if at != s.len() {
            return None;
        }
        if let Some(pm) = pm {
            if !(1..=12).contains(&hour) {
                return None;
            }
            hour = match (hour, pm) {
                (12, false) => 0,
                (12, true) => 12,
                (h, true) => h + 12,
                (h, false) => h,
            };
        }
        NaiveDate::from_ymd_opt(year?, month?, day?)?.and_hms_opt(hour, minute, second)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(pattern: &str, text: &str) -> Option<String> {
        Pattern::new(pattern)
            .unwrap()
            .parse(text)
            .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
    }

    #[test]
    fn reads_common_layouts() {
        assert_eq!(p("DD/MM/YYYY", "02/03/1984").unwrap(), "1984-03-02 00:00:00");
        assert_eq!(p("DD/MM/YYYY", "2/3/1984").unwrap(), "1984-03-02 00:00:00");
        assert_eq!(p("M/D/YY", "12/30/67").unwrap(), "1967-12-30 00:00:00");
        assert_eq!(p("YYYYMMDD", "19840302").unwrap(), "1984-03-02 00:00:00");
        assert_eq!(
            p("YYYYMMDDHHmm", "202611030812").unwrap(),
            "2026-11-03 08:12:00"
        );
        assert_eq!(
            p("D MMM YYYY h:mm A", "3 Nov 2026 8:12 PM").unwrap(),
            "2026-11-03 20:12:00"
        );
        assert_eq!(
            p("DD MMMM YYYY", "03 november 2026").unwrap(),
            "2026-11-03 00:00:00"
        );
        assert_eq!(
            p("YYYY-MM-DDTHH:mm:ss.SSS", "2026-11-03T08:12:05.250").unwrap(),
            "2026-11-03 08:12:05"
        );
    }

    #[test]
    fn rejects_what_doesnt_fit() {
        assert_eq!(p("DD/MM/YYYY", "31/02/1979"), None);
        assert_eq!(p("DD/MM/YYYY", "1984-03-02"), None);
        assert_eq!(p("YYYYMMDD", "1984032"), None);
        assert_eq!(p("DD/MM/YYYY", "02/03/1984 extra"), None);
        assert!(Pattern::new("HH:mm").is_err());
        assert!(Pattern::new("DD/MM/YYYY hh:mm").is_err());
    }
}
