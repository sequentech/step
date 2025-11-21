---
id: utility-components
title: Utility Components
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Utility Packages

## UI Essentials
- **Path**: `packages/ui-essentials/src`
- **Purpose**: Common frontend components, styles, and i18n support.
- **Directories**:
  - `components`: Reusable UI components (e.g., buttons, banners).
  - `services`: i18n translation and theme logic.
  - `stories`: Storybook entries for components.
  - `translations`: Language packs.

## Sequent Core
- **Path**: `packages/sequent-core/src`
- **Purpose**: Shared logic used by multiple backend services (e.g., Ballot Verifier, Voting Booth).
- **Directories**:
  - `serialization`: Data (de)serialization methods.
  - `services`: Core system-wide backend services.
  - `util`: Business logic utility functions.

- **Role**: Enables voters to audit and verify their cast ballots.
- **Path**: `step/packages/ballot-verifier`
- **Technologies**: Javascript, Typescript, GraphQL


