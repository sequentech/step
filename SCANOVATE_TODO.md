<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Scanovate on-premise TODO

Follow-ups of the Scanovate on-premise integration. See
[Scanovate On-Premise Services](docs/docusaurus/docs/integrations/scanovate_on_premise_guide.md).

## Agreed design

On premise we only get the Scanovate APIs: their own UIs (the iframes) aren't
available, and nothing is sent to any third party: B-Trust, Scanovate's cloud,
isn't used.

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
- **Reading the ID:** Keycloak sends the front and back, only after the face
  checks pass, to the OCR service (`/single_image_ocr`), which reads passports
  and validates their MRZ check digits.
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
- [x] Read the ID with the on-premise OCR service, and remove B-Trust: its
      client, the return endpoint, the `execution-mode`, `face-capture`
      (photo), `save-option` and `link-params` settings, and its mock.
- [x] Make the capture page liveness only, without the selfie and the video
      recorder of the photo face capture.
- [x] Switch the COMELEC template to the on-premise services, with the
      per-document minimum scores as Face Match similarities and the passport
      rules on the OCR fields.
- [x] Run the OCR service in the `scanovate` profile of the dev container, and
      point the dev realm and the sample election event to the services.

## Testing

- [ ] Run the smoke tests of the guide, including a Face Match comparison of a
      real ID with a selfie.
- [ ] Run the end-to-end test of the guide with a real face and ID, on desktop
      and on phones.
- [x] Install the Scanovate pages in the dev React theme
      (`step-dev keycloak prepare`), which only installed the sign-in pages.
- [x] Add `configure-realm.sh configure` to point a realm to the on-premise
      services.
- [ ] Check that a photo, a screen, another person and a covered ID photo are
      rejected with the right message.
- [ ] Tune `face-match-min-similarity` per document type with real documents:
      printed portraits score lower than two selfies.
- [ ] Check that no service or page sends anything outside our network: run
      the profile on a network without internet access, and watch the
      browser's requests.
- [ ] Check that the OCR service reads real Philippine passports, and that
      the rules on its fields hold.
- [ ] Extend the e2e mock server with a fake Liveness Plus API, Face Match and
      OCR, so the flow can be tested end to end without the Scanovate images.

## Scanovate

- [ ] Ask for a capture client (web SDK) that encrypts frames for injection
      attack detection on premise, usable from our own page.
- [ ] Ask how the OCR service reads the Philippine driver's license, PhilSys
      ID, IBP ID and Seafarer's Book: only its `regula` type could, through a
      Regula Document Reader server we don't have.
- [ ] Track the license baked into the PAD image, which expires on 2027-10-04,
      and ask how it's renewed.

## COMELEC

- [x] Resolve D8 of meta#13611: the Keycloak image ships the React login
      themes, and the COMELEC template selects `sequent-ui-voting`, which the
      capture page needs.
- [ ] Set `keycloak_scanovate_ocr_url`, `keycloak_scanovate_liveness_url`,
      `keycloak_scanovate_liveness_secret` and
      `keycloak_scanovate_face_match_url` in the janitor spreadsheets.
- [ ] Decide how voters enroll with the documents the OCR service can't read
      (only the passport is configured): a Regula server, manual entry with
      officer review, or passport only.

## Production

- [ ] Add the `/biometric/liveness/` API routes and the
      `/realms/*/identity-verification/liveness/` block to the production reverse proxy of
      Keycloak.
- [ ] Use secure random `JWT_SECRET_KEY`s (Liveness Plus and OCR) and
      `liveness-secret` from the secrets store.
- [ ] Tune the PAD thresholds, the session expiry and `max_active_sessions`
      with real devices and the expected load.

## Out of scope for now

- Document authenticity (photocopies, screens, edited documents): none of the
  on-premise images checks it.
