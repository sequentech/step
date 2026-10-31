// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum ETransparencyMode {
    DISABLED = "disabled",
    PUBLIC = "public",
    PRIVATE = "private",
}

/** The transparency mode in an election event's `presentation`. An absent or
 * unknown mode is `disabled`, as for events created before Vitrine. */
export const transparencyMode = (presentation: unknown): ETransparencyMode => {
    const mode =
        presentation !== null && typeof presentation === "object"
            ? (presentation as {transparency?: unknown}).transparency
            : undefined
    return mode === ETransparencyMode.PUBLIC || mode === ETransparencyMode.PRIVATE
        ? mode
        : ETransparencyMode.DISABLED
}
