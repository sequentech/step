// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const frenchTranslation: TranslationType = {
    translations: {
        common: {
            goBack: "Revenir",
        },
        breadcrumbSteps: {
            electionList: "Liste des Élections",
            ballot: "Bulletin de vote",
            review: "Révision",
            confirmation: "Confirmation",
            audit: "Auditer",
        },
        votingScreen: {
            backButton: "Retour",
            reviewButton: "Suivant",
            clearButton: "Effacer la sélection",
            ballotHelpDialog: {
                title: "Information : Écran de vote",
                content:
                    "Cet écran affiche le vote pour lequel vous êtes éligible. Vous pouvez sélectionner votre section en activant la case à droite Candidat/Réponse. Pour réinitialiser vos sélections, cliquez sur le bouton “<b>Effacer la sélection</b>”, pour passer à l'étape suivante, cliquez sur le bouton “<b>Suivant</b>”.",
                ok: "OK",
            },
            nonVotedDialog: {
                title: "Vote invalide ou blanc",
                content:
                    "Certaines de vos réponses pourraient rendre le bulletin invalide ou blanc dans une ou plusieurs questions.",
                ok: "Revenir et réviser",
                continue: "Continuer",
                cancel: "Annuler",
            },
        },
        startScreen: {
            startButton: "Commencer à voter",
            instructionsTitle: "Instructions",
            instructionsDescription: "Veuillez suivre ces étapes pour voter :",
            step1Title: "1. Sélectionnez votre option de vote",
            step1Description:
                "Sélectionnez vos candidats préférés et répondez aux questions de l'élection une par une au fur et à mesure qu'elles apparaissent. Vous pouvez modifier votre bulletin jusqu'à ce que vous soyez prêt à continuer.",
            step2Title: "2. Révisez votre bulletin",
            step2Description:
                "Une fois que vous êtes satisfait de vos sélections, nous chiffrerons votre bulletin et vous montrerons une révision finale de vos choix. Vous recevrez également un ID de suivi unique pour votre bulletin.",
            step3Title: "3. Envoyez votre vote",
            step3Description:
                "Envoyez votre bulletin : Enfin, vous pouvez envoyer votre bulletin pour qu'il soit correctement enregistré. Alternativement, vous pouvez opter pour auditer et confirmer que votre bulletin a été capturé et chiffré correctement.",
        },
        reviewScreen: {
            title: "Révisez votre vote",
            description:
                "Pour apporter des modifications à vos sélections, cliquez sur le bouton “<b>Modifier votre vote</b>”, pour confirmer vos sélections, cliquez sur le bouton “<b>Envoyer votre vote</b>” ci-dessous, et pour auditer votre bulletin, cliquez sur le bouton “<b>Auditer le bulletin</b>” ci-dessous. Notez qu'une fois que vous aurez envoyé votre bulletin, vous aurez voté et il ne vous sera plus possible de recevoir un autre bulletin pour cette élection.",
            descriptionNoAudit:
                "Pour apporter des modifications à vos sélections, cliquez sur le bouton “<b>Modifier votre vote</b>”, pour confirmer vos sélections, cliquez sur le bouton “<b>Envoyer votre vote</b>” ci-dessous. Notez qu'une fois que vous aurez envoyé votre bulletin, vous aurez voté et il ne vous sera plus possible de recevoir un autre bulletin pour cette élection.",
            backButton: "Modifier votre vote",
            castBallotButton: "Envoyer votre vote",
            auditButton: "Auditer le bulletin",
            reviewScreenHelpDialog: {
                title: "Information : Écran de révision",
                content: "Cet écran vous permet de réviser vos sélections avant de voter.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Vote non émis",
                content:
                    "<p>Vous êtes sur le point de copier le Localisateur de Vote, mais <b>votre vote n'a pas encore été émis</b>. Si vous tentez de rechercher le Localisateur de Vote, vous ne le trouverez pas.</p><p>La raison pour laquelle nous affichons le Localisateur de Vote à ce moment est pour que vous puissiez auditer la correction du vote chiffré avant de l'émettre. Si c'est la raison pour laquelle vous souhaitez copier le Localisateur de Vote, procédez à sa copie puis auditez votre vote.</p>",
                ok: "J'accepte que mon vote N'A PAS été émis",
                cancel: "Annuler",
            },
            auditBallotHelpDialog: {
                title: "Voulez-vous vraiment Auditer votre bulletin ?",
                content:
                    "<p>L'audit du bulletin l'invalidera et vous devrez recommencer le processus de vote si vous souhaitez émettre votre vote. Le processus d'audit du bulletin permet de vérifier qu'il est correctement codé. Ce processus nécessite des connaissances techniques importantes, donc il n'est pas recommandé si vous ne savez pas ce que vous faites.</p><p><b>Si vous souhaitez émettre votre vote, cliquez sur <u>Annuler</u> pour revenir à l'écran de révision du vote.</b></p>",
                ok: "Oui, je veux INVALIDER mon bulletin pour l'AUDITER",
                cancel: "Annuler",
            },
        },
        confirmationScreen: {
            title: "Votre vote a été émis",
            description:
                "Le code de confirmation ci-dessous vérifie que <b>votre vote a été émis correctement</b>. Vous pouvez utiliser ce code pour vérifier que votre vote a été comptabilisé.",
            ballotId: "Localisateur de Vote",
            printButton: "Imprimer",
            finishButton: "Terminer",
            verifyCastTitle: "Vérifiez que votre vote a été émis",
            verifyCastDescription:
                "Vous pouvez vérifier à tout moment que votre bulletin a été émis correctement en utilisant le code QR ci-dessous :",
            confirmationHelpDialog: {
                title: "Information : Écran de confirmation",
                content:
                    "Cet écran montre que votre vote a été émis correctement. Les informations fournies sur cette page vous permettent de vérifier que le bulletin a été stocké dans l'urne, ce processus peut être exécuté à tout moment pendant la période de vote et après que l'élection a été clôturée.",
                ok: "OK",
            },
            demoPrintDialog: {
                title: "Impression du bulletin de vote",
                content: "L'impression est désactivée en mode démo",
                ok: "OK",
            },
            demoBallotUrlDialog: {
                title: "Suivi du Bulletin",
                content: "Impossible d'utiliser le code, désactivé en mode démo.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Information : Localisateur de votre Bulletin",
                content:
                    "Le Localisateur de Bulletin est un code qui vous permet de retrouver votre bulletin dans l'urne, ce Localisateur est unique et ne contient aucune information sur vos sélections.",
                ok: "OK",
            },
            ballotIdDemoHelpDialog: {
                title: "Information : Identifiant de bulletin de vote",
                content:
                    "<p>L'identifiant de bulletin de vote est un code qui vous permet de retrouver votre bulletin dans l'urne. Cet identifiant est unique et ne contient aucune information sur vos choix.</p><p><b>Avis :</b> Ce bureau de vote est uniquement à des fins de démonstration. Votre vote n'a PAS été émis.</p>",
                ok: "OK",
            },
            errorDialogPrintVoteReceipt: {
                title: "Erreur",
                content: "Une erreur s'est produite, veuillez réessayer",
                ok: "Accepter",
            },
            demoQRText: "Le suivi des bulletins est désactivé en mode démo",
        },
        auditScreen: {
            printButton: "Imprimer",
            restartButton: "Démarrer le vote",
            title: "Auditez votre Bulletin",
            description: "Pour vérifier votre bulletin, vous devez suivre les étapes suivantes :",
            step1Title: "1. Téléchargez ou copiez les informations suivantes",
            step1Description:
                "Votre <b>Localisateur de Vote</b> qui apparaît en haut de l'écran et votre bulletin chiffré ci-dessous :",
            step1HelpDialog: {
                title: "Copier le Vote Chiffré",
                content:
                    "Vous pouvez télécharger ou copier votre Vote Chiffré pour l'auditer et vérifier que le contenu chiffré contient vos sélections.",
                ok: "OK",
            },
            downloadButton: "Télécharger",
            step2Title: "2. Vérifiez votre bulletin",
            step2Description:
                "<a class=\"link\" href='{{linkToBallotVerifier}}' target='_blank'>Accédez au vérificateur de vote</a>, qui s'ouvrira dans un nouvel onglet de votre navigateur.",
            step2HelpDialog: {
                title: "Tutoriel sur l'Audit du Vote",
                content:
                    "Pour auditer votre vote, vous devez suivre les étapes indiquées dans le tutoriel, qui incluent le téléchargement d'une application de bureau utilisée pour vérifier le vote chiffré indépendamment du site web.",
                ok: "OK",
            },
            bottomWarning:
                "Pour des raisons de sécurité, lorsque vous auditez votre bulletin, vous devrez l'invalider. Pour continuer avec le processus de vote, cliquez sur ‘<b>Démarrer le vote</b>’.",
        },
        electionSelectionScreen: {
            title: "Liste des Élections",
            description: "Sélectionnez l'élection pour laquelle vous souhaitez voter",
            chooserHelpDialog: {
                title: "Information : Liste des Élections",
                content:
                    "Bienvenue dans le bureau de vote, cet écran montre la liste des élections dans lesquelles vous pouvez voter. Les élections affichées sur cette liste peuvent être ouvertes au vote, programmées ou fermées. Vous ne pourrez accéder au vote que si la période de vote est ouverte.",
                ok: "OK",
            },
            noResults: "Il n'y a pas d'élections pour le moment.",
            demoDialog: {
                title: "Bureau de vote de démonstration",
                content:
                    "Vous entrez dans un bureau de vote de démonstration. <strong>Votre vote ne sera PAS compté.</strong> Ce bureau de vote est uniquement destiné à des fins de démonstration.",
                ok: "J'accepte que mon vote ne sera pas compté",
            },
            noVotingAreaError:
                "Zone de vote non attribuée à l'électeur. Veuillez contacter votre administrateur pour obtenir de l'aide.",
        },
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
            },
            explicit: {
                notAllowed:
                    "Vote marqué explicitement comme invalide mais la question ne le permet pas",
            },
            page: {
                oopsWithStatus: "Oups ! {{status}}",
                oopsWithoutStatus: "Oups ! Erreur inattendue",
                somethingWrong: "Quelque chose s'est mal passé.",
            },
        },
        materials: {
            common: {
                label: "Matériaux de Support",
                back: "Revenir à la Liste des Élections",
                close: "Fermer",
                preview: "Aperçu",
            },
        },
        ballotLocator: {
            title: "Localisez votre Bulletin",
            titleResult: "Résultat de la recherche de votre Bulletin",
            description: "Vérifiez que votre bulletin a été émis correctement",
            locate: "Localisez votre Bulletin",
            locateAgain: "Localisez un autre Bulletin",
            found: "Votre ID de Bulletin {{ballotId}} a été localisé",
            notFound: "Votre ID de Bulletin {{ballotId}} n'a pas été localisé",
            contentDesc: "Voici le contenu de votre Bulletin : ",
            wrongFormatBallotId: "Format incorrect pour l'ID du Bulletin",
            steps: {
                lookup: "Localisez votre Bulletin",
                result: "Résultat",
            },
            titleHelpDialog: {
                title: "Information : écran de Localisation de votre Bulletin",
                content:
                    "Cet écran permet au votant de trouver son bulletin en utilisant l'ID du Bulletin pour le récupérer. Cette procédure permet de vérifier que son vote a été émis correctement et que le vote enregistré correspond au vote chiffré émis.",
                ok: "OK",
            },
        },
    },
}

export default frenchTranslation
