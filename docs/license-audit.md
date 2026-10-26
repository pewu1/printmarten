# Dependency license audit

Review date: 2026-10-26. Application license: **GPL-3.0-only**.
This review covers the current Cargo.lock, default application features, bundled
assets and native-library build/package scripts. It is an engineering license
review, not a legal opinion or a blanket guarantee for future dependency changes.

## Rust dependencies

`cargo metadata --locked --offline` resolved 445 dependency package versions,
including platform-specific, build-time and optional packages. Every dependency
has a declared license expression; none declares an unknown license. Review OR
expressions by choosing a permitted branch, and preserve every obligation in
AND expressions. The chosen branches are recorded by cargo-about in generated
`THIRD_PARTY_LICENSES.html`, rather than relicensing dependency files as GPL.

The code licenses found are MIT, Apache-2.0 (including an optional LLVM exception),
BSD-2-Clause, BSD-3-Clause, ISC, Zlib, BSL-1.0, CC0-1.0 and Unicode-3.0, or
expressions offering these choices. No GPL-2.0-only, original advertising-clause
BSD, CDDL, SSPL or other incompatible code license was found in this graph.
`encoding_rs` additionally requires BSD-3-Clause; `unicode-ident` and
`unicode-normalization` additionally require Unicode-3.0. These AND terms must
not be discarded. `r-efi` offers an LGPL alternative, but its MIT option suffices.
There is no MPL-only crate in this snapshot.

Apache 2.0 can be included in GPLv3 software; compatibility is directional.
See the [Apache Software Foundation compatibility guidance](https://www.apache.org/licenses/GPL-compatibility)
and the [FSF license list](https://www.gnu.org/licenses/license-list.html) for
other code licenses. This review is specific to GPLv3, not GPLv2-only.

`cargo-about 0.8.2 generate --locked --offline --fail --format json` completed
successfully with no diagnostic warnings. Its default-feature graph covers 408
dependency package versions plus PrintMarten; the remaining 37 optional package
versions were reviewed through metadata. The per-package declared and selected
licenses are recorded in [dependency-licenses.csv](dependency-licenses.csv).
The 11 selected license identifiers include the application's GPL license and
three font/data license identifiers; they do not imply all assets are GPL code.
`cargo-about` also generates the packaged notice texts. CI uses `--locked --fail`;
unrecognized or rejected license expressions
must fail the build. `about.toml` is the policy applied after review, not evidence
of compatibility by itself. Recheck native code and assets when updating crates.

## Embedded fonts and icons

`epaint_default_fonts 0.33.3` declares `(MIT OR Apache-2.0) AND OFL-1.1 AND
Ubuntu-font-1.0`. The font files remain separately licensed font data:

| Asset | Terms retained |
| --- | --- |
| Hack Regular | Source Foundry MIT, DejaVu public domain, Bitstream Vera font notice |
| Noto Emoji Regular | SIL Open Font License 1.1 |
| Ubuntu Light | Ubuntu Font Licence 1.0 |
| emoji-icon-font | John Slegers MIT |
| Lucide 0.468.0 | ISC and the retained upstream MIT-derived notice |

The font licenses are not replaced with GPL. Their notices and copyright records
are included in THIRD_PARTY_NOTICES.md, including Hack's inherited notice, which
is more specific than the crate's top-level SPDX expression. Bundling font data
is distinct from combining software code. The
[Ubuntu font licence](https://ubuntu.com/legal/font-licence) expressly permits
bundling/embedding while requiring the fonts to remain under its terms. The
[FSF font-license discussion](https://www.gnu.org/licenses/license-list.html#Fonts)
likewise distinguishes ordinary font use from software-license compatibility.
Font-license allowances in about.toml are scoped to epaint_default_fonts, rather
than permitting future software dependencies under font licenses globally.
No fonts are modified. Lucide paths are unchanged; changing white stroke color
for UI tinting is documented beside the bundled SVGs.

## Native libraries and platform runtimes

| Component | Finding and distribution treatment |
| --- | --- |
| libheif 1.23.1 | Library headers specify LGPL-3.0-or-later. Built as a separate dynamic library; notices and exact source archive are included. |
| libde265 1.0.15 | Library headers specify LGPL-3.0-or-later. Same treatment as libheif. |
| Little CMS in lcms2-sys 4.0.7 | Native vendor/LICENSE is MIT; copied into third-party notices. Its source is included through the vendored Rust crate. |
| macOS frameworks/libc++ | System prerequisites, not copied into the application. Local otool inspection shows only libde265 and Apple system libraries behind libheif. |
| Linux GTK/C/C++/display libraries | Installed through the operating system/package manager; not copied into the archive. |
| Microsoft Visual C++ runtime | Installed separately from Microsoft. Proprietary runtime DLLs must not be copied into the GPL application payload. |

The pinned native scripts disable plugin loading and optional encoders/decoders;
libde265 is the only enabled HEIC decoder. Local CMake cache and Mach-O dependency
inspection agree with this configuration. LGPLv3 libraries may accompany GPLv3
software while retaining their own terms; see the
[LGPLv3 text](https://www.gnu.org/licenses/lgpl-3.0.html).

The previous Windows packager copied app-local Microsoft runtime DLLs. That
behavior has been removed and package validation rejects vcruntime, msvcp and
concrt DLLs in the payload. Users install the runtime independently. This avoids
the redistribution issue described in the
[FSF Windows-runtime FAQ](https://www.gnu.org/licenses/gpl-faq.html#WindowsRuntimeAndGPL).
No additional proprietary-library exception is added to PrintMarten's license.

## Corresponding source and release obligations

`scripts/package_source.py` creates a source archive with the application,
Cargo.lock, every vendored locked Rust package, native source archives verified
against the native-build script's hashes, assets, and build/install scripts.
It includes a Cargo vendor configuration for offline dependency resolution.
CI publishes this archive and its checksum beside binaries. General-purpose
compilers, SDKs and OS system libraries remain separately installed prerequisites.
See [GPLv3 sections 1 and 6(d)](https://www.gnu.org/licenses/gpl-3.0.html).

Keep corresponding source available while distributing the matching binaries.
Retain all upstream copyright/license notices and the LGPL library replacement
instructions. Do not characterize third-party components as relicensed under GPL.
The packaged notices describe them individually. Historical releases are not
rewritten by these working-tree changes.

Windows/Linux packaged execution and a published CI release still require
validation on their target systems. The local native dependency inspection is
specific to Intel macOS; it does not establish the runtime linkage of binaries
built elsewhere.
