#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -e

SCRIPT_PATH="$(cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P)"
PROJECT_ROOT=$(realpath "$SCRIPT_PATH/..")
AIRGAP_DIR="$PROJECT_ROOT/airgap"
PACKAGES_DIR="$PROJECT_ROOT/packages"
DEVCONTAINER_DIR="$PROJECT_ROOT/.devcontainer"
OUTPUT_DIR="$PROJECT_ROOT/airgap-output"

echo "--- [1/6] Cleaning and Preparing Output Directory ---"
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

echo "--- [2/6] Vendoring Dependencies (For Offline Build) ---"
cd "$PACKAGES_DIR"
mkdir -p .cargo
echo "Vendoring Rust crates..."
# We use a temporary file for the vendor output
cargo vendor > .cargo/config.toml.vendored

# Prepare .cargo/config.toml safely
if [ ! -f .cargo/config.toml ]; then
    touch .cargo/config.toml
fi

# We only append if the sections don't exist, or we replace them
# Remove existing vendor source configs to avoid duplicates
sed -i '/\[source.crates-io\]/d' .cargo/config.toml
sed -i '/replace-with = "vendored-sources"/d' .cargo/config.toml
sed -i '/\[source.vendored-sources\]/d' .cargo/config.toml
sed -i '/directory = "vendor"/d' .cargo/config.toml

# Append the new vendor config
cat .cargo/config.toml.vendored >> .cargo/config.toml
rm .cargo/config.toml.vendored

echo "Vendoring NPM packages..."
OFFLINE_MIRROR="$PACKAGES_DIR/npm-packages-offline-cache"
mkdir -p "$OFFLINE_MIRROR"
yarn config set yarn-offline-mirror "$OFFLINE_MIRROR"
yarn config set yarn-offline-mirror-pruning true
yarn install --frozen-lockfile

echo "--- [3/6] Building/Pulling Application and Infra Images ---"
# Build unified dev tooling image
docker build -t step-airgap-dev -f "$AIRGAP_DIR/Dockerfile.airgap" "$PROJECT_ROOT"

# Build production app images
docker build -t sequentech.local/admin-portal --build-arg SPA_NAME=admin-portal -f "$PACKAGES_DIR/Dockerfile.prod" "$PACKAGES_DIR"
docker build -t sequentech.local/voting-portal --build-arg SPA_NAME=voting-portal -f "$PACKAGES_DIR/Dockerfile.prod" "$PACKAGES_DIR"
docker build -t sequentech.local/harvest -f "$PACKAGES_DIR/harvest/Dockerfile" "$PACKAGES_DIR"
docker build -t sequentech.local/windmill -f "$PACKAGES_DIR/windmill/Dockerfile" "$PACKAGES_DIR"
docker build -t sequentech.local/b3 -f "$PACKAGES_DIR/b3/Dockerfile.prod" "$PACKAGES_DIR"

# Build custom infra images
docker build -t sequentech.local/postgresql "$DEVCONTAINER_DIR/postgresql"
docker build -t sequentech.local/postgresql-b3 "$DEVCONTAINER_DIR/postgresql-b3"
docker build -t sequentech.local/keycloak -f "$PACKAGES_DIR/Dockerfile.keycloak" "$PACKAGES_DIR"
docker build -t sequentech.local/immudb -f "$PACKAGES_DIR/Dockerfile.immudb" "$PACKAGES_DIR"
docker build -t sequentech.local/configure-minio "$DEVCONTAINER_DIR/minio"

# Pull external base/infra images
EXTERNAL_IMAGES=(
    "rust:1.90.0-slim-bookworm"
    "node:20-alpine"
    "node:20-bookworm-slim"
    "rustfs/rustfs:latest"
    "nginx:latest"
    "rabbitmq:3.12.11-management"
    "hasura/graphql-engine:v2.33.1.cli-migrations-v3"
    "hasura/graphql-data-connector:v2.31.0"
    "postgres:18-bookworm"
)
for img in "${EXTERNAL_IMAGES[@]}"; do echo "Pulling $img..."; docker pull "$img"; done

echo "--- [4/6] Downloading Debian Packages (for Offline OS Setup) ---"
mkdir -p "$OUTPUT_DIR/deb-packages"
cd "$OUTPUT_DIR/deb-packages"
echo "Downloading .deb files for Docker and Git..."
sudo apt-get update
sudo apt-get download docker.io docker-compose-v2 docker-buildx git || echo "Warning: apt-get download failed. Ensure you are on a Debian/Ubuntu system."

echo "--- [5/6] Packaging Management Tools and Configs ---"
cd "$PROJECT_ROOT"
# Application and Infra Images
ALL_IMAGES=(
    "step-airgap-dev"
    "sequentech.local/admin-portal"
    "sequentech.local/voting-portal"
    "sequentech.local/harvest"
    "sequentech.local/windmill"
    "sequentech.local/b3"
    "sequentech.local/postgresql"
    "sequentech.local/postgresql-b3"
    "sequentech.local/keycloak"
    "sequentech.local/immudb"
    "sequentech.local/configure-minio"
    "${EXTERNAL_IMAGES[@]}"
)
docker save -o "$OUTPUT_DIR/step-airgap-all-images.tar" "${ALL_IMAGES[@]}"

# Config files and scripts from the airgap/ directory
cp "$AIRGAP_DIR/docker-compose.dev.yml" "$OUTPUT_DIR/"
cp "$AIRGAP_DIR/docker-compose.server.yml" "$OUTPUT_DIR/"
cp "$AIRGAP_DIR/README.md" "$OUTPUT_DIR/"
cp "$AIRGAP_DIR/manage.sh" "$OUTPUT_DIR/"
chmod +x "$OUTPUT_DIR/manage.sh"

echo "--- [6/6] Packaging Source Code for Dev Machine ---"
tar -czf "$OUTPUT_DIR/step-source.tar.gz" \
    --exclude="./airgap-output" \
    --exclude="./target" \
    --exclude="./node_modules" \
    --exclude="*.tar" \
    .

echo "--- Preparation Complete! ---"
echo "Transfer the 'airgap-output/' directory to the airgap."
echo "Inside the airgap, run: ./manage.sh --help"
