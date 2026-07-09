#!/bin/bash

# SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# This script configures .env and nginx for remote deployment.
# It generates configs from templates, generates secrets, and configures URLs.

# Check if arguments are provided
if [ -z "$1" ] || [ -z "$2" ]; then
  echo "Usage: ./configure-environment.sh <domain> <subdomain_suffix>"
  echo "  <domain>: The root domain (e.g., sequent.vote)"
  echo "  <subdomain_suffix>: The suffix for all subdomains (e.g., remote-deployment, qa, staging)"
  echo ""
  echo "Example: ./configure-environment.sh sequent.vote remote-deployment"
  echo "This will configure URLs like:"
  echo "  - admin-remote-deployment.sequent.vote"
  echo "  - voting-remote-deployment.sequent.vote"
  echo "  - hasura-remote-deployment.sequent.vote"
  echo "  - login-remote-deployment.sequent.vote"
  echo "  - minio-remote-deployment.sequent.vote"
  exit 1
fi

# Function to generate random hex string
generate_hex() {
  local length=$1
  openssl rand -hex $length
}

# Function to generate random alphanumeric string (URL-safe, no special chars)
generate_base64() {
  local length=$1
  # Generate random bytes and convert to alphanumeric only
  # Using /dev/urandom, filter to alphanumeric, take required length
  LC_ALL=C tr -dc 'a-zA-Z0-9' < /dev/urandom | head -c "$length"
}

# Set variables
DOMAIN=$1
SUBDOMAIN_SUFFIX=$2

# Set paths relative to script location
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
DEVCONTAINER_DIR="$( cd "$SCRIPT_DIR/.." && pwd )"
ENV_TEMPLATE="$DEVCONTAINER_DIR/.env.remote-deployment.example"
ENV_FILE="$DEVCONTAINER_DIR/.env"
NGINX_TEMPLATE="$DEVCONTAINER_DIR/nginx/default.conf.template"
NGINX_CONF="$DEVCONTAINER_DIR/nginx/default.conf"

# Check if templates exist
if [ ! -f "$ENV_TEMPLATE" ]; then
  echo "Error: .env template not found at $ENV_TEMPLATE"
  exit 1
fi

if [ ! -f "$NGINX_TEMPLATE" ]; then
  echo "Error: nginx template not found at $NGINX_TEMPLATE"
  exit 1
fi

echo "================================================"
echo "Sequent Step Remote Deployment Configuration"
echo "================================================"
echo ""

# Step 1: Copy templates
echo "[1/4] Copying templates..."
cp "$ENV_TEMPLATE" "$ENV_FILE"
cp "$NGINX_TEMPLATE" "$NGINX_CONF"
echo "  ✓ Created .env from template"
echo "  ✓ Created nginx/default.conf from template"
echo ""

# Step 2: Generate secrets
echo "[2/4] Generating secrets..."
MASTER_SECRET=$(generate_hex 32)

# Update secrets in .env (macOS compatible)
sed -i.bak "s|^SECRETS_BACKEND=.*|SECRETS_BACKEND=EnvVarMasterSecret|g" "$ENV_FILE" && rm "$ENV_FILE.bak"
sed -i.bak "s|^MASTER_SECRET=.*|MASTER_SECRET=$MASTER_SECRET|g" "$ENV_FILE" && rm "$ENV_FILE.bak"

KEYCLOAK_JSON_FILE="$DEVCONTAINER_DIR/minio/public-assets/defaults/keycloak/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json"

KEYCLOAK_CLIENT_SECRET=$(generate_base64 32)
KEYCLOAK_CLI_CLIENT_SECRET=$(generate_base64 32)
HASURA_GRAPHQL_ADMIN_SECRET=$(generate_base64 32)
AWS_S3_ROOT_PASSWORD=$(generate_base64 32)
KEYCLOAK_ADMIN_PASSWORD=$(generate_base64 32)

sed -i.bak "s|^KEYCLOAK_CLIENT_SECRET=.*|KEYCLOAK_CLIENT_SECRET=$KEYCLOAK_CLIENT_SECRET|g" "$ENV_FILE" && rm "$ENV_FILE.bak"
jq --arg secret "$KEYCLOAK_CLIENT_SECRET" '(.clients[] | select(.clientId == "service-account") | .secret) = $secret' "$KEYCLOAK_JSON_FILE" > "$KEYCLOAK_JSON_FILE.tmp" && mv "$KEYCLOAK_JSON_FILE.tmp" "$KEYCLOAK_JSON_FILE"

sed -i.bak "s|^KEYCLOAK_CLI_CLIENT_SECRET=.*|KEYCLOAK_CLI_CLIENT_SECRET=$KEYCLOAK_CLI_CLIENT_SECRET|g" "$ENV_FILE" && rm "$ENV_FILE.bak"
jq --arg secret "$KEYCLOAK_CLI_CLIENT_SECRET" '(.clients[] | select(.clientId == "cli-account-admin") | .secret) = $secret' "$KEYCLOAK_JSON_FILE" > "$KEYCLOAK_JSON_FILE.tmp" && mv "$KEYCLOAK_JSON_FILE.tmp" "$KEYCLOAK_JSON_FILE"

sed -i.bak "s|^HASURA_GRAPHQL_ADMIN_SECRET=.*|HASURA_GRAPHQL_ADMIN_SECRET=$HASURA_GRAPHQL_ADMIN_SECRET|g" "$ENV_FILE" && rm "$ENV_FILE.bak"
sed -i.bak "s|^AWS_S3_ROOT_PASSWORD=.*|AWS_S3_ROOT_PASSWORD=$AWS_S3_ROOT_PASSWORD|g" "$ENV_FILE" && rm "$ENV_FILE.bak"
# AWS_S3_ACCESS_SECRET has to be set to AWS_S3_ROOT_PASSWORD
sed -i.bak "s|^AWS_S3_ACCESS_SECRET=.*|AWS_S3_ACCESS_SECRET=$AWS_S3_ROOT_PASSWORD|g" "$ENV_FILE" && rm "$ENV_FILE.bak"
sed -i.bak "s|^KEYCLOAK_ADMIN_PASSWORD=.*|KEYCLOAK_ADMIN_PASSWORD=$KEYCLOAK_ADMIN_PASSWORD|g" "$ENV_FILE" && rm "$ENV_FILE.bak"
# KEYCLOAK_ADMIN_CLIENT_SECRET has to be set to KEYCLOAK_ADMIN_PASSWORD
sed -i.bak "s|^KEYCLOAK_ADMIN_CLIENT_SECRET=.*|KEYCLOAK_ADMIN_CLIENT_SECRET=$KEYCLOAK_ADMIN_PASSWORD|g" "$ENV_FILE" && rm "$ENV_FILE.bak"

echo "  ✓ Generated MASTER_SECRET (32 bytes hex)"
echo "  ✓ Generated Keycloak client secrets"
echo "  ✓ Set SECRETS_BACKEND=EnvVarMasterSecret"
echo ""

# Step 3: Configure domain and URLs
echo "[3/4] Configuring domain: $DOMAIN with subdomain: $SUBDOMAIN_SUFFIX..."

# Update the DOMAIN variable (macOS compatible)
sed -i.bak "s|^DOMAIN=.*|DOMAIN=$DOMAIN|g" "$ENV_FILE" && rm "$ENV_FILE.bak"

# Update all subdomain-based URLs to use the new suffix
sed -i.bak "s|login-[a-zA-Z0-9_-]*\.\${DOMAIN}|login-$SUBDOMAIN_SUFFIX.\${DOMAIN}|g" "$ENV_FILE" && rm "$ENV_FILE.bak"
sed -i.bak "s|admin-[a-zA-Z0-9_-]*\.\${DOMAIN}|admin-$SUBDOMAIN_SUFFIX.\${DOMAIN}|g" "$ENV_FILE" && rm "$ENV_FILE.bak"
sed -i.bak "s|voting-[a-zA-Z0-9_-]*\.\${DOMAIN}|voting-$SUBDOMAIN_SUFFIX.\${DOMAIN}|g" "$ENV_FILE" && rm "$ENV_FILE.bak"
sed -i.bak "s|hasura-[a-zA-Z0-9_-]*\.\${DOMAIN}|hasura-$SUBDOMAIN_SUFFIX.\${DOMAIN}|g" "$ENV_FILE" && rm "$ENV_FILE.bak"
sed -i.bak "s|minio-[a-zA-Z0-9_-]*\.\${DOMAIN}|minio-$SUBDOMAIN_SUFFIX.\${DOMAIN}|g" "$ENV_FILE" && rm "$ENV_FILE.bak"

# Update hostname variables for webpack dev server
sed -i.bak "s|^VOTING_PORTAL_HOSTNAME=.*|VOTING_PORTAL_HOSTNAME=voting-$SUBDOMAIN_SUFFIX.\${DOMAIN}|g" "$ENV_FILE" && rm "$ENV_FILE.bak"
sed -i.bak "s|^ADMIN_PORTAL_HOSTNAME=.*|ADMIN_PORTAL_HOSTNAME=admin-$SUBDOMAIN_SUFFIX.\${DOMAIN}|g" "$ENV_FILE" && rm "$ENV_FILE.bak"

# Step 4: Configure nginx
echo "[4/4] Configuring nginx reverse proxy..."

# Update nginx config with domain and subdomain suffix (macOS compatible)
sed -i.bak "s|server_name admin-[a-zA-Z0-9_-]*\.\${DOMAIN};|server_name admin-$SUBDOMAIN_SUFFIX.$DOMAIN;|g" "$NGINX_CONF" && rm "$NGINX_CONF.bak"
sed -i.bak "s|server_name voting-[a-zA-Z0-9_-]*\.\${DOMAIN};|server_name voting-$SUBDOMAIN_SUFFIX.$DOMAIN;|g" "$NGINX_CONF" && rm "$NGINX_CONF.bak"
sed -i.bak "s|server_name hasura-[a-zA-Z0-9_-]*\.\${DOMAIN};|server_name hasura-$SUBDOMAIN_SUFFIX.$DOMAIN;|g" "$NGINX_CONF" && rm "$NGINX_CONF.bak"
sed -i.bak "s|server_name login-[a-zA-Z0-9_-]*\.\${DOMAIN};|server_name login-$SUBDOMAIN_SUFFIX.$DOMAIN;|g" "$NGINX_CONF" && rm "$NGINX_CONF.bak"
sed -i.bak "s|server_name minio-[a-zA-Z0-9_-]*\.\${DOMAIN};|server_name minio-$SUBDOMAIN_SUFFIX.$DOMAIN;|g" "$NGINX_CONF" && rm "$NGINX_CONF.bak"
sed -i.bak "s|server_name verifier-[a-zA-Z0-9_-]*\.\${DOMAIN};|server_name verifier-$SUBDOMAIN_SUFFIX.$DOMAIN;|g" "$NGINX_CONF" && rm "$NGINX_CONF.bak"

echo ""
echo "================================================"
echo "✓ Configuration completed successfully!"
echo "================================================"
echo ""
echo "Domain:    $DOMAIN"
echo "Subdomain: $SUBDOMAIN_SUFFIX"
echo ""
echo "Generated files:"
echo "  ✓ $ENV_FILE"
echo "  ✓ $NGINX_CONF"
echo ""
echo "Service URLs:"
echo "  Keycloak:      https://login-$SUBDOMAIN_SUFFIX.$DOMAIN"
echo "  Admin Portal:  https://admin-$SUBDOMAIN_SUFFIX.$DOMAIN"
echo "  Voting Portal: https://voting-$SUBDOMAIN_SUFFIX.$DOMAIN"
echo "  Hasura:        https://hasura-$SUBDOMAIN_SUFFIX.$DOMAIN"
echo "  MinIO:         https://minio-$SUBDOMAIN_SUFFIX.$DOMAIN"
echo ""
echo "Next steps:"
echo "  1. Set up DNS: ./.devcontainer/remote-deployment/setup-cloudflare.sh $DOMAIN <server_ip> $SUBDOMAIN_SUFFIX"
echo "  2. Deploy:     cd ~/.devcontainer && docker compose -f docker-compose-remote.yml up -d --build"
echo ""
