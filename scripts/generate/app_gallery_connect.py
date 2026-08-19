#!/usr/bin/env python3
"""Generate the AppGallery Connect Rust client from the normalized OAS file."""

import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
SPEC = Path(__file__).resolve().parent / "specs/app_gallery_connect.openapi.yaml"
OUTPUT = ROOT / "crates/stores/app_gallery_connect"
CACHE_DIR = ROOT / ".cache/app_gallery_connect"


def generate() -> None:
    print(f"[1/3] Validating source spec: {SPEC.relative_to(ROOT)}")
    if not SPEC.exists():
        raise FileNotFoundError(SPEC)

    print("[2/3] Generating Rust client via cargo progenitor...")
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    generated = CACHE_DIR / "generated"
    if generated.exists():
        shutil.rmtree(generated)

    result = subprocess.run(
        [
            "cargo",
            "progenitor",
            "-i",
            str(SPEC),
            "-o",
            str(generated),
            "--name",
            "fastforge_app_gallery_connect",
            "--version",
            "0.1.0",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(result.stderr, file=sys.stderr)
        raise SystemExit(result.returncode)

    OUTPUT.mkdir(parents=True, exist_ok=True)
    (OUTPUT / "src").mkdir(exist_ok=True)
    shutil.copy2(generated / "src/lib.rs", OUTPUT / "src/client.rs")

    generated_cargo = (generated / "Cargo.toml").read_text()
    start = generated_cargo.index("[dependencies]")
    dependencies = generated_cargo[start:]
    cargo = f'''[package]
name = "fastforge_app_gallery_connect"
version.workspace = true
edition.workspace = true
authors.workspace = true
description = "Huawei AppGallery Connect API client"
license.workspace = true

{dependencies}'''
    (CACHE_DIR / "generated-Cargo.toml").write_text(cargo)

    client = OUTPUT / "src/client.rs"
    print(
        f"[3/3] Generated {client.relative_to(ROOT)} "
        f"({sum(1 for _ in client.open())} lines)"
    )
    print("       Runtime/auth/CLI files are intentionally preserved.")


if __name__ == "__main__":
    generate()
