# Air-Gapped Monorepo Development & Deployment Guide

This folder contains everything needed to set up an offline development or server environment.

---

## Phase 1: OS Setup (Bare Metal Only)

If the airgapped machine does not have Docker or Git installed:
```bash
cd deb-packages
sudo dpkg -i *.deb
```

---

## Phase 2: Loading Images

On **both** Dev and Server machines, load the application and infrastructure images:
```bash
docker load -i step-airgap-all-images.tar
```

---

## Phase 3: Choose Your Role

### OPTION A: Dev Machine (Autonomous Building)
The Dev Machine requires the source code to build and test changes offline.

1.  **Extract Source:**
    ```bash
    tar -xzf step-source.tar.gz -C /path/to/workdir
    cd /path/to/workdir
    ```
2.  **Configure:**
    ```bash
    cp .devcontainer/.env.development .env
    ```
3.  **Start Dev Stack:**
    ```bash
    docker compose -f docker-compose.dev.yml up -d
    ```

---

### OPTION B: Server Machine (Production Run)
The Server Machine runs pre-built images and does not require the full source code.

1.  **Configure:**
    Create a `.env` file in this directory based on the project requirements.
2.  **Start Server Stack:**
    ```bash
    docker compose -f docker-compose.server.yml up -d
    ```

---

## Phase 4: Releasing Updates (Dev -> Server)

To move changes from **Dev** to **Server** while offline:

1.  **On Dev Machine:** Run `./scripts/airgap-local-release.sh`.
2.  **Transfer:** Move the resulting `step-airgap-updates.tar` to the Server machine.
3.  **On Server Machine:**
    ```bash
    docker load -i step-airgap-updates.tar
    docker compose -f docker-compose.server.yml up -d
    ```

---

## Maintenance & Tools

**Initialization Helper (Server):**
The `airgap-init` service runs automatically to sync certificates. Check logs with:
`docker logs -f step_airgap_init`

**Management Consoles:**
- Hasura: `http://localhost:8080`
- Keycloak: `http://localhost:8090`
- RustFS Console: `http://localhost:9001`
- RabbitMQ: `http://localhost:15672`
