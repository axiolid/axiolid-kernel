# Licensing Axiolid

Axiolid is licensed under the [Mozilla Public License 2.0](https://www.mozilla.org/MPL/2.0/) (**MPL-2.0**). The canonical text is included in [LICENSE](https://github.com/axiolid/axiolid-kernel/blob/main/LICENSE).

## The intended boundary

MPL is *file-level copyleft*. It keeps the kernel open while letting a product use it without relicensing the product itself.

- **Axiolid source:** If you distribute a modified Axiolid source file, make that file’s source available to recipients under MPL-2.0.
- **Your application:** Files you add for an application may remain proprietary, including when the application depends on or links Axiolid.
- **A larger work:** Distributing an executable or product that combines Axiolid with other code does not by itself place the other code under MPL-2.0.

## If you distribute a product

When you ship a binary that includes Axiolid, retain the copyright and licence notices and make the corresponding source for the MPL-covered Axiolid files available to recipients. A practical approach is a `third-party-notices` page plus a source-offer URL or archive for the exact Axiolid revision and any patches.

If you modify only application files, those files are outside Axiolid’s MPL-covered source boundary. If you copy Axiolid code into an application file or modify an Axiolid file, that source-file obligation applies to the resulting file.

This avoids LGPL’s usual static-linking/relinking friction in Rust while still requiring distributed improvements to the kernel itself to stay open.

## Contributions

By submitting a contribution, you agree that it is licensed under MPL-2.0. Keep third-party code in its own dependency or clearly documented import path; do not copy code into Axiolid without recording its provenance and licence compatibility.

## FAQ

### Can a closed-source application use Axiolid?

Yes. MPL-2.0 permits this. The application’s separate source files do not become MPL-2.0 merely because they use Axiolid.

### Must kernel changes be open sourced?

When distributing those modified MPL-covered files, yes: provide their source under MPL-2.0 to the recipients of the binary or source distribution.

### Is this legal advice?

No. This page explains the project’s intended licensing model, not legal advice. Consult counsel for a product-specific distribution decision.
