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
    echo "  --setup-server   Install K3s, load system images, and start the cluster."
    echo "  --setup-client   Install Git/SSH packages for the Ubuntu Desktop."
    echo "  --deploy         Load infrastructure images and apply Kubernetes manifests."
    echo "  --run-dev        Extract source and provide instructions for Gitea push."
    echo "  --help           Show this help message."
    echo ""
}

case "$1" in
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
        until sudo k3s kubectl get node | grep -q "Ready"; do sleep 5; done
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

        sudo k3s kubectl create secret docker-registry gitea-pull-secret \
          -n step-apps \
          --docker-server=gitea.gitea:3000 \
          --docker-username=admin \
          --docker-password="${GITEA_ADMIN_PASSWORD}" \
          --dry-run=client -o yaml | sudo k3s kubectl apply -f -

        echo "--- Loading Infrastructure Images into K3s (Background) ---"
        sudo mkdir -p /var/lib/rancher/k3s/agent/images/
        sudo cp "$PROJECT_ROOT/images/step-airgap-infra.tar" /var/lib/rancher/k3s/agent/images/
        
        echo "--- Applying Kubernetes Manifests ---"
        # Fully declarative. Kubernetes Jobs and InitContainers handle the state.
        sudo k3s kubectl apply -f "$PROJECT_ROOT/kubernetes/"
        
        echo "Stack is deploying! Use 'kubectl get pods -A' to monitor."
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
