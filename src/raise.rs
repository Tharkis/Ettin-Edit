//! Bringing the window to the front when Ettin Edit is started again with a
//! file (issue #36).
//!
//! The running window can't do this from its own frame loop: eframe skips the
//! app's `update` while the window is minimized (and on Wayland a minimized
//! window gets no frames at all), so the files handed over by
//! [`crate::instance`] would sit there until the user restored the window by
//! hand. Instead the instance listener thread calls [`Raiser::raise`] as soon
//! as a hand-over arrives, which talks to the window system directly:
//!
//! * Windows: the starting instance calls `AllowSetForegroundWindow` for the
//!   running one ([`allow_foreground`]); Windows only lets a process take the
//!   foreground when the user just interacted with it, and the double-clicked
//!   process is the one that has that right. The running instance then
//!   restores the window if it is minimized and calls `SetForegroundWindow`.
//! * X11: an EWMH `_NET_ACTIVE_WINDOW` request, which also un-minimizes. When
//!   the starting instance was launched with a startup-notification id, its
//!   time stamp is passed on so focus-stealing prevention lets it through.
//! * Wayland: windows can only take focus with an activation token from the
//!   compositor. The launcher (file manager, desktop menu) gives the starting
//!   instance one in `XDG_ACTIVATION_TOKEN`; it is passed on and used with
//!   `xdg_activation_v1.activate` on the running window's surface.

/// Activation details the starting instance passes to the running one.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Activation {
    /// Wayland `XDG_ACTIVATION_TOKEN`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wayland_token: Option<String>,
    /// X11 startup-notification id (`DESKTOP_STARTUP_ID`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub startup_id: Option<String>,
}

impl Activation {
    /// What this process was launched with.
    pub fn from_env() -> Self {
        let var = |name| std::env::var(name).ok().filter(|v: &String| !v.is_empty());
        Self {
            wayland_token: var("XDG_ACTIVATION_TOKEN"),
            startup_id: var("DESKTOP_STARTUP_ID"),
        }
    }
}

/// The X server time stamp in a startup-notification id such as
/// `dolphin-1234-host-_TIME98765`, or 0 when it has none.
#[cfg_attr(not(all(unix, not(target_os = "macos"))), allow(dead_code))]
pub fn startup_time(id: &str) -> u32 {
    id.rfind("_TIME")
        .and_then(|i| {
            let digits: String = id[i + 5..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            digits.parse().ok()
        })
        .unwrap_or(0)
}

/// Gives the running instance (process `pid`) the right to take the
/// foreground. Does nothing outside Windows.
#[cfg(windows)]
#[allow(unsafe_code)]
pub fn allow_foreground(pid: u32) {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn AllowSetForegroundWindow(pid: u32) -> i32;
    }
    // SAFETY: plain Win32 call with no pointers.
    unsafe {
        AllowSetForegroundWindow(pid);
    }
}

#[cfg(not(windows))]
pub fn allow_foreground(_pid: u32) {}

/// The main window, as something the instance thread can raise.
pub enum Raiser {
    None,
    #[cfg(windows)]
    Win32 {
        hwnd: isize,
    },
    #[cfg(all(unix, not(target_os = "macos")))]
    X11 {
        window: u32,
    },
    #[cfg(all(unix, not(target_os = "macos")))]
    Wayland {
        display: usize,
        surface: usize,
    },
}

// The Wayland pointers stay valid for as long as the window exists, which is
// as long as the app (and so the instance listener) runs.
#[allow(unsafe_code)]
unsafe impl Send for Raiser {}
#[allow(unsafe_code)]
unsafe impl Sync for Raiser {}

impl Raiser {
    /// Reads the native window from eframe's creation context.
    pub fn new(
        handles: &(impl raw_window_handle::HasWindowHandle + raw_window_handle::HasDisplayHandle),
    ) -> Self {
        #[allow(unused_imports)]
        use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
        let (Ok(window), Ok(display)) = (handles.window_handle(), handles.display_handle()) else {
            return Self::None;
        };
        #[allow(unreachable_patterns)]
        match (window.as_raw(), display.as_raw()) {
            #[cfg(windows)]
            (RawWindowHandle::Win32(w), _) => Self::Win32 { hwnd: w.hwnd.get() },
            #[cfg(all(unix, not(target_os = "macos")))]
            (RawWindowHandle::Xlib(w), _) => Self::X11 {
                window: w.window as u32,
            },
            #[cfg(all(unix, not(target_os = "macos")))]
            (RawWindowHandle::Xcb(w), _) => Self::X11 {
                window: w.window.get(),
            },
            #[cfg(all(unix, not(target_os = "macos")))]
            (RawWindowHandle::Wayland(w), RawDisplayHandle::Wayland(d)) => Self::Wayland {
                display: d.display.as_ptr() as usize,
                surface: w.surface.as_ptr() as usize,
            },
            _ => Self::None,
        }
    }

    /// Restores and focuses the window. Safe to call from any thread.
    pub fn raise(&self, activation: &Activation) {
        let _ = activation;
        match self {
            Self::None => {}
            #[cfg(windows)]
            Self::Win32 { hwnd } => win32::raise(*hwnd),
            #[cfg(all(unix, not(target_os = "macos")))]
            Self::X11 { window } => {
                let time = activation.startup_id.as_deref().map_or(0, startup_time);
                if let Err(e) = x11::raise(*window, time) {
                    eprintln!("ettin-edit: could not raise the window: {e}");
                }
            }
            #[cfg(all(unix, not(target_os = "macos")))]
            Self::Wayland { display, surface } => {
                if let Some(token) = &activation.wayland_token
                    && let Err(e) = wayland::activate(*display, *surface, token)
                {
                    eprintln!("ettin-edit: could not activate the window: {e}");
                }
            }
        }
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod win32 {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn IsIconic(hwnd: isize) -> i32;
        fn ShowWindowAsync(hwnd: isize, cmd: i32) -> i32;
        fn SetForegroundWindow(hwnd: isize) -> i32;
    }
    const SW_RESTORE: i32 = 9;

    pub fn raise(hwnd: isize) {
        // SAFETY: plain Win32 calls on our own window handle. The async
        // variant posts to the window's thread, so this thread never waits
        // on a busy UI thread.
        unsafe {
            if IsIconic(hwnd) != 0 {
                // Restores to the size it had, maximized or not.
                ShowWindowAsync(hwnd, SW_RESTORE);
            }
            SetForegroundWindow(hwnd);
        }
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod x11 {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{ClientMessageEvent, ConnectionExt as _, EventMask};

    /// Asks the window manager to activate `window` (EWMH: this also
    /// de-iconifies it and switches to its desktop).
    pub fn raise(window: u32, time: u32) -> Result<(), Box<dyn std::error::Error>> {
        let (conn, screen) = x11rb::connect(None)?;
        let root = conn.setup().roots[screen].root;
        let active = conn
            .intern_atom(false, b"_NET_ACTIVE_WINDOW")?
            .reply()?
            .atom;
        // Source 1 (an application) with the launch time when there is one;
        // otherwise 2, as a pager does for a request the user made, since
        // the user did just open a file with Ettin Edit.
        let source = if time != 0 { 1 } else { 2 };
        let event = ClientMessageEvent::new(32, window, active, [source, time, 0, 0, 0]);
        conn.send_event(
            false,
            root,
            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
            event,
        )?
        // Wait for the server: a request still buffered when the connection
        // drops is lost.
        .check()?;
        Ok(())
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
#[allow(unsafe_code)]
mod wayland {
    use wayland_backend::client::{Backend, ObjectId};
    use wayland_client::globals::{GlobalListContents, registry_queue_init};
    use wayland_client::protocol::wl_registry::WlRegistry;
    use wayland_client::protocol::wl_surface::WlSurface;
    use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
    use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1::XdgActivationV1;

    struct State;

    impl Dispatch<WlRegistry, GlobalListContents> for State {
        fn event(
            _: &mut Self,
            _: &WlRegistry,
            _: <WlRegistry as Proxy>::Event,
            _: &GlobalListContents,
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }

    impl Dispatch<XdgActivationV1, ()> for State {
        fn event(
            _: &mut Self,
            _: &XdgActivationV1,
            _: <XdgActivationV1 as Proxy>::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }

    /// Activates the window's surface with a token from the compositor, on
    /// winit's own Wayland connection (a surface only exists there) but on a
    /// private event queue.
    pub fn activate(
        display: usize,
        surface: usize,
        token: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // SAFETY: `display` is winit's live wl_display, which outlives the
        // app; a foreign backend never closes it.
        let backend = unsafe { Backend::from_foreign_display(display as *mut _) };
        let conn = Connection::from_backend(backend);
        let (globals, queue) = registry_queue_init::<State>(&conn)?;
        let qh = queue.handle();
        let activation: XdgActivationV1 = globals.bind(&qh, 1..=1, ())?;
        // SAFETY: `surface` is the main window's live wl_surface proxy.
        let id = unsafe { ObjectId::from_ptr(WlSurface::interface(), surface as *mut _) }?;
        let surface = WlSurface::from_id(&conn, id)?;
        activation.activate(token.to_owned(), &surface);
        activation.destroy();
        conn.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_ids_give_their_time_stamp() {
        assert_eq!(startup_time("dolphin-1234-host-_TIME98765"), 98765);
        assert_eq!(startup_time("xdg-open-5_TIME12_extra"), 12);
        assert_eq!(startup_time("no-time-here"), 0);
        assert_eq!(startup_time("_TIME"), 0);
    }

    #[test]
    fn activation_round_trips_and_old_messages_still_parse() {
        let a = Activation {
            wayland_token: Some("tok".into()),
            startup_id: None,
        };
        let text = serde_json::to_string(&a).unwrap();
        assert_eq!(text, r#"{"wayland_token":"tok"}"#);
        assert_eq!(serde_json::from_str::<Activation>(&text).unwrap(), a);
        assert_eq!(
            serde_json::from_str::<Activation>("{}").unwrap(),
            Activation::default()
        );
    }
}
