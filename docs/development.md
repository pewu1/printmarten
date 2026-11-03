# Developing PrintMarten

Rust is pinned by `rust-toolchain.toml`; dependencies by `Cargo.lock`.

```sh
cargo run --locked
cargo run --locked -- /path/to/photo.jpg /path/to/another.png
cargo run --locked -- --version
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```

Linux also needs native development dependencies:

```sh
sudo apt install libgtk-4-dev libgl1-mesa-dev libegl1-mesa-dev libxkbcommon-dev \
  libwayland-dev libx11-dev libxcb1-dev pkg-config dpkg-dev
```

Windows native dependencies require Visual Studio 2022's x64 C++ tools.
macOS native dependencies require Xcode Command Line Tools.

Build the HEIC runtime into the project (ordinary image imports and `--version`
do not require it):

```sh
python3 -m venv target/tools
# Windows: target/tools/Scripts/activate
source target/tools/bin/activate
python -m pip install --upgrade pip
python -m pip install -r scripts/requirements.txt
python scripts/build_native.py
```

The script downloads SHA-256-verified sources and builds separate libheif and
libde265 libraries without installing them system-wide. Development builds find
`target/native/lib`; `PRINTMARTEN_NATIVE_LIB_DIR` overrides the lookup directory.
On Windows, point that variable at `target/native/bin` if necessary. Source
archives are cached in `target/native-sources`.
The build applies the pinned libde265 source fixes in
`scripts/native_source_fixes.py`, including CMake policy compatibility and C++17
annotations for declarations used only in debug or experimental code. Compiler
warnings remain enabled. Local and CI builds upgrade pip to the newest release
supported by their Python interpreter before installing packaging tools.

To package locally after building native dependencies:

```sh
cargo build --release --locked
cargo install cargo-about --version 0.8.2 --locked
cargo about generate --locked packaging/licenses.hbs --output-file target/THIRD_PARTY_LICENSES.html
python scripts/package.py --platform macos --arch x64 --sha local
python scripts/validate_packages.py --platform macos --arch x64
```

Use `--platform windows --arch x64` or `--platform linux --arch x64` on their
respective operating systems. Windows packaging also requires NSIS 3.11. Install the Microsoft Visual C++
v14 x64 runtime separately for execution and package smoke checks; it is not
redistributed inside GPL application packages. CI builds each architecture on a
native runner, including separate Intel and Apple Silicon macOS runners.

After all four builds succeed on `main`, GitHub Actions publishes a normal
release tagged `main-<commit SHA>` and marks it as latest. The release includes
the application packages, checksums, corresponding application source, and
native codec source archives. Manual workflow runs on `main` use the same path.

## Architecture and validation

`model` holds physical units and photo instances; `layout` packs rectangles;
`imaging` decodes and color-converts sources; `output` renders PDF and raster
pages; `printing` retains platform print settings between selection and
submission. macOS and Linux print full-resolution image XObjects with vector
cut lines. Windows uses page rasters at 300 DPI through GDI. Preview uses the
same placement, crop, rotation, and contain geometry.

The Linux print dialog runs in a helper process to keep GTK's main context
separate from eframe/winit. It holds `GtkPrintSetup` while the user reviews any
layout changes, then submits the generated PDF without reopening the dialog.

Automated tests cover geometry, packing, sizes/copies, imports, PDF dimensions,
and printer-setting reconciliation. CI also validates package structure,
architecture, checksums, installation where supported, and headless execution.
GUI dialogs, physical output, printer-specific paper substitution, unusual
profiles/HDR HEIC files, and Linux desktop integration require manual validation
on the actual target systems; compilation alone does not establish those results.

See [validation.md](validation.md) for the manual checklist and recorded results.

## Licensing and source releases

PrintMarten is GPL-3.0-only. See [the dependency audit](license-audit.md) and
[third-party notices](../THIRD_PARTY_NOTICES.md). Re-run cargo-about against the
locked graph whenever dependencies change. The allowlist is not a substitute
for reviewing license compatibility and bundled native components.

After building the native libraries, generate complete corresponding source:

```sh
python scripts/package_source.py --sha local --offline
```

This packages the current source files, Cargo.lock, vendored Rust crates, native
codec source archives, assets, and build/install scripts. CI publishes this archive
with its checksum beside binary downloads. Keep it available for as long as the
matching binaries are distributed. System SDKs and general-purpose build tools
remain prerequisites.

## One-command local packages

Run on the matching native OS and CPU (cross compilation is not supported):

- Intel macOS: `./scripts/build-macos-intel.sh`
- Apple Silicon macOS: `./scripts/build-macos-apple-silicon.sh`
- Windows x64, from a Visual Studio x64 developer PowerShell: `./scripts/build-windows-x64.ps1`
- Linux x64: `./scripts/build-linux-x64.sh`

Install Python 3, the pinned Rust toolchain, and the OS dependencies listed above.
Windows additionally needs NSIS on PATH. Scripts create `target/local-tools`,
build the pinned HEIC runtime and release executable, generate license notices,
and package and validate the artifacts in `dist/`. The macOS DMGs are ad-hoc
signed. No publishing or notarization is performed. Network access is needed
for tools and pinned source downloads on the first build.

On Linux, PrintMarten selects X11 (XWayland on a Wayland desktop) whenever
DISPLAY is present, because winit 0.30 does not deliver native Wayland file drops.
On Wayland-only systems the application still starts, but use Add photos/folder
instead of dropping files. GTK printing continues to use its own helper process.
