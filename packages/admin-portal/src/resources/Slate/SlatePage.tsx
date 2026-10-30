// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useCallback, useContext, useMemo} from "react"
import {useGetList, useGetOne, useSidebarState} from "react-admin"
import {useNavigate, useParams} from "react-router-dom"
import {useTranslation} from "react-i18next"
import {Box, CircularProgress, Typography} from "@mui/material"
import {v4 as uuidv4} from "uuid"
import {
    ContestsOrder,
    IContest,
    IContestPresentation,
    IElectionPresentation,
    getSlateName,
    sortContestList,
    translateFromPresentation,
} from "@sequentech/ui-core"
import {
    Sequent_Backend_Candidate,
    Sequent_Backend_Contest,
    Sequent_Backend_Election,
} from "@/gql/graphql"
import ElectionHeader from "@/components/ElectionHeader"
import {Tabs} from "@/components/Tabs"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {IPermissions} from "@/types/keycloak"
import {readSlatesConfig} from "@/utils/slates"
import {SlateCandidateOption, SlateDataForm} from "./SlateDataForm"
import {slatePath} from "./slatePaths"

const ALL_RECORDS = {page: 1, perPage: 9999}

type SlateDataTabProps = Omit<React.ComponentProps<typeof SlateDataForm>, "candidates"> & {
    candidates: Array<Sequent_Backend_Candidate> | undefined
}

const SlateDataTab: React.FC<SlateDataTabProps> = ({candidates, ...props}) =>
    candidates ? <SlateDataForm candidates={candidates} {...props} /> : <CircularProgress />

const toSortableContest = (contest: Sequent_Backend_Contest): IContest => ({
    id: contest.id,
    tenant_id: contest.tenant_id,
    election_event_id: contest.election_event_id,
    election_id: contest.election_id,
    max_votes: contest.max_votes ?? 1,
    min_votes: contest.min_votes ?? 0,
    winning_candidates_num: contest.winning_candidates_num ?? 1,
    is_encrypted: contest.is_encrypted ?? false,
    candidates: [],
    presentation: (contest.presentation as IContestPresentation | null) ?? undefined,
})

export const SlatePage: React.FC = () => {
    const {id: electionId, slateId: slateIdParam} = useParams()
    const {t, i18n} = useTranslation()
    const navigate = useNavigate()
    const [tenantId] = useTenantStore()
    const [isSidebarOpen] = useSidebarState()
    const authContext = useContext(AuthContext)
    const aliasRenderer = useAliasRenderer()

    const newSlateId = useMemo(() => uuidv4(), [electionId])
    const slateId = slateIdParam ?? newSlateId

    const {data: election, isLoading: isElectionLoading} = useGetOne<Sequent_Backend_Election>(
        "sequent_backend_election",
        {id: electionId},
        {enabled: !!electionId}
    )
    const {data: contests} = useGetList<Sequent_Backend_Contest>(
        "sequent_backend_contest",
        {filter: {tenant_id: tenantId, election_id: electionId}, pagination: ALL_RECORDS},
        {enabled: !!tenantId && !!electionId}
    )
    const {data: candidates} = useGetList<Sequent_Backend_Candidate>(
        "sequent_backend_candidate",
        {
            filter: {tenant_id: tenantId, election_event_id: election?.election_event_id},
            pagination: ALL_RECORDS,
        },
        {enabled: !!tenantId && !!election?.election_event_id}
    )

    const canView = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ELECTION_DATA_TAB
    )
    const canEdit = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ELECTION_WRITE
    )
    const hasPermissionLabel =
        !election?.permission_label ||
        !authContext.permissionLabels ||
        authContext.permissionLabels.includes(election.permission_label)

    const presentation = election?.presentation as IElectionPresentation | null | undefined
    const defaultLanguage = presentation?.language_conf?.default_language_code ?? "en"
    const config = useMemo(() => readSlatesConfig(election?.annotations), [election?.annotations])
    const slate = slateIdParam ? config?.slates.find(({id}) => id === slateIdParam) : undefined

    const orderedContests = useMemo(() => {
        const byId = new Map((contests ?? []).map((contest) => [contest.id, contest]))
        return sortContestList(
            (contests ?? []).map(toSortableContest),
            presentation?.contests_order ?? ContestsOrder.ALPHABETICAL
        )
            .map(({id}) => byId.get(id))
            .filter((contest): contest is Sequent_Backend_Contest => !!contest)
    }, [contests, presentation?.contests_order])

    const candidateOptions = useMemo(
        (): Array<SlateCandidateOption> =>
            (candidates ?? [])
                .map((candidate) => ({
                    id: candidate.id,
                    contest_id: candidate.contest_id,
                    presentation: candidate.presentation,
                    label:
                        translateFromPresentation(candidate, "name", i18n.language) || candidate.id,
                }))
                .sort((a, b) => a.label.localeCompare(b.label, i18n.language)),
        [candidates, i18n.language]
    )

    const contestName = useCallback(
        (contest: Sequent_Backend_Contest) => aliasRenderer(contest, defaultLanguage),
        [aliasRenderer, defaultLanguage]
    )

    const onSaved = useCallback(
        (savedId: string) => {
            if (electionId) {
                navigate(slatePath(electionId, savedId), {replace: true})
            }
        },
        [electionId, navigate]
    )

    if (isElectionLoading) {
        return <CircularProgress />
    }

    const isMissing = !!slateIdParam && !slate
    if (!election || !canView || !hasPermissionLabel || isMissing) {
        return (
            <ResourceListStyles.EmptyBox>
                <Typography variant="h4" paragraph>
                    {isMissing
                        ? t("slateScreen.common.notFound")
                        : t("electionScreen.common.noPermission")}
                </Typography>
            </ResourceListStyles.EmptyBox>
        )
    }

    return (
        <Box
            sx={{
                maxWidth: `calc(100vw - ${isSidebarOpen ? "352px" : "96px"})`,
                bgcolor: "background.paper",
            }}
            className="slate-box"
        >
            <ElectionHeader
                title={
                    slate
                        ? getSlateName(slate, i18n.language, defaultLanguage)
                        : String(t("slateScreen.common.newSlate"))
                }
                subtitle="slateScreen.common.subtitle"
            />
            <Box sx={{bgcolor: "background.paper"}}>
                <Tabs
                    elements={[
                        {
                            label: t("slateScreen.tabs.data"),
                            component: SlateDataTab,
                            props: {
                                election,
                                slateId,
                                slate,
                                config,
                                contests: orderedContests,
                                contestName,
                                candidates: contests ? candidates : undefined,
                                candidateOptions,
                                defaultLanguage,
                                canEdit,
                                onSaved,
                            },
                        },
                    ]}
                />
            </Box>
        </Box>
    )
}
