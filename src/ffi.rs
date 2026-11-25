#![allow(non_snake_case, non_camel_case_types, dead_code, clippy::upper_case_acronyms)]

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::time::{SystemTime, UNIX_EPOCH};

pub type BOOL = i32;
pub type NTSTATUS = i32;
pub type HANDLE = isize;
pub type HWND = isize;
pub type HINSTANCE = isize;
pub type HMENU = isize;
pub type HICON = isize;
pub type HCURSOR = isize;
pub type HDC = isize;
pub type HBITMAP = isize;
pub type HFONT = isize;
pub type HBRUSH = isize;
pub type HGDIOBJ = isize;
pub type HACCEL = isize;
pub type HKEY = isize;
pub type SERVICE_STATUS_HANDLE = isize;
pub type WPARAM = usize;
pub type LPARAM = isize;
pub type LRESULT = isize;
pub type COLORREF = u32;

pub type DLGPROC = Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> isize>;
pub type TIMERPROC = Option<unsafe extern "system" fn(HWND, u32, usize, u32)>;
pub type LPSERVICE_MAIN_FUNCTIONW = Option<unsafe extern "system" fn(u32, *mut *mut u16)>;
pub type LPHANDLER_FUNCTION_EX =
    Option<unsafe extern "system" fn(u32, u32, *mut core::ffi::c_void, *mut core::ffi::c_void) -> u32>;

pub const FALSE: BOOL = 0;
pub const TRUE: BOOL = 1;
pub const INVALID_HANDLE_VALUE: HANDLE = -1;

// Privilege constants for RtlAdjustPrivilege
pub const SE_INCREASE_QUOTA_PRIVILEGE: u32 = 5;
pub const SE_PROF_SINGLE_PROCESS_PRIVILEGE: u32 = 13;
pub const SE_DEBUG_PRIVILEGE: u32 = 20;

// NT SystemInformationClass ordinals
pub const SYSTEM_MEMORY_LIST_INFORMATION_CLASS: u32 = 80;
pub const SYSTEM_FILECACHE_INFORMATION_EX_CLASS: u32 = 81;
pub const SYSTEM_COMBINE_PHYSICAL_MEMORY_INFORMATION_CLASS: u32 = 130;
pub const SYSTEM_REGISTRY_RECONCILIATION_INFORMATION_CLASS: u32 = 155;

// SYSTEM_MEMORY_LIST_COMMAND
pub const MEMORY_EMPTY_WORKING_SETS: u32 = 2;
pub const MEMORY_FLUSH_MODIFIED_LIST: u32 = 3;
pub const MEMORY_PURGE_STANDBY_LIST: u32 = 4;
pub const MEMORY_PURGE_LOW_PRIORITY_STANDBY_LIST: u32 = 5;

// Process / Toolhelp constants
pub const TH32CS_SNAPPROCESS: u32 = 0x00000002;
pub const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
pub const PROCESS_SET_QUOTA: u32 = 0x0100;

// File / Volume constants
pub const GENERIC_READ: u32 = 0x80000000;
pub const GENERIC_WRITE: u32 = 0x40000000;
pub const FILE_SHARE_READ: u32 = 0x00000001;
pub const FILE_SHARE_WRITE: u32 = 0x00000002;
pub const OPEN_EXISTING: u32 = 3;
pub const DRIVE_FIXED: u32 = 3;
pub const DRIVE_REMOVABLE: u32 = 2;

// Window Messages
pub const WM_DESTROY: u32 = 0x0002;
pub const WM_MOVE: u32 = 0x0003;
pub const WM_SIZE: u32 = 0x0005;
pub const WM_SETFOCUS: u32 = 0x0007;
pub const WM_PAINT: u32 = 0x000F;
pub const WM_CLOSE: u32 = 0x0010;
pub const WM_QUERYENDSESSION: u32 = 0x0011;
pub const WM_ENDSESSION: u32 = 0x0016;
pub const WM_QUIT: u32 = 0x0012;
pub const WM_SHOWWINDOW: u32 = 0x0018;
pub const WM_NOTIFY: u32 = 0x004E;
pub const WM_INITDIALOG: u32 = 0x0110;
pub const WM_COMMAND: u32 = 0x0111;
pub const WM_SYSCOMMAND: u32 = 0x0112;
pub const WM_TIMER: u32 = 0x0113;
pub const WM_HSCROLL: u32 = 0x0114;
pub const WM_VSCROLL: u32 = 0x0115;
pub const WM_CONTEXTMENU: u32 = 0x007B;
pub const WM_SETICON: u32 = 0x0080;
pub const WM_DPICHANGED: u32 = 0x02E0;
pub const WM_HOTKEY: u32 = 0x0312;
pub const WM_LBUTTONDOWN: u32 = 0x0201;
pub const WM_LBUTTONUP: u32 = 0x0202;
pub const WM_RBUTTONUP: u32 = 0x0205;
pub const WM_MBUTTONDOWN: u32 = 0x0207;
pub const WM_MBUTTONUP: u32 = 0x0208;
pub const WM_APP: u32 = 0x8000;
pub const WM_TRAYICON: u32 = WM_APP + 10;

pub const ICON_SMALL: WPARAM = 0;
pub const ICON_BIG: WPARAM = 1;

pub const SC_MINIMIZE: WPARAM = 0xF020;
pub const SC_CLOSE: WPARAM = 0xF060;

// ShowWindow / SetWindowPos
pub const SW_HIDE: i32 = 0;
pub const SW_SHOWNORMAL: i32 = 1;
pub const SW_SHOW: i32 = 5;
pub const SW_RESTORE: i32 = 9;

pub const HWND_TOPMOST: HWND = -1;
pub const HWND_NOTOPMOST: HWND = -2;
pub const SWP_NOSIZE: u32 = 0x0001;
pub const SWP_NOMOVE: u32 = 0x0002;
pub const SWP_NOZORDER: u32 = 0x0004;
pub const SWP_NOACTIVATE: u32 = 0x0010;
pub const SWP_SHOWWINDOW: u32 = 0x0040;

pub const DWLP_MSGRESULT: i32 = 0;

// Dialog / MessageBox IDs & Flags
pub const IDOK: i32 = 1;
pub const IDCANCEL: i32 = 2;
pub const IDYES: i32 = 6;
pub const IDNO: i32 = 7;

pub const MB_OK: u32 = 0x00000000;
pub const MB_YESNO: u32 = 0x00000004;
pub const MB_ICONSTOP: u32 = 0x00000010;
pub const MB_ICONQUESTION: u32 = 0x00000020;
pub const MB_ICONWARNING: u32 = 0x00000030;
pub const MB_ICONINFORMATION: u32 = 0x00000040;

// Button / Checkbox / ComboBox / ListBox / Edit / UpDown / Hotkey messages
pub const BM_GETCHECK: u32 = 0x00F0;
pub const BM_SETCHECK: u32 = 0x00F1;
pub const BST_UNCHECKED: usize = 0;
pub const BST_CHECKED: usize = 1;
pub const BCM_SETSHIELD: u32 = 0x160C;
pub const BCN_DROPDOWN: i32 = -1248;

pub const CB_ADDSTRING: u32 = 0x0143;
pub const CB_RESETCONTENT: u32 = 0x014B;
pub const CB_GETCURSEL: u32 = 0x0147;
pub const CB_SETCURSEL: u32 = 0x014E;
pub const CBN_SELCHANGE: u16 = 1;

pub const LB_ADDSTRING: u32 = 0x0180;
pub const LB_RESETCONTENT: u32 = 0x0184;
pub const LB_SETCURSEL: u32 = 0x0186;
pub const LB_GETCURSEL: u32 = 0x0188;
pub const LBN_SELCHANGE: u16 = 1;

pub const EN_CHANGE: u16 = 0x0300;
pub const STM_SETICON: u32 = 0x0170;

pub const UDM_SETRANGE32: u32 = 0x046F;
pub const UDM_SETPOS32: u32 = 0x0471;
pub const UDM_GETPOS32: u32 = 0x0472;
pub const UDM_GETBUDDY: u32 = 0x046A;

pub const HKM_SETHOTKEY: u32 = 0x0401;
pub const HKM_GETHOTKEY: u32 = 0x0402;
pub const HOTKEYF_SHIFT: u8 = 0x01;
pub const HOTKEYF_CONTROL: u8 = 0x02;
pub const HOTKEYF_ALT: u8 = 0x04;
pub const MOD_ALT: u32 = 0x0001;
pub const MOD_CONTROL: u32 = 0x0002;
pub const MOD_SHIFT: u32 = 0x0004;

// ListView constants
pub const LVM_FIRST: u32 = 0x1000;
pub const LVM_DELETEALLITEMS: u32 = LVM_FIRST + 9;
pub const LVM_GETITEMCOUNT: u32 = LVM_FIRST + 4;
pub const LVM_GETITEMSTATE: u32 = LVM_FIRST + 44;
pub const LVM_GETITEMW: u32 = LVM_FIRST + 75;
pub const LVM_SETITEMW: u32 = LVM_FIRST + 76;
pub const LVM_INSERTITEMW: u32 = LVM_FIRST + 77;
pub const LVM_DELETECOLUMN: u32 = LVM_FIRST + 28;
pub const LVM_SETCOLUMNWIDTH: u32 = LVM_FIRST + 30;
pub const LVM_SETEXTENDEDLISTVIEWSTYLE: u32 = LVM_FIRST + 54;
pub const LVM_INSERTCOLUMNW: u32 = LVM_FIRST + 97;
pub const LVM_INSERTGROUP: u32 = LVM_FIRST + 145;
pub const LVM_SETGROUPINFO: u32 = LVM_FIRST + 147;
pub const LVM_ENABLEGROUPVIEW: u32 = LVM_FIRST + 157;
pub const LVM_SETBKCOLOR: u32 = LVM_FIRST + 1;
pub const LVM_SETTEXTBKCOLOR: u32 = LVM_FIRST + 38;
pub const LVM_SETTEXTCOLOR: u32 = LVM_FIRST + 36;

pub const LVS_EX_CHECKBOXES: u32 = 0x00000004;
pub const LVS_EX_FULLROWSELECT: u32 = 0x00000020;
pub const LVS_EX_INFOTIP: u32 = 0x00000400;
pub const LVS_EX_LABELTIP: u32 = 0x00004000;
pub const LVS_EX_DOUBLEBUFFER: u32 = 0x00010000;

pub const LVCF_FMT: u32 = 0x0001;
pub const LVCF_WIDTH: u32 = 0x0002;
pub const LVCF_TEXT: u32 = 0x0004;
pub const LVCFMT_LEFT: i32 = 0x0000;
pub const LVCFMT_RIGHT: i32 = 0x0001;

pub const LVIF_TEXT: u32 = 0x0001;
pub const LVIF_PARAM: u32 = 0x0004;
pub const LVIF_STATE: u32 = 0x0008;
pub const LVIF_GROUPID: u32 = 0x0100;
pub const LVIS_STATEIMAGEMASK: u32 = 0xF000;

pub const LVGF_HEADER: u32 = 0x00000001;
pub const LVGF_STATE: u32 = 0x00000004;
pub const LVGF_GROUPID: u32 = 0x00000010;
pub const LVGS_COLLAPSIBLE: u32 = 0x00000008;

pub const NM_DBLCLK: i32 = -3;
pub const NM_CUSTOMDRAW: i32 = -12;
pub const LVN_ITEMCHANGED: i32 = -101;

pub const CDDS_PREPAINT: u32 = 0x00000001;
pub const CDDS_ITEMPREPAINT: u32 = 0x00010001;
pub const CDRF_DODEFAULT: isize = 0x00000000;
pub const CDRF_NEWFONT: isize = 0x00000002;
pub const CDRF_NOTIFYITEMDRAW: isize = 0x00000020;

// Menu constants
pub const MF_BYCOMMAND: u32 = 0x00000000;
pub const MF_BYPOSITION: u32 = 0x00000400;
pub const MF_SEPARATOR: u32 = 0x00000800;
pub const MF_ENABLED: u32 = 0x00000000;
pub const MF_GRAYED: u32 = 0x00000001;
pub const MF_DISABLED: u32 = 0x00000002;
pub const MF_UNCHECKED: u32 = 0x00000000;
pub const MF_CHECKED: u32 = 0x00000008;
pub const MF_STRING: u32 = 0x00000000;
pub const MIIM_STRING: u32 = 0x00000040;
pub const TPM_RIGHTBUTTON: u32 = 0x0002;

// GDI / Font / SystemMetrics
pub const SM_CXSCREEN: i32 = 0;
pub const SM_CYSCREEN: i32 = 1;
pub const SM_CXSMICON: i32 = 49;
pub const SM_CYSMICON: i32 = 50;
pub const LOGPIXELSY: i32 = 90;
pub const TRANSPARENT: i32 = 1;
pub const DC_BRUSH: i32 = 18;
pub const DC_PEN: i32 = 19;
pub const DEFAULT_CHARSET: u8 = 1;
pub const NONANTIALIASED_QUALITY: u8 = 3;
pub const CLEARTYPE_QUALITY: u8 = 5;
pub const FW_NORMAL: i32 = 400;
pub const DT_CENTER: u32 = 0x0001;
pub const DT_VCENTER: u32 = 0x0004;
pub const DT_SINGLELINE: u32 = 0x0020;
pub const DT_NOCLIP: u32 = 0x0100;
pub const DT_NOPREFIX: u32 = 0x0800;
pub const BI_RGB: u32 = 0;
pub const DIB_RGB_COLORS: u32 = 0;

pub const IDC_ARROW_ID: u16 = 32512;
pub const IDC_WAIT_ID: u16 = 32514;

// ChooseColor / ChooseFont
pub const CC_RGBINIT: u32 = 0x00000001;
pub const CC_FULLOPEN: u32 = 0x00000002;
pub const CF_SCREENFONTS: u32 = 0x00000001;
pub const CF_INITTOLOGFONTSTRUCT: u32 = 0x00000040;
pub const CF_FORCEFONTEXIST: u32 = 0x00010000;

// Shell / Tray constants
pub const NIM_ADD: u32 = 0x00000000;
pub const NIM_MODIFY: u32 = 0x00000001;
pub const NIM_DELETE: u32 = 0x00000002;
pub const NIM_SETVERSION: u32 = 0x00000004;
pub const NIF_MESSAGE: u32 = 0x00000001;
pub const NIF_ICON: u32 = 0x00000002;
pub const NIF_TIP: u32 = 0x00000004;
pub const NIF_INFO: u32 = 0x00000010;
pub const NIF_GUID: u32 = 0x00000020;
pub const NIF_SHOWTIP: u32 = 0x00000080;
pub const NOTIFYICON_VERSION_4: u32 = 4;
pub const NIIF_INFO: u32 = 0x00000001;
pub const NIIF_WARNING: u32 = 0x00000002;
pub const NIIF_NOSOUND: u32 = 0x00000010;
pub const NIN_KEYSELECT: u32 = 0x0401;

// QUERY_USER_NOTIFICATION_STATE
pub const QUNS_BUSY: i32 = 2;
pub const QUNS_RUNNING_D3D_FULL_SCREEN: i32 = 3;
pub const QUNS_PRESENTATION_MODE: i32 = 4;

// Registry constants
pub const HKEY_CURRENT_USER: HKEY = -2147483647i32 as isize; // 0x80000001
pub const HKEY_LOCAL_MACHINE: HKEY = -2147483646i32 as isize; // 0x80000002
pub const KEY_READ: u32 = 0x20019;
pub const KEY_WRITE: u32 = 0x20006;
pub const REG_SZ: u32 = 1;
pub const REG_BINARY: u32 = 3;
pub const REG_DWORD: u32 = 4;

// Windows Service constants
pub const SERVICE_WIN32_OWN_PROCESS: u32 = 0x00000010;
pub const SERVICE_STOPPED: u32 = 0x00000001;
pub const SERVICE_START_PENDING: u32 = 0x00000002;
pub const SERVICE_STOP_PENDING: u32 = 0x00000003;
pub const SERVICE_RUNNING: u32 = 0x00000004;
pub const SERVICE_ACCEPT_STOP: u32 = 0x00000001;
pub const SERVICE_ACCEPT_SHUTDOWN: u32 = 0x00000004;
pub const SERVICE_CONTROL_STOP: u32 = 0x00000001;
pub const SERVICE_CONTROL_INTERROGATE: u32 = 0x00000004;
pub const SERVICE_CONTROL_SHUTDOWN: u32 = 0x00000005;
pub const NO_ERROR: u32 = 0;
pub const ERROR_CALL_NOT_IMPLEMENTED: u32 = 120;
pub const WAIT_OBJECT_0: u32 = 0;

// DWM Dark Mode
pub const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;

// Common Controls classes
pub const ICC_LISTVIEW_CLASSES: u32 = 0x00000001;
pub const ICC_UPDOWN_CLASS: u32 = 0x00000010;
pub const ICC_HOTKEY_CLASS: u32 = 0x00000040;
pub const ICC_STANDARD_CLASSES: u32 = 0x00004000;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct GUID {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

pub const GUID_TRAY_ICON: GUID = GUID {
    data1: 0xAE9053F0,
    data2: 0x8D59,
    data3: 0x4803,
    data4: [0x9A, 0xBB, 0x74, 0xAF, 0xE6, 0x6B, 0x5F, 0xD2],
};

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct POINT {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: u32,
    pub wParam: WPARAM,
    pub lParam: LPARAM,
    pub time: u32,
    pub pt: POINT,
}

#[repr(C)]
pub struct INITCOMMONCONTROLSEX {
    pub dwSize: u32,
    pub dwICC: u32,
}

#[repr(C)]
pub struct SERVICE_TABLE_ENTRYW {
    pub lpServiceName: *mut u16,
    pub lpServiceProc: LPSERVICE_MAIN_FUNCTIONW,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SERVICE_STATUS {
    pub dwServiceType: u32,
    pub dwCurrentState: u32,
    pub dwControlsAccepted: u32,
    pub dwWin32ExitCode: u32,
    pub dwServiceSpecificExitCode: u32,
    pub dwCheckPoint: u32,
    pub dwWaitHint: u32,
}

#[repr(C)]
pub struct MEMORYSTATUSEX {
    pub dwLength: u32,
    pub dwMemoryLoad: u32,
    pub ullTotalPhys: u64,
    pub ullAvailPhys: u64,
    pub ullTotalPageFile: u64,
    pub ullAvailPageFile: u64,
    pub ullTotalVirtual: u64,
    pub ullAvailVirtual: u64,
    pub ullAvailExtendedVirtual: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SYSTEM_FILECACHE_INFORMATION {
    pub CurrentSize: usize,
    pub PeakSize: usize,
    pub PageFaultCount: u32,
    pub MinimumWorkingSet: usize,
    pub MaximumWorkingSet: usize,
    pub CurrentSizeIncludingTransitionInPages: usize,
    pub PeakSizeIncludingTransitionInPages: usize,
    pub TransitionRepurposeCount: u32,
    pub Flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SYSTEM_MEMORY_LIST_INFORMATION {
    pub ZeroPageCount: usize,
    pub FreePageCount: usize,
    pub ModifiedPageCount: usize,
    pub ModifiedNoWritePageCount: usize,
    pub BadPageCount: usize,
    pub PageCountByPriority: [usize; 8],
    pub RepurposedPagesByPriority: [usize; 8],
    pub ModifiedPageCountPageFile: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct MEMORY_COMBINE_INFORMATION_EX {
    pub Handle: HANDLE,
    pub PagesCombined: usize,
    pub Flags: u32,
}

#[repr(C)]
pub struct RTL_OSVERSIONINFOW {
    pub dwOSVersionInfoSize: u32,
    pub dwMajorVersion: u32,
    pub dwMinorVersion: u32,
    pub dwBuildNumber: u32,
    pub dwPlatformId: u32,
    pub szCSDVersion: [u16; 128],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TOKEN_ELEVATION {
    pub TokenIsElevated: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PROCESSENTRY32W {
    pub dwSize: u32,
    pub cntUsage: u32,
    pub th32ProcessID: u32,
    pub th32DefaultHeapID: usize,
    pub th32ModuleID: u32,
    pub cntThreads: u32,
    pub th32ParentProcessID: u32,
    pub pcPriClassBase: i32,
    pub dwFlags: u32,
    pub szExeFile: [u16; 260],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LOGFONTW {
    pub lfHeight: i32,
    pub lfWidth: i32,
    pub lfEscapement: i32,
    pub lfOrientation: i32,
    pub lfWeight: i32,
    pub lfItalic: u8,
    pub lfUnderline: u8,
    pub lfStrikeOut: u8,
    pub lfCharSet: u8,
    pub lfOutPrecision: u8,
    pub lfClipPrecision: u8,
    pub lfQuality: u8,
    pub lfPitchAndFamily: u8,
    pub lfFaceName: [u16; 32],
}

impl Default for LOGFONTW {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
pub struct ICONINFO {
    pub fIcon: BOOL,
    pub xHotspot: u32,
    pub yHotspot: u32,
    pub hbmMask: HBITMAP,
    pub hbmColor: HBITMAP,
}

#[repr(C)]
pub struct BITMAPINFOHEADER {
    pub biSize: u32,
    pub biWidth: i32,
    pub biHeight: i32,
    pub biPlanes: u16,
    pub biBitCount: u16,
    pub biCompression: u32,
    pub biSizeImage: u32,
    pub biXPelsPerMeter: i32,
    pub biYPelsPerMeter: i32,
    pub biClrUsed: u32,
    pub biClrImportant: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RGBQUAD {
    pub rgbBlue: u8,
    pub rgbGreen: u8,
    pub rgbRed: u8,
    pub rgbReserved: u8,
}

#[repr(C)]
pub struct BITMAPINFO {
    pub bmiHeader: BITMAPINFOHEADER,
    pub bmiColors: [RGBQUAD; 1],
}

#[repr(C)]
pub struct CHOOSECOLORW {
    pub lStructSize: u32,
    pub hwndOwner: HWND,
    pub hInstance: HWND,
    pub rgbResult: COLORREF,
    pub lpCustColors: *mut COLORREF,
    pub Flags: u32,
    pub lCustData: LPARAM,
    pub lpfnHook: usize,
    pub lpTemplateName: *const u16,
}

#[repr(C)]
pub struct CHOOSEFONTW {
    pub lStructSize: u32,
    pub hwndOwner: HWND,
    pub hDC: HDC,
    pub lpLogFont: *mut LOGFONTW,
    pub iPointSize: i32,
    pub Flags: u32,
    pub rgbColors: COLORREF,
    pub lCustData: LPARAM,
    pub lpfnHook: usize,
    pub lpTemplateName: *const u16,
    pub hInstance: HINSTANCE,
    pub lpszStyle: *mut u16,
    pub nFontType: u16,
    pub ___MISSING_ALIGNMENT__: u16,
    pub nSizeMin: i32,
    pub nSizeMax: i32,
}

#[repr(C)]
pub struct NOTIFYICONDATAW {
    pub cbSize: u32,
    pub hWnd: HWND,
    pub uID: u32,
    pub uFlags: u32,
    pub uCallbackMessage: u32,
    pub hIcon: HICON,
    pub szTip: [u16; 128],
    pub dwState: u32,
    pub dwStateMask: u32,
    pub szInfo: [u16; 256],
    pub uTimeoutOrVersion: u32,
    pub szInfoTitle: [u16; 64],
    pub dwInfoFlags: u32,
    pub guidItem: GUID,
    pub hBalloonIcon: HICON,
}

#[repr(C)]
pub struct MENUITEMINFOW {
    pub cbSize: u32,
    pub fMask: u32,
    pub fType: u32,
    pub fState: u32,
    pub wID: u32,
    pub hSubMenu: HMENU,
    pub hbmpChecked: HBITMAP,
    pub hbmpUnchecked: HBITMAP,
    pub dwItemData: usize,
    pub dwTypeData: *mut u16,
    pub cch: u32,
    pub hbmpItem: HBITMAP,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NMHDR {
    pub hwndFrom: HWND,
    pub idFrom: usize,
    pub code: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NMCUSTOMDRAW {
    pub hdr: NMHDR,
    pub dwDrawStage: u32,
    pub hdc: HDC,
    pub rc: RECT,
    pub dwItemSpec: usize,
    pub uItemState: u32,
    pub lItemlParam: LPARAM,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NMLVCUSTOMDRAW {
    pub nmcd: NMCUSTOMDRAW,
    pub clrText: COLORREF,
    pub clrTextBk: COLORREF,
    pub iSubItem: i32,
    pub dwItemType: u32,
    pub clrFace: COLORREF,
    pub iIconEffect: i32,
    pub iIconPhase: i32,
    pub iPartId: i32,
    pub iStateId: i32,
    pub rcText: RECT,
    pub uAlign: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NMITEMACTIVATE {
    pub hdr: NMHDR,
    pub iItem: i32,
    pub iSubItem: i32,
    pub uNewState: u32,
    pub uOldState: u32,
    pub uChanged: u32,
    pub ptAction: POINT,
    pub lParam: LPARAM,
    pub uKeyFlags: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NMLISTVIEW {
    pub hdr: NMHDR,
    pub iItem: i32,
    pub iSubItem: i32,
    pub uNewState: u32,
    pub uOldState: u32,
    pub uChanged: u32,
    pub ptAction: POINT,
    pub lParam: LPARAM,
}

#[repr(C)]
pub struct LVCOLUMNW {
    pub mask: u32,
    pub fmt: i32,
    pub cx: i32,
    pub pszText: *mut u16,
    pub cchTextMax: i32,
    pub iSubItem: i32,
    pub iImage: i32,
    pub iOrder: i32,
    pub cxMin: i32,
    pub cxDefault: i32,
    pub cxIdeal: i32,
}

#[repr(C)]
pub struct LVITEMW {
    pub mask: u32,
    pub iItem: i32,
    pub iSubItem: i32,
    pub state: u32,
    pub stateMask: u32,
    pub pszText: *mut u16,
    pub cchTextMax: i32,
    pub iImage: i32,
    pub lParam: LPARAM,
    pub iIndent: i32,
    pub iGroupId: i32,
    pub cColumns: u32,
    pub puColumns: *mut u32,
    pub piColFmt: *mut i32,
    pub iGroup: i32,
}

#[repr(C)]
pub struct LVGROUP {
    pub cbSize: u32,
    pub mask: u32,
    pub pszHeader: *mut u16,
    pub cchHeader: i32,
    pub pszFooter: *mut u16,
    pub cchFooter: i32,
    pub iGroupId: i32,
    pub stateMask: u32,
    pub state: u32,
    pub uAlign: u32,
    pub pszSubtitle: *mut u16,
    pub cchSubtitle: u32,
    pub pszTask: *mut u16,
    pub cchTask: u32,
    pub pszDescriptionTop: *mut u16,
    pub cchDescriptionTop: u32,
    pub pszDescriptionBottom: *mut u16,
    pub cchDescriptionBottom: u32,
    pub iTitleImage: i32,
    pub iExtendedImage: i32,
    pub iFirstItem: i32,
    pub cItems: u32,
    pub pszSubsetTitle: *mut u16,
    pub cchSubsetTitle: u32,
}

#[link(name = "ntdll")]
extern "system" {
    pub fn NtQuerySystemInformation(
        SystemInformationClass: u32,
        SystemInformation: *mut core::ffi::c_void,
        SystemInformationLength: u32,
        ReturnLength: *mut u32,
    ) -> NTSTATUS;

    pub fn NtSetSystemInformation(
        SystemInformationClass: u32,
        SystemInformation: *const core::ffi::c_void,
        SystemInformationLength: u32,
    ) -> NTSTATUS;

    pub fn RtlAdjustPrivilege(
        Privilege: u32,
        Enable: u8,
        CurrentThread: u8,
        Enabled: *mut u8,
    ) -> NTSTATUS;

    pub fn RtlGetVersion(lpVersionInformation: *mut RTL_OSVERSIONINFOW) -> NTSTATUS;
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleW(lpModuleName: *const u16) -> HINSTANCE;
    pub fn GetCurrentProcess() -> HANDLE;
    pub fn GetCurrentProcessId() -> u32;
    pub fn CloseHandle(hObject: HANDLE) -> BOOL;
    pub fn GlobalMemoryStatusEx(lpBuffer: *mut MEMORYSTATUSEX) -> BOOL;
    pub fn CreateMutexW(
        lpMutexAttributes: *const core::ffi::c_void,
        bInitialOwner: BOOL,
        lpName: *const u16,
    ) -> HANDLE;
    pub fn CreateEventW(
        lpEventAttributes: *const core::ffi::c_void,
        bManualReset: BOOL,
        bInitialState: BOOL,
        lpName: *const u16,
    ) -> HANDLE;
    pub fn SetEvent(hEvent: HANDLE) -> BOOL;
    pub fn WaitForSingleObject(hHandle: HANDLE, dwMilliseconds: u32) -> u32;
    pub fn GetLastError() -> u32;

    pub fn CreateToolhelp32Snapshot(dwFlags: u32, th32ProcessID: u32) -> HANDLE;
    pub fn Process32FirstW(hSnapshot: HANDLE, lppe: *mut PROCESSENTRY32W) -> BOOL;
    pub fn Process32NextW(hSnapshot: HANDLE, lppe: *mut PROCESSENTRY32W) -> BOOL;
    pub fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: BOOL, dwProcessId: u32) -> HANDLE;
    pub fn K32EmptyWorkingSet(hProcess: HANDLE) -> BOOL;

    pub fn FindFirstVolumeW(lpszVolumeName: *mut u16, cchBufferLength: u32) -> HANDLE;
    pub fn FindNextVolumeW(
        hFindVolume: HANDLE,
        lpszVolumeName: *mut u16,
        cchBufferLength: u32,
    ) -> BOOL;
    pub fn FindVolumeClose(hFindVolume: HANDLE) -> BOOL;
    pub fn GetDriveTypeW(lpRootPathName: *const u16) -> u32;
    pub fn CreateFileW(
        lpFileName: *const u16,
        dwDesiredAccess: u32,
        dwShareMode: u32,
        lpSecurityAttributes: *const core::ffi::c_void,
        dwCreationDisposition: u32,
        dwFlagsAndAttributes: u32,
        hTemplateFile: HANDLE,
    ) -> HANDLE;
    pub fn FlushFileBuffers(hFile: HANDLE) -> BOOL;

    pub fn MulDiv(nNumber: i32, nNumerator: i32, nDenominator: i32) -> i32;
}

#[link(name = "advapi32")]
extern "system" {
    pub fn OpenProcessToken(
        ProcessHandle: HANDLE,
        DesiredAccess: u32,
        TokenHandle: *mut HANDLE,
    ) -> BOOL;
    pub fn GetTokenInformation(
        TokenHandle: HANDLE,
        TokenInformationClass: u32,
        TokenInformation: *mut core::ffi::c_void,
        TokenInformationLength: u32,
        ReturnLength: *mut u32,
    ) -> BOOL;

    pub fn RegCreateKeyExW(
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
    pub fn RegOpenKeyExW(
        hKey: HKEY,
        lpSubKey: *const u16,
        ulOptions: u32,
        samDesired: u32,
        phkResult: *mut HKEY,
    ) -> i32;
    pub fn RegQueryValueExW(
        hKey: HKEY,
        lpValueName: *const u16,
        lpReserved: *mut u32,
        lpType: *mut u32,
        lpData: *mut u8,
        lpcbData: *mut u32,
    ) -> i32;
    pub fn RegSetValueExW(
        hKey: HKEY,
        lpValueName: *const u16,
        Reserved: u32,
        dwType: u32,
        lpData: *const u8,
        cbData: u32,
    ) -> i32;
    pub fn RegDeleteValueW(hKey: HKEY, lpValueName: *const u16) -> i32;
    pub fn RegDeleteKeyW(hKey: HKEY, lpSubKey: *const u16) -> i32;
    pub fn RegCloseKey(hKey: HKEY) -> i32;

    pub fn StartServiceCtrlDispatcherW(lpServiceStartTable: *const SERVICE_TABLE_ENTRYW) -> BOOL;
    pub fn RegisterServiceCtrlHandlerExW(
        lpServiceName: *const u16,
        lpHandlerProc: LPHANDLER_FUNCTION_EX,
        lpContext: *mut core::ffi::c_void,
    ) -> SERVICE_STATUS_HANDLE;
    pub fn SetServiceStatus(
        hServiceStatus: SERVICE_STATUS_HANDLE,
        lpServiceStatus: *mut SERVICE_STATUS,
    ) -> BOOL;
}

#[link(name = "user32")]
extern "system" {
    pub fn CreateDialogParamW(
        hInstance: HINSTANCE,
        lpTemplateName: *const u16,
        hWndParent: HWND,
        lpDialogFunc: DLGPROC,
        dwInitParam: LPARAM,
    ) -> HWND;
    pub fn DialogBoxParamW(
        hInstance: HINSTANCE,
        lpTemplateName: *const u16,
        hWndParent: HWND,
        lpDialogFunc: DLGPROC,
        dwInitParam: LPARAM,
    ) -> isize;
    pub fn EndDialog(hDlg: HWND, nResult: isize) -> BOOL;
    pub fn DestroyWindow(hWnd: HWND) -> BOOL;
    pub fn ShowWindow(hWnd: HWND, nCmdShow: i32) -> BOOL;
    pub fn IsWindowVisible(hWnd: HWND) -> BOOL;
    pub fn IsIconic(hWnd: HWND) -> BOOL;
    pub fn SetForegroundWindow(hWnd: HWND) -> BOOL;
    pub fn GetForegroundWindow() -> HWND;
    pub fn SetWindowPos(
        hWnd: HWND,
        hWndInsertAfter: HWND,
        X: i32,
        Y: i32,
        cx: i32,
        cy: i32,
        uFlags: u32,
    ) -> BOOL;
    pub fn FindWindowW(lpClassName: *const u16, lpWindowName: *const u16) -> HWND;

    pub fn GetMessageW(lpMsg: *mut MSG, hWnd: HWND, wMsgFilterMin: u32, wMsgFilterMax: u32) -> BOOL;
    pub fn TranslateMessage(lpMsg: *const MSG) -> BOOL;
    pub fn DispatchMessageW(lpMsg: *const MSG) -> LRESULT;
    pub fn IsDialogMessageW(hDlg: HWND, lpMsg: *mut MSG) -> BOOL;
    pub fn LoadAcceleratorsW(hInstance: HINSTANCE, lpTableName: *const u16) -> HACCEL;
    pub fn TranslateAcceleratorW(hWnd: HWND, hAccTable: HACCEL, lpMsg: *mut MSG) -> i32;
    pub fn PostQuitMessage(nExitCode: i32);

    pub fn SendMessageW(hWnd: HWND, Msg: u32, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub fn SendDlgItemMessageW(
        hDlg: HWND,
        nIDDlgItem: i32,
        Msg: u32,
        wParam: WPARAM,
        lParam: LPARAM,
    ) -> LRESULT;
    pub fn GetDlgItem(hDlg: HWND, nIDDlgItem: i32) -> HWND;
    pub fn GetDlgCtrlID(hWnd: HWND) -> i32;
    pub fn SetDlgItemTextW(hDlg: HWND, nIDDlgItem: i32, lpString: *const u16) -> BOOL;
    pub fn GetDlgItemTextW(
        hDlg: HWND,
        nIDDlgItem: i32,
        lpString: *mut u16,
        cchMax: i32,
    ) -> u32;
    pub fn SetWindowTextW(hWnd: HWND, lpString: *const u16) -> BOOL;
    pub fn EnableWindow(hWnd: HWND, bEnable: BOOL) -> BOOL;
    pub fn InvalidateRect(hWnd: HWND, lpRect: *const RECT, bErase: BOOL) -> BOOL;
    pub fn GetClientRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    pub fn GetWindowRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    pub fn GetCursorPos(lpPoint: *mut POINT) -> BOOL;

    pub fn SetWindowLongPtrW(hWnd: HWND, nIndex: i32, dwNewLong: isize) -> isize;
    pub fn GetWindowLongPtrW(hWnd: HWND, nIndex: i32) -> isize;

    pub fn SetTimer(
        hWnd: HWND,
        nIDEvent: usize,
        uElapse: u32,
        lpTimerFunc: TIMERPROC,
    ) -> usize;
    pub fn KillTimer(hWnd: HWND, uIDEvent: usize) -> BOOL;
    pub fn RegisterHotKey(hWnd: HWND, id: i32, fsModifiers: u32, vk: u32) -> BOOL;
    pub fn UnregisterHotKey(hWnd: HWND, id: i32) -> BOOL;
    pub fn RegisterWindowMessageW(lpString: *const u16) -> u32;

    pub fn GetMenu(hWnd: HWND) -> HMENU;
    pub fn GetSubMenu(hMenu: HMENU, nPos: i32) -> HMENU;
    pub fn LoadMenuW(hInstance: HINSTANCE, lpMenuName: *const u16) -> HMENU;
    pub fn CreatePopupMenu() -> HMENU;
    pub fn DestroyMenu(hMenu: HMENU) -> BOOL;
    pub fn AppendMenuW(hMenu: HMENU, uFlags: u32, uIDNewItem: usize, lpNewItem: *const u16) -> BOOL;
    pub fn DeleteMenu(hMenu: HMENU, uPosition: u32, uFlags: u32) -> BOOL;
    pub fn GetMenuItemCount(hMenu: HMENU) -> i32;
    pub fn CheckMenuItem(hMenu: HMENU, uIDCheckItem: u32, uCheck: u32) -> u32;
    pub fn CheckMenuRadioItem(
        hmenu: HMENU,
        first: u32,
        last: u32,
        check: u32,
        flags: u32,
    ) -> BOOL;
    pub fn EnableMenuItem(hMenu: HMENU, uIDEnableItem: u32, uEnable: u32) -> BOOL;
    pub fn SetMenuItemInfoW(hmenu: HMENU, item: u32, fByPositon: BOOL, lpmii: *const MENUITEMINFOW) -> BOOL;
    pub fn TrackPopupMenuEx(
        hMenu: HMENU,
        uFlags: u32,
        x: i32,
        y: i32,
        hwnd: HWND,
        lptpm: *const core::ffi::c_void,
    ) -> BOOL;

    pub fn MessageBoxW(hWnd: HWND, lpText: *const u16, lpCaption: *const u16, uType: u32) -> i32;
    pub fn LoadIconW(hInstance: HINSTANCE, lpIconName: *const u16) -> HICON;
    pub fn LoadCursorW(hInstance: HINSTANCE, lpCursorName: *const u16) -> HCURSOR;
    pub fn SetCursor(hCursor: HCURSOR) -> HCURSOR;
    pub fn CreateIconIndirect(piconinfo: *const ICONINFO) -> HICON;
    pub fn DestroyIcon(hIcon: HICON) -> BOOL;
    pub fn FillRect(hDC: HDC, lprc: *const RECT, hbr: HBRUSH) -> i32;
    pub fn DrawTextW(
        hdc: HDC,
        lpchText: *const u16,
        cchText: i32,
        lprc: *mut RECT,
        format: u32,
    ) -> i32;
    pub fn GetDC(hWnd: HWND) -> HDC;
    pub fn ReleaseDC(hWnd: HWND, hDC: HDC) -> i32;
    pub fn GetSystemMetrics(nIndex: i32) -> i32;
    pub fn GetDpiForWindow(hwnd: HWND) -> u32;
}

#[link(name = "gdi32")]
extern "system" {
    pub fn CreateCompatibleDC(hdc: HDC) -> HDC;
    pub fn DeleteDC(hdc: HDC) -> BOOL;
    pub fn SelectObject(hdc: HDC, h: HGDIOBJ) -> HGDIOBJ;
    pub fn DeleteObject(ho: HGDIOBJ) -> BOOL;
    pub fn GetStockObject(i: i32) -> HGDIOBJ;
    pub fn CreateSolidBrush(color: COLORREF) -> HBRUSH;
    pub fn SetBkMode(hdc: HDC, mode: i32) -> i32;
    pub fn SetBkColor(hdc: HDC, color: COLORREF) -> COLORREF;
    pub fn SetTextColor(hdc: HDC, color: COLORREF) -> COLORREF;
    pub fn SetDCPenColor(hdc: HDC, color: COLORREF) -> COLORREF;
    pub fn SetDCBrushColor(hdc: HDC, color: COLORREF) -> COLORREF;
    pub fn Rectangle(hdc: HDC, left: i32, top: i32, right: i32, bottom: i32) -> BOOL;
    pub fn RoundRect(
        hdc: HDC,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        width: i32,
        height: i32,
    ) -> BOOL;
    pub fn CreateFontIndirectW(lplf: *const LOGFONTW) -> HFONT;
    pub fn CreateDIBSection(
        hdc: HDC,
        pbmi: *const BITMAPINFO,
        usage: u32,
        ppvBits: *mut *mut core::ffi::c_void,
        hSection: HANDLE,
        offset: u32,
    ) -> HBITMAP;
    pub fn CreateBitmap(
        nWidth: i32,
        nHeight: i32,
        nPlanes: u32,
        nBitCount: u32,
        lpBits: *const core::ffi::c_void,
    ) -> HBITMAP;
    pub fn GetDeviceCaps(hdc: HDC, index: i32) -> i32;
}

#[link(name = "shell32")]
extern "system" {
    pub fn Shell_NotifyIconW(dwMessage: u32, lpData: *mut NOTIFYICONDATAW) -> BOOL;
    pub fn ShellExecuteW(
        hwnd: HWND,
        lpOperation: *const u16,
        lpFile: *const u16,
        lpParameters: *const u16,
        lpDirectory: *const u16,
        nShowCmd: i32,
    ) -> HINSTANCE;
    pub fn SHQueryUserNotificationState(pquns: *mut i32) -> i32;
    pub fn SetCurrentProcessExplicitAppUserModelID(AppID: *const u16) -> i32;
}

#[link(name = "comctl32")]
extern "system" {
    pub fn InitCommonControlsEx(picce: *const INITCOMMONCONTROLSEX) -> BOOL;
}

#[link(name = "comdlg32")]
extern "system" {
    pub fn ChooseColorW(lpcc: *mut CHOOSECOLORW) -> BOOL;
    pub fn ChooseFontW(lpcf: *mut CHOOSEFONTW) -> BOOL;
}

#[link(name = "dwmapi")]
extern "system" {
    pub fn DwmSetWindowAttribute(
        hwnd: HWND,
        dwAttribute: u32,
        pvAttribute: *const core::ffi::c_void,
        cbAttribute: u32,
    ) -> i32;
}

#[link(name = "uxtheme")]
extern "system" {
    pub fn SetWindowTheme(hwnd: HWND, pszSubAppName: *const u16, pszSubIdList: *const u16) -> i32;
}

pub fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

pub fn copy_wide_fixed(dst: &mut [u16], s: &str) {
    dst.fill(0);
    let max = dst.len().saturating_sub(1);
    for (i, ch) in OsStr::new(s).encode_wide().take(max).enumerate() {
        dst[i] = ch;
    }
}

pub fn from_wide_buf(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

pub const fn make_int_resource(id: u16) -> *const u16 {
    id as usize as *const u16
}

pub const fn nt_success(status: NTSTATUS) -> bool {
    status >= 0
}

pub const fn loword(v: usize) -> u16 {
    (v & 0xFFFF) as u16
}

pub const fn hiword(v: usize) -> u16 {
    ((v >> 16) & 0xFFFF) as u16
}

pub const fn indextostateimagemask(i: u32) -> u32 {
    i << 12
}

pub fn os_version() -> (u32, u32, u32) {
    unsafe {
        let mut vi: RTL_OSVERSIONINFOW = std::mem::zeroed();
        vi.dwOSVersionInfoSize = std::mem::size_of::<RTL_OSVERSIONINFOW>() as u32;
        if nt_success(RtlGetVersion(&mut vi)) {
            (vi.dwMajorVersion, vi.dwMinorVersion, vi.dwBuildNumber)
        } else {
            (10, 0, 0)
        }
    }
}

pub fn is_win8_1_or_greater() -> bool {
    let (maj, min, _) = os_version();
    maj > 6 || (maj == 6 && min >= 3)
}

pub fn is_win10_or_greater() -> bool {
    let (maj, _, _) = os_version();
    maj >= 10
}

pub fn is_elevated() -> bool {
    const TOKEN_QUERY: u32 = 0x0008;
    const TOKEN_ELEVATION_CLASS: u32 = 20;
    unsafe {
        let mut token: HANDLE = 0;
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == FALSE {
            return false;
        }
        let mut elev = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut ret_len = 0u32;
        let ok = GetTokenInformation(
            token,
            TOKEN_ELEVATION_CLASS,
            &mut elev as *mut _ as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        );
        CloseHandle(token);
        ok != FALSE && elev.TokenIsElevated != 0
    }
}

pub fn is_fullscreen_or_busy() -> bool {
    unsafe {
        let mut state: i32 = 0;
        if SHQueryUserNotificationState(&mut state) == 0 {
            matches!(
                state,
                QUNS_BUSY | QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE
            )
        } else {
            false
        }
    }
}

pub fn get_window_dpi(hwnd: HWND) -> i32 {
    unsafe {
        if hwnd != 0 {
            let dpi = GetDpiForWindow(hwnd);
            if dpi > 0 {
                return dpi as i32;
            }
        }
        let taskbar = FindWindowW(to_wide("Shell_TrayWnd").as_ptr(), std::ptr::null());
        if taskbar != 0 {
            let dpi = GetDpiForWindow(taskbar);
            if dpi > 0 {
                return dpi as i32;
            }
        }
        let hdc = GetDC(0);
        if hdc != 0 {
            let dpi = GetDeviceCaps(hdc, LOGPIXELSY);
            ReleaseDC(0, hdc);
            if dpi > 0 {
                return dpi;
            }
        }
        96
    }
}

pub fn font_pt_to_height(pt: i32, dpi: i32) -> i32 {
    unsafe { -MulDiv(pt.abs(), dpi.max(96), 72) }
}

pub fn font_height_to_pt(height: i32, dpi: i32) -> i32 {
    unsafe { MulDiv(height.abs(), 72, dpi.max(96)) }
}

pub fn color_brightness_text(clr: COLORREF) -> COLORREF {
    let r = (clr & 0xFF) as u32;
    let g = ((clr >> 8) & 0xFF) as u32;
    let b = ((clr >> 16) & 0xFF) as u32;
    let luma = (r * 299 + g * 587 + b * 114) / 1000;
    if luma >= 140 {
        0x000000
    } else {
        0xFFFFFF
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    let b = bytes as f64;
    if bytes >= TB as u64 {
        format!("{:.2} TB", b / TB)
    } else if bytes >= GB as u64 {
        format!("{:.2} GB", b / GB)
    } else if bytes >= MB as u64 {
        format!("{:.2} MB", b / MB)
    } else if bytes >= KB as u64 {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

pub fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
