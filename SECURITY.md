<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Security Policy

Sequent Tech welcomes reports of security vulnerabilities in the Sequent Voting
Platform, including the `Sequent Voting Platform - uniWAHL Version`, from
security researchers, consumers, election auditors and the general public.

Our
[Vulnerability Disclosure Policy](https://docs.sequentech.io/docusaurus/main/docs/technology/vulnerability-disclosure-policy)
is the authoritative statement of what is in scope, how to report, what you can
expect from us, what we ask of you and our safe harbor. Its source is in this
repository:
[docs/docusaurus/docs/06-technology/06-vulnerability-disclosure-policy.md](docs/docusaurus/docs/06-technology/06-vulnerability-disclosure-policy.md).

## Reporting a vulnerability

Please **do not** report security vulnerabilities through public GitHub issues,
discussions or pull requests.

Email [security@sequentech.io](mailto:security@sequentech.io). For sensitive
details, encrypt your report to the Sequent security team OpenPGP key:

- Fingerprint: `DB6B 035E 872A B5DA 90C7  194A 60D3 CBE3 0BAC 91ED`
- Download:
  https://keys.openpgp.org/vks/v1/by-fingerprint/DB6B035E872AB5DA90C7194A60D3CBE30BAC91ED

Encryption is recommended but never required. Where possible, include the
affected components and versions, a description of the issue and its impact,
reproduction steps or a safe proof of concept, and whether you want to be
credited.

Never test against real elections, production environments or real voter data:
use a local or dedicated test deployment.

The same contacts are published in machine-readable form
([RFC 9116](https://www.rfc-editor.org/rfc/rfc9116)) at
https://docs.sequentech.io/docusaurus/main/.well-known/security.txt.
