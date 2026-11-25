#![cfg_attr(not(test), windows_subsystem = "windows")]

mod cleaner;
mod cli;
mod config;
mod ffi;
mod ids;
mod locale;
mod service;
mod settings;
mod tray;
mod winutil;

use std::cell::RefCell;
use std::env;
use std::path::PathBuf;

use cleaner::*;
use cli::*;
use config::*;
use ffi::*;
use ids::*;
use locale::*;
use service::*;
use settings::*;
use tray::*;
use winutil::*;

pub struct AppState {
    pub config: AppConfig,
    pub locale: LocaleManager,
    pub tray: TrayRenderer,
    pub main_hwnd: HWND,
    pub settings_hwnd: HWND,
    pub taskbar_created_msg: u32,
    pub initialized_pos: bool,

    // Independent auto-clean timestamps so manual cleanups do not reset periodic interval
    pub last_auto_limit_ts: i64,
    pub last_auto_interval_ts: i64,

    // Session cleanup statistics
    pub session_cleaned_bytes: u64,
    pub session_clean_count: u32,

    // Cached dynamic tray popup arrays
    pub limits_arr: Vec<u32>,
    pub intervals_arr: Vec<u32>,
}

thread_local! {
    pub(crate) static APP_STATE: RefCell<Option<AppState>> = const { RefCell::new(None) };
}

pub fn with_state<R>(f: impl FnOnce(&AppState) -> R) -> R {
    APP_STATE.with(|cell| {
        let borrow = cell.borrow();
        f(borrow.as_ref().expect("AppState initialized"))
    })
}

pub fn with_state_mut<R>(f: impl FnOnce(&mut AppState) -> R) -> R {
    APP_STATE.with(|cell| {
        let mut borrow = cell.borrow_mut();
        f(borrow.as_mut().expect("AppState initialized"))
    })
}

fn init_app_state() {
    let config = AppConfig::load();
    config.save();
    let exe_dir = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let locale = LocaleManager::load_from_app_dir(&exe_dir, &config.language);
    let now = unix_now();

    let state = AppState {
        config,
        locale,
        tray: TrayRenderer::default(),
        main_hwnd: 0,
        settings_hwnd: 0,
        taskbar_created_msg: unsafe {
            RegisterWindowMessageW(to_wide("TaskbarCreated").as_ptr())
        },
        initialized_pos: false,
        last_auto_limit_ts: 0,
        last_auto_interval_ts: now,
        session_cleaned_bytes: 0,
        session_clean_count: 0,
        limits_arr: Vec::new(),
        intervals_arr: Vec::new(),
    };

    APP_STATE.with(|cell| {
        *cell.borrow_mut() = Some(state);
    });
}

fn main() {
    let cli_mode = parse_cli_args(env::args().skip(1));

    // Headless Windows Service dispatcher runs directly before GUI setup
    if cli_mode == CliMode::ServiceRun {
        run_windows_service_dispatcher();
        return;
    }

    // Set AppUserModelID and ensure Windows App Paths & Installed Apps recognize Mem Reduct
    ensure_windows_identity_and_registration();

    unsafe {
        let icc = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_LISTVIEW_CLASSES
                | ICC_UPDOWN_CLASS
                | ICC_HOTKEY_CLASS
                | ICC_STANDARD_CLASSES,
        };
        InitCommonControlsEx(&icc);
    }

    init_app_state();

    match cli_mode {
        CliMode::Help => {
            show_msgbox(0, APP_NAME, help_message(), MB_OK | MB_ICONINFORMATION);
        }

        CliMode::ServiceRun => {}

        CliMode::ServiceInstall { silent } => match install_and_start_service() {
            Ok(()) => {
                if !silent {
                    show_msgbox(
                        0,
                        APP_NAME,
                        "RAM Reduct Background Service has been registered and started.",
                        MB_OK | MB_ICONINFORMATION,
                    );
                }
            }
            Err(err) => {
                if !silent {
                    show_msgbox(0, APP_NAME, &err, MB_OK | MB_ICONSTOP);
                }
                std::process::exit(1);
            }
        },

        CliMode::ServiceUninstall { silent } => match stop_and_uninstall_service() {
            Ok(()) => {
                if !silent {
                    show_msgbox(
                        0,
                        APP_NAME,
                        "RAM Reduct Background Service has been removed.",
                        MB_OK | MB_ICONINFORMATION,
                    );
                }
            }
            Err(err) => {
                if !silent {
                    show_msgbox(0, APP_NAME, &err, MB_OK | MB_ICONSTOP);
                }
                std::process::exit(1);
            }
        },

        CliMode::Clean { mask, silent } => {
            let code = run_cli_clean(mask, silent);
            std::process::exit(code);
        }

        CliMode::Gui {
            start_minimized,
            from_background_autorun,
        } => {
            // If Windows Startup launched us non-elevated with -background and SkipUAC task exists,
            // seamlessly hand off to the elevated background task without showing a UAC dialog!
            if from_background_autorun && !is_elevated() && try_silent_elevate_background() {
                return;
            }
            run_gui_mode(start_minimized);
        }
    }
}

fn run_cli_clean(mask_opt: Option<u32>, silent: bool) -> i32 {
    if !is_elevated() {
        if silent {
            return 1;
        }
        if relaunch_as_admin() {
            return 0;
        }
        let msg = with_state(|st| st.locale.get(IDS_STATUS_NOPRIVILEGES));
        show_msgbox(0, APP_NAME, &msg, MB_OK | MB_ICONSTOP);
        return 1;
    }

    let mask = mask_opt.unwrap_or_else(|| with_state(|st| st.config.reduct_mask));
    perform_cleanup(0, CleanupSource::Cmdline, Some(mask), silent);
    0
}

fn run_gui_mode(cli_minimized: bool) {
    let mutex = unsafe {
        CreateMutexW(
            std::ptr::null(),
            FALSE,
            to_wide("Local\\RamReductRustSingleInstance").as_ptr(),
        )
    };
    if unsafe { GetLastError() } == 183 {
        if !cli_minimized {
            let existing = unsafe { FindWindowW(std::ptr::null(), to_wide(APP_NAME).as_ptr()) };
            if existing != 0 {
                toggle_window_visibility(existing, true);
            }
        }
        if mutex != 0 {
            unsafe {
                CloseHandle(mutex);
            }
        }
        return;
    }

    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        let hwnd = CreateDialogParamW(
            hinst,
            make_int_resource(IDD_MAIN),
            0,
            Some(main_dlg_proc),
            0,
        );
        if hwnd == 0 {
            if mutex != 0 {
                CloseHandle(mutex);
            }
            return;
        }

        let start_min = cli_minimized || with_state(|st| st.config.start_minimized);
        if !start_min {
            ShowWindow(hwnd, SW_SHOWNORMAL);
            SetForegroundWindow(hwnd);
        } else {
            ShowWindow(hwnd, SW_HIDE);
        }

        let haccel = LoadAcceleratorsW(hinst, make_int_resource(IDA_MAIN));
        let mut msg: MSG = std::mem::zeroed();

        while GetMessageW(&mut msg, 0, 0, 0) > 0 {
            let settings_hwnd = with_state(|st| st.settings_hwnd);
            if settings_hwnd != 0 && IsDialogMessageW(settings_hwnd, &mut msg) != FALSE {
                continue;
            }
            if haccel != 0 && TranslateAcceleratorW(hwnd, haccel, &mut msg) != 0 {
                continue;
            }
            if IsDialogMessageW(hwnd, &mut msg) != FALSE {
                continue;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        if mutex != 0 {
            CloseHandle(mutex);
        }
    }
}

pub fn open_config_file_in_editor() {
    let ini_path = with_state(|st| {
        st.config.save();
        st.config.ini_path.clone()
    });
    unsafe {
        ShellExecuteW(
            0,
            to_wide("open").as_ptr(),
            to_wide(&ini_path.to_string_lossy()).as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
    }
}

fn hide_to_background_tray(hwnd: HWND) {
    save_window_position(hwnd);
    unsafe {
        ShowWindow(hwnd, SW_HIDE);
    }
    let (should_notify, msg, sound) = with_state_mut(|st| {
        if !st.config.background_balloon_shown {
            st.config.background_balloon_shown = true;
            st.config.save();
            (
                true,
                st.locale.get(IDS_BG_RUNNING_INFO),
                st.config.notifications_sound,
            )
        } else {
            (false, String::new(), false)
        }
    });

    if should_notify {
        let mut flags = NIIF_INFO;
        if !sound {
            flags |= NIIF_NOSOUND;
        }
        let _ = tray_popup(hwnd, flags, APP_NAME, &msg);
    }
}

fn restore_saved_window_position(hwnd: HWND) {
    let (opt_x, opt_y) = with_state(|st| (st.config.window_pos_x, st.config.window_pos_y));
    if let (Some(x), Some(y)) = (opt_x, opt_y) {
        unsafe {
            let screen_w = GetSystemMetrics(SM_CXSCREEN).max(640);
            let screen_h = GetSystemMetrics(SM_CYSCREEN).max(480);
            let clamped_x = x.clamp(0, (screen_w - 120).max(0));
            let clamped_y = y.clamp(0, (screen_h - 120).max(0));
            SetWindowPos(
                hwnd,
                0,
                clamped_x,
                clamped_y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }
}

fn save_window_position(hwnd: HWND) {
    unsafe {
        if IsWindowVisible(hwnd) == FALSE || IsIconic(hwnd) != FALSE {
            return;
        }
        let mut rc = RECT::default();
        if GetWindowRect(hwnd, &mut rc) != FALSE {
            with_state_mut(|st| {
                if st.config.window_pos_x != Some(rc.left)
                    || st.config.window_pos_y != Some(rc.top)
                {
                    st.config.window_pos_x = Some(rc.left);
                    st.config.window_pos_y = Some(rc.top);
                    st.config.save();
                }
            });
        }
    }
}

pub fn perform_cleanup(
    hwnd: HWND,
    source: CleanupSource,
    explicit_mask: Option<u32>,
    silent: bool,
) {
    if !is_elevated() {
        if relaunch_as_admin() {
            if hwnd != 0 {
                unsafe {
                    DestroyWindow(hwnd);
                }
            }
            return;
        }
        if !silent {
            let (msg, sound) = with_state(|st| {
                (
                    st.locale.get(IDS_STATUS_NOPRIVILEGES),
                    st.config.notifications_sound,
                )
            });
            let mut flags = NIIF_WARNING;
            if !sound {
                flags |= NIIF_NOSOUND;
            }
            if hwnd == 0 || !tray_popup(hwnd, flags, APP_NAME, &msg) {
                show_msgbox(hwnd, APP_NAME, &msg, MB_OK | MB_ICONSTOP);
            }
        }
        return;
    }

    let (base_mask, allow_standby_auto, whitelist, show_confirm) = with_state(|st| {
        (
            explicit_mask.unwrap_or(st.config.reduct_mask),
            st.config.allow_standby_cleanup,
            st.config.process_whitelist.clone(),
            st.config.show_reduct_confirmation,
        )
    });

    let mut preview_mask = base_mask & REDUCT_MASK_ALL;
    if source == CleanupSource::Auto && !allow_standby_auto {
        preview_mask &= !REDUCT_MASK_FREEZES;
    }
    if preview_mask == 0 {
        return;
    }

    let regions_text = with_state(|st| format_regions_list(preview_mask, &st.locale));

    if source == CleanupSource::Manual && show_confirm && !silent && hwnd != 0 {
        let (q_txt, dont_ask_txt) = with_state(|st| {
            (
                st.locale.get(IDS_QUESTION),
                st.locale.get(IDS_QUESTION_FLAG_CHK),
            )
        });
        let prompt = format!("{q_txt}\r\n\r\n{regions_text}");
        let (confirmed, dont_ask) = show_confirm_dialog(hwnd, &prompt, &dont_ask_txt);
        if !confirmed {
            return;
        }
        if dont_ask {
            with_state_mut(|st| {
                st.config.show_reduct_confirmation = false;
                st.config.save();
                unsafe {
                    menu_check(GetMenu(hwnd), IDM_REDUCTCONFIRMATION_CHK, false);
                }
            });
        }
    }

    unsafe {
        SetCursor(LoadCursorW(0, make_int_resource(IDC_WAIT_ID)));
    }

    let outcome = execute_memory_clean(base_mask, source, allow_standby_auto, &whitelist);
    let cleaned_regions_text =
        with_state(|st| format_regions_list(outcome.effective_mask, &st.locale));

    unsafe {
        SetCursor(LoadCursorW(0, make_int_resource(IDC_ARROW_ID)));
    }

    let freed_str = format_bytes(outcome.reported_freed);

    let (result_message, balloon_enabled, sound_enabled, log_enabled) = with_state_mut(|st| {
        let now = unix_now();
        st.config.statistic_last_reduct = now;
        st.config.save();

        st.session_cleaned_bytes = st
            .session_cleaned_bytes
            .saturating_add(outcome.reported_freed);
        st.session_clean_count = st.session_clean_count.saturating_add(1);

        let headline = st.locale.format_cleaned(&freed_str);
        let region_title = st.locale.get(IDS_TITLE_3);
        let mut full_msg = format!("{headline}\r\n\r\n{region_title}:\r\n{cleaned_regions_text}");
        if let Some((trimmed, skipped)) = outcome.selective_trimmed {
            full_msg.push_str(&format!(
                "\r\n\r\nWhitelist active: {trimmed} trimmed, {skipped} skipped"
            ));
        }

        (
            full_msg,
            st.config.balloon_clean_results,
            st.config.notifications_sound,
            st.config.log_clean_results,
        )
    });

    if log_enabled {
        let mut log_detail = freed_str.clone();
        if !outcome.errors.is_empty() {
            log_detail.push_str(&format!(" (warnings: {})", outcome.errors.join(", ")));
        }
        with_state(|st| st.config.append_log(source.reason_label(), &log_detail));
    }

    if !silent {
        let mut flags = NIIF_INFO;
        if !sound_enabled {
            flags |= NIIF_NOSOUND;
        }

        if source == CleanupSource::Cmdline {
            if balloon_enabled && hwnd != 0 && tray_popup(hwnd, flags, APP_NAME, &result_message) {
                // Shown via tray
            } else {
                let title = with_state(|st| st.locale.get(IDS_SETTINGS_MEMORY));
                show_msgbox(hwnd, &title, &result_message, MB_OK | MB_ICONINFORMATION);
            }
        } else if hwnd != 0 && balloon_enabled {
            let _ = tray_popup(hwnd, flags, APP_NAME, &result_message);
        }
    }

    if hwnd != 0 {
        on_timer_tick(hwnd);
    }
}

pub fn refresh_all_ui_after_settings() {
    let (hwnd, dark, always_on_top, cfg) = with_state_mut(|st| {
        let h = st.main_hwnd;
        let d = st.config.use_dark_theme;
        let top = st.config.always_on_top;
        let c = st.config.clone();
        let dpi = get_window_dpi(h);
        st.tray.init(&c, dpi);
        (h, d, top, c)
    });

    if hwnd == 0 {
        return;
    }

    set_window_topmost(hwnd, always_on_top);
    apply_dark_mode_window(hwnd, dark);
    apply_listview_theme(hwnd, IDC_LISTVIEW, dark);
    localize_main_window(hwnd);
    sync_main_menu_checks(hwnd);
    register_clean_hotkey(hwnd, &cfg);
    on_timer_tick(hwnd);
}

fn setup_main_window(hwnd: HWND) {
    let elevated = is_elevated();
    if elevated {
        enable_required_privileges();
    } else {
        unsafe {
            SendDlgItemMessageW(hwnd, IDC_CLEAN, BCM_SETSHIELD, 0, TRUE as isize);
        }
    }

    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        let hicon = LoadIconW(hinst, make_int_resource(IDI_MAIN));
        if hicon != 0 {
            SendMessageW(hwnd, WM_SETICON, ICON_SMALL, hicon as isize);
            SendMessageW(hwnd, WM_SETICON, ICON_BIG, hicon as isize);
        }
    }

    restore_saved_window_position(hwnd);
    with_state_mut(|st| st.initialized_pos = true);

    lv_set_ex_style(
        hwnd,
        IDC_LISTVIEW,
        LVS_EX_FULLROWSELECT | LVS_EX_INFOTIP | LVS_EX_LABELTIP | LVS_EX_DOUBLEBUFFER,
    );
    lv_reset_columns_and_items(hwnd, IDC_LISTVIEW);
    lv_add_column(hwnd, IDC_LISTVIEW, 0, 120, LVCFMT_LEFT);
    lv_add_column(hwnd, IDC_LISTVIEW, 1, 90, LVCFMT_RIGHT);
    lv_enable_groups(hwnd, IDC_LISTVIEW, true);

    with_state(|st| {
        let loc = &st.locale;
        for g in 0..3 {
            lv_add_group(hwnd, IDC_LISTVIEW, g, &loc.get(IDS_GROUP_1 + g as u32));
            for j in 0..3 {
                let idx = g * 3 + j;
                lv_insert_item(
                    hwnd,
                    IDC_LISTVIEW,
                    idx,
                    &loc.get(IDS_ITEM_1 + j as u32),
                    Some(g),
                    0,
                );
            }
        }
        lv_add_group(hwnd, IDC_LISTVIEW, 3, &loc.get(IDS_GROUP_4));
        lv_insert_item(
            hwnd,
            IDC_LISTVIEW,
            9,
            &loc.get(IDS_ITEM_STANDBY),
            Some(3),
            0,
        );
        lv_insert_item(
            hwnd,
            IDC_LISTVIEW,
            10,
            &loc.get(IDS_ITEM_MODIFIED),
            Some(3),
            0,
        );
        lv_insert_item(
            hwnd,
            IDC_LISTVIEW,
            11,
            &loc.get(IDS_ITEM_SESSION_CLEANED),
            Some(3),
            0,
        );
    });

    lv_resize_two_columns_50_50(hwnd, IDC_LISTVIEW);

    let (dark, always_on_top, cfg) = with_state_mut(|st| {
        let dpi = get_window_dpi(hwnd);
        let c = st.config.clone();
        st.tray.init(&c, dpi);
        (st.config.use_dark_theme, st.config.always_on_top, c)
    });

    set_window_topmost(hwnd, always_on_top);
    apply_dark_mode_window(hwnd, dark);
    apply_listview_theme(hwnd, IDC_LISTVIEW, dark);

    localize_main_window(hwnd);
    sync_main_menu_checks(hwnd);

    let snap = query_memory_snapshot();
    let initial_icon = with_state_mut(|st| {
        let c = st.config.clone();
        st.tray.current_or_render(snap.phys_percent, &c)
    });
    tray_create(hwnd, initial_icon, APP_NAME);

    register_clean_hotkey(hwnd, &cfg);

    on_timer_tick(hwnd);
}

fn sync_main_menu_checks(hwnd: HWND) {
    unsafe {
        let hmenu = GetMenu(hwnd);
        if hmenu == 0 {
            return;
        }
        with_state(|st| {
            menu_check(hmenu, IDM_ALWAYSONTOP_CHK, st.config.always_on_top);
            menu_check(hmenu, IDM_RUNINBACKGROUND_CHK, st.config.run_in_background);
            menu_check(hmenu, IDM_USEDARKTHEME, st.config.use_dark_theme);
            menu_check(hmenu, IDM_LOADONSTARTUP_CHK, is_autorun_enabled());
            menu_check(hmenu, IDM_STARTMINIMIZED_CHK, st.config.start_minimized);
            menu_check(
                hmenu,
                IDM_REDUCTCONFIRMATION_CHK,
                st.config.show_reduct_confirmation,
            );
            menu_check(hmenu, IDM_SKIPUACWARNING_CHK, is_skip_uac_enabled());
            menu_check(hmenu, IDM_CHECKUPDATES_CHK, st.config.check_updates);
            menu_enable(hmenu, IDM_SKIPUACWARNING_CHK, is_elevated());
        });
    }
}

fn localize_main_window(hwnd: HWND) {
    with_state(|st| {
        let loc = &st.locale;
        set_dlg_text(hwnd, IDC_CLEAN, &loc.get(IDS_CLEAN));

        for g in 0..3 {
            lv_set_group_header(hwnd, IDC_LISTVIEW, g, &loc.get(IDS_GROUP_1 + g as u32));
            for j in 0..3 {
                lv_set_item_text(hwnd, IDC_LISTVIEW, g * 3 + j, 0, &loc.get(IDS_ITEM_1 + j as u32));
            }
        }
        lv_set_group_header(hwnd, IDC_LISTVIEW, 3, &loc.get(IDS_GROUP_4));
        lv_set_item_text(hwnd, IDC_LISTVIEW, 9, 0, &loc.get(IDS_ITEM_STANDBY));
        lv_set_item_text(hwnd, IDC_LISTVIEW, 10, 0, &loc.get(IDS_ITEM_MODIFIED));
        lv_set_item_text(
            hwnd,
            IDC_LISTVIEW,
            11,
            0,
            &loc.get(IDS_ITEM_SESSION_CLEANED),
        );

        unsafe {
            let hmenu = GetMenu(hwnd);
            if hmenu != 0 {
                menu_set_text(hmenu, 0, true, &loc.get(IDS_FILE));
                menu_set_text(hmenu, 1, true, &loc.get(IDS_VIEW));
                menu_set_text(hmenu, 2, true, &loc.get(IDS_SETTINGS));
                menu_set_text(hmenu, 3, true, &loc.get(IDS_HELP));

                menu_set_text(
                    hmenu,
                    IDM_SETTINGS as u32,
                    false,
                    &format!("{}...\tF2", loc.get(IDS_SETTINGS)),
                );
                menu_set_text(
                    hmenu,
                    IDM_OPENCONFIG as u32,
                    false,
                    &loc.get(IDS_OPENCONFIG),
                );
                menu_set_text(hmenu, IDM_EXIT as u32, false, &loc.get(IDS_EXIT));
                menu_set_text(
                    hmenu,
                    IDM_ALWAYSONTOP_CHK as u32,
                    false,
                    &loc.get(IDS_ALWAYSONTOP_CHK),
                );
                menu_set_text(
                    hmenu,
                    IDM_USEDARKTHEME as u32,
                    false,
                    &loc.get(IDS_USEDARKTHEME),
                );
                menu_set_text(
                    hmenu,
                    IDM_REDUCTCONFIRMATION_CHK as u32,
                    false,
                    &loc.get(IDS_REDUCTCONFIRMATION_CHK),
                );
                menu_set_text(
                    hmenu,
                    IDM_RUNINBACKGROUND_CHK as u32,
                    false,
                    &loc.get(IDS_RUNINBACKGROUND_CHK),
                );
                menu_set_text(
                    hmenu,
                    IDM_LOADONSTARTUP_CHK as u32,
                    false,
                    &loc.get(IDS_LOADONSTARTUP_CHK),
                );
                menu_set_text(
                    hmenu,
                    IDM_STARTMINIMIZED_CHK as u32,
                    false,
                    &loc.get(IDS_STARTMINIMIZED_CHK),
                );
                menu_set_text(
                    hmenu,
                    IDM_SKIPUACWARNING_CHK as u32,
                    false,
                    &loc.get(IDS_SKIPUACWARNING_CHK),
                );
                menu_set_text(
                    hmenu,
                    IDM_CHECKUPDATES_CHK as u32,
                    false,
                    &loc.get(IDS_CHECKUPDATES_CHK),
                );
                menu_set_text(hmenu, IDM_WEBSITE as u32, false, &loc.get(IDS_WEBSITE));
                menu_set_text(
                    hmenu,
                    IDM_CHECKUPDATES as u32,
                    false,
                    &loc.get(IDS_CHECKUPDATES),
                );
                menu_set_text(
                    hmenu,
                    IDM_ABOUT as u32,
                    false,
                    &format!("{}\tF1", loc.get(IDS_ABOUT)),
                );

                let hview = GetSubMenu(hmenu, 1);
                if hview != 0 {
                    menu_set_text(
                        hview,
                        4,
                        true,
                        &format!("{} (Language)", loc.get(IDS_LANGUAGE)),
                    );
                    let hlang = GetSubMenu(hview, 4);
                    if hlang != 0 {
                        while GetMenuItemCount(hlang) > 0 {
                            DeleteMenu(hlang, 0, MF_BYPOSITION);
                        }
                        AppendMenuW(
                            hlang,
                            MF_STRING,
                            IDX_LANGUAGE as usize,
                            to_wide("English (default)").as_ptr(),
                        );
                        let mut active_id = IDX_LANGUAGE as u32;
                        for (i, lang) in loc.languages.iter().enumerate() {
                            let id = (IDX_LANGUAGE + 1 + i as i32) as usize;
                            AppendMenuW(hlang, MF_STRING, id, to_wide(lang).as_ptr());
                            if *lang == loc.current_lang {
                                active_id = id as u32;
                            }
                        }
                        let last_id = (IDX_LANGUAGE + loc.languages.len() as i32) as u32;
                        CheckMenuRadioItem(
                            hlang,
                            IDX_LANGUAGE as u32,
                            last_id,
                            active_id,
                            MF_BYCOMMAND,
                        );
                    }
                }
            }
        }
    });
}

fn on_timer_tick(hwnd: HWND) {
    let snap = query_memory_snapshot();

    let should_auto = with_state_mut(|st| {
        if st.config.skip_fullscreen_autoclean && is_fullscreen_or_busy() {
            return false;
        }

        let now = unix_now();
        let mut fire = false;

        if st.config.autoreduct_enable && snap.phys_percent >= st.config.autoreduct_value {
            if now.saturating_sub(st.last_auto_limit_ts) >= AUTOREDUCT_COOLDOWN_SECS {
                st.last_auto_limit_ts = now;
                fire = true;
            }
        }

        if !fire && st.config.autoreduct_interval_enable {
            let interval_secs = (st.config.autoreduct_interval_value as i64) * 60;
            if now.saturating_sub(st.last_auto_interval_ts) >= interval_secs {
                st.last_auto_interval_ts = now;
                fire = true;
            }
        }

        fire
    });

    if should_auto {
        perform_cleanup(hwnd, CleanupSource::Auto, None, false);
        return;
    }

    let (new_icon, tip, session_bytes, session_cnt) = with_state_mut(|st| {
        let cfg = st.config.clone();
        let ic = st.tray.update_main_icon(snap.phys_percent, &cfg);
        let tip_str = format!(
            "{}: {:.1}%\r\n{}: {:.1}%\r\n{}: {:.1}%\r\n{}: {}",
            st.locale.get(IDS_GROUP_1),
            snap.phys_percent_f,
            st.locale.get(IDS_GROUP_2),
            snap.page_percent_f,
            st.locale.get(IDS_GROUP_3),
            snap.cache_percent_f,
            st.locale.get(IDS_ITEM_STANDBY),
            format_bytes(snap.standby_bytes),
        );
        (ic, tip_str, st.session_cleaned_bytes, st.session_clean_count)
    });

    tray_update(hwnd, new_icon, &tip);

    if unsafe { IsWindowVisible(hwnd) } == FALSE {
        return;
    }

    for i in 0..3 {
        lv_set_item_lparam(hwnd, IDC_LISTVIEW, i, snap.phys_percent as isize);
    }
    for i in 3..6 {
        lv_set_item_lparam(hwnd, IDC_LISTVIEW, i, snap.page_percent as isize);
    }
    for i in 6..9 {
        lv_set_item_lparam(hwnd, IDC_LISTVIEW, i, snap.cache_percent as isize);
    }

    lv_set_item_text(
        hwnd,
        IDC_LISTVIEW,
        0,
        1,
        &format!("{:.1}%", snap.phys_percent_f),
    );
    lv_set_item_text(hwnd, IDC_LISTVIEW, 1, 1, &format_bytes(snap.phys_free));
    lv_set_item_text(hwnd, IDC_LISTVIEW, 2, 1, &format_bytes(snap.phys_total));

    lv_set_item_text(
        hwnd,
        IDC_LISTVIEW,
        3,
        1,
        &format!("{:.1}%", snap.page_percent_f),
    );
    lv_set_item_text(hwnd, IDC_LISTVIEW, 4, 1, &format_bytes(snap.page_free));
    lv_set_item_text(hwnd, IDC_LISTVIEW, 5, 1, &format_bytes(snap.page_total));

    lv_set_item_text(
        hwnd,
        IDC_LISTVIEW,
        6,
        1,
        &format!("{:.1}%", snap.cache_percent_f),
    );
    lv_set_item_text(hwnd, IDC_LISTVIEW, 7, 1, &format_bytes(snap.cache_free));
    lv_set_item_text(hwnd, IDC_LISTVIEW, 8, 1, &format_bytes(snap.cache_total));

    lv_set_item_text(
        hwnd,
        IDC_LISTVIEW,
        9,
        1,
        &format!(
            "{} (P0: {})",
            format_bytes(snap.standby_bytes),
            format_bytes(snap.standby_prio0_bytes)
        ),
    );
    lv_set_item_text(
        hwnd,
        IDC_LISTVIEW,
        10,
        1,
        &format_bytes(snap.modified_bytes),
    );
    lv_set_item_text(
        hwnd,
        IDC_LISTVIEW,
        11,
        1,
        &format!("{} ({}x)", format_bytes(session_bytes), session_cnt),
    );
}

fn show_clean_split_dropdown(hwnd: HWND, btn_hwnd: HWND) {
    let menu = OwnedMenu(unsafe { CreatePopupMenu() });
    if menu.0 == 0 {
        return;
    }

    with_state(|st| {
        let loc = &st.locale;
        let entries: [(i32, String); 8] = [
            (IDM_CLEAN_WORKINGSET, loc.get(IDS_MEMREGION_WORKINGSET)),
            (
                IDM_CLEAN_SYSTEMFILECACHE,
                loc.get(IDS_MEMREGION_SYSTEMFILECACHE),
            ),
            (
                IDM_CLEAN_MODIFIEDFILECACHE,
                loc.get(IDS_MEMREGION_MODIFIEDFILECACHE),
            ),
            (
                IDM_CLEAN_MODIFIEDLIST,
                format!("{}*", loc.get(IDS_MEMREGION_MODIFIEDLIST)),
            ),
            (
                IDM_CLEAN_STANDBYLIST,
                format!("{}*", loc.get(IDS_MEMREGION_STANDBYLIST)),
            ),
            (
                IDM_CLEAN_STANDBYLISTPRIORITY0,
                loc.get(IDS_MEMREGION_STANDBYLISTPRIORITY0),
            ),
            (
                IDM_CLEAN_REGISTRYCACHE,
                format!("{} (win8.1+)", loc.get(IDS_MEMREGION_REGISTRYCACHE)),
            ),
            (
                IDM_CLEAN_COMBINEMEMORYLISTS,
                format!("{} (win10+)", loc.get(IDS_MEMREGION_COMBINEMEMORYLISTS)),
            ),
        ];

        for (id, text) in &entries {
            unsafe {
                AppendMenuW(menu.0, MF_STRING, *id as usize, to_wide(text).as_ptr());
            }
        }

        if !is_win8_1_or_greater() {
            menu_enable(menu.0, IDM_CLEAN_REGISTRYCACHE, false);
        }
        if !is_win10_or_greater() {
            menu_enable(menu.0, IDM_CLEAN_COMBINEMEMORYLISTS, false);
        }
    });

    unsafe {
        let mut rc = RECT::default();
        if GetWindowRect(btn_hwnd, &mut rc) != FALSE {
            SetForegroundWindow(hwnd);
            TrackPopupMenuEx(
                menu.0,
                TPM_RIGHTBUTTON,
                rc.left,
                rc.bottom,
                hwnd,
                std::ptr::null(),
            );
        }
    }
}

fn show_tray_context_menu(hwnd: HWND) {
    unsafe {
        SetForegroundWindow(hwnd);
        let hinst = GetModuleHandleW(std::ptr::null());
        let hmenu_guard = OwnedMenu(LoadMenuW(hinst, make_int_resource(IDM_TRAY)));
        if hmenu_guard.0 == 0 {
            return;
        }
        let hsub = GetSubMenu(hmenu_guard.0, 0);
        if hsub == 0 {
            return;
        }

        let hsub_regions = GetSubMenu(hsub, 4);
        let hsub_limits = GetSubMenu(hsub, 5);
        let hsub_intervals = GetSubMenu(hsub, 6);

        with_state_mut(|st| {
            let loc = &st.locale;
            menu_set_text(hsub, IDM_TRAY_SHOW as u32, false, &loc.get(IDS_TRAY_SHOW));
            menu_set_text(
                hsub,
                IDM_TRAY_CLEAN as u32,
                false,
                &format!("{}...", loc.get(IDS_CLEAN)),
            );
            menu_set_text(hsub, 4, true, &loc.get(IDS_TRAY_POPUP_1));
            menu_set_text(hsub, 5, true, &loc.get(IDS_TRAY_POPUP_2));
            menu_set_text(hsub, 6, true, &loc.get(IDS_TRAY_POPUP_3));
            menu_set_text(
                hsub,
                IDM_TRAY_SETTINGS as u32,
                false,
                &format!("{}...", loc.get(IDS_SETTINGS)),
            );
            menu_set_text(
                hsub,
                IDM_TRAY_WEBSITE as u32,
                false,
                &loc.get(IDS_WEBSITE),
            );
            menu_set_text(hsub, IDM_TRAY_ABOUT as u32, false, &loc.get(IDS_ABOUT));
            menu_set_text(hsub, IDM_TRAY_EXIT as u32, false, &loc.get(IDS_EXIT));

            if hsub_regions != 0 {
                let mask = st.config.reduct_mask;
                menu_set_text(
                    hsub_regions,
                    IDM_WORKINGSET_CHK as u32,
                    false,
                    &loc.get(IDS_MEMREGION_WORKINGSET),
                );
                menu_set_text(
                    hsub_regions,
                    IDM_SYSTEMFILECACHE_CHK as u32,
                    false,
                    &loc.get(IDS_MEMREGION_SYSTEMFILECACHE),
                );
                menu_set_text(
                    hsub_regions,
                    IDM_MODIFIEDFILECACHE_CHK as u32,
                    false,
                    &loc.get(IDS_MEMREGION_MODIFIEDFILECACHE),
                );
                menu_set_text(
                    hsub_regions,
                    IDM_MODIFIEDLIST_CHK as u32,
                    false,
                    &format!("{}*", loc.get(IDS_MEMREGION_MODIFIEDLIST)),
                );
                menu_set_text(
                    hsub_regions,
                    IDM_STANDBYLIST_CHK as u32,
                    false,
                    &format!("{}*", loc.get(IDS_MEMREGION_STANDBYLIST)),
                );
                menu_set_text(
                    hsub_regions,
                    IDM_STANDBYLISTPRIORITY0_CHK as u32,
                    false,
                    &loc.get(IDS_MEMREGION_STANDBYLISTPRIORITY0),
                );
                menu_set_text(
                    hsub_regions,
                    IDM_REGISTRYCACHE_CHK as u32,
                    false,
                    &format!("{} (win8.1+)", loc.get(IDS_MEMREGION_REGISTRYCACHE)),
                );
                menu_set_text(
                    hsub_regions,
                    IDM_COMBINEMEMORYLISTS_CHK as u32,
                    false,
                    &format!("{} (win10+)", loc.get(IDS_MEMREGION_COMBINEMEMORYLISTS)),
                );

                let region_pairs = [
                    (IDM_WORKINGSET_CHK, REDUCT_WORKINGSET),
                    (IDM_SYSTEMFILECACHE_CHK, REDUCT_SYSTEMFILECACHE),
                    (IDM_MODIFIEDFILECACHE_CHK, REDUCT_MODIFIEDFILECACHE),
                    (IDM_MODIFIEDLIST_CHK, REDUCT_MODIFIEDLIST),
                    (IDM_STANDBYLIST_CHK, REDUCT_STANDBYLIST),
                    (IDM_STANDBYLISTPRIORITY0_CHK, REDUCT_STANDBYPRIORITY0LIST),
                    (IDM_REGISTRYCACHE_CHK, REDUCT_REGISTRYCACHE),
                    (IDM_COMBINEMEMORYLISTS_CHK, REDUCT_COMBINEMEMORYLISTS),
                ];
                for (id, bit) in region_pairs {
                    menu_check(hsub_regions, id, (mask & bit) != 0);
                }
                if !is_win8_1_or_greater() {
                    menu_enable(hsub_regions, IDM_REGISTRYCACHE_CHK, false);
                }
                if !is_win10_or_greater() {
                    menu_enable(hsub_regions, IDM_COMBINEMEMORYLISTS_CHK, false);
                }
            }

            let disable_lbl = loc.get(IDS_TRAY_DISABLE);
            st.limits_arr = generate_dynamic_values(st.config.autoreduct_value, 99, 13);
            populate_tray_dynamic_menu(
                hsub_limits,
                IDX_TRAY_POPUP_1,
                &disable_lbl,
                &st.limits_arr,
                "%",
                st.config.autoreduct_value,
                st.config.autoreduct_enable,
                true,
            );

            st.intervals_arr =
                generate_dynamic_values(st.config.autoreduct_interval_value, 1440, 13);
            populate_tray_dynamic_menu(
                hsub_intervals,
                IDX_TRAY_POPUP_2,
                &disable_lbl,
                &st.intervals_arr,
                " min.",
                st.config.autoreduct_interval_value,
                st.config.autoreduct_interval_enable,
                true,
            );
        });

        let mut pt = POINT::default();
        GetCursorPos(&mut pt);
        TrackPopupMenuEx(
            hsub,
            TPM_RIGHTBUTTON,
            pt.x,
            pt.y,
            hwnd,
            std::ptr::null(),
        );
    }
}

unsafe extern "system" fn main_dlg_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    let taskbar_msg = APP_STATE.with(|c| {
        c.borrow()
            .as_ref()
            .map(|s| s.taskbar_created_msg)
            .unwrap_or(0)
    });
    if taskbar_msg != 0 && msg == taskbar_msg {
        let snap = query_memory_snapshot();
        let ic = with_state_mut(|st| {
            let dpi = get_window_dpi(hwnd);
            let cfg = st.config.clone();
            st.tray.init(&cfg, dpi);
            st.tray.current_or_render(snap.phys_percent, &cfg)
        });
        tray_create(hwnd, ic, APP_NAME);
        on_timer_tick(hwnd);
        return 1;
    }

    match msg {
        WM_INITDIALOG => {
            with_state_mut(|st| st.main_hwnd = hwnd);
            setup_main_window(hwnd);
            SetTimer(hwnd, TIMER_UID, TIMER_INTERVAL_MS, None);
            1
        }

        WM_TIMER => {
            if wparam == TIMER_UID {
                on_timer_tick(hwnd);
            }
            0
        }

        WM_MOVE => {
            if with_state(|st| st.initialized_pos) {
                save_window_position(hwnd);
            }
            0
        }

        WM_HOTKEY => {
            if wparam as i32 == HOTKEY_UID {
                perform_cleanup(hwnd, CleanupSource::Hotkey, None, false);
            }
            0
        }

        WM_SHOWWINDOW => {
            if wparam != 0 {
                on_timer_tick(hwnd);
            }
            0
        }

        WM_DPICHANGED => {
            if lparam != 0 {
                let suggested = &*(lparam as *const RECT);
                SetWindowPos(
                    hwnd,
                    0,
                    suggested.left,
                    suggested.top,
                    suggested.right - suggested.left,
                    suggested.bottom - suggested.top,
                    SWP_NOACTIVATE,
                );
            }
            lv_resize_two_columns_50_50(hwnd, IDC_LISTVIEW);
            with_state_mut(|st| {
                let dpi = get_window_dpi(hwnd);
                let cfg = st.config.clone();
                st.tray.init(&cfg, dpi);
            });
            on_timer_tick(hwnd);
            1
        }

        WM_SYSCOMMAND => {
            let cmd = wparam & 0xFFF0;
            if cmd == SC_MINIMIZE {
                hide_to_background_tray(hwnd);
                return 1;
            }
            if cmd == SC_CLOSE {
                if with_state(|st| st.config.run_in_background) {
                    hide_to_background_tray(hwnd);
                } else {
                    save_window_position(hwnd);
                    DestroyWindow(hwnd);
                }
                return 1;
            }
            0
        }

        WM_CLOSE => {
            if with_state(|st| st.config.run_in_background) {
                hide_to_background_tray(hwnd);
            } else {
                save_window_position(hwnd);
                DestroyWindow(hwnd);
            }
            1
        }

        WM_QUERYENDSESSION => 1,

        WM_ENDSESSION => {
            save_window_position(hwnd);
            with_state(|st| st.config.save());
            0
        }

        WM_DESTROY => {
            save_window_position(hwnd);
            KillTimer(hwnd, TIMER_UID);
            UnregisterHotKey(hwnd, HOTKEY_UID);
            tray_destroy(hwnd);
            with_state(|st| st.config.save());
            PostQuitMessage(0);
            1
        }

        WM_NOTIFY => {
            let nmhdr = &*(lparam as *const NMHDR);
            if nmhdr.code == BCN_DROPDOWN && nmhdr.idFrom as i32 == IDC_CLEAN {
                show_clean_split_dropdown(hwnd, nmhdr.hwndFrom);
                return 1;
            }
            if nmhdr.code == NM_CUSTOMDRAW && nmhdr.idFrom as i32 == IDC_LISTVIEW {
                let lvcd = &mut *(lparam as *mut NMLVCUSTOMDRAW);
                if lvcd.nmcd.dwDrawStage == CDDS_PREPAINT {
                    SetWindowLongPtrW(hwnd, DWLP_MSGRESULT, CDRF_NOTIFYITEMDRAW);
                    return CDRF_NOTIFYITEMDRAW;
                }
                if lvcd.nmcd.dwDrawStage == CDDS_ITEMPREPAINT {
                    let pct = lvcd.nmcd.lItemlParam as u32;
                    let (warn_lvl, danger_lvl, warn_clr, danger_clr) = with_state(|st| {
                        (
                            st.config.tray_level_warning,
                            st.config.tray_level_danger,
                            st.config.tray_color_warning,
                            st.config.tray_color_danger,
                        )
                    });
                    let mut ret = CDRF_DODEFAULT;
                    if pct >= danger_lvl {
                        lvcd.clrText = danger_clr;
                        ret = CDRF_NEWFONT;
                    } else if pct >= warn_lvl {
                        lvcd.clrText = warn_clr;
                        ret = CDRF_NEWFONT;
                    }
                    SetWindowLongPtrW(hwnd, DWLP_MSGRESULT, ret);
                    return ret;
                }
            }
            0
        }

        WM_TRAYICON => {
            let ev = loword(lparam as usize) as u32;
            match ev {
                NIN_KEYSELECT => {
                    if GetForegroundWindow() != hwnd {
                        toggle_window_visibility(hwnd, true);
                    }
                    1
                }
                WM_LBUTTONUP | WM_MBUTTONUP => {
                    let action = with_state(|st| {
                        if ev == WM_MBUTTONUP {
                            st.config.tray_action_mc
                        } else {
                            st.config.tray_action_sc
                        }
                    });
                    match action {
                        1 => perform_cleanup(hwnd, CleanupSource::Manual, None, false),
                        2 => open_task_manager(),
                        _ => toggle_window_visibility(hwnd, false),
                    }
                    1
                }
                WM_RBUTTONUP | WM_CONTEXTMENU => {
                    show_tray_context_menu(hwnd);
                    1
                }
                _ => 0,
            }
        }

        WM_COMMAND => {
            let id = loword(wparam) as i32;
            handle_main_command(hwnd, id);
            0
        }

        _ => 0,
    }
}

fn handle_main_command(hwnd: HWND, id: i32) {
    let lang_count = with_state(|st| st.locale.languages.len() as i32);
    if id >= IDX_LANGUAGE && id <= IDX_LANGUAGE + lang_count {
        with_state_mut(|st| {
            if id == IDX_LANGUAGE {
                st.config.language.clear();
                st.locale.set_language("");
            } else if let Some(lang) = st
                .locale
                .languages
                .get((id - IDX_LANGUAGE - 1) as usize)
                .cloned()
            {
                st.config.language = lang.clone();
                st.locale.set_language(&lang);
            }
            st.config.save();
        });
        localize_main_window(hwnd);
        let settings_hwnd = with_state(|st| st.settings_hwnd);
        if settings_hwnd != 0 {
            localize_settings_container_and_pages(settings_hwnd);
        }
        on_timer_tick(hwnd);
        return;
    }

    let limit_opt = with_state(|st| {
        let idx = id - IDX_TRAY_POPUP_1;
        if idx >= 0 && (idx as usize) < st.limits_arr.len() {
            Some(st.limits_arr[idx as usize])
        } else {
            None
        }
    });
    if let Some(val) = limit_opt {
        with_state_mut(|st| {
            st.config.autoreduct_enable = true;
            st.config.autoreduct_value = val;
            st.config.save();
        });
        sync_open_settings_from_config();
        return;
    }

    let interval_opt = with_state(|st| {
        let idx = id - IDX_TRAY_POPUP_2;
        if idx >= 0 && (idx as usize) < st.intervals_arr.len() {
            Some(st.intervals_arr[idx as usize])
        } else {
            None
        }
    });
    if let Some(val) = interval_opt {
        with_state_mut(|st| {
            st.config.autoreduct_interval_enable = true;
            st.config.autoreduct_interval_value = val;
            st.last_auto_interval_ts = unix_now();
            st.config.save();
        });
        sync_open_settings_from_config();
        return;
    }

    match id {
        IDOK | IDC_CLEAN | IDM_TRAY_CLEAN => {
            perform_cleanup(hwnd, CleanupSource::Manual, None, false);
        }

        IDCANCEL | IDM_TRAY_SHOW => {
            toggle_window_visibility(hwnd, false);
        }

        IDM_SETTINGS | IDM_TRAY_SETTINGS => {
            open_settings_dialog(hwnd);
        }

        IDM_OPENCONFIG => {
            open_config_file_in_editor();
        }

        IDM_EXIT | IDM_TRAY_EXIT => unsafe {
            save_window_position(hwnd);
            DestroyWindow(hwnd);
        },

        IDM_ALWAYSONTOP_CHK => {
            let new_val = with_state_mut(|st| {
                st.config.always_on_top = !st.config.always_on_top;
                st.config.save();
                st.config.always_on_top
            });
            set_window_topmost(hwnd, new_val);
            unsafe {
                menu_check(GetMenu(hwnd), IDM_ALWAYSONTOP_CHK, new_val);
            }
        }

        IDM_RUNINBACKGROUND_CHK => {
            let new_val = with_state_mut(|st| {
                st.config.run_in_background = !st.config.run_in_background;
                st.config.save();
                st.config.run_in_background
            });
            unsafe {
                menu_check(GetMenu(hwnd), IDM_RUNINBACKGROUND_CHK, new_val);
            }
        }

        IDM_USEDARKTHEME => {
            let new_val = with_state_mut(|st| {
                st.config.use_dark_theme = !st.config.use_dark_theme;
                st.config.save();
                st.config.use_dark_theme
            });
            apply_dark_mode_window(hwnd, new_val);
            apply_listview_theme(hwnd, IDC_LISTVIEW, new_val);
            unsafe {
                menu_check(GetMenu(hwnd), IDM_USEDARKTHEME, new_val);
            }
        }

        IDM_STARTMINIMIZED_CHK => {
            let new_val = with_state_mut(|st| {
                st.config.start_minimized = !st.config.start_minimized;
                st.config.save();
                st.config.start_minimized
            });
            unsafe {
                menu_check(GetMenu(hwnd), IDM_STARTMINIMIZED_CHK, new_val);
            }
        }

        IDM_REDUCTCONFIRMATION_CHK => {
            let new_val = with_state_mut(|st| {
                st.config.show_reduct_confirmation = !st.config.show_reduct_confirmation;
                st.config.save();
                st.config.show_reduct_confirmation
            });
            unsafe {
                menu_check(GetMenu(hwnd), IDM_REDUCTCONFIRMATION_CHK, new_val);
            }
        }

        IDM_LOADONSTARTUP_CHK => {
            let _ = set_autorun_enabled(!is_autorun_enabled());
            unsafe {
                menu_check(GetMenu(hwnd), IDM_LOADONSTARTUP_CHK, is_autorun_enabled());
            }
        }

        IDM_SKIPUACWARNING_CHK => {
            let _ = set_skip_uac_enabled(!is_skip_uac_enabled());
            unsafe {
                menu_check(GetMenu(hwnd), IDM_SKIPUACWARNING_CHK, is_skip_uac_enabled());
            }
        }

        IDM_CHECKUPDATES_CHK => {
            let new_val = with_state_mut(|st| {
                st.config.check_updates = !st.config.check_updates;
                st.config.save();
                st.config.check_updates
            });
            unsafe {
                menu_check(GetMenu(hwnd), IDM_CHECKUPDATES_CHK, new_val);
            }
        }

        IDM_TRAY_DISABLE_1 => {
            with_state_mut(|st| {
                st.config.autoreduct_enable = !st.config.autoreduct_enable;
                st.config.save();
            });
            sync_open_settings_from_config();
        }

        IDM_TRAY_DISABLE_2 => {
            with_state_mut(|st| {
                st.config.autoreduct_interval_enable = !st.config.autoreduct_interval_enable;
                st.last_auto_interval_ts = unix_now();
                st.config.save();
            });
            sync_open_settings_from_config();
        }

        IDM_WORKINGSET_CHK
        | IDM_SYSTEMFILECACHE_CHK
        | IDM_MODIFIEDFILECACHE_CHK
        | IDM_MODIFIEDLIST_CHK
        | IDM_STANDBYLIST_CHK
        | IDM_STANDBYLISTPRIORITY0_CHK
        | IDM_REGISTRYCACHE_CHK
        | IDM_COMBINEMEMORYLISTS_CHK => {
            let bit = match id {
                IDM_WORKINGSET_CHK => REDUCT_WORKINGSET,
                IDM_SYSTEMFILECACHE_CHK => REDUCT_SYSTEMFILECACHE,
                IDM_MODIFIEDFILECACHE_CHK => REDUCT_MODIFIEDFILECACHE,
                IDM_MODIFIEDLIST_CHK => REDUCT_MODIFIEDLIST,
                IDM_STANDBYLIST_CHK => REDUCT_STANDBYLIST,
                IDM_STANDBYLISTPRIORITY0_CHK => REDUCT_STANDBYPRIORITY0LIST,
                IDM_REGISTRYCACHE_CHK => REDUCT_REGISTRYCACHE,
                IDM_COMBINEMEMORYLISTS_CHK => REDUCT_COMBINEMEMORYLISTS,
                _ => return,
            };

            let (currently_set, need_warn) = with_state(|st| {
                (
                    (st.config.reduct_mask & bit) != 0,
                    st.config.show_warning_confirmation,
                )
            });

            if !currently_set && (bit & REDUCT_MASK_FREEZES) != 0 && need_warn {
                let (warn_txt, dont_ask_txt) = with_state(|st| {
                    (
                        st.locale.get(IDS_QUESTION_WARNING),
                        st.locale.get(IDS_QUESTION_FLAG_CHK),
                    )
                });
                let (ok, dont_ask) = show_confirm_dialog(hwnd, &warn_txt, &dont_ask_txt);
                if !ok {
                    return;
                }
                if dont_ask {
                    with_state_mut(|st| {
                        st.config.show_warning_confirmation = false;
                        st.config.save();
                    });
                }
            }

            with_state_mut(|st| {
                if currently_set {
                    st.config.reduct_mask &= !bit;
                } else {
                    st.config.reduct_mask |= bit;
                }
                st.config.save();
            });
            sync_open_settings_from_config();
        }

        IDM_CLEAN_WORKINGSET
        | IDM_CLEAN_SYSTEMFILECACHE
        | IDM_CLEAN_MODIFIEDFILECACHE
        | IDM_CLEAN_MODIFIEDLIST
        | IDM_CLEAN_STANDBYLIST
        | IDM_CLEAN_STANDBYLISTPRIORITY0
        | IDM_CLEAN_REGISTRYCACHE
        | IDM_CLEAN_COMBINEMEMORYLISTS => {
            let bit = match id {
                IDM_CLEAN_WORKINGSET => REDUCT_WORKINGSET,
                IDM_CLEAN_SYSTEMFILECACHE => REDUCT_SYSTEMFILECACHE,
                IDM_CLEAN_MODIFIEDFILECACHE => REDUCT_MODIFIEDFILECACHE,
                IDM_CLEAN_MODIFIEDLIST => REDUCT_MODIFIEDLIST,
                IDM_CLEAN_STANDBYLIST => REDUCT_STANDBYLIST,
                IDM_CLEAN_STANDBYLISTPRIORITY0 => REDUCT_STANDBYPRIORITY0LIST,
                IDM_CLEAN_REGISTRYCACHE => REDUCT_REGISTRYCACHE,
                IDM_CLEAN_COMBINEMEMORYLISTS => REDUCT_COMBINEMEMORYLISTS,
                _ => return,
            };
            perform_cleanup(hwnd, CleanupSource::ManualRegion, Some(bit), false);
        }

        IDM_WEBSITE | IDM_TRAY_WEBSITE => {
            open_url(APP_WEBSITE);
        }

        IDM_CHECKUPDATES => {
            open_url("https://github.com/henrypp/memreduct/releases");
        }

        IDM_ABOUT | IDM_TRAY_ABOUT => {
            show_about_box(hwnd);
        }

        _ => {}
    }
}
