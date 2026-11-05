// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import * as verify from "@vitrine/verify"
import {
    draftPolicyText,
    EVENT_ID,
    NOW,
    OTHER_EVENT_ID,
    policyHexOf,
    policyText,
    sealedPolicyBody,
    sidecarLogs,
    sidecarStatus,
} from "@/fixtures/vitrinePolicy"
import {
    boardCommands,
    draftFileName,
    EPolicyMismatch,
    EPolicyState,
    EPublicRefusal,
    ETransparencyMode,
    examinePolicy,
    isKnownRefusal,
    normalisePolicyText,
    policyHex,
    policyMismatch,
    readEnrolments,
    readLogs,
    readMode,
    safeExternalUrl,
    sameDraft,
    TPolicyExamination,
    vitrineLanguage,
} from "./Transparency"

beforeAll(async () => {
    await verify.init()
})

const sealed = (text: string) => {
    const e = examinePolicy(verify, text)
    if (e.state !== EPolicyState.SEALED) throw new Error(`not sealed: ${JSON.stringify(e)}`)
    return e
}

const draft = (text: string) => {
    const e = examinePolicy(verify, text)
    if (e.state !== EPolicyState.DRAFT) throw new Error(`not a draft: ${JSON.stringify(e)}`)
    return e
}

const reasonOf = (e: TPolicyExamination) => (e.state === EPolicyState.INVALID ? e.reason : null)

describe("readMode", () => {
    it.each([
        ["public", ETransparencyMode.PUBLIC],
        ["private", ETransparencyMode.PRIVATE],
        ["disabled", ETransparencyMode.DISABLED],
        [undefined, ETransparencyMode.DISABLED],
        [null, ETransparencyMode.DISABLED],
        ["PUBLIC", ETransparencyMode.DISABLED],
        ["strict", ETransparencyMode.DISABLED],
        [1, ETransparencyMode.DISABLED],
    ])("reads %p as %s", (value, mode) => {
        expect(readMode(value)).toBe(mode)
    })
})

describe("isKnownRefusal", () => {
    it("knows why Harvest refuses public transparency, and nothing else", () => {
        expect(isKnownRefusal(EPublicRefusal.UNLIMITED_REVOTES)).toBe(true)
        expect(isKnownRefusal("public-needs-something-else")).toBe(false)
        expect(isKnownRefusal("")).toBe(false)
    })
})

describe("vitrineLanguage", () => {
    it.each([
        ["cat", "ca"],
        ["en", "en"],
        ["es", "es"],
        ["es-ES", "es"],
        ["eu", "eu"],
        ["fr_FR", "fr"],
        ["gl", "gl"],
        ["nl", "nl"],
        ["tl", "tl"],
        ["ca", "ca"],
        ["de", "en"],
        ["", "en"],
        [undefined, "en"],
    ])("maps the portal's %p to the kit's %p", (app, kit) => {
        expect(vitrineLanguage(app)).toBe(kit)
    })
})

const GROUPS_13 = /^([0-9A-Z]{4}-){12}[0-9A-Z]{4}$/

describe("examinePolicy", () => {
    it("reads a sealed, signed policy with its ID", () => {
        const text = policyText()
        const e = sealed(text)
        const parsed = verify.parsePolicy(text)
        expect(e.policy.id).toBe(parsed.id)
        expect(e.policy.id).toBe(`sha256:${policyHexOf(text)}`)
        expect(verify.policyIdDisplay(e.policy.id)).toMatch(GROUPS_13)
        expect(verify.shortPolicyId(e.policy.id)).toBe(
            verify.policyIdDisplay(e.policy.id).split("-").slice(0, 4).join("-")
        )
        expect(e.summary.event).toBe("acme-2026")
    })

    it("reads the draft with its draft hash in groups, which is not a policy ID", () => {
        const text = draftPolicyText()
        const e = draft(text)
        const parsed = verify.parsePolicy(text, {allowDraft: true, requireSignatures: false})
        expect(e.draftId).toBe(verify.policyIdDisplay(parsed.id))
        expect(e.draftId).toMatch(GROUPS_13)
        expect(e.shortDraftId).toBe(e.draftId.split("-").slice(0, 4).join("-"))
        // The policy sealed from the draft has another ID: it hashes the seal too.
        expect(e.draftId).not.toBe(verify.policyIdDisplay(sealed(policyText()).policy.id))
    })

    it("refuses the sealed text without the Board's signature", () => {
        const e = examinePolicy(verify, sealedPolicyBody())
        expect(e.state).toBe(EPolicyState.INVALID)
        expect(typeof reasonOf(e)?.code).toBe("string")
    })

    it("refuses a signed policy whose text was changed", () => {
        const tampered = policyText().replace("mmd 60", "mmd 61")
        expect(examinePolicy(verify, tampered).state).toBe(EPolicyState.INVALID)
    })

    it.each([
        ["markup", "<img src=x onerror=alert(1)>"],
        ["a receipt chain", "vitrine/receipt-chain/v1\nnonsense\n"],
        ["nothing", ""],
    ])("says why %s isn't a policy", (_what, text) => {
        const e = examinePolicy(verify, text)
        expect(e.state).toBe(EPolicyState.INVALID)
        expect(typeof reasonOf(e)?.code).toBe("string")
    })
})

describe("normalisePolicyText", () => {
    it("gives the text the logs take, however it was pasted", () => {
        const text = policyText()
        expect(normalisePolicyText(text)).toBe(text)
        expect(normalisePolicyText(`\r\n  ${text.replace(/\n/g, "\r\n")}  \r\n`)).toBe(text)
        expect(normalisePolicyText(text.trimEnd())).toBe(text)
    })

    it("is empty for whitespace", () => {
        expect(normalisePolicyText(" \r\n\t ")).toBe("")
    })

    it("keeps a policy the Board signed checkable after a paste", () => {
        const pasted = normalisePolicyText(policyText().replace(/\n/g, "\r\n").trimEnd())
        expect(examinePolicy(verify, pasted).state).toBe(EPolicyState.SEALED)
    })
})

describe("policyHex", () => {
    const hex = "ab".repeat(32)

    it("reads a policy ID with or without its prefix, in any case", () => {
        expect(policyHex(hex)).toBe(hex)
        expect(policyHex(`sha256:${hex}`)).toBe(hex)
        expect(policyHex(`SHA256:${hex.toUpperCase()}`)).toBe(hex)
    })

    it.each([["ab"], [`${hex}00`], [`sha512:${hex}`], ["zz".repeat(32)], [42], [null]])(
        "refuses %p",
        (value) => {
            expect(policyHex(value)).toBeUndefined()
        }
    )
})

describe("policyMismatch", () => {
    it("accepts the event's own policy, whatever the ID's case", () => {
        const {policy} = sealed(policyText())
        expect(policyMismatch(policy, EVENT_ID, ETransparencyMode.PUBLIC)).toBeUndefined()
        expect(
            policyMismatch(policy, EVENT_ID.toUpperCase(), ETransparencyMode.PUBLIC)
        ).toBeUndefined()
    })

    it("refuses another event's policy", () => {
        const {policy} = sealed(policyText({eventId: OTHER_EVENT_ID}))
        expect(policyMismatch(policy, EVENT_ID, ETransparencyMode.PUBLIC)).toBe(
            EPolicyMismatch.EVENT
        )
    })

    it("refuses a policy whose visibility isn't the event's mode", () => {
        const {policy} = sealed(policyText({visibility: "private"}))
        expect(policyMismatch(policy, EVENT_ID, ETransparencyMode.PUBLIC)).toBe(
            EPolicyMismatch.VISIBILITY
        )
        expect(policyMismatch(policy, EVENT_ID, ETransparencyMode.PRIVATE)).toBeUndefined()
    })
})

describe("sameDraft", () => {
    it("tells whether a policy was sealed from a draft", () => {
        const {policy} = sealed(policyText())
        expect(sameDraft(policy, draft(draftPolicyText()).policy)).toBe(true)
        expect(sameDraft(policy, draft(draftPolicyText({name: "acme-2027"})).policy)).toBe(false)
    })
})

describe("the Board's commands", () => {
    it("check the downloaded draft, print its hash, then seal and sign it", () => {
        const name = draftFileName(EVENT_ID)
        expect(name).toBe(`vitrine-policy-draft-${EVENT_ID}.txt`)
        const commands = boardCommands(EVENT_ID)
        expect(commands[0]).toBe(`vitrine policy check ${name}`)
        expect(commands[1]).toBe(`vitrine policy id ${name}`)
        expect(commands[2]).toMatch(new RegExp(`^vitrine policy seal ${name} --key `))
        expect(commands[3]).toMatch(/^vitrine policy sign /)
    })
})

describe("readLogs", () => {
    const hex = "cd".repeat(32)

    it("merges each log's listing entry with its status, ballots first", () => {
        const logs = readLogs(sidecarLogs(hex), sidecarStatus(hex), ["hub-b.example"])
        expect(logs.map((l) => l.key)).toEqual(["ballots", "events"])
        const [ballots, events] = logs
        expect(ballots).toMatchObject({
            origin: "logs.acme.example/e/ballots",
            visibility: "public",
            size: 12,
            checkpoint: {size: 10, time: NOW - 30},
            policyHex: hex,
            frozen: false,
            queue: {rows: 2, oldestSeconds: 4.5},
        })
        expect(ballots.alert).toBeUndefined()
        expect(events.alert).toBe("witness-quorum-late")
        expect(events.checkpoint).toBeUndefined()
        expect(events.queue).toEqual({rows: 0})
    })

    it("says how far behind the served checkpoint each witness is", () => {
        const [ballots] = readLogs(sidecarLogs(hex), sidecarStatus(hex))
        expect(ballots.witnesses).toEqual([
            {name: "w1", current: true, alert: false, cosignedAgeSeconds: 30.2},
            {
                name: "w2",
                current: false,
                alert: true,
                behind: 3,
                cosignedAgeSeconds: 4000,
                lastError: "connection refused",
            },
            {name: "w3", current: false, alert: false},
        ])
    })

    it("lists the policy's hubs and the ones the log reports, with their last anchors", () => {
        const [ballots, events] = readLogs(sidecarLogs(hex), sidecarStatus(hex), ["h3"])
        expect(ballots.hubs).toEqual([
            {name: "h1", anchor: {size: 8, index: 41, time: NOW - 90}},
            {name: "h2"},
            {name: "h3"},
        ])
        expect(events.hubs).toEqual([{name: "h3"}])
    })

    it("reads the jsonb Harvest returns as text, too", () => {
        const logs = readLogs(JSON.stringify(sidecarLogs(hex)), JSON.stringify(sidecarStatus(hex)))
        expect(logs.map((l) => l.key)).toEqual(["ballots", "events"])
    })

    it("keeps a log the sidecar lists but hasn't reported a status for", () => {
        const logs = readLogs(sidecarLogs(hex), {})
        expect(logs.map((l) => l.key)).toEqual([`${EVENT_ID}-ballots`, `${EVENT_ID}-events`])
        expect(logs[0]).toMatchObject({size: 12, witnesses: [], hubs: []})
    })

    it.each([
        ["nothing", undefined, undefined],
        ["text that isn't JSON", "<html>", "{"],
        ["values of the wrong shape", [1, "x", null], {ballots: "x", events: [1]}],
    ])("reads %s as no logs", (_what, logs, status) => {
        expect(readLogs(logs, status)).toEqual([])
    })

    it("ignores fields of the wrong type", () => {
        const [log] = readLogs([], {
            ballots: {
                size: "12",
                checkpoint: {size: Number.NaN},
                policy: "not hex",
                witnesses: [{name: 7}, {current: true}, {name: "w1", size: "3"}],
                hubs: {h: "yesterday"},
            },
        })
        expect(log).toEqual({
            key: "ballots",
            frozen: false,
            witnesses: [{name: "w1", current: false, alert: false}],
            hubs: [{name: "h"}],
        })
    })
})

describe("readEnrolments", () => {
    it("reads each log's note", () => {
        const notes = [
            {log: "ballots", note: "vitrine/enrol/v1\n..."},
            {log: "events", note: "vitrine/enrol/v1\n..."},
        ]
        expect(readEnrolments(notes)).toEqual(notes)
        expect(readEnrolments(JSON.stringify(notes))).toEqual(notes)
    })

    it("skips entries without a log or a note", () => {
        expect(
            readEnrolments([{log: "ballots"}, {note: "x"}, {log: "events", note: ""}, null, "x"])
        ).toEqual([])
        expect(readEnrolments({log: "ballots", note: "x"})).toEqual([])
        expect(readEnrolments(undefined)).toEqual([])
    })
})

describe("safeExternalUrl", () => {
    it.each([
        ["https://explorer.acme.example/e/", "https://explorer.acme.example/e/"],
        ["http://localhost:8080/explorer", "http://localhost:8080/explorer"],
        ["http://127.0.0.1/explorer", "http://127.0.0.1/explorer"],
    ])("links to %s", (value, href) => {
        expect(safeExternalUrl(value)).toBe(href)
    })

    it.each([
        ["plain http", "http://explorer.acme.example/"],
        ["a script", "javascript:alert(1)"],
        ["data", "data:text/html,<script>alert(1)</script>"],
        ["credentials", "https://user:secret@explorer.acme.example/"],
        ["a relative path", "/explorer"],
        ["nothing", ""],
        ["a number", 42],
    ])("never links to %s", (_what, value) => {
        expect(safeExternalUrl(value)).toBeUndefined()
    })
})
