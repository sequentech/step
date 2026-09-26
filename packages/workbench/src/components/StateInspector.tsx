// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {Alert, Box, Chip, MenuItem, Stack, TextField, Typography} from "@mui/material"
import type {ScenarioSnapshot} from "@sequentech/ui-test-kit/fixtures/scenarios"
import type {SelectionValidation} from "voting-portal/src/preview/ballotPipeline"
import type {RootState} from "voting-portal/src/store/store"
import type {PolicyOverrides} from "../policies"
import type {SequentCoreInfo} from "../sequentCore"
import type {WorkbenchEvent} from "../state"

const ALL_SLICES = "all"

export interface StateInspectorProps {
    /** The snapshot the portal shows, overrides included. */
    snapshot: ScenarioSnapshot
    overrides: PolicyOverrides
    location: string
    production: RootState
    /** The voting screen's validation of the current selection, or why it failed. */
    validation?: SelectionValidation | Error
    contestNames: Readonly<Record<string, string>>
    events: readonly WorkbenchEvent[]
    sequentCore: SequentCoreInfo
}

const Section: React.FC<React.PropsWithChildren<{title: string}>> = ({title, children}) => (
    <Box component="section" aria-label={title} sx={{mb: 2}}>
        <Typography variant="subtitle2" component="h3" sx={{mb: 0.5}}>
            {title}
        </Typography>
        {children}
    </Box>
)

const Json: React.FC<{value: unknown; label: string}> = ({value, label}) => (
    <Box
        component="pre"
        aria-label={label}
        tabIndex={0}
        sx={{
            m: 0,
            p: 1,
            maxHeight: 320,
            overflow: "auto",
            fontSize: 11,
            bgcolor: "grey.100",
            borderRadius: 1,
        }}
    >
        {JSON.stringify(value, null, 2)}
    </Box>
)

const Facts: React.FC<{facts: [string, React.ReactNode][]}> = ({facts}) => (
    <Box
        component="dl"
        sx={{m: 0, display: "grid", gridTemplateColumns: "max-content 1fr", gap: "2px 12px"}}
    >
        {facts.map(([term, detail]) => (
            <React.Fragment key={term}>
                <Box component="dt" sx={{color: "text.secondary"}}>
                    {term}
                </Box>
                <Box component="dd" sx={{m: 0, wordBreak: "break-all"}}>
                    {detail}
                </Box>
            </React.Fragment>
        ))}
    </Box>
)

/** The snapshot, the validation of the current selection and the production store. */
export const StateInspector: React.FC<StateInspectorProps> = ({
    snapshot,
    overrides,
    location,
    production,
    validation,
    contestNames,
    events,
    sequentCore,
}) => {
    const [slice, setSlice] = useState<keyof RootState | typeof ALL_SLICES>("ballotSelections")
    const {provenance} = snapshot
    return (
        <Box className="state-inspector">
            <Section title="Snapshot">
                <Facts
                    facts={[
                        ["Scenario", snapshot.scenarioId],
                        ["Version", snapshot.version],
                        ["Origin", `${provenance.origin}, ${provenance.createdAt}`],
                        ["Channel", snapshot.channel],
                        ["Area", snapshot.areaId],
                        ["Route", location],
                    ]}
                />
                {provenance.changes.length ? (
                    <Box component="ul" aria-label="Snapshot changes" sx={{m: 0, pl: 2}}>
                        {provenance.changes.map((change) => (
                            <li key={change}>{change}</li>
                        ))}
                    </Box>
                ) : null}
            </Section>
            <Section title="Validation">
                {validation instanceof Error ? (
                    <Alert severity="error">{validation.message}</Alert>
                ) : validation ? (
                    <>
                        <Stack direction="row" spacing={1} sx={{mb: 1}}>
                            <Chip
                                size="small"
                                color={validation.nextBlocked ? "error" : "success"}
                                label={validation.nextBlocked ? "Next blocked" : "Next allowed"}
                            />
                            <Chip
                                size="small"
                                variant={validation.confirmBeforeReview ? "filled" : "outlined"}
                                label={
                                    validation.confirmBeforeReview
                                        ? "Asks to confirm"
                                        : "No confirmation"
                                }
                            />
                        </Stack>
                        {validation.contests.map(({contestId, errors, alerts}) => (
                            <Box key={contestId} sx={{fontSize: 12}}>
                                <strong>{contestNames[contestId] ?? contestId}</strong>:{" "}
                                {[...errors, ...alerts].join(", ") || "valid"}
                            </Box>
                        ))}
                    </>
                ) : (
                    <Typography variant="body2">No ballot is loaded.</Typography>
                )}
            </Section>
            <Section title="Policy overrides">
                <Json value={overrides} label="Policy overrides JSON" />
            </Section>
            <Section title="Production store">
                <TextField
                    select
                    label="Slice"
                    value={slice}
                    onChange={(event) => setSlice(event.target.value as typeof slice)}
                    sx={{mb: 1, minWidth: 200}}
                >
                    <MenuItem value={ALL_SLICES}>Whole state</MenuItem>
                    {Object.keys(production).map((key) => (
                        <MenuItem key={key} value={key}>
                            {key}
                        </MenuItem>
                    ))}
                </TextField>
                <Json
                    value={slice === ALL_SLICES ? production : production[slice]}
                    label="Production store JSON"
                />
            </Section>
            <Section title="sequent-core">
                <Facts
                    facts={[
                        ["Directory", sequentCore.directory],
                        ["WASM", `${sequentCore.wasmBytes} bytes, ${sequentCore.modifiedAt}`],
                        ["SHA-256", sequentCore.wasmSha256],
                    ]}
                />
            </Section>
            <Section title="Events">
                <Box component="ol" aria-label="Workbench events" sx={{m: 0, pl: 2, fontSize: 12}}>
                    {[...events].reverse().map(({id, kind, message}) => (
                        <li key={id}>
                            <strong>{kind}</strong> {message}
                        </li>
                    ))}
                </Box>
            </Section>
        </Box>
    )
}
