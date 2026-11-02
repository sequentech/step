// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useRef} from "react"
import {
    Alert,
    AlertTitle,
    Box,
    Button,
    Chip,
    MenuItem,
    Stack,
    TextField,
    Tooltip,
} from "@mui/material"
import {
    SnapshotOrigin,
    type ScenarioDefinition,
    type ScenarioId,
    type ScenarioSnapshot,
} from "@sequentech/ui-test-kit/fixtures/scenarios"

export interface ScenarioPickerProps {
    scenarios: readonly ScenarioDefinition[]
    /** The loaded snapshot, before local overrides. */
    snapshot: ScenarioSnapshot
    onSelect: (id: ScenarioId) => void
    onImport: (file: File) => void
    onExport: () => void
    onReset: () => void
    /** Why the last import was rejected. */
    importIssues?: string[]
}

export const ScenarioPicker: React.FC<ScenarioPickerProps> = ({
    scenarios,
    snapshot,
    onSelect,
    onImport,
    onExport,
    onReset,
    importIssues,
}) => {
    const fileInput = useRef<HTMLInputElement>(null)
    const {origin, changes} = snapshot.provenance
    const description = scenarios.find(({id}) => id === snapshot.scenarioId)?.description
    return (
        <Box className="scenario-picker">
            <Stack direction="row" spacing={1} alignItems="center" flexWrap="wrap" useFlexGap>
                <TextField
                    select
                    label="Scenario"
                    value={snapshot.scenarioId}
                    onChange={(event) => onSelect(event.target.value as ScenarioId)}
                    sx={{minWidth: 220}}
                >
                    {scenarios.map(({id, title}) => (
                        <MenuItem key={id} value={id}>
                            {title}
                        </MenuItem>
                    ))}
                </TextField>
                <Tooltip title={changes.length ? changes.join("; ") : "No local changes"}>
                    <Chip
                        label={origin === SnapshotOrigin.BUNDLED ? "Bundled" : "Imported"}
                        color={origin === SnapshotOrigin.BUNDLED ? "default" : "secondary"}
                        variant="outlined"
                    />
                </Tooltip>
                <Button variant="outlined" onClick={() => fileInput.current?.click()}>
                    Import snapshot
                </Button>
                <input
                    ref={fileInput}
                    hidden
                    type="file"
                    accept="application/json,.json"
                    onChange={(event) => {
                        const [file] = event.target.files ?? []
                        event.target.value = ""
                        if (file) onImport(file)
                    }}
                />
                <Button variant="outlined" onClick={onExport}>
                    Export snapshot
                </Button>
                <Button variant="outlined" color="warning" onClick={onReset}>
                    Reset
                </Button>
            </Stack>
            {description ? (
                <Box sx={{mt: 0.5, color: "text.secondary", fontSize: 12}}>{description}</Box>
            ) : null}
            {importIssues ? (
                <Alert severity="error" sx={{mt: 1}}>
                    <AlertTitle>The snapshot was not imported</AlertTitle>
                    <Box component="ul" sx={{m: 0, pl: 2}}>
                        {importIssues.map((issue) => (
                            <li key={issue}>{issue}</li>
                        ))}
                    </Box>
                </Alert>
            ) : null}
        </Box>
    )
}
