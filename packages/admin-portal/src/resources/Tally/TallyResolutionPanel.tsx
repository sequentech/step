// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useMemo, useState} from "react"
import {useAtomValue} from "jotai"
import {tallyQueryData} from "@/atoms/tally-candidates"
import {useTranslation} from "react-i18next"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {useGetList, useNotify} from "react-admin"
import {useMutation} from "@apollo/client"
import {
    Alert,
    AlertTitle,
    Autocomplete,
    Box,
    Button,
    Chip,
    CircularProgress,
    Popover,
    TextField,
    Typography,
} from "@mui/material"
import ViewColumnIcon from "@mui/icons-material/ViewColumn"
import InfoOutlineIcon from "@mui/icons-material/InfoOutlined"
import AssignmentIcon from "@mui/icons-material/Assignment"
import ChevronRightIcon from "@mui/icons-material/ChevronRight"
import {
    Sequent_Backend_Contest,
    Sequent_Backend_Election,
    Sequent_Backend_Tally_Session,
    Sequent_Backend_Tally_Session_Contest,
    Sequent_Backend_Tally_Session_Resolution,
    SubmitTallyResolutionOutput,
    TallyResolutionInput,
} from "@/gql/graphql"
import {SUBMIT_TALLY_RESOLUTION} from "@/queries/SubmitTallyResolution"
import {IPermissions} from "@/types/keycloak"
import {ITallyExecutionStatus} from "@/types/ceremonies"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {ITieBreakingPolicy} from "@sequentech/ui-core"

const GLOBAL_AREA_ID = "__global__"

type SubmitResolutionMutationResult = {submit_tally_resolution?: SubmitTallyResolutionOutput | null}
type SubmitResolutionMutationVariables = {
    election_event_id: string
    tally_session_id: string
    resolutions: TallyResolutionInput[]
}

interface TallyResolutionPanelProps {
    tallySession: Sequent_Backend_Tally_Session
    contests: Sequent_Backend_Contest[]
    elections: Sequent_Backend_Election[]
    electionEventId: string
    tenantId: string | null
    onResolutionSubmitted: () => void
}

interface TallySessionResolutionData {
    round_number?: number
    tied_candidate_ids: Array<string>
    vote_count: number
    method_used: ITieBreakingPolicy
    resolved_by_candidate_id?: string
}

export const TallyResolutionPanel: React.FC<TallyResolutionPanelProps> = ({
    tallySession,
    contests,
    elections,
    electionEventId,
    tenantId,
    onResolutionSubmitted,
}) => {
    const {t, i18n} = useTranslation()
    const aliasRenderer = useAliasRenderer()
    const notify = useNotify()
    const {globalSettings} = useContext(SettingsContext)
    const tallyData = useAtomValue(tallyQueryData)
    const [selectedResolutionId, setSelectedResolutionId] = useState<string | null>(null)
    // draftSelections: temporary, updated by radio onChange (not yet committed)
    const [draftSelections, setDraftSelections] = useState<Record<string, string>>({})
    // pendingSelections: committed by clicking Save; drives the Apply button and chip state
    const [pendingSelections, setPendingSelections] = useState<Record<string, string>>({})
    const [resolvedEditingIds, setResolvedEditingIds] = useState<Record<string, boolean>>({})
    const [submitting, setSubmitting] = useState(false)
    // Filter popover state
    const [filterAnchorEl, setFilterAnchorEl] = useState<HTMLButtonElement | null>(null)
    const [filterElections, setFilterElections] = useState<string[]>([])
    const [filterContests, setFilterContests] = useState<string[]>([])
    const [filterAreas, setFilterAreas] = useState<string[]>([])
    const [filterStatuses, setFilterStatuses] = useState<string[]>([])

    const [submitTallyResolution] = useMutation<
        SubmitResolutionMutationResult,
        SubmitResolutionMutationVariables
    >(SUBMIT_TALLY_RESOLUTION)

    const getContestId = (r: Sequent_Backend_Tally_Session_Resolution): string | undefined =>
        r.contest_id ?? undefined

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

    const {data: tallySessionContests} = useGetList<Sequent_Backend_Tally_Session_Contest>(
        "sequent_backend_tally_session_contest",
        {
            pagination: {page: 1, perPage: 9999},
            filter: {tally_session_id: tallySession.id, tenant_id: tenantId},
        },
        {enabled: !!tallySession.id && !!tenantId}
    )

    const areas = tallyData?.sequent_backend_area ?? []

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

    // All resolved resolutions, sorted most-recent first.
    // Excludes contests that currently have a pending resolution — those are superseded.
    const resolvedResolutions = useMemo(() => {
        const pendingContestIds = new Set(
            latestPendingResolutions.map((r) => r.contest_id).filter((id): id is string => !!id)
        )
        return (allResolutions ?? [])
            .filter(
                (r) =>
                    r.status === "resolved" &&
                    !(r.contest_id && pendingContestIds.has(r.contest_id))
            )
            .sort((a, b) => b.created_at.localeCompare(a.created_at))
    }, [allResolutions, latestPendingResolutions])

    // Combined display list: pending first, then resolved
    const allDisplayResolutions = useMemo(
        () => [...latestPendingResolutions, ...resolvedResolutions],
        [latestPendingResolutions, resolvedResolutions]
    )

    const allContestIds = useMemo(() => {
        const ids = new Set<string>()
        for (const r of allDisplayResolutions) {
            const cid = getContestId(r)
            if (cid) ids.add(cid)
        }
        return Array.from(ids)
    }, [allDisplayResolutions])

    const candidates = useMemo(
        () =>
            (tallyData?.sequent_backend_candidate ?? []).filter((c) =>
                allContestIds.includes(c.contest_id)
            ),
        [tallyData, allContestIds]
    )

    const electionMap = useMemo(() => {
        const map = new Map<string, string>()
        for (const e of elections) {
            map.set(e.id, aliasRenderer(e.presentation))
        }
        return map
    }, [elections, i18n.language])

    const areaMap = useMemo(() => {
        const map = new Map<string, string>()
        for (const a of areas ?? []) {
            if (a.name) map.set(a.id, a.name)
        }
        return map
    }, [areas])

    const totalTallyAreaCount = useMemo(() => {
        const ids = new Set<string>()
        for (const tsc of tallySessionContests ?? []) {
            ids.add(tsc.area_id)
        }
        return ids.size
    }, [tallySessionContests])

    // actual contest_id → area label for this tally session
    // Returns "Global" when the contest spans all areas in the tally session
    const contestAreaLabel = useMemo(() => {
        const map = new Map<string, string>()
        const contestAreaIds = new Map<string, string[]>()
        for (const tsc of tallySessionContests ?? []) {
            if (!tsc.contest_id) continue
            const ids = contestAreaIds.get(tsc.contest_id) ?? []
            if (!ids.includes(tsc.area_id)) ids.push(tsc.area_id)
            contestAreaIds.set(tsc.contest_id, ids)
        }
        contestAreaIds.forEach((areaIds, contestId) => {
            if (totalTallyAreaCount > 0 && areaIds.length === totalTallyAreaCount) {
                map.set(contestId, t("tally.pendingResolutions.globalArea"))
            } else {
                const names = areaIds
                    .map((id) => areaMap.get(id) ?? "")
                    .filter((name) => name !== "")
                map.set(contestId, names.join(", "))
            }
        })
        return map
    }, [tallySessionContests, areaMap, totalTallyAreaCount, t])

    // Filter dropdown options: only items that appear in the current resolution list
    const relevantElections = useMemo(() => {
        const ids = new Set<string>()
        for (const r of allDisplayResolutions) {
            const contest = contests.find((c) => c.id === getContestId(r))
            if (contest?.election_id) ids.add(contest.election_id)
        }
        return elections.filter((e) => ids.has(e.id))
    }, [allDisplayResolutions, contests, elections])

    const relevantContests = useMemo(() => {
        const ids = new Set(
            allDisplayResolutions.map((r) => getContestId(r)).filter((v): v is string => !!v)
        )
        return contests.filter((c) => ids.has(c.id))
    }, [allDisplayResolutions, contests])

    const relevantAreas = useMemo(() => {
        const contestIds = new Set(
            allDisplayResolutions.map((r) => getContestId(r)).filter((v): v is string => !!v)
        )
        const areaIds = new Set<string>()
        for (const tsc of tallySessionContests ?? []) {
            if (contestIds.has(tsc.contest_id)) areaIds.add(tsc.area_id)
        }
        return (areas ?? []).filter((a) => areaIds.has(a.id))
    }, [allDisplayResolutions, tallySessionContests, areas])

    const relevantAreaOptions = useMemo(() => {
        const globalLabel = t("tally.pendingResolutions.globalArea")
        const hasGlobal = allDisplayResolutions.some((r) => {
            const cid = getContestId(r)
            return cid && contestAreaLabel.get(cid) === globalLabel
        })
        const areaOptions = relevantAreas.map((a) => ({id: a.id, name: a.name ?? ""}))
        if (hasGlobal) {
            return [{id: GLOBAL_AREA_ID, name: globalLabel}, ...areaOptions]
        }
        return areaOptions
    }, [allDisplayResolutions, relevantAreas, contestAreaLabel, t])

    const activeFilterCount = [filterElections, filterContests, filterAreas, filterStatuses].filter(
        (f) => f.length > 0
    ).length

    const filteredResolutions = useMemo(() => {
        return allDisplayResolutions.filter((r) => {
            const contestId = getContestId(r)
            const contest = contests.find((c) => c.id === contestId)

            if (
                filterElections.length > 0 &&
                (!contest?.election_id || !filterElections.includes(contest.election_id))
            )
                return false
            if (filterContests.length > 0 && (!contestId || !filterContests.includes(contestId)))
                return false
            if (filterAreas.length > 0) {
                const globalLabel = t("tally.pendingResolutions.globalArea")
                const isGlobal = !!contestId && contestAreaLabel.get(contestId) === globalLabel
                const matchesArea = filterAreas.some((areaId) => {
                    if (areaId === GLOBAL_AREA_ID) return isGlobal
                    if (isGlobal) return false
                    const contestAreas = (tallySessionContests ?? []).filter(
                        (tsc) => tsc.contest_id === contestId
                    )
                    return contestAreas.some((tsc) => tsc.area_id === areaId)
                })
                if (!matchesArea) return false
            }
            if (filterStatuses.length > 0) {
                const isLocallyDecided = !!(r.contest_id && pendingSelections[r.contest_id])
                const matchesAny = filterStatuses.some((status) => {
                    if (status === "decided") return r.status === "pending" && isLocallyDecided
                    if (status === "pending") return r.status === "pending" && !isLocallyDecided
                    if (status === "resolved") return r.status === "resolved"
                    return false
                })
                if (!matchesAny) return false
            }
            return true
        })
    }, [
        allDisplayResolutions,
        filterElections,
        filterContests,
        filterAreas,
        filterStatuses,
        contests,
        tallySessionContests,
        contestAreaLabel,
        pendingSelections,
        t,
    ])

    const selectedResolution = useMemo(
        () => (allResolutions ?? []).find((r) => r.id === selectedResolutionId) ?? null,
        [allResolutions, selectedResolutionId]
    )

    const tiedCandidatesForSelected = useMemo(() => {
        if (!selectedResolution?.resolution_data) return []
        const tiedIds: string[] =
            (selectedResolution.resolution_data as TallySessionResolutionData).tied_candidate_ids ??
            []
        return (candidates ?? []).filter((c) => tiedIds.includes(c.id))
    }, [selectedResolution, candidates])

    const resolvedCandidateForSelected = useMemo(() => {
        if (!selectedResolution?.resolution_data) return null
        const candidateId = (selectedResolution.resolution_data as TallySessionResolutionData)
            .resolved_by_candidate_id
        return (candidates ?? []).find((c) => c.id === candidateId) ?? null
    }, [selectedResolution, candidates])

    const totalVotes: number | undefined = useMemo(() => {
        if (!selectedResolution?.contest_id) return undefined
        return (
            (tallyData?.sequent_backend_results_contest ?? []).find(
                (rc) => rc.contest_id === selectedResolution.contest_id
            )?.total_votes ?? undefined
        )
    }, [selectedResolution, tallyData])

    const hasResolvedRedecisions = resolvedResolutions.some(
        (r) =>
            r.contest_id &&
            pendingSelections[r.contest_id] &&
            pendingSelections[r.contest_id] !==
                (r.resolution_data as TallySessionResolutionData)?.resolved_by_candidate_id
    )

    // Apply button is enabled when:
    // - session is awaiting input and at least one pending resolution has been decided, OR
    // - any already-resolved resolution has been re-decided (allowed regardless of session status)
    const canApply =
        !submitting &&
        ((tallySession.execution_status === ITallyExecutionStatus.AWAITING_INPUT &&
            latestPendingResolutions.some(
                (r) => r.contest_id && pendingSelections[r.contest_id]
            )) ||
            hasResolvedRedecisions)

    const handleSave = () => {
        if (!selectedResolution?.contest_id) return
        const contestId = selectedResolution.contest_id
        const draftValue = draftSelections[contestId]
        if (!draftValue) return
        setPendingSelections((prev) => ({...prev, [contestId]: draftValue}))
        if (!isPendingSelected) {
            setResolvedEditingIds((prev) => {
                const next = {...prev}
                delete next[selectedResolution.id]
                return next
            })
        }
    }

    const handleUndoDecision = () => {
        if (!selectedResolution?.contest_id) return
        const contestId = selectedResolution.contest_id
        const savedCandidate = pendingSelections[contestId]
        if (savedCandidate) {
            setDraftSelections((prev) => ({...prev, [contestId]: savedCandidate}))
        }
        setPendingSelections((prev) => {
            const next = {...prev}
            delete next[contestId]
            return next
        })
        if (!isPendingSelected && selectedResolution) {
            setResolvedEditingIds((prev) => ({...prev, [selectedResolution.id]: true}))
        }
    }

    const handleStartResolvedEdit = () => {
        if (!selectedResolution?.contest_id) return
        const currentCandidate = (selectedResolution.resolution_data as TallySessionResolutionData)
            ?.resolved_by_candidate_id
        if (currentCandidate) {
            setDraftSelections((prev) => ({
                ...prev,
                [selectedResolution.contest_id!]: currentCandidate,
            }))
        }
        setResolvedEditingIds((prev) => ({...prev, [selectedResolution.id]: true}))
    }

    const handleSubmit = async () => {
        setSubmitting(true)
        const pendingContestIds = new Set(
            latestPendingResolutions.map((r) => r.contest_id).filter(Boolean)
        )
        const resolvedContestIds = new Set(
            resolvedResolutions.map((r) => r.contest_id).filter(Boolean)
        )
        try {
            await submitTallyResolution({
                variables: {
                    election_event_id: electionEventId,
                    tally_session_id: tallySession.id,
                    resolutions: Object.entries(pendingSelections)
                        .filter(
                            ([contest_id]) =>
                                pendingContestIds.has(contest_id) ||
                                resolvedContestIds.has(contest_id)
                        )
                        .map(([contest_id, selected_candidate_id]) => ({
                            contest_id,
                            selected_candidate_id,
                        })),
                },
                context: {
                    headers: {"x-hasura-role": IPermissions.TALLY_RESOLUTION_SUBMIT},
                },
            })
            notify(t("tally.pendingResolutions.submitSuccess"), {type: "success"})
            setPendingSelections({})
            setDraftSelections({})
            setResolvedEditingIds({})
            onResolutionSubmitted()
        } catch {
            notify(t("tally.pendingResolutions.submitError"), {type: "error"})
        } finally {
            setSubmitting(false)
        }
    }

    if (allDisplayResolutions.length === 0) {
        return null
    }

    const getResolutionSubtitle = (r: Sequent_Backend_Tally_Session_Resolution): string => {
        const contestId = getContestId(r)
        const contest = contests.find((c) => c.id === contestId)
        const parts: string[] = []
        if (contest) {
            const electionName = electionMap.get(contest.election_id)
            if (electionName) parts.push(electionName)
            parts.push(aliasRenderer(contest.presentation))
        }
        if (contestId) {
            const areaLabel = contestAreaLabel.get(contestId)
            if (areaLabel) parts.push(areaLabel)
        }
        const round = (r.resolution_data as TallySessionResolutionData)?.round_number
        if (round !== undefined && round !== null) {
            parts.push(t("tally.pendingResolutions.round", {round}))
        }
        return parts.join(" | ")
    }

    const renderResolutionItem = (resolution: Sequent_Backend_Tally_Session_Resolution) => {
        const isPending = resolution.status === "pending"
        const isDecided =
            isPending && !!(resolution.contest_id && pendingSelections[resolution.contest_id])
        const isResolvedRedecided =
            !isPending &&
            !!(resolution.contest_id && pendingSelections[resolution.contest_id]) &&
            pendingSelections[resolution.contest_id] !==
                (resolution.resolution_data as TallySessionResolutionData)?.resolved_by_candidate_id
        const isSelected = selectedResolutionId === resolution.id
        const subtitle = getResolutionSubtitle(resolution)
        const titleKey = isPending
            ? "tally.pendingResolutions.tieResolutionRequired"
            : "tally.pendingResolutions.tieResolved"

        let chipLabel: string
        let chipColor: "warning" | "info" | "success"
        if (isDecided || isResolvedRedecided) {
            chipLabel = t("tally.pendingResolutions.pendingApplyStatus")
            chipColor = "info"
        } else if (isPending) {
            chipLabel = t("tally.pendingResolutions.pendingResolutionStatus")
            chipColor = "warning"
        } else {
            chipLabel = t("tally.pendingResolutions.resolvedStatus")
            chipColor = "success"
        }

        const icon =
            resolution.resolution_type === "irv_tie_break" ? (
                <InfoOutlineIcon fontSize="small" color="action" />
            ) : (
                <AssignmentIcon fontSize="small" color="action" />
            )

        return (
            <Box
                key={resolution.id}
                onClick={() => setSelectedResolutionId(resolution.id)}
                sx={{
                    "display": "flex",
                    "alignItems": "center",
                    "minHeight": 75,
                    "px": 1.5,
                    "py": 1,
                    "borderBottom": "1px solid",
                    "borderColor": "divider",
                    "backgroundColor": isSelected ? "action.selected" : "transparent",
                    "cursor": "pointer",
                    "&:hover": {
                        backgroundColor: isSelected ? "action.selected" : "action.hover",
                    },
                }}
            >
                <Box sx={{mr: 2.5, flexShrink: 0, display: "flex", alignItems: "center"}}>
                    {icon}
                </Box>

                <Box sx={{flex: 1, minWidth: 0}}>
                    <Typography variant="body2">
                        <Box component="span" sx={{fontWeight: 700}}>
                            {t(titleKey)}
                        </Box>
                        {subtitle ? `: ${subtitle}` : ""}
                    </Typography>
                </Box>

                <Chip
                    label={chipLabel}
                    size="small"
                    color={chipColor}
                    sx={{ml: 1, flexShrink: 0, borderRadius: "100px"}}
                />
                <ChevronRightIcon fontSize="small" color="action" sx={{ml: 0.5, flexShrink: 0}} />
            </Box>
        )
    }

    const statusFilterOptions = [
        {id: "pending", name: t("tally.pendingResolutions.pendingResolutionStatus")},
        {id: "decided", name: t("tally.pendingResolutions.pendingApplyStatus")},
        {id: "resolved", name: t("tally.pendingResolutions.resolvedStatus")},
    ]

    const isPendingSelected = selectedResolution?.status === "pending"
    const isDecidedSelected =
        isPendingSelected &&
        !!(selectedResolution?.contest_id && pendingSelections[selectedResolution.contest_id])
    const isResolvedEditing =
        !isPendingSelected && !!(selectedResolution && resolvedEditingIds[selectedResolution.id])
    const isResolvedRedecided =
        !isPendingSelected &&
        !!(selectedResolution?.contest_id && pendingSelections[selectedResolution.contest_id])
    const tiedVoteCount: number =
        (selectedResolution?.resolution_data as TallySessionResolutionData)?.vote_count ?? 0
    const tiedVotePercent: string | undefined =
        tiedVoteCount !== undefined && totalVotes !== undefined && totalVotes > 0
            ? ((tiedVoteCount / totalVotes) * 100).toFixed(1)
            : totalVotes === 0
              ? "0.0"
              : undefined
    const tiedCandidateNames = tiedCandidatesForSelected
        .map((c) => aliasRenderer(c.presentation))
        .join(", ")
    // Draft value: show the draft selection if set, otherwise fall back to the committed selection
    const currentDraftValue = selectedResolution?.contest_id
        ? (draftSelections[selectedResolution.contest_id] ??
          pendingSelections[selectedResolution.contest_id] ??
          "")
        : ""

    return (
        <Box sx={{mt: 2}}>
            <Box sx={{display: "flex", height: 413, gap: "29px"}}>
                {/* LEFT PANEL */}
                <Box
                    sx={{
                        width: "40%",
                        minWidth: 220,
                        border: "1px solid",
                        borderColor: "rgba(0,0,0,0.2)",
                        display: "flex",
                        flexDirection: "column",
                    }}
                >
                    {/* Left panel header */}
                    <Box
                        sx={{
                            display: "flex",
                            justifyContent: "space-between",
                            alignItems: "center",
                            px: 3,
                            height: 75,
                            borderBottom: "1px solid",
                            borderColor: "rgba(0,0,0,0.2)",
                            flexShrink: 0,
                        }}
                    >
                        <Typography
                            variant="h6"
                            fontWeight={500}
                            sx={{color: "#282828", fontSize: 20, lineHeight: "32px"}}
                        >
                            {t("tally.pendingResolutions.pendingResolutionsHeader")}{" "}
                            <Box component="span" sx={{color: "primary.main"}}>
                                ({latestPendingResolutions.length})
                            </Box>
                        </Typography>
                        <Button
                            variant="outlined"
                            color="primary"
                            size="small"
                            startIcon={<ViewColumnIcon />}
                            sx={{flexShrink: 0}}
                            onClick={(e) => setFilterAnchorEl(e.currentTarget)}
                        >
                            {t("tally.pendingResolutions.filter")}
                            {activeFilterCount > 0 ? ` (${activeFilterCount})` : ""}
                        </Button>

                        {/* Filter popover */}
                        <Popover
                            open={Boolean(filterAnchorEl)}
                            anchorEl={filterAnchorEl}
                            onClose={() => setFilterAnchorEl(null)}
                            anchorOrigin={{vertical: "bottom", horizontal: "right"}}
                            transformOrigin={{vertical: "top", horizontal: "right"}}
                        >
                            <Box
                                sx={{
                                    p: 2,
                                    width: 280,
                                    display: "flex",
                                    flexDirection: "column",
                                    gap: 0,
                                }}
                            >
                                <Autocomplete
                                    multiple
                                    size="small"
                                    options={relevantElections}
                                    getOptionLabel={(o) => aliasRenderer(o.presentation)}
                                    isOptionEqualToValue={(o, v) => o.id === v.id}
                                    value={relevantElections.filter((e) =>
                                        filterElections.includes(e.id)
                                    )}
                                    onChange={(_, v) => setFilterElections(v.map((e) => e.id))}
                                    renderInput={(params) => (
                                        <TextField
                                            {...params}
                                            label={t("tally.pendingResolutions.filterElection")}
                                        />
                                    )}
                                />

                                <Autocomplete
                                    multiple
                                    size="small"
                                    options={relevantContests}
                                    getOptionLabel={(o) => aliasRenderer(o.presentation)}
                                    isOptionEqualToValue={(o, v) => o.id === v.id}
                                    value={relevantContests.filter((c) =>
                                        filterContests.includes(c.id)
                                    )}
                                    onChange={(_, v) => setFilterContests(v.map((c) => c.id))}
                                    renderInput={(params) => (
                                        <TextField
                                            {...params}
                                            label={t("tally.pendingResolutions.filterContest")}
                                        />
                                    )}
                                />

                                <Autocomplete
                                    multiple
                                    size="small"
                                    options={relevantAreaOptions}
                                    getOptionLabel={(o) => o.name}
                                    isOptionEqualToValue={(o, v) => o.id === v.id}
                                    value={relevantAreaOptions.filter((a) =>
                                        filterAreas.includes(a.id)
                                    )}
                                    onChange={(_, v) => setFilterAreas(v.map((a) => a.id))}
                                    renderInput={(params) => (
                                        <TextField
                                            {...params}
                                            label={t("tally.pendingResolutions.filterArea")}
                                        />
                                    )}
                                />

                                <Autocomplete
                                    multiple
                                    size="small"
                                    options={statusFilterOptions}
                                    getOptionLabel={(o) => o.name}
                                    isOptionEqualToValue={(o, v) => o.id === v.id}
                                    value={statusFilterOptions.filter((o) =>
                                        filterStatuses.includes(o.id)
                                    )}
                                    onChange={(_, v) => setFilterStatuses(v.map((o) => o.id))}
                                    renderInput={(params) => (
                                        <TextField
                                            {...params}
                                            label={t("tally.pendingResolutions.filterStatusLabel")}
                                        />
                                    )}
                                />

                                <Button
                                    size="small"
                                    onClick={() => {
                                        setFilterElections([])
                                        setFilterContests([])
                                        setFilterAreas([])
                                        setFilterStatuses([])
                                    }}
                                    disabled={activeFilterCount === 0}
                                >
                                    {t("tally.pendingResolutions.clearFilters")}
                                </Button>
                            </Box>
                        </Popover>
                    </Box>

                    {/* Resolution list */}
                    <Box sx={{flex: 1, overflowY: "auto"}}>
                        {filteredResolutions.map((r) => renderResolutionItem(r))}
                    </Box>

                    {/* Apply resolutions button */}
                    <Box
                        sx={{
                            px: 3,
                            py: 1,
                            display: "flex",
                            justifyContent: "flex-end",
                            flexShrink: 0,
                        }}
                    >
                        <Button
                            variant="contained"
                            color="primary"
                            disabled={!canApply}
                            onClick={handleSubmit}
                            startIcon={
                                submitting ? <CircularProgress size={16} color="inherit" /> : null
                            }
                        >
                            {t("tally.pendingResolutions.applyResolutions")}
                        </Button>
                    </Box>
                </Box>

                {/* RIGHT PANEL */}
                <Box
                    sx={{
                        flex: 1,
                        border: "1px solid",
                        borderColor: "rgba(0,0,0,0.2)",
                        display: "flex",
                        flexDirection: "column",
                    }}
                >
                    {/* Right panel header */}
                    <Box
                        sx={{
                            display: "flex",
                            alignItems: "center",
                            px: 3,
                            height: 75,
                            borderBottom: "1px solid",
                            borderColor: "rgba(0,0,0,0.2)",
                            flexShrink: 0,
                        }}
                    >
                        <Typography
                            variant="h6"
                            fontWeight={500}
                            sx={{color: "#282828", fontSize: 20, lineHeight: "32px"}}
                        >
                            {t("tally.pendingResolutions.resolutionTitle")}
                        </Typography>
                    </Box>

                    {/* Right panel content */}
                    {selectedResolution ? (
                        <Box
                            sx={{
                                flex: 1,
                                px: 3,
                                pt: 1.5,
                                overflow: "auto",
                                display: "flex",
                                flexDirection: "column",
                                gap: 2,
                            }}
                        >
                            {isPendingSelected ? (
                                <Alert severity="info">
                                    <AlertTitle>
                                        {t("tally.pendingResolutions.tieInfoTitle", {
                                            round:
                                                selectedResolution.resolution_data?.round_number ??
                                                "?",
                                        })}
                                    </AlertTitle>
                                    {t("tally.pendingResolutions.tieInfoBody", {
                                        candidates: tiedCandidateNames || "?",
                                        votes: tiedVoteCount ?? "?",
                                        percent: tiedVotePercent ?? "?",
                                    })}
                                </Alert>
                            ) : (
                                <Alert severity="success">
                                    <AlertTitle>
                                        {t("tally.pendingResolutions.tallyResumedTitle")}
                                    </AlertTitle>
                                    {t("tally.pendingResolutions.tallyResumedBody", {
                                        date: new Date(
                                            selectedResolution.resolved_at
                                        ).toLocaleDateString(i18n.language),
                                        user: selectedResolution.resolved_by_user ?? "",
                                    })}
                                </Alert>
                            )}

                            <Box
                                sx={{
                                    border: "1px solid",
                                    borderColor: "rgba(0,0,0,0.2)",
                                    py: 1,
                                    px: 2,
                                }}
                            >
                                <Typography variant="body2" sx={{my: 0}}>
                                    {t("tally.pendingResolutions.selectCandidateToAdvance")}
                                </Typography>

                                <Autocomplete
                                    options={tiedCandidatesForSelected}
                                    getOptionLabel={(c) => aliasRenderer(c.presentation)}
                                    isOptionEqualToValue={(o, v) => o.id === v.id}
                                    disabled={
                                        !(
                                            (isPendingSelected && !isDecidedSelected) ||
                                            isResolvedEditing
                                        )
                                    }
                                    value={
                                        tiedCandidatesForSelected.find(
                                            (c) =>
                                                c.id ===
                                                (isPendingSelected ||
                                                isResolvedEditing ||
                                                isResolvedRedecided
                                                    ? currentDraftValue
                                                    : (resolvedCandidateForSelected?.id ?? ""))
                                        ) ?? null
                                    }
                                    onChange={(_, candidate) => {
                                        setDraftSelections((prev) => ({
                                            ...prev,
                                            [selectedResolution.contest_id!]: candidate?.id ?? "",
                                        }))
                                    }}
                                    renderInput={(params) => <TextField {...params} size="small" />}
                                />
                            </Box>
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

                    {/* Right panel footer — matches left panel footer height */}
                    <Box
                        sx={{
                            px: 3,
                            py: 1,
                            display: "flex",
                            justifyContent: "flex-end",
                            flexShrink: 0,
                        }}
                    >
                        {selectedResolution && (
                            <>
                                {isDecidedSelected ? (
                                    <Button
                                        variant="outlined"
                                        color="primary"
                                        onClick={handleUndoDecision}
                                    >
                                        {t("tally.pendingResolutions.undoResolution")}
                                    </Button>
                                ) : isPendingSelected ? (
                                    <Button
                                        variant="contained"
                                        color="primary"
                                        disabled={!currentDraftValue}
                                        onClick={handleSave}
                                    >
                                        {t("tally.pendingResolutions.save")}
                                    </Button>
                                ) : isResolvedEditing ? (
                                    <Button
                                        variant="contained"
                                        color="primary"
                                        disabled={!currentDraftValue}
                                        onClick={handleSave}
                                    >
                                        {t("tally.pendingResolutions.save")}
                                    </Button>
                                ) : isResolvedRedecided ? (
                                    <Button
                                        variant="outlined"
                                        color="primary"
                                        onClick={handleUndoDecision}
                                    >
                                        {t("tally.pendingResolutions.undoResolution")}
                                    </Button>
                                ) : (
                                    <Button
                                        variant="outlined"
                                        color="primary"
                                        onClick={handleStartResolvedEdit}
                                    >
                                        {t("tally.pendingResolutions.undoResolution")}
                                    </Button>
                                )}
                            </>
                        )}
                    </Box>
                </Box>
            </Box>
        </Box>
    )
}
