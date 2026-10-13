//! Starting an app reliably.
//!
//! Some graphics drivers crash FilmCraft and EffectCraft while their window is created (both
//! projects document this). A crash there can't be caught inside the app, so the start window
//! stays open, watches the new process and, if it ends before its window has settled, starts it
//! again with the next graphics backend (`WGPU_BACKEND`: DirectX 12, Vulkan, OpenGL). The
//! backend that worked is remembered per app, so the next start uses it right away.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

/// A window that keeps drawing this long means the start worked.
const SETTLE: Duration = Duration::from_secs(4);
/// No window after this long: the app is slow, not crashed; stop watching.
const GIVE_UP_WAITING: Duration = Duration::from_secs(90);

/// Graphics backends to try, in order. `None` leaves the choice to the app.
const BACKENDS: [Option<&str>; 4] = [None, Some("dx12"), Some("vulkan"), Some("gl")];

pub fn backend_label(backend: Option<&str>) -> &'static str {
    match backend {
        None => "automatisch",
        Some("dx12") => "DirectX 12",
        Some("vulkan") => "Vulkan",
        Some("gl") => "OpenGL",
        Some(_) => "eigene Einstellung",
    }
}

pub enum Progress {
    /// Still starting: attempt number (1-based), attempts in total, backend of this attempt.
    Starting { attempt: usize, of: usize, backend: Option<String> },
    /// The app's window is up and stayed up (or the app is just slow): close the start window.
    Done,
    /// Every attempt failed.
    Failed(String),
}

pub struct Starter {
    name: &'static str,
    key: &'static str,
    exe: PathBuf,
    args: Vec<OsString>,
    plan: Vec<Option<String>>,
    attempt: usize,
    child: Option<Child>,
    since: Instant,
    window_since: Option<Instant>,
    errors: Vec<String>,
}

impl Starter {
    /// `key` names the app in the remembered settings (`filmcraft`, `effectcraft`).
    pub fn start(name: &'static str, key: &'static str, exe: PathBuf, args: Vec<OsString>) -> Result<Starter, String> {
        if !exe.is_file() {
            return Err(format!("{name} wurde nicht gefunden:\n{}", exe.display()));
        }
        let mut s = Starter { name, key, exe, args, plan: plan_for(key), attempt: 0, child: None, since: Instant::now(), window_since: None, errors: Vec::new() };
        s.spawn_current()?;
        Ok(s)
    }

    fn spawn_current(&mut self) -> Result<(), String> {
        let backend = self.plan.get(self.attempt).cloned().flatten();
        let mut cmd = Command::new(&self.exe);
        cmd.args(&self.args);
        if let Some(dir) = self.exe.parent() {
            cmd.current_dir(dir);
        }
        if let Some(b) = &backend {
            cmd.env("WGPU_BACKEND", b);
        }
        let child = cmd.spawn().map_err(|e| format!("{} konnte nicht gestartet werden:\n{e}", self.name))?;
        self.child = Some(child);
        self.since = Instant::now();
        self.window_since = None;
        Ok(())
    }

    /// Call regularly; tells whether to keep waiting, close, or show an error.
    pub fn poll(&mut self) -> Progress {
        let Some(child) = self.child.as_mut() else { return Progress::Done };
        match child.try_wait() {
            Ok(Some(status)) => {
                // Opened, then closed normally by the user: nothing to retry.
                if self.window_since.is_some() && status.success() {
                    self.child = None;
                    return Progress::Done;
                }
                let backend = backend_label(self.plan.get(self.attempt).cloned().flatten().as_deref());
                let code = status.code().map_or_else(|| "abgebrochen".to_string(), |c| format!("Code {c}"));
                self.errors.push(format!("{backend}: {code}"));
                self.child = None;
                self.next_attempt()
            }
            Ok(None) => {
                let pid = child.id();
                if self.window_since.is_none() && has_visible_window(pid, self.since.elapsed()) {
                    self.window_since = Some(Instant::now());
                }
                let settled = self.window_since.is_some_and(|t| t.elapsed() >= SETTLE);
                if settled || self.since.elapsed() >= GIVE_UP_WAITING {
                    if settled {
                        remember(self.key, self.plan.get(self.attempt).cloned().flatten().as_deref());
                    }
                    self.child = None;
                    return Progress::Done;
                }
                self.progress()
            }
            Err(e) => {
                self.errors.push(e.to_string());
                self.child = None;
                Progress::Failed(self.report())
            }
        }
    }

    fn next_attempt(&mut self) -> Progress {
        while self.attempt + 1 < self.plan.len() {
            self.attempt += 1;
            match self.spawn_current() {
                Ok(()) => return self.progress(),
                Err(e) => self.errors.push(e),
            }
        }
        forget(self.key);
        Progress::Failed(self.report())
    }

    fn progress(&self) -> Progress {
        Progress::Starting { attempt: self.attempt + 1, of: self.plan.len(), backend: self.plan.get(self.attempt).cloned().flatten() }
    }

    fn report(&self) -> String {
        format!("{} ließ sich mit keinem Grafikmodus starten.\n{}", self.name, self.errors.join("\n"))
    }

    /// Stop watching (the start window is being closed); the app keeps running.
    pub fn detach(&mut self) {
        self.child = None;
    }
}

/// The order of backends for `key`: the one that worked last time first. A `WGPU_BACKEND`
/// the user set is respected as the only choice.
fn plan_for(key: &str) -> Vec<Option<String>> {
    if std::env::var_os("WGPU_BACKEND").is_some_and(|v| !v.is_empty()) {
        return vec![None];
    }
    let mut plan: Vec<Option<String>> = Vec::new();
    if let Some(known) = remembered(key) {
        plan.push(if known == "auto" { None } else { Some(known) });
    }
    for b in BACKENDS {
        let b = b.map(str::to_string);
        if !plan.contains(&b) {
            plan.push(b);
        }
    }
    plan
}

fn settings_file() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA").or_else(|| std::env::var_os("XDG_CONFIG_HOME")).map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("CraftSuite").join("start.txt"))
}

fn read_settings() -> Vec<(String, String)> {
    let Some(path) = settings_file() else { return Vec::new() };
    let Ok(text) = std::fs::read_to_string(path) else { return Vec::new() };
    text.lines().filter_map(|l| l.split_once('=')).map(|(k, v)| (k.trim().to_string(), v.trim().to_string())).collect()
}

fn write_settings(entries: &[(String, String)]) {
    let Some(path) = settings_file() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let text: String = entries.iter().map(|(k, v)| format!("{k}={v}\n")).collect();
    let _ = std::fs::write(path, text);
}

fn remembered(key: &str) -> Option<String> {
    read_settings().into_iter().find(|(k, _)| k == key).map(|(_, v)| v).filter(|v| matches!(v.as_str(), "auto" | "dx12" | "vulkan" | "gl"))
}

fn remember(key: &str, backend: Option<&str>) {
    let mut entries: Vec<(String, String)> = read_settings().into_iter().filter(|(k, _)| k != key).collect();
    entries.push((key.to_string(), backend.unwrap_or("auto").to_string()));
    write_settings(&entries);
}

fn forget(key: &str) {
    let entries: Vec<(String, String)> = read_settings().into_iter().filter(|(k, _)| k != key).collect();
    write_settings(&entries);
}

/// Whether process `pid` shows a visible top-level window.
#[cfg(windows)]
fn has_visible_window(pid: u32, _running: Duration) -> bool {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId, IsWindowVisible};
    use windows_sys::core::BOOL;

    struct Search {
        pid: u32,
        found: bool,
    }

    unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: `lparam` is the `Search` passed to `EnumWindows` below, alive for the call.
        let search = unsafe { &mut *(lparam as *mut Search) };
        let mut owner = 0u32;
        // SAFETY: plain Win32 calls on a window handle EnumWindows gave us.
        unsafe { GetWindowThreadProcessId(hwnd, &mut owner) };
        if owner == search.pid && unsafe { IsWindowVisible(hwnd) } != 0 {
            search.found = true;
            return 0;
        }
        1
    }

    let mut search = Search { pid, found: false };
    // SAFETY: the callback only touches `search`, which outlives the call.
    unsafe { EnumWindows(Some(visit), &mut search as *mut Search as LPARAM) };
    search.found
}

/// Elsewhere (development on Linux / macOS): running for a few seconds counts as a window.
#[cfg(not(windows))]
fn has_visible_window(_pid: u32, running: Duration) -> bool {
    running >= Duration::from_secs(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_tries_every_backend_once() {
        let plan = BACKENDS.iter().map(|b| b.map(str::to_string)).collect::<Vec<_>>();
        let mut seen = plan.clone();
        seen.dedup();
        assert_eq!(plan.len(), seen.len());
        assert_eq!(plan[0], None);
    }

    #[test]
    fn labels() {
        assert_eq!(backend_label(Some("gl")), "OpenGL");
        assert_eq!(backend_label(None), "automatisch");
    }
}
