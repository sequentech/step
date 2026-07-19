#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -e

SCRIPT_PATH="$(cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P)"
PROJECT_ROOT="$SCRIPT_PATH"

show_help() {
    echo "Sequent K3s Airgap Management Tool"
    echo ""
    echo "Usage: ./manage.sh [command]"
    echo ""
    echo "Commands:"
    echo "  --verify         Verify the bundle's GPG signature and sha256 checksums."
    echo "  --setup-server   Install K3s, load system images, and start the cluster."
    echo "  --setup-client   Install Git/SSH packages for the Ubuntu Desktop."
    echo "  --deploy         Load infrastructure images and apply Kubernetes manifests."
    echo "  --update-os      Apply bundled OS security updates on the offline server."
    echo "  --run-dev        Extract source and provide instructions for Gitea push."
    echo "  --help           Show this help message."
    echo ""
}

case "$1" in
    --verify)
        echo "--- Verifying Airgap Bundle ---"
        CHECKSUMS_FILE="$PROJECT_ROOT/checksums.txt"
        SIGNATURE_FILE="$PROJECT_ROOT/checksums.txt.asc"
        PUBKEY_FILE="$PROJECT_ROOT/release/airgap-signing-pubkey.asc"

        for f in "$CHECKSUMS_FILE" "$SIGNATURE_FILE" "$PUBKEY_FILE"; do
            if [ ! -f "$f" ]; then
                echo "Error: required file not found: $f"
                exit 1
            fi
        done

        # Import the shipped public key into a throwaway keyring so verification
        # never depends on (or pollutes) the operator's own GPG configuration.
        VERIFY_GNUPGHOME=$(mktemp -d)
        chmod 700 "$VERIFY_GNUPGHOME"
        trap 'rm -rf "$VERIFY_GNUPGHOME"' EXIT
        GPG=(gpg --homedir "$VERIFY_GNUPGHOME" --batch)
        "${GPG[@]}" --import "$PUBKEY_FILE"

        # The bundle's public key alone only proves the archive is self-consistent.
        # Authenticity requires matching the key against the fingerprint the build
        # operator communicated out-of-band. Set EXPECTED_FINGERPRINT to enforce it.
        ACTUAL_FPR=$("${GPG[@]}" --list-keys --with-colons \
            | awk -F: '/^fpr:/ {print $10; exit}')
        echo "Signing key fingerprint: $ACTUAL_FPR"
        if [ -n "${EXPECTED_FINGERPRINT:-}" ]; then
            normalized_expected=$(printf '%s' "$EXPECTED_FINGERPRINT" | tr -d '[:space:]' | tr '[:lower:]' '[:upper:]')
            normalized_actual=$(printf '%s' "$ACTUAL_FPR" | tr '[:lower:]' '[:upper:]')
            if [ "$normalized_expected" != "$normalized_actual" ]; then
                echo "Error: fingerprint mismatch!"
                echo "  expected: $normalized_expected"
                echo "  actual:   $normalized_actual"
                exit 1
            fi
            echo "Fingerprint matches the expected value."
        else
            echo "Warning: EXPECTED_FINGERPRINT not set — confirm the fingerprint"
            echo "above matches the value communicated out-of-band by the builder."
        fi

        echo "--- Verifying GPG signature over checksums.txt ---"
        if ! "${GPG[@]}" --verify "$SIGNATURE_FILE" "$CHECKSUMS_FILE"; then
            echo "Error: GPG signature verification failed!"
            exit 1
        fi
        echo "Signature is valid."

        echo "--- Verifying sha256 checksums of all artifacts ---"
        ( cd "$PROJECT_ROOT" && sha256sum -c checksums.txt )
        echo "Bundle verified successfully."
        ;;

    --setup-server)
        echo "--- Installing K3s Server ---"
        ARCH=$(dpkg --print-architecture)
        sudo mkdir -p /var/lib/rancher/k3s/agent/images/
        sudo cp "$PROJECT_ROOT/k3s/$ARCH/k3s-airgap-images-${ARCH}.tar.zst" /var/lib/rancher/k3s/agent/images/
        sudo cp "$PROJECT_ROOT/k3s/$ARCH/k3s" /usr/local/bin/k3s
        
        # Install K3s in airgap mode using the local install script
        export INSTALL_K3S_SKIP_DOWNLOAD=true
        export INSTALL_K3S_BIN_DIR=/usr/local/bin
        sh "$PROJECT_ROOT/k3s/install.sh"
        
        echo "--- Configuring Internal Registry Trust ---"
        # We use a static ClusterIP (10.43.10.10) for Gitea to avoid host-level DNS resolution
        sudo mkdir -p /etc/rancher/k3s
        sudo tee /etc/rancher/k3s/registries.yaml > /dev/null <<EOF
mirrors:
  "gitea.local:3000":
    endpoint:
      - "http://10.43.10.10:3000"
  "gitea.gitea:3000":
    endpoint:
      - "http://10.43.10.10:3000"
configs:
  "gitea.local:3000":
    tls:
      insecure_skip_verify: true
  "gitea.gitea:3000":
    tls:
      insecure_skip_verify: true
EOF
        sudo systemctl restart k3s

        echo "K3s is installed! Waiting for node to be ready..."
        until sudo k3s kubectl get node | grep -v "NotReady" | grep -q "Ready"; do sleep 5; done
        echo "Node is Ready!"
        ;;

    --setup-client)
        echo "--- Installing Client Packages ---"
        ARCH=$(dpkg --print-architecture)
        if [ -d "$PROJECT_ROOT/deb-packages/$ARCH" ]; then
            cd "$PROJECT_ROOT/deb-packages/$ARCH"
            sudo dpkg -i *.deb
            echo "Git and SSH are installed for $ARCH!"
        else
            echo "Error: No packages found for architecture $ARCH"
            exit 1
        fi
        ;;

    --deploy)
        echo "--- Setting Passwords (override with env vars before running) ---"
        POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-password}"
        RUSTFS_PASSWORD="${RUSTFS_PASSWORD:-password}"
        GITEA_ADMIN_PASSWORD="${GITEA_ADMIN_PASSWORD:-admin123}"
        HASURA_ADMIN_SECRET="${HASURA_ADMIN_SECRET:-admin123}"
        ACTIONS_ADMIN_SECRET="${ACTIONS_ADMIN_SECRET:-admin123}"
        MASTER_SECRET="${MASTER_SECRET:-dummy_master_secret_for_airgap_certification}"
        S3_SECRET="${S3_SECRET:-password}"

        echo "--- Ensuring TLS Certificate is Provisioned ---"
        if ! sudo k3s kubectl get secret step-tls-cert -n step-apps &>/dev/null; then
            echo "Generating self-signed TLS certificate for portal.local and gitea.local..."
            sudo openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
              -keyout /tmp/tls.key -out /tmp/tls.crt \
              -subj '/CN=portal.local' \
              -addext 'subjectAltName = DNS:portal.local,DNS:gitea.local'

            for ns in step-apps step-infra gitea; do
                sudo k3s kubectl create secret tls step-tls-cert --key=/tmp/tls.key --cert=/tmp/tls.crt -n "$ns" --dry-run=client -o yaml | sudo k3s kubectl apply -f -
            done
            rm -f /tmp/tls.key /tmp/tls.crt
        else
            echo "TLS certificate already exists."
        fi

        echo "--- Provisioning Application Secrets ---"
        sudo k3s kubectl create secret generic step-secrets \
          -n step-infra \
          --from-literal=POSTGRES_PASSWORD="${POSTGRES_PASSWORD}" \
          --from-literal=RUSTFS_ROOT_PASSWORD="${RUSTFS_PASSWORD}" \
          --from-literal=KC_DB_PASSWORD="${POSTGRES_PASSWORD}" \
          --dry-run=client -o yaml | sudo k3s kubectl apply -f -

        sudo k3s kubectl create secret generic step-secrets \
          -n step-apps \
          --from-literal=POSTGRES_PASSWORD="${POSTGRES_PASSWORD}" \
          --from-literal=AWS_S3_ACCESS_SECRET="${S3_SECRET}" \
          --from-literal=KEYCLOAK_DB__PASSWORD="${POSTGRES_PASSWORD}" \
          --from-literal=HASURA_DB__PASSWORD="${POSTGRES_PASSWORD}" \
          --from-literal=B3_PG_PASSWORD="${POSTGRES_PASSWORD}" \
          --from-literal=ACTIONS_ADMIN_SECRET="${ACTIONS_ADMIN_SECRET}" \
          --from-literal=MASTER_SECRET="${MASTER_SECRET}" \
          --from-literal=HASURA_GRAPHQL_ADMIN_SECRET="${HASURA_ADMIN_SECRET}" \
          --dry-run=client -o yaml | sudo k3s kubectl apply -f -

        sudo k3s kubectl create secret generic step-secrets \
          -n gitea \
          --from-literal=GITEA_ADMIN_PASSWORD="${GITEA_ADMIN_PASSWORD}" \
          --from-literal=GITEA_REGISTRY_PASSWORD="${GITEA_ADMIN_PASSWORD}" \
          --dry-run=client -o yaml | sudo k3s kubectl apply -f -

        for ns in step-apps step-infra; do
          sudo k3s kubectl create secret docker-registry gitea-pull-secret \
            -n "$ns" \
            --docker-server=gitea.gitea:3000 \
            --docker-username=admin \
            --docker-password="${GITEA_ADMIN_PASSWORD}" \
            --dry-run=client -o yaml | sudo k3s kubectl apply -f -
        done

        echo "--- Loading Infrastructure Images into K3s (Background) ---"
        sudo mkdir -p /var/lib/rancher/k3s/agent/images/
        sudo cp "$PROJECT_ROOT/images/step-airgap-infra.tar" /var/lib/rancher/k3s/agent/images/
        
        echo "--- Applying Kubernetes Manifests ---"
        # Fully declarative. Kubernetes Jobs and InitContainers handle the state.
        sudo k3s kubectl apply -f "$PROJECT_ROOT/kubernetes/"
        
        echo "Stack is deploying! Use 'kubectl get pods -A' to monitor."
        ;;

    --update-os)
        echo "--- Applying Offline OS Security Updates ---"
        ARCH=$(dpkg --print-architecture)
        UPDATE_DIR="$PROJECT_ROOT/os-security-updates/$ARCH"
        if [ ! -d "$UPDATE_DIR" ]; then
            echo "Error: No OS update bundle found for architecture $ARCH"
            exit 1
        fi
        if ! ls "$UPDATE_DIR"/*.deb >/dev/null 2>&1; then
            echo "No security update packages bundled — nothing to apply."
            exit 0
        fi
        echo "Installing $(ls "$UPDATE_DIR"/*.deb | wc -l) package(s) from $UPDATE_DIR"
        # All dependencies are present in the bundle, so dpkg resolves ordering
        # from the full set. --force-confold keeps existing config files.
        sudo dpkg -i --force-confold "$UPDATE_DIR"/*.deb
        echo "OS security updates applied. Reboot if a kernel package was updated."
        ;;

    --run-dev)
        GITEA_ADMIN_PASSWORD="${GITEA_ADMIN_PASSWORD:-admin123}"

        echo "--- Preparing Development Source ---"
        if [ ! -d "$PROJECT_ROOT/source" ]; then
            mkdir -p "$PROJECT_ROOT/source"
            tar -xzf "$PROJECT_ROOT/step-source.tar.gz" -C "$PROJECT_ROOT/source"
        fi
        echo "Source extracted to $PROJECT_ROOT/source"
        echo ""
        echo "--- Configuring Local DNS Resolution ---"
        sudo sh -c 'grep -q "gitea.local" /etc/hosts || echo "127.0.0.1 gitea.local portal.local" >> /etc/hosts'
        echo "Domains configured."
        echo ""
        echo "--- Registering SSH Key with Gitea ---"
        SSH_PUB_KEY=""
        KEY_FILE=""
        for candidate in "$HOME/.ssh/id_ed25519.pub" "$HOME/.ssh/id_rsa.pub"; do
            if [ -f "$candidate" ]; then
                SSH_PUB_KEY=$(cat "$candidate")
                KEY_FILE="$candidate"
                break
            fi
        done
        if [ -z "$SSH_PUB_KEY" ]; then
            echo "No SSH public key found. Generate one first:"
            echo "  ssh-keygen -t ed25519"
            echo "Then re-run: ./manage.sh --run-dev"
            exit 1
        fi
        echo "Found key: $KEY_FILE"
        KEY_JSON=$(jq -n --arg key "$SSH_PUB_KEY" --arg title "airgap-$(hostname)" \
            '{"key": $key, "read_only": false, "title": $title}')
        HTTP_STATUS=$(curl -sk -o /dev/null -w "%{http_code}" \
            -X POST "https://gitea.local/api/v1/user/keys" \
            -u "admin:${GITEA_ADMIN_PASSWORD}" \
            -H "Content-Type: application/json" \
            -d "$KEY_JSON")
        case "$HTTP_STATUS" in
            201) echo "SSH key registered with Gitea." ;;
            422) echo "SSH key already registered with Gitea." ;;
            *)   echo "Warning: Gitea API returned HTTP $HTTP_STATUS. Add the key manually at https://gitea.local/-/user/settings/keys" ;;
        esac
        echo ""
        echo "To start developing:"
        echo "1. Browse to https://gitea.local (accept the self-signed certificate warning)"
        echo "2. Push the source to Gitea:"
        echo "   cd source"
        echo "   git remote add origin ssh://git@gitea.local:2222/admin/step.git"
        echo "   git push -u origin main"
        ;;

    *)
        show_help
        ;;
esac
