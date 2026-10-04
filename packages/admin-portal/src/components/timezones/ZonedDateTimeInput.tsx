// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo} from "react"
import {InputHelperText, useInput, type Validator} from "react-admin"
import {Alert, Box, Stack, TextField, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {TimeZonePicker} from "./TimeZonePicker"
import {useTimeZoneService, type IAdminTimeZones} from "./timeZoneService"
import {hasOffset, zonedLines} from "./ZonedDateTime"

/** A scheduled time as it is saved: the instant, and the wall time and zone it was entered in. */
export interface IZonedDateTimeValue {
    /** RFC 3339 instant in UTC (`…Z`): what the scheduler runs on. */
    scheduled_date: string
    /** `YYYY-MM-DDTHH:MM`, in `timezone`. */
    local: string
    /** IANA zone. */
    timezone: string
}

/** What a stored time may hold: older rows have only `scheduled_date`. */
export interface IStoredZonedDateTime {
    scheduled_date?: string | null
    local?: string | null
    timezone?: string | null
}

const LOCAL_PATTERN = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/

/** The wall time and zone to edit: as entered, else the instant in `zone`; empty for none. */
export const editableOf = (
    stored: IStoredZonedDateTime | null | undefined,
    zone: string,
    service: Pick<IAdminTimeZones, "instantToZoned">
): {local: string; timezone: string} => {
    if (stored?.local && stored.timezone) {
        return {local: stored.local.slice(0, 16), timezone: stored.timezone}
    }
    if (stored?.scheduled_date && hasOffset(stored.scheduled_date)) {
        return {local: service.instantToZoned(stored.scheduled_date, zone), timezone: zone}
    }
    return {local: "", timezone: zone}
}

/** The saved triple of a wall time in a zone; null while the wall time is incomplete. */
export const zonedValueOf = (
    local: string,
    timezone: string,
    service: Pick<IAdminTimeZones, "zonedToInstant">
): IZonedDateTimeValue | null =>
    LOCAL_PATTERN.test(local) && timezone
        ? {scheduled_date: service.zonedToInstant(local, timezone).instant, local, timezone}
        : null

export interface IZonedDateTimeFieldProps {
    /** The wall time and zone being edited. */
    local: string
    timezone: string
    /** Called with the saved triple, or null while the wall time is empty or incomplete. */
    onChange: (value: IZonedDateTimeValue | null, edited: {local: string; timezone: string}) => void
    label: string
    /** The zones offered (the event's configured list); every zone when absent. */
    zones?: ReadonlyArray<string>
    primary?: string
    /** Names the row's place in the preview ("Dubai PCG"). */
    place?: string
    required?: boolean
    disabled?: boolean
    error?: boolean
    helperText?: React.ReactNode
}

/**
 * A date and time in a chosen zone: the wall time, the timezone picker, and a
 * preview in the row's zone (with its place) and in my time. A time that
 * doesn't exist (DST gap) or happens twice (overlap) is explained.
 */
export const ZonedDateTimeField: React.FC<IZonedDateTimeFieldProps> = ({
    local,
    timezone,
    onChange,
    label,
    zones,
    primary,
    place,
    required,
    disabled,
    error,
    helperText,
}) => {
    const {t} = useTranslation()
    const service = useTimeZoneService()
    const resolved = useMemo(
        () =>
            LOCAL_PATTERN.test(local) && timezone ? service.zonedToInstant(local, timezone) : null,
        [local, timezone, service]
    )
    const instant = resolved ? new Date(resolved.instant) : undefined
    const change = (next: {local: string; timezone: string}) =>
        onChange(zonedValueOf(next.local, next.timezone, service), next)

    // The row's zone (with its place), then my time unless it reads the same.
    const inZone = instant
        ? place
            ? service.formatPlaceTime(instant, timezone, place, service.text)
            : service.formatDateTimeZone(instant, timezone, service.text)
        : null
    const mine = instant ? zonedLines(instant, timezone, service).mine : null
    const dst = resolved ? service.zonedTimeNote(local, timezone, service.text) : undefined
    return (
        <Stack spacing={1} sx={{width: "100%"}}>
            <Stack direction={{xs: "column", sm: "row"}} spacing={2}>
                <TextField
                    type="datetime-local"
                    label={label}
                    value={local}
                    required={required}
                    disabled={disabled}
                    error={error}
                    helperText={helperText}
                    onChange={(event) => change({local: event.target.value, timezone})}
                    slotProps={{inputLabel: {shrink: true}}}
                    sx={{flex: 1}}
                />
                <Box sx={{flex: 1}}>
                    <TimeZonePicker
                        label={t("lifecycle.input.timezone")}
                        value={timezone}
                        zones={zones}
                        primary={primary}
                        at={instant}
                        disabled={disabled}
                        onChange={(zone) => change({local, timezone: zone ?? timezone})}
                    />
                </Box>
            </Stack>
            {inZone ? (
                <Box
                    data-testid="zoned-preview"
                    sx={{bgcolor: "action.hover", borderRadius: 1, px: 1.5, py: 1}}
                >
                    <Typography variant="body2">{inZone}</Typography>
                    {mine ? (
                        <Typography variant="body2" color="text.secondary">
                            {mine}
                        </Typography>
                    ) : null}
                </Box>
            ) : null}
            {dst ? (
                <Alert severity={resolved?.kind === "gap" ? "warning" : "info"}>{dst}</Alert>
            ) : null}
        </Stack>
    )
}

export interface IZonedDateTimeInputProps extends Omit<
    IZonedDateTimeFieldProps,
    "local" | "timezone" | "onChange" | "error"
> {
    /** Where the triple `{scheduled_date, local, timezone}` is saved. */
    source: string
    /** The zone a new value starts in: the row's zone (election, else primary). */
    defaultZone: string
    validate?: Validator | Validator[]
}

/** The react-admin input of a zoned date and time; it saves `{scheduled_date, local, timezone}`. */
export const ZonedDateTimeInput: React.FC<IZonedDateTimeInputProps> = ({
    source,
    defaultZone,
    validate,
    helperText,
    ...props
}) => {
    const service = useTimeZoneService()
    const {field, fieldState} = useInput<IStoredZonedDateTime | null>({source, validate})
    const stored = field.value as IStoredZonedDateTime | null | undefined
    const [edited, setEdited] = React.useState(() => editableOf(stored, defaultZone, service))
    const [touched, setTouched] = React.useState(false)
    // The last value this input saved: a different one came from outside (a reset).
    const emitted = React.useRef<IZonedDateTimeValue | null | undefined>(undefined)
    React.useEffect(() => {
        if (touched && JSON.stringify(stored ?? null) !== JSON.stringify(emitted.current ?? null)) {
            // The form was reset (or set) from outside: show what it holds now.
            setTouched(false)
            return
        }
        if (touched) return
        // Until the time is edited here, it follows the record and the row's zone,
        // which may load after the form.
        const next = editableOf(stored, defaultZone, service)
        setEdited((current) =>
            current.local === next.local && current.timezone === next.timezone ? current : next
        )
    }, [touched, stored, defaultZone, service])
    return (
        <ZonedDateTimeField
            {...props}
            local={edited.local}
            timezone={edited.timezone}
            error={fieldState.invalid}
            // react-admin encodes validation errors; its helper text translates them.
            helperText={
                fieldState.error?.message ? (
                    <InputHelperText error={fieldState.error.message} helperText={helperText} />
                ) : (
                    helperText
                )
            }
            onChange={(value, next) => {
                setTouched(true)
                setEdited(next)
                emitted.current = value
                field.onChange(value)
            }}
        />
    )
}
