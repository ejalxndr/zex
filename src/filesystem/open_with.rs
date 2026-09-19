use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

#[derive(Clone, Debug)]
pub enum AppIcon {
    Raster(PathBuf),
    Svg(PathBuf),
}

#[derive(Clone, Debug)]
pub struct DesktopApp {
    pub name: String,
    pub command: String,
    pub icon: Option<AppIcon>,
    pub terminal: bool,
}

pub fn launch(path: &Path, app: &DesktopApp) -> std::io::Result<()> {
    if !app.terminal {
        return open::with_detached(path, app.command.clone());
    }

    let Some(terminal) = find_terminal_emulator() else {
        return Err(std::io::Error::other(format!(
            "{} needs a terminal, but none could be found",
            app.name
        )));
    };

    let mut command = std::process::Command::new(terminal);
    command.arg("-e").arg(&app.command).arg(path);
    spawn_detached(command)
}

pub fn open_paths_with(binary: &str, paths: &[PathBuf]) -> std::io::Result<()> {
    let mut command = std::process::Command::new(binary);
    command.args(paths);
    spawn_detached(command)
}

fn find_terminal_emulator() -> Option<String> {
    if let Ok(term) = std::env::var("TERMINAL")
        && !term.is_empty()
        && binary_exists(&term)
    {
        return Some(term);
    }

    [
        "x-terminal-emulator",
        "kitty",
        "alacritty",
        "wezterm",
        "foot",
        "konsole",
        "gnome-terminal",
        "xfce4-terminal",
        "terminator",
        "urxvt",
        "xterm",
    ]
    .into_iter()
    .find(|term| binary_exists(term))
    .map(str::to_string)
}

fn binary_exists(name: &str) -> bool {
    if name.contains('/') {
        return Path::new(name).exists();
    }
    let Some(path_var) = std::env::var_os("PATH") else { return false };
    std::env::split_paths(&path_var).any(|dir| dir.join(name).exists())
}

fn spawn_detached(mut command: std::process::Command) -> std::io::Result<()> {
    use std::process::Stdio;
    command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());

    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;
        command.pre_exec(|| {
            match libc::fork() {
                -1 => return Err(std::io::Error::last_os_error()),
                0 => (),
                _ => libc::_exit(0),
            }
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
    }

    command.spawn().map(|_| ())
}

pub fn discover_apps_for(path: &Path, timeout_ms: u64) -> Vec<DesktopApp> {
    let Some(mime) = query_mime_type(path, timeout_ms) else {
        return Vec::new();
    };

    let mut seen: HashSet<String> = HashSet::new();
    let mut apps = Vec::new();
    for dir in application_dirs() {
        collect_desktop_apps(&dir, &mime, &mut apps, &mut seen);
    }
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

fn query_mime_type(path: &Path, timeout_ms: u64) -> Option<String> {
    let dir = path.parent().unwrap_or(Path::new("/"));
    let cancel = AtomicBool::new(false);
    let output = crate::process::run_with_timeout(
        "xdg-mime",
        dir,
        &["query", "filetype", &path.to_string_lossy()],
        None,
        timeout_ms,
        &cancel,
    )?;
    if output.status_code != Some(0) {
        return None;
    }
    let mime = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!mime.is_empty()).then_some(mime)
}

fn application_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    match std::env::var_os("XDG_DATA_HOME") {
        Some(data_home) => dirs.push(PathBuf::from(data_home).join("applications")),
        None => {
            if let Some(home) = std::env::var_os("HOME") {
                dirs.push(PathBuf::from(home).join(".local/share/applications"));
            }
        }
    }

    let data_dirs = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
    for dir in data_dirs.split(':').filter(|dir| !dir.is_empty()) {
        dirs.push(PathBuf::from(dir).join("applications"));
    }

    dirs
}

fn collect_desktop_apps(dir: &Path, mime: &str, apps: &mut Vec<DesktopApp>, seen: &mut HashSet<String>) {
    let Ok(read_dir) = std::fs::read_dir(dir) else { return };
    for entry in read_dir.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_desktop_apps(&path, mime, apps, seen);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("desktop") {
            continue;
        }
        if let Some(app) = parse_desktop_entry(&path, mime)
            && seen.insert(app.command.clone())
        {
            apps.push(app);
        }
    }
}

fn parse_desktop_entry(path: &Path, mime: &str) -> Option<DesktopApp> {
    let contents = std::fs::read_to_string(path).ok()?;
    let mut in_entry_section = false;
    let mut name = None;
    let mut exec = None;
    let mut icon = None;
    let mut mime_types: Vec<String> = Vec::new();
    let mut no_display = false;
    let mut hidden = false;
    let mut terminal = false;

    for line in contents.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry_section = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry_section {
            continue;
        }
        if let Some(value) = line.strip_prefix("Name=") {
            if name.is_none() {
                name = Some(value.to_string());
            }
        } else if let Some(value) = line.strip_prefix("Exec=") {
            exec = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("Icon=") {
            icon = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("MimeType=") {
            mime_types.extend(value.split(';').filter(|entry| !entry.is_empty()).map(str::to_string));
        } else if line == "NoDisplay=true" {
            no_display = true;
        } else if line == "Hidden=true" {
            hidden = true;
        } else if line == "Terminal=true" {
            terminal = true;
        }
    }

    if no_display || hidden {
        return None;
    }
    if !mime_types.iter().any(|pattern| mime_matches(pattern, mime)) {
        return None;
    }

    let name = name?;
    let command = exec_binary(&exec?)?;
    let icon = icon.and_then(|name| resolve_icon(&name));
    Some(DesktopApp { name, command, icon, terminal })
}

fn resolve_icon(icon_name: &str) -> Option<AppIcon> {
    if icon_name.is_empty() {
        return None;
    }

    let direct = Path::new(icon_name);
    if direct.is_absolute() {
        return direct.exists().then(|| classify_icon_file(direct.to_path_buf()));
    }

    let mut icon_dirs = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        icon_dirs.push(PathBuf::from(&home).join(".local/share/icons"));
        icon_dirs.push(PathBuf::from(&home).join(".icons"));
    }
    icon_dirs.push(PathBuf::from("/usr/share/icons"));
    icon_dirs.push(PathBuf::from("/usr/local/share/icons"));

    let themes = ["hicolor", "Adwaita", "breeze", "Papirus", "Papirus-Dark"];
    let sizes = ["128x128", "96x96", "64x64", "48x48", "256x256", "512x512", "32x32", "22x22", "16x16"];

    for base in &icon_dirs {
        for theme in themes {
            for size in sizes {
                let candidate = base.join(theme).join(size).join("apps").join(format!("{icon_name}.png"));
                if candidate.exists() {
                    return Some(AppIcon::Raster(candidate));
                }
            }
        }
    }

    for ext in ["png", "xpm"] {
        let candidate = PathBuf::from("/usr/share/pixmaps").join(format!("{icon_name}.{ext}"));
        if candidate.exists() {
            return Some(AppIcon::Raster(candidate));
        }
    }

    for base in &icon_dirs {
        for theme in themes {
            let scalable = base.join(theme).join("scalable/apps").join(format!("{icon_name}.svg"));
            if scalable.exists() {
                return Some(AppIcon::Svg(scalable));
            }
            for size in sizes {
                let candidate = base.join(theme).join(size).join("apps").join(format!("{icon_name}.svg"));
                if candidate.exists() {
                    return Some(AppIcon::Svg(candidate));
                }
            }
        }
    }

    None
}

fn classify_icon_file(path: PathBuf) -> AppIcon {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("svg") => AppIcon::Svg(path),
        _ => AppIcon::Raster(path),
    }
}

fn mime_matches(pattern: &str, mime: &str) -> bool {
    if pattern == mime {
        return true;
    }
    match pattern.strip_suffix("/*") {
        Some(prefix) => mime.split('/').next() == Some(prefix),
        None => false,
    }
}

fn exec_binary(exec: &str) -> Option<String> {
    exec.split_whitespace()
        .find(|token| !token.starts_with('%'))
        .map(|token| token.trim_matches('"').to_string())
}
