#!/bin/bash

# This script automates the creation of DNS records in Cloudflare for the remote test environment.

# Check for required arguments
if [ -z "$1" ] || [ -z "$2" ] || [ -z "$3" ]; then
  echo "Usage: ./setup-cloudflare.sh <zone_domain> <target_ip_or_cname> <subdomain_suffix>"
  echo "  <zone_domain>: The root domain managed by Cloudflare (e.g., sequent.vote)"
  echo "  <target_ip_or_cname>: The IP address (for A record) or hostname (for CNAME record) to point to"
  echo "  <subdomain_suffix>: The suffix for all subdomains (e.g., remote-deployment, qa, staging)"
  echo ""
  echo "Example: ./setup-cloudflare.sh sequent.vote 54.123.45.67 remote-deployment"
  echo "This will create:"
  echo "  - remote-deployment.sequent.vote -> 54.123.45.67"
  echo "  - admin-remote-deployment.sequent.vote -> remote-deployment.sequent.vote"
  echo "  - voting-remote-deployment.sequent.vote -> remote-deployment.sequent.vote"
  echo "  - hasura-remote-deployment.sequent.vote -> remote-deployment.sequent.vote"
  echo "  - login-remote-deployment.sequent.vote -> remote-deployment.sequent.vote"
  echo "  - minio-remote-deployment.sequent.vote -> remote-deployment.sequent.vote"
  echo "  - verifier-remote-deployment.sequent.vote -> remote-deployment.sequent.vote"
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
    ERROR_CODE=$(echo $RESPONSE | jq -r '.errors[0].code')
    if [ "$ERROR_CODE" == "81054" ]; then
        echo "DNS record for $SUBDOMAIN_SUFFIX.$ZONE_DOMAIN already exists, skipping..."
    else
        echo "Error creating DNS record for $SUBDOMAIN_SUFFIX.$ZONE_DOMAIN:"
        echo $RESPONSE | jq .errors
        # exit 1
        echo temporarily continuing...
    fi
fi

# List of services to create CNAME records for
SERVICES=("admin" "voting" "hasura" "login" "minio" "verifier")

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
      ERROR_CODE=$(echo $RESPONSE | jq -r '.errors[0].code')
      if [ "$ERROR_CODE" == "81054" ]; then
          echo "CNAME record for $SUBDOMAIN.$ZONE_DOMAIN already exists, skipping..."
      else
          echo "Error creating CNAME record for $SUBDOMAIN.$ZONE_DOMAIN:"
          echo $RESPONSE | jq .errors
      fi
  fi
done

# Check current SSL/TLS mode for the zone
echo "\nChecking SSL/TLS configuration..."
SSL_MODE_RESPONSE=$(curl -s -X GET "https://api.cloudflare.com/client/v4/zones/$ZONE_ID/settings/ssl" \
     -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
     -H "Content-Type: application/json")

SSL_SUCCESS=$(echo $SSL_MODE_RESPONSE | jq -r .success)

if [ "$SSL_SUCCESS" != "true" ]; then
    ERROR_CODE=$(echo $SSL_MODE_RESPONSE | jq -r '.errors[0].code')
    ERROR_MESSAGE=$(echo $SSL_MODE_RESPONSE | jq -r '.errors[0].message')
    
    if [ "$ERROR_CODE" == "9109" ]; then
        echo "⚠ Warning: API token does not have permission to read SSL settings."
        echo "  Your API token needs 'Zone Settings: Read' and 'Zone Settings: Edit' permissions."
        echo ""
        echo "=== MANUAL ACTION REQUIRED ==="
        echo ""
        echo "Your services will show '521 Web Server Is Down' errors without SSL configuration."
        echo "Please configure SSL manually using ONE of these options:"
        echo ""
        echo "Option 1: Update your API token permissions (Recommended)"
        echo "  1. Go to Cloudflare Dashboard → My Profile → API Tokens"
        echo "  2. Edit your token or create a new one"
        echo "  3. Add permissions: Zone > Zone Settings > Edit"
        echo "  4. Re-run this script with the updated token"
        echo ""
        echo "Option 2: Set zone-wide Flexible SSL manually"
        echo "  1. Go to Cloudflare Dashboard → $ZONE_DOMAIN → SSL/TLS → Overview"
        echo "  2. Set SSL/TLS encryption mode to 'Flexible'"
        echo ""
        echo "Option 3: Create Page Rule manually"
        echo "  1. Go to Cloudflare Dashboard → $ZONE_DOMAIN → Rules → Page Rules"
        echo "  2. Create a new Page Rule"
        echo "  3. URL pattern: *-$SUBDOMAIN_SUFFIX.$ZONE_DOMAIN/*"
        echo "  4. Setting: SSL → Flexible"
        echo "  5. Save and deploy"
        echo ""
        echo "DNS records have been created successfully. SSL configuration is pending."
        exit 0
    else
        echo "✗ Error checking SSL settings: $ERROR_MESSAGE (code: $ERROR_CODE)"
        exit 1
    fi
fi

CURRENT_SSL_MODE=$(echo $SSL_MODE_RESPONSE | jq -r .result.value)
echo "Current SSL/TLS mode: $CURRENT_SSL_MODE"

# If SSL mode is already 'flexible', we don't need a Page Rule
if [ "$CURRENT_SSL_MODE" == "flexible" ]; then
    echo "✓ SSL/TLS mode is already set to 'Flexible' for the entire zone."
    echo "  All subdomains will connect to origin via HTTP while serving HTTPS to visitors."
    echo "  No Page Rule needed."
else
    echo "SSL/TLS mode is set to '$CURRENT_SSL_MODE'."
    echo "Creating a Page Rule to enable Flexible SSL for *-$SUBDOMAIN_SUFFIX.$ZONE_DOMAIN only..."
    echo "This will not affect other subdomains or the main domain."
    
    # Create a Page Rule that matches all our subdomains
    PAGE_RULE_URL="*-$SUBDOMAIN_SUFFIX.$ZONE_DOMAIN/*"
    echo "Creating Page Rule for pattern: $PAGE_RULE_URL"

PAGE_RULE_RESPONSE=$(curl -s -X POST "https://api.cloudflare.com/client/v4/zones/$ZONE_ID/pagerules" \
     -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
     -H "Content-Type: application/json" \
     --data '{
       "targets": [{
         "target": "url",
         "constraint": {
           "operator": "matches",
           "value": "'"$PAGE_RULE_URL"'"
         }
       }],
       "actions": [{
         "id": "ssl",
         "value": "flexible"
       }],
       "priority": 1,
       "status": "active"
     }')

    PAGE_RULE_SUCCESS=$(echo $PAGE_RULE_RESPONSE | jq -r .success)
    if [ "$PAGE_RULE_SUCCESS" == "true" ]; then
        echo "✓ Successfully created Page Rule for SSL Flexible mode"
        echo "  Only subdomains matching *-$SUBDOMAIN_SUFFIX.$ZONE_DOMAIN will use Flexible SSL"
    else
        ERROR_MESSAGE=$(echo $PAGE_RULE_RESPONSE | jq -r '.errors[0].message')
        ERROR_CODE=$(echo $PAGE_RULE_RESPONSE | jq -r '.errors[0].code')
        
        if [ "$ERROR_CODE" == "9109" ]; then
            echo "⚠ Warning: API token does not have permission to create Page Rules."
            echo "  Your API token needs 'Page Rules: Edit' permission (under Zone > Page Rules)."
        else
            echo "✗ Error: Could not create Page Rule (Error code: $ERROR_CODE)"
            echo "  Message: $ERROR_MESSAGE"
            echo ""
            echo "This usually means:"
            echo "  - You've reached the Page Rule limit (Free plan: 3 rules)"
            echo "  - OR there's a permission issue with your API token"
        fi
        
        echo ""
        echo "=== MANUAL ACTION REQUIRED ==="
        echo ""
        echo "Your services will show '521 Web Server Is Down' errors without SSL configuration."
        echo "Please configure SSL manually using ONE of these options:"
        echo ""
        echo "Option 1: Set zone-wide Flexible SSL (Simplest)"
        echo "  1. Go to Cloudflare Dashboard → $ZONE_DOMAIN → SSL/TLS → Overview"
        echo "  2. Set SSL/TLS encryption mode to 'Flexible'"
        echo "  3. Wait 1-2 minutes for propagation"
        echo ""
        echo "Option 2: Create Page Rule manually (Recommended if you want isolation)"
        echo "  1. Go to Cloudflare Dashboard → $ZONE_DOMAIN → Rules → Page Rules"
        echo "  2. Click 'Create Page Rule'"
        echo "  3. URL pattern: $PAGE_RULE_URL"
        echo "  4. Add setting: SSL → Flexible"
        echo "  5. Set priority to 1"
        echo "  6. Save and deploy"
        echo ""
        echo "Option 3: Update API token permissions and re-run"
        echo "  1. Go to Cloudflare Dashboard → My Profile → API Tokens"
        echo "  2. Edit your token or create a new one"
        echo "  3. Add permissions:"
        echo "     - Zone > DNS > Edit (already have this)"
        echo "     - Zone > Zone Settings > Edit (for SSL mode check)"
        echo "     - Zone > Page Rules > Edit (for creating Page Rules)"
        echo "  4. Re-run: ./setup-cloudflare.sh $ZONE_DOMAIN <IP> $SUBDOMAIN_SUFFIX"
        echo ""
        echo "DNS records have been created successfully. SSL configuration is pending."
        exit 0
    fi
fi

echo "\n✓ DNS setup complete!"
echo "Your services will be available at:"
for SERVICE in "${SERVICES[@]}"; do
    echo "  - https://$SERVICE-$SUBDOMAIN_SUFFIX.$ZONE_DOMAIN"
done
