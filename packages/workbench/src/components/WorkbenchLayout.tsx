// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useEffect, useMemo, useState, useSyncExternalStore} from "react"
import {matchPath, Outlet, useLocation} from "react-router"
import {Box, Paper, Tab, Tabs, Typography} from "@mui/material"
import {useWasm, WasmStatus, isPreferential, type ICountingAlgorithm} from "@sequentech/ui-core"
import {SCENARIOS, type PreviewContest} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {validateSelection} from "voting-portal/src/preview/ballotPipeline"
import {
    EVENT_ROUTE,
    PREVIEW_SCREENS,
    previewDeepLink,
    previewScreenPath,
    previewStoryId,
    previewTarget,
} from "voting-portal/src/preview/screens"
import {VoterPreview} from "voting-portal/src/preview/VoterPreview"
import {store} from "voting-portal/src/store/store"
import sequentCore from "virtual:workbench/sequent-core"
import {boundsIssue, contestPolicies, type PolicyOverrides} from "../policies"
import {
    ActionType,
    useCurrentScreen,
    useWorkbench,
    useWorkbenchActions,
    WorkbenchEventKind,
} from "../state"
import {BLOCKED_REQUEST_EVENT} from "../networkGuard"
import {PipelinePanel, WASM_RUNNER} from "./PipelinePanel"
import {PolicyPanel, type PolicyPanelContest} from "./PolicyPanel"
import {ScenarioPicker} from "./ScenarioPicker"
import {ScreenNav} from "./ScreenNav"
import {StateInspector} from "./StateInspector"

export enum SidePanel {
    POLICIES = "policies",
    PIPELINE = "pipeline",
    INSPECTOR = "inspector",
}

const STORYBOOK_URL = import.meta.env.WORKBENCH_STORYBOOK_URL ?? "http://localhost:6007"
const ELECTION_ROUTE = `${EVENT_ROUTE}/election/:electionId/*`

const contestName = (contest: PreviewContest) => String(contest.name ?? contest.id)

const policyContests = (
    contests: PreviewContest[],
    overrides: PolicyOverrides,
    wasmReady: boolean
): PolicyPanelContest[] =>
    contests.map((contest) => ({
        id: contest.id,
        name: contestName(contest),
        preferential: wasmReady && isPreferential(contest.counting_algorithm as ICountingAlgorithm),
        baseline: {
            ...contestPolicies(contest),
            min_votes: contest.min_votes,
            max_votes: contest.max_votes,
        },
        overrides: overrides[contest.id] ?? {},
        boundsIssue: overrides[contest.id] && boundsIssue(contest, overrides[contest.id]),
    }))

/** The workbench controls around the production voter screens. */
export const WorkbenchLayout: React.FC = () => {
    const {state, dispatch} = useWorkbench()
    const actions = useWorkbenchActions()
    const screen = useCurrentScreen()
    const {pathname} = useLocation()
    const production = useSyncExternalStore(store.subscribe, store.getState)
    const wasmReady = useWasm().status === WasmStatus.READY
    const [panel, setPanel] = useState(SidePanel.POLICIES)

    useEffect(() => {
        const blocked = (event: Event) =>
            dispatch({
                type: ActionType.LOG,
                kind: WorkbenchEventKind.NETWORK,
                message: `Blocked ${(event as CustomEvent<string>).detail}`,
            })
        window.addEventListener(BLOCKED_REQUEST_EVENT, blocked)
        return () => window.removeEventListener(BLOCKED_REQUEST_EVENT, blocked)
    }, [dispatch])

    const onLogout = useCallback(
        (redirectUrl?: string) =>
            dispatch({
                type: ActionType.LOG,
                kind: WorkbenchEventKind.LOGOUT,
                message: `The portal logged the voter out${redirectUrl ? ` to ${redirectUrl}` : ""}`,
            }),
        [dispatch]
    )

    const target = previewTarget(state.session.snapshot)
    const electionId = matchPath(ELECTION_ROUTE, pathname)?.params.electionId ?? target.electionId
    const ballotStyle = electionId ? production.ballotStyles[electionId] : undefined
    const selection = electionId ? production.ballotSelections[electionId] : undefined
    const input = useMemo(
        () => (ballotStyle && selection ? {ballotStyle, selection} : undefined),
        [ballotStyle, selection]
    )
    const validation = useMemo(() => {
        if (!input || !wasmReady) return undefined
        try {
            return validateSelection(input.ballotStyle, input.selection)
        } catch (error) {
            return error instanceof Error ? error : new Error(String(error))
        }
    }, [input, wasmReady])

    const baseStyle = state.base.preview.ballot_styles.find(
        ({area_id, election_id}) => area_id === state.base.areaId && election_id === electionId
    )
    const contests = baseStyle?.contests ?? []
    const contestNames = Object.fromEntries(
        contests.map((contest) => [contest.id, contestName(contest)])
    )
    const storyId = previewStoryId(state.base.scenarioId, screen)

    return (
        <Box
            className="workbench"
            sx={{
                display: "grid",
                gridTemplateColumns: "minmax(0, 1fr) 460px",
                gridTemplateRows: "auto minmax(0, 1fr)",
                height: "100vh",
                bgcolor: "background.default",
            }}
        >
            <Box
                component="header"
                sx={{
                    gridColumn: "1 / -1",
                    p: 1.5,
                    display: "flex",
                    flexDirection: "column",
                    gap: 1,
                    bgcolor: "background.paper",
                    borderBottom: 1,
                    borderColor: "divider",
                }}
            >
                <Typography variant="h6" component="h1" sx={{fontSize: 16}}>
                    Voting workbench
                </Typography>
                <ScenarioPicker
                    scenarios={SCENARIOS}
                    snapshot={state.base}
                    onSelect={(id) => actions.openScenario(id)}
                    onImport={(file) => void actions.importSnapshot(file)}
                    onExport={actions.exportSnapshot}
                    onReset={actions.reset}
                    importIssues={state.importIssues}
                />
                <ScreenNav
                    screens={PREVIEW_SCREENS.map((option) => ({
                        screen: option,
                        available: previewScreenPath(target, option) !== undefined,
                    }))}
                    current={screen}
                    onOpen={actions.openScreen}
                    links={{
                        deepLink: previewDeepLink(state.base.scenarioId, screen),
                        storyId,
                        storyUrl: `${STORYBOOK_URL}/?path=/story/${storyId}`,
                    }}
                />
            </Box>
            <Box component="main" sx={{overflow: "auto", p: 2}}>
                <Paper
                    variant="outlined"
                    className="portal-preview"
                    aria-label="Voting portal preview"
                    component="section"
                    sx={{minHeight: "100%", overflow: "hidden"}}
                >
                    <VoterPreview session={state.session} onLogout={onLogout}>
                        <Outlet />
                    </VoterPreview>
                </Paper>
            </Box>
            <Box
                component="aside"
                aria-label="Workbench panels"
                sx={{
                    overflow: "auto",
                    borderLeft: 1,
                    borderColor: "divider",
                    bgcolor: "background.paper",
                }}
            >
                <Tabs
                    value={panel}
                    onChange={(_, value: SidePanel) => setPanel(value)}
                    variant="fullWidth"
                    sx={{borderBottom: 1, borderColor: "divider"}}
                >
                    <Tab value={SidePanel.POLICIES} label="Policies" />
                    <Tab value={SidePanel.PIPELINE} label="Pipeline" />
                    <Tab value={SidePanel.INSPECTOR} label="Inspector" />
                </Tabs>
                <Box sx={{p: 2}}>
                    {panel === SidePanel.POLICIES ? (
                        <PolicyPanel
                            contests={policyContests(contests, state.overrides, wasmReady)}
                            onChange={actions.setOverride}
                            onClear={actions.clearOverrides}
                        />
                    ) : null}
                    {panel === SidePanel.PIPELINE ? (
                        <PipelinePanel runner={WASM_RUNNER} input={input} wasmReady={wasmReady} />
                    ) : null}
                    {panel === SidePanel.INSPECTOR ? (
                        <StateInspector
                            snapshot={state.session.snapshot}
                            overrides={state.overrides}
                            location={pathname}
                            production={production}
                            validation={validation}
                            contestNames={contestNames}
                            events={state.events}
                            sequentCore={sequentCore}
                        />
                    ) : null}
                </Box>
            </Box>
        </Box>
    )
}
