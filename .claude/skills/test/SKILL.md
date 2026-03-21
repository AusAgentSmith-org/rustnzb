---
name: test
description: Run the rustnzbd Rust test suite
disable-model-invocation: true
allowed-tools: Bash(cargo *)
user-invocable: true
argument-hint: "[test-name-or-module] [-- --nocapture]"
---

# Run Tests

Run the rustnzbd Rust test suite.

## Usage

- `/test` — Run all tests
- `/test decode` — Run tests matching "decode"
- `/test -- --nocapture` — Run all tests with output visible
- `/test nzb_core` — Run tests in nzb-core crate

## Steps

1. If no arguments, run all tests:
   ```bash
   cargo test --workspace
   ```

2. With arguments, pass directly:
   ```bash
   cargo test $ARGUMENTS
   ```

3. If tests fail:
   - Read the failure output
   - Identify the failing test and source file
   - Fix the code
   - Re-run the specific failing test to confirm
   - Run full suite to check for regressions
