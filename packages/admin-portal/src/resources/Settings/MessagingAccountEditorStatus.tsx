// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Box,
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
    Typography,
} from "@mui/material"
import {
    EApprovalSource,
    EMessagingProvider,
    IAccountCheck,
    IMessagingAccount,
    INumberChange,
} from "@/types/messaging"
import {accountApproval, checkedForAnotherIdentity} from "@/services/messaging"
import {reasonText} from "@/services/messagingReasons"

/** Meta's quality ratings with a translation; any other value is shown as Meta wrote it. */
const QUALITY_RATINGS = ["GREEN", "YELLOW", "RED", "UNKNOWN"]

/** Meta's template categories with a translation; any other value is shown as Meta wrote it. */
const TEMPLATE_CATEGORIES = ["AUTHENTICATION", "UTILITY", "MARKETING"]

/**
 * WhatsApp number statuses that are a quality warning: Meta still delivers, so
 * the number stays ready (R5-17). Shown with what they mean.
 */
const QUALITY_WARNING_STATUSES = ["FLAGGED", "RATE_LIMITED"]

/** Meta's template statuses with a translation; any other value is shown as Meta wrote it. */
const TEMPLATE_STATUSES = ["APPROVED", "PENDING", "REJECTED", "PAUSED", "DISABLED"]

const formatDate = (iso: string | null | undefined, language: string): string =>
    iso ? new Date(iso).toLocaleString(language) : ""

interface IRowProps {
    label: string
    value: string
    /** Shown below the value: why it isn't what it should be. */
    note?: string
}

const Row: React.FC<IRowProps> = ({label, value, note}) => (
    <Box sx={{display: "flex", gap: 2, alignItems: "baseline"}}>
        <Typography variant="body2" sx={{fontWeight: 500, minWidth: 200}}>
            {label}
        </Typography>
        <Box>
            <Typography variant="body2">{value}</Typography>
            {note ? (
                <Typography variant="caption" color="text.secondary">
                    {note}
                </Typography>
            ) : null}
        </Box>
    </Box>
)

interface INumberChangesProps {
    changes: INumberChange[]
}

/**
 * WhatsApp's number-change signals: the voters to ask to verify their contact
 * again. Nothing was changed automatically.
 */
export const NumberChanges: React.FC<INumberChangesProps> = ({changes}) => {
    const {t, i18n} = useTranslation()
    return (
        <Alert severity="warning">
            <Typography variant="body2" sx={{fontWeight: 500}}>
                {t("messagingAccounts.status.numberChanges.title")}
            </Typography>
            <Typography variant="body2">
                {t("messagingAccounts.status.numberChanges.description")}
            </Typography>
            <Box component="ul" sx={{margin: 0, paddingLeft: 3}}>
                {changes.map((change) => (
                    <Typography
                        key={`${change.reported_at}-${change.masked_old_number}`}
                        component="li"
                        variant="body2"
                    >
                        {t("messagingAccounts.status.numberChanges.item", {
                            old: change.masked_old_number,
                            new:
                                change.masked_new_number ??
                                t("messagingAccounts.status.notReported"),
                            date: formatDate(change.reported_at, i18n.language),
                        })}{" "}
                        {change.voter_ids?.length
                            ? t("messagingAccounts.status.numberChanges.voters", {
                                  voters: change.voter_ids.join(", "),
                              })
                            : t("messagingAccounts.status.numberChanges.noVoters")}
                    </Typography>
                ))}
            </Box>
        </Alert>
    )
}

interface IAccountStatusDetailsProps {
    account: IMessagingAccount
}

/**
 * What the provider's last check reported: token, number quality and limit,
 * the Page's setup, the partner login, the callback address, the provider's
 * approval and its source, the templates with their category and status per
 * language, and WhatsApp's number-change signals. Anything the check didn't
 * read says so; nothing is guessed.
 */
export const AccountStatusDetails: React.FC<IAccountStatusDetailsProps> = ({account}) => {
    const {t, i18n} = useTranslation()
    const check: IAccountCheck | null | undefined = account.status
    const details = check?.details ?? null
    const notReported = t("messagingAccounts.status.notReported")
    const yesNo = (value: boolean | null | undefined) =>
        value === null || value === undefined
            ? notReported
            : value
              ? t("messagingAccounts.status.yes")
              : t("messagingAccounts.status.no")
    const unchecked = checkedForAnotherIdentity(account)
    const isWhatsApp = account.provider === EMessagingProvider.WHATSAPP_CLOUD_API
    const isMessenger = account.provider === EMessagingProvider.MESSENGER_SEND_API
    const isViber = account.provider === EMessagingProvider.VIBER_INFOBIP
    const {approval, source} = accountApproval(account.provider_approval, check)
    const token = details?.token
    const number = details?.whatsapp_number
    const page = details?.messenger
    const templates = details?.templates ?? []
    const qualityText = (value: string | null | undefined) =>
        !value
            ? notReported
            : QUALITY_RATINGS.includes(value)
              ? t(`messagingAccounts.status.quality.${value}`)
              : value
    const templateCategoryText = (value: string | null | undefined) =>
        !value
            ? notReported
            : TEMPLATE_CATEGORIES.includes(value)
              ? t(`messagingAccounts.status.templateCategory.${value}`)
              : value
    const templateStatusText = (value: string) =>
        TEMPLATE_STATUSES.includes(value)
            ? t(`messagingAccounts.status.templateStatus.${value}`)
            : value

    return (
        <Box
            sx={{display: "flex", flexDirection: "column", gap: 1}}
            aria-labelledby="account-status-title"
        >
            <Typography variant="subtitle2" component="h3" id="account-status-title">
                {t("messagingAccounts.status.title")}
            </Typography>
            <Typography variant="body2" color="text.secondary">
                {check?.checked_at
                    ? t("messaging.readiness.lastCheck", {
                          date: formatDate(check.checked_at, i18n.language),
                      })
                    : t("messaging.readiness.neverChecked")}{" "}
                {t("messagingAccounts.status.daily")}
            </Typography>
            {unchecked ? (
                <Alert severity="info">{t("messagingAccounts.status.unchecked")}</Alert>
            ) : null}
            {check?.checked_at && !unchecked ? (
                <Row
                    label={t("messaging.readiness.connected")}
                    value={yesNo(check.connected)}
                    note={check.reason ? reasonText(t, check.reason) : undefined}
                />
            ) : null}
            {(isWhatsApp || isMessenger) && check?.checked_at && !unchecked ? (
                <Row
                    label={t("messagingAccounts.status.token")}
                    value={
                        token
                            ? token.valid
                                ? token.expires_at
                                    ? t("messagingAccounts.status.tokenExpires", {
                                          date: formatDate(token.expires_at, i18n.language),
                                      })
                                    : t("messagingAccounts.status.tokenNoExpiry")
                                : t("messagingAccounts.status.tokenInvalid")
                            : notReported
                    }
                    note={
                        token?.missing_scopes?.length
                            ? t("messagingAccounts.status.missingScopes", {
                                  scopes: token.missing_scopes.join(", "),
                              })
                            : undefined
                    }
                />
            ) : null}
            {(isWhatsApp || isMessenger) && check?.checked_at && !unchecked ? (
                <Row
                    label={t("messagingAccounts.status.appSecret")}
                    value={
                        !details?.app_secret
                            ? notReported
                            : details.app_secret.valid === true
                              ? t("messagingAccounts.status.accepted")
                              : details.app_secret.valid === false
                                ? t("messagingAccounts.status.refused")
                                : t("messagingAccounts.status.notChecked")
                    }
                    note={
                        details?.app_secret?.reason
                            ? reasonText(t, details.app_secret.reason)
                            : undefined
                    }
                />
            ) : null}
            {isWhatsApp && check?.checked_at && !unchecked ? (
                <>
                    <Row
                        label={t("messagingAccounts.status.numberStatus")}
                        value={number?.status ?? notReported}
                        note={
                            [
                                number?.status && QUALITY_WARNING_STATUSES.includes(number.status)
                                    ? t("messagingAccounts.status.numberQualityWarning")
                                    : "",
                                number?.name_status
                                    ? t("messagingAccounts.status.nameStatus", {
                                          status: number.name_status,
                                      })
                                    : "",
                            ]
                                .filter(Boolean)
                                .join(" ") || undefined
                        }
                    />
                    <Row
                        label={t("messagingAccounts.status.qualityRating")}
                        value={qualityText(number?.quality_rating)}
                    />
                    <Row
                        label={t("messagingAccounts.status.messagingLimit")}
                        value={number?.messaging_limit ?? notReported}
                    />
                </>
            ) : null}
            {isMessenger && check?.checked_at && !unchecked ? (
                <>
                    <Row
                        label={t("messagingAccounts.status.pagesMessaging")}
                        value={yesNo(page?.pages_messaging)}
                    />
                    <Row
                        label={t("messagingAccounts.status.subscribed")}
                        value={yesNo(page?.subscribed)}
                    />
                    <Row
                        label={t("messagingAccounts.status.getStarted")}
                        value={yesNo(page?.get_started)}
                        note={
                            page?.get_started === false
                                ? t("messagingAccounts.status.getStartedMissing")
                                : undefined
                        }
                    />
                </>
            ) : null}
            {isViber && check?.checked_at ? (
                <Row
                    label={t("messagingAccounts.status.partnerLogin")}
                    value={
                        details?.partner_login
                            ? details.partner_login.accepted
                                ? t("messagingAccounts.status.accepted")
                                : t("messagingAccounts.status.refused")
                            : notReported
                    }
                    note={
                        details?.partner_login?.reason
                            ? reasonText(t, details.partner_login.reason)
                            : undefined
                    }
                />
            ) : null}
            {details?.callback ? (
                <Row
                    label={t("messagingAccounts.status.callback")}
                    value={
                        details.callback.reason
                            ? reasonText(t, details.callback.reason)
                            : t("messagingAccounts.status.callbackReady")
                    }
                />
            ) : null}
            {isWhatsApp ? (
                <Row
                    label={t("messagingAccounts.field.provider_approval")}
                    value={approval ? t(`messaging.approval.${approval}`) : notReported}
                    note={[
                        source === EApprovalSource.PROVIDER_CHECK
                            ? t("messagingAccounts.status.approvalFromCheck")
                            : t("messagingAccounts.status.approvalEntered"),
                        details?.provider_approval_reason ?? "",
                    ]
                        .filter(Boolean)
                        .join(" · ")}
                />
            ) : null}
            {(isWhatsApp || isMessenger) && check?.checked_at && !unchecked ? (
                <Box>
                    <Typography variant="body2" sx={{fontWeight: 500}}>
                        {t("messagingAccounts.status.templates.title")}
                    </Typography>
                    {details?.templates_reason ? (
                        <Typography variant="caption" color="text.secondary">
                            {reasonText(t, details.templates_reason)}
                        </Typography>
                    ) : templates.length === 0 ? (
                        <Typography variant="caption" color="text.secondary">
                            {t("messagingAccounts.status.templates.none")}
                        </Typography>
                    ) : (
                        <Table
                            size="small"
                            aria-label={t("messagingAccounts.status.templates.title")}
                        >
                            <TableHead>
                                <TableRow>
                                    <TableCell>
                                        {t("messagingAccounts.status.templates.name")}
                                    </TableCell>
                                    <TableCell>
                                        {t("messagingAccounts.status.templates.language")}
                                    </TableCell>
                                    <TableCell>
                                        {t("messagingAccounts.status.templates.category")}
                                    </TableCell>
                                    <TableCell>
                                        {t("messagingAccounts.status.templates.status")}
                                    </TableCell>
                                </TableRow>
                            </TableHead>
                            <TableBody>
                                {templates.map((template) => (
                                    <TableRow key={`${template.name}-${template.language}`}>
                                        <TableCell>{template.name}</TableCell>
                                        <TableCell>{template.language}</TableCell>
                                        <TableCell>
                                            {templateCategoryText(template.category)}
                                        </TableCell>
                                        <TableCell>{templateStatusText(template.status)}</TableCell>
                                    </TableRow>
                                ))}
                            </TableBody>
                        </Table>
                    )}
                </Box>
            ) : null}
            {isViber ? (
                <Typography variant="caption" color="text.secondary">
                    {t("messagingAccounts.status.templates.entered")}
                </Typography>
            ) : null}
            {details?.number_changes?.length ? (
                <NumberChanges changes={details.number_changes} />
            ) : null}
        </Box>
    )
}
