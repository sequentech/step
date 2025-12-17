#!/bin/bash

# This script automates the creation of DNS records in Cloudflare for the remote test environment.

# Check for required arguments
if [ -z "$1" ] || [ -z "$2" ] || [ -z "$3" ]; then
  echo "Usage: ./setup-cloudflare.sh <zone_domain> <target_ip_or_cname> <subdomain_suffix>"
  echo "  <zone_domain>: The root domain managed by Cloudflare (e.g., sequent.vote)"
  echo "  <target_ip_or_cname>: The IP address (for A record) or hostname (for CNAME record) to point to"
  echo "  <subdomain_suffix>: The suffix for all subdomains (e.g., remote-test, qa, staging)"
  echo ""
  echo "Example: ./setup-cloudflare.sh sequent.vote 54.123.45.67 remote-test"
  echo "This will create:"
  echo "  - remote-test.sequent.vote -> 54.123.45.67"
  echo "  - admin-remote-test.sequent.vote -> remote-test.sequent.vote"
  echo "  - voting-remote-test.sequent.vote -> remote-test.sequent.vote"
  echo "  - hasura-remote-test.sequent.vote -> remote-test.sequent.vote"
  echo "  - login-remote-test.sequent.vote -> remote-test.sequent.vote"
  echo "  - minio-remote-test.sequent.vote -> remote-test.sequent.vote"
  exit 1
fi

# Set variables
ZONE_DOMAIN=$1
TARGET=$2
SUBDOMAIN_SUFFIX=$3

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
echo "Subdomain suffix: $SUBDOMAIN_SUFFIX"

# Create a main A/CNAME record for the base subdomain
RECORD_TYPE="A"
if ! [[ $TARGET =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  RECORD_TYPE="CNAME"
fi

echo "Creating a $RECORD_TYPE record for $SUBDOMAIN_SUFFIX.$ZONE_DOMAIN pointing to $TARGET..."
RESPONSE=$(curl -s -X POST "https://api.cloudflare.com/client/v4/zones/$ZONE_ID/dns_records" \
     -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
     -H "Content-Type: application/json" \
     --data "{\"type\":\"$RECORD_TYPE\",\"name\":\"$SUBDOMAIN_SUFFIX\",\"content\":\"$TARGET\",\"ttl\":120,\"proxied\":true}")

SUCCESS=$(echo $RESPONSE | jq -r .success)
if [ "$SUCCESS" == "true" ]; then
    echo "Successfully created DNS record for $SUBDOMAIN_SUFFIX.$ZONE_DOMAIN"
else
    echo "Error creating DNS record for $SUBDOMAIN_SUFFIX.$ZONE_DOMAIN:"
    echo $RESPONSE | jq .errors
    exit 1
fi

# List of services to create CNAME records for
SERVICES=("admin" "voting" "hasura" "login" "minio")

for SERVICE in "${SERVICES[@]}"; do
  SUBDOMAIN="$SERVICE-$SUBDOMAIN_SUFFIX"
  echo "Creating a CNAME record for $SUBDOMAIN.$ZONE_DOMAIN..."
  RESPONSE=$(curl -s -X POST "https://api.cloudflare.com/client/v4/zones/$ZONE_ID/dns_records" \
       -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
       -H "Content-Type: application/json" \
       --data "{\"type\":\"CNAME\",\"name\":\"$SUBDOMAIN\",\"content\":\"$SUBDOMAIN_SUFFIX.$ZONE_DOMAIN\",\"ttl\":120,\"proxied\":true}")

  SUCCESS=$(echo $RESPONSE | jq -r .success)
  if [ "$SUCCESS" == "true" ]; then
      echo "Successfully created CNAME record for $SUBDOMAIN.$ZONE_DOMAIN"
  else
      echo "Error creating CNAME record for $SUBDOMAIN.$ZONE_DOMAIN:"
      echo $RESPONSE | jq .errors
  fi
done
