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
            step2: "Étape 2 : Insérez votre identifiant de suivi.",
            description2:
                "Collez l'identifiant de suivi complet depuis le Portail de Vote : le bouton de copie à côté de votre identifiant de suivi le copie.",
            ballotIdHelpDialog: {
                title: "Information : Votre identifiant de suivi",
                ok: "OK",
                content:
                    "Collez l'identifiant de suivi complet fourni sur le Portail de Vote. Le bouton de copie à côté de celui-ci le copie.",
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
            verificationErrorTitle: "La vérification du vote a échoué",
            unpublishedStyleErrorDescription:
                "Ce vote auditable ne correspond à aucun bulletin publié pour cet événement électoral ; il ne peut donc pas être vérifié.",
            unavailableStyleErrorDescription:
                "Les bulletins publiés pour cet événement électoral n'ont pas pu être chargés ; ce vote auditable ne peut donc pas encore être vérifié. Veuillez réessayer.",
            styleErrorDescription:
                "Le style de vote de ce vote auditable n'est pas celui publié pour l'élection : sa clé publique ou ses scrutins et candidats diffèrent, ou il n'est pas publié. Ce vote n'est pas fiable.",
            legacyBallotWarning:
                "Ce vote auditable utilise un ancien format dont l'identifiant de suivi ne couvre pas le style de vote. Son identifiant ne peut pas confirmer que le vote a été chiffré avec la clé publiée de l'élection.",
            useSampleLink: "Utiliser un vote exemple",
            nextButton: "Continuer",
            fileUploaded: "Chargé",
            receiptTitle: "Reçu de transparence",
            receiptDescription:
                "Cette élection publie un reçu pour chaque bulletin déposé. Vérifiez le vôtre : qu'il figure dans le registre public, inchangé et identique pour tous.",
            receiptButton: "Vérifier un reçu",
        },
        "confirmationScreen": {
            ballotIdTitle: "Identifiant de suivi",
            ballotIdDescription:
                "Ci-dessous, le système affiche l'identifiant de suivi du bulletin décodé et celui que vous avez fourni.",
            ballotIdError: "Ne correspond pas à l'identifiant de suivi décodé.",
            decodedBallotId: "Identifiant de suivi décodé",
            decodedBallotIdHelpDialog: {
                title: "Information : Identifiant de suivi décodé",
                ok: "OK",
                content:
                    "Ceci est l'identifiant de suivi extrait du fichier du bulletin auditable décodé que vous avez fourni.",
            },
            yourBallotId: "L'identifiant de suivi que vous avez fourni",
            userBallotIdHelpDialog: {
                title: "Information : L'identifiant de suivi que vous avez fourni",
                ok: "OK",
                content:
                    "Ceci est l'identifiant de suivi que vous avez écrit à l'étape précédente et que vous avez pris de la Cabine de Vote.",
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
                    "Le lien du reçu indique où se trouve sa politique de confiance. Avant de l'épingler sur cet appareil, comparez les quatre premiers groupes de son ID de politique avec l'avis officiel de l'élection.",
                compareGroups: "Comparez les quatre premiers groupes, en gras, avec l'avis. Les outils comparent les 13.",
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
                draft: "Ceci est un brouillon de la politique, pas la politique scellée ; il ne peut donc pas être épinglé. Son ID de brouillon :",
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
                    "Le reçu n'apparaît comme le vôtre que pour le bulletin que vous avez déposé. Collez l'identifiant de suivi complet que le portail de vote vous a montré.",
                election: "ID de l'élection",
                electionPlaceholder: "Choisissez l'élection",
                event: "ID de l'événement électoral",
                check: "Vérifier le reçu",
                change: "Modifier l'identifiant de suivi",
                lookedFor: "Identifiant de suivi recherché",
                fullId: "Identifiant de suivi complet",
                stage: "Étape du bulletin",
                stages: {received: "Reçu", cast: "Déposé", audited: "Audité"},
            },
            result: {
                title: "Résultat",
                rounds: {
                    log: "Vérifié auprès des registres de l'élection. Dans la demi-heure qui suit, cette page interroge aussi les miroirs des registres, les hubs d'ancrage et les moniteurs, à un moment aléatoire, afin qu'ils ne puissent pas savoir quand vous avez vérifié.",
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
            status: {
                checking: {
                    head: "Vérification…",
                    sub: "Votre navigateur vérifie que votre bulletin figure dans le registre public.",
                },
                recorded: {
                    head: "Dans le registre public",
                    sub: "Votre navigateur a trouvé votre bulletin dans le registre public. Les témoins le confirment ensuite.",
                },
                witnessed: {
                    head: "Confirmé par des témoins",
                    sub: "Des témoins indépendants ont confirmé le registre. Les hubs publics sont interrogés ensuite, dans la demi-heure.",
                    subLater:
                        "Des témoins indépendants ont confirmé le registre. Les ancrages publics suivent dans les 10 minutes ; inutile d'attendre.",
                },
                verified: {
                    head: "Vérifié",
                    sub: "Votre bulletin figure dans le registre public, confirmé par des témoins indépendants et ancré publiquement.",
                },
                late: {
                    head: "Plus long que prévu",
                    sub: "Votre bulletin figure dans le registre public et est confirmé par des témoins, mais ses ancrages publics sont en retard.",
                    subOther: "Certaines des étapes ci-dessous sont plus longues que prévu.",
                },
                problem: {
                    head: "Le registre ne correspond pas",
                    sub: "L'entrée n° {{index}} du registre public n'est pas celle de ce reçu.",
                    subOther:
                        "Le registre public, ce reçu et votre identifiant de suivi ne concordent pas tous.",
                },
                damaged: {
                    head: "Vérification impossible",
                    sub: "Certaines données semblent endommagées : le reçu ou la réponse du serveur. Cela ne prouve pas que quelqu'un ait triché.",
                },
                evidence:
                    "Votre navigateur a conservé les preuves : ce reçu et l'entrée signée du registre. Envoyez-les à l'autorité électorale ou à un observateur ; n'importe qui peut les vérifier.",
                saveEvidence: "Enregistrer les preuves",
                howToReport: "Comment le signaler",
                captionChecking:
                    "Votre navigateur vérifie le registre public, avec un code que chacun peut inspecter.",
                captionChecked:
                    "Vérifié par votre navigateur, avec un code que chacun peut inspecter.",
                captionProblem:
                    "Constaté par votre navigateur, avec un code que chacun peut inspecter.",
                rows: {
                    record: "Dans le registre public",
                    witnesses: "Confirmé par des témoins",
                    anchors: "Ancré publiquement",
                    recordDone: "Entrée n° {{index}}. Votre navigateur en a vérifié la preuve.",
                    recordDoneNoIndex: "Votre navigateur en a vérifié la preuve.",
                    witnessesDone:
                        "{{signed}} témoins indépendants sur {{total}} l'ont signé ; la politique en exige {{needed}}.",
                    witnessesDoneAny: "Des témoins indépendants l'ont signé.",
                    hubs_one: "Dans {{count}} hub public",
                    hubs_many: "Dans {{count}} de hubs publics",
                    hubs_other: "Dans {{count}} hubs publics",
                    browser: ", et là où les navigateurs vérifient",
                    standIn: ", via un hub de remplacement pour l'instant",
                    unused: "Non utilisé par la politique de cette élection.",
                    checking: "Votre navigateur vérifie…",
                    anchorsActive: "Interrogé dans la demi-heure, à un moment aléatoire.",
                    anchorsPending: "Pas encore ancré. Vérifiez à nouveau dans quelques minutes.",
                    waiting: "En attente de l'étape précédente.",
                    lateAnchors: "Les hubs auraient dû l'ancrer avant {{time}}.",
                    lateOther: "Cela devrait déjà être fait.",
                    recordFailed: "L'entrée n° {{index}} du registre diffère de votre reçu.",
                    failed: "Cela ne concorde pas avec votre reçu et votre identifiant de suivi.",
                    damaged: "Vérification impossible : les données semblent endommagées.",
                    skipped: "Non vérifié, car le registre ne correspond pas.",
                    skippedDamaged:
                        "Non vérifié, car une étape précédente n'a pas pu être vérifiée.",
                    statusChecking: "Vérification…",
                    withinSeconds: "En quelques secondes",
                    within10: "Sous 10 min",
                    within30: "Sous 30 min",
                    statusLate: "En retard",
                    statusMismatch: "Écart",
                    statusDamaged: "Endommagé",
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
        "ballotIdField": {
            label: "Identifiant de suivi",
            placeholder: "Collez votre identifiant de suivi complet",
            help: "Les espaces sont facultatives.",
            shortError:
                "C'est l'ID court. Collez l'identifiant de suivi complet : la vérification le compare en entier.",
            formatError:
                "Vérifiez l'identifiant de suivi : il compte 64 caractères, des chiffres et les lettres de a à f.",
        },
        "auditedScreen": {
            title: "Bulletin",
            tag: "Audité",
            loading: "Vérification du bulletin dans le registre public…",
            retry: "Réessayer",
            header: {
                strip: {
                    withOperator:
                        "Un hub de transparence, géré par {{operator}}. Ce n’est pas le site de l’élection, et il ne vous demande jamais de voter, de vous connecter ni de payer.",
                    noOperator:
                        "Un hub de transparence. Ce n’est pas le site de l’élection, et il ne vous demande jamais de voter, de vous connecter ni de payer.",
                },
                subline: {
                    withOperator: "Géré par {{operator}} · un hub Vitrine",
                    noOperator: "Un hub Vitrine",
                },
                unnamed: "Hub de transparence",
            },
            trackingId: {
                full: "Identifiant de suivi complet",
                copy: "Copier l’identifiant de suivi complet",
                copied: "Identifiant de suivi copié",
                copyText: "Copier",
                copiedText: "Copié",
                selected: "Sélectionné. Appuyez sur Ctrl+C, ou ⌘C sur Mac, pour le copier.",
            },
            checks: {
                title: "Vérifié dans votre navigateur",
                record: {
                    title: "Dans le registre public",
                    text: "Entrée n° {{index}}, auditée à {{time}}. Jamais déposée.",
                    waiting: "Recherche de son audit dans le registre de l’élection.",
                    failed: "La preuve du registre pour cet audit n’est pas valide.",
                },
                secret: {
                    title: "Le secret correspond au bulletin",
                    text: "Le chiffrer à nouveau avec lui donne le même bulletin, octet par octet.",
                    waiting: "Vérifié une fois le registre vérifié.",
                    failed: "Le chiffrer à nouveau avec lui ne donne pas ce bulletin.",
                },
                decoded: {
                    title: "Décodé",
                    text: "Les choix à gauche, d’après le modèle de bulletin publié de l’élection.",
                    waiting: "Seulement si le secret correspond.",
                    failed: "Il n’a pas pu être décodé.",
                },
            },
            note: {
                named: "Avec du code que chacun peut inspecter. {{hub}} ne fait que le stocker et le servir, et ne demande jamais ce que vous avez choisi. Votre réponse reste dans ce navigateur.",
                unnamed:
                    "Avec du code que chacun peut inspecter. Ce hub ne fait que le stocker et le servir, et ne demande jamais ce que vous avez choisi. Votre réponse reste dans ce navigateur.",
            },
            states: {
                "bad-link": {
                    title: "Ce lien est incomplet",
                    text: "Il lui faut l’identifiant de suivi complet du bulletin et l’identifiant de politique de l’élection. Ouvrez-le à nouveau depuis le site de vote.",
                },
                "no-policy": {
                    title: "La politique de l’élection est introuvable",
                    text: "Cette page ne vérifie un bulletin qu’avec la politique indiquée par le lien, et n’a pas pu l’obtenir de l’élection. Réessayez plus tard.",
                },
                "other-election": {
                    title: "Ce lien concerne une autre élection",
                    text: "La politique indiquée par le lien ne couvre pas cette élection : le bulletin ne peut donc pas être vérifié avec elle.",
                },
                "not-found": {
                    title: "Aucun bulletin avec cet identifiant de suivi",
                    text: "Le registre public de l’élection ne contient aucun bulletin avec cet identifiant de suivi. Vérifiez que le lien est complet, ou réessayez dans une minute.",
                },
                "cast": {
                    title: "Ce bulletin a été déposé, pas audité",
                    text: "Seul un bulletin audité peut être décodé. Un bulletin déposé reste chiffré : personne ne voit comment il a été voté, ni ici ni ailleurs.",
                },
                "pending": {
                    title: "Ce bulletin n’est pas encore audité",
                    text: "Le registre public contient ce bulletin, mais pas encore son audit. Réessayez dans une minute.",
                },
                "no-content": {
                    title: "Le bulletin audité n’est pas encore publié",
                    text: "Le registre public contient l’audit de ce bulletin, mais pas le bulletin et son secret : il ne peut donc pas encore être décodé ici. Réessayez plus tard.",
                },
                "unavailable": {
                    title: "Le bulletin n’a pas pu être vérifié",
                    text: "Cette page n’a pas pu joindre le registre public de l’élection. Réessayez plus tard.",
                },
                "record-failed": {
                    title: "Le registre ne prouve pas cet audit",
                    text: "Le registre de l’élection a fourni une preuve de l’audit de ce bulletin qui n’est pas valide : rien n’est décodé. Informez l’autorité électorale.",
                },
                "mismatch": {
                    title: "Échec de l’audit",
                    text: "Le bulletin du registre public n’est pas celui que désigne son identifiant de suivi, ou le secret publié par le portail ne le reproduit pas. Ne votez pas sur l’appareil depuis lequel vous avez audité, et informez l’autorité électorale.",
                },
                "unchecked": {
                    title: "L’audit n’a pas pu être vérifié",
                    text: "Cette page n’a pas pu vérifier ce bulletin audité : aucun choix n’est affiché. Réessayez plus tard, ou sur un autre appareil.",
                },
            },
            compare: {
                kicker: "Audit du bulletin",
                title: "Vérifiez que votre bulletin a été chiffré comme vous l’avez choisi",
                note: "Voici le bulletin que vous avez audité, décodé par votre navigateur. Vous seul savez ce que vous avez choisi : vous seul pouvez donc faire cette vérification.",
                steps: {
                    label: "Comment fonctionne la vérification",
                    encrypted: {
                        title: "Chiffré sur votre appareil",
                        text: "Le portail a chiffré vos choix et a donné au bulletin son identifiant de suivi, avant que vous décidiez de l’auditer.",
                    },
                    decoded: {
                        title: "Décodé dans votre navigateur",
                        text: "Le bulletin et le secret publié par le portail viennent tous deux du registre public. Cette page a décodé l’un avec l’autre.",
                    },
                    compare: {
                        title: "Vous comparez",
                        text: "Regardez les choix ci-dessous. Sont-ils ceux que vous avez faits ?",
                    },
                },
                choices: "Choix décodés de ce bulletin",
                question: "Correspondent-ils à ce que vous avez choisi ?",
                yes: "Oui, ils correspondent",
                no: "Non, quelque chose est différent",
                change: "Changer ma réponse",
                match: {
                    title: "Votre bulletin a été chiffré comme vous l’avez choisi",
                    text: "Ce bulletin audité est maintenant épuisé : il n’a jamais été déposé et ne sera jamais compté. Retournez voter. Votre vote recevra un nouvel identifiant de suivi, et vous pouvez auditer à nouveau avant si vous le souhaitez.",
                    back: "Retour au site de vote",
                },
                mismatch: {
                    title: "Ne votez pas sur cet appareil",
                    text: "Ce qui a été chiffré n’est pas ce que vous avez choisi : cet appareil ou le portail de vote a peut-être été altéré. Votre navigateur a conservé la preuve : le bulletin chiffré du registre public, le secret révélé par le portail et ce qu’il décode.",
                    report: "Signalez-le à l’autorité électorale",
                    reportNoContact: "Signalez-le à l’autorité électorale.",
                    fromPolicy: "Tiré de la politique publiée de l’élection, pas de cette page.",
                    fromSettings: "Tiré des paramètres de l’élection.",
                    then: "Votez ensuite depuis un autre appareil.",
                    save: "Enregistrer la preuve",
                    email: "Écrire à l’autorité",
                    emailSubject: "Mon bulletin audité ne se décode pas en mes choix",
                    emailBody:
                        "J’ai audité un bulletin lors de cette élection et les choix décodés par mon navigateur ne sont pas ceux que j’ai faits.\n\nIdentifiant de suivi : {{id}}\n\nJ’ai enregistré la preuve dans un fichier et je peux l’envoyer.",
                },
                how: "Comment ça marche ?",
                faq: {
                    why: {
                        question: "Pourquoi auditer ?",
                        answer: "Votre vote est chiffré sur votre appareil : personne ne peut le lire, pas même vous. L’audit vous permet de vérifier que le portail a chiffré ce que vous avez choisi avant que vous votiez pour de bon.",
                    },
                    catch: {
                        question: "Pourquoi cela détecte-t-il la triche ?",
                        answer: "Le portail chiffre avant que vous décidiez d’auditer : il ne peut pas savoir quels bulletins seront vérifiés. S’il modifiait des votes, chaque électeur qui audite pourrait s’en apercevoir.",
                    },
                    spent: {
                        question: "Pourquoi ce bulletin ne peut-il pas être compté ?",
                        answer: "Son secret révélé, le bulletin montre comment vous avez voté. Pour garder votre vote secret, un bulletin audité n’est jamais déposé.",
                    },
                    differ: {
                        question: "Et s’ils ne correspondent pas ?",
                        answer: "Ne votez pas sur cet appareil. Enregistrez la preuve et signalez-la à l’autorité électorale, dont le contact provient de la politique de l’élection. Votez ensuite depuis un autre appareil.",
                    },
                    tip: "Pour la vérification la plus solide, ouvrez cette page sur un autre appareil que celui avec lequel vous avez voté, par exemple votre téléphone. Un appareil qui aurait modifié votre bulletin pourrait aussi modifier ce qu’il vous montre ici.",
                },
            },
        },
    },
}

export default frenchTranslation
