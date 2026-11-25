use std::env;
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::atomic::{AtomicIsize, Ordering};

use crate::cleaner::{enable_required_privileges, execute_memory_clean, query_memory_snapshot, CleanupSource};
use crate::config::AppConfig;
use crate::ffi::*;
use crate::ids::*;

const CREATE_NO_WINDOW: u32 = 0x08000000;

static SERVICE_STATUS_HDL: AtomicIsize = AtomicIsize::new(0);
static SERVICE_STOP_EVENT: AtomicIsize = AtomicIsize::new(0);

pub fn is_service_installed() -> bool {
    Command::new("sc.exe")
        .args(["query", SERVICE_NAME])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn install_and_start_service() -> Result<(), String> {
    if !is_elevated() {
        return Err("Administrator privileges are required to register a Windows Service.".into());
    }
    let exe = env::current_exe().map_err(|e| format!("Failed to locate executable: {e}"))?;
    let bin_arg = format!("\"{}\" -service", exe.display());

    if is_service_installed() {
        let _ = Command::new("sc.exe")
            .args(["stop", SERVICE_NAME])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        let _ = Command::new("sc.exe")
            .args(["delete", SERVICE_NAME])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }

    let create_out = Command::new("sc.exe")
        .args([
            "create",
            SERVICE_NAME,
            &format!("binPath= {bin_arg}"),
            "start= auto",
            &format!("DisplayName= \"{SERVICE_DISPLAY_NAME}\""),
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to execute sc.exe: {e}"))?;

    if !create_out.status.success() {
        let stderr = String::from_utf8_lossy(&create_out.stdout);
        return Err(format!("sc create failed: {}", stderr.trim()));
    }

    let _ = Command::new("sc.exe")
        .args([
            "description",
            SERVICE_NAME,
            "Monitors and automatically cleans Windows system memory caches in the background.",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    // Ensure current config is mirrored to %ProgramData%\Mem Reduct\memreduct.ini
    let cfg = AppConfig::load();
    cfg.save();

    let _ = Command::new("sc.exe")
        .args(["start", SERVICE_NAME])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    Ok(())
}

pub fn stop_and_uninstall_service() -> Result<(), String> {
    if !is_elevated() {
        return Err("Administrator privileges are required to remove a Windows Service.".into());
    }
    if !is_service_installed() {
        return Ok(());
    }
    let _ = Command::new("sc.exe")
        .args(["stop", SERVICE_NAME])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    let del_out = Command::new("sc.exe")
        .args(["delete", SERVICE_NAME])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to invoke sc.exe: {e}"))?;

    if !del_out.status.success() {
        let out = String::from_utf8_lossy(&del_out.stdout);
        return Err(format!("sc delete failed: {}", out.trim()));
    }
    Ok(())
}

pub fn run_windows_service_dispatcher() {
    let mut name_w = to_wide(SERVICE_NAME);
    let table = [
        SERVICE_TABLE_ENTRYW {
            lpServiceName: name_w.as_mut_ptr(),
            lpServiceProc: Some(service_main),
        },
        SERVICE_TABLE_ENTRYW {
            lpServiceName: std::ptr::null_mut(),
            lpServiceProc: None,
        },
    ];

    unsafe {
        let _ = StartServiceCtrlDispatcherW(table.as_ptr());
    }
}

unsafe extern "system" fn service_ctrl_handler(
    control: u32,
    _event_type: u32,
    _event_data: *mut core::ffi::c_void,
    _context: *mut core::ffi::c_void,
) -> u32 {
    match control {
        SERVICE_CONTROL_STOP | SERVICE_CONTROL_SHUTDOWN => {
            report_service_status(SERVICE_STOP_PENDING, NO_ERROR, 3000);
            let ev = SERVICE_STOP_EVENT.load(Ordering::SeqCst);
            if ev != 0 {
                SetEvent(ev);
            }
            NO_ERROR
        }
        SERVICE_CONTROL_INTERROGATE => NO_ERROR,
        _ => ERROR_CALL_NOT_IMPLEMENTED,
    }
}

unsafe fn report_service_status(state: u32, exit_code: u32, wait_hint: u32) {
    let hdl = SERVICE_STATUS_HDL.load(Ordering::SeqCst);
    if hdl == 0 {
        return;
    }
    let mut st = SERVICE_STATUS {
        dwServiceType: SERVICE_WIN32_OWN_PROCESS,
        dwCurrentState: state,
        dwControlsAccepted: if state == SERVICE_START_PENDING {
            0
        } else {
            SERVICE_ACCEPT_STOP | SERVICE_ACCEPT_SHUTDOWN
        },
        dwWin32ExitCode: exit_code,
        dwServiceSpecificExitCode: 0,
        dwCheckPoint: 0,
        dwWaitHint: wait_hint,
    };
    let _ = SetServiceStatus(hdl, &mut st);
}

unsafe extern "system" fn service_main(_argc: u32, _argv: *mut *mut u16) {
    let hdl = RegisterServiceCtrlHandlerExW(
        to_wide(SERVICE_NAME).as_ptr(),
        Some(service_ctrl_handler),
        std::ptr::null_mut(),
    );
    if hdl == 0 {
        return;
    }
    SERVICE_STATUS_HDL.store(hdl, Ordering::SeqCst);
    report_service_status(SERVICE_START_PENDING, NO_ERROR, 2000);

    let stop_event = CreateEventW(std::ptr::null(), TRUE, FALSE, std::ptr::null());
    if stop_event == 0 {
        report_service_status(SERVICE_STOPPED, 1, 0);
        return;
    }
    SERVICE_STOP_EVENT.store(stop_event, Ordering::SeqCst);

    enable_required_privileges();
    report_service_status(SERVICE_RUNNING, NO_ERROR, 0);

    let mut last_limit_ts: i64 = 0;
    let mut last_interval_ts: i64 = unix_now();
    let mut cfg = load_service_config();
    let mut reload_tick: u32 = 0;

    // Poll every 2 seconds until stop event is signaled
    loop {
        if WaitForSingleObject(stop_event, 2000) == WAIT_OBJECT_0 {
            break;
        }

        reload_tick = reload_tick.wrapping_add(1);
        if reload_tick % 5 == 0 {
            cfg = load_service_config();
        }

        let snap = query_memory_snapshot();
        let now = unix_now();
        let mut should_clean = false;

        // If user enabled threshold auto-clean (or default 90% in service mode when neither is enabled)
        let effective_limit_enabled =
            cfg.autoreduct_enable || !cfg.autoreduct_interval_enable;
        if effective_limit_enabled && snap.phys_percent >= cfg.autoreduct_value {
            if now.saturating_sub(last_limit_ts) >= AUTOREDUCT_COOLDOWN_SECS {
                last_limit_ts = now;
                should_clean = true;
            }
        }

        if !should_clean && cfg.autoreduct_interval_enable {
            let interval_secs = (cfg.autoreduct_interval_value as i64) * 60;
            if now.saturating_sub(last_interval_ts) >= interval_secs {
                last_interval_ts = now;
                should_clean = true;
            }
        }

        if should_clean {
            let outcome = execute_memory_clean(
                cfg.reduct_mask,
                CleanupSource::Auto,
                cfg.allow_standby_cleanup,
                &cfg.process_whitelist,
            );
            if cfg.log_clean_results {
                cfg.append_log(
                    "Service (Auto)",
                    &format_bytes(outcome.reported_freed),
                );
            }
        }
    }

    CloseHandle(stop_event);
    SERVICE_STOP_EVENT.store(0, Ordering::SeqCst);
    report_service_status(SERVICE_STOPPED, NO_ERROR, 0);
}

fn load_service_config() -> AppConfig {
    if let Some(pd_ini) = AppConfig::shared_programdata_ini() {
        if pd_ini.exists() {
            let log = pd_ini
                .parent()
                .map(|p| p.join("memreduct-service.log"))
                .unwrap_or_else(|| pd_ini.with_extension("log"));
            return AppConfig::load_from_path(pd_ini, log);
        }
    }
    AppConfig::load()
}
