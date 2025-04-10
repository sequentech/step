// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, Button, CircularProgress, Typography, Alert} from "@mui/material"
import React, {useContext, useEffect, useMemo, useState, useRef} from "react"
import {useTranslation} from "react-i18next"
import {Dialog, IconButton, PageLimit, SelectElection, theme} from "@sequentech/ui-essentials"
import {
    stringToHtml,
    translateElection,
    EVotingStatus,
    IElectionEventStatus,
    isUndefined,
    IElectionStatus,
    IElection,
} from "@sequentech/ui-core"
import {AuthContext} from "../providers/AuthContextProvider"
import {faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
import {styled} from "@mui/material/styles"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {
    IBallotStyle,
    selectBallotStyleByElectionId,
    selectBallotStyleElectionIds,
    selectFirstBallotStyle,
} from "../store/ballotStyles/ballotStylesSlice"
import {selectElectionById, setElection, selectElectionIds} from "../store/elections/electionsSlice"
import {addCastVotes, selectCastVotesByElectionId} from "../store/castVotes/castVotesSlice"
import {useLocation, useNavigate, useParams} from "react-router-dom"
import {useMutation, useQuery} from "@apollo/client"
import {
    GetCastVotesQuery,
    GetSupportMaterialsQuery,
} from "../gql/graphql"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {GET_CAST_VOTES} from "../queries/GetCastVotes"
import {
    ElectionScreenErrorType,
    ElectionScreenMsgType,
    VotingPortalError,
    VotingPortalErrorType,
} from "../services/VotingPortalError"
import {
    IElectionEvent,
    selectElectionEventById,
    setElectionEvent,
} from "../store/electionEvents/electionEventsSlice"
import {TenantEventType} from ".."
import Stepper from "../components/Stepper"
import {selectBypassChooser, setBypassChooser} from "../store/extra/extraSlice"
import {
    updateBallotStyleAndSelection2,
} from "../services/BallotStyles"
import {fetchJson} from "../services/FetchS3BallotFiles"
import useUpdateTranslation from "../hooks/useUpdateTranslation"
import {GET_SUPPORT_MATERIALS} from "../queries/GetSupportMaterials"
import {GET_BALLOT_FILES_URLS} from "../queries/GetBallotFilesUrls"
import {setSupportMaterial} from "../store/supportMaterials/supportMaterialsSlice"

const StyledTitle = styled(Typography)`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
    font-size: 24px;
    font-weight: 500;
    line-height: 27px;
    margin-top: 20px;
    margin-bottom: 16px;
`

const ElectionContainer = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 30px;
    margin-bottom: 30px;
`

interface ElectionWrapperProps {
    electionId: string
    bypassChooser: boolean
    canVoteTest: boolean
}

const isElectionEventOpen = (electionEvent?: IElectionEvent): boolean => {
    return (
        ((electionEvent?.status as IElectionEventStatus | null)?.voting_status ??
            EVotingStatus.CLOSED) === EVotingStatus.OPEN
    )
}

const ElectionWrapper: React.FC<ElectionWrapperProps> = ({
    electionId,
    bypassChooser,
    canVoteTest,
}) => {
    const navigate = useNavigate()
    const location = useLocation()
    const {i18n} = useTranslation()

    const {tenantId, eventId} = useParams<TenantEventType>()
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    const election = useAppSelector(selectElectionById(electionId))
    const ballotStyle = useAppSelector(selectBallotStyleByElectionId(electionId))
    const castVotes = useAppSelector(selectCastVotesByElectionId(String(electionId)))
    const [visitedBypassChooser, setVisitedBypassChooser] = useState(false)
    const authContext = useContext(AuthContext)
    const isKiosk = authContext.isKiosk()

    if (!election) {
        throw new VotingPortalError(VotingPortalErrorType.INTERNAL_ERROR)
    }

    const electionStatus = election?.status as IElectionStatus | null
    const isVotingOpen =
        (electionStatus?.voting_status === EVotingStatus.OPEN ||
            (isKiosk && electionStatus?.kiosk_voting_status === EVotingStatus.OPEN)) &&
        isElectionEventOpen(electionEvent)
    const canVote = () => {
        if (!canVoteTest && !election.name?.includes("TEST")) {
            return false
        }

        if (ballotStyle?.ballot_eml.num_allowed_revotes === 0) {
            return true
        }

        return castVotes.length < (ballotStyle?.ballot_eml.num_allowed_revotes ?? 1) && isVotingOpen
    }

    const onClickToVote = () => {
        console.log("onClickToVote")
        if (!canVote() || !isElectionEventOpen(electionEvent)) {
            console.log("cannot vote")
            return
        }
        navigate(
            `/tenant/${tenantId}/event/${eventId}/election/${electionId}/start${location.search}`
        )
    }

    const handleClickBallotLocator = () => {
        navigate(`../election/${electionId}/ballot-locator${location.search}`)
    }

    useEffect(() => {
        if (visitedBypassChooser) {
            console.log("visitedBypassChooser")
            return
        }
        if (bypassChooser && ballotStyle) {
            console.log("setVisitedBypassChooser")
            setVisitedBypassChooser(true)
            onClickToVote()
        }
    }, [bypassChooser, visitedBypassChooser, setVisitedBypassChooser, ballotStyle])

    return (
        <SelectElection
            isActive={canVote()}
            isOpen={isVotingOpen}
            title={translateElection(election, "name", i18n.language) || "-"}
            hasVoted={castVotes.length > 0}
            onClickToVote={canVote() ? onClickToVote : undefined}
            onClickBallotLocator={handleClickBallotLocator}
            electionDates={ballotStyle?.ballot_eml?.election_dates}
        />
    )
}

const ElectionSelectionScreen: React.FC = () => {
    const {t} = useTranslation()
    const navigate = useNavigate()
    const location = useLocation()

    const {globalSettings} = useContext(SettingsContext)
    const {eventId, tenantId} = useParams<{eventId?: string; tenantId?: string}>()
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    useUpdateTranslation({electionEvent}) // Overwrite translations
    const electionIds = useAppSelector(selectElectionIds)
    const oneBallotStyle = useAppSelector(selectFirstBallotStyle)
    const dispatch = useAppDispatch()
    const [canVoteTest, setCanVoteTest] = useState<boolean>(true)
    const [testElectionId, setTestElectionId] = useState<string | null>(null)
    const castVotesTestElection = useAppSelector(
        selectCastVotesByElectionId(String(testElectionId || tenantId))
    )
    const [openChooserHelp, setOpenChooserHelp] = useState(false)
    const [isMaterialsActivated, setIsMaterialsActivated] = useState<boolean>(false)
    const [openDemoModal, setOpenDemoModal] = useState<boolean | undefined>(undefined)
    const isDemo = useMemo(() => {
        return oneBallotStyle?.ballot_eml.public_key?.is_demo
    }, [oneBallotStyle])
    const bypassChooser = useAppSelector(selectBypassChooser())
    const [errorMsg, setErrorMsg] = useState<VotingPortalErrorType | ElectionScreenErrorType>()
    const [alertMsg, setAlertMsg] = useState<ElectionScreenMsgType>()
    const [getBallotFilesUrls] = useMutation(GET_BALLOT_FILES_URLS)
    const urls = useRef<string[] | undefined>(undefined)
    const requestingS3Data = useRef<boolean>(false)
    const loadingS3Data = useRef<boolean>(true)
    const [dataBallotStyles, setDataBallotStyles] = useState<IBallotStyle[] | undefined>(undefined)
    const [dataElections, setDataElections] = useState<IElection[] | undefined>(undefined)
    const [dataElectionEvent, setDataElectionEvent] = useState<IElectionEvent | undefined>(
        undefined
    )

    // Materials
    const {
        data: dataMaterials,
        error: errorMaterials,
        loading: loadingMaterials,
    } = useQuery<GetSupportMaterialsQuery>(GET_SUPPORT_MATERIALS, {
        variables: {
            electionEventId: eventId || "",
            tenantId: tenantId || "",
        },
        skip: globalSettings.DISABLE_AUTH || !isMaterialsActivated, // Skip query if in demo mode
    })

    const {data: castVotes, error: errorCastVote} = useQuery<GetCastVotesQuery>(GET_CAST_VOTES, {
        skip: globalSettings.DISABLE_AUTH,
    })

    const handleNavigateMaterials = () => {
        navigate(`/tenant/${tenantId}/event/${eventId}/materials${location.search}`)
    }

    const hasNoElections = !loadingS3Data.current && dataElections?.length === 0
    const isPublished = useMemo(
        () => !!(dataElectionEvent?.status as IElectionEventStatus | undefined)?.is_published,
        [dataElectionEvent]
    )

    async function fetchS3Data() {
        try {
            const res = await getBallotFilesUrls({
                variables: {
                    eventId,
                },
            })
            // The election event file and the elections file are the first two urls followed by as any urls as ballot styles.
            let dataUrls = (res.data?.get_ballot_files_urls?.urls as string[]) ?? []
            if (!dataUrls || dataUrls.length < 3) {
                throw new Error("Not enough urls")
            }

            urls.current = dataUrls
            const contents = await Promise.all(
                dataUrls.map(async (url) => {
                    const content = await fetchJson(url)
                    return {url, content}
                })
            )
            if (!contents || contents.length < 3) {
                throw new Error("Not enough contents")
            }
            setDataElectionEvent(contents[0].content as IElectionEvent)
            setDataElections(contents[1].content as IElection[])
            let ballotStyles: IBallotStyle[] = []
            for (let i = 2; i < contents.length; i++) {
                const ballotStyle = contents[i].content as IBallotStyle
                ballotStyles.push(ballotStyle)
            }
            setDataBallotStyles(ballotStyles)
        } catch (error) {
            console.log("Error getting signed urls", error)
            setErrorMsg(t(`electionSelectionScreen.errors.${ElectionScreenErrorType.NETWORK}`))
            setAlertMsg(t(`electionSelectionScreen.alerts.${ElectionScreenMsgType.NOT_PUBLISHED}`))
            loadingS3Data.current = false
        }
    }

    useEffect(() => {
        if (!urls.current && eventId && !requestingS3Data.current) {
            requestingS3Data.current = true
            fetchS3Data()
        }
    }, [])

    useEffect(() => {
        if (!dataMaterials || globalSettings.DISABLE_AUTH || !isMaterialsActivated) {
            return
        }
        for (let material of dataMaterials.sequent_backend_support_material) {
            dispatch(setSupportMaterial(material))
        }
    }, [dataMaterials, globalSettings.DISABLE_AUTH, isMaterialsActivated])

    // Errors handling
    useEffect(() => {
        if (globalSettings.DISABLE_AUTH) {
            return
        }

        if (!dataElectionEvent) {
            setErrorMsg(
                t(`electionSelectionScreen.errors.${ElectionScreenErrorType.NO_ELECTION_EVENT}`)
            )
        } else if (!isPublished) {
            setAlertMsg(t(`electionSelectionScreen.alerts.${ElectionScreenMsgType.NOT_PUBLISHED}`))
        } else if (hasNoElections) {
            if (electionIds.length > 0) {
                setErrorMsg(
                    t(
                        `electionSelectionScreen.errors.${ElectionScreenErrorType.OBTAINING_ELECTION}`,
                        {electionIds: JSON.stringify(electionIds)}
                    )
                )
            } else {
                setAlertMsg(
                    t(`electionSelectionScreen.alerts.${ElectionScreenMsgType.NO_ELECTIONS}`)
                )
            }
        } else {
            setAlertMsg(undefined)
            setErrorMsg(undefined)
        }
    }, [errorCastVote, isPublished, hasNoElections, dataElectionEvent, globalSettings.DISABLE_AUTH])

    useEffect(() => {
        if (dataBallotStyles && dataBallotStyles.length > 0) {
            loadingS3Data.current = false
            try {
                updateBallotStyleAndSelection2((dataBallotStyles as IBallotStyle[]) || [], dispatch)
            } catch {
                setErrorMsg(
                    t(`electionSelectionScreen.errors.${ElectionScreenErrorType.BALLOT_STYLES_EML}`)
                )
            }
        }
    }, [globalSettings.DISABLE_AUTH, dataBallotStyles, dispatch])

    useEffect(() => {
        if (dataElections && dataElections.length > 0) {
            for (let election of dataElections) {
                dispatch(
                    setElection({
                        ...election,
                        image_document_id: "",
                        contests: [],
                        description: election.description ?? undefined,
                        alias: election.alias ?? undefined,
                    })
                )
            }

            let foundTestElection = dataElections.find((election) =>
                election.name?.includes("TEST")
            )

            if (foundTestElection) {
                setCanVoteTest(false)
            }

            setTestElectionId(foundTestElection?.id || null)
        }
    }, [dataElections, dispatch])

    useEffect(() => {
        if (!testElectionId) {
            return
        }
        setCanVoteTest(castVotesTestElection.length > 0)
    }, [castVotesTestElection, testElectionId, setCanVoteTest])

    useEffect(() => {
        const record = dataElectionEvent
        if (record) {
            dispatch(setElectionEvent(record))
        }
    }, [dataElectionEvent, dispatch])

    useEffect(() => {
        setIsMaterialsActivated(electionEvent?.presentation?.materials?.activated || false)
    }, [electionEvent?.presentation?.materials?.activated])

    useEffect(() => {
        if (castVotes?.sequent_backend_cast_vote) {
            dispatch(addCastVotes(castVotes.sequent_backend_cast_vote))
        }
    }, [castVotes, dispatch])

    useEffect(() => {
        const skipPolicy =
            oneBallotStyle?.ballot_eml.election_event_presentation?.skip_election_list ?? false
        console.log("skipPolicy", skipPolicy)
        const newBypassChooser =
            skipPolicy &&
            1 === electionIds.length &&
            !errorCastVote &&
            !isUndefined(castVotes) &&
            !!electionEvent &&
            !!dataElections

        if (newBypassChooser && !bypassChooser) {
            console.log("new baypass chooser", newBypassChooser)
            dispatch(setBypassChooser(newBypassChooser))
        }
    }, [
        castVotes,
        electionIds,
        errorCastVote,
        castVotes,
        electionEvent,
        dataElections,
        oneBallotStyle,
    ])

    useEffect(() => {
        if (isDemo && openDemoModal === undefined) {
            setOpenDemoModal(true)
        }
    }, [isDemo])

    if (loadingS3Data.current) return <CircularProgress />

    return (
        <PageLimit maxWidth="lg" className="election-selection-screen screen">
            <Box marginTop="48px">
                <Stepper selected={0} />
            </Box>

            <Box
                sx={{
                    display: "flex",
                    flexDirection: "row",
                    justifyContent: "space-between",
                    alignItems: "center",
                    minHeight: "100px",
                }}
            >
                <Box sx={{width: "100%"}}>
                    <StyledTitle variant="h1">
                        <Box>{t("electionSelectionScreen.title")}</Box>
                        <IconButton
                            icon={faCircleQuestion}
                            sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                            fontSize="16px"
                            onClick={() => setOpenChooserHelp(true)}
                        />
                        <Dialog
                            handleClose={() => setOpenChooserHelp(false)}
                            open={openChooserHelp}
                            title={t("electionSelectionScreen.chooserHelpDialog.title")}
                            ok={t("electionSelectionScreen.chooserHelpDialog.ok")}
                            variant="info"
                        >
                            {stringToHtml(t("electionSelectionScreen.chooserHelpDialog.content"))}
                        </Dialog>
                        <Dialog
                            handleClose={() => setOpenDemoModal(false)}
                            open={openDemoModal ? openDemoModal : false}
                            title={t("electionSelectionScreen.demoDialog.title")}
                            ok={t("electionSelectionScreen.demoDialog.ok")}
                            variant="warning"
                        >
                            {stringToHtml(t("electionSelectionScreen.demoDialog.content"))}
                        </Dialog>
                    </StyledTitle>
                    {errorMsg || alertMsg ? (
                        <Alert severity="warning">{errorMsg || alertMsg}</Alert>
                    ) : (
                        <Typography
                            variant="body1"
                            sx={{color: theme.palette.customGrey.contrastText}}
                        >
                            {stringToHtml(t("electionSelectionScreen.description"))}
                        </Typography>
                    )}
                </Box>
                {isMaterialsActivated ? (
                    <Button onClick={handleNavigateMaterials}>{t("materials.common.label")}</Button>
                ) : null}
            </Box>
            <ElectionContainer className="elections-list">
                {!hasNoElections ? (
                    electionIds.map((electionId) => (
                        <ElectionWrapper
                            electionId={electionId}
                            key={electionId}
                            bypassChooser={bypassChooser}
                            canVoteTest={canVoteTest}
                        />
                    ))
                ) : (
                    <Box sx={{margin: "auto"}}>
                        <Typography>{t("electionSelectionScreen.noResults")}</Typography>
                    </Box>
                )}
            </ElectionContainer>
        </PageLimit>
    )
}

export default ElectionSelectionScreen
