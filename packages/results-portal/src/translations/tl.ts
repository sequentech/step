// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {TranslationType} from "./en"

const tagalogTranslation: TranslationType = {
    translations: {
        language: "Tagalog",
        footer: {
            poweredBy: "Pinapagana ng <1></1>",
        },
        version: {
            header: "Bersyon:",
        },
        hash: {
            header: "Hash:",
        },
        logout: {
            buttonText: "Mag-logout",
            modal: {
                title: "Sigurado ka bang gusto mong mag-logout?",
                content: "Isasara mo na ang aplikasyong ito.",
                ok: "OK",
                close: "Isara",
            },
        },
        header: {
            profile: "Profile",
            welcome: "Maligayang pagdating,<br><span>{{name}}</span>",
            session: {
                title: "Malapit nang mag-expire ang iyong session.",
                timeLeft: "May {{time}} ka pa.",
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minuto at {{time}} segundo",
                timeLeftSeconds: "{{timeLeft}} segundo",
            },
        },
        resultsPortal: {
            pageTitle: "Mga Resulta ng Halalan",
            publishedResultsDescription: "Mga nailathalang resulta para sa election event na ito.",
            resultsAndParticipationTitle: "Mga Resulta at Partisipasyon",
            electionsTitle: "Mga Halalan",
            contestsTitle: "Mga Labanan",
            areasTitle: "Mga Lugar",
            globalArea: "Global",
            noResultsForSelection: "Walang available na resulta para sa seleksyong ito.",
            version: "Bersyon {{version}}",
            publicAccess: "Pampublikong access",
            signedInAccess: "Access na naka-sign in",
            acclaimed: "Nanalo sa pamamagitan ng aklamasyon",
            acclamationNote:
                "Nanalo sa pamamagitan ng aklamasyon. Ang paligsahang ito ay napagpasyahan nang walang botohan, kaya walang naitalang boto.",
            published: "Nailathala",
            notPublishedYet: "Hindi pa nailalathala",
            position: "{{count}} posisyon",
            position_plural: "{{count}} posisyon",
            fallbackElectionName: "Halalan",
            fallbackContestName: "Labanan {{contestId}}",
            state: {
                unexpectedErrorTitle: "Hindi inaasahang error",
                loadErrorMessage:
                    "Hindi namin ma-load ang mga resulta ngayon. Pakisubukan muli pagkalipas ng ilang minuto.",
                signInErrorMessage:
                    "Hindi namin makumpleto ang pag-sign in para sa mga resulta ngayon. Pakisubukan muli pagkalipas ng ilang minuto.",
                signInRequiredTitle: "Kailangang mag-sign in",
                signInRequiredMessage:
                    "Mag-sign in gamit ang iyong voter account upang makita ang mga resultang ito.",
                notPublishedTitle: "Hindi pa nailalathala ang mga resulta",
                notPublishedMessage:
                    "Hindi available ang mga resulta sa ngayon. Pakisuri muli mamaya.",
            },
            summary: {
                title: "Pangkalahatang impormasyon",
                ariaLabel: "Pangkalahatang impormasyon ng mga resulta",
                election: "Halalan",
                eligibleVoters: "Mga botanteng karapat-dapat",
                totalVotesCounted: "Kabuuang botong nabilang",
                validVotes: "Mga wastong boto",
                participation: "Partisipasyon",
                totalBlankBallots: "Kabuuang blangkong balota",
            },
            resultsAndParticipation: {
                participationSummary: "Buod ng Partisipasyon",
                candidateResults: "Mga Resulta ng Kandidato",
                total: "Kabuuan",
                turnout: "%",
                eligibleCensus: "Mga Botanteng Karapat-dapat",
                totalAuditableVotes: "Kabuuang Naa-audit na Boto",
                totalVotesCounted: "Kabuuang Botong Nabilang",
                totalValidVotes: "Kabuuang Wastong Boto",
                totalInvalidVotes: "Kabuuang Di-wastong Boto",
                explicitInvalidVotes: "Hayagang Di-wastong Boto",
                implicitInvalidVotes: "Di-tuwirang Di-wastong Boto",
                blankVotes: "Blangkong Boto",
                explicitBlankVotes: "Hayagang Blangkong Boto",
                implicitBlankVotes: "Di-tuwirang Blangkong Boto",
                blankVotesChart: "Blangkong Boto",
                weight: "Timbang",
                options: "Mga Opsyon",
                castVotes: "Bilang ng mga Boto",
                castVotesPercent: "Porsyento ng mga Boto",
                winningPosition: "Panalong posisyon",
                votesForCandidates: "Mga Boto para sa mga Kandidato",
                invalidVotes: "Di-wastong Boto",
                nonVoters: "Hindi Bumoto",
                others: "Iba pa",
                candidate: "Kandidato",
                round: "Round",
                winner: "Nanalo",
                eliminated: "Naalis",
                empty: "Walang item",
                previousRounds: "Pumunta sa mga nakaraang round",
                nextRounds: "Pumunta sa mga susunod na round",
                participationByChannel: "Paglahok ayon sa channel",
                channel: "Channel",
                channelOnline: "Online",
                channelKiosk: "Kiosk",
                channelEarlyVoting: "Maagang pagboto",
                channelTelephone: "Telepono",
                channelPaper: "Papel",
                channelPostal: "Koreo",
                channelInPerson: "Personal",
            },
        },
    },
}

export default tagalogTranslation
