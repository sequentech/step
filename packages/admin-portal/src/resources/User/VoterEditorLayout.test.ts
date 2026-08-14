// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {UserProfileAttribute, UserProfileAttributeGroup} from "@/gql/graphql"
import {
    getVoterInputType,
    groupVoterAttributes,
    VoterAttributeGroups,
    VoterEditorRoot,
    VoterField,
    VOTER_EDITOR_FIXED_FIELDS,
} from "./VoterEditorLayout"

const attribute = (name: string, group?: string, inputType?: string): UserProfileAttribute =>
    ({
        annotations: inputType ? {inputType} : undefined,
        group,
        name,
    }) as UserProfileAttribute

const group = (name: string): UserProfileAttributeGroup =>
    ({name, display_header: name}) as UserProfileAttributeGroup

describe("groupVoterAttributes", () => {
    it("preserves profile order and repeats non-contiguous group runs", () => {
        const attributes = [
            attribute("first_name", "identity"),
            attribute("email"),
            attribute("last_name", "identity"),
        ]

        const runs = groupVoterAttributes(attributes, [group("identity")])

        expect(runs.map((run) => run.name)).toEqual(["identity", undefined, "identity"])
        expect(runs.map((run) => run.key)).toEqual(["identity-1", "ungrouped-1", "identity-2"])
        expect(runs.flatMap((run) => run.attributes.map((item) => item.name))).toEqual([
            "first_name",
            "email",
            "last_name",
        ])
    })

    it("coalesces contiguous fields and omits groups with no visible fields", () => {
        const runs = groupVoterAttributes(
            [attribute("email", "contact"), attribute("mobile", "contact")],
            [group("empty"), group("contact")]
        )

        expect(runs).toHaveLength(1)
        expect(runs[0].attributes.map((item) => item.name)).toEqual(["email", "mobile"])
        expect(runs[0].group?.name).toBe("contact")
    })

    it("does not mutate attributes or group configuration", () => {
        const attributes = [attribute("email", "contact"), attribute("first_name")]
        const groups = [group("contact")]
        const before = JSON.stringify({attributes, groups})

        groupVoterAttributes(attributes, groups)

        expect(JSON.stringify({attributes, groups})).toBe(before)
    })
})

describe("voter editor stable selectors", () => {
    it("keeps canonical field names and metadata in data attributes", () => {
        const element = VoterField({
            children: React.createElement("input"),
            inputType: "select",
            name: "custom.name-with_punctuation",
            required: true,
        }) as React.ReactElement<any>

        expect(element.props.className).toBe("voter-field")
        expect(element.props["data-field-name"]).toBe("custom.name-with_punctuation")
        expect(element.props["data-input-type"]).toBe("select")
        expect(element.props["data-required"]).toBe("true")
    })

    it.each(["create", "edit"] as const)("exposes %s mode on the editor root", (mode) => {
        const wrapper = VoterEditorRoot({children: "form", mode}) as React.ReactElement<any>
        const element = wrapper.props.children as React.ReactElement<any>

        expect(element.props.className).toBe("voter-editor")
        expect(element.props["data-mode"]).toBe(mode)
    })

    it("keeps canonical group names in the group selector", () => {
        const runs = groupVoterAttributes(
            [attribute("email", "group.with-punctuation")],
            [group("group.with-punctuation")]
        )
        const element = VoterAttributeGroups({
            getDescription: () => "",
            getHeader: () => "Header",
            renderField: () => "field",
            runs,
        }) as React.ReactElement<any>
        const fieldset = element.props.children[0] as React.ReactElement<any>

        expect(element.props.className).toBe("voter-editor__groups")
        expect(fieldset.props.className).toBe("voter-attribute-group")
        expect(fieldset.props["data-group-name"]).toBe("group.with-punctuation")
    })

    it("classifies every specialized input branch", () => {
        expect(getVoterInputType(attribute("status", undefined, "select"))).toBe("select")
        expect(getVoterInputType(attribute("birth_date", undefined, "html5-date"))).toBe(
            "html5-date"
        )
        expect(getVoterInputType(attribute("sequent.read-only.mobile-number"))).toBe("tel")
        expect(getVoterInputType(attribute("trustee"))).toBe("trustee-select")
        expect(getVoterInputType(attribute("authorized-election-ids"))).toBe("election-multiselect")
        expect(getVoterInputType(attribute("permission_labels"))).toBe("permission-labels")
        expect(getVoterInputType(attribute("plain"))).toBe("text")
    })

    it("publishes stable names for every Step-owned field", () => {
        expect(VOTER_EDITOR_FIXED_FIELDS).toEqual({
            enabled: "checkbox",
            area: "area-select",
            password: "password",
            confirm_password: "password",
            password_temporary: "checkbox",
        })
        expect(new Set(Object.keys(VOTER_EDITOR_FIXED_FIELDS)).size).toBe(5)
    })
})
