// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {TranslationType} from "./en"

const spanishInformalTranslation: TranslationType = {
    translations: {
        language: "Español (tú)",
        footer: {
            poweredBy: "Funciona con <1></1>",
        },
        version: {
            header: "Versión:",
        },
        hash: {
            header: "Hash:",
        },
        logout: {
            buttonText: "Cerrar sesión",
            modal: {
                title: "¿Seguro que quieres cerrar sesión?",
                content: "Estás a punto de cerrar esta aplicación.",
                ok: "OK",
                close: "Cerrar",
            },
        },
        header: {
            profile: "Perfil",
            welcome: "Bienvenido/a,<br><span>{{name}}</span>",
            session: {
                title: "Tu sesión va a caducar.",
                timeLeft: "Te queda {{time}}.",
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minutos y {{time}} segundos",
                timeLeftSeconds: "{{timeLeft}} segundos",
            },
        },
        resultsPortal: {
            pageTitle: "Resultados de la elección",
            publishedResultsDescription: "Resultados publicados para este evento electoral.",
            resultsAndParticipationTitle: "Resultados y participación",
            electionsTitle: "Elecciones",
            contestsTitle: "Contiendas",
            areasTitle: "Áreas",
            globalArea: "Global",
            noResultsForSelection: "No hay resultados disponibles para esta selección.",
            version: "Versión {{version}}",
            publicAccess: "Acceso público",
            signedInAccess: "Acceso con sesión iniciada",
            published: "Publicado",
            notPublishedYet: "Todavía no publicado",
            position_one: "{{count}} puesto",
            position_other: "{{count}} puestos",
            fallbackElectionName: "Elección",
            fallbackContestName: "Contienda {{contestId}}",
            state: {
                unexpectedErrorTitle: "Error inesperado",
                loadErrorMessage:
                    "No hemos podido cargar los resultados ahora mismo. Inténtalo de nuevo en unos minutos.",
                signInErrorMessage:
                    "No hemos podido completar el inicio de sesión para los resultados ahora mismo. Inténtalo de nuevo en unos minutos.",
                signInRequiredTitle: "Inicio de sesión requerido",
                signInRequiredMessage:
                    "Inicia sesión con tu cuenta de votante para ver estos resultados.",
                notPublishedTitle: "Resultados todavía no publicados",
                notPublishedMessage:
                    "Los resultados no están disponibles en este momento. Vuelve a comprobarlo más tarde.",
            },
            summary: {
                title: "Información general",
                ariaLabel: "Información general de resultados",
                election: "Elección",
                eligibleVoters: "Votantes elegibles",
                totalVotesCounted: "Total de votos contados",
                validVotes: "Votos válidos",
                participation: "Participación",
            },
            resultsAndParticipation: {
                participationSummary: "Resumen de participación",
                candidateResults: "Resultados de candidatos",
                total: "Total",
                turnout: "%",
                eligibleCensus: "Votantes elegibles",
                totalAuditableVotes: "Total de votos auditables",
                totalVotesCounted: "Total de votos contados",
                totalValidVotes: "Total de votos válidos",
                totalInvalidVotes: "Total de votos inválidos",
                explicitInvalidVotes: "Votos explícitamente inválidos",
                implicitInvalidVotes: "Votos implícitamente inválidos",
                blankVotes: "Votos en blanco",
                explicitBlankVotes: "Votos en blanco explícitos",
                implicitBlankVotes: "Votos en blanco implícitos",
                blankVotesChart: "Votos en blanco",
                weight: "Peso",
                options: "Opciones",
                castVotes: "Número de votos",
                castVotesPercent: "Porcentaje de votos",
                winningPosition: "Puesto ganador",
                votesForCandidates: "Votos a candidatos",
                invalidVotes: "Votos inválidos",
                nonVoters: "No votantes",
                others: "Otros",
                candidate: "Candidato",
                round: "Ronda",
                winner: "Ganador",
                eliminated: "Eliminado",
                empty: "Sin elementos",
                previousRounds: "Ir a rondas anteriores",
                nextRounds: "Ir a rondas siguientes",
                participationByChannel: "Participación por canal",
                channel: "Canal",
                channelOnline: "En línea",
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

export default spanishInformalTranslation
