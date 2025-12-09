// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const frenchTranslation: TranslationType = {
    translations: {
        "welcome": "Commençons : Importez le bulletin auditable...",
        "404": {
            title: "Page non trouvée",
            subtitle: "La page que vous cherchez n'existe pas",
        },
        "homeScreen": {
            step1: "Étape 1 : Importez votre bulletin de vote.",
            description1:
                "Pour continuer, veuillez importer les données des bulletins cryptés fournis sur le Portail de Vote :",
            importBallotHelpDialog: {
                title: "Information : Importez votre bulletin de vote",
                ok: "OK",
                content:
                    "Pour continuer, veuillez importer les données des bulletins cryptés fournis sur le Portail de Vote.",
            },
            step2: "Étape 2 : Insérez votre ID de bulletin.",
            description2: "Veuillez entrer l'ID du bulletin fourni sur le Portail de Vote :",
            ballotIdHelpDialog: {
                title: "Information : Votre ID de bulletin",
                ok: "OK",
                content: "Veuillez entrer l'ID du bulletin fourni sur le Portail de Vote.",
            },
            startButton: "Sélectionnez fichier",
            dragDropOption: "Ou glissez le fichier ici",
            importErrorDescription:
                "Il y a eu un problème lors de l'importation du vote auditable. Avez-vous choisi le bon fichier ?",
            importErrorMoreInfo: "Plus d'informations",
            importErrorTitle: "Erreur",
            useSampleLink: "Utiliser un vote exemple",
            nextButton: "Continuer",
            ballotIdLabel: "ID du bulletin",
            ballotIdPlaceholder: "Écrivez ici votre ID de bulletin",
            fileUploaded: "Chargé",
        },
        "confirmationScreen": {
            ballotIdTitle: "ID du bulletin",
            ballotIdDescription:
                "Ensuite, le système affiche l'ID du bulletin décodé et celui généré par le vérificateur.",
            ballotIdError: "Ne correspond pas à l'ID du bulletin décodé.",
            decodedBallotId: "ID du bulletin décodé",
            decodedBallotIdHelpDialog: {
                title: "Information : ID du bulletin décodé",
                ok: "OK",
                content:
                    "Ceci est l'ID du bulletin extrait du fichier du bulletin auditable décodé que vous avez fourni.",
            },
            yourBallotId: "L'ID du bulletin que vous avez fourni",
            userBallotIdHelpDialog: {
                title: "Information : L'ID du bulletin que vous avez fourni",
                ok: "OK",
                content:
                    "Ceci est l'ID du bulletin que vous avez écrit à l'étape précédente et que vous avez pris de la Cabine de Vote.",
            },
            backButton: "Retour",
            printButton: "Imprimer",
            finishButton: "Vérifié",
            verifySelectionsTitle: "Vérifiez vos sélections sur le bulletin",
            verifySelectionsDescription:
                "Les sélections suivantes du bulletin ont été décodées du bulletin que vous avez importé. Veuillez les revoir et vous assurer qu'elles correspondent aux sélections que vous avez faites sur le Portail de Vote. Si vos sélections ne correspondent pas, veuillez contacter les autorités électorales...",
            verifySelectionsHelpDialog: {
                title: "Information : Vérifiez vos sélections sur le bulletin",
                ok: "OK",
                content:
                    "Les sélections suivantes du bulletin ont été décodées du bulletin que vous avez importé. Veuillez les revoir et vous assurer qu'elles correspondent aux sélections que vous avez faites sur le Portail de Vote. Si vos sélections ne correspondent pas, veuillez contacter les autorités électorales...",
            },
            markedInvalid: "Vote explicitement marqué invalide",
            points: "({{points}} Points)",
            contestNotFound: "Question non trouvée : {{contestId}}",
        },
        "footer": {
            poweredBy: "Propulsé par <1></1>",
        },
        "errors": {
            encoding: {
                notEnoughChoices: "Il n'y a pas assez d'options pour décoder",
                writeInChoiceOutOfRange: "Option de vote écrite hors de portée : {{index}}",
                writeInNotEndInZero: "Option de vote écrite ne finit pas en 0",
                bytesToUtf8Conversion:
                    "Erreur de conversion des octets de l'option de vote écrite en chaîne UTF-8 : {{errorMessage}}",
                ballotTooLarge: "Vote plus grand que prévu",
            },
            implicit: {
                selectedMax:
                    "Le nombre d'options sélectionnées {{numSelected}} est supérieur au maximum {{max}}",
                selectedMin:
                    "Le nombre d'options sélectionnées {{numSelected}} est inférieur au minimum {{min}}",
            },
            explicit: {
                notAllowed:
                    "Vote marqué explicitement comme invalide mais la question ne le permet pas",
            },
        },
    },
}

export default frenchTranslation
