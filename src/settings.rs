use crate::config::*;
use crate::ffi::*;
use crate::ids::*;
use crate::service::{install_and_start_service, is_service_installed, stop_and_uninstall_service};
use crate::winutil::*;
use crate::{open_config_file_in_editor, refresh_all_ui_after_settings, with_state, with_state_mut};

const PAGE_TEMPLATE_IDS: [u16; 5] = [
    IDD_SETTINGS_GENERAL,
    IDD_SETTINGS_MEMORY,
    IDD_SETTINGS_APPEARANCE,
    IDD_SETTINGS_TRAY,
    IDD_SETTINGS_ADVANCED,
];

// Deterministic mapping of IDC_REGIONS row index -> memory region bitmask
pub const REGION_BITS_BY_INDEX: [u32; 8] = [
    REDUCT_WORKINGSET,
    REDUCT_SYSTEMFILECACHE,
    REDUCT_MODIFIEDLIST,
    REDUCT_STANDBYLIST,
    REDUCT_STANDBYPRIORITY0LIST,
    REDUCT_MODIFIEDFILECACHE,
    REDUCT_REGISTRYCACHE,
    REDUCT_COMBINEMEMORYLISTS,
];

struct SettingsUiState {
    pages: [HWND; 5],
    suppress_events: bool,
}

thread_local! {
    static SETTINGS_UI: std::cell::RefCell<SettingsUiState> = const {
        std::cell::RefCell::new(SettingsUiState {
            pages: [0; 5],
            suppress_events: false,
        })
    };
}

fn set_suppress(val: bool) {
    SETTINGS_UI.with(|c| c.borrow_mut().suppress_events = val);
}

pub fn open_settings_dialog(parent: HWND) {
    let existing = with_state(|st| st.settings_hwnd);
    if existing != 0 {
        unsafe {
            ShowWindow(existing, SW_RESTORE);
            SetForegroundWindow(existing);
        }
        return;
    }

    unsafe {
        let hdlg = CreateDialogParamW(
            GetModuleHandleW(std::ptr::null()),
            make_int_resource(IDD_SETTINGS_CONTAINER),
            parent,
            Some(settings_container_proc),
            0,
        );
        if hdlg != 0 {
            with_state_mut(|st| st.settings_hwnd = hdlg);
            ShowWindow(hdlg, SW_SHOW);
        }
    }
}

/// Syncs an already-open Settings dialog when config is changed externally (e.g. from Tray menu)
pub fn sync_open_settings_from_config() {
    let settings_hwnd = with_state(|st| st.settings_hwnd);
    if settings_hwnd == 0 {
        return;
    }
    set_suppress(true);
    let pages = SETTINGS_UI.with(|c| c.borrow().pages);
    for (idx, &p) in pages.iter().enumerate() {
        if p != 0 {
            init_settings_page_inner(p, PAGE_TEMPLATE_IDS[idx]);
            localize_settings_page_inner(p, PAGE_TEMPLATE_IDS[idx]);
        }
    }
    set_suppress(false);
}

fn read_regions_mask_from_ui(mem_page: HWND) -> u32 {
    let mut mask = 0u32;
    for (idx, &bit) in REGION_BITS_BY_INDEX.iter().enumerate() {
        if (!is_win8_1_or_greater() && bit == REDUCT_REGISTRYCACHE)
            || (!is_win10_or_greater() && bit == REDUCT_COMBINEMEMORYLISTS)
        {
            continue;
        }
        if lv_is_checked(mem_page, IDC_REGIONS, idx as i32) {
            mask |= bit;
        }
    }
    mask
}

/// Explicitly reads & saves all control states across all 5 child pages BEFORE tab switch or window close
pub fn commit_all_settings_pages() {
    let pages = SETTINGS_UI.with(|c| c.borrow().pages);

    with_state_mut(|st| {
        // Page 0: General
        let p0 = pages[0];
        if p0 != 0 {
            st.config.always_on_top = is_checked(p0, IDC_ALWAYSONTOP_CHK);
            st.config.run_in_background = is_checked(p0, IDC_RUNINBACKGROUND_CHK);
            st.config.start_minimized = is_checked(p0, IDC_STARTMINIMIZED_CHK);
            st.config.show_reduct_confirmation = is_checked(p0, IDC_REDUCTCONFIRMATION_CHK);
            st.config.check_updates = is_checked(p0, IDC_CHECKUPDATES_CHK);
        }

        // Page 1: Memory cleaning
        let p1 = pages[1];
        if p1 != 0 {
            st.config.reduct_mask = read_regions_mask_from_ui(p1);
            st.config.autoreduct_enable = is_checked(p1, IDC_AUTOREDUCTENABLE_CHK);
            if let Some(v) = updown_get_opt(p1, IDC_AUTOREDUCTVALUE, IDC_AUTOREDUCTVALUE_CTRL) {
                st.config.autoreduct_value = v.clamp(1, 99) as u32;
            }

            let prev_interval_en = st.config.autoreduct_interval_enable;
            st.config.autoreduct_interval_enable =
                is_checked(p1, IDC_AUTOREDUCTINTERVALENABLE_CHK);
            if st.config.autoreduct_interval_enable && !prev_interval_en {
                st.last_auto_interval_ts = unix_now();
            }
            if let Some(v) =
                updown_get_opt(p1, IDC_AUTOREDUCTINTERVALVALUE, IDC_AUTOREDUCTINTERVALVALUE_CTRL)
            {
                st.config.autoreduct_interval_value = v.clamp(1, 1440) as u32;
            }

            st.config.hotkey_clean_enable = is_checked(p1, IDC_HOTKEY_CLEAN_CHK);
            let hk =
                unsafe { SendDlgItemMessageW(p1, IDC_HOTKEY_CLEAN, HKM_GETHOTKEY, 0, 0) } as u32;
            if hk != 0 {
                st.config.hotkey_clean = hk;
            }
        }

        // Page 2: Appearance
        let p2 = pages[2];
        if p2 != 0 {
            st.config.tray_use_transparency = is_checked(p2, IDC_TRAYUSETRANSPARENCY_CHK);
            st.config.tray_show_border = is_checked(p2, IDC_TRAYSHOWBORDER_CHK);
            st.config.tray_round_corners = is_checked(p2, IDC_TRAYROUNDCORNERS_CHK);
            st.config.tray_change_bg = is_checked(p2, IDC_TRAYCHANGEBG_CHK);
            st.config.tray_use_antialiasing = is_checked(p2, IDC_TRAYUSEANTIALIASING_CHK);
        }

        // Page 3: Tray icon
        let p3 = pages[3];
        if p3 != 0 {
            if let Some(w) = updown_get_opt(p3, IDC_TRAYLEVELWARNING, IDC_TRAYLEVELWARNING_CTRL) {
                st.config.tray_level_warning = w.clamp(1, 98) as u32;
            }
            if let Some(d) = updown_get_opt(p3, IDC_TRAYLEVELDANGER, IDC_TRAYLEVELDANGER_CTRL) {
                st.config.tray_level_danger = d.clamp(2, 99) as u32;
            }
            st.config.normalize_warning_danger(true);

            let sc =
                unsafe { SendDlgItemMessageW(p3, IDC_TRAYACTIONSC, CB_GETCURSEL, 0, 0) } as i32;
            if (0..=2).contains(&sc) {
                st.config.tray_action_sc = sc;
            }
            let mc =
                unsafe { SendDlgItemMessageW(p3, IDC_TRAYACTIONMC, CB_GETCURSEL, 0, 0) } as i32;
            if (0..=2).contains(&mc) {
                st.config.tray_action_mc = mc;
            }

            st.config.balloon_clean_results = is_checked(p3, IDC_SHOW_CLEAN_RESULT_CHK);
            st.config.notifications_sound = is_checked(p3, IDC_NOTIFICATIONSOUND_CHK);
        }

        // Page 4: Advanced
        let p4 = pages[4];
        if p4 != 0 {
            st.config.allow_standby_cleanup = is_checked(p4, IDC_ALLOWSTANDBYLISTCLEANUP_CHK);
            st.config.log_clean_results = is_checked(p4, IDC_LOGRESULTS_CHK);
            st.config.skip_fullscreen_autoclean = is_checked(p4, IDC_SKIPFULLSCREEN_CHK);
            st.config.process_whitelist = get_dlg_text(p4, IDC_WHITELIST_EDIT);
        }

        st.config.save();
    });

    refresh_all_ui_after_settings();
}

unsafe extern "system" fn settings_container_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    _lparam: LPARAM,
) -> isize {
    match msg {
        WM_INITDIALOG => {
            // CRITICAL FIX: Suppress child control EN_CHANGE / LVN_ITEMCHANGED BEFORE CreateDialogParamW
            // so initial empty EDITTEXT controls cannot overwrite config values with 1!
            set_suppress(true);

            let dark = with_state(|st| st.config.use_dark_theme);
            apply_dark_mode_window(hwnd, dark);

            let hinst = GetModuleHandleW(std::ptr::null());
            let mut created = [0; 5];
            for (idx, &tpl) in PAGE_TEMPLATE_IDS.iter().enumerate() {
                let page = CreateDialogParamW(
                    hinst,
                    make_int_resource(tpl),
                    hwnd,
                    Some(settings_page_proc),
                    tpl as isize,
                );
                created[idx] = page;
                if page != 0 {
                    apply_dark_mode_window(page, dark);
                    init_settings_page_inner(page, tpl);
                    ShowWindow(page, if idx == 0 { SW_SHOW } else { SW_HIDE });
                }
            }

            SETTINGS_UI.with(|cell| {
                cell.borrow_mut().pages = created;
            });

            localize_settings_container_and_pages(hwnd);
            SendDlgItemMessageW(hwnd, IDC_SETTINGS_NAV, LB_SETCURSEL, 0, 0);

            set_suppress(false);
            1
        }

        WM_COMMAND => {
            let id = loword(wparam) as i32;
            let code = hiword(wparam);

            if id == IDC_SETTINGS_NAV && code == LBN_SELCHANGE {
                commit_all_settings_pages();
                let sel = SendDlgItemMessageW(hwnd, IDC_SETTINGS_NAV, LB_GETCURSEL, 0, 0) as usize;
                SETTINGS_UI.with(|cell| {
                    let pages = cell.borrow().pages;
                    for (idx, &p) in pages.iter().enumerate() {
                        if p != 0 {
                            ShowWindow(p, if idx == sel { SW_SHOW } else { SW_HIDE });
                        }
                    }
                    if sel == 2 && pages[2] != 0 {
                        refresh_appearance_preview(pages[2]);
                    }
                });
                return 1;
            }

            if id == IDC_OPENCONFIG_BTN {
                commit_all_settings_pages();
                open_config_file_in_editor();
                return 1;
            }

            if id == IDC_SETTINGS_RESET {
                let prompt = with_state(|st| st.locale.get(IDS_QUESTION_RESET));
                if show_msgbox(hwnd, APP_NAME, &prompt, MB_YESNO | MB_ICONWARNING) == IDYES {
                    set_suppress(true);
                    with_state_mut(|st| {
                        st.config.reset_to_defaults();
                        let lang = st.config.language.clone();
                        st.locale.set_language(&lang);
                    });
                    let pages = SETTINGS_UI.with(|cell| cell.borrow().pages);
                    for (idx, &p) in pages.iter().enumerate() {
                        if p != 0 {
                            init_settings_page_inner(p, PAGE_TEMPLATE_IDS[idx]);
                        }
                    }
                    localize_settings_container_and_pages(hwnd);
                    set_suppress(false);
                    refresh_all_ui_after_settings();
                }
                return 1;
            }

            if id == IDC_SETTINGS_CLOSE || id == IDOK || id == IDCANCEL {
                commit_all_settings_pages();
                set_suppress(true);
                DestroyWindow(hwnd);
                return 1;
            }

            0
        }

        WM_CLOSE => {
            commit_all_settings_pages();
            set_suppress(true);
            DestroyWindow(hwnd);
            1
        }

        WM_DESTROY => {
            SETTINGS_UI.with(|cell| {
                let mut ui = cell.borrow_mut();
                ui.pages = [0; 5];
                ui.suppress_events = false;
            });
            with_state_mut(|st| {
                st.settings_hwnd = 0;
                st.config.save();
            });
            1
        }

        _ => 0,
    }
}

pub fn localize_settings_container_and_pages(container_hwnd: HWND) {
    if container_hwnd == 0 {
        return;
    }

    let was_suppressed = SETTINGS_UI.with(|c| c.borrow().suppress_events);
    set_suppress(true);

    let (title, reset_txt, open_cfg_txt, close_txt, nav_labels) = with_state(|st| {
        (
            st.locale.get(IDS_SETTINGS),
            st.locale.get(IDS_RESET),
            st.locale.get(IDS_OPENCONFIG),
            st.locale.get(IDS_CLOSE),
            [
                st.locale.get(IDS_SETTINGS_GENERAL),
                st.locale.get(IDS_SETTINGS_MEMORY),
                st.locale.get(IDS_SETTINGS_APPEARANCE),
                st.locale.get(IDS_SETTINGS_TRAY),
                st.locale.get(IDS_TITLE_ADVANCED),
            ],
        )
    });

    unsafe {
        SetWindowTextW(container_hwnd, to_wide(&title).as_ptr());
        set_dlg_text(container_hwnd, IDC_SETTINGS_RESET, &reset_txt);
        set_dlg_text(container_hwnd, IDC_OPENCONFIG_BTN, &open_cfg_txt);
        set_dlg_text(container_hwnd, IDC_SETTINGS_CLOSE, &close_txt);

        let cur = SendDlgItemMessageW(container_hwnd, IDC_SETTINGS_NAV, LB_GETCURSEL, 0, 0).max(0);
        SendDlgItemMessageW(container_hwnd, IDC_SETTINGS_NAV, LB_RESETCONTENT, 0, 0);
        for lbl in &nav_labels {
            SendDlgItemMessageW(
                container_hwnd,
                IDC_SETTINGS_NAV,
                LB_ADDSTRING,
                0,
                to_wide(lbl).as_ptr() as isize,
            );
        }
        SendDlgItemMessageW(container_hwnd, IDC_SETTINGS_NAV, LB_SETCURSEL, cur as usize, 0);
    }

    let pages = SETTINGS_UI.with(|cell| cell.borrow().pages);
    for (idx, &p) in pages.iter().enumerate() {
        if p != 0 {
            localize_settings_page_inner(p, PAGE_TEMPLATE_IDS[idx]);
        }
    }

    set_suppress(was_suppressed);
}

fn init_settings_page_inner(hwnd: HWND, page_id: u16) {
    let elevated = is_elevated();
    let cfg = with_state(|st| st.config.clone());

    match page_id {
        IDD_SETTINGS_GENERAL => {
            set_check(hwnd, IDC_ALWAYSONTOP_CHK, cfg.always_on_top);
            set_check(hwnd, IDC_RUNINBACKGROUND_CHK, cfg.run_in_background);
            set_check(hwnd, IDC_LOADONSTARTUP_CHK, is_autorun_enabled());
            set_check(hwnd, IDC_STARTMINIMIZED_CHK, cfg.start_minimized);
            set_check(
                hwnd,
                IDC_REDUCTCONFIRMATION_CHK,
                cfg.show_reduct_confirmation,
            );
            enable_ctrl(hwnd, IDC_SKIPUACWARNING_CHK, elevated);
            set_check(hwnd, IDC_SKIPUACWARNING_CHK, is_skip_uac_enabled());
            set_check(hwnd, IDC_CHECKUPDATES_CHK, cfg.check_updates);

            populate_language_combo(hwnd);
        }

        IDD_SETTINGS_MEMORY => {
            lv_set_ex_style(
                hwnd,
                IDC_REGIONS,
                LVS_EX_CHECKBOXES
                    | LVS_EX_FULLROWSELECT
                    | LVS_EX_INFOTIP
                    | LVS_EX_LABELTIP
                    | LVS_EX_DOUBLEBUFFER,
            );
            lv_reset_columns_and_items(hwnd, IDC_REGIONS);
            lv_add_column(hwnd, IDC_REGIONS, 0, 220, LVCFMT_LEFT);

            populate_regions_listview(hwnd, cfg.reduct_mask);
            lv_resize_single_column_full(hwnd, IDC_REGIONS);
            apply_listview_theme(hwnd, IDC_REGIONS, cfg.use_dark_theme);

            set_check(hwnd, IDC_AUTOREDUCTENABLE_CHK, cfg.autoreduct_enable);
            updown_init(
                hwnd,
                IDC_AUTOREDUCTVALUE,
                IDC_AUTOREDUCTVALUE_CTRL,
                1,
                99,
                cfg.autoreduct_value as i32,
            );

            set_check(
                hwnd,
                IDC_AUTOREDUCTINTERVALENABLE_CHK,
                cfg.autoreduct_interval_enable,
            );
            updown_init(
                hwnd,
                IDC_AUTOREDUCTINTERVALVALUE,
                IDC_AUTOREDUCTINTERVALVALUE_CTRL,
                1,
                1440,
                cfg.autoreduct_interval_value as i32,
            );

            set_check(hwnd, IDC_HOTKEY_CLEAN_CHK, cfg.hotkey_clean_enable);
            unsafe {
                SendDlgItemMessageW(
                    hwnd,
                    IDC_HOTKEY_CLEAN,
                    HKM_SETHOTKEY,
                    cfg.hotkey_clean as usize,
                    0,
                );
            }

            // Always allow configuring Memory Cleaning options; buddy controls follow checkbox state
            enable_ctrl(hwnd, IDC_REGIONS, true);
            enable_ctrl(hwnd, IDC_AUTOREDUCTENABLE_CHK, true);
            enable_ctrl(hwnd, IDC_AUTOREDUCTINTERVALENABLE_CHK, true);
            enable_ctrl(hwnd, IDC_HOTKEY_CLEAN_CHK, true);

            updown_enable_with_buddy(hwnd, IDC_AUTOREDUCTVALUE, cfg.autoreduct_enable);
            updown_enable_with_buddy(
                hwnd,
                IDC_AUTOREDUCTINTERVALVALUE,
                cfg.autoreduct_interval_enable,
            );
            enable_ctrl(hwnd, IDC_HOTKEY_CLEAN, cfg.hotkey_clean_enable);
        }

        IDD_SETTINGS_APPEARANCE => {
            set_check(hwnd, IDC_TRAYUSETRANSPARENCY_CHK, cfg.tray_use_transparency);
            set_check(hwnd, IDC_TRAYSHOWBORDER_CHK, cfg.tray_show_border);
            set_check(hwnd, IDC_TRAYROUNDCORNERS_CHK, cfg.tray_round_corners);
            set_check(hwnd, IDC_TRAYCHANGEBG_CHK, cfg.tray_change_bg);
            set_check(hwnd, IDC_TRAYUSEANTIALIASING_CHK, cfg.tray_use_antialiasing);

            set_font_button_label(hwnd, IDC_FONT, &cfg);

            lv_set_ex_style(
                hwnd,
                IDC_COLORS,
                LVS_EX_DOUBLEBUFFER | LVS_EX_FULLROWSELECT | LVS_EX_INFOTIP | LVS_EX_LABELTIP,
            );
            lv_reset_columns_and_items(hwnd, IDC_COLORS);
            lv_add_column(hwnd, IDC_COLORS, 0, 220, LVCFMT_LEFT);

            with_state(|st| {
                lv_insert_item(
                    hwnd,
                    IDC_COLORS,
                    0,
                    &st.locale.get(IDS_COLOR_TEXT_HINT),
                    None,
                    cfg.tray_color_text as isize,
                );
                lv_insert_item(
                    hwnd,
                    IDC_COLORS,
                    1,
                    &st.locale.get(IDS_COLOR_BACKGROUND_HINT),
                    None,
                    cfg.tray_color_bg as isize,
                );
                lv_insert_item(
                    hwnd,
                    IDC_COLORS,
                    2,
                    &st.locale.get(IDS_COLOR_WARNING_HINT),
                    None,
                    cfg.tray_color_warning as isize,
                );
                lv_insert_item(
                    hwnd,
                    IDC_COLORS,
                    3,
                    &st.locale.get(IDS_COLOR_DANGER_HINT),
                    None,
                    cfg.tray_color_danger as isize,
                );
            });
            lv_resize_single_column_full(hwnd, IDC_COLORS);
            refresh_appearance_preview(hwnd);
        }

        IDD_SETTINGS_TRAY => {
            updown_init(
                hwnd,
                IDC_TRAYLEVELWARNING,
                IDC_TRAYLEVELWARNING_CTRL,
                1,
                98,
                cfg.tray_level_warning as i32,
            );
            updown_init(
                hwnd,
                IDC_TRAYLEVELDANGER,
                IDC_TRAYLEVELDANGER_CTRL,
                2,
                99,
                cfg.tray_level_danger as i32,
            );

            set_check(hwnd, IDC_SHOW_CLEAN_RESULT_CHK, cfg.balloon_clean_results);
            set_check(hwnd, IDC_NOTIFICATIONSOUND_CHK, cfg.notifications_sound);
        }

        IDD_SETTINGS_ADVANCED => {
            set_check(
                hwnd,
                IDC_ALLOWSTANDBYLISTCLEANUP_CHK,
                cfg.allow_standby_cleanup,
            );
            set_check(hwnd, IDC_LOGRESULTS_CHK, cfg.log_clean_results);
            set_check(
                hwnd,
                IDC_SKIPFULLSCREEN_CHK,
                cfg.skip_fullscreen_autoclean,
            );
            enable_ctrl(hwnd, IDC_WINSERVICE_CHK, elevated);
            set_check(hwnd, IDC_WINSERVICE_CHK, is_service_installed());
            set_dlg_text(hwnd, IDC_WHITELIST_EDIT, &cfg.process_whitelist);
        }

        _ => {}
    }
}

fn populate_language_combo(hwnd: HWND) {
    unsafe {
        SendDlgItemMessageW(hwnd, IDC_LANGUAGE, CB_RESETCONTENT, 0, 0);
        SendDlgItemMessageW(
            hwnd,
            IDC_LANGUAGE,
            CB_ADDSTRING,
            0,
            to_wide("English (default)").as_ptr() as isize,
        );
        let (langs, cur) =
            with_state(|st| (st.locale.languages.clone(), st.locale.current_lang.clone()));
        let mut sel_idx = 0usize;
        for (i, lang) in langs.iter().enumerate() {
            SendDlgItemMessageW(
                hwnd,
                IDC_LANGUAGE,
                CB_ADDSTRING,
                0,
                to_wide(lang).as_ptr() as isize,
            );
            if *lang == cur {
                sel_idx = i + 1;
            }
        }
        SendDlgItemMessageW(hwnd, IDC_LANGUAGE, CB_SETCURSEL, sel_idx, 0);
    }
}

fn populate_regions_listview(hwnd: HWND, mask: u32) {
    with_state(|st| {
        let items: [(String, u32); 8] = [
            (st.locale.get(IDS_MEMREGION_WORKINGSET), REDUCT_WORKINGSET),
            (
                st.locale.get(IDS_MEMREGION_SYSTEMFILECACHE),
                REDUCT_SYSTEMFILECACHE,
            ),
            (
                format!("{}*", st.locale.get(IDS_MEMREGION_MODIFIEDLIST)),
                REDUCT_MODIFIEDLIST,
            ),
            (
                format!("{}*", st.locale.get(IDS_MEMREGION_STANDBYLIST)),
                REDUCT_STANDBYLIST,
            ),
            (
                st.locale.get(IDS_MEMREGION_STANDBYLISTPRIORITY0),
                REDUCT_STANDBYPRIORITY0LIST,
            ),
            (
                st.locale.get(IDS_MEMREGION_MODIFIEDFILECACHE),
                REDUCT_MODIFIEDFILECACHE,
            ),
            (
                format!("{} (win8.1+)", st.locale.get(IDS_MEMREGION_REGISTRYCACHE)),
                REDUCT_REGISTRYCACHE,
            ),
            (
                format!(
                    "{} (win10+)",
                    st.locale.get(IDS_MEMREGION_COMBINEMEMORYLISTS)
                ),
                REDUCT_COMBINEMEMORYLISTS,
            ),
        ];

        for (idx, (label, bit)) in items.iter().enumerate() {
            let os_supported = (is_win8_1_or_greater() || *bit != REDUCT_REGISTRYCACHE)
                && (is_win10_or_greater() || *bit != REDUCT_COMBINEMEMORYLISTS);
            lv_insert_item(hwnd, IDC_REGIONS, idx as i32, label, None, *bit as isize);
            lv_set_check(
                hwnd,
                IDC_REGIONS,
                idx as i32,
                os_supported && (mask & *bit) != 0,
            );
        }
    });
}

fn localize_settings_page_inner(hwnd: HWND, page_id: u16) {
    with_state(|st| {
        let loc = &st.locale;
        match page_id {
            IDD_SETTINGS_GENERAL => {
                set_dlg_text(hwnd, IDC_TITLE_1, &format!("{}:", loc.get(IDS_TITLE_1)));
                set_dlg_text(
                    hwnd,
                    IDC_TITLE_2,
                    &format!("{}: (Language)", loc.get(IDS_TITLE_2)),
                );
                set_dlg_text(hwnd, IDC_ALWAYSONTOP_CHK, &loc.get(IDS_ALWAYSONTOP_CHK));
                set_dlg_text(
                    hwnd,
                    IDC_RUNINBACKGROUND_CHK,
                    &loc.get(IDS_RUNINBACKGROUND_CHK),
                );
                set_dlg_text(hwnd, IDC_LOADONSTARTUP_CHK, &loc.get(IDS_LOADONSTARTUP_CHK));
                set_dlg_text(
                    hwnd,
                    IDC_STARTMINIMIZED_CHK,
                    &loc.get(IDS_STARTMINIMIZED_CHK),
                );
                set_dlg_text(
                    hwnd,
                    IDC_REDUCTCONFIRMATION_CHK,
                    &loc.get(IDS_REDUCTCONFIRMATION_CHK),
                );
                set_dlg_text(
                    hwnd,
                    IDC_SKIPUACWARNING_CHK,
                    &loc.get(IDS_SKIPUACWARNING_CHK),
                );
                set_dlg_text(hwnd, IDC_CHECKUPDATES_CHK, &loc.get(IDS_CHECKUPDATES_CHK));
                set_dlg_text(hwnd, IDC_LANGUAGE_HINT, &loc.get(IDS_LANGUAGE_HINT));
            }

            IDD_SETTINGS_MEMORY => {
                set_dlg_text(hwnd, IDC_TITLE_3, &format!("{}:", loc.get(IDS_TITLE_3)));
                set_dlg_text(hwnd, IDC_TITLE_4, &format!("{}:", loc.get(IDS_TITLE_4)));
                set_dlg_text(hwnd, IDC_TITLE_5, &format!("{}:", loc.get(IDS_TITLE_5)));
                set_dlg_text(
                    hwnd,
                    IDC_AUTOREDUCTENABLE_CHK,
                    &loc.get(IDS_AUTOREDUCTENABLE_CHK),
                );
                set_dlg_text(
                    hwnd,
                    IDC_AUTOREDUCTINTERVALENABLE_CHK,
                    &loc.get(IDS_AUTOREDUCTINTERVALENABLE_CHK),
                );
                set_dlg_text(hwnd, IDC_HOTKEY_CLEAN_CHK, &loc.get(IDS_HOTKEY_CLEAN_CHK));

                lv_set_item_text(hwnd, IDC_REGIONS, 0, 0, &loc.get(IDS_MEMREGION_WORKINGSET));
                lv_set_item_text(
                    hwnd,
                    IDC_REGIONS,
                    1,
                    0,
                    &loc.get(IDS_MEMREGION_SYSTEMFILECACHE),
                );
                lv_set_item_text(
                    hwnd,
                    IDC_REGIONS,
                    2,
                    0,
                    &format!("{}*", loc.get(IDS_MEMREGION_MODIFIEDLIST)),
                );
                lv_set_item_text(
                    hwnd,
                    IDC_REGIONS,
                    3,
                    0,
                    &format!("{}*", loc.get(IDS_MEMREGION_STANDBYLIST)),
                );
                lv_set_item_text(
                    hwnd,
                    IDC_REGIONS,
                    4,
                    0,
                    &loc.get(IDS_MEMREGION_STANDBYLISTPRIORITY0),
                );
                lv_set_item_text(
                    hwnd,
                    IDC_REGIONS,
                    5,
                    0,
                    &loc.get(IDS_MEMREGION_MODIFIEDFILECACHE),
                );
                lv_set_item_text(
                    hwnd,
                    IDC_REGIONS,
                    6,
                    0,
                    &format!("{} (win8.1+)", loc.get(IDS_MEMREGION_REGISTRYCACHE)),
                );
                lv_set_item_text(
                    hwnd,
                    IDC_REGIONS,
                    7,
                    0,
                    &format!("{} (win10+)", loc.get(IDS_MEMREGION_COMBINEMEMORYLISTS)),
                );
            }

            IDD_SETTINGS_APPEARANCE => {
                set_dlg_text(hwnd, IDC_TITLE_1, &format!("{}:", loc.get(IDS_TITLE_1)));
                set_dlg_text(hwnd, IDC_TITLE_6, &format!("{}:", loc.get(IDS_TITLE_6)));
                set_dlg_text(
                    hwnd,
                    IDC_TRAYUSETRANSPARENCY_CHK,
                    &loc.get(IDS_TRAYUSETRANSPARENCY_CHK),
                );
                set_dlg_text(
                    hwnd,
                    IDC_TRAYSHOWBORDER_CHK,
                    &loc.get(IDS_TRAYSHOWBORDER_CHK),
                );
                set_dlg_text(
                    hwnd,
                    IDC_TRAYROUNDCORNERS_CHK,
                    &loc.get(IDS_TRAYROUNDCORNERS_CHK),
                );
                set_dlg_text(
                    hwnd,
                    IDC_TRAYCHANGEBG_CHK,
                    &loc.get(IDS_TRAYCHANGEBG_CHK),
                );
                set_dlg_text(
                    hwnd,
                    IDC_TRAYUSEANTIALIASING_CHK,
                    &loc.get(IDS_TRAYUSEANTIALIASING_CHK),
                );
                set_dlg_text(hwnd, IDC_TRAY_PREVIEW_HINT, &loc.get(IDS_PREVIEW_HINT));
                set_dlg_text(hwnd, IDC_FONT_HINT, &loc.get(IDS_FONT_HINT));

                lv_set_item_text(hwnd, IDC_COLORS, 0, 0, &loc.get(IDS_COLOR_TEXT_HINT));
                lv_set_item_text(hwnd, IDC_COLORS, 1, 0, &loc.get(IDS_COLOR_BACKGROUND_HINT));
                lv_set_item_text(hwnd, IDC_COLORS, 2, 0, &loc.get(IDS_COLOR_WARNING_HINT));
                lv_set_item_text(hwnd, IDC_COLORS, 3, 0, &loc.get(IDS_COLOR_DANGER_HINT));
            }

            IDD_SETTINGS_TRAY => {
                set_dlg_text(hwnd, IDC_TITLE_7, &format!("{}:", loc.get(IDS_TITLE_7)));
                set_dlg_text(hwnd, IDC_TITLE_8, &format!("{}:", loc.get(IDS_TITLE_8)));
                set_dlg_text(hwnd, IDC_TITLE_9, &format!("{}:", loc.get(IDS_TITLE_9)));
                set_dlg_text(
                    hwnd,
                    IDC_TRAYLEVELWARNING_HINT,
                    &loc.get(IDS_TRAYLEVELWARNING_HINT),
                );
                set_dlg_text(
                    hwnd,
                    IDC_TRAYLEVELDANGER_HINT,
                    &loc.get(IDS_TRAYLEVELDANGER_HINT),
                );
                set_dlg_text(hwnd, IDC_TRAYACTIONSC_HINT, &loc.get(IDS_TRAYACTIONSC_HINT));
                set_dlg_text(hwnd, IDC_TRAYACTIONMC_HINT, &loc.get(IDS_TRAYACTIONMC_HINT));

                unsafe {
                    SendDlgItemMessageW(hwnd, IDC_TRAYACTIONSC, CB_RESETCONTENT, 0, 0);
                    SendDlgItemMessageW(hwnd, IDC_TRAYACTIONMC, CB_RESETCONTENT, 0, 0);
                    for action_str_id in [IDS_TRAY_ACTION_1, IDS_TRAY_ACTION_2, IDS_TRAY_ACTION_3]
                    {
                        let s = loc.get(action_str_id);
                        let w = to_wide(&s);
                        SendDlgItemMessageW(
                            hwnd,
                            IDC_TRAYACTIONSC,
                            CB_ADDSTRING,
                            0,
                            w.as_ptr() as isize,
                        );
                        SendDlgItemMessageW(
                            hwnd,
                            IDC_TRAYACTIONMC,
                            CB_ADDSTRING,
                            0,
                            w.as_ptr() as isize,
                        );
                    }
                    SendDlgItemMessageW(
                        hwnd,
                        IDC_TRAYACTIONSC,
                        CB_SETCURSEL,
                        st.config.tray_action_sc as usize,
                        0,
                    );
                    SendDlgItemMessageW(
                        hwnd,
                        IDC_TRAYACTIONMC,
                        CB_SETCURSEL,
                        st.config.tray_action_mc as usize,
                        0,
                    );
                }

                set_dlg_text(
                    hwnd,
                    IDC_SHOW_CLEAN_RESULT_CHK,
                    &loc.get(IDS_SHOW_CLEAN_RESULT_CHK),
                );
                set_dlg_text(
                    hwnd,
                    IDC_NOTIFICATIONSOUND_CHK,
                    &loc.get(IDS_NOTIFICATIONSOUND_CHK),
                );
            }

            IDD_SETTINGS_ADVANCED => {
                set_dlg_text(hwnd, IDC_TITLE_1, &format!("{}:", loc.get(IDS_TITLE_1)));
                set_dlg_text(
                    hwnd,
                    IDC_ALLOWSTANDBYLISTCLEANUP_CHK,
                    &loc.get(IDS_ALLOWSTANDBYLISTCLEANUP_CHK),
                );
                set_dlg_text(hwnd, IDC_LOGRESULTS_CHK, &loc.get(IDS_LOGRESULTS_CHK));
                set_dlg_text(
                    hwnd,
                    IDC_SKIPFULLSCREEN_CHK,
                    &loc.get(IDS_SKIPFULLSCREEN_CHK),
                );
                set_dlg_text(hwnd, IDC_WINSERVICE_CHK, &loc.get(IDS_WINSERVICE_CHK));
                set_dlg_text(
                    hwnd,
                    IDC_TITLE_WHITELIST,
                    &format!("{}:", loc.get(IDS_TITLE_WHITELIST)),
                );
                set_dlg_text(hwnd, IDC_WHITELIST_HINT, &loc.get(IDS_WHITELIST_HINT));
            }

            _ => {}
        }
    });
}

fn refresh_appearance_preview(appearance_hwnd: HWND) {
    if appearance_hwnd == 0 {
        return;
    }
    let icons = with_state_mut(|st| {
        let cfg = st.config.clone();
        st.tray.render_preview_trio(&cfg)
    });
    unsafe {
        SendDlgItemMessageW(
            appearance_hwnd,
            IDC_TRAY_PREVIEW_NORMAL,
            STM_SETICON,
            icons[0] as usize,
            0,
        );
        SendDlgItemMessageW(
            appearance_hwnd,
            IDC_TRAY_PREVIEW_WARN,
            STM_SETICON,
            icons[1] as usize,
            0,
        );
        SendDlgItemMessageW(
            appearance_hwnd,
            IDC_TRAY_PREVIEW_DANGER,
            STM_SETICON,
            icons[2] as usize,
            0,
        );
    }
}

unsafe extern "system" fn settings_page_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    let suppressed = SETTINGS_UI.with(|c| c.borrow().suppress_events);

    match msg {
        WM_INITDIALOG => 1,

        WM_NOTIFY => {
            let nmhdr = &*(lparam as *const NMHDR);
            match nmhdr.code {
                NM_CUSTOMDRAW => {
                    let lvcd = &mut *(lparam as *mut NMLVCUSTOMDRAW);
                    let id_from = lvcd.nmcd.hdr.idFrom as i32;
                    if id_from != IDC_REGIONS && id_from != IDC_COLORS {
                        return 0;
                    }

                    if lvcd.nmcd.dwDrawStage == CDDS_PREPAINT {
                        SetWindowLongPtrW(hwnd, DWLP_MSGRESULT, CDRF_NOTIFYITEMDRAW);
                        return CDRF_NOTIFYITEMDRAW;
                    }
                    if lvcd.nmcd.dwDrawStage == CDDS_ITEMPREPAINT {
                        let mut ret = CDRF_DODEFAULT;
                        if id_from == IDC_REGIONS {
                            let idx = lvcd.nmcd.dwItemSpec;
                            let bit = REGION_BITS_BY_INDEX.get(idx).copied().unwrap_or(0);
                            if (!is_win8_1_or_greater() && bit == REDUCT_REGISTRYCACHE)
                                || (!is_win10_or_greater() && bit == REDUCT_COMBINEMEMORYLISTS)
                            {
                                lvcd.clrText = rgb(0x88, 0x88, 0x88);
                                ret = CDRF_NEWFONT;
                            }
                        } else if id_from == IDC_COLORS {
                            let clr = lvcd.nmcd.lItemlParam as COLORREF;
                            lvcd.clrText = color_brightness_text(clr);
                            lvcd.clrTextBk = clr;
                            let brush = CreateSolidBrush(clr);
                            FillRect(lvcd.nmcd.hdc, &lvcd.nmcd.rc, brush);
                            DeleteObject(brush);
                            ret = CDRF_NEWFONT;
                        }
                        SetWindowLongPtrW(hwnd, DWLP_MSGRESULT, ret);
                        return ret;
                    }
                    0
                }

                NM_DBLCLK => {
                    let nmia = &*(lparam as *const NMITEMACTIVATE);
                    if nmia.hdr.idFrom as i32 != IDC_COLORS || nmia.iItem < 0 {
                        return 0;
                    }
                    let current_clr = lv_get_item_lparam(hwnd, IDC_COLORS, nmia.iItem) as COLORREF;
                    let mut custom_colors: [COLORREF; 16] = [
                        TRAY_COLOR_DANGER,
                        TRAY_COLOR_WARNING,
                        TRAY_COLOR_BG,
                        TRAY_COLOR_TEXT,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                    ];
                    let mut cc = CHOOSECOLORW {
                        lStructSize: std::mem::size_of::<CHOOSECOLORW>() as u32,
                        hwndOwner: hwnd,
                        hInstance: 0,
                        rgbResult: current_clr,
                        lpCustColors: custom_colors.as_mut_ptr(),
                        Flags: CC_RGBINIT | CC_FULLOPEN,
                        lCustData: 0,
                        lpfnHook: 0,
                        lpTemplateName: std::ptr::null(),
                    };
                    if ChooseColorW(&mut cc) != FALSE {
                        with_state_mut(|st| {
                            match nmia.iItem {
                                0 => st.config.tray_color_text = cc.rgbResult,
                                1 => st.config.tray_color_bg = cc.rgbResult,
                                2 => st.config.tray_color_warning = cc.rgbResult,
                                3 => st.config.tray_color_danger = cc.rgbResult,
                                _ => {}
                            }
                            st.config.save();
                        });
                        lv_set_item_lparam(hwnd, IDC_COLORS, nmia.iItem, cc.rgbResult as isize);
                        InvalidateRect(GetDlgItem(hwnd, IDC_COLORS), std::ptr::null(), TRUE);
                        refresh_all_ui_after_settings();
                        refresh_appearance_preview(hwnd);
                    }
                    1
                }

                LVN_ITEMCHANGED => {
                    if suppressed {
                        return 0;
                    }
                    let nmlv = &*(lparam as *const NMLISTVIEW);
                    if nmlv.hdr.idFrom as i32 != IDC_REGIONS || (nmlv.uChanged & LVIF_STATE) == 0 {
                        return 0;
                    }
                    if nmlv.iItem < 0 || (nmlv.iItem as usize) >= REGION_BITS_BY_INDEX.len() {
                        return 0;
                    }

                    let new_chk = nmlv.uNewState & LVIS_STATEIMAGEMASK;
                    let old_chk = nmlv.uOldState & LVIS_STATEIMAGEMASK;
                    if new_chk == old_chk {
                        return 0;
                    }
                    let is_checking = new_chk == indextostateimagemask(2);
                    let is_unchecking = new_chk == indextostateimagemask(1);
                    if !is_checking && !is_unchecking {
                        return 0;
                    }

                    // CRITICAL FIX: Look up bit by item index (Windows leaves nmlv.lParam == 0 on LVIF_STATE!)
                    let bit = REGION_BITS_BY_INDEX[nmlv.iItem as usize];

                    if (!is_win8_1_or_greater() && bit == REDUCT_REGISTRYCACHE)
                        || (!is_win10_or_greater() && bit == REDUCT_COMBINEMEMORYLISTS)
                    {
                        set_suppress(true);
                        lv_set_check(hwnd, IDC_REGIONS, nmlv.iItem, false);
                        set_suppress(false);
                        return 0;
                    }

                    if is_checking && (bit & REDUCT_MASK_FREEZES) != 0 {
                        let need_warn = with_state(|st| st.config.show_warning_confirmation);
                        if need_warn {
                            let (warn_txt, dont_ask_txt) = with_state(|st| {
                                (
                                    st.locale.get(IDS_QUESTION_WARNING),
                                    st.locale.get(IDS_QUESTION_FLAG_CHK),
                                )
                            });
                            let (ok, dont_ask) = show_confirm_dialog(hwnd, &warn_txt, &dont_ask_txt);
                            if !ok {
                                set_suppress(true);
                                lv_set_check(hwnd, IDC_REGIONS, nmlv.iItem, false);
                                set_suppress(false);
                                return 0;
                            }
                            if dont_ask {
                                with_state_mut(|st| {
                                    st.config.show_warning_confirmation = false;
                                    st.config.save();
                                });
                            }
                        }
                    }

                    let full_mask = read_regions_mask_from_ui(hwnd);
                    with_state_mut(|st| {
                        st.config.reduct_mask = full_mask;
                        st.config.save();
                    });
                    1
                }

                _ => 0,
            }
        }

        WM_HSCROLL | WM_VSCROLL => {
            if suppressed {
                return 0;
            }
            let ctrl_id = GetDlgCtrlID(lparam as HWND);
            handle_spin_change(hwnd, ctrl_id, true);
            0
        }

        WM_COMMAND => {
            if suppressed {
                return 0;
            }
            let ctrl_id = loword(wparam) as i32;
            let code = hiword(wparam);
            handle_page_command(hwnd, ctrl_id, code);
            0
        }

        _ => 0,
    }
}

fn handle_spin_change(hwnd: HWND, ctrl_id: i32, from_scroll: bool) {
    match ctrl_id {
        IDC_AUTOREDUCTVALUE | IDC_AUTOREDUCTVALUE_CTRL => {
            if let Some(raw) = updown_get_opt(hwnd, IDC_AUTOREDUCTVALUE, IDC_AUTOREDUCTVALUE_CTRL) {
                let v = raw.clamp(1, 99) as u32;
                with_state_mut(|st| {
                    st.config.autoreduct_value = v;
                    st.config.save();
                });
                if from_scroll {
                    set_suppress(true);
                    updown_set(hwnd, IDC_AUTOREDUCTVALUE, IDC_AUTOREDUCTVALUE_CTRL, v as i32);
                    set_suppress(false);
                }
            }
        }
        IDC_AUTOREDUCTINTERVALVALUE | IDC_AUTOREDUCTINTERVALVALUE_CTRL => {
            if let Some(raw) = updown_get_opt(
                hwnd,
                IDC_AUTOREDUCTINTERVALVALUE,
                IDC_AUTOREDUCTINTERVALVALUE_CTRL,
            ) {
                let v = raw.clamp(1, 1440) as u32;
                with_state_mut(|st| {
                    st.config.autoreduct_interval_value = v;
                    st.config.save();
                });
                if from_scroll {
                    set_suppress(true);
                    updown_set(
                        hwnd,
                        IDC_AUTOREDUCTINTERVALVALUE,
                        IDC_AUTOREDUCTINTERVALVALUE_CTRL,
                        v as i32,
                    );
                    set_suppress(false);
                }
            }
        }
        IDC_TRAYLEVELWARNING | IDC_TRAYLEVELWARNING_CTRL => {
            if let Some(raw) = updown_get_opt(hwnd, IDC_TRAYLEVELWARNING, IDC_TRAYLEVELWARNING_CTRL)
            {
                let v = raw.clamp(1, 98) as u32;
                let (new_w, new_d) = with_state_mut(|st| {
                    st.config.tray_level_warning = v;
                    st.config.normalize_warning_danger(true);
                    st.config.save();
                    (st.config.tray_level_warning, st.config.tray_level_danger)
                });
                set_suppress(true);
                updown_set(
                    hwnd,
                    IDC_TRAYLEVELDANGER,
                    IDC_TRAYLEVELDANGER_CTRL,
                    new_d as i32,
                );
                if from_scroll || new_w != raw as u32 {
                    updown_set(
                        hwnd,
                        IDC_TRAYLEVELWARNING,
                        IDC_TRAYLEVELWARNING_CTRL,
                        new_w as i32,
                    );
                }
                set_suppress(false);
                refresh_all_ui_after_settings();
            }
        }
        IDC_TRAYLEVELDANGER | IDC_TRAYLEVELDANGER_CTRL => {
            if let Some(raw) = updown_get_opt(hwnd, IDC_TRAYLEVELDANGER, IDC_TRAYLEVELDANGER_CTRL) {
                let v = raw.clamp(2, 99) as u32;
                let (new_w, new_d) = with_state_mut(|st| {
                    st.config.tray_level_danger = v;
                    st.config.normalize_warning_danger(false);
                    st.config.save();
                    (st.config.tray_level_warning, st.config.tray_level_danger)
                });
                set_suppress(true);
                updown_set(
                    hwnd,
                    IDC_TRAYLEVELWARNING,
                    IDC_TRAYLEVELWARNING_CTRL,
                    new_w as i32,
                );
                if from_scroll || new_d != raw as u32 {
                    updown_set(
                        hwnd,
                        IDC_TRAYLEVELDANGER,
                        IDC_TRAYLEVELDANGER_CTRL,
                        new_d as i32,
                    );
                }
                set_suppress(false);
                refresh_all_ui_after_settings();
            }
        }
        _ => {}
    }
}

fn handle_page_command(hwnd: HWND, ctrl_id: i32, code: u16) {
    match ctrl_id {
        IDC_AUTOREDUCTVALUE_CTRL
        | IDC_AUTOREDUCTINTERVALVALUE_CTRL
        | IDC_TRAYLEVELWARNING_CTRL
        | IDC_TRAYLEVELDANGER_CTRL => {
            if code == EN_CHANGE {
                handle_spin_change(hwnd, ctrl_id, false);
            }
        }

        IDC_ALWAYSONTOP_CHK => {
            let checked = is_checked(hwnd, ctrl_id);
            let main_hwnd = with_state_mut(|st| {
                st.config.always_on_top = checked;
                st.config.save();
                st.main_hwnd
            });
            set_window_topmost(main_hwnd, checked);
            unsafe {
                menu_check(GetMenu(main_hwnd), IDM_ALWAYSONTOP_CHK, checked);
            }
        }

        IDC_RUNINBACKGROUND_CHK => {
            let checked = is_checked(hwnd, ctrl_id);
            let main_hwnd = with_state_mut(|st| {
                st.config.run_in_background = checked;
                st.config.save();
                st.main_hwnd
            });
            unsafe {
                menu_check(GetMenu(main_hwnd), IDM_RUNINBACKGROUND_CHK, checked);
            }
        }

        IDC_LOADONSTARTUP_CHK => {
            let want = is_checked(hwnd, ctrl_id);
            let _ = set_autorun_enabled(want);
            let actual = is_autorun_enabled();
            set_check(hwnd, ctrl_id, actual);
            let main_hwnd = with_state(|st| st.main_hwnd);
            unsafe {
                menu_check(GetMenu(main_hwnd), IDM_LOADONSTARTUP_CHK, actual);
            }
        }

        IDC_STARTMINIMIZED_CHK => {
            let checked = is_checked(hwnd, ctrl_id);
            let main_hwnd = with_state_mut(|st| {
                st.config.start_minimized = checked;
                st.config.save();
                st.main_hwnd
            });
            unsafe {
                menu_check(GetMenu(main_hwnd), IDM_STARTMINIMIZED_CHK, checked);
            }
        }

        IDC_REDUCTCONFIRMATION_CHK => {
            let checked = is_checked(hwnd, ctrl_id);
            let main_hwnd = with_state_mut(|st| {
                st.config.show_reduct_confirmation = checked;
                st.config.save();
                st.main_hwnd
            });
            unsafe {
                menu_check(GetMenu(main_hwnd), IDM_REDUCTCONFIRMATION_CHK, checked);
            }
        }

        IDC_SKIPUACWARNING_CHK => {
            let want = is_checked(hwnd, ctrl_id);
            let _ = set_skip_uac_enabled(want);
            let actual = is_skip_uac_enabled();
            set_check(hwnd, ctrl_id, actual);
            let main_hwnd = with_state(|st| st.main_hwnd);
            unsafe {
                menu_check(GetMenu(main_hwnd), IDM_SKIPUACWARNING_CHK, actual);
            }
        }

        IDC_CHECKUPDATES_CHK => {
            let checked = is_checked(hwnd, ctrl_id);
            let main_hwnd = with_state_mut(|st| {
                st.config.check_updates = checked;
                st.config.save();
                st.main_hwnd
            });
            unsafe {
                menu_check(GetMenu(main_hwnd), IDM_CHECKUPDATES_CHK, checked);
            }
        }

        IDC_LANGUAGE => {
            if code == CBN_SELCHANGE {
                let sel =
                    unsafe { SendDlgItemMessageW(hwnd, IDC_LANGUAGE, CB_GETCURSEL, 0, 0) } as usize;
                with_state_mut(|st| {
                    if sel == 0 {
                        st.config.language.clear();
                        st.locale.set_language("");
                    } else if let Some(lang) = st.locale.languages.get(sel - 1).cloned() {
                        st.config.language = lang.clone();
                        st.locale.set_language(&lang);
                    }
                    st.config.save();
                });
                let container = with_state(|st| st.settings_hwnd);
                localize_settings_container_and_pages(container);
                refresh_all_ui_after_settings();
            }
        }

        IDC_AUTOREDUCTENABLE_CHK => {
            let checked = is_checked(hwnd, ctrl_id);
            updown_enable_with_buddy(hwnd, IDC_AUTOREDUCTVALUE, checked);
            with_state_mut(|st| {
                st.config.autoreduct_enable = checked;
                if let Some(v) =
                    updown_get_opt(hwnd, IDC_AUTOREDUCTVALUE, IDC_AUTOREDUCTVALUE_CTRL)
                {
                    st.config.autoreduct_value = v.clamp(1, 99) as u32;
                }
                st.config.save();
            });
        }

        IDC_AUTOREDUCTINTERVALENABLE_CHK => {
            let checked = is_checked(hwnd, ctrl_id);
            updown_enable_with_buddy(hwnd, IDC_AUTOREDUCTINTERVALVALUE, checked);
            with_state_mut(|st| {
                st.config.autoreduct_interval_enable = checked;
                if let Some(v) = updown_get_opt(
                    hwnd,
                    IDC_AUTOREDUCTINTERVALVALUE,
                    IDC_AUTOREDUCTINTERVALVALUE_CTRL,
                ) {
                    st.config.autoreduct_interval_value = v.clamp(1, 1440) as u32;
                }
                st.last_auto_interval_ts = unix_now();
                st.config.save();
            });
        }

        IDC_HOTKEY_CLEAN_CHK => {
            let checked = is_checked(hwnd, ctrl_id);
            enable_ctrl(hwnd, IDC_HOTKEY_CLEAN, checked);
            let hk =
                unsafe { SendDlgItemMessageW(hwnd, IDC_HOTKEY_CLEAN, HKM_GETHOTKEY, 0, 0) } as u32;
            let (main_hwnd, cfg) = with_state_mut(|st| {
                st.config.hotkey_clean_enable = checked;
                if hk != 0 {
                    st.config.hotkey_clean = hk;
                }
                st.config.save();
                (st.main_hwnd, st.config.clone())
            });
            register_clean_hotkey(main_hwnd, &cfg);
        }

        IDC_HOTKEY_CLEAN => {
            if code == EN_CHANGE {
                let hk = unsafe {
                    SendDlgItemMessageW(hwnd, IDC_HOTKEY_CLEAN, HKM_GETHOTKEY, 0, 0)
                } as u32;
                let (main_hwnd, cfg, enabled) = with_state_mut(|st| {
                    if hk != 0 {
                        st.config.hotkey_clean = hk;
                        st.config.save();
                    }
                    (
                        st.main_hwnd,
                        st.config.clone(),
                        st.config.hotkey_clean_enable,
                    )
                });
                if enabled && hk != 0 {
                    register_clean_hotkey(main_hwnd, &cfg);
                }
            }
        }

        IDC_TRAYUSETRANSPARENCY_CHK
        | IDC_TRAYSHOWBORDER_CHK
        | IDC_TRAYROUNDCORNERS_CHK
        | IDC_TRAYCHANGEBG_CHK
        | IDC_TRAYUSEANTIALIASING_CHK => {
            let checked = is_checked(hwnd, ctrl_id);
            with_state_mut(|st| {
                match ctrl_id {
                    IDC_TRAYUSETRANSPARENCY_CHK => st.config.tray_use_transparency = checked,
                    IDC_TRAYSHOWBORDER_CHK => st.config.tray_show_border = checked,
                    IDC_TRAYROUNDCORNERS_CHK => st.config.tray_round_corners = checked,
                    IDC_TRAYCHANGEBG_CHK => st.config.tray_change_bg = checked,
                    IDC_TRAYUSEANTIALIASING_CHK => st.config.tray_use_antialiasing = checked,
                    _ => {}
                }
                st.config.save();
            });
            refresh_all_ui_after_settings();
            refresh_appearance_preview(hwnd);
        }

        IDC_FONT => {
            let dpi = get_window_dpi(hwnd);
            let mut lf = with_state(|st| {
                let mut f = LOGFONTW::default();
                copy_wide_fixed(&mut f.lfFaceName, &st.config.tray_font_face);
                f.lfHeight = font_pt_to_height(st.config.tray_font_size, dpi);
                f.lfWeight = st.config.tray_font_weight;
                f.lfItalic = if st.config.tray_font_italic { 1 } else { 0 };
                f.lfCharSet = DEFAULT_CHARSET;
                f
            });

            let mut cf: CHOOSEFONTW = unsafe { std::mem::zeroed() };
            cf.lStructSize = std::mem::size_of::<CHOOSEFONTW>() as u32;
            cf.hwndOwner = hwnd;
            cf.lpLogFont = &mut lf;
            cf.Flags = CF_SCREENFONTS | CF_INITTOLOGFONTSTRUCT | CF_FORCEFONTEXIST;

            if unsafe { ChooseFontW(&mut cf) } != FALSE {
                let updated_cfg = with_state_mut(|st| {
                    st.config.tray_font_face = from_wide_buf(&lf.lfFaceName);
                    st.config.tray_font_size = font_height_to_pt(lf.lfHeight, dpi).clamp(6, 32);
                    st.config.tray_font_weight = lf.lfWeight.clamp(100, 900);
                    st.config.tray_font_italic = lf.lfItalic != 0;
                    st.config.save();
                    st.config.clone()
                });
                set_font_button_label(hwnd, IDC_FONT, &updated_cfg);
                refresh_all_ui_after_settings();
                refresh_appearance_preview(hwnd);
            }
        }

        IDC_TRAYACTIONSC => {
            if code == CBN_SELCHANGE {
                let sel =
                    unsafe { SendDlgItemMessageW(hwnd, IDC_TRAYACTIONSC, CB_GETCURSEL, 0, 0) }
                        as i32;
                with_state_mut(|st| {
                    st.config.tray_action_sc = sel.clamp(0, 2);
                    st.config.save();
                });
            }
        }

        IDC_TRAYACTIONMC => {
            if code == CBN_SELCHANGE {
                let sel =
                    unsafe { SendDlgItemMessageW(hwnd, IDC_TRAYACTIONMC, CB_GETCURSEL, 0, 0) }
                        as i32;
                with_state_mut(|st| {
                    st.config.tray_action_mc = sel.clamp(0, 2);
                    st.config.save();
                });
            }
        }

        IDC_SHOW_CLEAN_RESULT_CHK => {
            let checked = is_checked(hwnd, ctrl_id);
            with_state_mut(|st| {
                st.config.balloon_clean_results = checked;
                st.config.save();
            });
        }

        IDC_NOTIFICATIONSOUND_CHK => {
            let checked = is_checked(hwnd, ctrl_id);
            with_state_mut(|st| {
                st.config.notifications_sound = checked;
                st.config.save();
            });
        }

        IDC_ALLOWSTANDBYLISTCLEANUP_CHK => {
            let checked = is_checked(hwnd, ctrl_id);
            with_state_mut(|st| {
                st.config.allow_standby_cleanup = checked;
                st.config.save();
            });
        }

        IDC_LOGRESULTS_CHK => {
            let checked = is_checked(hwnd, ctrl_id);
            with_state_mut(|st| {
                st.config.log_clean_results = checked;
                st.config.save();
            });
        }

        IDC_SKIPFULLSCREEN_CHK => {
            let checked = is_checked(hwnd, ctrl_id);
            with_state_mut(|st| {
                st.config.skip_fullscreen_autoclean = checked;
                st.config.save();
            });
        }

        IDC_WINSERVICE_CHK => {
            let want = is_checked(hwnd, ctrl_id);
            let res = if want {
                with_state(|st| st.config.save());
                install_and_start_service()
            } else {
                stop_and_uninstall_service()
            };
            if let Err(err) = res {
                show_msgbox(hwnd, APP_NAME, &err, MB_OK | MB_ICONWARNING);
            }
            set_check(hwnd, ctrl_id, is_service_installed());
        }

        IDC_WHITELIST_EDIT => {
            if code == EN_CHANGE {
                let val = get_dlg_text(hwnd, IDC_WHITELIST_EDIT);
                with_state_mut(|st| {
                    st.config.process_whitelist = val;
                    st.config.save();
                });
            }
        }

        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locale::LocaleManager;
    use crate::tray::TrayRenderer;
    use crate::{AppState, APP_STATE};
    use std::env;
    use std::fs;

    #[test]
    fn test_settings_memory_cleaning_persists_across_open_and_close() {
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

        let tmp = env::temp_dir().join("memreduct_settings_persist_test");
        let _ = fs::create_dir_all(&tmp);
        let ini_path = tmp.join("memreduct.ini");
        let log_path = tmp.join("memreduct.log");
        let _ = fs::remove_file(&ini_path);

        let mut cfg = AppConfig::defaults_with_paths(ini_path.clone(), log_path.clone());
        cfg.show_warning_confirmation = false;
        cfg.save();

        APP_STATE.with(|c| {
            *c.borrow_mut() = Some(AppState {
                config: cfg,
                locale: LocaleManager::default(),
                tray: TrayRenderer::default(),
                main_hwnd: 0,
                settings_hwnd: 0,
                taskbar_created_msg: 0,
                initialized_pos: false,
                last_auto_limit_ts: 0,
                last_auto_interval_ts: 0,
                session_cleaned_bytes: 0,
                session_clean_count: 0,
                limits_arr: Vec::new(),
                intervals_arr: Vec::new(),
            });
        });

        // 1. Open Settings dialog (previously clobbered AutoreductValue & TrayLevelWarning to 1!)
        open_settings_dialog(0);
        let container = with_state(|st| st.settings_hwnd);
        assert_ne!(container, 0, "Settings container dialog must be created");

        // Verify defaults were NOT clobbered to 1 on open!
        with_state(|st| {
            assert_eq!(st.config.autoreduct_value, 90);
            assert_eq!(st.config.autoreduct_interval_value, 30);
            assert_eq!(st.config.tray_level_warning, 70);
            assert_eq!(st.config.tray_level_danger, 90);
        });

        // 2. Modify Memory Cleaning page controls (regions checkboxes, threshold, interval, hotkey)
        let mem_page = SETTINGS_UI.with(|c| c.borrow().pages[1]);
        assert_ne!(mem_page, 0);

        // Uncheck Working Set (row 0), check Standby List (row 3) & Modified List (row 2)
        lv_set_check(mem_page, IDC_REGIONS, 0, false);
        lv_set_check(mem_page, IDC_REGIONS, 2, true);
        lv_set_check(mem_page, IDC_REGIONS, 3, true);

        set_check(mem_page, IDC_AUTOREDUCTENABLE_CHK, true);
        set_dlg_text(mem_page, IDC_AUTOREDUCTVALUE_CTRL, "78");

        set_check(mem_page, IDC_AUTOREDUCTINTERVALENABLE_CHK, true);
        set_dlg_text(mem_page, IDC_AUTOREDUCTINTERVALVALUE_CTRL, "15");

        set_check(mem_page, IDC_HOTKEY_CLEAN_CHK, true);
        unsafe {
            SendDlgItemMessageW(mem_page, IDC_HOTKEY_CLEAN, HKM_SETHOTKEY, 0x0373, 0);
        }

        // 3. Click Close on Settings container (WM_COMMAND -> IDC_SETTINGS_CLOSE)
        unsafe {
            SendMessageW(container, WM_COMMAND, IDC_SETTINGS_CLOSE as usize, 0);
        }

        // 4. Reload INI directly from disk and verify nothing reset!
        let reloaded = AppConfig::load_from_path(ini_path, log_path);
        assert_eq!(reloaded.reduct_mask & REDUCT_WORKINGSET, 0);
        assert_ne!(reloaded.reduct_mask & REDUCT_MODIFIEDLIST, 0);
        assert_ne!(reloaded.reduct_mask & REDUCT_STANDBYLIST, 0);
        assert!(reloaded.autoreduct_enable);
        assert_eq!(reloaded.autoreduct_value, 78);
        assert!(reloaded.autoreduct_interval_enable);
        assert_eq!(reloaded.autoreduct_interval_value, 15);
        assert!(reloaded.hotkey_clean_enable);
        assert_eq!(reloaded.hotkey_clean, 0x0373);
        assert_eq!(reloaded.tray_level_warning, 70);
        assert_eq!(reloaded.tray_level_danger, 90);

        let _ = fs::remove_dir_all(&tmp);
    }
}
