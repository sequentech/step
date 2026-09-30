<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Scanovate on-premise TODO

Follow-ups of the Scanovate on-premise integration. See
[Scanovate On-Premise Services](docs/docusaurus/docs/integrations/scanovate_on_premise_guide.md).

## Once we have access to the images

- [x] Get Docker Hub access from Scanovate for the Liveness Plus images.
- [ ] Get AWS ECR access for the Face Match image
      (`495947449196.dkr.ecr.eu-central-1.amazonaws.com/ngfacematch:version_3.9.0_ba58397_79`),
      and store the credentials in the team's password manager.
- [x] Check that Liveness Plus reads its config from `/app/config`. Our mount
      replaces the whole directory, including the image's `default_ui.json`
      and `locales/default`.
- [x] Move Face Match and Valkey to their own `scanovate-face-match` compose
      profile, so the liveness services start without ECR access.
- [ ] Run the smoke tests of the guide: `/alive`, the liveness UI with a
      camera, and `GET /facematch1N/get_groups` with `x-company-id: sequent-dev`.
- [ ] Try the flow from Keycloak with the mock server
      ("Trying the flow from Keycloak" in the guide).
- [ ] Check that no service, nor the liveness client in the browser, sends
      anything outside our network (e.g. analytics): run the profile on a
      network without internet access, and watch the browser's requests.
- [x] Check whether any container needs `security_opt: [seccomp=unconfined]`:
      none of the liveness ones do. Idle, PAD takes ~5.4 GB, IAD ~2.9 GB and
      Liveness Plus ~0.5 GB, so 14 GB is enough without Face Match.
- [x] Create a liveness session through `/biometric/liveness/create_session`:
      it returns the `sequent` texts and the `sequent_ui` theme.
- [ ] Get the Face Match request and response schemas from its OpenAPI
      description (`/docs` or `/openapi.json`), and document them.
- [ ] Check whether the Face Match image also serves a 1:1 API, like B-Trust's
      standalone `POST /face_match` (`SUPPORT_1_TO_MANY` defaults to `false`,
      which suggests it does), and whether it finds the portrait in a photo of
      the whole ID.
- [ ] Confirm whether `DELETE /delete_group` deletes the templates of the
      group.

## Keycloak integration

Agreed design:

- **ID front/back:** our own capture page keeps guiding the voter with the
  `id-capture` WASM analyzers (centering, distance, alignment, glare, blur,
  stillness) until the photo is readable. Liveness Plus doesn't capture
  documents.
- **Selfie and liveness:** the Liveness Plus iframe replaces our face step. Its
  UI captures the frames (injection detection needs its own client) and gives
  the voter its own face guidance, so our `FaceAnalyzer` guidance isn't used
  for this step. The verdict comes from the server callback, never from
  `postMessage`.
- Our page releases the camera before loading the iframe: two pages holding
  the same camera is unreliable, especially on mobile.

- The "video holding the ID" step is dropped.
- The ID photos stay in the browser and are uploaded at the end with the
  liveness picture: B-Trust has no per-photo check.
- The picture of the liveness callback is uploaded to B-Trust as
  `face_image`, for the match against the ID portrait.

- [x] Add a liveness face capture to `scanovate-authenticator`
      (`face-capture=liveness`) that embeds the Liveness Plus UI in an iframe
      (`allow="camera *; microphone *"`), with a one-time `token` and the
      B-Trust process id as `case_id`.
- [x] Protect the token verification and callback endpoints with a `secret`
      query parameter that the browser never sees (`liveness-secret`).
- [x] Set `send_video_in_results` to `false`: the callback would carry the
      video in base64, and we don't store it.
- [x] Add a Sequent `ui_theme` (`sequent_ui`) and `sequent` texts (en, es) for
      the Liveness Plus UI.
- [x] Allow the liveness origin in Keycloak's Content Security Policy
      (`frame-src`).
- [x] Add the token verification endpoint
      (`GET /realms/{realm}/scanovate/liveness/verify`) and set
      `onprem.token_verification_url`.
- [x] Add the callback endpoint
      (`POST /realms/{realm}/scanovate/liveness/callback`) and set
      `onprem.callback_url`. Never trust the browser's `postMessage` outcome.
- [x] Mount our own Liveness Plus config in the dev container.
- [x] Unit tests first (TDD), then the implementation, and update the guides.
- [ ] Confirm with Scanovate that B-Trust accepts media captured outside its
      flow (the proposed `POST /api/v3/mobile_interaction/{processId}/media`),
      now with the liveness `face_image` and without a video.
- [ ] Ask Scanovate whether the liveness UI can run full page with a return
      URL, or be driven by our own UI through a documented client API or SDK,
      as alternatives to the iframe.
- [x] Decide the public URL: `https://<keycloak host>/biometric/`, same origin
      as Keycloak (`CLIENT_BASE_URL_PREFIX=biometric/`), routed by
      `keycloak-nginx` in dev, which also blocks
      `/realms/*/scanovate/liveness/`.
- [x] Check `CLIENT_BASE_URL_PREFIX` with the image: the service itself serves
      everything under `/biometric/liveness/`, so the proxy passes the path
      unchanged (it doesn't strip it). The service sends no framing headers
      (`X-Frame-Options`, CSP).
- [ ] Add the `/biometric/` route and the `/realms/*/scanovate/liveness/`
      block to the production reverse proxy of Keycloak.
- [ ] Try the flow end to end with the images and a real camera, on desktop
      and on phones (the iframe is full screen there), and check that the
      `sequent_ui` theme renders as expected.
- [ ] Optionally, reject enrollments whose face is already enrolled in the
      election event with Face Match (`search_image` on the event's group, then
      `insert_image`), and decide how company ids and groups map to tenants and
      election events.
- [ ] Delete Face Match templates when voter data is deleted.
- [ ] Match the liveness picture against the ID portrait with Face Match (1:1),
      so the voter's face doesn't go to B-Trust, and drop the
      `biometric_match` rule for those realms.

## Keeping data in our network

With the delivered services, only liveness and face matching run on our
premises. OCR and document authenticity stay in B-Trust, so the ID photos
leave our network.

- [ ] Ask Scanovate whether they ship OCR and document authenticity
      (`document_liveness_plus`) on premise.
- [ ] Ask Scanovate whether a B-Trust flow can run only OCR and document
      authenticity on uploaded ID photos, without its own liveness and face
      match.
- [ ] Decide whether realms that need it use `save-option` `do_not_save`.
- [ ] Extend the e2e mock server with a fake Liveness Plus (UI posting
      `done` and the callbacks), so the flow can be tested end to end without
      the Scanovate images.

## Production

- [ ] Mirror the images to our own registry.
- [ ] Track the licenses baked into the images: PAD's expires on 2027-10-04 and
      IAD's on 2027-08-30. Ask Scanovate how they're renewed.
- [ ] Serve Liveness Plus over HTTPS, and make sure proxies don't buffer
      Server-Sent Events (`/sse/events`).
- [ ] Use a secure random `JWT_SECRET_KEY` from the secrets store.
- [ ] Persist, back up and restrict access to the Valkey volume.
- [ ] Tune the PAD thresholds, the session expiry and `max_active_sessions`
      with real devices and the expected load.
