use std::collections::HashSet;

use crate::ffi::*;
use crate::ids::*;
use crate::locale::LocaleManager;

const PAGE_SIZE_BYTES: u64 = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CleanupSource {
    Auto,
    Manual,
    ManualRegion,
    Hotkey,
    Cmdline,
}

impl CleanupSource {
    pub fn reason_label(self) -> &'static str {
        match self {
            CleanupSource::Auto => "Cleanup (Auto)",
            CleanupSource::Manual | CleanupSource::ManualRegion => "Cleanup (Manual)",
            CleanupSource::Hotkey => "Cleanup (Hotkey)",
            CleanupSource::Cmdline => "Cleanup (Command-line)",
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MemorySnapshot {
    pub phys_used: u64,
    pub phys_free: u64,
    pub phys_total: u64,
    pub phys_percent_f: f64,
    pub phys_percent: u32,

    pub page_used: u64,
    pub page_free: u64,
    pub page_total: u64,
    pub page_percent_f: f64,
    pub page_percent: u32,

    pub cache_used: u64,
    pub cache_free: u64,
    pub cache_total: u64,
    pub cache_percent_f: f64,
    pub cache_percent: u32,

    pub standby_bytes: u64,
    pub standby_prio0_bytes: u64,
    pub modified_bytes: u64,
}

#[derive(Clone, Debug, Default)]
pub struct CleanupOutcome {
    pub effective_mask: u32,
    pub freed_physical: u64,
    pub freed_cache_lists: u64,
    pub reported_freed: u64,
    pub selective_trimmed: Option<(u32, u32)>,
    pub errors: Vec<String>,
}

pub fn enable_required_privileges() {
    unsafe {
        let mut prev: u8 = 0;
        let _ = RtlAdjustPrivilege(SE_PROF_SINGLE_PROCESS_PRIVILEGE, 1, 0, &mut prev);
        let _ = RtlAdjustPrivilege(SE_INCREASE_QUOTA_PRIVILEGE, 1, 0, &mut prev);
        let _ = RtlAdjustPrivilege(SE_DEBUG_PRIVILEGE, 1, 0, &mut prev);
    }
}

pub fn query_memory_snapshot() -> MemorySnapshot {
    let mut snap = MemorySnapshot::default();

    unsafe {
        let mut ms: MEMORYSTATUSEX = std::mem::zeroed();
        ms.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        if GlobalMemoryStatusEx(&mut ms) != FALSE {
            snap.phys_total = ms.ullTotalPhys;
            snap.phys_free = ms.ullAvailPhys;
            snap.phys_used = ms.ullTotalPhys.saturating_sub(ms.ullAvailPhys);
            if snap.phys_total > 0 {
                snap.phys_percent_f = (snap.phys_used as f64 * 100.0) / (snap.phys_total as f64);
                snap.phys_percent = (snap.phys_percent_f.round() as u32).clamp(0, 100);
            }

            snap.page_total = ms.ullTotalPageFile;
            snap.page_free = ms.ullAvailPageFile;
            snap.page_used = ms.ullTotalPageFile.saturating_sub(ms.ullAvailPageFile);
            if snap.page_total > 0 {
                snap.page_percent_f = (snap.page_used as f64 * 100.0) / (snap.page_total as f64);
                snap.page_percent = (snap.page_percent_f.round() as u32).clamp(0, 100);
            }
        }

        let mut sfci = SYSTEM_FILECACHE_INFORMATION::default();
        let mut ret_len = 0u32;
        let st = NtQuerySystemInformation(
            SYSTEM_FILECACHE_INFORMATION_EX_CLASS,
            &mut sfci as *mut _ as *mut _,
            std::mem::size_of::<SYSTEM_FILECACHE_INFORMATION>() as u32,
            &mut ret_len,
        );
        if nt_success(st) {
            snap.cache_used = sfci.CurrentSize as u64;
            snap.cache_total = (sfci.PeakSize as u64).max(snap.cache_used);
            snap.cache_free = snap.cache_total.saturating_sub(snap.cache_used);
            if snap.cache_total > 0 {
                snap.cache_percent_f =
                    (snap.cache_used as f64 * 100.0) / (snap.cache_total as f64);
                snap.cache_percent = (snap.cache_percent_f.round() as u32).clamp(0, 100);
            }
        }

        let mut mli = SYSTEM_MEMORY_LIST_INFORMATION::default();
        let full_size = std::mem::size_of::<SYSTEM_MEMORY_LIST_INFORMATION>() as u32;
        let mut st_list = NtQuerySystemInformation(
            SYSTEM_MEMORY_LIST_INFORMATION_CLASS,
            &mut mli as *mut _ as *mut _,
            full_size,
            &mut ret_len,
        );
        if !nt_success(st_list) {
            // Fallback for older OS struct without ModifiedPageCountPageFile
            let legacy_size = (full_size as usize - std::mem::size_of::<usize>()) as u32;
            st_list = NtQuerySystemInformation(
                SYSTEM_MEMORY_LIST_INFORMATION_CLASS,
                &mut mli as *mut _ as *mut _,
                legacy_size,
                &mut ret_len,
            );
        }
        if nt_success(st_list) {
            let total_standby_pages: u64 = mli
                .PageCountByPriority
                .iter()
                .map(|&p| p as u64)
                .sum();
            snap.standby_bytes = total_standby_pages.saturating_mul(PAGE_SIZE_BYTES);
            snap.standby_prio0_bytes =
                (mli.PageCountByPriority[0] as u64).saturating_mul(PAGE_SIZE_BYTES);
            snap.modified_bytes = (mli.ModifiedPageCount as u64).saturating_mul(PAGE_SIZE_BYTES);
        }
    }

    snap
}

pub fn parse_whitelist_set(whitelist_raw: &str) -> HashSet<String> {
    whitelist_raw
        .split([';', ',', '\r', '\n'])
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .map(|s| {
            if s.ends_with(".exe") {
                s
            } else {
                format!("{s}.exe")
            }
        })
        .collect()
}

fn trim_working_sets_selective(whitelist: &HashSet<String>) -> (u32, u32) {
    let mut trimmed = 0u32;
    let mut skipped = 0u32;
    let self_pid = unsafe { GetCurrentProcessId() };

    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE || snap == 0 {
            return (0, 0);
        }

        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snap, &mut entry) != FALSE {
            loop {
                let pid = entry.th32ProcessID;
                if pid > 4 && pid != self_pid {
                    let exe_name = from_wide_buf(&entry.szExeFile).to_ascii_lowercase();
                    if whitelist.contains(&exe_name) {
                        skipped += 1;
                    } else {
                        let hproc = OpenProcess(
                            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_QUOTA,
                            FALSE,
                            pid,
                        );
                        if hproc != 0 {
                            if K32EmptyWorkingSet(hproc) != FALSE {
                                trimmed += 1;
                            }
                            CloseHandle(hproc);
                        }
                    }
                }

                if Process32NextW(snap, &mut entry) == FALSE {
                    break;
                }
            }
        }

        CloseHandle(snap);
    }

    (trimmed, skipped)
}

fn flush_all_volume_caches() {
    unsafe {
        let mut vol_buf = [0u16; 260];
        let find_handle = FindFirstVolumeW(vol_buf.as_mut_ptr(), vol_buf.len() as u32);
        if find_handle == INVALID_HANDLE_VALUE || find_handle == 0 {
            return;
        }

        loop {
            let dtype = GetDriveTypeW(vol_buf.as_ptr());
            if dtype == DRIVE_FIXED || dtype == DRIVE_REMOVABLE {
                // FindFirstVolumeW returns "\\?\Volume{GUID}\" — strip trailing backslash to open raw volume
                let mut open_path = from_wide_buf(&vol_buf);
                if open_path.ends_with('\\') {
                    open_path.pop();
                }
                let open_w = to_wide(&open_path);
                let hvol = CreateFileW(
                    open_w.as_ptr(),
                    GENERIC_WRITE,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    std::ptr::null(),
                    OPEN_EXISTING,
                    0,
                    0,
                );
                if hvol != INVALID_HANDLE_VALUE && hvol != 0 {
                    let _ = FlushFileBuffers(hvol);
                    CloseHandle(hvol);
                }
            }

            vol_buf.fill(0);
            if FindNextVolumeW(find_handle, vol_buf.as_mut_ptr(), vol_buf.len() as u32) == FALSE {
                break;
            }
        }

        FindVolumeClose(find_handle);
    }
}

pub fn format_regions_list(mask: u32, locale: &LocaleManager) -> String {
    let mut lines: Vec<String> = Vec::new();

    if (mask & REDUCT_WORKINGSET) != 0 {
        lines.push(format!("- {}", locale.get(IDS_MEMREGION_WORKINGSET)));
    }
    if (mask & REDUCT_SYSTEMFILECACHE) != 0 {
        lines.push(format!("- {}", locale.get(IDS_MEMREGION_SYSTEMFILECACHE)));
    }
    if (mask & REDUCT_MODIFIEDFILECACHE) != 0 {
        lines.push(format!("- {}", locale.get(IDS_MEMREGION_MODIFIEDFILECACHE)));
    }
    if (mask & REDUCT_MODIFIEDLIST) != 0 {
        lines.push(format!("- {}*", locale.get(IDS_MEMREGION_MODIFIEDLIST)));
    }
    if (mask & REDUCT_STANDBYLIST) != 0 {
        lines.push(format!("- {}*", locale.get(IDS_MEMREGION_STANDBYLIST)));
    }
    if (mask & REDUCT_STANDBYPRIORITY0LIST) != 0 {
        lines.push(format!(
            "- {}",
            locale.get(IDS_MEMREGION_STANDBYLISTPRIORITY0)
        ));
    }
    if (mask & REDUCT_REGISTRYCACHE) != 0 {
        lines.push(format!(
            "- {} (win8.1+)",
            locale.get(IDS_MEMREGION_REGISTRYCACHE)
        ));
    }
    if (mask & REDUCT_COMBINEMEMORYLISTS) != 0 {
        lines.push(format!(
            "- {} (win10+)",
            locale.get(IDS_MEMREGION_COMBINEMEMORYLISTS)
        ));
    }

    lines.join("\r\n")
}

pub fn execute_memory_clean(
    requested_mask: u32,
    source: CleanupSource,
    allow_standby_on_auto: bool,
    whitelist_raw: &str,
) -> CleanupOutcome {
    let mut mask = requested_mask & REDUCT_MASK_ALL;
    if source == CleanupSource::Auto && !allow_standby_on_auto {
        mask &= !REDUCT_MASK_FREEZES;
    }

    let mut outcome = CleanupOutcome {
        effective_mask: mask,
        ..Default::default()
    };
    if mask == 0 {
        return outcome;
    }

    enable_required_privileges();
    let before = query_memory_snapshot();
    let whitelist = parse_whitelist_set(whitelist_raw);

    unsafe {
        if (mask & REDUCT_WORKINGSET) != 0 {
            if !whitelist.is_empty() {
                let (trimmed, skipped) = trim_working_sets_selective(&whitelist);
                outcome.selective_trimmed = Some((trimmed, skipped));
            } else {
                let cmd: u32 = MEMORY_EMPTY_WORKING_SETS;
                let st = NtSetSystemInformation(
                    SYSTEM_MEMORY_LIST_INFORMATION_CLASS,
                    &cmd as *const _ as *const _,
                    std::mem::size_of::<u32>() as u32,
                );
                if !nt_success(st) {
                    outcome
                        .errors
                        .push(format!("MemoryEmptyWorkingSets: 0x{:08X}", st as u32));
                }
            }
        }

        if (mask & REDUCT_SYSTEMFILECACHE) != 0 {
            let mut sfci = SYSTEM_FILECACHE_INFORMATION::default();
            sfci.MinimumWorkingSet = usize::MAX;
            sfci.MaximumWorkingSet = usize::MAX;
            let st = NtSetSystemInformation(
                SYSTEM_FILECACHE_INFORMATION_EX_CLASS,
                &sfci as *const _ as *const _,
                std::mem::size_of::<SYSTEM_FILECACHE_INFORMATION>() as u32,
            );
            if !nt_success(st) {
                outcome
                    .errors
                    .push(format!("SystemFileCacheInformationEx: 0x{:08X}", st as u32));
            }
        }

        if (mask & REDUCT_MODIFIEDFILECACHE) != 0 {
            flush_all_volume_caches();
        }

        if (mask & REDUCT_MODIFIEDLIST) != 0 {
            let cmd: u32 = MEMORY_FLUSH_MODIFIED_LIST;
            let st = NtSetSystemInformation(
                SYSTEM_MEMORY_LIST_INFORMATION_CLASS,
                &cmd as *const _ as *const _,
                std::mem::size_of::<u32>() as u32,
            );
            if !nt_success(st) {
                outcome
                    .errors
                    .push(format!("MemoryFlushModifiedList: 0x{:08X}", st as u32));
            }
        }

        if (mask & REDUCT_STANDBYLIST) != 0 {
            let cmd: u32 = MEMORY_PURGE_STANDBY_LIST;
            let st = NtSetSystemInformation(
                SYSTEM_MEMORY_LIST_INFORMATION_CLASS,
                &cmd as *const _ as *const _,
                std::mem::size_of::<u32>() as u32,
            );
            if !nt_success(st) {
                outcome
                    .errors
                    .push(format!("MemoryPurgeStandbyList: 0x{:08X}", st as u32));
            }
        }

        if (mask & REDUCT_STANDBYPRIORITY0LIST) != 0 {
            let cmd: u32 = MEMORY_PURGE_LOW_PRIORITY_STANDBY_LIST;
            let st = NtSetSystemInformation(
                SYSTEM_MEMORY_LIST_INFORMATION_CLASS,
                &cmd as *const _ as *const _,
                std::mem::size_of::<u32>() as u32,
            );
            if !nt_success(st) {
                outcome.errors.push(format!(
                    "MemoryPurgeLowPriorityStandbyList: 0x{:08X}",
                    st as u32
                ));
            }
        }

        if is_win8_1_or_greater() && (mask & REDUCT_REGISTRYCACHE) != 0 {
            let st = NtSetSystemInformation(
                SYSTEM_REGISTRY_RECONCILIATION_INFORMATION_CLASS,
                std::ptr::null(),
                0,
            );
            if !nt_success(st) {
                outcome.errors.push(format!(
                    "SystemRegistryReconciliationInformation: 0x{:08X}",
                    st as u32
                ));
            }
        }

        if is_win10_or_greater() && (mask & REDUCT_COMBINEMEMORYLISTS) != 0 {
            let combine = MEMORY_COMBINE_INFORMATION_EX::default();
            let st = NtSetSystemInformation(
                SYSTEM_COMBINE_PHYSICAL_MEMORY_INFORMATION_CLASS,
                &combine as *const _ as *const _,
                std::mem::size_of::<MEMORY_COMBINE_INFORMATION_EX>() as u32,
            );
            if !nt_success(st) {
                outcome.errors.push(format!(
                    "SystemCombinePhysicalMemoryInformation: 0x{:08X}",
                    st as u32
                ));
            }
        }
    }

    let after = query_memory_snapshot();
    outcome.freed_physical = before.phys_used.saturating_sub(after.phys_used);
    let before_lists = before.standby_bytes.saturating_add(before.modified_bytes);
    let after_lists = after.standby_bytes.saturating_add(after.modified_bytes);
    outcome.freed_cache_lists = before_lists.saturating_sub(after_lists);

    // If user purged standby/modified lists where physical_used didn't drop, report cache freed too
    outcome.reported_freed = if outcome.freed_physical > 0 {
        outcome.freed_physical
    } else {
        outcome.freed_cache_lists
    };

    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_whitelist_normalizes_exe() {
        let set = parse_whitelist_set("cs2; VALORANT.EXE, obs64.exe ; ");
        assert!(set.contains("cs2.exe"));
        assert!(set.contains("valorant.exe"));
        assert!(set.contains("obs64.exe"));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_query_memory_snapshot_live() {
        let snap = query_memory_snapshot();
        assert!(snap.phys_total > 0);
        assert!(snap.phys_percent <= 100);
    }
}
