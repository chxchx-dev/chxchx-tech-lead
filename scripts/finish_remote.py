"""Finishes the web client's build: puts a build id and the list of files that make the app start
into the service worker. Run by scripts/build-remote.sh on the output folder."""

import hashlib
import json
import sys
from pathlib import Path

dist = Path(sys.argv[1])
wasm = dist / "pkg" / "app_bg.wasm"
build = hashlib.sha256(wasm.read_bytes()).hexdigest()[:12]

# What the app needs to start. The big icons are only fetched when the app is installed.
shell = ["/", "/boot.js", "/app.css", "/manifest.webmanifest", "/icons/favicon-32.png", "/icons/apple-touch-icon.png",
         "/img/mark-64.png", "/img/mark-128.png", "/img/mascot-96.png", "/img/mascot-288.png"]
shell += sorted("/" + path.relative_to(dist).as_posix() for path in (dist / "pkg").rglob("*") if path.is_file())

sw = dist / "sw.js"
text = sw.read_text()
# The exact code that holds each placeholder, each exactly once: a mention in a comment must
# never be replaced (that once put the whole list inside a comment and broke the worker).
build_code, shell_code = 'const BUILD = "__BUILD__";', "const SHELL = __SHELL__;"
assert text.count(build_code) == 1 and text.count(shell_code) == 1, "sw.js lost its placeholders"
text = text.replace(build_code, f'const BUILD = "{build}";').replace(shell_code, f"const SHELL = {json.dumps(shell, indent=2)};")
assert "__BUILD__" not in text and "__SHELL__" not in text
sw.write_text(text)
print(f"build {build}, {len(shell)} files in the shell")
