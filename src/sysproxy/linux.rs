use super::{Autoproxy, Error, Result, Sysproxy};
use std::{env, process::Command, str::from_utf8, sync::LazyLock};

const CMD_KEY: &str = "org.gnome.system.proxy";

static IS_APPIMAGE: LazyLock<bool> = LazyLock::new(|| std::env::var("APPIMAGE").is_ok());

impl Sysproxy {
    pub fn get_system_proxy() -> Result<Sysproxy> {
        let enable = Sysproxy::get_enable()?;

        let mut socks = get_proxy("socks")?;
        let https = get_proxy("https")?;
        let http = get_proxy("http")?;

        // Fix #5 (linux equivalent): prefer HTTP over HTTPS, only fall back to
        // HTTPS when HTTP is also unset.
        if socks.host.is_empty() {
            if !http.host.is_empty() {
                socks.host = http.host;
                socks.port = http.port;
            } else if !https.host.is_empty() {
                socks.host = https.host;
                socks.port = https.port;
            }
        }

        socks.enable = enable;
        socks.bypass = Sysproxy::get_bypass().unwrap_or("".into());

        Ok(socks)
    }

    pub fn set_system_proxy(&self) -> Result<()> {
        self.set_enable()?;

        // Fix #12: always set host/port and bypass so that disabling also
        // clears the stored values rather than leaving stale ones.
        self.set_socks()?;
        self.set_https()?;
        self.set_http()?;
        self.set_bypass()?;

        Ok(())
    }

    pub fn get_enable() -> Result<bool> {
        if is_kde() {
            let xdg_dir = xdg::BaseDirectories::new()?;
            let config = xdg_dir.get_config_file("kioslaverc");
            let config = config.to_str().ok_or(Error::ParseStr("config".into()))?;

            let mode = kreadconfig()
                .args([
                    "--file",
                    config,
                    "--group",
                    "Proxy Settings",
                    "--key",
                    "ProxyType",
                ])
                .output()?;
            let mode = from_utf8(&mode.stdout)
                .or(Err(Error::ParseStr("mode".into())))?
                .trim();
            Ok(mode == "1")
        } else {
            let mode = gsettings().args(["get", CMD_KEY, "mode"]).output()?;
            let mode = from_utf8(&mode.stdout)
                .or(Err(Error::ParseStr("mode".into())))?
                .trim();
            Ok(mode == "'manual'")
        }
    }

    pub fn get_bypass() -> Result<String> {
        if is_kde() {
            let xdg_dir = xdg::BaseDirectories::new()?;
            let config = xdg_dir.get_config_file("kioslaverc");
            let config = config.to_str().ok_or(Error::ParseStr("config".into()))?;

            let bypass = kreadconfig()
                .args([
                    "--file",
                    config,
                    "--group",
                    "Proxy Settings",
                    "--key",
                    "NoProxyFor",
                ])
                .output()?;
            let bypass = from_utf8(&bypass.stdout)
                .or(Err(Error::ParseStr("bypass".into())))?
                .trim();

            let bypass = bypass
                .split(',')
                .map(|h| strip_str(h.trim()))
                .collect::<Vec<&str>>()
                .join(",");

            Ok(bypass)
        } else {
            let bypass = gsettings()
                .args(["get", CMD_KEY, "ignore-hosts"])
                .output()?;
            let bypass = from_utf8(&bypass.stdout)
                .or(Err(Error::ParseStr("bypass".into())))?
                .trim();

            let bypass = bypass.strip_prefix('[').unwrap_or(bypass);
            let bypass = bypass.strip_suffix(']').unwrap_or(bypass);

            let bypass = bypass
                .split(',')
                .map(|h| strip_str(h.trim()))
                .collect::<Vec<&str>>()
                .join(",");

            Ok(bypass)
        }
    }

    pub fn get_http() -> Result<Sysproxy> {
        get_proxy("http")
    }

    pub fn get_https() -> Result<Sysproxy> {
        get_proxy("https")
    }

    pub fn get_socks() -> Result<Sysproxy> {
        get_proxy("socks")
    }

    pub fn set_enable(&self) -> Result<()> {
        if is_kde() {
            let xdg_dir = xdg::BaseDirectories::new()?;
            let config = xdg_dir.get_config_file("kioslaverc");
            let config = config.to_str().ok_or(Error::ParseStr("config".into()))?;
            let mode = if self.enable { "1" } else { "0" };
            kwriteconfig()
                .args([
                    "--file",
                    config,
                    "--group",
                    "Proxy Settings",
                    "--key",
                    "ProxyType",
                    mode,
                ])
                .status()?;
            let gmode = if self.enable { "'manual'" } else { "'none'" };
            gsettings().args(["set", CMD_KEY, "mode", gmode]).status()?;
        } else {
            let mode = if self.enable { "'manual'" } else { "'none'" };
            gsettings().args(["set", CMD_KEY, "mode", mode]).status()?;
        }
        Ok(())
    }

    pub fn set_bypass(&self) -> Result<()> {
        // Fix #13: escape single quotes inside host values to avoid breaking
        // GVariant string syntax.
        let to_gvariant_list = |bypass: &str| -> String {
            let entries = bypass
                .split(',')
                .map(|h| {
                    let h = h.trim().replace('\'', "\\'");
                    format!("'{h}'")
                })
                .collect::<Vec<String>>()
                .join(", ");
            format!("[{entries}]")
        };

        if is_kde() {
            let xdg_dir = xdg::BaseDirectories::new()?;
            let config = xdg_dir.get_config_file("kioslaverc");
            let config = config.to_str().ok_or(Error::ParseStr("config".into()))?;

            let bypass = to_gvariant_list(&self.bypass);
            gsettings()
                .args(["set", CMD_KEY, "ignore-hosts", bypass.as_str()])
                .status()?;

            kwriteconfig()
                .args([
                    "--file",
                    config,
                    "--group",
                    "Proxy Settings",
                    "--key",
                    "NoProxyFor",
                    self.bypass.as_str(),
                ])
                .status()?;
        } else {
            let bypass = to_gvariant_list(&self.bypass);
            gsettings()
                .args(["set", CMD_KEY, "ignore-hosts", bypass.as_str()])
                .status()?;
        }
        Ok(())
    }

    pub fn set_http(&self) -> Result<()> {
        set_proxy(self, "http")
    }

    pub fn set_https(&self) -> Result<()> {
        set_proxy(self, "https")
    }

    pub fn set_socks(&self) -> Result<()> {
        set_proxy(self, "socks")
    }
}

/// Returns true when the current desktop is KDE Plasma.
/// `XDG_CURRENT_DESKTOP` may be colon-separated (e.g. "KDE:GNOME"), so we
/// check each component instead of doing an exact string match. (Fix #10)
fn is_kde() -> bool {
    env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .split(':')
        .any(|s| s.trim() == "KDE")
}

fn gsettings() -> Command {
    let mut command = Command::new("gsettings");
    if *IS_APPIMAGE {
        command.env_remove("LD_LIBRARY_PATH");
    }
    command
}

fn kreadconfig() -> Command {
    let command = match env::var("KDE_SESSION_VERSION").unwrap_or_default().as_str() {
        "6" => "kreadconfig6",
        _ => "kreadconfig5",
    };
    let mut command = Command::new(command);
    if *IS_APPIMAGE {
        command.env_remove("LD_LIBRARY_PATH");
    }
    command
}

fn kwriteconfig() -> Command {
    let command = match env::var("KDE_SESSION_VERSION").unwrap_or_default().as_str() {
        "6" => "kwriteconfig6",
        _ => "kwriteconfig5",
    };
    let mut command = Command::new(command);
    if *IS_APPIMAGE {
        command.env_remove("LD_LIBRARY_PATH");
    }
    command
}

fn set_proxy(proxy: &Sysproxy, service: &str) -> Result<()> {
    if is_kde() {
        let schema = format!("{CMD_KEY}.{service}");

        // Fix #13: escape single quotes in host for GVariant string syntax.
        let host_escaped = proxy.host.replace('\'', "\\'");
        let host_gvariant = format!("'{host_escaped}'");
        let port = format!("{}", proxy.port);

        gsettings()
            .args(["set", schema.as_str(), "host", host_gvariant.as_str()])
            .status()?;
        gsettings()
            .args(["set", schema.as_str(), "port", port.as_str()])
            .status()?;

        let xdg_dir = xdg::BaseDirectories::new()?;
        let config = xdg_dir.get_config_file("kioslaverc");
        let config = config.to_str().ok_or(Error::ParseStr("config".into()))?;

        let key = format!("{service}Proxy");

        // Fix #11: use standard "scheme://host:port" format instead of the
        // non-standard space-separated "scheme://host port" format.
        let scheme = match service {
            "socks" => "socks",
            _ => "http",
        };
        let host = proxy.host.as_str();
        let value = format!("{scheme}://{host}:{port}");

        kwriteconfig()
            .args([
                "--file",
                config,
                "--group",
                "Proxy Settings",
                "--key",
                key.as_str(),
                value.as_str(),
            ])
            .status()?;

        Ok(())
    } else {
        let schema = format!("{CMD_KEY}.{service}");

        // Fix #13: escape single quotes in host for GVariant string syntax.
        let host_escaped = proxy.host.replace('\'', "\\'");
        let host_gvariant = format!("'{host_escaped}'");
        let port = format!("{}", proxy.port);

        gsettings()
            .args(["set", schema.as_str(), "host", host_gvariant.as_str()])
            .status()?;
        gsettings()
            .args(["set", schema.as_str(), "port", port.as_str()])
            .status()?;

        Ok(())
    }
}

fn get_proxy(service: &str) -> Result<Sysproxy> {
    if is_kde() {
        let xdg_dir = xdg::BaseDirectories::new()?;
        let config = xdg_dir.get_config_file("kioslaverc");
        let config = config.to_str().ok_or(Error::ParseStr("config".into()))?;

        let key = format!("{service}Proxy");

        let output = kreadconfig()
            .args(["--file", config, "--group", "Proxy Settings", "--key", key.as_str()])
            .output()?;
        let value = from_utf8(&output.stdout)
            .or(Err(Error::ParseStr("schema".into())))?
            .trim();

        // Fix #9: when no proxy is configured kreadconfig returns an empty
        // string; return a default rather than failing with ParseStr.
        if value.is_empty() {
            return Ok(Sysproxy {
                enable: false,
                host: String::new(),
                port: 0,
                bypass: String::new(),
            });
        }

        // Fix #11: handle both the old space-separated format ("host port") and
        // the standard colon-separated format ("scheme://host:port").
        let addr = value
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .trim_start_matches("socks://")
            .trim_start_matches("socks5://");

        let (host, port) = if let Some((h, p)) = addr.split_once(':') {
            // Standard "host:port" format.
            (h, p.parse().unwrap_or(0u16))
        } else if let Some((h, p)) = addr.split_once(' ') {
            // Legacy space-separated "host port" format written by older versions.
            (h, p.parse().unwrap_or(0u16))
        } else {
            return Err(Error::ParseStr(value.to_string()));
        };

        Ok(Sysproxy {
            enable: false,
            host: strip_str(host).to_string(),
            port,
            bypass: String::new(),
        })
    } else {
        let schema = format!("{CMD_KEY}.{service}");

        let host = gsettings().args(["get", schema.as_str(), "host"]).output()?;
        let host = from_utf8(&host.stdout)
            .or(Err(Error::ParseStr("host".into())))?
            .trim();
        let host = strip_str(host);

        let port = gsettings().args(["get", schema.as_str(), "port"]).output()?;
        let port = from_utf8(&port.stdout)
            .or(Err(Error::ParseStr("port".into())))?
            .trim();
        let port = port.parse().unwrap_or(0u16);

        Ok(Sysproxy {
            enable: false,
            host: host.to_string(),
            port,
            bypass: String::new(),
        })
    }
}

fn strip_str(text: &str) -> &str {
    // Fix #8 (same pattern as macos): use the stripped value as the fallback,
    // not the original, so asymmetric quoting doesn't re-introduce the prefix.
    let s = text.strip_prefix('\'').unwrap_or(text);
    s.strip_suffix('\'').unwrap_or(s)
}

impl Autoproxy {
    pub fn get_auto_proxy() -> Result<Autoproxy> {
        let (enable, url) = if is_kde() {
            let xdg_dir = xdg::BaseDirectories::new()?;
            let config = xdg_dir.get_config_file("kioslaverc");
            let config = config.to_str().ok_or(Error::ParseStr("config".into()))?;

            let mode = kreadconfig()
                .args([
                    "--file",
                    config,
                    "--group",
                    "Proxy Settings",
                    "--key",
                    "ProxyType",
                ])
                .output()?;
            let mode = from_utf8(&mode.stdout)
                .or(Err(Error::ParseStr("mode".into())))?
                .trim();
            let url = kreadconfig()
                .args([
                    "--file",
                    config,
                    "--group",
                    "Proxy Settings",
                    "--key",
                    "Proxy Config Script",
                ])
                .output()?;
            let url = from_utf8(&url.stdout)
                .or(Err(Error::ParseStr("url".into())))?
                .trim();
            (mode == "2", url.to_string())
        } else {
            let mode = gsettings().args(["get", CMD_KEY, "mode"]).output()?;
            let mode = from_utf8(&mode.stdout)
                .or(Err(Error::ParseStr("mode".into())))?
                .trim();
            let url = gsettings()
                .args(["get", CMD_KEY, "autoconfig-url"])
                .output()?;
            let url: &str = from_utf8(&url.stdout)
                .or(Err(Error::ParseStr("url".into())))?
                .trim();
            let url = strip_str(url);
            (mode == "'auto'", url.to_string())
        };

        Ok(Autoproxy { enable, url })
    }

    pub fn set_auto_proxy(&self) -> Result<()> {
        if is_kde() {
            let xdg_dir = xdg::BaseDirectories::new()?;
            let config = xdg_dir.get_config_file("kioslaverc");
            let config = config.to_str().ok_or(Error::ParseStr("config".into()))?;
            let mode = if self.enable { "2" } else { "0" };
            kwriteconfig()
                .args([
                    "--file",
                    config,
                    "--group",
                    "Proxy Settings",
                    "--key",
                    "ProxyType",
                    mode,
                ])
                .status()?;
            kwriteconfig()
                .args([
                    "--file",
                    config,
                    "--group",
                    "Proxy Settings",
                    "--key",
                    "Proxy Config Script",
                    &self.url,
                ])
                .status()?;
            let gmode = if self.enable { "'auto'" } else { "'none'" };
            gsettings().args(["set", CMD_KEY, "mode", gmode]).status()?;
            gsettings()
                .args(["set", CMD_KEY, "autoconfig-url", &self.url])
                .status()?;
        } else {
            let mode = if self.enable { "'auto'" } else { "'none'" };
            gsettings().args(["set", CMD_KEY, "mode", mode]).status()?;
            gsettings()
                .args(["set", CMD_KEY, "autoconfig-url", &self.url])
                .status()?;
        }

        Ok(())
    }
}
