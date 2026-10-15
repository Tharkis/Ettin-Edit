//! Help > Check for Updates: asks GitHub for the latest release and compares
//! its version with this one. Runs on a background thread.

use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use egui::{Id, RichText};

use crate::theme::Palette;

/// Where the latest release is read from.
pub const RELEASES_API: &str = "https://api.github.com/repos/Tharkis/Ettin-Edit/releases/latest";
/// Where people download releases.
pub const RELEASES_PAGE: &str = "https://github.com/Tharkis/Ettin-Edit/releases/latest";
/// Days between automatic checks.
pub const CHECK_EVERY_DAYS: i64 = 7;

#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub version: String,
    pub page: String,
    pub notes: String,
}

#[derive(Debug)]
pub enum Outcome {
    Newer(Release),
    UpToDate(String),
    Failed(String),
}

/// A version's numbers: "v1.2.3-beta" is [1, 2, 3].
fn numbers(version: &str) -> Vec<u64> {
    version
        .trim()
        .trim_start_matches(['v', 'V'])
        .split(['-', '+'])
        .next()
        .unwrap_or("")
        .split('.')
        .map(|p| p.parse().unwrap_or(0))
        .collect()
}

/// True when `candidate` is a later version than `current`.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    let (mut a, mut b) = (numbers(candidate), numbers(current));
    let len = a.len().max(b.len());
    a.resize(len, 0);
    b.resize(len, 0);
    a > b
}

/// Reads the release's version, page and notes from the GitHub API's JSON.
pub fn parse_release(json: &str) -> Result<Release, String> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| t!("update-bad-json", error = e.to_string()))?;
    let tag = value["tag_name"]
        .as_str()
        .ok_or_else(|| t!("update-no-version"))?;
    Ok(Release {
        version: tag.trim_start_matches(['v', 'V']).to_owned(),
        page: value["html_url"]
            .as_str()
            .unwrap_or(RELEASES_PAGE)
            .to_owned(),
        notes: value["body"].as_str().unwrap_or("").trim().to_owned(),
    })
}

/// An HTTP client that trusts the system's certificates (so it works behind
/// proxies that add their own) and gives up after `timeout`.
pub fn http_agent(timeout: Duration) -> ureq::Agent {
    use ureq::tls::{RootCerts, TlsConfig};
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .tls_config(
            TlsConfig::builder()
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .user_agent(concat!("Ettin-Edit/", env!("CARGO_PKG_VERSION")))
        .build()
        .new_agent()
}

fn fetch() -> Result<Release, String> {
    let mut response = http_agent(Duration::from_secs(15))
        .get(RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| match e {
            ureq::Error::StatusCode(404) => t!("update-not-found"),
            ureq::Error::StatusCode(code) => t!("update-http-error", code = code),
            other => t!("update-unreachable", error = other.to_string()),
        })?;
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|e| t!("update-read-failed", error = e.to_string()))?;
    parse_release(&body)
}

#[derive(Default)]
pub struct UpdateCheck {
    rx: Option<Receiver<Outcome>>,
    /// Shown in the dialog; `None` when it's closed.
    shown: Option<Outcome>,
    /// The person asked, so "up to date" and errors are shown too.
    asked: bool,
}

/// What the dialog asks the app to do.
pub enum UpdateEvent {
    OpenPage(String),
    /// Don't check by itself any more.
    StopChecking,
}

impl UpdateCheck {
    /// Starts a check. `asked` is false for the automatic check, which only
    /// speaks up when there's a newer version.
    pub fn start(&mut self, ctx: &egui::Context, asked: bool) {
        if self.rx.is_some() {
            self.asked |= asked;
            return;
        }
        self.asked = asked;
        if asked {
            self.shown = None;
        }
        let (tx, rx) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let current = env!("CARGO_PKG_VERSION");
            let outcome = match fetch() {
                Ok(r) if is_newer(&r.version, current) => Outcome::Newer(r),
                Ok(r) => Outcome::UpToDate(r.version),
                Err(e) => Outcome::Failed(e),
            };
            let _ = tx.send(outcome);
            ctx.request_repaint();
        });
        self.rx = Some(rx);
    }

    pub fn is_open(&self) -> bool {
        self.shown.is_some() || (self.asked && self.rx.is_some())
    }

    pub fn ui(&mut self, ctx: &egui::Context, palette: &Palette) -> Vec<UpdateEvent> {
        let mut events = Vec::new();
        if let Some(rx) = &self.rx
            && let Ok(outcome) = rx.try_recv()
        {
            self.rx = None;
            if self.asked || matches!(outcome, Outcome::Newer(_)) {
                self.shown = Some(outcome);
            }
        }
        if !self.is_open() {
            return events;
        }
        let mut close = false;
        egui::Modal::new(Id::new("update-check")).show(ctx, |ui| {
            ui.set_width(420.0);
            ui.label(
                RichText::new(t!("check-for-updates-title"))
                    .strong()
                    .size(16.0),
            );
            ui.add_space(6.0);
            let current = env!("CARGO_PKG_VERSION");
            match &self.shown {
                None => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(t!("update-asking"));
                    });
                }
                Some(Outcome::Newer(release)) => {
                    ui.label(
                        RichText::new(t!("update-available", version = release.version.clone()))
                            .color(palette.accent),
                    );
                    ui.label(t!("update-current", version = current));
                    if !release.notes.is_empty() {
                        ui.add_space(6.0);
                        egui::ScrollArea::vertical()
                            .max_height(200.0)
                            .show(ui, |ui| ui.label(RichText::new(&release.notes).small()));
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button(t!("open-download-page")).clicked() {
                            events.push(UpdateEvent::OpenPage(release.page.clone()));
                            close = true;
                        }
                        if !self.asked && ui.button(t!("dont-check-automatically")).clicked() {
                            events.push(UpdateEvent::StopChecking);
                            close = true;
                        }
                    });
                }
                Some(Outcome::UpToDate(latest)) if latest == current => {
                    ui.label(t!("update-latest", version = current));
                }
                Some(Outcome::UpToDate(latest)) => {
                    ui.label(t!(
                        "update-newer-local",
                        version = current,
                        latest = latest.clone()
                    ));
                }
                Some(Outcome::Failed(error)) => {
                    ui.label(RichText::new(t!("update-failed")).color(palette.brace_bad));
                    ui.label(RichText::new(error).small());
                }
            }
            ui.add_space(8.0);
            if ui.button(t!("close")).clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                close = true;
            }
        });
        if close {
            self.shown = None;
            self.asked = false;
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions_by_number() {
        assert!(is_newer("1.0.1", "1.0.0"));
        assert!(is_newer("v1.10.0", "1.9.3"));
        assert!(is_newer("2", "1.99"));
        assert!(!is_newer("1.0.0", "1.0.0"));
        assert!(!is_newer("v1.0", "1.0.0"));
        assert!(!is_newer("0.10.0", "1.0.0"));
        assert!(!is_newer("1.0.0-beta", "1.0.0"));
    }

    #[test]
    fn reads_the_release() {
        let json = r#"{"tag_name":"v1.2.0","html_url":"https://example.com/r","body":" Notes \n"}"#;
        let r = parse_release(json).unwrap();
        assert_eq!(r.version, "1.2.0");
        assert_eq!(r.page, "https://example.com/r");
        assert_eq!(r.notes, "Notes");
        assert!(parse_release("{}").is_err());
    }
}
