use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ids::*;

#[derive(Clone, Debug, Default)]
pub struct LocaleManager {
    pub languages: Vec<String>,
    pub tables: HashMap<String, HashMap<u32, String>>,
    pub current_lang: String,
}

impl LocaleManager {
    pub fn load_from_app_dir(exe_dir: &Path, configured_lang: &str) -> Self {
        let mut mgr = Self {
            languages: Vec::new(),
            tables: HashMap::new(),
            current_lang: String::new(),
        };

        let candidates: [PathBuf; 4] = [
            exe_dir.join("ramreduct.lng"),
            exe_dir.join("memreduct.lng"),
            exe_dir.join("bin").join("memreduct.lng"),
            exe_dir
                .parent()
                .and_then(|p| p.parent())
                .map(|p| p.join("bin").join("memreduct.lng"))
                .unwrap_or_else(|| PathBuf::from("bin/memreduct.lng")),
        ];

        for path in &candidates {
            if path.is_file() {
                if let Ok(raw) = fs::read(path) {
                    if let Some(text) = decode_lng_bytes(&raw) {
                        mgr.parse_lng(&text);
                        break;
                    }
                }
            }
        }

        mgr.languages.sort();
        mgr.set_language(configured_lang);
        mgr
    }

    pub fn parse_lng(&mut self, content: &str) {
        let mut current_section: Option<String> = None;

        for raw_line in content.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') && line.len() > 2 {
                let sec = line[1..line.len() - 1].trim().to_string();
                if !self.tables.contains_key(&sec) {
                    self.languages.push(sec.clone());
                    self.tables.insert(sec.clone(), HashMap::new());
                }
                current_section = Some(sec);
                continue;
            }
            if let Some(ref sec) = current_section {
                if let Some((k, v)) = line.split_once('=') {
                    if let Ok(id) = k.trim().parse::<u32>() {
                        let unescaped = v.replace("\\r\\n", "\r\n").replace("\\n", "\n");
                        if let Some(map) = self.tables.get_mut(sec) {
                            map.insert(id, unescaped);
                        }
                    }
                }
            }
        }
    }

    pub fn set_language(&mut self, lang: &str) {
        let trimmed = lang.trim();
        if trimmed.is_empty()
            || trimmed.eq_ignore_ascii_case("English (default)")
            || trimmed.eq_ignore_ascii_case("English")
        {
            self.current_lang.clear();
        } else if self.tables.contains_key(trimmed) {
            self.current_lang = trimmed.to_string();
        } else {
            self.current_lang.clear();
        }
    }

    pub fn get(&self, id: u32) -> String {
        if !self.current_lang.is_empty() {
            if let Some(map) = self.tables.get(&self.current_lang) {
                if let Some(val) = map.get(&id) {
                    if !val.is_empty() {
                        return val.clone();
                    }
                }
            }
        }
        default_english(id).to_string()
    }

    pub fn format_cleaned(&self, size_text: &str) -> String {
        let tpl = self.get(IDS_STATUS_CLEANED);
        if tpl.contains("%s") {
            tpl.replacen("%s", size_text, 1)
        } else if tpl.contains("٪ s") {
            tpl.replacen("٪ s", size_text, 1)
        } else {
            format!("Memory was released with {size_text} result.")
        }
    }
}

pub fn decode_lng_bytes(bytes: &[u8]) -> Option<String> {
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        let u16s: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        return Some(String::from_utf16_lossy(&u16s));
    }
    if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        return String::from_utf8(bytes[3..].to_vec()).ok();
    }
    String::from_utf8(bytes.to_vec()).ok()
}

pub fn default_english(id: u32) -> &'static str {
    match id {
        IDS_UPDATE_NO => "No updates available.",
        IDS_UPDATE_YES => "Update available, download and install them?",
        IDS_FILE => "File",
        IDS_VIEW => "View",
        IDS_SETTINGS => "Settings",
        IDS_EXIT => "Exit",
        IDS_LANGUAGE => "Language",
        IDS_HELP => "Help",
        IDS_WEBSITE => "Website",
        IDS_CHECKUPDATES => "Check for updates",
        IDS_ABOUT => "About",
        IDS_DONATE => "Give thanks!",
        IDS_TRAY_SHOW => "Show/Hide",
        IDS_TRAY_DISABLE => "Disable",
        IDS_TRAY_POPUP_1 => "Clean areas",
        IDS_TRAY_POPUP_2 => "Clean when above",
        IDS_TRAY_POPUP_3 => "Clean every",
        IDS_CLEAN => "Clean memory",
        IDS_GROUP_1 => "Physical memory",
        IDS_GROUP_2 => "Pagefile",
        IDS_GROUP_3 => "System working set",
        IDS_ITEM_1 => "Usage",
        IDS_ITEM_2 => "Available",
        IDS_ITEM_3 => "Total available",
        IDS_SETTINGS_GENERAL => "General",
        IDS_SETTINGS_MEMORY => "Memory cleaning",
        IDS_SETTINGS_APPEARANCE => "Appearance",
        IDS_SETTINGS_TRAY => "Tray icon",
        IDS_RESET => "Reset",
        IDS_CLOSE => "Close",
        IDS_TITLE_1 => "General configuration",
        IDS_TITLE_2 => "Language",
        IDS_TITLE_3 => "Memory regions to be cleaned",
        IDS_TITLE_4 => "Memory management",
        IDS_TITLE_5 => "Hotkeys (global)",
        IDS_TITLE_6 => "Style",
        IDS_TITLE_7 => "Color indication",
        IDS_TITLE_8 => "Mouse control",
        IDS_TITLE_9 => "Balloon tips",
        IDS_ALWAYSONTOP_CHK => "Always on top",
        IDS_LOADONSTARTUP_CHK => "Load on system startup",
        IDS_STARTMINIMIZED_CHK => "Start minimized",
        IDS_REDUCTCONFIRMATION_CHK => "Confirm memory cleaning start",
        IDS_SKIPUACWARNING_CHK => "Skip \"User Account Control\" prompt warning",
        IDS_CHECKUPDATES_CHK => "Periodically check for updates (recommended)",
        IDS_LANGUAGE_HINT => "Select language:",
        IDS_AUTOREDUCTENABLE_CHK => "Clean when above: (%)",
        IDS_AUTOREDUCTINTERVALENABLE_CHK => "Clean every: (min.)",
        IDS_HOTKEY_CLEAN_CHK => "Clean memory:",
        IDS_TRAYUSETRANSPARENCY_CHK => "Use transparent background",
        IDS_TRAYSHOWBORDER_CHK => "Show border",
        IDS_TRAYROUNDCORNERS_CHK => "Round corners",
        IDS_TRAYCHANGEBG_CHK => "Change background color (instead text and border)",
        IDS_TRAYUSEANTIALIASING_CHK => "Use antialiasing",
        IDS_FONT_HINT => "Font:",
        IDS_COLOR_TEXT_HINT => "Text color",
        IDS_COLOR_BACKGROUND_HINT => "Background color",
        IDS_COLOR_WARNING_HINT => "\"Warning level\" color",
        IDS_COLOR_DANGER_HINT => "\"Danger level\" color",
        IDS_TRAYLEVELWARNING_HINT => "\"Warning level\" started at: (%)",
        IDS_TRAYLEVELDANGER_HINT => "\"Danger level\" started at: (%)",
        IDS_TRAYACTIONSC_HINT => "Single click:",
        IDS_TRAYACTIONMC_HINT => "Middle click:",
        IDS_TRAY_ACTION_1 => "Show/Hide",
        IDS_TRAY_ACTION_2 => "Clean memory",
        IDS_TRAY_ACTION_3 => "Open task manager",
        IDS_SHOW_CLEAN_RESULT_CHK => "Show memory cleaning results",
        IDS_QUESTION => "Are you sure you want to clean the memory?",
        IDS_QUESTION_WARNING => "Switching-on \"Standby list\" and \"Modified page list\" may freeze your computer for almost a minute or so, are you sure to enable this option?",
        IDS_QUESTION_RESET => "Are you really sure you want to reset all application settings?",
        IDS_QUESTION_FLAG_CHK => "Do not ask again",
        IDS_STATUS_CLEANED => "Memory was released with %s result.",
        IDS_STATUS_NOPRIVILEGES => "Required administrator privileges, please run program as administrator!",
        IDS_NOTIFICATIONSOUND_CHK => "Enable notifications sound",
        IDS_TITLE_ADVANCED => "Advanced",
        IDS_USEDARKTHEME => "Use dark theme",
        IDS_ALLOWSTANDBYLISTCLEANUP_CHK => "Allow \"Standby lists\" and \"Modified page list\" cleanup on autoreduct",
        IDS_LOGRESULTS_CHK => "Log cleaning results into a debug log",
        IDS_MEMREGION_WORKINGSET => "Working set",
        IDS_MEMREGION_SYSTEMFILECACHE => "System file cache",
        IDS_MEMREGION_MODIFIEDFILECACHE => "Modified file cache",
        IDS_MEMREGION_MODIFIEDLIST => "Modified page list",
        IDS_MEMREGION_STANDBYLIST => "Standby list",
        IDS_MEMREGION_STANDBYLISTPRIORITY0 => "Standby list (without priority)",
        IDS_MEMREGION_REGISTRYCACHE => "Registry cache",
        IDS_MEMREGION_COMBINEMEMORYLISTS => "Combine memory lists",
        IDS_GROUP_4 => "Cache & session stats",
        IDS_ITEM_STANDBY => "Standby cache",
        IDS_ITEM_MODIFIED => "Modified pages",
        IDS_ITEM_SESSION_CLEANED => "Cleaned (session)",
        IDS_SKIPFULLSCREEN_CHK => "Skip automatic cleanup during fullscreen / game mode",
        IDS_TITLE_WHITELIST => "Process whitelist",
        IDS_WHITELIST_HINT => "Skip working set trim for processes (; separated):",
        IDS_PREVIEW_HINT => "Live preview:",
        IDS_RUNINBACKGROUND_CHK => "Run in background when window is closed (system tray)",
        IDS_WINSERVICE_CHK => "Run as Windows Background Service (services.msc)",
        IDS_OPENCONFIG => "Open config file",
        IDS_BG_RUNNING_INFO => "RAM Reduct continues running in the background (system tray).",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_utf16le_and_parse() {
        let sample = "[TestLang]\r\n003=Berkas\r\n075=Memori dibersihkan sebesar %s.\r\n";
        let mut bytes = vec![0xFF, 0xFE];
        for u in sample.encode_utf16() {
            bytes.extend_from_slice(&u.to_le_bytes());
        }
        let decoded = decode_lng_bytes(&bytes).expect("decode UTF-16LE");
        let mut mgr = LocaleManager::default();
        mgr.parse_lng(&decoded);
        mgr.set_language("TestLang");
        assert_eq!(mgr.get(IDS_FILE), "Berkas");
        assert_eq!(mgr.format_cleaned("512 MB"), "Memori dibersihkan sebesar 512 MB.");
    }

    #[test]
    fn test_real_memreduct_lng_loads_all_languages() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mgr = LocaleManager::load_from_app_dir(&manifest_dir, "");
        assert!(
            mgr.languages.len() >= 20,
            "Expected >= 20 languages in bin/memreduct.lng, got {}",
            mgr.languages.len()
        );
    }
}
