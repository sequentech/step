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
    REVOKE_RESULTS_PUBLICATION,
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

type RouteScope = "event" | "election"
type ResultsAccess = "public" | "authenticated"
type VisibilityScope = "full_event" | "area_based"

interface ResultsPublicationRecord extends RaRecord {
    id: string
    version: number
    publication_status: string
    route_scope: RouteScope
    route_election_id?: string | null
    access: ResultsAccess
    published_contest_ids?: unknown
    published_at?: string | null
}

const statusColor = (
    status?: string
): "default" | "success" | "warning" | "error" | "primary" | "secondary" | "info" => {
    switch (status) {
        case "Published":
            return "success"
        case "Publishing":
            return "warning"
        case "Failed":
            return "error"
        case "Revoked":
        case "Superseded":
            return "default"
        default:
            return "default"
    }
}

const normalizeAccess = (value?: string): ResultsAccess =>
    value === EResultsWebsiteAccess.AUTHENTICATED ? "authenticated" : "public"

const normalizeVisibilityScope = (
    value?: string,
    access: ResultsAccess = "public"
): VisibilityScope =>
    value === EResultsWebsiteVisibilityScope.AREA_BASED && access === "authenticated"
        ? "area_based"
        : "full_event"

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
    const [routeScope, setRouteScope] = useState<RouteScope>(
        (tallySession?.election_ids?.length ?? 0) > 1 ? "event" : "election"
    )
    const [routeElectionId, setRouteElectionId] = useState<string>(
        tallySession?.election_ids?.[0] ?? ""
    )
    const [access, setAccess] = useState<ResultsAccess>(
        normalizeAccess(resultsWebsitePolicy?.access)
    )
    const [visibilityScope, setVisibilityScope] = useState<VisibilityScope>(
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

    const [publishResultsWebsite, {loading: publishing}] = useMutation(PUBLISH_RESULTS_WEBSITE, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.PUBLISH_RESULTS_WRITE,
            },
        },
    })
    const [revokeResultsPublication, {loading: revoking}] = useMutation(
        REVOKE_RESULTS_PUBLICATION,
        {
            context: {
                headers: {
                    "x-hasura-role": IPermissions.PUBLISH_RESULTS_WRITE,
                },
            },
        }
    )

    useEffect(() => {
        const electionIds = tallySession?.election_ids ?? []

        if (!routeElectionId && electionIds.length > 0) {
            setRouteElectionId(electionIds[0])
        }

        if (!routeElectionId && electionIds.length > 1 && routeScope === "election") {
            setRouteScope("event")
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
        return routeScope === "election" && routeElectionId ? [routeElectionId] : tallyElectionIds
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
        if (access === "public") {
            setVisibilityScope("full_event")
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
        (routeScope === "event" || !!routeElectionId)

    const routeUrl = (publication?: ResultsPublicationRecord) => {
        const base = globalSettings.RESULTS_PORTAL_URL?.replace(/\/+$/, "")
        if (!base) return undefined
        const scope = publication?.route_scope ?? routeScope
        const electionId = publication?.route_election_id ?? routeElectionId
        const routePath =
            scope === "election"
                ? `${base}/${electionEventId}/elections/${electionId}`
                : `${base}/${electionEventId}`

        return publication?.publication_status === "Published" ? routePath : undefined
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
                    route_election_id: routeScope === "election" ? routeElectionId : null,
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
                        onChange={(event) => setRouteScope(event.target.value as RouteScope)}
                    >
                        <MenuItem value="event">
                            {t("tally.resultsPublication.eventResults")}
                        </MenuItem>
                        <MenuItem value="election">
                            {t("tally.resultsPublication.electionResults")}
                        </MenuItem>
                    </Select>
                </FormControl>
                {routeScope === "election" && (
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
                        onChange={(event) => setAccess(event.target.value as ResultsAccess)}
                    >
                        <MenuItem value="public">
                            {t("tally.resultsPublication.publicAccess")}
                        </MenuItem>
                        <MenuItem value="authenticated">
                            {t("tally.resultsPublication.authenticatedAccess")}
                        </MenuItem>
                    </Select>
                </FormControl>
                <FormControl fullWidth disabled={access === "public" || visibilityLockedByPolicy}>
                    <InputLabel id="results-visibility-label">
                        {t("tally.resultsPublication.visibility")}
                    </InputLabel>
                    <Select
                        labelId="results-visibility-label"
                        label={t("tally.resultsPublication.visibility")}
                        value={visibilityScope}
                        onChange={(event) =>
                            setVisibilityScope(event.target.value as VisibilityScope)
                        }
                    >
                        <MenuItem value="full_event">
                            {t("tally.resultsPublication.fullPublishedScope")}
                        </MenuItem>
                        <MenuItem value="area_based">
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
                                            {publication.route_scope === "election"
                                                ? `/${electionEventId}/elections/${publication.route_election_id}`
                                                : `/${electionEventId}`}
                                        </TableCell>
                                        <TableCell>{publication.access}</TableCell>
                                        <TableCell>{publishedContestCount(publication)}</TableCell>
                                        <TableCell>{publication.published_at ?? "-"}</TableCell>
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
                                                {publication.publication_status === "Published" && (
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
                                        <TableCell colSpan={7}>
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
