// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// No client is named in the messaging product code or copy (AP-F19): sender
// names, Pages, home countries and organization names come from the tenant
// and its accounts. Tests, stories and their fixtures may name one; the
// second-organization stories show the screens work without it. The guard
// covers the TypeScript portals, the Rust messaging crates and services, and
// the Java Keycloak messaging extensions with their templates and messages.
import {existsSync, readFileSync, readdirSync, statSync} from "fs"
import {join, relative, resolve} from "path"

const PACKAGES = resolve(__dirname, "../../..")

const CLIENT_NAME = /COMELEC/i

/**
 * Tests, stories and their fixtures, which may name a client: TypeScript
 * tests and stories, Rust test files (`tests.rs`, `*_tests.rs`, the
 * adapters' `test_server.rs`, `tests/`) and Java's `src/test`. Rust's inline
 * `#[cfg(test)] mod … { … }` blocks are left out by `productText`.
 */
const NOT_PRODUCT =
    /(\.test\.tsx?|\.stories\.tsx?|\.d\.ts|(^|\/|_)tests?\.rs|\/test_server\.rs)$|\/(__stories__|__fixtures__|__mocks__|fixtures|tests?)\/|Fixture/

/** Every file under `dir` (relative to the packages directory). */
const walk = (dir: string): string[] => {
    const root = resolve(PACKAGES, dir)
    if (!existsSync(root)) return []
    return readdirSync(root).flatMap((name) => {
        const path = join(root, name)
        if (name === "node_modules") return []
        return statSync(path).isDirectory()
            ? walk(relative(PACKAGES, path))
            : [relative(PACKAGES, path)]
    })
}

const isSource = (path: string) =>
    /\.(tsx?|ftl|rs|java|properties)$/.test(path) && !NOT_PRODUCT.test(path)

const read = (path: string) => readFileSync(resolve(PACKAGES, path), "utf8")

/** Admin Portal: messaging by path (screens, queries, services, Logs), and every translation. */
const ADMIN_MESSAGING =
    /messag|SentMessages|SendTemplate|LedgerStatus|electoralLogChannel|components\/logs\/|AudiencePreview|MonthlyVolumes|supportReference|SupportReference/i

const adminFiles = () =>
    walk("admin-portal/src")
        .filter(isSource)
        .filter(
            (path) =>
                !path.startsWith("admin-portal/src/gql/") &&
                (path.startsWith("admin-portal/src/translations/") || ADMIN_MESSAGING.test(path))
        )

/** Keycloak voter pages: every login source that deals with channels, codes or messages. */
const KEYCLOAK_MESSAGING = /messag|channel|courier|otp/i

const keycloakFiles = () =>
    [...walk("keycloak-ui/src/login"), "keycloak-ui/context.ftl"]
        .filter(isSource)
        .filter((path) => KEYCLOAK_MESSAGING.test(read(path)))

/** The voting portal's menu entry for the voter's channel, and its translations. */
const votingPortalFiles = () =>
    [
        "voting-portal/src/components/PortalChrome.tsx",
        ...walk("voting-portal/src/translations"),
    ].filter(isSource)

/** The channel labels and icons the three share. */
const sharedFiles = () => walk("ui-essentials/src/components/ChannelIcon").filter(isSource)

/**
 * Rust: the messaging crates, windmill's messaging services and queries,
 * harvest's messaging routes and OTP capacity, and sequent-core's messaging
 * and voter channel types.
 */
const rustFiles = () =>
    [
        ...walk("messaging/src"),
        ...walk("messaging-webhooks/src"),
        ...walk("windmill/src/services/messaging"),
        ...walk("windmill/src/postgres").filter((path) =>
            /messag|rate_bucket|voter_count/.test(path)
        ),
        ...walk("harvest/src/routes").filter((path) => /messag|fake_provider/.test(path)),
        "harvest/src/services/otp_capacity.rs",
        "sequent-core/src/types/messaging.rs",
        "sequent-core/src/types/voter_channel.rs",
    ].filter(isSource)

/**
 * Java: the message OTP authenticator (sources, templates and message
 * bundles), and the voter enrollment sources and templates that deal with
 * channels, codes or messages, with their message bundles.
 */
const javaFiles = () => [
    ...walk("keycloak-extensions/message-otp-authenticator/src/main").filter(isSource),
    ...walk("keycloak-extensions/voter-enrollment/src/main")
        .filter(isSource)
        .filter((path) => path.endsWith(".properties") || KEYCLOAK_MESSAGING.test(read(path))),
]

const scanned = () => [
    ...adminFiles(),
    ...keycloakFiles(),
    ...votingPortalFiles(),
    ...sharedFiles(),
    ...rustFiles(),
    ...javaFiles(),
]

/**
 * Where the `{` at `open` closes in Rust source, skipping comments, strings,
 * raw strings and char literals. Unclosed: the end of the text.
 */
const closingBrace = (text: string, open: number): number => {
    let depth = 0
    let i = open
    while (i < text.length) {
        const rest = text.slice(i, i + 16)
        if (rest.startsWith("//")) {
            const end = text.indexOf("\n", i)
            if (end < 0) return text.length
            i = end
            continue
        }
        if (rest.startsWith("/*")) {
            const end = text.indexOf("*/", i + 2)
            if (end < 0) return text.length
            i = end + 2
            continue
        }
        const raw = /^b?r(#*)"/.exec(rest)
        if (raw && !/\w/.test(text[i - 1] ?? "")) {
            const end = text.indexOf(`"${raw[1]}`, i + raw[0].length)
            if (end < 0) return text.length
            i = end + 1 + raw[1].length
            continue
        }
        if (text[i] === '"') {
            i += 1
            while (i < text.length && text[i] !== '"') i += text[i] === "\\" ? 2 : 1
            i += 1
            continue
        }
        const char = /^'(?:\\.[^']*|[^\\'])'/.exec(rest)
        if (char) {
            i += char[0].length
            continue
        }
        if (text[i] === "{") depth += 1
        if (text[i] === "}") {
            depth -= 1
            if (depth === 0) return i
        }
        i += 1
    }
    return text.length
}

const TEST_MODULE = /#\[cfg\(test\)\]\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{/g

/**
 * A file's product text: Rust without its `#[cfg(test)] mod … { … }`
 * blocks, blanked so line numbers stay; other files as they are.
 */
const productText = (path: string, text: string): string => {
    if (!path.endsWith(".rs")) return text
    const chars = text.split("")
    for (const match of Array.from(text.matchAll(TEST_MODULE))) {
        const end = closingBrace(text, match.index + match[0].length - 1)
        for (let i = match.index; i <= end && i < chars.length; i += 1) {
            if (chars[i] !== "\n") chars[i] = " "
        }
    }
    return chars.join("")
}

/** `path:line: text` of each line naming a client. */
const namesIn = (path: string, text: string): string[] =>
    text
        .split("\n")
        .flatMap((line, index) =>
            CLIENT_NAME.test(line) ? [`${path}:${index + 1}: ${line.trim()}`] : []
        )

/** Each line naming a client, in the product text of `paths`. */
const clientNames = (paths: string[]): string[] =>
    paths.flatMap((path) => namesIn(path, productText(path, read(path))))

describe("messaging names no client", () => {
    it("scans the messaging sources and translations of each portal", () => {
        const files = scanned()
        expect(files).toEqual(
            expect.arrayContaining([
                "admin-portal/src/types/messaging.ts",
                "admin-portal/src/services/messaging.ts",
                "admin-portal/src/resources/Settings/SettingsMessaging.tsx",
                "admin-portal/src/resources/Settings/SentMessages/SentMessages.tsx",
                "admin-portal/src/resources/ElectionEvent/EditElectionEventMessaging.tsx",
                "admin-portal/src/resources/User/SendTemplate.tsx",
                "admin-portal/src/resources/User/VoterMessagingSummary.tsx",
                "admin-portal/src/components/logs/LedgerStatus.tsx",
                "admin-portal/src/translations/en.ts",
                "admin-portal/src/translations/tl.ts",
                "keycloak-ui/src/login/pages/MessageOtpLogin.tsx",
                "keycloak-ui/src/login/enrollment/ChannelChoice.tsx",
                "keycloak-ui/src/login/i18n.ts",
                "keycloak-ui/context.ftl",
                "voting-portal/src/components/PortalChrome.tsx",
                "voting-portal/src/translations/es.ts",
                "ui-essentials/src/components/ChannelIcon/ChannelIcon.tsx",
                "messaging/src/sender.rs",
                "messaging/src/providers/whatsapp.rs",
                "messaging/src/webhooks/meta.rs",
                "messaging-webhooks/src/lib.rs",
                "windmill/src/services/messaging/dispatch.rs",
                "windmill/src/postgres/messaging.rs",
                "harvest/src/routes/messaging.rs",
                "harvest/src/services/otp_capacity.rs",
                "sequent-core/src/types/messaging.rs",
                "sequent-core/src/types/voter_channel.rs",
                "keycloak-extensions/message-otp-authenticator/src/main/java/sequent/keycloak/authenticator/messaging/HarvestMessageSenderProvider.java",
                "keycloak-extensions/message-otp-authenticator/src/main/java/sequent/keycloak/authenticator/messaging/VoterChannel.java",
                "keycloak-extensions/message-otp-authenticator/src/main/resources/theme-resources/messages/messages_en.properties",
                "keycloak-extensions/message-otp-authenticator/src/main/resources/theme-resources/templates/message-otp.login.ftl",
                "keycloak-extensions/voter-enrollment/src/main/java/sequent/keycloak/voter_enrollment/EnrollmentChannels.java",
                "keycloak-extensions/voter-enrollment/src/main/resources/theme-resources/messages/messages_en.properties",
                "keycloak-extensions/voter-enrollment/src/main/resources/theme-resources/templates/registration-finish.ftl",
            ])
        )
        // Test files are not scanned: Rust's, Java's and the portals'.
        for (const path of [
            "messaging/src/test_server.rs",
            "messaging-webhooks/src/tests.rs",
            "windmill/src/services/messaging/insights_tests.rs",
            "sequent-core/src/types/voter_channel_tests.rs",
        ]) {
            expect(files).not.toContain(path)
        }
        expect(files.some((path) => path.includes("/src/test/"))).toBe(false)
        // Tests, stories and fixtures are not product code.
        expect(files.filter((path) => NOT_PRODUCT.test(path))).toEqual([])
    })

    it("finds a client name where there is one", () => {
        expect(
            clientNames(["admin-portal/src/translations/messagingTranslations.test.ts"])
        ).not.toEqual([])
        expect(
            clientNames([
                "keycloak-extensions/voter-enrollment/src/test/java/sequent/keycloak/voter_enrollment/EnrollmentWindowCheckTest.java",
            ])
        ).not.toEqual([])
    })

    it("leaves out only Rust's test modules", () => {
        const source = [
            'const OPEN: &str = "{"; // }',
            "const BRACE: char = '{';",
            "fn label<'a>(name: &'a str) -> &'a str { name }",
            "#[cfg(test)]",
            "mod tests {",
            '    const RAW: &str = r#"}"#;',
            '    fn sender() -> &\'static str { "COMELEC" }',
            "}",
            'fn after() -> &\'static str { "Comelec" }',
            "#[cfg(test)]",
            "mod helpers;",
            'const LAST: &str = "COMELEC";',
        ].join("\n")
        expect(namesIn("sample.rs", productText("sample.rs", source))).toEqual([
            'sample.rs:9: fn after() -> &\'static str { "Comelec" }',
            'sample.rs:12: const LAST: &str = "COMELEC";',
        ])
        // The same lines outside Rust are all scanned.
        expect(namesIn("sample.ts", productText("sample.ts", source))).toHaveLength(3)
    })

    it("has none in the product code or copy", () => {
        expect(clientNames(scanned())).toEqual([])
    })
})
