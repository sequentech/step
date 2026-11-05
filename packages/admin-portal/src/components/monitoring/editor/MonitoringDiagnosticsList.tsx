// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Box,
    Chip,
    List,
    ListItem,
    ListItemButton,
    ListItemIcon,
    ListItemText,
    Typography,
} from "@mui/material"
import ErrorOutlineIcon from "@mui/icons-material/ErrorOutline"
import WarningAmberIcon from "@mui/icons-material/WarningAmber"
import CheckCircleOutlineIcon from "@mui/icons-material/CheckCircleOutline"
import type {IEditorDiagnostic} from "@/components/monitoring/lib/diagnostics"
import {EMonitoringProblemSeverity} from "./types"

export interface MonitoringDiagnosticsListProps {
    diagnostics: IEditorDiagnostic[]
    /** No local checks ran: say that problems appear only after a render or Validate. */
    localUnavailable?: boolean
    onSelect?: (diagnostic: IEditorDiagnostic) => void
}

/** The problems found in a document, each linked to its line in the YAML editor. */
export const MonitoringDiagnosticsList: React.FC<MonitoringDiagnosticsListProps> = ({
    diagnostics,
    localUnavailable = false,
    onSelect,
}) => {
    const {t} = useTranslation()
    return (
        <Box
            component="section"
            aria-label={t("monitoring.editor.diagnostics.title")}
            sx={{display: "flex", flexDirection: "column", gap: 1}}
        >
            <Typography variant="subtitle2" component="h3">
                {t("monitoring.editor.diagnostics.title")}
            </Typography>
            {localUnavailable ? (
                <Alert severity="info" variant="outlined">
                    {t("monitoring.editor.diagnostics.localUnavailable")}
                </Alert>
            ) : null}
            {diagnostics.length === 0 ? (
                <Box sx={{display: "flex", alignItems: "center", gap: 1, color: "success.main"}}>
                    <CheckCircleOutlineIcon fontSize="small" aria-hidden />
                    <Typography variant="body2">
                        {t("monitoring.editor.diagnostics.none")}
                    </Typography>
                </Box>
            ) : (
                <List dense disablePadding>
                    {diagnostics.map((diagnostic, index) => {
                        const isError = diagnostic.severity === EMonitoringProblemSeverity.ERROR
                        const severity = t(
                            `monitoring.editor.diagnostics.severity.${diagnostic.severity}`
                        )
                        return (
                            <ListItem
                                key={`${diagnostic.origin}-${diagnostic.path}-${index}`}
                                disablePadding
                            >
                                <ListItemButton
                                    onClick={() => onSelect?.(diagnostic)}
                                    aria-label={`${severity}: ${diagnostic.message}`}
                                >
                                    <ListItemIcon sx={{minWidth: 32}}>
                                        {isError ? (
                                            <ErrorOutlineIcon color="error" fontSize="small" />
                                        ) : (
                                            <WarningAmberIcon color="warning" fontSize="small" />
                                        )}
                                    </ListItemIcon>
                                    <ListItemText
                                        primary={diagnostic.message}
                                        secondary={
                                            <Box
                                                component="span"
                                                sx={{
                                                    display: "flex",
                                                    flexWrap: "wrap",
                                                    gap: 0.5,
                                                    alignItems: "center",
                                                }}
                                            >
                                                <span>
                                                    {t("monitoring.editor.diagnostics.line", {
                                                        line: diagnostic.line,
                                                    })}
                                                </span>
                                                {diagnostic.path ? (
                                                    <code>{diagnostic.path}</code>
                                                ) : null}
                                                <Chip
                                                    size="small"
                                                    variant="outlined"
                                                    component="span"
                                                    label={t(
                                                        `monitoring.editor.diagnostics.origin.${diagnostic.origin}`
                                                    )}
                                                />
                                                {diagnostic.engine_code ? (
                                                    <Chip
                                                        size="small"
                                                        component="span"
                                                        label={t(
                                                            "monitoring.editor.diagnostics.engineCode",
                                                            {code: diagnostic.engine_code}
                                                        )}
                                                    />
                                                ) : null}
                                            </Box>
                                        }
                                    />
                                </ListItemButton>
                            </ListItem>
                        )
                    })}
                </List>
            )}
        </Box>
    )
}

export default MonitoringDiagnosticsList
