//! Opening the window maximized without a flicker on Windows (issue #30).
//!
//! eframe remembers that the window was maximized and asks winit to create it
//! maximized but hidden (it shows the window after the first frame is
//! painted). On Windows, winit maximizes with `ShowWindow(SW_MAXIMIZE)`, which
//! also shows the window, so it popped up empty, vanished again and then came
//! back: a visible flicker on every start.
//!
//! Instead, the saved maximized state is taken off the window builder
//! ([`take_maximized`]) so the window is created hidden at its normal size.
//! On the first frame [`StartupWindow::update`] cloaks the window with DWM
//! (Windows draws nothing for a cloaked window, not even its maximize
//! animation), maximizes it, and uncloaks it once a frame has been painted at
//! the maximized size. Other platforms are left as they were.

use std::sync::atomic::{AtomicBool, Ordering};

use eframe::egui;

/// Set by [`take_maximized`] when the saved window was maximized.
static START_MAXIMIZED: AtomicBool = AtomicBool::new(false);

/// Frames to wait for the window to report it is maximized before showing it
/// anyway, so a window manager that ignores the request can't leave it hidden.
const MAX_WAIT_FRAMES: u32 = 30;

/// eframe `window_builder` hook: on Windows, opens a window that was saved
/// maximized at its normal size and leaves the maximizing to
/// [`StartupWindow`].
pub fn take_maximized(builder: egui::ViewportBuilder) -> egui::ViewportBuilder {
    if !cfg!(windows) {
        return builder;
    }
    let (builder, maximized) = strip_maximized(builder);
    START_MAXIMIZED.store(maximized, Ordering::Relaxed);
    builder
}

/// Removes a saved maximized state from `builder` (not when it is full
/// screen, which wins anyway) and says whether there was one.
fn strip_maximized(builder: egui::ViewportBuilder) -> (egui::ViewportBuilder, bool) {
    if builder.maximized == Some(true) && builder.fullscreen != Some(true) {
        (builder.with_maximized(false), true)
    } else {
        (builder, false)
    }
}

#[derive(Debug, PartialEq)]
enum Stage {
    /// Nothing to do.
    Done,
    /// Cloak, size and maximize on the next frame.
    Start,
    /// Cloaked and maximizing; `painted` once a frame was drawn maximized.
    Waiting { frames: u32, painted: bool },
}

pub struct StartupWindow {
    stage: Stage,
}

impl StartupWindow {
    pub fn new() -> Self {
        let stage = if START_MAXIMIZED.load(Ordering::Relaxed) {
            Stage::Start
        } else {
            Stage::Done
        };
        Self { stage }
    }

    /// Call at the start of every frame, before anything is drawn.
    pub fn update(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        match &mut self.stage {
            Stage::Done => {}
            Stage::Start => {
                let cloaked = cloak(frame, true);
                let (monitor, outer) =
                    ctx.input(|i| (i.viewport().monitor_size, i.viewport().outer_rect));
                // eframe saved the maximized size as the window size, so
                // restoring the window would leave it covering the screen.
                // Give it the first-launch size, centred where it is now
                // (that keeps it on the same monitor).
                if let (Some(monitor), Some(outer)) = (monitor, outer) {
                    let size = crate::app::first_window_size(monitor);
                    if size.x < outer.width() && size.y < outer.height() {
                        let pos = outer.min + (outer.size() - size) / 2.0;
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
                        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
                    }
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
                self.stage = if cloaked {
                    Stage::Waiting {
                        frames: 0,
                        painted: false,
                    }
                } else {
                    Stage::Done
                };
                ctx.request_repaint();
            }
            Stage::Waiting { frames, painted } => {
                *frames += 1;
                // The previous frame was painted maximized: safe to show.
                if *painted || *frames > MAX_WAIT_FRAMES {
                    cloak(frame, false);
                    self.stage = Stage::Done;
                    return;
                }
                *painted = ctx.input(|i| i.viewport().maximized == Some(true));
                ctx.request_repaint();
            }
        }
    }
}

/// Cloaks (hides without unmapping) or uncloaks the window. Returns whether
/// it worked.
#[cfg(windows)]
#[allow(unsafe_code)]
fn cloak(frame: &eframe::Frame, on: bool) -> bool {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmSetWindowAttribute(
            hwnd: isize,
            attribute: u32,
            value: *const core::ffi::c_void,
            size: u32,
        ) -> i32;
    }
    const DWMWA_CLOAK: u32 = 13;

    let Ok(handle) = frame.window_handle() else {
        return false;
    };
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return false;
    };
    let value: i32 = on.into();
    // SAFETY: the handle is our live window and `value` is a BOOL that
    // outlives the call.
    let result = unsafe {
        DwmSetWindowAttribute(
            win32.hwnd.get(),
            DWMWA_CLOAK,
            (&value as *const i32).cast(),
            std::mem::size_of::<i32>() as u32,
        )
    };
    result >= 0
}

#[cfg(not(windows))]
fn cloak(_frame: &eframe::Frame, _on: bool) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_maximized_window_opens_at_its_normal_size() {
        let builder = egui::ViewportBuilder::default()
            .with_inner_size([1920.0, 1040.0])
            .with_maximized(true);
        let (builder, maximized) = strip_maximized(builder);
        assert!(maximized);
        assert_eq!(builder.maximized, Some(false));
        assert_eq!(builder.inner_size, Some(egui::vec2(1920.0, 1040.0)));
    }

    #[test]
    fn other_windows_are_left_alone() {
        let (builder, maximized) = strip_maximized(egui::ViewportBuilder::default());
        assert!(!maximized);
        assert_eq!(builder.maximized, None);

        let normal = egui::ViewportBuilder::default().with_maximized(false);
        assert!(!strip_maximized(normal).1);

        let full = egui::ViewportBuilder::default()
            .with_maximized(true)
            .with_fullscreen(true);
        let (full, maximized) = strip_maximized(full);
        assert!(!maximized);
        assert_eq!(full.maximized, Some(true));
    }
}
