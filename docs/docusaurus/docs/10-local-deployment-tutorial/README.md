# Local Deployment Tutorial

This tutorial will guide you through setting up a local development environment for Sequent Step on a remote server. This is useful for development and testing when you need a publicly accessible instance.

## Prerequisites

*   A fresh Ubuntu server (20.04 or later recommended). You can use the `provision-server.sh` script as a starting point, which contains links to tutorials for creating a new virtual machine instance on popular cloud providers.
*   A domain name managed by Cloudflare.
*   A Cloudflare API token with DNS editing permissions.

## 1. Server Preparation

First, you need to prepare your server by installing git, Docker, and Docker Compose, and then clone the repository.

1.  SSH into your remote server.

2.  Download and run the `prepare-server.sh` script from the `step` repository:

    ```bash
    curl -fsSL https://raw.githubusercontent.com/sequentech/step/main/scripts/prepare-server.sh -o prepare-server.sh
    chmod +x prepare-server.sh
    ./prepare-server.sh
    ```

3.  Log out and log back in to apply the Docker group membership changes. The `step` repository is now cloned at `/home/your-user/step`.

## 2. Environment Configuration

Next, you need to configure the environment variables for the Docker Compose setup.

1.  Navigate to the cloned repository's `.devcontainer` directory:

    ```bash
    cd /home/your-user/step/.devcontainer
    ```
    Replace `your-user` with your username on the server.

2.  Copy the `.env.example` file to `.env`:

    ```bash
    cp .env.example .env
    ```

3.  Open the `.env` file and fill in the required secrets and any other necessary configuration values. The file is pre-configured with sensible defaults for a local setup.

## 3. URL and IP Configuration

Now, you need to replace the placeholder `localhost` values in the `.env` file with your server's public IP address or domain name.

1.  Run the `configure-urls.sh` script:

    ```bash
    cd ../scripts
    ./configure-urls.sh your-server-ip-or-domain
    ```

    Replace `your-server-ip-or-domain` with the actual public IP address or a domain/subdomain that points to your server.

## 4. Cloudflare DNS Setup

This step will automate the creation of the necessary DNS records in Cloudflare to point to your server.

1.  Make sure you have your Cloudflare API token set as an environment variable:

    ```bash
    export CLOUDFLARE_API_TOKEN="your-cloudflare-api-token"
    ```

2.  Run the `setup-cloudflare.sh` script:

    ```bash
    ./setup-cloudflare.sh your-domain.com your-server-ip-or-domain 3000,3002,8080,8090,9002
    ```

    Replace `your-domain.com` with your domain and `your-server-ip-or-domain` with your server's public IP or CNAME. This will set up DNS records for the five essential services.

## 5. Deployment

Finally, you can start the Docker Compose stack.

1.  Navigate to the `.devcontainer` directory:

    ```bash
    cd ../.devcontainer
    ```

2.  Start the services using the `docker-compose-remote.yml` file:

    ```bash
    docker-compose -f docker-compose-remote.yml up -d
    ```

Your Sequent Step development environment should now be up and running. You can access the different services through the subdomains created by the Cloudflare script.
