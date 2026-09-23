//! Launch at Windows sign-in via `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
//!
//! Same pattern as desktop-remote-kit. Non-Windows returns [`Error::Unsupported`].

use std::path::Path;
use thiserror::Error;

#[cfg(windows)]
const APP_ID: &str = "McpStatus";

/// Autostart errors.
#[derive(Debug, Error)]
pub enum Error {
    #[error("autostart is not implemented on this platform")]
    Unsupported,
    #[error("executable path is invalid (need absolute path)")]
    InvalidPath,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub fn is_enabled() -> Result<bool, Error> {
    #[cfg(windows)]
    {
        windows::is_enabled(APP_ID)
    }
    #[cfg(not(windows))]
    {
        Err(Error::Unsupported)
    }
}

pub fn enable(exe: &Path, args: &str) -> Result<(), Error> {
    if !exe.is_absolute() {
        return Err(Error::InvalidPath);
    }
    #[cfg(windows)]
    {
        windows::enable(APP_ID, exe, args)
    }
    #[cfg(not(windows))]
    {
        let _ = (exe, args);
        Err(Error::Unsupported)
    }
}

pub fn disable() -> Result<(), Error> {
    #[cfg(windows)]
    {
        windows::disable(APP_ID)
    }
    #[cfg(not(windows))]
    {
        Err(Error::Unsupported)
    }
}

#[cfg(windows)]
mod windows {
    use super::Error;
    use std::path::Path;
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

    fn open_run(write: bool) -> std::io::Result<RegKey> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if write {
            Ok(hkcu.create_subkey(RUN_KEY)?.0)
        } else {
            Ok(hkcu.open_subkey(RUN_KEY)?)
        }
    }

    pub fn is_enabled(id: &str) -> Result<bool, Error> {
        let key = open_run(false)?;
        match key.get_value::<String, _>(id) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    pub fn enable(id: &str, exe: &Path, args: &str) -> Result<(), Error> {
        let key = open_run(true)?;
        let exe_s = exe.to_string_lossy();
        let value = if args.trim().is_empty() {
            format!("\"{exe_s}\"")
        } else {
            format!("\"{exe_s}\" {args}")
        };
        key.set_value(id, &value)?;
        Ok(())
    }

    pub fn disable(id: &str) -> Result<(), Error> {
        let key = open_run(true)?;
        match key.delete_value(id) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}
