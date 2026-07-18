# Third-party notices

## SJTU-Canvas-Helper

This project contains protocol work derived from analysis of
[SJTU-Canvas-Helper](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper), pinned for research at
commit `b5d895af57aaa74dfd53cef80dfb64c76c023c20` (2026-07-04). The upstream project is licensed under
the MIT License. Unrelated Tauri application features are not copied into this project.

Upstream license text:

> MIT License
>
> Copyright (c) 2025 Zihong Lin
>
> Permission is hereby granted, free of charge, to any person obtaining a copy
> of this software and associated documentation files (the "Software"), to deal
> in the Software without restriction, including without limitation the rights
> to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
> copies of the Software, and to permit persons to whom the Software is
> furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in all
> copies or substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
> IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
> FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
> AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
> LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
> OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
> SOFTWARE.

## Frontend runtime and test toolchain

The Phase 3 frontend uses the following direct open-source packages. Transitive package versions and
integrity hashes are fixed by `frontend/package-lock.json`; their license files remain in installed npm
packages and distributions as required.

- React and React DOM — MIT
- React Router — MIT
- TanStack Query — MIT
- Zod — MIT
- qrcode.react — ISC
- Vite and Vitest — MIT
- Playwright — Apache-2.0

No school, Canvas, or upstream video-platform logo or proprietary frontend asset is bundled.

## Invitation persistence

The optional one-time invitation store uses `rusqlite` (MIT) with the bundled SQLite library. SQLite is
dedicated to the public domain by its authors. Exact Rust dependency versions are fixed by `Cargo.lock`.
