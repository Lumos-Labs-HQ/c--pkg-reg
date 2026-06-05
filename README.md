# cpkg — A Package Manager for C/C++

[![Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![Packages](https://img.shields.io/badge/packages-2831%20via%20vcpkg%20catalog-green.svg)](https://vcpkg.io)

`cpkg` is what npm is for Node — but for C and C++. One command to install any library, a local `.cpkg/` directory that keeps your project self-contained, and a `cpkg.toml` that tracks everything.

No CMake wrangling. No manual header copying. No system pollution.

---

## Why cpkg?

C++ has never had a real package manager. Every library means:
- Download a tarball manually
- Figure out the build system
- Copy headers somewhere your compiler can find them
- Repeat for every machine and every project

cpkg automates all of it. It pulls from the **vcpkg catalog** (2831 packages, maintained by Microsoft) to find the correct GitHub repo for any library, downloads the release tarball, builds if needed, and symlinks headers and libs into `.cpkg/` — ready to use with `-I.cpkg/include -L.cpkg/lib`.

---

## Installation

```bash
git clone <repo-url>
cd cpkg
cargo build --release
# add target/release/cpkg to your PATH
```

---

## Quick Start

```bash
# Create a new project
cpkg init my-app
cd my-app

# Install libraries
cpkg install fmt
cpkg install nlohmann-json
cpkg install yyjson

# Write your code, then build and run
cpkg build
cpkg run run
```

That's it. No other config needed.

---

## cpkg.toml

```toml
[package]
name = "my-app"
version = "0.1.0"

[[dependencies]]
name = "fmt"
version = "*"

[[dependencies]]
name = "nlohmann-json"
version = "^3.11"

[[scripts]]
name = "build"
command = "g++ -std=c++17 src/main.cpp -I.cpkg/include -L.cpkg/lib -lfmt -o build/app"

[[scripts]]
name = "run"
command = "./build/app"

[[scripts]]
name = "test"
command = "g++ -std=c++17 src/test.cpp -I.cpkg/include -L.cpkg/lib -lfmt -o build/test && ./build/test"
```

### Version constraints

```toml
version = "*"        # any version
version = "^9.0"     # 9.x.x  (compatible)
version = ">=1.11"   # minimum
version = "=3.11.3"  # exact
```

---

## Using Installed Packages

After `cpkg install`, every library is available via two flags:

```bash
g++ -std=c++17 main.cpp -I.cpkg/include -L.cpkg/lib -lfmt -o app
```

Include paths match each library's documented style — no surprises:

```cpp
#include <fmt/core.h>           // fmt
#include <nlohmann/json.hpp>    // nlohmann-json
#include <magic_enum/magic_enum.hpp>  // magic-enum
#include <yyjson.h>             // yyjson
#include <spdlog/spdlog.h>      // spdlog
#include <Eigen/Dense>          // eigen
```

---

## Commands

| Command | Description |
|---|---|
| `cpkg init [path]` | Create a new `cpkg.toml` |
| `cpkg install <name>` | Install a package locally |
| `cpkg install <name> --version "^9.0"` | Install with version constraint |
| `cpkg install <name> --global` | Install globally (shared across projects) |
| `cpkg remove <name>` | Remove a package |
| `cpkg build` | Run the `build` script |
| `cpkg run <script>` | Run any named script |
| `cpkg compilers` | List detected C++ compilers |
| `cpkg -v <command>` | Verbose output |

---

## Directory Layout

```
my-app/
├── cpkg.toml               # project config + scripts
├── cpkg.lock               # pinned versions (commit this)
├── src/
│   └── main.cpp
└── .cpkg/                  # like node_modules — don't commit
    ├── packages/           # extracted source
    │   ├── fmt-12.1.0/
    │   └── yyjson-0.12.0/
    ├── include/            # symlinked headers
    │   ├── fmt/
    │   └── yyjson.h
    ├── lib/                # symlinked libraries
    │   ├── libfmt.a
    │   └── libyyjson.a
    └── cache/              # downloaded tarballs + vcpkg catalog
```

Add `.cpkg/` to your `.gitignore`. Commit `cpkg.lock`.

---

## Global Installs

```bash
cpkg install nlohmann-json --global
```

| Platform | Location |
|---|---|
| Linux | `~/.local/share/cpkg/` |
| macOS | `~/Library/Application Support/cpkg/` |
| Windows | `%APPDATA%\cpkg\` |

---

## Package Discovery

cpkg uses the **vcpkg catalog** (`vcpkg.io/output.json`) to resolve plain package names to their correct GitHub repos. The catalog covers 2831 packages and is cached locally for 24 hours.

```bash
cpkg install eigen          # resolves via vcpkg catalog → libeigen/eigen
cpkg install libeigen/eigen # explicit owner/repo — always works
```

If a package isn't in the vcpkg catalog, cpkg falls back to GitHub search (filtered to C/C++ repos by stars).

---

## Performance

Benchmarked on a typical broadband connection, installing `yyjson` (1.6 MB tarball):

| Scenario | cpkg | vcpkg |
|---|---|---|
| Cold install (first ever) | **~17s** | 2–10 min |
| Warm install (catalog cached) | **~4s** | 30s–2 min |
| Already installed | **~0.3s** | — |

**Why cpkg is faster:**
- vcpkg compiles every package from source using its full CMake toolchain
- cpkg symlinks headers directly — for header-only libs (fmt, eigen, nlohmann, magic_enum, …) there's nothing to compile at all
- For compiled libs, cpkg only runs cmake configure + build once, then symlinks the `.a`

---

## How Install Works

1. Look up package name in vcpkg catalog → get `owner/repo`
2. Query GitHub tags → pick best version matching your constraint
3. Download release tarball → cache in `.cpkg/cache/`
4. Extract to `.cpkg/packages/<name>-<version>/`
5. Detect build system: CMake → Makefile → skip (header-only)
6. Symlink `include/` subdirs directly into `.cpkg/include/`
7. Symlink `.a`/`.so` files into `.cpkg/lib/`
8. Update `cpkg.toml` and `cpkg.lock`

---

## Building from Source

```bash
# Requires Rust toolchain (rustup.rs)
cargo build --release
cargo test
```

---

## Architecture

| Module | Purpose |
|---|---|
| `src/cli.rs` + `src/main.rs` | CLI parsing and dispatch |
| `src/types.rs` | All shared structs and enums |
| `src/errors.rs` | Unified error type |
| `src/paths.rs` | `.cpkg/` layout, global/local resolution |
| `src/config/resolver.rs` | vcpkg catalog lookup + GitHub tag resolution |
| `src/config/project.rs` | `cpkg.toml` load/save |
| `src/config/lockfile.rs` | `cpkg.lock` load/save |
| `src/package/mod.rs` | Install/uninstall orchestration |
| `src/package/fetch.rs` | Tarball download |
| `src/package/extract.rs` | `.tar.gz` extraction |
| `src/package/build.rs` | CMake / Makefile build |
| `src/package/link.rs` | Symlink or copy headers and libs |
| `src/compiler/detect.rs` | Auto-detect g++, clang++, MSVC |
| `src/init.rs` | Project scaffolding |
| `src/script.rs` | Script runner |

---

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for project structure, how the resolver and linker work, and the PR checklist.

---

## License

## License

MIT — see [LICENSE](./LICENSE)
