// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo, useState} from "react"
import {useInput} from "react-admin"
import {
    Autocomplete,
    Box,
    Chip,
    FormControl,
    FormControlLabel,
    FormHelperText,
    FormLabel,
    Radio,
    RadioGroup,
    Stack,
    TextField,
    Typography,
} from "@mui/material"
import {useTranslation} from "react-i18next"
import {
    DEFAULT_TIME_ZONE,
    ELogTimeZonePolicy,
    type IElectionEventTimeZones,
    type ITimeZoneOption,
} from "@sequentech/ui-core"
import {TimeZoneOptionRow, TimeZonePicker, optionText, useTimeZoneOptions} from "./TimeZonePicker"

/** Chips shown before the rest collapse into "+N" while the field isn't focused. */
const VISIBLE_CHIPS = 8

/** An election as the zone-in-use check reads it. */
export interface IElectionZone {
    id: string
    name: string
    /** The election's own timezone; empty uses the primary. */
    zone?: string | null
}

/** The event's zones as saved, with the migration default when none are configured. */
export const eventTimeZonesOf = (
    value: Partial<IElectionEventTimeZones> | null | undefined
): IElectionEventTimeZones => {
    const primary = value?.primary || DEFAULT_TIME_ZONE
    const configured = value?.configured?.length ? value.configured : [primary]
    return {
        configured: configured.includes(primary) ? configured : [primary, ...configured],
        primary,
        logs: value?.logs ?? ELogTimeZonePolicy.ELECTION,
    }
}

/** Why a configured zone can't be removed: the primary, or elections that use it. */
export const removalProblem = (
    zone: string,
    timezones: IElectionEventTimeZones,
    elections: ReadonlyArray<IElectionZone>
): {key: "primary"} | {key: "inUse"; names: Array<string>} | null => {
    if (zone === timezones.primary) return {key: "primary"}
    const names = elections.filter((election) => election.zone === zone).map(({name}) => name)
    return names.length ? {key: "inUse", names} : null
}

/**
 * Event > Data > Language, Date and Time: the configured timezones (searchable,
 * the primary marked), the primary timezone and the zone of Logs. A zone that is
 * the primary or used by an election can't be removed; at least one stays.
 */
export const EventTimeZoneSettings: React.FC<{
    elections: ReadonlyArray<IElectionZone>
    disabled?: boolean
    /** The form path of the event's `presentation.timezones`. */
    source?: string
}> = ({elections, disabled, source = "presentation.timezones"}) => {
    const {t, i18n} = useTranslation()
    const {field} = useInput<IElectionEventTimeZones | null>({source})
    const timezones = eventTimeZonesOf(field.value)
    const {service, options, optionOf, filter} = useTimeZoneOptions()
    const [problem, setProblem] = useState<string | null>(null)
    // Kept by value: the Autocomplete resets its search when its value's identity changes.
    const configuredKey = timezones.configured.join("|")
    const selected = useMemo(
        () => (configuredKey ? configuredKey.split("|") : []).map(optionOf),
        [configuredKey, optionOf]
    )
    const save = (next: Partial<IElectionEventTimeZones>) => field.onChange({...timezones, ...next})

    const names = (list: Array<string>) =>
        new Intl.ListFormat(i18n.language, {type: "conjunction"}).format(list)

    const onConfiguredChange = (next: ReadonlyArray<ITimeZoneOption>) => {
        const zones = next.map(({zone}) => zone)
        const removed = timezones.configured.filter((zone) => !zones.includes(zone))
        for (const zone of removed) {
            const refused = removalProblem(zone, timezones, elections)
            if (refused) {
                setProblem(
                    refused.key === "primary"
                        ? t("lifecycle.settings.primaryInUse", {zone: optionOf(zone).label})
                        : t("lifecycle.settings.inUse", {
                              zone: optionOf(zone).label,
                              names: names(refused.names),
                          })
                )
                return
            }
        }
        setProblem(null)
        save({configured: zones})
    }

    const primaryAbbr = service.zoneLabel(timezones.primary, service.text)

    return (
        <Stack spacing={3} sx={{width: "100%"}} data-testid="event-time-zones">
            <Typography component="h3" variant="body1" sx={{fontWeight: "bold"}}>
                {t("lifecycle.settings.dateAndTime")}
            </Typography>
            <Autocomplete<ITimeZoneOption, true, true, false>
                multiple
                disableClearable
                disabled={disabled}
                options={options}
                value={selected}
                limitTags={VISIBLE_CHIPS}
                getLimitTagsText={(more) => t("lifecycle.settings.moreZones", {count: more})}
                isOptionEqualToValue={(option, value) => option.zone === value.zone}
                getOptionLabel={(option) => option.label}
                filterOptions={(_options, {inputValue}) =>
                    filter(inputValue).filter(
                        (option) => !timezones.configured.includes(option.zone)
                    )
                }
                onChange={(_event, next) => onConfiguredChange(next)}
                noOptionsText={t("lifecycle.picker.noMatch")}
                renderOption={({key, ...props}, option) => (
                    <TimeZoneOptionRow key={key} option={option} props={props} />
                )}
                renderValue={(value, getItemProps) =>
                    value.map((option, index) => {
                        const {key, ...itemProps} = getItemProps({index})
                        const primary = option.zone === timezones.primary
                        return (
                            <Chip
                                key={key}
                                {...itemProps}
                                size="small"
                                color={primary ? "primary" : "default"}
                                label={optionText(option, timezones.primary, service)}
                            />
                        )
                    })
                }
                renderInput={(params) => (
                    <TextField
                        {...params}
                        label={t("lifecycle.settings.configured")}
                        error={!!problem}
                        helperText={
                            problem ??
                            t("lifecycle.settings.configuredHelp", {
                                count: timezones.configured.length,
                            })
                        }
                    />
                )}
            />
            <Stack direction={{xs: "column", md: "row"}} spacing={3}>
                <Box sx={{flex: 1}}>
                    <TimeZonePicker
                        label={t("lifecycle.settings.primary")}
                        value={timezones.primary}
                        zones={timezones.configured}
                        disabled={disabled}
                        helperText={t("lifecycle.settings.primaryHelp")}
                        onChange={(zone) => zone && save({primary: zone})}
                    />
                </Box>
                <FormControl sx={{flex: 1}} disabled={disabled}>
                    <FormLabel id="event-log-time-zone">{t("lifecycle.settings.logs")}</FormLabel>
                    <RadioGroup
                        aria-labelledby="event-log-time-zone"
                        value={timezones.logs}
                        onChange={(event) => save({logs: event.target.value as ELogTimeZonePolicy})}
                    >
                        <FormControlLabel
                            value={ELogTimeZonePolicy.PRIMARY}
                            control={<Radio />}
                            label={t("lifecycle.settings.logsPrimary", {abbr: primaryAbbr})}
                        />
                        <FormControlLabel
                            value={ELogTimeZonePolicy.ELECTION}
                            control={<Radio />}
                            label={t("lifecycle.settings.logsElection")}
                        />
                    </RadioGroup>
                    <FormHelperText>{t("lifecycle.settings.logsHelp")}</FormHelperText>
                </FormControl>
            </Stack>
        </Stack>
    )
}
