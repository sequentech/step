// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useMemo, useState} from "react"
import {useAtom} from "jotai"
import archivedElectionEventSelection from "@/atoms/archived-election-event-selection"
import {useLocation, useNavigate} from "react-router-dom"
import styled from "@emotion/styled"
import {Dialog, IconButton, adminTheme} from "@sequentech/ui-essentials"
import {
    Sequent_Backend_Election_Event,
    Sequent_Backend_Election,
    Sequent_Backend_Contest,
    Sequent_Backend_Candidate,
} from "@/gql/graphql"
import {
    ICandidatePresentation,
    IContestPresentation,
    IElectionEventPresentation,
    IContest,
    IElection,
    ICandidate,
} from "@sequentech/ui-core"
import SearchIcon from "@mui/icons-material/Search"
import {
    Button,
    Box,
    CircularProgress,
    TextField,
    MenuItem as MMenuItem,
    Menu as MMenu,
} from "@mui/material"
import {Menu, useGetOne, useNotify, useSidebarState} from "react-admin"
import {TreeMenu} from "./election-events/TreeMenu"
import {faPlusCircle} from "@fortawesome/free-solid-svg-icons"
import WebIcon from "@mui/icons-material/Web"
import {HorizontalBox} from "../../HorizontalBox"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTranslation} from "react-i18next"
import {IPermissions} from "../../../types/keycloak"
import {useTreeMenuData} from "./use-tree-menu-hook"
import {cloneDeep, debounce} from "lodash"
import {useUrlParams} from "@/hooks/useUrlParams"
import {useCreateElectionEventStore} from "@/providers/CreateElectionEventContextProvider"
import {useLazyQuery} from "@apollo/client"
import {
    FETCH_CANDIDATE_TREE,
    FETCH_CONTEST_TREE,
    FETCH_ELECTION_EVENTS_TREE,
    FETCH_ELECTIONS_TREE,
} from "@/queries/GetElectionEventsTree"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {sortCandidatesInContest, sortContestList, sortElectionList} from "@sequentech/ui-core"

const MenuItem = styled(Menu.Item)`
    color: ${adminTheme.palette.brandColor};

    &.RaMenuItemLink-active,
    .MuiIconButton-root {
        color: ${adminTheme.palette.brandColor};
    }
`

const StyledIconButton = styled(IconButton)`
    &:hover {
        padding: unset !important;
    }
    font-size: 1rem;
    line-height: 1.5rem;
`

const StyledButton = styled(Button)(({theme}) => ({
    "backgroundColor": "white",
    "color": theme.palette.brandColor,
    "border": "none",
    "boxShadow": "none",
    "&:hover": {
        color: theme.palette.brandColor,
        backgroundColor: "rgba(0, 0, 0, 0.04)",
        boxShadow: "none",
    },
    "&:active": {
        color: theme.palette.brandColor,
        backgroundColor: "rgba(0, 0, 0, 0.04)",
        border: "none",
        boxShadow: "none",
    },
    "&:focus": {
        color: theme.palette.brandColor,
        backgroundColor: "rgba(0, 0, 0, 0.04)",
        border: "none",
        boxShadow: "none",
    },
}))

const Container = styled("div")<{isActive?: boolean}>`
    background-color: ${({isActive}) => (isActive ? adminTheme.palette.green.light : "initial")};
`

const SideBarContainer = styled("div")`
    display: flex;
    align-items: center;
    background-color: white;
    padding-left: 1rem;
    padding-right: 1rem;
    & > *:not(:last-child) {
        margin-right: 1rem;
    }
`

export type ResourceName =
    | "sequent_backend_election_event"
    | "sequent_backend_election"
    | "sequent_backend_contest"
    | "sequent_backend_candidate"

export type EntityFieldName = "electionEvents" | "elections" | "contests" | "candidates"

export function mapDataChildren(key: ResourceName): EntityFieldName {
    const map: Record<ResourceName, EntityFieldName> = {
        sequent_backend_election_event: "electionEvents",
        sequent_backend_election: "elections",
        sequent_backend_contest: "contests",
        sequent_backend_candidate: "candidates",
    }
    return map[key]
}

export const TREE_RESOURCE_NAMES: Array<ResourceName> = [
    "sequent_backend_election_event",
    "sequent_backend_election",
    "sequent_backend_contest",
    "sequent_backend_candidate",
]

const ENTITY_FIELD_NAMES: Array<EntityFieldName> = [
    "electionEvents",
    "elections",
    "contests",
    "candidates",
]

type BaseType = {__typename: ResourceName; id: string; name: string; alias?: string}

export type CandidateType = BaseType & {
    __typename: "sequent_backend_candidate"
    election_event_id: string
    contest_id: string
    presentation: ICandidatePresentation
}

export type ContestType = BaseType &
    IContest & {
        __typename: "sequent_backend_contest"
        election_event_id: string
        election_id: string
        presentation: IContestPresentation
        candidates: Array<CandidateType>
    }

export type ElectionType = BaseType & {
    __typename: "sequent_backend_election"
    election_event_id: string
    image_document_id: string
    contests: Array<ContestType>
}

export type ElectionEventType = BaseType & {
    __typename: "sequent_backend_election_event"
    is_archived: boolean
    elections: Array<ElectionType>
    presentation: IElectionEventPresentation
}

export type DynEntityType = {
    electionEvents?: ElectionEventType[]
    elections?: ElectionType[]
    contests?: ContestType[]
    candidates?: CandidateType[]
}

export type DataTreeMenuType = (BaseType | CandidateType | ElectionType | ElectionEventType) & {
    active?: boolean
}

function filterTree(tree: any, filterName: string): any {
    if (Array.isArray(tree)) {
        return tree.map((subTree) => filterTree(subTree, filterName)).filter((v) => v !== null)
    } else if (typeof tree === "object" && tree !== null) {
        for (let key in tree) {
            if (
                tree.name?.toLowerCase().search(filterName.toLowerCase()) > -1 ||
                tree.alias?.toLowerCase().search(filterName.toLowerCase()) > -1
            ) {
                return tree
            } else if (ENTITY_FIELD_NAMES.includes(key as EntityFieldName)) {
                let filteredSubTree = filterTree(tree[key], filterName)
                if (filteredSubTree.length > 0) {
                    let filteredObj = {...tree}
                    filteredObj[key] = filteredSubTree
                    return filteredObj
                }
            }
        }
    }

    return null
}

export default function ElectionEvents() {
    const [tenantId] = useTenantStore()
    const [isOpenSidebar] = useSidebarState()
    const [instantSearchInput, setInstantSearchInput] = useState<string>("")
    const [searchInput, setSearchInput] = useState<string>("")
    const navigate = useNavigate()

    const [isArchivedElectionEvents, setArchivedElectionEvents] = useAtom(
        archivedElectionEventSelection
    )
    const {openCreateDrawer, openImportDrawer} = useCreateElectionEventStore()
    const {election_event_id, election_id, contest_id, candidate_id} = useUrlParams()
    const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null)
    const {data, loading, refetch: originalRefetch} = useTreeMenuData(isArchivedElectionEvents)

    const authContext = useContext(AuthContext)
    const showAddElectionEvent = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.ELECTION_EVENT_CREATE
    )
    const {t, i18n} = useTranslation()

    const [electionEventId, setElectionEventId] = useState<string | null>("")
    const [electionId, setElectionId] = useState<string | null>("")
    const [contestId, setContestId] = useState<string | null>("")
    const [candidateId, setCandidateId] = useState<string | null>("")

    const {getCandidateIdFlag} = useElectionEventTallyStore()

    const [openModal, setOpenModal] = React.useState(false)

    useEffect(() => {
        const referrer = document.referrer
        const baseUrl = window.location.origin
        const isFromMainPath =
            referrer === baseUrl ||
            referrer === `${baseUrl}/` ||
            window.location.href === baseUrl ||
            window.location.href === `${baseUrl}/`

        if (localStorage.getItem("has-token") && isFromMainPath) {
            setOpenModal(true)
        }
    }, [])

    /**
     * Hooks to load data for entities
     */
    const {data: electionEventData, refetch: electionEventDataRefetch} =
        useGetOne<Sequent_Backend_Election_Event>(
            "sequent_backend_election_event",
            {id: election_event_id},
            {
                enabled: !!election_event_id,
                onSuccess: (data) => {
                    setElectionEventId(data.id)
                },
            }
        )
    const {refetch: refetchElectionData} = useGetOne<Sequent_Backend_Election>(
        "sequent_backend_election",
        {id: election_id},
        {
            enabled: !!election_id,
            onSuccess: (data) => {
                setElectionEventId(data.election_event_id)
                setElectionId(data.id)
                setContestId("")
                setCandidateId("")
            },
        }
    )
    const {refetch: refetchContestData} = useGetOne<Sequent_Backend_Contest>(
        "sequent_backend_contest",
        {id: contest_id || contestId},
        {
            enabled: !!contest_id,
            onSuccess: (data) => {
                setElectionId(data.election_id)
                setElectionEventId(data.election_event_id)
                setContestId(data.id)
                setCandidateId("")
            },
        }
    )
    const {refetch: candidateData} = useGetOne<Sequent_Backend_Candidate>(
        "sequent_backend_candidate",
        {id: candidate_id},
        {
            enabled: !!candidate_id,
            onSuccess: (data) => {
                setContestId(data.contest_id)
                setElectionEventId(data.election_event_id)
                setCandidateId(data.id)
            },
        }
    )
    // Get subtrees
    const [
        getElectionEventTree,
        {data: electionEventTreeData, refetch: _electionEventTreeRefetch},
    ] = useLazyQuery(FETCH_ELECTION_EVENTS_TREE)

    const [getElectionTree, {data: electionTreeData, refetch: _electionTreeRefetch}] =
        useLazyQuery(FETCH_ELECTIONS_TREE)

    const [getContestTree, {data: contestTreeData, refetch: _contestTreeRefetch}] =
        useLazyQuery(FETCH_CONTEST_TREE)

    const [getCandidateTree, {data: candidateTreeData, refetch: _candidateTreeRefetch}] =
        useLazyQuery(FETCH_CANDIDATE_TREE)

    // Wrapper refetch functions: only call the internal refetch if variables
    // are set
    const electionEventTreeRefetch = () => {
        if (tenantId && electionEventId) {
            getElectionEventTree({
                variables: {
                    tenantId,
                    isArchived: isArchivedElectionEvents,
                },
            })
        }
    }

    const electionTreeRefetch = () => {
        if (tenantId && electionEventId) {
            _electionTreeRefetch({
                variables: {
                    tenantId,
                    electionEventId,
                },
            })
        }
    }

    const contestTreeRefetch = () => {
        if (tenantId && electionId) {
            _contestTreeRefetch({
                variables: {
                    tenantId,
                    electionId,
                },
            })
        }
    }

    const candidateTreeRefetch = () => {
        if (tenantId && contestId) {
            _candidateTreeRefetch({
                variables: {
                    tenantId,
                    contestId,
                },
            })
        }
    }

    // Instead of setting variables in the options, we now call the lazy queries
    // only when variables exist.
    // Force reload election event data when tenant ID changes or component mounts
    useEffect(() => {
        if (tenantId) {
            getElectionEventTree({
                variables: {
                    tenantId,
                    isArchived: isArchivedElectionEvents,
                },
            })

            // Also reload other data that might depend on tenant ID
            originalRefetch()
        }
    }, [tenantId, isArchivedElectionEvents, getElectionEventTree, originalRefetch])

    useEffect(() => {
        if (tenantId && electionEventId) {
            getElectionTree({
                variables: {
                    tenantId,
                    electionEventId,
                },
            })
        }
    }, [tenantId, electionEventId, getElectionTree])

    useEffect(() => {
        if (tenantId && electionId) {
            getContestTree({
                variables: {
                    tenantId,
                    electionId,
                },
            })
        }
    }, [tenantId, electionId, getContestTree])

    useEffect(() => {
        if (tenantId && contestId) {
            getCandidateTree({
                variables: {
                    tenantId,
                    contestId,
                },
            })
        }
    }, [tenantId, contestId, candidateId, getCandidateTree])

    const location = useLocation()
    const isElectionEventActive = TREE_RESOURCE_NAMES.some(
        (route) => location.pathname.search(route) > -1
    )

    useEffect(() => {
        const callerPath = location.pathname.split("/")[1]

        if (callerPath === "sequent_backend_election") {
            electionTreeRefetch()
            refetchElectionData()
        } else if (callerPath === "sequent_backend_contest") {
            contestTreeRefetch()
            refetchContestData()
        } else if (callerPath === "sequent_backend_candidate") {
            candidateData()
            candidateTreeRefetch()
        } else {
            // do nothing
        }
    }, [location])

    useEffect(() => {
        const hasCandidateIdFlag = location.pathname
            .toLowerCase()
            .includes("/sequent_backend_candidate/")

        if (location.pathname.split("/").length > 2 && hasCandidateIdFlag) {
            if (getCandidateIdFlag() === location.pathname.split("/")[2]) {
                refetchContestData()

                setTimeout(() => {
                    candidateTreeRefetch()
                }, 400)
            }
        }
    }, [getCandidateIdFlag, candidate_id])

    useEffect(() => {
        if (!electionEventData || !electionEventId) return
        if (electionEventData?.id === electionEventId && electionEventData?.is_archived) {
            setArchivedElectionEvents(electionEventData?.is_archived ?? false)
        }
    }, [electionEventId, electionEventData, setArchivedElectionEvents])

    let resultData = {...data}

    function changeArchiveSelection(val: number) {
        setArchivedElectionEvents(val === 1)
        resultData = {}
        navigate("/sequent_backend_election_event/")
    }

    const handleOpenCreateElectionEventMenu = (e: React.MouseEvent<HTMLElement>) => {
        setAnchorEl(e.currentTarget)
    }

    const handleOpenCreateElectionEventForm = (e: React.MouseEvent<HTMLElement>) => {
        setAnchorEl(null)
        openCreateDrawer?.()
    }

    const handleOpenImportElectionEventForm = (e: React.MouseEvent<HTMLElement>) => {
        setAnchorEl(null)
        openImportDrawer?.()
    }

    const transformElectionsForSort = (elections: ElectionType[]): IElection[] => {
        return elections.map((election) => {
            return {
                ...election,
                tenant_id: tenantId || "",
                image_document_id: election.image_document_id ?? "",
                contests: [],
            }
        })
    }

    const transformContestsForSort = (contests: ContestType[]): IContest[] => {
        return contests.map((contest): IContest => {
            return {
                ...contest,
                tenant_id: tenantId || "",
                candidates: [],
                max_votes: 0,
                min_votes: 0,
                winning_candidates_num: 0,
                is_encrypted: false,
            }
        })
    }

    const transformCandidatesForSort = (candidates: ICandidate[]): ICandidate[] => {
        return candidates.map((candidate: ICandidate) => {
            return {
                ...candidate,
                id: candidate.id,
                election_id: electionId || "",
                tenant_id: tenantId || "",
            }
        })
    }

    if (!loading && data && data.sequent_backend_election_event) {
        resultData = {
            electionEvents: [...(data.sequent_backend_election_event ?? [])],
        }
    }

    let finalResultData = useMemo(() => {
        return {
            electionEvents: cloneDeep(resultData?.electionEvents ?? [])?.map(
                (electionEvent: ElectionEventType) => {
                    const electionOrderType = electionEvent?.presentation?.elections_order
                    return {
                        ...electionEvent,
                        ...(electionEvent.id === electionEventId
                            ? {
                                  active: true,
                                  elections:
                                      sortElectionList(
                                          transformElectionsForSort(
                                              electionTreeData?.sequent_backend_election || []
                                          ),
                                          electionOrderType
                                      )?.map?.((e: any) => {
                                          const contestOrderType = e?.presentation?.contests_order
                                          return {
                                              ...e,
                                              ...(e.id === electionId
                                                  ? {
                                                        active: true,
                                                        contests:
                                                            sortContestList(
                                                                transformContestsForSort(
                                                                    contestTreeData?.sequent_backend_contest ||
                                                                        []
                                                                ),
                                                                contestOrderType
                                                            )?.map?.((c: any) => {
                                                                let orderType =
                                                                    c.presentation?.candidates_order
                                                                return {
                                                                    ...c,
                                                                    ...(c.id === contestId
                                                                        ? {
                                                                              active: true,
                                                                              candidates:
                                                                                  sortCandidatesInContest(
                                                                                      transformCandidatesForSort(
                                                                                          candidateTreeData?.sequent_backend_candidate ||
                                                                                              []
                                                                                      ),
                                                                                      orderType
                                                                                  )
                                                                                      ?.map(
                                                                                          (
                                                                                              ca: any
                                                                                          ) => ({
                                                                                              ...ca,
                                                                                              active:
                                                                                                  ca.id ===
                                                                                                  candidateId,
                                                                                          })
                                                                                      )
                                                                                      ?.map(
                                                                                          (
                                                                                              ca: any
                                                                                          ) => ({
                                                                                              ...ca,
                                                                                              active:
                                                                                                  ca.id ===
                                                                                                  candidateId,
                                                                                          })
                                                                                      ) ?? [],
                                                                          }
                                                                        : {
                                                                              active: false,
                                                                              candidates:
                                                                                  sortCandidatesInContest(
                                                                                      transformCandidatesForSort(
                                                                                          candidateTreeData?.sequent_backend_candidate ||
                                                                                              []
                                                                                      ),
                                                                                      orderType
                                                                                  )?.map(
                                                                                      (
                                                                                          ca: any
                                                                                      ) => ({
                                                                                          ...ca,
                                                                                          active: false,
                                                                                      })
                                                                                  ) ?? [],
                                                                          }),
                                                                }
                                                            }) ?? [],
                                                    }
                                                  : {active: false, contests: []}),
                                          }
                                      }) || [],
                              }
                            : {active: false, elections: []}),
                    }
                }
            ),
        }
    }, [
        electionEventId,
        electionId,
        contestId,
        candidateId,
        electionEventTreeData,
        electionTreeData,
        contestTreeData,
        candidateTreeData,
        data,
        searchInput,
    ])

    finalResultData = filterTree(finalResultData, searchInput)

    const reloadTreeMenu = () => {
        candidateTreeRefetch()
        contestTreeRefetch()
        electionTreeRefetch()
        electionEventTreeRefetch()

        originalRefetch()
        navigate("/sequent_backend_election_event/")
    }

    const debouncedSearchChange = useMemo(() => {
        const debouncedFn = debounce((value) => {
            console.log(`edu: debounce: ${value}`)
            // Expensive operation or API call
            setSearchInput(value)
        }, 300)

        return (value: any) => {
            // Update state immediately
            setInstantSearchInput(value)
            debouncedFn(value)
        }
    }, [])

    const treeMenu = loading ? (
        <CircularProgress />
    ) : (
        <TreeMenu
            data={finalResultData}
            treeResourceNames={TREE_RESOURCE_NAMES}
            isArchivedElectionEvents={isArchivedElectionEvents}
            onArchiveElectionEventsSelect={changeArchiveSelection}
            reloadTree={reloadTreeMenu}
        />
    )

    return (
        <>
            <Container isActive={isElectionEventActive}>
                <HorizontalBox
                    sx={{
                        alignItems: "center",
                        paddingRight: i18n.dir(i18n.language) === "rtl" ? 0 : "16px",
                        paddingLeft: i18n.dir(i18n.language) === "rtl" ? "32px" : 0,
                    }}
                >
                    <MenuItem
                        to="/sequent_backend_election_event"
                        primaryText={isOpenSidebar && t("sideMenu.electionEvents")}
                        leftIcon={<WebIcon sx={{color: adminTheme.palette.brandColor}} />}
                        sx={{
                            flexGrow: 2,
                            paddingLeft: i18n.dir(i18n.language) === "rtl" ? 0 : "16px",
                            paddingRight: i18n.dir(i18n.language) === "rtl" ? "16px" : 0,
                        }}
                    />
                    {isOpenSidebar && showAddElectionEvent ? (
                        <StyledIconButton
                            onClick={handleOpenCreateElectionEventMenu}
                            className="election-event-create-button"
                            icon={faPlusCircle}
                            size="xs"
                        />
                    ) : null}
                </HorizontalBox>

                {isOpenSidebar && (
                    <>
                        <SideBarContainer dir={i18n.dir(i18n.language)}>
                            <TextField
                                dir={i18n.dir(i18n.language)}
                                label={t("sideMenu.search")}
                                size="small"
                                value={instantSearchInput}
                                onChange={(e) => debouncedSearchChange(e.target.value)}
                            />
                            <SearchIcon />
                        </SideBarContainer>

                        {treeMenu}
                    </>
                )}
            </Container>

            <MMenu
                id="treemenu-create-election-event-menu"
                anchorEl={anchorEl}
                anchorOrigin={{
                    vertical: "bottom",
                    horizontal: "right",
                }}
                keepMounted
                transformOrigin={{
                    vertical: "top",
                    horizontal: "left",
                }}
                open={Boolean(anchorEl)}
                onClose={() => setAnchorEl(null)}
            >
                <MMenuItem
                    className="menu-sidebar-item"
                    onClick={handleOpenCreateElectionEventForm}
                >
                    <Box
                        sx={{
                            textOverflow: "ellipsis",
                            whiteSpace: "nowrap",
                            overflow: "hidden",
                        }}
                    >
                        <span className="help-menu-item" title={"Create Election Event"}>
                            {t("createResource.electionEvent")}
                        </span>
                    </Box>
                </MMenuItem>
                <MMenuItem
                    className="menu-sidebar-item"
                    onClick={handleOpenImportElectionEventForm}
                >
                    <Box
                        sx={{
                            textOverflow: "ellipsis",
                            whiteSpace: "nowrap",
                            overflow: "hidden",
                        }}
                    >
                        <span className="help-menu-item" title={"Import Election Event"}>
                            {t("electionEventScreen.import.eetitle")}
                        </span>
                    </Box>
                </MMenuItem>
            </MMenu>
            <Dialog
                variant="info"
                hasCloseButton={false}
                open={openModal}
                cancel={t("common.label.logout")}
                ok={t("common.label.continue")}
                title={t("common.label.warning")}
                handleClose={(result: boolean) => {
                    if (result) {
                        localStorage.removeItem("has-token")
                        setOpenModal(false)
                    } else {
                        authContext.logout()
                    }
                }}
            >
                {t("common.message.continueOrLogout")}
            </Dialog>
        </>
    )
}
