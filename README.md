# cpkg — A C++ Package Manager

[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)]()

`cpkg` is a package manager for C++ — like npm for Node or pip for Python, but built for the C++ ecosystem. It detects compilers, manages dependencies locally, runs build scripts, and keeps your project portable. Written in Rust.

## Why?

C++ has no standard package manager. Every library you use requires manual download, CMake configuration, and path wrangling. `cpkg` automates this: one command to install, one config file to track everything, and a local `.cpkg/` directory (like `node_modules`) that keeps dependencies isolated per project.

## Features

- **Compiler detection** — auto-detects g++, clang++, and MSVC on any platform
- **Local dependency isolation** — packages live in `.cpkg/`, never pollute your system
- **Global installs** — `--global` flag installs to a platform-standard shared location
- **Declarative config** — `cpkg.toml` tracks project metadata, dependencies, C++ standard, compiler preferences, and build scripts
- **Lockfile** — `cpkg.lock` pins exact versions for reproducible builds
- **Script runner** — `cpkg run build` / `cpkg run test` just like npm scripts
- **Semver resolution** — version constraints like `^9.0` or `>=1.11`
- **Cross-platform** — symlinks on Unix, copy on Windows
- **Build system aware** — auto-detects CMake and Makefile; treats header-only libs correctly

## Installation

```bash
# Install from source (requires Rust toolchain)
git clone <repo-url>
cd cpkg
cargo build --release
```

The binary will be at `target/release/cpkg`. Add it to your PATH.

## Quick Start

```bash
# Create a new project
cpkg init my-app
cd my-app

# See what compilers are available
cpkg compilers

# Install a dependency (header-only library)
cpkg install fmt

# Or a specific version
cpkg install spdlog --version ">=1.11"

# Install globally (available to all projects)
cpkg install nlohmann_json --global

# Run your build
cpkg run build

# Remove a dependency
cpkg remove fmt
```

## cpkg.toml — Project Configuration

```toml
[package]
name = "my-app"
version = "0.1.0"
description = "My C++ project"
authors = ["You <you@example.com>"]

[dependencies]
fmt = "^9.0"
spdlog = ">=1.11"

[compiler]
kind = "gcc"
min_version = "12.0"

cpp_standard = "c++17"

[[scripts]]
name = "build"
command = "g++ -std=c++17 -O2 src/*.cpp -I.cpkg/include -o build/app"

[[scripts]]
name = "run"
command = "./build/app"

[[scripts]]
name = "test"
command = "g++ -std=c++17 src/test/*.cpp -o build/test && ./build/test"
```

### Dependencies Format

Dependencies use the TOML inline table syntax (name = version):

```toml
[dependencies]
fmt = "^9.0"           # semver constraint — any 9.x.x
spdlog = ">=1.11"      # minimum version
nlohmann_json = "*"    # any version
```

You can also use the full table form:

```toml
[[dependencies]]
name = "fmt"
version = "^9.0"
global = false         # local install (default)
```

## Scripts

Scripts are shell commands stored in `cpkg.toml`:

```toml
[[scripts]]
name = "build"
command = "g++ -std=c++17 -O2 src/*.cpp -o build/app"
```

Run them with:

```bash
cpkg run build        # executes the command via sh -c
cpkg run test         # run your test script
cpkg build            # shortcut — runs the "build" script
```

Extra arguments are forwarded:

```bash
cpkg run build -- --extra-flag
```

## How It Works

### Directory Layout

```
my-app/
├── cpkg.toml               # project config
├── cpkg.lock               # pinned dependency versions
├── src/
│   └── main.cpp
└── .cpkg/                  # local dependency environment
    ├── packages/           # full extracted packages
    │   ├── fmt-9.0.0/
    │   └── spdlog-1.14.0/
    ├── include/            # symlinked include tree
    │   ├── fmt/    → ../packages/fmt-9.0.0/include/fmt/
    │   └── spdlog/ → ../packages/spdlog-1.14.0/include/spdlog/
    ├── lib/                # symlinked libraries
    ├── cache/              # downloaded tarballs
    └── bin/                # package executables
```

Add `-I.cpkg/include` to your compiler flags and all installed headers are available.

### Global Install Locations

| Platform | Path |
|----------|------|
| Linux    | `~/.local/share/cpkg/` |
| macOS    | `~/Library/Application Support/cpkg/` |
| Windows  | `%APPDATA%\cpkg\` |

### Install Flow

1. Resolve version via semver against registry
2. Download tarball → cache in `.cpkg/cache/`
3. Verify SHA-256 checksum (if available)
4. Extract to `.cpkg/packages/<name>-<version>/`
5. Detect and run build system (CMake → Makefile → skip for header-only)
6. Symlink includes to `.cpkg/include/<name>/`
7. Symlink libraries to `.cpkg/lib/`
8. Update `cpkg.toml` dependencies and `cpkg.lock`

## Commands

| Command | Description |
|---------|-------------|
| `cpkg init [path]` | Create a new cpkg.toml |
| `cpkg compilers` | List detected C++ compilers |
| `cpkg install <name>` | Install a package locally |
| `cpkg install <name> --global` | Install globally |
| `cpkg install <name> --version "<constraint>"` | Pin version |
| `cpkg remove <name>` | Remove a package |
| `cpkg run <script>` | Execute a named script |
| `cpkg build` | Run the "build" script |
| `cpkg --help` | Show help |
| `cpkg -v <command>` | Verbose output |

## Building from Source

```bash
# Prerequisites: Rust toolchain (rustup)
cargo build --release
cargo test
```

## Architecture

The codebase is modular with separated concerns:

| Module | Purpose |
|--------|---------|
| `src/types.rs` | All shared structs, enums |
| `src/errors.rs` | Unified error types |
| `src/paths.rs` | .cpkg/ layout, global/local resolution |
| `src/compiler/` | Compiler detection and version parsing |
| `src/config/` | cpkg.toml, cpkg.lock, semver resolver |
| `src/package/` | Fetch, verify, extract, build, link |
| `src/script.rs` | Script execution |
| `src/init.rs` | Project scaffolding |
| `src/cli.rs` + `src/main.rs` | CLI dispatch |

## License

MIT OR Apache-2.0
