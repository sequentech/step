// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext} from "react"
import {useLocation, useParams} from "react-router-dom"
import {useTranslation} from "react-i18next"
import {useQuery} from "@apollo/client/react"
import {AudioInstructions} from "@sequentech/ui-essentials"
import {
    EAudioInstructionsPolicy,
    EAudioInstructionsScreen,
    IElectionEventPresentation,
    findAudioInstructionsRecording,
} from "@sequentech/ui-core"
import {TenantEventType} from "../.."
import {GetDocumentQuery, GetSupportMaterialsQuery} from "../../gql/graphql"
import {GET_DOCUMENT} from "../../queries/GetDocument"
import {GET_SUPPORT_MATERIALS} from "../../queries/GetSupportMaterials"
import {useGetPublicDocumentUrl} from "../../hooks/public-document-url"
import {SettingsContext} from "../../providers/SettingsContextProvider"
import {useAppSelector} from "../../store/hooks"
import {selectFirstBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {selectElectionEventById} from "../../store/electionEvents/electionEventsSlice"

const SCREEN_PATHS: [RegExp, EAudioInstructionsScreen][] = [
    [/\/election-chooser\/?$/, EAudioInstructionsScreen.ELECTION_CHOOSER],
    [/\/materials\/?$/, EAudioInstructionsScreen.SUPPORT_MATERIALS],
    [/\/election\/[^/]+\/start\/?$/, EAudioInstructionsScreen.START],
    [/\/election\/[^/]+\/vote\/?$/, EAudioInstructionsScreen.BALLOT],
    [/\/election\/[^/]+\/review\/?$/, EAudioInstructionsScreen.REVIEW],
    [/\/election\/[^/]+\/confirmation\/?$/, EAudioInstructionsScreen.CONFIRMATION],
    [/\/election\/[^/]+\/audit\/?$/, EAudioInstructionsScreen.AUDIT],
    [/\/election\/[^/]+\/ballot-locator(\/[^/]*)?\/?$/, EAudioInstructionsScreen.BALLOT_LOCATOR],
]

/** The voter screen a path shows, for the screens that have audio instructions. */
export const audioInstructionsScreenAt = (pathname: string): EAudioInstructionsScreen | undefined =>
    SCREEN_PATHS.find(([pattern]) => pattern.test(pathname))?.[1]

interface ScreenInstructionsProps {
    policy: EAudioInstructionsPolicy
    screen: EAudioInstructionsScreen
    defaultLanguage?: string
}

const SpokenInstructions: React.FC<ScreenInstructionsProps & {recordingUrl?: string}> = ({
    policy,
    screen,
    recordingUrl,
}) => {
    const {t, i18n} = useTranslation()

    return (
        <AudioInstructions
            policy={policy}
            text={t(`audioInstructions.screens.${screen}`, {defaultValue: ""})}
            language={i18n.language}
            recordingUrl={recordingUrl}
        />
    )
}

/** Looks for the event's recording for the screen among its support materials. */
const RecordedInstructions: React.FC<ScreenInstructionsProps> = (props) => {
    const {screen, defaultLanguage} = props
    const {i18n} = useTranslation()
    const {tenantId, eventId} = useParams<TenantEventType>()
    const {getDocumentUrl} = useGetPublicDocumentUrl()

    const {data: materials} = useQuery<GetSupportMaterialsQuery>(GET_SUPPORT_MATERIALS, {
        variables: {electionEventId: eventId ?? "", tenantId: tenantId ?? ""},
        skip: !eventId || !tenantId,
    })
    const recording = findAudioInstructionsRecording(
        materials?.sequent_backend_support_material ?? [],
        screen,
        i18n.language,
        defaultLanguage
    )
    const documentId = recording?.document_id ?? undefined

    const {data: documents} = useQuery<GetDocumentQuery>(GET_DOCUMENT, {
        variables: {ids: [documentId ?? ""], electionEventId: eventId, tenantId: tenantId ?? ""},
        skip: !documentId,
    })
    const documentName = documents?.sequent_backend_document.find(
        (document) => document.id === documentId
    )?.name

    return (
        <SpokenInstructions
            {...props}
            recordingUrl={
                documentId && documentName ? getDocumentUrl(documentId, documentName) : undefined
            }
        />
    )
}

/**
 * The audio instructions for the screen the voter is on, where the event enables them.
 *
 * Without a session (the demo and the embedded preview) there is no recording to fetch, so
 * the text is spoken by the browser.
 */
export const ScreenAudioInstructions: React.FC = () => {
    const {pathname} = useLocation()
    const {eventId} = useParams<TenantEventType>()
    const {globalSettings} = useContext(SettingsContext)
    const ballotStyle = useAppSelector(selectFirstBallotStyle)
    const electionEvent = useAppSelector(selectElectionEventById(eventId))

    const presentation: IElectionEventPresentation | undefined =
        ballotStyle?.ballot_eml.election_event_presentation ??
        electionEvent?.presentation ??
        undefined
    const policy = presentation?.audio_instructions_policy
    const screen = audioInstructionsScreenAt(pathname)

    if (!policy || policy === EAudioInstructionsPolicy.DISABLED || !screen) {
        return null
    }

    const props = {
        policy,
        screen,
        defaultLanguage: presentation?.language_conf?.default_language_code,
    }
    return globalSettings.DISABLE_AUTH ? (
        <SpokenInstructions {...props} />
    ) : (
        <RecordedInstructions {...props} />
    )
}
