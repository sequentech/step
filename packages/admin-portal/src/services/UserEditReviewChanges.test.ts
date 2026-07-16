// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {UserProfileAttribute} from "@/gql/graphql"
import {computeUserDiff, formatFieldValue, UserBaseline} from "./UserEditReviewChanges"

const t = (key: string) => key

const userAttributes = [
    {name: "first_name", display_name: "First name"},
    {name: "last_name", display_name: "Last name"},
    {name: "email", display_name: "Email"},
    {name: "username", display_name: "Username"},
    {name: "area-id", display_name: "Area"},
    {name: "sequent.read-only.mobile-number", display_name: "Mobile"},
    {name: "trustee", display_name: "Trustee"},
    {name: "permission_labels", display_name: "Permission labels"},
] as UserProfileAttribute[]

const buildBaseline = (): UserBaseline => ({
    user: {
        id: "voter-1",
        first_name: "Jane",
        last_name: "Doe",
        email: "jane@example.com",
        username: "jane.doe",
        enabled: true,
        area: {id: "area-1"},
        attributes: {
            "sequent.read-only.mobile-number": ["+1000"],
            "trustee": ["Trustee A"],
            "permission_labels": ["voter", "admin"],
        },
    },
    phoneInputs: {},
})

describe("computeUserDiff", () => {
    // An empty diff is what EditUserForm uses to decide review should not be shown.
    it("returns no rows when nothing changed", () => {
        const baseline = buildBaseline()
        const diff = computeUserDiff(
            baseline,
            {
                user: baseline.user,
                phoneInputs: {},
                selectedActedTrustee: baseline.user.attributes?.trustee?.[0] as string,
            },
            userAttributes,
            t
        )
        expect(diff).toEqual([])
    })

    it("does not report selectedActedTrustee as changed when it only mirrors the baseline (auto-populate on mount)", () => {
        const baseline = buildBaseline()
        const diff = computeUserDiff(
            baseline,
            {user: baseline.user, phoneInputs: {}, selectedActedTrustee: "Trustee A"},
            userAttributes,
            t
        )
        expect(diff.find((row) => row.field === "trustee")).toBeUndefined()
    })

    it("reports one row per changed field with correct labels and values", () => {
        const baseline = buildBaseline()
        const current = {
            user: {
                ...baseline.user,
                first_name: "Janet",
                enabled: false,
                area: {id: "area-2"},
            },
            phoneInputs: {"sequent.read-only.mobile-number": ["+2000"]},
            selectedActedTrustee: "Trustee B",
        }
        const diff = computeUserDiff(baseline, current, userAttributes, t)
        const fields = diff.map((row) => row.field).sort()
        expect(fields).toEqual(
            [
                "area-id",
                "enabled",
                "first_name",
                "sequent.read-only.mobile-number",
                "trustee",
            ].sort()
        )

        const firstName = diff.find((row) => row.field === "first_name")
        expect(firstName).toMatchObject({
            label: "First name",
            currentValue: "Jane",
            newValue: "Janet",
        })

        const enabled = diff.find((row) => row.field === "enabled")
        expect(enabled?.currentValue).toBe("common.label.yes")
        expect(enabled?.newValue).toBe("common.label.no")
    })

    it("omits unchanged fields even when other fields change", () => {
        const baseline = buildBaseline()
        const current = {
            user: {...baseline.user, last_name: "Smith"},
            phoneInputs: {},
            selectedActedTrustee: "Trustee A",
        }
        const diff = computeUserDiff(baseline, current, userAttributes, t)
        expect(diff).toHaveLength(1)
        expect(diff[0].field).toBe("last_name")
    })

    it("never reports username, since it is not editable in edit mode", () => {
        const baseline = buildBaseline()
        const current = {
            user: {...baseline.user, username: "someone.else"},
            phoneInputs: {},
            selectedActedTrustee: "Trustee A",
        }
        const diff = computeUserDiff(baseline, current, userAttributes, t)
        expect(diff.find((row) => row.field === "username")).toBeUndefined()
    })

    it("treats reordered multivalued attributes as unchanged", () => {
        const baseline = buildBaseline()
        const current = {
            user: {
                ...baseline.user,
                attributes: {
                    ...baseline.user.attributes,
                    permission_labels: ["admin", "voter"],
                },
            },
            phoneInputs: {},
            selectedActedTrustee: "Trustee A",
        }
        const diff = computeUserDiff(baseline, current, userAttributes, t)
        expect(diff.find((row) => row.field === "permission_labels")).toBeUndefined()
    })

    it("only diffs mobile-number from phoneInputs once the admin actually touches it", () => {
        const baseline = buildBaseline()
        const current = {
            user: baseline.user,
            phoneInputs: {},
            selectedActedTrustee: "Trustee A",
        }
        const diff = computeUserDiff(baseline, current, userAttributes, t)
        expect(diff.find((row) => row.field === "sequent.read-only.mobile-number")).toBeUndefined()
    })
})

describe("formatFieldValue", () => {
    it("formats booleans via the translated yes/no labels", () => {
        expect(formatFieldValue(true, t)).toBe("common.label.yes")
        expect(formatFieldValue(false, t)).toBe("common.label.no")
    })

    it("formats missing values as a dash", () => {
        expect(formatFieldValue(undefined, t)).toBe("-")
        expect(formatFieldValue(null, t)).toBe("-")
        expect(formatFieldValue("", t)).toBe("-")
    })

    it("joins array values with a comma", () => {
        expect(formatFieldValue(["voter", "admin"], t)).toBe("voter, admin")
    })
})
