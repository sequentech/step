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

# Detect Architecture
case $(uname -m) in
    x86_64) ARCH="amd64" ;;
    aarch64|arm64) ARCH="arm64" ;;
    *) echo "Unsupported architecture: $(uname -m)"; exit 1 ;;
esac
echo "Target Architecture: $ARCH"

echo "--- [1/7] Cleaning and Preparing Output Directory ---"
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR/k3s/$ARCH" "$OUTPUT_DIR/deb-packages/$ARCH" "$OUTPUT_DIR/images"

echo "--- [2/7] Downloading K3s Airgap Artifacts ($ARCH) ---"
K3S_SUFFIX=""
if [ "$ARCH" = "arm64" ]; then K3S_SUFFIX="-arm64"; fi

curl -Lo "$OUTPUT_DIR/k3s/$ARCH/k3s" "https://github.com/k3s-io/k3s/releases/download/${K3S_VERSION}/k3s${K3S_SUFFIX}"
curl -Lo "$OUTPUT_DIR/k3s/$ARCH/k3s-airgap-images-${ARCH}.tar.zst" "https://github.com/k3s-io/k3s/releases/download/${K3S_VERSION}/k3s-airgap-images-${ARCH}.tar.zst"
curl -Lo "$OUTPUT_DIR/k3s/install.sh" "https://get.k3s.io"
chmod +x "$OUTPUT_DIR/k3s/$ARCH/k3s" "$OUTPUT_DIR/k3s/install.sh"

echo "--- [3/7] Downloading Debian Packages (Git/SSH for $ARCH) ---"
# Detect OS version for the target (default to jammy if not on linux)
if command -v lsb_release >/dev/null 2>&1; then
    OS_CODENAME=$(lsb_release -cs)
else
    OS_CODENAME="jammy"
fi

PACKAGES=("git" "openssh-client" "curl" "jq")
echo "Downloading packages for $ARCH ($OS_CODENAME)..."
# Use docker to get dependencies correctly for any host
docker run --rm --platform "linux/$ARCH" \
    -v "$OUTPUT_DIR/deb-packages/$ARCH:/output" \
    "ubuntu:$OS_CODENAME" bash -c "
        apt-get update
        apt-get install -y --download-only ${PACKAGES[*]}
        cp /var/cache/apt/archives/*.deb /output/
    "


echo "--- [5/7] Pulling Infrastructure & CI Base Images ---"
# Pulling only for current host architecture to keep it simple, 
# but infrastructure images are usually multi-arch.
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

echo "--- [6.5/7] Vendoring Offline Dependencies (Rust & Node) ---"
echo "Vendoring Rust crates..."
docker run --rm --platform "linux/$ARCH" -v "$PROJECT_ROOT/packages:/workspace" -w /workspace rust:1.90.0-slim-bookworm bash -c "
    mkdir -p .cargo
    cargo vendor > .cargo/config.toml
"

echo "Vendoring Node.js packages to yarn offline mirror..."
docker run --rm --platform "linux/$ARCH" -v "$PROJECT_ROOT/packages:/workspace" -w /workspace node:20-bookworm-slim bash -c "
    npm install -g yarn
    echo 'yarn-offline-mirror "./npm-packages-offline-cache"' > .yarnrc
    echo 'yarn-offline-mirror-pruning true' >> .yarnrc
    # Run yarn install to populate the offline mirror based on yarn.lock
    yarn install --frozen-lockfile
"

echo "--- [7/7] Finalizing Output ---"
cp -r "$AIRGAP_DIR/kubernetes" "$OUTPUT_DIR/"
cp "$AIRGAP_DIR/manage.sh" "$OUTPUT_DIR/"
cp "$AIRGAP_DIR/README.md" "$OUTPUT_DIR/"
chmod +x "$OUTPUT_DIR/manage.sh"

tar -czf "$OUTPUT_DIR/step-source.tar.gz" \
    -C "$PROJECT_ROOT" \
    --exclude="./airgap-output" \
    --exclude="./target" \
    --exclude="./node_modules" \
    .

echo "--- Preparation Complete! ---"
echo "Transfer 'airgap-output/' to the airgap machine."
