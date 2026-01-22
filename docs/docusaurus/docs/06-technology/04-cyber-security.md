---
id: cyber-security
title: Cyber Security and Vulnerabilities
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This page provides supplementary cybersecurity information for the Sequent Voting Platform (SVP), including how to report vulnerabilities and how security issues are handled.

## Reporting security vulnerabilities

To report a suspected vulnerability, email: [security@sequentech.io](mailto:security@sequentech.io)

Please include:

- A clear description of the issue and its potential impact.
- Affected component(s) and version(s) (if known).
- Reproduction steps or a proof of concept (safe and non-destructive).
- Any relevant logs or screenshots, with sensitive data removed.
- Your preferred contact details for follow-up.

If you need to send sensitive details and email is not suitable for your situation, state that in your message and we will propose an alternative channel.

## Scope

We accept security reports affecting:

- Sequent Voting Platform (SVP) components documented in this site.
- Deployment configurations and operational guidance described in these docs.

We generally do not accept reports for:

- Vulnerabilities in third-party infrastructure not operated by Sequent.
- Issues requiring physical access to customer-managed environments (unless explicitly in scope for your deployment).
- Social engineering, phishing, and generic denial-of-service testing against production systems.

## Coordinated disclosure

We support coordinated vulnerability disclosure and ask reporters to:

- Avoid testing against real elections or production environments.
- Avoid accessing, altering, or exfiltrating real voter data.
- Give us a reasonable opportunity to investigate and remediate before public disclosure.

Sequent Tech discloses vulnerabilities transparently when issuing patch releases that address security issues. For details, see the Vulnerability management process below.

## Vulnerability management process

When a report is received at [security@sequentech.io](mailto:security@sequentech.io), we follow a documented triage and remediation process:

1. **Acknowledgement**: we aim to acknowledge receipt within **3 business days**.
2. **Triage**: we assess impact, affected versions, and exploitability, and may request clarifications.
3. **Remediation**: we develop and validate a fix or mitigation.
4. **Release and communication**: we publish the fix in a release and provide advisory information.

Security fixes may be delivered as patch releases when needed. For release cadence and how security releases are handled, see [Product lifecycle and release cadence](../05-reference/03-product_lifecycle_and_release_cadence.md).

## Security advisories and updates

Security-relevant changes are communicated through release notes in this documentation.
See the [Release Notes](../08-releases/).

## Encryption and cryptographic design

For deeper cryptographic and security design references, see [Reference](../05-reference/).