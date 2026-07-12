// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const frenchTranslation: TranslationType = {
    translations: {
        common: {
            goBack: "Revenir",
            showMore: "Afficher plus",
            showLess: "Afficher moins",
        },
        candidatesList: {
            collapseToggle: "Basculer la liste {{listTitle}}",
            showCandidates: "Afficher les candidats",
            hideCandidates: "Masquer les candidats",
            selectedCandidate: "{{count}} candidat sélectionné",
            selectedCandidates: "{{count}} candidats sélectionnés",
            expandAll: "Tout développer",
            collapseAll: "Tout réduire",
        },
        breadcrumbSteps: {
            electionList: "Élections",
            ballot: "Bulletin de vote",
            review: "Révision",
            confirmation: "Confirmer",
            audit: "Auditer",
        },
        footer: {
            poweredBy: "Propulsé par <1></1>",
        },
        votingScreen: {
            backButton: "Retour",
            reviewButton: "Suivant",
            clearButton: "Effacer la sélection",
            ballotHelpDialog: {
                title: "À propos de l'écran de vote",
                content:
                    "Cet écran affiche le vote pour lequel vous êtes éligible. Activez la case à droite pour sélectionner un Candidat/Réponse. Pour réinitialiser, cliquez sur “<b>Effacer la sélection</b>”, pour continuer, cliquez sur “<b>Suivant</b>”.",
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
            warningDialog: {
                title: "Vérifiez votre bulletin",
                content:
                    "Votre bulletin contient des sélections qui peuvent nécessiter votre attention (comme sélectionner moins d'options que permis). Votre bulletin est valide et sera compté tel que soumis.",
                ok: "Retour et vérification",
                continue: "Continuer",
                cancel: "Annuler",
            },
        },
        startScreen: {
            startButton: "Commencer à voter",
            declineToVoteButton: "S’abstenir de voter",
            declineToVoteDialog: {
                title: "Confirmer l’abstention de vote",
                content:
                    "Êtes-vous sûr de vouloir vous abstenir de voter ?<br />Vous accéderez directement à la révision et votre statut de participation sera enregistré comme <b>S’est abstenu de voter</b>.",
                continue: "S’abstenir de voter",
                cancel: "Annuler",
            },
            instructionsTitle: "Comment voter",
            instructionsDescription: "Suivez ces étapes pour voter :",
            step1Title: "1. Sélectionnez votre option de vote",
            step1Description:
                "Sélectionnez vos candidats et répondez aux questions. Vous pouvez modifier votre bulletin jusqu'à être prêt.",
            step2Title: "2. Révisez votre bulletin",
            step2Description:
                "Nous chiffrerons votre bulletin et vous montrerons une révision finale. Vous recevrez un ID de suivi unique.",
            step3Title: "3. Envoyez votre vote",
            step3Description:
                "Envoyez votre bulletin pour qu'il soit enregistré, ou choisissez d'auditer pour confirmer qu'il a été chiffré correctement.",
        },
        reviewScreen: {
            title: "Révisez votre vote",
            description:
                "Cliquez sur “<b>Modifier votre vote</b>” pour changer vos sélections, “<b>Envoyer le vote</b>” pour confirmer, ou “<b>Vérifier le bulletin</b>” pour l'auditer.",
            descriptionNoAudit:
                "Cliquez sur “<b>Modifier votre vote</b>” pour changer vos sélections, ou “<b>Envoyer le vote</b>” pour confirmer.",
            backButton: "Modifier votre vote",
            castBallotButton: "Envoyer le vote",
            auditButton: "Vérifier le bulletin",
            reviewScreenHelpDialog: {
                title: "À propos de l'écran de révision",
                content: "Cet écran vous permet de réviser vos sélections avant de voter.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Vote non émis",
                content:
                    "<p>Vous êtes sur le point de copier le Localisateur de Vote, mais <b>votre vote n'a pas encore été émis</b>. Si vous tentez de rechercher le Localisateur de Vote, vous ne le trouverez pas.</p><p>La raison pour laquelle nous affichons le Localisateur de Vote à ce moment est pour que vous puissiez auditer la correction du vote chiffré avant de l'émettre. Si c'est la raison pour laquelle vous souhaitez copier le Localisateur de Vote, procédez à sa copie puis auditez votre vote.</p>",
                ok: "Je comprends que mon vote n'a pas été émis",
                cancel: "Annuler",
            },
            auditBallotHelpDialog: {
                title: "Voulez-vous vraiment Auditer votre bulletin ?",
                content:
                    "<p>L'audit du bulletin l'invalidera et vous devrez recommencer le processus de vote si vous souhaitez émettre votre vote. Le processus d'audit du bulletin permet de vérifier qu'il est correctement codé. Ce processus nécessite des connaissances techniques importantes, donc il n'est pas recommandé si vous ne savez pas ce que vous faites.</p><p><b>Si vous souhaitez émettre votre vote, cliquez sur <u>Annuler</u> pour revenir à l'écran de révision du vote.</b></p>",
                ok: "Oui, je veux invalider mon bulletin pour l'auditer",
                cancel: "Annuler",
            },
            confirmCastVoteDialog: {
                title: "Êtes-vous sûr de vouloir voter?",
                content: "Après confirmation, votre vote sera émis.",
                ok: "Oui, je veux voter",
                cancel: "Annuler",
            },
            error: {
                NETWORK_ERROR:
                    "Un problème de réseau est survenu. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                UNABLE_TO_FETCH_DATA:
                    "Un problème est survenu lors de la récupération des données. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                LOAD_ELECTION_EVENT:
                    "Impossible de charger l'événement électoral. Veuillez réessayer plus tard.",
                CAST_VOTE:
                    "Une erreur inconnue est survenue lors du vote. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                CAST_VOTE_CheckStatusFailed:
                    "L'élection ne permet pas de voter. L'élection peut être clôturée, archivée ou vous essayez peut-être de voter en dehors de la période de grâce.",
                CAST_VOTE_AreaNotFound:
                    "Une erreur est survenue lors du vote : Zone introuvable. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                CAST_VOTE_InternalServerError:
                    "Une erreur interne est survenue lors du vote. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                CAST_VOTE_QueueError:
                    "Un problème est survenu lors du traitement de votre vote. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                CAST_VOTE_Unauthorized:
                    "Vous n'êtes pas autorisé à voter. Veuillez contacter le support pour obtenir de l'aide.",
                CAST_VOTE_ElectionEventNotFound:
                    "L'événement électoral n'a pas pu être trouvé. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                CAST_VOTE_ElectoralLogNotFound:
                    "Votre enregistrement de vote n'a pas pu être trouvé. Veuillez contacter le support pour obtenir de l'aide.",
                CAST_VOTE_CheckPreviousVotesFailed:
                    "Une erreur est survenue lors de la vérification de votre statut de vote. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                CAST_VOTE_GetClientCredentialsFailed:
                    "Échec de la vérification de vos informations d'identification. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                CAST_VOTE_GetAreaIdFailed:
                    "Une erreur est survenue lors de la vérification de votre zone de vote. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                CAST_VOTE_GetTransactionFailed:
                    "Une erreur est survenue lors du traitement de votre vote. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                CAST_VOTE_DeserializeBallotFailed:
                    "Une erreur est survenue lors de la lecture de votre bulletin de vote. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                CAST_VOTE_DeserializeContestsFailed:
                    "Une erreur est survenue lors de la lecture de vos sélections. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                CAST_VOTE_PokValidationFailed:
                    "Échec de la validation de votre vote. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                CAST_VOTE_UuidParseFailed:
                    "Une erreur est survenue lors du traitement de votre demande. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                CAST_VOTE_unexpected:
                    "Une erreur inconnue est survenue lors du vote. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                CAST_VOTE_timeout:
                    "Erreur de délai pour voter. Veuillez réessayer ultérieurement ou contacter l'assistance pour obtenir de l'aide.",
                CAST_VOTE_InsertFailedExceedsAllowedRevotes:
                    "Vous avez dépassé la limite de votes. Veuillez réessayer ultérieurement ou contacter l'assistance pour obtenir de l'aide.",
                CAST_VOTE_CheckRevotesFailed:
                    "Vous avez dépassé le nombre autorisé de votes. Veuillez réessayer ultérieurement ou contacter l'assistance pour obtenir de l'aide.",
                CAST_VOTE_CheckVotesInOtherAreasFailed:
                    "Vous avez déjà voté dans une autre zone. Veuillez réessayer ultérieurement ou contacter l'assistance pour obtenir de l'aide.",
                CAST_VOTE_UnknownError:
                    "Une erreur inconnue est survenue lors du vote. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                NO_BALLOT_SELECTION:
                    "L'état de sélection pour cette élection est introuvable. Veuillez vous assurer que vous avez sélectionné vos choix correctement ou contactez le support.",
                NO_BALLOT_STYLE:
                    "Le style du bulletin de vote n'est pas disponible. Veuillez contacter le support.",
                NO_AUDITABLE_BALLOT:
                    "Aucun bulletin de vote vérifiable n'est disponible. Veuillez contacter le support.",
                INCONSISTENT_HASH:
                    "Une erreur liée au processus de hachage du bulletin de vote est survenue. Le BallotId: {{ballotId}} n'est pas cohérent avec le Hash du bulletin vérifiable: {{auditableBallotHash}}. Veuillez signaler ce problème au support.",
                ELECTION_EVENT_NOT_OPEN:
                    "L'événement électoral est fermé. Veuillez contacter le support.",
                PARSE_ERROR:
                    "Une erreur est survenue lors de l'analyse du bulletin de vote. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                DESERIALIZE_AUDITABLE_ERROR:
                    "Une erreur est survenue lors de la désérialisation du bulletin vérifiable. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                DESERIALIZE_HASHABLE_ERROR:
                    "Une erreur est survenue lors de la désérialisation du bulletin haché. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                CONVERT_ERROR:
                    "Une erreur est survenue lors de la conversion du bulletin de vote. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                SERIALIZE_ERROR:
                    "Une erreur est survenue lors de la sérialisation du bulletin de vote. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                UNKNOWN_ERROR:
                    "Une erreur est survenue. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                REAUTH_FAILED:
                    "L'authentification a échoué. Veuillez réessayer ou contacter le support pour obtenir de l'aide.",
                SESSION_EXPIRED: "Votre session a expiré. Veuillez recommencer depuis le début.",
                CAST_VOTE_BallotIdMismatch:
                    "L'identifiant du bulletin ne correspond pas à celui du vote exprimé.",
                SESSION_STORAGE_ERROR:
                    "Le stockage de session n'est pas disponible. Veuillez réessayer ou contacter le support.",
                PARSE_BALLOT_DATA_ERROR:
                    "Une erreur s'est produite lors de l'analyse des données du bulletin de vote. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                NOT_VALID_BALLOT_DATA_ERROR:
                    "Les données du bulletin de vote ne sont pas valides. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                FETCH_DATA_TIMEOUT_ERROR:
                    "Erreur de délai d'attente pour récupérer les données. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                TO_HASHABLE_BALLOT_ERROR:
                    "Erreur lors de la conversion en bulletin de vote hashable. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                INTERNAL_ERROR:
                    "Une erreur interne s'est produite lors du vote. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
            },
            declineToVote: "S’abstenir de voter",
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
                title: "À propos de l'écran de confirmation",
                content:
                    "Cet écran montre que votre vote a été émis correctement. Vous pouvez vérifier que le bulletin a été stocké dans l'urne.",
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
                title: "À propos du Localisateur de Bulletin",
                content:
                    "Le Localisateur de Bulletin est un code qui vous permet de retrouver votre bulletin dans l'urne, ce Localisateur est unique et ne contient aucune information sur vos sélections.",
                ok: "OK",
            },
            ballotIdDemoHelpDialog: {
                title: "À propos de l'identifiant de bulletin",
                content:
                    "L'identifiant de bulletin de vote est un code qui vous permet de retrouver votre bulletin dans l'urne. Cet identifiant est unique et ne contient aucune information sur vos choix.",
                ok: "OK",
            },
            errorDialogPrintBallotReceipt: {
                title: "Erreur",
                content: "Une erreur s'est produite, veuillez réessayer",
                ok: "Accepter",
            },
            demoQRText: "Le suivi des bulletins est désactivé en mode démo",
        },
        auditScreen: {
            printButton: "Imprimer",
            restartButton: "Démarrer le vote",
            title: "Vérifiez votre Bulletin",
            description: "Pour vérifier votre bulletin, suivez les étapes suivantes :",
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
                "<VerifierLink>Accédez au vérificateur de vote</VerifierLink>, qui s'ouvrira dans un nouvel onglet de votre navigateur.",
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
            title: "Élections",
            description: "Sélectionnez l'élection pour laquelle vous souhaitez voter",
            chooserHelpDialog: {
                title: "À propos de la liste des élections",
                content:
                    "Cet écran montre la liste des élections dans lesquelles vous pouvez voter. L'accès est possible uniquement si la période de vote est ouverte.",
                ok: "OK",
            },
            noResults: "Aucun bulletin disponible pour le moment.",
            demoDialog: {
                title: "Bureau de vote de démonstration",
                content:
                    "Vous entrez dans un bureau de vote de démonstration. <strong>Votre vote ne sera pas compté.</strong> Uniquement à des fins de démonstration.",
                ok: "Je comprends que mon vote ne sera pas compté",
            },
            errors: {
                noVotingArea: "Zone de vote non assignée. Veuillez réessayer plus tard.",
                networkError:
                    "Il y a eu un problème de réseau. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                unableToFetchData:
                    "Il y a eu un problème pour récupérer les données. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                noElectionEvent:
                    "L'événement électoral n'existe pas. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                ballotStylesEmlError:
                    "Il y a eu une erreur avec la publication du style de bulletin. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                obtainingElectionFromID:
                    "Il y a eu une erreur pour obtenir les élections associées aux identifiants d'élection suivants : {{electionIds}}. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
            },
            alerts: {
                noElections:
                    "Il n'y a pas d'élections pour lesquelles vous pouvez voter. Cela pourrait être parce que la zone n'a aucun concours associé. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
                electionEventNotPublished:
                    "L'événement électoral n'a pas encore été publié. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
            },
        },
        errors: {
            encoding: {
                notEnoughChoices: "Pas assez de choix à décoder",
                writeInChoiceOutOfRange: "Choix écrit hors de la plage : {{index}}",
                writeInNotEndInZero: "L'écrit ne se termine pas par 0",
                writeInCharsExceeded:
                    "L'écrit dépasse la longueur maximale de {{numCharsExceeded}} caractères. Veuillez le raccourcir.",
                bytesToUtf8Conversion:
                    "Erreur lors de la conversion de l'écrit de bytes en chaîne UTF-8 : {{errorMessage}}",
                ballotTooLarge: "Bulletin plus grand que prévu",
            },
            implicit: {
                selectedMax:
                    "Surcote : le nombre de choix sélectionnés {{numSelected}} est supérieur au maximum {{max}}",
                selectedMin:
                    "Le nombre de choix sélectionnés {{numSelected}} est inférieur au minimum {{min}}",
                maxSelectionsPerType:
                    "Le nombre de choix sélectionnés {{numSelected}} pour la liste {{type}} est supérieur au maximum {{max}}",
                underVote:
                    "Sous-vote : le nombre de choix sélectionnés {{numSelected}} est inférieur au maximum {{max}}",
                overVoteDisabled:
                    "Maximum atteint : vous avez sélectionné le maximum de {{numSelected}} choix. Pour modifier votre sélection, veuillez d'abord désélectionner une autre option.",
                blankVote: "Vote blanc : 0 choix sélectionnés",
            },
            explicit: {
                notAllowed:
                    "Le bulletin est marqué comme explicitement invalide, mais la question ne le permet pas",
                alert: "Cette sélection sera comptée comme un vote invalide",
            },
            page: {
                oopsWithStatus: "Oups ! {{status}}",
                oopsWithoutStatus: "Oups ! Erreur inattendue",
                somethingWrong: "Quelque chose s'est mal passé.",
                certAuthFailedTitle: "Échec de l'Authentification par Certificat",
                certAuthFailedMessage:
                    "Votre certificat n'a pas pu être vérifié. Veuillez vérifier que vous utilisez un certificat de votant valide et réessayez.",
            },
        },
        materials: {
            common: {
                label: "Matériaux de Support",
                back: "Revenir à la liste des élections",
                close: "Fermer",
                preview: "Aperçu",
            },
        },
        ballotLocator: {
            title: "Trouvez votre Bulletin",
            titleResult: "Résultat de la recherche de votre Bulletin",
            description: "Vérifiez que votre bulletin a été émis correctement",
            locate: "Trouvez votre Bulletin",
            locateAgain: "Trouvez un autre Bulletin",
            found: "Votre ID de Bulletin {{ballotId}} a été trouvé",
            notFound: "Votre ID de Bulletin {{ballotId}} n'a pas été trouvé",
            ambiguous:
                "Plusieurs de vos bulletins correspondent à {{ballotId}}. Utilisez l'ID complet du bulletin.",
            contentDesc: "Voici le contenu de votre Bulletin : ",
            wrongFormatBallotId: "Format incorrect pour l'ID du Bulletin",
            ballotIdNotFoundAtFilter:
                "Non trouvé, veuillez verifier que l'ID du Bulletin soit correct et appartenir a cet utilisateur.",
            filterByBallotId: "Filtrez par ID de Bulletin",
            totalBallots: "Total: {{total}}",
            steps: {
                lookup: "Trouvez votre Bulletin",
                result: "Résultat",
            },
            titleHelpDialog: {
                title: "À propos du Localisateur de Bulletin",
                content:
                    "Le Localisateur de Bulletin vous permet de saisir l'ID du Bulletin pour trouver votre vote et confirmer qu'il a été enregistré correctement.",
                ok: "OK",
            },
            tabs: {
                logs: "Logs",
                ballotLocator: "Recherche de Bulletin",
            },
            column: {
                statement_kind: "Type",
                statement_timestamp: "Marque de temps",
                username: "Nom d'utilisateur",
                ballot_id: "ID de Bulletin",
                message: "Message",
            },
        },
    },
}

export default frenchTranslation
