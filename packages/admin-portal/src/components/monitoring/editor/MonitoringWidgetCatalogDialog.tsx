// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Box,
    Button,
    Chip,
    CircularProgress,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    List,
    ListItem,
    ListItemText,
    ListSubheader,
    TextField,
} from "@mui/material"
import {filterCatalog, groupBySource, type IWidgetCatalogEntry} from "./catalog"
import {humanize} from "./formValues"

export interface MonitoringWidgetCatalogDialogProps {
    open: boolean
    /** `undefined` while the catalog loads. */
    entries?: IWidgetCatalogEntry[]
    error?: string
    /** Widgets already on the dashboard; they can be added again, with other values. */
    onDashboard: ReadonlyArray<string>
    onAdd: (widgetId: string) => void
    onClose: () => void
}

/** Add widget: the event's widgets grouped by data source, searchable by requirement ID. */
export const MonitoringWidgetCatalogDialog: React.FC<MonitoringWidgetCatalogDialogProps> = ({
    open,
    entries,
    error,
    onDashboard,
    onAdd,
    onClose,
}) => {
    const {t} = useTranslation()
    const [query, setQuery] = useState("")
    const groups = groupBySource(filterCatalog(entries ?? [], query))
    const sourceLabel = (id: string) =>
        id ? t(`monitoring.editor.sources.${id}`, {defaultValue: humanize(id)}) : "—"

    return (
        <Dialog
            open={open}
            onClose={onClose}
            maxWidth="sm"
            fullWidth
            aria-labelledby="monitoring-catalog-title"
        >
            <DialogTitle id="monitoring-catalog-title">
                {t("monitoring.editor.catalog.title")}
            </DialogTitle>
            <DialogContent dividers>
                <TextField
                    fullWidth
                    size="small"
                    type="search"
                    label={t("monitoring.editor.catalog.search")}
                    value={query}
                    onChange={(event) => setQuery(event.target.value)}
                    sx={{mb: 2}}
                />
                {error ? <Alert severity="error">{error}</Alert> : null}
                {!entries && !error ? (
                    <Box sx={{display: "flex", justifyContent: "center", py: 4}}>
                        <CircularProgress aria-label={t("monitoring.editor.catalog.title")} />
                    </Box>
                ) : null}
                {entries && groups.length === 0 ? (
                    <Alert severity="info">{t("monitoring.editor.catalog.empty")}</Alert>
                ) : null}
                {groups.map((group) => (
                    <List
                        key={group.source}
                        dense
                        aria-label={sourceLabel(group.source)}
                        subheader={
                            <ListSubheader disableSticky>{sourceLabel(group.source)}</ListSubheader>
                        }
                    >
                        {group.entries.map((entry) => (
                            <ListItem
                                key={entry.id}
                                secondaryAction={
                                    <Button
                                        size="small"
                                        onClick={() => onAdd(entry.id)}
                                        aria-label={`${t("monitoring.editor.catalog.add")} ${entry.title}`}
                                    >
                                        {t("monitoring.editor.catalog.add")}
                                    </Button>
                                }
                            >
                                <ListItemText
                                    primary={entry.title}
                                    secondary={
                                        <Box
                                            component="span"
                                            sx={{
                                                display: "flex",
                                                gap: 0.5,
                                                flexWrap: "wrap",
                                                mt: 0.5,
                                            }}
                                        >
                                            {entry.requirements.map((requirement) => (
                                                <Chip
                                                    key={requirement}
                                                    component="span"
                                                    size="small"
                                                    variant="outlined"
                                                    label={requirement}
                                                />
                                            ))}
                                            {onDashboard.includes(entry.id) ? (
                                                <Chip
                                                    component="span"
                                                    size="small"
                                                    label={t(
                                                        "monitoring.editor.catalog.onDashboard"
                                                    )}
                                                />
                                            ) : null}
                                        </Box>
                                    }
                                />
                            </ListItem>
                        ))}
                    </List>
                ))}
            </DialogContent>
            <DialogActions>
                <Button onClick={onClose}>{t("monitoring.editor.catalog.close")}</Button>
            </DialogActions>
        </Dialog>
    )
}

export default MonitoringWidgetCatalogDialog
