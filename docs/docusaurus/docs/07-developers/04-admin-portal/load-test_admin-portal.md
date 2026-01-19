---
id: load-test_admin-portal
title: Load Testing the Admin Portal
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


## Introduction

This tutorial will allow you to create an election with 1M voters, cast 1K votes
using a headless chrome web browser and then duplicate votes faster using the
step cli.

## Requirements

You need:
- [Kubectl installed](https://kubernetes.io/docs/tasks/tools/install-kubectl-linux/)
- A [kubeconfig file](https://kubernetes.io/docs/concepts/configuration/organize-cluster-access-kubeconfig/) that gives access to the cluster. We'll assume it's in `~/.kube/prod1-euw1-kubeconfig.yml` throughout the tutorial.

## Duplicating votes

To duplicate votes
