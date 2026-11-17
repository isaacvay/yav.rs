# Asset attribution and maintenance notes

## Brand assets

The SVG/PNG/JPG files in this directory are original Yav.rs brand artwork
created for this repository. They are covered by the repository's dual
`MIT OR Apache-2.0` license (`LICENSE-MIT`, `LICENSE-APACHE` at the root), not
by a separate third-party license.

Third-party icon catalogs embedded in the framework are a different matter:
their licenses live in `crates/yav-icons/` (`LICENSE-LUCIDE` ISC,
`LICENSE-MYNAUI` MIT, `LICENSE-PHOSPHOR` MIT, `NOTICE.md`). These files are
packaged with the published `yav-icons` crate and checked by CI.

## Duplicated image files

`yav-document.png` occurs in four places: `assets/png/`,
`editors/vscode/icons/`, `crates/yav-cli/editors/vscode/icons/` and
`test-app/public/`. The VS Code extension copies must remain because
`crates/yav-cli/src/ext.rs` embeds them with `include_bytes!` at compile time;
the `test-app/` copy is a standalone demo asset. They are intentional copies of
the same artwork, not four different licensed images.

The extension itself also exists twice: `editors/vscode/` is the publishable
source, `crates/yav-cli/editors/vscode/` is the copy embedded into the CLI so
`yav ext install` works without network access. Keep both in sync — CI does not
currently enforce this, so a manual diff is required when editing either one.

## Local generator scripts

`generate_all_svgs.py` and `process_assets.py` are **local maintenance tools**,
not part of the framework or the published crates. They resolve paths relative
to their own location (no machine-specific absolute paths).

`process_assets.py` requires Python packages that are not bundled here:
see `requirements.txt`. Install them in a virtual environment before running:

```bash
python -m venv .venv
. .venv/bin/activate        # Windows: .venv\Scripts\activate
pip install -r assets/requirements.txt
python assets/process_assets.py
```

`preview.html` is a local preview page and loads Google Fonts from a CDN. It is
never served by the framework and has no effect on production CSP policies.

## Binary size

Several PNG/JPG files are 1–2 MB each. They are documentation/promotional
artwork and are not linked into any Rust binary. If repository size becomes a
concern, regenerate them at a lower resolution rather than committing further
full-resolution variants.
