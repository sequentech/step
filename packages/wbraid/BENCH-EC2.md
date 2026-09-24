<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# Benchmarking on a temporary EC2 instance

`bench-ec2.sh` runs the wbraid benchmarks (`bench.sh`, and optionally an
interleaved before/after of the five targets between two commits) on an EC2
instance that exists only for the session. It solves the two problems a
developer laptop cannot: **quiescence** (a fresh instance runs nothing else) and
a **reference machine** (the same instance type is the same hardware every
session, so numbers are comparable across time). It is also the only practical
path to a GPU for the deferred GPU question in `PERFORMANCE.md`.

A session costs on the order of a dollar. The design goal that matters more
than cost is that **nothing can outlive the session** (see Guardrails).

## Reference configuration

Every results file records these, so a number is never quoted without them.

| | |
|---|---|
| Region | `eu-west-1` |
| CPU reference instance | **`c7i.8xlarge`** — 32 vCPU (16 physical cores + HT), Intel Xeon Platinum 8488C, the same AVX2 dalek backend as the laptops; ~$1.6/h. PERFORMANCE.md's Status is measured on it (`INSTANCE_TYPE=c7i.8xlarge`); the script's default for cheaper validation sessions is `c7i.4xlarge` (16 vCPU, ~$0.8/h), whose absolute numbers are not comparable with Status |
| Image | newest official Canonical **Ubuntu 24.04 amd64 gp3** AMI, resolved at launch by `describe-images` (owner `099720109477`), never hardcoded |
| Root volume | 30 GB gp3, `DeleteOnTermination` |
| Toolchain | Rust `1.96.0` (rustup, minimal profile), `build-essential`, `libssl-dev`, AWS CLI v2 |
| Access | **SSM only** — no SSH key pair, no inbound security-group rule; IMDSv2 required |
| Lifetime cap | 120 min (`LIFETIME_MIN`), 25 min for `smoke`; the full Status grid against the fork point needs `LIFETIME_MIN=240` (it ran 136 min on `c7i.8xlarge`) |

Comparability note: cloud vCPUs are hyperthreads, and the laptop's 16 logical
cores are similar — so absolute numbers differ from the laptop. **The same
instance type is not always the same speed**: on 2026-09-23 two sessions an
hour apart, same type, AZ, CPU model and kernel, differed by ~20% on every
stage (host placement; PERFORMANCE.md, Status → Resolution). Within one session
numbers agree to under 1%, so a before/after is exact only **interleaved in one
session** — which is how the differential runs — and snapshots from different
sessions compare only to that ~20% tolerance. Every session therefore keeps at
least one small snapshot cell (the default grid has `1000:2`) as its host
calibration, and a comparison that matters is re-run interleaved rather than
read across sessions. Keep the methodology of `PERFORMANCE.md` (Tooling and
method: interleaved A/B, median of reps).

## One-time account setup

Done once per AWS account, in the console. It creates one purpose-scoped
compartment: a drop-box bucket, an instance role that can reach SSM and that
bucket only, and a CLI identity whose permissions are an allow-list. The
`REGION`, `BUCKET` and role-ARN placeholders below are filled with your values;
no account id is recorded in this repository.

1. **S3 bucket** `BUCKET` in `REGION`, Block-all-public-access on, with a
   lifecycle rule expiring all objects after **1 day** (so a forgotten tarball
   or result can never accrue cost).
2. **Instance role** `wbraid-bench-instance-role` — trusted entity EC2; attach
   the AWS-managed `AmazonSSMManagedInstanceCore` (lets SSM run commands with
   no inbound network access); add this inline policy so the machine can reach
   only the drop box:

   ```json
   {
     "Version": "2012-10-17",
     "Statement": [
       { "Effect": "Allow", "Action": ["s3:GetObject", "s3:PutObject"],
         "Resource": "arn:aws:s3:::BUCKET/*" },
       { "Effect": "Allow", "Action": ["s3:ListBucket"],
         "Resource": "arn:aws:s3:::BUCKET" }
     ]
   }
   ```
   Creating the role through the EC2 use case also creates the instance
   profile of the same name, which the script passes by name at launch.
3. **CLI identity** — IAM user `wbraid-bench-cli` (no console access) with this
   customer-managed policy `WbraidBenchCli`, and one access key configured
   locally as `aws configure --profile wbraid-bench` (never committed, never
   pasted into a chat). Deactivate the key between campaigns; delete it when the
   EC2 work is over.

   ```json
   {
     "Version": "2012-10-17",
     "Statement": [
       { "Sid": "Ec2InOneRegion", "Effect": "Allow",
         "Action": ["ec2:RunInstances", "ec2:TerminateInstances", "ec2:CreateTags",
                    "ec2:DescribeInstances", "ec2:DescribeInstanceStatus", "ec2:DescribeImages",
                    "ec2:DescribeInstanceTypes", "ec2:DescribeInstanceTypeOfferings",
                    "ec2:DescribeSubnets", "ec2:DescribeVpcs", "ec2:DescribeSecurityGroups",
                    "ec2:DescribeAvailabilityZones", "ec2:DescribeVolumes"],
         "Resource": "*",
         "Condition": { "StringEquals": { "aws:RequestedRegion": "REGION" } } },
       { "Sid": "HandRoleToInstances", "Effect": "Allow",
         "Action": "iam:PassRole",
         "Resource": "arn:aws:iam::ACCOUNT_ID:role/wbraid-bench-instance-role",
         "Condition": { "StringEquals": { "iam:PassedToService": "ec2.amazonaws.com" } } },
       { "Sid": "SsmInOneRegion", "Effect": "Allow",
         "Action": ["ssm:SendCommand", "ssm:GetCommandInvocation", "ssm:ListCommandInvocations",
                    "ssm:ListCommands", "ssm:DescribeInstanceInformation"],
         "Resource": "*",
         "Condition": { "StringEquals": { "aws:RequestedRegion": "REGION" } } },
       { "Sid": "BenchBucketObjects", "Effect": "Allow",
         "Action": ["s3:GetObject", "s3:PutObject", "s3:DeleteObject"],
         "Resource": "arn:aws:s3:::BUCKET/*" },
       { "Sid": "BenchBucket", "Effect": "Allow",
         "Action": ["s3:ListBucket", "s3:GetBucketLocation"],
         "Resource": "arn:aws:s3:::BUCKET" },
       { "Sid": "ReadQuotas", "Effect": "Allow",
         "Action": ["servicequotas:GetServiceQuota", "servicequotas:ListServiceQuotas"],
         "Resource": "*" }
     ]
   }
   ```
   What is deliberately absent: any permission to create volumes, snapshots,
   Elastic IPs, key pairs or security groups — nothing that could bill quietly.
   EC2 and SSM are locked to one region; `PassRole` to the one role.
4. **Guardrails only the account owner can set**: a Budgets cost alert (e.g.
   $20/month), and the EC2 quotas — *Running On-Demand Standard instances* must
   be ≥ 16 vCPUs for `c7i.4xlarge`; *Running On-Demand G and VT instances* is 0
   on most accounts and needs a request (allow a day) before any GPU work.

Verify from the CLI (all read-only): `aws sts get-caller-identity`,
`ec2 describe-vpcs --filters Name=isDefault,Values=true` (SSM launches need only
the region's default VPC), `s3 ls s3://BUCKET`, and the two quotas via
`service-quotas get-service-quota --service-code ec2 --quota-code L-1216C47A`
(Standard) / `L-DB2E81BA` (G and VT).

## Running

From `packages/wbraid`:

```sh
./bench-ec2.sh smoke                         # first, and after any change to the script
./bench-ec2.sh session                       # benchmark HEAD
./bench-ec2.sh session HEAD 657cb05c20       # HEAD, plus interleaved before/after vs a baseline commit
CELLS="10000:2 100000:2" REPS=3 ./bench-ec2.sh session
DIFF_CELLS="100000:2 100000:5" DIFF_REPS=3 ./bench-ec2.sh session HEAD 657cb05c20
TALLY_CELLS="100000:2:3 100000:2:5" ./bench-ec2.sh session   # the global target's N:W:Q grid (default 100000:2:3 100000:5:3; "" skips)
TALLY_SER_CELLS="100000:2:3" ./bench-ec2.sh session          # tally cells run again with --ser (message encode/decode on the path)
TALLY_DIFF_CELLS="100000:2:3" ./bench-ec2.sh session HEAD 2d23452f05   # interleaved tally before/after (both commits need examples/tally.rs)
BREAKDOWN=1 BREAKDOWN_CELLS="100000:2 100000:5" ./bench-ec2.sh session # stage breakdown: bench-ec2/breakdown.patch applied to a scratch copy, built with --features profile
BASE_EXAMPLES_DIR=bench-ec2/forkpoint ./bench-ec2.sh session HEAD 657cb05c20   # before/after vs the fork point, with the frozen programs that build there
# The session PERFORMANCE.md's Status is generated from (2026-09-24, 136 min, ~$3.60):
INSTANCE_TYPE=c7i.8xlarge LIFETIME_MIN=240   CELLS="1000:2 10000:2 10000:5 100000:2 100000:5 1000000:1" REPS=3   DIFF_CELLS="100000:2 100000:5 1000000:1" DIFF_REPS=3   TALLY_CELLS="100000:2:3 100000:5:3 1000000:1:2 100000:2:3:ser" TALLY_REPS=3   TALLY_DIFF_CELLS="100000:2:3 100000:5:3 1000000:1:2 100000:2:3:ser" TALLY_DIFF_REPS=3   BASE_EXAMPLES_DIR=bench-ec2/forkpoint BREAKDOWN=1 BREAKDOWN_CELLS="100000:2 100000:5"   ./bench-ec2.sh session HEAD 657cb05c20
GUIDANCE=1 ./bench-ec2.sh session            # also run the criterion guidance benches (off by default here)
CELLS="1000:2" REPS=1 ./bench-ec2.sh session  # a minimal session: validates the rig end to end for cents
./bench-ec2.sh sweep                         # any time: proves nothing tagged is alive
```

`smoke` launches a `t3.micro`, waits for the bootstrap, prints `uname`/CPU/
toolchain versions, checks bucket access from the instance, terminates and
sweeps — about a cent, and it exercises every stage of the lifecycle including
the teardown. Run it before trusting the script with an hour-long session.

`session` does, in order: `git archive` the requested commit(s) of
`packages/wbraid` (pinning exactly what is measured) → upload to
`s3://BUCKET/<session>/` → launch → wait for the SSM agent → run
`bench-ec2/remote-bench.sh` via SSM — it builds `targets` at the tip and runs
the **snapshot grid** itself (`CELLS × REPS`, default the five cells × 3), then
the **global target** (`examples/tally.rs`, one tally's critical path for a
quorum of Q) over `TALLY_CELLS × TALLY_REPS` (default `100000:2:3
100000:5:3` × 3; skipped for a commit that predates the example), and again
with `--ser` over `TALLY_SER_CELLS` (default none); with `BREAKDOWN=1`, the
**stage breakdown**: the production code carries no timers, so the remote
fetches a scratch copy of the tip, applies `bench-ec2/breakdown.patch` (the
`timed(Category::…)` wrappers on the outer call sites, ~35 one-liners), builds
`targets --features profile` there and writes each `BREAKDOWN_CELLS` cell's
wall-clock per cost category to `profile-<sha>.txt`; a patch that no longer
applies fails the session, and is regenerated from the per-site audit in
PERFORMANCE.md (Constraints);
with `GUIDANCE=1` also the criterion guidance benches straight from cargo
(skipping any the packaged commit lacks); with a baseline, the **before/after**
(grid `DIFF_CELLS`, default `10000:2 100000:2`, `DIFF_REPS` 3; a baseline that
predates `examples/targets.rs` gets the tip's copy, which builds only while
the APIs it uses exist there — the tip's programs measure production form, so
since `009b443add` they need `strip_all`. For older baselines,
`BASE_EXAMPLES_DIR` names a directory of **frozen measurement programs** that
the remote copies into the baseline instead: `bench-ec2/forkpoint/` holds
`targets.rs` and `tally.rs` that build against the fork point `657cb05c20`
(the milestone's `targets.rs`, and the tally with per-item `strip` in place of
`strip_all`; same composition, same CSVs; never edited to track the live
examples). With both sides carrying a tally, the **tally before/after** runs
over `TALLY_DIFF_CELLS` × `TALLY_DIFF_REPS`, default `100000:2:3` × 3,
interleaved the same way into `tally-differential-<base>-vs-<sha>.csv`). The remote script owns every grid loop rather than
calling the packaged commit's `bench.sh`, so the knobs work for any commit;
`bench.sh`/`bench.ps1` remain the local tools. It uploads
`snapshot-<sha>.csv`, `tally-<sha>.csv`, `differential-<base>-vs-<sha>.csv`,
`tally-differential-<base>-vs-<sha>.csv`, `profile-<sha>.txt`,
`guidance-<sha>.txt`, `machine.txt` and the log → `collect` also renders
**`SUMMARY.md`** beside them (`bench-ec2/summarize.sh`: machine header,
snapshot medians per cell, the tally's `T` and `V` with each stage's share,
both before/afters with speedup factors, the stage breakdown, and the
snapshot-vs-"after" spread —
the same tip binary run standalone vs interleaved — as the measurement's
resolution; the guidance benches excluded by design)
and prints it, so the key results are readable without opening a CSV;
`./bench-ec2.sh summarize DIR` re-renders any results directory →
`aws s3 sync` the results to `bench-results/ec2-<session>/` (git-ignored) →
terminate → sweep. Budget ~45–60 min; the lifetime cap is 120.

Results are then written into the Status section of `PERFORMANCE.md` by hand, with the reference
configuration quoted.

## Guardrails — why nothing can be left behind

Five independent layers; any one of them alone ends the instance.

1. `--instance-initiated-shutdown-behavior terminate` plus a `shutdown -h
   +LIFETIME_MIN` as the **first line of user-data**: the instance terminates
   itself at the cap even if this driver, the network, or the laptop dies.
2. The remote script's last act after uploading results is `shutdown -h +2`.
3. The driver's `EXIT` trap terminates the instance on every exit path —
   success, failure, or Ctrl-C — and then runs `sweep`.
4. Everything is **tagged** `Project=wbraid-bench`, `Ephemeral=true` (instance
   and volume), so `sweep` can find it in any non-terminated state — including
   **stopped**, which still bills for its volume and is exactly the resource
   that goes under the radar. The end state is always *terminate*, never stop.
5. No resource that bills while idle is ever created: no Elastic IP, no extra
   volumes, no snapshots, no key pairs; the root volume is
   `DeleteOnTermination`; bucket objects expire after a day and are deleted at
   collect time anyway.

If a session is interrupted, `./bench-ec2.sh terminate --all-tagged` then
`./bench-ec2.sh sweep` restores the clean state.

## Bootstrap time vs. a baked AMI

Bootstrap (apt, AWS CLI, rustup) takes a few minutes per session and runs in
parallel with the SSM agent's registration. If sessions become frequent, the
right optimization is to bake an AMI once (a visible ~$1/month snapshot), not to
keep an instance around.

## GPU sessions (later)

Same lifecycle with `INSTANCE_TYPE=g4dn.xlarge` (T4) or `g5.xlarge` (A10G),
~$0.6–1.1/h, once the *G and VT* quota is granted. The CUDA toolkit is the
extra prerequisite (an NVIDIA/Deep Learning AMI, or a driver install step in
user-data); the candidate GPU MSM and its security posture are discussed in
`PERFORMANCE.md`, Remaining levers.

## End of a campaign

Deactivate (later delete) the `wbraid-bench-cli` access key; run `sweep` once
more; the bucket, role and policy cost nothing and may stay for the next
campaign.
