// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {useTranslation} from "react-i18next"
import {useEventZonedFormat} from "@/hooks/useZonedFormat"
import {useNotify} from "react-admin"
import {
    Alert,
    Box,
    Chip,
    CircularProgress,
    IconButton,
    Table,
    TableBody,
    TableCell,
    TableContainer,
    TableHead,
    TableRow,
    Typography,
} from "@mui/material"
import EditIcon from "@mui/icons-material/Edit"
import VisibilityIcon from "@mui/icons-material/Visibility"
import {visuallyHidden} from "@mui/utils"
import {SigningAction, SigningRequirement, type ISigningRule} from "@/lib/signing/types"
import {
    actionGroups,
    afterLockdown,
    expiryKey,
    isTrusteeAction,
    lastChange,
    ruleOf,
    sortRolesByName,
} from "./signingSettings"
import type {ISignaturesAccess} from "./signingSettings"
import {useRuleCapacities, useSigningRules} from "./useSigningSettings"
import {RuleDrawer} from "./RuleDrawer"

const DASH = "–"

export interface ISignaturesSubTabProps {
    electionEventId: string
    access: ISignaturesAccess
    /** The event is locked down: its rules belong to the configuration version. */
    lockedDown?: boolean
}

/** The ten protected actions by group, with each action's rule and who can sign it. */
export const ProtectedActionsTab: React.FC<ISignaturesSubTabProps> = ({
    electionEventId,
    access: permitted,
    lockedDown = false,
}) => {
    const access = afterLockdown(permitted, lockedDown)
    const {t, i18n} = useTranslation()
    const zoned = useEventZonedFormat(electionEventId)
    const {rules, loading, error, refetch} = useSigningRules(electionEventId)
    const {
        capacityOf,
        configVersion,
        loaded: capacitiesLoaded,
        error: capacitiesError,
        refetch: refetchCapacities,
    } = useRuleCapacities(electionEventId)
    const [editing, setEditing] = useState<SigningAction | null>(null)
    const notify = useNotify()

    /** After a save: the new rule, and the roles and waiting requests it changed. */
    const reload = async () => {
        try {
            await Promise.all([refetch(), refetchCapacities()])
        } catch {
            notify(t("signing.loadError"), {type: "error"})
        }
    }

    if (error) {
        return <Alert severity="error">{t("signing.loadError")}</Alert>
    }
    // Who can sign and the Posts' signers come with the capacities; the rules alone would
    // show every action without signers.
    if (loading || !rules || !(capacitiesLoaded || capacitiesError)) {
        return <CircularProgress aria-label={t("common.label.loadingData")} />
    }

    const changed = lastChange(rules)
    const changedOn = changed && zoned.format(changed.updated_at)
    const footer = [
        configVersion === null
            ? null
            : configVersion > 0
              ? t("signing.protectedActions.footerVersion", {version: configVersion})
              : // Before the first publication there is no version 0 to name.
                t("signing.protectedActions.footerFirstVersion"),
        changed?.name
            ? t("signing.protectedActions.footerChangedBy", {date: changedOn, name: changed.name})
            : changed
              ? t("signing.protectedActions.footerChanged", {date: changedOn})
              : null,
    ].filter(Boolean)

    const signaturesText = (rule: ISigningRule) => {
        if (rule.requirement === SigningRequirement.NotRequired) {
            return t("signing.protectedActions.off")
        }
        return isTrusteeAction(rule.action)
            ? t("signing.protectedActions.eachTrustee")
            : String(rule.signatures)
    }

    return (
        <Box>
            <Box sx={{display: "flex", alignItems: "center", gap: 2, mb: 2}}>
                <Typography variant="body2" sx={{flex: 1}}>
                    {t("signing.protectedActions.intro")}
                </Typography>
                {!access.rulesWrite && (
                    <Chip size="small" variant="outlined" label={t("signing.readOnly.chip")} />
                )}
            </Box>
            {lockedDown && (
                <Alert severity="info" sx={{mb: 2}}>
                    {t("signing.protectedActions.lockedDown")}
                </Alert>
            )}
            {capacitiesError && (
                <Alert severity="warning" sx={{mb: 2}}>
                    {t("signing.protectedActions.capacityError")}
                </Alert>
            )}
            <TableContainer sx={{border: 1, borderColor: "divider", borderRadius: 1}}>
                <Table size="small" aria-label={t("signing.tab.protectedActions")}>
                    <TableHead>
                        <TableRow>
                            <TableCell>{t("signing.protectedActions.columns.action")}</TableCell>
                            <TableCell>{t("signing.protectedActions.columns.appliesTo")}</TableCell>
                            <TableCell>
                                {t("signing.protectedActions.columns.whoCanSign")}
                            </TableCell>
                            <TableCell align="center">
                                {t("signing.protectedActions.columns.signaturesNeeded")}
                            </TableCell>
                            <TableCell>
                                {t("signing.protectedActions.columns.requestExpires")}
                            </TableCell>
                            <TableCell align="center">
                                {t("signing.protectedActions.columns.waiting")}
                            </TableCell>
                            <TableCell>
                                <Box component="span" sx={visuallyHidden}>
                                    {t("common.label.actions")}
                                </Box>
                            </TableCell>
                        </TableRow>
                    </TableHead>
                    <TableBody>
                        {actionGroups().map(({group, actions}) => (
                            <React.Fragment key={group}>
                                <TableRow sx={{bgcolor: "action.hover"}}>
                                    <TableCell colSpan={7}>
                                        <Typography variant="subtitle2" sx={{fontWeight: 600}}>
                                            {t(`signing.groups.${group}`)}
                                        </Typography>
                                    </TableCell>
                                </TableRow>
                                {actions.map((action) => {
                                    const rule = ruleOf(action, rules)
                                    const off = rule.requirement === SigningRequirement.NotRequired
                                    const trustee = isTrusteeAction(action)
                                    const capacity = capacityOf(action)
                                    const waiting = capacity?.waiting ?? 0
                                    const label = t(`signing.actions.${action}.label`)
                                    // Greyed, yet readable: an off rule is still a setting.
                                    const cell = off ? {color: "text.secondary"} : undefined
                                    return (
                                        <TableRow key={action} data-action={action}>
                                            <TableCell sx={{...cell, fontWeight: off ? 400 : 600}}>
                                                {label}
                                            </TableCell>
                                            <TableCell sx={cell}>
                                                {t(`signing.actions.${action}.appliesTo`)}
                                            </TableCell>
                                            <TableCell sx={cell}>
                                                {off || !capacity?.roles?.length ? (
                                                    DASH
                                                ) : (
                                                    <Box
                                                        sx={{
                                                            display: "flex",
                                                            flexWrap: "wrap",
                                                            gap: 0.5,
                                                        }}
                                                    >
                                                        {sortRolesByName(
                                                            capacity.roles,
                                                            i18n.language
                                                        ).map(({id, name}) => (
                                                            <Chip
                                                                key={id}
                                                                size="small"
                                                                label={name}
                                                            />
                                                        ))}
                                                    </Box>
                                                )}
                                            </TableCell>
                                            <TableCell
                                                align="center"
                                                sx={{...cell, fontWeight: 600}}
                                            >
                                                {signaturesText(rule)}
                                            </TableCell>
                                            <TableCell sx={cell}>
                                                {off || trustee
                                                    ? DASH
                                                    : t(
                                                          `signing.expiry.${expiryKey(
                                                              rule.expires_minutes
                                                          )}`,
                                                          {count: rule.expires_minutes ?? 0}
                                                      )}
                                            </TableCell>
                                            <TableCell align="center">
                                                {waiting > 0 ? (
                                                    <Chip
                                                        size="small"
                                                        variant="outlined"
                                                        sx={{borderColor: "warning.main"}}
                                                        label={waiting}
                                                        aria-label={t(
                                                            "signing.protectedActions.waitingCount",
                                                            {count: waiting}
                                                        )}
                                                    />
                                                ) : (
                                                    DASH
                                                )}
                                            </TableCell>
                                            <TableCell align="right">
                                                <IconButton
                                                    size="small"
                                                    aria-label={t(
                                                        access.rulesWrite
                                                            ? "signing.protectedActions.edit"
                                                            : "signing.protectedActions.view",
                                                        {action: label}
                                                    )}
                                                    onClick={() => setEditing(action)}
                                                    sx={cell}
                                                >
                                                    {access.rulesWrite ? (
                                                        <EditIcon fontSize="small" />
                                                    ) : (
                                                        <VisibilityIcon fontSize="small" />
                                                    )}
                                                </IconButton>
                                            </TableCell>
                                        </TableRow>
                                    )
                                })}
                            </React.Fragment>
                        ))}
                    </TableBody>
                </Table>
            </TableContainer>
            {footer.length > 0 && (
                <Typography variant="body2" color="text.secondary" sx={{mt: 2}}>
                    {footer.join(" ")}
                </Typography>
            )}
            {editing && (
                <RuleDrawer
                    key={editing}
                    electionEventId={electionEventId}
                    rule={ruleOf(editing, rules)}
                    capacity={capacityOf(editing)}
                    access={access}
                    lockedDown={lockedDown}
                    onClose={() => setEditing(null)}
                    onSaved={() => {
                        setEditing(null)
                        void reload()
                    }}
                />
            )}
        </Box>
    )
}
