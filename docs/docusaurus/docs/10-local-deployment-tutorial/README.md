# Remote Test Environment Deployment Tutorial

This tutorial will guide you through setting up a Sequent Step development environment on a remote server, accessible via a public domain with a reverse proxy managing traffic.

## Prerequisites

*   A cloud server with the following specifications:
    *   **Minimum Recommended:** 8 vCPUs, 16 GB RAM, **100 GB SSD** (minimum)
    *   **Recommended Instance Types:**
        *   AWS: `c5.2xlarge` (8 vCPUs, 16 GB RAM)
        *   GCP: `n2-standard-8` (8 vCPUs, 32 GB RAM)
        *   Azure: `Standard_D8s_v3` (8 vCPUs, 32 GB RAM)
    *   OS: Ubuntu 22.04 LTS or 24.04 LTS
    *   See `.devcontainer/remote-deployment/provision-server.sh` for detailed specifications and provisioning examples.
*   A domain name managed by Cloudflare (e.g., `sequent.vote`).
*   A Cloudflare API token with DNS editing permissions.

## 1. Server Preparation

First, you need to prepare your server by installing git, Docker, and Docker Compose, and then clone the `step` repository.

1.  SSH into your remote server.

2.  Download and run the `prepare-server.sh` script:

    ```bash
    curl -fsSL https://raw.githubusercontent.com/sequentech/step/main/.devcontainer/remote-deployment/prepare-server.sh -o prepare-server.sh
    chmod +x prepare-server.sh
    ./prepare-server.sh
    ```

3.  Log out and log back in to apply the Docker group membership changes. The `step` repository is now cloned at `/home/your-user/step`.

## 2. Environment Configuration

Next, you need to configure the environment variables for the reverse proxy setup.

1.  Navigate to the cloned repository's `.devcontainer` directory:

    ```bash
    cd ~/step/.devcontainer
    ```

2.  Copy the `.env.remote-test.example` file to `.env`:

    ```bash
    cp .env.remote-test.example .env
    ```

3.  Configure the URLs using the `configure-urls.sh` script:

    ```bash
    cd ~/step
    ./.devcontainer/remote-deployment/configure-urls.sh sequent.vote remote-test
    ```
    ```

    *   Replace `sequent.vote` with your root domain.
    *   Replace `remote-test` with your chosen subdomain suffix (e.g., `qa`, `staging`, `prod`).

    This will automatically configure all service URLs in your `.env` file:
    *   `login-remote-test.sequent.vote` (Keycloak)
    *   `admin-remote-test.sequent.vote` (Admin Portal)
    *   `voting-remote-test.sequent.vote` (Voting Portal)
    *   `hasura-remote-test.sequent.vote` (Hasura)
    *   `minio-remote-test.sequent.vote` (MinIO)

4.  **(Optional)** Review the `.env` file:

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

## 3. Cloudflare DNS Setup

This step will automate the creation of the necessary DNS records in Cloudflare. It will create one primary A or CNAME record for the base subdomain (e.g., `remote-test.sequent.vote`) and then CNAME records for each service pointing to the primary record.

1.  Set your Cloudflare API token as an environment variable:

    ```bash
    export CLOUDFLARE_API_TOKEN="your-cloudflare-api-token"
    ```

2.  Run the `setup-cloudflare.sh` script:

    ```bash
    cd ~/step/.devcontainer/remote-deployment
    ./setup-cloudflare.sh sequent.vote YOUR_SERVER_IP remote-test
    ```
    ```

    *   Replace `sequent.vote` with your root domain.
    *   Replace `YOUR_SERVER_IP` with your server's public IP address (or CNAME target).
    *   Replace `remote-test` with your subdomain suffix (must match what you used in step 2).

    The script will create:
    *   `remote-test.sequent.vote` → YOUR_SERVER_IP (A record)
    *   `admin-remote-test.sequent.vote` → remote-test.sequent.vote (CNAME)
    *   `voting-remote-test.sequent.vote` → remote-test.sequent.vote (CNAME)
    *   `hasura-remote-test.sequent.vote` → remote-test.sequent.vote (CNAME)
    *   `login-remote-test.sequent.vote` → remote-test.sequent.vote (CNAME)
    *   `minio-remote-test.sequent.vote` → remote-test.sequent.vote (CNAME)

## 4. Deployment

Finally, you can start the Docker Compose stack with the Nginx reverse proxy.

1.  Navigate to the `.devcontainer` directory:

    ```bash
    cd ~/step/.devcontainer
    ```

2.  Start the services using the `docker-compose-remote.yml` file:

    **First time deployment:**
    ```bash
    docker-compose -f docker-compose-remote.yml up -d --build
    ```

    **Subsequent restarts (if images already built):**
    ```bash
    docker-compose -f docker-compose-remote.yml up -d
    ```

    **Notes:**
    - The `--build` flag is required for first-time setup to build all images locally
    - Once images are built, you can omit `--build` for faster startups and to save disk space
    - Only use `--build` again if you've updated Dockerfiles or need to rebuild images
    - The `full` profile is enabled by default in the `.env` file via `COMPOSE_PROFILES=full`

3.  Monitor the startup process:

    ```bash
    docker-compose -f docker-compose-remote.yml logs -f
    ```

    Press `Ctrl+C` to stop following the logs.

4.  Check service status:

    ```bash
    docker-compose -f docker-compose-remote.yml ps
    ```

Your Sequent Step environment should now be up and running! You can access the different services through their subdomains:

*   **Admin Portal:** `https://admin-remote-test.sequent.vote`
*   **Voting Portal:** `https://voting-remote-test.sequent.vote`
*   **Hasura Console:** `https://hasura-remote-test.sequent.vote`
*   **Keycloak:** `https://login-remote-test.sequent.vote`
*   **MinIO:** `https://minio-remote-test.sequent.vote`

## Troubleshooting

### Check DNS Resolution

```bash
nslookup admin-remote-test.sequent.vote
```

### Check Docker Logs

```bash
# View logs for a specific service
docker logs <container-name>

# View logs for all services
docker-compose -f ~/step/.devcontainer/docker-compose-remote.yml logs
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
docker-compose -f docker-compose-remote.yml restart
```

### Stop All Services

```bash
cd ~/step/.devcontainer
docker-compose -f docker-compose-remote.yml down
```

### Clean Up Docker System (if experiencing image corruption)

If you encounter errors like `'ContainerConfig'` or `ImageNotFound`, clean up and rebuild:

```bash
# Stop and remove all containers
cd ~/step/.devcontainer
docker-compose -f docker-compose-remote.yml down

# Build all missing images (uses cache for existing layers)
docker-compose -f docker-compose-remote.yml build

# Start everything
docker-compose -f docker-compose-remote.yml up -d
```

**Alternative: Full cleanup if needed**
```bash
# Stop all services
cd ~/step/.devcontainer
docker-compose -f docker-compose-remote.yml down

# Clean up dangling images and build cache (keeps built images)
docker system prune -f

# Rebuild and start
docker-compose -f docker-compose-remote.yml up -d --build
```

**Note:** Use `docker system prune -f` (without `-a`) to keep your built images. Only use `docker system prune -a -f` if you need to free maximum space and don't mind rebuilding everything.

### Rebuild Specific Services

If some images were deleted but others exist, rebuild only the missing ones:

```bash
cd ~/step/.devcontainer

# Build only specific services
docker-compose -f docker-compose-remote.yml build harvest windmill mock_server beat b3

# Then start all services
docker-compose -f docker-compose-remote.yml up -d
```
