# Validation record

## Automated coverage

- Unit parsing and physical conversion, invalid input, and independent size overrides.
- Deterministic mixed-size packing, rotation, page overflow, gaps, hardware margins, exact-fit pages, and oversized rejection.
- Crop/contain geometry, import failure, alpha flattening, orientation metadata, and ICC conversion.
- PDF paper/photo measurements, image reuse, vector cut lines, and raster placement.
- Printer geometry reconciliation, conflicting scaling, invalid paper, and empty jobs.
- UI state transitions for selections, copies, removal, and size resets.
- Workflow syntax and package structure, checksums, architecture, and executable smoke checks.
- Photo-paper borderless defaults, full-page portrait/landscape layouts, mixed full-page/fixed-size jobs, rejection of non-borderless printer geometry, and edge-to-edge PDF/raster coverage.
- Long-edge alignment for image pixels and print rectangles, equal extra spacing in rows, centered partial rows, and exact passport/ID and wallet preset sizes.
- Recursive folder discovery, filename order, mixed-case extensions, duplicate paths, symlink cycles, empty/missing folders, and background loading with corrupt-image reporting.

## Manual acceptance checklist (each target)

Targets: macOS Intel, macOS Apple Silicon, Windows x64, Linux x64 on X11 and Wayland.

1. Install the package, launch from the desktop, and import several JPEG/PNG/HEIC photos by picker and drag-and-drop.
2. Set A4 and Letter paper, both orientations; verify visible sheets and default photo sizes.
3. Select several photos, customize their dimensions, change the default, and verify overrides survive.
4. Add copies, modify a copy, remove selected photos, and reset selected sizes to default.
5. Check centered crop and fit modes using images with visible corner markers; check 90° packing rotation.
6. Change margins/gaps, enable cut lines, and verify oversized items prevent printing without being silently shrunk.
7. Open the system print dialog and verify paper preselection, printer switching, media options, copies, and page ranges.
8. Cancel the dialog and verify nothing is submitted. Change paper or hardware margins and verify changed-layout review precedes submission.
9. Save through the system PDF destination and measure photo rectangles and cut lines. Check portrait/landscape, rotation, and nonzero margins.
10. Print a physical sample at 100% with driver enlargement disabled; measure requested 100 × 150 mm rectangles and inspect image quality.
11. Exercise missing/offline printers, no default printer, unsupported paper, and submission failure; verify the job is retained.
12. Verify installer uninstall behavior and portable execution where applicable.
13. Launch a new session and verify A4, Full page, Photo paper, Borderless, and Crop to fill defaults. Select ordinary and photo-paper presets and check stock-option behavior. Turn Borderless off and verify saved margins return.
14. Mix Full page overrides with custom and standard fixed sizes. Verify full-page sheets appear first and omit guides; smaller prints share packed sheets. Change the default and reset selected overrides. Toggle Crop to fill for all photos, then override one photo with Fit whole image. Check both sheet orientations and print-dialog reflow.
15. On a borderless-capable printer, check the initial borderless paper selection for the default printer, then switch printers and explicitly select a borderless mode if required. Print and measure an edge-to-edge photo. Check a printer/paper combination without borderless support: it must report unprintable margins and require settings changes rather than silently clip the job.
16. Import a nested folder via Add folder, drag-and-drop, and a command-line path. Confirm only supported photos appear and other files are skipped. Confirm the window remains responsive and corrupt images are reported while valid photos load.
17. Print landscape and portrait source images onto both sheet orientations. Check that image and photo-rectangle long edges follow the sheet's long edge, including in Full page mode. Choose passport/ID sizes and verify the physical cut dimensions.
18. Add several equal-sized photos and change minimum margins/gaps. Check equal gaps across each row, balanced rows, centered partial rows, and unchanged physical photo sizes. Repeat with mixed sizes and the selected printer's actual margins.

19. Resize to 800 × 560 and 1180 × 820. Check centered toolbar actions, right-aligned counts, narrow-window wrapping, readable Lucide icons, and padded thumbnail outlines with long filenames.
20. Open About and verify PrintMarten branding, Piotr Wenta, the correct GitHub destination, GPL v3 text, and closing behavior.
21. Check cut guides extend up to 3 mm, stop before neighboring photos or printable boundaries, and match PDF/raster output. Check grouped sizes do not needlessly increase sheets.
22. On a clean Windows system, install Microsoft's x64 Visual C++ runtime separately, then verify installer/portable execution. Confirm no Microsoft runtime DLLs are bundled. Resolve/build the corresponding-source archive using its vendored Rust crates.

## Current change validation (2026-10-26)

- 36 Rust tests pass on Intel macOS, including the actual HEIC fixture, mixed jobs, crop/contain output, guide extension/clipping, grouping, SVG icon loading, and headless toolbar/About geometry at 800- and 1180-pixel widths.
- Formatting and strict Clippy checks pass for all local targets.
- cargo-about 0.8.2 validates the default-feature cross-platform license graph with `--locked --offline --fail`; the reviewed metadata inventory includes optional packages too. See [license-audit.md](license-audit.md).
- The corresponding-source packager creates an archive with vendored dependencies and native source archives. It checks offline dependency resolution from the packaged vendor configuration before archiving.
- Native GUI interaction, current Windows/Linux packaging, physical output, and a published CI run remain unverified for these changes.

## Earlier baseline validation (2026-10-26, before these changes)

- Intel macOS: all 27 tests pass, including the real libheif HEIC fixture, headless preview, PDF measurements, raster placement, photo-paper defaults, full-page borderless output, long-edge rotation, balanced spacing, photo presets, and recursive folder importing.
- Formatting and strict Clippy checks pass. Workflow syntax passes actionlint; packaging scripts pass Python syntax checks.
- The complete application passes an Apple Silicon `cargo check`. This is a cross-compile check, not execution on Apple Silicon.
- The actual Windows adapter passes strict Clippy/type checks for the MSVC target using a compile-only harness. No Windows executable has been run locally.
- The actual Linux adapter passes strict Clippy/type checks against GTK bindings using their documentation mode to omit native linking. This does not validate GTK runtime behavior.
- Native HEIC libraries build from the pinned source archives. The Intel macOS DMG mounts successfully, its bundle and ad-hoc signature verify, and its bundled executable decodes the 1280 × 854 HEIC fixture with development library overrides removed.
- Dependency license notices are generated successfully from Cargo.lock.

## Current limitations of local validation

The development environment is macOS 15 on Intel. Native print-dialog interaction,
native Windows/Linux execution,
Apple Silicon execution, printer-specific behavior, signing/notarization, and
physical print measurements must be recorded separately after running on those
systems. GitHub workflow execution is pending the first push to `main`.
