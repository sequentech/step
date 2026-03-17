---
id: cyber-security
title: Cyber Security and Vulnerabilities
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This page provides supplementary cybersecurity information for the `Sequent Voting Platform - uniWAHL Version`, including how to report vulnerabilities and how security issues are handled.

## Reporting security vulnerabilities

To report a suspected vulnerability, use **GitHub Security Advisories**:

[Report via GitHub Security Advisories](https://github.com/sequentech/step/security/advisories/new)

Please include:

- A clear description of the issue and its potential impact.
- Affected component(s) and version(s) (if known).
- Reproduction steps or a proof of concept (safe and non-destructive).
- Any relevant logs or screenshots, with sensitive data removed.
- Your preferred contact details for follow-up.

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

When a report is received via GitHub Security Advisories, we follow a documented triage and remediation process:

### 1. Acknowledgement
- **Target:** Within **3 business days** of receiving the report
- **Content:** Confirmation of receipt, initial assessment, and tracking ID (GitHub Security Advisory ID)
- **Method:** Acknowledgement sent via GitHub Security Advisory

### 2. Triage and Severity Assessment
- **Target:** Within 7 business days
- **Activities:**
  - Reproduce the vulnerability
  - Assess severity using CVSS v3.1 scoring
  - Classify severity level (Critical/High/Medium/Low)
  - Determine affected versions and components
  - Identify root cause
  - Assess impact on election integrity, voter privacy, and system availability
  - Determine if BSI notification is required (for certification-relevant vulnerabilities)

**Severity Classification:**

| Severity | Definition | Response SLA | Example |
|----------|-----------|--------------|---------|
| **Critical** | Immediate risk to election integrity, voter privacy, or system availability. Actively exploited or trivially exploitable. | 30 days | Vote tampering, ballot secrecy breach, remote code execution, cryptographic key exposure |
| **High** | Significant risk requiring authentication or specific conditions. Could lead to unauthorized access or system compromise. | 60 days | Authentication bypass, privilege escalation, SQL injection, XSS in admin panels |
| **Medium** | Moderate risk requiring complex attack chains or having limited impact. | 90 days | Information disclosure, CSRF, insecure direct object references |
| **Low** | Minimal risk or theoretical vulnerabilities with no demonstrated exploit path. | 120 days | Security misconfigurations with minimal impact, best practice recommendations |

### 3. Remediation
- **Assignment:** Security team assigns to development team based on component expertise
- **Development:** Fix developed following [Secure Development Lifecycle](../05-reference/) procedures
- **Code Review:** Mandatory security-focused code review by minimum 2 reviewers
- **Testing:** Unit tests, integration tests, and security regression tests
- **Validation:** Security team validates fix effectiveness

### 4. Release and Communication
- **Security Patch Release:** Follow standard release procedures
- **Customer Notification:** Email notification to all affected customers
- **Public Disclosure:** Coordinated disclosure after customer notification (typically 7-14 days post-release)
- **Security Advisory:** Published on GitHub Security Advisories and release notes

Security fixes may be delivered as patch releases when needed. For release cadence and how security releases are handled, see [Product lifecycle and release cadence](../05-reference/03-product_lifecycle_and_release_cadence.md).

### Internal Vulnerability Detection

Beyond external reports, we use automated detection:
- **Dependabot:** Automated dependency vulnerability scanning with automatic pull requests for updates
- **CodeQL (SAST):** Static application security testing for common vulnerability patterns (OWASP Top 10)
- **Secret Scanning:** Prevents commits containing credentials or keys

### Tracking and Audit Trail

All vulnerabilities are tracked using:
- **GitHub Security Advisories:** Private vulnerability tracking until public disclosure in the [sequentech/step repository](https://github.com/sequentech/step/security/advisories)
- **GitHub Issues:** For security-specific issues with restricted access
- **Tracking Information:**
  - Unique identifier (GitHub Advisory ID or Issue number)
  - Discovery date and reporter
  - Severity classification and CVSS score
  - Affected versions and components
  - Remediation status and timeline
  - Communication log
  - Link to fix pull request
  - Release version containing fix
  - Disclosure date

**GitHub Security Advisories Process:**
1. Security vulnerability reported via [GitHub private vulnerability reporting](https://github.com/sequentech/step/security/advisories/new)
2. Security team creates or uses existing private GitHub Security Advisory in sequentech/step repository
3. Advisory tracks vulnerability details, affected versions, and remediation progress
4. Fixes developed in private fork associated with the advisory
5. Security patches released and advisory published publicly
6. CVE ID requested and assigned through GitHub (if applicable)

**Benefits of GitHub Security Advisories:**
- Private collaboration with reporter until fix is ready
- Secure environment for discussing vulnerability details
- Automatic CVE assignment through GitHub
- Integrated with GitHub's security infrastructure
- Complete audit trail maintained in GitHub

### BSI Notification (Certification-Relevant Vulnerabilities)

For BSI EUCC certification compliance, we notify BSI within **48 hours** for certification-relevant vulnerabilities that:
- Affect security functions documented in the Security Target
- Impact conformance to BSI-CC-PP-0121 Protection Profile
- Violate security functional requirements (SFRs)
- Affect cryptographic mechanisms or key management
- Could invalidate certification claims

**Notification Method:** Email to BSI certification office (zertdokus@bsi.bund.de) with detailed vulnerability description, impact assessment, and proposed remediation approach.

## Security advisories and updates

Security-relevant changes are communicated through release notes in this documentation.
See the [Release Notes](../08-releases/).
