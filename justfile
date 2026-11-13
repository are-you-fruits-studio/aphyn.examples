# Build and run the Aphyn examples.
#
#   just                    list recipes
#   just dev cubes          cargo run the example (fast iteration, debug build)
#   just build cubes        build a standalone game with `aphyn build` into examples/cubes/builds/
#   just run cubes          run that build
#   just lint               fmt check + clippy for every example (needs bash)
#
# `just build` reads the engine tag from examples/<name>/scripts/Cargo.toml, downloads the
# matching released `aphyn` binary into .engine/<tag>/ once, and checks it against the
# release's published SHA-256 before using it.

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# All examples share one target dir, so the engine is compiled once. `just build` opts out,
# because `aphyn build` reads the output from examples/<name>/scripts/target.
export CARGO_TARGET_DIR := justfile_directory() / "target"

[private]
default:
    @just --list

# Fails unless `name` is a plain example name with examples/<name>/scripts/Cargo.toml
[private]
check name:
    @{{ if name =~ '^[a-z0-9_]+$' { if path_exists("examples/" + name + "/scripts/Cargo.toml") == "true" { "" } else { error("no such example: " + name) } } else { error("invalid example name: " + name) } }}

# Run an example with cargo (debug build)
dev name: (check name)
    cargo run --manifest-path examples/{{name}}/scripts/Cargo.toml

# Build a standalone game with `aphyn build`
[unix]
build name: (check name)
    #!/usr/bin/env bash
    set -euo pipefail
    tag=$(sed -n 's/^aphyn = .*tag = "\([^"]*\)".*/\1/p' "examples/{{name}}/scripts/Cargo.toml")
    [ -n "$tag" ] || { echo "no aphyn tag in examples/{{name}}/scripts/Cargo.toml" >&2; exit 1; }
    dir=".engine/$tag/linux"
    if [ ! -x "$dir/aphyn" ]; then
        asset="aphyn-${tag#v}-x86_64-unknown-linux-gnu.tar.gz"
        url="https://github.com/are-you-fruits-studio/aphyn/releases/download/$tag/$asset"
        tmp=$(mktemp -d)
        trap 'rm -rf "$tmp"' EXIT
        echo "Downloading aphyn $tag: $url"
        curl -fsSL -o "$tmp/$asset" "$url"
        curl -fsSL -o "$tmp/$asset.sha256" "$url.sha256"
        expected=$(cut -d' ' -f1 "$tmp/$asset.sha256")
        actual=$(sha256sum "$tmp/$asset" | cut -d' ' -f1)
        [ "$expected" = "$actual" ] || { echo "checksum mismatch for $asset" >&2; exit 1; }
        mkdir "$tmp/out"
        tar -xzf "$tmp/$asset" -C "$tmp/out"
        mkdir -p "$dir"
        mv "$tmp/out/aphyn" "$dir/aphyn"
    fi
    # `aphyn build` expects the output in scripts/target, not the shared target dir
    env -u CARGO_TARGET_DIR "$dir/aphyn" build "$(pwd)/examples/{{name}}"

# Build a standalone game with `aphyn build`
[windows]
build name: (check name)
    $ErrorActionPreference = 'Stop'; \
    $m = Select-String -Path "examples/{{name}}/scripts/Cargo.toml" -Pattern '^aphyn = .*tag = "([^"]+)"'; \
    if (-not $m) { Write-Error "no aphyn tag in examples/{{name}}/scripts/Cargo.toml" }; \
    $tag = $m.Matches[0].Groups[1].Value; \
    $dir = ".engine/$tag/windows"; \
    if (!(Test-Path "$dir/aphyn.exe")) { \
        $asset = "aphyn-$($tag.TrimStart('v'))-x86_64-pc-windows-msvc.zip"; \
        $url = "https://github.com/are-you-fruits-studio/aphyn/releases/download/$tag/$asset"; \
        $tmp = Join-Path ([IO.Path]::GetTempPath()) ([IO.Path]::GetRandomFileName()); \
        New-Item -ItemType Directory -Path $tmp | Out-Null; \
        try { \
            Write-Host "Downloading aphyn ${tag}: $url"; \
            Invoke-WebRequest -Uri $url -OutFile "$tmp/$asset"; \
            Invoke-WebRequest -Uri "$url.sha256" -OutFile "$tmp/$asset.sha256"; \
            $expected = ((Get-Content "$tmp/$asset.sha256" -Raw).Trim() -split '\s+')[0]; \
            $actual = (Get-FileHash "$tmp/$asset" -Algorithm SHA256).Hash; \
            if ($expected -ne $actual) { Write-Error "checksum mismatch for $asset" }; \
            Expand-Archive -Path "$tmp/$asset" -DestinationPath "$tmp/out"; \
            New-Item -ItemType Directory -Force -Path $dir | Out-Null; \
            Move-Item "$tmp/out/aphyn.exe" "$dir/aphyn.exe"; \
        } finally { Remove-Item -Recurse -Force $tmp }; \
    }; \
    Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue; \
    & (Resolve-Path "$dir/aphyn.exe").Path build "$((Get-Location).Path)/examples/{{name}}"

# Run a game built with `just build`
[unix]
run name: (check name)
    cd examples/{{name}}/builds && ./{{name}}

# Run a game built with `just build`
[windows]
run name: (check name)
    Set-Location examples/{{name}}/builds; & ./{{name}}.exe

# Build, then run
build-n-run name: (build name) (run name)

# Format one example
fmt name: (check name)
    cargo fmt --manifest-path examples/{{name}}/scripts/Cargo.toml

# Clippy for one example, warnings as errors
clippy name: (check name)
    cargo clippy --manifest-path examples/{{name}}/scripts/Cargo.toml -- -D warnings

# Fmt check + clippy for every example (needs bash)
lint:
    bash .github/scripts/examples.sh lint

# Check that every example pins the same aphyn tag (needs bash)
check-tags:
    bash .github/scripts/examples.sh check-tags

# Remove an example's target/ and builds/
[unix]
clean name: (check name)
    rm -rf examples/{{name}}/scripts/target examples/{{name}}/builds

# Remove an example's target/ and builds/
[windows]
clean name: (check name)
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue examples/{{name}}/scripts/target, examples/{{name}}/builds
