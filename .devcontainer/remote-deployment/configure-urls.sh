#!/bin/bash

# This script configures .env and nginx for remote deployment.
# It generates configs from templates, generates secrets, and configures URLs.

# Check if arguments are provided
if [ -z "$1" ] || [ -z "$2" ]; then
  echo "Usage: ./configure-urls.sh <domain> <subdomain_suffix>"
  echo "  <domain>: The root domain (e.g., sequent.vote)"
  echo "  <subdomain_suffix>: The suffix for all subdomains (e.g., remote-test, qa, staging)"
  echo ""
  echo "Example: ./configure-urls.sh sequent.vote remote-test"
  echo "This will configure URLs like:"
  echo "  - admin-remote-test.sequent.vote"
  echo "  - voting-remote-test.sequent.vote"
  echo "  - hasura-remote-test.sequent.vote"
  echo "  - login-remote-test.sequent.vote"
  echo "  - minio-remote-test.sequent.vote"
  exit 1
fi

# Function to generate random hex string
generate_hex() {
  local length=$1
  openssl rand -hex $length
}

# Function to generate random base64 string
generate_base64() {
  local bytes=$1
  openssl rand -base64 $bytes | tr -d '\n'
}

# Set variables
DOMAIN=$1
SUBDOMAIN_SUFFIX=$2

# Set paths
DEVCONTAINER_DIR="$HOME/step/.devcontainer"
ENV_TEMPLATE="$DEVCONTAINER_DIR/.env.remote-test.example"
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
KEYCLOAK_CLIENT_SECRET=$(generate_base64 32)
KEYCLOAK_ADMIN_CLIENT_SECRET=$(generate_base64 32)
KEYCLOAK_CLI_CLIENT_SECRET=$(generate_base64 32)

# Update secrets in .env
sed -i "s|^SECRETS_BACKEND=.*|SECRETS_BACKEND=EnvVarMasterSecret|g" "$ENV_FILE"
sed -i "s|^MASTER_SECRET=.*|MASTER_SECRET=$MASTER_SECRET|g" "$ENV_FILE"
sed -i "s|^KEYCLOAK_CLIENT_SECRET=.*|KEYCLOAK_CLIENT_SECRET=$KEYCLOAK_CLIENT_SECRET|g" "$ENV_FILE"
sed -i "s|^KEYCLOAK_ADMIN_CLIENT_SECRET=.*|KEYCLOAK_ADMIN_CLIENT_SECRET=$KEYCLOAK_ADMIN_CLIENT_SECRET|g" "$ENV_FILE"
sed -i "s|^KEYCLOAK_CLI_CLIENT_SECRET=.*|KEYCLOAK_CLI_CLIENT_SECRET=$KEYCLOAK_CLI_CLIENT_SECRET|g" "$ENV_FILE"
sed -i "s|^ACTIONS_ADMIN_SECRET=.*|ACTIONS_ADMIN_SECRET=$KEYCLOAK_ADMIN_CLIENT_SECRET|g" "$ENV_FILE"
echo "  ✓ Generated MASTER_SECRET (32 bytes hex)"
echo "  ✓ Generated Keycloak client secrets"
echo "  ✓ Set SECRETS_BACKEND=EnvVarMasterSecret"
echo ""

# Step 3: Configure domain and URLs
echo "[3/4] Configuring domain: $DOMAIN with subdomain: $SUBDOMAIN_SUFFIX..."

# Update the DOMAIN variable
sed -i "s|^DOMAIN=.*|DOMAIN=$DOMAIN|g" "$ENV_FILE"

# Update all subdomain-based URLs to use the new suffix
sed -i "s|login-[a-zA-Z0-9_-]*\.\${DOMAIN}|login-$SUBDOMAIN_SUFFIX.\${DOMAIN}|g" "$ENV_FILE"
sed -i "s|admin-[a-zA-Z0-9_-]*\.\${DOMAIN}|admin-$SUBDOMAIN_SUFFIX.\${DOMAIN}|g" "$ENV_FILE"
sed -i "s|voting-[a-zA-Z0-9_-]*\.\${DOMAIN}|voting-$SUBDOMAIN_SUFFIX.\${DOMAIN}|g" "$ENV_FILE"
sed -i "s|hasura-[a-zA-Z0-9_-]*\.\${DOMAIN}|hasura-$SUBDOMAIN_SUFFIX.\${DOMAIN}|g" "$ENV_FILE"
sed -i "s|minio-[a-zA-Z0-9_-]*\.\${DOMAIN}|minio-$SUBDOMAIN_SUFFIX.\${DOMAIN}|g" "$ENV_FILE"
sed -i "s|^HARVEST_DOMAIN=.*|HARVEST_DOMAIN=$SUBDOMAIN_SUFFIX.\${DOMAIN}|g" "$ENV_FILE"

# Update hostname variables for webpack dev server
sed -i "s|^VOTING_PORTAL_HOSTNAME=.*|VOTING_PORTAL_HOSTNAME=voting-$SUBDOMAIN_SUFFIX.\${DOMAIN}|g" "$ENV_FILE"
sed -i "s|^ADMIN_PORTAL_HOSTNAME=.*|ADMIN_PORTAL_HOSTNAME=admin-$SUBDOMAIN_SUFFIX.\${DOMAIN}|g" "$ENV_FILE"

# Step 4: Configure nginx
echo "[4/4] Configuring nginx reverse proxy..."

# Update nginx config with domain and subdomain suffix
sed -i "s|server_name admin-[a-zA-Z0-9_-]*\.\${DOMAIN};|server_name admin-$SUBDOMAIN_SUFFIX.$DOMAIN;|g" "$NGINX_CONF"
sed -i "s|server_name voting-[a-zA-Z0-9_-]*\.\${DOMAIN};|server_name voting-$SUBDOMAIN_SUFFIX.$DOMAIN;|g" "$NGINX_CONF"
sed -i "s|server_name hasura-[a-zA-Z0-9_-]*\.\${DOMAIN};|server_name hasura-$SUBDOMAIN_SUFFIX.$DOMAIN;|g" "$NGINX_CONF"
sed -i "s|server_name login-[a-zA-Z0-9_-]*\.\${DOMAIN};|server_name login-$SUBDOMAIN_SUFFIX.$DOMAIN;|g" "$NGINX_CONF"
sed -i "s|server_name minio-[a-zA-Z0-9_-]*\.\${DOMAIN};|server_name minio-$SUBDOMAIN_SUFFIX.$DOMAIN;|g" "$NGINX_CONF"
sed -i "s|server_name verifier-[a-zA-Z0-9_-]*\.\${DOMAIN};|server_name verifier-$SUBDOMAIN_SUFFIX.$DOMAIN;|g" "$NGINX_CONF"

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
echo "  2. Deploy:     cd ~/.devcontainer && docker-compose -f docker-compose-remote.yml up -d --build"
echo ""
