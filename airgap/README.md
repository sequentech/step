# K3s & Gitea Airgap Guide

This guide explains how to prepare, install, and manage a single-node K3s cluster and Gitea CI/CD environment in a completely offline (airgapped) environment.

---

## 1. Online Preparation (Online Machine)

Before going to the lab, you must bundle all required artifacts (binaries, deb packages, and container images).

1.  Clone the repository on a machine with internet access.
2.  Run the preparation script:
    ```bash
    ./airgap/prepare.sh
    ```
3.  This will create an `airgap-output/` directory containing:
    - K3s and Kubectl binaries.
    - Offline Debian packages (git, curl, etc.).
    - A 3.5GB `images/step-airgap-infra.tar` containing all required base images.
4.  Copy the entire `airgap-output/` directory to your USB drive.

---

## 2. Server Machine Setup (Lab Machine)

### Install Cluster
1.  Plug in the USB and copy `airgap-output` to the server.
2.  Install the airgapped K3s cluster:
    ```bash
    cd airgap-output
    sudo ./manage.sh --setup-server
    ```
    *Note: This automatically configures internal registry trust for `gitea.gitea:3000` at the static IP `10.43.10.10`.*

### Deploy Infrastructure & Apps
This will load the infrastructure images and apply all Kubernetes manifests in a declarative manner.
```bash
sudo ./manage.sh --deploy
```
**What happens under the hood:**
- **Static Routing**: Gitea is assigned a static IP (`10.43.10.10`) so the Kubelet can always resolve the registry.
- **Auto-Config**: Gitea starts with a pre-configured admin user (`admin/admin123`).
- **Automated Runner**: A Kubernetes Job (`gitea-setup`) waits for Gitea, generates a registration token, and stores it in a Secret. The Gitea Runner starts automatically once the Secret is available.

---

## 3. Client Machine Setup (Ubuntu Desktop)

1.  Plug in the USB.
2.  Install required CLI tools (git, kubectl, etc.) from the offline bundle:
    ```bash
    cd airgap-output
    sudo ./manage.sh --setup-client
    ```

---

## 4. Development Workflow

### Initial Code Push
1.  Log in to Gitea at `http://gitea.local` using **admin / admin123**.
2.  Create a new repository called **`step`** under the **admin** user.
3.  Prepare your source code:
    ```bash
    ./manage.sh --run-dev
    ```
4.  Push to the local registry:
    ```bash
    cd source
    git remote add origin http://gitea.local/admin/step.git
    git push -u origin main
    ```

### CI/CD Pipeline
Gitea Actions will automatically trigger on push. The pipeline:
1.  Uses a `dind` (Docker-in-Docker) runner with pre-cached base images.
2.  Builds services (Harvest, Windmill, Admin Portal) using `Dockerfile.airgap`.
3.  Pushes images to the internal registry: `gitea.gitea:3000/admin/...`.
4.  **Automatic Rollout**: Uses a secure ServiceAccount to restart deployments in the `step-apps` namespace upon success.

---

## Management & Access
- **Gitea:** `http://gitea.local` (admin/admin123)
- **Keycloak:** `http://keycloak.local`
- **Admin Portal:** `http://portal.local`

---
*SPDX-License-Identifier: AGPL-3.0-only*
