---
id: api_authentication
title: API Authentication with Keycloak
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# API Authentication with Keycloak

This tutorial demonstrates how to authenticate with the Sequent Voting Platform API using Keycloak. You'll learn how to obtain access tokens, refresh them, and use them to make authenticated API requests.

## Overview

The Sequent Voting Platform uses [Keycloak](https://www.keycloak.org/) for authentication and authorization. To interact with the GraphQL API, you need to:

1. Obtain an access token from Keycloak using your credentials
2. Include the token in API requests as a Bearer token
3. Refresh the token before it expires

## Prerequisites

Before starting, ensure you have:

- Access to a Sequent Voting Platform instance
- Valid credentials (username and password)
- Client credentials (client ID and client secret)
- Your tenant ID
- Python 3.8 or higher (for Python examples)
- The `requests` library installed: `pip install requests`

## Environment Setup

Set up the following environment variables with your instance details:

| Variable | Description | Example |
|----------|-------------|---------|
| `KEYCLOAK_URL` | Base URL of your Keycloak instance | `https://keycloak.example.sequent.vote` |
| `TENANT_ID` | Your tenant identifier | `my-tenant-123` |
| `CLIENT_ID` | OAuth2 client ID for CLI/admin operations | `admin-portal` |
| `CLIENT_SECRET` | OAuth2 client secret | `admin-portal-client-secret` |
| `USERNAME` | Your username | `user@example.com` |
| `PASSWORD` | Your password | `your-secure-password` |

**Important Notes:**
- For **CLI and administrative operations** (like importing election events, managing users), use the `admin-portal` client ID which has the necessary permissions
- **Security Note:** Never commit credentials to version control. Use environment variables or a secure secrets management system.

### Setting Up Environment Variables

For **bash/CURL** (Linux/macOS):

```bash
# Export environment variables in your terminal
export KEYCLOAK_URL="https://keycloak.example.sequent.vote"
export TENANT_ID="my-tenant-123"
export CLIENT_ID="admin-portal"
export CLIENT_SECRET="admin-portal-client-secret"
export USERNAME="user@example.com"
export PASSWORD="your-secure-password"
```

Alternatively, create a `.env` file and source it:

```bash
# Create .env file (don't commit this to Git!)
cat > .env <<EOF
export KEYCLOAK_URL="https://keycloak.example.sequent.vote"
export TENANT_ID="my-tenant-123"
export CLIENT_ID="admin-portal"
export CLIENT_SECRET="admin-portal-client-secret"
export USERNAME="user@example.com"
export PASSWORD="your-secure-password"
EOF

# Load the environment variables
source .env
```

For **Python**, environment variables are automatically available via `os.getenv()`.

## 1. Obtaining an Access Token

### Understanding the Token Endpoint

Keycloak uses the OAuth2 password grant flow for authentication. The token endpoint follows this URL pattern:

```
POST {keycloak_url}/realms/tenant-{tenant_id}/protocol/openid-connect/token
```

For example, if your Keycloak URL is `https://keycloak.example.sequent.vote` and your tenant ID is `my-tenant-123`, the endpoint would be:

```
https://keycloak.example.sequent.vote/realms/tenant-my-tenant-123/protocol/openid-connect/token
```

### CURL Example

```bash
# Using environment variables
curl -X POST "${KEYCLOAK_URL}/realms/tenant-${TENANT_ID}/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=password" \
  -d "scope=openid" \
  -d "client_id=${CLIENT_ID}" \
  -d "client_secret=${CLIENT_SECRET}" \
  -d "username=${USERNAME}" \
  -d "password=${PASSWORD}"
```

### Python Example

```python
import requests
import os

def get_access_token():
    """Obtain an access token from Keycloak."""
    keycloak_url = os.getenv('KEYCLOAK_URL')
    tenant_id = os.getenv('TENANT_ID')
    client_id = os.getenv('CLIENT_ID')
    client_secret = os.getenv('CLIENT_SECRET')
    username = os.getenv('USERNAME')
    password = os.getenv('PASSWORD')

    # Build the token endpoint URL
    realm = f"tenant-{tenant_id}"
    token_url = f"{keycloak_url}/realms/{realm}/protocol/openid-connect/token"

    # Prepare the request parameters
    data = {
        'grant_type': 'password',
        'scope': 'openid',
        'client_id': client_id,
        'client_secret': client_secret,
        'username': username,
        'password': password
    }

    # Make the request
    try:
        response = requests.post(token_url, data=data)
        response.raise_for_status()

        token_data = response.json()
        return token_data
    except requests.exceptions.RequestException as e:
        print(f"Error obtaining token: {e}")
        if hasattr(e.response, 'text'):
            print(f"Response: {e.response.text}")
        raise

# Usage
token_response = get_access_token()
access_token = token_response['access_token']
refresh_token = token_response['refresh_token']
expires_in = token_response['expires_in']

print(f"Access token obtained, expires in {expires_in} seconds")
```

### Sample Response

A successful response will look like this:

```json
{
  "access_token": "eyJhbGciOiJSUzI1NiIsInR5cCIgOiAiSldUIiwia2lkIiA6ICJC...",
  "expires_in": 300,
  "refresh_expires_in": 1800,
  "refresh_token": "eyJhbGciOiJIUzUxMiIsInR5cCIgOiAiSldUIiwia2lkIiA6IC...",
  "token_type": "Bearer",
  "id_token": "eyJhbGciOiJSUzI1NiIsInR5cCIgOiAiSldUIiwia2lkIiA6ICJC...",
  "not-before-policy": 1712715173,
  "session_state": "0986ebd3-b08b-9981-191a-5a418381d0bd",
  "scope": "openid profile email"
}
```

Key fields:
- `access_token`: Use this to authenticate API requests (JWT format)
- `refresh_token`: Use this to obtain a new access token when it expires
- `expires_in`: Token lifetime in seconds (typically 300 = 5 minutes)
- `refresh_expires_in`: Refresh token lifetime in seconds (typically 1800 = 30 minutes)
- `token_type`: Always "Bearer" for Keycloak
- `id_token`: OpenID Connect ID token containing user identity claims
- `scope`: Granted OAuth2 scopes

## 2. Refreshing Access Tokens

Access tokens expire after a set period (typically 1 hour). Instead of re-authenticating with your username and password, use the refresh token to obtain a new access token.

### When to Refresh

Refresh your token before it expires. A good practice is to refresh when there's less than 5 minutes remaining:

```python
import time

# Store token acquisition time
token_acquired_at = time.time()
expires_in = token_response['expires_in']

# Check if token needs refresh (with 5 minute buffer)
if time.time() - token_acquired_at > expires_in - 300:
    # Refresh the token
    pass
```

### CURL Example

```bash
# Save the refresh token from the initial authentication
REFRESH_TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."

# Use it to get a new access token
curl -X POST "${KEYCLOAK_URL}/realms/tenant-${TENANT_ID}/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=refresh_token" \
  -d "client_id=${CLIENT_ID}" \
  -d "client_secret=${CLIENT_SECRET}" \
  -d "refresh_token=${REFRESH_TOKEN}"
```

### Python Example

```python
def refresh_access_token(refresh_token):
    """Refresh an access token using a refresh token."""
    keycloak_url = os.getenv('KEYCLOAK_URL')
    tenant_id = os.getenv('TENANT_ID')
    client_id = os.getenv('CLIENT_ID')
    client_secret = os.getenv('CLIENT_SECRET')

    # Build the token endpoint URL
    realm = f"tenant-{tenant_id}"
    token_url = f"{keycloak_url}/realms/{realm}/protocol/openid-connect/token"

    # Prepare the request parameters
    data = {
        'grant_type': 'refresh_token',
        'client_id': client_id,
        'client_secret': client_secret,
        'refresh_token': refresh_token
    }

    # Make the request
    try:
        response = requests.post(token_url, data=data)
        response.raise_for_status()

        token_data = response.json()
        return token_data
    except requests.exceptions.RequestException as e:
        print(f"Error refreshing token: {e}")
        if hasattr(e.response, 'text'):
            print(f"Response: {e.response.text}")
        raise

# Usage
new_token_response = refresh_access_token(refresh_token)
access_token = new_token_response['access_token']
refresh_token = new_token_response['refresh_token']  # Also get new refresh token
```

## 3. Using Tokens in API Requests

Once you have an access token, include it in the `Authorization` header of your API requests using the Bearer authentication scheme.

### Example GraphQL Request

```bash
# Save the access token from authentication
ACCESS_TOKEN="eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9..."

# Use it in API requests
curl -X POST "https://api.example.sequent.vote/graphql" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${ACCESS_TOKEN}" \
  -d '{
    "query": "query { __typename }"
  }'
```

**Expected Response:**
```json
{
  "data": {
    "__typename": "query_root"
  }
}
```

### Python Example

```python
def make_authenticated_request(access_token, query):
    """Make an authenticated GraphQL request."""
    api_url = "https://api.example.sequent.vote/graphql"

    headers = {
        'Content-Type': 'application/json',
        'Authorization': f'Bearer {access_token}'
    }

    payload = {
        'query': query
    }

    response = requests.post(api_url, json=payload, headers=headers)
    response.raise_for_status()
    return response.json()

# Usage
query = "query { __typename }"
result = make_authenticated_request(access_token, query)
print(result)
```

## 4. Troubleshooting

### 401 Unauthorized - Invalid Credentials

**Problem:** Authentication fails with HTTP 401 status.

**Possible Causes:**
- Incorrect username or password
- Incorrect client ID or client secret
- User account disabled or locked

**Solution:**
- Verify all credentials are correct
- Check that the user account is active in Keycloak
- Ensure client credentials match your Keycloak client configuration

### 403 Forbidden - Insufficient Permissions

**Problem:** Request fails with HTTP 403 status or GraphQL mutation not found.

**Possible Causes:**
- User lacks required permissions
- Client lacks required scopes
- Using wrong client ID (e.g., `service-account` instead of `admin-portal` for administrative operations)
- Incorrect tenant ID

**Solution:**
- **Use the correct client ID**: For administrative operations like importing election events or managing users, use `admin-portal`
- Verify the user has appropriate roles in Keycloak
- Check client scope configuration in Keycloak
- Confirm you're using the correct tenant ID

**Example:** If you receive an error like `"field 'get_upload_url' not found"`, ensure you're using the `admin-portal` client credentials.

### Token Expiration

**Problem:** API requests start failing after some time with 401 errors.

**Solution:**
- Implement automatic token refresh before expiration
- Use the `SequentAuthClient` class which handles this automatically
- Check the `expires_in` field and refresh with a buffer

### Invalid Realm/Tenant Error

**Problem:** Error message indicates realm not found.

**Possible Causes:**
- Incorrect tenant ID
- Tenant doesn't exist in Keycloak
- Wrong Keycloak URL

**Solution:**
- Verify tenant ID matches your Keycloak realm name (without the `tenant-` prefix)
- Check that the realm exists: `https://keycloak.example.sequent.vote/realms/tenant-{tenant_id}`
- Confirm Keycloak URL is correct and accessible

### SSL/TLS Errors

**Problem:** Certificate verification failures.

**Solution:**
- Ensure your Keycloak instance has a valid SSL certificate
- Update your system's CA certificates if needed
- Use HTTPS for all production environments

## 5. Security Considerations

### Never Commit Credentials

Store credentials securely:
- Use environment variables (`.env` files with `.gitignore`)
- Use secrets management systems (HashiCorp Vault, AWS Secrets Manager, etc.)
- Never hardcode credentials in source code
- Never commit `.env` files to version control

### Always Use HTTPS

- Keycloak must use HTTPS in production
- Never send credentials over HTTP
- Validate SSL certificates (don't disable verification)

### Secure Token Storage

- Store tokens in memory when possible
- If persisting tokens, encrypt them
- Clear tokens on logout
- Never log tokens in plaintext

### Rotate Credentials Regularly

- Change passwords periodically
- Rotate client secrets
- Revoke old tokens when no longer needed

### Token Scope

- Request only the scopes you need (principle of least privilege)
- The `openid` scope is typically sufficient for API authentication

## Next Steps

Now that you can authenticate with the Sequent API, you're ready to perform operations:

- [Import Election Event](./06-import-election-event.md) - Learn how to upload and import election events via the API
- [GraphQL API Reference](../01-graphql-api.md) - Explore available GraphQL queries and mutations
- [CLI Tutorials](../02-cli/02-tutorials/01-cli-tutorials-getting-started.md) - Learn about the Sequent CLI tool

For questions or support, refer to the [Support](../../09-support/01-support.md) section.
