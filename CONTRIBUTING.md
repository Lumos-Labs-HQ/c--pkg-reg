# Contributing to cpkg

## Project structure

```
src/
├── main.rs          — entry point, CLI dispatch
├── cli.rs           — clap command definitions
├── types.rs         — all shared structs (CpkgToml, LockedPackage, etc.)
├── errors.rs        — CpkgError enum + CpkgResult<T>
├── paths.rs         — .cpkg/ directory layout
├── init.rs          — cpkg init scaffolding
├── script.rs        — cpkg run / cpkg build
├── config/
│   ├── resolver.rs  — vcpkg catalog lookup + GitHub tag resolution
│   ├── project.rs   — cpkg.toml load/save
│   └── lockfile.rs  — cpkg.lock load/save
├── package/
│   ├── mod.rs       — install/uninstall orchestration
│   ├── fetch.rs     — tarball download
│   ├── extract.rs   — .tar.gz extraction
│   ├── build.rs     — CMake / Makefile build
│   └── link.rs      — symlink headers and libs
└── compiler/
    └── detect.rs    — auto-detect g++, clang++, MSVC
```

## Adding support for a new package

Most packages resolve automatically via the vcpkg catalog. If a package has an unusual homepage URL (not `github.com`) you can add it to `priority_repo()` in `src/config/resolver.rs` — but this should be rare.

## Working on the resolver

The resolution flow is:
1. Plain name → vcpkg catalog (`vcpkg.io/output.json`, cached 24h) → extract `owner/repo` from `homepage` field
2. Not in catalog → GitHub search, filter by C/C++ language
3. Explicit `owner/repo` → skip catalog, query tags directly

The catalog is cached at `.cpkg/cache/vcpkg-catalog.json`. Delete it to force a refresh.

## Working on install / linking

The install pipeline lives in `src/package/mod.rs`:
- `find_include_dir` — heuristic to locate headers inside an extracted tarball
- `link_include_tree` — links each subdir of `include/` directly into `.cpkg/include/` so `#include <lib/header.h>` works without any prefix
- `find_lib_dir` — walks the build output for `.a`/`.so` files

When adding a new linking strategy or heuristic, make sure it handles these cases:
- Headers at `include/<namespace>/` (e.g. `include/nlohmann/`) → link `nlohmann/` directly
- Headers at `include/*.h` (e.g. `include/yyjson.h`) → link the whole dir as `<pkg-name>/`
- Header-only libs with no `include/` → fall back to `src/` or root

## Running locally

```bash
cargo build --release
ln -sf $(pwd)/target/release/cpkg ~/.local/bin/cpkg
```

## Tests

```bash
cargo test
```

Integration tests live in `tests/`. Unit tests are inline with `#[cfg(test)]` modules.

## Pull request checklist

- `cargo build` passes with no errors or warnings
- New packages resolve correctly via vcpkg catalog (test with `cpkg install <name>`)
- Include paths match the library's documented `#include` style
- No new hardcoded package entries unless the vcpkg catalog genuinely cannot cover it
