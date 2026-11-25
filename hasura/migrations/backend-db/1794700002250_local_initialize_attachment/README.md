# Felix3 local Security Officer certificate attachment

The callable phase is in `step/packages/windmill/src/services/signing/local_initialize_attachment.rs`. It registers the responsibly issued public leaf and attaches the resulting staff certificate to the original version-one enrollment in one PostgreSQL transaction. The existing Admin Portal at `http://localhost:3002` is the intended interface. This source slice has no mounted officer page or handler.

## Operator installation

1. Start the ordinary local app. Once the synthetic Post has been published, install the reviewed 2100 marker and protected installation descriptor for that exact tenant, event, Post, publication and `softhsm-test` profile. Install the 2200 permit naming the nominee, a different responsible Security Officer and its finite expiry.
2. Provision the approved synthetic staff root for this event with purpose `staff-signatures`, its fingerprint and its signed current full CRL. This phase supports a directly issued leaf and exactly one CRL for that root. The registry must report a successful CRL observation within 24 hours, and Windmill must validate the certificate path, signing usage and revocation evidence. The operator maintains that trust through the environment's authorized provisioning mechanism.
3. Install 2250 transactionally after 2200. Supply separate protected Unix-socket pools to the optional local composition. The migration creates no login or service grant. The existing portal, setup-device, approval and private assessment grants do not change.

| Installed principal | Exact additional access |
| --- | --- |
| `felix3_local_attachment_officer` | Unix peer of the same name, dedicated to the server-side officer adapter. Requires usage of the setup binding and both private schemas, and execution of `review_csr`, `inspect` and `admit` only. 2200's existing principal check applies, including zero memberships and no direct staff or operational table access. |
| Pinned ordinary registry writer | Uses its existing Windmill staff registration and audit permissions. Add usage of the attachment schema and execution of `finish(uuid,uuid)` only. Requires a distinct local peer connection with no memberships, ownership or write access to the protected setup, approval, request or trust state. The writer cannot admit a transaction. |

The registry writer must already support the ordinary domain service's scoped staff reads, registration insert and transactional audit/outbox work. Its effective privileges must pass `registry_writer`. An administrative Hasura pool fails that test. The two fixed lock helpers run under the migration administrator. They give the ledger API owner access to the required locks without giving it general application-table write privileges. Registry and trust locks use `NOWAIT` and last for the current transaction. The offline source does not establish the full installation's registry/audit ACLs.

All four service entries are in `felix3_initialize_attachment`. In these signatures, `binding` means `felix3_initialize_setup.binding`.

- `review_csr(binding, uuid, uuid, text[], text[], timestamptz)`
- `inspect(binding, uuid, uuid, text[], text[], timestamptz, jsonb)`
- `admit(binding, uuid, uuid, text[], text[], timestamptz, jsonb, text, timestamptz, name, integer, xid8)`
- `finish(uuid, uuid)`

## The two people in the Portal

1. The nominee completes their own Gold setup and explicit registration click. Once the device commits the original generation, the protected handoff identifies its attempt, enrollment ID, CSR PEM SHA-256 and P-256 SPKI SHA-256. A CSR is still awaiting a certificate.
2. The responsible officer signs into their own Portal session, has `SIGNING_CERTIFICATES_REGISTER`, and completes a fresh Gold step-up. **Review certificate request** calls `review_csr` for the installed request. The officer receives the signed public PKCS#10. Its CN and OU identify the nominee and tenant. The approved synthetic staff CA separately issues the leaf for that public key. The operator stages the already-issued public DER and its fingerprint for the officer's review, together with the approved issuer ID, root fingerprint and CRL DER fingerprint. The Portal action creates the staff certificate row.
3. The officer reviews the Post, nominee, CSR digest and leaf fingerprint. Their explicit **Register and attach certificate** action calls `register_and_attach` with the two digests they reviewed. The real staff registry assigns the certificate ID with Post scope, `security-officer` registration and the authenticated officer as registrant. The same transaction calls 1200 and commits both changes together. A failure after registration rolls back the registry insert and its audit. A matching lost-response retry returns the original certificate ID, fingerprints and attachment time after fresh authorization and trust checks.

1200 requires an independent registrant and matching evidence. SQL alone does not authenticate a human click. This Portal integration uses the officer's own fresh decision for each key. The nominee never assumes the officer identity. Approval signing, device liveness and the private CA assessment remain separate actions.

## Normal Portal composition hook

Expose the module only from Windmill's exact `felix3-local-initialize` feature in the full workspace. It uses the signing module's existing OpenSSL policy helpers and the `x509-parser` 0.18 API used by Beyond for wire SPKIs. The module excludes the demo feature. Construct `LocalAttachment::from_installation` from the trusted local descriptor, current committed public key proof, installed ordinary Portal client and pinned registry login. The SQL binding is the existing 2200 type in its original field order. None of these installation values comes from browser JSON.

The following are the proposed **private-to-the-normal-Portal handler contracts**. Mount them on the existing localhost app only after the normal session guard and exact local authorizer are available.

| Portal POST | Browser input | Server action |
| --- | --- | --- |
| `/operational-signing/initialize/setup/officer/csr` | Reviewed request ID and payload digest | Resolve the exact protected binding. Pass the officer's verified ordinary claims and a fresh `CurrentNominee` to `review_csr`. Return only the public CSR and its review context. |
| `/operational-signing/initialize/setup/officer/attach` | Same request and payload digest, reviewed CSR and leaf SHA-256 | Resolve the protected issued leaf. Pass the officer's verified claims, a new `CurrentNominee`, the leaf and reviewed hashes to `register_and_attach`. Return `AttachmentEvidence` only after commit. |

For each action, the nominee's own authorized Portal context calls `current_nominee(tx, verified_nominee_claims, verify_local_post)`. The method calls the real `prepare_initialize_signing`. Its required continuation calls the existing `LocalInitializePost::verify` on that same transaction and returned scope, including the live enabled-signer and Post checks. Complete that transaction before passing the in-process, single-use result to the officer action. The result contains the bound request and attempt, labels, registry username and a deadline capped at 30 seconds. Keep bearer tokens in their own authenticated sessions. Derive the officer from the separately verified subject and tenant. Recheck the current frozen and live Post labels for both actors. Status, polling and page loads must never dispatch the attach action.

The officer verifier's admission binds the exact public pins and locked trust snapshot to the registry connection's actual login, backend PID and full transaction ID. It expires by the earliest authorization, request, permit, certificate or CRL deadline. The writer's only continuation also checks that the new staff row was written in that transaction. An expired or rolled-back admission can be replaced by a fresh review of the same seven public pins. A sealed decision permits evidence recovery only while the original pins and trust stamp still match. A replaced CRL or a later trust observation on a sealed decision requires reconciliation. Preserve decision records when uninstalling.

## Integration boundary and source test

The partial workspace omits the normal local `LocalInitializePost` implementation, Windmill's manifest and registry implementation, and the 2100 installation migration. The candidate bearer guard in this checkout accepts HTTPS issuers, while the local realm named in the handoff uses HTTP on localhost. The normal composition owner must supply the exact verified local guard, per-session handoff, feature/module declaration and two Portal actions. If the full registry service enforces a deeper legacy-only restriction, the scoped reuse needs review there. Preserve that gate. Without those dependencies and a qualifying registry pool, the officer action stays unavailable.

The executable source check runs with `python3 -I -B step/packages/harvest/tests/local_initialize_attachment_source.py -v`. It evaluates the shipped admission, registry, original-generation, Post-label and trust predicates on inert scalar rows and checks the Rust authorization and transaction ordering. Full-workspace acceptance still needs PostgreSQL concurrency and privilege tests, rollback after a refused attachment, loss of the commit response, and signed public CSR/leaf/CRL verification through two genuine Portal sessions.
