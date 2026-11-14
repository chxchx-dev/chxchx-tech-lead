# Native Studio third-party components

Studio fetches these source revisions at CMake configure time:

| Component | Upstream | Revision | License |
| --- | --- | --- | --- |
| Scintilla Qt widget | [mirror/scintilla](https://github.com/mirror/scintilla) | `a1c86144eed9e3d2187e3a8b391d11ca909f00d2` (`rel-5-5-2`) | Historical Scintilla license in upstream `License.txt` |
| Lexilla language lexers | [ScintillaOrg/lexilla](https://github.com/ScintillaOrg/lexilla) | `8fe5438b65cffc34340ecd60357ce5a6f72eecbe` (`rel-5-5-4`) | Historical Scintilla license in upstream `License.txt` |
| libvterm terminal parser | [neovim/libvterm](https://github.com/neovim/libvterm) | `9d6d2112335080312ef8c36667fa717ded4f7daf` (`v0.3.3`) | MIT; include upstream `LICENSE` |
| Qt | [Qt Project](https://www.qt.io/licensing/) | Qt 6.4 or newer; CI currently uses 6.8.3 | LGPLv3/GPLv3/commercial options; see Qt terms |

Scintilla and Lexilla grant broad permission to use, modify, and redistribute,
provided their copyright and permission notices are retained in supporting
materials. When distributing Studio, include both upstream `License.txt`
files alongside the application. Source is fetched by hash so configure does
not follow moving branches.

libvterm is built as a static library from its upstream C sources. Retain its
MIT license when redistributing Studio.

The Qt Scintilla port uses Qt Core5Compat for `QTextCodec`. Linux developers
need the Qt Core5Compat development package in addition to Core and Widgets.
