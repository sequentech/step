---
title: Deploy load workers
sidebar_position: 8
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

The same prepared workload runs as local processes, Docker containers or indexed Kubernetes pods. Configure target URLs before preparation. Worker count controls independent processes; `workload.concurrency` controls simultaneous voters within each process.

## Remote deployment

Authenticate `step-cli step config` against the synthetic tenant in your target deployment, then initialize its workload:

```bash
step-cli load init \
  --target remote \
  --portal-url https://vote.example.org \
  --storage-origin https://ballots.example.org \
  --output remote.yaml
```

Use your deployment's portal and public S3/CDN addresses in place of the example domains. Keycloak and GraphQL addresses come from the authenticated CLI configuration. For multiple storage hosts, list each in `target.storage_origins`. These hosts must be reachable from every worker. Signed URLs are never edited.

Remote initialization selects `target.upload_mode: direct`. The `local` mode is only for devcontainer upload routing. The deployment's configured portal URL must agree with `target.portal_url`, because it determines Keycloak redirects. Install any private CA in the coordinator and worker trust stores.

```bash
read -rs -p 'Shared synthetic voter password: ' LOAD_PASSWORD
export LOAD_PASSWORD
step-cli load check remote.yaml
```

Choose **one** execution method below, finish its configuration, then prepare and run. For local processes, use the quickstart commands with `remote.yaml`. Start with a small cohort; increasing generator capacity does not establish server capacity.

## Docker

For a remote target, set the image and Docker network in `remote.yaml` before preparation:

```yaml group="engine" tab="k6"
execution:
  image: voting-load:k6
  network: bridge
```

```yaml group="engine" tab="Chromium"
execution:
  image: voting-load:chromium
  network: bridge
```

Build the selected worker image, then prepare and run:

```bash group="engine" tab="k6"
step-cli load image \
  --engine k6 \
  --tag voting-load:k6
step-cli load prepare remote.yaml \
  --engine k6 \
  --output runs/remote
step-cli load run runs/remote \
  --executor docker \
  --workers 4
```

```bash group="engine" tab="Chromium"
step-cli load image \
  --engine chromium \
  --tag voting-load:chromium
step-cli load prepare remote.yaml \
  --engine chromium \
  --output runs/remote
step-cli load run runs/remote \
  --executor docker \
  --workers 4
```

Local initialization selects the coordinator's network so a loopback-only portal remains reachable. In the repository devcontainer this is `container:<devcontainer hostname>`; on a Linux host it is `host`. For older configurations targeting a portal on the named devcontainer, use:

```yaml
execution:
  network: container:devcontainer
```

The images contain a standalone Rust worker and the selected engine; the coordinator mounts prepared inputs and passes only the synthetic password. The CLI translates devcontainer bind mounts to daemon-host paths automatically. For unusual remote-daemon layouts, set `execution.docker_mount_source` to the host path of the prepared `inputs` directory.

## Kubernetes

Use your current kubectl context, with permission to create Jobs, Pods, Secrets and PVCs in the configured namespace. Your cluster needs a ReadWriteMany storage class. Configure the image, namespace, storage class and volume size before preparation:

```yaml group="engine" tab="k6"
execution:
  executor: kubernetes
  workers: 20
  image: registry.example.org/team/voting-load:k6
  namespace: load-testing
  storage_class: shared-storage
  storage_size: 20Gi
  wait_timeout: 1h
```

```yaml group="engine" tab="Chromium"
execution:
  executor: kubernetes
  workers: 20
  image: registry.example.org/team/voting-load:chromium
  namespace: load-testing
  storage_class: shared-storage
  storage_size: 20Gi
  wait_timeout: 1h
```

Create the namespace if it does not already exist:

```bash
kubectl create namespace load-testing
```

Build and publish the image using your Docker registry credentials:

```bash group="engine" tab="k6"
step-cli load image \
  --engine k6 \
  --tag registry.example.org/team/voting-load:k6 \
  --push
step-cli load prepare remote.yaml \
  --engine k6 \
  --output runs/remote
step-cli load run runs/remote \
  --executor kubernetes \
  --workers 20
```

```bash group="engine" tab="Chromium"
step-cli load image \
  --engine chromium \
  --tag registry.example.org/team/voting-load:chromium \
  --push
step-cli load prepare remote.yaml \
  --engine chromium \
  --output runs/remote
step-cli load run runs/remote \
  --executor kubernetes \
  --workers 20
```

The CLI creates a Secret and PVC, transfers prepared inputs, starts the indexed Job, and collects worker results. It prints resource names and retains the Job, PVC and Secret for reconciliation. After collecting and reviewing results, use the resource name printed by the CLI:

```bash
read -r -p 'Run resource name printed by the CLI: ' LOAD_RESOURCE
kubectl delete job,pod,pvc,secret "$LOAD_RESOURCE" \
  --namespace load-testing \
  --ignore-not-found
```

Use your configured namespace in place of `load-testing`. Failed or interrupted jobs are not automatically retried. Keep worker clocks synchronized; the combined measured interval uses timestamps from all workers.

## Existing events and preparation

To reuse an already provisioned event, set `preparation.existing_event` to a previous run's `inputs/config.json`, and choose an unused `workload.start` range. The CLI imports the new census and prepares fresh ballots; it does not republish that event. The existing event must still be open and eligible.

Custom fixtures use `preparation.template`; explicit ballot selections use `preparation.choices`. Paths resolve relative to the workload YAML. Deployments that run S3 publication preparation separately can set `preparation.publication_preparer` to their application writer executable; its database and S3 environment must be configured on the coordinator.
