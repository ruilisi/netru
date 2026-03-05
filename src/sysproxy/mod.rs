//! Get/Set system proxy. Supports Windows, macOS and Linux (via gsettings/kconfig).

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(feature = "utils")]
pub mod utils;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Sysproxy {
    pub enable: bool,
    pub host: String,
    pub port: u16,
    pub bypass: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Autoproxy {
    pub enable: bool,
    pub url: String,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed to parse string `{0}`")]
    ParseStr(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("failed to get default network interface")]
    NetworkInterface,

    #[error("failed to set proxy for this environment")]
    NotSupport,

    #[cfg(target_os = "linux")]
    #[error(transparent)]
    Xdg(#[from] xdg::BaseDirectoriesError),

    #[cfg(target_os = "windows")]
    #[error("system call failed")]
    SystemCall(#[from] windows::Win32Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Sysproxy {
    pub fn is_support() -> bool {
        cfg!(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "windows",
        ))
    }

    /// Check whether the system proxy is currently enabled.
    pub fn check() -> Result<bool> {
        Ok(Sysproxy::get_system_proxy()?.enable)
    }

    /// Enable the system proxy from a `"host:port"` string.
    pub fn enable(addr: &str) -> Result<()> {
        let (host, port) = addr
            .rsplit_once(':')
            .ok_or_else(|| Error::ParseStr(addr.into()))?;
        let port = port.parse::<u16>().map_err(|_| Error::ParseStr(addr.into()))?;
        Sysproxy {
            enable: true,
            host: host.into(),
            port,
            bypass: String::new(),
        }
        .set_system_proxy()
    }

    /// Disable the system proxy.
    pub fn disable() -> Result<()> {
        Sysproxy::get_system_proxy()
            .unwrap_or_default()
            .set_system_proxy_enable(false)
    }

    /// Set only the enabled state, preserving existing host/port/bypass.
    pub fn set_system_proxy_enable(&self, enable: bool) -> Result<()> {
        Sysproxy { enable, ..self.clone() }.set_system_proxy()
    }
}

impl Autoproxy {
    pub fn is_support() -> bool {
        cfg!(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "windows",
        ))
    }
}
