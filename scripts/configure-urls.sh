#!/bin/bash

# This script configures the .env file with the server's public IP or domain name.

# Check if an argument is provided
if [ -z "$1" ]; then
  echo "Usage: ./configure-urls.sh <public_ip_or_domain>"
  exit 1
fi

# Set the public IP or domain name
PUBLIC_IP_OR_DOMAIN=$1

# Set the path to the .env file
ENV_FILE="$HOME/step/.devcontainer/.env"

# Check if the .env file exists
if [ ! -f "$ENV_FILE" ]; then
  echo "Error: .env file not found at $ENV_FILE. Please copy .env.example to .env first."
  exit 1
fi

# Replace localhost with the public IP or domain name
sed -i "s/localhost/$PUBLIC_IP_OR_DOMAIN/g" "$ENV_FILE"

echo ".env file configured successfully."
