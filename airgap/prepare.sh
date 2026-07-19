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
TRIVY_VERSION="0.58.1"

# Release version stamped onto built images and recorded in the release manifest.
# Override with RELEASE_VERSION=x.y.z ./prepare.sh
RELEASE_VERSION="${RELEASE_VERSION:-$(git -C "$PROJECT_ROOT" describe --tags --always --dirty 2>/dev/null || date +%Y%m%d)}"
echo "Release Version: $RELEASE_VERSION"

# Detect Architecture
case $(uname -m) in
    x86_64) ARCH="amd64" ;;
    aarch64|arm64) ARCH="arm64" ;;
    *) echo "Unsupported architecture: $(uname -m)"; exit 1 ;;
esac
echo "Target Architecture: $ARCH"

echo "--- [1/9] Cleaning and Preparing Output Directory ---"
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR/k3s/$ARCH" "$OUTPUT_DIR/deb-packages/$ARCH" \
    "$OUTPUT_DIR/os-security-updates/$ARCH" "$OUTPUT_DIR/images" "$OUTPUT_DIR/release"

echo "--- [2/9] Downloading K3s Airgap Artifacts ($ARCH) ---"
K3S_SUFFIX=""
if [ "$ARCH" = "arm64" ]; then K3S_SUFFIX="-arm64"; fi

curl -Lo "$OUTPUT_DIR/k3s/$ARCH/k3s" "https://github.com/k3s-io/k3s/releases/download/${K3S_VERSION}/k3s${K3S_SUFFIX}"
curl -Lo "$OUTPUT_DIR/k3s/$ARCH/kubectl" "https://dl.k8s.io/release/v1.30.0/bin/linux/$ARCH/kubectl"
curl -Lo "$OUTPUT_DIR/k3s/$ARCH/k3s-airgap-images-${ARCH}.tar.zst" "https://github.com/k3s-io/k3s/releases/download/${K3S_VERSION}/k3s-airgap-images-${ARCH}.tar.zst"
curl -Lo "$OUTPUT_DIR/k3s/install.sh" "https://get.k3s.io"
chmod +x "$OUTPUT_DIR/k3s/$ARCH/k3s" "$OUTPUT_DIR/k3s/$ARCH/kubectl" "$OUTPUT_DIR/k3s/install.sh"

echo "--- [3/9] Downloading Debian Packages (Git/SSH for $ARCH) ---"
# Detect OS version for the target (default to jammy if not on linux)
if command -v lsb_release >/dev/null 2>&1; then
    OS_CODENAME=$(lsb_release -cs)
else
    OS_CODENAME="jammy"
fi

PACKAGES=("git" "openssh-client" "curl" "jq" "gnupg")
echo "Downloading packages for $ARCH ($OS_CODENAME)..."
# Use docker to get dependencies correctly for any host
docker run --rm --platform "linux/$ARCH" \
    -v "$OUTPUT_DIR/deb-packages/$ARCH:/output" \
    "ubuntu:$OS_CODENAME" bash -c "
        apt-get update
        apt-get install -y --download-only ${PACKAGES[*]}
        cp /var/cache/apt/archives/*.deb /output/
    "


echo "--- [4/9] Downloading OS Security Updates ($ARCH) ---"
# Bundle the security-pocket .debs so the offline server can be patched without
# internet access. manage.sh --update-os installs these on the airgapped node.
# We resolve them against the base image; the server applies whatever is newer
# than its installed set. Only the *-security pocket is used (no feature updates).
docker run --rm --platform "linux/$ARCH" \
    -v "$OUTPUT_DIR/os-security-updates/$ARCH:/output" \
    "ubuntu:$OS_CODENAME" bash -c '
        set -e
        codename="$(. /etc/os-release; echo "$VERSION_CODENAME")"
        # Restrict apt to the security pocket only. Handle both the legacy
        # sources.list and the deb822 ubuntu.sources (24.04+) layouts.
        rm -f /etc/apt/sources.list.d/ubuntu.sources
        cat > /etc/apt/sources.list <<L
deb http://security.ubuntu.com/ubuntu ${codename}-security main restricted universe multiverse
L
        apt-get update
        apt-get -y --download-only upgrade
        if ls /var/cache/apt/archives/*.deb >/dev/null 2>&1; then
            cp /var/cache/apt/archives/*.deb /output/
            echo "Bundled $(ls /output/*.deb | wc -l) security package(s)."
        else
            echo "No pending security updates for the base image."
        fi
    '


echo "--- [5/9] Pulling Infrastructure & CI Base Images ---"
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
echo "Building custom CI builder base image..."
docker build -t sequentech.local/ci-builder:latest -f "$PROJECT_ROOT/packages/Dockerfile.ci-builder" "$PROJECT_ROOT/packages"

echo "Building custom Gitea runner base image..."
docker build -t sequentech.local/runner-ubuntu:22.04 -f "$PROJECT_ROOT/packages/Dockerfile.gitea-runner" "$PROJECT_ROOT/packages"

echo "Building offline dependencies base image..."
docker build -t sequentech.local/offline-dependencies:latest -f "$PROJECT_ROOT/packages/Dockerfile.offline-dependencies" "$PROJECT_ROOT/packages"

echo "--- [6/9] Building Application Images ---"
echo "Building Immudb..."
docker build -t sequentech.local/immudb:latest -f "$PROJECT_ROOT/packages/Dockerfile.immudb" "$PROJECT_ROOT/packages"

echo "Building B4..."
docker build -t sequentech.local/b4:latest -f "$PROJECT_ROOT/packages/b4/Dockerfile.prod" "$PROJECT_ROOT/packages"

echo "Building Braid (Trustees)..."
docker build -t sequentech.local/braid:latest -f "$PROJECT_ROOT/packages/braid/Dockerfile.prod" "$PROJECT_ROOT/packages"

# Stamp every Sequent-built image with the release version (kept alongside the
# :latest tag the manifests reference) for traceable, reproducible releases.
SEQUENT_IMAGES=(
    "sequentech.local/postgresql"
    "sequentech.local/keycloak"
    "sequentech.local/ci-builder:latest"
    "sequentech.local/runner-ubuntu:22.04"
    "sequentech.local/offline-dependencies:latest"
    "sequentech.local/immudb:latest"
    "sequentech.local/b4:latest"
    "sequentech.local/braid:latest"
)
VERSIONED_IMAGES=()
for img in "${SEQUENT_IMAGES[@]}"; do
    repo="${img%%:*}"
    versioned="${repo}:${RELEASE_VERSION}"
    docker tag "$img" "$versioned"
    VERSIONED_IMAGES+=("$versioned")
done

echo "--- [7/9] Saving Images to Tarball ---"
ALL_IMAGES=(
    "${CI_RUNNER_IMAGES[@]}"
    "${CI_BUILD_ENV_IMAGES[@]}"
    "${INFRA_IMAGES[@]}"
    "${SEQUENT_IMAGES[@]}"
    "${VERSIONED_IMAGES[@]}"
)
docker save -o "$OUTPUT_DIR/images/step-airgap-infra.tar" "${ALL_IMAGES[@]}"

echo "--- [8/9] Scanning Images & Recording Release Manifest ---"
# Record the image content digest (immutable sha256 ID) of every bundled image.
IMAGE_DIGESTS_FILE="$OUTPUT_DIR/release/image-digests.txt"
{
    echo "# Sequent airgap release $RELEASE_VERSION ($ARCH)"
    echo "# Generated $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "# <image>  <sha256 image id>"
} > "$IMAGE_DIGESTS_FILE"
for img in "${ALL_IMAGES[@]}"; do
    digest=$(docker image inspect --format '{{.Id}}' "$img")
    printf '%s  %s\n' "$img" "$digest" >> "$IMAGE_DIGESTS_FILE"
done
echo "Wrote image digests to $IMAGE_DIGESTS_FILE"

TRIVY_REPORT="$OUTPUT_DIR/release/trivy-report.txt"
: > "$TRIVY_REPORT"
# Scan each Sequent-built image via the official Trivy container.
# The Docker socket is mounted so Trivy can reach local images directly.
# Do not fail the build on findings — the report is an audit artifact.
for img in "${SEQUENT_IMAGES[@]}"; do
    echo "Scanning $img..."
    {
        echo "================================================================"
        echo "Image: $img"
        echo "================================================================"
        docker run --rm \
            -v /var/run/docker.sock:/var/run/docker.sock \
            "aquasec/trivy:${TRIVY_VERSION}" \
            image --severity HIGH,CRITICAL --no-progress "$img" || true
        echo ""
    } >> "$TRIVY_REPORT"
done
echo "Wrote vulnerability report to $TRIVY_REPORT"

echo "--- [9/9] Finalizing Output ---"
cp -r "$AIRGAP_DIR/kubernetes" "$OUTPUT_DIR/"
cp "$AIRGAP_DIR/manage.sh" "$OUTPUT_DIR/"
cp "$AIRGAP_DIR/README.md" "$OUTPUT_DIR/"
chmod +x "$OUTPUT_DIR/manage.sh"

tar -czf "$OUTPUT_DIR/step-source.tar.gz" \
    -C "$PROJECT_ROOT" \
    --exclude="./airgap-output" \
    --exclude="./target" \
    --exclude="./node_modules" \
    --exclude="./.git" \
    .

echo "Generating SHA256 checksums for all release artifacts..."
# checksums.txt lets the airgap operator verify every transferred artifact
# (image tarball, source bundle, binaries, packages) with `sha256sum -c`.
CHECKSUMS_FILE="$OUTPUT_DIR/checksums.txt"
( cd "$OUTPUT_DIR" && \
    find . -type f ! -name checksums.txt -print0 \
        | sort -z | xargs -0 sha256sum > "$CHECKSUMS_FILE" )
echo "Wrote $CHECKSUMS_FILE"

echo "Signing checksums.txt with GPG..."
# A detached GPG signature over checksums.txt gives the airgap operator
# authenticity (who built the bundle), not just integrity (sha256sum). The
# private key never leaves this machine; only the signature and the public key
# travel in the bundle, and the operator verifies with `manage.sh --verify`.
#
# Provide a maintained signing identity via GPG_SIGNING_KEY_ID=<fingerprint> to
# sign from your own keyring. Otherwise a dedicated keypair is generated (and
# reused across runs) in AIRGAP_GNUPGHOME so the fingerprint stays stable and can
# be trusted out-of-band.
AIRGAP_GNUPGHOME="${AIRGAP_GNUPGHOME:-$PROJECT_ROOT/.airgap-gpg}"
GPG_KEY_UID="${GPG_KEY_UID:-Sequent Airgap Release Signing <legal@sequentech.io>}"
PUBKEY_FILE="$OUTPUT_DIR/release/airgap-signing-pubkey.asc"
SIGNATURE_FILE="$OUTPUT_DIR/checksums.txt.asc"

if [ -n "${GPG_SIGNING_KEY_ID:-}" ]; then
    GPG=(gpg --batch)
    GPG_KEY_ID="$GPG_SIGNING_KEY_ID"
else
    mkdir -p "$AIRGAP_GNUPGHOME"
    chmod 700 "$AIRGAP_GNUPGHOME"
    GPG=(gpg --homedir "$AIRGAP_GNUPGHOME" --batch --pinentry-mode loopback --passphrase '')
    if ! "${GPG[@]}" --list-secret-keys "$GPG_KEY_UID" >/dev/null 2>&1; then
        echo "Generating airgap signing keypair in $AIRGAP_GNUPGHOME..."
        "${GPG[@]}" --quick-generate-key "$GPG_KEY_UID" ed25519 sign never
    fi
    GPG_KEY_ID=$("${GPG[@]}" --list-secret-keys --with-colons "$GPG_KEY_UID" \
        | awk -F: '/^fpr:/ {print $10; exit}')
fi

"${GPG[@]}" --yes --armor --local-user "$GPG_KEY_ID" \
    --detach-sign --output "$SIGNATURE_FILE" "$CHECKSUMS_FILE"
"${GPG[@]}" --yes --armor --export "$GPG_KEY_ID" > "$PUBKEY_FILE"
GPG_FPR=$("${GPG[@]}" --list-keys --with-colons "$GPG_KEY_ID" \
    | awk -F: '/^fpr:/ {print $10; exit}')
echo "Wrote signature to $SIGNATURE_FILE"
echo "Wrote public key to $PUBKEY_FILE"

echo "--- Preparation Complete! ---"
echo "Release version: $RELEASE_VERSION"
echo "Signing key fingerprint: $GPG_FPR"
echo "Record this fingerprint and communicate it out-of-band so the airgap"
echo "operator can confirm the bundle's authenticity."
echo "Transfer 'airgap-output/' to the airgap machine."
echo "Verify authenticity and integrity on arrival with: ./manage.sh --verify"
