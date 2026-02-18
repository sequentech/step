// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useMemo, useState} from "react"
import {useTranslation} from "react-i18next"
import {useGetList, useNotify} from "react-admin"
import {useMutation} from "@apollo/client"
import {
    Box,
    Button,
    Chip,
    CircularProgress,
    Divider,
    FormControlLabel,
    Radio,
    RadioGroup,
    Typography,
} from "@mui/material"
import {
    Sequent_Backend_Candidate,
    Sequent_Backend_Contest,
    Sequent_Backend_Tally_Session,
    Sequent_Backend_Tally_Session_Resolution,
    SubmitTallyResolutionOutput,
    TallyResolutionInput,
} from "@/gql/graphql"
import {SUBMIT_TALLY_RESOLUTION} from "@/queries/SubmitTallyResolution"
import {IPermissions} from "@/types/keycloak"
import {SettingsContext} from "@/providers/SettingsContextProvider"

type SubmitResolutionMutationResult = {submit_tally_resolution?: SubmitTallyResolutionOutput | null}
type SubmitResolutionMutationVariables = {
    election_event_id: string
    tally_session_id: string
    resolutions: TallyResolutionInput[]
}

interface TallyResolutionPanelProps {
    tallySession: Sequent_Backend_Tally_Session
    contests: Sequent_Backend_Contest[]
    electionEventId: string
    tenantId: string | null
    onResolutionSubmitted: () => void
}

export const TallyResolutionPanel: React.FC<TallyResolutionPanelProps> = ({
    tallySession,
    contests,
    electionEventId,
    tenantId,
    onResolutionSubmitted,
}) => {
    const {t} = useTranslation()
    const notify = useNotify()
    const {globalSettings} = useContext(SettingsContext)
    const [selectedResolutionId, setSelectedResolutionId] = useState<string | null>(null)
    const [pendingSelections, setPendingSelections] = useState<Record<string, string>>({})
    const [submitting, setSubmitting] = useState(false)

    const [submitTallyResolution] = useMutation<
        SubmitResolutionMutationResult,
        SubmitResolutionMutationVariables
    >(SUBMIT_TALLY_RESOLUTION)

    // The DB column contest_id references results_contest.id (FK), not contest.id.
    // The actual contest_id is stored in resolution_data.contest_id by windmill.
    const getContestId = (r: Sequent_Backend_Tally_Session_Resolution): string | undefined =>
        r.resolution_data?.contest_id ?? r.contest_id ?? undefined

    // Fetch resolutions directly — ra-data-hasura does not include nested
    // relationships in useGetOne queries, so we query this table separately.
    const {data: allResolutions} = useGetList<Sequent_Backend_Tally_Session_Resolution>(
        "sequent_backend_tally_session_resolution",
        {
            pagination: {page: 1, perPage: 9999},
            sort: {field: "created_at", order: "DESC"},
            filter: {
                tally_session_id: tallySession.id,
                tenant_id: tenantId,
            },
        },
        {
            refetchInterval: globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
            refetchIntervalInBackground: true,
            refetchOnWindowFocus: false,
            enabled: !!tallySession.id && !!tenantId,
        }
    )

    // Keep only the latest pending resolution per contest (handles re-submissions)
    const latestPendingResolutions = useMemo(() => {
        const pending = (allResolutions ?? []).filter((r) => r.status === "pending")
        const byContest = new Map<string, Sequent_Backend_Tally_Session_Resolution>()
        for (const r of pending) {
            if (!r.contest_id) continue
            const existing = byContest.get(r.contest_id)
            if (!existing || r.created_at > existing.created_at) {
                byContest.set(r.contest_id, r)
            }
        }
        return Array.from(byContest.values())
    }, [allResolutions])

    // All resolved resolutions, sorted most-recent first
    const resolvedResolutions = useMemo(
        () =>
            (allResolutions ?? [])
                .filter((r) => r.status === "resolved")
                .sort((a, b) => b.created_at.localeCompare(a.created_at)),
        [allResolutions]
    )

    const allContestIds = useMemo(() => {
        const ids = new Set<string>()
        for (const r of [...latestPendingResolutions, ...resolvedResolutions]) {
            const cid = getContestId(r)
            if (cid) ids.add(cid)
        }
        return Array.from(ids)
    }, [latestPendingResolutions, resolvedResolutions])

    const {data: candidates} = useGetList<Sequent_Backend_Candidate>(
        "sequent_backend_candidate",
        {
            pagination: {page: 1, perPage: 9999},
            filter: {
                tenant_id: tenantId,
                contest_id:
                    allContestIds.length > 0
                        ? {
                              format: "hasura-raw-query",
                              value: {_in: allContestIds},
                          }
                        : undefined,
            },
        },
        {
            enabled: allContestIds.length > 0,
        }
    )

    const selectedResolution = useMemo(
        () => (allResolutions ?? []).find((r) => r.id === selectedResolutionId) ?? null,
        [allResolutions, selectedResolutionId]
    )

    const selectedContestName = useMemo(() => {
        if (!selectedResolution) return ""
        const cid = getContestId(selectedResolution)
        return contests.find((c) => c.id === cid)?.name ?? cid ?? ""
    }, [contests, selectedResolution])

    const tiedCandidatesForSelected = useMemo(() => {
        if (!selectedResolution?.resolution_data) return []
        const tiedIds: string[] = selectedResolution.resolution_data.tied_candidate_ids ?? []
        return (candidates ?? []).filter((c) => tiedIds.includes(c.id))
    }, [selectedResolution, candidates])

    const resolvedCandidateForSelected = useMemo(() => {
        if (!selectedResolution?.resolution) return null
        const candidateId = selectedResolution.resolution.resolved_by_candidate_id
        return (candidates ?? []).find((c) => c.id === candidateId) ?? null
    }, [selectedResolution, candidates])

    const allResolved =
        latestPendingResolutions.length > 0 &&
        latestPendingResolutions.every((r) => r.contest_id && pendingSelections[r.contest_id])

    const handleSubmit = async () => {
        setSubmitting(true)
        try {
            await submitTallyResolution({
                variables: {
                    election_event_id: electionEventId,
                    tally_session_id: tallySession.id,
                    resolutions: Object.entries(pendingSelections).map(
                        ([contest_id, selected_candidate_id]) => ({
                            contest_id,
                            selected_candidate_id,
                        })
                    ),
                },
                context: {
                    headers: {"x-hasura-role": IPermissions.ADMIN_CEREMONY},
                },
            })
            notify("tally.pendingResolutions.submitSuccess", {type: "success"})
            setPendingSelections({})
            onResolutionSubmitted()
        } catch {
            notify("tally.pendingResolutions.submitError", {type: "error"})
        } finally {
            setSubmitting(false)
        }
    }

    if ((allResolutions ?? []).length === 0) {
        return null
    }

    const renderResolutionItem = (
        resolution: Sequent_Backend_Tally_Session_Resolution,
        isPending: boolean
    ) => {
        const cid = getContestId(resolution)
        const contestName = contests.find((c) => c.id === cid)?.name ?? cid
        const isSelected = selectedResolutionId === resolution.id
        const isDecided = !!(
            isPending &&
            resolution.contest_id &&
            pendingSelections[resolution.contest_id]
        )
        return (
            <Box
                key={resolution.id}
                onClick={() => setSelectedResolutionId(resolution.id)}
                sx={{
                    "cursor": "pointer",
                    "p": 1.5,
                    "mb": 1,
                    "border": "1px solid",
                    "borderColor": isSelected ? "primary.main" : "divider",
                    "borderRadius": 1,
                    "backgroundColor": isSelected ? "action.selected" : "background.paper",
                    "&:hover": {backgroundColor: "action.hover"},
                }}
            >
                <Typography variant="subtitle2" noWrap>
                    {contestName}
                </Typography>
                <Typography variant="caption" color="text.secondary">
                    {t("tally.pendingResolutions.round", {
                        round: resolution.resolution_data?.round_number ?? "?",
                    })}
                </Typography>
                <Box sx={{mt: 0.5}}>
                    <Chip
                        label={
                            isPending
                                ? isDecided
                                    ? t("tally.pendingResolutions.decided")
                                    : t("tally.pendingResolutions.pendingDecision")
                                : t("tally.pendingResolutions.resolved")
                        }
                        color={!isPending || isDecided ? "success" : "warning"}
                        size="small"
                    />
                </Box>
            </Box>
        )
    }

    return (
        <Box
            sx={{
                mt: 2,
                p: 2,
                border: "1px solid",
                borderColor: "divider",
                borderRadius: 1,
                width: "100%",
            }}
        >
            <Typography variant="h6" gutterBottom>
                {t("tally.pendingResolutions.title")}
            </Typography>

            <Box sx={{display: "flex", gap: 2, mt: 1}}>
                {/* LEFT PANEL - list of resolutions */}
                <Box sx={{width: "40%", minWidth: 220}}>
                    {latestPendingResolutions.length > 0 && (
                        <>
                            <Typography
                                variant="overline"
                                color="text.secondary"
                                sx={{display: "block", mb: 0.5}}
                            >
                                {t("tally.pendingResolutions.pendingSection")}
                            </Typography>
                            {latestPendingResolutions.map((r) => renderResolutionItem(r, true))}
                        </>
                    )}

                    {resolvedResolutions.length > 0 && (
                        <>
                            {latestPendingResolutions.length > 0 && <Divider sx={{my: 1}} />}
                            <Typography
                                variant="overline"
                                color="text.secondary"
                                sx={{display: "block", mb: 0.5}}
                            >
                                {t("tally.pendingResolutions.resolvedSection")}
                            </Typography>
                            {resolvedResolutions.map((r) => renderResolutionItem(r, false))}
                        </>
                    )}
                </Box>

                {/* RIGHT PANEL */}
                {selectedResolution ? (
                    <Box sx={{flex: 1}}>
                        <Typography variant="subtitle1" gutterBottom>
                            {t("tally.pendingResolutions.resolutionTitle")}
                        </Typography>
                        <Typography variant="body2" color="text.secondary" gutterBottom>
                            {t("tally.pendingResolutions.roundInfo", {
                                round: selectedResolution.resolution_data?.round_number ?? "?",
                                contest: selectedContestName,
                            })}
                        </Typography>

                        {selectedResolution.status === "pending" ? (
                            <>
                                <RadioGroup
                                    value={
                                        pendingSelections[selectedResolution.contest_id ?? ""] ?? ""
                                    }
                                    onChange={(e) =>
                                        setPendingSelections((prev) => ({
                                            ...prev,
                                            [selectedResolution.contest_id!]: e.target.value,
                                        }))
                                    }
                                >
                                    {tiedCandidatesForSelected.map((candidate) => (
                                        <FormControlLabel
                                            key={candidate.id}
                                            value={candidate.id}
                                            control={<Radio />}
                                            label={candidate.name ?? candidate.id}
                                        />
                                    ))}
                                </RadioGroup>
                                {pendingSelections[selectedResolution.contest_id ?? ""] && (
                                    <Button
                                        variant="outlined"
                                        size="small"
                                        sx={{mt: 1}}
                                        onClick={() =>
                                            setPendingSelections((prev) => {
                                                const next = {...prev}
                                                delete next[selectedResolution.contest_id!]
                                                return next
                                            })
                                        }
                                    >
                                        {t("tally.pendingResolutions.undoResolution")}
                                    </Button>
                                )}
                            </>
                        ) : (
                            <Typography variant="body2" sx={{mt: 1}}>
                                {t("tally.pendingResolutions.winner", {
                                    candidate:
                                        resolvedCandidateForSelected?.name ??
                                        selectedResolution.resolution?.resolved_by_candidate_id ??
                                        "?",
                                })}
                            </Typography>
                        )}
                    </Box>
                ) : (
                    <Box
                        sx={{
                            flex: 1,
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "center",
                        }}
                    >
                        <Typography color="text.secondary">
                            {t("tally.pendingResolutions.selectContest")}
                        </Typography>
                    </Box>
                )}
            </Box>

            {latestPendingResolutions.length > 0 && (
                <Box sx={{mt: 2, display: "flex", justifyContent: "flex-end"}}>
                    <Button
                        variant="contained"
                        color="primary"
                        disabled={!allResolved || submitting}
                        onClick={handleSubmit}
                        startIcon={
                            submitting ? <CircularProgress size={16} color="inherit" /> : null
                        }
                    >
                        {t("tally.pendingResolutions.applyResolutions")}
                    </Button>
                </Box>
            )}
        </Box>
    )
}
