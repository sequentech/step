import {IElectionPresentation, translateElection} from "@sequentech/ui-core"
import {use, useCallback, useEffect, useMemo} from "react"
import {useTranslation} from "react-i18next"
import {IConfirmationBallot} from "../../services/BallotService"

const MAX_CLASSNAME_LENGTH = 40
const ROOT_CLASS_PREFIX = "e-"

function toValidClassName(input?: string): string | null {
    if (!input) return null

    let result = input.replace(/\s+/g, "")
    result = result.replace(/[^a-zA-Z0-9-_]/g, "")

    result = `${ROOT_CLASS_PREFIX}${result}`
    result = result.slice(0, MAX_CLASSNAME_LENGTH)

    return result.length ? result : null
}

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

        const root = document.documentElement
        root.classList.add(electionClassName)

        return () => {
            root.classList.remove(electionClassName)
        }
    }, [electionClassName])
}
