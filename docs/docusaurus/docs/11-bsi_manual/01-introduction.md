---
id: bsi_introduction
title: "BSI User Manual — Part 1: Introduction"
---
<!--
-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Part 1: Introduction

*Sequent Voting Platform – uniWAHL Version | User Manual*

## 1.1 Purpose of This Manual

This manual describes how to install, configure, and operate the **Sequent Voting Platform – uniWAHL Version** (hereinafter: the TOE). It is written to satisfy the guidance documentation requirements of the Common Criteria evaluation at EAL1, as defined in the Protection Profile for E-Voting Systems for Non-Political Elections (BSI-CC-PP-0121).

The manual is intended for three audiences, using the role terminology defined in BSI-CC-PP-0121 (section 3.1.2):

- **Administrator** — responsible for the technical management of the TOE: installation, configuration, and CLI operations
- **Election board** — responsible for organizational management of the election: importing election data, monitoring execution, initiating the tally, and reviewing audit records
- **Election organizer** — the group hosting the e-voting, responsible for defining election parameters and authorizing the election process

This manual covers only the **Target of Evaluation (TOE)** as defined in the Security Target.

:::caution TOE scope — pending confirmation from engineering
The full scope of the TOE, including which components and interfaces are in scope for certification, must be confirmed with engineering before this section is finalized. See Section 1.2.
:::

## 1.2 What Is the TOE?

:::caution Rewrite pending — confirm with Eduardo
The TOE scope must be confirmed with engineering before this section is rewritten. Specifically: (1) does the TOE include the Admin Portal UI or only the backend server components and CLI? (2) Is the development workflow (develop → fix → deploy) part of the TOE scope as Guy noted?
:::

The TOE is the software that powers a secure, verifiable, non-political e-voting election. It handles election creation, voter authentication, ballot encryption, vote collection, and result verification.

The TOE does **not** include the terminal device used by voters to cast their vote. According to the PP (section 1.3.3), the terminal device is explicitly non-TOE hardware.

### 1.2.1 TOE Components

The TOE consists of the following server-side components, deployed as Docker images:

| Component | Short name | Purpose |
|---|---|---|
| Main backend | `harvest` | Core server managing election data and API |
| Background worker | `windmill` | Handles asynchronous processing tasks |
| Identity & access management | `keycloak` | Manages user authentication and roles |
| Ballot verifier | `ballot-verifier` | Allows verification of individual encrypted ballots |
| Election verifier | `election-verifier` | Allows full verification of election results |
| Trustee server | `braid` | Manages cryptographic trustee operations |
| Bulletin board | `b4` | Immutable, append-only record of all election events |

The `step` command-line tool is used by the administrator to perform all management and ceremony operations.

## 1.3 What Elections Is This System For?

The Sequent Voting Platform – uniWAHL Version is designed for **non-political elections**, such as:

- Works council elections (Betriebsratswahl)
- Equal opportunity officer elections
- Union internal elections
- University governing body elections
- Association elections

The Protection Profile (BSI-CC-PP-0121, section 1.3) defines six election principles. The TOE is responsible for implementing the following three, which can be enforced by software:

| Principle | Definition | Enforced by TOE |
|---|---|---|
| **Secret** | The voting process is conducted such that third parties cannot trace how individual voters voted | Yes |
| **Equal** | Each cast vote has the same weight and the same influence on the election result | Yes |
| **Public** | Essential parts of the election — in particular the correct counting — are verifiable by the public | Yes |

The remaining principles (Universal, Direct, Free) are based on organizational requirements that cannot be fully controlled by the TOE, and are therefore the responsibility of the election organizer.

## 1.4 Security Certification Context

The TOE is being evaluated under the **Common Criteria** framework against the Protection Profile for E-Voting Systems for Non-Political Elections, version 1.0 (BSI-CC-PP-0121), published by Germany's Federal Office for Information Security (BSI).

| Field | Value |
|---|---|
| TOE name | Sequent Voting Platform – uniWAHL Version |
| TOE version | v10.0.0 |
| Assurance level | EAL1 |
| Evaluation lab | Electric Paper Informationssysteme GmbH (EPI) |
| Protection Profile | BSI-CC-PP-0121 (E-Voting, non-political) |
| Security Target version | 1.0 (March 10, 2026) |

The Security Target is the primary reference document for the TOE's security claims. This user manual is a companion document that describes how to operate the TOE in conformance with those claims.

## 1.5 How to Use This Manual

This manual is organized into four parts:

| Part | Contents |
|---|---|
| **Part 1: Introduction** (this document) | Overview of the TOE, certification context, and terminology |
| [Part 2: Preparative Procedures](./02-preparative_procedures.md) | How to install and configure the TOE securely before operation |
| [Part 3: Operational User Guidance](./03-operational_guidance.md) | How to operate the TOE, by role |
| [Part 4: Security Policies](./04-security_policies.md) | Organizational security responsibilities and audit logging |

Read Part 2 before first deployment. Read Part 3 before running any election. Part 4 applies throughout the system's lifetime.

## 1.6 Terminology and Abbreviations

The following terms are used throughout this manual. Role terms (administrator, election board, election organizer, voter) are as defined in BSI-CC-PP-0121, section 3.1.2 and section 1.4.

| Term | Definition |
|---|---|
| **TOE** | Target of Evaluation — the specific version of the system being certified |
| **PP** | Protection Profile — BSI-CC-PP-0121, the requirements document that defines what must be certified |
| **ST** | Security Target — Sequent's document describing how the TOE meets the PP |
| **EAL1** | Evaluation Assurance Level 1 — the minimum Common Criteria assurance level |
| **BSI** | Bundesamt für Sicherheit in der Informationstechnik — Germany's Federal Office for Information Security |
| **EPI** | Electric Paper Informationssysteme GmbH — the licensed evaluation lab performing the certification |
| **Administrator** | User role (PP): manages the TOE from a technical perspective (installation, configuration, CLI operations) |
| **Election board** | User role (PP): manages the TOE from an organizational perspective (data import, monitoring, tally initiation, audit review) |
| **Election organizer** | User role (PP): the group hosting the e-voting; defines election parameters and authorizes the process |
| **Trustee** | A designated person who holds a share of the cryptographic key required to decrypt election results |
| **step CLI** | Command-line tool used by the administrator for all TOE management and ceremony operations |
| **Key ceremony** | A formal, documented procedure in which trustees jointly generate the encryption key for the election |
| **Tally ceremony** | A formal, documented procedure in which trustees jointly decrypt and count the votes |
| **Bulletin board (b4)** | The immutable, append-only log of all election events |
| **Airgap** | A deployment mode in which the TOE server has no internet connection — all software is delivered via USB |
