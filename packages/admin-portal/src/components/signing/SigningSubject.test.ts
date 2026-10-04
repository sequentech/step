// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {TFunction} from "i18next"
import i18next from "i18next"
import en from "@/translations/en"
import type {ISigningPanelData} from "@/lib/signing/api"
import {SigningAction} from "@/lib/signing/types"
import {requestTitle} from "./format"
import {subjectNotes, worded} from "./SigningSubject"

let t: TFunction

beforeAll(async () => {
    const instance = i18next.createInstance()
    await instance.init({lng: "en", resources: {en}, interpolation: {escapeValue: false}})
    t = instance.getFixedT("en", "translations")
})

describe("a signed value in the organization's words", () => {
    it("names each channel", () => {
        expect(worded(t, "channels", ["ONLINE", "TELEPHONE"], "ONLINE, TELEPHONE")).toBe(
            "Online, Telephone"
        )
    })

    it("names each channel's status before, not CHANNEL=STATUS", () => {
        expect(worded(t, "from", ["ONLINE=OPEN"], "ONLINE=OPEN")).toBe("Online: Open")
        expect(
            worded(t, "from", ["ONLINE=PAUSED", "TELEPHONE=NOT_STARTED"], "ONLINE=PAUSED, …")
        ).toBe("Online: Paused, Telephone: Not started")
    })

    it("keeps a code it has no words for as signed", () => {
        expect(worded(t, "from", ["SMOKE=SIGNAL"], "SMOKE=SIGNAL")).toBe("SMOKE: SIGNAL")
        expect(worded(t, "channels", ["SMOKE"], "SMOKE")).toBe("SMOKE")
        expect(worded(t, "count", 3, "3")).toBe("3")
    })
})

describe("the signing rules a configuration version changes", () => {
    it("says each rule after and before the version", () => {
        expect(worded(t, "signing_rules", ["close-voting=1>2"], "close-voting=1>2")).toBe(
            "Close voting: needs 2 (was 1)"
        )
        expect(
            worded(t, "signing_rules", ["transmit-results=off>2", "open-voting=3>off"], "…")
        ).toBe("Transmit results: needs 2 (was off), Open voting: off (was 3)")
    })

    it("leaves out what it was when that is unknown or the same", () => {
        expect(worded(t, "signing_rules", ["close-voting=2"], "close-voting=2")).toBe(
            "Close voting: needs 2"
        )
        expect(worded(t, "signing_rules", ["close-voting=2>2"], "close-voting=2>2")).toBe(
            "Close voting: needs 2"
        )
    })

    it("names the action alone in a request started before rules carried numbers", () => {
        expect(worded(t, "signing_rules", ["close-voting"], "close-voting")).toBe("Close voting")
    })
})

describe("the request's title", () => {
    const panel = (action: SigningAction, configRevision: string | null) =>
        ({
            request: {action, config_revision: configRevision},
            election_name: "Madrid PE",
            area_name: "Spain",
        }) as unknown as ISigningPanelData

    it("numbers the configuration version it approves", () => {
        expect(requestTitle(t, panel(SigningAction.ApproveConfiguration, "18"))).toBe(
            "Configuration version 18 · Madrid PE · Spain"
        )
    })

    it("keeps the action's short name without a version", () => {
        expect(requestTitle(t, panel(SigningAction.ApproveConfiguration, null))).toBe(
            "Configuration version · Madrid PE · Spain"
        )
        expect(requestTitle(t, panel(SigningAction.GenerateElectionReturns, "18"))).toBe(
            "Election returns · Madrid PE · Spain"
        )
    })
})

describe("the rows that explain an action beside what it signs", () => {
    it("says what happens after a voter's approval", () => {
        expect(subjectNotes(t, SigningAction.ApproveVoter)).toEqual([
            {label: "After approval", value: "The voter's credentials are issued and sent to them"},
        ])
    })

    it("says a trustee's key share was checked and where it is recorded", () => {
        expect(subjectNotes(t, SigningAction.ConfirmKeyShare)).toEqual([
            {label: "Your key share", value: "Checked: it is your key share for this ceremony"},
            {label: "Recorded in", value: "The keys ceremony and the bulletin board"},
        ])
        expect(subjectNotes(t, SigningAction.ContributeKeyShare)).toEqual([
            {label: "Your key share", value: "Checked: it is your key share for this ceremony"},
            {label: "Recorded in", value: "The tally session"},
        ])
    })

    it("adds nothing to the other actions", () => {
        expect(subjectNotes(t, SigningAction.CloseVoting)).toEqual([])
    })
})

describe("the signing copy in the organization's words", () => {
    it("names a Post with the organization's term in the certificate checks", async () => {
        const renamed = JSON.parse(JSON.stringify(en))
        renamed.translations.signing.terms.post = "Faculty"
        const instance = i18next.createInstance()
        await instance.init({
            lng: "en",
            resources: {en: renamed},
            interpolation: {escapeValue: false},
        })
        const faculty = instance.getFixedT("en", "translations")
        expect(faculty("signing.dialog.checks.passed.post-binding")).toBe(
            "Registered for this Faculty"
        )
        expect(faculty("signing.dialog.checks.failed.post-binding")).toBe(
            "Registered for another Faculty"
        )
    })

    it("says the tally starts the election returns and Reports only the participation report", () => {
        const returns = t("signing.actions.generate-election-returns.description")
        expect(returns).toMatch(/^Started by the tally/)
        expect(returns).not.toMatch(/Reports/)
        expect(t("signing.actions.generate-reports.description")).toMatch(
            /^Started by the tally for the Initialization Report and in Reports for the participation report\./
        )
    })
})
