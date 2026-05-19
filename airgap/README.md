# K3s & Gitea Airgap Guide

This guide explains how to set up your single-node K3s cluster and Gitea CI/CD environment completely offline.

---

## 1. Server Machine Setup

### Install OS & Cluster
1.  Plug in the USB.
2.  `cd airgap-output`
3.  `./manage.sh --setup-server`

### Deploy Infrastructure
This will load all images into the K3s runtime and start Gitea, Postgres, etc.
`./manage.sh --deploy`

---

## 2. Client Machine Setup (Ubuntu Desktop)

1.  Plug in the USB.
2.  `cd airgap-output`
3.  `./manage.sh --setup-client`

---

## 3. Development Workflow

### First Time: Initial Push
1.  Open `http://gitea.local` in your browser.
2.  Create an account and a new repo called **`step`**.
3.  Extract source: `./manage.sh --run-dev`
4.  Push your code:
    ```bash
    cd source
    git remote add origin http://gitea.local/youruser/step.git
    git push -u origin main
    ```

### CI/CD
Gitea Actions will automatically detect the `.gitea/workflows/` files. It will:
1.  Start a DinD container.
2.  Build your Dockerfiles using the pre-cached base images (`rust:1.90`, etc.).
3.  Push the resulting image to the Gitea Container Registry.
4.  Update the Kubernetes deployment.

---

## Management
- **Gitea:** `http://gitea.local`
- **Keycloak:** `http://keycloak.local`
- **Portal:** `http://portal.local`
