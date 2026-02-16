---
id: api_import_election_event
title: Import Election Event via API
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Import Election Event via API

This tutorial demonstrates how to import an election event into the Sequent Voting Platform using the GraphQL API. You'll learn the complete workflow from uploading a file to triggering the import process.

## Overview

Importing an election event is a three-step process:

1. **Get Upload URL**: Request a pre-signed URL from the API for uploading your file
2. **Upload File**: Upload your election event JSON file to object storage (S3/MinIO)
3. **Import Election Event**: Trigger the import process using the document ID

## Prerequisites

Before starting, ensure you have:

- Completed the [API Authentication](./05-api-authentication.md) tutorial
- A valid election event JSON file to import
- An authenticated access token from Keycloak
- Python 3.8 or higher (for Python examples)
- The `requests` library installed: `pip install requests`

### Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `HASURA_URL` | GraphQL API endpoint | `https://api.example.sequent.vote/graphql` |
| `TENANT_ID` | Your tenant identifier | `my-tenant-123` |
| `ACCESS_TOKEN` | Keycloak access token | `eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...` |

## 1. Understanding the Workflow

The import process is split into three distinct operations for security and scalability:

```
┌─────────────────────┐
│  1. Get Upload URL  │  Request pre-signed URL from API
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│   2. Upload File    │  PUT file directly to object storage
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ 3. Import Election  │  Trigger import with document_id
└─────────────────────┘
```

This approach:
- Offloads file uploads from the API server to object storage
- Provides secure, time-limited upload URLs
- Supports large files efficiently
- Separates upload from processing

## 2. Get Upload URL (Step 1)

### GraphQL Mutation

```graphql
mutation GetUploadUrl(
    $name: String!
    $media_type: String!
    $size: Int!
    $is_public: Boolean!
    $is_local: Boolean
    $election_event_id: String
) {
    get_upload_url(
        name: $name
        media_type: $media_type
        size: $size
        is_public: $is_public
        is_local: $is_local
        election_event_id: $election_event_id
    ) {
        url
        document_id
    }
}
```

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `name` | String | Yes | Filename (e.g., "my-election.json") |
| `media_type` | String | Yes | MIME type (e.g., "application/json") |
| `size` | Int | Yes | File size in bytes |
| `is_public` | Boolean | Yes | Whether file is publicly accessible (use `false` for election imports) |
| `is_local` | Boolean | No | Set to `true` if using local MinIO instead of S3 |
| `election_event_id` | String | No | Optional election event ID to associate with the file |

### Response

```json
{
  "data": {
    "get_upload_url": {
      "url": "https://s3.example.com/uploads/abc123.json?signature=...",
      "document_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
    }
  }
}
```

- `url`: Pre-signed S3/MinIO URL for uploading (time-limited, typically 15 minutes)
- `document_id`: UUID to reference this document in subsequent operations

### CURL Example

```bash
# Read file size
FILE_PATH="election-event.json"
FILE_SIZE=$(wc -c < "$FILE_PATH")

# Make GraphQL request
curl -X POST "https://api.example.sequent.vote/graphql" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -d '{
    "query": "mutation GetUploadUrl($name: String!, $media_type: String!, $size: Int!, $is_public: Boolean!, $is_local: Boolean) { get_upload_url(name: $name, media_type: $media_type, size: $size, is_public: $is_public, is_local: $is_local) { url document_id } }",
    "variables": {
      "name": "election-event.json",
      "media_type": "application/json",
      "size": '"$FILE_SIZE"',
      "is_public": false,
      "is_local": false
    }
  }'
```

### Python Example

```python
import requests
import os

def get_upload_url(file_path, access_token, is_local=False):
    """Get a pre-signed upload URL for a file."""
    hasura_url = os.getenv('HASURA_URL', 'https://api.example.sequent.vote/graphql')

    # Get file information
    file_size = os.path.getsize(file_path)
    file_name = os.path.basename(file_path)

    # Determine MIME type based on extension
    if file_path.endswith('.json'):
        media_type = 'application/json'
    elif file_path.endswith('.zip'):
        media_type = 'application/zip'
    else:
        media_type = 'application/octet-stream'

    # GraphQL mutation
    query = """
    mutation GetUploadUrl(
        $name: String!
        $media_type: String!
        $size: Int!
        $is_public: Boolean!
        $is_local: Boolean
    ) {
        get_upload_url(
            name: $name
            media_type: $media_type
            size: $size
            is_public: $is_public
            is_local: $is_local
        ) {
            url
            document_id
        }
    }
    """

    variables = {
        'name': file_name,
        'media_type': media_type,
        'size': file_size,
        'is_public': False,
        'is_local': is_local
    }

    headers = {
        'Content-Type': 'application/json',
        'Authorization': f'Bearer {access_token}'
    }

    response = requests.post(
        hasura_url,
        json={'query': query, 'variables': variables},
        headers=headers
    )
    response.raise_for_status()

    result = response.json()

    # Check for GraphQL errors
    if 'errors' in result:
        error_messages = [e['message'] for e in result['errors']]
        raise Exception(f"GraphQL errors: {', '.join(error_messages)}")

    return result['data']['get_upload_url']

# Usage
upload_info = get_upload_url('election-event.json', access_token)
upload_url = upload_info['url']
document_id = upload_info['document_id']

print(f"Upload URL: {upload_url}")
print(f"Document ID: {document_id}")
```

## 3. Upload File to Storage (Step 2)

Once you have the pre-signed URL, upload your file directly to object storage using a PUT request.

### Important Notes

- Use the exact URL returned from `get_upload_url` (includes authentication signature)
- Set the `Content-Type` header to match the `media_type` from step 1
- Upload the raw file contents
- The URL is time-limited (typically 15 minutes) - use it immediately

### CURL Example

```bash
# Upload file using the pre-signed URL
curl -X PUT "$UPLOAD_URL" \
  -H "Content-Type: application/json" \
  --data-binary "@election-event.json"
```

### Python Example

```python
def upload_file(file_path, upload_url, media_type='application/json'):
    """Upload a file to the pre-signed URL."""
    with open(file_path, 'rb') as f:
        file_contents = f.read()

    headers = {
        'Content-Type': media_type
    }

    response = requests.put(
        upload_url,
        data=file_contents,
        headers=headers
    )
    response.raise_for_status()

    print(f"File uploaded successfully (status: {response.status_code})")

# Usage
upload_file('election-event.json', upload_url, 'application/json')
```

## 4. Import Election Event (Step 3)

After uploading the file, trigger the import process using the `document_id` from step 1.

### GraphQL Mutation

```graphql
mutation ImportElectionEvent(
    $tenant_id: String!
    $document_id: String!
    $check_only: Boolean
) {
    import_election_event(
        tenant_id: $tenant_id
        document_id: $document_id
        check_only: $check_only
    ) {
        id
        message
        error
    }
}
```

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `tenant_id` | String | Yes | Your tenant identifier |
| `document_id` | String | Yes | Document ID from `get_upload_url` response |
| `check_only` | Boolean | No | If `true`, validates the import without executing (default: `false`) |

### Response

```json
{
  "data": {
    "import_election_event": {
      "id": "b2c3d4e5-f6a7-8901-bcde-f12345678901",
      "message": "Election event imported successfully",
      "error": null
    }
  }
}
```

- `id`: UUID of the created election event (if successful)
- `message`: Status message
- `error`: Error message (if import failed, otherwise `null`)

### CURL Example

```bash
curl -X POST "https://api.example.sequent.vote/graphql" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -d '{
    "query": "mutation ImportElectionEvent($tenant_id: String!, $document_id: String!, $check_only: Boolean) { import_election_event(tenant_id: $tenant_id, document_id: $document_id, check_only: $check_only) { id message error } }",
    "variables": {
      "tenant_id": "my-tenant-123",
      "document_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
      "check_only": false
    }
  }'
```

### Python Example

```python
def import_election_event(document_id, access_token, tenant_id, check_only=False):
    """Import an election event using the uploaded document."""
    hasura_url = os.getenv('HASURA_URL', 'https://api.example.sequent.vote/graphql')

    query = """
    mutation ImportElectionEvent(
        $tenant_id: String!
        $document_id: String!
        $check_only: Boolean
    ) {
        import_election_event(
            tenant_id: $tenant_id
            document_id: $document_id
            check_only: $check_only
        ) {
            id
            message
            error
        }
    }
    """

    variables = {
        'tenant_id': tenant_id,
        'document_id': document_id,
        'check_only': check_only
    }

    headers = {
        'Content-Type': 'application/json',
        'Authorization': f'Bearer {access_token}'
    }

    response = requests.post(
        hasura_url,
        json={'query': query, 'variables': variables},
        headers=headers
    )
    response.raise_for_status()

    result = response.json()

    # Check for GraphQL errors
    if 'errors' in result:
        error_messages = [e['message'] for e in result['errors']]
        raise Exception(f"GraphQL errors: {', '.join(error_messages)}")

    import_result = result['data']['import_election_event']

    # Check for mutation-level errors
    if import_result['error']:
        raise Exception(f"Import error: {import_result['error']}")

    return import_result

# Usage
import_result = import_election_event(
    document_id=document_id,
    access_token=access_token,
    tenant_id=os.getenv('TENANT_ID')
)

print(f"Election event imported successfully!")
print(f"Election Event ID: {import_result['id']}")
print(f"Message: {import_result['message']}")
```

## 5. Complete End-to-End Example

### Bash Script

```bash
#!/bin/bash
set -e

# Configuration
FILE_PATH="election-event.json"
HASURA_URL="${HASURA_URL:-https://api.example.sequent.vote/graphql}"
TENANT_ID="${TENANT_ID:-my-tenant-123}"
ACCESS_TOKEN="${ACCESS_TOKEN}"

if [ -z "$ACCESS_TOKEN" ]; then
    echo "Error: ACCESS_TOKEN environment variable not set"
    exit 1
fi

if [ ! -f "$FILE_PATH" ]; then
    echo "Error: File not found: $FILE_PATH"
    exit 1
fi

echo "Step 1: Getting upload URL..."
FILE_SIZE=$(wc -c < "$FILE_PATH")

UPLOAD_RESPONSE=$(curl -s -X POST "$HASURA_URL" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -d '{
    "query": "mutation GetUploadUrl($name: String!, $media_type: String!, $size: Int!, $is_public: Boolean!) { get_upload_url(name: $name, media_type: $media_type, size: $size, is_public: $is_public) { url document_id } }",
    "variables": {
      "name": "'"$(basename "$FILE_PATH")"'",
      "media_type": "application/json",
      "size": '"$FILE_SIZE"',
      "is_public": false
    }
  }')

UPLOAD_URL=$(echo "$UPLOAD_RESPONSE" | grep -o '"url":"[^"]*' | cut -d'"' -f4)
DOCUMENT_ID=$(echo "$UPLOAD_RESPONSE" | grep -o '"document_id":"[^"]*' | cut -d'"' -f4)

echo "Document ID: $DOCUMENT_ID"

echo "Step 2: Uploading file..."
curl -X PUT "$UPLOAD_URL" \
  -H "Content-Type: application/json" \
  --data-binary "@$FILE_PATH"

echo -e "\nStep 3: Importing election event..."
IMPORT_RESPONSE=$(curl -s -X POST "$HASURA_URL" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -d '{
    "query": "mutation ImportElectionEvent($tenant_id: String!, $document_id: String!) { import_election_event(tenant_id: $tenant_id, document_id: $document_id) { id message error } }",
    "variables": {
      "tenant_id": "'"$TENANT_ID"'",
      "document_id": "'"$DOCUMENT_ID"'"
    }
  }')

echo "$IMPORT_RESPONSE"

ELECTION_ID=$(echo "$IMPORT_RESPONSE" | grep -o '"id":"[^"]*' | head -1 | cut -d'"' -f4)
ERROR=$(echo "$IMPORT_RESPONSE" | grep -o '"error":"[^"]*' | cut -d'"' -f4)

if [ -n "$ERROR" ] && [ "$ERROR" != "null" ]; then
    echo "Import failed: $ERROR"
    exit 1
fi

echo "Success! Election Event ID: $ELECTION_ID"
```

### Complete Python Script

```python
import requests
import os
import sys

class ElectionEventImporter:
    """Complete importer for election events via the Sequent API."""

    def __init__(self, hasura_url, tenant_id, access_token):
        self.hasura_url = hasura_url
        self.tenant_id = tenant_id
        self.access_token = access_token

    def import_election_event(self, file_path, check_only=False, is_local=False):
        """
        Import an election event from a file.

        Args:
            file_path: Path to the election event JSON file
            check_only: If True, validate without importing
            is_local: If True, use local MinIO instead of S3

        Returns:
            Dictionary with 'id', 'message', and 'error' fields
        """
        try:
            # Step 1: Get upload URL
            print("Step 1: Getting upload URL...")
            upload_info = self._get_upload_url(file_path, is_local)
            upload_url = upload_info['url']
            document_id = upload_info['document_id']
            print(f"  Document ID: {document_id}")

            # Step 2: Upload file
            print("Step 2: Uploading file...")
            media_type = self._get_media_type(file_path)
            self._upload_file(file_path, upload_url, media_type)
            print("  File uploaded successfully")

            # Step 3: Import election event
            print("Step 3: Importing election event...")
            result = self._import_election_event(document_id, check_only)

            if result['error']:
                print(f"  Error: {result['error']}")
                return result

            print(f"  Success! Election Event ID: {result['id']}")
            return result

        except Exception as e:
            print(f"Error during import: {e}")
            raise

    def _get_upload_url(self, file_path, is_local):
        """Get pre-signed upload URL from API."""
        file_size = os.path.getsize(file_path)
        file_name = os.path.basename(file_path)
        media_type = self._get_media_type(file_path)

        query = """
        mutation GetUploadUrl(
            $name: String!,
            $media_type: String!,
            $size: Int!,
            $is_public: Boolean!,
            $is_local: Boolean
        ) {
            get_upload_url(
                name: $name,
                media_type: $media_type,
                size: $size,
                is_public: $is_public,
                is_local: $is_local
            ) {
                url
                document_id
            }
        }
        """

        variables = {
            'name': file_name,
            'media_type': media_type,
            'size': file_size,
            'is_public': False,
            'is_local': is_local
        }

        result = self._graphql_request(query, variables)
        return result['data']['get_upload_url']

    def _upload_file(self, file_path, upload_url, media_type):
        """Upload file to pre-signed URL."""
        with open(file_path, 'rb') as f:
            file_contents = f.read()

        headers = {'Content-Type': media_type}
        response = requests.put(upload_url, data=file_contents, headers=headers)
        response.raise_for_status()

    def _import_election_event(self, document_id, check_only):
        """Trigger the election event import."""
        query = """
        mutation ImportElectionEvent(
            $tenant_id: String!,
            $document_id: String!,
            $check_only: Boolean
        ) {
            import_election_event(
                tenant_id: $tenant_id,
                document_id: $document_id,
                check_only: $check_only
            ) {
                id
                message
                error
            }
        }
        """

        variables = {
            'tenant_id': self.tenant_id,
            'document_id': document_id,
            'check_only': check_only
        }

        result = self._graphql_request(query, variables)
        return result['data']['import_election_event']

    def _graphql_request(self, query, variables):
        """Make a GraphQL request."""
        headers = {
            'Content-Type': 'application/json',
            'Authorization': f'Bearer {self.access_token}'
        }

        response = requests.post(
            self.hasura_url,
            json={'query': query, 'variables': variables},
            headers=headers
        )
        response.raise_for_status()

        result = response.json()

        if 'errors' in result:
            error_messages = [e['message'] for e in result['errors']]
            raise Exception(f"GraphQL errors: {', '.join(error_messages)}")

        return result

    @staticmethod
    def _get_media_type(file_path):
        """Determine MIME type from file extension."""
        if file_path.endswith('.json'):
            return 'application/json'
        elif file_path.endswith('.zip'):
            return 'application/zip'
        else:
            return 'application/octet-stream'


# Example usage
if __name__ == '__main__':
    # Get configuration from environment
    hasura_url = os.getenv('HASURA_URL', 'https://api.example.sequent.vote/graphql')
    tenant_id = os.getenv('TENANT_ID')
    access_token = os.getenv('ACCESS_TOKEN')

    if not tenant_id or not access_token:
        print("Error: TENANT_ID and ACCESS_TOKEN environment variables required")
        sys.exit(1)

    # Create importer
    importer = ElectionEventImporter(hasura_url, tenant_id, access_token)

    # Import election event
    file_path = 'election-event.json'
    result = importer.import_election_event(file_path)

    if result['error']:
        print(f"Import failed: {result['error']}")
        sys.exit(1)

    print(f"Successfully imported election event: {result['id']}")
```

## 6. Validation Mode

Before importing an election event, you can validate it without actually creating it using the `check_only` parameter.

### Example

```python
# Validate the election event file without importing
result = importer.import_election_event(
    'election-event.json',
    check_only=True
)

if result['error']:
    print(f"Validation failed: {result['error']}")
else:
    print(f"Validation passed: {result['message']}")
    # Now import for real
    result = importer.import_election_event('election-event.json')
```

This is useful for:
- Catching validation errors early
- Testing election event configurations
- CI/CD pipelines that validate before deployment

## 7. Troubleshooting

### Upload URL Expired

**Problem:** File upload fails with 403 or signature errors.

**Cause:** The pre-signed URL has expired (typically after 15 minutes).

**Solution:**
- Request a new upload URL with `get_upload_url`
- Upload the file immediately after receiving the URL
- Don't reuse old upload URLs

### Invalid JSON Format

**Problem:** Import fails with "Invalid JSON" error.

**Cause:** The uploaded file is not valid JSON or doesn't match the expected schema.

**Solution:**
- Validate your JSON file syntax before uploading
- Use `check_only: true` to validate the structure
- Ensure the file matches the election event schema
- Check for required fields in the JSON structure

### Authentication Errors

**Problem:** GraphQL requests fail with 401 Unauthorized.

**Cause:** Invalid or expired access token.

**Solution:**
- Verify your access token is current
- Refresh your token if it has expired
- Check the `Authorization` header format: `Bearer {token}`
- Refer to the [Authentication tutorial](./05-api-authentication.md)

### Permission Denied

**Problem:** Request fails with permission errors.

**Cause:** User lacks required permissions to import election events.

**Solution:**
- Verify your user has the appropriate role in Keycloak
- Check tenant permissions
- Contact your administrator for role assignment

### Document Not Found

**Problem:** Import fails with "Document not found" error.

**Cause:** The `document_id` doesn't exist or is invalid.

**Solution:**
- Verify you completed step 2 (file upload) successfully
- Use the exact `document_id` returned from step 1
- Don't reuse document IDs from previous uploads
- Check that the file upload returned a 200 status

### Import Validation Errors

**Problem:** Import fails with validation errors about election configuration.

**Cause:** The election event structure doesn't meet requirements.

**Solution:**
- Review the error message for specific validation failures
- Check required fields are present (name, description, dates, etc.)
- Validate election dates are in the correct format and order
- Ensure all referenced IDs exist and are valid UUIDs
- Use CLI to export a sample election event for reference:
  ```bash
  cli step export-election-event --election-event-id UUID > sample.json
  ```

### Large File Timeouts

**Problem:** Upload times out for large files.

**Cause:** File is too large or network is slow.

**Solution:**
- Check file size before uploading
- Ensure stable network connection
- For very large files, consider compressing to ZIP format
- Monitor upload progress in your code

## 8. Best Practices

### Validate Before Importing

Always use `check_only: true` first to catch errors early:

```python
# Validate first
validation = importer.import_election_event(file_path, check_only=True)
if validation['error']:
    print(f"Validation failed: {validation['error']}")
    sys.exit(1)

# Then import
result = importer.import_election_event(file_path)
```

### Error Handling

Always check for errors in the response:

```python
result = import_election_event(document_id, access_token, tenant_id)

if result['error']:
    # Handle mutation-level error
    print(f"Import failed: {result['error']}")
else:
    # Success
    print(f"Imported: {result['id']}")
```

### Handle Large Files

For large election events:
- Use progress indicators during upload
- Implement retry logic for network failures
- Consider compressing files to ZIP format
- Monitor memory usage when reading files

### Idempotency

Election event imports are not idempotent. Importing the same file multiple times will create multiple election events. To avoid duplicates:
- Check if the election event already exists before importing
- Use unique names or external IDs to track imports
- Implement deduplication logic in your application

### Monitor Import Status

The import process may take time for large election events. Consider:
- Polling the election event status after import
- Implementing webhooks for import completion notifications
- Logging import operations for audit trails

## 9. Next Steps

Now that you can import election events via the API, explore other operations:

- [GraphQL API Reference](../01-graphql-api.md) - Explore all available mutations and queries
- [CLI Tutorials](../02-cli/02-tutorials/02-cli-tutorials-creating-an-election-event.md) - Learn about creating election events with the CLI
- [Export Election Events](../02-cli/03-reference/01-cli_reference.md#export-election-event) - Learn how to export election events

For questions or support, refer to the [Support](../../09-support/01-support.md) section.
