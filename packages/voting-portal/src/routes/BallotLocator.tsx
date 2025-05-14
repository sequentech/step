// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useState, useRef} from "react"
import {useTranslation} from "react-i18next"
import {
    BreadCrumbSteps,
    PageLimit,
    theme,
    Icon,
    InfoDataBox,
    IconButton,
    Dialog,
} from "@sequentech/ui-essentials"
import {stringToHtml} from "@sequentech/ui-core"
import {Box, TextField, Typography, Button, Stack} from "@mui/material"
import {styled} from "@mui/material/styles"
import Tabs from '@mui/material/Tabs';
import Tab from '@mui/material/Tab';
import {Link, useLocation, useNavigate, useParams} from "react-router-dom"
import {GET_CAST_VOTE} from "../queries/GetCastVote"
import {useQuery, useMutation} from "@apollo/client"
import {
    GetBallotStylesQuery,
    GetCastVoteQuery,
    GetElectionEventQuery,
    ListCastVoteMessagesMutation,
} from "../gql/graphql"
import {faAngleLeft, faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
import {GET_BALLOT_STYLES} from "../queries/GetBallotStyles"
import {LIST_CAST_VOTE_MESSAGES} from "../queries/listCastVoteMessages"
import {updateBallotStyleAndSelection} from "../services/BallotStyles"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {selectFirstBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import useLanguage from "../hooks/useLanguage"
import {SettingsContext} from "../providers/SettingsContextProvider"
import useUpdateTranslation from "../hooks/useUpdateTranslation"
import {GET_ELECTION_EVENT} from "../queries/GetElectionEvent"
import {IElectionEvent} from "../store/electionEvents/electionEventsSlice"

const StyledLink = styled(Link)`
    text-decoration: none;
`

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

const StyledError = styled(Typography)`
    position: absolute;
    margin-top: -12px;
    color: ${({theme}) => theme.palette.red.main};
`

const MessageSuccess = styled(Box)`
    display: flex;
    padding: 10px 22px;
    color: ${({theme}) => theme.palette.green.dark};
    background-color: ${({theme}) => theme.palette.green.light};
    gap: 8px;
    border-radius: 4px;
    border: 1px solid ${({theme}) => theme.palette.green.dark};
    align-items: center;
    margin-right: auto;
    margin-left: auto;
    overflow-wrap: anywhere;
`

const MessageFailed = styled(Box)`
    display: flex;
    padding: 10px 22px;
    color: ${({theme}) => theme.palette.red.dark};
    background-color: ${({theme}) => theme.palette.red.light};
    gap: 8px;
    border-radius: 4px;
    border: 1px solid ${({theme}) => theme.palette.red.dark};
    align-items: center;
    margin-right: auto;
    margin-left: auto;
    overflow-wrap: anywhere;
`

function isHex(str: string) {
    if (str.trim() === "") {
        return true
    }

    const regex = /^[0-9a-fA-F]+$/
    return regex.test(str)
}

const StyledApp = styled(Stack)<{css: string}>`
    min-height: 100vh;
    min-width: 100vw;
    ${({css}) => css}
`

interface TabPanelProps {
  children?: React.ReactNode
  index: number
  value: number
}

const CustomTabPanel: React.FC<TabPanelProps> = ({children, index, value}) => {

    return (
        <div
            role="tabpanel"
            hidden={value !== index}
            id={`simple-tabpanel-${index}`}
            aria-labelledby={`simple-tab-${index}`}
        >
            {value === index && <Box sx={{ p: 3 }}>{children}</Box>}
        </div>
    );
}

const BallotLocator: React.FC = () => {

    const a11yProps = (index: number) => {
        return {
            id: `simple-tab-${index}`,
            'aria-controls': `simple-tabpanel-${index}`,
        };
    }

    const [value, setValue] = React.useState(0);

    const handleChange = (event: React.SyntheticEvent, newValue: number) => {
        setValue(newValue);
    };

    return (
        <Box sx={{ width: '100%' }}>
            <Box sx={{ borderBottom: 1, borderColor: 'divider' }}>
                <Tabs value={value} onChange={handleChange} aria-label="basic tabs example">
                    <Tab label="Item One" {...a11yProps(0)} />
                    <Tab label="Item Two" {...a11yProps(1)} />
                </Tabs>
            </Box>
            <CustomTabPanel value={value} index={0}>
                <BallotLocatorLogic />
            </CustomTabPanel>
            <CustomTabPanel value={value} index={1}>
                Item Two
            </CustomTabPanel>
        </Box>
    );
}

const BallotLocatorLogic: React.FC = () => {
    const {tenantId, eventId, electionId, ballotId} = useParams()
    const [openTitleHelp, setOpenTitleHelp] = useState<boolean>(false)
    const navigate = useNavigate()
    const location = useLocation()
    const {t} = useTranslation()
    const [inputBallotId, setInputBallotId] = useState<string>("")
    const {globalSettings} = useContext(SettingsContext)
    const [listCastVoteMessages] =
        useMutation<ListCastVoteMessagesMutation>(LIST_CAST_VOTE_MESSAGES)
    const sendRequest = useRef<boolean>(true)

    const hasBallotId = !!ballotId
    const {data: dataBallotStyles} = useQuery<GetBallotStylesQuery>(GET_BALLOT_STYLES)

    const dispatch = useAppDispatch()
    const ballotStyle = useAppSelector(selectFirstBallotStyle)
    useLanguage({ballotStyle})

    const {data, loading} = useQuery<GetCastVoteQuery>(GET_CAST_VOTE, {
        variables: {
            tenantId,
            electionEventId: eventId,
            electionId,
            ballotId,
        },
        skip: globalSettings.DISABLE_AUTH, // Skip query if in demo mode
    })
    const requestCVMsgs = async () => {
        let result = await listCastVoteMessages({
            variables: {
                tenantId,
                electionEventId: eventId,
                electionId,
                ballotId,
            },
        })
    }

    useEffect(() => {
        if (hasBallotId && sendRequest.current) {
            sendRequest.current = false
            requestCVMsgs()
        }
    }, [hasBallotId])

    const {data: dataElectionEvent} = useQuery<GetElectionEventQuery>(GET_ELECTION_EVENT, {
        variables: {
            electionEventId: eventId,
            tenantId,
        },
        skip: globalSettings.DISABLE_AUTH, // Skip query if in demo mode
    })

    useUpdateTranslation({
        electionEvent: dataElectionEvent?.sequent_backend_election_event[0] as IElectionEvent,
    }) // Overwrite translations

    useEffect(() => {
        if (dataBallotStyles && dataBallotStyles.sequent_backend_ballot_style.length > 0) {
            updateBallotStyleAndSelection(dataBallotStyles, dispatch)
        }
    }, [dataBallotStyles, dispatch])

    const validatedBallotId = isHex(inputBallotId ?? "")

    const ballotContent =
        data?.["sequent_backend_cast_vote"]?.find((item) => item.ballot_id === ballotId)?.content ??
        null

    const locate = (withBallotId = false) => {
        let id = withBallotId ? inputBallotId : ""

        setInputBallotId("")

        navigate(
            `/tenant/${tenantId}/event/${eventId}/election/${electionId}/ballot-locator/${id}${location.search}`
        )
    }

    const captureEnter: React.KeyboardEventHandler<HTMLDivElement> = (event) => {
        if ("Enter" === event.key) {
            locate(true)
        }
    }

    return (
        <StyledApp
            css={dataElectionEvent?.sequent_backend_election_event[0]?.presentation?.css ?? ""}
        >
            <PageLimit maxWidth="lg" className="ballot-locator-screen screen">
                <Box marginTop="48px">
                    <BreadCrumbSteps
                        labels={["ballotLocator.steps.lookup", "ballotLocator.steps.result"]}
                        selected={hasBallotId ? 1 : 0}
                    />
                </Box>

                <Box
                    sx={{
                        display: "flex",
                        flexDirection: {xs: "column", md: "row"},
                        justifyContent: "space-between",
                        alignItems: "flex-start",
                    }}
                >
                    <Box
                        sx={{
                            order: {xs: 2, md: 1},
                        }}
                    >
                        <StyledTitle variant="h1">
                            {!hasBallotId ? (
                                <Box>{t("ballotLocator.title")}</Box>
                            ) : (
                                <Box>{t("ballotLocator.titleResult")}</Box>
                            )}
                            <IconButton
                                icon={faCircleQuestion}
                                sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                                fontSize="16px"
                                onClick={() => setOpenTitleHelp(true)}
                            />
                            <Dialog
                                handleClose={() => setOpenTitleHelp(false)}
                                open={openTitleHelp}
                                title={t("ballotLocator.titleHelpDialog.title")}
                                ok={t("ballotLocator.titleHelpDialog.ok")}
                                variant="info"
                            >
                                {stringToHtml(t("ballotLocator.titleHelpDialog.content"))}
                            </Dialog>
                        </StyledTitle>

                        <Typography
                            variant="body1"
                            sx={{color: theme.palette.customGrey.contrastText}}
                        >
                            {t("ballotLocator.description")}
                        </Typography>
                    </Box>
                    <Box sx={{order: {xs: 1, md: 2}, marginTop: "20px"}}>
                        <StyledLink
                            to={`/tenant/${tenantId}/event/${eventId}/election-chooser${location.search}`}
                        >
                            <Button variant="secondary" className="secondary">
                                <Icon icon={faAngleLeft} size="sm" />
                                <Box paddingLeft="12px">{t("votingScreen.backButton")}</Box>
                            </Button>
                        </StyledLink>
                    </Box>
                </Box>

                {hasBallotId && !loading && (
                    <Box>
                        {hasBallotId && !!ballotContent ? (
                            <MessageSuccess>{t("ballotLocator.found", {ballotId})}</MessageSuccess>
                        ) : (
                            <MessageFailed>{t("ballotLocator.notFound", {ballotId})}</MessageFailed>
                        )}
                    </Box>
                )}
                {!hasBallotId && (
                    <>
                        <TextField
                            onChange={(event: React.ChangeEvent<HTMLInputElement>) => {
                                setInputBallotId(event.target.value)
                            }}
                            value={inputBallotId}
                            InputLabelProps={{
                                shrink: true,
                            }}
                            label="Ballot ID"
                            placeholder={t("ballotLocator.description")}
                            onKeyDown={captureEnter}
                        />
                        {!validatedBallotId && (
                            <StyledError>{t("ballotLocator.wrongFormatBallotId")}</StyledError>
                        )}
                    </>
                )}

                {hasBallotId && ballotContent && (
                    <>
                        <Typography>{t("ballotLocator.contentDesc")}</Typography>
                        <InfoDataBox>{ballotContent}</InfoDataBox>
                    </>
                )}

                {!hasBallotId ? (
                    <Button
                        sx={{marginTop: "10px"}}
                        disabled={!validatedBallotId || inputBallotId.trim() === ""}
                        className="normal"
                        onClick={() => locate(true)}
                    >
                        <span>{t("ballotLocator.locate")}</span>
                    </Button>
                ) : (
                    <>
                        <Button
                            sx={{marginTop: "10px"}}
                            className="normal"
                            onClick={() => locate()}
                        >
                            <span>{t("ballotLocator.locateAgain")}</span>
                        </Button>
                    </>
                )}
            </PageLimit>
        </StyledApp>
    )
}

export default BallotLocator
