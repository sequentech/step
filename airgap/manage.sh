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
        sudo mkdir -p /etc/rancher/k3s
        sudo tee /etc/rancher/k3s/registries.yaml > /dev/null <<EOF
mirrors:
  "gitea.local:3000":
    endpoint:
      - "http://gitea.gitea:3000"
configs:
  "gitea.local:3000":
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
        echo "--- Loading Infrastructure Images into K3s ---"
        sudo mkdir -p /var/lib/rancher/k3s/agent/images/infra/
        sudo cp "$PROJECT_ROOT/images/step-airgap-infra.tar" /var/lib/rancher/k3s/agent/images/infra/
        sudo k3s ctr images import /var/lib/rancher/k3s/agent/images/infra/step-airgap-infra.tar
        
        echo "--- Applying Kubernetes Manifests ---"
        sudo k3s kubectl apply -f "$PROJECT_ROOT/kubernetes/01-namespaces.yaml"
        sudo k3s kubectl apply -f "$PROJECT_ROOT/kubernetes/02-gitea.yaml"
        sudo k3s kubectl apply -f "$PROJECT_ROOT/kubernetes/03-infra.yaml"
        sudo k3s kubectl apply -f "$PROJECT_ROOT/kubernetes/05-ingress.yaml"
        sudo k3s kubectl apply -f "$PROJECT_ROOT/kubernetes/06-ci-runner.yaml"
        
        echo "Stack is deploying! Use 'kubectl get pods -A' to monitor."
        ;;

    --run-dev)
        echo "--- Preparing Development Source ---"
        if [ ! -d "$PROJECT_ROOT/source" ]; then
            mkdir -p "$PROJECT_ROOT/source"
            tar -xzf "$PROJECT_ROOT/step-source.tar.gz" -C "$PROJECT_ROOT/source"
        fi
        echo "Source extracted to $PROJECT_ROOT/source"
        echo ""
        echo "To start developing:"
        echo "1. Log in to Gitea at http://gitea.local"
        echo "2. Create an organization named 'actions'"
        echo "3. Push the cached actions to Gitea:"
        echo "   for repo in actions-repos/*.git; do"
        echo "     name=\$(basename \$repo .git)"
        echo "     # Create repo in Gitea via UI or API first, then:"
        echo "     cd \$repo && git push --mirror http://gitea.local/actions/\$name.git"
        echo "   done"
        echo "4. Create your 'step' repo and push source:"
        echo "   cd source && git remote add origin http://gitea.local/user/step.git"
        echo "   git push -u origin main"
        ;;

    *)
        show_help
        ;;
esac
