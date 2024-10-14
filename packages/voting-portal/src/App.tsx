// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useContext, useMemo, useState} from "react"
import {Outlet, ScrollRestoration, useLocation, useParams} from "react-router-dom"
import {styled} from "@mui/material/styles"
import {Footer, Header, PageBanner} from "@sequentech/ui-essentials"
import {EVotingPortalCountdownPolicy, IElectionEventPresentation} from "@sequentech/ui-core"
import Stack from "@mui/material/Stack"
import {useNavigate} from "react-router-dom"
import {AuthContext} from "./providers/AuthContextProvider"
import {SettingsContext} from "./providers/SettingsContextProvider"
import {TenantEventType, PreviewPublicationEventType} from "."
import {ApolloWrapper} from "./providers/ApolloContextProvider"
import {VotingPortalError, VotingPortalErrorType} from "./services/VotingPortalError"
import {useAppDispatch, useAppSelector} from "./store/hooks"
import {selectElectionIds} from "./store/elections/electionsSlice"
import {
    IBallotStyle,
    selectBallotStyleByElectionId,
    selectFirstBallotStyle,
    setBallotStyle,
} from "./store/ballotStyles/ballotStylesSlice"
import WatermarkBackground from "./components/WaterMark/Watermark"
import SequentLogo from "@sequentech/ui-essentials/public/Sequent_logo.svg"
import BlankLogoImg from "@sequentech/ui-essentials/public/blank_logo.svg"
import { IBallotStyle as IElectionDTO }  from "@sequentech/ui-core"
import { cloneDeep } from "lodash"
import { GetBallotPublicationChangesOutput } from "./gql/graphql"

const StyledApp = styled(Stack)<{css: string}>`
    min-height: 100vh;
    ${({css}) => css}
`

const HeaderWithContext: React.FC = () => {
    const authContext = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)
    const {eventId} = useParams<TenantEventType>()

    const ballotStyle = useAppSelector(selectFirstBallotStyle)

    let presentation: IElectionEventPresentation | undefined =
        ballotStyle?.ballot_eml.election_event_presentation

    let languagesList = presentation?.language_conf?.enabled_language_codes ?? ["en"]
    let showUserProfile = presentation?.show_user_profile ?? true
    const countdownPolicy = useMemo(() => {
        return ballotStyle?.ballot_eml.election_event_presentation?.voting_portal_countdown_policy
    }, [ballotStyle])

    const logoImg =
        presentation?.logo_url === undefined
            ? BlankLogoImg
            : presentation?.logo_url === null
            ? SequentLogo
            : presentation?.logo_url

    return (
        <Header
            appVersion={{main: globalSettings.APP_VERSION}}
            appHash={{main: globalSettings.APP_HASH}}
            userProfile={{
                firstName: authContext.firstName,
                username: authContext.username,
                email: authContext.email,
                openLink: showUserProfile ? authContext.openProfileLink : undefined,
            }}
            languagesList={languagesList}
            logoutFn={authContext.isAuthenticated ? authContext.logout : undefined}
            logoUrl={logoImg}
            expiry={{
                alertAt: countdownPolicy?.countdown_alert_anticipation_secs,
                countdown: countdownPolicy?.policy ?? EVotingPortalCountdownPolicy.NO_COUNTDOWN,
                countdownAt: countdownPolicy?.countdown_anticipation_secs,
                endTime: authContext.getExpiry(),
                duration: countdownPolicy?.countdown_anticipation_secs,
            }}
        />
    )
}

const App = () => {
    const navigate = useNavigate()
    const {globalSettings} = useContext(SettingsContext)
    const location = useLocation()
    const {tenantId, eventId} = useParams<TenantEventType>()
    const {tenantId: documentTenant, documentId, areaId} = useParams<PreviewPublicationEventType>()
    const {isAuthenticated, setTenantEvent, } = useContext(AuthContext)

    const electionIds = useAppSelector(selectElectionIds)
    const isPreviewRoute = location.pathname.includes("/preview/");
    const ballotStyle = useAppSelector(isPreviewRoute ? selectFirstBallotStyle : selectBallotStyleByElectionId(String(electionIds[0])))
    const [ballotStyleJson, setBballotStyleJson] = useState<GetBallotPublicationChangesOutput>() // State to store the JSON data
    const dispatch = useAppDispatch();

    const previewUrl = useMemo(() => {
        return `http://127.0.0.1:9000/public/tenant-${tenantId}/document-${documentId}/preview.json`;
      }, [tenantId, documentId])

    useEffect(() => {
        const fetchPreviewData = async () => {
            try {
                const response = await fetch(previewUrl)
                if (!response.ok) {
                    throw new Error(`Error: ${response.statusText}`)
                }
                const data = await response.json()
                setBballotStyleJson(data) 
            } catch (err: any) {
                console.log("error")
            } 
        }

        if (documentTenant && documentId) {
            fetchPreviewData()
        }
    }, [documentTenant, documentId])
  
    useEffect(() => {
        if (ballotStyleJson && areaId && tenantId) {
            try {
                const ballotStyle = ballotStyleJson.current.ballot_styles.find(
                    (style: any) => style.area_id === areaId
                );
                const eml: IElectionDTO = cloneDeep(ballotStyle);

                const formattedBallotStyle: IBallotStyle = {
                    id: ballotStyle.election_id,
                    election_id: ballotStyle.election_id,
                    election_event_id: ballotStyle.election_event_id,
                    tenant_id: documentTenant || "",
                    ballot_eml: eml,
                    ballot_signature: null,
                    created_at: "",
                    area_id: areaId,
                    annotations: null,
                    labels: null,
                    last_updated_at: "",
                }
                dispatch(setBallotStyle(formattedBallotStyle))
                
            } catch (error) {
                console.log(`Error loading EML: ${error}`)
                // throw new VotingPortalError(VotingPortalErrorType.INTERNAL_ERROR)
            }
        }
        
    }, [ballotStyleJson])

    useEffect(() => {
        if (location.pathname.includes('preview')) {
            if (ballotStyle && documentTenant) {
                navigate(
                    `/tenant/${documentTenant}/event/${ballotStyle.election_event_id}/election-chooser${location.search}`    
                )
            } else return
            //TODO logic
        }
        else if (globalSettings.DISABLE_AUTH) {
            navigate(
                `/tenant/${globalSettings.DEFAULT_TENANT_ID}/event/${globalSettings.DEFAULT_EVENT_ID}/election-chooser${location.search}`
            )
        } else {
            if (location.pathname === "/") {
                throw new VotingPortalError(VotingPortalErrorType.NO_ELECTION_EVENT)
            }
        }
    }, [
        globalSettings.DEFAULT_TENANT_ID,
        globalSettings.DEFAULT_EVENT_ID,
        globalSettings.DISABLE_AUTH,
        navigate,
        location.pathname,
        ballotStyle,
        documentTenant
    ])

    useEffect(() => {
        if (!isAuthenticated && !!tenantId && !!eventId) {
            setTenantEvent(
                tenantId,
                eventId,
                location.pathname.includes("/enroll") ? "register" : "login"
            )
        }
    }, [tenantId, eventId, isAuthenticated, setTenantEvent])

    return (
        <StyledApp
            className="app-root"
            css={ballotStyle?.ballot_eml.election_event_presentation?.css ?? ""}
        >
            <ScrollRestoration />
            <ApolloWrapper> 
                {globalSettings.DISABLE_AUTH ? <Header /> : <HeaderWithContext />}
                <PageBanner
                    marginBottom="auto"
                    sx={{display: "flex", position: "relative", flex: 1}}
                >
                    <WatermarkBackground />
                    <Outlet />
                </PageBanner>
            </ApolloWrapper>
            <Footer />
        </StyledApp>
    );
}

export default App
