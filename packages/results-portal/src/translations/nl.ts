// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {TranslationType} from "./en"

const dutchTranslation: TranslationType = {
    translations: {
        language: "Nederlands",
        footer: {
            poweredBy: "Mogelijk gemaakt door <1></1>",
        },
        version: {
            header: "Versie:",
        },
        hash: {
            header: "Hash:",
        },
        logout: {
            buttonText: "Uitloggen",
            modal: {
                title: "Weet je zeker dat je wilt uitloggen?",
                content: "Je staat op het punt deze applicatie te sluiten.",
                ok: "OK",
                close: "Sluiten",
            },
        },
        header: {
            profile: "Profiel",
            welcome: "Welkom,<br><span>{{name}}</span>",
            session: {
                title: "Je sessie verloopt binnenkort.",
                timeLeft: "Je hebt nog {{time}}.",
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minuten en {{time}} seconden",
                timeLeftSeconds: "{{timeLeft}} seconden",
            },
        },
        resultsPortal: {
            pageTitle: "Verkiezingsresultaten",
            publishedResultsDescription: "Gepubliceerde resultaten voor dit verkiezingsevenement.",
            resultsAndParticipationTitle: "Resultaten en deelname",
            electionsTitle: "Verkiezingen",
            contestsTitle: "Wedstrijden",
            areasTitle: "Gebieden",
            globalArea: "Globaal",
            noResultsForSelection: "Er zijn geen resultaten beschikbaar voor deze selectie.",
            version: "Versie {{version}}",
            publicAccess: "Openbare toegang",
            signedInAccess: "Toegang na inloggen",
            published: "Gepubliceerd",
            notPublishedYet: "Nog niet gepubliceerd",
            position_one: "{{count}} positie",
            position_other: "{{count}} posities",
            fallbackElectionName: "Verkiezing",
            fallbackContestName: "Wedstrijd {{contestId}}",
            state: {
                unexpectedErrorTitle: "Onverwachte fout",
                loadErrorMessage:
                    "We konden de resultaten nu niet laden. Probeer het over een paar minuten opnieuw.",
                signInErrorMessage:
                    "We konden het inloggen voor de resultaten nu niet voltooien. Probeer het over een paar minuten opnieuw.",
                signInRequiredTitle: "Inloggen vereist",
                signInRequiredMessage:
                    "Log in met je kiezersaccount om deze resultaten te bekijken.",
                notPublishedTitle: "Resultaten nog niet gepubliceerd",
                notPublishedMessage:
                    "Resultaten zijn momenteel niet beschikbaar. Controleer het later opnieuw.",
            },
            summary: {
                title: "Algemene informatie",
                ariaLabel: "Algemene resultaatinformatie",
                election: "Verkiezing",
                eligibleVoters: "Stemgerechtigden",
                totalVotesCounted: "Totaal getelde stemmen",
                validVotes: "Geldige stemmen",
                participation: "Opkomst",
            },
            resultsAndParticipation: {
                participationSummary: "Samenvatting deelname",
                candidateResults: "Kandidaatresultaten",
                total: "Totaal",
                turnout: "%",
                eligibleCensus: "Stemgerechtigden",
                totalAuditableVotes: "Totaal controleerbare stemmen",
                totalVotesCounted: "Totaal getelde stemmen",
                totalValidVotes: "Totaal geldige stemmen",
                totalInvalidVotes: "Totaal ongeldige stemmen",
                explicitInvalidVotes: "Expliciet ongeldige stemmen",
                implicitInvalidVotes: "Impliciet ongeldige stemmen",
                blankVotes: "Blanco stemmen",
                explicitBlankVotes: "Expliciete blanco stemmen",
                implicitBlankVotes: "Impliciete blanco stemmen",
                blankVotesChart: "Blanco stemmen",
                weight: "Gewicht",
                options: "Opties",
                castVotes: "Aantal stemmen",
                castVotesPercent: "Percentage stemmen",
                winningPosition: "Winnende positie",
                votesForCandidates: "Stemmen voor kandidaten",
                invalidVotes: "Ongeldige stemmen",
                nonVoters: "Niet-stemmers",
                others: "Overige",
                candidate: "Kandidaat",
                round: "Ronde",
                winner: "Winnaar",
                eliminated: "Geëlimineerd",
                empty: "Geen items",
                previousRounds: "Ga naar vorige rondes",
                nextRounds: "Ga naar volgende rondes",
                participationByChannel: "Deelname per kanaal",
                channel: "Kanaal",
                channelOnline: "Online",
                channelKiosk: "Kiosk",
                channelEarlyVoting: "Vervroegd stemmen",
                channelTelephone: "Telefoon",
                channelPaper: "Papier",
                channelPostal: "Post",
                channelInPerson: "Persoonlijk",
            },
        },
    },
}

export default dutchTranslation
