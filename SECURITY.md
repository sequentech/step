<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Security Policy

Sequent Tech welcomes reports from security researchers, consumers, election
auditors and the general public. Our
[Vulnerability Disclosure Policy](https://docs.sequentech.io/docusaurus/main/docs/technology/vulnerability-disclosure-policy)
explains what is in scope, how we handle your report, the safe harbor we offer
and how we credit reporters. It is the authoritative version; this file only
summarizes how to reach us.

## Reporting a vulnerability

**Please do not report security vulnerabilities through public GitHub issues,
discussions or pull requests.**

Email [security@sequentech.io](mailto:security@sequentech.io) instead. For
sensitive details, encrypt your report to the Sequent Security Team OpenPGP key:

- Fingerprint: `DB6B 035E 872A B5DA 90C7  194A 60D3 CBE3 0BAC 91ED`
- Download: <https://keys.openpgp.org/vks/v1/by-fingerprint/DB6B035E872AB5DA90C7194A60D3CBE30BAC91ED>

Encryption is recommended but not required. Where possible, include the
affected component and version, reproduction steps or a safe proof of concept,
the potential impact, and whether you wish to be publicly credited.

We acknowledge reports within 5 business days.

**Never test against real elections or production environments.** Use a local
or dedicated test deployment; the
[Try It Out](https://docs.sequentech.io/docusaurus/main/docs/try_it_out/full-standalone-deployment)
section explains how to run the platform yourself.

## Machine-readable contact

Our [RFC 9116](https://www.rfc-editor.org/rfc/rfc9116) `security.txt` is
published at
<https://docs.sequentech.io/docusaurus/main/.well-known/security.txt>. Its
signed master copy is
[`docs/docusaurus/static/.well-known/security.txt`](docs/docusaurus/static/.well-known/security.txt).
