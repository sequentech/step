// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {keepUnsubmittedValues} from "./keepUnsubmittedValues"

const formState = {
    ballot_identity: "pseudonym",
    network_data_policy: "do-not-store",
    receipts: {policy: "signed-by-ballot-box", checks_period_policy: "unlimited", unknown: "kept"},
    voter_signing_policy: "with-signature",
    ballot_box_seal_policy: "seal-at-close",
    unknown_setting: {nested: {value: 7}},
    logo_url: "https://logo.example.test/council.png",
    custom_filters: [{id: 1}, {id: 2}],
}

describe("keepUnsubmittedValues", () => {
    it("restores the values react-hook-form left out of the submit, at any depth", () => {
        // What the form submitted with receipts.policy and voter_signing_policy disabled.
        const submitted = {
            ballot_identity: "pseudonym",
            network_data_policy: "do-not-store",
            receipts: {checks_period_policy: "unlimited", unknown: "kept"},
            ballot_box_seal_policy: "seal-at-close",
            unknown_setting: {nested: {value: 7}},
            logo_url: "https://logo.example.test/council.png",
            custom_filters: [{id: 1}, {id: 2}],
        }
        expect(keepUnsubmittedValues(submitted, formState)).toEqual(formState)
    })

    it("restores an object react-hook-form removed because it was left empty", () => {
        const {receipts: _receipts, ...submitted} = formState
        expect(
            keepUnsubmittedValues(submitted, {...formState, receipts: {policy: "disabled"}})
        ).toEqual({...formState, receipts: {policy: "disabled"}})
    })

    it("keeps the submitted values, cleared ones included", () => {
        const submitted = {
            ...formState,
            receipts: {...formState.receipts, policy: "disabled"},
            voter_signing_policy: "no-signature",
            logo_url: "",
            ballot_box_seal_policy: null,
            network_data_policy: undefined,
            custom_filters: [{id: 2}],
        }
        const kept = keepUnsubmittedValues(submitted, formState)
        expect(kept).toEqual(submitted)
        expect(Object.prototype.hasOwnProperty.call(kept, "network_data_policy")).toBe(true)
        expect(JSON.parse(JSON.stringify(kept))).not.toHaveProperty("network_data_policy")
    })

    it("does not change the submitted values or merge into arrays", () => {
        const submitted = {receipts: {checks_period_policy: "until-date"}, custom_filters: []}
        const copy = JSON.parse(JSON.stringify(submitted))
        const kept = keepUnsubmittedValues(submitted, formState)
        expect(submitted).toEqual(copy)
        expect(kept.custom_filters).toEqual([])
        expect(kept.receipts).toEqual({
            policy: "signed-by-ballot-box",
            checks_period_policy: "until-date",
            unknown: "kept",
        })
    })

    it("leaves values that are not objects as they are", () => {
        expect(keepUnsubmittedValues(undefined, formState)).toBeUndefined()
        expect(keepUnsubmittedValues({a: 1}, undefined)).toEqual({a: 1})
        expect(keepUnsubmittedValues({a: {b: 1}}, {a: "x"})).toEqual({a: {b: 1}})
    })
})
