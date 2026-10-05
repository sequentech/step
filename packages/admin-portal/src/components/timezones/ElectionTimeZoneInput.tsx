// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useInput} from "react-admin"
import {Box} from "@mui/material"
import {useTranslation} from "react-i18next"
import {TimeZonePicker} from "./TimeZonePicker"
import {useTimeZoneService} from "./timeZoneService"
import type {ITimeZoneContext} from "./useTimeZoneContext"

/**
 * Election > Data > Language, Date and Time: the election's timezone, one of
 * the event's configured zones. Empty uses the event primary, which the field
 * names; every area under the election uses this zone.
 */
export const ElectionTimeZoneInput: React.FC<{
    context: Pick<ITimeZoneContext, "configured" | "primary">
    disabled?: boolean
    source?: string
}> = ({context, disabled, source = "presentation.timezone"}) => {
    const {t} = useTranslation()
    const service = useTimeZoneService()
    // A zone the event no longer configures can't be saved.
    const configuredOnly = (value: unknown) =>
        typeof value === "string" && value && !context.configured.includes(value)
            ? t("lifecycle.settings.electionUnconfiguredSave")
            : undefined
    const {field} = useInput<string | null>({source, validate: configuredOnly})
    const value = typeof field.value === "string" && field.value ? field.value : null
    const primary = service.timeZoneOption(context.primary, service.text).label
    // A zone the event no longer configures is shown, and the election uses the primary.
    const unconfigured = value !== null && !context.configured.includes(value)
    return (
        <Box sx={{width: "100%", minWidth: 0}}>
            <TimeZonePicker
                label={t("lifecycle.settings.electionZone")}
                value={value}
                zones={context.configured}
                primary={context.primary}
                clearable
                disabled={disabled}
                placeholder={t("lifecycle.settings.electionPrimary", {zone: primary})}
                error={unconfigured}
                helperText={
                    unconfigured
                        ? t("lifecycle.settings.electionUnconfigured", {zone: primary})
                        : t("lifecycle.settings.electionZoneHelp")
                }
                onChange={(zone) => field.onChange(zone)}
            />
        </Box>
    )
}
