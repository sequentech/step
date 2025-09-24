---
id: braid_trustees_configuration
title: Braid Trustees Configuration Guide
sidebar_label: Braid Trustees Configuration
---

# Braid Trustees Configuration Guide

This document provides a comprehensive guide to configuring Braid Trustees, which are critical components in the STEP voting system responsible for managing cryptographic keys and participating in the election process.

## Overview

Braid Trustees are containerized services that handle key generation, management, and cryptographic operations during elections. Each trustee holds a share of the election's private key, ensuring that no single entity can decrypt votes without cooperation from other trustees.

## Configuration Options

### Environment Variables

The following environment variables can be used to configure Braid Trustees:

#### Required Configuration

| Variable | Description | Required | Default |
|----------|-------------|----------|---------|
| `TRUSTEE_NAME` | Unique identifier for the trustee instance | Yes* | - |
| `B3_URL` | URL of the B3 service for trustee communication | Yes | - |
| `IMMUDB_URL` | ImmuDB database connection URL | Yes | - |
| `IMMUDB_USER` | ImmuDB username for authentication | Yes | - |
| `IMMUDB_PASSWORD` | ImmuDB password for authentication | Yes | - |

*Required unless `TRUSTEE_CONFIG_PATH` is set with an existing configuration file.

#### Optional Configuration

| Variable | Description | Required | Default |
|----------|-------------|----------|---------|
| `TRUSTEE_CONFIG_PATH` | Path to the trustee configuration file | No | `/opt/braid/trustee.toml` |
| `SECRETS_BACKEND` | Backend service for secrets management | No | `AwsSecretManager` |
| `AWS_SM_KEY_PREFIX` | Prefix for AWS Secrets Manager keys | Conditional* | - |
| `VAULT_SERVER_URL` | HashiCorp Vault server URL | Conditional** | - |
| `VAULT_TOKEN` | HashiCorp Vault authentication token | Conditional** | - |
| `TRUSTEE_CONFIG` | Direct configuration content (for EnvVarMasterSecret backend) | Conditional*** | - |
| `IGNORE_BOARDS` | Boards to ignore during processing | No | - |

*Required when `SECRETS_BACKEND` is set to `AwsSecretManager` and no existing config file is present.

**Required when `SECRETS_BACKEND` is set to `HashiCorpVault`.

***Used when `SECRETS_BACKEND` is set to `EnvVarMasterSecret`.

### Secrets Backend Options

The trustee supports three different secrets management backends:

#### 1. AWS Secrets Manager (`AwsSecretManager`)

When using AWS Secrets Manager:
- Set `SECRETS_BACKEND=AwsSecretManager`
- Provide `AWS_SM_KEY_PREFIX` for key namespacing
- The trustee will store/retrieve configuration from AWS Secrets Manager
- Secret key format: `${AWS_SM_KEY_PREFIX}secrets/${TRUSTEE_NAME}_config`

Example:
```bash
export SECRETS_BACKEND="AwsSecretManager"
export AWS_SM_KEY_PREFIX="production/elections/"
export TRUSTEE_NAME="trustee-1"
```

#### 2. HashiCorp Vault (`HashiCorpVault`)

When using HashiCorp Vault:
- Set `SECRETS_BACKEND=HashiCorpVault`
- Provide `VAULT_SERVER_URL` and `VAULT_TOKEN`
- The trustee will store/retrieve configuration from Vault
- Secret key format: `secrets/${TRUSTEE_NAME}_config`

Example:
```bash
export SECRETS_BACKEND="HashiCorpVault"
export VAULT_SERVER_URL="https://vault.example.com:8200"
export VAULT_TOKEN="s.XXXXXXXXXXXXXXXXXXXXXXXX"
export TRUSTEE_NAME="trustee-1"
```

#### 3. Environment Variable (`EnvVarMasterSecret`)

When using environment variables directly:
- Set `SECRETS_BACKEND=EnvVarMasterSecret`
- Provide configuration directly via `TRUSTEE_CONFIG` environment variable
- Useful for development or when secrets are managed externally

Example:
```bash
export SECRETS_BACKEND="EnvVarMasterSecret"
export TRUSTEE_NAME="trustee1"
export TRUSTEE_CONFIG="signing_key_sk = \"MC4CAQAwBQYDK2VwBCIEIJAtmrHtGFYiS5tUQepIlrFtCCcKHeSzzuJ2pZqH4bat\"
signing_key_pk = \"MCowBQYDK2VwAyEAy1vJM4P85hJ1WAPZpRX3/QsOT2usIAuVy4/+t5VHHDs=\"
encryption_key = \"lQr2vrVuZJ5PAoOkVSfLfuIG7mxt8exlgAnRMBi+4rg\""
export TRUSTEE_CONFIG_PATH="/opt/braid/trustee-new.toml"
```

### Configuration File

If you prefer to use a configuration file instead of secrets management, you can mount a `trustee.toml` file at the path specified by `TRUSTEE_CONFIG_PATH`. When a configuration file exists at this path, the trustee will use it instead of querying the secrets backend.

Example `trustee.toml`:
```toml
signing_key_sk = "MC4CAQAwBQYDK2VwBCIEIJAtmrHtGFYiS5tUQepIlrFtCCcKHeSzzuJ2pZqH4bat"
signing_key_pk = "MCowBQYDK2VwAyEAy1vJM4P85hJ1WAPZpRX3/QsOT2usIAuVy4/+t5VHHDs="
encryption_key = "lQr2vrVuZJ5PAoOkVSfLfuIG7mxt8exlgAnRMBi+4rg"
```

## Configuration Generation

If no configuration exists (either as a file or in the secrets backend), the trustee will automatically generate a new configuration using the `gen_trustee_config` utility. This ensures that trustees can be deployed without manual configuration preparation.

The generated configuration will be:
1. Created by `gen_trustee_config`
2. Stored in the configured secrets backend (if not using `EnvVarMasterSecret`)
3. Written to `TRUSTEE_CONFIG_PATH` for use by the trustee process

## Security Considerations

1. **Secrets Management**: Always use a proper secrets backend in production. The `EnvVarMasterSecret` option should only be used for development.

2. **Network Security**: Ensure that communication between trustees, B3 service, and ImmuDB is properly secured using TLS.

3. **Access Control**: Limit access to trustee configuration and secrets to authorized personnel only.

4. **Key Storage**: The trustee's cryptographic keys are critical. Ensure proper backup and access control procedures.

## Troubleshooting

### Common Issues

1. **Missing TRUSTEE_NAME**: If you see "Error: TRUSTEE_NAME must be set", ensure you've provided either:
   - The `TRUSTEE_NAME` environment variable, or
   - A configuration file at `TRUSTEE_CONFIG_PATH`

2. **Missing AWS_SM_KEY_PREFIX**: When using AWS Secrets Manager, this prefix is required to namespace your secrets properly.

3. **Configuration Generation Failures**: Check that the `gen_trustee_config` binary is available in the container and has proper permissions.

4. **Secrets Backend Connection Issues**: Verify network connectivity and authentication credentials for your chosen secrets backend.

### Logging

The trustee startup script logs important events with timestamps. Monitor these logs for:
- Configuration source (file, secrets backend, or generated)
- Successful configuration writes
- Connection attempts to external services

## Related Documentation

- [Admin Portal Trustees Settings](../../admin_portal/02-Reference/User-Manual/Settings/settings_trustees.md)
