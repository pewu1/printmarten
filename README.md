# PrintMarten

Print photos at exact physical sizes, arrange several prints on shared sheets,
and send them to your printer through the operating system's print dialog.

## Features

- Full-page prints and standard photo, passport/ID, wallet, or custom sizes.
- Mixed jobs containing full-page sheets and smaller photos packed together.
- Automatic placement that favors fewer sheets, then groups matching photo sizes.
- Crop to fill or fit the complete image, with per-photo choices and independent copies.
- Portrait/landscape sheets, photo-paper and borderless options, minimum margins/gaps, and extended cutting guides.
- File/folder importing, drag-and-drop, sheet preview, zoom, and low-resolution warnings.
- JPEG, PNG, WebP, TIFF, BMP, HEIC, and HEIF support. Image orientation metadata is applied; RGB color profiles are converted to sRGB and transparency is flattened onto white.

## Install

Download the latest package from [GitHub Releases](https://github.com/pewu1/printmarten/releases).
Choose the package for your operating system and processor. Releases include SHA-256
checksums and matching source archives.

| Platform | Package | Installation |
| --- | --- | --- |
| macOS Intel | `macos-x64` DMG | Open the DMG and drag PrintMarten to Applications. |
| macOS Apple Silicon | `macos-arm64` DMG | Open the DMG and drag PrintMarten to Applications. |
| Windows x64 | `windows-x64-setup.exe` | Install the runtime below, then run the per-user installer. |
| Windows x64 portable | `windows-x64-portable.zip` | Install the runtime below, extract the entire ZIP, and launch `printmarten.exe`. |
| Linux x64 | `linux-x64.deb` | Run `sudo apt install ./PrintMarten-….deb` using the downloaded filename. |
| Linux x64 archive | `linux-x64.tar.gz` | Extract the archive and run `./PrintMarten/printmarten`. Install its GTK/system dependencies separately. |

Intended minimum platforms: macOS 13, Windows 10/11 x64, and Ubuntu 24.04 x64
or an equivalent Linux distribution with GTK 4.14+ and configured printing.
The Linux archive is not a standalone AppImage.

**Windows prerequisite:** install Microsoft's
[Visual C++ v14 x64 Redistributable](https://aka.ms/vc14/vc_redist.x64.exe)
separately. It is required by the application/native image libraries and is not
bundled with PrintMarten. If Windows reports a missing `VCRUNTIME140.dll` or
`MSVCP140.dll`, install or repair this runtime.

Packages are not publisher-signed or notarized. macOS may require **Open Anyway**
in Privacy & Security; Windows may show an unsigned-application prompt.

## Your first print

1. Choose **Add photos…** or **Add folder…**, or drop files/folders into the window.
2. Select your sheet size and **Portrait** or **Landscape** orientation.
3. Choose the default photo size. **Full page** starts selected and gives each photo its own sheet. Select a standard or custom size to pack smaller prints together.
4. Review the preview. Right-click individual photos to choose their sizes, crop/fit behavior, copies, or removal.
5. Choose **Print…** and select your printer and paper settings. If the printer changes the arrangement, review the updated sheets and choose **Print reviewed layout**.

Start with one sample sheet when accurate physical dimensions matter. In the
printer dialog, use **100% scale**, **one page per sheet**, and disable driver
enlargement/borderless expansion. PrintMarten preserves requested dimensions;
it does not shrink oversized photos to make them fit.

## Settings and options

| Option | What it does |
| --- | --- |
| Sheet size | A3, A4 (initial sheet), A5, A6, Letter, Legal, photo-paper presets, or custom dimensions. |
| Orientation | Portrait or Landscape. Images and fixed-size print rectangles rotate as needed to align their long edge with the sheet. |
| Photo paper | Starts checked. Selecting photo-paper stock enables Borderless; you can also use A4 or custom photo stock. Choosing an ordinary sheet preset resets the stock options. |
| Borderless | Starts checked. Uses zero application margins and requires a printer/paper mode with zero unprintable margins. Turn it off for ordinary printers. |
| Default photo size | Full page (initial choice), standard photo sizes, passport/ID and wallet sizes, or Custom size. Individual overrides survive default changes. |
| Crop to fill | Starts checked. Crops around the image center to fill its print rectangle. Toggling applies to all current photos and future imports. Uncheck to keep the whole image with white space as needed. Individual right-click choices can override it afterward. |
| Custom size | Enter width/height in centimeters, millimeters, or inches. Units are selected inside this dialog. Decimal points and decimal commas are accepted. |
| Min. margin | Minimum application margin when Borderless is off. The printer's hardware margins can increase it. The saved value returns when Borderless is turned off. |
| Min. gap | Minimum gap between smaller prints. Spare space is distributed evenly across rows and around the photos. |
| Print cut lines | Draws edge guides extending up to 3 mm beyond photo corners, stopping at printable bounds or neighboring photos. Full-page sheets omit guides. |
| Preview zoom | Changes the on-screen preview only; physical print sizes stay the same. |

Gap and cutting controls are disabled for jobs containing only full-page prints.
Mixed jobs print full-page sheets first in import order, followed by packed
smaller prints. Packing runs automatically after changes; it tries several
arrangements and does not guarantee a mathematical optimum.

## Working with photos

- Click a thumbnail or preview photo to select it. **Ctrl/Cmd-click** selects several; **Ctrl/Cmd+A** selects all.
- Right-click for **Print size**, **Full page**, **Custom size…**, **Reset to default size**, **Crop to fill**, **Fit whole image**, **Add one copy**, and **Remove**.
- Copies share the original image data but can have independent print sizes and crop/fit choices.
- **Delete/Backspace** removes selected photos when you are not typing in a field.
- **Clear job** removes all imported photos. It is disabled while importing.
- Folder imports include supported photos in subfolders, in filename order. Other file types and folder symlinks are skipped. Duplicate paths within one import are loaded once; corrupt files are reported while valid photos continue loading.
- **About** shows the application version, author, GitHub link, and GNU GPL v3 license.

Jobs are session-only. Closing PrintMarten discards the arrangement and leaves
source photos unchanged. RAW files, animation, and multi-frame editing are not
supported; HEIF uses the primary still image.

## Printing help

**Borderless is rejected:** choose a borderless paper size/mode and matching
photo media in the printer's settings, or turn off Borderless in PrintMarten.
Support depends on both your printer and the selected paper.

**A photo does not fit:** use a larger sheet or choose a smaller photo size.
PrintMarten keeps the photo in the list and prevents printing until all prints fit.
Long-edge alignment may require more sheets than unrestricted rotation.

**A photo may look soft:** the warning means its effective resolution is below
150 DPI at the requested print size. Use a higher-resolution source or a smaller print.

**The print dialog changed the layout:** review the updated sheets before
submitting. **Change printer…** lets you choose different printer settings.
Document copies in the system dialog repeat the whole job; **Add one copy**
duplicates an individual photo. “Submitted” means accepted by the printing
system, rather than confirmation that a physical sheet has finished printing.

Very large images are limited during decoding, and retained source pixels are
capped at 1 GB per session. Clear the job before importing more if this limit is reached.

## License and source

PrintMarten is licensed under **GNU GPL version 3 only (`GPL-3.0-only`)**.
See [LICENSE](LICENSE). Third-party libraries, icons, and fonts retain their own
licenses; see [third-party notices](THIRD_PARTY_NOTICES.md) and the packaged
`THIRD_PARTY_LICENSES.html`.

Each new release includes a corresponding-source archive with the application,
locked Rust dependencies, native library sources, and build scripts. For building
or contributing, see [developer instructions](docs/development.md). Dependency
review details are in the [license audit](docs/license-audit.md), and platform
validation is recorded in [validation notes](docs/validation.md).
