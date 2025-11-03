<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# SimpleSAMLphp IdP Reference Implementation

## Purpose

This directory contains a **reference implementation** of a SAML 2.0 Identity Provider (IdP) using SimpleSAMLphp. It serves two purposes:

1. **For Sequent Developers:** Provides a local test IdP for developing and testing the Keycloak SP/broker integration
2. **For Third-Party Integrators:** Demonstrates best practices for implementing IdP-initiated SSO with Sequent's voting platform

## Important Notes

- **This is NOT a production deployment of SimpleSAMLphp for Sequent**
- **Third-party organizations will deploy their own IdP** (which may or may not use SimpleSAMLphp)
- This implementation shows the **pattern** and **configuration approach** that third-parties should follow

## Directory Structure

```
.devcontainer/simplesamlphp/
├── README.md                          # This file
├── config.php                         # Centralized configuration (REFERENCE PATTERN)
├── .env.example                       # Template for third-party integrators
├── Dockerfile                         # Docker setup (dev environment only)
├── cert/                              # SAML signing certificates
│   ├── server.pem                     # Private key
│   └── server.crt                     # Public certificate
├── config/                            # SimpleSAMLphp configuration files
│   ├── authsources.php                # Authentication sources (example users)
│   ├── config-override-base.php       # Base configuration
│   └── config-override.php            # Environment-specific overrides
├── metadata/                          # SAML metadata configuration
│   ├── saml20-idp-hosted.php          # This IdP's metadata (REFERENCE PATTERN)
│   ├── saml20-sp-remote.php           # Keycloak SP metadata (REFERENCE PATTERN)
│   └── saml20-idp-remote.php          # (unused in IdP-initiated flow)
├── public/                            # Web-accessible files
│   └── idp-initiated-sso.php          # SSO trigger page (REFERENCE PATTERN)
├── scripts/                           # Container scripts
│   └── entrypoint.sh                  # Docker entrypoint
└── simplesaml.conf                    # Apache configuration
```

## Key Concepts for Third-Party Integrators

### Centralized Configuration Pattern

The `config.php` file demonstrates how to:

- **Store all deployment-specific values in one place**
- **Use environment variables** for development/production flexibility
- **Avoid hardcoding** tenant IDs, event IDs, URLs, certificates
- **Support multiple environments** (localhost, staging, production)

**Location:** `config.php`

**Key configuration values:**
```php
return [
    'idp_base_url' => getenv('IDP_BASE_URL') ?: 'http://localhost:8083/simplesaml',
    'tenant_id' => getenv('TENANT_ID') ?: '...',
    'event_id' => getenv('EVENT_ID') ?: '...',
    'sp_base_url' => getenv('SP_BASE_URL') ?: 'http://127.0.0.1:8090',
    // sp_realm is automatically derived from tenant_id and event_id
    'sp_realm' => 'tenant-' . (getenv('TENANT_ID') ?: '...') . '-event-' . (getenv('EVENT_ID') ?: '...'),
    // ... etc
];
```

### Dynamic Metadata Configuration

The metadata files (`metadata/saml20-idp-hosted.php`, `metadata/saml20-sp-remote.php`) show how to:

- **Load the centralized configuration**
- **Build Entity IDs and URLs dynamically**
- **Use configuration values instead of hardcoded strings**

**Example from `metadata/saml20-sp-remote.php`:**
```php
<?php
// Load centralized configuration
require __DIR__ . '/../config/config.php';

// Build URLs dynamically
$keycloakSpAcsUrl = sprintf(
    '%s/realms/%s/broker/%s/endpoint/clients/%s',
    $config['sp_base_url'],
    $config['sp_realm'],
    $config['sp_idp_alias'],
    $config['sp_client_id']
);

$metadata[$config['sp_realm']] = [
    'AssertionConsumerService' => [
        ['Location' => $keycloakSpAcsUrl],
    ],
    // ...
];
```

### SSO Initiation Pattern

The `public/idp-initiated-sso.php` file demonstrates:

- **How to build the SSO initiation URL**
- **How to construct the RelayState** (final destination)
- **How to trigger the SAML flow**

This is the entry point where users would start their authentication journey in a third-party system.

## For Sequent Developers

### Running Locally

The SimpleSAMLphp IdP runs automatically when you start the dev container. It's configured via:

- **Environment:** `.devcontainer/.env.development`
- **Docker Compose:** `.devcontainer/docker-compose.yml`

**Access points:**
- SimpleSAMLphp Admin: `http://localhost:8083/simplesaml/`
- IdP Metadata: `http://localhost:8083/simplesaml/saml2/idp/metadata.php`
- SSO Trigger Page: `http://localhost:8083/simplesaml/idp-initiated-sso.php`

**Test users** (configured in `config/authsources.php`):
- `student` / `studentpass` (email: student@example.com)
- `employee` / `employeepass` (email: employee@example.com)

### Testing the Integration

1. Start dev container
2. Navigate to `http://localhost:8083/simplesaml/idp-initiated-sso.php`
3. Click "Login to Service Provider (Keycloak)"
4. Authenticate with test credentials
5. Should redirect to voting portal at `http://localhost:3000/tenant/.../event/.../login`

### Updating Configuration

To test with different tenant/event IDs or URLs:

1. Update values in `.devcontainer/.env.development`
2. Restart the SimpleSAMLphp container
3. Configuration is automatically loaded from environment variables

## For Third-Party Integrators

### Using This as a Reference

**DO:**
- Study the configuration pattern in `config.php`
- Review how metadata files load and use configuration
- Understand the SSO initiation flow in `public/idp-initiated-sso.php`
- Copy the `.env.example` as a template
- Adapt the patterns to your IdP platform (ADFS, Okta, Shibboleth, etc.)
- Test this implementation locally against Sequent's staging environment

**DON'T:**
- Copy this implementation directly to production without customization
- Assume you must use SimpleSAMLphp (any SAML 2.0 IdP works)
- Hardcode tenant IDs, event IDs, or URLs in your implementation
- Share this implementation with end users (it's a developer/integration tool only)

### Configuration Values You'll Need from Sequent

Contact Sequent to obtain:

- `TENANT_ID`: Your organization's tenant UUID
- `EVENT_ID`: Your voting event UUID
- `SP_BASE_URL`: Service Provider URL (format: `https://auth-{subdomain}.sequentech.io`)
- `VOTING_PORTAL_URL`: Voting portal base URL (format: `https://voting-{subdomain}.sequentech.io`)
- `SP_IDP_ALIAS`: Your IdP alias in Sequent's authentication service
- `SP_CLIENT_ID`: SAML client ID (typically `vp-sso`)
- `SP_CERT_DATA`: Service Provider's public certificate

**Note**: The realm identifier is automatically constructed as `tenant-{TENANT_ID}-event-{EVENT_ID}` - you don't need to configure it separately.

See `.env.example` for a complete template.

### Integration Documentation

**For detailed integration instructions, see:**

📖 [IdP-Initiated SSO Integration Guide](../../docs/docusaurus/docs/integrations/idp_initiated_sso_integration_guide.md)

This guide includes:
- Complete integration steps
- Production deployment checklist
- Security considerations
- Troubleshooting guide
- Code examples for various platforms

## Security Notes

### Certificate Management

- **Development:** Self-signed certificates in `cert/` directory
- **Production:** Third-parties must use valid certificates from a trusted CA
- **Never commit private keys** to version control

### HTTPS Requirements

- **Development:** HTTP acceptable for localhost testing
- **Production:** HTTPS is **mandatory** for all endpoints

### SAML Signing

- All SAML responses/assertions must be signed
- Use RSA-SHA256 or stronger signing algorithms
- Validate signatures on both IdP and SP sides

## Support

**For Sequent developers:**
- Internal documentation: [Delivery Team Configuration Guide](../../docs/docusaurus/docs/developers/10-Tutorials/03-setup_idp_initiated_sso.md)
- Design documentation: [IdP-Initiated SSO Design & Implementation](../../docs/docusaurus/docs/developers/06-Keycloak/idp_initiated_sso_design_implementation.md)

**For third-party integrators:**
- Integration guide: [IdP-Initiated SSO Integration Guide](../../docs/docusaurus/docs/integrations/idp_initiated_sso_integration_guide.md)
- Technical support: support@sequentech.io

## About This Implementation

This SimpleSAMLphp setup is **not** part of Sequent's production infrastructure. It exists solely as a reference and testing tool. In production deployments:

- **Third-party organizations** run their own IdP using their chosen platform
- **Sequent** operates the Service Provider (authentication service) side only
- This reference implementation demonstrates the **patterns and configuration** that third-parties should replicate in their own IdP

The value of this implementation is showing **how** to structure configuration, **how** to build metadata, and **how** the SSO flow works - not providing a ready-made production solution.
