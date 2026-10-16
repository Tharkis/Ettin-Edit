//! Help > Check for Updates: asks GitHub for the latest release and compares
//! its version with this one, then offers to download and install it (see
//! [`crate::self_update`]). The check and the update run on background threads.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use egui::{Id, RichText};

use crate::self_update::{self, Asset, Install, Installed, Progress};
use crate::theme::Palette;

/// Where the latest release is read from.
pub const RELEASES_API: &str = "https://api.github.com/repos/Tharkis/Ettin-Edit/releases/latest";

/// The release API to ask. Debug builds read `ETTIN_UPDATE_API` so the updater
/// can be tried against a test server.
fn releases_api() -> String {
    #[cfg(debug_assertions)]
    if let Ok(url) = std::env::var("ETTIN_UPDATE_API") {
        return url;
    }
    RELEASES_API.to_owned()
}
/// Where people download releases.
pub const RELEASES_PAGE: &str = "https://github.com/Tharkis/Ettin-Edit/releases/latest";
/// Days between automatic checks.
pub const CHECK_EVERY_DAYS: i64 = 7;

#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub version: String,
    pub page: String,
    pub notes: String,
    pub assets: Vec<Asset>,
}

#[derive(Debug)]
pub enum Outcome {
    /// A newer release, and how this copy was installed.
    Newer(Release, Install),
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
        assets: value["assets"]
            .as_array()
            .map(|list| list.iter().filter_map(parse_asset).collect())
            .unwrap_or_default(),
    })
}

fn parse_asset(value: &serde_json::Value) -> Option<Asset> {
    Some(Asset {
        name: value["name"].as_str()?.to_owned(),
        url: value["browser_download_url"].as_str()?.to_owned(),
        size: value["size"].as_u64().unwrap_or(0),
        // GitHub lists each file's checksum as "sha256:<hex>".
        sha256: value["digest"]
            .as_str()
            .and_then(|d| d.strip_prefix("sha256:"))
            .filter(|h| h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit()))
            .map(str::to_ascii_lowercase),
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
        .get(releases_api())
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

/// Downloading and installing a release.
enum Stage {
    /// Offering the update.
    Offer,
    Working {
        progress: Arc<Progress>,
        rx: Receiver<Result<Installed, String>>,
    },
    Done(Result<Installed, String>),
}

#[derive(Default)]
pub struct UpdateCheck {
    rx: Option<Receiver<Outcome>>,
    /// Shown in the dialog; `None` when it's closed.
    shown: Option<Outcome>,
    /// The person asked, so "up to date" and errors are shown too.
    asked: bool,
    stage: Option<Stage>,
}

/// What the dialog asks the app to do.
pub enum UpdateEvent {
    OpenPage(String),
    /// Don't check by itself any more.
    StopChecking,
    /// Close Ettin Edit (asking about unsaved work as usual); what comes next
    /// is set with [`self_update::set_after_exit`].
    Quit,
}

impl UpdateCheck {
    /// Starts a check. `asked` is false for the automatic check, which only
    /// speaks up when there's a newer version.
    pub fn start(&mut self, ctx: &egui::Context, asked: bool) {
        if self.rx.is_some() || self.is_busy() {
            self.asked |= asked;
            return;
        }
        self.asked = asked;
        if asked {
            self.shown = None;
            self.stage = None;
        }
        let (tx, rx) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let current = env!("CARGO_PKG_VERSION");
            let outcome = match fetch() {
                Ok(r) if is_newer(&r.version, current) => Outcome::Newer(r, self_update::detect()),
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

    /// Downloading or installing.
    fn is_busy(&self) -> bool {
        matches!(self.stage, Some(Stage::Working { .. }))
    }

    /// Downloads and installs `asset` on a background thread.
    fn begin(&mut self, ctx: &egui::Context, asset: Asset, install: Install) {
        let progress = Arc::new(Progress::default());
        let (tx, rx) = channel();
        let shared = progress.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            // Repaint while the download runs, so the progress bar moves.
            let ticker = {
                let ctx = ctx.clone();
                let shared = shared.clone();
                std::thread::spawn(move || {
                    while !shared.cancel.load(Ordering::Relaxed) && Arc::strong_count(&shared) > 2 {
                        ctx.request_repaint();
                        std::thread::sleep(Duration::from_millis(150));
                    }
                })
            };
            let result = self_update::download(&asset, &shared).and_then(|file| {
                shared.installing.store(true, Ordering::Relaxed);
                ctx.request_repaint();
                self_update::install(&install, &file)
            });
            let _ = tx.send(result);
            drop(shared);
            ctx.request_repaint();
            let _ = ticker.join();
        });
        self.stage = Some(Stage::Working { progress, rx });
    }

    pub fn ui(&mut self, ctx: &egui::Context, palette: &Palette) -> Vec<UpdateEvent> {
        let mut events = Vec::new();
        if let Some(rx) = &self.rx
            && let Ok(outcome) = rx.try_recv()
        {
            self.rx = None;
            if self.asked || matches!(outcome, Outcome::Newer(..)) {
                self.shown = Some(outcome);
                self.stage = Some(Stage::Offer);
            }
        }
        if let Some(Stage::Working { rx, .. }) = &self.stage
            && let Ok(result) = rx.try_recv()
        {
            self.stage = Some(Stage::Done(result));
        }
        if !self.is_open() {
            return events;
        }
        let mut close = false;
        let mut start = None;
        egui::Modal::new(Id::new("update-check")).show(ctx, |ui| {
            ui.set_width(440.0);
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
                Some(Outcome::Newer(release, install)) => {
                    ui.label(
                        RichText::new(t!("update-available", version = release.version.clone()))
                            .color(palette.accent),
                    );
                    ui.label(t!("update-current", version = current));
                    let asset = self_update::pick_asset(&release.assets, install);
                    match &self.stage {
                        None | Some(Stage::Offer) => {
                            if !release.notes.is_empty() {
                                ui.add_space(6.0);
                                egui::ScrollArea::vertical()
                                    .max_height(200.0)
                                    .show(ui, |ui| ui.label(RichText::new(&release.notes).small()));
                            }
                            ui.add_space(6.0);
                            let how = match (install, asset) {
                                (Install::Manual(reason), _) => reason.clone(),
                                (_, None) => t!("self-update-no-file"),
                                (install, Some(_)) => install.describe(),
                            };
                            ui.label(RichText::new(how).small().weak());
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if let Some(asset) = asset {
                                    if ui
                                        .button(RichText::new(t!("update-now")).strong())
                                        .clicked()
                                    {
                                        start = Some((asset.clone(), install.clone()));
                                    }
                                    if ui.button(t!("update-release-notes")).clicked() {
                                        events.push(UpdateEvent::OpenPage(release.page.clone()));
                                    }
                                } else if ui.button(t!("open-download-page")).clicked() {
                                    events.push(UpdateEvent::OpenPage(release.page.clone()));
                                    close = true;
                                }
                                if !self.asked
                                    && ui.button(t!("dont-check-automatically")).clicked()
                                {
                                    events.push(UpdateEvent::StopChecking);
                                    close = true;
                                }
                            });
                        }
                        Some(Stage::Working { progress, .. }) => {
                            ui.add_space(8.0);
                            if progress.installing.load(Ordering::Relaxed) {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label(t!("self-update-installing"));
                                });
                                if matches!(install, Install::Arch | Install::Deb) {
                                    ui.label(
                                        RichText::new(t!("self-update-password")).small().weak(),
                                    );
                                }
                            } else {
                                let done = progress.done.load(Ordering::Relaxed);
                                let total = progress.total.load(Ordering::Relaxed).max(1);
                                let mb = |b: u64| format!("{:.1}", b as f64 / 1_048_576.0);
                                ui.add(egui::ProgressBar::new(done as f32 / total as f32).text(
                                    t!(
                                        "self-update-downloading",
                                        done = mb(done),
                                        total = mb(total)
                                    ),
                                ));
                                ui.add_space(4.0);
                                if ui.button(t!("cancel")).clicked() {
                                    progress.cancel.store(true, Ordering::Relaxed);
                                }
                            }
                        }
                        Some(Stage::Done(Ok(Installed::Restart))) => {
                            ui.add_space(6.0);
                            ui.label(t!(
                                "self-update-installed",
                                version = release.version.clone()
                            ));
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if ui
                                    .button(RichText::new(t!("restart-now")).strong())
                                    .clicked()
                                {
                                    self_update::restart_after_exit();
                                    events.push(UpdateEvent::Quit);
                                    close = true;
                                }
                            });
                        }
                        Some(Stage::Done(Ok(Installed::AfterExit))) => {
                            ui.add_space(6.0);
                            ui.label(t!("self-update-ready", version = release.version.clone()));
                            ui.add_space(8.0);
                            if ui
                                .button(RichText::new(t!("install-and-restart")).strong())
                                .clicked()
                            {
                                events.push(UpdateEvent::Quit);
                                close = true;
                            }
                        }
                        Some(Stage::Done(Ok(Installed::RunInTerminal(command)))) => {
                            ui.add_space(6.0);
                            ui.label(t!("self-update-run-in-terminal"));
                            ui.add(
                                egui::TextEdit::multiline(&mut command.as_str())
                                    .code_editor()
                                    .desired_rows(2)
                                    .desired_width(f32::INFINITY),
                            );
                            if ui.button(t!("copy")).clicked() {
                                ui.ctx().copy_text(command.clone());
                            }
                        }
                        Some(Stage::Done(Err(error))) => {
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new(t!("self-update-failed")).color(palette.brace_bad),
                            );
                            ui.label(RichText::new(error).small());
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if asset.is_some() && ui.button(t!("try-again")).clicked() {
                                    self.stage = Some(Stage::Offer);
                                }
                                if ui.button(t!("open-download-page")).clicked() {
                                    events.push(UpdateEvent::OpenPage(release.page.clone()));
                                }
                            });
                        }
                    }
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
            let busy = self.is_busy();
            let label = match &self.stage {
                Some(Stage::Done(Ok(Installed::Restart | Installed::AfterExit))) => t!("later"),
                _ => t!("close"),
            };
            if !busy
                && (ui.button(label).clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)))
            {
                close = true;
            }
        });
        if let Some((asset, install)) = start {
            self.begin(ctx, asset, install);
        }
        if close {
            self.shown = None;
            self.asked = false;
            self.stage = None;
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
    fn reads_the_files_and_checksums() {
        let json = r#"{"tag_name":"v1.8.0","assets":[
            {"name":"ettin-edit_1.8.0_amd64.deb","size":5,
             "browser_download_url":"https://example.com/a.deb",
             "digest":"sha256:65715C314046EF23DB367A29A23A3C8F673DFBF07D29731238F40E884A21A0FB"},
            {"name":"old.zip","size":1,"browser_download_url":"https://example.com/old.zip","digest":null},
            {"name":"bad.zip","browser_download_url":"https://example.com/b","digest":"sha256:xyz"},
            {"name":"no-url.zip"}]}"#;
        let r = parse_release(json).unwrap();
        assert_eq!(r.assets.len(), 3);
        assert_eq!(r.assets[0].size, 5);
        assert_eq!(
            r.assets[0].sha256.as_deref(),
            Some("65715c314046ef23db367a29a23a3c8f673dfbf07d29731238f40e884a21a0fb")
        );
        assert_eq!(r.assets[1].sha256, None);
        assert_eq!(r.assets[2].sha256, None);
    }

    #[test]
    fn reads_the_release() {
        let json = r#"{"tag_name":"v1.2.0","html_url":"https://example.com/r","body":" Notes \n"}"#;
        let r = parse_release(json).unwrap();
        assert_eq!(r.version, "1.2.0");
        assert_eq!(r.page, "https://example.com/r");
        assert_eq!(r.notes, "Notes");
        assert!(r.assets.is_empty());
        assert!(parse_release("{}").is_err());
    }
}
