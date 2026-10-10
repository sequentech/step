// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const canTrusteeProceedToDownload = (
    trusteeParticipating: boolean,
    keysGenerated: boolean
): boolean => trusteeParticipating && keysGenerated
