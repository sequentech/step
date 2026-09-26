# K3s & Gitea Airgap Guide

This guide explains how to prepare, install, and manage a single-node K3s cluster and Gitea CI/CD environment in a completely offline (airgapped) environment.

---

## 1. Architecture & Design Decisions

The solution is designed to provide a secure, production-grade "Cloud-in-a-Box" experience without any internet connectivity.

```
                  +---------------------------------------+
                  |         Client (Ubuntu 26.04)         |
                  +---------------------------------------+
                                      |
                     (HTTPS / Port 443 | SSH / Port 2222)
                                      v
+---------------------------------------------------------------------------------+
|                            Server VM (Ubuntu 26.04)                             |
|                                                                                 |
|  +--------------------------- Traefik Ingress Controller --------------------+  |
|  |                                                                           |  |
|  |  +-- https://portal.local ---------------------------------------------+  |  |
|  |  |  * Port 443 (TLS)                                                   |  |  |
|  |  |  * Routes / -> voting-portal                                        |  |  |
|  |  |  * Routes /hasura -> hasura (GraphQL Engine)                        |  |  |
|  |  |  * Routes /storage -> rustfs (S3 Assets)                            |  |  |
|  |  |  * Routes /realms & /resources -> keycloak (Port 8090)              |  |  |
|  |  +---------------------------------------------------------------------+  |  |
|  |                                                                           |  |
|  |  +-- https://gitea.local ---------------------------------------------+  |  |
|  |  |  * Port 443 (TLS) / SSH Port 2222 (TCP passthrough)               |  |  |
|  |  |  * Routes / -> gitea (Port 3000)                                   |  |  |
|  |  +---------------------------------------------------------------------+  |  |
|  +---------------------------------------------------------------------------+  |
|                                                                                 |
+---------------------------------------------------------------------------------+
```

### Core Components
- **K3s (Single-Node)**: Chosen for its lightweight footprint and built-in support for airgapped environments (binary-only installation and auto-importing images).
- **Gitea**: Serves as both the **Git Source Control** and the **OCI Container Registry**. Consolidating these services reduces the architectural surface area.
- **Gitea Runner (Actions)**: Runs inside the cluster using a **Docker-in-Docker (DinD)** sidecar to build and push images locally.

### Key Architectural Decisions
- **Unified Single-Domain TLS routing**: To prevent CORS blocks, cookie-transmission blocks, and DNS-over-HTTPS (DoH) issues (where browsers bypass `/etc/hosts` for different subdomains), **the entire application has been refactored into a single host (`portal.local`)**. Keycloak's frontend endpoints (`/realms` and `/resources`), Hasura (`/hasura`), and RustFS (`/storage`) are multiplexed securely under the same TLS domain.
- **Turnkey Self-Signed Certificates**: During deployment, our management script automatically checks for TLS secrets and generates a multi-domain self-signed certificate for `portal.local` dynamically, injecting it into `step-apps` and `step-infra` namespaces.
- **Static Registry Routing**: Gitea is assigned a fixed ClusterIP (`10.43.10.10`). During installation, K3s is pre-configured to trust `gitea.gitea:3000` at this IP. This bypasses the need for host-level DNS resolution (`/etc/hosts`) and ensures the Kubelet can always pull images.
- **Declarative Automation**: Instead of complex bash loops, a Kubernetes Job (`gitea-setup`) handles runner registration. It waits for Gitea to be ready, generates a token, and saves it to a Secret. This makes the deployment self-healing and asynchronous.
- **Secure CI Rollouts**: The runner is assigned a dedicated `ServiceAccount` with RBAC permissions limited to restarting deployments in the `step-apps` namespace. This allows the CI pipeline to trigger updates without needing root access to the node.
- **Offline Image Lifecycle**:
  - **Online**: `prepare.sh` bundles every required base image into a 3.5GB tarball.
  - **Offline**: `manage.sh` copies this tarball to K3s's `/var/lib/rancher/k3s/agent/images/` folder, where it is automatically imported into the `containerd` store on startup.

---

## 2. Environment Specifications

### Operating Systems
- **Online Preparation Machine**: Ubuntu 26.04 LTS (Noble Numbat)
- **Server Machine (Cluster Node)**: Ubuntu 26.04 LTS Server (Noble Numbat)
- **Client Machine (Desktop)**: Ubuntu 26.04 LTS Desktop (Noble Numbat)

### Toolchain Requirements (Online Preparation Machine)
To successfully run `./airgap/prepare.sh` and build the offline bundle, the online prep machine must have the following tools installed:
1.  **Bash (v5+)**: Unix shell environment.
2.  **Coreutils (`uname`, `realpath`, `mkdir`, `rm`, `chmod`)**.
3.  **Curl**: Used for downloading K3s, Kubectl, and installation scripts.
4.  **Docker (CE / Community Edition, v25+)**: 
    - Must be configured to run containers without sudo (user added to `docker` group).
    - Must support emulation/multi-arch building (if preparing ARM64 artifacts from an x86_64 host).
5.  **Tar / Gzip**: For compressing the source files bundle.

---

## 3. Server Network Configuration (Ubuntu 26.04)

For the cluster to run reliably in an offline lab environment, the server must be assigned a **Static IP address**. 

### Step 1: Configure Static IP on Server
On **Ubuntu 26.04**, static IPs are configured via Netplan. Edit your Netplan configuration file (typically `/etc/netplan/50-cloud-init.yaml` or `/etc/netplan/01-netcfg.yaml`):

```bash
sudo nano /etc/netplan/50-cloud-init.yaml
```

Modify the network interface (e.g., `eth0` or `enp3s0`) to have static IP definitions. Here is an example of setting the Server IP to `192.168.1.100`:

```yaml
network:
  version: 2
  renderer: networkd
  ethernets:
    eth0:
      dhcp4: no
      addresses:
        - 192.168.1.100/24
      routes:
        - to: default
          via: 192.168.1.1
      nameservers:
        addresses:
          - 192.168.1.1
```

Apply the changes:
```bash
sudo netplan apply
```

### Step 2: Configure Hosts File on Client Machine
On the **Client Machine (Ubuntu 26.04 Desktop)**, map the static IP of your Server to the local domains. Open `/etc/hosts`:

```bash
sudo nano /etc/hosts
```

Add the following entry (replacing `192.168.1.100` with your Server's actual static IP):

```text
192.168.1.100 portal.local gitea.local
```

*Note: `keycloak.local` is no longer needed since Keycloak is multiplexed directly under `portal.local`!*

---

## 4. Online Preparation (Online Machine)

Before going to the lab, you must bundle all required artifacts.

1.  Clone the repository on an online machine running Ubuntu 26.04.
2.  Run the preparation script:
    ```bash
    ./airgap/prepare.sh
    ```
3.  This will create an `airgap-output/` directory containing:
    - K3s and Kubectl binaries.
    - Offline Debian packages (git, curl, etc.).
    - Bundled OS security-update packages (`os-security-updates/`).
    - A 3.5GB `images/step-airgap-infra.tar` containing all required base images.
    - `release/image-digests.txt` — sha256 image IDs of every bundled image.
    - `release/trivy-report.txt` — HIGH/CRITICAL vulnerability scan of the Sequent-built images.
    - `release/airgap-signing-pubkey.asc` — GPG public key used to sign the release.
    - `checksums.txt` — sha256 of every release artifact (for `sha256sum -c`).
    - `checksums.txt.asc` — detached GPG signature over `checksums.txt`.
4.  Copy the entire `airgap-output/` directory to your USB drive.

    At the end of the run, `prepare.sh` prints the **signing key fingerprint**.
    Record it and communicate it **out-of-band** (not on the same USB drive) so the
    airgap operator can confirm the bundle's authenticity on arrival.

### Release Versioning & Integrity

Every Sequent-built image is tagged with a release version (from `git describe`,
overridable via `RELEASE_VERSION=x.y.z ./airgap/prepare.sh`) alongside `:latest`.

On arrival at the airgap machine, verify the bundle **before deploying**. The
`--verify` command imports the shipped public key into a throwaway keyring,
verifies the GPG signature over `checksums.txt`, and then runs `sha256sum -c`:

```bash
cd airgap-output
# Pass the fingerprint you received out-of-band to enforce authenticity:
EXPECTED_FINGERPRINT="<fingerprint from prepare.sh>" ./manage.sh --verify
```

If `EXPECTED_FINGERPRINT` is omitted the signature and checksums are still
verified, but you must manually confirm the printed fingerprint matches the one
the builder communicated — otherwise a re-signed tampered bundle would pass.

Review `release/trivy-report.txt` for known HIGH/CRITICAL CVEs and
`release/image-digests.txt` for the exact image IDs shipped in this release.

#### Signing key

By default `prepare.sh` generates a dedicated Ed25519 signing keypair in
`.airgap-gpg/` (git-ignored, never shipped) and reuses it across runs so the
fingerprint stays stable. To sign with an existing maintained identity from your
own keyring instead, export `GPG_SIGNING_KEY_ID=<fingerprint>` before running
`prepare.sh`.

---

## 5. Server Machine Setup (Lab Machine)

### Install Cluster
1.  Plug in the USB and copy `airgap-output` to your server.
2.  Verify the bundle before installing anything (see *Release Versioning &
    Integrity* above for the fingerprint check):
    ```bash
    cd airgap-output
    EXPECTED_FINGERPRINT="<fingerprint from prepare.sh>" ./manage.sh --verify
    ```
3.  Install the airgapped K3s cluster:
    ```bash
    sudo ./manage.sh --setup-server
    ```
    *Note: This automatically configures internal registry trust for `gitea.gitea:3000` at the static IP `10.43.10.10`.*

### Deploy Infrastructure & Apps
This will load the infrastructure images and apply all Kubernetes manifests in a declarative manner.
```bash
sudo ./manage.sh --deploy
```

---

### Applying OS Security Updates (Offline)

The bundle ships the Ubuntu `-security` pocket packages captured during preparation.
To patch the offline server without internet access:

```bash
cd airgap-output
sudo ./manage.sh --update-os
```

This installs every bundled security `.deb` via `dpkg` (dependencies are resolved
from the complete bundled set). Reboot the server afterwards if a kernel package
was updated. Refresh the bundle by re-running `./airgap/prepare.sh` on the online
machine whenever new security updates are published.

---

## 6. Client Machine Setup (Ubuntu 26.04 Desktop)

1.  Plug in the USB.
2.  Install required CLI tools (git, curl, jq) from the offline bundle:
    ```bash
    cd airgap-output
    sudo ./manage.sh --setup-client
    ```

---

## 7. Development Workflow

### Initial Code Push
1.  Prepare your source code and register your SSH key with Gitea in one step:
    ```bash
    ./manage.sh --run-dev
    ```
    This extracts the source, registers your `~/.ssh/id_ed25519.pub` (or `id_rsa.pub`) with
    the Gitea admin account via the API, and prints the push instructions.
    If you don't have an SSH key yet, generate one first: `ssh-keygen -t ed25519`

2.  Push to Gitea (the repository is auto-created on first push):
    ```bash
    cd source
    git remote add origin ssh://git@gitea.local:2222/admin/step.git
    git push -u origin main
    ```
    Accept the host key fingerprint on first connect. Gitea is accessible in a browser
    at `https://gitea.local` (accept the self-signed certificate warning).

---

## 8. Service Access Map

Once deployed, access each service using the mapping below:

| Service | Protocol | Access URL | Description |
| :--- | :--- | :--- | :--- |
| **Voting Portal** | **HTTPS (Port 443)** | `https://portal.local` | The voter-facing SPA interface. |
| **Keycloak (Auth)** | **HTTPS (Port 443)** | `https://portal.local/realms/...` | Handled on the same single-domain to prevent browser cookie-blocks. |
| **Hasura Engine** | **HTTPS (Port 443)** | `https://portal.local/hasura/v1/graphql` | Public-facing GraphQL API. |
| **RustFS (S3)** | **HTTPS (Port 443)** | `https://portal.local/storage/public/` | Public asset storage bucket. |
| **Gitea (Web UI)** | **HTTPS (Port 443)** | `https://gitea.local` | Source control web interface. |
| **Gitea (SSH git)** | **SSH (Port 2222)** | `ssh://git@gitea.local:2222/<repo>.git` | Git push/clone via SSH (TCP passthrough via Traefik). |

### Internal Services (cluster-only, not externally exposed)

| Service | Namespace | Description |
| :--- | :--- | :--- |
| **Windmill** | `step-apps` | Celery-based async task worker. |
| **Beat** | `step-apps` | Celery beat scheduler (uses the windmill image). |
| **B4** | `step-apps` | Bulletin board backend (gRPC port 50051). |
| **Harvest** | `step-apps` | Election management REST API. |
| **RabbitMQ** | `step-infra` | Task queue for Celery workers (AMQP port 5672). |
| **Trustee 1** | `step-infra` | Key ceremony trustee node (braid). |
| **Trustee 2** | `step-infra` | Key ceremony trustee node (braid). |
| **ImmuDB** | `step-infra` | Tamper-evident audit log (gRPC port 3322). |
| **PostgreSQL** | `step-infra` | Primary relational database. |

### Important Note on Self-Signed Certificates:
Since `https://portal.local` uses a self-signed TLS certificate generated natively:
1.  When you first access `https://portal.local` in your browser, you will see a privacy warning.
2.  Click **Advanced** -> **Proceed to portal.local (unsafe)**.
3.  Once accepted, your browser will mark the context as **Secure**, unlocking the **Web Crypto API** natively and allowing Keycloak OIDC authentication to initialize and log in.

---
*SPDX-License-Identifier: AGPL-3.0-only*
