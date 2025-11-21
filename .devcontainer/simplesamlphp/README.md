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
├── Dockerfile                         # Docker setup (dev environment only)
├── cert/                              # SAML signing certificates
│   ├── server.pem                     # Private key
│   └── server.crt                     # Public certificate
├── config/                            # SimpleSAMLphp configuration files
│   ├── authsources.php                # Authentication sources (example users)
│   ├── config-override-base.php       # Configuration
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
