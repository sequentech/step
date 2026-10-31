// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Stylesheets, as far as jest is concerned.
 *
 * A bundler turns `import "x/index.css"` into a side effect; jest hands the file
 * to its JavaScript parser and reports `SyntaxError: Unexpected token '.'` from
 * inside a dependency, which reads as a broken dependency.
 *
 * Reached here through this package's own barrel: importing anything from it pulls
 * `TallyResults`, which pulls `@mui/x-data-grid`, which imports its own CSS — none
 * of which a ballot touches. That chain is the argument for the
 * narrow ballot entry point in `EA-F1-005`; this stub is what makes the barrel
 * loadable until then.
 */

export default {}
