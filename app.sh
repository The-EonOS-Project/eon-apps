#!/usr/bin/env bash
set -euo pipefail

APP_REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EONOS_ROOT="${EONOS_ROOT:-$(cd "$APP_REPO/.." && pwd)}"
BUILD_DIR="${EONOS_BUILD_DIR:-$APP_REPO/build}"
TARGET="$EONOS_ROOT/config/targets/x86_64-eonos-user.json"
SDK="$EONOS_ROOT/userland/app-sdk"

if [[ ! -f "$TARGET" || ! -f "$EONOS_ROOT/userland/linker.ld" || ! -d "$SDK" ]]; then
    echo "Could not find the EonOS checkout next to eon-apps. Set EONOS_ROOT to its path." >&2
    exit 2
fi

usage() {
    cat <<'EOF'
Usage:
  ./app.sh new <directory> <SHELL|ERA>
  ./app.sh build <app-directory>

`new` creates a small no_std Rust application project. `build` compiles it for
EonOS and writes build/apps/<id>.eonapp, plus a copy in this repository for
apps directly under its root. Copy the package onto the EonOS data disk, then
run `install /disk/<id>.eonapp` in EonOS.
EOF
}

valid_id() { [[ "$1" =~ ^[a-z][a-z0-9_-]{0,31}$ ]]; }

new_app() {
    local destination="${1:-}" kind="${2:-}" id sdk_rel
    if [[ -z "$destination" || ( "$kind" != SHELL && "$kind" != ERA ) ]]; then usage; exit 2; fi
    id="$(basename "$destination")"
    if ! valid_id "$id"; then echo "App directory name must be a 1-32 character lowercase id." >&2; exit 2; fi
    if [[ -e "$destination" ]]; then echo "Destination already exists: $destination" >&2; exit 2; fi
    mkdir -p "$destination/src"
    sdk_rel="$(python3 - "$destination" "$SDK" <<'PY'
import os, sys
print(os.path.relpath(os.path.abspath(sys.argv[2]), os.path.abspath(sys.argv[1])))
PY
)"
    cat > "$destination/Cargo.toml" <<EOF
[package]
name = "$id"
version = "0.1.0"
edition = "2021"

[package.metadata.eonos]
id = "$id"
type = "$kind"
name = "${id//-/ }"
description = "An EonOS $kind application"

[dependencies]
eonos-app = { path = "$sdk_rel" }


[profile.release]
panic = "abort"
strip = true
lto = true
opt-level = 2
EOF
    if [[ "$kind" == SHELL ]]; then
        cat > "$destination/src/main.rs" <<'EOF'
#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

fn main(_args: eonos_app::Args<'_>) -> i32 {
    let _ = eonos_app::system::print("Hello from an EonOS Shell app!\n");
    0
}

eonos_app::app!(SHELL, main);
EOF
    else
        cat > "$destination/src/main.rs" <<'EOF'
#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

fn main(_args: eonos_app::Args<'_>) -> i32 {
    let Ok(mut window) = eonos_app::era::Window::open("My EonOS app", 480, 300) else { return 1; };
    let _ = window.fill_rect(0, 0, 480, 300, 0x202A44);
    let _ = window.fill_rect(24, 24, 432, 252, 0xF0F0F0);
    let _ = window.text(48, 52, 0x12377B, "Hello from an Era app!");
    let _ = window.present();
    loop {
        match window.poll_event() {
            Ok(Some(eonos_app::era::Event::Close)) => break,
            Ok(Some(eonos_app::era::Event::Key { usage: 0x29, pressed: true, .. })) => break,
            _ => { let _ = eonos_app::system::sleep_ms(16); }
        }
    }
    let _ = window.close();
    0
}

eonos_app::app!(ERA, main);
EOF
    fi
    echo "Created $destination. Build it with: ./app.sh build $destination"
}

build_app() {
    local source="${1:-}" metadata kind id name description bin package_file binary marker packaged_copy
    if [[ -z "$source" ]]; then usage; exit 2; fi
    source="$(cd "$source" && pwd)"
    metadata="$(python3 - "$source/Cargo.toml" <<'PY'
import sys, tomllib
with open(sys.argv[1], "rb") as f:
    package = tomllib.load(f)["package"]
meta = package.get("metadata", {}).get("eonos", {})
for key in ("id", "type", "name", "description"):
    value = meta.get(key, "")
    if "\n" in str(value) or "\r" in str(value):
        raise SystemExit(f"invalid newline in metadata field {key}")
    print(value)
print(package["name"])
PY
)"
    mapfile -t fields <<< "$metadata"
    id="${fields[0]:-}"; kind="${fields[1]:-}"; name="${fields[2]:-}"; description="${fields[3]:-}"; bin="${fields[4]:-}"
    if ! valid_id "$id" || [[ "$kind" != SHELL && "$kind" != ERA ]] || [[ -z "$name" || -z "$description" || -z "$bin" ]]; then
        echo "Cargo.toml must define [package.metadata.eonos] id, type (SHELL or ERA), name, and description." >&2
        exit 2
    fi
    mkdir -p "$BUILD_DIR/app-target"
    RUSTFLAGS="-C link-arg=-T$EONOS_ROOT/userland/linker.ld -C link-arg=-z -C link-arg=max-page-size=4096 -C link-arg=-z -C link-arg=noexecstack" \
        cargo build --manifest-path "$source/Cargo.toml" --target "$TARGET" \
            --target-dir "$BUILD_DIR/app-target" --release -Zjson-target-spec \
            -Zbuild-std=core,alloc,compiler_builtins \
            -Zbuild-std-features=compiler-builtins-mem --bin "$bin"
    binary="$BUILD_DIR/app-target/x86_64-eonos-user/release/$bin"
    marker="EONOSAPP1:$kind"
    if ! grep -a -F -q "$marker" "$binary"; then
        echo "The ELF app type marker does not match Cargo metadata ($kind). Use eonos_app::app!($kind, main)." >&2
        exit 1
    fi
    package_file="$BUILD_DIR/apps/$id.eonapp"
    mkdir -p "$BUILD_DIR/apps"
    python3 - "$binary" "$package_file" "$id" "$kind" "$name" "$description" <<'PY'
import pathlib, struct, sys

elf_path, package_path, app_id, app_kind, name, description = sys.argv[1:]
metadata = f"EONOSAPP1\n{app_id}\n{app_kind}\n{name}\n{description}\n".encode("utf-8")
if not name or len(name.encode("utf-8")) > 80 or len(description.encode("utf-8")) > 160:
    raise SystemExit("EonOS app name must be 1-80 bytes and description at most 160 bytes")
if len(metadata) > 1024:
    raise SystemExit("EonOS app metadata exceeds the 1024 byte package limit")
elf = pathlib.Path(elf_path).read_bytes()
if len(elf) > 64 * 1024 * 1024:
    raise SystemExit("application ELF exceeds the 64 MiB package limit")
pathlib.Path(package_path).write_bytes(
    b"EONAPP1\0" + struct.pack("<IQ", len(metadata), len(elf)) + metadata + elf
)
PY
    if [[ "$(dirname "$source")" == "$APP_REPO" ]]; then
        packaged_copy="$APP_REPO/$id.eonapp"
        cp "$package_file" "$packaged_copy"
        echo "Copied package to $packaged_copy"
    fi
    echo "Built $kind app '$name': $package_file"
}

case "${1:-}" in
    new) shift; new_app "${1:-}" "${2:-}" ;;
    build) shift; build_app "${1:-}" ;;
    *) usage; exit 2 ;;
esac
