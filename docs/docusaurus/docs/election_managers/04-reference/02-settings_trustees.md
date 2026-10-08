---
id: admin_portal_reference_user_manual_settings_trustees
title: Trustees Settings
sidebar_label: Trustees
---

# Trustees Settings

The Trustees settings section in the Admin Portal allows you to manage and configure the trustees for your elections. Trustees are critical components that hold shares of the election's cryptographic keys, ensuring the security and integrity of the voting process.

## Overview

In the STEP voting system, trustees work together to:
- Generate and manage cryptographic keys for elections
- Participate in the decryption process during tallying
- Ensure no single entity can compromise the election's security

## Configuration

Trustees in STEP are deployed as containerized services using the Braid trustee implementation. Each trustee requires specific configuration including:

- Unique trustee identifier
- Connection details to the B3 service
- Database credentials
- Secrets management backend configuration

## Technical Documentation

For detailed information about configuring Braid Trustees, including:
- Environment variables
- Secrets management options
- Docker deployment
- Security considerations

Please refer to the comprehensive [Braid Trustees Configuration Guide](/docs/developers/Braid/braid_trustees_configuration).

## Managing Trustees in Admin Portal

*This section will be updated with specific instructions for managing trustees through the Admin Portal interface.*

### Adding Trustees

Coming soon...

### Monitoring Trustee Status

Coming soon...

### Key Ceremony Process

Coming soon...
