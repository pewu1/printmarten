# PrintMarten

Arrange photos at exact physical sizes, automatically pack them onto sheets,
and print through your operating system's printer dialog.

## Using PrintMarten

1. Choose **Add photos…**, **Add folder…**, or drag image files/folders into the
   window. Folders include supported photos from subfolders, in filename order;
   other file types and folder symlinks are skipped. Duplicate file paths within
   one import are loaded once. Folder scanning and decoding run in the background.
2. Select paper size/orientation and a default photo size. Custom sizes accept
   centimeters, millimeters, or inches, including decimal-comma input.
3. Click a photo in the sheet preview or thumbnail list. Ctrl/Cmd-click selects
   several. Right-click for sizes, **Reset to default size**, **Crop to fill**,
   **Fit whole image**, **Add one copy**, and **Remove**.
4. Adjust **Min. margin** and **Min. gap**. Spare space is distributed evenly
   around photos and between rows. Enable **Print cut lines** if wanted.
5. Choose **Print…** and select a printer. The dialog starts with your paper
   size and orientation. If printer settings change the arrangement, inspect
   the updated sheets and choose **Print reviewed layout**.

For full-page photos, select your sheet size and enable **Full page · one photo
per sheet**. Each photo is cropped to fill one sheet; turn this off to return to
your saved individual sizes and crop/fit choices. Full-page mode omits cut lines.
Enable **Borderless** for edge-to-edge output; with it off, application and
printer margins still apply.
Choose **Photo paper** for photo stock, including A4 or custom sizes. It enables
**Borderless** by default; selecting a photo-paper size such as 10 × 15 cm or
4 × 6 in also enables it. You can turn borderless off or enable it for other
paper. Borderless uses zero application margins and keeps your normal margin
value for when borderless is off.

The print dialog starts with a matching borderless paper option when the default
printer exposes one. If necessary, select the printer's borderless paper size or
mode in its properties, along with the appropriate photo media type. The selected
printer settings must report zero unprintable margins for borderless printing;
otherwise PrintMarten asks you to change the settings. Printer support depends
on the selected paper size and media type.

Photo sizes include passport/ID formats (35 × 45 mm and 2 × 2 in), smaller ID
formats, wallet prints, and additional standard photo sizes. Presets are available
both as the default size and in each photo's **Print size** menu.

In packed-sheet mode, individual size overrides survive changes to the default
size. Copies share image data but can be resized independently. Photos and their
print rectangles rotate 90° as needed so their long edge follows the sheet's
long edge. Full-page mode uses the same image orientation rule. Physical print
sizes are preserved; a photo that cannot fit in that orientation is rejected.
Keeping long edges aligned can require more sheets than unrestricted rotation.
The layout tries several deterministic row orders, then balances each row and
the space between rows within the printable area. Margins and gaps are minimums;
remaining space is shared equally, with differences of at most one micrometer
from rounding. This does not guarantee a mathematically optimal packing.

JPEG, PNG, WebP, TIFF, BMP, HEIC, and HEIF are supported. Orientation metadata is
applied, RGB ICC profiles are converted to sRGB, and transparency is flattened
onto white. HEIF decoding uses the primary still image; RAW, animation, and
multi-frame editing are outside this version. Large images are limited during
decoding and retained source pixels are capped at 1 GB per session. Images
under 150 effective DPI show a quality warning. Jobs are session-only; closing
the application discards the arrangement without changing source photos.

Print at **100% scale**, with **one page per sheet** and driver enlargement /
borderless expansion disabled to preserve dimensions. Hardware printer margins
can reduce available space. Driver-specific enhancements can still affect the
physical output, so measure a sample when dimensional accuracy is critical.
Document copies in the system dialog duplicate the entire job; **Add one copy**
duplicates individual photos. “Submitted” means accepted by the printing system,
not confirmation that a physical sheet has finished printing.

## Downloads

Every push to `main` runs [Build PrintMarten](../../actions/workflows/build.yml).
After all platform builds and package checks pass, the workflow publishes a
prerelease for that commit in [GitHub Releases](../../releases). Open the newest
prerelease and download the package for your platform from **Assets**:

| Platform | Package | Installation |
| --- | --- | --- |
| macOS Intel | `PrintMarten-…-macos-x64-….dmg` | Open the DMG and drag PrintMarten to Applications |
| macOS Apple Silicon | `PrintMarten-…-macos-arm64-….dmg` | Open the DMG and drag PrintMarten to Applications |
| Windows x64 | `PrintMarten-…-windows-x64-…-setup.exe` | Run the per-user installer; no administrator access needed |
| Windows x64 portable | `PrintMarten-…-windows-x64-…-portable.zip` | Extract the entire ZIP and run `printmarten.exe` |
| Linux x64 | `PrintMarten-…-linux-x64-….deb` | Install with `sudo apt install ./PrintMarten-….deb` |
| Linux x64 archive | `PrintMarten-…-linux-x64-….tar.gz` | Extract and run `./PrintMarten/printmarten`; system GTK dependencies are required |

Release assets include SHA-256 checksum files and matching LGPL library source
archives. Downloads remain available in Releases beyond the 30-day workflow
artifact retention period. Each commit uses a `main-<commit SHA>` release tag;
rerunning a build updates the same release. The workflow can also be started
manually; runs on `main` publish releases after the same checks pass.

Packages are not publisher-signed or notarized. macOS bundles are ad-hoc signed
for execution, including on Apple Silicon, and may require the system's
**Open Anyway** approval. Windows may display its unsigned-application prompt.

Intended minimum platforms: macOS 13, Windows 10/11 x64, and Ubuntu 24.04 x64
or an equivalent distribution with GTK 4.14+ and configured system printing.
The Linux archive is not a dependency-free AppImage.

## Development

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
python -m pip install -r scripts/requirements.txt
python scripts/build_native.py
```

The script downloads SHA-256-verified sources and builds separate libheif and
libde265 libraries without installing them system-wide. Development builds find
`target/native/lib`; `PRINTMARTEN_NATIVE_LIB_DIR` overrides the lookup directory.
On Windows, point that variable at `target/native/bin` if necessary. Source
archives are cached in `target/native-sources`.

To package locally after building native dependencies:

```sh
cargo build --release --locked
cargo install cargo-about --version 0.8.2 --locked
cargo about generate --locked packaging/licenses.hbs --output-file target/THIRD_PARTY_LICENSES.html
python scripts/package.py --platform macos --arch x64 --sha local
python scripts/validate_packages.py --platform macos --arch x64
```

Use `--platform windows --arch x64` or `--platform linux --arch x64` on their
respective operating systems. Windows packaging also requires NSIS 3.11 and an
MSVC developer shell with `VCToolsRedistDir`. CI builds each architecture on a
native runner, including separate Intel and Apple Silicon macOS runners.

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

See [docs/validation.md](docs/validation.md) for the manual checklist and recorded results.
