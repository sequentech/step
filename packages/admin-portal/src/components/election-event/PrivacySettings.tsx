// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useId, useState} from "react"
import {
    Alert,
    Box,
    Button,
    Chip,
    CircularProgress,
    Link as MuiLink,
    Radio,
    RadioGroup,
    Stack,
    Typography,
} from "@mui/material"
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome"
import {faArrowRight, faBan, faCircleCheck, faKey, faLock} from "@fortawesome/free-solid-svg-icons"
import {useQuery} from "@apollo/client"
import {BALLOT_PRIVACY_SETTINGS, type IBallotPrivacySettings} from "@/queries/BallotPrivacySettings"
import {IPermissions} from "@/types/keycloak"
import {publicKeySha256, shortKeyFingerprint} from "@/services/transmissionPackage"
import {useInput, useRecordContext} from "react-admin"
import {Link} from "react-router"
import {useTranslation} from "react-i18next"
import {PseudonymChip} from "@sequentech/ui-essentials"
import {
    EBallotIdentity,
    ENetworkDataPolicy,
    type IElectionEventPresentation,
} from "@sequentech/ui-core"

const NetworkPolicyCards: React.FC<{disabled: boolean; labelledBy: string}> = ({
    disabled,
    labelledBy,
}) => {
    const {t} = useTranslation()
    const {field} = useInput({
        source: "presentation.network_data_policy",
        defaultValue: ENetworkDataPolicy.DO_NOT_STORE,
    })
    return (
        <RadioGroup
            aria-labelledby={labelledBy}
            name={field.name}
            value={field.value}
            onChange={field.onChange}
            onBlur={field.onBlur}
            sx={{display: "grid", gridTemplateColumns: {xs: "1fr", md: "1fr 1fr"}, gap: 1.5}}
        >
            {[
                {id: ENetworkDataPolicy.FORENSIC_STORE, label: "forensic", icon: faLock},
                {id: ENetworkDataPolicy.DO_NOT_STORE, label: "none", icon: faBan},
            ].map(({id, label, icon}) => {
                const selected = field.value === id
                return (
                    <Box
                        component="label"
                        key={id}
                        sx={{
                            "display": "grid",
                            "gridTemplateColumns": "40px minmax(0, 1fr) auto",
                            "gap": 1.5,
                            "p": "14px 12px 14px 16px",
                            "borderRadius": "8px",
                            "border": "2px solid",
                            "borderColor": selected ? "#0F054C" : "#DFE6EF",
                            "bgcolor": selected ? "#F7F9FE" : "background.paper",
                            "cursor": disabled ? "default" : "pointer",
                            "opacity": disabled ? 0.7 : 1,
                            "&:focus-within": {
                                outline: "2px solid",
                                outlineColor: "primary.main",
                                outlineOffset: 2,
                            },
                        }}
                    >
                        <Box
                            sx={{
                                width: 40,
                                height: 40,
                                borderRadius: "50%",
                                display: "grid",
                                placeItems: "center",
                                bgcolor: selected ? "#0F054C" : "#F1F5F9",
                                color: selected ? "primary.contrastText" : "#475569",
                            }}
                        >
                            <FontAwesomeIcon icon={icon} />
                        </Box>
                        <Box sx={{minWidth: 0}}>
                            <Typography
                                component="span"
                                sx={{m: 0, display: "block", fontWeight: 600, fontSize: 15}}
                            >
                                {t(`ballotPrivacy.settings.${label}`)}
                            </Typography>
                            <Typography
                                component="span"
                                variant="body2"
                                color="text.secondary"
                                sx={{m: 0, display: "block", mt: 0.25, fontSize: 13.5}}
                            >
                                {t(`ballotPrivacy.settings.${label}Help`)}
                            </Typography>
                        </Box>
                        <Radio
                            value={id}
                            disabled={disabled}
                            slotProps={{
                                input: {
                                    "aria-label": t(`ballotPrivacy.settings.${label}`),
                                    "ref": field.ref,
                                },
                            }}
                            sx={{
                                "p": 0.5,
                                "alignSelf": "start",
                                "color": "#94A3B8",
                                "&.Mui-checked": {color: "#0F054C"},
                            }}
                        />
                    </Box>
                )
            })}
        </RadioGroup>
    )
}

export const PrivacySettings: React.FC<{canEdit: boolean}> = ({canEdit}) => {
    const {t} = useTranslation()
    const id = useId()
    const record = useRecordContext<{id: string; presentation?: IElectionEventPresentation}>()
    const [fingerprint, setFingerprint] = useState<string>()
    const {data, loading, error, refetch} = useQuery<{
        ballot_privacy_settings: {payload: IBallotPrivacySettings}
    }>(BALLOT_PRIVACY_SETTINGS, {
        variables: {electionEventId: record?.id},
        skip: !record?.id,
        fetchPolicy: "no-cache",
        context: {headers: {"x-hasura-role": IPermissions.ELECTION_EVENT_READ}},
    })
    const summary = data?.ballot_privacy_settings.payload
    useEffect(() => {
        let active = true
        setFingerprint(undefined)
        const key = summary?.system_public_key
        if (key)
            void publicKeySha256(`-----BEGIN PUBLIC KEY-----\n${key}\n-----END PUBLIC KEY-----`)
                .then((hash) => {
                    if (active) setFingerprint(hash)
                })
                .catch(() => undefined)
        return () => {
            active = false
        }
    }, [summary?.system_public_key])
    const pseudonymous = record?.presentation?.ballot_identity === EBallotIdentity.PSEUDONYM
    return (
        <Stack
            spacing={3.5}
            sx={{
                width: "100%",
                minWidth: 0,
                py: 1,
                fontFamily: "Roboto, Helvetica, Arial, sans-serif",
            }}
        >
            <Box role="group" aria-labelledby={`${id}-identity`}>
                <Typography
                    id={`${id}-identity`}
                    component="h3"
                    sx={{m: 0, fontSize: 16, fontWeight: 700, mb: 1.5}}
                >
                    {t("ballotPrivacy.settings.identity")}
                </Typography>
                <Box
                    sx={{
                        display: "grid",
                        gridTemplateColumns: "24px minmax(0, 1fr)",
                        gap: 1.25,
                        p: "14px 16px",
                        borderRadius: "8px",
                        border: "1px solid",
                        borderColor: pseudonymous ? "#A7F3D0" : "divider",
                        bgcolor: pseudonymous ? "#ECFDF5" : "#F7F9FC",
                        color: pseudonymous ? "#065F46" : "text.secondary",
                    }}
                >
                    <FontAwesomeIcon
                        icon={pseudonymous ? faCircleCheck : faLock}
                        style={{marginTop: 3, fontSize: 20}}
                    />
                    <Box sx={{minWidth: 0}}>
                        <Typography sx={{m: 0, fontSize: 15, fontWeight: 600}}>
                            {t(`ballotPrivacy.settings.${pseudonymous ? "on" : "legacy"}`)}
                        </Typography>
                        <Stack
                            direction="row"
                            useFlexGap
                            flexWrap="wrap"
                            alignItems="center"
                            gap={1}
                            sx={{mt: 0.75, fontSize: 13.5}}
                        >
                            {pseudonymous && (
                                <>
                                    <span>{t("ballotPrivacy.settings.example")}</span>
                                    <PseudonymChip>7QKM-2HXD</PseudonymChip>
                                </>
                            )}
                            <span>{t("ballotPrivacy.settings.immutable")}</span>
                        </Stack>
                        {fingerprint && (
                            <Stack direction="row" alignItems="center" spacing={1} sx={{mt: 0.75}}>
                                <FontAwesomeIcon icon={faKey} />
                                <Typography variant="body2" title={fingerprint} sx={{m: 0}}>
                                    {t("ballotPrivacy.settings.authority", {
                                        fingerprint: shortKeyFingerprint(fingerprint),
                                    })}
                                </Typography>
                            </Stack>
                        )}
                    </Box>
                </Box>
            </Box>
            <Box>
                <Typography
                    id={`${id}-network`}
                    component="h3"
                    sx={{m: 0, fontSize: 16, fontWeight: 700}}
                >
                    {t("ballotPrivacy.settings.network")}
                </Typography>
                <Typography variant="body2" color="text.secondary" sx={{m: 0, mt: 0.25, mb: 1.5}}>
                    {t("ballotPrivacy.settings.description")}
                </Typography>
                <NetworkPolicyCards
                    disabled={!canEdit || !pseudonymous}
                    labelledBy={`${id}-network`}
                />
                {!pseudonymous && (
                    <Alert severity="info" sx={{mt: 1.5}}>
                        {t("ballotPrivacy.settings.legacyNetwork")}
                    </Alert>
                )}
            </Box>
            <Box role="group" aria-labelledby={`${id}-permissions`}>
                <Typography
                    id={`${id}-permissions`}
                    component="h3"
                    sx={{m: 0, fontSize: 16, fontWeight: 700}}
                >
                    {t("ballotPrivacy.settings.permissions")}
                </Typography>
                <Typography variant="body2" color="text.secondary" sx={{m: 0, mt: 0.25, mb: 1.5}}>
                    {t("ballotPrivacy.settings.permissionHelp")}
                </Typography>
                {loading ? (
                    <CircularProgress size={24} aria-label={t("ballotPrivacy.settings.loading")} />
                ) : error || !summary ? (
                    <Alert
                        severity="error"
                        action={
                            <Button onClick={() => void refetch().catch(() => undefined)}>
                                {t("ballotPrivacy.retry")}
                            </Button>
                        }
                    >
                        {t("ballotPrivacy.settings.loadError")}
                    </Alert>
                ) : null}
                <Box
                    component="dl"
                    sx={{
                        m: 0,
                        border: "1px solid",
                        borderColor: "divider",
                        borderRadius: "8px",
                        overflow: "hidden",
                    }}
                >
                    {(["forensic", "resolve"] as const).map((name, index) => {
                        const roles =
                            name === "forensic"
                                ? summary?.roles.forensic_read
                                : summary?.roles.pseudonym_resolve
                        return (
                            <Box
                                key={name}
                                sx={{
                                    display: "grid",
                                    gridTemplateColumns: {xs: "1fr", md: "1fr 1fr"},
                                    columnGap: 3,
                                    rowGap: 1,
                                    px: 2,
                                    py: 1.5,
                                    borderTop: index ? "1px solid" : 0,
                                    borderColor: "divider",
                                }}
                            >
                                <Box component="dt">
                                    <Typography
                                        component="span"
                                        sx={{
                                            m: 0,
                                            display: "block",
                                            fontSize: 14.5,
                                            fontWeight: 600,
                                        }}
                                    >
                                        {t(`ballotPrivacy.settings.${name}Permission`)}
                                    </Typography>
                                    <Typography
                                        component="span"
                                        variant="body2"
                                        color="text.secondary"
                                        sx={{m: 0, display: "block", mt: 0.25, fontSize: 13.5}}
                                    >
                                        {t(`ballotPrivacy.settings.${name}PermissionHelp`)}
                                    </Typography>
                                </Box>
                                <Box
                                    component="dd"
                                    sx={{
                                        m: 0,
                                        display: "flex",
                                        flexWrap: "wrap",
                                        alignItems: "center",
                                        gap: 0.75,
                                        minWidth: 0,
                                    }}
                                >
                                    {summary &&
                                        !loading &&
                                        !error &&
                                        (roles?.length ? (
                                            roles.map((role) => (
                                                <Chip
                                                    key={role}
                                                    label={role}
                                                    size="small"
                                                    sx={{
                                                        "height": "auto",
                                                        "minHeight": 24,
                                                        "bgcolor": "#F1F0FA",
                                                        "color": "#0F054C",
                                                        "fontWeight": 500,
                                                        "& .MuiChip-label": {
                                                            whiteSpace: "normal",
                                                            overflowWrap: "anywhere",
                                                        },
                                                    }}
                                                />
                                            ))
                                        ) : (
                                            <Typography
                                                variant="body2"
                                                color="text.secondary"
                                                sx={{m: 0}}
                                            >
                                                {t("ballotPrivacy.settings.noRoles")}
                                            </Typography>
                                        ))}
                                </Box>
                            </Box>
                        )
                    })}
                </Box>
                <MuiLink
                    component={Link}
                    to="/user-roles"
                    underline="hover"
                    sx={{
                        display: "inline-flex",
                        alignItems: "center",
                        gap: 0.75,
                        mt: 1.5,
                        fontSize: 14.5,
                        fontWeight: 500,
                        color: "#0F054C",
                    }}
                >
                    {t("ballotPrivacy.settings.roles")}
                    <FontAwesomeIcon icon={faArrowRight} />
                </MuiLink>
            </Box>
        </Stack>
    )
}
