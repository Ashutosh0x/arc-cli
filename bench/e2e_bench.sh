#!/usr/bin/env bash
set -e

echo "🚀 ARC Hyperfine E2E Benchmarks"

# Note: Ensure ARC is built in release mode first!
cargo build --release

ARC_BIN="./target/release/arc"

echo "----------------------------------------"
echo "1. Cold Start Time (No network, just load)"
echo "----------------------------------------"
hyperfine --warmup 3 \
  "$ARC_BIN --help" \
  "gh copilot --help"

echo "----------------------------------------"
echo "2. Local File Scanning Speed"
echo "----------------------------------------"
# Create a dummy large file structure if none exists
touch dummy.txt

hyperfine --warmup 1 \
  "$ARC_BIN chat 'Read dummy.txt and summarize'"
