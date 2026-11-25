# RAM Reduct (Rust Edition v3.5.3)

Lightweight real-time memory management application for Windows to monitor and clean system memory using native Win32 & undocumented Windows Native APIs (`ntdll`), written in **100% pure Rust** with zero external crate dependencies.

## Highlights & Features
- **Pure Rust + Native Win32/NTAPI**: Zero runtime bloat, fast startup, tiny standalone `.exe` with embedded Win32 dialogs, Per-Monitor v2 DPI manifest, and icon resources.
- **8 Memory Regions Supported**:
  - Working set (`MemoryEmptyWorkingSets` or selective per-PID `K32EmptyWorkingSet`)
  - System file cache (`SystemFileCacheInformationEx`)
  - Modified file cache (raw volume buffer flush)
  - Modified page list (`MemoryFlushModifiedList`)
  - Standby list (`MemoryPurgeStandbyList`)
  - Standby priority-0 list (`MemoryPurgeLowPriorityStandbyList`)
  - Registry cache (`SystemRegistryReconciliationInformation`, Win 8.1+)
  - Combine memory lists (`SystemCombinePhysicalMemoryInformation`, Win 10+)
- **Background Tray & Windows Service Mode**: Continues running in the system tray when closed/minimized (`RunInBackground=true`) or registers as a native Windows Background Service (`services.msc`).
- **Process Whitelist (Selective Working Set Trim)**: Exclude latency-sensitive apps or games (`cs2.exe;valorant.exe;obs64.exe`) from working-set trimming in **Settings -> Advanced**.
- **Fullscreen / Game Mode Guard**: Automatically pauses background auto-clean when a fullscreen game, D3D exclusive app, or presentation is active (`SHQueryUserNotificationState`).
- **Live Standby / Modified & Session Metrics**: Displays real-time Standby cache (including Priority-0), Modified page list size, and cumulative session memory cleaned directly in the main window.
- **Live Tray Icon Preview & Auto-Clamped Thresholds**: Preview Normal, Warning, and Danger tray badges live in **Settings -> Appearance**, with automatic `Warning < Danger` threshold clamping.
- **Portable & Multi-Language**: Saves `ramreduct.ini` (portable if placed next to `ramreduct.exe`, otherwise `%APPDATA%\RAM Reduct\ramreduct.ini` with automatic migration from legacy configs) and loads translations from `bin/memreduct.lng`.

## Command-Line Options
```text
ramreduct.exe -background                 Run silently in background (system tray)
ramreduct.exe -minimized                  Start GUI minimized in the system tray
ramreduct.exe -clean                      Clear configured/default memory regions
ramreduct.exe -clean:full                 Clear all memory regions
ramreduct.exe -clean:mask=0x27            Clear custom bitmask regions
ramreduct.exe -clean:mask=ws,standby0     Clear named regions (ws, filecache, standby0, standby, modified, combine, registry, flush)
ramreduct.exe -silent                     Suppress popups/dialogs (ideal for Task Scheduler & scripts)
ramreduct.exe -install-service            Register & start Windows Background Service
ramreduct.exe -uninstall-service          Stop & remove Windows Background Service
ramreduct.exe -help                       Show CLI help
```

## Building
Requires Rust (`x86_64-pc-windows-msvc`) and Windows 10/11 SDK (`rc.exe`):

```powershell
cargo test
.\build.bat
```

Outputs `bin\64\ramreduct.exe` and `bin\64\ramreduct-3.5.3-setup.exe`.

## License
GPL-3.0 (Based on Mem Reduct (c) 2011-2026 Henry++).
