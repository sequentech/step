// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * A voter pseudonym as it is shown: `FTBE:MHRX`, with a colon, so it is never
 * taken for a Tracking ID (`FTBE-MHRX`). The stored form keeps its hyphen.
 */
export const pseudonymDisplay = (code: string): string =>
    /^[0-9A-Za-z]{4}-[0-9A-Za-z]{4}$/.test(code) ? `${code.slice(0, 4)}:${code.slice(5)}` : code
