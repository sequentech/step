// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createInstance} from "i18next"
import en from "@/translations/en"
import {
    configurationPackageDescription,
    configurationPackageFieldLabel,
} from "./configurationPackageLog"
import type {ILogMessage} from "./logMessage"

async function translate() {
    const i18n = createInstance()
    await i18n.init({
        lng: "en",
        resources: {en: {translation: structuredClone(en.translations)}},
        interpolation: {escapeValue: false},
    })
    return i18n
}

it.each(["ConfigurationPackage", "ConfigurationPackageWithContentHash"])(
    "keeps hashes out of the displayed description of %s without changing the signed message",
    async (variant) => {
        const i18n = await translate()
        const message: ILogMessage = {
            statement: {
                head: {description: `Imported (manifest ${"ab".repeat(32)}).`},
                body: {
                    [variant]: [
                        "event-1",
                        {
                            external_id: "approved-configuration",
                            revision: 9,
                            manifest_sha256: "ab".repeat(32),
                            ...(variant === "ConfigurationPackageWithContentHash"
                                ? {content_hash: "cd".repeat(32)}
                                : {}),
                        },
                    ],
                },
            },
        }
        const original = JSON.stringify(message)
        expect(
            configurationPackageDescription(i18n.t, "ConfigurationPackageImported", message)
        ).toBe("Configuration approved-configuration revision 9 imported from a signed package.")
        expect(JSON.stringify(message)).toBe(original)
    }
)

it("uses translated field names and preserves unrelated and unknown fields", async () => {
    const i18n = await translate()
    i18n.addResource(
        "en",
        "translation",
        "logsScreen.configurationPackage.fields.content_hash",
        "Content fingerprint"
    )
    expect(
        configurationPackageFieldLabel(i18n.t, "ConfigurationPackageImported", "content_hash")
    ).toBe("Content fingerprint")
    expect(
        configurationPackageFieldLabel(i18n.t, "ConfigurationPackageImported", "manifest_sha256")
    ).toBe("Manifest SHA-256")
    expect(
        configurationPackageFieldLabel(i18n.t, "ConfigurationPackageImported", "future_field")
    ).toBe("future_field")
    expect(configurationPackageFieldLabel(i18n.t, "AdminChange", "content_hash")).toBe(
        "content_hash"
    )
})

it("leaves unrelated entries and incomplete import bodies to the original description", async () => {
    const i18n = await translate()
    const message: ILogMessage = {
        statement: {body: {ConfigurationPackage: ["event-1", {external_id: "configuration"}]}},
    }
    expect(configurationPackageDescription(i18n.t, "AdminChange", message)).toBeNull()
    expect(configurationPackageDescription(i18n.t, "ConfigurationPackageImported", null)).toBeNull()
    expect(
        configurationPackageDescription(i18n.t, "ConfigurationPackageImported", message)
    ).toBeNull()
})
