// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const frenchTranslation: TranslationType = {
    translations: {
        language: "Français",
        welcome: "Commençons : Importation de bulletin de vote vérifiable.",
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
                "Le vérificateur de vote est utilisé lorsque l'électeur choisit d'auditer son bulletin de vote dans l'isoloir. La vérification doit prendre de 1 à 2 minutes.",
            description2:
                "Le vérificateur de vote permet à l'électeur de s'assurer que le bulletin chiffré reflète fidèlement les choix effectués dans l'isoloir. Cette vérification permet de détecter les erreurs et les tentatives de manipulation lors du chiffrement du bulletin.",
            descriptionMore: "Plus d'informations",
            startButton: "Sélectionnez un fichier",
            dragDropOption: "Ou faites glisser le fichier ici",
            importErrorDescription:
                "Une erreur est survenue lors de l'importation du bulletin vérifiable. Avez-vous sélectionné le bon fichier ?",
            importErrorMoreInfo: "Plus d'informations",
            importErrorTitle: "Erreur",
            useSampleText: "Vous n'avez pas de vote vérifiable ?",
            useSampleLink: "Utilisez un exemple de vote vérifiable",
        },
        confirmationScreen: {
            title: "Vérificateur de vote Sequent",
            topDescription1:
                "À partir des informations du bulletin vérifiable importé, nous avons calculé que :",
            topDescription2: "Si ce numéro d'identification de vote est affiché dans l'isoloir :",
            bottomDescription1:
                "Votre vote a été correctement chiffré. Vous pouvez maintenant fermer cette fenêtre et retourner à l'isoloir.",
            bottomDescription2:
                "Si elles ne correspondent pas, cliquez ici pour plus d'informations sur les raisons possibles et les mesures à prendre.",
            ballotChoicesDescription: "Vos choix de vote sont:",
            helpAndFaq: "Aide et FAQ",
            backButton: "Retour",
            markedInvalid: "Vote explicitement marqué comme invalide",
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
                notEnoughChoices: "Nombre d'options insuffisant pour le déchiffrement",
                writeInChoiceOutOfRange: "Entrée de saisie libre hors limites: {{index}}",
                writeInNotEndInZero: "Le texte de saisie libre ne se termine pas par 0",
                bytesToUtf8Conversion:
                    "Erreur lors de la conversion des octets de l'entrée en saisie libre en chaîne UTF-8 : {{errorMessage}}",
                ballotTooLarge: "La taille du bulletin dépasse la limite prévue",
            },
            implicit: {
                selectedMax:
                    "Le nombre d'options sélectionnées {{numSelected}} est supérieur au maximum {{max}}",
                selectedMin:
                    "Le nombre d'options sélectionnées {{numSelected}} est inférieur au minimum {{min}}",
            },
            explicit: {
                notAllowed: "Vote marqué comme invalide mais la question ne l'autorise pas",
            },
        },
        ballotHash: "Votre localisateur de vote : {{ballotId}}",
        version: {
            header: "Version:",
        },
        hash: {
            header: "Hachage:",
        },
        logout: {
            buttonText: "Fermer la session",
            modal: {
                title: "Êtes-vous sûr de vouloir fermer la session ?",
                content:
                    "Vous êtes sur le point de fermer cette application. Cette action ne pourra pas être annulée.",
                ok: "OK",
                close: "Fermer",
            },
        },
        stories: {
            openDialog: "Ouvrir la boîte de dialogue",
        },
        dragNDrop: {
            firstLine: "Glissez-déposez des fichiers ou",
            browse: "Sélectionner un fichier",
            format: "Formats pris en charge : txt",
        },
        selectElection: {
            electionWebsite: "Site web électoral",
            countdown:
                "L’élection commence dans {{years}} ans, {{months}} mois, {{weeks}} semaines, {{days}} jours, {{hours}} heures, {{minutes}} minutes, {{seconds}} secondes",
            openElection: "Ouverte",
            closedElection: "Fermée",
            voted: "Vote enregistré",
            notVoted: "Vote non enregistré",
            resultsButton: "Résultats de l'élection",
            voteButton: "Cliquez pour voter",
            openDate: "Ouverture : ",
            closeDate: "Clôture : ",
            ballotLocator: "Localisez votre bulletin",
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
