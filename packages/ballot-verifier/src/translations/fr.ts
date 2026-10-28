// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const frenchTranslation: TranslationType = {
    translations: {
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
            ciphertextErrorTitle: "La vérification du vote a échoué",
            ciphertextErrorDescription:
                "Le texte chiffré de ce vote auditable ne correspond pas au chiffrement du texte en clair et de l'aléa qu'il contient. Ce vote n'est pas fiable.",
            useSampleLink: "Utiliser un vote exemple",
            nextButton: "Continuer",
            ballotIdLabel: "ID du bulletin",
            ballotIdPlaceholder: "Écrivez ici votre ID de bulletin",
            fileUploaded: "Chargé",
            receiptTitle: "Reçu de transparence",
            receiptDescription:
                "Cette élection publie un reçu pour chaque bulletin déposé. Vérifiez le vôtre : qu'il figure dans le registre public, inchangé et identique pour tous.",
            receiptButton: "Vérifier un reçu",
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
            points_one: "({{count}} Point)",
            points_many: "({{count}} Points)",
            points_other: "({{count}} Points)",
            contestNotFound: "Scrutin non trouvé : {{contestId}}",
            declineToVote: "A choisi de ne pas voter",
            blankBallot: "Bulletin blanc",
            acclamationDescription:
                "Ce scrutin a été tranché par acclamation. Ses candidat(e)s sont élu(e)s sans vote : aucune option ne peut être sélectionnée et aucune voix n'est enregistrée.",
        },
        "receiptScreen": {
            title: "Vérifier un reçu de transparence",
            description:
                "Un reçu de transparence montre que votre bulletin figure dans le registre public de l'élection, inchangé et identique pour tous. Cette page le vérifie dans votre navigateur, avec la politique de confiance publiée par l'autorité électorale.",
            loading: "Chargement du vérificateur de reçus…",
            loadFailed:
                "Le vérificateur de reçus n'a pas pu se charger. Rechargez la page pour réessayer.",
            receipt: {
                title: "Votre reçu",
                description:
                    "Ouvrez le fichier du reçu que vous avez enregistré après avoir voté, ou collez le reçu, son lien ou sa référence (le texte de son code QR).",
                fileFormat: "Les fichiers de reçu se terminent par .vitrine-receipt",
                pasteLabel: "Reçu, lien ou référence",
                pastePlaceholder: "Collez-le ici",
                open: "Ouvrir",
                another: "Vérifier un autre reçu",
                problems: {
                    "empty": "Collez d'abord un reçu, son lien ou sa référence.",
                    "too-large": "C'est trop volumineux pour être un reçu.",
                    "bad-link":
                        "Ce lien ne contient pas de reçu lisible. Ouvrez plutôt le fichier du reçu.",
                    "bad-reference":
                        "Cette référence est incomplète ou endommagée. Copiez-la à nouveau ou ouvrez le fichier du reçu.",
                    "bad-chain": "Ce reçu est endommagé : il ne peut pas être lu.",
                    "not-https":
                        "Le lien indique que la politique se trouve à une adresse non sécurisée (https) ; elle n'est donc pas utilisée.",
                    "wrong-box":
                        "Ceci est une politique de confiance, pas un reçu. Ouvrez d'abord le reçu.",
                    "unrecognised": "Ceci n'est ni un reçu, ni le lien ou la référence d'un reçu.",
                    "unreadable": "Le fichier n'a pas pu être lu.",
                },
            },
            policy: {
                title: "Politique de confiance",
                description:
                    "Les reçus sont vérifiés avec la politique de confiance publiée par l'autorité électorale. Indiquez-la ici, par son texte ou son adresse https, telle qu'elle figure dans l'avis officiel de l'élection.",
                fromLink:
                    "Le lien du reçu indique où se trouve sa politique de confiance. Avant de l'épingler sur cet appareil, comparez ses mots et son ID avec l'avis officiel de l'élection.",
                inputLabel: "Politique de confiance ou son adresse",
                inputPlaceholder: "Collez la politique ou son adresse https",
                load: "Charger la politique",
                openFile: "Ouvrir un fichier de politique",
                loading: "Chargement de la politique de confiance…",
                pin: "Ils correspondent : épingler cette politique",
                reject: "Ils ne correspondent pas",
                rejected:
                    "N'utilisez pas cette politique. Prévenez l'autorité électorale et indiquez la politique de l'avis officiel de l'élection.",
                pinned: "Vérifié avec la politique de confiance que vous avez épinglée sur cet appareil.",
                unpin: "Oublier cette politique",
                draft: "Ceci est un brouillon de la politique, pas la politique scellée ; il ne peut donc pas être épinglé. Ses mots de brouillon :",
                invalid: "Ceci n'est pas une politique de confiance valide.",
                otherPolicy:
                    "Cette politique de confiance est valide, mais le reçu en concerne une autre.",
                ambiguous:
                    "Plusieurs politiques épinglées correspondent à cette référence ; elle n'est donc pas utilisée. Ouvrez plutôt le fichier du reçu.",
                problems: {
                    "empty": "Collez d'abord la politique de confiance ou son adresse.",
                    "too-large": "C'est trop volumineux pour être une politique de confiance.",
                    "not-https": "Seules les adresses commençant par https sont utilisées.",
                    "wrong-box": "Ceci est un reçu, pas une politique de confiance.",
                    "unrecognised": "Ceci n'est ni une politique de confiance ni son adresse.",
                    "unreadable": "Le fichier n'a pas pu être lu.",
                },
                fetchProblems: {
                    "not-https": "Seules les adresses commençant par https sont utilisées.",
                    "unreachable":
                        "La politique de confiance n'a pas pu être chargée. Vérifiez l'adresse ou collez plutôt la politique.",
                    "status":
                        "Le serveur n'a pas renvoyé la politique de confiance (statut {{status}}).",
                    "too-large": "C'est trop volumineux pour être une politique de confiance.",
                    "not-text": "L'adresse n'a pas renvoyé de politique de confiance.",
                },
            },
            ballot: {
                title: "Votre bulletin",
                description:
                    "Le reçu n'apparaît comme le vôtre que pour le bulletin que vous avez déposé. Saisissez l'ID de bulletin que le portail de vote vous a montré.",
                ballotId: "ID de bulletin",
                ballotIdPlaceholder: "Saisissez votre ID de bulletin",
                election: "ID de l'élection",
                electionPlaceholder: "Choisissez l'élection",
                event: "ID de l'événement électoral",
                check: "Vérifier le reçu",
                change: "Modifier l'ID de bulletin",
                invalid:
                    "Vérifiez l'ID de bulletin : il compte 64 caractères, des chiffres et des lettres de a à f.",
                lookedFor: "ID de bulletin recherché :",
            },
            result: {
                title: "Résultat",
                checking: "Vérification…",
                rounds: {
                    log: "Vérifié auprès des registres de l'élection. Dans la demi-heure qui suit, cette page interroge aussi les miroirs des registres, les hubs d'ancrage et les moniteurs, à un moment aléatoire, afin qu'ils ne puissent pas savoir quand vous avez ouvert votre reçu. Gardez la page ouverte, ou vérifiez tout maintenant.",
                    full: "Vérifié auprès des registres de l'élection, de leurs miroirs, des hubs d'ancrage et des moniteurs.",
                    offline:
                        "Les registres de cette élection sont privés ; le reçu a donc été vérifié avec ce qu'il contient.",
                },
                checkEverything: "Tout vérifier maintenant",
                checkAgain: "Vérifier à nouveau",
                retry: "Réessayer",
                problems: {
                    "policy-mismatch": "Cette référence concerne une autre politique de confiance.",
                    "unknown-log": "La politique de confiance n'a aucun registre de ce nom.",
                    "quiet-period":
                        "Le registre ne fournit pas de preuves pendant sa période de silence. Réessayez après {{time}}.",
                    "not-found":
                        "Le registre n'a pas encore cette entrée. Si vous avez voté récemment, réessayez plus tard.",
                    "entry-mismatch":
                        "L'entrée du registre à cette position n'est pas celle de la référence.",
                    "private-reference":
                        "Les registres de cette élection sont privés ; une référence ne peut donc pas être vérifiée ici. Ouvrez plutôt le fichier du reçu.",
                    "failed": "Le reçu n'a pas pu être vérifié. Réessayez.",
                },
                evidenceSaved:
                    "Le fichier du reçu a été enregistré. Il contient les objets signés qui se contredisent : un auditeur peut le vérifier, et en tirer le fichier de preuves, avec le vérificateur Vitrine.",
                advice: {
                    failed: "Une vérification a échoué : des données signées se contredisent ou contredisent la politique de confiance. Enregistrez les preuves et remettez-les, avec le fichier du reçu, à l'autorité électorale et aux observateurs que nomme la politique de confiance.",
                    late: "Quelque chose qui devrait déjà être là ne l'est pas encore. Ce n'est peut-être qu'une lenteur : vérifiez à nouveau plus tard et, si le retard persiste, prévenez l'autorité électorale. Conservez le fichier du reçu.",
                },
            },
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
            explicit: {
                notAllowed:
                    "Vote marqué explicitement comme invalide mais le scrutin ne le permet pas",
            },
        },
    },
}

export default frenchTranslation
