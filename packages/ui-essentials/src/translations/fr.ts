// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const frenchTranslation: TranslationType = {
    translations: {
        language: "Français",
        welcome: "Commençons : Importation de bulletin de vote auditable.",
        breadcrumbSteps: {
            select: "Sélectionner un vérificateur",
            import: "Importer des données",
            verify: "Vérifier",
            finish: "Terminer",
        },
        electionEventBreadcrumbSteps: {
            created: "Créé",
            keys: "Clés",
            publish: "Publier",
            started: "Commencé",
            ended: "Terminé",
            results: "Résultats",
        },
        candidate: {
            moreInformationLink: "Plus d'informations",
            writeInsPlaceholder: "Tapez ici le candidat par écrit",
            blankVote: "Vote blanc",
        },
        homeScreen: {
            title: "Vérificateur de vote Sequent",
            description1:
                "Le vérificateur de vote est utilisé lorsque l'électeur choisit d'auditer le bulletin de vote dans l'isoloir. La vérification doit prendre de 1 à 2 minutes.",
            description2:
                "Le vérificateur de vote permet à l'électeur de s'assurer que le vote chiffré capture correctement les choix faits dans l'isoloir. Permettre cette vérification est appelé vérifiabilité de transmission telle que prévue et empêche les erreurs et les activités malveillantes pendant le chiffrement du vote.",
            descriptionMore: "Plus d'informations",
            startButton: "Sélectionnez un fichier",
            dragDropOption: "Ou glissez le fichier ici",
            importErrorDescription:
                "Il y a eu un problème lors de l'importation du vote auditable. Avez-vous choisi le bon fichier ?",
            importErrorMoreInfo: "Plus d'informations",
            importErrorTitle: "Erreur",
            useSampleText: "Vous n'avez pas de vote vérifiable ?",
            useSampleLink: "Utilisez un exemple de vote vérifiable",
        },
        confirmationScreen: {
            title: "Vérificateur de vote Sequent",
            topDescription1:
                "Sur la base des informations du vote auditable importé, nous calculons que :",
            topDescription2: "Si cet ID de vote est affiché dans l'isoloir :",
            bottomDescription1:
                "Votre vote a été correctement chiffré. Vous pouvez maintenant fermer cette fenêtre et retourner à l'isoloir.",
            bottomDescription2:
                "Si elles ne correspondent pas, cliquez ici pour plus d'informations sur les raisons possibles et les mesures à prendre.",
            ballotChoicesDescription: "Et vos choix de vote sont :",
            helpAndFaq: "Aide et FAQ",
            backButton: "Retour",
            markedInvalid: "Vote explicitement marqué invalide",
        },
        ballotSelectionsScreen: {
            statusModal: {
                title: "État",
                content:
                    "Le panneau d'état vous donne des informations sur les vérifications effectuées.",
                ok: "OK",
            },
        },
        poweredBy: "Propulsé par",
        errors: {
            encoding: {
                notEnoughChoices: "Il n'y a pas assez d'options pour décoder",
                writeInChoiceOutOfRange: "Option de vote écrite hors de portée : {{index}}",
                writeInNotEndInZero: "Option de vote écrite ne finit pas en 0",
                writeInCharsExceeded:
                    "Option de vote écrite dépasse le nombre de caractères de {{numCharsExceeded}} caractères. Nécessite une correction.",
                bytesToUtf8Conversion:
                    "Erreur de conversion des octets de l'option de vote écrite en chaîne UTF-8 : {{errorMessage}}",
                ballotTooLarge: "Bulletin plus grand que prévu",
            },
            implicit: {
                selectedMax:
                    "Survote: Le nombre d'options sélectionnées {{numSelected}} est supérieur au maximum {{max}}",
                selectedMin:
                    "Le nombre d'options sélectionnées {{numSelected}} est inférieur au maximum {{min}}",
                maxSelectionsPerType:
                    "Le nombre d'options sélectionnées {{numSelected}} pour la liste {{type}} est supérieur au maximum {{max}}",
                underVote:
                    "Sous-vote: Le nombre de choix sélectionnés {{numSelected}} est inférieur au maximum autorisé de {{max}}",
                overVoteDisabled:
                    "Maximum atteint : Vous avez sélectionné le maximum de {{numSelected}} choix. Pour changer votre sélection, veuillez d'abord désélectionner une autre option.",
                blankVote: "Vote Blanc: 0 options sélectionnées",
            },
            explicit: {
                notAllowed:
                    "Vote marqué explicitement comme invalide mais la question ne le permet pas",
                alert: "La sélection marquée sera considérée comme un vote invalide.",
            },
            page: {
                oopsWithStatus: "Oups ! {{status}}",
                oopsWithoutStatus: "Oups ! Erreur inattendue",
                somethingWrong: "Quelque chose s'est mal passé.",
            },
        },
        ballotHash: "Votre Localisateur de Vote : {{ballotId}}",
        version: {
            header: "Version:",
        },
        hash: {
            header: "Hash:",
        },
        logout: {
            buttonText: "Fermer la session",
            modal: {
                title: "Êtes-vous sûr de vouloir fermer la session ?",
                content:
                    "Vous êtes sur le point de fermer cette application. Cette action ne peut pas être annulée.",
                ok: "OK",
                close: "Fermer",
            },
        },
        stories: {
            openDialog: "Ouvrir le dialogue",
        },
        dragNDrop: {
            firstLine: "Glisser-déposer des fichiers ou",
            browse: "Charger un fichier",
            format: "Formats supportés : txt",
        },
        selectElection: {
            electionWebsite: "Site web électoral",
            countdown:
                "L’élection commence dans {{years}} ans, {{months}} mois, {{weeks}} semaines, {{days}} jours, {{hours}} heures, {{minutes}} minutes, {{seconds}} secondes",
            openElection: "Ouverte",
            closedElection: "Fermée",
            voted: "Voté",
            notVoted: "Non voté",
            resultsButton: "Résultats de l'élection",
            voteButton: "Cliquez pour voter",
            openDate: "Ouverte : ",
            closeDate: "Fermée : ",
            ballotLocator: "Localisez votre vote",
        },
        header: {
            profile: "Profil",
            welcome: "Bienvenue,<br><span>{{name}}</span>",
            session: {
                title: "Votre session est sur le point d'expirer.",
                timeLeft: "Il vous reste {{time}} pour voter.",
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minutes et {{time}} secondes",
                timeLeftSeconds: "{{timeLeft}} secondes",
            },
        },
    },
}

export default frenchTranslation
