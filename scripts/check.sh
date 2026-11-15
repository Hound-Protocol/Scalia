#!/usr/bin/env bash
# Quiet wrapper for verify.sh: full log to /tmp/verify.log; prints one line on success, or the failing step
# with the last 30 lines on failure. Use this in agent sessions (saves reading the whole log).
cd "$(dirname "$0")/.."
if bash scripts/verify.sh > /tmp/verify.log 2>&1; then
  echo "verify: ALL CHECKS PASSED ($(grep -c '^== ' /tmp/verify.log) steps)"
else
  echo "verify: FAILED at: $(grep '^== ' /tmp/verify.log | tail -1)"
  grep -nE 'panicked|^error|FAILED|^FAIL' /tmp/verify.log | head -15
  echo "--- last 30 lines ---"; tail -30 /tmp/verify.log
  exit 1
fi
