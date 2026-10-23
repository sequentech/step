// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useCallback, useContext, useState} from "react"
import {useApolloClient} from "@apollo/client/react"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {IBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import {useAppSelector} from "../store/hooks"
import {runBoothCheck, TBoothOutcome} from "../services/vitrine/booth"
import {EBoothCheckReason, EBoothRefusalState} from "../services/vitrine/boothCheck"

export interface IBoothRefusal {
    reason: EBoothCheckReason
    state: EBoothRefusalState
}

const isPreview = (): boolean => {
    try {
        return sessionStorage.getItem("isDemo") === "true"
    } catch {
        return false
    }
}

/** The Vitrine check before encrypting (step-integration.md, section 2.6). */
export const useBoothCheck = () => {
    const client = useApolloClient()
    const {globalSettings} = useContext(SettingsContext)
    const events = useAppSelector((state) => state.electionEvent)
    const [boothRefusal, setBoothRefusal] = useState<IBoothRefusal | undefined>()

    const checkBeforeEncrypting = useCallback(
        async (ballotStyle: IBallotStyle): Promise<boolean> => {
            let outcome: TBoothOutcome
            try {
                outcome = await runBoothCheck({
                    client,
                    style: {
                        electionEventId: ballotStyle.election_event_id,
                        electionId: ballotStyle.election_id,
                        publicKey: ballotStyle.ballot_eml.public_key?.public_key,
                        styleText: ballotStyle.ballot_eml_text,
                        presentation: ballotStyle.ballot_eml.election_event_presentation,
                    },
                    eventPresentation: events[ballotStyle.election_event_id]?.presentation,
                    release: globalSettings.APP_VERSION,
                    skip: globalSettings.DISABLE_AUTH || isPreview(),
                })
            } catch (error) {
                console.error("Vitrine: the check before encrypting failed:", error)
                outcome = {
                    ok: false,
                    reason: EBoothCheckReason.VERIFIER_UNAVAILABLE,
                    state: EBoothRefusalState.PENDING,
                }
            }
            setBoothRefusal(outcome.ok ? undefined : {reason: outcome.reason, state: outcome.state})
            return outcome.ok
        },
        [client, events, globalSettings.APP_VERSION, globalSettings.DISABLE_AUTH]
    )

    const dismissBoothRefusal = useCallback(() => setBoothRefusal(undefined), [])

    return {checkBeforeEncrypting, boothRefusal, dismissBoothRefusal}
}
