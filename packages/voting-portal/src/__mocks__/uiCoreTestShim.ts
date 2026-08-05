// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// CI runs each package's jest suite in isolation, without building sibling
// workspace packages first (see .github/workflows/tests.yml). @sequentech/ui-core
// resolves to its built dist/ (see ui-core/package.json "main"), which won't
// exist unless something built it first, so importing it directly breaks
// under a fresh `yarn install` with no build step.
//
// admin-portal's jest.config.cjs already works around this the same way,
// for the same reason, by mapping the specifier straight to a narrow
// ui-core source file. This shim generalizes that: it re-exports every
// *runtime* (non-type-only) value voting-portal's source/tests import from
// "@sequentech/ui-core", sourced directly from ui-core/src rather than its
// built dist. Type-only imports (interfaces, type aliases) don't need an
// entry here -- swc erases them at compile time, so they never reach
// module resolution.
//
// If a test starts importing a new runtime value from "@sequentech/ui-core"
// and this shim doesn't re-export it, jest will fail with "no such export"
// -- add the missing export's source file below.
export {isUndefined} from "../../../ui-core/src/utils/typechecks"
export * from "../../../ui-core/src/types/ContestPresentation"
export * from "../../../ui-core/src/types/AreaPresentation"
export * from "../../../ui-core/src/types/CoreTypes"
