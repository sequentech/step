---
id: release-credentials
title: Release Credentials
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Release Credentials

The GitHub Actions jobs that publish images or start downstream releases and
deployments read their credentials from a GitHub environment named `release`.

## Jobs in the `release` environment

| Workflow | Jobs |
| --- | --- |
| `reusable_build_push.yml` | `build-and-push` |
| `reusable_build_push_root.yml` | `build-and-push` |
| `release.yml` | `trigger-beyond-release`, `trigger-deployer` |
| `publish_release.yml` | `trigger-beyond-release`, `trigger-deployer` |

`release.yml`, `publish_release.yml` and `pr_build.yml` call the reusable build
workflows and pass only the secrets those workflows declare.

## Secrets

| Secret | Used for |
| --- | --- |
| `PAT` | Checking out `sequentech/beyond` and dispatching the beyond release and the deployer workflows |
| `DOCKERHUB_TOKEN` | Docker Hub login during image builds |
| `AWS_ACCESS_KEY_ID_GLOBALDOT`, `AWS_SECRET_ACCESS_KEY_GLOBALDOT` | Amazon ECR login in `reusable_build_push.yml` |
| `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` | Amazon ECR login in `reusable_build_push_root.yml` |

The documentation preview and CLA workflows use the workflow's `GITHUB_TOKEN`
and need none of these secrets.

## Configuring the environment

1. In the repository settings, open **Environments** and select `release`,
   or create it if it does not exist yet.
2. Under **Deployment branches and tags**, choose **Selected branches and
   tags** and add `main`, `release/*` and the `v*` tag pattern, plus any other
   branch that runs release workflows.
3. Optionally add **Required reviewers** so that each release run waits for an
   approval.
4. Add the secrets above as environment secrets of `release`.
5. Remove the repository secrets with the same names, and any organization
   secrets with those names that this repository can access.

After step 2, the jobs above, including the build started by
`Build Docker images` (`pr_build.yml`), run only from the allowed branches and
tags. Until step 5, they keep reading the repository secrets.

## Build cache

Image builds read the `<service>:buildcache` registry cache. Only builds that
push images (`push: true`) write it.

## Workflow check

The `Workflow policy` workflow runs `.github/scripts/check_workflow_secrets.py`
on every pull request that changes `.github/`. It fails when:

- a job reads one of the secrets above outside the `release` environment;
- a workflow triggered by `pull_request`, `pull_request_target` or
  `issue_comment` reads one of them;
- a job passes secrets to a reusable workflow with `secrets: inherit`.

Run it locally with:

```bash
python3 .github/scripts/check_workflow_secrets.py .github/workflows
python3 -m unittest discover -s .github/scripts -p 'test_*.py'
```
