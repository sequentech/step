// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useState} from "react"
import {useQuery} from "@apollo/client"
import {
    Alert,
    Button,
    CircularProgress,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    MenuItem,
    TextField,
} from "@mui/material"
import {useTranslation} from "react-i18next"
import {AuthContext} from "@/providers/AuthContextProvider"
import type {
    GetInitializationCountriesQuery,
    GetInitializationContestsQuery,
    GetInitializationStylesQuery,
} from "@/gql/graphql"
import {IPermissions} from "@/types/keycloak"
import {
    GET_INITIALIZATION_COUNTRIES,
    GET_INITIALIZATION_CONTESTS,
    GET_INITIALIZATION_STYLES,
} from "@/queries/GetInitializationCountries"
import {
    initializationCountries,
    initializationAreaIds,
    type InitializationCountriesData,
} from "./initializationCountries"

import type {ILifecycleSnapshotEntry} from "@/queries/Lifecycle"

export const InitializationCountryDialog = ({
    electionEventId,
    electionId,
    busy,
    snapshots = [],
    onClose,
    onGenerate,
}: {
    electionEventId: string
    electionId: string
    busy: boolean
    snapshots?: readonly ILifecycleSnapshotEntry[]
    onClose: () => void
    onGenerate: (areaIds?: string[]) => Promise<boolean>
}) => {
    const {t} = useTranslation()
    const [country, setCountry] = useState("")
    const {hasRole} = useContext(AuthContext)
    const roleFor = (permission: IPermissions) =>
        hasRole(permission)
            ? permission
            : hasRole(IPermissions.ADMIN_USER)
              ? IPermissions.ADMIN_USER
              : null
    const areaRole = roleFor(IPermissions.AREA_READ)
    const contestRole = roleFor(IPermissions.CONTEST_READ)
    const styleRole = roleFor(IPermissions.PUBLISH_READ)
    const areaQuery = useQuery<GetInitializationCountriesQuery>(GET_INITIALIZATION_COUNTRIES, {
        variables: {electionEventId},
        context: {headers: {"x-hasura-role": areaRole}},
        skip: !areaRole,
        fetchPolicy: "network-only",
    })
    const contestQuery = useQuery<GetInitializationContestsQuery>(GET_INITIALIZATION_CONTESTS, {
        variables: {electionEventId, electionId},
        context: {headers: {"x-hasura-role": contestRole}},
        skip: !contestRole,
        fetchPolicy: "network-only",
    })
    const styleQuery = useQuery<GetInitializationStylesQuery>(GET_INITIALIZATION_STYLES, {
        variables: {electionEventId, electionId},
        context: {headers: {"x-hasura-role": styleRole}},
        skip: !styleRole,
        fetchPolicy: "network-only",
    })
    const loading = areaQuery.loading || contestQuery.loading || styleQuery.loading
    const error =
        areaQuery.error ||
        contestQuery.error ||
        styleQuery.error ||
        !areaRole ||
        !contestRole ||
        !styleRole
    const data: InitializationCountriesData | undefined =
        areaQuery.data && contestQuery.data && styleQuery.data
            ? {
                  sequent_backend_area: areaQuery.data.sequent_backend_area,
                  sequent_backend_area_contest: areaQuery.data.sequent_backend_area_contest.map(
                      (link) => ({
                          area_id: link.area_id,
                          contest:
                              contestQuery.data?.sequent_backend_contest.find(
                                  ({id}) => id === link.contest_id
                              ) ?? null,
                      })
                  ),
                  sequent_backend_ballot_style: styleQuery.data.sequent_backend_ballot_style,
              }
            : undefined
    const countries = data ? initializationCountries(data, electionId, snapshots) : []
    const published = snapshots.find(
        (entry) => entry.election_id === null || entry.election_id === electionId
    )
    const knownEmpty = published?.snapshot.initialization_countries?.[electionId]?.length === 0
    const canGenerate = countries.length > 0 || knownEmpty
    const selectionValid = !country || countries.some((area) => area.id === country)
    const generate = async () => {
        if (busy || loading || error || !canGenerate || !selectionValid) return
        if (
            await onGenerate(
                countries.length ? initializationAreaIds(country, countries) : undefined
            )
        )
            onClose()
    }
    return (
        <Dialog open onClose={busy ? undefined : onClose} fullWidth maxWidth="sm">
            <DialogTitle>{t("publish.action.generateInitializationReport")}</DialogTitle>
            <DialogContent>
                <Alert severity="info" sx={{mb: 2}}>
                    {t("publish.initialization.countryInfo")}
                </Alert>
                {loading ? (
                    <CircularProgress size={24} />
                ) : error ? (
                    <Alert severity="error">{t("publish.initialization.countriesError")}</Alert>
                ) : !canGenerate ? (
                    <Alert severity="error">{t("publish.initialization.noCountries")}</Alert>
                ) : (
                    <TextField
                        select
                        fullWidth
                        label={t("publish.initialization.country")}
                        SelectProps={{displayEmpty: true}}
                        InputLabelProps={{shrink: true}}
                        value={country}
                        onChange={(event) => setCountry(event.target.value)}
                        disabled={busy}
                    >
                        <MenuItem value="">{t("publish.initialization.entirePost")}</MenuItem>
                        {countries.map((area) => (
                            <MenuItem key={area.id} value={area.id}>
                                {area.name || area.id}
                            </MenuItem>
                        ))}
                    </TextField>
                )}
            </DialogContent>
            <DialogActions>
                <Button onClick={onClose} disabled={busy}>
                    {t("common.label.cancel")}
                </Button>
                <Button
                    onClick={generate}
                    disabled={busy || loading || !!error || !canGenerate || !selectionValid}
                >
                    {busy ? (
                        <CircularProgress size={16} />
                    ) : (
                        t("publish.action.generateInitializationReport")
                    )}
                </Button>
            </DialogActions>
        </Dialog>
    )
}
