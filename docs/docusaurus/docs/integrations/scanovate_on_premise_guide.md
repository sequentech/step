---
id: scanovate_on_premise_guide
title: Scanovate On-Premise Services
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Scanovate On-Premise Services

## Overview

The [Scanovate Identity Verification](scanovate_identity_verification_guide.md)
authenticator only uses Scanovate services that run on our own infrastructure.
Nothing is sent to any third party:

- **OCR** scans the voter's identity document in its own capture page, reads
  its fields and validates its machine readable zone (MRZ).
- **Liveness Plus** checks, in its own capture page, that a real, live person
  is in front of the camera. It runs Presentation Attack Detection (PAD,
  including deepfake detection) and Injection Attack Detection (IAD) on the
  frames of that page, and posts the verdict to Keycloak.
- **Document Liveness** checks that the photo OCR read the document from shows
  a physical document: not one shown on a screen (screen replay), a printed
  copy, a document whose portrait was replaced, or a digitally edited image.
  Keycloak sends it that photo, server to server.
- **Face Match** compares two face images (1:1) and returns their similarity.
  Keycloak uses it to compare the portrait OCR read from the document with the
  live face.

We integrate them as Scanovate's own on-premise docker-compose kit, its
*On-Prem Full Onboarding Flow – Integration Guide (with Document Liveness)* and
its *Liveness Plus Service: On-Prem Deployment, Configuration, Logic &
Integration Guide* show: Keycloak's capture page embeds, in iframes on
Keycloak's origin, first OCR's capture page and then Liveness Plus's. Each
service checks Keycloak's one-time token before it starts a session and posts
its result to Keycloak, server to server. Keycloak then calls Document Liveness
and Face Match. Our capture page
never carries a verdict; the only photo it takes itself is the voter holding
their document, after the face check, taken automatically once the image has
been still for the holding time, which it uploads to Keycloak (the
authenticator's `holding-photo` policy and `video-seconds`).

This page describes how to run the services, how to configure them, and the
contract between them, the capture page and Keycloak.

| Service | Image | Role |
| --- | --- | --- |
| `scanovate-ocr` | `133529410358.dkr.ecr.eu-west-1.amazonaws.com/scanovate/ocr:version_3.9.0_aefa131_285` | OCR capture page and its API. Manages the sessions, reads the document, sends the callbacks. |
| `scanovate-liveness` | `133529410358.dkr.ecr.eu-west-1.amazonaws.com/scanovate/liveness-plus-service:version_3.9.0_e0dc72e_115` | Liveness capture page and its API. Manages the sessions, checks the face quality of each frame, calls PAD and IAD, sends the callbacks. |
| `scanovate-presentation-detection` | `133529410358.dkr.ecr.eu-west-1.amazonaws.com/scanovate/liveness-presentation-detection-service:release_1.52.0` | PAD server, with the `pad-r-2` (presentation attacks) and `dfd-3` (deepfakes) pipelines. |
| `scanovate-injection-detection` | `133529410358.dkr.ecr.eu-west-1.amazonaws.com/scanovate/liveness-injection-detection-service:release-2.6.2` | IAD server: checks that the frames come from a real camera. |
| `scanovate-face-match` | `133529410358.dkr.ecr.eu-west-1.amazonaws.com/scanovate/ngfacematch:version_3.9.0_ba58397_79` | Face Match API. We use its 1:1 API (`/facematch11`). |
| `scanovate-document-liveness` | `133529410358.dkr.ecr.eu-west-1.amazonaws.com/scanovate/document-liveness-service:release-3.5.5` | Document Liveness API (ID R&D's IDLive Doc Server 3.5.5), with the `default-sr` (screen replay), `default-pc` (printed copy), `default-ps` (portrait substitution) and `default-dm` (digital manipulation) pipelines. |

The Face Match image also has a 1:N API, which we don't use, see
[Face Match](#face-match).

### What runs where

| Check | Where it runs |
| --- | --- |
| Reading the document (name, date of birth, number, nationality, expiry) and its MRZ check digits | Our network: OCR, for passports. Other documents would need a Regula Document Reader server, see [Document types](scanovate_identity_verification_guide.md#document-types). |
| Liveness: presentation attacks and deepfakes | Our network: Liveness Plus and PAD. |
| Liveness: injection attacks (virtual cameras, emulators, tampered browsers) | Our network: Liveness Plus and IAD. |
| The voter's live face against the portrait on the document (1:1) | Our network: Face Match, called by Keycloak. |
| Document liveness: a document shown on a screen, a printed copy, a replaced portrait or a digitally edited image | Our network: Document Liveness, called by Keycloak with the photo OCR read the document from. |
| Document authenticity beyond that (security features, the chip) | Not checked: OCR's `template_matching_valid` and `document_in_frame_valid` are always `true` for passports, see [OCR result](#ocr-result). |

The services only call each other and Keycloak, and the capture pages load
their scripts, styles and fonts from the services themselves, through
Keycloak's origin. See [Air-gapped networks](#air-gapped-networks).

### Order of the checks

```mermaid
sequenceDiagram
    participant V as Capture page
    participant K as Keycloak
    participant O as OCR
    participant L as Liveness Plus
    participant D as Document Liveness
    participant F as Face Match
    K->>V: capture page (URLs of the OCR and Liveness Plus pages, each with a one-time token and the case id)
    V->>O: iframe /biometric/ocr/?ocr_type=passport&case_id=...&token=...
    O->>K: GET ocr/verify (X-token, case-id)
    O->>K: POST ocr/callback (start)
    O-->>V: the voter scans the document; postMessage done
    O->>K: POST ocr/callback (result: fields, checks, portrait, document photo)
    V->>L: iframe /biometric/liveness/?case_id=...&token=...
    L->>K: GET liveness/verify (X-token, case-id)
    L->>K: POST liveness/callback (start)
    L-->>V: the voter follows the face guidance; postMessage done
    L->>K: POST liveness/callback (result: verdicts, live face)
    V->>K: POST login actions URL (action=capture)
    K->>K: wait for both results, reject unless they passed
    K->>D: check_liveness (document photo)
    K->>F: compare_images (OCR portrait, live face)
    K->>K: validate the rules on what OCR read, store the attributes
```

1. Keycloak renders the capture page with a new case id, and a one-time token
   for each service. The case id is also the case id of both services'
   sessions.
2. The page embeds OCR's capture page. OCR checks the token with Keycloak,
   guides the voter through the scan of their document, and posts the result
   to Keycloak: the fields read, its checks, the portrait cropped from the
   document and the camera frame it read them from (`input_image`). The page
   only learns that OCR is done (`postMessage`).
3. The page then embeds Liveness Plus's capture page, which checks its token
   the same way, guides the voter's face, runs PAD and IAD on its frames and
   posts the verdict and the live face to Keycloak.
4. Once both pages are done, the page posts the capture to Keycloak.
5. Keycloak waits for both results (`liveness-result-wait-seconds`), and
   rejects the attempt unless OCR read the document and found its portrait and
   frame.
6. Keycloak sends the frame to Document Liveness, and rejects the attempt
   unless every pipeline checked it and found a physical document.
7. Keycloak rejects the attempt unless the liveness checks passed.
8. Keycloak asks Face Match whether the live face is the one on the document's
   portrait, and, with the holding photo, whether the voter holding the
   document is the live face.
9. Only then, Keycloak validates what OCR read with the realm rules and stores
   the voter's data, which later steps compare with the voter registry.

## OCR capture resolution

OCR requests a 1920 × 1080 camera stream. Its `input_image` is also used for
Document Liveness, whose [image requirements](https://docs.idrnd.net/idlivedoc/)
specify Full HD as the minimum. Keep the development OCR video configuration,
the air-gap `scanovate-theme.yaml` copy and hosted capture configuration in sync.
This is a capture-page setting, not an election Excel or realm threshold setting.

The pinned vendor client passes width and height as numeric `getUserMedia`
preferences, not mandatory minimum constraints. A browser may negotiate a lower
resolution on limited hardware. It sends JPEG frames at the actual video-track
settings dimensions, without upscaling; increasing the configured request does
not prove every camera produces Full HD. Verify the actual frame dimensions on
supported mobile and desktop cameras before rollout, including limited cameras
and permission denial. Prefer a supported rear-facing camera for documents.
Keep document-liveness checks and thresholds unchanged; do not bypass failed
checks to accommodate a lower-resolution camera. Video compression settings are
separate from the still-image capture and are unchanged.

## Access to the images

We run the images from our own mirror on AWS ECR, in the `133529410358` account
(`eu-west-1`), with the same tags and digests as Scanovate's private images on
Docker Hub:

| Repository | Tag |
| --- | --- |
| `133529410358.dkr.ecr.eu-west-1.amazonaws.com/scanovate/liveness-plus-service` | `version_3.9.0_e0dc72e_115` |
| `133529410358.dkr.ecr.eu-west-1.amazonaws.com/scanovate/liveness-presentation-detection-service` | `release_1.52.0` |
| `133529410358.dkr.ecr.eu-west-1.amazonaws.com/scanovate/liveness-injection-detection-service` | `release-2.6.2` |
| `133529410358.dkr.ecr.eu-west-1.amazonaws.com/scanovate/ngfacematch` | `version_3.9.0_ba58397_79` |
| `133529410358.dkr.ecr.eu-west-1.amazonaws.com/scanovate/ocr` | `version_3.9.0_aefa131_285` |
| `133529410358.dkr.ecr.eu-west-1.amazonaws.com/scanovate/document-liveness-service` | `release-3.5.5` (`sha256:5ed5717a1c6f60b12f01fbe3aa714ae857113639d2cf133a142ac4ed7e75c839`, `linux/amd64`) |

Log in to the registry in the Docker client that pulls the images: your host,
since the dev container talks to the host's Docker daemon, or each server that
runs them. With an AWS SSO profile for that account (e.g. `sequent-ecr`, role
`ECRPushPull`):

```bash
aws sso login --profile sequent-ecr
aws ecr get-login-password --profile sequent-ecr --region eu-west-1 \
  | docker login --username AWS --password-stdin 133529410358.dkr.ecr.eu-west-1.amazonaws.com
```

ECR login tokens expire after 12 hours: log in again before pulling a new tag.
Without a valid login, pulling fails with `no basic auth credentials` or
`authorization token has expired`. Check the access without downloading
anything:

```bash
docker manifest inspect 133529410358.dkr.ecr.eu-west-1.amazonaws.com/scanovate/liveness-plus-service:version_3.9.0_e0dc72e_115
```

Some rules:

- Mirror a new version by pulling it from Scanovate's Docker Hub repositories
  (`scanovate/<name>:<tag>`, with the access Scanovate gives us) and pushing it
  with the same tag. Pin tags, never `latest`, and check that the digests match
  (`docker buildx imagetools inspect <image>`).
- Never commit credentials, or put them in `.env.development`, election event
  data or tickets.
- `docker login` stores the token in `~/.docker/config.json`, in plain text
  unless a
  [credential helper](https://docs.docker.com/reference/cli/docker/login/#credential-stores)
  is configured. Log out (`docker logout`) on shared machines.
- Air-gapped servers can load the images with `docker save` and `docker load`.

## Running them in the development environment

The services belong to the `scanovate` docker compose profile of the dev
container (`.devcontainer/docker-compose-base.yml`). No mode starts them, since
pulling their images needs an ECR login and they need about 28 GB of memory:
start them from the host, in the checkout, next to the `backend` or `full` mode.

```bash
# Once every 12 hours: log in to our ECR mirror (see Access to the images)
aws sso login --profile sequent-ecr
aws ecr get-login-password --profile sequent-ecr --region eu-west-1 \
  | docker login --username AWS --password-stdin 133529410358.dkr.ecr.eu-west-1.amazonaws.com

.devcontainer/scripts/initialize-command.sh
(cd .devcontainer && docker compose --profile scanovate up -d)

# Stop them, and free their memory, when done
(cd .devcontainer && docker compose --profile scanovate stop)
```

`step-dev mode status` lists them under `outsideModes`. The development realm
and the [sample election event](scanovate_identity_verification_guide.md#sample-election-event)
already point to them.

`initialize-command.sh` writes the `SCANOVATE_*` variables of
`.env.development` into `.devcontainer/.env`. You only need to run it again if
your `.env` predates them.

| Variable | Default | Description |
| --- | --- | --- |
| `SCANOVATE_LIVENESS_PORT` | `5050` | Host port of `scanovate-liveness`, for direct checks. The voter's browser reaches it through `keycloak-nginx`. |
| `SCANOVATE_FACE_MATCH_PORT` | `5060` | Host port of `scanovate-face-match`, for direct checks. Scanovate's examples use `3000` or `3002`, which the voting and admin portals use. |
| `SCANOVATE_OCR_PORT` | `5070` | Host port of `scanovate-ocr`, for direct checks. The voter's browser reaches it through `keycloak-nginx`. |
| `SCANOVATE_DOCUMENT_LIVENESS_PORT` | `5080` | Host port of `scanovate-document-liveness`, on `127.0.0.1` only, for direct checks. Only Keycloak calls it. |
| `SCANOVATE_LIVENESS_JWT_SECRET_KEY` | `liveness-dev-secret` | Secret of the liveness session tokens. |
| `SCANOVATE_OCR_JWT_SECRET_KEY` | `ocr-dev-secret` | Secret of the OCR session tokens. |

The PAD and IAD servers aren't published on the host: only Liveness Plus talks
to them. Inside the compose network the services are reachable by name, e.g.
`http://scanovate-face-match:3000` and `http://scanovate-document-liveness:8080`
from Keycloak, and Keycloak is
`http://keycloak:8090` from OCR and Liveness Plus.

Scanovate's requirements, on Intel CPUs:

| Service | Needs |
| --- | --- |
| Liveness Plus | 1 CPU, plus 1 CPU and 1 GB of memory per concurrent session. |
| PAD | 1 CPU and 8 GB of memory to start, plus 0.5 CPU and 0.5 GB per concurrent session. |
| IAD | 1 CPU and 4 GB of memory to start, plus 0.5 CPU and 0.5 GB per concurrent session. |
| OCR | By default 1 CPU and 2 GB of memory per concurrent session (`CPU_PER_SESSION`, `MEMORY_PER_SESSION_GB`). |
| Document Liveness | 4 CPU cores with AVX2 (`grep -o -m1 avx2 /proc/cpuinfo`; a virtual machine must pass the flag through) and 12 GB of memory. |

Idle, PAD takes about 5.4 GB of memory, the OCR service 7.8 GB, Document
Liveness 6.6 GB with its four pipelines (1.9 GB with screen replay only),
Face Match 1.7 GB, Liveness Plus 0.5 GB and IAD 3.0 GB: give Docker at least
28 GB for the profile, on top of the rest of the dev container. If a container exits
right after starting, check `docker compose logs <service>`.

### Smoke tests

```bash
# OCR, Liveness Plus and Face Match
curl -i http://127.0.0.1:5070/alive
curl -i http://127.0.0.1:5050/alive
curl -i http://127.0.0.1:5060/alive

# Document Liveness: its version, enabled pipelines and their licence expiry
curl -s http://127.0.0.1:5080/api_version | jq

# Document Liveness: a photo of a physical passport scores at least 0.5 on
# every pipeline, with status_code OK; one of a passport on a phone does not
jq -n --rawfile a <(base64 -w0 passport.jpg) \
  '{image: $a, errors_to_ignore: [],
    pipelines: [("default-sr","default-pc","default-ps","default-dm")
                | {pipeline_name: ., calibration: "REGULAR"}]}' \
  | curl -s -H 'content-type: application/json' --data @- \
    http://127.0.0.1:5080/check_liveness | jq '.pipeline_results[] | {pipeline_name, result}'

# Face Match 1:1: with two photos of the same person, success is true and the
# similarity is above the threshold (0.67)
jq -n --rawfile a <(base64 -w0 id.jpg) --rawfile b <(base64 -w0 selfie.jpg) \
  '{image_1_base64: $a, image_2_base64: $b, allow_multiple_faces: false}' \
  | curl -s -H 'content-type: application/json' --data @- \
    http://127.0.0.1:5060/facematch11/compare_images

# Through keycloak-nginx: both capture pages are served (200), a session with
# a token Keycloak doesn't know is rejected (401), and the services' docs and
# Keycloak's endpoints for the services are not routed (404)
curl -sk -o /dev/null -w '%{http_code}\n' https://localhost:8443/biometric/ocr/
curl -sk -o /dev/null -w '%{http_code}\n' https://localhost:8443/biometric/liveness/
curl -sk -o /dev/null -w '%{http_code}\n' -X POST -H 'content-type: application/json' \
  -d '{"token": "x"}' https://localhost:8443/biometric/liveness/create_session
curl -sk -o /dev/null -w '%{http_code}\n' https://localhost:8443/biometric/liveness/docs
curl -sk -o /dev/null -w '%{http_code}\n' https://localhost:8443/realms/master/identity-verification/ocr/verify
```

## End-to-end test

This runs the whole enrollment in the dev container with your own face and
passport, on the real services: OCR reads your passport, Liveness Plus, PAD and
IAD check your face, you take a photo holding your passport, and Face Match
compares your face with the portrait of your passport and with that photo.

Everything stays on your machine. OCR and Liveness Plus keep their sessions in
memory for 5 minutes, Face Match keeps nothing, and Keycloak keeps the portrait,
what OCR read, the live face and the holding photo in its single-use store until the capture is
checked (at most 30 minutes). No image is logged.

### What you need

- Access to our ECR mirror of the Scanovate images (see
  [Access to the images](#access-to-the-images)), and about 28 GB of memory for
  Docker on top of the dev container, on a CPU with AVX2.
- A computer with a webcam of at least 1280×720, and Chrome or Firefox.
- Your passport: the data page, with your photo and the machine readable zone.
- This branch checked out, and the dev container (`base` profile) working.

### 1. Start the services

From the host, in the checkout:

```bash
# Writes the SCANOVATE_* variables into .devcontainer/.env if it predates them
.devcontainer/scripts/initialize-command.sh
aws ecr get-login-password --profile sequent-ecr --region eu-west-1 \
  | docker login --username AWS --password-stdin 133529410358.dkr.ecr.eu-west-1.amazonaws.com

cd .devcontainer
docker compose --profile scanovate up -d
```

Wait until the Scanovate services answer. PAD, IAD and the OCR service take a
minute or two to load their models:

```bash
docker compose ps scanovate-ocr scanovate-liveness scanovate-presentation-detection \
  scanovate-injection-detection scanovate-face-match
curl -s http://127.0.0.1:5050/alive
curl -s http://127.0.0.1:5060/alive
curl -s http://127.0.0.1:5070/alive
```

### 2. Build Keycloak with the capture page

The capture page is in the React login theme, which the Keycloak image builds
from `packages/keycloak-ui` and ships. From the host, rebuild Keycloak with this
branch's extensions and themes, and `keycloak-nginx` with its proxy rules, then
start them:

```bash
cd .devcontainer
docker compose build keycloak keycloak-nginx
docker compose up -d keycloak keycloak-nginx
```

After changing `packages/keycloak-ui` or `packages/keycloak-extensions`,
rebuild Keycloak. To iterate on the capture page without rebuilding, build the
themes in a `devenv shell` (it needs Node, Yarn and Maven) and mount them over
the image's with the `docker-compose-keycloak-ui.yml` overlay:

```bash
scripts/dev/step-dev keycloak prepare --runtime built
cd .devcontainer
docker compose -f docker-compose.yml -f docker-compose-keycloak-ui.yml up -d keycloak keycloak-nginx
```

Recreating Keycloak without the overlay (a plain `docker compose up`) goes back
to the themes in the image.

Check the proxy with the last commands of the [Smoke tests](#smoke-tests): the
pages answer `200`, a session with an unknown token `401`, and the services'
docs and Keycloak's `identity-verification` endpoints `404`.

### 3. Import the election event

Start the admin portal from `packages/` in a `devenv shell`:

```bash
yarn && yarn build:ui-core && yarn build:ui-essentials && yarn start:admin-portal
```

In the admin portal (http://127.0.0.1:3002), import the
[sample election event](scanovate_identity_verification_guide.md#sample-election-event)
(`packages/step-cli/data/scanovate-enrollment/election-event.json`) with
**Import Election Event**. Its realm is
`tenant-<tenant id>-event-<election event id>`, which the Keycloak admin
console (http://127.0.0.1:8090) lists. It already uses the `sequent-ui-voting`
login theme and the services of the dev container.

The import gives the election event a new id, not the one in the file, so read
the realm name from the admin portal or the import's output. step-cli imports
it the same way, with a configured `step-cli step config`:

```bash
step-cli step import-election --file-path packages/step-cli/data/scanovate-enrollment/election-event.json --is-local
step-cli step import-voters --election-event-id <election event id> \
  --file-path packages/step-cli/data/scanovate-enrollment/voters.csv --is-local
```

To run the enrollment again from scratch with the same document, delete the
election event (`step-cli step delete-election-event --election-event-id <id>`)
and import it and its voters again.

### 4. Load yourself as a voter

Enrollment only succeeds if the names and date of birth read from your passport
match a voter of the election event. In the election event, **Voters**, import
a CSV with your names and date of birth exactly as in your passport's machine
readable zone:

```csv
username,first_name,last_name,enabled,area_name,dateOfBirth,embassy,country
tester,<FIRST NAMES>,<LAST NAME>,true,Japan - Tokyo PE,<yyyy-MM-dd>,Tokyo PE,Japan/Tokyo PE
```

### 5. Configure other realms

The sample election event is ready. For another realm, such as an export of an
election event configured for the B-Trust integration,
`configure-realm.sh` points its Scanovate steps to the services, removes the
B-Trust settings, warns about rules written for B-Trust, and sets the React
login theme on the realm and on the clients that set their own. It needs
`curl` and `jq`:

```bash
SCANOVATE_OCR_TYPES='{"philippinePassport": "passport"}' \
SCANOVATE_LIVENESS_URL=https://localhost:8443/biometric \
SCANOVATE_LIVENESS_SECRET=liveness-dev-callback-secret \
SCANOVATE_FACE_MATCH_URL=http://scanovate-face-match:3000 \
SCANOVATE_DOCUMENT_LIVENESS_URL=http://scanovate-document-liveness:8080 \
SCANOVATE_LOGIN_THEME=sequent-ui-voting \
  packages/keycloak-extensions/scanovate-authenticator/scripts/configure-realm.sh \
  configure tenant-<tenant id>-event-<election event id>
```

`liveness-dev-callback-secret` is the secret of both
`.devcontainer/scanovate/ocr/service_config.json` and
`.devcontainer/scanovate/liveness/service_config.json`. Set
`SCANOVATE_FACE_MATCH_MIN_SIMILARITY` to change the threshold
(`{"default": 0.67}`), `SCANOVATE_DOCUMENT_LIVENESS_PIPELINES` for the
document checks (all four by default), `SCANOVATE_HOLDING_PHOTO` (`REQUIRED` or `NONE`) for the
photo holding the document, and `SCANOVATE_HOLDING_SECONDS` for its holding
time (`video-seconds`, 5 by default). The script also deletes the settings of
the earlier integrations, including `ocr-url`, `capture-sides` and
`max-image-bytes` of our former capture page, which the authenticator
ignores. `KEYCLOAK_URL`, `KEYCLOAK_ADMIN` and
`KEYCLOAK_ADMIN_PASSWORD` default to the dev container Keycloak
(`http://127.0.0.1:8090`, `admin`/`admin`). The same settings can be edited in
the Keycloak admin console: **Authentication**, the registration flow, then the
settings of the Scanovate step.

### 6. Enroll

Open the enrollment page from the voting portal's **Enroll** link, or directly
on `https://localhost:8443`. Accept the development certificate of
`https://localhost:8443` first, which serves the OCR and Liveness Plus pages:
the dev voting portal sends voters to Keycloak on `http://localhost:8090`,
whose capture page then embeds those pages from `https://localhost:8443`, and
an iframe can't ask you to accept a certificate. The `redirect_uri` can be any
voting portal URL of the client, such as
`http://localhost:3000/tenant/<tenant id>/event/<election event id>/login`:

```text
https://localhost:8443/realms/<realm>/protocol/openid-connect/registrations?client_id=voting-portal&response_type=code&scope=openid&redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F
```

1. Fill in the form: **Passport**, **Japan/Tokyo PE** and **Tokyo PE**, an
   email, a phone and a password. The development senders don't deliver the
   codes, and the logs mask them (`Your OTP is ******`). To enter a fixed code,
   set both `test-mode` to `true` and `test-mode-code` (e.g. `123456`) in the
   settings of each `message-otp-authenticator` step of the registration flow,
   including those in the `deferred-otp-subflow*` subflows: `test-mode` alone
   accepts no code. Keycloak reads them when it checks the code, so they apply
   to a code already sent. Never set them outside development.
2. **Scan your document**: OCR's page asks for the camera, then guides you
   through the scan of your passport's data page, flat, with the whole page in
   the frame.
3. **Check your face**: Liveness Plus's page guides you to keep your face in
   its frame, looking at the camera, until it completes.
4. **Checking**: Keycloak waits for both results, checks the photo of your
   passport with Document Liveness, and compares your face with the portrait
   of your passport.
5. **Check the details from your ID**: your names, passport number and date of
   birth. **Confirm and enroll**. Keycloak then redirects to the
   `redirect_uri`; if the voting portal isn't running there, that page fails to
   load, but the enrollment is done by then.

Follow it in the logs, from `.devcontainer`:

```bash
docker compose logs -f scanovate-ocr           # token check, start and result callbacks
docker compose logs -f scanovate-liveness      # token check, frames, start and result callbacks
docker compose logs -f scanovate-document-liveness
docker compose logs -f keycloak | grep -E "ScanovateAuthenticator|OnPremSessions|capture:|readDocument"
```

Keycloak logs each result (`record: ocr session of case <case id> ended with
completed, passed=true`), every pipeline of the document check with its
probability and image quality warnings, e.g. `capture: document liveness of
<case id>: GENUINE, [PipelineCheck[pipeline=default-sr, status=OK,
probability=0.97, warnings=[]], ...]`, and the similarity of the comparison,
e.g. `capture: scanovate_face_match_document of <case id>: MATCH,
similarity=0.8123, statuses=0/0`. The Keycloak events (**Realm settings**,
**Events**, or the admin API) carry the lowest probability of the document
check as `scanovate_document_liveness` and the similarity as
`scanovate_face_match_document`, with the case id as `scanovate_case_id`.

### 7. Try to break it

Each of these must fail, and use up an attempt out of `max-attempts` (3):

| Try | Expected |
| --- | --- |
| Show a photo of yourself, printed or on a screen, in the face step | `scanovateLivenessError` |
| Use a virtual camera (e.g. OBS's) in the face step | `scanovateLivenessError` (injection attack) |
| Show someone else's passport, or have someone else do the face step | `scanovateFaceMismatchError` |
| Show a photo of your passport on a phone or a screen, or a printed copy of it, in the document step | `scanovateDocumentAuthenticationError` if OCR reads it and Document Liveness finds it isn't physical |
| Hold the passport so that part of it is outside the frame, or a second document is in it | `scanovateDocumentUnreadableError` if Document Liveness can't check the photo (`DOCUMENT_CROPPED`, `TOO_MANY_DOCUMENTS`, ...) |
| Cover the photo on the passport | `scanovateDocumentUnreadableError` if OCR finds no portrait, `scanovateFaceNotFoundError` if Face Match finds no face in it |
| Cover the machine readable zone, or show another kind of document | `scanovateDocumentUnreadableError` |

These must not use up an attempt:

- Deny the camera: the page explains how to allow it, and offers to try again.
- Stop a service's container (`docker compose stop scanovate-ocr`) after its
  page is done and before the checking step: `scanovateInternalError`, which
  the voter can retry.
- Let a Scanovate page's session expire: the page offers to try again, which
  opens a new session with the same token. Each token allows 3 sessions; then
  the page only offers to start over, with new tokens.

The similarities of your tries are a first hint for
`face-match-min-similarity`: note the ones of genuine and of mismatched
faces for each document type.

### 8. Clean up

- Delete the enrolled voter from the election event to enroll again.
- `docker compose restart scanovate-ocr scanovate-liveness keycloak` drops
  what's still in memory, and `docker compose --profile scanovate stop` frees the memory of the
  Scanovate services.
- Never commit, attach or share photos of your passport or face, including
  screenshots or recordings of the capture page.

## Configuration

### Liveness Plus

Environment variables:

| Variable | Description |
| --- | --- |
| `MODE` | `onprem`. |
| `LIVENESS_MODE` | `full`: PAD and IAD, as in Scanovate's kit. `PRESENTATION` would run PAD only. See [Injection attacks](#injection-attacks). |
| `JWT_SECRET_KEY` | Secret of the session tokens. Required. A secure random value outside development. |
| `PRESENTATION_DETECTION_SERVER_URL` | PAD server URL, e.g. `http://scanovate-presentation-detection:8080`. |
| `INJECTION_DETECTION_SERVER_URL` | IAD server URL, e.g. `http://scanovate-injection-detection:8080`. |
| `CLIENT_BASE_URL_PREFIX` | Prefix of the paths of the page, its assets and its API, ending with a slash. We use `biometric/`, see [Deployment on Keycloak's origin](#deployment-on-keycloaks-origin). |
| `ENABLE_HTTPS` | `true` to serve HTTPS, with `SSL_KEYFILE`, `SSL_CERTFILE` and `SSL_CAFILE` (e.g. under `/app/certs`, mounted read-only). |
| `SSL_EXTERNAL_CA_VERIFICATION_FILE` | CA bundle to verify the callback and token verification URLs. |
| `SSL_EXTERNAL_USE_CA_VERIFICATION` | `false` disables that verification. Never in production. |

The PAD server needs `IDFACE_SERVER_AVAILABLE_PIPELINES` (`pad-r-2,dfd-3`) and
both detection servers `SERVER_ALLOW_CORS=true`, as in Scanovate's kit.

The service reads JSON files from `/app/config`. The dev container replaces
`service_config.json`, with
`.devcontainer/scanovate/liveness/service_config.json`, and adds our UI theme,
`.devcontainer/scanovate/liveness/sequent_ui.json` (see
[Theming](#theming)); the others (`scan_config.json`, `video_config.json`,
`default_ui.json` and the texts under `locales/`) are the image's own.

Our `service_config.json` is Scanovate's with the `onprem` section pointing to
Keycloak (see [Integration contract](#integration-contract)), without the video
in the results (`send_video_in_results: false`), with English as the
default language of the page, our theme as its default UI theme, and with
`mask_threshold` raised from
Scanovate's 0.5 to 0.8, like `sunglasses_threshold`: at 0.5, faces without
any covering (a beard, side light) scored 0.64–0.74 and were asked to remove
a face covering on every frame. Both scores are only quality hints; the PAD
server catches presentation attacks. Its `onprem` section:

| Key | Our value | Description |
| --- | --- | --- |
| `callback_url` | Keycloak's `.../identity-verification/liveness/callback?secret=...` | Where the start and the result are posted. |
| `token_verification_url` | Keycloak's `.../identity-verification/liveness/verify?secret=...` | Where the session token is verified. **Empty accepts any token**, never do it. |
| `max_active_sessions` | `0` | Maximum concurrent sessions (`0`: no limit). |
| `send_results_to_server` | `true` | Post the result to `callback_url`. |
| `send_debug_results_to_server` | `false` | Also post the full internal session. |
| `send_video_in_results` | `false` | Include the video of the session in the result. Keycloak doesn't use it. |
| `send_results_to_client` | `false` | Return the result to the browser. Keep it `false`: the browser is never trusted. |
| `default_parameters` | `scan_config`, `video_config`, `default`, `en`, `sequent_ui` | Configuration files, texts and theme used when the page URL doesn't name them. Keycloak's page URLs only carry `case_id` and `token` (and OCR's `ocr_type`). |

Other settings of the file: sessions expire after 90 seconds
(`expiry_timeout_in_seconds`) and are deleted 5 minutes later
(`deletion_timeout_in_minutes`). The PAD pipelines are `pad-r-2` (main,
threshold `0.56`) and `dfd-3` (`0.5`): the check passes only if every pipeline
scores above its threshold. Exactly one pipeline must be `main`, and the names
must match `IDFACE_SERVER_AVAILABLE_PIPELINES` of the PAD server. `face_config`
sets the face quality a frame needs before PAD runs: a face of at least
200×200 pixels, 60 pixels between the eyes, the nose near the centre of the
frame, the head turned less than 15° and tilted less than 18°, sharp, lit, and
without sunglasses or a mask.

### Face Match

The 1:1 API needs no configuration file. Environment variables:

| Variable | Description |
| --- | --- |
| `MODE` | `onprem`. |
| `SUPPORT_1_TO_MANY` | `true` also serves the 1:N API (`/facematch1N`), which needs a Valkey or Redis database. We leave it unset. |
| `ENABLE_HTTPS`, `SSL_KEYFILE`, `SSL_CERTFILE`, `SSL_CAFILE`, `SSL_KEY_PASSWORD`, `SSL_USE_TLS_1_2` | Serve HTTPS. |
| `OMP_NUM_THREADS`, `OPENBLAS_NUM_THREADS`, `MKL_NUM_THREADS`, `ONNXRUNTIME_INTER_OP_NUM_THREADS`, `ONNXRUNTIME_INTRA_OP_NUM_THREADS` | Limit the CPU threads it uses (all of them by default). |
| `LOG_IMAGES` | Logs face images, only for debugging: never enable it with real voters. |

The match threshold is the authenticator's `face-match-min-similarity`, per
document type. Keycloak uses it or the service's own threshold (`0.67`),
whichever is higher.

### OCR

Environment variables:

| Variable | Description |
| --- | --- |
| `MODE` | `onprem`. In `saas` mode the service fetches its configuration from Scanovate's cloud (`AUTH_URL`) and can send analytics (`ENABLE_ANALYTICS`): never use it. |
| `JWT_SECRET_KEY` | Secret of the session tokens. Required. A secure random value outside development. |
| `CLIENT_BASE_URL_PREFIX` | `biometric/`, as for Liveness Plus: the service serves its page, assets and API under `/biometric/ocr/`. |
| `USE_CPU_MEMORY_LIMITS`, `CPU_PER_SESSION`, `MEMORY_PER_SESSION_GB` | Limit the concurrent sessions by the CPUs and memory available (off by default). |
| `ENABLE_HTTPS`, `SSL_KEYFILE`, `SSL_CERTFILE`, `SSL_CAFILE`, `SSL_USE_TLS_1_2` | Serve HTTPS. |
| `SSL_EXTERNAL_CA_VERIFICATION_FILE`, `SSL_EXTERNAL_USE_CA_VERIFICATION` | As for Liveness Plus, for the callback and token verification URLs. |
| `STICKY_SESSION_HEADER` | Header a load balancer uses for session affinity, `x-eg-session-router` by default. |

The dev container replaces `service_config.json`, with
`.devcontainer/scanovate/ocr/service_config.json`, and adds our UI theme,
`.devcontainer/scanovate/ocr/sequent_ui.json`, and
`.devcontainer/scanovate/ocr/video_config_no_mirror.json`, and serves its own
copy of the page, `.devcontainer/scanovate/ocr/index.html` (see below); the others
(`scan_config.json`, which says which sides each OCR type scans,
`video_config.json`, `default_ui.json` and the texts) are the image's own. Our `service_config.json`
is the image's with an `onprem` section like Liveness Plus's:

| Key | Our value | Description |
| --- | --- | --- |
| `callback_url` | Keycloak's `.../identity-verification/ocr/callback?secret=...` | Where the start and the result are posted. |
| `token_verification_url` | Keycloak's `.../identity-verification/ocr/verify?secret=...` | Where the session token is verified. **Empty accepts any token**, never do it. |
| `send_results_to_server` | `true` | Post the result to `callback_url`. |
| `send_debug_results_to_server` | `false` | Also post the full internal session. |
| `send_video_in_results` | `false` | No video in the result. |
| `send_results_to_client` | `false` | Keep it `false`: the browser is never trusted. |
| `default_parameters` | `passport`, `scan_config`, `video_config_no_mirror`, `default`, `en`, `sequent_ui` | OCR type, configuration files, texts and theme used when the page URL doesn't name them. Keycloak always sets `ocr_type`, from `ocr-types`. |

`video_config` names the file of `/app/config` that rotates or mirrors the
preview (`rotate_view`, `mirror_view`) and the frames sent to the service
(`rotate_buffer`, `mirror_buffer`), as Scanovate advised; a page URL can also
name one with its `video_config` parameter. `video_config_no_mirror.json` is
the image's `video_config.json`, whose `mirror_view` is already `false`. In
this image the page still mirrors the preview whenever the browser isn't a
phone or a tablet (`mirror_view || !isMobile()` in its client), whatever the
file says, which makes a passport hard to place on a laptop. We have reported
it to Scanovate. Until it follows `video_config`, we serve the image's own
page, `/app/static/index.html`, with one rule added,
`.video-element { transform: none !important; }`, which shows the preview as
the camera sees it on every device. It only changes what the voter sees: the
frames sent to the service were never mirrored. `transform: none` also drops
the page's rotation, so keep `rotate_view` at `0`. The page names the image's
hashed scripts (`index-C243eWap.js`, `index-DTxKRzC9.css`), so it belongs to
one OCR image: when the image changes, copy its `index.html` again and add the
rule back. `scripts/dev/test_scanovate_ocr_page.py` checks the mount, the rule
and the asset names against the pinned tag.
### Document Liveness

Environment variables, as in Scanovate's onboarding guide:

| Variable | Description |
| --- | --- |
| `IDLIVEDOC_SERVER_AVAILABLE_PIPELINES` | `default-sr,default-pc,default-ps,default-dm`. The pipelines it loads: each pipeline in the authenticator's `document-liveness-pipelines` must be one of them, or the service answers `500` (*unsupported pipeline*). Each one loaded takes memory: 1.9 GB idle with `default-sr` alone, 6.6 GB with all four. |
| `HASP_RTE_STARTUP_TIMEOUT` | `10`: seconds to wait for the licence manager the image starts. |
| `SERVER_ALLOW_CORS` | `true`. |

The licence is in the image, for the four pipelines, until 2027-10-25
(`GET /api_version` and `GET /license_info` show it); the service needs no
licence server and nothing outside our network. It answers on port 8080, with
no configuration file, and writes usage counters (`GET /bi_report`) to `/logs`.
It is stateless and keeps no image. Only Keycloak calls it: keep it, its
`/metrics` and `/v3/api-docs` off public networks.

The authenticator's settings:

| Setting | Default | Description |
| --- | --- | --- |
| `document-liveness-url` | none (required); the presets write `http://scanovate-document-liveness:8080` | Internal base URL of the service. |
| `document-liveness-pipelines` | `default-sr,default-pc,default-ps,default-dm` | The checks the document photo must pass. |
| `document-liveness-calibration` | `REGULAR` | `SOFT` rejects fewer genuine documents, `HARD` misses fewer attacks. |
| `document-liveness-min-probability` | `0.5` | Minimum `liveness_probability` of every pipeline. |

Image sessions expire after 120 seconds (`image_ocr_expiry_timeout_in_seconds`),
video sessions after 90 (`video_ocr_expiry_timeout_in_seconds`), and both are
deleted 5 minutes later. The `regula.url` of `service_config.json` is where the
`regula` OCR type forwards the images: empty, as we have no Regula server, so
that type fails.

## Integration contract

### Capture pages

Keycloak's capture page (`packages/keycloak-ui/src/login/pages/ScanovateCapture.tsx`)
receives the URL of each page, built from the authenticator's `liveness-url`
(`https://<keycloak host>/biometric`):

| Page | URL |
| --- | --- |
| OCR | `<liveness-url>/ocr/?ocr_type=<OCR type>&case_id=<case id>&token=<one-time token>` |
| Liveness Plus | `<liveness-url>/liveness/?case_id=<case id>&token=<one-time token>` |

It embeds them one after the other in an iframe with
`allow="camera *; microphone *"`, focused once loaded, as Scanovate's guide
asks. The pages talk to their own API (sessions, frames, server-sent events
with the guidance), under the same prefix, and post messages to the parent
window:

| Message | The capture page |
| --- | --- |
| `{"type": "init", ...}` | Ignores it. |
| `{"type": "done", ...}` | Moves on: from OCR to Liveness Plus, and after Liveness Plus posts `action=capture` to Keycloak. |
| `{"type": "error", "error_code": ..., ...}` | Shows a problem: the camera (`1003`, `1004`, `1005`, `1013`), an expired session (`1006`), a closed page (`1009`), an invalid token (`1014`), or any other error. |

It only accepts messages from the window of the iframe it shows, from
Keycloak's origin or the page URL's. **Try again** mounts the page again, which
opens a new session with the same token; after an invalid token, only **Start
over** is offered, which posts `action=retry` for new tokens without using up
an attempt. None of these messages carries a verdict: Keycloak only reads the
results the services post to it.

### Token verification and callbacks

Both services call Keycloak, server to server, with the secret in the query
string of the URLs of their `service_config.json`:

1. Before starting a session, `GET {token_verification_url}` with the
   `X-token`, `service-name` and, when known, `case-id` headers. Any answer
   but `2xx` rejects the session, and the page reports an invalid token.
2. `POST {callback_url}`: first `message_type: "start"` (status
   `session_started`), then `message_type: "result"`, with `onprem_params`
   (`case_id`, and the `token`). Liveness Plus also sends the `x-token` and
   `case-id` headers, OCR `service-name` and `case-id`.

Keycloak's endpoints, under any realm (the dev configuration uses `master`):

| Endpoint | Answers |
| --- | --- |
| `GET /realms/{realm}/identity-verification/{ocr,liveness}/verify?secret=...` | `200` if the `X-token` is a token of that service, matches the `case-id` and hasn't used its 3 sessions, `401` otherwise. |
| `PUT /realms/{realm}/identity-verification/capture/holding` | Called by the voter's browser, not the services: the photo holding the document, as the raw body. `200` once stored, `401` without the capture page's token in `X-Capture-Token`, `400` for an empty body, `413` above 8 MiB, `415` for anything but a JPEG. |
| `POST /realms/{realm}/identity-verification/{ocr,liveness}/callback?secret=...` | `200` once the `start` or `result` message is recorded (other message types, such as `result_debug`, are ignored), `401` for an unknown token, a wrong secret or another case, `400` for a malformed body or image. The token is read from `x-token`, or from `onprem_params.token`. |

The `secret` is the authenticator's `liveness-secret`, the same in the four
URLs. The tokens are random, single-use per capture (Keycloak discards them
once it reads the result, or on a new capture) and expire after 30 minutes.

### OCR result

```json
{"message_type": "result", "status": "completed", "ocr_type": "passport",
 "onprem_params": {"case_id": "<case id>", "token": "..."},
 "front": {"processing_result": {
   "status": "success", "card_type": "MRZ",
   "fields": {"document_number": "P1234567A", "first_name_english": "JUAN",
              "last_name_english": "DELA CRUZ", "date_of_birth": "900115",
              "date_of_expiry": "310101", "issuing_country_code": "PHL",
              "nationality_code": "PHL", "gender": "M", "mrz_type": "TD3", ...},
   "images": {"face_image": "...", "input_image": "...", "cropped_image": "...", ...}}},
 "auth": {"template_matching_valid": true, "document_in_frame_valid": true,
          "expiry_date_valid": true, "face_size_valid": true, ...},
 "screenshots": [...], ...}
```

The sides the service scanned are under `front`, `back` or `scan`. Keycloak
accepts the result only if `status` is `completed`, the `processing_result` of
every side (a passport has one) has status `success`, and a side has the
portrait (`images.face_image`) and the camera frame it was read from
(`images.input_image`), which Document Liveness checks.
Otherwise the attempt fails with `scanovateDocumentUnreadableError`. The other
statuses of `processing_result` say why the document couldn't be read:
`card_not_detected`, `card_out_of_bounds`, `card_too_small`,
`card_quality_low`, `fail_to_recognize_mrz` (no MRZ, or invalid check digits),
`wrong_card_type` or `error`. Session statuses other than `completed` are
`expired`, `aborted` and `client_error`.

Keycloak keeps the `fields` and the `auth` checks, converts the MRZ dates, keeps
the portrait and the frame until the checks, and discards the other images, the
screenshots and the frames. See
[Rules](scanovate_identity_verification_guide.md#rules) for the results the
rules read: the same fields as before, such as `/document_number`,
`/first_name_english`, `/issuing_country_code` and `/date_of_birth`.

The passport reader returns the names of the MRZ: `last_name_english` is the
primary identifier and `first_name_english` every given name, the middle name
included (`JUAN<CARLOS<SANTOS` gives `JUAN CARLOS SANTOS`). There is no middle
name field. Store `first_name_english` in `firstName` and no `middleName`: the
registry lookup then compares the passport's given names with the voter's first
and middle name joined (see
[Approvals](../02-election_managers/02-reference/02-election-event/14-election_management_election-event_approvals.md#what-we-compare)),
so a voter with a middle name in the registry is matched, and one whose middle
name differs isn't.

The service reads the MRZ in the OCR-B typeface. With the `passport` type,
`template_matching_valid` and `document_in_frame_valid` are always `true`: they
don't check the document's authenticity: Document Liveness checks that it is
physical.

### Document Liveness

Keycloak calls `POST /check_liveness` with OCR's `input_image` (the whole
frame, so that the borders of the document are visible), every configured
pipeline at the configured calibration, and no validation error ignored:

```json
{"image": "...", "errors_to_ignore": [],
 "pipelines": [{"pipeline_name": "default-sr", "calibration": "REGULAR"},
               {"pipeline_name": "default-pc", "calibration": "REGULAR"}, ...]}
```

```json
{"pipeline_results": [
   {"pipeline_name": "default-sr", "calibration": "REGULAR",
    "result": {"status_code": "OK", "liveness_probability": 0.97,
               "liveness_score": 3.42, "image_quality_warnings": []}}, ...],
 "aggregate_liveness_probability": 0.93, "aggregate_image_quality_warnings": []}
```

As Scanovate's guide recommends, the document is physical only if every
pipeline answers `status_code` `OK` and a `liveness_probability` of at least
`document-liveness-min-probability`; otherwise the attempt fails with
`scanovateDocumentAuthenticationError`. Any other `status_code` means the photo
couldn't be checked (`DOCUMENT_NOT_FOUND`, `DOCUMENT_PHOTO_NOT_FOUND`,
`DOCUMENT_CROPPED`, `DOCUMENT_TOO_SMALL`, `DOCUMENT_TOO_CLOSE_TO_CAMERA`,
`DOCUMENT_TOO_CLOSE_TO_BORDER`, `TOO_MANY_DOCUMENTS`, `DOCUMENT_IS_COLORLESS`),
never that it's genuine: the attempt fails with
`scanovateDocumentUnreadableError`, so the voter scans it again. The image
quality warnings (`IMAGE_TOO_BLURRY`, `GLARE_ON_IMAGE`, ...) are logged with each
probability. A service that doesn't answer is a retryable
`scanovateInternalError`.

### Liveness result

A completed session has `status: "completed"` and, in
`scan.processing_result`, the verdicts (`liveness_check_passed`,
`presentation_attack_check_passed`, `injection_attack_check_passed`), the PAD,
deepfake and injection probabilities, the checked frame (`image`) and the face
cropped from it (`face_image`). Otherwise `status` is `aborted`, `expired` or
`client_error`, with an `error_message`.

Keycloak accepts the liveness check only if the result is `completed` and
`liveness_check_passed`, `presentation_attack_check_passed` and
`injection_attack_check_passed` are all `true`, with the frame (or, without
it, the cropped face), which is what Face Match compares. Otherwise the attempt
fails with `scanovateLivenessError`.

If either result doesn't arrive within `liveness-result-wait-seconds` (15) of
the capture, the voter gets a retryable `scanovateInternalError`.

### Face Match

Keycloak calls `POST /facematch11/compare_images`:

```json
{"image_1_base64": "...", "image_2_base64": "...", "allow_multiple_faces": false}
```

```json
{"success": true, "similarity": 0.81, "threshold": 0.67,
 "image_1_status": {"code": 0, "message": "ok"},
 "image_2_status": {"code": 0, "message": "ok"}}
```

As Scanovate's guide recommends, `allow_multiple_faces` is `false` when
Keycloak compares the document's portrait with the live face: a second face
where one is expected is a risk, and fails with status `1102`. It is `true`
only for the holding photo, where the document's portrait is a second, smaller
face, and the largest face is compared. The image status codes are `0` (ok),
`1000` (cannot read the file), `1001` (invalid image), `1100` (image too
small), `1101` (face not found), `1102` (several faces, without
`allow_multiple_faces`) and `1103` (cannot align the face).

A comparison passes only if `success` is `true`, both statuses are `0` and the
similarity reaches both the service's threshold and the configured minimum.
Keycloak compares the portrait OCR cropped from the document (`image_1`) with
the liveness frame (`image_2`), then the holding photo with the liveness
frame. A status other than `0` fails the attempt with
`scanovateFaceNotFoundError`, a low similarity with
`scanovateFaceMismatchError`.

The API also has `POST /facematch11/upload_and_compare_images` (the same, as a
multipart upload), `/facematch11/create_template` and
`/facematch11/compare_image_and_template` (to keep a template instead of an
image), `POST /faceutils/crop_face_image` (returns the face found in an image)
and `POST /faceutils/check_face_quality`. `GET /openapi.json` describes them.

### Injection attacks

Scanovate's injection attack detection (IAD) checks that the frames come from a
real camera, and not from a virtual camera, an emulator or a tampered browser.
It needs the frames encrypted by Scanovate's own capture client, which is part
of its Liveness Plus page: that is why we embed the page instead of sending
frames from ours. Liveness Plus runs in `full` mode, with the IAD server, and
Keycloak requires `injection_attack_check_passed` to be `true`.

On top of IAD:

- The frame must pass PAD and the deepfake pipeline at the configured
  thresholds, and match the portrait of the document.
- Tokens are one-time, tied to the authentication session and to the case,
  and allow 3 sessions each. Liveness sessions expire after 90 seconds.
- Attempts are limited by `max-attempts`, and every verification is logged
  with its case id and similarity.

The document itself is checked by Document Liveness: a document shown on a
screen, a printed copy, a replaced portrait or an edited image fails, see
[Document Liveness](#document-liveness-1).

### Theming

Both pages take their colours, fonts and icons from a UI theme file of the
service, `<name>.json` in `/app/config`, named by the `ui_theme` page parameter
or by `onprem.default_parameters.ui_theme`. We mount `sequent_ui.json` in each
service and make it the default:

| Service | Theme | Structure |
| --- | --- | --- |
| OCR | `.devcontainer/scanovate/ocr/sequent_ui.json` | `image_ocr` and `video_ocr`, one per capture mode. |
| Liveness Plus | `.devcontainer/scanovate/liveness/sequent_ui.json` | `title`, `instruction`, `frame`, `close_button`, `loading_screen`, `success_indicator`. |

Each is the image's own `default_ui.json` with the palette of the voter login
theme, from `packages/keycloak-ui/src/login/auth.css`, so that the pages look
like the card around them: primary teal `#0d796a` (hover `#086b5e`) for
buttons, the spinner, the shield icon of the loading screen and the
instruction banner (`rgba(13,121,106,0.85)` over the camera), accent green
`#43e3a1` for progress segments and checkmarks, `#173449`, `#5b6e7b` and
`#7c8d98` for text, `#e3eaee` for borders and `#f4f7f8` for light backgrounds,
and Roboto, the font behind the login theme's *Sequent Sans*, through the
`Roboto-Regular`, `Roboto-Medium` and `Roboto-Bold` families that the services'
own CSS declares. To change a theme, start from the image's `default_ui.json`
(e.g. `docker run --rm --entrypoint cat <image> /app/config/default_ui.json`)
so that no key is missing, and recreate the service: it reads the file when it
creates a session.

### Air-gapped networks

The pages load their scripts, styles and fonts from the service, through
Keycloak's origin. Their analytics only load when the service sets
`ENABLE_ANALYTICS`, which is a SaaS setting that we never set on premise, and
with `MODE=onprem` the services fetch nothing from Scanovate's cloud. Run the
profile on a network without internet access, and watch the browser's
requests, to confirm it for each new image version.

## Deployment on Keycloak's origin

We serve the OCR and Liveness Plus pages on the same origin as Keycloak, under
`/biometric/`:

| | URL |
| --- | --- |
| OCR page and its API | `https://<keycloak host>/biometric/ocr/` |
| Liveness Plus page and its API | `https://<keycloak host>/biometric/liveness/` |
| Authenticator `liveness-url` | `https://<keycloak host>/biometric` |
| Keycloak endpoints for OCR and Liveness Plus | `http://<keycloak internal host>/realms/master/identity-verification/{ocr,liveness}/{verify,callback}`, internal only |
| Authenticator `face-match-url` | `http://<face match internal host>:3000`, internal only |
| Authenticator `document-liveness-url` | `http://<document liveness internal host>:8080`, internal only |

`/biometric/` is vendor neutral, and none of Keycloak's own top-level paths
(`/realms`, `/admin`, `/resources`, `/js`, `/health`, `/metrics`) use it. Being
same origin, the pages need no CORS, and the voter grants the camera to a
single origin, which the iframes can use.

- Keycloak's Content Security Policy only lets its pages frame its own origin
  (`frame-src 'self'`). On the capture page, the authenticator adds the origin
  of `liveness-url` (its scheme and authority) to `frame-src`
  (`SecurityHeadersProvider.allowFrameSrc`), so the pages can also be served
  from another origin, as in development, where the capture page is on
  `http://localhost:8090` and the pages on `https://localhost:8443`.
- `CLIENT_BASE_URL_PREFIX=biometric/` makes each service serve everything under
  `/biometric/<service>/`. Only `/alive` also answers without the prefix.
- Keycloak's public reverse proxy routes `/biometric/ocr/` and
  `/biometric/liveness/` with the path unchanged: stripping the prefix makes
  the services answer `404`. The pages stream their guidance with server-sent
  events, so the proxy must not buffer the responses, and the frames and video
  chunks need bodies bigger than nginx's 1 MiB default.
- It answers `404` for the services' `/docs`, `/redoc`, `/openapi.json`,
  `/metrics` and `/active_sessions`, and for the rest of `/biometric/`. PAD,
  IAD and Face Match stay internal.
- It answers `404` for `/realms/*/identity-verification/ocr/` and
  `/realms/*/identity-verification/liveness/`: only OCR and Liveness Plus call
  those endpoints, on the internal network. The capture page's upload,
  `/realms/*/identity-verification/capture/holding`, stays reachable, with
  bodies of up to 10 MiB.

The dev container does this in `keycloak-nginx`
(`.devcontainer/keycloak-nginx/keycloak-mtls.conf.template`), on
`https://localhost:8443`. The template is built into the image: after changing
it, run `docker compose build keycloak-nginx` and
`docker compose up -d keycloak-nginx` from `.devcontainer`.

```nginx
location ~ ^/biometric/(liveness|ocr)/(docs|redoc|openapi\.json|metrics|active_sessions)(/|$) {
    return 404;
}

location /biometric/liveness/ {
    set $scanovate_upstream http://scanovate-liveness:5050;
    proxy_pass $scanovate_upstream;
    proxy_http_version 1.1;
    proxy_set_header Connection "";
    proxy_buffering off;          # server-sent events
    proxy_read_timeout 120s;
    client_max_body_size 10m;     # frames and video chunks
}

location /biometric/ocr/ {
    set $scanovate_upstream http://scanovate-ocr:5040;
    # the same settings
}

location /biometric/ {
    return 404;
}

location ~ ^/realms/[^/]+/identity-verification/(ocr|liveness)/ {
    return 404;
}

location / {
    # Keycloak, as before, with room for the holding photo
    client_max_body_size 10m;
}
```

## Production

- Serve the OCR and Liveness Plus pages over HTTPS on Keycloak's origin (see
  [Deployment on Keycloak's origin](#deployment-on-keycloaks-origin)):
  browsers only give camera access to secure origins.
- Keep PAD, IAD, Face Match and Document Liveness on the internal network, and
  don't route the services' docs, metrics and session listings publicly.
- Run OCR and Liveness Plus with `MODE=onprem`, Liveness Plus with
  `LIVENESS_MODE=full`, and each with its own secure random `JWT_SECRET_KEY`
  from the secrets store.
- Set `token_verification_url` and `callback_url` of both services with a
  random `secret`, the same as the authenticator's `liveness-secret`, and test
  them before going live. The services reach Keycloak on the internal network.
- Block `/realms/*/identity-verification/ocr/` and
  `/realms/*/identity-verification/liveness/` on the public proxy; keep
  `/realms/*/identity-verification/capture/holding` reachable for the voter's
  browser.
- Mount the Sequent theme (`sequent_ui.json`) in both services, see
  [Theming](#theming), and our OCR page (`index.html`), which shows the
  preview unmirrored on desktop browsers, the one of the OCR image you run.
- Pull the images from our ECR mirror (see
  [Access to the images](#access-to-the-images)).
- Pin image tags, never `latest`.
- Tune the PAD thresholds and the Face Match minimum similarity with real
  devices and each accepted document type, and check that the OCR service reads
  every accepted document type.
- Size the servers for the expected concurrent sessions (see the requirements
  in [Running them in the development environment](#running-them-in-the-development-environment)),
  and set `max_active_sessions` of Liveness Plus to match.
- Run Document Liveness on CPUs with AVX2, with 12 GB of memory, the same
  pipelines in `IDLIVEDOC_SERVER_AVAILABLE_PIPELINES` and in
  `document-liveness-pipelines`, and tune `document-liveness-min-probability`
  and `document-liveness-calibration` with real passports, screens and prints.
- The PAD image carries a license that expires on 2027-10-04, and the Document
  Liveness image one that expires on 2027-10-25: renew them with Scanovate
  before then.
