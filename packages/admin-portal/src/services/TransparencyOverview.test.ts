// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import * as verify from "@vitrine/verify"
import {NOW, policyHexOf, policyText, sidecarLogs, sidecarStatus} from "@/fixtures/vitrinePolicy"
import {ETransparencyMode, ILogView, readLogs} from "./Transparency"
import {
    EHealth,
    EHealthIssue,
    EPolicyCheck,
    EWitnessState,
    hubRows,
    IHealthInput,
    policyFacts,
    recordFigures,
    recordHealth,
    witnessQuorum,
    witnessRows,
} from "./TransparencyOverview"

beforeAll(async () => {
    await verify.init()
})

const signed = policyText()
const hex = policyHexOf(signed)
const policy = () => verify.parsePolicy(signed)
const logs = (): ILogView[] => readLogs(sidecarLogs(hex), sidecarStatus(hex), ["h1", "h2"])

const quiet = (): ILogView[] =>
    logs().map((log) => ({
        ...log,
        alert: undefined,
        witnesses: log.witnesses.map((w) => ({...w, alert: false})),
    }))

const health = (o: Partial<IHealthInput> = {}) =>
    recordHealth({
        mode: ETransparencyMode.PUBLIC,
        policy: EPolicyCheck.SEALED,
        policyHex: hex,
        loggedPolicyId: hex,
        loggedPolicyHex: hex,
        logs: quiet(),
        ...o,
    })

describe("recordHealth", () => {
    it("is healthy when the logs run under the sealed policy and report no alert", () => {
        expect(health()).toEqual({health: EHealth.HEALTHY, issues: []})
    })

    it.each([
        [ETransparencyMode.DISABLED, EPolicyCheck.SEALED, EHealth.DISABLED],
        [ETransparencyMode.PUBLIC, EPolicyCheck.CHECKING, EHealth.CHECKING],
        [ETransparencyMode.PUBLIC, EPolicyCheck.UNCHECKED, EHealth.UNCHECKED],
        [ETransparencyMode.PRIVATE, EPolicyCheck.NONE, EHealth.NO_POLICY],
    ])("reads mode %s with policy %s as %s", (mode, check, expected) => {
        expect(health({mode, policy: check})).toEqual({health: expected, issues: []})
    })

    it("says the logs haven't reported when there are none yet", () => {
        expect(health({logs: []}).health).toBe(EHealth.UNREPORTED)
    })

    it("lists every alert the logs and their witnesses report", () => {
        expect(health({logs: logs()})).toEqual({
            health: EHealth.ATTENTION,
            issues: [
                {issue: EHealthIssue.WITNESS_ALERT, log: "ballots", witness: "w2"},
                {issue: EHealthIssue.LOG_ALERT, log: "events", alert: "witness-quorum-late"},
            ],
        })
    })

    it("needs attention when the logs run under another policy", () => {
        const other = "ef".repeat(32)
        const result = health({
            loggedPolicyId: other,
            loggedPolicyHex: other,
            logs: quiet().map((log) => ({...log, policyHex: other})),
        })
        expect(result.health).toBe(EHealth.ATTENTION)
        expect(result.issues).toEqual([
            {issue: EHealthIssue.POLICY_ID_MISMATCH},
            {issue: EHealthIssue.LOG_OTHER_POLICY, log: "ballots"},
            {issue: EHealthIssue.LOG_OTHER_POLICY, log: "events"},
        ])
    })

    it("needs attention when the stored policy isn't valid, even before the logs report", () => {
        expect(
            health({
                policy: EPolicyCheck.INVALID,
                policyHex: undefined,
                loggedPolicyId: null,
                logs: [],
            })
        ).toEqual({
            health: EHealth.ATTENTION,
            issues: [{issue: EHealthIssue.STORED_POLICY_INVALID}],
        })
    })

    it("takes a policy ID Harvest doesn't return as no mismatch", () => {
        expect(health({loggedPolicyId: null, loggedPolicyHex: undefined}).health).toBe(
            EHealth.HEALTHY
        )
    })
})

describe("policyFacts", () => {
    it("reads the quorum, hubs, monitors and periods the overview shows", () => {
        expect(policyFacts(policy())).toEqual({
            visibility: ETransparencyMode.PUBLIC,
            operator: "sequent",
            quorum: {k: 2, n: 3},
            appHubs: 2,
            outsideHubs: 2,
            bootstrapHubs: [],
            mmd: 60,
            anchorPeriod: 10,
            browserPeriod: 600,
            monitors: [
                {name: "observers", party: "monitorco1"},
                {name: "observers2", party: "monitorco2"},
            ],
            monitorQuorum: 2,
        })
    })

    it("counts the hubs run by neither the operator nor the platform", () => {
        const facts = policyFacts(
            verify.parsePolicy(policyText({platform: "hubco2", anchorHubs: true}))
        )
        expect(facts.platform).toBe("hubco2")
        expect(facts.appHubs).toBe(2)
        expect(facts.outsideHubs).toBe(1)
        expect(facts.hubRule).toEqual({min: 2, outside: 1})
    })

    it("counts a bootstrap hub apart from the app hubs", () => {
        const facts = policyFacts(verify.parsePolicy(policyText({bootstrap: true})))
        expect(facts.appHubs).toBe(2)
        expect(facts.bootstrapHubs).toEqual(["h3"])
    })

    it("reads a private policy as private", () => {
        expect(
            policyFacts(verify.parsePolicy(policyText({visibility: "private"}))).visibility
        ).toBe(ETransparencyMode.PRIVATE)
    })

    it("has no redaction approvers when the policy allows no redaction", () => {
        expect(policyFacts(policy()).redaction).toBeUndefined()
    })
})

describe("witnessQuorum", () => {
    it("reads the quorum's group as k of its members", () => {
        expect(witnessQuorum(policy().witnesses)).toEqual({k: 2, n: 3})
    })

    it("reads a quorum that names one witness as 1 of 1", () => {
        expect(witnessQuorum({groups: new Map(), quorum: "w1"})).toEqual({k: 1, n: 1})
    })
})

describe("witnessRows", () => {
    it("gives each of the policy's witnesses its state, with its party when the policy names it", () => {
        expect(witnessRows(policy(), logs())).toEqual([
            {name: "w1", state: EWitnessState.SIGNING},
            {name: "w2", party: "observers", state: EWitnessState.ALERT, lagSeconds: 4000},
            {name: "w3", state: EWitnessState.LAGGING},
        ])
    })

    it("says which witnesses no log reports", () => {
        expect(witnessRows(policy(), []).map((w) => w.state)).toEqual([
            EWitnessState.UNREPORTED,
            EWitnessState.UNREPORTED,
            EWitnessState.UNREPORTED,
        ])
    })

    it("keeps each witness's worst state across the logs", () => {
        const [ballots] = quiet()
        const events: ILogView = {
            ...ballots,
            key: "events",
            witnesses: [{name: "w1", current: false, alert: false, cosignedAgeSeconds: 14}],
        }
        const rows = witnessRows(policy(), [ballots, events])
        expect(rows[0]).toEqual({name: "w1", state: EWitnessState.LAGGING, lagSeconds: 14})
    })

    it("lists a witness a log reports that the policy doesn't name", () => {
        const [ballots] = quiet()
        const log = {...ballots, witnesses: [{name: "w9", current: true, alert: false}]}
        expect(witnessRows(undefined, [log])).toEqual([{name: "w9", state: EWitnessState.SIGNING}])
    })
})

describe("hubRows and recordFigures", () => {
    it("gives each app hub its operator and last anchor", () => {
        expect(hubRows(policy(), logs())).toEqual([
            {name: "h1", operator: "hubco1", lastAnchor: NOW - 90},
            {name: "h2", operator: "hubco2"},
        ])
    })

    it("leaves bootstrap hubs out of the app hubs", () => {
        const p = verify.parsePolicy(policyText({bootstrap: true}))
        expect(hubRows(p, logs()).map((h) => h.name)).toEqual(["h1", "h2"])
    })

    it("adds up what the logs report", () => {
        expect(recordFigures(hubRows(policy(), logs()), logs())).toEqual({
            entries: 10,
            witnessedLogs: 1,
            queueRows: 2,
            queueOldestSeconds: 4.5,
            lastAnchor: NOW - 90,
            anchoredHubs: 1,
        })
    })

    it("reports nothing it wasn't given", () => {
        expect(recordFigures([], [])).toEqual({witnessedLogs: 0, anchoredHubs: 0})
    })
})
