pub struct ConfigDoc {
    pub key: &'static str,
    pub section: &'static str,
    pub summary: &'static str,
    pub type_hint: &'static str,
    pub default: &'static str,
    pub body: &'static str,
}

pub const SECTIONS: &[&str] = &["General", "Sidebar", "Theme", "Git", "Search"];

pub const CONFIG_DOCS: &[ConfigDoc] = &[
    ConfigDoc {
        key: "inherit_from_zed",
        section: "General",
        summary: "Inherit settings from Zed's own config.",
        type_hint: "bool | string",
        default: "false",
        body: "Set to true to load $XDG_CONFIG_HOME/zed/settings.json as a base, or to a \
string to point at a specific settings file (e.g. \"~/.config/zed/settings.json\" or an \
absolute path). Only fields zex understands are used from it -- anything Zed-specific with \
no zex equivalent is ignored.\n\n\
Fields read from Zed's config:\n\
- icon_theme\n\
- ui_font_family\n\
- ui_font_size\n\
- ui_font_weight\n\
- theme.mode, theme.light, theme.dark\n\n\
Everything else in zex's own schema (show_hidden_files, sidebar, sidebar_visible, git, \
disk_usage) is zex-specific and has no Zed equivalent to inherit, even if set to true.\n\n\
Any field you also declare in zex's own config.json wins, merged in field by field rather \
than replacing whole sections -- so setting \"theme\": { \"light\": \"...\" } in zex overrides \
just that one field while still picking up everything else (icon theme, fonts, the rest of \
the theme block, etc.) from Zed's config.",
    },
    ConfigDoc {
        key: "icon_theme",
        section: "General",
        summary: "Icon theme to use for files and folders.",
        type_hint: "string",
        default: "null",
        body: "Name of a Zed-schema icon theme. zex scans installed Zed extensions under \
$XDG_DATA_HOME/zed/extensions/installed/*/extension.toml for an icon_themes entry, and \
matches on the theme's name. If unset or not found, zex falls back to its own bundled \
default icon theme.",
    },
    ConfigDoc {
        key: "ui_font_family",
        section: "General",
        summary: "UI font family.",
        type_hint: "string",
        default: "null (bundled default)",
        body: "Font family name used throughout the UI. Must be a font available to the \
system's font stack.",
    },
    ConfigDoc {
        key: "ui_font_size",
        section: "General",
        summary: "UI font size, in pixels.",
        type_hint: "number",
        default: "null (bundled default)",
        body: "Base font size for UI text.",
    },
    ConfigDoc {
        key: "ui_font_weight",
        section: "General",
        summary: "UI font weight.",
        type_hint: "number",
        default: "null (bundled default)",
        body: "Font weight (e.g. 400 for regular, 500 for medium, 700 for bold), following \
standard CSS-style numeric font weights.",
    },
    ConfigDoc {
        key: "show_hidden_files",
        section: "General",
        summary: "Show dotfiles by default.",
        type_hint: "bool",
        default: "false",
        body: "Whether hidden files (dotfiles) are shown when zex starts. Can be toggled at \
runtime with Ctrl-H regardless of this setting.",
    },
    ConfigDoc {
        key: "image_previews",
        section: "General",
        summary: "Show image thumbnails and the click-to-preview pane.",
        type_hint: "bool",
        default: "true",
        body: "When enabled, entries for image files (png, jpg, jpeg, jfif, gif, bmp, webp, \
ico, tiff, tif, qoi, tga) render a small thumbnail of the image itself instead of the \
generic icon, and selecting a single image opens a preview pane on the right edge of the \
window. Thumbnails are generated in the background as rows come into view and cached in \
memory. Files larger than 32 MB are left with the generic icon.",
    },
    ConfigDoc {
        key: "sidebar",
        section: "Sidebar",
        summary: "Pinned locations and sections shown in the sidebar.",
        type_hint: "array",
        default: "[]",
        body: "A list of entries, sections, and/or separators. An entry is { \"name\": \
string, \"path\": string }. A section is { \"section\": string, \"entries\": [entry, ...] }. \
The literal string \"separator\" draws a horizontal divider line. Entries can be listed flat \
or grouped into named sections -- mix and match as needed. An entry's path may start with \
\"~\" to reference your home directory.\n\n\
Example:\n\"sidebar\": [\n    { \"name\": \"Home\", \"path\": \"~\" },\n    \"separator\",\n    {\n      \"section\": \"Work\",\n      \"entries\": [\n        { \"name\": \"Projects\", \"path\": \"~/Projects\" },\n        { \"name\": \"Downloads\", \"path\": \"~/Downloads\" }\n      ]\n    }\n  ]",
    },
    ConfigDoc {
        key: "theme.mode",
        section: "Theme",
        summary: "Which palette to resolve: dark, light, or system.",
        type_hint: "\"dark\" | \"light\" | \"system\"",
        default: "\"dark\"",
        body: "\"dark\" / \"light\" force a palette; \"system\" follows the OS light/dark \
setting at launch. Color themes are resolved once at startup -- changing this requires \
restarting zex.",
    },
    ConfigDoc {
        key: "theme.light",
        section: "Theme",
        summary: "Theme name to use when the resolved mode is light.",
        type_hint: "string",
        default: "null",
        body: "A theme name is resolved the same way icon themes are: zex scans installed \
Zed extensions under $XDG_DATA_HOME/zed/extensions/installed/*/extension.toml for a \
themes = [...] entry, parses the referenced theme JSON (Zed's real theme schema), and \
matches on the theme's name. zex also vendors Zed's own built-in theme families (Ayu, \
Gruvbox, One) under assets/themes/zed_builtin/, so \"Gruvbox Dark\" resolves without that \
theme being installed as an extension. If the name isn't found, or this is left unset while \
the resolved mode is light, zex falls back to its own bundled default. A matched theme only \
needs to define the handful of keys zex actually uses (background, borders, text, \
selection/hover states, and the generic modified/created/deleted/renamed/conflict/ignored \
status roles) -- anything it omits falls back to the bundled default rather than erroring.",
    },
    ConfigDoc {
        key: "theme.dark",
        section: "Theme",
        summary: "Theme name to use when the resolved mode is dark.",
        type_hint: "string",
        default: "null",
        body: "Same resolution rules as theme.light, applied when the resolved mode is dark.",
    },
    ConfigDoc {
        key: "git.enabled",
        section: "Git",
        summary: "Master switch for git awareness.",
        type_hint: "bool",
        default: "false",
        body: "Everything else under git is inert until this is true.",
    },
    ConfigDoc {
        key: "git.status.enabled",
        section: "Git",
        summary: "Per-file/folder status decoration in the file list.",
        type_hint: "bool",
        default: "true",
        body: "",
    },
    ConfigDoc {
        key: "git.status.show_untracked",
        section: "Git",
        summary: "Include untracked files in status.",
        type_hint: "bool",
        default: "true",
        body: "",
    },
    ConfigDoc {
        key: "git.status.show_ignored",
        section: "Git",
        summary: "Include .gitignore'd files/folders in status at all.",
        type_hint: "bool",
        default: "false",
        body: "",
    },
    ConfigDoc {
        key: "git.status.dim_ignored",
        section: "Git",
        summary: "Fade ignored rows.",
        type_hint: "bool",
        default: "true",
        body: "Only matters if show_ignored is on.",
    },
    ConfigDoc {
        key: "git.status.aggregate_folders",
        section: "Git",
        summary: "A folder shows the worst status of anything changed inside it.",
        type_hint: "bool",
        default: "true",
        body: "",
    },
    ConfigDoc {
        key: "git.status.badge_style",
        section: "Git",
        summary: "How file status is rendered.",
        type_hint: "\"text_color\" | \"icon\" | \"both\"",
        default: "\"text_color\"",
        body: "\"text_color\" recolors the filename, \"icon\" adds a small colored letter \
chip (M/A/D/R/U/I/!), \"both\" does both. Colors always come from the active color theme, \
not a separate override.",
    },
    ConfigDoc {
        key: "git.branch.enabled",
        section: "Git",
        summary: "Master switch for the branch indicator.",
        type_hint: "bool",
        default: "true",
        body: "",
    },
    ConfigDoc {
        key: "git.branch.show_in_status_bar",
        section: "Git",
        summary: "Show the current branch name in the status bar.",
        type_hint: "bool",
        default: "true",
        body: "",
    },
    ConfigDoc {
        key: "git.branch.show_dirty_indicator",
        section: "Git",
        summary: "Small dot next to the branch name when the working tree isn't clean.",
        type_hint: "bool",
        default: "true",
        body: "",
    },
    ConfigDoc {
        key: "git.branch.show_ahead_behind",
        section: "Git",
        summary: "Show up/down arrow counts relative to the upstream branch.",
        type_hint: "bool",
        default: "true",
        body: "",
    },
    ConfigDoc {
        key: "git.refresh.poll_interval_ms",
        section: "Git",
        summary: "How often to re-run git status in the background.",
        type_hint: "number",
        default: "2000",
        body: "In addition to the refresh that already happens on navigation and filesystem \
changes. 0 disables polling (git-only changes, like a commit from another terminal, won't \
be picked up until you navigate away and back).",
    },
    ConfigDoc {
        key: "git.cli.binary_path",
        section: "Git",
        summary: "Override the git binary used.",
        type_hint: "string",
        default: "null",
        body: "Defaults to whatever git resolves to on PATH.",
    },
    ConfigDoc {
        key: "git.cli.timeout_ms",
        section: "Git",
        summary: "Kill and ignore the git status call if it takes longer than this.",
        type_hint: "number",
        default: "3000",
        body: "",
    },
    ConfigDoc {
        key: "git.cli.max_repo_entries",
        section: "Git",
        summary: "Skip decorating repos larger than this.",
        type_hint: "number",
        default: "null",
        body: "If the status listing has more entries than this, skip decorating rather \
than render a huge repo's worth of badges.",
    },
    ConfigDoc {
        key: "search.max_results",
        section: "Search",
        summary: "Cap on how many matches/files Ctrl+F search shows at once.",
        type_hint: "number",
        default: "500",
        body: "Once this many results have been found, the search stops collecting more and \
shows a \"refine your search\" hint instead.",
    },
    ConfigDoc {
        key: "search.default_case",
        section: "Search",
        summary: "Default case-sensitivity mode when opening the search panel.",
        type_hint: "\"sensitive\" | \"insensitive\" | \"smart\"",
        default: "\"smart\"",
        body: "Smart case matches case-insensitively unless the query contains an uppercase \
letter, matching ripgrep's own --smart-case behavior. Can still be cycled per-search from the \
panel.",
    },
    ConfigDoc {
        key: "search.respect_gitignore",
        section: "Search",
        summary: "Whether Ctrl+F search skips .gitignore'd files by default.",
        type_hint: "bool",
        default: "true",
        body: "Can still be toggled per-search from the panel.",
    },
    ConfigDoc {
        key: "search.include_hidden",
        section: "Search",
        summary: "Whether Ctrl+F search includes dotfiles by default.",
        type_hint: "bool",
        default: "false",
        body: "Can still be toggled per-search from the panel.",
    },
    ConfigDoc {
        key: "search.cli.binary_path",
        section: "Search",
        summary: "Override the ripgrep (rg) binary used.",
        type_hint: "string",
        default: "null",
        body: "Defaults to whatever rg resolves to on PATH. Ctrl+F search does nothing useful \
without ripgrep installed.",
    },
    ConfigDoc {
        key: "search.cli.timeout_ms",
        section: "Search",
        summary: "Kill and ignore a search if it takes longer than this.",
        type_hint: "number",
        default: "5000",
        body: "",
    },
];

fn wrap(text: &str, width: usize, indent: &str) -> String {
    let mut out = String::new();
    let mut line_len = 0;

    for word in text.split(' ') {
        if line_len > 0 && line_len + 1 + word.len() > width {
            out.push('\n');
            line_len = 0;
        }
        if line_len == 0 {
            out.push_str(indent);
        } else {
            out.push(' ');
            line_len += 1;
        }
        out.push_str(word);
        line_len += word.len();
    }

    out
}

pub fn print_list() {
    println!("zex reads a JSONC config from $XDG_CONFIG_HOME/zex/config.json");
    println!("(falling back to ~/.config/zex/config.json). Every field is optional.\n");

    let key_width = CONFIG_DOCS.iter().map(|d| d.key.len()).max().unwrap_or(0) + 2;

    for section in SECTIONS {
        println!("{section}");
        for doc in CONFIG_DOCS.iter().filter(|d| d.section == *section) {
            println!("  {:<width$} {}", doc.key, doc.summary, width = key_width);
        }
        println!();
    }

    println!("Run `zex config <key>` for details on any of the above.");
}

pub fn print_key(key: &str) -> Result<(), String> {
    let doc = CONFIG_DOCS
        .iter()
        .find(|d| d.key == key)
        .ok_or_else(|| format!("zex: unknown config key: '{key}'\nRun `zex config` to list all keys."))?;

    println!("{}", doc.key);
    println!("  Type: {}", doc.type_hint);
    println!("  Default: {}", doc.default);
    if !doc.body.is_empty() {
        println!();
        for paragraph in doc.body.split("\n\n") {
            if paragraph.contains('\n') {
                for line in paragraph.lines() {
                    if line.is_empty() {
                        println!();
                    } else {
                        println!("  {line}");
                    }
                }
            } else {
                println!("{}", wrap(paragraph, 78, "  "));
            }
            println!();
        }
    }

    Ok(())
}
