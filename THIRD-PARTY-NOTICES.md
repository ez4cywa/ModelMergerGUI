# Third-party notices

## MiSans

Official release builds of Cast Model Merger GUI embed the original MiSans Medium face supplied by Xiaomi Inc. for the Simplified Chinese interface.

MiSans is not covered by this project's MIT License. It is used and distributed under the **MiSans Fonts Intellectual Property License Agreement**. The software visibly acknowledges its use of MiSans, as required by that agreement. The font files have not been modified.

- Official font site: <https://hyperos.mi.com/font/en/>
- Official license information: <https://hyperos.mi.com/font/en/download/>
- Source-build installer: `scripts/Install-MiSans.ps1`

MiSans font files are intentionally excluded from the Git repository. Source builders must accept Xiaomi's agreement and use the installer script to download the official archive. The font files may not be separately redistributed, sublicensed, adapted, or sold except as permitted by Xiaomi's agreement. Refer to the official license for the complete terms.

## Rust libraries

The native application uses the following principal Rust libraries under their upstream open-source licenses:

- `eframe`, `egui`, `egui-wgpu`, `wgpu`, `accesskit`, `image`, `serde` and `serde_json`: MIT or Apache-2.0.
- `rfd`: MIT.
- `winit`: Apache-2.0.

Exact versions and the complete transitive dependency set are locked in `rust/Cargo.lock`; Cargo package metadata records each upstream license. These libraries are separate works and are not relicensed by this project's MIT License.
