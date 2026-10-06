// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useCallback, useId, useMemo, useState} from "react"
import {useTranslation} from "react-i18next"
import {useGetList, useNotify} from "react-admin"
import {
    Alert,
    Autocomplete,
    Box,
    Button,
    Checkbox,
    Chip,
    Divider,
    Drawer,
    FormControl,
    FormControlLabel,
    IconButton,
    InputLabel,
    MenuItem,
    Select,
    Switch,
    TextField,
    Typography,
} from "@mui/material"
import CloseIcon from "@mui/icons-material/Close"
import type {IRole} from "@sequentech/ui-core"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {
    RequesterSigning,
    SIGNING_ACTIONS,
    SIGNING_EXPIRY_OPTIONS,
    SigningRequirement,
    SigningScope,
    SigningRuleWarning,
    type ISaveSigningRuleOutput,
    type ISigningRule,
    type ISigningRuleCapacity,
    MAX_SIGNATURES,
} from "@/lib/signing/types"
import {
    SignaturesProblem,
    checkSignatures,
    draftChanged,
    draftOf,
    expiryKey,
    isTrusteeAction,
    parseSignatures,
    rolesChanged,
    sortRolesByName,
    ruleInputOf,
    type ISignaturesAccess,
    type IRuleDraft,
} from "./signingSettings"
import {usePutRule, useWriteErrorMessage} from "./useSigningSettings"
import {RuleChangeNotice, appliesKey, useRuleChangePreview} from "./RuleChangeNotice"

export interface IRuleDrawerProps {
    electionEventId: string
    rule: ISigningRule
    /** What the Posts can give and who can sign; unknown while it loads. */
    capacity: ISigningRuleCapacity | undefined
    access: ISignaturesAccess
    /** A refused save of a locked-down event is the lockdown's. */
    lockedDown?: boolean
    onClose: () => void
    onSaved: () => void
}

/** The value of the expiry select; the select can't hold null. */
const NO_LIMIT = "none"

/** Edits one action's rule, or shows it read-only without signing-rules-write. */
export const RuleDrawer: React.FC<IRuleDrawerProps> = ({
    electionEventId,
    rule,
    capacity,
    access,
    lockedDown = false,
    onClose,
    onSaved,
}) => {
    const {t, i18n} = useTranslation()
    const notify = useNotify()
    const writeErrorMessage = useWriteErrorMessage()
    const [tenantId] = useTenantStore()
    const readOnly = !access.rulesWrite
    // Needs role-read (the options) and role-write; without the capacity the current roles
    // are unknown, so they can't be changed.
    const editRoles = access.rolesWrite && !!capacity
    const trustee = isTrusteeAction(rule.action)
    // The draft holds group ids; the capacity and the role list give their names.
    const initialRoles = useMemo(() => (capacity?.roles ?? []).map(({id}) => id), [capacity])
    const initial = useMemo(() => draftOf(rule, initialRoles), [rule, initialRoles])
    const [draft, setDraft] = useState<IRuleDraft>(initial)
    const [saveError, setSaveError] = useState<string | null>(null)
    const titleId = useId()
    const [putRule, {loading: saving}] = usePutRule()
    const {data: roleRecords} = useGetList<IRole & {id: string}>(
        "role",
        {filter: {tenant_id: tenantId}, pagination: {page: 1, perPage: 1000}},
        {enabled: editRoles}
    )
    const roleNames = useMemo(() => {
        const names = new Map<string, string>()
        for (const {id, name} of capacity?.roles ?? []) names.set(id, name)
        for (const {id, name} of roleRecords ?? []) if (name) names.set(id, name)
        return names
    }, [roleRecords, capacity])
    const sortRoleIds = useCallback(
        (ids: string[]) =>
            sortRolesByName(
                ids.map((id) => ({id, name: roleNames.get(id) ?? id})),
                i18n.language
            ).map(({id}) => id),
        [roleNames, i18n.language]
    )
    const roleOptions = useMemo(
        () => sortRoleIds(Array.from(roleNames.keys())),
        [roleNames, sortRoleIds]
    )
    const roleName = (id: string) => roleNames.get(id) ?? id

    const action = t(`signing.actions.${rule.action}.label`)
    const permissionName = t(`signing.actions.${rule.action}.permissionName`)
    const required = draft.requirement === SigningRequirement.Required
    // The capacity counts the saved roles; with other roles the server checks the number.
    const newRoles = rolesChanged(initial, draft)
    const check = checkSignatures(parseSignatures(draft.signatures), capacity, {
        rolesChanged: newRoles,
        requesterExcluded: draft.requesterSigning === RequesterSigning.NotAllowed,
    })
    const eventWide = SIGNING_ACTIONS[rule.action].scope === SigningScope.Event
    const posts = (names: string[]) =>
        new Intl.ListFormat(i18n.language, {type: "conjunction"}).format(names)
    const signaturesHelp = (() => {
        switch (check.problem) {
            case SignaturesProblem.AtLeastOne:
                return t("signing.validation.atLeastOne")
            case SignaturesProblem.OutOfRange:
                return t("signing.validation.atMost", {max: check.max})
            case SignaturesProblem.TooMany:
                return t(
                    eventWide ? "signing.validation.tooManyEvent" : "signing.validation.tooMany",
                    {
                        n: draft.signatures.trim(),
                        max: check.max,
                    }
                )
        }
        if (newRoles) return t("signing.rule.checkedOnSave")
        // Event-wide actions have no Posts to count signers in.
        if (check.shortPosts.length || check.minSigners === null || !capacity?.posts.length) {
            return t("signing.rule.signaturesNeededShortHelp")
        }
        return t("signing.rule.signaturesNeededHelp", {n: check.minSigners})
    })()
    const changed = draftChanged(initial, draft)
    // The rule as it would be saved, for the preview of how it applies to scheduled transitions.
    const pending = {
        required: draft.requirement === SigningRequirement.Required,
        signatures: parseSignatures(draft.signatures),
    }
    const preview = useRuleChangePreview(electionEventId, rule.action, pending, changed)
    const valid = !required || trustee || check.problem === null
    const waiting = capacity?.waiting ?? 0

    const update = (change: Partial<IRuleDraft>) => {
        setSaveError(null)
        setDraft((current) => ({...current, ...change}))
    }

    const save = async () => {
        setSaveError(null)
        const input = ruleInputOf(electionEventId, rule, draft, initialRoles)
        let saved: ISaveSigningRuleOutput | undefined
        try {
            const {data} = await putRule({variables: {...input, roles: input.roles ?? null}})
            saved = data?.signingPutRule
        } catch (error) {
            setSaveError(writeErrorMessage(error, "signing.rule.saveError", {lockedDown}))
            return
        }
        // With new roles, the server tells which Posts can't reach the number yet.
        const short = saved?.short_posts ?? []
        // The warning palette's notification is too faint to read; the text says it all.
        if (short.length) {
            notify(
                t("signing.rule.savedShort", {
                    count: short.length,
                    posts: posts(short.map(({name}) => name)),
                }),
                {type: "success"}
            )
        } else if (saved?.warnings?.includes(SigningRuleWarning.RequesterExcluded)) {
            notify(t("signing.rule.savedRequesterShort"), {type: "success"})
        } else {
            // How the save applies to scheduled transitions, in the server's words.
            const applies = preview?.applies_message_key ?? appliesKey(preview?.applies)
            notify(applies ? `${t("signing.rule.saved")} ${t(applies)}` : t("signing.rule.saved"), {
                type: "success",
            })
        }
        onSaved()
    }

    return (
        <Drawer
            anchor="right"
            open
            onClose={onClose}
            slotProps={{paper: {"aria-labelledby": titleId}}}
            sx={{
                "& .MuiDrawer-paper": {
                    width: {xs: "100%", sm: "max(560px, 50vw)"},
                    maxWidth: "100%",
                },
            }}
        >
            <Box
                component="section"
                sx={{
                    width: "100%",
                    minWidth: 0,
                    boxSizing: "border-box",
                    p: {xs: 2, sm: 3},
                    display: "grid",
                    gridTemplateColumns: "minmax(0, 1fr)",
                    gap: 3,
                }}
            >
                <Box sx={{display: "flex", alignItems: "flex-start", gap: 1}}>
                    <Box sx={{flex: 1}}>
                        <Typography id={titleId} variant="h5" component="h2">
                            {action}
                        </Typography>
                        <Typography variant="body2" color="text.secondary">
                            {t(`signing.actions.${rule.action}.description`)}
                        </Typography>
                    </Box>
                    <IconButton aria-label={t("common.label.close")} onClick={onClose}>
                        <CloseIcon />
                    </IconButton>
                </Box>

                {readOnly && <Alert severity="info">{t("signing.readOnly.rules")}</Alert>}

                <Box>
                    <FormControlLabel
                        control={
                            <Switch
                                checked={required}
                                disabled={readOnly}
                                onChange={(event) =>
                                    update({
                                        requirement: event.target.checked
                                            ? SigningRequirement.Required
                                            : SigningRequirement.NotRequired,
                                    })
                                }
                            />
                        }
                        label={t(
                            trustee ? "signing.rule.trusteesSign" : "signing.rule.needsSignatures"
                        )}
                    />
                    {trustee && (
                        <Typography variant="body2" color="text.secondary">
                            {t("signing.rule.trusteesHelp")}
                        </Typography>
                    )}
                </Box>

                {required && !trustee && (
                    <>
                        <Box>
                            {editRoles ? (
                                <Autocomplete
                                    multiple
                                    options={roleOptions}
                                    value={sortRoleIds(draft.roles)}
                                    getOptionLabel={roleName}
                                    onChange={(_event, roles) => update({roles})}
                                    renderInput={(params) => (
                                        <TextField
                                            {...params}
                                            label={t("signing.rule.whoCanSign")}
                                            helperText={t("signing.rule.whoCanSignHelp", {
                                                action: permissionName,
                                            })}
                                        />
                                    )}
                                />
                            ) : (
                                <>
                                    <Typography variant="body2" color="text.secondary">
                                        {t("signing.rule.whoCanSign")}
                                    </Typography>
                                    <Box sx={{display: "flex", flexWrap: "wrap", gap: 0.5, my: 1}}>
                                        {initialRoles.length
                                            ? sortRoleIds(initialRoles).map((role) => (
                                                  <Chip
                                                      key={role}
                                                      size="small"
                                                      label={roleName(role)}
                                                  />
                                              ))
                                            : "–"}
                                    </Box>
                                    <Typography variant="body2" color="text.secondary">
                                        {t("signing.readOnly.whoCanSign", {action: permissionName})}
                                    </Typography>
                                </>
                            )}
                        </Box>

                        <TextField
                            type="number"
                            label={t("signing.rule.signaturesNeeded")}
                            value={draft.signatures}
                            error={check.problem !== null}
                            helperText={signaturesHelp}
                            onChange={(event) => update({signatures: event.target.value})}
                            slotProps={{
                                htmlInput: {min: 1, max: check.max ?? MAX_SIGNATURES, readOnly},
                            }}
                            fullWidth
                        />

                        {check.problem === null && check.shortPosts.length > 0 && (
                            <Alert severity="warning">
                                {check.shortPosts.map(({signers, posts: names}) => (
                                    <div key={signers}>
                                        {t("signing.validation.shortPosts", {
                                            count: names.length,
                                            posts: posts(names),
                                            n: signers,
                                            required: parseSignatures(draft.signatures),
                                        })}
                                    </div>
                                ))}
                            </Alert>
                        )}

                        {check.problem === null && check.requesterShort.length > 0 && (
                            <Alert severity="warning">
                                {check.requesterShort.map(({signers, posts: names}) => (
                                    <div key={signers}>
                                        {t("signing.validation.requesterShort", {
                                            count: names.length,
                                            posts: posts(names),
                                            n: signers,
                                            required: parseSignatures(draft.signatures),
                                        })}
                                    </div>
                                ))}
                            </Alert>
                        )}

                        <FormControlLabel
                            control={
                                <Checkbox
                                    checked={draft.requesterSigning === RequesterSigning.Allowed}
                                    disabled={readOnly}
                                    onChange={(event) =>
                                        update({
                                            requesterSigning: event.target.checked
                                                ? RequesterSigning.Allowed
                                                : RequesterSigning.NotAllowed,
                                        })
                                    }
                                />
                            }
                            label={t("signing.rule.requesterSigning")}
                        />

                        <FormControl fullWidth disabled={readOnly}>
                            <InputLabel id="signing-rule-expiry">
                                {t("signing.rule.expiresAfter")}
                            </InputLabel>
                            <Select
                                labelId="signing-rule-expiry"
                                label={t("signing.rule.expiresAfter")}
                                value={
                                    draft.expiresMinutes === null
                                        ? NO_LIMIT
                                        : String(draft.expiresMinutes)
                                }
                                onChange={(event) =>
                                    update({
                                        expiresMinutes:
                                            event.target.value === NO_LIMIT
                                                ? null
                                                : Number(event.target.value),
                                    })
                                }
                            >
                                {expiryOptions(draft.expiresMinutes).map((minutes) => (
                                    <MenuItem
                                        key={minutes ?? NO_LIMIT}
                                        value={minutes === null ? NO_LIMIT : String(minutes)}
                                    >
                                        {t(`signing.expiry.${expiryKey(minutes)}`, {
                                            count: minutes ?? 0,
                                        })}
                                    </MenuItem>
                                ))}
                            </Select>
                        </FormControl>
                    </>
                )}

                {!readOnly && <RuleChangeNotice preview={preview} />}

                {!readOnly && waiting > 0 && (
                    <Alert severity="info">{t("signing.pendingRequests", {count: waiting})}</Alert>
                )}

                {saveError && <Alert severity="error">{saveError}</Alert>}

                <Divider />
                <Box sx={{display: "flex", justifyContent: "flex-end", gap: 1}}>
                    {readOnly ? (
                        <Button onClick={onClose}>{t("common.label.close")}</Button>
                    ) : (
                        <>
                            <Button onClick={onClose}>{t("signing.rule.cancel")}</Button>
                            <Button
                                variant="contained"
                                disabled={!changed || !valid || saving}
                                onClick={save}
                            >
                                {t("signing.rule.save")}
                            </Button>
                        </>
                    )}
                </Box>
                <Typography variant="body2" color="text.secondary">
                    {t("signing.rule.footer")}
                </Typography>
            </Box>
        </Drawer>
    )
}

/** The drawer's expiries, plus a stored one it doesn't offer so the select can show it. */
const expiryOptions = (current: number | null): Array<number | null> =>
    SIGNING_EXPIRY_OPTIONS.includes(current)
        ? [...SIGNING_EXPIRY_OPTIONS]
        : [current, ...SIGNING_EXPIRY_OPTIONS]
