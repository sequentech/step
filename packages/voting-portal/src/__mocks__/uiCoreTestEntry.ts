// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The @sequentech/ui-core package entry points at its built dist/ bundle,
// which a clean `yarn install` does not produce, so jest can't resolve the
// package on CI. These unit tests only need dependency-free enums and type
// guards, so they are re-exported straight from ui-core's sources instead of
// loading the whole barrel (which pulls in React, i18next and the WASM
// context). Mapped in jest.config.cjs.
export * from "../../../ui-core/src/types/AreaPresentation"
export * from "../../../ui-core/src/types/ContestPresentation"
export * from "../../../ui-core/src/types/CoreTypes"
export * from "../../../ui-core/src/utils/typechecks"
