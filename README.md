# netr

A Rust crate for cross-platform network libraries and utilities. The goal is to build a comprehensive collection of network tools, libraries, and helpers — all under one dependency.

## Installation

```toml
[dependencies]
netr = "0.1"
```

Disable the `utils` feature to drop the `iptools` dependency:

```toml
netr = { version = "0.1", default-features = false }
```

## Usage

All types are available directly at the crate root:

```rust
use netr::{Sysproxy, Autoproxy};
```

### Manual Proxy

```rust
use netr::Sysproxy;

// Read current system proxy
let proxy = Sysproxy::get_system_proxy()?;
println!("enabled={} host={} port={}", proxy.enable, proxy.host, proxy.port);

// Set a proxy
Sysproxy {
    enable: true,
    host: "127.0.0.1".into(),
    port: 7890,
    bypass: "localhost,127.0.0.1/8".into(),
}.set_system_proxy()?;

// Disable
Sysproxy { enable: false, ..Default::default() }.set_system_proxy()?;
```

### Auto-Proxy (PAC)

```rust
use netr::Autoproxy;

// Set a PAC URL
Autoproxy {
    enable: true,
    url: "http://example.com/proxy.pac".into(),
}.set_auto_proxy()?;

// Read current PAC setting
let auto = Autoproxy::get_auto_proxy()?;
println!("url={} enabled={}", auto.url, auto.enable);
```

### CIDR to Wildcard

Useful for building bypass lists. Requires the `utils` feature (on by default).

```rust
use netr::utils::ipv4_cidr_to_wildcard;

let w = ipv4_cidr_to_wildcard("192.168.1.0/24")?; // ["192.168.1.*"]
let w = ipv4_cidr_to_wildcard("10.0.0.0/8")?;     // ["10.*"]
```

## API

```rust
// Types
netr::Sysproxy       // Manual proxy config
netr::Autoproxy      // PAC / auto-proxy config
netr::Error          // Error type
netr::Result<T>      // Result alias

// Sysproxy
Sysproxy::get_system_proxy() -> Result<Sysproxy>
Sysproxy::set_system_proxy(&self) -> Result<()>
Sysproxy::is_support() -> bool

// Autoproxy
Autoproxy::get_auto_proxy() -> Result<Autoproxy>
Autoproxy::set_auto_proxy(&self) -> Result<()>
Autoproxy::is_support() -> bool

// utils (feature = "utils")
netr::utils::ipv4_cidr_to_wildcard(cidr: &str) -> Result<Vec<String>>
```

## Platform Support

| Platform | Backend |
|----------|---------|
| macOS | `networksetup` |
| Windows | `WinInet` API + registry |
| Linux (GNOME) | `gsettings` |
| Linux (KDE) | `kreadconfig5/6` + `kwriteconfig5/6` |

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `utils` | ✅ | CIDR/IP utility helpers (pulls in `iptools`) |

## Development

### Prerequisites

- Rust 1.80+
- macOS: `networksetup` (built-in)
- Linux: `gsettings` (GNOME) or `kreadconfig5`/`kreadconfig6` (KDE)
- Windows: no extra dependencies

### Build

```bash
cargo build
cargo build --release
```

### Test

```bash
# Run all tests
cargo test

# Run only unit tests — safe, does not touch system proxy settings
cargo test utils

# Run integration tests — temporarily modifies your system proxy
cargo test -- --test-threads=1
```

> `test_system_enable` and `test_auto_enable` modify your system proxy settings during the test and restore a disabled state when complete. They are serialized with `serial_test` to avoid races.

### Lint & Format

```bash
cargo fmt
cargo clippy
```

## Publish

**1. Check the package is valid:**

```bash
cargo package --list   # preview files included in the published crate
cargo publish --dry-run
```

**2. Login to crates.io (one-time):**

```bash
cargo login
# paste your API token from https://crates.io/settings/tokens
```

**3. Bump the version in `Cargo.toml`, then publish:**

```bash
cargo publish
```

## Project Structure

```
netr/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs              # Flat re-exports (pub use sysproxy::*)
│   └── sysproxy/
│       ├── mod.rs          # Types, Error, Result
│       ├── macos.rs
│       ├── windows.rs
│       ├── linux.rs
│       └── utils.rs
└── tests/
    └── test.rs
```

Internal modules live under `src/<module>/` but are never exposed as nested paths — everything is re-exported flat at `netr::*`. When a new module grows too many types, it gets its own subdirectory following the same pattern.

## Roadmap

| Type | Description | Status |
|------|-------------|--------|
| `Sysproxy` / `Autoproxy` | OS system proxy get/set | ✅ Stable |
| `NetInfo` | Network interface enumeration and stats | Planned |
| `DnsResolver` | Async DNS resolver with caching | Planned |
| `PortScanner` | TCP/UDP port scanner | Planned |
| `ProxyChecker` | Proxy health checker (latency, anonymity) | Planned |
| `PacFile` | PAC file parser and evaluator | Planned |
| `NetMonitor` | Real-time bandwidth and connection monitor | Planned |
| `Traceroute` | Cross-platform traceroute | Planned |
| `HttpProbe` | HTTP connectivity prober | Planned |

## Contributing

To add a new module:

1. Create `src/<module>/mod.rs` with its types and logic
2. Add platform-specific files as `src/<module>/<platform>.rs` if needed
3. Re-export the public types in `src/lib.rs`
4. Add an entry to the Roadmap table above
5. Gate heavy dependencies behind an optional feature flag

## License

MIT
