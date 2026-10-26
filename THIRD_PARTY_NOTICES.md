# Third-party software

PrintMarten packages include a decode-only HEIC runtime built from unmodified
libheif 1.23.1 and libde265 1.0.15, dynamically loaded as separate libraries.
Their LGPL license texts are in `native-licenses` beside this notice.

- libheif: https://github.com/strukturag/libheif/tree/v1.23.1
- libde265: https://github.com/strukturag/libde265/tree/v1.0.15

Matching source archives and SHA-256 checksums are pinned in
`scripts/build_native.py`. The workflow uploads these source archives beside
the application packages. That script documents how to rebuild replacement
libraries. Users may replace the shipped dynamic libraries with compatible
modified versions; macOS replacements may require ad-hoc re-signing the bundle.
PrintMarten imposes no restriction on reverse engineering for debugging such changes.

Rust dependency license texts are included in `THIRD_PARTY_LICENSES.html`,
generated from the committed Cargo.lock with cargo-about. Little CMS is built
into PrintMarten under the MIT license. Windows packages also include Microsoft's
app-local Visual C++ runtime redistributable, subject to its redistribution terms.

The original application icon is supplied in this repository. No third-party
image assets are downloaded at runtime.
