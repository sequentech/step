// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The external ID an election event's Data form saves. A stored ID is
 * read-only, so it is saved exactly as stored, as is one the form did not
 * submit (a disabled field). An entered one is saved without the blanks around
 * it, or as null when blank so it can still be set later.
 */
export const externalIdToSave = (
    stored: string | null | undefined,
    entered: string | null | undefined
): string | null =>
    entered === undefined || entered === stored ? (stored ?? null) : entered?.trim() || null
