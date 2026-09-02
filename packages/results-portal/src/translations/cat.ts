// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {TranslationType} from "./en"

const catalanTranslation: TranslationType = {
    translations: {
        language: "Català",
        footer: {
            poweredBy: "Funciona amb <1></1>",
        },
        version: {
            header: "Versió:",
        },
        hash: {
            header: "Hash:",
        },
        logout: {
            buttonText: "Tancar sessió",
            modal: {
                title: "Segur que vols tancar la sessió?",
                content: "Estàs a punt de tancar aquesta aplicació.",
                ok: "D'acord",
                close: "Tancar",
            },
        },
        header: {
            profile: "Perfil",
            welcome: "Benvingut/uda,<br><span>{{name}}</span>",
            session: {
                title: "La teva sessió està a punt de caducar.",
                timeLeft: "Et queda {{time}}.",
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minuts i {{time}} segons",
                timeLeftSeconds: "{{timeLeft}} segons",
            },
        },
        resultsPortal: {
            pageTitle: "Resultats de l'elecció",
            publishedResultsDescription: "Resultats publicats per a aquest esdeveniment electoral.",
            resultsAndParticipationTitle: "Resultats i participació",
            electionsTitle: "Eleccions",
            contestsTitle: "Conteses",
            areasTitle: "Àrees",
            globalArea: "Global",
            noResultsForSelection: "No hi ha resultats disponibles per a aquesta selecció.",
            version: "Versió {{version}}",
            publicAccess: "Accés públic",
            signedInAccess: "Accés amb sessió iniciada",
            acclaimed: "Elegit per aclamació",
            acclamationNote:
                "Elegit per aclamació. Aquesta votació es va resoldre sense votació, per la qual cosa no es va registrar cap vot.",
            published: "Publicat",
            notPublishedYet: "Encara no publicat",
            position: "{{count}} posició",
            position_plural: "{{count}} posicions",
            fallbackElectionName: "Elecció",
            fallbackContestName: "Contesa {{contestId}}",
            state: {
                unexpectedErrorTitle: "Error inesperat",
                loadErrorMessage:
                    "No hem pogut carregar els resultats ara mateix. Torna-ho a provar d'aquí a uns minuts.",
                signInErrorMessage:
                    "No hem pogut completar l'inici de sessió per als resultats ara mateix. Torna-ho a provar d'aquí a uns minuts.",
                signInRequiredTitle: "Cal iniciar sessió",
                signInRequiredMessage:
                    "Inicia sessió amb el teu compte de votant per veure aquests resultats.",
                notPublishedTitle: "Resultats encara no publicats",
                notPublishedMessage:
                    "Els resultats no estan disponibles en aquest moment. Torna-ho a comprovar més tard.",
            },
            summary: {
                title: "Informació general",
                ariaLabel: "Informació general dels resultats",
                election: "Elecció",
                eligibleVoters: "Votants elegibles",
                totalVotesCounted: "Total de vots comptats",
                validVotes: "Vots vàlids",
                participation: "Participació",
                totalBlankBallots: "Total de paperetes en blanc",
            },
            resultsAndParticipation: {
                participationSummary: "Resum de participació",
                candidateResults: "Resultats de candidats",
                total: "Total",
                turnout: "%",
                eligibleCensus: "Votants elegibles",
                totalAuditableVotes: "Total de vots auditables",
                totalVotesCounted: "Total de vots comptats",
                totalValidVotes: "Total de vots vàlids",
                totalInvalidVotes: "Total de vots invàlids",
                explicitInvalidVotes: "Vots explícitament invàlids",
                implicitInvalidVotes: "Vots implícitament invàlids",
                blankVotes: "Vots en blanc",
                explicitBlankVotes: "Vots en blanc explícits",
                implicitBlankVotes: "Vots en blanc implícits",
                blankVotesChart: "Vots en blanc",
                weight: "Pes",
                options: "Opcions",
                castVotes: "Nombre de vots",
                castVotesPercent: "Percentatge de vots",
                winningPosition: "Posició guanyadora",
                votesForCandidates: "Vots a candidats",
                invalidVotes: "Vots invàlids",
                nonVoters: "No votants",
                others: "Altres",
                candidate: "Candidat",
                round: "Ronda",
                winner: "Guanyador",
                eliminated: "Eliminat",
                empty: "Sense elements",
                previousRounds: "Anar a rondes anteriors",
                nextRounds: "Anar a rondes següents",
                participationByChannel: "Participació per canal",
                channel: "Canal",
                channelOnline: "En línia",
                channelKiosk: "Quiosc",
                channelEarlyVoting: "Votació anticipada",
                channelTelephone: "Telèfon",
                channelPaper: "Paper",
                channelPostal: "Postal",
                channelInPerson: "Presencial",
            },
        },
    },
}

export default catalanTranslation
