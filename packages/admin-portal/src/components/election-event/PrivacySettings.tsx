// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {Alert, Box, Button, Chip, CircularProgress, Stack, Typography} from "@mui/material"
import {useQuery} from "@apollo/client"
import {BALLOT_PRIVACY_SETTINGS, type IBallotPrivacySettings} from "@/queries/BallotPrivacySettings"
import {IPermissions} from "@/types/keycloak"
import {publicKeySha256, shortKeyFingerprint} from "@/services/transmissionPackage"
import {RadioButtonGroupInput, useRecordContext} from "react-admin"
import {Link} from "react-router"
import {useTranslation} from "react-i18next"
import {
    EBallotIdentity,
    ENetworkDataPolicy,
    type IElectionEventPresentation,
} from "@sequentech/ui-core"

export const PrivacySettings: React.FC<{canEdit: boolean}> = ({canEdit}) => {
    const {t} = useTranslation()
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
        <Stack spacing={2} sx={{minWidth: 0}}>
            <Typography variant="subtitle1" fontWeight={600}>
                {t("ballotPrivacy.settings.identity")}
            </Typography>
            <Typography>{t(`ballotPrivacy.settings.${pseudonymous ? "on" : "legacy"}`)}</Typography>
            {pseudonymous && <Chip label="7QKM-2HXD" sx={{alignSelf: "flex-start"}} />}
            <Typography variant="body2" color="text.secondary">
                {t("ballotPrivacy.settings.immutable")}
            </Typography>
            {fingerprint && (
                <Typography variant="body2" title={fingerprint}>
                    {t("ballotPrivacy.settings.authority", {
                        fingerprint: shortKeyFingerprint(fingerprint),
                    })}
                </Typography>
            )}
            <Typography variant="subtitle1" fontWeight={600}>
                {t("ballotPrivacy.settings.network")}
            </Typography>
            <Typography variant="body2">{t("ballotPrivacy.settings.description")}</Typography>
            <RadioButtonGroupInput
                source="presentation.network_data_policy"
                label={false}
                disabled={!canEdit || !pseudonymous}
                defaultValue={ENetworkDataPolicy.DO_NOT_STORE}
                row={false}
                choices={[
                    {
                        id: ENetworkDataPolicy.FORENSIC_STORE,
                        name: t("ballotPrivacy.settings.forensic"),
                    },
                    {id: ENetworkDataPolicy.DO_NOT_STORE, name: t("ballotPrivacy.settings.none")},
                ]}
            />
            {!pseudonymous && (
                <Alert severity="info">{t("ballotPrivacy.settings.legacyNetwork")}</Alert>
            )}
            <Typography variant="body2">{t("ballotPrivacy.settings.forensicHelp")}</Typography>
            <Typography variant="body2">{t("ballotPrivacy.settings.noneHelp")}</Typography>
            <Box>
                <Typography variant="subtitle1" fontWeight={600}>
                    {t("ballotPrivacy.settings.permissions")}
                </Typography>
                <Typography variant="body2">
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
                <Box component="dl" sx={{m: 0, mt: 1}}>
                    {["forensic", "resolve"].map((name) => (
                        <React.Fragment key={name}>
                            <Typography component="dt" fontWeight={600}>
                                {t(`ballotPrivacy.settings.${name}Permission`)}
                            </Typography>
                            <Typography component="dd" variant="body2" sx={{m: 0, mb: 1}}>
                                {t(`ballotPrivacy.settings.${name}PermissionHelp`)}
                                {summary && !loading && !error && (
                                    <Box component="span" sx={{display: "block", mt: 0.5}}>
                                        {(name === "forensic"
                                            ? summary.roles.forensic_read
                                            : summary.roles.pseudonym_resolve
                                        ).length
                                            ? (name === "forensic"
                                                  ? summary.roles.forensic_read
                                                  : summary.roles.pseudonym_resolve
                                              ).join(", ")
                                            : t("ballotPrivacy.settings.noRoles")}
                                    </Box>
                                )}
                            </Typography>
                        </React.Fragment>
                    ))}
                </Box>
                <Button component={Link} to="/user-roles">
                    {t("ballotPrivacy.settings.roles")}
                </Button>
            </Box>
        </Stack>
    )
}
