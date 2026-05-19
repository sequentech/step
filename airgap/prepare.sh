#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -e

SCRIPT_PATH="$(cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P)"
PROJECT_ROOT=$(realpath "$SCRIPT_PATH/..")
AIRGAP_DIR="$PROJECT_ROOT/airgap"
OUTPUT_DIR="$PROJECT_ROOT/airgap-output"

# Versions
K3S_VERSION="v1.35.4+k3s1"

echo "--- [1/7] Cleaning and Preparing Output Directory ---"
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR/k3s" "$OUTPUT_DIR/deb-packages" "$OUTPUT_DIR/images" "$OUTPUT_DIR/actions-repos"

echo "--- [2/7] Downloading K3s Airgap Artifacts (Stable: $K3S_VERSION) ---"
curl -Lo "$OUTPUT_DIR/k3s/k3s" "https://github.com/k3s-io/k3s/releases/download/${K3S_VERSION}/k3s"
curl -Lo "$OUTPUT_DIR/k3s/k3s-airgap-images-amd64.tar.zst" "https://github.com/k3s-io/k3s/releases/download/${K3S_VERSION}/k3s-airgap-images-amd64.tar.zst"
curl -Lo "$OUTPUT_DIR/k3s/install.sh" "https://get.k3s.io"
chmod +x "$OUTPUT_DIR/k3s/k3s" "$OUTPUT_DIR/k3s/install.sh"

echo "--- [3/7] Downloading Debian Packages (Git/SSH) ---"
cd "$OUTPUT_DIR/deb-packages"
sudo apt-get update
sudo apt-get download git openssh-client curl jq
cd "$PROJECT_ROOT"

echo "--- [4/7] Caching GitHub Action Repositories ---"
ACTIONS=(
    "actions/checkout"
    "docker/setup-buildx-action"
    "docker/login-action"
    "docker/build-push-action"
)
for action in "${ACTIONS[@]}"; do
    echo "Cloning action: $action..."
    git clone --bare "https://github.com/$action.git" "$OUTPUT_DIR/actions-repos/${action##*/}.git"
done

echo "--- [5/7] Pulling Infrastructure & CI Base Images ---"
# Base images for CI builds
# We use catthehacker's images which are optimized for 'act' (used by Gitea)
CI_RUNNER_IMAGES=(
    "catthehacker/ubuntu:act-22.04"
    "docker:dind"
    "gitea/act_runner:latest"
)
CI_BUILD_ENV_IMAGES=(
    "rust:1.90.0-slim-bookworm"
    "node:20-alpine"
    "node:20-bookworm-slim"
    "debian:bookworm"
)
# Application Infra
INFRA_IMAGES=(
    "gitea/gitea:latest"
    "rustfs/rustfs:latest"
    "rabbitmq:3.12.11-management"
    "hasura/graphql-engine:v2.33.1.cli-migrations-v3"
    "postgres:18-bookworm"
)

for img in "${CI_RUNNER_IMAGES[@]}" "${CI_BUILD_ENV_IMAGES[@]}" "${INFRA_IMAGES[@]}"; do
    echo "Pulling $img..."
    docker pull "$img"
done

# Build custom infra
echo "Building custom Postgres/Keycloak..."
docker build -t sequentech.local/postgresql "$PROJECT_ROOT/.devcontainer/postgresql"
docker build -t sequentech.local/keycloak -f "$PROJECT_ROOT/packages/Dockerfile.keycloak" "$PROJECT_ROOT/packages"

echo "--- [6/7] Saving Images to Tarball ---"
ALL_IMAGES=(
    "${CI_RUNNER_IMAGES[@]}" 
    "${CI_BUILD_ENV_IMAGES[@]}" 
    "${INFRA_IMAGES[@]}" 
    "sequentech.local/postgresql" 
    "sequentech.local/keycloak"
)
docker save -o "$OUTPUT_DIR/images/step-airgap-infra.tar" "${ALL_IMAGES[@]}"

echo "--- [7/7] Finalizing Output ---"
cp -r "$AIRGAP_DIR/kubernetes" "$OUTPUT_DIR/"
cp "$AIRGAP_DIR/manage.sh" "$OUTPUT_DIR/"
cp "$AIRGAP_DIR/README.md" "$OUTPUT_DIR/"
chmod +x "$OUTPUT_DIR/manage.sh"

# Package source code
tar -czf "$OUTPUT_DIR/step-source.tar.gz" \
    --exclude="./airgap-output" \
    --exclude="./target" \
    --exclude="./node_modules" \
    .

echo "--- Preparation Complete! ---"
echo "Transfer 'airgap-output/' to the airgap machine."
