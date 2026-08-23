// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {TranslationType} from "./en"

const basqueTranslation: TranslationType = {
    translations: {
        language: "Euskara",
        footer: {
            poweredBy: "Honek bultzatuta: <1></1>",
        },
        version: {
            header: "Bertsioa:",
        },
        hash: {
            header: "Hash:",
        },
        logout: {
            buttonText: "Saioa itxi",
            modal: {
                title: "Ziur zaude saioa itxi nahi duzula?",
                content: "Aplikazio hau ixtear zaude.",
                ok: "Ados",
                close: "Itxi",
            },
        },
        header: {
            profile: "Profila",
            welcome: "Ongi etorri,<br><span>{{name}}</span>",
            session: {
                title: "Zure saioa iraungitzear dago.",
                timeLeft: "{{time}} geratzen zaizu.",
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minutu eta {{time}} segundo",
                timeLeftSeconds: "{{timeLeft}} segundo",
            },
        },
        resultsPortal: {
            pageTitle: "Hauteskundearen emaitzak",
            publishedResultsDescription: "Hauteskunde-ekitaldi honetarako argitaratutako emaitzak.",
            resultsAndParticipationTitle: "Emaitzak eta parte-hartzea",
            electionsTitle: "Hauteskundeak",
            contestsTitle: "Lehiaketak",
            areasTitle: "Eremuak",
            globalArea: "Globala",
            noResultsForSelection: "Ez dago emaitzarik eskuragarri hautapen honetarako.",
            version: "{{version}} bertsioa",
            publicAccess: "Sarbide publikoa",
            signedInAccess: "Saioa hasita sartzea",
            published: "Argitaratua",
            notPublishedYet: "Oraindik argitaratu gabe",
            position_one: "{{count}} postu",
            position_many: "{{count}} postu",
            position_other: "{{count}} postu",
            fallbackElectionName: "Hauteskundea",
            fallbackContestName: "{{contestId}} lehiaketa",
            state: {
                unexpectedErrorTitle: "Ustekabeko errorea",
                loadErrorMessage:
                    "Ezin izan ditugu emaitzak une honetan kargatu. Saiatu berriro minutu batzuk barru.",
                signInErrorMessage:
                    "Ezin izan dugu emaitzetarako saio-hasiera osatu une honetan. Saiatu berriro minutu batzuk barru.",
                signInRequiredTitle: "Saioa hastea beharrezkoa da",
                signInRequiredMessage:
                    "Hasi saioa zure hautesle-kontuarekin emaitza hauek ikusteko.",
                notPublishedTitle: "Emaitzak oraindik ez dira argitaratu",
                notPublishedMessage:
                    "Emaitzak ez daude eskuragarri une honetan. Begiratu berriro geroago.",
            },
            summary: {
                title: "Informazio orokorra",
                ariaLabel: "Emaitzen informazio orokorra",
                election: "Hauteskundea",
                eligibleVoters: "Hautesle hautagarriak",
                totalVotesCounted: "Zenbatutako botoak guztira",
                validVotes: "Baliozko botoak",
                participation: "Parte-hartzea",
                totalBlankBallots: "Boto-txartel zuriak guztira",
            },
            resultsAndParticipation: {
                participationSummary: "Parte-hartzearen laburpena",
                candidateResults: "Hautagaien emaitzak",
                total: "Guztira",
                turnout: "%",
                eligibleCensus: "Hautesle hautagarriak",
                totalAuditableVotes: "Ikuskatu daitezkeen botoak guztira",
                totalVotesCounted: "Zenbatutako botoak guztira",
                totalValidVotes: "Baliozko botoak guztira",
                totalInvalidVotes: "Baliogabeko botoak guztira",
                explicitInvalidVotes: "Berariaz baliogabeko botoak",
                implicitInvalidVotes: "Inplizituki baliogabeko botoak",
                blankVotes: "Boto zuriak",
                explicitBlankVotes: "Boto zuri esplizituak",
                implicitBlankVotes: "Boto zuri inplizituak",
                blankVotesChart: "Boto zuriak",
                weight: "Pisua",
                options: "Aukerak",
                castVotes: "Boto kopurua",
                castVotesPercent: "Botoen ehunekoa",
                winningPosition: "Irabazle postua",
                votesForCandidates: "Hautagaientzako botoak",
                invalidVotes: "Baliogabeko botoak",
                nonVoters: "Bozkatu ez dutenak",
                others: "Besteak",
                candidate: "Hautagaia",
                round: "Txanda",
                winner: "Irabazlea",
                eliminated: "Kanporatua",
                empty: "Elementurik ez",
                previousRounds: "Joan aurreko txandetara",
                nextRounds: "Joan hurrengo txandetara",
                participationByChannel: "Parte-hartzea kanalaren arabera",
                channel: "Kanala",
                channelOnline: "Linean",
                channelKiosk: "Kioskoa",
                channelEarlyVoting: "Boto aurreratua",
                channelTelephone: "Telefonoa",
                channelPaper: "Papera",
                channelPostal: "Posta",
                channelInPerson: "Aurrez aurre",
            },
        },
    },
}

export default basqueTranslation
