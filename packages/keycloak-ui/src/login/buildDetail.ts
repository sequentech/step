// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * A build detail worth showing in the login header, or nothing.
 *
 * Keycloak fills `${env.APP_VERSION}` style theme properties from its environment and
 * leaves a reference it cannot resolve as it is, so a server without the variable (or
 * one still running a sequent-theme that gives it no default) hands the raw reference
 * over. Neither that nor an empty value deserves a label.
 */
export function buildDetail(value: string | undefined): string | undefined {
    const trimmed = value?.trim()
    return trimmed && !trimmed.includes("${") ? trimmed : undefined
}
