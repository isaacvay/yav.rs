# Third-party icon attributions — `yav-icons`

The icon data compiled into this crate is derived from three upstream icon
projects. Their license texts are distributed with this crate and **must be
retained in any redistribution**:

| Family in Yav | Upstream project | License | File |
| --- | --- | --- | --- |
| Classic | [Lucide](https://lucide.dev) | ISC | `LICENSE-LUCIDE` |
| Minimal | [MynaUI Icons](https://mynaui.com) | MIT | `LICENSE-MYNAUI` |
| System | [Phosphor Icons](https://phosphoricons.com) | MIT | `LICENSE-PHOSPHOR` |

The `yav` / `yav-icons` Rust source code itself is licensed under the
workspace terms (`MIT OR Apache-2.0`), see `LICENSE-MIT` and
`LICENSE-APACHE` at the repository root. The table above concerns the
**icon artwork and generated SVG data only**.

The raw source SVG/JSON trees (`lucide/`, `mynaui-icons-main/`,
`phosphor-icons/`) are intentionally excluded from the published crate to
keep package size down; the generated Rust catalog in `src/catalog/` is
what gets shipped. The upstream license texts above are therefore included
at the crate root so that the published artifact remains compliant.
