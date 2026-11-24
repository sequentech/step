// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const signingExperience = (
    mode: unknown,
    action: string
): "legacy" | "managed" | "unavailable" => {
    if (mode === "legacy-p12") return "legacy"
    return mode === "hsm-required" && action === "initialize-voting" ? "managed" : "unavailable"
}

export const approvalControl = (phase: string, reviewed: boolean): "set-up" | "approve" | null => {
    if (phase === "setup-required") return "set-up"
    return phase === "challenge-available" && reviewed ? "approve" : null
}
