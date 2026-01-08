# Remote Deployment Scripts

This directory contains all scripts needed for deploying Sequent Step to a remote server.

## Files

- **`provision-server.sh`** - Placeholder script with machine specifications and cloud provider provisioning examples
- **`prepare-server.sh`** - Installs Docker, Docker Compose, and clones the repository on a fresh Ubuntu server
- **`configure-urls.sh`** - Configures the `.env` file with domain and subdomain settings
- **`setup-cloudflare.sh`** - Automates DNS record creation in Cloudflare

## Related Files

The following files work together with these scripts but are kept in the main `.devcontainer` directory:

- **`.env.remote-deployment.example`** - Example environment configuration (copy to `.env`)
- **`docker-compose-remote.yml`** - Docker Compose file for remote deployment with reverse proxy

## Full Documentation

For complete deployment instructions, see:
`/docs/docusaurus/docs/10-local-deployment-tutorial/README.md`

## Quick Start

1. Provision a server (see `provision-server.sh` for specs)
2. Run `prepare-server.sh` on the server
3. Copy `.env.remote-deployment.example` to `.env`
4. Run `configure-urls.sh <domain> <subdomain_suffix>`
5. Run `setup-cloudflare.sh <domain> <server_ip> <subdomain_suffix>`
6. Run `docker-compose -f docker-compose-remote.yml up -d --build`
