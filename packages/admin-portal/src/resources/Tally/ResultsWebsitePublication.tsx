// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useMemo, useState} from "react"
import {
    Alert,
    Box,
    Button,
    Checkbox,
    Chip,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    FormControl,
    FormControlLabel,
    InputLabel,
    MenuItem,
    Select,
    SelectChangeEvent,
    Stack,
    Table,
    TableBody,
    TableCell,
    TableContainer,
    TableHead,
    TableRow,
    Typography,
} from "@mui/material"
import {useMutation} from "@apollo/client"
import {RaRecord, useGetList, useNotify} from "react-admin"
import {useAtomValue} from "jotai"
import {useTranslation} from "react-i18next"
import {
    EResultsWebsiteAccess,
    EResultsPublicationStatus,
    EResultsRouteScope,
    EResultsWebsiteStatus,
    EResultsWebsiteVisibilityScope,
    IResultsWebsitePolicy,
    translateFromPresentation,
} from "@sequentech/ui-core"
import {tallyQueryData} from "@/atoms/tally-candidates"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {
    PUBLISH_RESULTS_WEBSITE,
    PublishResultsWebsiteData,
    PublishResultsWebsiteVariables,
    REVOKE_RESULTS_PUBLICATION,
    RevokeResultsPublicationData,
    RevokeResultsPublicationVariables,
} from "@/queries/ResultsWebsitePublication"
import {IPermissions} from "@/types/keycloak"
import {ETasksExecution} from "@/types/tasksExecution"
import {WidgetProps} from "@/components/Widget"
import {
    Sequent_Backend_Contest,
    Sequent_Backend_Election,
    Sequent_Backend_Tally_Session,
    Sequent_Backend_Tally_Session_Execution,
} from "@/gql/graphql"

interface ResultsWebsitePublicationProps {
    tenantId: string
    electionEventId: string
    tallySession?: Sequent_Backend_Tally_Session
    tallySessionExecution?: Sequent_Backend_Tally_Session_Execution
    resultsEventId: string | null
    contests: Sequent_Backend_Contest[]
    elections: Sequent_Backend_Election[]
    resultsWebsitePolicy?: IResultsWebsitePolicy | null
}

interface ResultsPublicationRecord extends RaRecord {
    id: string
    version: number
    publication_status: EResultsPublicationStatus
    route_scope: EResultsRouteScope
    route_election_id?: string | null
    access: EResultsWebsiteAccess
    published_contest_ids?: string[]
    published_at?: string | null
    revoked_at?: string | null
}

const statusColor = (
    status?: EResultsPublicationStatus
): "default" | "success" | "warning" | "error" | "primary" | "secondary" | "info" => {
    switch (status) {
        case EResultsPublicationStatus.PUBLISHED:
            return "success"
        case EResultsPublicationStatus.PUBLISHING:
            return "warning"
        case EResultsPublicationStatus.FAILED:
            return "error"
        case EResultsPublicationStatus.REVOKED:
        case EResultsPublicationStatus.SUPERSEDED:
            return "default"
        default:
            return "default"
    }
}

const normalizeAccess = (value?: EResultsWebsiteAccess): EResultsWebsiteAccess =>
    value === EResultsWebsiteAccess.AUTHENTICATED
        ? EResultsWebsiteAccess.AUTHENTICATED
        : EResultsWebsiteAccess.PUBLIC

const normalizeVisibilityScope = (
    value?: EResultsWebsiteVisibilityScope,
    access: EResultsWebsiteAccess = EResultsWebsiteAccess.PUBLIC
): EResultsWebsiteVisibilityScope =>
    value === EResultsWebsiteVisibilityScope.AREA_BASED &&
    access === EResultsWebsiteAccess.AUTHENTICATED
        ? EResultsWebsiteVisibilityScope.AREA_BASED
        : EResultsWebsiteVisibilityScope.FULL_EVENT

export const ResultsWebsitePublication: React.FC<ResultsWebsitePublicationProps> = ({
    tenantId,
    electionEventId,
    tallySession,
    tallySessionExecution,
    resultsEventId,
    contests,
    elections,
    resultsWebsitePolicy,
}) => {
    const {t} = useTranslation()
    const notify = useNotify()
    const {globalSettings} = useContext(SettingsContext)
    const authContext = useContext(AuthContext)
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()
    const tallyData = useAtomValue(tallyQueryData)
    const [routeScope, setRouteScope] = useState<EResultsRouteScope>(
        (tallySession?.election_ids?.length ?? 0) > 1
            ? EResultsRouteScope.EVENT
            : EResultsRouteScope.ELECTION
    )
    const [routeElectionId, setRouteElectionId] = useState<string>(
        tallySession?.election_ids?.[0] ?? ""
    )
    const [access, setAccess] = useState<EResultsWebsiteAccess>(
        normalizeAccess(resultsWebsitePolicy?.access)
    )
    const [visibilityScope, setVisibilityScope] = useState<EResultsWebsiteVisibilityScope>(
        normalizeVisibilityScope(resultsWebsitePolicy?.visibility_scope, access)
    )
    const [selectedContestIds, setSelectedContestIds] = useState<string[]>([])
    const [confirmOpen, setConfirmOpen] = useState(false)
    const canReadResultsPublication = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.PUBLISH_RESULTS_READ
    )
    const canWriteResultsPublication = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.PUBLISH_RESULTS_WRITE
    )
    const policyEnabled = resultsWebsitePolicy?.status === EResultsWebsiteStatus.ENABLED
    const policyAccess = normalizeAccess(resultsWebsitePolicy?.access)
    const policyVisibilityScope = normalizeVisibilityScope(
        resultsWebsitePolicy?.visibility_scope,
        policyAccess
    )
    const accessLockedByPolicy = !!resultsWebsitePolicy?.access
    const visibilityLockedByPolicy = !!resultsWebsitePolicy?.visibility_scope

    const [publishResultsWebsite, {loading: publishing}] = useMutation<
        PublishResultsWebsiteData,
        PublishResultsWebsiteVariables
    >(PUBLISH_RESULTS_WEBSITE, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.PUBLISH_RESULTS_WRITE,
            },
        },
    })
    const [revokeResultsPublication, {loading: revoking}] = useMutation<
        RevokeResultsPublicationData,
        RevokeResultsPublicationVariables
    >(REVOKE_RESULTS_PUBLICATION, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.PUBLISH_RESULTS_WRITE,
            },
        },
    })

    useEffect(() => {
        const electionIds = tallySession?.election_ids ?? []

        if (!routeElectionId && electionIds.length > 0) {
            setRouteElectionId(electionIds[0])
        }

        if (
            !routeElectionId &&
            electionIds.length > 1 &&
            routeScope === EResultsRouteScope.ELECTION
        ) {
            setRouteScope(EResultsRouteScope.EVENT)
        }
    }, [routeElectionId, routeScope, tallySession?.election_ids])

    useEffect(() => {
        if (resultsWebsitePolicy?.access) {
            setAccess(policyAccess)
        }

        if (resultsWebsitePolicy?.visibility_scope) {
            setVisibilityScope(policyVisibilityScope)
        }
    }, [
        policyAccess,
        policyVisibilityScope,
        resultsWebsitePolicy?.access,
        resultsWebsitePolicy?.visibility_scope,
    ])

    const {data: publications, refetch: refetchPublications} = useGetList<ResultsPublicationRecord>(
        "sequent_backend_tally_results_publication",
        {
            pagination: {page: 1, perPage: 50},
            sort: {field: "version", order: "DESC"},
            filter: {
                tenant_id: tenantId,
                election_event_id: electionEventId,
            },
        },
        {
            enabled: canReadResultsPublication,
        }
    )

    const talliedContestIds = useMemo(() => {
        return new Set(
            tallyData?.sequent_backend_results_contest?.map((contest) => contest.contest_id) ?? []
        )
    }, [tallyData?.sequent_backend_results_contest])

    const scopedElectionIds = useMemo(() => {
        const tallyElectionIds = tallySession?.election_ids ?? []
        return routeScope === EResultsRouteScope.ELECTION && routeElectionId
            ? [routeElectionId]
            : tallyElectionIds
    }, [routeElectionId, routeScope, tallySession?.election_ids])

    const eligibleContests = useMemo(() => {
        return contests.filter((contest) => {
            const inPublishedScope = scopedElectionIds.includes(contest.election_id)
            const hasResults = talliedContestIds.has(contest.id)
            return inPublishedScope && hasResults
        })
    }, [contests, scopedElectionIds, talliedContestIds])

    const eligibleContestIds = useMemo(
        () => eligibleContests.map((contest) => contest.id),
        [eligibleContests]
    )

    useEffect(() => {
        setSelectedContestIds((current) => {
            const retained = current.filter((contestId) => eligibleContestIds.includes(contestId))
            const next = retained.length > 0 ? retained : eligibleContestIds

            if (
                next.length === current.length &&
                next.every((id, index) => current[index] === id)
            ) {
                return current
            }

            return next
        })
    }, [eligibleContestIds])

    useEffect(() => {
        if (access === EResultsWebsiteAccess.PUBLIC) {
            setVisibilityScope(EResultsWebsiteVisibilityScope.FULL_EVENT)
        }
    }, [access])

    const selectedEligibleContestIds = useMemo(
        () => selectedContestIds.filter((contestId) => eligibleContestIds.includes(contestId)),
        [eligibleContestIds, selectedContestIds]
    )

    const canPublish =
        canWriteResultsPublication &&
        policyEnabled &&
        !!tallySession?.id &&
        !!tallySessionExecution?.id &&
        !!resultsEventId &&
        selectedEligibleContestIds.length > 0 &&
        scopedElectionIds.length > 0 &&
        (routeScope === EResultsRouteScope.EVENT || !!routeElectionId)

    const routeUrl = (publication?: ResultsPublicationRecord) => {
        const base = globalSettings.RESULTS_PORTAL_URL?.replace(/\/+$/, "")
        if (!base) return undefined
        const scope = publication?.route_scope ?? routeScope
        const electionId = publication?.route_election_id ?? routeElectionId
        const routePath =
            scope === EResultsRouteScope.ELECTION
                ? `${base}/${electionEventId}/elections/${electionId}`
                : `${base}/${electionEventId}`

        return publication?.publication_status === EResultsPublicationStatus.PUBLISHED
            ? routePath
            : undefined
    }

    const publishedContestCount = (publication: ResultsPublicationRecord) =>
        Array.isArray(publication.published_contest_ids)
            ? publication.published_contest_ids.length
            : "-"

    const contestLabel = (contest: Sequent_Backend_Contest) =>
        translateFromPresentation(contest, "name", "en") ?? contest.id

    const electionLabel = (electionId: string) => {
        const election = elections.find((item) => item.id === electionId)
        return translateFromPresentation(election, "name", "en") ?? electionId
    }

    const handleToggleContest = (contestId: string) => {
        setSelectedContestIds((current) =>
            current.includes(contestId)
                ? current.filter((id) => id !== contestId)
                : [...current, contestId]
        )
    }

    const handlePublish = async () => {
        if (!canPublish || !tallySession || !tallySessionExecution || !resultsEventId) {
            return
        }

        let currWidget: WidgetProps | undefined
        try {
            currWidget = addWidget(ETasksExecution.PUBLISH_RESULTS_WEBSITE, undefined)

            const result = await publishResultsWebsite({
                variables: {
                    election_event_id: electionEventId,
                    tally_session_id: tallySession.id,
                    tally_session_execution_id: tallySessionExecution.id,
                    results_event_id: resultsEventId,
                    route_scope: routeScope,
                    route_election_id:
                        routeScope === EResultsRouteScope.ELECTION ? routeElectionId : null,
                    election_ids: scopedElectionIds,
                    contest_ids: selectedEligibleContestIds,
                    access,
                    visibility_scope: visibilityScope,
                },
            })

            const publishResult = result.data?.publishResultsWebsite
            const errorMsg = publishResult?.error_msg
            if (errorMsg) {
                notify(errorMsg, {type: "warning"})
                updateWidgetFail(currWidget.identifier)
            } else {
                notify(t("tally.resultsPublication.publishStarted"), {type: "success"})
                publishResult?.task_execution_id
                    ? setWidgetTaskId(currWidget.identifier, publishResult.task_execution_id, () =>
                          refetchPublications?.()
                      )
                    : updateWidgetFail(currWidget.identifier)
            }
            refetchPublications?.()
        } catch (error) {
            console.error(error)
            notify(t("tally.resultsPublication.publishError"), {type: "error"})
            currWidget && updateWidgetFail(currWidget.identifier)
        }
        setConfirmOpen(false)
    }

    const handleRevoke = async (publicationId: string) => {
        if (!canWriteResultsPublication) {
            return
        }

        try {
            await revokeResultsPublication({
                variables: {
                    election_event_id: electionEventId,
                    publication_id: publicationId,
                },
            })
            notify(t("tally.resultsPublication.revoked"), {type: "success"})
            refetchPublications?.()
        } catch (error) {
            console.error(error)
            notify(t("tally.resultsPublication.revokeError"), {type: "error"})
        }
    }

    return (
        <Stack spacing={3} sx={{width: "100%"}}>
            {!resultsEventId || !tallySessionExecution?.id ? (
                <Alert severity="info">{t("tally.resultsPublication.waitingForTally")}</Alert>
            ) : null}
            {!canWriteResultsPublication ? (
                <Alert severity="warning">
                    {t("tally.resultsPublication.writePermissionRequired")}
                </Alert>
            ) : null}
            {!policyEnabled ? (
                <Alert severity="warning">{t("tally.resultsPublication.disabledPolicy")}</Alert>
            ) : null}

            <Stack direction={{xs: "column", md: "row"}} spacing={2}>
                <FormControl fullWidth>
                    <InputLabel id="results-route-scope-label">
                        {t("tally.resultsPublication.route")}
                    </InputLabel>
                    <Select
                        labelId="results-route-scope-label"
                        label={t("tally.resultsPublication.route")}
                        value={routeScope}
                        onChange={(event: SelectChangeEvent<EResultsRouteScope>) =>
                            setRouteScope(event.target.value)
                        }
                    >
                        <MenuItem value={EResultsRouteScope.EVENT}>
                            {t("tally.resultsPublication.eventResults")}
                        </MenuItem>
                        <MenuItem value={EResultsRouteScope.ELECTION}>
                            {t("tally.resultsPublication.electionResults")}
                        </MenuItem>
                    </Select>
                </FormControl>
                {routeScope === EResultsRouteScope.ELECTION && (
                    <FormControl fullWidth>
                        <InputLabel id="results-route-election-label">
                            {t("tally.resultsPublication.election")}
                        </InputLabel>
                        <Select
                            labelId="results-route-election-label"
                            label={t("tally.resultsPublication.election")}
                            value={routeElectionId}
                            onChange={(event) => setRouteElectionId(event.target.value)}
                        >
                            {(tallySession?.election_ids ?? []).map((electionId) => (
                                <MenuItem key={electionId} value={electionId}>
                                    {electionLabel(electionId)}
                                </MenuItem>
                            ))}
                        </Select>
                    </FormControl>
                )}
            </Stack>

            <Stack direction={{xs: "column", md: "row"}} spacing={2}>
                <FormControl fullWidth disabled={accessLockedByPolicy}>
                    <InputLabel id="results-access-label">
                        {t("tally.resultsPublication.access")}
                    </InputLabel>
                    <Select
                        labelId="results-access-label"
                        label={t("tally.resultsPublication.access")}
                        value={access}
                        onChange={(event: SelectChangeEvent<EResultsWebsiteAccess>) =>
                            setAccess(event.target.value)
                        }
                    >
                        <MenuItem value={EResultsWebsiteAccess.PUBLIC}>
                            {t("tally.resultsPublication.publicAccess")}
                        </MenuItem>
                        <MenuItem value={EResultsWebsiteAccess.AUTHENTICATED}>
                            {t("tally.resultsPublication.authenticatedAccess")}
                        </MenuItem>
                    </Select>
                </FormControl>
                <FormControl
                    fullWidth
                    disabled={access === EResultsWebsiteAccess.PUBLIC || visibilityLockedByPolicy}
                >
                    <InputLabel id="results-visibility-label">
                        {t("tally.resultsPublication.visibility")}
                    </InputLabel>
                    <Select
                        labelId="results-visibility-label"
                        label={t("tally.resultsPublication.visibility")}
                        value={visibilityScope}
                        onChange={(event: SelectChangeEvent<EResultsWebsiteVisibilityScope>) =>
                            setVisibilityScope(event.target.value)
                        }
                    >
                        <MenuItem value={EResultsWebsiteVisibilityScope.FULL_EVENT}>
                            {t("tally.resultsPublication.fullPublishedScope")}
                        </MenuItem>
                        <MenuItem value={EResultsWebsiteVisibilityScope.AREA_BASED}>
                            {t("tally.resultsPublication.personalVisibility")}
                        </MenuItem>
                    </Select>
                </FormControl>
            </Stack>

            <Box>
                <Typography variant="h6" sx={{mb: 1}}>
                    {t("tally.resultsPublication.contests")}
                </Typography>
                <Stack spacing={1}>
                    {eligibleContests.map((contest) => (
                        <FormControlLabel
                            key={contest.id}
                            control={
                                <Checkbox
                                    checked={selectedContestIds.includes(contest.id)}
                                    onChange={() => handleToggleContest(contest.id)}
                                />
                            }
                            label={`${electionLabel(contest.election_id)} - ${contestLabel(contest)}`}
                        />
                    ))}
                    {eligibleContests.length === 0 && (
                        <Typography color="text.secondary">
                            {t("tally.resultsPublication.noTalliedContests")}
                        </Typography>
                    )}
                </Stack>
            </Box>

            <Stack direction="row" spacing={2} alignItems="center">
                <Button
                    variant="contained"
                    disabled={!canPublish || publishing}
                    onClick={() => setConfirmOpen(true)}
                >
                    {t("tally.resultsPublication.publishSelectedContests")}
                </Button>
                <Typography color="text.secondary">
                    {t("tally.resultsPublication.selectedContestCount", {
                        count: selectedEligibleContestIds.length,
                    })}
                </Typography>
            </Stack>

            {canReadResultsPublication ? (
                <Box>
                    <Typography variant="h6" sx={{mb: 1}}>
                        {t("tally.resultsPublication.history")}
                    </Typography>
                    <TableContainer>
                        <Table size="small">
                            <TableHead>
                                <TableRow>
                                    <TableCell>{t("tally.resultsPublication.version")}</TableCell>
                                    <TableCell>{t("tally.resultsPublication.status")}</TableCell>
                                    <TableCell>{t("tally.resultsPublication.route")}</TableCell>
                                    <TableCell>{t("tally.resultsPublication.access")}</TableCell>
                                    <TableCell>{t("tally.resultsPublication.contests")}</TableCell>
                                    <TableCell>{t("tally.resultsPublication.published")}</TableCell>
                                    <TableCell>{t("tally.resultsPublication.revokedAt")}</TableCell>
                                    <TableCell align="right">
                                        {t("tally.resultsPublication.actions")}
                                    </TableCell>
                                </TableRow>
                            </TableHead>
                            <TableBody>
                                {(publications ?? []).map((publication) => (
                                    <TableRow key={publication.id}>
                                        <TableCell>{publication.version}</TableCell>
                                        <TableCell>
                                            <Chip
                                                size="small"
                                                label={publication.publication_status}
                                                color={statusColor(publication.publication_status)}
                                            />
                                        </TableCell>
                                        <TableCell>
                                            {publication.route_scope === EResultsRouteScope.ELECTION
                                                ? `/${electionEventId}/elections/${publication.route_election_id}`
                                                : `/${electionEventId}`}
                                        </TableCell>
                                        <TableCell>{publication.access}</TableCell>
                                        <TableCell>{publishedContestCount(publication)}</TableCell>
                                        <TableCell>{publication.published_at ?? "-"}</TableCell>
                                        <TableCell>{publication.revoked_at ?? "-"}</TableCell>
                                        <TableCell align="right">
                                            <Stack
                                                direction="row"
                                                spacing={1}
                                                justifyContent="flex-end"
                                            >
                                                {routeUrl(publication) && (
                                                    <Button
                                                        size="small"
                                                        component="a"
                                                        href={routeUrl(publication)}
                                                        target="_blank"
                                                        rel="noreferrer"
                                                    >
                                                        {t("tally.resultsPublication.open")}
                                                    </Button>
                                                )}
                                                {publication.publication_status ===
                                                    EResultsPublicationStatus.PUBLISHED && (
                                                    <Button
                                                        size="small"
                                                        color="error"
                                                        disabled={
                                                            !canWriteResultsPublication || revoking
                                                        }
                                                        onClick={() => handleRevoke(publication.id)}
                                                    >
                                                        {t("tally.resultsPublication.revoke")}
                                                    </Button>
                                                )}
                                            </Stack>
                                        </TableCell>
                                    </TableRow>
                                ))}
                                {(publications ?? []).length === 0 && (
                                    <TableRow>
                                        <TableCell colSpan={8}>
                                            <Typography color="text.secondary">
                                                {t("tally.resultsPublication.noPublications")}
                                            </Typography>
                                        </TableCell>
                                    </TableRow>
                                )}
                            </TableBody>
                        </Table>
                    </TableContainer>
                </Box>
            ) : (
                <Alert severity="warning">
                    {t("tally.resultsPublication.readPermissionRequired")}
                </Alert>
            )}

            <Dialog open={confirmOpen} onClose={() => setConfirmOpen(false)}>
                <DialogTitle>{t("tally.resultsPublication.confirmTitle")}</DialogTitle>
                <DialogContent>
                    <Typography>{t("tally.resultsPublication.confirmDescription")}</Typography>
                </DialogContent>
                <DialogActions>
                    <Button onClick={() => setConfirmOpen(false)}>
                        {t("tally.resultsPublication.close")}
                    </Button>
                    <Button variant="contained" disabled={publishing} onClick={handlePublish}>
                        {t("tally.resultsPublication.publishSelectedContests")}
                    </Button>
                </DialogActions>
            </Dialog>
        </Stack>
    )
}
