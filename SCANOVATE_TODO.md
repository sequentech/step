<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Scanovate on-premise TODO

Follow-ups of the Scanovate on-premise integration. See
[Scanovate On-Premise Services](docs/docusaurus/docs/integrations/scanovate_on_premise_guide.md).

## Agreed design

On premise we only get the Scanovate APIs: Liveness Plus's own UI (the iframe)
isn't available. With `face-capture=liveness`:

- **Photos:** our capture page takes them all, guided by the `id-capture` WASM
  analyzers: the front and back of the ID, the face, and the voter holding the
  ID next to their face (the four capture steps of the meta#13611 drafts).
- **Liveness:** the page sends the face frames to the Liveness Plus API
  (`create_session`, `check_liveness`, `client_session_data`), with Keycloak's
  one-time token. The verdict and the checked frame reach Keycloak in the
  server callback, never from the browser.
- **Face comparisons:** Keycloak compares the liveness frame with the front of
  the ID and with the photo holding the ID, with the Face Match 1:1 API
  (`/facematch11/compare_images`, `allow_multiple_faces`).
- **B-Trust:** only reads the ID (OCR) and checks that it's authentic. Keycloak
  uploads the front and back only after the face checks pass. The voter's face
  never leaves our network.
- **Injection attacks:** Liveness Plus runs in `PRESENTATION` mode (PAD and
  deepfakes), since injection detection needs Scanovate's own capture client.

## Done

- [x] Mirror the images to our ECR (`133529410358.dkr.ecr.eu-west-1.amazonaws.com/scanovate/*`, same tags and
      digests as Scanovate's Docker Hub images), and pull them from there.
- [x] Confirm the Face Match 1:1 API (`/facematch11/*`, `/faceutils/*`) in its
      OpenAPI description, and that it needs no configuration file or database.
- [x] Read the Liveness Plus API from the service: without Scanovate's client,
      frames must be plain JPEGs in `PRESENTATION` mode; the scan completes on
      the first frame that passes the face quality checks; analytics are only
      sent in SaaS mode.
- [x] Run Liveness Plus in `PRESENTATION` mode, drop the IAD and Valkey
      services, and mount only our `service_config.json`.
- [x] Route only the four API endpoints the capture page uses through
      `keycloak-nginx`: the service's UI, `/docs` and `/metrics` stay internal.
- [x] Drive the Liveness Plus API from the capture page, and take a photo of
      the voter holding the ID instead of a video.
- [x] Check the liveness verdict (`liveness_check_passed`,
      `presentation_attack_check_passed`, `injection_attack_check_passed` not
      `false`) and compare faces with Face Match in Keycloak
      (`face-match-url`, `face-match-min-similarity`).
- [x] Upload only the ID photos to B-Trust, and accept that in the mock server.
- [x] Switch the COMELEC template to `embedded` with the `liveness` face
      capture, with the per-document minimum scores as Face Match similarities,
      and without the B-Trust `liveness_plus` and `biometric_match` rules.

## Testing

- [ ] Run the smoke tests of the guide, including a Face Match comparison of a
      real ID with a selfie.
- [ ] Run the end-to-end test of the guide with a real face and ID, on desktop
      and on phones.
- [x] Install the Scanovate pages in the dev React theme
      (`step-dev keycloak prepare`), which only installed the sign-in pages.
- [x] Add `btrust.sh configure-liveness` to switch a realm to the on-premise
      face checks.
- [ ] Check that a photo, a screen, another person and a covered ID photo are
      rejected with the right message.
- [ ] Tune `face-match-min-similarity` per document type with real documents:
      printed portraits score lower than two selfies.
- [ ] Check that no service or page sends anything outside our network: run
      the profile on a network without internet access, and watch the
      browser's requests.
- [ ] Extend the e2e mock server with a fake Liveness Plus API and Face Match,
      so the flow can be tested end to end without the Scanovate images.

## Scanovate

- [ ] Ask for a capture client (web SDK) that encrypts frames for injection
      attack detection on premise, usable from our own page.
- [ ] Confirm that B-Trust accepts media captured outside its flow (the
      proposed `POST /api/v3/mobile_interaction/{processId}/media`), now with
      only the ID photos, and a flow with only OCR and Document Liveness Plus.
- [ ] Ask whether they ship OCR and document authenticity on premise, so the
      ID photos don't leave our network either.
- [ ] Track the license baked into the PAD image, which expires on 2027-10-04,
      and ask how it's renewed.

## COMELEC

- [x] Resolve D8 of meta#13611: the Keycloak image ships the React login
      themes, and the COMELEC template selects `sequent-ui-voting`, which the
      capture page needs.
- [ ] Set `keycloak_scanovate_liveness_url`,
      `keycloak_scanovate_liveness_secret` and
      `keycloak_scanovate_face_match_url` in the janitor spreadsheets.

## Production

- [ ] Add the `/biometric/liveness/` API routes and the
      `/realms/*/identity-verification/liveness/` block to the production reverse proxy of
      Keycloak.
- [ ] Use a secure random `JWT_SECRET_KEY` and `liveness-secret` from the
      secrets store.
- [ ] Tune the PAD thresholds, the session expiry and `max_active_sessions`
      with real devices and the expected load.
