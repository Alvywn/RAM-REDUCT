use std::collections::BTreeMap;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::ffi::*;
use crate::ids::*;
use crate::locale::decode_lng_bytes;

const INI_SECTION: &str = "ramreduct";
const AUTORUN_REG_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const STARTUP_APPROVED_REG_PATH: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
const APP_PATHS_REG_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\App Paths\ramreduct.exe";
const HKLM_UNINSTALL_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\RAM Reduct";
const HKCU_UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\RAM Reduct";

const SKIP_UAC_TASK_NAME: &str = "RamReductSkipUac";
const SKIP_UAC_BG_TASK_NAME: &str = "RamReductSkipUacBg";
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub ini_path: PathBuf,
    pub log_path: PathBuf,

    pub always_on_top: bool,
    pub run_in_background: bool,
    pub background_balloon_shown: bool,
    pub start_minimized: bool,
    pub show_reduct_confirmation: bool,
    pub show_warning_confirmation: bool,
    pub check_updates: bool,
    pub use_dark_theme: bool,
    pub language: String,

    pub window_pos_x: Option<i32>,
    pub window_pos_y: Option<i32>,

    pub reduct_mask: u32,
    pub autoreduct_enable: bool,
    pub autoreduct_value: u32,
    pub autoreduct_interval_enable: bool,
    pub autoreduct_interval_value: u32,

    pub hotkey_clean_enable: bool,
    pub hotkey_clean: u32, // low byte = VK, high byte = HOTKEYF_*

    pub tray_use_transparency: bool,
    pub tray_show_border: bool,
    pub tray_round_corners: bool,
    pub tray_change_bg: bool,
    pub tray_use_antialiasing: bool,

    pub tray_font_face: String,
    pub tray_font_size: i32,
    pub tray_font_weight: i32,
    pub tray_font_italic: bool,

    pub tray_color_text: u32,
    pub tray_color_bg: u32,
    pub tray_color_warning: u32,
    pub tray_color_danger: u32,

    pub tray_level_warning: u32,
    pub tray_level_danger: u32,
    pub tray_action_sc: i32,
    pub tray_action_mc: i32,

    pub balloon_clean_results: bool,
    pub notifications_sound: bool,

    pub allow_standby_cleanup: bool,
    pub log_clean_results: bool,
    pub skip_fullscreen_autoclean: bool,
    pub process_whitelist: String,

    pub statistic_last_reduct: i64,
}

impl AppConfig {
    pub fn shared_programdata_ini() -> Option<PathBuf> {
        env::var_os("ProgramData")
            .map(PathBuf::from)
            .map(|p| p.join("RAM Reduct").join("ramreduct.ini"))
    }

    pub fn resolve_paths() -> (PathBuf, PathBuf) {
        let exe_dir = env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));

        // 1. Portable mode: ramreduct.ini (or legacy memreduct.ini) next to executable
        let portable_ini = exe_dir.join("ramreduct.ini");
        if portable_ini.exists() {
            return (portable_ini, exe_dir.join("ramreduct.log"));
        }
        let legacy_portable = exe_dir.join("memreduct.ini");
        if legacy_portable.exists() {
            return (legacy_portable, exe_dir.join("ramreduct.log"));
        }

        // 2. Per-user %APPDATA%\RAM Reduct\ramreduct.ini (unless running under SYSTEM service account)
        let username = env::var("USERNAME").unwrap_or_default();
        let is_system_profile = username.eq_ignore_ascii_case("SYSTEM")
            || env::var("APPDATA")
                .unwrap_or_default()
                .to_ascii_lowercase()
                .contains("systemprofile");

        if !is_system_profile {
            if let Some(appdata) = env::var_os("APPDATA") {
                let appdata_base = PathBuf::from(appdata);
                let dir = appdata_base.join("RAM Reduct");
                let _ = fs::create_dir_all(&dir);
                let primary_ini = dir.join("ramreduct.ini");

                // Migrate from previous %APPDATA%\Mem Reduct or Henry++\Mem Reduct if primary doesn't exist yet
                if !primary_ini.exists() {
                    let prev_mem = appdata_base.join("Mem Reduct").join("memreduct.ini");
                    let legacy = appdata_base
                        .join("Henry++")
                        .join("Mem Reduct")
                        .join("memreduct.ini");
                    if prev_mem.exists() {
                        let _ = fs::copy(&prev_mem, &primary_ini);
                    } else if legacy.exists() {
                        let _ = fs::copy(&legacy, &primary_ini);
                    }
                }

                return (primary_ini, dir.join("ramreduct.log"));
            }
        }

        // 3. System / Service profile fallback: %ProgramData%\RAM Reduct\ramreduct.ini
        if let Some(pd_ini) = Self::shared_programdata_ini() {
            if let Some(parent) = pd_ini.parent() {
                let _ = fs::create_dir_all(parent);
                return (pd_ini.clone(), parent.join("ramreduct.log"));
            }
        }

        (portable_ini, exe_dir.join("ramreduct.log"))
    }

    pub fn defaults_with_paths(ini_path: PathBuf, log_path: PathBuf) -> Self {
        Self {
            ini_path,
            log_path,
            always_on_top: false,
            run_in_background: true,
            background_balloon_shown: false,
            start_minimized: false,
            show_reduct_confirmation: true,
            show_warning_confirmation: true,
            check_updates: false,
            use_dark_theme: false,
            language: String::new(),
            window_pos_x: None,
            window_pos_y: None,
            reduct_mask: REDUCT_MASK_DEFAULT,
            autoreduct_enable: false,
            autoreduct_value: DEFAULT_AUTOREDUCT_VAL,
            autoreduct_interval_enable: false,
            autoreduct_interval_value: DEFAULT_AUTOREDUCTINTERVAL_VAL,
            hotkey_clean_enable: false,
            hotkey_clean: (VK_F1_U32) | ((HOTKEYF_CONTROL as u32) << 8),
            tray_use_transparency: false,
            tray_show_border: false,
            tray_round_corners: false,
            tray_change_bg: true,
            tray_use_antialiasing: false,
            tray_font_face: "Lucida Console".to_string(),
            tray_font_size: 8,
            tray_font_weight: FW_NORMAL,
            tray_font_italic: false,
            tray_color_text: TRAY_COLOR_TEXT,
            tray_color_bg: TRAY_COLOR_BG,
            tray_color_warning: TRAY_COLOR_WARNING,
            tray_color_danger: TRAY_COLOR_DANGER,
            tray_level_warning: DEFAULT_WARNING_LEVEL,
            tray_level_danger: DEFAULT_DANGER_LEVEL,
            tray_action_sc: 0,
            tray_action_mc: 1,
            balloon_clean_results: true,
            notifications_sound: true,
            allow_standby_cleanup: true,
            log_clean_results: false,
            skip_fullscreen_autoclean: true,
            process_whitelist: String::new(),
            statistic_last_reduct: 0,
        }
    }

    pub fn load() -> Self {
        let (ini_path, log_path) = Self::resolve_paths();
        Self::load_from_path(ini_path, log_path)
    }

    pub fn load_from_path(ini_path: PathBuf, log_path: PathBuf) -> Self {
        let mut cfg = Self::defaults_with_paths(ini_path.clone(), log_path);
        if !ini_path.exists() {
            // Create initial config file immediately so user always has a persistent memreduct.ini on disk
            cfg.save();
            return cfg;
        }

        let map = parse_ini_file(&ini_path);
        if map.is_empty() {
            return cfg;
        }

        cfg.always_on_top = map_bool(&map, "AlwaysOnTop", cfg.always_on_top);
        cfg.run_in_background = map_bool(&map, "RunInBackground", cfg.run_in_background);
        cfg.background_balloon_shown =
            map_bool(&map, "BackgroundBalloonShown", cfg.background_balloon_shown);
        cfg.start_minimized = map_bool(&map, "IsStartMinimized", cfg.start_minimized);
        cfg.show_reduct_confirmation = map_bool(
            &map,
            "IsShowReductConfirmation",
            cfg.show_reduct_confirmation,
        );
        cfg.show_warning_confirmation = map_bool(
            &map,
            "IsShowWarningConfirmation",
            cfg.show_warning_confirmation,
        );
        cfg.check_updates = map_bool(&map, "CheckUpdates", cfg.check_updates);
        cfg.use_dark_theme = map_bool(&map, "IsDarkThemeEnabled", cfg.use_dark_theme);
        cfg.language = map_str(&map, "Language", "");

        cfg.window_pos_x = map.get("WindowPosX").and_then(|s| s.parse::<i32>().ok());
        cfg.window_pos_y = map.get("WindowPosY").and_then(|s| s.parse::<i32>().ok());

        cfg.reduct_mask = map_u32(&map, "ReductMask2", cfg.reduct_mask) & REDUCT_MASK_ALL;
        if cfg.reduct_mask == 0 {
            cfg.reduct_mask = REDUCT_MASK_DEFAULT;
        }
        cfg.autoreduct_enable = map_bool(&map, "AutoreductEnable", cfg.autoreduct_enable);
        cfg.autoreduct_value = map_u32(&map, "AutoreductValue", cfg.autoreduct_value).clamp(1, 99);
        cfg.autoreduct_interval_enable = map_bool(
            &map,
            "AutoreductIntervalEnable",
            cfg.autoreduct_interval_enable,
        );
        cfg.autoreduct_interval_value = map_u32(
            &map,
            "AutoreductIntervalValue",
            cfg.autoreduct_interval_value,
        )
        .clamp(1, 1440);

        cfg.hotkey_clean_enable = map_bool(&map, "HotkeyCleanEnable", cfg.hotkey_clean_enable);
        cfg.hotkey_clean = map_u32(&map, "HotkeyClean", cfg.hotkey_clean);

        cfg.tray_use_transparency =
            map_bool(&map, "TrayUseTransparency", cfg.tray_use_transparency);
        cfg.tray_show_border = map_bool(&map, "TrayShowBorder", cfg.tray_show_border);
        cfg.tray_round_corners = map_bool(&map, "TrayRoundCorners", cfg.tray_round_corners);
        cfg.tray_change_bg = map_bool(&map, "TrayChangeBg", cfg.tray_change_bg);
        cfg.tray_use_antialiasing =
            map_bool(&map, "TrayUseAntialiasing", cfg.tray_use_antialiasing);

        let font_spec = map_str(&map, "TrayFont", "");
        if !font_spec.is_empty() {
            cfg.parse_font_spec(&font_spec);
        }

        cfg.tray_color_text = map_u32(&map, "TrayColorText", cfg.tray_color_text);
        cfg.tray_color_bg = map_u32(&map, "TrayColorBg", cfg.tray_color_bg);
        cfg.tray_color_warning = map_u32(&map, "TrayColorWarning", cfg.tray_color_warning);
        cfg.tray_color_danger = map_u32(&map, "TrayColorDanger", cfg.tray_color_danger);

        cfg.tray_level_warning =
            map_u32(&map, "TrayLevelWarning", cfg.tray_level_warning).clamp(1, 98);
        cfg.tray_level_danger =
            map_u32(&map, "TrayLevelDanger", cfg.tray_level_danger).clamp(2, 99);
        cfg.normalize_warning_danger(true);

        // Auto-heal values clobbered by earlier un-suppressed EN_CHANGE bug (1/1/1/2 signature)
        if cfg.autoreduct_value == 1
            && cfg.autoreduct_interval_value == 1
            && cfg.tray_level_warning == 1
            && cfg.tray_level_danger == 2
        {
            cfg.autoreduct_value = DEFAULT_AUTOREDUCT_VAL;
            cfg.autoreduct_interval_value = DEFAULT_AUTOREDUCTINTERVAL_VAL;
            cfg.tray_level_warning = DEFAULT_WARNING_LEVEL;
            cfg.tray_level_danger = DEFAULT_DANGER_LEVEL;
        }

        cfg.tray_action_sc =
            (map_u32(&map, "TrayActionDc", cfg.tray_action_sc as u32) as i32).clamp(0, 2);
        cfg.tray_action_mc =
            (map_u32(&map, "TrayActionMc", cfg.tray_action_mc as u32) as i32).clamp(0, 2);

        cfg.balloon_clean_results =
            map_bool(&map, "BalloonCleanResults", cfg.balloon_clean_results);
        cfg.notifications_sound = map_bool(&map, "IsNotificationsSound", cfg.notifications_sound);

        cfg.allow_standby_cleanup =
            map_bool(&map, "IsAllowStandbyListCleanup", cfg.allow_standby_cleanup);
        cfg.log_clean_results = map_bool(&map, "LogCleanResults", cfg.log_clean_results);
        cfg.skip_fullscreen_autoclean = map_bool(
            &map,
            "SkipFullscreenAutoClean",
            cfg.skip_fullscreen_autoclean,
        );
        cfg.process_whitelist = map_str(&map, "ProcessWhitelist", "");
        cfg.statistic_last_reduct = map
            .get("StatisticLastReduct")
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);

        cfg
    }

    pub fn normalize_warning_danger(&mut self, edited_warning: bool) {
        self.tray_level_warning = self.tray_level_warning.clamp(1, 98);
        self.tray_level_danger = self.tray_level_danger.clamp(2, 99);
        if self.tray_level_warning >= self.tray_level_danger {
            if edited_warning {
                self.tray_level_danger = (self.tray_level_warning + 1).min(99);
                if self.tray_level_warning >= self.tray_level_danger {
                    self.tray_level_warning = self.tray_level_danger - 1;
                }
            } else {
                self.tray_level_warning = self.tray_level_danger.saturating_sub(1).max(1);
            }
        }
    }

    pub fn parse_font_spec(&mut self, spec: &str) {
        let mut parts = spec.split(';');
        if let Some(face) = parts.next() {
            let f = face.trim();
            if !f.is_empty() {
                self.tray_font_face = f.to_string();
            }
        }
        if let Some(sz) = parts.next().and_then(|v| v.trim().parse::<i32>().ok()) {
            self.tray_font_size = sz.clamp(6, 32);
        }
        if let Some(wt) = parts.next().and_then(|v| v.trim().parse::<i32>().ok()) {
            self.tray_font_weight = wt.clamp(100, 900);
        }
        if let Some(it) = parts.next() {
            self.tray_font_italic = it.trim() == "1" || it.trim().eq_ignore_ascii_case("true");
        }
    }

    pub fn format_font_spec(&self) -> String {
        format!(
            "{};{};{};{}",
            self.tray_font_face,
            self.tray_font_size,
            self.tray_font_weight,
            if self.tray_font_italic { 1 } else { 0 }
        )
    }

    pub fn to_ini_string(&self) -> String {
        let mut out = String::with_capacity(1536);
        out.push_str(&format!("[{INI_SECTION}]\r\n"));
        push_kv(&mut out, "AlwaysOnTop", bool_str(self.always_on_top));
        push_kv(&mut out, "RunInBackground", bool_str(self.run_in_background));
        push_kv(
            &mut out,
            "BackgroundBalloonShown",
            bool_str(self.background_balloon_shown),
        );
        push_kv(&mut out, "IsStartMinimized", bool_str(self.start_minimized));
        push_kv(
            &mut out,
            "IsShowReductConfirmation",
            bool_str(self.show_reduct_confirmation),
        );
        push_kv(
            &mut out,
            "IsShowWarningConfirmation",
            bool_str(self.show_warning_confirmation),
        );
        push_kv(&mut out, "CheckUpdates", bool_str(self.check_updates));
        push_kv(&mut out, "IsDarkThemeEnabled", bool_str(self.use_dark_theme));
        push_kv(&mut out, "Language", &self.language);

        if let Some(x) = self.window_pos_x {
            push_kv(&mut out, "WindowPosX", &x.to_string());
        }
        if let Some(y) = self.window_pos_y {
            push_kv(&mut out, "WindowPosY", &y.to_string());
        }

        push_kv(&mut out, "ReductMask2", &self.reduct_mask.to_string());
        push_kv(&mut out, "AutoreductEnable", bool_str(self.autoreduct_enable));
        push_kv(&mut out, "AutoreductValue", &self.autoreduct_value.to_string());
        push_kv(
            &mut out,
            "AutoreductIntervalEnable",
            bool_str(self.autoreduct_interval_enable),
        );
        push_kv(
            &mut out,
            "AutoreductIntervalValue",
            &self.autoreduct_interval_value.to_string(),
        );

        push_kv(
            &mut out,
            "HotkeyCleanEnable",
            bool_str(self.hotkey_clean_enable),
        );
        push_kv(&mut out, "HotkeyClean", &self.hotkey_clean.to_string());

        push_kv(
            &mut out,
            "TrayUseTransparency",
            bool_str(self.tray_use_transparency),
        );
        push_kv(&mut out, "TrayShowBorder", bool_str(self.tray_show_border));
        push_kv(
            &mut out,
            "TrayRoundCorners",
            bool_str(self.tray_round_corners),
        );
        push_kv(&mut out, "TrayChangeBg", bool_str(self.tray_change_bg));
        push_kv(
            &mut out,
            "TrayUseAntialiasing",
            bool_str(self.tray_use_antialiasing),
        );
        push_kv(&mut out, "TrayFont", &self.format_font_spec());

        push_kv(&mut out, "TrayColorText", &self.tray_color_text.to_string());
        push_kv(&mut out, "TrayColorBg", &self.tray_color_bg.to_string());
        push_kv(
            &mut out,
            "TrayColorWarning",
            &self.tray_color_warning.to_string(),
        );
        push_kv(
            &mut out,
            "TrayColorDanger",
            &self.tray_color_danger.to_string(),
        );

        push_kv(
            &mut out,
            "TrayLevelWarning",
            &self.tray_level_warning.to_string(),
        );
        push_kv(
            &mut out,
            "TrayLevelDanger",
            &self.tray_level_danger.to_string(),
        );
        push_kv(&mut out, "TrayActionDc", &self.tray_action_sc.to_string());
        push_kv(&mut out, "TrayActionMc", &self.tray_action_mc.to_string());

        push_kv(
            &mut out,
            "BalloonCleanResults",
            bool_str(self.balloon_clean_results),
        );
        push_kv(
            &mut out,
            "IsNotificationsSound",
            bool_str(self.notifications_sound),
        );
        push_kv(
            &mut out,
            "IsAllowStandbyListCleanup",
            bool_str(self.allow_standby_cleanup),
        );
        push_kv(&mut out, "LogCleanResults", bool_str(self.log_clean_results));
        push_kv(
            &mut out,
            "SkipFullscreenAutoClean",
            bool_str(self.skip_fullscreen_autoclean),
        );
        push_kv(&mut out, "ProcessWhitelist", &self.process_whitelist);
        push_kv(
            &mut out,
            "StatisticLastReduct",
            &self.statistic_last_reduct.to_string(),
        );
        out
    }

    pub fn save(&self) {
        let content = self.to_ini_string();
        write_ini_atomic(&self.ini_path, &content);

        // Mirror config to %ProgramData%\Mem Reduct\memreduct.ini when elevated so Windows Service shares config
        if is_elevated() {
            if let Some(pd_ini) = Self::shared_programdata_ini() {
                if pd_ini != self.ini_path {
                    write_ini_atomic(&pd_ini, &content);
                }
            }
        }
    }

    pub fn reset_to_defaults(&mut self) {
        let ini = self.ini_path.clone();
        let log = self.log_path.clone();
        *self = Self::defaults_with_paths(ini, log);
        self.save();
    }

    pub fn append_log(&self, reason: &str, details: &str) {
        if let Some(parent) = self.log_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
        {
            let ts = unix_now();
            let _ = writeln!(f, "[{ts}] {reason}: {details}");
        }
    }
}

const VK_F1_U32: u32 = 0x70;

fn bool_str(v: bool) -> &'static str {
    if v {
        "true"
    } else {
        "false"
    }
}

fn push_kv(buf: &mut String, key: &str, val: &str) {
    buf.push_str(key);
    buf.push('=');
    buf.push_str(val);
    buf.push_str("\r\n");
}

fn write_ini_atomic(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        if parent.as_os_str().is_empty() {
            return;
        }
        let _ = fs::create_dir_all(parent);
    }
    // Write UTF-8 with BOM (readable in Notepad, VS Code, and our parser)
    let mut bytes = Vec::with_capacity(3 + text.len());
    bytes.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    bytes.extend_from_slice(text.as_bytes());

    let tmp = path.with_extension("ini.tmp");
    if fs::write(&tmp, &bytes).is_ok() {
        if fs::rename(&tmp, path).is_err() {
            let _ = fs::write(path, &bytes);
            let _ = fs::remove_file(&tmp);
        }
    } else {
        let _ = fs::write(path, &bytes);
    }
}

pub fn parse_ini_file(path: &Path) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let raw = match fs::read(path) {
        Ok(b) => b,
        Err(_) => return map,
    };
    let text = match decode_lng_bytes(&raw) {
        Some(s) => s,
        None => String::from_utf8_lossy(&raw).into_owned(),
    };

    let mut in_section = true;
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            let sec = line[1..line.len() - 1].trim();
            in_section =
                sec.eq_ignore_ascii_case(INI_SECTION) || sec.eq_ignore_ascii_case("memreduct");
            continue;
        }
        if in_section {
            if let Some((k, v)) = line.split_once('=') {
                map.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }
    map
}

fn map_str(map: &BTreeMap<String, String>, key: &str, default: &str) -> String {
    map.get(key).cloned().unwrap_or_else(|| default.to_string())
}

fn map_bool(map: &BTreeMap<String, String>, key: &str, default: bool) -> bool {
    match map.get(key) {
        Some(val) => matches!(
            val.trim().to_ascii_lowercase().as_str(),
            "true" | "1" | "yes" | "on"
        ),
        None => default,
    }
}

fn map_u32(map: &BTreeMap<String, String>, key: &str, default: u32) -> u32 {
    let Some(val) = map.get(key) else {
        return default;
    };
    let t = val.trim();
    if t.is_empty() {
        return default;
    }
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).unwrap_or(default)
    } else {
        t.parse::<u32>().unwrap_or(default)
    }
}

// ============================================================================
// Windows OS Registration (AppUserModelID, App Paths, Installed Apps, Startup)
// ============================================================================

pub fn ensure_windows_identity_and_registration() {
    unsafe {
        let _ = SetCurrentProcessExplicitAppUserModelID(to_wide(APP_USER_MODEL_ID).as_ptr());
    }

    let Ok(exe) = env::current_exe() else {
        return;
    };
    let exe_str = exe.to_string_lossy().into_owned();
    let exe_dir = exe
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    // 1. Register HKCU\Software\Microsoft\Windows\CurrentVersion\App Paths\memreduct.exe
    unsafe {
        let mut hkey: HKEY = 0;
        let mut disp = 0u32;
        if RegCreateKeyExW(
            HKEY_CURRENT_USER,
            to_wide(APP_PATHS_REG_KEY).as_ptr(),
            0,
            std::ptr::null_mut(),
            0,
            KEY_WRITE,
            std::ptr::null(),
            &mut hkey,
            &mut disp,
        ) == 0
        {
            reg_set_sz(hkey, "", &exe_str);
            reg_set_sz(hkey, "Path", &exe_dir);
            RegCloseKey(hkey);
        }
    }

    // 2. If not already installed system-wide in HKLM Uninstall, register in HKCU Uninstall
    //    so Windows Settings -> Apps -> Installed Apps detects "Mem Reduct" immediately!
    let has_hklm = unsafe {
        let mut hkey: HKEY = 0;
        let ok = RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            to_wide(HKLM_UNINSTALL_KEY).as_ptr(),
            0,
            KEY_READ,
            &mut hkey,
        ) == 0;
        if ok {
            RegCloseKey(hkey);
        }
        ok
    };

    if !has_hklm {
        unsafe {
            let mut hkey: HKEY = 0;
            let mut disp = 0u32;
            if RegCreateKeyExW(
                HKEY_CURRENT_USER,
                to_wide(HKCU_UNINSTALL_KEY).as_ptr(),
                0,
                std::ptr::null_mut(),
                0,
                KEY_WRITE,
                std::ptr::null(),
                &mut hkey,
                &mut disp,
            ) == 0
            {
                reg_set_sz(hkey, "DisplayName", &format!("{APP_NAME} {APP_VERSION}"));
                reg_set_sz(hkey, "DisplayVersion", APP_VERSION);
                reg_set_sz(hkey, "Publisher", APP_AUTHOR);
                reg_set_sz(hkey, "InstallLocation", &exe_dir);
                reg_set_sz(hkey, "DisplayIcon", &format!("{exe_str},0"));
                reg_set_sz(hkey, "URLInfoAbout", APP_WEBSITE);
                reg_set_sz(
                    hkey,
                    "UninstallString",
                    &format!("cmd.exe /c reg delete \"HKCU\\{HKCU_UNINSTALL_KEY}\" /f"),
                );
                reg_set_dword(hkey, "NoModify", 1);
                reg_set_dword(hkey, "NoRepair", 1);
                reg_set_dword(hkey, "EstimatedSize", 512);
                RegCloseKey(hkey);
            }
        }
    }
}

unsafe fn reg_set_sz(hkey: HKEY, name: &str, value: &str) {
    let w = to_wide(value);
    let name_w = to_wide(name);
    RegSetValueExW(
        hkey,
        if name.is_empty() {
            std::ptr::null()
        } else {
            name_w.as_ptr()
        },
        0,
        REG_SZ,
        w.as_ptr() as *const u8,
        (w.len() * 2) as u32,
    );
}

unsafe fn reg_set_dword(hkey: HKEY, name: &str, value: u32) {
    RegSetValueExW(
        hkey,
        to_wide(name).as_ptr(),
        0,
        REG_DWORD,
        &value as *const u32 as *const u8,
        4,
    );
}

// Autorun via HKCU\Software\Microsoft\Windows\CurrentVersion\Run + StartupApproved
pub fn is_autorun_enabled() -> bool {
    unsafe {
        let mut hkey: HKEY = 0;
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            to_wide(AUTORUN_REG_PATH).as_ptr(),
            0,
            KEY_READ,
            &mut hkey,
        ) != 0
        {
            return false;
        }
        let mut reg_type = 0u32;
        let mut size = 0u32;
        let status = RegQueryValueExW(
            hkey,
            to_wide(APP_NAME).as_ptr(),
            std::ptr::null_mut(),
            &mut reg_type,
            std::ptr::null_mut(),
            &mut size,
        );
        RegCloseKey(hkey);
        status == 0 && size > 0
    }
}

pub fn set_autorun_enabled(enable: bool) -> bool {
    let exe = match env::current_exe() {
        Ok(p) => p,
        Err(_) => return false,
    };
    unsafe {
        let mut hkey: HKEY = 0;
        let mut disp = 0u32;
        if RegCreateKeyExW(
            HKEY_CURRENT_USER,
            to_wide(AUTORUN_REG_PATH).as_ptr(),
            0,
            std::ptr::null_mut(),
            0,
            KEY_WRITE,
            std::ptr::null(),
            &mut hkey,
            &mut disp,
        ) != 0
        {
            return false;
        }
        let ok = if enable {
            let cmd = format!("\"{}\" -background", exe.display());
            reg_set_sz(hkey, APP_NAME, &cmd);

            // Ensure Task Manager -> Startup Apps marks "Mem Reduct" as Enabled (0x02)
            let mut happroved: HKEY = 0;
            if RegCreateKeyExW(
                HKEY_CURRENT_USER,
                to_wide(STARTUP_APPROVED_REG_PATH).as_ptr(),
                0,
                std::ptr::null_mut(),
                0,
                KEY_WRITE,
                std::ptr::null(),
                &mut happroved,
                &mut disp,
            ) == 0
            {
                let enabled_bytes: [u8; 12] = [0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
                RegSetValueExW(
                    happroved,
                    to_wide(APP_NAME).as_ptr(),
                    0,
                    REG_BINARY,
                    enabled_bytes.as_ptr(),
                    enabled_bytes.len() as u32,
                );
                RegCloseKey(happroved);
            }

            // If currently elevated, also register background Skip-UAC task so Windows startup auto-elevates silently!
            if is_elevated() {
                let _ = create_skip_uac_tasks(&exe);
            }
            true
        } else {
            let r = RegDeleteValueW(hkey, to_wide(APP_NAME).as_ptr());
            r == 0 || r == 2
        };
        RegCloseKey(hkey);
        ok
    }
}

fn create_skip_uac_tasks(exe: &Path) -> bool {
    let tr_gui = format!("\"{}\"", exe.display());
    let tr_bg = format!("\"{}\" -background", exe.display());

    let ok1 = Command::new("schtasks.exe")
        .args([
            "/Create",
            "/F",
            "/RL",
            "HIGHEST",
            "/SC",
            "ONCE",
            "/ST",
            "00:00",
            "/TN",
            SKIP_UAC_TASK_NAME,
            "/TR",
            &tr_gui,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    let _ = Command::new("schtasks.exe")
        .args([
            "/Create",
            "/F",
            "/RL",
            "HIGHEST",
            "/SC",
            "ONCE",
            "/ST",
            "00:00",
            "/TN",
            SKIP_UAC_BG_TASK_NAME,
            "/TR",
            &tr_bg,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .status();

    ok1
}

// UAC Skip via Windows Task Scheduler (schtasks)
pub fn is_skip_uac_enabled() -> bool {
    Command::new("schtasks.exe")
        .args(["/Query", "/TN", SKIP_UAC_TASK_NAME])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn set_skip_uac_enabled(enable: bool) -> bool {
    if !is_elevated() {
        return false;
    }
    if enable {
        let exe = match env::current_exe() {
            Ok(p) => p,
            Err(_) => return false,
        };
        create_skip_uac_tasks(&exe)
    } else {
        let _ = Command::new("schtasks.exe")
            .args(["/Delete", "/F", "/TN", SKIP_UAC_BG_TASK_NAME])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
        Command::new("schtasks.exe")
            .args(["/Delete", "/F", "/TN", SKIP_UAC_TASK_NAME])
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

pub fn try_silent_elevate_background() -> bool {
    Command::new("schtasks.exe")
        .args(["/Run", "/TN", SKIP_UAC_BG_TASK_NAME])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn relaunch_as_admin() -> bool {
    if is_skip_uac_enabled() {
        let ok = Command::new("schtasks.exe")
            .args(["/Run", "/TN", SKIP_UAC_TASK_NAME])
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            return true;
        }
    }

    let exe = match env::current_exe() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let args: Vec<String> = env::args().skip(1).collect();
    let params = args.join(" ");

    let op_w = to_wide("runas");
    let file_w = to_wide(&exe.to_string_lossy());
    let params_w = to_wide(&params);

    let res = unsafe {
        ShellExecuteW(
            0,
            op_w.as_ptr(),
            file_w.as_ptr(),
            if params.is_empty() {
                std::ptr::null()
            } else {
                params_w.as_ptr()
            },
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    res > 32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_warning_danger_auto_clamp() {
        let mut cfg = AppConfig::defaults_with_paths(PathBuf::new(), PathBuf::new());
        cfg.tray_level_warning = 95;
        cfg.tray_level_danger = 50;
        cfg.normalize_warning_danger(true);
        assert!(cfg.tray_level_warning < cfg.tray_level_danger);
        assert_eq!(cfg.tray_level_danger, 96);

        cfg.tray_level_danger = 40;
        cfg.normalize_warning_danger(false);
        assert_eq!(cfg.tray_level_warning, 39);
        assert!(cfg.tray_level_warning < cfg.tray_level_danger);
    }

    #[test]
    fn test_ini_save_and_load_roundtrip() {
        let tmp_dir = env::temp_dir().join("memreduct_rust_test_cfg");
        let _ = fs::create_dir_all(&tmp_dir);
        let ini = tmp_dir.join("memreduct.ini");
        let log = tmp_dir.join("memreduct.log");

        let mut cfg = AppConfig::defaults_with_paths(ini.clone(), log.clone());
        cfg.always_on_top = true;
        cfg.run_in_background = true;
        cfg.autoreduct_enable = true;
        cfg.autoreduct_value = 82;
        cfg.process_whitelist = "cs2.exe;obs64.exe".to_string();
        cfg.window_pos_x = Some(240);
        cfg.window_pos_y = Some(180);
        cfg.save();

        let loaded = AppConfig::load_from_path(ini.clone(), log);
        assert!(loaded.always_on_top);
        assert!(loaded.run_in_background);
        assert!(loaded.autoreduct_enable);
        assert_eq!(loaded.autoreduct_value, 82);
        assert_eq!(loaded.process_whitelist, "cs2.exe;obs64.exe");
        assert_eq!(loaded.window_pos_x, Some(240));
        assert_eq!(loaded.window_pos_y, Some(180));

        let _ = fs::remove_dir_all(&tmp_dir);
    }
}
