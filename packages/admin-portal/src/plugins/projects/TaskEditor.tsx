// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {
    Box,
    Button,
    FormControlLabel,
    MenuItem,
    Radio,
    RadioGroup,
    Stack,
    TextField,
    Typography,
} from "@mui/material"
import moment from "moment-timezone"
import {useTranslation} from "react-i18next"
import {
    DAY,
    LocalTimeError,
    MAX_NOTES,
    MAX_TITLE,
    Task,
    TaskKind,
    TaskStatus,
    formatOffset,
    fromLocalInput,
    taskErrors,
    toLocalInput,
} from "./model"

type DateField = "start" | "end"
/** Text typed into a date field that is not yet a usable instant. */
interface Pending {
    text: string
    error: LocalTimeError
    candidates?: string[]
}

interface Props {
    draft: Task
    setDraft: (task: Task) => void
    timezone: string
    editable: boolean
    busy: boolean
    existing: boolean
    onSave: () => void
    onDelete: () => void
    onCancel: () => void
    /** Reports whether a date field holds text that is not saved into the draft. */
    onPendingChange: (pending: boolean) => void
}

export const TaskEditor: React.FC<Props> = ({
    draft,
    setDraft,
    timezone,
    editable,
    busy,
    existing,
    onSave,
    onDelete,
    onCancel,
    onPendingChange,
}) => {
    const {t} = useTranslation()
    const [pending, setPending] = useState<Partial<Record<DateField, Pending>>>({})
    const errors = taskErrors(draft)
    const blocked = Object.keys(pending).length > 0

    useEffect(() => onPendingChange(blocked), [blocked, onPendingChange])

    const setDate = (field: DateField, text: string) => {
        const current = draft[field] ?? undefined
        const result = fromLocalInput(text, timezone, current)
        if ("instant" in result) {
            setPending(({[field]: _, ...rest}) => rest)
            setDraft({...draft, [field]: result.instant})
        } else {
            setPending((previous) => ({
                ...previous,
                [field]: {
                    text,
                    error: result.error,
                    candidates: "candidates" in result ? result.candidates : undefined,
                },
            }))
        }
    }

    const choose = (field: DateField, instant: string) => {
        setPending(({[field]: _, ...rest}) => rest)
        setDraft({...draft, [field]: instant})
    }

    const setType = (type: TaskKind) => {
        setPending(({end: _, ...rest}) => rest)
        setDraft({
            ...draft,
            type,
            end:
                type === TaskKind.Milestone
                    ? null
                    : (draft.end ??
                      moment(Date.parse(draft.start) + DAY)
                          .tz(timezone)
                          .format("YYYY-MM-DDTHH:mm:ssZ")),
        })
    }

    const dateField = (field: DateField, label: string) => {
        const value = draft[field]
        const entry = pending[field]
        const message = entry
            ? t(`projects.editor.localTime.${entry.error}`, {timezone})
            : field === "end" && errors.includes("range")
              ? t("projects.validation.range")
              : value
                ? t("projects.editor.offset", {offset: formatOffset(value, timezone)})
                : ""
        return (
            <Box sx={{flex: 1, minWidth: 0}}>
                <TextField
                    label={label}
                    type="datetime-local"
                    value={entry?.text ?? (value ? toLocalInput(value, timezone) : "")}
                    onChange={(event) => setDate(field, event.target.value)}
                    disabled={busy}
                    InputProps={{readOnly: !editable}}
                    InputLabelProps={{shrink: true}}
                    error={Boolean(entry) || (field === "end" && errors.includes("range"))}
                    helperText={message}
                    required
                    fullWidth
                />
                {entry?.candidates && editable && (
                    <RadioGroup
                        aria-label={t("projects.editor.chooseInstant")}
                        onChange={(event) => choose(field, event.target.value)}
                    >
                        {entry.candidates.map((candidate) => (
                            <FormControlLabel
                                key={candidate}
                                value={candidate}
                                control={<Radio size="small" />}
                                label={t("projects.editor.instantChoice", {
                                    offset: formatOffset(candidate, timezone),
                                })}
                            />
                        ))}
                    </RadioGroup>
                )}
            </Box>
        )
    }

    // A new task's empty title is not an error until something is typed.
    const titleError = editable && errors.includes("title") && (existing || draft.title !== "")
    const shownErrors = errors.filter((error) => error !== "range" && error !== "offset")
    const canSave = editable && !busy && errors.length === 0 && !blocked

    return (
        <Box
            component="form"
            noValidate
            onSubmit={(event) => {
                event.preventDefault()
                if (canSave) onSave()
            }}
        >
            <Box
                sx={{
                    display: "grid",
                    gridTemplateColumns: {
                        xs: "minmax(0, 1fr)",
                        md: "minmax(0, 1fr) minmax(0, 1fr)",
                    },
                    columnGap: 4,
                    rowGap: 2.5,
                }}
            >
                <Stack spacing={2.5}>
                    <TextField
                        label={t("projects.editor.title")}
                        value={draft.title}
                        onChange={(event) => setDraft({...draft, title: event.target.value})}
                        disabled={busy}
                        InputProps={{readOnly: !editable}}
                        inputProps={{maxLength: MAX_TITLE}}
                        error={titleError}
                        helperText={titleError ? t("projects.validation.title") : ""}
                        required
                        fullWidth
                    />
                    <TextField
                        select
                        label={t("projects.editor.type")}
                        value={draft.type}
                        onChange={(event) => setType(event.target.value as TaskKind)}
                        disabled={busy || !editable}
                        fullWidth
                    >
                        {Object.values(TaskKind).map((value) => (
                            <MenuItem key={value} value={value}>
                                {t(`projects.type.${value}`)}
                            </MenuItem>
                        ))}
                    </TextField>
                    <Stack direction={{xs: "column", sm: "row"}} spacing={2}>
                        {dateField(
                            "start",
                            t(
                                draft.type === TaskKind.Milestone
                                    ? "projects.editor.at"
                                    : "projects.editor.start"
                            )
                        )}
                        {draft.type === TaskKind.Task && dateField("end", t("projects.editor.end"))}
                    </Stack>
                    <Typography variant="caption" color="text.secondary">
                        {t("projects.editor.timezoneNote", {timezone})}
                    </Typography>
                </Stack>
                <Stack spacing={2.5}>
                    <TextField
                        select
                        label={t("projects.editor.status")}
                        value={draft.status}
                        onChange={(event) =>
                            setDraft({...draft, status: event.target.value as TaskStatus})
                        }
                        disabled={busy || !editable}
                        fullWidth
                    >
                        {Object.values(TaskStatus).map((value) => (
                            <MenuItem key={value} value={value}>
                                {t(`projects.status.${value}`)}
                            </MenuItem>
                        ))}
                    </TextField>
                    <TextField
                        label={t("projects.editor.notes")}
                        value={draft.notes}
                        onChange={(event) => setDraft({...draft, notes: event.target.value})}
                        disabled={busy}
                        InputProps={{readOnly: !editable}}
                        inputProps={{maxLength: MAX_NOTES}}
                        multiline
                        minRows={3}
                        fullWidth
                    />
                    <TextField
                        label={t("projects.editor.order")}
                        type="number"
                        value={draft.order}
                        onChange={(event) =>
                            setDraft({...draft, order: Number(event.target.value)})
                        }
                        disabled={busy}
                        InputProps={{readOnly: !editable}}
                        inputProps={{min: 0, max: 1000000, step: 1}}
                        error={editable && errors.includes("order")}
                        helperText={t("projects.editor.orderHelp")}
                        fullWidth
                    />
                </Stack>
            </Box>
            {editable && (
                <Stack
                    direction="row"
                    spacing={1}
                    alignItems="center"
                    flexWrap="wrap"
                    useFlexGap
                    sx={{mt: 3, pt: 2, borderTop: 1, borderColor: "divider"}}
                >
                    <Typography
                        variant="body2"
                        color="text.secondary"
                        role="status"
                        sx={{mr: "auto"}}
                    >
                        {busy
                            ? t("projects.editor.saving")
                            : errors.length || blocked
                              ? t("projects.editor.toFix", {
                                    count: errors.length + Object.keys(pending).length,
                                })
                              : ""}
                    </Typography>
                    {shownErrors.length > 0 && (
                        <Typography variant="body2" color="error" sx={{width: "100%"}}>
                            {shownErrors
                                .map((error) => t(`projects.validation.${error}`))
                                .join(" ")}
                        </Typography>
                    )}
                    {existing && (
                        <Button color="error" disabled={busy} onClick={onDelete}>
                            {t("projects.editor.delete")}
                        </Button>
                    )}
                    <Button variant="outlined" disabled={busy} onClick={onCancel}>
                        {t("projects.cancel")}
                    </Button>
                    <Button variant="contained" type="submit" disabled={!canSave}>
                        {t("projects.editor.save")}
                    </Button>
                </Stack>
            )}
        </Box>
    )
}
