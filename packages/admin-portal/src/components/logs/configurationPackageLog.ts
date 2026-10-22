// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {TFunction} from "i18next"
import type {ILogMessage} from "./logMessage"

const IMPORTED = "ConfigurationPackageImported"
const FIELDS = [
    "action",
    "external_id",
    "revision",
    "manifest_sha256",
    "content_hash",
    "design_digests",
]
const isObject = (value: unknown): value is Record<string, unknown> =>
    !!value && typeof value === "object" && !Array.isArray(value)

/** Keep hashes in the details while leaving historical signed descriptions intact. */
export const configurationPackageDescription = (
    t: TFunction,
    kind: string | null | undefined,
    message: ILogMessage | null
): string | null => {
    if (kind !== IMPORTED) return null
    const body = message?.statement?.body
    if (!isObject(body)) return null
    const imported = body.ConfigurationPackageWithContentHash ?? body.ConfigurationPackage
    if (!Array.isArray(imported) || !isObject(imported[1])) return null
    const details = imported[1]
    if (typeof details.external_id !== "string" || typeof details.revision !== "number") return null
    return String(
        t("logsScreen.configurationPackage.imported", {
            externalId: details.external_id,
            revision: details.revision,
        })
    )
}

export const configurationPackageFieldLabel = (
    t: TFunction,
    kind: string | null | undefined,
    name: string
): string =>
    kind === IMPORTED && FIELDS.includes(name)
        ? String(t(`logsScreen.configurationPackage.fields.${name}`))
        : name
