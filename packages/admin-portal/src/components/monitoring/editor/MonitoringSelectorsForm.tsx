// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useState} from "react"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Box,
    Button,
    FormControl,
    IconButton,
    InputLabel,
    MenuItem,
    Paper,
    Select,
    TextField,
    Tooltip,
    Typography,
} from "@mui/material"
import AddIcon from "@mui/icons-material/Add"
import ArrowUpwardIcon from "@mui/icons-material/ArrowUpward"
import ArrowDownwardIcon from "@mui/icons-material/ArrowDownward"
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline"
import {deleteIn, moveIn, renameKey, setIn} from "@/components/monitoring/lib/yamlPatch"
import {asWidget, freshName} from "./formValues"
import {EFormAccess} from "./yamlDraft"
import type {IMonitoringSelectorDefinition} from "./types"

export interface MonitoringSelectorsFormProps {
    value: unknown
    formAccess: EFormAccess
    onPatch: (change: (text: string) => string) => void
}

const CONTROLS = ["dropdown", "toggle"]
const DEFAULT_CONTROL = "dropdown"
const SELECTOR_PREFIX = "selector"
const OPTION_PREFIX = "option"

/**
 * A name typed into a field that renames a YAML key: the rename happens
 * when the field loses focus, so a half-typed name never collides.
 */
const KeyField: React.FC<{
    label: string
    value: string
    disabled: boolean
    onRename: (next: string) => void
}> = ({label, value, disabled, onRename}) => {
    const [text, setText] = useState(value)
    useEffect(() => setText(value), [value])
    return (
        <TextField
            size="small"
            label={label}
            value={text}
            disabled={disabled}
            onChange={(event) => setText(event.target.value)}
            onBlur={() => {
                const next = text.trim()
                if (next && next !== value) onRename(next)
                else setText(value)
            }}
            sx={{flex: "1 1 140px"}}
        />
    )
}

/** Selectors: the widget header's controls, whose values feed its query. */
export const MonitoringSelectorsForm: React.FC<MonitoringSelectorsFormProps> = ({
    value,
    formAccess,
    onPatch,
}) => {
    const {t} = useTranslation()
    const readOnly = formAccess !== EFormAccess.EDITABLE
    const selectors = asWidget(value).selectors ?? {}
    const names = Object.keys(selectors)

    const addSelector = () => {
        const name = freshName(SELECTOR_PREFIX, names)
        const option = `${OPTION_PREFIX}_1`
        onPatch((text) =>
            setIn(text, ["selectors", name], {
                label: t("monitoring.editor.selectors.newLabel"),
                options: {[option]: t("monitoring.editor.selectors.newOption")},
                default: option,
            })
        )
    }

    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 2}}>
            {readOnly ? (
                <Alert severity="warning">{t("monitoring.editor.yaml.readOnly")}</Alert>
            ) : null}
            <Typography variant="body2" color="text.secondary">
                {t("monitoring.editor.selectors.help")}
            </Typography>
            {names.length === 0 ? (
                <Typography variant="body2">{t("monitoring.editor.selectors.none")}</Typography>
            ) : null}
            {names.map((name, position) => (
                <SelectorCard
                    key={name}
                    name={name}
                    selector={selectors[name] ?? {}}
                    position={position}
                    count={names.length}
                    readOnly={readOnly}
                    onPatch={onPatch}
                />
            ))}
            <Box>
                <Button startIcon={<AddIcon />} onClick={addSelector} disabled={readOnly}>
                    {t("monitoring.editor.selectors.addSelector")}
                </Button>
            </Box>
        </Box>
    )
}

const SelectorCard: React.FC<{
    name: string
    selector: IMonitoringSelectorDefinition
    position: number
    count: number
    readOnly: boolean
    onPatch: (change: (text: string) => string) => void
}> = ({name, selector, position, count, readOnly, onPatch}) => {
    const {t} = useTranslation()
    const path = ["selectors", name]
    const options = selector.options ?? {}
    const optionValues = Object.keys(options)
    const set = (key: Array<string | number>, next: unknown) =>
        onPatch((text) => setIn(text, [...path, ...key], next))

    const removeOption = (option: string) =>
        onPatch((text) => {
            const without = deleteIn(text, [...path, "options", option])
            return selector.default === option
                ? setIn(
                      without,
                      [...path, "default"],
                      optionValues.find((item) => item !== option)
                  )
                : without
        })

    return (
        <Paper
            variant="outlined"
            component="section"
            aria-label={selector.label || name}
            sx={{p: 1.5, display: "flex", flexDirection: "column", gap: 1.5}}
        >
            <Box sx={{display: "flex", gap: 1, flexWrap: "wrap", alignItems: "center"}}>
                <KeyField
                    label={t("monitoring.editor.selectors.name")}
                    value={name}
                    disabled={readOnly}
                    onRename={(next) =>
                        onPatch((text) => renameKey(text, ["selectors"], name, next))
                    }
                />
                <TextField
                    size="small"
                    label={t("monitoring.editor.selectors.label")}
                    value={selector.label ?? ""}
                    disabled={readOnly}
                    onChange={(event) => set(["label"], event.target.value)}
                    sx={{flex: "2 1 180px"}}
                />
                <FormControl size="small" sx={{flex: "1 1 120px"}} disabled={readOnly}>
                    <InputLabel id={`control-${name}`}>
                        {t("monitoring.editor.selectors.control")}
                    </InputLabel>
                    <Select
                        labelId={`control-${name}`}
                        label={t("monitoring.editor.selectors.control")}
                        value={selector.control ?? DEFAULT_CONTROL}
                        onChange={(event) =>
                            set(
                                ["control"],
                                event.target.value === DEFAULT_CONTROL
                                    ? undefined
                                    : event.target.value
                            )
                        }
                    >
                        {CONTROLS.map((control) => (
                            <MenuItem key={control} value={control}>
                                {t(`monitoring.editor.selectors.controls.${control}`)}
                            </MenuItem>
                        ))}
                    </Select>
                </FormControl>
                <Box sx={{display: "flex"}}>
                    <Tooltip title={t("monitoring.editor.selectors.moveUp", {name})}>
                        <span>
                            <IconButton
                                aria-label={t("monitoring.editor.selectors.moveUp", {name})}
                                disabled={readOnly || position === 0}
                                onClick={() =>
                                    onPatch((text) =>
                                        moveIn(text, ["selectors"], position, position - 1)
                                    )
                                }
                            >
                                <ArrowUpwardIcon fontSize="small" />
                            </IconButton>
                        </span>
                    </Tooltip>
                    <Tooltip title={t("monitoring.editor.selectors.moveDown", {name})}>
                        <span>
                            <IconButton
                                aria-label={t("monitoring.editor.selectors.moveDown", {name})}
                                disabled={readOnly || position === count - 1}
                                onClick={() =>
                                    onPatch((text) =>
                                        moveIn(text, ["selectors"], position, position + 1)
                                    )
                                }
                            >
                                <ArrowDownwardIcon fontSize="small" />
                            </IconButton>
                        </span>
                    </Tooltip>
                    <Tooltip title={t("monitoring.editor.selectors.removeSelector", {name})}>
                        <span>
                            <IconButton
                                aria-label={t("monitoring.editor.selectors.removeSelector", {name})}
                                disabled={readOnly}
                                onClick={() => onPatch((text) => deleteIn(text, path))}
                            >
                                <DeleteOutlineIcon fontSize="small" />
                            </IconButton>
                        </span>
                    </Tooltip>
                </Box>
            </Box>
            {selector.when ? (
                <Typography variant="caption" color="text.secondary">
                    {t("monitoring.editor.selectors.shownWhen", {
                        selector: selector.when.selector,
                        values: (selector.when.in ?? []).join(", "),
                    })}
                </Typography>
            ) : null}
            {selector.options_from ? (
                <Typography variant="body2" color="text.secondary">
                    {t("monitoring.editor.selectors.dynamic", {source: selector.options_from})}
                </Typography>
            ) : (
                <>
                    {optionValues.map((option) => (
                        <Box key={option} sx={{display: "flex", gap: 1, flexWrap: "wrap"}}>
                            <KeyField
                                label={t("monitoring.editor.selectors.optionValue")}
                                value={option}
                                disabled={readOnly}
                                onRename={(next) =>
                                    onPatch((text) => {
                                        const renamed = renameKey(
                                            text,
                                            [...path, "options"],
                                            option,
                                            next
                                        )
                                        return renamed !== text && selector.default === option
                                            ? setIn(renamed, [...path, "default"], next)
                                            : renamed
                                    })
                                }
                            />
                            <TextField
                                size="small"
                                label={t("monitoring.editor.selectors.optionLabel")}
                                value={options[option] ?? ""}
                                disabled={readOnly}
                                onChange={(event) => set(["options", option], event.target.value)}
                                sx={{flex: "2 1 180px"}}
                            />
                            <IconButton
                                aria-label={t("monitoring.editor.selectors.removeOption", {option})}
                                disabled={readOnly || optionValues.length <= 1}
                                onClick={() => removeOption(option)}
                            >
                                <DeleteOutlineIcon fontSize="small" />
                            </IconButton>
                        </Box>
                    ))}
                    <Box sx={{display: "flex", gap: 1, flexWrap: "wrap", alignItems: "center"}}>
                        <FormControl size="small" sx={{flex: "1 1 180px"}} disabled={readOnly}>
                            <InputLabel id={`default-${name}`}>
                                {t("monitoring.editor.selectors.default")}
                            </InputLabel>
                            <Select
                                labelId={`default-${name}`}
                                label={t("monitoring.editor.selectors.default")}
                                value={
                                    selector.default && optionValues.includes(selector.default)
                                        ? selector.default
                                        : ""
                                }
                                onChange={(event) => set(["default"], event.target.value)}
                            >
                                {optionValues.map((option) => (
                                    <MenuItem key={option} value={option}>
                                        {options[option] || option}
                                    </MenuItem>
                                ))}
                            </Select>
                        </FormControl>
                        <Button
                            startIcon={<AddIcon />}
                            disabled={readOnly}
                            onClick={() =>
                                set(
                                    ["options", freshName(OPTION_PREFIX, optionValues)],
                                    t("monitoring.editor.selectors.newOption")
                                )
                            }
                        >
                            {t("monitoring.editor.selectors.addOption")}
                        </Button>
                    </Box>
                </>
            )}
        </Paper>
    )
}

export default MonitoringSelectorsForm
