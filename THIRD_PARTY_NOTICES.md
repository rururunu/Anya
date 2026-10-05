# Third-Party Notices

## DeepSeek Harness

The native DeepSeek tool path includes tool schemas and prompt text derived from DeepSeek Harness, revision `639ed015397290b3745d163aafe02ffee4aa3f84`, copyright 2026 DeepSeek, under the MIT License. The retained license and source metadata are in `src-tauri/prompts/dsh/LICENSE` and `SOURCE.json`. Source: https://github.com/deepseek-ai/deepseek-harness. Anya implements the core executors in Rust and does not bundle the official harness runtime.

## LibreOffice Kit

The Office conversion runtime includes `@deepseek-ai/libreoffice-kit` 0.1.5 and its matching platform engine, licensed under MPL-2.0. Packages retain their LICENSE, NOTICE, third-party licenses, source pins, modifications and build recipes. The Node runtime retains its LICENSE. Corresponding kit source is available at https://github.com/deepseek-ai/dsh-libreoffice-kit; engine-specific corresponding sources are identified in each engine package's `sources/` materials. The packaged `office/kit/` directory contains the complete selected engine and its production dependencies. Generated `office/THIRD_PARTY_LICENSES.txt` lists the bundled versions and licenses. Anya does not modify the kit or LibreOffice engine source.

## Material Icon Theme

File icons are sourced from Material Icon Theme, licensed under the MIT License. Delivery cards bundle unchanged SVGs from revision `735a0166cb72f3514a717f7c17905a77374d5df4`, copyright 2025 Material Extensions. Source: https://github.com/material-extensions/vscode-material-icon-theme.

The existing code-file icon license is included at `public/file-icons/LICENSE.txt`. Delivery-card source metadata and license are in `src/assets/material-file-icons/`; the license is also packaged at `public/licenses/material-icon-theme-MIT.txt`.
