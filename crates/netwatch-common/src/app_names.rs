//! Resolve socket cgroup ids to app display names (API.md §3 `live.apps`,
//! Phase C C4). Validated in `spikes/ebpf-aya/`: group by the socket cgroup,
//! never by `current` (softirq). Unknown/scoped apps get a low-confidence
//! fallback; `.desktop` entries give a high-confidence name.

use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::LiveAppRate;

const CGROUP_ROOT: &str = "/sys/fs/cgroup";
const RESCAN_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone)]
struct Name {
    display: String,
    confidence: &'static str,
}

/// Thread-safe cgroup-id -> path/name cache with throttled rescans.
pub struct AppNames {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    paths: HashMap<u64, String>,
    last_scan: Option<Instant>,
    names: HashMap<u64, Name>,
    desktop: Vec<(String, String)>,
}

impl Default for AppNames {
    fn default() -> Self {
        Self {
            inner: Mutex::new(Inner {
                desktop: desktop_names(),
                ..Inner::default()
            }),
        }
    }
}

impl AppNames {
    /// Best-effort PID hint from the cgroup scope name (e.g.
    /// `app-motrix-31433.scope` -> 31433). Returns 0 when no id is present.
    /// This is a display hint only: measured in-kernel pid sources returned
    /// the softirq thread, so a missing hint must render as "—", never 0.
    #[must_use]
    pub fn pid_hint(&self, cgroup_id: u64) -> u32 {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let path = inner.path(cgroup_id).unwrap_or_default();
        let scope = path.rsplit('/').next().unwrap_or("");
        let stem = scope.strip_suffix(".scope").unwrap_or(scope);
        stem.rsplit('-')
            .next()
            .and_then(|part| part.parse::<u32>().ok())
            .unwrap_or(0)
    }

    /// Resolve one `AppUsage` row into a wire `LiveAppRate`.
    pub fn resolve(&self, cgroup_id: u64, down_bps: u64, up_bps: u64) -> LiveAppRate {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let name = inner.names.get(&cgroup_id).cloned().unwrap_or_else(|| {
            let path = inner.path(cgroup_id);
            let (display, confidence) = display_for(path.as_deref().unwrap_or(""), &inner.desktop);
            let resolved = Name {
                display,
                confidence,
            };
            inner.names.insert(cgroup_id, resolved.clone());
            resolved
        });
        LiveAppRate {
            cgroup_id,
            name: name.display,
            down_bps,
            up_bps,
            confidence: name.confidence.to_string(),
        }
    }
}

impl Inner {
    /// Cgroupfs inode == cgroup id; rescan at most once per second for new ids.
    fn path(&mut self, cgroup_id: u64) -> Option<String> {
        if cgroup_id <= 1 {
            return None;
        }
        if let Some(path) = self.paths.get(&cgroup_id) {
            return Some(path.clone());
        }
        if self
            .last_scan
            .is_none_or(|at| at.elapsed() >= RESCAN_INTERVAL)
        {
            let mut found = HashMap::new();
            walk(Path::new(CGROUP_ROOT), &mut found, 0);
            self.paths = found;
            self.last_scan = Some(Instant::now());
        }
        self.paths.get(&cgroup_id).cloned()
    }
}

fn walk(dir: &Path, out: &mut HashMap<u64, String>, depth: u32) {
    if depth > 8 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            out.insert(meta.ino(), path.to_string_lossy().into_owned());
        }
        walk(&path, out, depth + 1);
    }
}

/// `app-<name>-<pid>.scope` / `app-<name>@<hash>.service` -> `<name>` (high);
/// `.desktop` Exec/stem match -> desktop Name (high); everything else keeps
/// the scope leaf with `low` confidence (DESIGN.md: never invent an app name).
fn display_for(path: &str, desktop: &[(String, String)]) -> (String, &'static str) {
    if path.is_empty() || path == "/" {
        return ("Unattributed / system".to_string(), "low");
    }
    let leaf = Path::new(path).file_name().map_or_else(
        || path.to_string(),
        |name| name.to_string_lossy().into_owned(),
    );

    if let Some(rest) = leaf.strip_prefix("app-") {
        let mut name = rest.to_string();
        for suffix in [".scope", ".service"] {
            if let Some(stripped) = name.strip_suffix(suffix) {
                name = stripped.to_string();
            }
        }
        if let Some(index) = name.rfind(['-', '@']) {
            let tail = &name[index + 1..];
            if tail
                .chars()
                .all(|c| c.is_ascii_hexdigit() || c.is_ascii_digit())
            {
                name = name[..index].to_string();
            }
        }
        if let Some((_, display)) = desktop
            .iter()
            .find(|(hint, _)| hint.eq_ignore_ascii_case(&name))
        {
            return (display.clone(), "high");
        }
        if !name.is_empty() {
            return (name, "high");
        }
    }
    if let Some((_, display)) = desktop
        .iter()
        .find(|(hint, _)| hint.eq_ignore_ascii_case(&leaf))
    {
        return (display.clone(), "high");
    }
    (leaf, "low")
}

fn desktop_names() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut dirs = vec![PathBuf::from("/usr/share/applications")];
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(Path::new(&home).join(".local/share/applications"));
    }
    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            let mut display = None;
            let mut exec = None;
            for line in text.lines() {
                if display.is_none() && line.starts_with("Name=") {
                    display = Some(line[5..].trim().to_string());
                } else if exec.is_none() && line.starts_with("Exec=") {
                    exec = Some(line[5..].trim().to_string());
                }
            }
            let stem = path
                .file_stem()
                .map_or_default(|s| s.to_string_lossy().into_owned());
            if let Some((exe, display)) = exec.zip(display.clone()) {
                let first = exe.split_whitespace().next().unwrap_or("");
                let hint = Path::new(first)
                    .file_name()
                    .map_or_else(|| first.to_string(), |s| s.to_string_lossy().into_owned());
                out.push((hint, display));
            }
            if let Some(display) = display {
                out.push((stem, display));
            }
        }
    }
    out
}
