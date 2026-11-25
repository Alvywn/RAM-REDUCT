#![cfg_attr(not(test), windows_subsystem = "windows")]
#![allow(non_snake_case, non_camel_case_types, dead_code)]

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const APP_NAME: &str = "RAM Reduct";
const APP_VERSION: &str = "3.5.3";
const APP_PUBLISHER: &str = "Henry++";
const APP_WEBSITE: &str = "https://github.com/henrypp/memreduct";
const APP_USER_MODEL_ID: &str = "Henrypp.RamReduct";

const UNINSTALL_REG_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\RAM Reduct";
const HKCU_UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\RAM Reduct";
const APP_PATHS_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\ramreduct.exe";
const AUTORUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const STARTUP_APPROVED_KEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

const SKIP_UAC_TASK_NAME: &str = "RamReductSkipUac";
const SKIP_UAC_BG_TASK_NAME: &str = "RamReductSkipUacBg";
const SERVICE_NAME: &str = "RamReductService";

const PAYLOAD_EXE: &[u8] = include_bytes!("../../target/release/ramreduct.exe");
const PAYLOAD_LNG: &[u8] = include_bytes!("../../bin/memreduct.lng");
const PAYLOAD_LICENSE: &[u8] = include_bytes!("../../bin/License.txt");
const PAYLOAD_README: &[u8] = include_bytes!("../../bin/Readme.txt");
const PAYLOAD_HISTORY: &[u8] = include_bytes!("../../bin/History.txt");

// Resource IDs
const IDI_SETUP: u16 = 100;
const IDD_INSTALLER: u16 = 101;
const IDC_PATH_EDIT: i32 = 1004;
const IDC_BROWSE_BTN: i32 = 1005;
const IDC_DESKTOP_CHK: i32 = 1006;
const IDC_STARTMENU_CHK: i32 = 1007;
const IDC_LAUNCH_CHK: i32 = 1008;
const IDC_STATUS_TEXT: i32 = 1009;
const IDC_INSTALL_BTN: i32 = 1010;
const IDC_UNINSTALL_BTN: i32 = 1011;
const IDC_CLOSE_BTN: i32 = 1012;
const IDC_AUTORUN_CHK: i32 = 1013;

type HWND = isize;
type HINSTANCE = isize;
type HICON = isize;
type HKEY = isize;
type WPARAM = usize;
type LPARAM = isize;
type BOOL = i32;
type DLGPROC = Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> isize>;

const FALSE: BOOL = 0;
const TRUE: BOOL = 1;
const WM_INITDIALOG: u32 = 0x0110;
const WM_COMMAND: u32 = 0x0111;
const WM_CLOSE: u32 = 0x0010;
const WM_SETICON: u32 = 0x0080;
const ICON_SMALL: WPARAM = 0;
const ICON_BIG: WPARAM = 1;
const BM_GETCHECK: u32 = 0x00F0;
const BM_SETCHECK: u32 = 0x00F1;
const BST_CHECKED: usize = 1;
const BST_UNCHECKED: usize = 0;
const IDOK: i32 = 1;
const IDCANCEL: i32 = 2;
const IDYES: i32 = 6;
const MB_OK: u32 = 0x00000000;
const MB_YESNO: u32 = 0x00000004;
const MB_ICONINFORMATION: u32 = 0x00000040;
const MB_ICONWARNING: u32 = 0x00000030;
const MB_ICONSTOP: u32 = 0x00000010;
const SW_SHOWNORMAL: i32 = 1;

const HKEY_CURRENT_USER: HKEY = -2147483647i32 as isize;
const HKEY_LOCAL_MACHINE: HKEY = -2147483646i32 as isize;
const KEY_READ: u32 = 0x20019;
const KEY_WRITE: u32 = 0x20006;
const REG_SZ: u32 = 1;
const REG_BINARY: u32 = 3;
const REG_DWORD: u32 = 4;
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[repr(C)]
struct BROWSEINFOW {
    hwndOwner: HWND,
    pidlRoot: isize,
    pszDisplayName: *mut u16,
    lpszTitle: *const u16,
    ulFlags: u32,
    lpfn: usize,
    lParam: LPARAM,
    iImage: i32,
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(lpModuleName: *const u16) -> HINSTANCE;
}

#[link(name = "user32")]
extern "system" {
    fn DialogBoxParamW(
        hInstance: HINSTANCE,
        lpTemplateName: *const u16,
        hWndParent: HWND,
        lpDialogFunc: DLGPROC,
        dwInitParam: LPARAM,
    ) -> isize;
    fn EndDialog(hDlg: HWND, nResult: isize) -> BOOL;
    fn SendMessageW(hWnd: HWND, Msg: u32, wParam: WPARAM, lParam: LPARAM) -> isize;
    fn SendDlgItemMessageW(
        hDlg: HWND,
        nIDDlgItem: i32,
        Msg: u32,
        wParam: WPARAM,
        lParam: LPARAM,
    ) -> isize;
    fn SetDlgItemTextW(hDlg: HWND, nIDDlgItem: i32, lpString: *const u16) -> BOOL;
    fn GetDlgItemTextW(
        hDlg: HWND,
        nIDDlgItem: i32,
        lpString: *mut u16,
        cchMax: i32,
    ) -> u32;
    fn GetDlgItem(hDlg: HWND, nIDDlgItem: i32) -> HWND;
    fn EnableWindow(hWnd: HWND, bEnable: BOOL) -> BOOL;
    fn LoadIconW(hInstance: HINSTANCE, lpIconName: *const u16) -> HICON;
    fn MessageBoxW(hWnd: HWND, lpText: *const u16, lpCaption: *const u16, uType: u32) -> i32;
}

#[link(name = "shell32")]
extern "system" {
    fn SHBrowseForFolderW(lpbi: *mut BROWSEINFOW) -> isize;
    fn SHGetPathFromIDListW(pidl: isize, pszPath: *mut u16) -> BOOL;
    fn ShellExecuteW(
        hwnd: HWND,
        lpOperation: *const u16,
        lpFile: *const u16,
        lpParameters: *const u16,
        lpDirectory: *const u16,
        nShowCmd: i32,
    ) -> HINSTANCE;
}

#[link(name = "ole32")]
extern "system" {
    fn CoTaskMemFree(pv: isize);
}

#[link(name = "advapi32")]
extern "system" {
    fn RegCreateKeyExW(
        hKey: HKEY,
        lpSubKey: *const u16,
        Reserved: u32,
        lpClass: *mut u16,
        dwOptions: u32,
        samDesired: u32,
        lpSecurityAttributes: *const core::ffi::c_void,
        phkResult: *mut HKEY,
        lpdwDisposition: *mut u32,
    ) -> i32;
    fn RegOpenKeyExW(
        hKey: HKEY,
        lpSubKey: *const u16,
        ulOptions: u32,
        samDesired: u32,
        phkResult: *mut HKEY,
    ) -> i32;
    fn RegQueryValueExW(
        hKey: HKEY,
        lpValueName: *const u16,
        lpReserved: *mut u32,
        lpType: *mut u32,
        lpData: *mut u8,
        lpcbData: *mut u32,
    ) -> i32;
    fn RegSetValueExW(
        hKey: HKEY,
        lpValueName: *const u16,
        Reserved: u32,
        dwType: u32,
        lpData: *const u8,
        cbData: u32,
    ) -> i32;
    fn RegDeleteValueW(hKey: HKEY, lpValueName: *const u16) -> i32;
    fn RegDeleteKeyW(hKey: HKEY, lpSubKey: *const u16) -> i32;
    fn RegCloseKey(hKey: HKEY) -> i32;
}

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

const fn make_int_resource(id: u16) -> *const u16 {
    id as usize as *const u16
}

fn default_install_dir() -> PathBuf {
    if let Some(pf) = env::var_os("ProgramFiles") {
        PathBuf::from(pf).join(APP_NAME)
    } else {
        PathBuf::from(r"C:\Program Files\RAM Reduct")
    }
}

fn read_installed_location() -> Option<PathBuf> {
    unsafe {
        let mut hkey: HKEY = 0;
        if RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            to_wide(UNINSTALL_REG_KEY).as_ptr(),
            0,
            KEY_READ,
            &mut hkey,
        ) != 0
        {
            return None;
        }
        let mut buf = vec![0u16; 512];
        let mut bytes = (buf.len() * 2) as u32;
        let mut reg_type = 0u32;
        let ok = RegQueryValueExW(
            hkey,
            to_wide("InstallLocation").as_ptr(),
            std::ptr::null_mut(),
            &mut reg_type,
            buf.as_mut_ptr() as *mut u8,
            &mut bytes,
        );
        RegCloseKey(hkey);
        if ok == 0 && bytes >= 2 {
            let chars = (bytes as usize / 2).saturating_sub(1);
            let s = String::from_utf16_lossy(&buf[..chars]);
            if !s.trim().is_empty() {
                return Some(PathBuf::from(s));
            }
        }
        None
    }
}

fn stop_running_memreduct() {
    let _ = Command::new("sc.exe")
        .args(["stop", SERVICE_NAME])
        .creation_flags(CREATE_NO_WINDOW)
        .status();
    let _ = Command::new("taskkill.exe")
        .args(["/F", "/IM", "ramreduct.exe"])
        .creation_flags(CREATE_NO_WINDOW)
        .status();
    let _ = Command::new("taskkill.exe")
        .args(["/F", "/IM", "memreduct.exe"])
        .creation_flags(CREATE_NO_WINDOW)
        .status();
}

fn create_shortcut(lnk_path: &Path, target: &Path, work_dir: &Path) {
    if let Some(parent) = lnk_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let script = format!(
        "$s=(New-Object -COM WScript.Shell).CreateShortcut('{}');\
         $s.TargetPath='{}';\
         $s.WorkingDirectory='{}';\
         $s.Description='Lightweight real-time memory management application';\
         $s.IconLocation='{},0';\
         $s.Save()",
        lnk_path.display().to_string().replace('\'', "''"),
        target.display().to_string().replace('\'', "''"),
        work_dir.display().to_string().replace('\'', "''"),
        target.display().to_string().replace('\'', "''"),
    );
    let _ = Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .status();
}

fn start_menu_shortcut_path() -> PathBuf {
    let base = env::var_os("ProgramData")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
    base.join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join(format!("{APP_NAME}.lnk"))
}

fn desktop_shortcut_path() -> PathBuf {
    let base = env::var_os("PUBLIC")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(r"C:\Users\Public"));
    base.join("Desktop").join(format!("{APP_NAME}.lnk"))
}

fn write_reg_sz(hkey: HKEY, name: &str, value: &str) {
    let w = to_wide(value);
    let name_w = to_wide(name);
    unsafe {
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
}

fn write_reg_dword(hkey: HKEY, name: &str, value: u32) {
    unsafe {
        RegSetValueExW(
            hkey,
            to_wide(name).as_ptr(),
            0,
            REG_DWORD,
            &value as *const u32 as *const u8,
            4,
        );
    }
}

fn register_windows_integration(install_dir: &Path, enable_autorun: bool) {
    let exe_path = install_dir.join("ramreduct.exe");
    let exe_str = exe_path.to_string_lossy().into_owned();
    let dir_str = install_dir.to_string_lossy().into_owned();

    unsafe {
        // Remove per-user HKCU duplicate uninstall entry if present so Installed Apps shows one clean entry
        RegDeleteKeyW(HKEY_CURRENT_USER, to_wide(HKCU_UNINSTALL_KEY).as_ptr());

        // 1. HKLM Uninstall (Windows Settings -> Apps -> Installed Apps & Control Panel)
        let mut hkey: HKEY = 0;
        let mut disp = 0u32;
        if RegCreateKeyExW(
            HKEY_LOCAL_MACHINE,
            to_wide(UNINSTALL_REG_KEY).as_ptr(),
            0,
            std::ptr::null_mut(),
            0,
            KEY_WRITE,
            std::ptr::null(),
            &mut hkey,
            &mut disp,
        ) == 0
        {
            let uninst_path = install_dir.join("uninstall.exe");
            let uninst_cmd = format!("\"{}\" /uninstall", uninst_path.display());
            let quiet_uninst_cmd = format!("\"{}\" /uninstall /S", uninst_path.display());

            write_reg_sz(hkey, "DisplayName", &format!("{APP_NAME} {APP_VERSION}"));
            write_reg_sz(hkey, "DisplayVersion", APP_VERSION);
            write_reg_sz(hkey, "Publisher", APP_PUBLISHER);
            write_reg_sz(hkey, "InstallLocation", &dir_str);
            write_reg_sz(hkey, "DisplayIcon", &format!("{exe_str},0"));
            write_reg_sz(hkey, "UninstallString", &uninst_cmd);
            write_reg_sz(hkey, "QuietUninstallString", &quiet_uninst_cmd);
            write_reg_sz(hkey, "URLInfoAbout", APP_WEBSITE);
            write_reg_sz(hkey, "AppUserModelID", APP_USER_MODEL_ID);
            write_reg_dword(hkey, "NoModify", 1);
            write_reg_dword(hkey, "NoRepair", 1);
            write_reg_dword(hkey, "EstimatedSize", 1024);
            RegCloseKey(hkey);
        }

        // 2. HKLM App Paths (Win+R -> memreduct & Windows Search)
        if RegCreateKeyExW(
            HKEY_LOCAL_MACHINE,
            to_wide(APP_PATHS_KEY).as_ptr(),
            0,
            std::ptr::null_mut(),
            0,
            KEY_WRITE,
            std::ptr::null(),
            &mut hkey,
            &mut disp,
        ) == 0
        {
            write_reg_sz(hkey, "", &exe_str);
            write_reg_sz(hkey, "Path", &dir_str);
            RegCloseKey(hkey);
        }

        // 3. Skip UAC Scheduled Tasks (so background startup & manual launch run elevated seamlessly)
        let tr_gui = format!("\"{exe_str}\"");
        let tr_bg = format!("\"{exe_str}\" -background");
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
                SKIP_UAC_TASK_NAME,
                "/TR",
                &tr_gui,
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
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

        // 4. Windows Startup (Task Manager -> Startup Apps & Windows Settings -> Startup)
        if enable_autorun {
            if RegCreateKeyExW(
                HKEY_CURRENT_USER,
                to_wide(AUTORUN_KEY).as_ptr(),
                0,
                std::ptr::null_mut(),
                0,
                KEY_WRITE,
                std::ptr::null(),
                &mut hkey,
                &mut disp,
            ) == 0
            {
                write_reg_sz(hkey, APP_NAME, &tr_bg);
                RegCloseKey(hkey);
            }

            if RegCreateKeyExW(
                HKEY_CURRENT_USER,
                to_wide(STARTUP_APPROVED_KEY).as_ptr(),
                0,
                std::ptr::null_mut(),
                0,
                KEY_WRITE,
                std::ptr::null(),
                &mut hkey,
                &mut disp,
            ) == 0
            {
                let enabled_bytes: [u8; 12] = [0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
                RegSetValueExW(
                    hkey,
                    to_wide(APP_NAME).as_ptr(),
                    0,
                    REG_BINARY,
                    enabled_bytes.as_ptr(),
                    enabled_bytes.len() as u32,
                );
                RegCloseKey(hkey);
            }
        }
    }
}

fn do_install(
    install_dir: &Path,
    create_start_menu: bool,
    create_desktop: bool,
    enable_autorun: bool,
    launch_after: bool,
) -> Result<(), String> {
    stop_running_memreduct();
    fs::create_dir_all(install_dir)
        .map_err(|e| format!("Failed to create folder {}: {e}", install_dir.display()))?;

    let exe_path = install_dir.join("ramreduct.exe");
    fs::write(&exe_path, PAYLOAD_EXE).map_err(|e| format!("Failed writing ramreduct.exe: {e}"))?;
    fs::write(install_dir.join("memreduct.lng"), PAYLOAD_LNG)
        .map_err(|e| format!("Failed writing memreduct.lng: {e}"))?;
    fs::write(install_dir.join("License.txt"), PAYLOAD_LICENSE)
        .map_err(|e| format!("Failed writing License.txt: {e}"))?;
    fs::write(install_dir.join("Readme.txt"), PAYLOAD_README)
        .map_err(|e| format!("Failed writing Readme.txt: {e}"))?;
    fs::write(install_dir.join("History.txt"), PAYLOAD_HISTORY)
        .map_err(|e| format!("Failed writing History.txt: {e}"))?;

    if let Ok(self_exe) = env::current_exe() {
        let uninst = install_dir.join("uninstall.exe");
        if !self_exe.eq_ignore_ascii_case_path(&uninst) {
            let _ = fs::copy(&self_exe, &uninst);
        }
    }

    if create_start_menu {
        create_shortcut(&start_menu_shortcut_path(), &exe_path, install_dir);
    }
    if create_desktop {
        create_shortcut(&desktop_shortcut_path(), &exe_path, install_dir);
    }

    register_windows_integration(install_dir, enable_autorun);

    if launch_after {
        unsafe {
            ShellExecuteW(
                0,
                to_wide("open").as_ptr(),
                to_wide(&exe_path.to_string_lossy()).as_ptr(),
                to_wide("-background").as_ptr(),
                to_wide(&install_dir.to_string_lossy()).as_ptr(),
                SW_SHOWNORMAL,
            );
        }
    }

    Ok(())
}

fn do_uninstall(install_dir: &Path) {
    stop_running_memreduct();
    let _ = Command::new("sc.exe")
        .args(["delete", SERVICE_NAME])
        .creation_flags(CREATE_NO_WINDOW)
        .status();
    let _ = Command::new("schtasks.exe")
        .args(["/Delete", "/F", "/TN", SKIP_UAC_TASK_NAME])
        .creation_flags(CREATE_NO_WINDOW)
        .status();
    let _ = Command::new("schtasks.exe")
        .args(["/Delete", "/F", "/TN", SKIP_UAC_BG_TASK_NAME])
        .creation_flags(CREATE_NO_WINDOW)
        .status();

    let _ = fs::remove_file(start_menu_shortcut_path());
    let _ = fs::remove_file(desktop_shortcut_path());

    for file in [
        "ramreduct.exe",
        "memreduct.exe",
        "memreduct.lng",
        "License.txt",
        "Readme.txt",
        "History.txt",
        "ramreduct.log",
        "memreduct.log",
    ] {
        let _ = fs::remove_file(install_dir.join(file));
    }

    unsafe {
        let mut hkey: HKEY = 0;
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            to_wide(AUTORUN_KEY).as_ptr(),
            0,
            KEY_WRITE,
            &mut hkey,
        ) == 0
        {
            RegDeleteValueW(hkey, to_wide(APP_NAME).as_ptr());
            RegCloseKey(hkey);
        }
        RegDeleteKeyW(HKEY_LOCAL_MACHINE, to_wide(UNINSTALL_REG_KEY).as_ptr());
        RegDeleteKeyW(HKEY_LOCAL_MACHINE, to_wide(APP_PATHS_KEY).as_ptr());
        RegDeleteKeyW(HKEY_CURRENT_USER, to_wide(HKCU_UNINSTALL_KEY).as_ptr());
    }

    let uninst = install_dir.join("uninstall.exe");
    let running_from_install = env::current_exe()
        .ok()
        .map(|p| p.eq_ignore_ascii_case_path(&uninst))
        .unwrap_or(false);

    if running_from_install {
        let cmd = format!(
            "ping -n 2 127.0.0.1 >nul & del /f /q \"{}\" & rmdir /q \"{}\"",
            uninst.display(),
            install_dir.display()
        );
        let _ = Command::new("cmd.exe")
            .args(["/C", &cmd])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    } else {
        let _ = fs::remove_file(&uninst);
        let _ = fs::remove_dir(install_dir);
    }
}

trait PathEqIgnoreCase {
    fn eq_ignore_ascii_case_path(&self, other: &Path) -> bool;
}

impl PathEqIgnoreCase for PathBuf {
    fn eq_ignore_ascii_case_path(&self, other: &Path) -> bool {
        self.to_string_lossy()
            .eq_ignore_ascii_case(&other.to_string_lossy())
    }
}

fn set_text(hwnd: HWND, id: i32, text: &str) {
    unsafe {
        SetDlgItemTextW(hwnd, id, to_wide(text).as_ptr());
    }
}

fn get_text(hwnd: HWND, id: i32) -> String {
    let mut buf = vec![0u16; 1024];
    let len = unsafe { GetDlgItemTextW(hwnd, id, buf.as_mut_ptr(), buf.len() as i32) } as usize;
    String::from_utf16_lossy(&buf[..len.min(buf.len())])
}

fn set_chk(hwnd: HWND, id: i32, checked: bool) {
    unsafe {
        SendDlgItemMessageW(
            hwnd,
            id,
            BM_SETCHECK,
            if checked { BST_CHECKED } else { BST_UNCHECKED },
            0,
        );
    }
}

fn get_chk(hwnd: HWND, id: i32) -> bool {
    unsafe { SendDlgItemMessageW(hwnd, id, BM_GETCHECK, 0, 0) as usize == BST_CHECKED }
}

unsafe extern "system" fn installer_dlg_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    _lparam: LPARAM,
) -> isize {
    match msg {
        WM_INITDIALOG => {
            let hinst = GetModuleHandleW(std::ptr::null());
            let hicon = LoadIconW(hinst, make_int_resource(IDI_SETUP));
            if hicon != 0 {
                SendMessageW(hwnd, WM_SETICON, ICON_SMALL, hicon);
                SendMessageW(hwnd, WM_SETICON, ICON_BIG, hicon);
            }

            let existing = read_installed_location();
            let dir = existing.clone().unwrap_or_else(default_install_dir);
            set_text(hwnd, IDC_PATH_EDIT, &dir.to_string_lossy());
            set_chk(hwnd, IDC_STARTMENU_CHK, true);
            set_chk(hwnd, IDC_DESKTOP_CHK, false);
            set_chk(hwnd, IDC_AUTORUN_CHK, true);
            set_chk(hwnd, IDC_LAUNCH_CHK, true);

            let uninst_btn = GetDlgItem(hwnd, IDC_UNINSTALL_BTN);
            if uninst_btn != 0 {
                EnableWindow(uninst_btn, if existing.is_some() { TRUE } else { FALSE });
            }

            if existing.is_some() {
                set_text(
                    hwnd,
                    IDC_STATUS_TEXT,
                    "Existing installation detected. Click Install to update or Uninstall.",
                );
            }
            1
        }

        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as i32;
            match id {
                IDC_BROWSE_BTN => {
                    let mut display = [0u16; 260];
                    let title = to_wide("Select installation folder:");
                    let mut bi = BROWSEINFOW {
                        hwndOwner: hwnd,
                        pidlRoot: 0,
                        pszDisplayName: display.as_mut_ptr(),
                        lpszTitle: title.as_ptr(),
                        ulFlags: 0x0040 | 0x0001,
                        lpfn: 0,
                        lParam: 0,
                        iImage: 0,
                    };
                    let pidl = SHBrowseForFolderW(&mut bi);
                    if pidl != 0 {
                        let mut path_buf = [0u16; 512];
                        if SHGetPathFromIDListW(pidl, path_buf.as_mut_ptr()) != FALSE {
                            let end = path_buf.iter().position(|&c| c == 0).unwrap_or(0);
                            let chosen = String::from_utf16_lossy(&path_buf[..end]);
                            let target = PathBuf::from(chosen).join(APP_NAME);
                            set_text(hwnd, IDC_PATH_EDIT, &target.to_string_lossy());
                        }
                        CoTaskMemFree(pidl);
                    }
                    1
                }

                IDC_INSTALL_BTN | IDOK => {
                    let dir_str = get_text(hwnd, IDC_PATH_EDIT);
                    let dir = PathBuf::from(dir_str.trim());
                    set_text(hwnd, IDC_STATUS_TEXT, "Installing RAM Reduct...");
                    let start_menu = get_chk(hwnd, IDC_STARTMENU_CHK);
                    let desktop = get_chk(hwnd, IDC_DESKTOP_CHK);
                    let autorun = get_chk(hwnd, IDC_AUTORUN_CHK);
                    let launch = get_chk(hwnd, IDC_LAUNCH_CHK);

                    match do_install(&dir, start_menu, desktop, autorun, launch) {
                        Ok(()) => {
                            set_text(hwnd, IDC_STATUS_TEXT, "Installation completed!");
                            MessageBoxW(
                                hwnd,
                                to_wide(
                                    "RAM Reduct has been installed and registered with Windows.",
                                )
                                .as_ptr(),
                                to_wide(APP_NAME).as_ptr(),
                                MB_OK | MB_ICONINFORMATION,
                            );
                            EndDialog(hwnd, 0);
                        }
                        Err(err) => {
                            set_text(hwnd, IDC_STATUS_TEXT, "Installation failed.");
                            MessageBoxW(
                                hwnd,
                                to_wide(&err).as_ptr(),
                                to_wide(APP_NAME).as_ptr(),
                                MB_OK | MB_ICONSTOP,
                            );
                        }
                    }
                    1
                }

                IDC_UNINSTALL_BTN => {
                    let confirm = MessageBoxW(
                        hwnd,
                        to_wide("Are you sure you want to uninstall RAM Reduct?").as_ptr(),
                        to_wide(APP_NAME).as_ptr(),
                        MB_YESNO | MB_ICONWARNING,
                    );
                    if confirm == IDYES {
                        let dir = read_installed_location()
                            .unwrap_or_else(|| PathBuf::from(get_text(hwnd, IDC_PATH_EDIT).trim()));
                        do_uninstall(&dir);
                        MessageBoxW(
                            hwnd,
                            to_wide("RAM Reduct has been uninstalled.").as_ptr(),
                            to_wide(APP_NAME).as_ptr(),
                            MB_OK | MB_ICONINFORMATION,
                        );
                        EndDialog(hwnd, 0);
                    }
                    1
                }

                IDC_CLOSE_BTN | IDCANCEL => {
                    EndDialog(hwnd, 0);
                    1
                }

                _ => 0,
            }
        }

        WM_CLOSE => {
            EndDialog(hwnd, 0);
            1
        }

        _ => 0,
    }
}

fn main() {
    let mut silent = false;
    let mut uninstall = false;
    let mut custom_dir: Option<PathBuf> = None;

    for arg in env::args().skip(1) {
        let lower = arg.trim().to_ascii_lowercase();
        if lower == "/s" || lower == "-s" || lower == "-silent" {
            silent = true;
        } else if lower == "/uninstall" || lower == "-uninstall" {
            uninstall = true;
        } else if let Some(rest) = arg.strip_prefix("/D=").or_else(|| arg.strip_prefix("/d=")) {
            let trimmed = rest.trim().trim_matches('"');
            if !trimmed.is_empty() {
                custom_dir = Some(PathBuf::from(trimmed));
            }
        }
    }

    if uninstall {
        let dir = custom_dir
            .or_else(read_installed_location)
            .unwrap_or_else(default_install_dir);
        if silent {
            do_uninstall(&dir);
            return;
        }
        let confirm = unsafe {
            MessageBoxW(
                0,
                to_wide("Uninstall RAM Reduct from this computer?").as_ptr(),
                to_wide(APP_NAME).as_ptr(),
                MB_YESNO | MB_ICONWARNING,
            )
        };
        if confirm == IDYES {
            do_uninstall(&dir);
            unsafe {
                MessageBoxW(
                    0,
                    to_wide("RAM Reduct has been uninstalled.").as_ptr(),
                    to_wide(APP_NAME).as_ptr(),
                    MB_OK | MB_ICONINFORMATION,
                );
            }
        }
        return;
    }

    if silent {
        let dir = custom_dir
            .or_else(read_installed_location)
            .unwrap_or_else(default_install_dir);
        if do_install(&dir, true, false, true, true).is_err() {
            std::process::exit(1);
        }
        return;
    }

    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        DialogBoxParamW(
            hinst,
            make_int_resource(IDD_INSTALLER),
            0,
            Some(installer_dlg_proc),
            0,
        );
    }
}
