// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box, Typography, styled} from "@mui/material"
import {UserProfileAttribute, UserProfileAttributeGroup} from "@/gql/graphql"

export type VoterEditorMode = "create" | "edit"

export interface VoterAttributeRun {
    attributes: UserProfileAttribute[]
    group?: UserProfileAttributeGroup
    key: string
    name?: string
}

export const VOTER_EDITOR_FIXED_FIELDS = {
    enabled: "checkbox",
    area: "area-select",
    password: "password",
    confirm_password: "password",
    password_temporary: "checkbox",
} as const

export const groupVoterAttributes = (
    attributes: UserProfileAttribute[],
    groups: UserProfileAttributeGroup[]
): VoterAttributeRun[] => {
    const groupsByName = new Map(
        groups.flatMap((group) => (group.name ? [[group.name, group] as const] : []))
    )
    const occurrences = new Map<string, number>()

    return attributes.reduce<VoterAttributeRun[]>((runs, attribute) => {
        const name = attribute.group ?? undefined
        const previous = runs.at(-1)
        if (previous && previous.name === name) {
            previous.attributes.push(attribute)
            return runs
        }

        const occurrenceKey = name ?? "ungrouped"
        const occurrence = (occurrences.get(occurrenceKey) ?? 0) + 1
        occurrences.set(occurrenceKey, occurrence)
        runs.push({
            attributes: [attribute],
            group: name ? groupsByName.get(name) : undefined,
            key: `${occurrenceKey}-${occurrence}`,
            name,
        })
        return runs
    }, [])
}

export const getVoterInputType = (attribute: UserProfileAttribute): string => {
    const annotatedType = attribute.annotations?.inputType
    if (typeof annotatedType === "string" && annotatedType) {
        return annotatedType
    }

    const name = attribute.name?.toLowerCase() ?? ""
    if (name.includes("mobile-number")) return "tel"
    if (name.includes("trustee")) return "trustee-select"
    if (name.includes("authorized-election-ids")) return "election-multiselect"
    if (name.includes("permission_labels")) return "permission-labels"
    return "text"
}

export const VoterEditorRoot: React.FC<{
    children: React.ReactNode
    customCss?: string
    mode: VoterEditorMode
}> = ({children, customCss = "", mode}) => (
    <TenantStyledEditor customCss={customCss}>
        <Box className="voter-editor" data-mode={mode}>
            {children}
        </Box>
    </TenantStyledEditor>
)

const TenantStyledEditor = styled(Box, {
    shouldForwardProp: (prop) => prop !== "customCss",
})<{customCss: string}>`
    ${({customCss}) => customCss}
`

const Field = styled(Box)`
    box-sizing: border-box;
    max-width: 100%;
    min-width: 0;
    width: 100%;

    & > * {
        box-sizing: border-box;
        max-width: 100%;
        min-width: 0;
        width: 100%;
    }

    .MuiFormControl-root {
        max-width: 100% !important;
        min-width: 0 !important;
    }
`

export const VoterField: React.FC<{
    children: React.ReactNode
    inputType: string
    name: string
    required: boolean
}> = ({children, inputType, name, required}) => (
    <Field
        className="voter-field"
        data-field-name={name}
        data-input-type={inputType}
        data-required={required ? "true" : "false"}
    >
        {children}
    </Field>
)

const Groups = styled(Box)`
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 24px;
    min-width: 0;
    width: 100%;

    .voter-attribute-group {
        border: 0;
        box-sizing: border-box;
        margin: 0;
        min-width: 0;
        padding: 0;
        width: 100%;
    }

    .voter-attribute-group__legend {
        box-sizing: border-box;
        font-size: 1rem;
        font-weight: 600;
        margin-bottom: 4px;
        max-width: 100%;
        overflow-wrap: anywhere;
        padding: 0;
        white-space: normal;
    }

    .voter-attribute-group__description {
        margin-bottom: 12px;
    }

    .voter-attribute-group__grid {
        box-sizing: border-box;
        display: grid;
        gap: 16px;
        grid-template-columns: minmax(0, 1fr);
        min-width: 0;
        width: 100%;
    }
`

interface VoterAttributeGroupsProps {
    getDescription: (run: VoterAttributeRun) => string
    getHeader: (run: VoterAttributeRun) => string
    renderField: (attribute: UserProfileAttribute, index: number) => React.ReactNode
    runs: VoterAttributeRun[]
}

export const VoterAttributeGroups: React.FC<VoterAttributeGroupsProps> = ({
    getDescription,
    getHeader,
    renderField,
    runs,
}) => (
    <Groups className="voter-editor__groups">
        {runs.map((run) => {
            const description = getDescription(run)
            const descriptionId = description
                ? `voter-attribute-group-description-${run.key}`
                : undefined
            const header = getHeader(run)
            return (
                <Box
                    component="fieldset"
                    className="voter-attribute-group"
                    data-group-name={run.name ?? ""}
                    aria-describedby={descriptionId}
                    key={run.key}
                >
                    {header && (
                        <Box component="legend" className="voter-attribute-group__legend">
                            {header}
                        </Box>
                    )}
                    {description && (
                        <Typography
                            className="voter-attribute-group__description"
                            color="text.secondary"
                            id={descriptionId}
                            variant="body2"
                        >
                            {description}
                        </Typography>
                    )}
                    <Box className="voter-attribute-group__grid">
                        {run.attributes.map(renderField)}
                    </Box>
                </Box>
            )
        })}
    </Groups>
)
