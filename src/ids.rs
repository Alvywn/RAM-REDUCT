#![allow(dead_code)]

pub const APP_NAME: &str = "RAM Reduct";
pub const APP_NAME_SHORT: &str = "ramreduct";
pub const APP_VERSION: &str = "3.5.3";
pub const APP_AUTHOR: &str = "Henry++";
pub const APP_WEBSITE: &str = "https://github.com/henrypp/memreduct";
pub const APP_USER_MODEL_ID: &str = "Henrypp.RamReduct";
pub const SERVICE_NAME: &str = "RamReductService";
pub const SERVICE_DISPLAY_NAME: &str = "RAM Reduct Background Service";

pub const TIMER_UID: usize = 1337;
pub const TIMER_INTERVAL_MS: u32 = 1000;
pub const HOTKEY_UID: i32 = 1337;

pub const DEFAULT_AUTOREDUCT_VAL: u32 = 90;
pub const DEFAULT_AUTOREDUCTINTERVAL_VAL: u32 = 30;
pub const AUTOREDUCT_COOLDOWN_SECS: i64 = 30;

pub const DEFAULT_DANGER_LEVEL: u32 = 90;
pub const DEFAULT_WARNING_LEVEL: u32 = 70;

// GDI COLORREF: 0x00BBGGRR
pub const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

pub const TRAY_COLOR_BLACK: u32 = rgb(0x00, 0x00, 0x00);
pub const TRAY_COLOR_WHITE: u32 = rgb(0xFF, 0xFF, 0xFF);
pub const TRAY_COLOR_TEXT: u32 = rgb(0xFF, 0xFF, 0xFF);
pub const TRAY_COLOR_BG: u32 = rgb(0x00, 0x80, 0x40);
pub const TRAY_COLOR_WARNING: u32 = rgb(0xFF, 0x80, 0x40);
pub const TRAY_COLOR_DANGER: u32 = rgb(0xEC, 0x1C, 0x24);

// Memory cleaning bitmasks
pub const REDUCT_WORKINGSET: u32 = 0x01;
pub const REDUCT_SYSTEMFILECACHE: u32 = 0x02;
pub const REDUCT_STANDBYPRIORITY0LIST: u32 = 0x04;
pub const REDUCT_STANDBYLIST: u32 = 0x08;
pub const REDUCT_MODIFIEDLIST: u32 = 0x10;
pub const REDUCT_COMBINEMEMORYLISTS: u32 = 0x20;
pub const REDUCT_REGISTRYCACHE: u32 = 0x40;
pub const REDUCT_MODIFIEDFILECACHE: u32 = 0x80;

pub const REDUCT_MASK_ALL: u32 = REDUCT_WORKINGSET
    | REDUCT_SYSTEMFILECACHE
    | REDUCT_STANDBYPRIORITY0LIST
    | REDUCT_STANDBYLIST
    | REDUCT_MODIFIEDLIST
    | REDUCT_COMBINEMEMORYLISTS
    | REDUCT_REGISTRYCACHE
    | REDUCT_MODIFIEDFILECACHE;

pub const REDUCT_MASK_DEFAULT: u32 = REDUCT_WORKINGSET
    | REDUCT_SYSTEMFILECACHE
    | REDUCT_STANDBYPRIORITY0LIST
    | REDUCT_REGISTRYCACHE
    | REDUCT_COMBINEMEMORYLISTS
    | REDUCT_MODIFIEDFILECACHE;

pub const REDUCT_MASK_FREEZES: u32 = REDUCT_STANDBYLIST | REDUCT_MODIFIEDLIST;

// Resource IDs (must match src/resource.h)
pub const IDA_MAIN: u16 = 1;
pub const IDI_MAIN: u16 = 100;

pub const IDM_MAIN: u16 = 100;
pub const IDM_TRAY: u16 = 101;

pub const IDD_MAIN: u16 = 100;
pub const IDD_SETTINGS_GENERAL: u16 = 101;
pub const IDD_SETTINGS_MEMORY: u16 = 102;
pub const IDD_SETTINGS_APPEARANCE: u16 = 103;
pub const IDD_SETTINGS_TRAY: u16 = 104;
pub const IDD_SETTINGS_ADVANCED: u16 = 105;
pub const IDD_SETTINGS_CONTAINER: u16 = 106;
pub const IDD_CONFIRM: u16 = 107;

pub const IDC_LISTVIEW: i32 = 102;
pub const IDC_CLEAN: i32 = 103;

pub const IDC_TITLE_1: i32 = 104;
pub const IDC_TITLE_2: i32 = 105;
pub const IDC_TITLE_3: i32 = 106;
pub const IDC_TITLE_4: i32 = 107;
pub const IDC_TITLE_5: i32 = 108;
pub const IDC_TITLE_6: i32 = 109;
pub const IDC_TITLE_7: i32 = 110;
pub const IDC_TITLE_8: i32 = 111;
pub const IDC_TITLE_9: i32 = 112;

pub const IDC_ALWAYSONTOP_CHK: i32 = 113;
pub const IDC_LOADONSTARTUP_CHK: i32 = 114;
pub const IDC_STARTMINIMIZED_CHK: i32 = 115;
pub const IDC_SKIPUACWARNING_CHK: i32 = 116;
pub const IDC_REDUCTCONFIRMATION_CHK: i32 = 117;
pub const IDC_CHECKUPDATES_CHK: i32 = 118;

pub const IDC_LANGUAGE_HINT: i32 = 119;
pub const IDC_LANGUAGE: i32 = 120;

pub const IDC_REGIONS: i32 = 121;

pub const IDC_AUTOREDUCTENABLE_CHK: i32 = 122;
pub const IDC_AUTOREDUCTVALUE_CTRL: i32 = 123;
pub const IDC_AUTOREDUCTVALUE: i32 = 124;

pub const IDC_AUTOREDUCTINTERVALENABLE_CHK: i32 = 125;
pub const IDC_AUTOREDUCTINTERVALVALUE_CTRL: i32 = 126;
pub const IDC_AUTOREDUCTINTERVALVALUE: i32 = 127;

pub const IDC_HOTKEY_CLEAN_CHK: i32 = 128;
pub const IDC_HOTKEY_CLEAN: i32 = 129;

pub const IDC_TRAYUSETRANSPARENCY_CHK: i32 = 130;
pub const IDC_TRAYSHOWBORDER_CHK: i32 = 131;
pub const IDC_TRAYROUNDCORNERS_CHK: i32 = 132;
pub const IDC_TRAYCHANGEBG_CHK: i32 = 133;
pub const IDC_TRAYUSEANTIALIASING_CHK: i32 = 134;

pub const IDC_FONT_HINT: i32 = 135;
pub const IDC_FONT: i32 = 136;

pub const IDC_COLORS: i32 = 137;

pub const IDC_TRAYLEVELWARNING_HINT: i32 = 138;
pub const IDC_TRAYLEVELWARNING_CTRL: i32 = 139;
pub const IDC_TRAYLEVELWARNING: i32 = 140;

pub const IDC_TRAYLEVELDANGER_HINT: i32 = 141;
pub const IDC_TRAYLEVELDANGER_CTRL: i32 = 142;
pub const IDC_TRAYLEVELDANGER: i32 = 143;

pub const IDC_TRAYACTIONSC_HINT: i32 = 144;
pub const IDC_TRAYACTIONSC: i32 = 145;

pub const IDC_TRAYACTIONMC_HINT: i32 = 146;
pub const IDC_TRAYACTIONMC: i32 = 147;

pub const IDC_NOTIFICATIONSOUND_CHK: i32 = 148;
pub const IDC_SHOW_CLEAN_RESULT_CHK: i32 = 149;

pub const IDC_ALLOWSTANDBYLISTCLEANUP_CHK: i32 = 150;
pub const IDC_LOGRESULTS_CHK: i32 = 151;

pub const IDM_SETTINGS: i32 = 152;
pub const IDM_EXIT: i32 = 153;
pub const IDM_ALWAYSONTOP_CHK: i32 = 154;
pub const IDM_LOADONSTARTUP_CHK: i32 = 155;
pub const IDM_USEDARKTHEME: i32 = 156;
pub const IDM_STARTMINIMIZED_CHK: i32 = 157;
pub const IDM_REDUCTCONFIRMATION_CHK: i32 = 158;
pub const IDM_SKIPUACWARNING_CHK: i32 = 159;
pub const IDM_CHECKUPDATES_CHK: i32 = 160;

pub const IDM_WEBSITE: i32 = 161;
pub const IDM_CHECKUPDATES: i32 = 162;
pub const IDM_ABOUT: i32 = 163;

pub const IDM_TRAY_SHOW: i32 = 164;
pub const IDM_TRAY_CLEAN: i32 = 165;
pub const IDM_TRAY_DISABLE_1: i32 = 166;
pub const IDM_TRAY_DISABLE_2: i32 = 167;
pub const IDM_TRAY_SETTINGS: i32 = 168;
pub const IDM_TRAY_WEBSITE: i32 = 169;
pub const IDM_TRAY_ABOUT: i32 = 170;
pub const IDM_TRAY_EXIT: i32 = 171;

pub const IDM_WORKINGSET_CHK: i32 = 172;
pub const IDM_SYSTEMFILECACHE_CHK: i32 = 173;
pub const IDM_MODIFIEDFILECACHE_CHK: i32 = 174;
pub const IDM_MODIFIEDLIST_CHK: i32 = 175;
pub const IDM_STANDBYLIST_CHK: i32 = 176;
pub const IDM_STANDBYLISTPRIORITY0_CHK: i32 = 177;
pub const IDM_REGISTRYCACHE_CHK: i32 = 178;
pub const IDM_COMBINEMEMORYLISTS_CHK: i32 = 179;

pub const IDM_CLEAN_WORKINGSET: i32 = 180;
pub const IDM_CLEAN_SYSTEMFILECACHE: i32 = 181;
pub const IDM_CLEAN_MODIFIEDFILECACHE: i32 = 182;
pub const IDM_CLEAN_MODIFIEDLIST: i32 = 183;
pub const IDM_CLEAN_STANDBYLIST: i32 = 184;
pub const IDM_CLEAN_STANDBYLISTPRIORITY0: i32 = 185;
pub const IDM_CLEAN_REGISTRYCACHE: i32 = 186;
pub const IDM_CLEAN_COMBINEMEMORYLISTS: i32 = 187;

pub const IDC_SETTINGS_NAV: i32 = 190;
pub const IDC_SETTINGS_RESET: i32 = 191;
pub const IDC_SETTINGS_CLOSE: i32 = 192;

pub const IDC_TRAY_PREVIEW_HINT: i32 = 193;
pub const IDC_TRAY_PREVIEW_NORMAL: i32 = 194;
pub const IDC_TRAY_PREVIEW_WARN: i32 = 195;
pub const IDC_TRAY_PREVIEW_DANGER: i32 = 196;

pub const IDC_SKIPFULLSCREEN_CHK: i32 = 197;
pub const IDC_TITLE_WHITELIST: i32 = 198;
pub const IDC_WHITELIST_HINT: i32 = 199;
pub const IDC_WHITELIST_EDIT: i32 = 200;

pub const IDC_CONFIRM_TEXT: i32 = 201;
pub const IDC_CONFIRM_DONTASK: i32 = 202;

pub const IDC_RUNINBACKGROUND_CHK: i32 = 203;
pub const IDC_WINSERVICE_CHK: i32 = 204;
pub const IDC_OPENCONFIG_BTN: i32 = 205;
pub const IDM_RUNINBACKGROUND_CHK: i32 = 206;
pub const IDM_OPENCONFIG: i32 = 207;

pub const IDX_LANGUAGE: i32 = 1000;
pub const IDX_TRAY_POPUP_1: i32 = 2000;
pub const IDX_TRAY_POPUP_2: i32 = 3000;

// String IDs (match memreduct.lng 001..098 + custom IDs 200+)
pub const IDS_UPDATE_NO: u32 = 1;
pub const IDS_UPDATE_YES: u32 = 2;
pub const IDS_FILE: u32 = 3;
pub const IDS_SETTINGS: u32 = 4;
pub const IDS_EXIT: u32 = 5;
pub const IDS_LANGUAGE: u32 = 6;
pub const IDS_HELP: u32 = 7;
pub const IDS_WEBSITE: u32 = 8;
pub const IDS_CHECKUPDATES: u32 = 9;
pub const IDS_ABOUT: u32 = 10;
pub const IDS_DONATE: u32 = 11;
pub const IDS_TRAY_SHOW: u32 = 12;
pub const IDS_TRAY_DISABLE: u32 = 13;
pub const IDS_TRAY_POPUP_1: u32 = 14;
pub const IDS_TRAY_POPUP_2: u32 = 15;
pub const IDS_TRAY_POPUP_3: u32 = 16;
pub const IDS_CLEAN: u32 = 17;
pub const IDS_GROUP_1: u32 = 18;
pub const IDS_GROUP_2: u32 = 19;
pub const IDS_GROUP_3: u32 = 20;
pub const IDS_ITEM_1: u32 = 21;
pub const IDS_ITEM_2: u32 = 22;
pub const IDS_ITEM_3: u32 = 23;
pub const IDS_SETTINGS_GENERAL: u32 = 24;
pub const IDS_SETTINGS_MEMORY: u32 = 25;
pub const IDS_SETTINGS_APPEARANCE: u32 = 26;
pub const IDS_SETTINGS_TRAY: u32 = 27;
pub const IDS_CLOSE: u32 = 28;
pub const IDS_TITLE_1: u32 = 29;
pub const IDS_TITLE_2: u32 = 30;
pub const IDS_TITLE_3: u32 = 31;
pub const IDS_TITLE_4: u32 = 32;
pub const IDS_TITLE_5: u32 = 33;
pub const IDS_TITLE_6: u32 = 34;
pub const IDS_TITLE_7: u32 = 35;
pub const IDS_TITLE_8: u32 = 36;
pub const IDS_TITLE_9: u32 = 37;
pub const IDS_ALWAYSONTOP_CHK: u32 = 38;
pub const IDS_LOADONSTARTUP_CHK: u32 = 39;
pub const IDS_STARTMINIMIZED_CHK: u32 = 40;
pub const IDS_REDUCTCONFIRMATION_CHK: u32 = 41;
pub const IDS_SKIPUACWARNING_CHK: u32 = 42;
pub const IDS_CHECKUPDATES_CHK: u32 = 43;
pub const IDS_LANGUAGE_HINT: u32 = 44;
pub const IDS_AUTOREDUCTENABLE_CHK: u32 = 51;
pub const IDS_AUTOREDUCTINTERVALENABLE_CHK: u32 = 52;
pub const IDS_HOTKEY_CLEAN_CHK: u32 = 53;
pub const IDS_TRAYUSETRANSPARENCY_CHK: u32 = 54;
pub const IDS_TRAYSHOWBORDER_CHK: u32 = 55;
pub const IDS_TRAYROUNDCORNERS_CHK: u32 = 56;
pub const IDS_TRAYCHANGEBG_CHK: u32 = 57;
pub const IDS_TRAYUSEANTIALIASING_CHK: u32 = 58;
pub const IDS_FONT_HINT: u32 = 59;
pub const IDS_COLOR_TEXT_HINT: u32 = 60;
pub const IDS_COLOR_BACKGROUND_HINT: u32 = 61;
pub const IDS_COLOR_WARNING_HINT: u32 = 62;
pub const IDS_COLOR_DANGER_HINT: u32 = 63;
pub const IDS_TRAYLEVELWARNING_HINT: u32 = 64;
pub const IDS_TRAYLEVELDANGER_HINT: u32 = 65;
pub const IDS_TRAYACTIONSC_HINT: u32 = 66;
pub const IDS_TRAYACTIONMC_HINT: u32 = 67;
pub const IDS_TRAY_ACTION_1: u32 = 68;
pub const IDS_TRAY_ACTION_2: u32 = 69;
pub const IDS_TRAY_ACTION_3: u32 = 70;
pub const IDS_SHOW_CLEAN_RESULT_CHK: u32 = 71;
pub const IDS_QUESTION: u32 = 72;
pub const IDS_QUESTION_WARNING: u32 = 73;
pub const IDS_QUESTION_FLAG_CHK: u32 = 74;
pub const IDS_STATUS_CLEANED: u32 = 75;
pub const IDS_STATUS_NOPRIVILEGES: u32 = 76;
pub const IDS_RESET: u32 = 82;
pub const IDS_QUESTION_RESET: u32 = 83;
pub const IDS_NOTIFICATIONSOUND_CHK: u32 = 84;
pub const IDS_TITLE_ADVANCED: u32 = 85;
pub const IDS_VIEW: u32 = 86;
pub const IDS_USEDARKTHEME: u32 = 87;
pub const IDS_ALLOWSTANDBYLISTCLEANUP_CHK: u32 = 89;
pub const IDS_LOGRESULTS_CHK: u32 = 90;
pub const IDS_MEMREGION_WORKINGSET: u32 = 91;
pub const IDS_MEMREGION_SYSTEMFILECACHE: u32 = 92;
pub const IDS_MEMREGION_MODIFIEDFILECACHE: u32 = 93;
pub const IDS_MEMREGION_MODIFIEDLIST: u32 = 94;
pub const IDS_MEMREGION_STANDBYLIST: u32 = 95;
pub const IDS_MEMREGION_STANDBYLISTPRIORITY0: u32 = 96;
pub const IDS_MEMREGION_REGISTRYCACHE: u32 = 97;
pub const IDS_MEMREGION_COMBINEMEMORYLISTS: u32 = 98;

// Extended string IDs for new features
pub const IDS_GROUP_4: u32 = 200;
pub const IDS_ITEM_STANDBY: u32 = 201;
pub const IDS_ITEM_MODIFIED: u32 = 202;
pub const IDS_ITEM_SESSION_CLEANED: u32 = 203;
pub const IDS_SKIPFULLSCREEN_CHK: u32 = 204;
pub const IDS_TITLE_WHITELIST: u32 = 205;
pub const IDS_WHITELIST_HINT: u32 = 206;
pub const IDS_PREVIEW_HINT: u32 = 207;
pub const IDS_RUNINBACKGROUND_CHK: u32 = 208;
pub const IDS_WINSERVICE_CHK: u32 = 209;
pub const IDS_OPENCONFIG: u32 = 210;
pub const IDS_BG_RUNNING_INFO: u32 = 211;
