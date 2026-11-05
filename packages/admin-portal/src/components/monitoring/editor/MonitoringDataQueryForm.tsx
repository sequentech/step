// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Box,
    Checkbox,
    FormControl,
    FormControlLabel,
    FormGroup,
    FormHelperText,
    FormLabel,
    InputLabel,
    MenuItem,
    Select,
    TextField,
    Typography,
} from "@mui/material"
import {setIn} from "@/components/monitoring/lib/yamlPatch"
import {
    SCOPE_SELECTORS,
    asWidget,
    followedSelectors,
    humanize,
    selectorRef,
    stringList,
    toggleFollowed,
} from "./formValues"
import {EFormAccess} from "./yamlDraft"
import type {IMonitoringSourceInfo, IMonitoringTable} from "./types"
import {MonitoringQueryResult} from "./MonitoringQueryResult"

export interface MonitoringDataQueryFormProps {
    /** The widget as its YAML reads now. */
    value: unknown
    formAccess: EFormAccess
    sources: Record<string, IMonitoringSourceInfo>
    /** Applies a change to the YAML text. */
    onPatch: (change: (text: string) => string) => void
    /** The query result of the latest preview. */
    table?: IMonitoringTable | null
}

const NONE = ""
const TEMPLATE_ORDER = ""
const SORT_KEYS = ["label", "value", "ratio"]
const SORT_ORDERS = ["asc", "desc"]
const DEFAULT_SORT_ORDER = "desc"

/** Data & query: the widget's source, its query template and parameters, and a look at the result. */
export const MonitoringDataQueryForm: React.FC<MonitoringDataQueryFormProps> = ({
    value,
    formAccess,
    sources,
    onPatch,
    table,
}) => {
    const {t} = useTranslation()
    const widget = asWidget(value)
    const readOnly = formAccess !== EFormAccess.EDITABLE
    const source = widget.source ?? ""
    const info = sources[source]
    const query = widget.query ?? {}
    const several = !widget.query && widget.queries && Object.keys(widget.queries).length > 0
    const set = (path: Array<string | number>, next: unknown) =>
        onPatch((text) => setIn(text, path, next))

    const sourceIds = Array.from(new Set([...Object.keys(sources), ...(source ? [source] : [])]))
    const templates = Array.from(
        new Set([...(info?.templates ?? []), ...(query.template ? [query.template] : [])])
    )
    const measures = info?.measures ?? []
    const dimensions = info?.dimensions ?? []
    const sourceLabel = (id: string) =>
        t(`monitoring.editor.sources.${id}`, {defaultValue: humanize(id)})
    const fromSelector = (name: string) => t("monitoring.editor.dataQuery.fromSelector", {name})

    const measuresRef = selectorRef(query.measures)
    const ratioRef = selectorRef(query.ratio)
    const groupRef = selectorRef(query.group_by)
    const ratio = Array.isArray(query.ratio) ? stringList(query.ratio) : []
    const sort =
        query.sort && typeof query.sort === "object" && !selectorRef(query.sort)
            ? (query.sort as {by?: string; order?: string})
            : undefined
    const limit = typeof query.limit === "number" ? query.limit : undefined

    const setRatio = (position: 0 | 1, measure: string) => {
        const next = [ratio[0] ?? NONE, ratio[1] ?? NONE]
        next[position] = measure
        set(["query", "ratio"], next[0] || next[1] ? next : undefined)
    }

    const helpKey = info ? `monitoring.editor.dataQuery.sourceHelp.${info.counting_unit}` : ""
    const help = info
        ? t(helpKey, {defaultValue: t("monitoring.editor.dataQuery.sourceHelp.default")})
        : t("monitoring.editor.dataQuery.sourceHelp.default")

    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 2}}>
            {readOnly ? (
                <Alert severity="warning">{t("monitoring.editor.yaml.readOnly")}</Alert>
            ) : null}
            <TextField
                label={t("monitoring.editor.dataQuery.title")}
                value={widget.title ?? ""}
                disabled={readOnly}
                onChange={(event) => set(["title"], event.target.value)}
                size="small"
                fullWidth
            />
            <FormControl size="small" fullWidth disabled={readOnly}>
                <InputLabel id="monitoring-source-label">
                    {t("monitoring.editor.dataQuery.source")}
                </InputLabel>
                <Select
                    labelId="monitoring-source-label"
                    label={t("monitoring.editor.dataQuery.source")}
                    value={source}
                    onChange={(event) => set(["source"], event.target.value)}
                >
                    {sourceIds.map((id) => (
                        <MenuItem key={id} value={id}>
                            {sourceLabel(id)}
                        </MenuItem>
                    ))}
                </Select>
                <FormHelperText>{help}</FormHelperText>
            </FormControl>
            {info?.producer === "NOT_CONNECTED" ? (
                <Alert severity="info">
                    {t("monitoring.editor.dataQuery.notConnected", {reason: info.reason ?? ""})}
                </Alert>
            ) : null}
            {several ? (
                <Alert severity="info">{t("monitoring.editor.dataQuery.manyQueries")}</Alert>
            ) : (
                <>
                    <FormControl size="small" fullWidth disabled={readOnly}>
                        <InputLabel id="monitoring-template-label">
                            {t("monitoring.editor.dataQuery.template")}
                        </InputLabel>
                        <Select
                            labelId="monitoring-template-label"
                            label={t("monitoring.editor.dataQuery.template")}
                            value={query.template ?? ""}
                            onChange={(event) => set(["query", "template"], event.target.value)}
                        >
                            {templates.map((template) => (
                                <MenuItem key={template} value={template}>
                                    {t(`monitoring.editor.templates.${template}`, {
                                        defaultValue: humanize(template),
                                    })}
                                </MenuItem>
                            ))}
                        </Select>
                    </FormControl>
                    {measuresRef ? (
                        <TextField
                            size="small"
                            label={t("monitoring.editor.dataQuery.measures")}
                            value={fromSelector(measuresRef)}
                            disabled
                        />
                    ) : (
                        <FormControl size="small" fullWidth disabled={readOnly}>
                            <InputLabel id="monitoring-measures-label">
                                {t("monitoring.editor.dataQuery.measures")}
                            </InputLabel>
                            <Select
                                labelId="monitoring-measures-label"
                                label={t("monitoring.editor.dataQuery.measures")}
                                multiple
                                value={stringList(query.measures)}
                                onChange={(event) => {
                                    const next =
                                        typeof event.target.value === "string"
                                            ? event.target.value.split(",")
                                            : event.target.value
                                    set(["query", "measures"], next.length ? next : undefined)
                                }}
                                renderValue={(selected) => selected.map(humanize).join(", ")}
                            >
                                {measures.map((measure) => (
                                    <MenuItem key={measure} value={measure}>
                                        {humanize(measure)}
                                    </MenuItem>
                                ))}
                            </Select>
                        </FormControl>
                    )}
                    <Box component="fieldset" sx={{border: 0, p: 0, m: 0}}>
                        <Typography component="legend" variant="body2" sx={{mb: 1}}>
                            {t("monitoring.editor.dataQuery.ratio")}
                        </Typography>
                        {ratioRef ? (
                            <TextField
                                size="small"
                                fullWidth
                                label={t("monitoring.editor.dataQuery.ratio")}
                                value={fromSelector(ratioRef)}
                                disabled
                            />
                        ) : (
                            <Box sx={{display: "flex", gap: 1, flexWrap: "wrap"}}>
                                {(["numerator", "denominator"] as const).map((part, position) => (
                                    <FormControl
                                        key={part}
                                        size="small"
                                        sx={{flex: "1 1 160px"}}
                                        disabled={readOnly}
                                    >
                                        <InputLabel id={`monitoring-${part}-label`}>
                                            {t(`monitoring.editor.dataQuery.${part}`)}
                                        </InputLabel>
                                        <Select
                                            labelId={`monitoring-${part}-label`}
                                            label={t(`monitoring.editor.dataQuery.${part}`)}
                                            value={ratio[position] ?? NONE}
                                            onChange={(event) =>
                                                setRatio(position as 0 | 1, event.target.value)
                                            }
                                        >
                                            <MenuItem value={NONE}>
                                                {t("monitoring.editor.dataQuery.none")}
                                            </MenuItem>
                                            {measures.map((measure) => (
                                                <MenuItem key={measure} value={measure}>
                                                    {humanize(measure)}
                                                </MenuItem>
                                            ))}
                                        </Select>
                                    </FormControl>
                                ))}
                            </Box>
                        )}
                    </Box>
                    {groupRef ? (
                        <TextField
                            size="small"
                            label={t("monitoring.editor.dataQuery.groupBy")}
                            value={fromSelector(groupRef)}
                            disabled
                        />
                    ) : (
                        <FormControl size="small" fullWidth disabled={readOnly}>
                            <InputLabel id="monitoring-group-label">
                                {t("monitoring.editor.dataQuery.groupBy")}
                            </InputLabel>
                            <Select
                                labelId="monitoring-group-label"
                                label={t("monitoring.editor.dataQuery.groupBy")}
                                value={typeof query.group_by === "string" ? query.group_by : NONE}
                                onChange={(event) =>
                                    set(["query", "group_by"], event.target.value || undefined)
                                }
                            >
                                <MenuItem value={NONE}>
                                    {t("monitoring.editor.dataQuery.none")}
                                </MenuItem>
                                {dimensions.map((dimension) => (
                                    <MenuItem key={dimension} value={dimension}>
                                        {humanize(dimension)}
                                    </MenuItem>
                                ))}
                            </Select>
                        </FormControl>
                    )}
                    <Box sx={{display: "flex", gap: 1, flexWrap: "wrap"}}>
                        <FormControl size="small" sx={{flex: "2 1 200px"}} disabled={readOnly}>
                            <InputLabel id="monitoring-sort-label">
                                {t("monitoring.editor.dataQuery.sort")}
                            </InputLabel>
                            <Select
                                labelId="monitoring-sort-label"
                                label={t("monitoring.editor.dataQuery.sort")}
                                value={sort?.by ?? TEMPLATE_ORDER}
                                onChange={(event) =>
                                    set(
                                        ["query", "sort"],
                                        event.target.value
                                            ? {
                                                  by: event.target.value,
                                                  order: sort?.order ?? DEFAULT_SORT_ORDER,
                                              }
                                            : undefined
                                    )
                                }
                            >
                                <MenuItem value={TEMPLATE_ORDER}>
                                    {t("monitoring.editor.dataQuery.templateOrder")}
                                </MenuItem>
                                {SORT_KEYS.map((key) => (
                                    <MenuItem key={key} value={key}>
                                        {t(`monitoring.editor.dataQuery.sortBy.${key}`)}
                                    </MenuItem>
                                ))}
                            </Select>
                        </FormControl>
                        <FormControl
                            size="small"
                            sx={{flex: "1 1 140px"}}
                            disabled={readOnly || !sort}
                        >
                            <InputLabel id="monitoring-order-label">
                                {t("monitoring.editor.dataQuery.sortOrder")}
                            </InputLabel>
                            <Select
                                labelId="monitoring-order-label"
                                label={t("monitoring.editor.dataQuery.sortOrder")}
                                value={sort?.order ?? DEFAULT_SORT_ORDER}
                                onChange={(event) =>
                                    set(["query", "sort", "order"], event.target.value)
                                }
                            >
                                {SORT_ORDERS.map((order) => (
                                    <MenuItem key={order} value={order}>
                                        {t(`monitoring.editor.dataQuery.order.${order}`)}
                                    </MenuItem>
                                ))}
                            </Select>
                        </FormControl>
                        <TextField
                            size="small"
                            type="number"
                            sx={{flex: "1 1 100px"}}
                            label={t("monitoring.editor.dataQuery.limit")}
                            placeholder={t("monitoring.editor.dataQuery.noLimit")}
                            value={limit ?? ""}
                            disabled={readOnly}
                            slotProps={{htmlInput: {min: 1}}}
                            onChange={(event) => {
                                const next = Number.parseInt(event.target.value, 10)
                                set(
                                    ["query", "limit"],
                                    Number.isFinite(next) && next > 0 ? next : undefined
                                )
                            }}
                        />
                    </Box>
                </>
            )}
            <FormControl component="fieldset" disabled={readOnly}>
                <FormLabel component="legend">{t("monitoring.editor.dataQuery.follow")}</FormLabel>
                <FormGroup row>
                    {SCOPE_SELECTORS.map((selector) => (
                        <FormControlLabel
                            key={selector}
                            label={t(`monitoring.editor.scopeSelector.${selector}`)}
                            control={
                                <Checkbox
                                    checked={followedSelectors(widget).includes(selector)}
                                    onChange={(event) =>
                                        set(
                                            ["follows"],
                                            toggleFollowed(widget, selector, event.target.checked)
                                        )
                                    }
                                />
                            }
                        />
                    ))}
                </FormGroup>
                <FormHelperText>{t("monitoring.editor.dataQuery.followHelp")}</FormHelperText>
            </FormControl>
            <MonitoringQueryResult table={table} />
        </Box>
    )
}

export default MonitoringDataQueryForm
