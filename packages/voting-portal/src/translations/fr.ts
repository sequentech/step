// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const frenchTranslation: TranslationType = {
    translations: {
        common: {
            goBack: "Retour",
            showMore: "Afficher plus",
            showLess: "Afficher moins",
        },
        a11y: {
            skipToContent: "Aller au contenu principal",
            helpAbout: "Aide à propos de {{topic}}",
            copyToClipboard: "Copier {{label}} dans le presse-papiers",
            previewMaterial: "Aperçu de {{title}}",
            ballotsTable: "Bulletins",
            ballotLocatorTabs: "Sections du localisateur de bulletin",
            ballotIdLabel: "Identifiant de vote",
            votingProgress: "Progression du vote",
            stepOf: "Étape {{current}} sur {{total}}",
            selectUpTo_one: "Sélectionnez jusqu'à {{count}} option",
            selectUpTo_other: "Sélectionnez jusqu'à {{count}} options",
            selectExactly_one: "Sélectionnez {{count}} option",
            selectExactly_other: "Sélectionnez {{count}} options",
            selectBetween: "Sélectionnez entre {{min}} et {{max}} options",
        },
        candidatesList: {
            collapseToggle: "Masquer la liste {{listTitle}}",
            showCandidates: "Afficher les candidats",
            hideCandidates: "Masquer les candidats",
            selectedCandidate: "{{count}} candidat sélectionné",
            selectedCandidates: "{{count}} candidats sélectionnés",
            expandAll: "Tout afficher",
            collapseAll: "Tout réduire",
        },
        breadcrumbSteps: {
            electionList: "Liste des Élections",
            ballot: "Bulletin de vote",
            review: "Révision",
            confirmation: "Confirmation",
            audit: "Auditer",
        },
        footer: {
            poweredBy: "Développé par <1></1>",
        },
        votingScreen: {
            backButton: "Retour",
            reviewButton: "Suivant",
            clearButton: "Effacer la sélection",
            ballotHelpDialog: {
                title: "Information : Écran de vote",
                content:
                    "Cet écran affiche le vote auquel vous êtes éligible. Vous pouvez effectuer votre sélection en cochant la case à droite du/de la Candidat(e). Pour réinitialiser vos sélections, cliquez sur le bouton “<b>Effacer la sélection</b>”, pour passer à l'étape suivante, cliquez sur le bouton “<b>Suivant</b>”.",
                ok: "OK",
            },
            nonVotedDialog: {
                title: "Vote invalide ou blanc",
                content:
                    "Certaines de vos réponses pourraient rendre le bulletin invalide ou blanc dans une ou plusieurs sélections.",
                ok: "Réviser mes réponses",
                continue: "Continuer",
                cancel: "Annuler",
            },
            warningDialog: {
                title: "Vérifiez votre bulletin",
                content:
                    "Votre bulletin contient des sélections à vérifier (par exemple, si vous avez sélectionné moins d'options que le nombre autorisé). Votre bulletin est valide et sera comptabilisé tel qu'il a été soumis.",
                ok: "Réviser mes réponses",
                continue: "Continuer",
                cancel: "Annuler",
            },
            blankBallotDialog: {
                title: "Vous n'avez sélectionné aucun candidat",
                content:
                    "Vous n'avez fait aucune sélection. Votre bulletin sera déposé comme bulletin blanc, un choix valide et délibéré qui sera comptabilisé comme tel.",
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
                    "Êtes-vous certain de vouloir vous abstenir de voter ?<br />Vous accéderez directement à l'étape de vérification et votre statut de participation sera enregistré comme <b>S’est abstenu de voter</b>.",
                continue: "S’abstenir de voter",
                cancel: "Annuler",
            },
            instructionsTitle: "Instructions",
            instructionsDescription: "Veuillez suivre les étapes suivantes pour voter:",
            step1Title: "1. Sélectionnez votre option de vote",
            step1Description:
                "Sélectionnez vos candidats préférés et répondez aux questions de l'élection une par une au fur et à mesure qu'elles apparaissent. Vous pourrez modifier votre bulletin jusqu'au moment de le soumettre.",
            step2Title: "2. Révisez votre bulletin",
            step2Description:
                "Une fois que vous êtes satisfait de vos sélections, votre bulletin sera chiffré, puis une dernière vérification de vos choix vous sera présentée. Vous recevrez également un identifiant de suivi unique pour votre bulletin.",
            step3Title: "3. Envoyez votre vote",
            step3Description:
                "Soumettez votre bulletin : Enfin, vous pouvez soumettre votre bulletin pour qu'il soit correctement enregistré. Vous pouvez également lancer un audit afin de vérifier que votre bulletin a été correctement saisi et chiffré.",
        },
        reviewScreen: {
            title: "Révisez votre vote",
            description:
                "Pour apporter des modifications à vos sélections, cliquez sur le bouton “<b>Modifier votre vote</b>”, pour confirmer vos sélections, cliquez sur le bouton “<b>Envoyer votre vote</b>” ci-dessous, et pour auditer votre bulletin, cliquez sur le bouton “<b>Auditer le bulletin</b>” ci-dessous. ",
            descriptionNoAudit:
                "Pour apporter des modifications à vos sélections, cliquez sur le bouton “<b>Modifier votre vote</b>”, pour confirmer vos sélections, cliquez sur le bouton “<b>Envoyer votre vote</b>” ci-dessous. ",
            backButton: "Modifier votre vote",
            castBallotButton: "Envoyer votre vote",
            auditButton: "Auditer le bulletin",
            copyBallotId: "Copier l’identifiant du bulletin",
            ballotIdCopied: "Identifiant du bulletin copié",
            ballotIdCopyError: "Impossible de copier l’identifiant du bulletin",
            reviewScreenHelpDialog: {
                title: "Information : Écran de révision",
                content: "Cet écran vous permet de réviser vos sélections avant de voter.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Vote non émis",
                content:
                    "<p>Vous êtes sur le point de copier le Localisateur de vote, mais <b>votre vote n'a pas encore été soumis</b>. Si vous essayez de rechercher le Localisateur de vote, vous ne le trouverez pas.</p><p>Nous affichons le Localisateur de vote à cette étape afin que vous puissiez vérifier, au moyen d'un audit, que votre bulletin a été correctement chiffré avant de le soumettre. Si c'est la raison pour laquelle vous souhaitez copier le Localisateur de vote, copiez-le d'abord, ensuite effectuez un audit de votre vote.</p>",
                ok: "J'accepte que mon vote N'A PAS été émis",
                cancel: "Annuler",
            },
            auditBallotHelpDialog: {
                title: "Voulez-vous vraiment Auditer votre bulletin ?",
                content:
                    "<p>L'audit du bulletin l'invalidera et vous devrez recommencer le processus de vote si vous souhaitez soumettre votre vote. L'audit du bulletin permet de vérifier que celui-ci a été correctement chiffré. Cette procédure requiert des connaissances techniques approfondies et n'est pas recommandée si vous ne maîtrisez pas son fonctionnement.</p><p><b>Si vous souhaitez soumettre votre vote, cliquez sur <u>Annuler</u> pour revenir à l'écran de vérification de votre bulletin.</b></p>",
                ok: "Oui, je veux INVALIDER mon bulletin pour l'AUDITER",
                cancel: "Annuler",
            },
            confirmCastVoteDialog: {
                title: "Êtes-vous sûr de vouloir voter?",
                content: "Votre vote ne sera plus modifiable une fois confirmé.",
                ok: "Oui, je veux VOTER",
                cancel: "Annuler",
            },
            confirmCastBlankBallotDialog: {
                title: "Êtes-vous sûr de vouloir déposer un bulletin blanc ?",
                content:
                    "Vous n'avez sélectionné aucun candidat. Après confirmation, votre bulletin sera déposé blanc.",
                ok: "Oui, je veux déposer mon bulletin blanc",
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
                    "Une erreur est survenue lors de la vérification du statut du vote. Veuillez réessayer ultérieurement ou contacter le support pour obtenir de l'aide.",
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
                    "Une erreur inconnue est survenue lors du vote. Veuillez réessayer ultérieurement ou contacter le support pour obtenir de l'aide.",
                NO_BALLOT_SELECTION:
                    "L'état de sélection pour cette élection est introuvable. Veuillez vous assurer que vous avez sélectionné correctement vos choix ou contactez le support.",
                NO_BALLOT_STYLE:
                    "Le style du bulletin de vote n'est pas disponible. Veuillez contacter le support.",
                NO_AUDITABLE_BALLOT:
                    "Aucun bulletin de vote vérifiable n'est disponible. Veuillez contacter le support.",
                INCONSISTENT_HASH:
                    "Une erreur est survenue lors du calcul du hachage du bulletin. L'identifiant du bulletin ({{ballotId}}) ne correspond pas au hachage du bulletin vérifiable ({{auditableBallotHash}}). Veuillez signaler ce problème au service d'aide.",
                ELECTION_EVENT_NOT_OPEN:
                    "L'événement électoral est fermé. Veuillez contacter le support.",
                PARSE_ERROR:
                    "Une erreur est survenue lors de l'analyse du bulletin de vote. Veuillez réessayer ultérieurement ou contacter le support pour obtenir de l'aide.",
                DESERIALIZE_AUDITABLE_ERROR:
                    "Une erreur est survenue lors de la désérialisation du bulletin vérifiable. Veuillez réessayer ultérieurement ou contacter le support pour obtenir de l'aide.",
                DESERIALIZE_HASHABLE_ERROR:
                    "Une erreur est survenue lors de la désérialisation du bulletin haché. Veuillez réessayer ultérieurement ou contacter le support pour obtenir de l'aide.",
                CONVERT_ERROR:
                    "Une erreur est survenue lors de la conversion du bulletin de vote. Veuillez réessayer ultérieurement ou contacter le support pour obtenir de l'aide.",
                SERIALIZE_ERROR:
                    "Une erreur est survenue lors de la sérialisation du bulletin de vote. Veuillez réessayer ultérieurement ou contacter le support pour obtenir de l'aide.",
                UNKNOWN_ERROR:
                    "Une erreur est survenue. Veuillez réessayer ultérieurement ou contacter le support pour obtenir de l'aide.",
                REAUTH_FAILED:
                    "L'authentification a échoué. Veuillez réessayer ou contacter le support pour obtenir de l'aide.",
                SESSION_EXPIRED: "Votre session a expiré. Veuillez recommencer depuis le début.",
                CAST_VOTE_BallotIdMismatch:
                    "L'identifiant du bulletin ne correspond pas à celui du vote exprimé.",
                SESSION_STORAGE_ERROR:
                    "Le stockage de session n'est pas disponible. Veuillez réessayer ou contacter le support.",
                PARSE_BALLOT_DATA_ERROR:
                    "Une erreur s'est produite lors de l'analyse des données du bulletin de vote. Veuillez réessayer ultérieurement ou contacter le support pour obtenir de l'aide.",
                NOT_VALID_BALLOT_DATA_ERROR:
                    "Les données du bulletin de vote ne sont pas valides. Veuillez réessayer ultérieurement ou contacter le support pour obtenir de l'aide.",
                FETCH_DATA_TIMEOUT_ERROR:
                    "Erreur de délai d'attente pour récupérer les données. Veuillez réessayer ultérieurement ou contacter le support pour obtenir de l'aide.",
                TO_HASHABLE_BALLOT_ERROR:
                    "Erreur lors de la conversion en bulletin de vote hashable. Veuillez réessayer ultérieurement ou contacter le support pour obtenir de l'aide.",
                INTERNAL_ERROR:
                    "Une erreur interne s'est produite lors du vote. Veuillez réessayer ultérieurement ou contacter le support pour obtenir de l'aide.",
            },
            declineToVote: "S’abstenir de voter",
            blankBallot: "Bulletin blanc",
        },
        confirmationScreen: {
            title: "Votre vote a été émis",
            description:
                "Le code de confirmation ci-dessous vérifie que <b>votre vote a été émis correctement</b>. Vous pouvez utiliser ce code pour vérifier que votre vote a été comptabilisé.",
            blankBallot: {
                description: "Votre bulletin a été déposé blanc, un choix valide et délibéré.",
            },
            ballotId: "Localisateur de Vote",
            printButton: "Imprimer",
            finishButton: "Terminer",
            verifyCastTitle: "Vérifiez que votre vote a été émis",
            verifyCastDescription:
                "Vous pouvez vérifier à tout moment que votre bulletin a été émis correctement en utilisant le code QR ci-dessous:",
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
                content: "Impossible d'utiliser le code, celui-ci est désactivé en mode démo.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Information : Localisateur de votre Bulletin",
                content:
                    "Le Localisateur de Bulletin est un code qui vous permet de retrouver votre bulletin dans l'urne, ce Localisateur est unique et ne contient aucune information sur vos choix.",
                ok: "OK",
            },
            ballotIdDemoHelpDialog: {
                title: "Information : Identifiant de bulletin de vote",
                content:
                    "<p>L'identifiant de bulletin de vote est un code qui vous permet de retrouver votre bulletin dans l'urne. Cet identifiant est unique et ne contient aucune information sur vos choix.</p><p><b>Avis :</b> Ce bureau de vote est uniquement à des fins de démonstration. Votre vote n'a PAS été émis.</p>",
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
            title: "Auditez votre Bulletin",
            description: "Pour vérifier votre bulletin, vous devez suivre les étapes suivantes:",
            step1Title: "1. Téléchargez ou copiez les informations suivantes",
            step1Description:
                "Votre <b>Localisateur de Vote</b> apparaît en haut de l'écran et votre bulletin chiffré ci-dessous:",
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
                    "Pour auditer votre vote, vous devez suivre les étapes décrites dans le tutoriel, notamment télécharger une application de bureau permettant de vérifier le bulletin chiffré de manière indépendante, sans passer par le site Web.",
                ok: "OK",
            },
            bottomWarning:
                "Pour des raisons de sécurité, lorsque vous auditez votre bulletin, vous devrez l'invalider. Pour continuer avec le processus de vote, cliquez sur ‘<b>Démarrer le vote</b>’.",
        },
        electionSelectionScreen: {
            title: "Liste des Élections",
            description: "Sélectionnez l'élection à laquelle vous souhaitez voter",
            chooserHelpDialog: {
                title: "Information : Liste des Élections",
                content:
                    "Bienvenue dans le bureau de vote, cet écran montre la liste des élections auxquelles vous pouvez voter. Les élections affichées sur cette liste peuvent être ouvertes au vote, programmées ou fermées. Vous ne pourrez accéder au vote que si la période de vote est ouverte.",
                ok: "OK",
            },
            noResults: "Il n'y a pas d'élections pour le moment.",
            resultsButton: "Voir les résultats",
            demoDialog: {
                title: "Bureau de vote de démonstration",
                content:
                    "Vous entrez dans un bureau de vote de démonstration. <strong>Votre vote ne sera PAS comptabilisé.</strong> Ce bureau de vote est uniquement destiné à des fins de démonstration.",
                ok: "J'accepte que mon vote ne sera pas comptabilisé",
            },
            errors: {
                noVotingArea:
                    "Zone de vote non assignée à l'électeur. Veuillez réessayer ultérieurement ou contacter le support pour obtenir de l'aide.",
                networkError:
                    "Il y a eu un problème de réseau. Veuillez réessayer ultérieurement ou contacter le support pour obtenir de l'aide.",
                unableToFetchData:
                    "Il y a eu un problème pour récupérer les données. Veuillez réessayer ultérieurement ou contacter le support pour obtenir de l'aide.",
                noElectionEvent:
                    "L'événement électoral n'existe pas. Veuillez réessayer ultérieurement ou contacter le support pour obtenir de l'aide.",
                ballotStylesEmlError:
                    "Il y a eu une erreur avec la publication du style de bulletin. Veuillez réessayer ultérieurement ou contacter le support pour obtenir de l'aide.",
                obtainingElectionFromID:
                    "Il y a eu une erreur pour obtenir les élections associées aux identifiants d'élection suivants : {{electionIds}}. Veuillez réessayer ultérieurement ou contacter le support pour obtenir de l'aide.",
            },
            alerts: {
                noElections:
                    "Il n'y a pas d'élections pour lesquelles vous pouvez voter. Cela pourrait être parce que la zone n'a aucun concours associé. Veuillez réessayer ultérieurement ou contacter le support pour obtenir de l'aide.",
                electionEventNotPublished:
                    "L'événement électoral n'a pas encore été publié. Veuillez réessayer ultérieurement ou contacter le support pour obtenir de l'aide.",
            },
            materialsGate: {
                instructions:
                    "Vous devez lire <MaterialsLink>{{materialsTitle}}</MaterialsLink> avant de pouvoir voter.",
            },
        },
        errors: {
            page: {
                oopsWithStatus: "Oups ! {{status}}",
                oopsWithoutStatus: "Oups ! Une erreur inattendue est survenue.",
                somethingWrong: "Une erreur est survenue.",
                certAuthFailedTitle: "Échec de l'authentification par certificat",
                certAuthFailedMessage:
                    "Votre certificat n'a pas pu être vérifié. Veuillez vous assurer que vous utilisez un certificat d'électeur valide, puis réessayez.",
            },
        },
        materials: {
            common: {
                label: "Documentation et support",
                back: "Retour à la Liste des Élections",
                close: "Fermer",
                preview: "Aperçu",
                download: "Télécharger",
            },
            mandatory: {
                checkboxLabel: "J'ai lu la documentation et le support",
                continueButton: "Continuer",
                error: "Un problème est survenu lors de l'enregistrement de votre confirmation. Veuillez réessayer.",
            },
        },
        ballotLocator: {
            title: "Localisez votre bulletin de vote",
            titleResult: "Résultat de la recherche de votre Bulletin",
            description: "Vérifiez que votre bulletin a été émis correctement",
            locate: "Trouvez votre bulletin",
            locateAgain: "Trouvez un autre bulletin",
            found: "Votre numéro d'identification de bulletin {{ballotId}} a été trouvé",
            notFound: "Votre numéro d'identification de bulletin {{ballotId}} n'a pas été trouvé",
            ambiguous:
                "Plusieurs de vos bulletins correspondent à {{ballotId}}. Utilisez le numéro d'identification complet du bulletin.",
            contentDesc: "Voici le contenu de votre bulletin : ",
            wrongFormatBallotId: "Format incorrect pour le numéro d'identification du bulletin",
            ballotIdNotFoundAtFilter:
                "Non trouvé, veuillez vérifier que le numéro d'identification du bulletin soit correct et appartenir à cet utilisateur.",
            filterByBallotId: "Filtrez par numéro d'identification du bulletin",
            totalBallots: "Total: {{total}}",
            steps: {
                lookup: "Localisez votre bulletin de vote",
                result: "Résultat",
            },
            titleHelpDialog: {
                title: "Information : écran de localisation de votre bulletin",
                content:
                    "Cet écran permet au votant de trouver son bulletin en utilisant le numéro d'identification du bulletin pour le récupérer. Cette procédure permet de vérifier que son vote a été émis correctement et que le vote enregistré correspond au vote chiffré émis.",
                ok: "OK",
            },
            tabs: {
                logs: "Logs",
                ballotLocator: "Localisez votre bulletin",
            },
            column: {
                statement_kind: "Type",
                statement_timestamp: "Marque de temps",
                username: "Nom d'utilisateur",
                ballot_id: "Numéro d'identification du bulletin",
                message: "Message",
            },
        },
    },
}

export default frenchTranslation
