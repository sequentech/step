# Remote Test Environment Deployment Tutorial

This tutorial will guide you through setting up a Sequent Step development environment on a remote server, accessible via a public domain with a reverse proxy managing traffic.

## Prerequisites

*   A fresh Ubuntu server (20.04 or later recommended). You can use the `provision-server.sh` script as a starting point, which contains links to tutorials for creating a new virtual machine instance on popular cloud providers.
*   A domain name managed by Cloudflare (e.g., `sequent.vote`).
*   A Cloudflare API token with DNS editing permissions.

## 1. Server Preparation

First, you need to prepare your server by installing git, Docker, and Docker Compose, and then clone the `step` repository.

1.  SSH into your remote server.

2.  Download and run the `prepare-server.sh` script:

    ```bash
    curl -fsSL https://raw.githubusercontent.com/sequentech/step/main/scripts/prepare-server.sh -o prepare-server.sh
    chmod +x prepare-server.sh
    ./prepare-server.sh
    ```

3.  Log out and log back in to apply the Docker group membership changes. The `step` repository is now cloned at `/home/your-user/step`.

## 2. Environment Configuration

Next, you need to configure the environment variables for the reverse proxy setup.

1.  Navigate to the cloned repository's `.devcontainer` directory:

    ```bash
    cd /home/your-user/step/.devcontainer
    ```
    *(Replace `your-user` with your username on the server.)*

2.  Copy the `.env.remote-test.example` file to `.env`:

    ```bash
    cp .env.remote-test.example .env
    ```

3.  Open the `.env` file and fill in your root domain and any required secrets. The file is pre-configured for the reverse proxy setup.

## 3. Cloudflare DNS Setup

This step will automate the creation of the necessary DNS records in Cloudflare. It will create one primary A or CNAME record for `remote-test.your-domain.com` and then CNAME records for each service (e.g., `admin-remote-test.your-domain.com`) pointing to the primary record.

1.  Set your Cloudflare API token as an environment variable:

    ```bash
    export CLOUDFLARE_API_TOKEN="your-cloudflare-api-token"
    ```

2.  Run the `setup-cloudflare.sh` script:

    ```bash
    cd ../scripts
    ./setup-cloudflare.sh your-domain.com your-server-ip-or-cname
    ```

    *   Replace `your-domain.com` with your root domain (e.g., `sequent.vote`).
    *   Replace `your-server-ip-or-cname` with your server's public IP address or an existing CNAME.

## 4. Deployment

Finally, you can start the Docker Compose stack with the Nginx reverse proxy.

1.  Navigate to the `.devcontainer` directory:

    ```bash
    cd ../.devcontainer
    ```

2.  Start the services using the `docker-compose-remote.yml` file:

    ```bash
    docker-compose -f docker-compose-remote.yml up -d
    ```

Your Sequent Step development environment should now be up and running. You can access the different services through their subdomains:

*   **Admin Portal:** `https://admin-remote-test.your-domain.com`
*   **Voting Portal:** `https://voting-remote-test.your-domain.com`
*   **Hasura Console:** `https://hasura-remote-test.your-domain.com`
*   **Keycloak:** `https://login-remote-test.your-domain.com`
*   **Minio:** `https://minio-remote-test.your-domain.com`
