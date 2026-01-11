#!/bin/bash
# Test script to verify Rust commands work in dshell

# Test 1: Run cargo --version in a subshell via bash
echo "Test 1: Running 'bash -c cargo --version' in dshell..."
echo 'bash -c "cargo --version"' | timeout 3 ./target/release/dshell 2>&1 | grep -A5 "cargo" || echo "FAILED: cargo not found"

# Test 2: Check if rustc works
echo ""
echo "Test 2: Running 'bash -c rustc --version' in dshell..."
echo 'bash -c "rustc --version"' | timeout 3 ./target/release/dshell 2>&1 | grep -A5 "rustc" || echo "FAILED: rustc not found"

echo ""
echo "Note: Errors about 'No such device or address' are expected when not running in a real TTY"
echo "The important thing is whether the commands execute and return version info."
