<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Scanovate on-premise TODO

Follow-ups of the Scanovate on-premise integration. See
[Scanovate On-Premise Services](docs/docusaurus/docs/integrations/scanovate_on_premise_guide.md).

## Agreed design

We integrate the services as Scanovate's own on-premise docker-compose kit
shows, and nothing is sent to any third party: B-Trust, Scanovate's cloud,
isn't used.

- **Capture:** Keycloak's capture page embeds, in iframes on Keycloak's origin
  (`/biometric/ocr/` and `/biometric/liveness/`), first OCR's own capture page
  and then Liveness Plus's, each with a one-time token issued by Keycloak. The
  pages only tell ours that they are done.
- **Results:** each service checks its token with Keycloak
  (`identity-verification/<ocr|liveness>/verify`) and posts its result to
  Keycloak (`identity-verification/<ocr|liveness>/callback`), server to
  server, with a secret the browser never sees.
- **Reading the ID:** OCR reads passports and validates their MRZ check
  digits. Its result (completed, the side read with `success`, with the
  portrait) carries the fields the rules read.
- **Liveness:** Liveness Plus runs in `full` mode, with PAD (presentation
  attacks and deepfakes) and IAD (injection attacks); Keycloak requires the
  liveness, presentation attack and injection attack checks to pass.
- **Document liveness:** Keycloak sends OCR's `input_image` to the Document
  Liveness service (`/check_liveness`, screen replay, printed copy, portrait
  substitution and digital manipulation) and requires every pipeline to answer
  `OK` with a probability of at least 0.5, as Scanovate's onboarding guide
  recommends.
- **Face comparison:** Keycloak compares the OCR portrait with the live face,
  with the Face Match 1:1 API (`/facematch11/compare_images`), without
  `allow_multiple_faces`; only the holding photo allows several faces.
- **Theme:** both pages use `sequent_ui.json`, the image's default UI theme
  with the colours and fonts of the voter login theme.

## Done

- [x] Mirror the images to our ECR (`133529410358.dkr.ecr.eu-west-1.amazonaws.com/scanovate/*`, same tags and
      digests as Scanovate's Docker Hub images), and pull them from there.
- [x] Confirm the Face Match 1:1 API (`/facematch11/*`, `/faceutils/*`) in its
      OpenAPI description, and that it needs no configuration file or database.
- [x] Embed Scanovate's OCR and Liveness Plus pages instead of our own capture
      page: remove the camera UI, the `id-capture` WASM guidance from
      keycloak-ui, the upload endpoint and Keycloak's calls to
      `/single_image_ocr`.
- [x] Verify the services' tokens and receive their callbacks in Keycloak, for
      OCR and Liveness Plus, with one shared `liveness-secret`.
- [x] Cover injection attacks: run Liveness Plus in `full` mode with the IAD
      server, and require `injection_attack_check_passed`.
- [x] Proxy `/biometric/ocr/` and `/biometric/liveness/` through
      `keycloak-nginx` (pages, assets, unbuffered server-sent events), block
      their docs, metrics and session listings, and block
      `/realms/*/identity-verification/` for browsers.
- [x] Allow the pages' origin in Keycloak's CSP `frame-src` on the capture
      page.
- [x] Theme both pages with the voter login theme (`sequent_ui.json`).
- [x] Compare the OCR portrait with the live face (one Face Match comparison),
      and check the rules on what OCR read.
- [x] Read the ID with the on-premise OCR service, and remove B-Trust: its
      client, the return endpoint, the `execution-mode`, `face-capture`
      (photo), `save-option` and `link-params` settings, and its mock.
- [x] Switch the COMELEC template to the on-premise services, with the
      per-document minimum scores as Face Match similarities and the passport
      rules on the OCR fields.
- [x] Run OCR, Liveness Plus, PAD, IAD and Face Match in the `scanovate`
      profile of the dev container, and point the dev realm and the sample
      election event to the services.

## Testing

- [ ] Tune `document-liveness-min-probability` and
      `document-liveness-calibration` with real passports, passports shown on
      phones and monitors, and colour prints.

- [ ] Run the smoke tests of the guide, including a Face Match comparison of a
      real ID with a selfie.
- [ ] Run the end-to-end test of the guide with a real face and ID, on desktop
      and on phones.
- [x] Install the Scanovate pages in the dev React theme
      (`step-dev keycloak prepare`), which only installed the sign-in pages.
- [x] Add `configure-realm.sh configure` to point a realm to the on-premise
      services.
- [ ] Check that a photo, a screen, a virtual camera, another person and a
      covered ID photo are rejected with the right message.
- [ ] Tune `face-match-min-similarity` per document type with real documents:
      printed portraits score lower than two selfies.
- [ ] Check that no service or page sends anything outside our network: run
      the profile on a network without internet access, and watch the
      browser's requests (the pages' analytics only load with
      `ENABLE_ANALYTICS`, which we never set).
- [ ] Check that the OCR service reads real Philippine passports, and that
      the rules on its fields hold.
- [ ] Extend the e2e mock server with fake OCR and Liveness Plus pages (token
      check, callbacks, `postMessage`) and Face Match, so the flow can be
      tested end to end without the Scanovate images.
- [ ] Measure the memory of the IAD server, and the session sizing of OCR's
      own capture page.

## Scanovate

- [x] Ask for document liveness on premise: Scanovate delivered
      `document-liveness-service:release-3.5.5`, mirrored to our ECR, and its
      onboarding guide.
- [ ] Ask Scanovate to make OCR's preview mirroring follow `video_config`:
      its client mirrors on every desktop browser (`mirror_view ||
      !isMobile()`), so `video_config_no_mirror` doesn't change it there.
- [ ] Track the license baked into the Document Liveness image, which expires
      on 2027-10-25.
- [ ] Ask how the OCR service reads the Philippine driver's license, PhilSys
      ID, IBP ID and Seafarer's Book: only its `regula` type could, through a
      Regula Document Reader server we don't have.
- [ ] Track the license baked into the PAD image, which expires on 2027-10-04,
      and ask how it's renewed.

## COMELEC

- [x] Resolve D8 of meta#13611: the Keycloak image ships the React login
      themes, and the COMELEC template selects `sequent-ui-voting`, which the
      capture page needs.
- [ ] Set `keycloak_scanovate_liveness_url`,
      `keycloak_scanovate_liveness_secret` and
      `keycloak_scanovate_face_match_url` in the janitor spreadsheets.
- [x] Stop writing the ignored `ocr-url` and `capture-sides` in the realm
      templates (sequent-core presets, COMELEC janitor template, step-cli
      sample data), and drop `keycloak_scanovate_ocr_url`.
- [ ] Decide how voters enroll with the documents the OCR service can't read
      (only the passport is configured): a Regula server, manual entry with
      officer review, or passport only.

## Production

- [ ] Add the `/biometric/ocr/` and `/biometric/liveness/` routes (no
      buffering, 10 MiB bodies), their docs/metrics blocks and the
      `/realms/*/identity-verification/` block to the production reverse proxy
      of Keycloak.
- [ ] Set the callback and token verification URLs of both services'
      `service_config.json` to the internal Keycloak, with the production
      `liveness-secret`, and mount `sequent_ui.json` in both.
- [ ] Use secure random `JWT_SECRET_KEY`s (Liveness Plus and OCR) and
      `liveness-secret` from the secrets store.
- [ ] Tune the PAD thresholds, the session expiry and `max_active_sessions`
      with real devices and the expected load.

## Out of scope for now

- Document authenticity beyond document liveness (security features, the
  chip): OCR's passport checks don't verify them.
