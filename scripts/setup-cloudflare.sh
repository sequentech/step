#!/bin/bash

# This script automates the creation of a DNS record in Cloudflare.
# It can create either an A record or a CNAME record based on the target.

# Check for required arguments
if [ -z "$1" ] || [ -z "$2" ] || [ -z "$3" ]; then
  echo "Usage: ./setup-cloudflare.sh <zone_domain> <full_domain> <target_ip_or_cname>"
  echo "  <zone_domain>: The root domain managed by Cloudflare (e.g., sequent.vote)"
  echo "  <full_domain>: The full subdomain to create (e.g., remote-test.sequent.vote)"
  echo "  <target_ip_or_cname>: The IP address (for A record) or hostname (for CNAME record) to point to"
  exit 1
fi

# Set variables
ZONE_DOMAIN=$1
FULL_DOMAIN=$2
TARGET=$3
RECORD_TYPE="A"

# Simple check to see if target is an IP address. If not, assume it's a CNAME.
if ! [[ $TARGET =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  RECORD_TYPE="CNAME"
fi

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
echo "Creating a $RECORD_TYPE record for $FULL_DOMAIN pointing to $TARGET..."

# Create the DNS record
RESPONSE=$(curl -s -X POST "https://api.cloudflare.com/client/v4/zones/$ZONE_ID/dns_records" \
     -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
     -H "Content-Type: application/json" \
     --data "{\"type\":\"$RECORD_TYPE\",\"name\":\"$FULL_DOMAIN\",\"content\":\"$TARGET\",\"ttl\":120,\"proxied\":false}")

# Check if the record was created successfully
SUCCESS=$(echo $RESPONSE | jq -r .success)
if [ "$SUCCESS" == "true" ]; then
    echo "Successfully created DNS record:"
    echo $RESPONSE | jq
else
    echo "Error creating DNS record:"
    echo $RESPONSE | jq .errors
    exit 1
fi
