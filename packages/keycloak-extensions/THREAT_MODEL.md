<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# keycloak-extensions threat model

keycloak-extensions holds the Java SPI providers that Sequent loads into the Keycloak 26.6.1 server (`packages/Dockerfile.keycloak`): authenticators, form actions, required actions, action-token handlers, custom realm REST resources, protocol mappers, an event listener, a truststore provider, email and SMS senders, and the login theme. They run inside the Keycloak JVM with full access to the realm, user and credential models. Voters log in and enrol through them in the `tenant-<tenant>-event-<event>` realms, and election managers log in through them in the `tenant-<tenant>` realms. Hasura and harvest trust the claims in the JWTs that Keycloak issues, so a flaw here sits upstream of all their authorization checks: whoever controls a voter session can cast that voter's ballot. A Maven profile also bundles private modules from `../../beyond/packages/keycloak-extensions`; this model does not cover them. See the [system threat model](../../THREAT_MODEL.md).

Paths below shorten `src/main/java/sequent/keycloak/<module package>/` to `.../`.

## Assets

- **Voter and admin credentials**: passwords, message-OTP credentials, security-question answers and voter-secret attributes encrypted with keys derived from `MASTER_SECRET`. Confidentiality and integrity matter: an attacker who holds them can cast a ballot or act as an admin.
- **One-time proofs**: OTP codes, login and reset links and temporary passwords. Each works as a bearer credential until it is used or expires. Confidentiality, expiry and single use matter.
- **Authentication-session state**: the state that carries a login or enrollment flow between steps. Integrity matters.
- **Hasura claims in tokens**: `x-hasura-tenant-id`, `x-hasura-area-id`, `x-hasura-election-event-id`, `x-hasura-authorized-election-ids` and `permission_labels`. Their integrity decides which rows and which elections a token reaches.
- **Identity-verification state**: `emailVerified` and the `sequent.read-only.*` attributes, which gate voter eligibility. Integrity matters.
- **Service and provider secrets**: `KEYCLOAK_CLIENT_SECRET` and the service tokens obtained with it, `MASTER_SECRET`, `TWILIO_*`, `AMQP_ADDR`, the AWS credential chain and the reCAPTCHA secret. Confidentiality matters.
- **Voter personal data**: names, ID numbers, date of birth, email, phone, `login_hint__*` values, applicant data and client-certificate subjects. Confidentiality matters (GDPR).
- **Electoral-log events**: Keycloak events that windmill writes to the append-only immudb log. Integrity, completeness and confidentiality matter, because entries cannot be erased.
- **Per-event CA trust anchors**: these decide which client certificates authenticate voters. Integrity matters.
- **Login availability**: voters must be able to log in during the voting window.

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| Custom realm REST resources `ivr-config` (`ivr-config-provider`) and `manual-verification` (`conditional-authenticators`) | Any client of the Keycloak hostname; meant for internal services | untrusted network, internal service intended | See keycloak-extensions-T1 |
| `POST /realms/{realm}/redirect-provider/redirect` (`message-otp-authenticator/.../idp_initiated_sso/SamlRedirectProvider.java`) | Browser after IdP-initiated SAML | untrusted | Public by design. `RelayState` is checked with `RedirectUtils.verifyRedirectUri` against the `vp-sso` client |
| `/realms/{realm}/login-actions/action-token` for `OTLActionToken`, `LoginBridgeActionToken` and `ManualVerificationToken` | Anyone holding a link | untrusted | See keycloak-extensions-T2 |
| Browser, registration, reset-credentials, first-broker-login and direct-grant flows: authenticators and form actions in `message-otp-authenticator`, `voter-enrollment`, `security-question-authenticator`, `conditional-authenticators` and `idp-linking-authenticator` | Anyone on the internet | untrusted | CSPRNG OTP with a TTL and constant-time comparison. Dummy hashing, throttling and brute-force checks on the attribute-based logins and enrollment lookups |
| `voter-enrollment/.../LoginHintAuthorizationRequestFilter.java` (`@PreMatching` on the auth and registration endpoints) | Any browser | untrusted | `validateRawQuery` enforces strict decoding, duplicate, count (`MAX_HINT_COUNT`) and length limits. `LoginHintPrefill` never prefills credential, hidden or read-only fields and rejects a changed locked hint (`findModifiedLockedHints`) |
| Required actions `MFAMethodSelector`, `ResetEmailOTPRequiredAction`, `ResetMobileOTPRequiredAction`, `ResetMessageOTPRequiredAction`, `VerifyOTPEmailRequiredAction`, `SecurityQuestionRequiredAction`, also through `kc_action` | Logged-in user | authenticated voter | Act on the session's own user. The OTP goes to the new contact. Resend timer, receiver-reuse cap and country-code list in `BaseResetMessageOTPRequiredAction` |
| Protocol mappers `AuthorizedElectionsUserAttributeMapper` and `HasuraMultivaluedUserAttributeMapper` (`conditional-authenticators/.../protocol/oidc/mappers/`) | Keycloak token issuance | internal service | Claims are computed server-side from the realm, the user's attributes and Hasura |
| Client-certificate header (`ssl-client-cert` by default) read by `conditional-authenticators/.../X509CertClassifierAuthenticator.java` | Reverse proxy | internal service | `X509UserResolutionAuthenticator` extends Keycloak's `X509ClientCertificateAuthenticator`. The reverse proxy sets the header (deployment) |
| `custom-event-listener/.../CustomEventListenerProvider.java` `onEvent` | Keycloak event pipeline | internal service | Tenant and event from the realm name. Serialized with Jackson into a Celery message |
| `url-truststore-provider/.../UrlTruststoreProviderFactory.java` (per-event CA PEM from harvest) | Keycloak | internal service | Network isolation (deployment) |
| Responses from harvest and Hasura | harvest, Hasura | internal service | Parsed with Jackson `ObjectMapper`, with no polymorphic typing |
| `message-otp-authenticator/.../smart_link/SmartLinkAuthenticatorFactory.java` `postInit` (realm post-create hook) | Keycloak, when any realm is created | internal service | Adds the smart-link flow to the new realm through `SmartLink.realmPostCreate` |
| Authenticator, required-action and mapper configuration in realm JSON | Realm admins, windmill and step-cli | authenticated admin | Keycloak admin permissions |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| keycloak-extensions-T1 | Spoofing | A custom realm REST resource is called by someone other than its intended caller. | Bearer-token and realm-role checks inside the resources; network isolation of internal-only paths (deployment). `SamlRedirectProvider` is public by design and validates `RelayState` | Partial |
| keycloak-extensions-T2 | Spoofing | An action-token link is forged, replayed, used after its intended lifetime, used in a realm it was not meant for, or used to send the browser to an untrusted URL. | Keycloak signs and verifies action tokens. Handlers of single-use tokens refuse reuse (`canUseTokenRepeatedly`) | Partial |
| keycloak-extensions-T3 | Tampering | A user alters the state that a multi-step login or enrollment flow carries between steps. | Keycloak keeps authentication-session state on the server | Not verified |
| keycloak-extensions-T4 | Spoofing | Online guessing of an OTP code, password, security answer or attribute tuple. | OTP codes come from Keycloak's `SecretGenerator`, carry a TTL and are compared in constant time. `message-otp-authenticator/.../forgot_password/MultiAttributeCredentialResolver.java`: dummy hashing, `maxCandidates` and a per-tuple failure throttle in `SingleUseObjectProvider`. Dummy hashing and brute-force checks on enrollment lookups (`voter-enrollment`) | Partial |
| keycloak-extensions-T5 | Spoofing | An alternative login or recovery path lets a user in without the second factor. | `message-otp-authenticator/.../forgot_password/MultiAttributePasswordDirectGrantAuthenticator.java` shares the browser resolver. `.../forgot_password/ResetMessageOTP.java` and `ResetOTP.java` run inside the reset-credentials flow. Which flows apply is set in the realm templates | Not verified |
| keycloak-extensions-T6 | Denial of service | Self-service password reset is abused against a voter's account or to learn which accounts exist. | Reset needs a matching username and email, compared in constant time, with optional reCAPTCHA (fail closed). Unknown users get the same response as known ones | Partial |
| keycloak-extensions-T7 | Information disclosure | Credentials, one-time proofs or service secrets reach logs, Keycloak events or the electoral log, where their readers can use them. | `message-otp-authenticator/.../forgot_password/EncryptedAttributeCredential.java` wipes key buffers and never logs key material. `MultiAttributeCredentialResolver` logs counts and a hashed tuple key, not submitted values | Partial |
| keycloak-extensions-T8 | Information disclosure | `AuthorizedElectionsUserAttributeMapper` logged the client secret and tokens, did not URL-encode the token form, and returned error bodies as tokens. | `conditional-authenticators/.../protocol/oidc/mappers/AuthorizedElectionsUserAttributeMapper.java` `authenticate`, changed by the fix | Open on release/10.0 (fix in sequentech/step#3522) |
| keycloak-extensions-T9 | Information disclosure | Voter personal data leaves Keycloak beyond what the receiver needs. | Contacts are masked on login pages (`obscureEmail`, `obscurePhoneNumber`) | Partial |
| keycloak-extensions-T10 | Elevation of privilege | Hasura claims in a token grant more than the user is entitled to. | Claims are computed server-side by protocol mappers | Partial |
| keycloak-extensions-T11 | Tampering | A response from an internal service is spoofed or altered in transit. | Responses are parsed with Jackson. Network isolation (deployment) | Partial |
| keycloak-extensions-T12 | Spoofing | A client forges the client-certificate header that the reverse proxy is meant to set. | The reverse proxy owns the header (deployment). The header name is set in authenticator config | Not verified |
| keycloak-extensions-T13 | Tampering | First-broker-login links an external identity to the wrong existing voter. | `idp-linking-authenticator/.../CustomAttributeIdpLinkingAuthenticator.java` `authenticate` needs exactly one matching user and fails when the match is ambiguous. The IdP claim it matches on is operator configuration | Not verified |
| keycloak-extensions-T14 | Repudiation | Keycloak events are lost or altered on their way to the electoral log. | Events are serialized with Jackson and published as persistent messages to a durable queue. Network isolation (deployment) | Partial |
| keycloak-extensions-T15 | Denial of service | SMS or email sending is abused for toll fraud or spam, or login is made unavailable. | Resend timers, a receiver-reuse cap and a country-code allow-list on OTP sending (`message-otp-authenticator`) | Partial |
| keycloak-extensions-T16 | Tampering | Script or markup is injected into Keycloak login pages or into official emails. | FreeMarker HTML auto-escaping in the theme templates. Text emails use `output_format="plainText"`. `kcSanitize(...)` for rich message bundles (`sequent-theme`, `message-otp-authenticator/src/main/resources/theme*`) | Partial |
| keycloak-extensions-T17 | Elevation of privilege | Identity-verification state (`emailVerified`, `sequent.read-only.*`) is set for a user without the proof it stands for, so an unverified person passes an eligibility gate. | User-profile permissions keep `sequent.read-only.*` admin-edit only (realm configuration) | Not verified |
| keycloak-extensions-T18 | Tampering | Provider configuration in a production realm is set so that a check is weakened. | `UrlTruststoreProviderFactory.init` rejects an unknown `hostname-verification-policy` | Not verified |

## Assumptions

- **Keycloak core** keeps realms isolated, verifies action-token signatures, expires authentication sessions and enforces the declarative user profile.
- **Realm templates** (step-cli, windmill imports, production templates) enable Keycloak brute-force detection.
- **Operators** make the tenant, area, election and permission-label attributes and every `sequent.read-only.*` attribute admin-edit only in the user-profile configuration.
- **Realm templates** keep development settings off, redirect URIs narrow (including the `vp-sso` client), the same factors on direct-grant and browser flows, and reCAPTCHA where reset is exposed.
- **Election-event realms** are created only by privileged automation.
- **The reverse proxy** strips or overwrites the client-certificate header on every request and terminates TLS.
- **The network** between Keycloak and harvest, Hasura and RabbitMQ is private.
- **harvest and Hasura** apply their own authorization to the requests they serve.
- **harvest** authenticates the enrollment-verification requests it receives and protects the applicant data it stores.
- **windmill** treats electoral-log message bodies as data, and only authorized roles can read the electoral log.
- **Keycloak application logs and stored events** are access-controlled.
- **Operators** select only production email and SMS senders outside development.
- **The private `beyond` modules** bundled through the Maven profile get their own review.

## Review focus

1. **Custom realm REST resources and action-token handlers.**
2. **State carried between the steps of login and enrollment flows.**
3. **Secrets and personal data in logs, Keycloak events and the electoral log.**
4. **Where identity-verification state is set.**
5. **The OTP and one-time-link lifecycle.**
6. **Protocol mappers that feed Hasura permissions.**
7. **Factor parity across every login and recovery flow** in the realm templates.
8. **Enrollment verification with harvest.**
9. **Calls to internal services**, and how far the client-certificate header is trusted.
10. **Rate limits on SMS and email sending.**
11. **First-broker-login linking rules** and the IdP claims they rely on.
12. **Output encoding in login pages and emails.**
13. **Provider configuration in production realms.**
