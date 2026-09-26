// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Alert, Box, Button, Chip, MenuItem, Stack, TextField, Typography} from "@mui/material"
import {
    BoundKey,
    POLICY_FIELDS,
    PolicyScope,
    type ContestOverrides,
    type ContestPolicies,
    type PolicyKey,
} from "../policies"

export interface PolicyPanelContest {
    id: string
    name: string
    preferential: boolean
    /** The snapshot's own values. */
    baseline: ContestPolicies & Record<BoundKey, number>
    overrides: ContestOverrides
    /** Why the overridden bounds are not applied, if they are not. */
    boundsIssue?: string
}

export interface PolicyPanelProps {
    contests: readonly PolicyPanelContest[]
    /** An undefined value restores the snapshot's own value. */
    onChange: (contestId: string, key: PolicyKey | BoundKey, value?: string | number) => void
    onClear: () => void
}

const SCENARIO_VALUE = ""
const OVERRIDDEN = "Overridden"

const BOUND_LABELS: Record<BoundKey, string> = {
    [BoundKey.MIN]: "Minimum votes",
    [BoundKey.MAX]: "Maximum votes",
}

const ContestPolicyForm: React.FC<{
    contest: PolicyPanelContest
    onChange: PolicyPanelProps["onChange"]
}> = ({contest, onChange}) => {
    const titleId = `policy-contest-${contest.id}`
    const fields = POLICY_FIELDS.filter(
        ({scope}) => scope === PolicyScope.ALL || contest.preferential
    )
    return (
        <Box component="section" aria-labelledby={titleId} sx={{mb: 2}}>
            <Stack direction="row" spacing={1} alignItems="center" sx={{mb: 1}}>
                <Typography id={titleId} variant="subtitle2" component="h3">
                    {contest.name}
                </Typography>
                <Chip
                    size="small"
                    variant="outlined"
                    label={contest.preferential ? "Preferential" : "Plurality"}
                />
            </Stack>
            <Box sx={{display: "grid", gridTemplateColumns: "1fr 1fr", gap: 1}}>
                {fields.map(({key, label, values}) => (
                    <TextField
                        key={key}
                        select
                        label={label}
                        value={contest.overrides[key] ?? SCENARIO_VALUE}
                        helperText={contest.overrides[key] ? OVERRIDDEN : undefined}
                        slotProps={{inputLabel: {shrink: true}, select: {displayEmpty: true}}}
                        onChange={(event) =>
                            onChange(contest.id, key, event.target.value || undefined)
                        }
                    >
                        <MenuItem value={SCENARIO_VALUE}>
                            <em>Scenario: {contest.baseline[key] ?? "portal default"}</em>
                        </MenuItem>
                        {values.map((value) => (
                            <MenuItem key={value} value={value}>
                                {value}
                            </MenuItem>
                        ))}
                    </TextField>
                ))}
                {Object.values(BoundKey).map((key) => (
                    <TextField
                        key={key}
                        type="number"
                        label={BOUND_LABELS[key]}
                        value={contest.overrides[key] ?? contest.baseline[key]}
                        helperText={contest.overrides[key] === undefined ? undefined : OVERRIDDEN}
                        slotProps={{htmlInput: {min: 0, step: 1}}}
                        onChange={(event) => {
                            const value = Number(event.target.value)
                            if (event.target.value === "" || value === contest.baseline[key])
                                onChange(contest.id, key, undefined)
                            else if (Number.isInteger(value) && value >= 0)
                                onChange(contest.id, key, value)
                        }}
                    />
                ))}
            </Box>
            {contest.boundsIssue ? (
                <Alert severity="warning" sx={{mt: 1}}>
                    {contest.boundsIssue}; the scenario bounds stay in use.
                </Alert>
            ) : null}
        </Box>
    )
}

/** Local overrides of each contest's presentation and validation policies. */
export const PolicyPanel: React.FC<PolicyPanelProps> = ({contests, onChange, onClear}) => {
    const overridden = contests.some(({overrides}) => Object.keys(overrides).length > 0)
    return (
        <Box className="policy-panel">
            <Typography variant="body2" color="text.secondary" sx={{mb: 2}}>
                Changes apply to the snapshot document and reload the production screens with a
                fresh voter session. Exported snapshots include them.
            </Typography>
            {contests.length ? (
                contests.map((contest) => (
                    <ContestPolicyForm key={contest.id} contest={contest} onChange={onChange} />
                ))
            ) : (
                <Typography variant="body2">This area has no contests to configure.</Typography>
            )}
            <Button variant="outlined" onClick={onClear} disabled={!overridden}>
                Clear overrides
            </Button>
        </Box>
    )
}
