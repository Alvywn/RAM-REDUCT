use crate::config::AppConfig;
use crate::ffi::*;
use crate::ids::*;

pub struct TrayRenderer {
    hdc: HDC,
    hdc_mask: HDC,
    hbitmap: HBITMAP,
    hbitmap_mask: HBITMAP,
    hfont: HFONT,
    icon_size: RECT,
    current_icon: HICON,
    preview_icons: [HICON; 3],
    pub prev_percent: Option<u32>,
}

impl Default for TrayRenderer {
    fn default() -> Self {
        Self {
            hdc: 0,
            hdc_mask: 0,
            hbitmap: 0,
            hbitmap_mask: 0,
            hfont: 0,
            icon_size: RECT {
                left: 0,
                top: 0,
                right: 16,
                bottom: 16,
            },
            current_icon: 0,
            preview_icons: [0; 3],
            prev_percent: None,
        }
    }
}

impl TrayRenderer {
    pub fn init(&mut self, cfg: &AppConfig, dpi: i32) {
        self.cleanup_gdi();
        self.prev_percent = None;

        unsafe {
            let mut lf = LOGFONTW::default();
            copy_wide_fixed(&mut lf.lfFaceName, &cfg.tray_font_face);
            lf.lfHeight = font_pt_to_height(cfg.tray_font_size, dpi);
            lf.lfWeight = cfg.tray_font_weight;
            lf.lfItalic = if cfg.tray_font_italic { 1 } else { 0 };
            lf.lfCharSet = DEFAULT_CHARSET;
            lf.lfQuality = if cfg.tray_use_antialiasing {
                CLEARTYPE_QUALITY
            } else {
                NONANTIALIASED_QUALITY
            };

            self.hfont = CreateFontIndirectW(&lf);

            let base_cx = GetSystemMetrics(SM_CXSMICON).max(16);
            let base_cy = GetSystemMetrics(SM_CYSMICON).max(16);
            let cx = MulDiv(base_cx, dpi.max(96), 96).max(16);
            let cy = MulDiv(base_cy, dpi.max(96), 96).max(16);
            self.icon_size = RECT {
                left: 0,
                top: 0,
                right: cx,
                bottom: cy,
            };

            let screen_dc = GetDC(0);
            if screen_dc == 0 {
                return;
            }

            self.hdc = CreateCompatibleDC(screen_dc);
            self.hdc_mask = CreateCompatibleDC(screen_dc);

            let mut bmi: BITMAPINFO = std::mem::zeroed();
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = cx;
            bmi.bmiHeader.biHeight = cy;
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = BI_RGB;

            let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
            self.hbitmap =
                CreateDIBSection(screen_dc, &bmi, DIB_RGB_COLORS, &mut bits, 0, 0);
            self.hbitmap_mask = CreateBitmap(cx, cy, 1, 1, std::ptr::null());

            ReleaseDC(0, screen_dc);
        }
    }

    pub fn render_icon_raw(&mut self, percent: u32, cfg: &AppConfig) -> HICON {
        if self.hdc == 0 || self.hbitmap == 0 || self.hbitmap_mask == 0 {
            return 0;
        }

        let mut text_color = cfg.tray_color_text;
        let mut bg_color = cfg.tray_color_bg;
        let mut is_transparent = cfg.tray_use_transparency;
        let is_border = cfg.tray_show_border;
        let is_round = cfg.tray_round_corners;

        let has_danger = percent >= cfg.tray_level_danger;
        let has_warn = percent >= cfg.tray_level_warning;

        if has_danger || has_warn {
            let alert_clr = if has_danger {
                cfg.tray_color_danger
            } else {
                cfg.tray_color_warning
            };
            if cfg.tray_change_bg {
                bg_color = alert_clr;
                is_transparent = false;
            } else {
                text_color = alert_clr;
            }
        }

        let label = percent.to_string();
        let label_w = to_wide(&label);
        let label_len = (label_w.len().saturating_sub(1)) as i32;

        unsafe {
            // 1. Color bitmap
            let prev_bmp = SelectObject(self.hdc, self.hbitmap);
            let prev_font = SelectObject(self.hdc, self.hfont);
            let prev_bk = SetBkMode(self.hdc, TRANSPARENT);

            draw_icon_bg(
                self.hdc,
                bg_color,
                if is_border { text_color } else { bg_color },
                if is_transparent { text_color } else { bg_color },
                &self.icon_size,
                is_round,
            );

            let mut rc = self.icon_size;
            SetTextColor(self.hdc, text_color);
            DrawTextW(
                self.hdc,
                label_w.as_ptr(),
                label_len,
                &mut rc,
                DT_VCENTER | DT_CENTER | DT_SINGLELINE | DT_NOCLIP | DT_NOPREFIX,
            );

            SetBkMode(self.hdc, prev_bk);
            SelectObject(self.hdc, prev_font);
            SelectObject(self.hdc, prev_bmp);

            // 2. Monochrome AND mask
            let prev_mbmp = SelectObject(self.hdc_mask, self.hbitmap_mask);
            let prev_mfont = SelectObject(self.hdc_mask, self.hfont);
            let prev_mbk = SetBkMode(self.hdc_mask, TRANSPARENT);

            draw_icon_bg(
                self.hdc_mask,
                TRAY_COLOR_WHITE,
                if is_border {
                    TRAY_COLOR_BLACK
                } else {
                    TRAY_COLOR_WHITE
                },
                if is_transparent {
                    TRAY_COLOR_WHITE
                } else {
                    TRAY_COLOR_BLACK
                },
                &self.icon_size,
                is_round,
            );

            let mut mrc = self.icon_size;
            SetTextColor(self.hdc_mask, TRAY_COLOR_BLACK);
            DrawTextW(
                self.hdc_mask,
                label_w.as_ptr(),
                label_len,
                &mut mrc,
                DT_VCENTER | DT_CENTER | DT_SINGLELINE | DT_NOCLIP | DT_NOPREFIX,
            );

            SetBkMode(self.hdc_mask, prev_mbk);
            SelectObject(self.hdc_mask, prev_mfont);
            SelectObject(self.hdc_mask, prev_mbmp);

            let ii = ICONINFO {
                fIcon: TRUE,
                xHotspot: 0,
                yHotspot: 0,
                hbmMask: self.hbitmap_mask,
                hbmColor: self.hbitmap,
            };
            CreateIconIndirect(&ii)
        }
    }

    pub fn update_main_icon(&mut self, percent: u32, cfg: &AppConfig) -> Option<HICON> {
        // Bug fix: Option<u32> handles percent == 0 cleanly without infinite redraws
        if self.prev_percent == Some(percent) && self.current_icon != 0 {
            return None;
        }
        self.prev_percent = Some(percent);
        let new_icon = self.render_icon_raw(percent, cfg);
        if new_icon != 0 {
            if self.current_icon != 0 {
                unsafe {
                    DestroyIcon(self.current_icon);
                }
            }
            self.current_icon = new_icon;
            Some(self.current_icon)
        } else {
            None
        }
    }

    pub fn current_or_render(&mut self, percent: u32, cfg: &AppConfig) -> HICON {
        let _ = self.update_main_icon(percent, cfg);
        self.current_icon
    }

    pub fn render_preview_trio(&mut self, cfg: &AppConfig) -> [HICON; 3] {
        for ic in &mut self.preview_icons {
            if *ic != 0 {
                unsafe {
                    DestroyIcon(*ic);
                }
                *ic = 0;
            }
        }
        let normal_pct = cfg.tray_level_warning.saturating_sub(20).max(25);
        let warn_pct = cfg.tray_level_warning;
        let danger_pct = cfg.tray_level_danger;

        self.preview_icons[0] = self.render_icon_raw(normal_pct, cfg);
        self.preview_icons[1] = self.render_icon_raw(warn_pct, cfg);
        self.preview_icons[2] = self.render_icon_raw(danger_pct, cfg);
        self.preview_icons
    }

    fn cleanup_gdi(&mut self) {
        unsafe {
            if self.hbitmap_mask != 0 {
                DeleteObject(self.hbitmap_mask);
                self.hbitmap_mask = 0;
            }
            if self.hbitmap != 0 {
                DeleteObject(self.hbitmap);
                self.hbitmap = 0;
            }
            if self.hfont != 0 {
                DeleteObject(self.hfont);
                self.hfont = 0;
            }
            if self.hdc_mask != 0 {
                DeleteDC(self.hdc_mask);
                self.hdc_mask = 0;
            }
            if self.hdc != 0 {
                DeleteDC(self.hdc);
                self.hdc = 0;
            }
        }
    }
}

impl Drop for TrayRenderer {
    fn drop(&mut self) {
        self.cleanup_gdi();
        unsafe {
            if self.current_icon != 0 {
                DestroyIcon(self.current_icon);
                self.current_icon = 0;
            }
            for ic in &mut self.preview_icons {
                if *ic != 0 {
                    DestroyIcon(*ic);
                    *ic = 0;
                }
            }
        }
    }
}

unsafe fn draw_icon_bg(
    hdc: HDC,
    bg_clr: COLORREF,
    pen_clr: COLORREF,
    brush_clr: COLORREF,
    rect: &RECT,
    is_round: bool,
) {
    let prev_brush = SelectObject(hdc, GetStockObject(DC_BRUSH));
    let prev_pen = SelectObject(hdc, GetStockObject(DC_PEN));
    let prev_bk = SetBkColor(hdc, bg_clr);

    let fill_brush = CreateSolidBrush(bg_clr);
    FillRect(hdc, rect, fill_brush);
    DeleteObject(fill_brush);

    SetDCPenColor(hdc, pen_clr);
    SetDCBrushColor(hdc, brush_clr);

    if is_round {
        RoundRect(
            hdc,
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
            (rect.right - 2).max(2),
            (rect.right / 2).max(2),
        );
    } else {
        Rectangle(hdc, rect.left, rect.top, rect.right, rect.bottom);
    }

    SelectObject(hdc, prev_brush);
    SelectObject(hdc, prev_pen);
    SetBkColor(hdc, prev_bk);
}

pub fn tray_create(hwnd: HWND, hicon: HICON, tip: &str) -> bool {
    unsafe {
        let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = TIMER_UID as u32;
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_GUID | NIF_SHOWTIP;
        nid.uCallbackMessage = WM_TRAYICON;
        nid.hIcon = hicon;
        nid.guidItem = GUID_TRAY_ICON;
        copy_wide_fixed(&mut nid.szTip, tip);

        if Shell_NotifyIconW(NIM_ADD, &mut nid) == FALSE {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &mut nid);
        }
        nid.uTimeoutOrVersion = NOTIFYICON_VERSION_4;
        Shell_NotifyIconW(NIM_SETVERSION, &mut nid) != FALSE
    }
}

pub fn tray_update(hwnd: HWND, hicon: Option<HICON>, tip: &str) {
    unsafe {
        let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = TIMER_UID as u32;
        nid.uFlags = NIF_TIP | NIF_GUID | NIF_SHOWTIP;
        if let Some(ic) = hicon {
            nid.uFlags |= NIF_ICON;
            nid.hIcon = ic;
        }
        nid.guidItem = GUID_TRAY_ICON;
        copy_wide_fixed(&mut nid.szTip, tip);
        let _ = Shell_NotifyIconW(NIM_MODIFY, &mut nid);
    }
}

pub fn tray_popup(hwnd: HWND, flags: u32, title: &str, message: &str) -> bool {
    if hwnd == 0 {
        return false;
    }
    unsafe {
        let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = TIMER_UID as u32;
        nid.uFlags = NIF_INFO | NIF_GUID | NIF_SHOWTIP;
        nid.dwInfoFlags = flags;
        nid.guidItem = GUID_TRAY_ICON;
        copy_wide_fixed(&mut nid.szInfoTitle, title);
        copy_wide_fixed(&mut nid.szInfo, message);
        Shell_NotifyIconW(NIM_MODIFY, &mut nid) != FALSE
    }
}

pub fn tray_destroy(hwnd: HWND) {
    unsafe {
        let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = TIMER_UID as u32;
        nid.uFlags = NIF_GUID;
        nid.guidItem = GUID_TRAY_ICON;
        let _ = Shell_NotifyIconW(NIM_DELETE, &mut nid);
    }
}
