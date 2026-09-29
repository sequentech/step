// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useMemo, useRef, useState} from "react"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Box,
    Button,
    Checkbox,
    CircularProgress,
    Dialog,
    DialogActions,
    DialogContent,
    DialogContentText,
    DialogTitle,
    FormControl,
    FormControlLabel,
    FormGroup,
    FormLabel,
    IconButton,
    InputLabel,
    Menu,
    MenuItem,
    Paper,
    Select,
    Tab,
    Tabs,
    TextField,
    Tooltip,
    Typography,
} from "@mui/material"
import AddIcon from "@mui/icons-material/Add"
import ArrowUpwardIcon from "@mui/icons-material/ArrowUpward"
import ArrowDownwardIcon from "@mui/icons-material/ArrowDownward"
import DragIndicatorIcon from "@mui/icons-material/DragIndicator"
import MoreVertIcon from "@mui/icons-material/MoreVert"
import {countBySeverity, type IEditorDiagnostic} from "@/components/monitoring/lib/diagnostics"
import {deleteIn, insertIn, moveIn, setIn} from "@/components/monitoring/lib/yamlPatch"
import type {IMonitoringEditorApi} from "./api"
import {
    EMonitoringConfigKind,
    EMonitoringSaveChange,
    EMonitoringSaveStatus,
    MONITORING_GRID_COLUMNS,
    type IMonitoringDashboardDefinition,
    type IMonitoringLayoutItem,
} from "./types"
import {useYamlDraft} from "./useYamlDraft"
import {EFormAccess, type TLocalValidate} from "./yamlDraft"
import {MonitoringYamlEditor, type IMonitoringYamlEditorHandle} from "./MonitoringYamlEditor"
import {MonitoringDiagnosticsList} from "./MonitoringDiagnosticsList"
import {MonitoringPreviewFooter} from "./MonitoringPreviewFooter"
import {MonitoringConflictDialog} from "./MonitoringConflictDialog"
import {MonitoringWidgetCatalogDialog} from "./MonitoringWidgetCatalogDialog"
import {MonitoringThemeEditorDialog} from "./MonitoringThemeEditorDialog"
import {MonitoringResetToPresetDialog} from "./MonitoringResetToPresetDialog"
import {
    DEFAULT_THEME,
    countThemeWidgets,
    loadWidgetCatalog,
    type IWidgetCatalogEntry,
} from "./catalog"
import {SCOPE_SELECTORS, copyId, stringList} from "./formValues"
import {
    EDocumentLoad,
    EMessageTone,
    authorName,
    useConfigDocument,
    type IDocumentMessages,
} from "./useConfigDocument"

export enum EDashboardTab {
    WIDGETS = "WIDGETS",
    YAML = "YAML",
}

export interface MonitoringDashboardEditorProps {
    open: boolean
    api: IMonitoringEditorApi
    dashboardId: string
    /** Opens Configure widget for a widget on this dashboard. */
    onConfigureWidget?: (widgetId: string) => void
    localValidate?: TLocalValidate
    themeValidate?: TLocalValidate
    onClose: () => void
    onSaved?: (revision: number) => void
    /** The event's configuration was replaced by a preset. */
    onReset?: () => void
}

const DASHBOARD_MESSAGES: IDocumentMessages = {
    loadFailed: "monitoring.editor.document.loadFailed",
    saved: "monitoring.editor.dashboard.saved",
    refused: "monitoring.editor.dashboard.refused",
    validated: "monitoring.editor.document.validated",
    invalid: "monitoring.editor.document.invalid",
    requestFailed: "monitoring.editor.dashboard.requestFailed",
}
const NEW_WIDGET_WIDTH = 6
const WIDTHS = Array.from({length: MONITORING_GRID_COLUMNS}, (_, index) => index + 1)
const LAYOUT = ["layout"]

const asDashboard = (value: unknown): Partial<IMonitoringDashboardDefinition> =>
    value && typeof value === "object" && !Array.isArray(value)
        ? (value as Partial<IMonitoringDashboardDefinition>)
        : {}

const layoutItems = (value: unknown): IMonitoringLayoutItem[] => {
    const layout = asDashboard(value).layout
    return Array.isArray(layout)
        ? layout.filter(
              (item): item is IMonitoringLayoutItem =>
                  Boolean(item) && typeof item === "object" && typeof item.widget === "string"
          )
        : []
}

const reason = (error: unknown) => (error instanceof Error ? error.message : String(error))

/**
 * Editing dashboard: its title, selectors and theme, and its widgets, which
 * can be added from the catalog, reordered (buttons or drag and drop),
 * resized, duplicated and removed. Every change is a change to the
 * dashboard's YAML, saved as one revision.
 */
export const MonitoringDashboardEditor: React.FC<MonitoringDashboardEditorProps> = ({
    open,
    api,
    dashboardId,
    onConfigureWidget,
    localValidate,
    themeValidate,
    onClose,
    onSaved,
    onReset,
}) => {
    const {t} = useTranslation()
    const [tab, setTab] = useState(EDashboardTab.WIDGETS)
    const [catalog, setCatalog] = useState<IWidgetCatalogEntry[] | undefined>()
    const [catalogError, setCatalogError] = useState("")
    const [themes, setThemes] = useState<string[]>([])
    const [adding, setAdding] = useState(false)
    const [theming, setTheming] = useState<{key: string; count: number} | null>(null)
    const [resetting, setResetting] = useState(false)
    const [confirmDiscard, setConfirmDiscard] = useState(false)
    const [menu, setMenu] = useState<{anchor: HTMLElement; index: number} | null>(null)
    const [dragging, setDragging] = useState<number | null>(null)
    /** The item being dragged, read at drop time, before any re-render. */
    const dragged = useRef<number | null>(null)
    const [over, setOver] = useState<number | null>(null)
    const editor = useRef<IMonitoringYamlEditorHandle>(null)

    const draft = useYamlDraft({text: "", localValidate})
    const {controller} = draft
    const stored = useConfigDocument({
        api,
        kind: EMonitoringConfigKind.DASHBOARD,
        key: dashboardId,
        open,
        controller,
        messages: DASHBOARD_MESSAGES,
        onSaved,
    })
    const {setMessage} = stored

    useEffect(() => {
        if (!open) return
        let current = true
        setCatalog(undefined)
        setCatalogError("")
        loadWidgetCatalog(api).then(
            (entries) => current && setCatalog(entries),
            (error) =>
                current &&
                setCatalogError(
                    t("monitoring.editor.dashboard.requestFailed", {reason: reason(error)})
                )
        )
        api.listConfig().then(
            (documents) =>
                current &&
                setThemes(
                    documents
                        .filter((entry) => entry.kind === EMonitoringConfigKind.THEME)
                        .map((entry) => entry.key)
                ),
            () => undefined
        )
        return () => {
            current = false
        }
    }, [open, api, t])

    const dashboard = asDashboard(draft.value)
    const layout = layoutItems(draft.value)
    const readOnly = draft.formAccess !== EFormAccess.EDITABLE
    const ready = stored.load === EDocumentLoad.READY
    const theme = typeof dashboard.theme === "string" ? dashboard.theme : DEFAULT_THEME
    const selectors = stringList(dashboard.selectors)
    const counts = useMemo(() => countBySeverity(draft.diagnostics), [draft.diagnostics])
    const titles = useMemo(
        () => new Map((catalog ?? []).map((entry) => [entry.id, entry.title])),
        [catalog]
    )
    const titleOf = (widgetId: string) => titles.get(widgetId) ?? widgetId
    const patch = (change: (text: string) => string) => controller.patch(change)
    const move = (from: number, to: number) => patch((text) => moveIn(text, LAYOUT, from, to))

    const toggleSelector = (selector: string, checked: boolean) => {
        const next = SCOPE_SELECTORS.filter((item) =>
            item === selector ? checked : selectors.includes(item)
        )
        patch((text) => setIn(text, ["selectors"], next.length ? next : undefined))
    }

    const add = (widgetId: string) => {
        patch((text) =>
            insertIn(text, LAYOUT, layout.length, {widget: widgetId, width: NEW_WIDGET_WIDTH})
        )
        setAdding(false)
    }

    const duplicate = async (index: number) => {
        const item = layout[index]
        if (!item) return
        const taken = new Set([
            ...(catalog ?? []).map((entry) => entry.id),
            ...layout.map((entry) => entry.widget),
        ])
        const id = copyId(item.widget, taken)
        try {
            const source = await api.getConfig({
                kind: EMonitoringConfigKind.WIDGET,
                key: item.widget,
            })
            const yaml = setIn(source.yaml, ["id"], id)
            const outcome = await api.saveConfig({
                kind: EMonitoringConfigKind.WIDGET,
                key: id,
                yaml,
                change: EMonitoringSaveChange.UPSERT,
            })
            if (outcome.status !== EMonitoringSaveStatus.SAVED) {
                setMessage({
                    tone: EMessageTone.ERROR,
                    text: t("monitoring.editor.dashboard.refused"),
                })
                if (outcome.status === EMonitoringSaveStatus.INVALID) {
                    controller.setServerProblems(outcome.problems)
                }
                return
            }
            patch((text) => insertIn(text, LAYOUT, index + 1, {...item, widget: id}))
            setCatalog((entries) => {
                if (!entries) return entries
                const original = entries.find((entry) => entry.id === item.widget)
                return [
                    ...entries,
                    {
                        id,
                        title: original?.title ?? item.widget,
                        source: original?.source ?? "",
                        requirements: original?.requirements ?? [],
                    },
                ]
            })
            setMessage({
                tone: EMessageTone.SUCCESS,
                text: t("monitoring.editor.dashboard.duplicated", {id}),
            })
        } catch (error) {
            setMessage({
                tone: EMessageTone.ERROR,
                text: t("monitoring.editor.dashboard.requestFailed", {reason: reason(error)}),
            })
        }
    }

    const editTheme = async () => {
        let count = layout.length
        try {
            count = await countThemeWidgets(api, theme, dashboardId, draft.value)
        } catch {
            // This dashboard's own widgets, when the others cannot be read.
        }
        setTheming({key: theme, count})
    }

    const reveal = (diagnostic: IEditorDiagnostic) => {
        setTab(EDashboardTab.YAML)
        setTimeout(() => editor.current?.reveal(diagnostic.from, diagnostic.to), 0)
    }

    const cancel = () => {
        if (draft.dirty) setConfirmDiscard(true)
        else onClose()
    }

    const drop = (index: number) => {
        const from = dragged.current
        if (from !== null && from !== index) move(from, index)
        dragged.current = null
        setDragging(null)
        setOver(null)
    }

    const menuItem = menu ? layout[menu.index] : undefined

    return (
        <Dialog
            open={open}
            onClose={cancel}
            maxWidth="lg"
            fullWidth
            aria-labelledby="monitoring-dashboard-editor-title"
        >
            <DialogTitle id="monitoring-dashboard-editor-title">
                {t("monitoring.editor.dashboard.editing")}
                {dashboard.title ? ` · ${dashboard.title}` : ""}
            </DialogTitle>
            <DialogContent dividers>
                {stored.load === EDocumentLoad.LOADING ? (
                    <Box sx={{display: "flex", justifyContent: "center", py: 6}}>
                        <CircularProgress aria-label={t("monitoring.editor.dashboard.editing")} />
                    </Box>
                ) : null}
                {stored.load === EDocumentLoad.FAILED ? (
                    <Alert severity="error">{stored.loadError}</Alert>
                ) : null}
                {ready ? (
                    <Box
                        sx={{
                            display: "grid",
                            gap: 2,
                            gridTemplateColumns: {xs: "1fr", md: "minmax(0, 2fr) minmax(0, 1fr)"},
                        }}
                    >
                        <Box sx={{minWidth: 0}}>
                            <Tabs
                                value={tab}
                                onChange={(_, next: EDashboardTab) => setTab(next)}
                                aria-label={t("monitoring.editor.dashboard.editing")}
                                sx={{mb: 2}}
                            >
                                <Tab
                                    value={EDashboardTab.WIDGETS}
                                    id="dashboard-tab-widgets"
                                    aria-controls="dashboard-panel-widgets"
                                    label={t("monitoring.editor.dashboard.widgets")}
                                />
                                <Tab
                                    value={EDashboardTab.YAML}
                                    id="dashboard-tab-yaml"
                                    aria-controls="dashboard-panel-yaml"
                                    label={t("monitoring.editor.yaml.label")}
                                />
                            </Tabs>
                            <div
                                role="tabpanel"
                                id="dashboard-panel-widgets"
                                aria-labelledby="dashboard-tab-widgets"
                                hidden={tab !== EDashboardTab.WIDGETS}
                            >
                                {tab === EDashboardTab.WIDGETS ? (
                                    <Box sx={{display: "flex", flexDirection: "column", gap: 2}}>
                                        {readOnly ? (
                                            <Alert severity="warning">
                                                {t("monitoring.editor.yaml.readOnly")}
                                            </Alert>
                                        ) : null}
                                        <TextField
                                            size="small"
                                            label={t("monitoring.editor.dashboard.title")}
                                            value={dashboard.title ?? ""}
                                            disabled={readOnly}
                                            onChange={(event) =>
                                                patch((text) =>
                                                    setIn(text, ["title"], event.target.value)
                                                )
                                            }
                                        />
                                        <FormControl component="fieldset" disabled={readOnly}>
                                            <FormLabel component="legend">
                                                {t("monitoring.editor.dashboard.selectors")}
                                            </FormLabel>
                                            <FormGroup row>
                                                {SCOPE_SELECTORS.map((selector) => (
                                                    <FormControlLabel
                                                        key={selector}
                                                        label={t(
                                                            `monitoring.editor.scopeSelector.${selector}`
                                                        )}
                                                        control={
                                                            <Checkbox
                                                                checked={selectors.includes(
                                                                    selector
                                                                )}
                                                                onChange={(event) =>
                                                                    toggleSelector(
                                                                        selector,
                                                                        event.target.checked
                                                                    )
                                                                }
                                                            />
                                                        }
                                                    />
                                                ))}
                                            </FormGroup>
                                        </FormControl>
                                        <Box
                                            sx={{
                                                display: "flex",
                                                gap: 1,
                                                flexWrap: "wrap",
                                                alignItems: "center",
                                            }}
                                        >
                                            <FormControl
                                                size="small"
                                                sx={{flex: "1 1 200px"}}
                                                disabled={readOnly}
                                            >
                                                <InputLabel id="dashboard-theme-label">
                                                    {t("monitoring.editor.dashboard.theme")}
                                                </InputLabel>
                                                <Select
                                                    labelId="dashboard-theme-label"
                                                    label={t("monitoring.editor.dashboard.theme")}
                                                    value={theme}
                                                    onChange={(event) =>
                                                        patch((text) =>
                                                            setIn(
                                                                text,
                                                                ["theme"],
                                                                event.target.value ===
                                                                    DEFAULT_THEME &&
                                                                    dashboard.theme === undefined
                                                                    ? undefined
                                                                    : event.target.value
                                                            )
                                                        )
                                                    }
                                                >
                                                    {Array.from(
                                                        new Set([DEFAULT_THEME, ...themes, theme])
                                                    ).map((key) => (
                                                        <MenuItem key={key} value={key}>
                                                            {key}
                                                        </MenuItem>
                                                    ))}
                                                </Select>
                                            </FormControl>
                                            <Button onClick={editTheme}>
                                                {t("monitoring.editor.dashboard.editTheme")}
                                            </Button>
                                            <Button
                                                color="error"
                                                onClick={() => setResetting(true)}
                                            >
                                                {t("monitoring.editor.dashboard.resetToPreset")}
                                            </Button>
                                        </Box>
                                        <Typography variant="subtitle2" component="h3">
                                            {t("monitoring.editor.dashboard.widgets")}
                                        </Typography>
                                        {layout.length === 0 ? (
                                            <Typography variant="body2" color="text.secondary">
                                                {t("monitoring.editor.dashboard.empty")}
                                            </Typography>
                                        ) : null}
                                        <Box
                                            component="ol"
                                            sx={{
                                                listStyle: "none",
                                                p: 0,
                                                m: 0,
                                                display: "flex",
                                                flexDirection: "column",
                                                gap: 1,
                                            }}
                                        >
                                            {layout.map((item, index) => {
                                                const title = titleOf(item.widget)
                                                return (
                                                    <Paper
                                                        key={`${item.widget}-${index}`}
                                                        component="li"
                                                        variant="outlined"
                                                        aria-label={title}
                                                        draggable={!readOnly}
                                                        onDragStart={(event: React.DragEvent) => {
                                                            event.dataTransfer.effectAllowed =
                                                                "move"
                                                            dragged.current = index
                                                            setDragging(index)
                                                        }}
                                                        onDragOver={(event: React.DragEvent) => {
                                                            event.preventDefault()
                                                            setOver(index)
                                                        }}
                                                        onDrop={(event: React.DragEvent) => {
                                                            event.preventDefault()
                                                            drop(index)
                                                        }}
                                                        onDragEnd={() => {
                                                            dragged.current = null
                                                            setDragging(null)
                                                            setOver(null)
                                                        }}
                                                        sx={{
                                                            p: 1,
                                                            display: "flex",
                                                            gap: 1,
                                                            alignItems: "center",
                                                            flexWrap: "wrap",
                                                            opacity: dragging === index ? 0.5 : 1,
                                                            borderStyle:
                                                                over === index && dragging !== null
                                                                    ? "dashed"
                                                                    : "solid",
                                                            borderColor:
                                                                over === index && dragging !== null
                                                                    ? "primary.main"
                                                                    : undefined,
                                                        }}
                                                    >
                                                        <Tooltip
                                                            title={t(
                                                                "monitoring.editor.dashboard.dragHandle",
                                                                {title}
                                                            )}
                                                        >
                                                            <DragIndicatorIcon
                                                                titleAccess={t(
                                                                    "monitoring.editor.dashboard.dragHandle",
                                                                    {title}
                                                                )}
                                                                sx={{
                                                                    cursor: readOnly
                                                                        ? "default"
                                                                        : "move",
                                                                    color: "text.secondary",
                                                                }}
                                                            />
                                                        </Tooltip>
                                                        <Box sx={{flex: "1 1 160px", minWidth: 0}}>
                                                            <Typography variant="body2" noWrap>
                                                                {title}
                                                            </Typography>
                                                            <Typography
                                                                variant="caption"
                                                                color="text.secondary"
                                                                noWrap
                                                                component="p"
                                                            >
                                                                {item.widget}
                                                            </Typography>
                                                        </Box>
                                                        <FormControl
                                                            size="small"
                                                            sx={{width: 120}}
                                                            disabled={readOnly}
                                                        >
                                                            <InputLabel
                                                                id={`dashboard-width-${index}`}
                                                            >
                                                                {t(
                                                                    "monitoring.editor.dashboard.width"
                                                                )}
                                                            </InputLabel>
                                                            <Select
                                                                labelId={`dashboard-width-${index}`}
                                                                label={t(
                                                                    "monitoring.editor.dashboard.width"
                                                                )}
                                                                value={
                                                                    WIDTHS.includes(item.width)
                                                                        ? item.width
                                                                        : ""
                                                                }
                                                                onChange={(event) =>
                                                                    patch((text) =>
                                                                        setIn(
                                                                            text,
                                                                            [
                                                                                "layout",
                                                                                index,
                                                                                "width",
                                                                            ],
                                                                            Number(
                                                                                event.target.value
                                                                            )
                                                                        )
                                                                    )
                                                                }
                                                            >
                                                                {WIDTHS.map((width) => (
                                                                    <MenuItem
                                                                        key={width}
                                                                        value={width}
                                                                    >
                                                                        {t(
                                                                            "monitoring.editor.dashboard.widthValue",
                                                                            {n: width}
                                                                        )}
                                                                    </MenuItem>
                                                                ))}
                                                            </Select>
                                                        </FormControl>
                                                        <IconButton
                                                            aria-label={`${t("monitoring.editor.dashboard.moveUp")}: ${title}`}
                                                            disabled={readOnly || index === 0}
                                                            onClick={() => move(index, index - 1)}
                                                        >
                                                            <ArrowUpwardIcon fontSize="small" />
                                                        </IconButton>
                                                        <IconButton
                                                            aria-label={`${t("monitoring.editor.dashboard.moveDown")}: ${title}`}
                                                            disabled={
                                                                readOnly ||
                                                                index === layout.length - 1
                                                            }
                                                            onClick={() => move(index, index + 1)}
                                                        >
                                                            <ArrowDownwardIcon fontSize="small" />
                                                        </IconButton>
                                                        <IconButton
                                                            aria-label={t(
                                                                "monitoring.editor.dashboard.actions",
                                                                {title}
                                                            )}
                                                            aria-haspopup="menu"
                                                            onClick={(event) =>
                                                                setMenu({
                                                                    anchor: event.currentTarget,
                                                                    index,
                                                                })
                                                            }
                                                        >
                                                            <MoreVertIcon fontSize="small" />
                                                        </IconButton>
                                                    </Paper>
                                                )
                                            })}
                                        </Box>
                                        <Box>
                                            <Button
                                                startIcon={<AddIcon />}
                                                disabled={readOnly}
                                                onClick={() => setAdding(true)}
                                            >
                                                {t("monitoring.editor.dashboard.addWidget")}
                                            </Button>
                                        </Box>
                                    </Box>
                                ) : null}
                            </div>
                            <div
                                role="tabpanel"
                                id="dashboard-panel-yaml"
                                aria-labelledby="dashboard-tab-yaml"
                                hidden={tab !== EDashboardTab.YAML}
                            >
                                <MonitoringYamlEditor
                                    ref={editor}
                                    label={t("monitoring.editor.yaml.label")}
                                    value={draft.text}
                                    onChange={(text) => controller.setText(text)}
                                    diagnostics={draft.diagnostics}
                                    minHeight={360}
                                />
                            </div>
                        </Box>
                        <MonitoringDiagnosticsList
                            diagnostics={draft.diagnostics}
                            localUnavailable={draft.localUnavailable}
                            onSelect={reveal}
                        />
                    </Box>
                ) : null}
                {stored.message ? (
                    <Alert
                        severity={stored.message.tone}
                        sx={{mt: 2}}
                        onClose={() => setMessage(null)}
                    >
                        {stored.message.text}
                    </Alert>
                ) : null}
            </DialogContent>
            <Box sx={{px: 3, py: 1.5}}>
                <MonitoringPreviewFooter
                    hasPreview={false}
                    errors={counts.errors}
                    warnings={counts.warnings}
                    revision={stored.revision.revision}
                    savedAt={stored.revision.createdAt}
                    savedBy={authorName(stored.revision.author)}
                    dirty={draft.dirty}
                    busy={stored.busy}
                    saveLabel={t("monitoring.editor.dashboard.save")}
                    saveBlocked={!ready || !draft.dirty}
                    onCancel={cancel}
                    onValidate={ready ? stored.validate : undefined}
                    onSave={() => void stored.save()}
                />
            </Box>
            <Menu open={Boolean(menu)} anchorEl={menu?.anchor} onClose={() => setMenu(null)}>
                {onConfigureWidget ? (
                    <MenuItem
                        onClick={() => {
                            if (menuItem) onConfigureWidget(menuItem.widget)
                            setMenu(null)
                        }}
                    >
                        {t("monitoring.editor.dashboard.configure")}
                    </MenuItem>
                ) : null}
                <MenuItem
                    disabled={readOnly}
                    onClick={() => {
                        if (menu) void duplicate(menu.index)
                        setMenu(null)
                    }}
                >
                    {t("monitoring.editor.dashboard.duplicate")}
                </MenuItem>
                <MenuItem
                    disabled={readOnly}
                    onClick={() => {
                        if (menu) {
                            const index = menu.index
                            patch((text) => deleteIn(text, ["layout", index]))
                        }
                        setMenu(null)
                    }}
                >
                    {t("monitoring.editor.dashboard.remove")}
                </MenuItem>
            </Menu>
            <MonitoringWidgetCatalogDialog
                open={adding}
                entries={catalog}
                error={catalogError}
                onDashboard={layout.map((item) => item.widget)}
                onAdd={add}
                onClose={() => setAdding(false)}
            />
            {theming ? (
                <MonitoringThemeEditorDialog
                    open
                    api={api}
                    themeKey={theming.key}
                    widgetCount={theming.count}
                    preview={layout[0] ? {dashboardId, widgetId: layout[0].widget} : undefined}
                    localValidate={themeValidate}
                    onClose={() => setTheming(null)}
                />
            ) : null}
            {resetting ? (
                <MonitoringResetToPresetDialog
                    open
                    api={api}
                    onClose={() => setResetting(false)}
                    onReset={() => onReset?.()}
                />
            ) : null}
            <Dialog
                open={confirmDiscard}
                onClose={() => setConfirmDiscard(false)}
                aria-labelledby="monitoring-dashboard-discard-title"
            >
                <DialogTitle id="monitoring-dashboard-discard-title">
                    {t("monitoring.editor.configureWidget.discardTitle")}
                </DialogTitle>
                <DialogContent>
                    <DialogContentText>
                        {t("monitoring.editor.dashboard.discardBody")}
                    </DialogContentText>
                </DialogContent>
                <DialogActions>
                    <Button onClick={() => setConfirmDiscard(false)}>
                        {t("monitoring.editor.configureWidget.keepEditing")}
                    </Button>
                    <Button
                        color="error"
                        onClick={() => {
                            setConfirmDiscard(false)
                            onClose()
                        }}
                    >
                        {t("monitoring.editor.configureWidget.discard")}
                    </Button>
                </DialogActions>
            </Dialog>
            {stored.conflict ? (
                <MonitoringConflictDialog
                    open
                    mine={draft.text}
                    theirs={stored.conflict.theirs}
                    currentRevision={stored.conflict.currentRevision}
                    author={stored.conflict.author}
                    time={stored.conflict.time}
                    onReload={stored.reload}
                    onKeepEditing={stored.keepEditing}
                />
            ) : null}
        </Dialog>
    )
}

export default MonitoringDashboardEditor
