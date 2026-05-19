#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -e

SCRIPT_PATH="$(cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P)"
PROJECT_ROOT="$SCRIPT_PATH" # Management script runs from the transfer folder

show_help() {
    echo "Sequent Airgap Management Tool"
    echo ""
    echo "Usage: ./manage.sh [command]"
    echo ""
    echo "Commands:"
    echo "  --setup          Install Docker/Git packages and load all images."
    echo "  --run-dev        Extract source code and start the development stack."
    echo "  --run-server     Start the production server stack and initialize storage."
    echo "  --release        (Dev Only) Build and package production images from current source."
    echo "  --help           Show this help message."
    echo ""
}

# Internal function for JWKS sync (integrated from server-init.sh)
init_server_storage() {
    echo "--- Initializing Server Storage (JWKS Sync) ---"
    
    # Wait for Keycloak to be ready
    echo "Waiting for Keycloak to be healthy..."
    # We use 'docker exec' into one of our running containers or just curl from host if available
    # Since we are on the host, we'll try to use a temporary tooling container to reach the network
    docker run --rm --network service:postgres step-airgap-dev bash -c '
        until curl -s http://keycloak:8090/health/live | grep -q "UP"; do
            sleep 5
            echo -n "."
        done
        echo " Keycloak is UP!"
        
        echo "Fetching JWKS..."
        curl -s http://keycloak:8090/realms/master/protocol/openid-connect/certs > /tmp/certs.json
        
        echo "Uploading to RustFS..."
        mc alias set myminio http://rustfs:9000 ${AWS_S3_ROOT_USER:-admin} ${AWS_S3_ROOT_PASSWORD:-password}
        mc cp /tmp/certs.json myminio/public/certs.json
    '
    echo "Storage Initialization Complete!"
}

case "$1" in
    --setup)
        echo "--- Installing OS Packages (Docker/Git) ---"
        if [ -d "$PROJECT_ROOT/deb-packages" ]; then
            cd "$PROJECT_ROOT/deb-packages"
            sudo dpkg -i *.deb
            echo "OS Setup Complete!"
        else
            echo "Error: deb-packages folder not found."
            exit 1
        fi

        echo "--- Loading Docker Images ---"
        if [ -f "$PROJECT_ROOT/step-airgap-all-images.tar" ]; then
            docker load -i "$PROJECT_ROOT/step-airgap-all-images.tar"
            echo "Images Loaded!"
        else
            echo "Error: image tarball not found."
            exit 1
        fi
        ;;

    --run-dev)
        echo "--- Starting Development Stack ---"
        if [ ! -f "$PROJECT_ROOT/step-source.tar.gz" ]; then
            echo "Error: step-source.tar.gz not found."
            exit 1
        fi

        if [ ! -d "$PROJECT_ROOT/source" ]; then
            mkdir -p "$PROJECT_ROOT/source"
            echo "Extracting source code..."
            tar -xzf "$PROJECT_ROOT/step-source.tar.gz" -C "$PROJECT_ROOT/source"
        fi

        cd "$PROJECT_ROOT/source"
        if [ ! -f ".env" ]; then
            echo "Configuring default .env..."
            cp .devcontainer/.env.development .env
        fi

        docker compose -f docker-compose.dev.yml up -d
        echo "Development stack is UP! Access portal at http://localhost:3000"
        ;;

    --run-server)
        echo "--- Starting Server Stack ---"
        if [ ! -f "$PROJECT_ROOT/.env" ]; then
            echo "Error: .env file not found. Please create one from the template."
            exit 1
        fi
        
        docker compose -f docker-compose.server.yml up -d
        
        # Run integrated initialization
        init_server_storage
        
        echo "Server stack is UP and Initialized!"
        ;;

    --release)
        echo "--- Building Local Release (Dev -> Server) ---"
        if [ ! -d "$PROJECT_ROOT/source" ]; then
            echo "Error: This command must be run on a Dev machine with extracted source."
            exit 1
        fi

        cd "$PROJECT_ROOT/source"
        PACKAGES_DIR="./packages"
        
        echo "Rebuilding images from current offline source..."
        docker build -t sequentech.local/harvest -f "$PACKAGES_DIR/harvest/Dockerfile" "$PACKAGES_DIR"
        docker build -t sequentech.local/windmill -f "$PACKAGES_DIR/windmill/Dockerfile" "$PACKAGES_DIR"
        docker build -t sequentech.local/b3 -f "$PACKAGES_DIR/b3/Dockerfile.prod" "$PACKAGES_DIR"
        
        echo "Saving update tarball..."
        docker save -o "$PROJECT_ROOT/step-airgap-updates.tar" \
            sequentech.local/harvest \
            sequentech.local/windmill \
            sequentech.local/b3

        echo "--- Done! ---"
        echo "Transfer '$PROJECT_ROOT/step-airgap-updates.tar' to the Server machine."
        echo "Then on Server run: docker load -i step-airgap-updates.tar && ./manage.sh --run-server"
        ;;

    *)
        show_help
        ;;
esac
