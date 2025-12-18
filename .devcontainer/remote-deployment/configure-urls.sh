#!/bin/bash

# This script configures the .env file for remote deployment with proper domain and subdomain settings.

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

# Set variables
DOMAIN=$1
SUBDOMAIN_SUFFIX=$2

# Set the path to the .env file
ENV_FILE="$HOME/step/.devcontainer/.env"
NGINX_CONF="$HOME/step/.devcontainer/nginx/default.conf"

# Check if the .env file exists
if [ ! -f "$ENV_FILE" ]; then
  echo "Error: .env file not found at $ENV_FILE."
  echo "Please copy .env.remote-test.example to .env first:"
  echo "  cp $HOME/step/.devcontainer/.env.remote-test.example $HOME/step/.devcontainer/.env"
  exit 1
fi

# Check if nginx config exists
if [ ! -f "$NGINX_CONF" ]; then
  echo "Error: nginx config not found at $NGINX_CONF."
  exit 1
fi

echo "Configuring .env file for domain: $DOMAIN with subdomain suffix: $SUBDOMAIN_SUFFIX"

# Update the DOMAIN variable
sed -i "s|^DOMAIN=.*|DOMAIN=$DOMAIN|g" "$ENV_FILE"

# Update all subdomain-based URLs to use the new suffix
sed -i "s|login-[a-zA-Z0-9_-]*\.\${DOMAIN}|login-$SUBDOMAIN_SUFFIX.\${DOMAIN}|g" "$ENV_FILE"
sed -i "s|admin-[a-zA-Z0-9_-]*\.\${DOMAIN}|admin-$SUBDOMAIN_SUFFIX.\${DOMAIN}|g" "$ENV_FILE"
sed -i "s|voting-[a-zA-Z0-9_-]*\.\${DOMAIN}|voting-$SUBDOMAIN_SUFFIX.\${DOMAIN}|g" "$ENV_FILE"
sed -i "s|hasura-[a-zA-Z0-9_-]*\.\${DOMAIN}|hasura-$SUBDOMAIN_SUFFIX.\${DOMAIN}|g" "$ENV_FILE"
sed -i "s|minio-[a-zA-Z0-9_-]*\.\${DOMAIN}|minio-$SUBDOMAIN_SUFFIX.\${DOMAIN}|g" "$ENV_FILE"
sed -i "s|^HARVEST_DOMAIN=.*|HARVEST_DOMAIN=$SUBDOMAIN_SUFFIX.\${DOMAIN}|g" "$ENV_FILE"

echo "Configuring nginx reverse proxy..."

# Update nginx config with domain and subdomain suffix
sed -i "s|server_name admin-[a-zA-Z0-9_-]*\.\${DOMAIN};|server_name admin-$SUBDOMAIN_SUFFIX.$DOMAIN;|g" "$NGINX_CONF"
sed -i "s|server_name voting-[a-zA-Z0-9_-]*\.\${DOMAIN};|server_name voting-$SUBDOMAIN_SUFFIX.$DOMAIN;|g" "$NGINX_CONF"
sed -i "s|server_name hasura-[a-zA-Z0-9_-]*\.\${DOMAIN};|server_name hasura-$SUBDOMAIN_SUFFIX.$DOMAIN;|g" "$NGINX_CONF"
sed -i "s|server_name login-[a-zA-Z0-9_-]*\.\${DOMAIN};|server_name login-$SUBDOMAIN_SUFFIX.$DOMAIN;|g" "$NGINX_CONF"
sed -i "s|server_name minio-[a-zA-Z0-9_-]*\.\${DOMAIN};|server_name minio-$SUBDOMAIN_SUFFIX.$DOMAIN;|g" "$NGINX_CONF"
sed -i "s|server_name verifier-[a-zA-Z0-9_-]*\.\${DOMAIN};|server_name verifier-$SUBDOMAIN_SUFFIX.$DOMAIN;|g" "$NGINX_CONF"

echo ""
echo "✓ .env file configured successfully!"
echo "✓ nginx config configured successfully!"
echo ""
echo "Configuration:"
echo "  Domain: $DOMAIN"
echo "  Subdomain suffix: $SUBDOMAIN_SUFFIX"
echo ""
echo "Configured URLs:"
echo "  - Keycloak:      https://login-$SUBDOMAIN_SUFFIX.$DOMAIN"
echo "  - Admin Portal:  https://admin-$SUBDOMAIN_SUFFIX.$DOMAIN"
echo "  - Voting Portal: https://voting-$SUBDOMAIN_SUFFIX.$DOMAIN"
echo "  - Hasura:        https://hasura-$SUBDOMAIN_SUFFIX.$DOMAIN"
echo "  - MinIO:         https://minio-$SUBDOMAIN_SUFFIX.$DOMAIN"
echo ""
echo "Next steps:"
echo "  1. Review and update any remaining placeholders in $ENV_FILE"
echo "  2. Set up DNS records with: ./.devcontainer/remote-deployment/setup-cloudflare.sh $DOMAIN <server_ip> $SUBDOMAIN_SUFFIX"
echo "  3. Start the services with: docker-compose -f .devcontainer/docker-compose-remote.yml up -d --build"
