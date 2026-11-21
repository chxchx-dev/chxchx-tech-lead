#!/usr/bin/env python3
"""Sign a release manifest with ed25519 (the key the program trusts is in packaging/release-public-key.txt).

The private key (32 bytes as 64 hexadecimal digits) is read from the environment variable named by
--key-env and never printed. The signature is written as hexadecimal next to the manifest as
``<manifest>.sig``; the program (`chxchx-tech manager update`) verifies it before applying anything.
"""

from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey


def sign(manifest: bytes, seed_hex: str) -> str:
    seed = bytes.fromhex(seed_hex.strip())
    if len(seed) != 32:
        raise ValueError("La clave privada debe ser de 32 bytes (64 dígitos hexadecimales).")
    return Ed25519PrivateKey.from_private_bytes(seed).sign(manifest).hex()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("--key-env", default="RELEASE_SIGNING_KEY")
    args = parser.parse_args()
    seed = os.environ.get(args.key_env, "")
    if not seed.strip():
        print(f"Falta la variable {args.key_env}: el lanzamiento no se firma.", file=sys.stderr)
        return 2
    signature = sign(args.manifest.read_bytes(), seed)
    out = args.manifest.with_name(args.manifest.name + ".sig")
    out.write_text(signature + "\n", encoding="utf-8")
    print(f"Firmado: {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
