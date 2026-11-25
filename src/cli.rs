use crate::ids::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CliMode {
    Gui {
        start_minimized: bool,
        from_background_autorun: bool,
    },
    Clean {
        mask: Option<u32>,
        silent: bool,
    },
    ServiceRun,
    ServiceInstall {
        silent: bool,
    },
    ServiceUninstall {
        silent: bool,
    },
    Help,
}

pub fn parse_cli_args<I, S>(args: I) -> CliMode
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut is_clean = false;
    let mut clean_mask: Option<u32> = None;
    let mut silent = false;
    let mut minimized = false;
    let mut background_autorun = false;
    let mut is_service = false;
    let mut install_service = false;
    let mut uninstall_service = false;

    for raw in args {
        let arg = raw.as_ref().trim();
        if arg.is_empty() {
            continue;
        }
        let stripped = arg.trim_start_matches(['-', '/']);
        let lower = stripped.to_ascii_lowercase();

        match lower.as_str() {
            "help" | "h" | "?" => return CliMode::Help,
            "silent" | "quiet" | "q" => silent = true,
            "minimized" | "tray" => minimized = true,
            "background" | "bg" | "autostart" => {
                minimized = true;
                background_autorun = true;
            }
            "service" => is_service = true,
            "install-service" | "service:install" => install_service = true,
            "uninstall-service" | "service:uninstall" => uninstall_service = true,
            "clean" => is_clean = true,
            "clean:full" => {
                is_clean = true;
                clean_mask = Some(REDUCT_MASK_ALL);
            }
            _ => {
                if let Some(rest) = lower.strip_prefix("clean:mask=") {
                    is_clean = true;
                    if let Some(parsed) = parse_mask_spec(rest) {
                        clean_mask = Some(parsed);
                    }
                } else if let Some(rest) = lower.strip_prefix("clean:") {
                    is_clean = true;
                    if let Some(parsed) = parse_mask_spec(rest) {
                        clean_mask = Some(parsed);
                    }
                }
            }
        }
    }

    if is_service {
        return CliMode::ServiceRun;
    }
    if install_service {
        return CliMode::ServiceInstall { silent };
    }
    if uninstall_service {
        return CliMode::ServiceUninstall { silent };
    }
    if is_clean {
        return CliMode::Clean {
            mask: clean_mask,
            silent,
        };
    }

    CliMode::Gui {
        start_minimized: minimized,
        from_background_autorun: background_autorun,
    }
}

pub fn parse_mask_spec(spec: &str) -> Option<u32> {
    let s = spec.trim();
    if s.is_empty() {
        return None;
    }
    if s == "full" || s == "all" {
        return Some(REDUCT_MASK_ALL);
    }
    if s == "default" {
        return Some(REDUCT_MASK_DEFAULT);
    }
    if let Some(hex) = s.strip_prefix("0x") {
        if let Ok(v) = u32::from_str_radix(hex, 16) {
            let clamped = v & REDUCT_MASK_ALL;
            return if clamped != 0 { Some(clamped) } else { None };
        }
    }
    if let Ok(v) = s.parse::<u32>() {
        let clamped = v & REDUCT_MASK_ALL;
        return if clamped != 0 { Some(clamped) } else { None };
    }

    let mut mask = 0u32;
    for token in s.split([',', '+', '|', ';']) {
        match token.trim() {
            "workingset" | "ws" => mask |= REDUCT_WORKINGSET,
            "systemfilecache" | "filecache" | "sfc" => mask |= REDUCT_SYSTEMFILECACHE,
            "standby0" | "standbypriority0" | "lowstandby" => mask |= REDUCT_STANDBYPRIORITY0LIST,
            "standby" | "standbylist" => mask |= REDUCT_STANDBYLIST,
            "modified" | "modifiedlist" => mask |= REDUCT_MODIFIEDLIST,
            "combine" | "combinememorylists" => mask |= REDUCT_COMBINEMEMORYLISTS,
            "registry" | "registrycache" => mask |= REDUCT_REGISTRYCACHE,
            "modifiedfilecache" | "volumecache" | "flush" => mask |= REDUCT_MODIFIEDFILECACHE,
            _ => {}
        }
    }

    if mask != 0 {
        Some(mask)
    } else {
        None
    }
}

pub fn help_message() -> &'static str {
    "Available options for ramreduct.exe:\r\n\r\n\
     -background                 Run silently in background (system tray)\r\n\
     -minimized                  Start GUI minimized to system tray\r\n\
     -clean                      Clear configured/default memory regions\r\n\
     -clean:full                 Clear all memory regions\r\n\
     -clean:mask=<hex|names>     Clear custom regions (e.g. -clean:mask=0x27 or -clean:mask=ws,standby0,registry)\r\n\
     -silent                     Suppress notifications and dialogs\r\n\
     -install-service            Register & start Windows Background Service (services.msc)\r\n\
     -uninstall-service          Stop & remove Windows Background Service\r\n\
     -help                       Show this help message"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_variants() {
        assert_eq!(
            parse_cli_args(["-clean", "-silent"]),
            CliMode::Clean {
                mask: None,
                silent: true
            }
        );
        assert_eq!(
            parse_cli_args(["-clean:full"]),
            CliMode::Clean {
                mask: Some(REDUCT_MASK_ALL),
                silent: false
            }
        );
        assert_eq!(
            parse_cli_args(["-clean:mask=0x05", "-silent"]),
            CliMode::Clean {
                mask: Some(REDUCT_WORKINGSET | REDUCT_STANDBYPRIORITY0LIST),
                silent: true
            }
        );
        assert_eq!(
            parse_cli_args(["-background"]),
            CliMode::Gui {
                start_minimized: true,
                from_background_autorun: true,
            }
        );
        assert_eq!(parse_cli_args(["-service"]), CliMode::ServiceRun);
        assert_eq!(
            parse_cli_args(["-install-service", "-silent"]),
            CliMode::ServiceInstall { silent: true }
        );
        assert_eq!(parse_cli_args(["/?"]), CliMode::Help);
    }
}
