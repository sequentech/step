// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import * as verify from "@vitrine/verify"
import * as pure from "@vitrine/verify/pure"
import {
    ELECTION_ID,
    EVENT_ID,
    KEYS,
    NOW,
    PUBLIC_KEY,
    RELEASE,
    STYLE_TEXT,
    policyIdOf,
    policyText,
    scenario,
    signNote,
    styleHashes,
    type IConfigFields,
    type IScenario,
    type IScenarioOptions,
} from "../../fixtures/vitrine"
import {checkBooth, EBoothCheckReason, EBoothRefusalState, IBoothCheckInput} from "./boothCheck"
import {readTransparency} from "./presentation"
import type {IVitrineSdk} from "./sdk"
import {ETransparencyMode, IVitrineVoterContext} from "./types"

const sdk: IVitrineSdk = {verify, pure}

beforeAll(async () => {
    await verify.init()
})

const OTHER_ID = "9d0f6a1e-3b2c-4d5e-8f70-1a2b3c4d5e6f"

const inputFor = (s: IScenario, overrides: Partial<IBoothCheckInput> = {}): IBoothCheckInput => ({
    mode: s.mode,
    presentation: readTransparency(s.presentation).vitrine,
    context: s.context,
    eventId: EVENT_ID,
    electionId: ELECTION_ID,
    publicKey: PUBLIC_KEY,
    styleText: STYLE_TEXT,
    release: RELEASE,
    now: NOW,
    ...overrides,
})

const check = (options: IScenarioOptions = {}, overrides: Partial<IBoothCheckInput> = {}) =>
    checkBooth(sdk, inputFor(scenario(options), overrides))

const withContext = (s: IScenario, context: Partial<IVitrineVoterContext>) =>
    inputFor(s, {context: {...s.context, ...context}})

const expectRefusal = (
    result: ReturnType<typeof checkBooth>,
    reason: EBoothCheckReason,
    state: EBoothRefusalState
) => {
    expect(result).toMatchObject({ok: false, reason, state})
}

describe("checkBooth", () => {
    it.each([ETransparencyMode.PUBLIC, ETransparencyMode.PRIVATE] as const)(
        "passes a %s event whose statements are logged and match the booth",
        (mode) => {
            const s = scenario({mode})
            const result = checkBooth(sdk, inputFor(s))
            expect(result.ok).toBe(true)
            if (result.ok) {
                expect(result.policy.id).toBe(s.policyId)
                expect(result.config.election).toBe(ELECTION_ID)
            }
        }
    )

    it("waits for the voter context", () => {
        expectRefusal(
            check({}, {context: undefined}),
            EBoothCheckReason.CONTEXT_MISSING,
            EBoothRefusalState.PENDING
        )
    })

    it("refuses a context whose mode isn't the event's", () => {
        const s = scenario()
        expectRefusal(
            checkBooth(sdk, withContext(s, {mode: ETransparencyMode.PRIVATE})),
            EBoothCheckReason.MODE_MISMATCH,
            EBoothRefusalState.INVALID
        )
    })

    it("pins a public event's policy from the presentation only", () => {
        const s = scenario()
        const presentation = {...readTransparency(s.presentation).vitrine, policy: undefined}
        expectRefusal(
            checkBooth(sdk, inputFor(s, {presentation, context: {...s.context, policy: s.policy}})),
            EBoothCheckReason.POLICY_MISSING,
            EBoothRefusalState.PENDING
        )
    })

    it("pins a private event's policy from the context", () => {
        const s = scenario({mode: ETransparencyMode.PRIVATE})
        expectRefusal(
            checkBooth(sdk, withContext(s, {policy: null})),
            EBoothCheckReason.POLICY_MISSING,
            EBoothRefusalState.PENDING
        )
    })

    it("refuses a policy the board didn't sign", () => {
        const s = scenario({mode: ETransparencyMode.PRIVATE})
        const text = pure.notes.parseNote(s.policy).text
        expectRefusal(
            checkBooth(sdk, withContext(s, {policy: signNote(text, [KEYS.stranger])})),
            EBoothCheckReason.POLICY_INVALID,
            EBoothRefusalState.INVALID
        )
    })

    it("refuses a policy whose ID isn't the pinned one", () => {
        const s = scenario({mode: ETransparencyMode.PRIVATE})
        const other = policyText(ETransparencyMode.PRIVATE, ["epoch 1200"])
        expectRefusal(
            checkBooth(sdk, withContext(s, {policy: other})),
            EBoothCheckReason.POLICY_ID_MISMATCH,
            EBoothRefusalState.INVALID
        )
    })

    it("refuses a policy whose visibility isn't the event's mode", () => {
        const s = scenario()
        const other = policyText(ETransparencyMode.PRIVATE)
        const presentation = {policy_id: policyIdOf(other), policy: other}
        expectRefusal(
            checkBooth(sdk, inputFor(s, {presentation})),
            EBoothCheckReason.POLICY_VISIBILITY_MISMATCH,
            EBoothRefusalState.INVALID
        )
    })

    it("waits for the config statement and the opening milestone", () => {
        const s = scenario()
        const config = checkBooth(sdk, withContext(s, {config: null}))
        expectRefusal(config, EBoothCheckReason.CONFIG_MISSING, EBoothRefusalState.PENDING)
        expect(config.ok ? undefined : config.policy?.id).toBe(s.policyId)
        expectRefusal(
            checkBooth(sdk, withContext(s, {opened: null})),
            EBoothCheckReason.OPENED_MISSING,
            EBoothRefusalState.PENDING
        )
    })

    it("refuses a config statement no authority signed", () => {
        expectRefusal(
            check({configSigners: [KEYS.stranger]}),
            EBoothCheckReason.CONFIG_INVALID,
            EBoothRefusalState.INVALID
        )
    })

    it("refuses a config whose receipt is for another entry", () => {
        const s = scenario()
        const config = {...s.context.config!, chain: s.context.opened!.chain}
        expectRefusal(
            checkBooth(sdk, withContext(s, {config})),
            EBoothCheckReason.CONFIG_INVALID,
            EBoothRefusalState.INVALID
        )
    })

    it("waits while the events checkpoint doesn't meet the quorum", () => {
        expectRefusal(
            check({cosigners: 1}),
            EBoothCheckReason.CONFIG_UNVERIFIED,
            EBoothRefusalState.PENDING
        )
    })

    it("refuses a milestone that doesn't open voting", () => {
        expectRefusal(
            check({openedKind: "voting-closed"}),
            EBoothCheckReason.OPENED_INVALID,
            EBoothRefusalState.INVALID
        )
    })

    it("refuses a milestone that opens another configuration", () => {
        expectRefusal(
            check({openedConfigLeaf: pure.encoding.b64encode(new Uint8Array(32))}),
            EBoothCheckReason.OPENED_CONFIG_MISMATCH,
            EBoothRefusalState.FAILED
        )
    })

    const contradictions: Array<[Partial<IConfigFields>, EBoothCheckReason]> = [
        [{event: OTHER_ID}, EBoothCheckReason.EVENT_MISMATCH],
        [{election: OTHER_ID}, EBoothCheckReason.ELECTION_MISMATCH],
        [{policy: `sha256:${"00".repeat(32)}`}, EBoothCheckReason.POLICY_MISMATCH],
        [{transparency: ETransparencyMode.PRIVATE}, EBoothCheckReason.TRANSPARENCY_MISMATCH],
        [{key: pure.encoding.b64encode(new Uint8Array(32))}, EBoothCheckReason.KEY_MISMATCH],
        [{styles: pure.encoding.b64encode(new Uint8Array(32))}, EBoothCheckReason.STYLES_MISMATCH],
        [{releases: ["sequent/voting-portal 10.3.0"]}, EBoothCheckReason.RELEASE_MISMATCH],
    ]

    it.each(contradictions)("refuses a logged config with %o", (config, reason) => {
        expectRefusal(check({config}), reason, EBoothRefusalState.FAILED)
    })

    it("refuses a key other than the logged one", () => {
        const key = pure.encoding.b64encode(new Uint8Array(32).fill(7)).replace(/=+$/, "")
        expectRefusal(
            check({}, {publicKey: key}),
            EBoothCheckReason.KEY_MISMATCH,
            EBoothRefusalState.FAILED
        )
    })

    it("refuses a ballot style that isn't in the logged list", () => {
        expectRefusal(
            check({}, {styleText: `${STYLE_TEXT} `}),
            EBoothCheckReason.STYLES_MISMATCH,
            EBoothRefusalState.FAILED
        )
    })

    it("refuses a style list that isn't sorted or has repeats", () => {
        const s = scenario()
        const sorted = styleHashes()
        for (const list of [[...sorted].reverse(), [sorted[0], ...sorted]]) {
            expectRefusal(
                checkBooth(sdk, withContext(s, {style_hashes: list})),
                EBoothCheckReason.STYLES_MISMATCH,
                EBoothRefusalState.FAILED
            )
        }
    })

    it("refuses a release the config doesn't name", () => {
        expectRefusal(
            check({}, {release: "10.5.0"}),
            EBoothCheckReason.RELEASE_MISMATCH,
            EBoothRefusalState.FAILED
        )
    })
})
