use std::collections::BTreeSet;

use crate::config::AppConfig;
use crate::ffi::*;
use crate::ids::*;

pub fn set_dlg_text(hwnd: HWND, ctrl_id: i32, text: &str) {
    let w = to_wide(text);
    unsafe {
        SetDlgItemTextW(hwnd, ctrl_id, w.as_ptr());
    }
}

pub fn get_dlg_text(hwnd: HWND, ctrl_id: i32) -> String {
    let mut buf = vec![0u16; 1024];
    let len = unsafe { GetDlgItemTextW(hwnd, ctrl_id, buf.as_mut_ptr(), buf.len() as i32) } as usize;
    String::from_utf16_lossy(&buf[..len.min(buf.len())])
}

pub fn set_check(hwnd: HWND, ctrl_id: i32, checked: bool) {
    unsafe {
        SendDlgItemMessageW(
            hwnd,
            ctrl_id,
            BM_SETCHECK,
            if checked { BST_CHECKED } else { BST_UNCHECKED },
            0,
        );
    }
}

pub fn is_checked(hwnd: HWND, ctrl_id: i32) -> bool {
    unsafe { SendDlgItemMessageW(hwnd, ctrl_id, BM_GETCHECK, 0, 0) as usize == BST_CHECKED }
}

pub fn enable_ctrl(hwnd: HWND, ctrl_id: i32, enable: bool) {
    unsafe {
        let h = GetDlgItem(hwnd, ctrl_id);
        if h != 0 {
            EnableWindow(h, if enable { TRUE } else { FALSE });
        }
    }
}

pub fn set_window_topmost(hwnd: HWND, topmost: bool) {
    if hwnd == 0 {
        return;
    }
    unsafe {
        SetWindowPos(
            hwnd,
            if topmost {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            },
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

pub fn toggle_window_visibility(hwnd: HWND, force_show: bool) {
    if hwnd == 0 {
        return;
    }
    unsafe {
        let visible = IsWindowVisible(hwnd) != FALSE;
        let iconic = IsIconic(hwnd) != FALSE;
        if force_show || !visible || iconic {
            ShowWindow(hwnd, if iconic { SW_RESTORE } else { SW_SHOW });
            SetForegroundWindow(hwnd);
        } else {
            ShowWindow(hwnd, SW_HIDE);
        }
    }
}

pub fn apply_dark_mode_window(hwnd: HWND, dark: bool) {
    if hwnd == 0 {
        return;
    }
    unsafe {
        let val: i32 = if dark { 1 } else { 0 };
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &val as *const _ as *const _,
            std::mem::size_of::<i32>() as u32,
        );
        let theme = if dark { "DarkMode_Explorer" } else { "Explorer" };
        let _ = SetWindowTheme(hwnd, to_wide(theme).as_ptr(), std::ptr::null());
        InvalidateRect(hwnd, std::ptr::null(), TRUE);
    }
}

pub fn apply_listview_theme(hwnd: HWND, ctrl_id: i32, dark: bool) {
    unsafe {
        let hlv = GetDlgItem(hwnd, ctrl_id);
        if hlv == 0 {
            return;
        }
        let theme = if dark { "DarkMode_Explorer" } else { "Explorer" };
        let _ = SetWindowTheme(hlv, to_wide(theme).as_ptr(), std::ptr::null());
        let (bk, txt) = if dark {
            (rgb(0x20, 0x20, 0x20), rgb(0xF0, 0xF0, 0xF0))
        } else {
            (rgb(0xFF, 0xFF, 0xFF), rgb(0x00, 0x00, 0x00))
        };
        SendMessageW(hlv, LVM_SETBKCOLOR, 0, bk as isize);
        SendMessageW(hlv, LVM_SETTEXTBKCOLOR, 0, bk as isize);
        SendMessageW(hlv, LVM_SETTEXTCOLOR, 0, txt as isize);
        InvalidateRect(hlv, std::ptr::null(), TRUE);
    }
}

// Bug #7 fix: actually uses passed ctrl_id instead of hardcoded IDC_FONT
pub fn set_font_button_label(hwnd: HWND, ctrl_id: i32, cfg: &AppConfig) {
    let label = format!(
        "{}, {}pt, {}",
        cfg.tray_font_face, cfg.tray_font_size, cfg.tray_font_weight
    );
    set_dlg_text(hwnd, ctrl_id, &label);
}

// UpDown helpers
pub fn updown_init(hwnd: HWND, ud_id: i32, edit_id: i32, min_val: i32, max_val: i32, val: i32) {
    unsafe {
        SendDlgItemMessageW(
            hwnd,
            ud_id,
            UDM_SETRANGE32,
            min_val as usize,
            max_val as isize,
        );
        SendDlgItemMessageW(hwnd, ud_id, UDM_SETPOS32, 0, val as isize);
    }
    set_dlg_text(hwnd, edit_id, &val.to_string());
}

pub fn updown_set(hwnd: HWND, ud_id: i32, edit_id: i32, val: i32) {
    unsafe {
        SendDlgItemMessageW(hwnd, ud_id, UDM_SETPOS32, 0, val as isize);
    }
    set_dlg_text(hwnd, edit_id, &val.to_string());
}

pub fn updown_get_opt(hwnd: HWND, ud_id: i32, edit_id: i32) -> Option<i32> {
    let txt = get_dlg_text(hwnd, edit_id);
    let trimmed = txt.trim();
    if let Ok(parsed) = trimmed.parse::<i32>() {
        return Some(parsed);
    }
    if trimmed.is_empty() {
        return None;
    }
    let mut err: BOOL = FALSE;
    let v = unsafe {
        SendDlgItemMessageW(hwnd, ud_id, UDM_GETPOS32, 0, &mut err as *mut BOOL as isize)
    };
    if err == FALSE {
        Some(v as i32)
    } else {
        None
    }
}

pub fn lv_is_checked(hwnd: HWND, ctrl_id: i32, item_idx: i32) -> bool {
    unsafe {
        let state = SendDlgItemMessageW(
            hwnd,
            ctrl_id,
            LVM_GETITEMSTATE,
            item_idx as usize,
            LVIS_STATEIMAGEMASK as isize,
        ) as u32;
        ((state >> 12) & 0xF) == 2
    }
}

// Bug #2 fix: enables/disables both UpDown and its buddy Edit control cleanly
pub fn updown_enable_with_buddy(hwnd: HWND, ud_id: i32, enable: bool) {
    unsafe {
        let hud = GetDlgItem(hwnd, ud_id);
        if hud != 0 {
            let buddy = SendMessageW(hud, UDM_GETBUDDY, 0, 0) as HWND;
            if buddy != 0 {
                EnableWindow(buddy, if enable { TRUE } else { FALSE });
            }
            EnableWindow(hud, if enable { TRUE } else { FALSE });
        }
    }
}

// ListView helpers
pub fn lv_set_ex_style(hwnd: HWND, ctrl_id: i32, ex_style: u32) {
    unsafe {
        SendDlgItemMessageW(
            hwnd,
            ctrl_id,
            LVM_SETEXTENDEDLISTVIEWSTYLE,
            0,
            ex_style as isize,
        );
    }
}

pub fn lv_reset_columns_and_items(hwnd: HWND, ctrl_id: i32) {
    unsafe {
        let hlv = GetDlgItem(hwnd, ctrl_id);
        if hlv == 0 {
            return;
        }
        SendMessageW(hlv, LVM_DELETEALLITEMS, 0, 0);
        for _ in 0..4 {
            if SendMessageW(hlv, LVM_DELETECOLUMN, 0, 0) == 0 {
                break;
            }
        }
    }
}

pub fn lv_add_column(hwnd: HWND, ctrl_id: i32, col_idx: i32, width: i32, fmt: i32) {
    unsafe {
        let mut empty = [0u16; 1];
        let mut col: LVCOLUMNW = std::mem::zeroed();
        col.mask = LVCF_FMT | LVCF_WIDTH | LVCF_TEXT;
        col.fmt = fmt;
        col.cx = width;
        col.pszText = empty.as_mut_ptr();
        SendDlgItemMessageW(
            hwnd,
            ctrl_id,
            LVM_INSERTCOLUMNW,
            col_idx as usize,
            &col as *const _ as isize,
        );
    }
}

pub fn lv_resize_two_columns_50_50(hwnd: HWND, ctrl_id: i32) {
    unsafe {
        let hlv = GetDlgItem(hwnd, ctrl_id);
        if hlv == 0 {
            return;
        }
        let mut rc = RECT::default();
        if GetClientRect(hlv, &mut rc) != FALSE {
            let total = (rc.right - rc.left).max(120);
            let col0 = (total * 54) / 100;
            let col1 = (total - col0 - 4).max(40);
            SendMessageW(hlv, LVM_SETCOLUMNWIDTH, 0, col0 as isize);
            SendMessageW(hlv, LVM_SETCOLUMNWIDTH, 1, col1 as isize);
        }
    }
}

pub fn lv_resize_single_column_full(hwnd: HWND, ctrl_id: i32) {
    unsafe {
        let hlv = GetDlgItem(hwnd, ctrl_id);
        if hlv == 0 {
            return;
        }
        let mut rc = RECT::default();
        if GetClientRect(hlv, &mut rc) != FALSE {
            let w = (rc.right - rc.left - 4).max(100);
            SendMessageW(hlv, LVM_SETCOLUMNWIDTH, 0, w as isize);
        }
    }
}

pub fn lv_enable_groups(hwnd: HWND, ctrl_id: i32, enable: bool) {
    unsafe {
        SendDlgItemMessageW(
            hwnd,
            ctrl_id,
            LVM_ENABLEGROUPVIEW,
            if enable { 1 } else { 0 },
            0,
        );
    }
}

pub fn lv_add_group(hwnd: HWND, ctrl_id: i32, group_id: i32, header: &str) {
    let mut wide = to_wide(header);
    unsafe {
        let mut grp: LVGROUP = std::mem::zeroed();
        grp.cbSize = std::mem::size_of::<LVGROUP>() as u32;
        grp.mask = LVGF_HEADER | LVGF_GROUPID | LVGF_STATE;
        grp.pszHeader = wide.as_mut_ptr();
        grp.cchHeader = wide.len() as i32;
        grp.iGroupId = group_id;
        grp.stateMask = LVGS_COLLAPSIBLE;
        grp.state = LVGS_COLLAPSIBLE;
        SendDlgItemMessageW(
            hwnd,
            ctrl_id,
            LVM_INSERTGROUP,
            usize::MAX,
            &grp as *const _ as isize,
        );
    }
}

pub fn lv_set_group_header(hwnd: HWND, ctrl_id: i32, group_id: i32, header: &str) {
    let mut wide = to_wide(header);
    unsafe {
        let mut grp: LVGROUP = std::mem::zeroed();
        grp.cbSize = std::mem::size_of::<LVGROUP>() as u32;
        grp.mask = LVGF_HEADER;
        grp.pszHeader = wide.as_mut_ptr();
        grp.cchHeader = wide.len() as i32;
        SendDlgItemMessageW(
            hwnd,
            ctrl_id,
            LVM_SETGROUPINFO,
            group_id as usize,
            &grp as *const _ as isize,
        );
    }
}

pub fn lv_insert_item(
    hwnd: HWND,
    ctrl_id: i32,
    item_idx: i32,
    text: &str,
    group_id: Option<i32>,
    lparam: isize,
) {
    let mut wide = to_wide(text);
    unsafe {
        let mut item: LVITEMW = std::mem::zeroed();
        item.mask = LVIF_TEXT | LVIF_PARAM;
        if let Some(gid) = group_id {
            item.mask |= LVIF_GROUPID;
            item.iGroupId = gid;
        }
        item.iItem = item_idx;
        item.iSubItem = 0;
        item.pszText = wide.as_mut_ptr();
        item.lParam = lparam;
        SendDlgItemMessageW(
            hwnd,
            ctrl_id,
            LVM_INSERTITEMW,
            0,
            &item as *const _ as isize,
        );
    }
}

pub fn lv_set_item_text(hwnd: HWND, ctrl_id: i32, item_idx: i32, sub_idx: i32, text: &str) {
    let mut wide = to_wide(text);
    unsafe {
        let mut item: LVITEMW = std::mem::zeroed();
        item.mask = LVIF_TEXT;
        item.iItem = item_idx;
        item.iSubItem = sub_idx;
        item.pszText = wide.as_mut_ptr();
        SendDlgItemMessageW(hwnd, ctrl_id, LVM_SETITEMW, 0, &item as *const _ as isize);
    }
}

pub fn lv_set_item_lparam(hwnd: HWND, ctrl_id: i32, item_idx: i32, lparam: isize) {
    unsafe {
        let mut item: LVITEMW = std::mem::zeroed();
        item.mask = LVIF_PARAM;
        item.iItem = item_idx;
        item.iSubItem = 0;
        item.lParam = lparam;
        SendDlgItemMessageW(hwnd, ctrl_id, LVM_SETITEMW, 0, &item as *const _ as isize);
    }
}

pub fn lv_get_item_lparam(hwnd: HWND, ctrl_id: i32, item_idx: i32) -> isize {
    unsafe {
        let mut item: LVITEMW = std::mem::zeroed();
        item.mask = LVIF_PARAM;
        item.iItem = item_idx;
        item.iSubItem = 0;
        SendDlgItemMessageW(
            hwnd,
            ctrl_id,
            LVM_GETITEMW,
            0,
            &mut item as *mut LVITEMW as isize,
        );
        item.lParam
    }
}

pub fn lv_set_check(hwnd: HWND, ctrl_id: i32, item_idx: i32, checked: bool) {
    unsafe {
        let mut item: LVITEMW = std::mem::zeroed();
        item.mask = LVIF_STATE;
        item.iItem = item_idx;
        item.stateMask = LVIS_STATEIMAGEMASK;
        item.state = indextostateimagemask(if checked { 2 } else { 1 });
        SendDlgItemMessageW(hwnd, ctrl_id, LVM_SETITEMW, 0, &item as *const _ as isize);
    }
}

// Menu helpers with RAII guard so menus never leak on early exit
pub struct OwnedMenu(pub HMENU);

impl Drop for OwnedMenu {
    fn drop(&mut self) {
        if self.0 != 0 {
            unsafe {
                DestroyMenu(self.0);
            }
        }
    }
}

pub fn menu_set_text(hmenu: HMENU, item: u32, by_pos: bool, text: &str) {
    if hmenu == 0 {
        return;
    }
    let mut wide = to_wide(text);
    unsafe {
        let mut mii: MENUITEMINFOW = std::mem::zeroed();
        mii.cbSize = std::mem::size_of::<MENUITEMINFOW>() as u32;
        mii.fMask = MIIM_STRING;
        mii.dwTypeData = wide.as_mut_ptr();
        SetMenuItemInfoW(hmenu, item, if by_pos { TRUE } else { FALSE }, &mii);
    }
}

pub fn menu_check(hmenu: HMENU, cmd_id: i32, checked: bool) {
    if hmenu == 0 {
        return;
    }
    unsafe {
        CheckMenuItem(
            hmenu,
            cmd_id as u32,
            MF_BYCOMMAND | if checked { MF_CHECKED } else { MF_UNCHECKED },
        );
    }
}

pub fn menu_enable(hmenu: HMENU, cmd_id: i32, enabled: bool) {
    if hmenu == 0 {
        return;
    }
    unsafe {
        EnableMenuItem(
            hmenu,
            cmd_id as u32,
            MF_BYCOMMAND | if enabled { MF_ENABLED } else { MF_GRAYED | MF_DISABLED },
        );
    }
}

pub fn generate_dynamic_values(current: u32, max_value: u32, max_len: usize) -> Vec<u32> {
    let mut set = BTreeSet::new();
    for i in 1..9u32 {
        let v = i * 10;
        if v <= max_value {
            set.insert(v);
        }
    }
    let start = current.saturating_sub(2).max(1);
    let end = (current + 2).min(max_value);
    for v in start..=end {
        if v >= 5 && v <= max_value {
            set.insert(v);
        }
    }
    if current >= 1 && current <= max_value {
        set.insert(current);
    }
    set.into_iter().take(max_len).collect()
}

pub fn populate_tray_dynamic_menu(
    hsubmenu: HMENU,
    base_id: i32,
    disable_label: &str,
    values: &[u32],
    suffix: &str,
    current_val: u32,
    is_enabled: bool,
    elevated: bool,
) {
    if hsubmenu == 0 {
        return;
    }
    menu_set_text(hsubmenu, 0, true, disable_label);

    let mut checked_any = false;
    for (idx, &v) in values.iter().enumerate() {
        let cmd_id = (base_id + idx as i32) as usize;
        let label = format!("{v}{suffix}");
        unsafe {
            AppendMenuW(hsubmenu, MF_STRING, cmd_id, to_wide(&label).as_ptr());
        }
        if !elevated {
            menu_enable(hsubmenu, cmd_id as i32, false);
        }
        if is_enabled && v == current_val {
            unsafe {
                CheckMenuRadioItem(
                    hsubmenu,
                    cmd_id as u32,
                    cmd_id as u32,
                    cmd_id as u32,
                    MF_BYCOMMAND,
                );
            }
            checked_any = true;
        }
    }

    if !is_enabled || !checked_any {
        unsafe {
            CheckMenuRadioItem(hsubmenu, 0, 0, 0, MF_BYPOSITION);
        }
    }
}

pub fn register_clean_hotkey(hwnd: HWND, cfg: &AppConfig) {
    if hwnd == 0 {
        return;
    }
    unsafe {
        let _ = UnregisterHotKey(hwnd, HOTKEY_UID);
        if !cfg.hotkey_clean_enable || cfg.hotkey_clean == 0 {
            return;
        }
        let vk = cfg.hotkey_clean & 0xFF;
        let hk_mods = ((cfg.hotkey_clean >> 8) & 0xFF) as u8;
        let mut fs_mods = 0u32;
        if (hk_mods & HOTKEYF_CONTROL) != 0 {
            fs_mods |= MOD_CONTROL;
        }
        if (hk_mods & HOTKEYF_SHIFT) != 0 {
            fs_mods |= MOD_SHIFT;
        }
        if (hk_mods & HOTKEYF_ALT) != 0 {
            fs_mods |= MOD_ALT;
        }
        if RegisterHotKey(hwnd, HOTKEY_UID, fs_mods, vk) == FALSE {
            let msg = format!("Could not register hotkey (error {}).", GetLastError());
            MessageBoxW(
                hwnd,
                to_wide(&msg).as_ptr(),
                to_wide(APP_NAME).as_ptr(),
                MB_OK | MB_ICONWARNING,
            );
        }
    }
}

// Confirmation dialog with "Do not ask again"
struct ConfirmCtx<'a> {
    message: &'a str,
    dont_ask_label: &'a str,
    dont_ask_checked: bool,
}

unsafe extern "system" fn confirm_dlg_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    match msg {
        WM_INITDIALOG => {
            SetWindowLongPtrW(hwnd, -21, lparam); // GWLP_USERDATA
            if lparam != 0 {
                let ctx = &*(lparam as *const ConfirmCtx<'_>);
                set_dlg_text(hwnd, IDC_CONFIRM_TEXT, ctx.message);
                set_dlg_text(hwnd, IDC_CONFIRM_DONTASK, ctx.dont_ask_label);
            }
            1
        }
        WM_COMMAND => {
            let id = loword(wparam) as i32;
            if id == IDYES || id == IDOK {
                let ptr = GetWindowLongPtrW(hwnd, -21) as *mut ConfirmCtx<'_>;
                if !ptr.is_null() {
                    (*ptr).dont_ask_checked = is_checked(hwnd, IDC_CONFIRM_DONTASK);
                }
                EndDialog(hwnd, IDYES as isize);
                1
            } else if id == IDNO || id == IDCANCEL {
                EndDialog(hwnd, IDNO as isize);
                1
            } else {
                0
            }
        }
        WM_CLOSE => {
            EndDialog(hwnd, IDNO as isize);
            1
        }
        _ => 0,
    }
}

/// Returns `(confirmed, dont_ask_again_checked)`.
pub fn show_confirm_dialog(parent: HWND, message: &str, dont_ask_label: &str) -> (bool, bool) {
    let mut ctx = ConfirmCtx {
        message,
        dont_ask_label,
        dont_ask_checked: false,
    };
    let res = unsafe {
        DialogBoxParamW(
            GetModuleHandleW(std::ptr::null()),
            make_int_resource(IDD_CONFIRM),
            parent,
            Some(confirm_dlg_proc),
            &mut ctx as *mut _ as isize,
        )
    };
    (res == IDYES as isize, ctx.dont_ask_checked)
}

pub fn show_msgbox(hwnd: HWND, title: &str, text: &str, flags: u32) -> i32 {
    unsafe {
        MessageBoxW(
            hwnd,
            to_wide(text).as_ptr(),
            to_wide(title).as_ptr(),
            flags,
        )
    }
}

pub fn open_url(url: &str) {
    unsafe {
        ShellExecuteW(
            0,
            to_wide("open").as_ptr(),
            to_wide(url).as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
    }
}

pub fn open_task_manager() {
    unsafe {
        ShellExecuteW(
            0,
            to_wide("open").as_ptr(),
            to_wide("taskmgr.exe").as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
    }
}

pub fn show_about_box(hwnd: HWND) {
    let about = format!(
        "{APP_NAME} v{APP_VERSION} (Rust Edition)\r\n\
         Original author: {APP_AUTHOR}\r\n\r\n\
         Lightweight real-time system memory monitor & cleaner.\r\n\
         Includes selective working-set whitelist, fullscreen game-mode guard,\r\n\
         live Standby/Modified metrics, and headless CLI flags.\r\n\r\n\
         Website: {APP_WEBSITE}"
    );
    show_msgbox(hwnd, APP_NAME, &about, MB_OK | MB_ICONINFORMATION);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_dynamic_values_sorted_and_clamped() {
        let vals = generate_dynamic_values(85, 99, 13);
        assert!(vals.windows(2).all(|w| w[0] < w[1]));
        assert!(vals.contains(&85));
        assert!(vals.iter().all(|&x| x <= 99));
    }
}
