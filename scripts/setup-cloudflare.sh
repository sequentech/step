#!/bin/bash

# This script automates the creation of DNS records in Cloudflare for the remote test environment.

# Check for required arguments
if [ -z "$1" ] || [ -z "$2" ]; then
  echo "Usage: ./setup-cloudflare.sh <zone_domain> <target_ip_or_cname>"
  echo "  <zone_domain>: The root domain managed by Cloudflare (e.g., sequent.vote)"
  echo "  <target_ip_or_cname>: The IP address (for A record) or hostname (for CNAME record) to point to"
  exit 1
fi

# Set variables
ZONE_DOMAIN=$1
TARGET=$2

# Check if Cloudflare API token is set
if [ -z "$CLOUDFLARE_API_TOKEN" ]; then
  echo "Error: CLOUDFLARE_API_TOKEN environment variable is not set."
  exit 1
fi

# Get the Zone ID for the domain
ZONE_ID=$(curl -s -X GET "https://api.cloudflare.com/client/v4/zones?name=$ZONE_DOMAIN" \
     -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
     -H "Content-Type: application/json" | jq -r ".result[0].id")

# Check if Zone ID was found
if [ -z "$ZONE_ID" ] || [ "$ZONE_ID" == "null" ]; then
  echo "Error: Could not find Zone ID for domain $ZONE_DOMAIN. Please make sure the domain is added to your Cloudflare account."
  exit 1
fi

echo "Found Zone ID: $ZONE_ID for zone $ZONE_DOMAIN"

# Create a main A/CNAME record for the remote-test subdomain
RECORD_TYPE="A"
if ! [[ $TARGET =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  RECORD_TYPE="CNAME"
fi

echo "Creating a $RECORD_TYPE record for remote-test.$ZONE_DOMAIN pointing to $TARGET..."
RESPONSE=$(curl -s -X POST "https://api.cloudflare.com/client/v4/zones/$ZONE_ID/dns_records" \
     -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
     -H "Content-Type: application/json" \
     --data "{\"type\":\"$RECORD_TYPE\",\"name\":\"remote-test\",\"content\":\"$TARGET\",\"ttl\":120,\"proxied\":true}")

SUCCESS=$(echo $RESPONSE | jq -r .success)
if [ "$SUCCESS" == "true" ]; then
    echo "Successfully created DNS record for remote-test.$ZONE_DOMAIN"
else
    echo "Error creating DNS record for remote-test.$ZONE_DOMAIN:"
    echo $RESPONSE | jq .errors
fi

# List of services to create CNAME records for
SERVICES=("admin" "voting" "hasura" "login" "minio")

for SERVICE in "${SERVICES[@]}"; do
  SUBDOMAIN="$SERVICE-remote-test"
  echo "Creating a CNAME record for $SUBDOMAIN.$ZONE_DOMAIN..."
  RESPONSE=$(curl -s -X POST "https://api.cloudflare.com/client/v4/zones/$ZONE_ID/dns_records" \
       -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
       -H "Content-Type: application/json" \
       --data "{\"type\":\"CNAME\",\"name\":\"$SUBDOMAIN\",\"content\":\"remote-test.$ZONE_DOMAIN\",\"ttl\":120,\"proxied\":true}")

  SUCCESS=$(echo $RESPONSE | jq -r .success)
  if [ "$SUCCESS" == "true" ]; then
      echo "Successfully created CNAME record for $SUBDOMAIN.$ZONE_DOMAIN"
  else
      echo "Error creating CNAME record for $SUBDOMAIN.$ZONE_DOMAIN:"
      echo $RESPONSE | jq .errors
  fi
done
