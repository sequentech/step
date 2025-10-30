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
  --voting-url <value>    Voting URL (falls back to $VOTING_URL or $LOADTESTING_VOTING_URL if not provided)
  Other options are forwarded to /run_bg_voting.sh (e.g. --batches, --instances, --save-screenshots, --env chrome)
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

#### Environment variables required for duplicate votes

The duplicate-votes action connects to two PostgreSQL databases (Keycloak and Hasura). Connection parameters are read from environment variables inside the loadtesting container. Ensure these are set in the Deployment (typically via Secrets/ConfigMaps) for the loadtesting pod:

- Keycloak DB
  - KEYCLOAK_DB__DBNAME
  - KEYCLOAK_DB__USER
  - KEYCLOAK_DB__PASSWORD
  - KEYCLOAK_DB__HOST
  - KEYCLOAK_DB__PORT

- Hasura DB
  - HASURA_DB__DBNAME
  - HASURA_DB__USER
  - HASURA_DB__PASSWORD
  - HASURA_DB__HOST
  - HASURA_DB__PORT

You can quickly verify that these variables are present in the running pod:

```bash
kubectl exec -it deployment/loadtesting -n test-apps -- env | grep -E '^(KEYCLOAK_DB__|HASURA_DB__)'
```

Note: If any are missing or incorrect, update the loadtesting Deployment (or its referenced Secret/ConfigMap) and redeploy so the pod picks them up.

#### duplicate-votes arguments

The duplicate-votes subcommand accepts the following arguments:

- --num-votes `<int>` (required)
  - Number of votes to insert by duplicating existing votes.
- --election-event-id `<uuid>` (required)
  - The Election Event ID the votes belong to.
- --election-id `<uuid>` (optional)
  - If omitted, the tool discovers an election_id with at least one existing vote in the event and uses that.
- --tenant-id `<uuid>` (optional; default: 90505c8a-23a9-4cdf-a26b-4e19f6a097d5)
  - Used to build the Keycloak realm name as `tenant-{tenant_id}-event-{election_event_id}` for querying eligible voters.

Operational notes:
- The tool will:
  1) Find an existing vote for the given election event (and election if specified) to use as a base template.
  2) Determine the area_id and election_id (if not passed).
  3) Fetch up to --num-votes random eligible voter IDs from Keycloak for that area.
  4) Duplicate existing cast_vote rows, reassigning voter_id_string to the fetched users, and bulk-insert via COPY for speed.
- The election should allow revoting to avoid collisions, since random users may already have cast a vote.
- There must already be at least one cast vote in the target election/area to serve as the duplication template.

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

## Cast votes using chromium headless
### 1. Executing the `vote-cast` command to perform vote loading tests

The `vote-cast` subcommand drives a Nightwatch-based browser test inside the loadtesting pod to cast votes through the public voting UI.

How it works (pipeline):
- `/entrypoint.sh vote-cast` forwards arguments to `/run_bg_voting.sh`.
- `/run_bg_voting.sh` orchestrates parallel Nightwatch runs using the base test at `/nightwatch/src/voting.js` by default.
- Nightwatch runs headlessly by default (env `default`); you can switch to non-headless with `--env chrome`.

Key flags and environment variables:
- `--voting-url <URL>`
  - The voting login URL. If omitted, the script uses `$VOTING_URL` or `$LOADTESTING_VOTING_URL` if set in the pod.
  - Always quote the URL.
- `--batches <N>`
  - Total iterations each Nightwatch instance will perform. This maps 1:1 to `NUMBER_OF_ITERATIONS` consumed by `nightwatch/src/voting.js`.
- `--instances <N>`
  - Parallelism. The orchestrator duplicates the base test into N files and runs them concurrently using Nightwatch workers.
- `--save-screenshots <true|false>` (default: `false`)
  - When `true`, screenshots are saved during the flow.
- `--number-of-voters <N>` (default: `4096`)
  - Used by the test to randomize test users.
- `--voter-min-index <N>` (default: `1`)
  - The ids for the voters will be selected between `voter-min-index` and `voter-min-index + number-of-voters - 1`.
- `--username-pattern <pattern>` (default: `user{n}`)
- `--password-pattern <pattern>` (default: `user{n}`)
  - `{n}` is replaced by the randomized user index.
- `--env <default|chrome>` (default: `default`)
  - Nightwatch environment. `default` runs Chrome headless; `chrome` is non-headless (not recommended in pods).
- Advanced:
  - `--base-test <relative_path>` (default: `nightwatch/src/voting.js`)
    - Allows running another test file, e.g., `nightwatch/src/voting2.js`.
  - `--keep-parallel-files`
    - Keeps the generated duplicate test files under `/nightwatch/src/_parallel_<RUN_ID>`.

Outputs and logs:
- Aggregated Nightwatch log: `/logs/nightwatch_<timestamp>.log` (inside the container).
- Screenshots (when enabled): `/nightwatch/screenshots` in the container.
- Temporary per-run test copies: `/nightwatch/src/_parallel_<timestamp>_PID` (removed by default unless `--keep-parallel-files`).

Examples:
- Single instance, single iteration (sanity check):
```bash
kubectl exec -it deployment/loadtesting -n test-apps -- /entrypoint.sh \
  vote-cast \
  --voting-url "https://voting-test.sequent.vote/tenant/90505c8a-23a9-4cdf-a26b-4e19f6a097d5/event/7d7f840a-4e75-4ba4-b431-633196da1a2c/login" \
  --batches 1 \
  --instances 1
```

- 8 instances in parallel, 200 iterations each:
```bash
kubectl exec -it deployment/loadtesting -n test-apps -- /entrypoint.sh \
  vote-cast \
  --voting-url "https://voting-test.sequent.vote/tenant/90505c8a-23a9-4cdf-a26b-4e19f6a097d5/event/7d7f840a-4e75-4ba4-b431-633196da1a2c/login" \
  --batches 200 \
  --instances 8 \
  --save-screenshots false
```

- Debugging with non-headless Chrome (use sparingly; pods may not support it):
```bash
kubectl exec -it deployment/loadtesting -n test-apps -- /entrypoint.sh \
  vote-cast \
  --voting-url "https://voting-test.sequent.vote/tenant/90505c8a-23a9-4cdf-a26b-4e19f6a097d5/event/7d7f840a-4e75-4ba4-b431-633196da1a2c/login" \
  --batches 1 \
  --instances 1 \
  --env chrome \
  --save-screenshots true
```

Troubleshooting:
- If your run prints defaults (e.g., `INSTANCES: 4` or `ITERATIONS: 10`) despite passing flags, ensure the flags follow `vote-cast` and that the URL is quoted.
- If the site markup requires different selectors, consider providing an alternative base test via `--base-test nightwatch/src/voting2.js`.
- To keep the generated parallel files for inspection, add `--keep-parallel-files` and check `/nightwatch/src/_parallel_*`.

## Managing an election event through the `step-cli`

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
  export-cast-votes             Export a cast vote
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
