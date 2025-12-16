#!/bin/bash

# This script automates the creation of DNS records in Cloudflare.

# Check for required arguments
if [ -z "$1" ] || [ -z "$2" ] || [ -z "$3" ]; then
  echo "Usage: ./setup-cloudflare.sh <domain> <cname_or_ip> <ports>"
  echo "  <domain>: The domain name to create records for (e.g., example.com)"
  echo "  <cname_or_ip>: The CNAME or IP address of the server"
  echo "  <ports>: A comma-separated list of ports to create records for (e.g., 80,443,8080)"
  exit 1
fi

# Set variables
DOMAIN=$1
CNAME_OR_IP=$2
PORTS=$3

# Check if Cloudflare API token is set
if [ -z "$CLOUDFLARE_API_TOKEN" ]; then
  echo "Error: CLOUDFLARE_API_TOKEN environment variable is not set."
  exit 1
fi

# Get the Zone ID for the domain
ZONE_ID=$(curl -s -X GET "https://api.cloudflare.com/client/v4/zones?name=$DOMAIN" \
     -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
     -H "Content-Type: application/json" | jq -r ".result[0].id")

# Check if Zone ID was found
if [ -z "$ZONE_ID" ]; then
  echo "Error: Could not find Zone ID for domain $DOMAIN. Please make sure the domain is added to your Cloudflare account."
  exit 1
fi

# Loop through the ports and create DNS records
IFS=',' read -ra PORT_ARRAY <<< "$PORTS"
for port in "${PORT_ARRAY[@]}"; do
  # Create a subdomain for each port
  SUBDOMAIN="port$port.$DOMAIN"

  # Create the DNS record
  curl -s -X POST "https://api.cloudflare.com/client/v4/zones/$ZONE_ID/dns_records" \
       -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
       -H "Content-Type: application/json" \
       --data '{"type":"A","name":"'$SUBDOMAIN'","content":"'$CNAME_OR_IP'","ttl":120,"proxied":false}' | jq
done

echo "Cloudflare DNS records created successfully."
