---
id: keycloak-extensions
title: Keycloak extension tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

From `packages/keycloak-extensions` (the Maven reactor root), run `mvn test`
using Java 17 and Maven 3.9.9. This directory resolves the sibling extension
modules together. Fetch dependency
artifacts before using `mvn -o test` in a network-isolated worker. Surefire can
resolve extra provider artifacts that `dependency:go-offline` alone misses.

Existing suites cover OTP/reset flows, credential parsing, enrollment, IDP
linking, token claims, email/SMS boundaries, truststores and theme resources.
Mocks replace Keycloak/service boundaries and use synthetic identities. Auth-note
condition tests check literal equality, whole-value regex matching, invalid
patterns, blank/missing values, negation and absent configuration/session state.
A missing session or note must not activate a negated flow.

These unit suites do not replace a running Keycloak login flow, provider-loading
compatibility checks, or real mail/SMS/broker delivery. Security-question and
certificate-based flows need additional focused fixtures; passing reactor tests
must not be interpreted as full extension coverage. No coverage percentage is
published without an instrumented source inventory and comparable base run.
