---
id: open_source
title: Open Source
---
<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## What Is Open Source?

Open source software follows the [Open Source Definition](https://opensource.org/osd) maintained by the Open Source Initiative (OSI), which guarantees key software freedoms: access to source code, free redistribution, and the ability to create derivative works.

The OSI curates the official [list of approved licenses](https://opensource.org/licenses/) that comply with this definition.

For the Sequent Voting Platform this means we:
- Publish the full codebase under the [GNU Affero General Public License v3.0 only (AGPL-3.0-only)](https://opensource.org/license/agpl-v3/), an OSI-approved copyleft license designed for networked services.
- Provide direct access to the license text and repository at [github.com/sequentech/step](https://github.com/sequentech/step), including the [AGPL-3.0-only license file](https://github.com/sequentech/step?tab=AGPL-3.0-1-ov-file#readme).

We keep the entire repository publicly open so anyone can inspect, audit, and contribute to the codebase.

## Why Open Source Matters for Secure E-Voting

- [Election infrastructure is formally recognized as critical infrastructure](https://www.cisa.gov/election-security) in multiple jurisdictions, underscoring the need for transparency, resilience, and public oversight.

- The [Council of Europe recommends the use of open source for e-voting systems](https://www.coe.int/en/web/electoral-assistance/e-voting), emphasizing auditability, accountability, and verifiability.

- Openness strengthens sovereignty and software independence, aligning with the [European recommendations on digital sovereignty and open source](https://digital-strategy.ec.europa.eu/en/library/recommendations-and-roadmap-european-sovereignty-open-source-hardware-software-and-risc-v).

- **Transparent code builds trust** in election outcomes by allowing stakeholders to verify security controls, cryptographic protocols, and procedural safeguards end-to-end.

- Open development **lets us collaborate seamlessly** with academics, regulators, certification bodies, and civil society, reducing friction in reviews, audits, and joint research.

- Operating in the open **differentiates us in the market**: we can prove the quality of our solution, welcome scrutiny, and foster a healthier competitive ecosystem where interoperability and shared improvements are the norm.

    This approach mirrors successful projects like **Linux**, which powers everything from smartphones to supercomputers, and **Android**, which became the world's most widely adopted mobile operating system—both demonstrating how open source can drive innovation and market leadership.

- We believe openness is a sound business strategy—community contributions accelerate innovation, while customers value the assurance that the platform remains accessible, auditable, and adaptable.

## What Is Not Open Source

- A **visible license** alone does not guarantee openness; beware of “source-available” licenses that are not approved by the OSI or the Free Software Foundation (FSF), as they may restrict redistribution, modification, or commercial use.

- Some vendors publish only **isolated components** — such as a ballot verifier, client widget, or library — while keeping the rest proprietary. Partial transparency is welcome, but presenting the entire solution as open source would be misleading.

- Closed development models that require **NDAs, paid access**, or individually negotiated terms contradict the spirit and practice of open source, even if snippets of code are published elsewhere.

- Claims of “open standards” without access to the implementation or build tooling also fall short; real openness includes the full stack, from infrastructure as code to client applications.

## Questions to Ask Providers Claiming to Be Open Source

1. Is the complete code repository publicly available? What is the direct link to access it?
2. Which specific OSI-approved license do you use for your voting platform?
3. Is the full set of advertised product features available under the OSI-approved license?
4. Can third-party audits, certifications, or regulator reviews be conducted on the publicly available code?
