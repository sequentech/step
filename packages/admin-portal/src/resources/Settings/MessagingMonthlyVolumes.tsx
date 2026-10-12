// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useMemo, useState} from "react"
import {useQuery} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {
    Box,
    CircularProgress,
    Table,
    TableBody,
    TableCell,
    TableContainer,
    TableFooter,
    TableHead,
    TableRow,
    TextField,
    Typography,
} from "@mui/material"
import {ChannelLabel} from "@sequentech/ui-essentials"
import type {TFunction} from "i18next"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {roleContext} from "@/hooks/useMessagingReadRole"
import {IPermissions} from "@/types/keycloak"
import {
    GET_MESSAGING_MONTHLY_VOLUMES,
    IGetMessagingMonthlyVolumesResult,
    IMonthlyVolume,
} from "@/queries/GetMessagingMonthlyVolumes"
import {useFormatCount} from "@/services/messagingFormat"
import {EMessageChannel} from "@/types/messaging"

/** The current month in UTC, as `YYYY-MM`: months are counted in UTC. */
export const currentUtcMonth = (now: Date = new Date()): string => now.toISOString().slice(0, 7)

const MONTH = /^\d{4}-(0[1-9]|1[0-2])$/

/**
 * A destination country's name in the viewer's language, with its code.
 * Older records hold a calling code instead, shown as it is.
 */
export const countryLabel = (code: string | null | undefined, language: string): string | null => {
    if (!code) return null
    if (!/^[A-Z]{2}$/.test(code)) return code
    try {
        const name = new Intl.DisplayNames([language], {type: "region"}).of(code)
        return name && name !== code ? `${name} (${code})` : code
    } catch {
        return code
    }
}

/** The billing categories the backend records (`BillingCategory`). */
export const BILLING_CATEGORIES = [
    "AUTHENTICATION",
    "UTILITY",
    "SERVICE",
    "TRANSACTIONAL",
    "EMAIL",
] as const

/** A billing category in the viewer's language; one the portal doesn't know shows as recorded. */
export const billingCategoryLabel = (t: TFunction, category: string | null | undefined): string => {
    if (!category) return t("messaging.volumes.notReported")
    return (BILLING_CATEGORIES as readonly string[]).includes(category)
        ? t(`messaging.volumes.category.${category}`)
        : category
}

/**
 * Who an attempt was sent from. An account deleted since leaves no account on
 * its attempts, and the ledger records no sender either: on WhatsApp, Viber
 * and Messenger that can only be a deleted account, while on email and SMS it
 * can also be the deployment's own sender, which the ledger can't tell apart.
 */
export const volumeAccountLabel = (
    t: TFunction,
    row: Pick<IMonthlyVolume, "channel" | "account_id" | "account_name">
): string => {
    if (row.account_id) return row.account_name ?? t("messaging.audience.deletedAccount")
    return row.channel === EMessageChannel.EMAIL || row.channel === EMessageChannel.SMS
        ? t("messaging.volumes.deploymentOrDeleted")
        : t("messaging.audience.deletedAccount")
}

export interface MessagingMonthlyVolumesProps {
    /** The month first shown (`YYYY-MM`); the current UTC month by default. */
    initialMonth?: string
}

/**
 * Settings > Messaging, Monthly volumes: the attempts handed to a provider in a
 * month, by channel, account, destination country and billing category, for
 * reconciling the providers' invoices (`messaging_monthly_volumes`). Each
 * attempt counts in the month it was made, so a retry counts again; the
 * billing category is the one expected when sending unless the provider
 * reported another, and the billable flag is what the provider reported, or
 * "Not reported". No prices are shown.
 */
export const MessagingMonthlyVolumes: React.FC<MessagingMonthlyVolumesProps> = ({initialMonth}) => {
    const {t, i18n} = useTranslation()
    const formatCount = useFormatCount()
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const role = authContext.isAuthorized(true, tenantId, IPermissions.MESSAGING_ACCOUNT_READ)
        ? IPermissions.MESSAGING_ACCOUNT_READ
        : authContext.isAuthorized(true, tenantId, IPermissions.MESSAGING_CONFIG_READ)
          ? IPermissions.MESSAGING_CONFIG_READ
          : null
    const [month, setMonth] = useState(initialMonth ?? currentUtcMonth())
    const validMonth = MONTH.test(month)
    const {data, loading, error} = useQuery<IGetMessagingMonthlyVolumesResult>(
        GET_MESSAGING_MONTHLY_VOLUMES,
        {
            variables: {month},
            skip: !role || !validMonth,
            context: roleContext(role),
        }
    )
    const rows: IMonthlyVolume[] = useMemo(
        () => data?.messaging_monthly_volumes?.rows ?? [],
        [data]
    )
    const total = rows.reduce((sum, row) => sum + row.messages, 0)
    const notReported = t("messaging.volumes.notReported")
    const billable = (value?: boolean | null) =>
        value === true
            ? t("messaging.volumes.billableYes")
            : value === false
              ? t("messaging.volumes.billableNo")
              : notReported

    return (
        <Box
            component="section"
            aria-labelledby="messaging-monthly-volumes"
            sx={{display: "flex", flexDirection: "column", gap: 1, padding: 2}}
        >
            <Typography id="messaging-monthly-volumes" variant="h6">
                {t("messaging.volumes.title")}
            </Typography>
            <Typography variant="body2" color="text.secondary">
                {t("messaging.volumes.intro")}
            </Typography>
            {!role ? (
                <Typography variant="body2">{t("messaging.volumes.noPermission")}</Typography>
            ) : (
                <>
                    <TextField
                        type="month"
                        size="small"
                        label={t("messaging.volumes.month")}
                        value={month}
                        onChange={(event) => setMonth(event.target.value)}
                        error={!validMonth}
                        helperText={t("messaging.volumes.monthHelp")}
                        InputLabelProps={{shrink: true}}
                        sx={{maxWidth: 240}}
                    />
                    {loading ? (
                        <CircularProgress size={24} aria-label={t("messaging.volumes.loading")} />
                    ) : error ? (
                        <Typography color="error" role="status">
                            {t("messaging.volumes.loadError")}
                        </Typography>
                    ) : validMonth && rows.length === 0 ? (
                        <Typography variant="body2">
                            {t("messaging.volumes.empty", {month})}
                        </Typography>
                    ) : rows.length > 0 ? (
                        <TableContainer>
                            <Table size="small" aria-label={t("messaging.volumes.title")}>
                                <TableHead>
                                    <TableRow>
                                        <TableCell>{t("messaging.send.channelColumn")}</TableCell>
                                        <TableCell>
                                            {t("messaging.volumes.column.account")}
                                        </TableCell>
                                        <TableCell>
                                            {t("messaging.volumes.column.country")}
                                        </TableCell>
                                        <TableCell>
                                            {t("messaging.volumes.column.category")}
                                        </TableCell>
                                        <TableCell>
                                            {t("messaging.volumes.column.billable")}
                                        </TableCell>
                                        <TableCell align="right">
                                            {t("messaging.volumes.column.messages")}
                                        </TableCell>
                                    </TableRow>
                                </TableHead>
                                <TableBody>
                                    {rows.map((row, index) => (
                                        <TableRow key={index} data-channel={row.channel}>
                                            <TableCell>
                                                <ChannelLabel
                                                    channel={row.channel}
                                                    label={t(`messaging.channel.${row.channel}`)}
                                                />
                                            </TableCell>
                                            <TableCell>{volumeAccountLabel(t, row)}</TableCell>
                                            <TableCell>
                                                {countryLabel(
                                                    row.destination_country,
                                                    i18n.language
                                                ) ?? "–"}
                                            </TableCell>
                                            <TableCell>
                                                {billingCategoryLabel(t, row.billing_category)}
                                            </TableCell>
                                            <TableCell>{billable(row.billable)}</TableCell>
                                            <TableCell align="right">
                                                {formatCount(row.messages)}
                                            </TableCell>
                                        </TableRow>
                                    ))}
                                </TableBody>
                                <TableFooter>
                                    <TableRow>
                                        <TableCell colSpan={5}>
                                            {t("messaging.volumes.total")}
                                        </TableCell>
                                        <TableCell align="right">{formatCount(total)}</TableCell>
                                    </TableRow>
                                </TableFooter>
                            </Table>
                        </TableContainer>
                    ) : null}
                </>
            )}
        </Box>
    )
}
