---
id: load_testing
title: Load Testing
---

## Introduction

This tutorial will allow you to create an election with 1M voters, cast 1K votes
using a headless chrome web browser and then duplicate votes faster using the
step cli.

## Requirements

You need:
- Basic knowledge of command line terminal usage.
- [Kubectl installed][kubectl].
- A [kubeconfig file][kubeconfig] that gives access to the cluster. We'll assume
  it's in `~/.kube/prod1-euw1-kubeconfig.yml` throughout the tutorial.

## Creating an election

## Cast votes using chromium headless

## Duplicating votes

### 1. Access and Configuration

First we will set the path to the kubeconfig file so that we can use it for all
our `kubectl` plugins:

```bash
export KUBECONFIG=~/.kube/prod1-euw1-kubeconfig.yml
```

Let's review the loadtesting pod using the following command:

```bash
kubectl get pods -n test-apps -l app.kubernetes.io/name=loadtesting
```

The output should looks something like:

```bash
NAME                           READY   STATUS    RESTARTS   AGE
loadtesting-86c5944494-j7gnq   1/1     Running   0          4d21h
```

Please note that we are filtering for pods in `test-apps` namespace. Change this
accordingly to the name of your environment. For example, the `ehu` environment
would require to use here the `ehu-apps`.

We can connect to any of these loadtesting pods using the following kind of
command. Please change the pod name and the namespace name accordingly:

```bash
$ kubectl exec -it deployment/loadtesting -n test-apps -- /entrypoint.sh --help
Usage: /entrypoint.sh <subcommand> [options]
Subcommands:
  load-tool    Run the load-tool tool
  vote-cast    Run vote casting load tests [--voting-url <value>]
  shell        Start an interactive shell
  sleep        Sleeps for an infinite amount of time
  step-cli     Run step-cli

For vote-cast subcommand:
  --voting-url <value>    Voting URL (falls back to $LOADTESTING_VOTING_URL if not provided)
```

With this command, you can check what load testing actions and commands are available.

### 2. Executing the `load-tool` script to duplicate votes

We can check what `load-tool` options are available by running the entrypoint:

```bash
$ kubectl exec -it deployment/loadtesting -n test-apps -- /entrypoint.sh load-tool --help
usage: load_tool.py [-h] [--working-directory WORKING_DIRECTORY] {generate-voters,duplicate-votes,generate-applications,generate-activity-logs} ...

Load Testing Tool

positional arguments:
  {generate-voters,duplicate-votes,generate-applications,generate-activity-logs}
                        Action to perform
    generate-voters     Generate random voters CSV file
    duplicate-votes     Duplicate cast votes in the database
    generate-applications
                        Generate applications in different states
    generate-activity-logs
                        Generate activity logs

options:
  -h, --help            show this help message and exit
  --working-directory WORKING_DIRECTORY
                        Path to working directory (input/output directory)
```

At this stage we are assuming we have:
1. The election event created.
2. The Keys ceremony has been executed.
3. The election event has been published.
4. The eligible voters have been loaded and there's enough voters to add more
   votes.
5. The voting period is open, so votes can be cast.
6. There's at least one vote cast.
7. The election allows revoting, because the votes are added randomly and
   otherwise in some cases more than 1 vote might be added for a single voter.

Given the above, we can just duplicate votes with a command like below, please
change the election event id accordingly:

```bash
$ kubectl exec -it deployment/loadtesting -n test-apps -- /entrypoint.sh \
    load-tool duplicate-votes \
	--num-votes 10 \
	--election-event-id 7d7f840a-4e75-4ba4-b431-633196da1a2c
```

The election event id can be found in the admin portal in the URL of the
election event.

Please find below a short video that shows how we:
1. Enter the Dashboard of the Election Event, which currently has 400K voters
   and 12 votes cast today and 566 votes in total.
2. Copy the election event id from the Admin Portal URL.
3. Execute the `duplicate-votes` subcommand adding 10 votes.
4. Show in the Dashboard that 10 votes have been added, having now 22 votes
   cast today and 576 in total.

<video controls width="600">
  <source src="./assets/duplicate_votes_usage.mp4" type="video/mp4" />
  Your browser does not support the video tag.
</video>

[kubectl]: https://kubernetes.io/docs/tasks/tools/install-kubectl-linux/
[kubeconfig]: https://kubernetes.io/docs/concepts/configuration/organize-cluster-access-kubeconfig/

### 2. Executing the `vote-cast` command to perform vote loading tests

```bash
$ kubectl exec -it deployment/loadtesting -n test-apps -- /entrypoint.sh \
    vote-cast \
	--voting-url https://voting-test.sequent.vote/tenant/90505c8a-23a9-4cdf-a26b-4e19f6a097d5/event/7d7f840a-4e75-4ba4-b431-633196da1a2c/login
```

### 3. Executing the `tally` load test

You can check what options are available to you by calling the `step-cli` CLI tool:

```bash
$ kubectl exec -it deployment/loadtesting -n test-apps -- /entrypoint.sh step-cli step --help
Usage: step-cli step <COMMAND>

Commands:
  config                        Create a config file
  create-election-event         Create a new election event
  create-election               Create a new election
  create-contest                Create a new contest
  create-candidate              Create a new candidate
  create-area                   Create a new area
  create-area-contest           Create area contest
  create-voter                  Create a new voter
  export-cast-votes             Export casted a vote
  update-voter                  Edit a voter
  update-election-event-status  Update election event status
  update-election-status        Update election status
  import-election               Import Election Event
  publish                       Publish election event ballot changes
  refresh-token                 Refresh auth jwt
  start-key-ceremony            Start Key Ceremony
  complete-key-ceremony         Complete Key Ceremony
  start-tally                   Start Tally Ceremony
  update-tally                  Update tally status
  confirm-key-tally             Confirm trustee key for tally ceremony
  render-template               Render a handlebars-rs template with variables
  generate-voters
  duplicate-votes
  create-applications
  create-electoral-logs
  hash-password                 Process a CSV file to hash passwords and generate salts
  help                          Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```
