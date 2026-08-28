// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {TranslationType} from "./en"

const frenchTranslation: TranslationType = {
    translations: {
        language: "Français",
        footer: {
            poweredBy: "Propulsé par <1></1>",
        },
        version: {
            header: "Version :",
        },
        hash: {
            header: "Hash :",
        },
        logout: {
            buttonText: "Déconnexion",
            modal: {
                title: "Voulez-vous vraiment vous déconnecter ?",
                content: "Vous êtes sur le point de fermer cette application.",
                ok: "OK",
                close: "Fermer",
            },
        },
        header: {
            profile: "Profil",
            welcome: "Bienvenue,<br><span>{{name}}</span>",
            session: {
                title: "Votre session va expirer.",
                timeLeft: "Il vous reste {{time}}.",
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minutes et {{time}} secondes",
                timeLeftSeconds: "{{timeLeft}} secondes",
            },
        },
        resultsPortal: {
            pageTitle: "Résultats de l'élection",
            publishedResultsDescription: "Résultats publiés pour cet événement électoral.",
            resultsAndParticipationTitle: "Résultats et participation",
            electionsTitle: "Élections",
            contestsTitle: "Scrutins",
            areasTitle: "Secteurs",
            globalArea: "Global",
            noResultsForSelection: "Aucun résultat n'est disponible pour cette sélection.",
            version: "Version {{version}}",
            publicAccess: "Accès public",
            signedInAccess: "Accès connecté",
            acclaimed: "Élu par acclamation",
            acclamationNote:
                "Élu par acclamation. Ce vote a été acquis sans scrutin : aucune voix n'a été enregistrée.",
            published: "Publié",
            notPublishedYet: "Pas encore publié",
            position: "{{count}} position",
            position_plural: "{{count}} positions",
            fallbackElectionName: "Élection",
            fallbackContestName: "Scrutin {{contestId}}",
            state: {
                unexpectedErrorTitle: "Erreur inattendue",
                loadErrorMessage:
                    "Nous n'avons pas pu charger les résultats pour le moment. Veuillez réessayer dans quelques minutes.",
                signInErrorMessage:
                    "Nous n'avons pas pu terminer la connexion aux résultats pour le moment. Veuillez réessayer dans quelques minutes.",
                signInRequiredTitle: "Connexion requise",
                signInRequiredMessage:
                    "Veuillez vous connecter avec votre compte électeur pour consulter ces résultats.",
                notPublishedTitle: "Résultats pas encore publiés",
                notPublishedMessage:
                    "Les résultats ne sont pas disponibles pour le moment. Veuillez réessayer plus tard.",
            },
            summary: {
                title: "Informations générales",
                ariaLabel: "Informations générales sur les résultats",
                election: "Élection",
                eligibleVoters: "Électeurs éligibles",
                totalVotesCounted: "Total des votes comptés",
                validVotes: "Votes valides",
                participation: "Participation",
                totalBlankBallots: "Total des bulletins blancs",
            },
            resultsAndParticipation: {
                participationSummary: "Résumé de participation",
                candidateResults: "Résultats des candidats",
                total: "Total",
                turnout: "%",
                eligibleCensus: "Électeurs éligibles",
                totalAuditableVotes: "Total des votes auditables",
                totalVotesCounted: "Total des votes comptés",
                totalValidVotes: "Total des votes valides",
                totalInvalidVotes: "Total des votes invalides",
                explicitInvalidVotes: "Votes explicitement invalides",
                implicitInvalidVotes: "Votes implicitement invalides",
                blankVotes: "Votes blancs",
                explicitBlankVotes: "Votes blancs explicites",
                implicitBlankVotes: "Votes blancs implicites",
                blankVotesChart: "Votes blancs",
                weight: "Poids",
                options: "Options",
                castVotes: "Nombre de votes",
                castVotesPercent: "Pourcentage des votes",
                winningPosition: "Position gagnante",
                votesForCandidates: "Votes pour les candidats",
                invalidVotes: "Votes invalides",
                nonVoters: "Non-votants",
                others: "Autres",
                candidate: "Candidat",
                round: "Tour",
                winner: "Gagnant",
                eliminated: "Éliminé",
                empty: "Aucun élément",
                previousRounds: "Aller aux tours précédents",
                nextRounds: "Aller aux tours suivants",
                participationByChannel: "Participation par canal",
                channel: "Canal",
                channelOnline: "En ligne",
                channelKiosk: "Kiosque",
                channelEarlyVoting: "Vote anticipé",
                channelTelephone: "Téléphone",
                channelPaper: "Papier",
                channelPostal: "Postal",
                channelInPerson: "En personne",
            },
        },
    },
}

export default frenchTranslation
