# Validation record

## Automated coverage

- Unit parsing and physical conversion, invalid input, and independent size overrides.
- Deterministic mixed-size packing, rotation, page overflow, gaps, hardware margins, exact-fit pages, and oversized rejection.
- Crop/contain geometry, import failure, alpha flattening, orientation metadata, and ICC conversion.
- PDF paper/photo measurements, image reuse, vector cut lines, and raster placement.
- Printer geometry reconciliation, conflicting scaling, invalid paper, and empty jobs.
- UI state transitions for selections, copies, removal, and size resets.
- Workflow syntax and package structure, checksums, architecture, and executable smoke checks.
- Photo-paper borderless defaults, full-page portrait/landscape layouts, preserved custom sizes, rejection of non-borderless printer geometry, and edge-to-edge PDF/raster coverage.

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
13. Select 10 × 15 cm / 4 × 6 in paper and verify Photo paper and Borderless start enabled. For A4 photo stock, enable Photo paper explicitly. Verify Borderless can be turned off and normal margins return.
14. Enable Full page with mixed image shapes and customized photo sizes. Verify one cropped photo per sheet, no cut lines, and coverage to every page edge. Switch orientation and paper size in both the app and print dialog; review any changed layout. Turn Full page off and verify custom sizes/crop choices return.
15. On a borderless-capable printer, check the initial borderless paper selection for the default printer, then switch printers and explicitly select a borderless mode if required. Print and measure an edge-to-edge photo. Check a printer/paper combination without borderless support: it must report unprintable margins and require settings changes rather than silently clip the job.

## Local results (2026-10-26)

- Intel macOS: all 19 tests pass, including the real libheif HEIC fixture, headless preview, PDF measurements, raster placement, photo-paper defaults, and full-page borderless output.
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
