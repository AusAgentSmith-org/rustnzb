---
name: build
description: Build rustnzbd locally with cargo
disable-model-invocation: true
allowed-tools: Bash(cargo *)
user-invocable: true
argument-hint: "[--release] [--docker]"
---

# Build rustnzbd

Build the rustnzbd project.

## Usage

- `/build` — Debug build
- `/build --release` — Release build
- `/build --docker` — Build Docker image locally

## Steps

1. If `--docker`:
   ```bash
   docker build -t rustnzbd:local .
   ```

2. If `--release`:
   ```bash
   cargo build --release
   ```

3. Default (debug):
   ```bash
   cargo build
   ```

4. Report build result — if errors, show the first error clearly
5. On success, show binary size for release builds:
   ```bash
   ls -lh target/release/rustnzbd
   ```
