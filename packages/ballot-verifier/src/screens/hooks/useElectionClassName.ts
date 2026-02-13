// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {IElectionPresentation, toValidClassName, translateElection} from "@sequentech/ui-core"
import {useEffect, useMemo} from "react"
import {useTranslation} from "react-i18next"
import {IConfirmationBallot} from "../../services/BallotService"

/**
 * Manages election class on <html>
 */
export const useElectionClassName = (confirmationBallot: IConfirmationBallot | null) => {
    const {i18n} = useTranslation()

    const extractElectionName = (election: {presentation: IElectionPresentation; id: string}) => {
        let language = i18n.resolvedLanguage || i18n.language
        return (
            translateElection(election, "alias", language) ||
            translateElection(election, "name", language) ||
            election.id
        )
    }

    const electionClassName = useMemo(() => {
        if (!confirmationBallot || !confirmationBallot.election_config.election_presentation)
            return null
        let election = {
            presentation: confirmationBallot.election_config.election_presentation,
            id: confirmationBallot.election_config.election_id,
        }
        return toValidClassName(extractElectionName(election))
    }, [extractElectionName, confirmationBallot])

    useEffect(() => {
        if (!electionClassName) return

        const appRoot = document.querySelector(".app-root")
        if (!appRoot) return
        appRoot.classList.add(electionClassName)

        return () => {
            appRoot.classList.remove(electionClassName)
        }
    }, [electionClassName])
}
