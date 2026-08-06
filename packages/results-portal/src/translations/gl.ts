// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {TranslationType} from "./en"

const galegoTranslation: TranslationType = {
    translations: {
        language: "Galego",
        footer: {
            poweredBy: "Desenvolvido por <1></1>",
        },
        version: {
            header: "Versión:",
        },
        hash: {
            header: "Hash:",
        },
        logout: {
            buttonText: "Pechar sesión",
            modal: {
                title: "Seguro que queres pechar a sesión?",
                content: "Estás a piques de pechar esta aplicación.",
                ok: "Aceptar",
                close: "Pechar",
            },
        },
        header: {
            profile: "Perfil",
            welcome: "Benvido/a,<br><span>{{name}}</span>",
            session: {
                title: "A túa sesión vai caducar.",
                timeLeft: "Quédanche {{time}}.",
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minutos e {{time}} segundos",
                timeLeftSeconds: "{{timeLeft}} segundos",
            },
        },
        resultsPortal: {
            pageTitle: "Resultados da elección",
            publishedResultsDescription: "Resultados publicados para este evento electoral.",
            resultsAndParticipationTitle: "Resultados e participación",
            electionsTitle: "Eleccións",
            contestsTitle: "Contendas",
            areasTitle: "Áreas",
            globalArea: "Global",
            noResultsForSelection: "Non hai resultados dispoñibles para esta selección.",
            version: "Versión {{version}}",
            publicAccess: "Acceso público",
            signedInAccess: "Acceso con sesión iniciada",
            published: "Publicado",
            notPublishedYet: "Aínda non publicado",
            position_one: "{{count}} posto",
            position_other: "{{count}} postos",
            fallbackElectionName: "Elección",
            fallbackContestName: "Contenda {{contestId}}",
            state: {
                unexpectedErrorTitle: "Erro inesperado",
                loadErrorMessage:
                    "Non puidemos cargar os resultados agora mesmo. Téntao de novo nuns minutos.",
                signInErrorMessage:
                    "Non puidemos completar o inicio de sesión para os resultados agora mesmo. Téntao de novo nuns minutos.",
                signInRequiredTitle: "Cómpre iniciar sesión",
                signInRequiredMessage:
                    "Inicia sesión coa túa conta de votante para ver estes resultados.",
                notPublishedTitle: "Resultados aínda non publicados",
                notPublishedMessage:
                    "Os resultados non están dispoñibles neste momento. Volve comprobalo máis tarde.",
            },
            summary: {
                title: "Información xeral",
                ariaLabel: "Información xeral dos resultados",
                election: "Elección",
                eligibleVoters: "Votantes elixibles",
                totalVotesCounted: "Total de votos contados",
                validVotes: "Votos válidos",
                participation: "Participación",
            },
            resultsAndParticipation: {
                participationSummary: "Resumo de participación",
                candidateResults: "Resultados de candidatos",
                total: "Total",
                turnout: "%",
                eligibleCensus: "Votantes elixibles",
                totalAuditableVotes: "Total de votos auditables",
                totalVotesCounted: "Total de votos contados",
                totalValidVotes: "Total de votos válidos",
                totalInvalidVotes: "Total de votos inválidos",
                explicitInvalidVotes: "Votos explicitamente inválidos",
                implicitInvalidVotes: "Votos implicitamente inválidos",
                blankVotes: "Votos en branco",
                explicitBlankVotes: "Votos en branco explícitos",
                implicitBlankVotes: "Votos en branco implícitos",
                blankVotesChart: "Votos en branco",
                weight: "Peso",
                options: "Opcións",
                castVotes: "Número de votos",
                castVotesPercent: "Porcentaxe de votos",
                winningPosition: "Posto gañador",
                votesForCandidates: "Votos a candidatos",
                invalidVotes: "Votos inválidos",
                nonVoters: "Non votantes",
                others: "Outros",
                candidate: "Candidato",
                round: "Rolda",
                winner: "Gañador",
                eliminated: "Eliminado",
                empty: "Sen elementos",
                previousRounds: "Ir a roldas anteriores",
                nextRounds: "Ir a roldas seguintes",
                participationByChannel: "Participación por canle",
                channel: "Canle",
                channelOnline: "En liña",
                channelKiosk: "Quiosco",
                channelEarlyVoting: "Votación anticipada",
                channelTelephone: "Teléfono",
                channelPaper: "Papel",
                channelPostal: "Postal",
                channelInPerson: "Presencial",
            },
        },
    },
}

export default galegoTranslation
