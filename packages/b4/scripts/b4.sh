#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
set -euo pipefail

# --- Environment variables ---
export AWS_ENDPOINT_URL="http://localhost:4566"
export AWS_ACCESS_KEY_ID="test"
export AWS_SECRET_ACCESS_KEY="test"
export AWS_REGION="us-east-1"
export S3_BUCKET_NAME="wbraid-messages"
export AWS_FORCE_PATH_STYLE="true"
export RUST_LOG="b4=info"

# Set DATABASE_URL to workspace root b4.db
workspace_root="$(pwd)"
export DATABASE_URL="sqlite:${workspace_root}/b4.db?mode=rwc"

# Save current directory
original_dir="$(pwd)"

# Ensure we return to the original directory even if script errors
cleanup() {
    cd "$original_dir"
}
trap cleanup EXIT

# --- Script logic ---

# Remove old DB inside crates/b4 if it exists
rm -f "./b4/b4.db"

# Run service from workspace root
cargo run --bin b4 --release
