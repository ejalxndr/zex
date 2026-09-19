use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Default, Debug, PartialEq)]
pub struct Settings {
    pub icon_theme: Option<String>,
    pub ui_font_family: Option<String>,
    pub ui_font_size: Option<f32>,
    pub ui_font_weight: Option<f32>,
    pub show_hidden_files: Option<bool>,
    pub image_previews: Option<bool>,
    pub sidebar_visible: Option<bool>,
    #[serde(default)]
    pub sidebar: Vec<SidebarItem>,
    #[serde(default)]
    pub git: GitSettings,
    #[serde(default)]
    pub disk_usage: DiskUsageSettings,
    #[serde(default)]
    pub search: SearchSettings,
    #[serde(default)]
    pub theme: ThemeSettings,
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct ThemeSettings {
    pub mode: ThemeMode,
    pub light: Option<String>,
    pub dark: Option<String>,
}

#[derive(Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
    System,
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct GitSettings {
    pub enabled: bool,
    pub status: GitStatusSettings,
    pub branch: GitBranchSettings,
    pub refresh: GitRefreshSettings,
    pub cli: GitCliSettings,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GitBadgeStyle {
    TextColor,
    Icon,
    Both,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct GitStatusSettings {
    pub enabled: bool,
    pub show_untracked: bool,
    pub show_ignored: bool,
    pub dim_ignored: bool,
    pub aggregate_folders: bool,
    pub badge_style: GitBadgeStyle,
}

impl Default for GitStatusSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            show_untracked: true,
            show_ignored: false,
            dim_ignored: true,
            aggregate_folders: true,
            badge_style: GitBadgeStyle::TextColor,
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct GitBranchSettings {
    pub enabled: bool,
    pub show_in_status_bar: bool,
    pub show_dirty_indicator: bool,
    pub show_ahead_behind: bool,
}

impl Default for GitBranchSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            show_in_status_bar: true,
            show_dirty_indicator: true,
            show_ahead_behind: true,
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct GitRefreshSettings {
    pub poll_interval_ms: u64,
}

impl Default for GitRefreshSettings {
    fn default() -> Self {
        Self {
            poll_interval_ms: 2000,
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct GitCliSettings {
    pub binary_path: Option<String>,
    pub timeout_ms: u64,
    pub max_repo_entries: Option<u64>,
}

impl Default for GitCliSettings {
    fn default() -> Self {
        Self {
            binary_path: None,
            timeout_ms: 3000,
            max_repo_entries: None,
        }
    }
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct DiskUsageSettings {
    pub cross_filesystem_boundaries: bool,
    pub follow_symlinks: bool,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct SearchSettings {
    pub cli: SearchCliSettings,
    pub max_results: u32,
    pub default_case: CaseSensitivity,
    pub respect_gitignore: bool,
    pub include_hidden: bool,
}

impl Default for SearchSettings {
    fn default() -> Self {
        Self {
            cli: SearchCliSettings::default(),
            max_results: 500,
            default_case: CaseSensitivity::Smart,
            respect_gitignore: true,
            include_hidden: false,
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct SearchCliSettings {
    pub binary_path: Option<String>,
    pub timeout_ms: u64,
}

impl Default for SearchCliSettings {
    fn default() -> Self {
        Self {
            binary_path: None,
            timeout_ms: 5000,
        }
    }
}

#[derive(Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CaseSensitivity {
    Sensitive,
    Insensitive,
    #[default]
    Smart,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct SidebarEntry {
    pub name: String,
    pub path: String,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct SidebarSection {
    pub section: String,
    #[serde(default)]
    pub entries: Vec<SidebarEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeparatorMarker;

impl<'de> Deserialize<'de> for SeparatorMarker {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if value == "separator" {
            Ok(SeparatorMarker)
        } else {
            Err(serde::de::Error::custom(format!(
                "expected \"separator\", got \"{value}\""
            )))
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum SidebarItem {
    Section(SidebarSection),
    Entry(SidebarEntry),
    Separator(SeparatorMarker),
}

fn config_dir() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/"));
            home.join(".config")
        })
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}

fn default_zed_settings_path() -> PathBuf {
    config_dir().join("zed").join("settings.json")
}

pub(crate) fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        return home_dir().join(rest);
    }
    if path == "~" {
        return home_dir();
    }
    PathBuf::from(path)
}

fn resolve_inherit_path(zex_value: &Value) -> Option<PathBuf> {
    match zex_value.get("inherit_from_zed")? {
        Value::Bool(true) => Some(default_zed_settings_path()),
        Value::String(path) => Some(expand_tilde(path)),
        _ => None,
    }
}

fn strip_json_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;

    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if c == '\\' {
                if let Some(escaped) = chars.next() {
                    out.push(escaped);
                }
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }

        match c {
            '"' => {
                in_string = true;
                out.push(c);
            }
            '/' if chars.peek() == Some(&'/') => {
                chars.next();
                for c in chars.by_ref() {
                    if c == '\n' {
                        out.push(c);
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut prev = '\0';
                for c in chars.by_ref() {
                    if prev == '*' && c == '/' {
                        break;
                    }
                    prev = c;
                }
            }
            _ => out.push(c),
        }
    }

    out
}

fn strip_trailing_commas(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;

    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if c == '\\' {
                if let Some(escaped) = chars.next() {
                    out.push(escaped);
                }
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }

        match c {
            '"' => {
                in_string = true;
                out.push(c);
            }
            ',' => {
                let mut lookahead = chars.clone();
                let mut precedes_closer = false;
                while let Some(&next) = lookahead.peek() {
                    if next.is_whitespace() {
                        lookahead.next();
                        continue;
                    }
                    precedes_closer = next == '}' || next == ']';
                    break;
                }
                if !precedes_closer {
                    out.push(c);
                }
            }
            _ => out.push(c),
        }
    }

    out
}

fn parse_jsonc(contents: &str) -> serde_json::Result<Value> {
    let without_comments = strip_json_comments(contents);
    let without_trailing_commas = strip_trailing_commas(&without_comments);
    serde_json::from_str(&without_trailing_commas)
}

fn merge_json(base: &mut Value, overlay: Value) {
    match overlay {
        Value::Object(overlay_map) => {
            if let Value::Object(base_map) = base {
                for (key, value) in overlay_map {
                    match base_map.get_mut(&key) {
                        Some(existing) => merge_json(existing, value),
                        None => {
                            base_map.insert(key, value);
                        }
                    }
                }
            } else {
                *base = Value::Object(overlay_map);
            }
        }
        other => *base = other,
    }
}

fn read_jsonc_file(path: &Path) -> Option<Value> {
    let contents = std::fs::read_to_string(path).ok()?;
    parse_jsonc(&contents).ok()
}

pub fn load(config_override: Option<&Path>) -> Settings {
    let path = config_override
        .map(Path::to_path_buf)
        .unwrap_or_else(|| config_dir().join("zex").join("config.json"));

    let zex_value = read_jsonc_file(&path).unwrap_or_else(|| Value::Object(Default::default()));

    let merged = match resolve_inherit_path(&zex_value).and_then(|path| read_jsonc_file(&path)) {
        Some(mut zed_value) => {
            merge_json(&mut zed_value, zex_value);
            zed_value
        }
        None => zex_value,
    };

    serde_json::from_value(merged).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_icon_theme_field() {
        let contents = r#"{ "icon_theme": "Colored Zed Icons Theme Dark" }"#;

        let settings: Settings = serde_json::from_str(contents).unwrap();

        assert_eq!(
            settings.icon_theme.as_deref(),
            Some("Colored Zed Icons Theme Dark")
        );
    }

    #[test]
    fn empty_object_defaults_icon_theme_to_none() {
        let settings: Settings = serde_json::from_str("{}").unwrap();

        assert_eq!(settings.icon_theme, None);
        assert_eq!(settings.ui_font_family, None);
        assert_eq!(settings.ui_font_size, None);
        assert_eq!(settings.ui_font_weight, None);
        assert_eq!(settings.show_hidden_files, None);
        assert_eq!(settings.sidebar_visible, None);
        assert_eq!(settings.sidebar, Vec::<SidebarItem>::new());
    }

    #[test]
    fn parses_ui_font_family_field() {
        let contents = r#"{ "ui_font_family": "JetBrainsMono Nerd Font Mono" }"#;

        let settings: Settings = serde_json::from_str(contents).unwrap();

        assert_eq!(
            settings.ui_font_family.as_deref(),
            Some("JetBrainsMono Nerd Font Mono")
        );
    }

    #[test]
    fn parses_ui_font_size_and_weight_fields() {
        let contents = r#"{ "ui_font_size": 15.0, "ui_font_weight": 500.0 }"#;

        let settings: Settings = serde_json::from_str(contents).unwrap();

        assert_eq!(settings.ui_font_size, Some(15.0));
        assert_eq!(settings.ui_font_weight, Some(500.0));
    }

    #[test]
    fn parses_show_hidden_files_field() {
        let contents = r#"{ "show_hidden_files": true }"#;

        let settings: Settings = serde_json::from_str(contents).unwrap();

        assert_eq!(settings.show_hidden_files, Some(true));
    }

    #[test]
    fn parses_sidebar_visible_field() {
        let contents = r#"{ "sidebar_visible": false }"#;

        let settings: Settings = serde_json::from_str(contents).unwrap();

        assert_eq!(settings.sidebar_visible, Some(false));
    }

    #[test]
    fn parses_sidebar_entries() {
        let contents = r#"{
            "sidebar": [
                { "name": "Projects", "path": "/home/user/Projects" },
                { "name": "Root", "path": "/" }
            ]
        }"#;

        let settings: Settings = serde_json::from_str(contents).unwrap();

        assert_eq!(
            settings.sidebar,
            vec![
                SidebarItem::Entry(SidebarEntry {
                    name: "Projects".into(),
                    path: "/home/user/Projects".into(),
                }),
                SidebarItem::Entry(SidebarEntry {
                    name: "Root".into(),
                    path: "/".into(),
                }),
            ]
        );
    }

    #[test]
    fn parses_sidebar_separator() {
        let contents = r#"{
            "sidebar": [
                { "name": "Home", "path": "/home/shdw" },
                "separator",
                { "name": "Trash", "path": ":trash" }
            ]
        }"#;

        let settings: Settings = serde_json::from_str(contents).unwrap();

        assert_eq!(
            settings.sidebar,
            vec![
                SidebarItem::Entry(SidebarEntry {
                    name: "Home".into(),
                    path: "/home/shdw".into(),
                }),
                SidebarItem::Separator(SeparatorMarker),
                SidebarItem::Entry(SidebarEntry {
                    name: "Trash".into(),
                    path: ":trash".into(),
                }),
            ]
        );
    }

    #[test]
    fn rejects_unrecognized_sidebar_strings() {
        let contents = r#"{ "sidebar": ["not-a-separator"] }"#;

        assert!(serde_json::from_str::<Settings>(contents).is_err());
    }

    #[test]
    fn parses_sidebar_sections_with_nested_entries() {
        let contents = r#"{
            "sidebar": [
                { "name": "Root", "path": "/" },
                { "section": "Work", "entries": [
                    { "name": "Projects", "path": "/home/user/Projects" },
                    { "name": "Downloads", "path": "/home/user/Downloads" }
                ] }
            ]
        }"#;

        let settings: Settings = serde_json::from_str(contents).unwrap();

        assert_eq!(
            settings.sidebar,
            vec![
                SidebarItem::Entry(SidebarEntry {
                    name: "Root".into(),
                    path: "/".into(),
                }),
                SidebarItem::Section(SidebarSection {
                    section: "Work".into(),
                    entries: vec![
                        SidebarEntry {
                            name: "Projects".into(),
                            path: "/home/user/Projects".into(),
                        },
                        SidebarEntry {
                            name: "Downloads".into(),
                            path: "/home/user/Downloads".into(),
                        },
                    ],
                }),
            ]
        );
    }

    #[test]
    fn ignores_unknown_fields() {
        let contents = r#"{ "icon_theme": "Foo", "some_future_setting": 42 }"#;

        let settings: Settings = serde_json::from_str(contents).unwrap();

        assert_eq!(settings.icon_theme.as_deref(), Some("Foo"));
    }

    #[test]
    fn git_defaults_to_disabled_when_omitted() {
        let settings: Settings = serde_json::from_str("{}").unwrap();

        assert!(!settings.git.enabled);
        assert!(settings.git.status.enabled);
        assert!(settings.git.status.show_untracked);
        assert!(!settings.git.status.show_ignored);
        assert!(settings.git.status.dim_ignored);
        assert!(settings.git.status.aggregate_folders);
        assert_eq!(settings.git.status.badge_style, GitBadgeStyle::TextColor);
        assert!(settings.git.branch.enabled);
        assert!(settings.git.branch.show_in_status_bar);
        assert!(settings.git.branch.show_dirty_indicator);
        assert!(settings.git.branch.show_ahead_behind);
        assert_eq!(settings.git.refresh.poll_interval_ms, 2000);
        assert_eq!(settings.git.cli.binary_path, None);
        assert_eq!(settings.git.cli.timeout_ms, 3000);
        assert_eq!(settings.git.cli.max_repo_entries, None);
    }

    #[test]
    fn git_empty_object_uses_same_defaults() {
        let settings: Settings = serde_json::from_str(r#"{ "git": {} }"#).unwrap();

        assert_eq!(settings.git, GitSettings::default());
    }

    #[test]
    fn git_enabled_flag_parses_with_defaults_for_the_rest() {
        let settings: Settings =
            serde_json::from_str(r#"{ "git": { "enabled": true } }"#).unwrap();

        assert!(settings.git.enabled);
        assert!(settings.git.status.enabled);
        assert_eq!(settings.git.refresh.poll_interval_ms, 2000);
    }

    #[test]
    fn git_granular_overrides_parse() {
        let contents = r#"{
            "git": {
                "enabled": true,
                "status": {
                    "show_ignored": true,
                    "badge_style": "icon"
                },
                "branch": {
                    "show_ahead_behind": false
                },
                "refresh": {
                    "poll_interval_ms": 0
                },
                "cli": {
                    "binary_path": "/usr/bin/git",
                    "timeout_ms": 500,
                    "max_repo_entries": 20000
                }
            }
        }"#;

        let settings: Settings = serde_json::from_str(contents).unwrap();

        assert!(settings.git.status.show_ignored);
        assert_eq!(settings.git.status.badge_style, GitBadgeStyle::Icon);
        assert!(settings.git.status.dim_ignored);
        assert!(!settings.git.branch.show_ahead_behind);
        assert!(settings.git.branch.show_in_status_bar);
        assert_eq!(settings.git.refresh.poll_interval_ms, 0);
        assert_eq!(
            settings.git.cli.binary_path.as_deref(),
            Some("/usr/bin/git")
        );
        assert_eq!(settings.git.cli.timeout_ms, 500);
        assert_eq!(settings.git.cli.max_repo_entries, Some(20000));
    }

    #[test]
    fn disk_usage_defaults_to_safe_values_when_omitted() {
        let settings: Settings = serde_json::from_str("{}").unwrap();

        assert!(!settings.disk_usage.cross_filesystem_boundaries);
        assert!(!settings.disk_usage.follow_symlinks);
    }

    #[test]
    fn disk_usage_granular_overrides_parse() {
        let contents = r#"{
            "disk_usage": {
                "cross_filesystem_boundaries": true,
                "follow_symlinks": true
            }
        }"#;

        let settings: Settings = serde_json::from_str(contents).unwrap();

        assert!(settings.disk_usage.cross_filesystem_boundaries);
        assert!(settings.disk_usage.follow_symlinks);
    }

    #[test]
    fn theme_defaults_to_dark_mode_with_no_names_when_omitted() {
        let settings: Settings = serde_json::from_str("{}").unwrap();

        assert_eq!(settings.theme.mode, ThemeMode::Dark);
        assert_eq!(settings.theme.light, None);
        assert_eq!(settings.theme.dark, None);
    }

    #[test]
    fn theme_parses_mode_and_names() {
        let contents = r#"{
            "theme": {
                "mode": "dark",
                "light": "VSCode Light Modern",
                "dark": "VSCode Dark Modern"
            }
        }"#;

        let settings: Settings = serde_json::from_str(contents).unwrap();

        assert_eq!(settings.theme.mode, ThemeMode::Dark);
        assert_eq!(settings.theme.light.as_deref(), Some("VSCode Light Modern"));
        assert_eq!(settings.theme.dark.as_deref(), Some("VSCode Dark Modern"));
    }

    #[test]
    fn theme_mode_parses_light_and_system() {
        let light: Settings = serde_json::from_str(r#"{ "theme": { "mode": "light" } }"#).unwrap();
        let system: Settings =
            serde_json::from_str(r#"{ "theme": { "mode": "system" } }"#).unwrap();

        assert_eq!(light.theme.mode, ThemeMode::Light);
        assert_eq!(system.theme.mode, ThemeMode::System);
    }

    #[test]
    fn parse_jsonc_strips_line_and_block_comments() {
        let contents = r#"{
            // top-level comment
            "icon_theme": "Foo", /* inline */ "ui_font_size": 15.0
        }"#;

        let value = parse_jsonc(contents).unwrap();

        assert_eq!(value["icon_theme"], "Foo");
        assert_eq!(value["ui_font_size"], 15.0);
    }

    #[test]
    fn parse_jsonc_strips_trailing_commas() {
        let contents = r#"{
            "sidebar": [
                { "name": "A", "path": "/a", },
                { "name": "B", "path": "/b" },
            ],
        }"#;

        let value = parse_jsonc(contents).unwrap();

        assert_eq!(value["sidebar"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn parse_jsonc_leaves_commas_and_slashes_inside_strings_untouched() {
        let contents = r#"{ "ui_font_family": "A, B // not a comment, /* also not */" }"#;

        let value = parse_jsonc(contents).unwrap();

        assert_eq!(value["ui_font_family"], "A, B // not a comment, /* also not */");
    }

    #[test]
    fn merge_json_overlays_scalars_and_deep_merges_nested_objects() {
        let mut base: Value = serde_json::from_str(
            r#"{
                "icon_theme": "Zed Icons",
                "theme": { "mode": "dark", "light": "A", "dark": "B" }
            }"#,
        )
        .unwrap();
        let overlay: Value =
            serde_json::from_str(r#"{ "theme": { "light": "Overridden" } }"#).unwrap();

        merge_json(&mut base, overlay);

        assert_eq!(base["icon_theme"], "Zed Icons");
        assert_eq!(base["theme"]["mode"], "dark");
        assert_eq!(base["theme"]["light"], "Overridden");
        assert_eq!(base["theme"]["dark"], "B");
    }

    #[test]
    fn merge_json_overlay_array_replaces_base_array_wholesale() {
        let mut base: Value = serde_json::from_str(r#"{ "sidebar": [1, 2, 3] }"#).unwrap();
        let overlay: Value = serde_json::from_str(r#"{ "sidebar": [4] }"#).unwrap();

        merge_json(&mut base, overlay);

        assert_eq!(base["sidebar"], serde_json::json!([4]));
    }

    #[test]
    fn resolve_inherit_path_true_uses_default_zed_settings_path() {
        let value: Value = serde_json::from_str(r#"{ "inherit_from_zed": true }"#).unwrap();

        assert_eq!(resolve_inherit_path(&value), Some(default_zed_settings_path()));
    }

    #[test]
    fn resolve_inherit_path_false_or_absent_disables_inheritance() {
        let disabled: Value = serde_json::from_str(r#"{ "inherit_from_zed": false }"#).unwrap();
        let absent: Value = serde_json::from_str("{}").unwrap();

        assert_eq!(resolve_inherit_path(&disabled), None);
        assert_eq!(resolve_inherit_path(&absent), None);
    }

    #[test]
    fn resolve_inherit_path_string_expands_tilde() {
        let value: Value =
            serde_json::from_str(r#"{ "inherit_from_zed": "~/custom/zed-settings.json" }"#)
                .unwrap();

        assert_eq!(
            resolve_inherit_path(&value),
            Some(home_dir().join("custom/zed-settings.json"))
        );
    }

    #[test]
    fn resolve_inherit_path_string_absolute_path_used_as_is() {
        let value: Value = serde_json::from_str(
            r#"{ "inherit_from_zed": "/etc/zed/settings.json" }"#,
        )
        .unwrap();

        assert_eq!(
            resolve_inherit_path(&value),
            Some(PathBuf::from("/etc/zed/settings.json"))
        );
    }

    #[test]
    fn zex_values_win_over_inherited_zed_values_end_to_end() {
        let zed_contents = r#"{
            // zed's own settings
            "icon_theme": "Colored Zed Icons Theme Dark",
            "ui_font_family": "JetBrainsMono Nerd Font Mono",
            "ui_font_size": 16.0,
            "theme": {
                "mode": "dark",
                "light": "VSCode Dark Modern",
                "dark": "VSCode Dark Modern",
            },
        }"#;
        let zex_contents = r#"{
            "inherit_from_zed": true,
            "ui_font_size": 13.0,
            "theme": { "light": "VSCode Light Modern" }
        }"#;

        let mut zed_value = parse_jsonc(zed_contents).unwrap();
        let zex_value = parse_jsonc(zex_contents).unwrap();
        merge_json(&mut zed_value, zex_value);

        let settings: Settings = serde_json::from_value(zed_value).unwrap();

        assert_eq!(
            settings.icon_theme.as_deref(),
            Some("Colored Zed Icons Theme Dark")
        );
        assert_eq!(
            settings.ui_font_family.as_deref(),
            Some("JetBrainsMono Nerd Font Mono")
        );
        assert_eq!(settings.ui_font_size, Some(13.0));
        assert_eq!(settings.theme.mode, ThemeMode::Dark);
        assert_eq!(settings.theme.light.as_deref(), Some("VSCode Light Modern"));
        assert_eq!(settings.theme.dark.as_deref(), Some("VSCode Dark Modern"));
    }
}
