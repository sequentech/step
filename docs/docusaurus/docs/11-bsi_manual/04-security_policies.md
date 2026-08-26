---
id: bsi_security_policies
title: "BSI User Manual — Part 4: Security Policies"
---
<!--
-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Part 4: Security Policies and Responsibilities

*Sequent Voting Platform – uniWAHL Version | User Manual*

## 4.1 Organizational Security Requirements

The TOE operator is responsible for ensuring the following organizational conditions are in place before and during operation:

- Physical access to the server infrastructure is restricted to authorized personnel only
- The airgap environment is maintained — no unauthorized network connections to or from the TOE during operation
- All key ceremony participants are identified, authorized, and present in person during cryptographic operations
- Election data and audit logs are backed up after every election and stored securely
- Any suspected security incident is reported immediately to Sequent through the support channel defined in your service agreement

## 4.2 Access Control Policies

User roles within the TOE are managed via Keycloak. The following rules apply:

- Each user must be assigned the minimum role necessary to perform their function
- Shared accounts are not permitted — every user must have an individual identity
- Credentials must not be shared between users or written down in insecure locations
- Accounts must be deactivated promptly when a user leaves the organization or their role changes
- Keycloak enforces multi-factor authentication (MFA) for all administrative roles, using time-based one-time passwords (TOTP) per IETF RFC 6238

For role definitions, see the Security Target (Section 6.1.1).

## 4.3 Audit Logging

The bulletin board (`b4`) provides an immutable, append-only log of all election events. Its design ensures that no record can be modified or deleted once written.

The following logs must be preserved for the full duration of the election and for the minimum retention period required by applicable electoral regulation:

- Operator terminal session transcripts (for example, captured with `script(1)` or an equivalent mechanism) for all `cli step ...` commands executed during election setup and ceremonies

Logs must not be modified, deleted, or transferred outside the airgap environment without authorization from the election organizer and Sequent.

## 4.4 Incident Response

This section describes the incident response process applicable to the TOE during operation. The process follows Sequent's documented procedures (Business Continuity + Disaster Recovery + Incident Response, revision 1.0, approved by CEO, 2026).

### 4.4.1 Operator Actions

If a security incident is suspected during an election, the operator must:

1. **Preserve all logs** — do not modify, delete, or restart any system components
2. **Suspend voting** if voter data integrity may be at risk — use `cli step update-event-voting-status --election-event-id <ELECTION_EVENT_ID> --voting-status CLOSED`
3. **Contact Sequent immediately** through the support channel defined in your service agreement
4. **Do not attempt remediation** without authorization from Sequent and the election organizer

### 4.4.2 Sequent's Response Workflow

Upon notification, Sequent follows a structured four-phase response:

**Detection and Triage** — Sequent confirms the incident, determines its scope (affected components, data, and accounts), classifies its severity, and notifies relevant parties.

**Containment** — Technical and security functions isolate affected components, stop further impact, and begin root cause investigation.

**Recovery** — Services are restored and validated. In the airgap environment, recovery is performed on-site by the operator with remote guidance from Sequent.

**Post-incident Review** — Sequent produces a written retrospective covering root cause, corrective actions, and a communications summary.

### 4.4.3 Escalation Levels

Sequent's incident response uses three escalation levels:

| Level | Handled by | Description |
|---|---|---|
| **L1** | Support and incident handlers | Initial triage and stabilization |
| **L2** | Engineering | Deep technical investigation and remediation |
| **L3** | Executive and Security Leadership | Major incident oversight and formal reporting |

### 4.4.4 Severity and Update Cadence

| Severity | Definition | Update frequency |
|---|---|---|
| **Severity 1** | Active impact on voting or ballot integrity | Every 30 minutes until stable, then hourly |
| **Severity 2** | Partial degradation, no direct voter impact | Every 60 minutes or on material change |
| **Severity 3** | No voter impact | As agreed with the election organizer |

## 4.5 Security Vulnerability Management

This section describes how Sequent identifies, prioritizes, and remediates security vulnerabilities in the TOE. The process is described in Sequent's Security of Voting Servers and Protocol (revision 1.0, 2025).

### 4.5.1 Identification

Sequent conducts regular security assessments across all TOE components, including:

- Automated vulnerability scanning of dependencies and container images
- Code review for all software changes prior to release
- Penetration testing prior to major releases (most recent: BSI Germany, December 2022; University of Alicante, 2024)
- File integrity monitoring using Wazuh, which detects unauthorized changes to system files in real time using SHA-256 hashes

### 4.5.2 Prioritization and Remediation

Once a vulnerability is identified, Sequent classifies it by severity:

- **Critical or high severity** — addressed immediately
- **Medium or low severity** — scheduled for the next update cycle

Corrective measures may include patching, software updates, configuration changes, or targeted code fixes. All changes go through Sequent's standard code review and testing pipeline before release.

### 4.5.3 Reporting a Vulnerability

To report a suspected vulnerability in the Sequent Voting Platform, email [security@sequentech.io](mailto:security@sequentech.io).

When reporting, please include:

- A clear description of the issue and its potential impact
- The affected component(s) and TOE version
- Steps to reproduce the issue (if known), using only safe and non-destructive methods
- Any relevant logs or evidence, with sensitive voter data removed

Sequent aims to acknowledge receipt within **3 business days**, then follows a structured process: triage (assessing impact and exploitability) → remediation (developing and validating a fix) → release and communication (publishing the fix and advisory).

Sequent follows coordinated disclosure practices. Reporters are asked to avoid testing against real elections or production data, and to give Sequent a reasonable opportunity to investigate and remediate before public disclosure.
