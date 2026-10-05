// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useCallback, useMemo} from "react"
import {Autocomplete, Box, TextField, Typography, type AutocompleteProps} from "@mui/material"
import {useTranslation} from "react-i18next"
import type {ITimeZoneOption} from "@sequentech/ui-core"
import {useTimeZoneService, type IAdminTimeZones} from "./timeZoneService"

/** The picker's text of an option: `timezones.optionPrimary` marks the primary. */
export const optionText = (
    option: ITimeZoneOption,
    primary: string | undefined,
    service: Pick<IAdminTimeZones, "timeZonePrimaryOptionLabel" | "text">
): string =>
    option.zone === primary
        ? service.timeZonePrimaryOptionLabel(option, service.text)
        : option.label

/** The options of `zones` (every zone when absent) at `at`, and the search over them. */
export const useTimeZoneOptions = (zones?: ReadonlyArray<string>, at?: Date) => {
    const service = useTimeZoneService()
    const universe = useMemo(() => (zones ? [...zones] : service.listTimeZones()), [zones, service])
    // By value: a caller's new Date of the same instant keeps the options (and the
    // selected option's identity, which the Autocomplete compares).
    const atTime = at?.getTime()
    const options = useMemo(
        () =>
            service.searchTimeZones(
                "",
                universe,
                service.text,
                atTime === undefined ? undefined : new Date(atTime)
            ),
        [service, universe, atTime]
    )
    const byZone = useMemo(
        () => new Map(options.map((option) => [option.zone, option] as const)),
        [options]
    )
    const optionOf = useCallback(
        (zone: string): ITimeZoneOption =>
            byZone.get(zone) ??
            service.timeZoneOption(
                zone,
                service.text,
                atTime === undefined ? undefined : new Date(atTime)
            ),
        [byZone, service, atTime]
    )
    const filter = (query: string): Array<ITimeZoneOption> =>
        query.trim() ? service.searchTimeZones(query, universe, service.text, at) : options
    return {service, options, optionOf, filter}
}

/** One option: "(GMT+08:00) Manila · primary" over "Philippines · Philippine Standard Time". */
export const TimeZoneOptionRow: React.FC<{
    option: ITimeZoneOption
    primary?: string
    props: React.HTMLAttributes<HTMLLIElement>
}> = ({option, primary, props}) => {
    const service = useTimeZoneService()
    return (
        <Box component="li" {...props} sx={{flexDirection: "column", alignItems: "start"}}>
            <Typography component="span">{optionText(option, primary, service)}</Typography>
            {option.detail ? (
                <Typography component="span" variant="body2" color="text.secondary">
                    {option.detail}
                </Typography>
            ) : null}
        </Box>
    )
}

export interface ITimeZonePickerProps {
    /** The selected IANA zone; null when none is chosen. */
    value: string | null
    onChange: (zone: string | null) => void
    /** The zones offered (the event's configured list); every zone when absent. */
    zones?: ReadonlyArray<string>
    /** Explicit non-zone choices, such as using each log row's election zone. */
    additionalOptions?: ReadonlyArray<ITimeZoneOption>
    /** The event's primary zone, marked in the list. */
    primary?: string
    label: string
    helperText?: React.ReactNode
    error?: boolean
    disabled?: boolean
    required?: boolean
    /** Allows clearing the value (e.g. an election falling back to the primary). */
    clearable?: boolean
    /** Offsets and labels are taken at this instant (e.g. the scheduled time); now when absent. */
    at?: Date
    /** Shown in the field while no zone is chosen. */
    placeholder?: string
    size?: AutocompleteProps<ITimeZoneOption, false, boolean, false>["size"]
}

/**
 * A searchable timezone picker: type a city, country, zone name,
 * abbreviation or offset; arrows, Enter and Escape work as in any combobox.
 */
export const TimeZonePicker: React.FC<ITimeZonePickerProps> = ({
    value,
    onChange,
    zones,
    additionalOptions = [],
    primary,
    label,
    helperText,
    error,
    disabled,
    required,
    clearable = false,
    at,
    placeholder,
    size,
}) => {
    const {t} = useTranslation()
    const {service, options, optionOf, filter} = useTimeZoneOptions(zones, at)
    const selected = value
        ? (additionalOptions.find((option) => option.zone === value) ?? optionOf(value))
        : null
    return (
        <Autocomplete<ITimeZoneOption, false, boolean, false>
            options={[...additionalOptions, ...options]}
            value={selected}
            disabled={disabled}
            disableClearable={!clearable}
            size={size}
            autoHighlight
            onChange={(_event, option) => onChange(option?.zone ?? null)}
            isOptionEqualToValue={(option, current) => option.zone === current.zone}
            getOptionLabel={(option) => optionText(option, primary, service)}
            // Autocomplete passes "" while the field shows the selected option.
            filterOptions={(_options, {inputValue}) => [
                ...additionalOptions.filter((option) =>
                    option.label.toLocaleLowerCase().includes(inputValue.toLocaleLowerCase())
                ),
                ...filter(inputValue),
            ]}
            noOptionsText={t("lifecycle.picker.noMatch")}
            renderOption={({key, ...props}, option) => (
                <TimeZoneOptionRow key={key} option={option} primary={primary} props={props} />
            )}
            renderInput={(params) => (
                <TextField
                    {...params}
                    label={label}
                    required={required}
                    error={error}
                    helperText={helperText}
                    placeholder={placeholder}
                    // A placeholder names what an empty value means, so it stays visible.
                    InputLabelProps={
                        placeholder
                            ? {...params.InputLabelProps, shrink: true}
                            : params.InputLabelProps
                    }
                />
            )}
        />
    )
}
