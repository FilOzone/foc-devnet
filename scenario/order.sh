#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# order.sh — Declares the scenario execution order.
#
# Each entry is the basename of a script under scenario/.
# Scenarios share the same running devnet and execute serially
# in the order listed here.
# ─────────────────────────────────────────────────────────────

# shellcheck disable=SC2034  # SCENARIOS is used by run.sh which sources this file
SCENARIOS=(
  test_containers
  test_basic_balances
)
