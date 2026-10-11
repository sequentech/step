---
sidebar_label: Remote Standalone Deployment
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Remote Standalone Deployment Tutorial

This tutorial will guide you through setting up a Sequent Step development environment on a single remote server, accessible via a public domain with a reverse proxy managing traffic.

> **⚠️ Deployment Limitations**  
> This tutorial provides a **standalone, single-server deployment** suitable for development, testing, and demonstration purposes. It does **not** include:  
> - High availability or redundancy
> - Automated backups and disaster recovery
> - Load balancing across multiple servers
> - Production-grade monitoring and alerting
> - Security hardening for production environments
> - Automated scaling or failover mechanisms
>
> **For production deployments** with enterprise-grade reliability, managed infrastructure, support, and SLA guarantees, please contact **[Sequent](https://sequentech.io/)**

## Prerequisites

*   A cloud server with the following specifications:
    *   **Minimum Recommended:** 8 vCPUs, 16 GB RAM, **100 GB SSD** (minimum)
    *   **Recommended Instance Types:**
        *   AWS: `c5.2xlarge` (8 vCPUs, 16 GB RAM)
        *   GCP: `n2-standard-8` (8 vCPUs, 32 GB RAM)
        *   Azure: `Standard_D8s_v3` (8 vCPUs, 32 GB RAM)
    *   OS: Ubuntu 24.04 LTS
    *   See `.devcontainer/remote-deployment/provision-server.sh` for detailed specifications and provisioning examples.
    *   optional: See also 'Cloudflare source IP addresses list' in the bottom of the document for firewall allowence.
*   A domain name managed by Cloudflare (e.g., `sequent.vote`).
*   Choose A subdomain suffix for the domain record we'll create. E.g. in admin-mycorporate.domain.com, 'mycorporate' is the subdomain suffix. domain.com is the domain.
*   A Cloudflare API token with the following permissions:
    *   **Required:** `Zone > DNS > Edit` (for creating DNS records)
    *   **Recommended for automatic SSL setup:**
        *   `Zone > Zone Settings > Read` (for checking current SSL mode)
        *   `Zone > Zone Settings > Edit` (for modifying SSL settings)
        *   `Zone > Page Rules > Edit` (for creating SSL Page Rules)
    *   Note: If your token lacks SSL/Page Rule permissions, the script will complete DNS setup and provide manual SSL configuration instructions
*   **Cloudflare SSL Configuration:** One of the following:
    *   **Option 1 (Recommended):** Zone-wide SSL/TLS mode set to "Flexible" (simplest)
    *   **Option 2:** At least 1 available Page Rule slot on your Cloudflare plan (Free plan includes 3 Page Rules)
    *   Note: The setup script will automatically configure SSL if your API token has the required permissions

## 1. Server Preparation

First, you need to prepare your server by installing git, Docker, and Docker Compose, and then clone the `step` repository (currently with default branch - 'main').

1.  SSH into your remote server.

2.  Download and run the `prepare-server.sh` script:

    ```bash
    curl -fsSL https://raw.githubusercontent.com/sequentech/step/refs/heads/main/.devcontainer/remote-deployment/prepare-server.sh -o prepare-server.sh
    chmod +x prepare-server.sh
    ./prepare-server.sh
    ```
    **note:** The link is currently to a feature branch. Need to chang it back to release name or use default main.
                We also need to change the prepare-server.sh to clone a specific release of step repo, with param
3.  Log out and log back in to apply the Docker group membership changes. The `step` repository is now cloned at `/home/your-user/step`.

## 2. Environment Configuration

The configuration script generates all necessary files from templates, including secrets.

1.  Run the configuration script:

    ```bash
    cd ~/step
    chmod +x .devcontainer/remote-deployment/configure-environment.sh
    ./.devcontainer/remote-deployment/configure-environment.sh <DOMAIN (sequent.vote)> <SUBDOMAIN_SUFFIX (remote-deployment)>
    ```

    *   Replace `sequent.vote` with your root domain.
    *   Replace `remote-deployment` with your chosen subdomain suffix (e.g., `qa`, `staging`, `prod`).

    This script will:
    *   Generate `.env` from `.env.remote-deployment.example`
    *   Generate `nginx/default.conf` from `nginx/default.conf.template`
    *   Generate random secrets (MASTER_SECRET, Keycloak client secrets)
    *   Auto configure all service URLs (in the .env config file):
        *   `login-remote-deployment.sequent.vote` (Keycloak)
        *   `admin-remote-deployment.sequent.vote` (Admin Portal)
        *   `voting-remote-deployment.sequent.vote` (Voting Portal)
        *   `hasura-remote-deployment.sequent.vote` (Hasura)
        *   `minio-remote-deployment.sequent.vote` (MinIO)

2.  **(Optional)** Review the generated `.env` file:

    ```bash
    nano ~/step/.devcontainer/.env
    ```

    **Note:** For a basic dev/demo deployment, the default values are sufficient. The system will work with:
    - Default database passwords (`postgrespassword`)
    - Default Keycloak admin credentials (`admin`/`admin`)
    - Dummy email/SMS transports (logs to console)

    You only need to configure additional secrets if you want:
    - Real Twilio SMS (`TWILIO_*` variables)
    - Cloudflare DNS management (`CLOUDFLARE_*` variables)
    - Production secrets management (`VAULT_*` or `MASTER_SECRET`)
    - SimpleSAMLphp integration (`SSP_*`, `TENANT_ID`, etc.)

## 3. Cloudflare DNS and SSL Setup

This step automates the creation of DNS records and configures SSL/TLS for HTTPS access.

### What the script does:

1. **Creates DNS records** (A record + CNAMEs for each service)
2. **Configures SSL/TLS mode**:
   - Checks if zone SSL mode is already "Flexible"
   - If not, creates a Page Rule for Flexible SSL on your subdomains only
   - This allows visitors to use HTTPS while the server uses HTTP internally

### Running the script:

1.  Set your Cloudflare API token. 
    - If you don't have a token yet, [create one here](https://developers.cloudflare.com/fundamentals/api/get-started/create-token/).
    - After creating the token, go to the permissions section, category: `Zone` and add:
    - Permission: `DNS` > Access level: `Edit` (for creating Page Rules) and `Read.
    - Permission: `Page Rules` > Access level: `Edit` (for creating Page Rules).
    - Set the token as an environment variable:
    
    ```bash
    export CLOUDFLARE_API_TOKEN="your-cloudflare-api-token"
    ```

2.  Run the `setup-cloudflare.sh` script:

    ```bash
    cd ~/step/.devcontainer/remote-deployment
    chmod +x ./setup-cloudflare.sh
    ./setup-cloudflare.sh sequent.vote YOUR_SERVER_IP remote-deployment
    ```

    *   Replace `sequent.vote` with your root domain.
    *   Replace `YOUR_SERVER_IP` with your server's public IP address.
    *   Replace `remote-deployment` with your subdomain suffix (must match what you used in step 2).

### What gets created:

**DNS Records:**
*   `remote-deployment.sequent.vote` → YOUR_SERVER_IP (A record)
*   `admin-remote-deployment.sequent.vote` → remote-deployment.sequent.vote (CNAME)
*   `voting-remote-deployment.sequent.vote` → remote-deployment.sequent.vote (CNAME)
*   `hasura-remote-deployment.sequent.vote` → remote-deployment.sequent.vote (CNAME)
*   `login-remote-deployment.sequent.vote` → remote-deployment.sequent.vote (CNAME)
*   `minio-remote-deployment.sequent.vote` → remote-deployment.sequent.vote (CNAME)
*   `verifier-remote-deployment.sequent.vote` → remote-deployment.sequent.vote (CNAME)

**SSL Configuration:**
*   If zone SSL mode is already "Flexible": Nothing additional needed ✓
*   Otherwise: Creates a Page Rule for `*-remote-deployment.sequent.vote/*` with Flexible SSL

**Note:** If the script reports an error about Page Rules, see the **Troubleshooting** section below for manual configuration options.

## 4. Deployment

Finally, you can start the Docker Compose stack with the Nginx reverse proxy.

1.  Navigate to the `.devcontainer` directory:

    ```bash
    cd ~/step/.devcontainer
    ```

2.  Build and start the services using the `docker-compose-remote.yml` file:
    
    ```bash
    # Build and start everything
    docker compose -f docker-compose-remote.yml up -d --build
    ```

    **Subsequent restarts (if images already built):**
    ```bash
    docker compose -f docker-compose-remote.yml up -d
    ```

    **Notes:**
    - Building base images separately prevents Docker parallel build conflicts
    - The `--build` flag is required for first-time setup to build all images locally
    - Once images are built, you can omit `--build` for faster startups and to save disk space
    - Only use `--build` again if you've updated Dockerfiles or need to rebuild images
    - The `full` profile is enabled by default in the `.env` file via `COMPOSE_PROFILES=full`

3.  Monitor the startup process:

    ```bash
    docker compose -f docker-compose-remote.yml logs -f <application/container name | leave empty> 
    ```

    Press `Ctrl+C` to stop following the logs.

4.  Check service status:

    ```bash
    docker compose -f docker-compose-remote.yml ps
    ```

5.  Upload JWKS certificate to MinIO:

    The JWKS file must be fetched from Keycloak and uploaded to MinIO. The file in the repo is empty by design.

    ```bash
    cd ~/step
    
    # Export MinIO credentials from .env file
    export AWS_S3_ACCESS_SECRET=$(grep AWS_S3_ACCESS_SECRET .devcontainer/.env | cut -d '=' -f2)
    
    # Wait for Keycloak to be fully started (can take 1-2 minutes)
    echo "Waiting for Keycloak..."
    until docker logs keycloak 2>&1 | grep -q "Listening on"; do sleep 2; done
    echo "Keycloak is ready"
    
    # Fetch JWKS from Keycloak and upload to MinIO
    docker compose -f .devcontainer/docker-compose-remote.yml exec -T windmill \
      curl -s http://keycloak:8090/realms/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5/protocol/openid-connect/certs \
      > /tmp/certs_temp.json
    
    # Upload to MinIO public bucket
    cat /tmp/certs_temp.json | docker compose -f .devcontainer/docker-compose-remote.yml run \
      --rm --entrypoint="" -T -i configure-minio sh -c "
        mc alias set myminio http://minio:9000 minio $AWS_S3_ACCESS_SECRET > /dev/null 2>&1 && \
        cat > /tmp/certs.json && \
        mc cp --attr Cache-Control=max-age=30 /tmp/certs.json myminio/public/certs.json && \
        echo '✓ JWKS uploaded successfully'
      "
    
    rm -f /tmp/certs_temp.json
    
    # Verify upload
    docker compose -f .devcontainer/docker-compose-remote.yml run --rm --entrypoint="" -T configure-minio \
      sh -c "mc alias set myminio http://minio:9000 minio $AWS_S3_ACCESS_SECRET > /dev/null 2>&1 && \
             CONTENT=\$(mc cat myminio/public/certs.json 2>/dev/null) && \
             [ -n \"\$CONTENT\" ] && \
             echo '✓ JWKS file verified in MinIO' || echo '✗ ERROR: JWKS file is empty or invalid'"
    ```

    **Note:** This fetches JWT signing certificates from Keycloak that Hasura needs to verify authentication tokens. If you see errors, ensure Keycloak is fully started before running these commands.

Your Sequent Step environment should now be up and running! You can access the different services through their subdomains:

*   **Admin Portal:** `https://admin-remote-deployment.sequent.vote`
*   **Voting Portal:** `https://voting-remote-deployment.sequent.vote`
*   **Hasura Console:** `https://hasura-remote-deployment.sequent.vote`
*   **Keycloak:** `https://login-remote-deployment.sequent.vote`
*   **MinIO:** `https://minio-remote-deployment.sequent.vote`

> **⚠️ Security Warning:** The default credentials for the Admin Portal are set to `admin` / `admin`. **You MUST change this immediately** in a production environment: Login to the Admin Portal, go to Users and Roles → For the User name `admin` click on Actions → Change Password. 
> - **Keycloak admin console password:** If you need to login to the keycloak console, the password is stored in the `.env` file as `KEYCLOAK_ADMIN_PASSWORD`. This password was randomly generated during the deployment process and is unique to your deployment, the same for MinIO and Hasura Console.

## Troubleshooting

### Cloudflare SSL/TLS Errors (521 Web Server Is Down)

If you see **"521 Web Server Is Down"** errors when accessing your sites, this means Cloudflare cannot connect to your origin server due to SSL configuration mismatch.

**Cause:** The nginx server only accepts HTTP connections, but Cloudflare is trying to connect via HTTPS.

**Symptoms:**
- Error 521 in browser
- `curl -I https://admin-yoursubdomain.yourdomain.com` returns `HTTP/2 521`
- Services work when accessed via HTTP directly from the server

**Solution - Option 1: Set zone-wide Flexible SSL (Simplest)**

1. Go to Cloudflare Dashboard → Your Domain → SSL/TLS → Overview
2. Set SSL/TLS encryption mode to **"Flexible"**
3. Wait 1-2 minutes for changes to propagate
4. Test: `curl -I https://admin-yoursubdomain.yourdomain.com` (should return `HTTP/2 200`)

**Solution - Option 2: Create Page Rule manually**

If you want to keep other subdomains on a different SSL mode:

1. Go to Cloudflare Dashboard → Your Domain → Rules → Page Rules
2. Click "Create Page Rule"
3. URL pattern: `*-yoursubdomain.yourdomain.com/*` (replace with your actual subdomain suffix)
4. Add setting: **SSL** → **Flexible**
5. Set priority to 1 (highest)
6. Save and deploy
7. Wait 1-2 minutes for propagation

**Solution - Option 3: Re-run setup script**

If you've freed up a Page Rule slot or set zone-wide Flexible SSL:

```bash
cd ~/step/.devcontainer/remote-deployment
export CLOUDFLARE_API_TOKEN="your-token"
./setup-cloudflare.sh yourdomain.com YOUR_SERVER_IP your-subdomain-suffix
```

**Verification:**

```bash
# Should return HTTP/2 200 (not 521)
curl -I https://admin-yoursubdomain.yourdomain.com

# Check from server that nginx is responding locally
ssh user@your-server 'curl -I http://localhost:80 -H "Host: admin-yoursubdomain.yourdomain.com"'
```

### Check DNS Resolution

```bash
nslookup admin-remote-deployment.sequent.vote
```

### Check Docker Logs

```bash
# View logs for a specific service
docker logs <container-name>

# View logs for all services
docker compose -f ~/step/.devcontainer/docker-compose-remote.yml logs
```

### Check Resource Usage

```bash
# Monitor container resource usage
docker stats

# Check system resources
htop
```

### Restart Services

```bash
cd ~/step/.devcontainer
docker compose -f docker-compose-remote.yml restart
```

### Stop All Services

```bash
cd ~/step/.devcontainer
docker compose -f docker-compose-remote.yml down
```

### Clean Up Docker System (if experiencing image corruption)

If you encounter errors like `'ContainerConfig'` or `ImageNotFound`, this usually means a container is corrupted and needs to be forcefully removed:

```bash
# Find the corrupted container
docker ps -a | grep <service-name>

# Force remove the specific container by ID or name
docker rm -f <container-id-or-name>

# Recreate the container
cd ~/step/.devcontainer
docker compose -f docker-compose-remote.yml up -d
```

**If the above doesn't work, try a full cleanup:**

```bash
# Stop and remove all containers
cd ~/step/.devcontainer
docker compose -f docker-compose-remote.yml down

# Build all missing images (uses cache for existing layers)
docker compose -f docker-compose-remote.yml build

# Start everything
docker compose -f docker-compose-remote.yml up -d
```

**Alternative: Full cleanup if needed**
```bash
# Stop all services
cd ~/step/.devcontainer
docker compose -f docker-compose-remote.yml down

# Clean up dangling images and build cache (keeps built images)
docker system prune -f

# Rebuild and start
docker compose -f docker-compose-remote.yml up -d --build
```

**Note:** Use `docker system prune -f` (without `-a`) to keep your built images. Only use `docker system prune -a -f` if you need to free maximum space and don't mind rebuilding everything.

### Rebuild Specific Services

If some images were deleted but others exist, rebuild only the missing ones:

```bash
cd ~/step/.devcontainer

# Build only specific services
docker compose -f docker-compose-remote.yml build harvest windmill mock_server beat b3

# Then start all services
docker compose -f docker-compose-remote.yml up -d
```

### Updating Configuration After Code Changes

When pulling updates from git that include configuration changes:

```bash
cd ~/step
git reset --hard  # Discard local changes if any
git pull

# Reconfigure URLs (if domain/subdomain changed)
cd ~/step/.devcontainer
remote-deployment/configure-environment.sh sequent.vote remote-deployment

# Rebuild affected services (if Dockerfiles or build configs changed)
docker compose -f docker-compose-remote.yml build <service-name>

# Restart services to apply changes
docker compose -f docker-compose-remote.yml up -d
```

### Fixing Nginx Proxy Issues

If you get 502 Bad Gateway or services are unreachable:

```bash
# Check if nginx-proxy is running
docker ps | grep nginx-proxy

# If not running, start it
docker start nginx-proxy

# Or restart it via docker compose
cd ~/step/.devcontainer
docker compose -f docker-compose-remote.yml restart nginx-proxy

# Check nginx logs for errors
docker logs nginx-proxy
```

### Production Build Notes

The admin-portal and voting-portal use production builds (static files served by nginx) in the remote deployment:

- They run on internal port 8000 (not 3000/3002)
- Webpack dev server is NOT used
- Changes to frontend code require rebuilding:
  ```bash
  cd ~/step/.devcontainer
  docker compose -f docker-compose-remote.yml build admin-portal voting-portal
  docker compose -f docker-compose-remote.yml up -d admin-portal voting-portal
  ```

### Build Race Condition Errors

If you see errors like:
- `ERROR [harvest] exporting to image`
- `failed to solve: image "sequentech.local/cargo-packages:latest": already exists`
- `Image sequentech.local/cargo-packages` - `Error failed to resolve reference`

This is a Docker parallel build race condition. **Solution:**

```bash
cd ~/step/.devcontainer

# Build base images first
docker compose -f docker-compose-remote.yml build cargo-packages postgresql keycloak

# Then build everything else
docker compose -f docker-compose-remote.yml up -d --build
```

Alternatively, if the first attempt fails, simply run the build command again - the base images will now exist and the second attempt will succeed.

### Cloudflare source IP addresses list:
In case of firewall restrictions, please add the following IP addresses in your firewall:
```    
    "173.245.48.0/20",
    "103.21.244.0/22",
    "103.22.200.0/22",
    "103.31.4.0/22",
    "141.101.64.0/18",
    "108.162.192.0/18",
    "190.93.240.0/20",
    "188.114.96.0/20",
    "197.234.240.0/22",
    "198.41.128.0/17",
    "162.158.0.0/15",
    "104.16.0.0/13",
    "104.24.0.0/14",
    "172.64.0.0/13",
    "131.0.72.0/22"
```
