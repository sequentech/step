// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import englishTranslation, {TranslationType} from "./en"

const frenchTranslation: TranslationType = {
    translations: {
        ballotPrivacy: {
            ...englishTranslation.translations.ballotPrivacy,
            title: "Tous les bulletins",
            subtitle: "Tous les bulletins envoyés dans cette élection, une seule fois, à leur dernière étape. Les électeurs figurent sous un pseudonyme et non sous leur nom.",
            own: "Votre bulletin est dans le journal",
            copy: "Copier l’identifiant de suivi",
            ballotIdHelp: "À propos de votre identifiant de suivi",
            stageTime: "{{stage}} {{time}}",
            noMatchHelp: "Vérifiez les fautes de frappe. Un identifiant de suivi comporte un trait d’union et au moins 8 caractères, comme FTBE-MHRX. Un pseudonyme comporte deux-points, comme K7QM:2XWD.",
            copied: "Identifiant de suivi copié",
            copyFailed: "Impossible de copier l’identifiant de suivi. Sélectionnez le code pour le copier.",
            search: "Rechercher un identifiant de suivi ou un pseudonyme",
            placeholder: "FTBE-MHRX ou K7QM:2XWD",
            matchesBoth: "{{tracking}} correspondent par identifiant de suivi · {{pseudonym}} par pseudonyme",
            ballotId: "Identifiant de suivi",
            lastStep: "Dernière étape",
            time: "Heure",
            trackingId: "Identifiant de suivi",
            trackingIdHelp: "Calculé à partir du bulletin chiffré et de la clé de l’électeur.",
            noMatch: "Aucun identifiant de suivi ni pseudonyme ne commence par {{prefix}}",
            noBallots: "Aucun bulletin n’a encore été envoyé.",
            invalidPrefix: "Saisissez au moins 8 caractères d’un identifiant de suivi avec un trait d’union, comme FTBE-MHRX, ou jusqu’à 8 d’un pseudonyme avec deux-points, comme K7QM:2XWD. Les codes utilisent les chiffres de 0 à 9 et des lettres majuscules, sans I, L, O ni U.",
            stages: {
                received: {
                    label: "Reçu",
                    title: "Bulletin reçu",
                    help: "L’urne a signé un reçu pour ce bulletin et le journal public a promis de l’ajouter. Il n’est pas compté à moins que son électeur ne l’émette.",
                    castState: "Pas encore émis",
                },
                cast: {
                    label: "Émis",
                    title: "Bulletin émis",
                    help: "L’urne a confirmé que l’électeur a émis ce bulletin.",
                    castState: "Réussi",
                },
                audited: {
                    label: "Audité",
                    title: "Bulletin audité",
                    help: "Son électeur l’a ouvert pour le vérifier. Un bulletin audité n’est jamais émis ni compté.",
                    castState: "Jamais émis",
                },
            },
            checkLabels: {
                ...englishTranslation.translations.ballotPrivacy.checkLabels,
                ballotId: "Identifiant de suivi",
            },
            checkDescriptions: {
                ...englishTranslation.translations.ballotPrivacy.checkDescriptions,
                ballotId: "L’identifiant de suivi est calculé à partir du bulletin chiffré et de la clé de l’électeur.",
            },
        },
        startup: {
            error: "Le portail de vote n’a pas pu démarrer. Veuillez réessayer.",
            retry: "Réessayer",
        },
        contactChannel: {
            change: "Mode de contact",
            updated: "Votre contact a été modifié. Les codes et les avis y sont désormais envoyés.",
            cancelled: "Votre contact n’a pas été modifié.",
            failed: "Nous n’avons pas pu modifier la façon dont nous vous contactons. Réessayez plus tard ou contactez l’assistance.",
            close: "Fermer",
        },
        common: {
            goBack: "Retour",
            showMore: "Afficher plus",
            showLess: "Afficher moins",
        },
        audioInstructions: {
            screens: {
                "election-chooser":
                    "Cette page présente les élections auxquelles vous pouvez voter. Utilisez la touche Tabulation pour passer d'une élection à la suivante. Chaque élection a un bouton pour commencer à voter. Appuyez sur Entrée sur ce bouton pour ouvrir son bulletin.",
                "start":
                    "Cette page explique comment voter. Vous choisissez d'abord vos options, puis vous les vérifiez, et enfin vous déposez votre bulletin. Utilisez la touche Tabulation pour atteindre le bouton qui lance le vote et appuyez sur Entrée. Si une déclaration est affichée, cochez d'abord sa case avec la barre d'espace.",
                "ballot":
                    "Voici votre bulletin. Chaque question est un groupe d'options, et la question indique combien vous pouvez en choisir. Utilisez la touche Tabulation pour passer d'une option à l'autre et la barre d'espace pour en cocher ou décocher une. Quand vous avez terminé, allez au bouton pour continuer, à la fin de la page, et appuyez sur Entrée pour vérifier vos choix. Il y a aussi un bouton pour effacer tous vos choix. Rien n'est déposé tant que vous n'avez pas confirmé sur la page de vérification.",
                "review":
                    "Cette page montre les choix que vous avez faits. Vérifiez-les attentivement. Pour modifier quelque chose, utilisez le bouton pour modifier votre bulletin. Pour voter, allez au bouton pour déposer votre bulletin et appuyez sur Entrée. L'identifiant de votre bulletin se trouve en haut de la page. Vous pouvez le copier pour retrouver votre bulletin plus tard.",
                "confirmation":
                    "Votre vote a été déposé. Cette page affiche l'identifiant de votre bulletin, qui vous permet de vérifier qu'il a été enregistré. Vous pouvez le copier ou imprimer cette page. Quand vous avez terminé, utilisez le bouton à la fin de la page pour terminer.",
                "audit":
                    "Cette page vous permet d'auditer votre bulletin au lieu de le déposer. Elle affiche les informations que vous pouvez contrôler avec le vérificateur de bulletins. Un bulletin audité n'est pas déposé : revenez ensuite à votre bulletin pour voter.",
                "ballot-locator":
                    "Cette page vous permet de vérifier qu'un bulletin a été enregistré. Saisissez ou collez un identifiant de bulletin dans le champ de texte et appuyez sur Entrée. Le résultat s'affiche sous le champ.",
                "support-materials":
                    "Cette page présente des documents qui vous aident à voter. Utilisez la touche Tabulation pour passer de l'un à l'autre et appuyez sur Entrée sur le bouton d'un document pour l'ouvrir. Utilisez le bouton de retour pour revenir à la liste des élections.",
            },
        },
        a11y: {
            skipToContent: "Aller au contenu principal",
            helpAbout: "Aide à propos de {{topic}}",
            previewMaterial: "Aperçu de {{title}}",
            ballotsTable: "Bulletins",
            ballotLocatorTabs: "Sections du suivi du bulletin",
            ballotIdLabel: "Identifiant de suivi",
            votingProgress: "Progression du vote",
            stepOf: "Étape {{current}} sur {{total}}",
            selectUpTo_one: "Sélectionnez jusqu'à {{count}} option",
            selectUpTo_many: "Sélectionnez jusqu'à {{count}} options",
            selectUpTo_other: "Sélectionnez jusqu'à {{count}} options",
            selectExactly_one: "Sélectionnez {{count}} option",
            selectExactly_many: "Sélectionnez {{count}} options",
            selectExactly_other: "Sélectionnez {{count}} options",
            selectBetween: "Sélectionnez entre {{min}} et {{max}} options",
        },
        candidatesList: {
            collapseToggle: "Masquer la liste {{listTitle}}",
            showCandidates: "Afficher les candidats",
            hideCandidates: "Masquer les candidats",
            selectedCandidates_one: "{{count}} candidat sélectionné",
            selectedCandidates_many: "{{count}} candidats sélectionnés",
            selectedCandidates_other: "{{count}} candidats sélectionnés",
            expandAll: "Tout afficher",
            collapseAll: "Tout réduire",
        },
        breadcrumbSteps: {
            electionList: "Liste des élections",
            ballot: "Bulletin de vote",
            review: "Révision",
            confirmation: "Confirmer",
            audit: "Auditer",
        },
        footer: {
            poweredBy: "Développé par <1></1>",
        },
        contest: {
            acclamation: {
                description:
                    "Ce scrutin a été tranché par acclamation. Ses candidat(e)s sont élu(e)s sans vote : aucune option ne peut être sélectionnée et aucune voix n'est enregistrée.",
            },
        },
        votingScreen: {
            backButton: "Retour",
            reviewButton: "Suivant",
            clearButton: "Effacer la sélection",
            ballotHelpDialog: {
                title: "À propos de l'écran de vote",
                content:
                    "Cet écran affiche les scrutins auxquels vous êtes éligible. Cochez la case à droite pour sélectionner un(e) Candidat(e). Pour réinitialiser, cliquez sur “<b>Effacer la sélection</b>”, pour continuer, cliquez sur “<b>Suivant</b>”.",
                ok: "OK",
            },
            nonVotedDialog: {
                title: "Vote invalide ou blanc",
                content:
                    "Certaines de vos réponses pourraient rendre le bulletin invalide ou blanc dans un ou plusieurs scrutins.",
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
            instructionsTitle: "Comment voter",
            instructionsDescription: "Suivez les étapes suivantes pour voter:",
            step1Title: "1. Sélectionnez votre option de vote",
            step1Description:
                "Sélectionnez vos candidats et faites vos choix dans chaque scrutin. Vous pourrez modifier votre bulletin jusqu'au moment de le soumettre.",
            step2Title: "2. Révisez votre bulletin",
            step2Description:
                "Votre bulletin sera chiffré, puis une dernière vérification vous sera présentée. Vous recevrez un identifiant de suivi unique.",
            step3Title: "3. Envoyez votre vote",
            step3Description:
                "Soumettez votre bulletin pour qu'il soit enregistré, ou lancez un audit afin de vérifier qu'il a été correctement chiffré.",
        },
        reviewScreen: {
            reloaded: {
                title: "Votre bulletin n’a pas été émis",
                text: "Le rechargement de la page a effacé la clé à usage unique qui l’émet. Vérifiez vos choix et envoyez de nouveau votre bulletin. Le bulletin que vous avez envoyé reste dans le registre comme reçu et n’est jamais compté.",
                back: "Retour à mon bulletin",
            },
            castRecovery: {
                uncertain:
                    "Le résultat du vote n’a pas été confirmé. Vous pouvez réessayer la même demande ou vérifier votre identifiant de bulletin avant de recommencer.",
                checkBallot: "Vérifier le bulletin",
            },
            acclamation: {
                title: "Acquis par acclamation",
                helpDialog: {
                    title: "Information : Acclamation",
                    content:
                        "Cet écran présente ce qui a été acquis par acclamation. Aucune option ne pouvant être sélectionnée, aucun bulletin n'est déposé et il n'y a rien à vérifier ensuite.",
                    ok: "OK",
                },
                description:
                    "Vérifiez ce qui a été acquis par acclamation lors de cette élection. Aucun bulletin ne sera déposé.",
                finishButton: "Terminer",
            },
            title: "Révisez votre vote",
            description:
                "Cliquez sur “<b>Modifier votre vote</b>” pour changer vos sélections, “<b>Envoyer le vote</b>” pour confirmer, ou “<b>Vérifier le bulletin</b>” pour l'auditer.",
            descriptionNoAudit:
                "Cliquez sur “<b>Modifier votre vote</b>” pour changer vos sélections, ou “<b>Envoyer le vote</b>” pour confirmer.",
            backButton: "Modifier votre vote",
            castBallotButton: "Envoyer le vote",
            auditButton: "Vérifier le bulletin",
            copyBallotId: "Copier l’identifiant de suivi",
            ballotIdCopied: "Identifiant de suivi copié",
            ballotIdCopyError: "Impossible de copier l’identifiant de suivi",
            reviewScreenHelpDialog: {
                title: "À propos de l'écran de révision",
                content: "Cet écran vous permet de réviser vos sélections avant de voter.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Vote non émis",
                content:
                    "<p>Voici votre identifiant de suivi, mais <b>votre vote n'a pas encore été émis</b>. Si vous essayez de le suivre maintenant, rien n'apparaîtra.</p><p>Nous affichons l'identifiant de suivi à cette étape afin que vous puissiez auditer le bulletin chiffré avant de l'émettre.</p>",
                ok: "Je comprends que mon vote n'a pas été émis",
                cancel: "Annuler",
            },
            auditBallotHelpDialog: {
                title: "Voulez-vous vraiment Auditer votre bulletin ?",
                content:
                    "<p>L'audit du bulletin l'invalidera et vous devrez recommencer le processus de vote si vous souhaitez soumettre votre vote. L'audit du bulletin permet de vérifier que celui-ci a été correctement chiffré. Cette procédure requiert des connaissances techniques approfondies et n'est pas recommandée si vous ne maîtrisez pas son fonctionnement.</p><p><b>Si vous souhaitez soumettre votre vote, cliquez sur <u>Annuler</u> pour revenir à l'écran de vérification de votre bulletin.</b></p>",
                ok: "Oui, je veux invalider mon bulletin pour l'auditer",
                cancel: "Annuler",
            },
            confirmCastVoteDialog: {
                title: "Êtes-vous sûr de vouloir voter?",
                content: "Après confirmation, votre vote sera émis.",
                ok: "Oui, je veux voter",
                cancel: "Annuler",
            },
            confirmCastBlankBallotDialog: {
                title: "Êtes-vous sûr de vouloir déposer un bulletin blanc ?",
                content:
                    "Vous n'avez sélectionné aucun candidat. Après confirmation, votre bulletin sera déposé blanc.",
                ok: "Oui, je veux déposer mon bulletin blanc",
                cancel: "Annuler",
            },
            unfilledContestsDialog: {
                title: "Certaines sélections sont incomplètes",
                content:
                    "Vous pouvez laisser une sélection vide ou choisir moins d'options que le maximum autorisé. Vérifiez les éléments suivants avant de continuer.",
                selected: "{{selected}} sur {{max}} sélectionnées",
                nothingSelected: "Aucune sélection",
                ok: "Continuer avec ces sélections",
                cancel: "Revoir les sélections",
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
                    "Une erreur est survenue lors de la vérification du statut du vote. Veuillez réessayer plus tard ou contacter le support pour obtenir de l'aide.",
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
                    "Une erreur est survenue lors du calcul du hachage du bulletin. L'identifiant de suivi ({{ballotId}}) ne correspond pas au hachage du bulletin vérifiable ({{auditableBallotHash}}). Veuillez signaler ce problème au service d'aide.",
                NOT_RECEIVED: "Le registre public ne contient pas encore votre bulletin, il n'a donc pas été envoyé. Revenez à votre bulletin et réessayez.",
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
                    "L'identifiant de suivi ne correspond pas à celui du vote exprimé.",
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
            acclamation: {
                title: "Acquis par acclamation",
                description:
                    "Tous les scrutins de cette élection ont été tranchés par acclamation : aucun bulletin n'a été déposé et il n'y a pas d'identifiant de suivi.",
                helpDialog: {
                    title: "Information : Acclamation",
                    content:
                        "Tous les scrutins de cette élection ont été tranchés par acclamation : leurs candidat(e)s sont élu(e)s sans vote. Comme aucun bulletin n'a été déposé, il n'y a ni identifiant de suivi, ni reçu, ni code QR à vérifier.",
                    ok: "OK",
                },
            },
            title: "Votre vote a été émis",
            description:
                "Le code de confirmation ci-dessous vérifie que <b>votre vote a été émis correctement</b>. Vous pouvez utiliser ce code pour vérifier que votre vote a été comptabilisé.",
            blankBallot: {
                description: "Votre bulletin a été déposé blanc, un choix valide et délibéré.",
            },
            ballotId: "Identifiant de suivi",
            printButton: "Imprimer",
            finishButton: "Terminer",
            remainingElectionsError:
                "Nous n’avons pas pu vérifier s’il vous reste des élections auxquelles voter. Veuillez réessayer.",
            retryButton: "Réessayer",
            verifyCastTitle: "Vérifiez que votre vote a été émis",
            verifyCastDescription:
                "Vous pouvez vérifier à tout moment que votre bulletin a été émis correctement en utilisant le code QR ci-dessous:",
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
                title: "Identifiant de suivi",
                content: "Impossible d'utiliser le code, celui-ci est désactivé en mode démo.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "À propos de l'identifiant de suivi",
                content:
                    "L'identifiant de suivi est un code qui vous permet de retrouver votre bulletin dans l'urne. Cet identifiant est unique et ne contient aucune information sur vos choix.",
                ok: "OK",
            },
            ballotIdDemoHelpDialog: {
                title: "À propos de l'identifiant de suivi",
                content:
                    "L'identifiant de suivi est un code qui vous permet de retrouver votre bulletin dans l'urne. Cet identifiant est unique et ne contient aucune information sur vos choix.",
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
            confirmingAudit: "Confirmation que ce bulletin ne peut plus être déposé…",
            auditUnavailable:
                "Le bulletin n’a pas pu être audité. Son contenu n’a pas été révélé. Veuillez réessayer.",
            printButton: "Imprimer",
            restartButton: "Démarrer le vote",
            title: "Vérifiez votre Bulletin",
            description: "Pour vérifier votre bulletin, suivez les étapes suivantes:",
            step1Title: "1. Téléchargez ou copiez les informations suivantes",
            step1Description:
                "Votre <b>identifiant de suivi</b> apparaît en haut de l'écran et votre bulletin chiffré ci-dessous:",
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
            title: "Élections",
            description: "Sélectionnez l'élection à laquelle vous souhaitez voter",
            chooserHelpDialog: {
                title: "À propos de la liste des élections",
                content:
                    "Cet écran montre la liste des élections auxquelles vous pouvez voter. Vous pouvez y accéder uniquement si la période de vote est ouverte.",
                ok: "OK",
            },
            noResults: "Il n'y a pas d'élections pour le moment.",
            resultsButton: "Voir les résultats",
            demoDialog: {
                title: "Bureau de vote de démonstration",
                content:
                    "Vous entrez dans un bureau de vote de démonstration. <strong>Votre vote ne sera pas comptabilisé.</strong> Uniquement à des fins de démonstration.",
                ok: "Je comprends que mon vote ne sera pas comptabilisé",
            },
            errors: {
                noVotingArea:
                    "Vous n'êtes pas inscrit comme votant pour cette élection. Veuillez contacter le support.",
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
                    "Il n'y a pas d'élections pour lesquelles vous pouvez voter. Cela pourrait être parce que la zone n'a aucun scrutin associé. Veuillez réessayer ultérieurement ou contacter le support pour obtenir de l'aide.",
                electionEventNotPublished:
                    "L'événement électoral n'a pas encore été publié. Veuillez réessayer ultérieurement ou contacter le support pour obtenir de l'aide.",
            },
            materialsGate: {
                instructions:
                    "Vous devez lire <MaterialsLink>{{materialsTitle}}</MaterialsLink> avant de pouvoir voter.",
            },
            votingClosedAt: "Le vote a été clôturé le {{close}} ({{localClose}}).",
            votingClosedOn: "Le vote a été clôturé le {{close}}.",
        },
        errors: {
            encoding: {
                notEnoughChoices: "Nombre d'options insuffisant pour le déchiffrement",
                writeInChoiceOutOfRange: "Entrée de saisie libre hors limites : {{index}}",
                writeInNotEndInZero: "Le texte saisi ne se termine pas par 0",
                bytesToUtf8Conversion:
                    "Erreur lors de la conversion du texte de bytes en chaîne UTF-8 : {{errorMessage}}",
                ballotTooLarge: "La taille du bulletin dépasse la limite prévue",
            },
            explicit: {
                notAllowed: "Bulletin marqué comme invalide, mais ce scrutin ne l'autorise pas",
                alert: "Cette sélection sera comptée comme un vote invalide",
            },
            page: {
                oopsWithStatus: "Oups ! {{status}}",
                oopsWithoutStatus: "Oups ! Une erreur inattendue est survenue.",
                somethingWrong: "Une erreur est survenue.",
                invalidLoginHintParametersTitle: "Lien de vote non valide",
                invalidLoginHintParametersMessage:
                    "Ce lien de vote contient des informations de connexion non valides. Demandez un nouveau lien et réessayez.",
                certAuthFailedTitle: "Échec de l'authentification par certificat",
                certAuthFailedMessage:
                    "Votre certificat n'a pas pu être vérifié. Veuillez vous assurer que vous utilisez un certificat d'électeur valide, puis réessayez.",
            },
        },
        materials: {
            common: {
                label: "Documentation et support",
                back: "Retour à la liste des élections",
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
        slates: {
            title: "Listes",
            description:
                "Une liste est un groupe de candidats qui se présentent ensemble. Chaque candidat indique la liste à laquelle il appartient.",
            independent: "Indépendant",
            contestMembers: "Candidats de {{slate}} pour {{contest}}",
            noCandidate: "Aucun candidat",
            coverage: {
                full: "Liste complète",
                singleContest: "{{contest}} uniquement",
                partial: "Liste partielle",
                candidates_one: "{{count}} candidat",
                candidates_other: "{{count}} candidats",
                offices_one: "{{count}} poste",
                offices_other: "{{count}} postes",
            },
            candidateList: {
                show: "Afficher les candidatures",
                hide: "Masquer les candidatures",
            },
            tabs: {
                label: "Façons de remplir votre bulletin",
                slates: "Choisir une liste",
                candidates: "Candidatures individuelles",
            },
            review: {
                title: "Vos sélections",
                total: "Candidatures sélectionnées : {{selected}} sur {{seats}}",
                slate: "{{slate}} : {{status}}",
                independent: "Candidatures indépendantes sélectionnées : {{count}}",
                note: "Votre vote est enregistré pour chaque candidature sélectionnée. Une liste n'est pas un vote en soi.",
                contestCount: "{{selected}} sur {{max}} sélectionnées",
                edit: "Modifier",
                editLabel: "Modifier {{contest}}",
            },
            selection: {
                all: "Les {{total}} sélectionnées",
                mixed: "Mixte · {{selected}} sur {{total}} sélectionnées",
                partly: "Sélection partielle · {{selected}} sur {{total}}",
                selected: "Sélectionnée",
                edit: "Modifier la sélection",
            },
            apply: {
                button: "Choisir cette liste",
                buttonLabel: "Choisir la liste {{slate}}",
                chosen: "{{slate}} choisie. Candidats sélectionnés : {{candidates}}. Scrutins : {{contests}}.",
                overMaximum:
                    "{{slate}} ne peut pas être choisie : elle compte {{candidates}} candidats pour {{contest}}, qui en autorise {{max}}. Vous pouvez toujours choisir des candidats individuellement.",
                unavailable:
                    "{{slate}} ne peut pas être choisie sur ce bulletin. Vous pouvez toujours choisir des candidats individuellement.",
                replaceDialog: {
                    title: "Remplacer vos choix actuels ?",
                    content:
                        "Choisir {{slate}} remplace vos choix dans les scrutins ci-dessous. Vos autres choix ne changent pas.",
                    removed: "Retiré :",
                    added: "Sélectionné à la place :",
                    ok: "Remplacer les choix",
                    cancel: "Conserver mes choix",
                },
            },
        },
        ballotLocator: {
            title: "Suivi du bulletin",
            titleResult: "Suivi du bulletin",
            description: "Vérifiez que votre bulletin a été émis correctement",
            descriptionAudited:
                "Ce bulletin a été audité ; il n'a donc jamais été émis. Vérifiez que les sélections décodées ci-dessous sont bien celles que vous avez faites.",
            descriptionAuditedBrief: "Ce bulletin a été audité ; il n'a donc jamais été émis.",
            locate: "Trouvez votre bulletin",
            locateAgain: "Trouvez un autre bulletin",
            found: "Votre identifiant de suivi {{ballotId}} a été trouvé",
            notFound: "Votre identifiant de suivi {{ballotId}} n'a pas été trouvé",
            contentDesc: "Voici le contenu de votre bulletin : ",
            wrongFormatBallotId: "Format incorrect pour l'identifiant de suivi",
            ambiguous: "Plusieurs de vos bulletins commencent par {{ballotId}}. Choisissez-en un :",
            ambiguousMany: "Plusieurs de vos bulletins commencent par {{ballotId}}. Saisissez davantage de l'identifiant de suivi.",
            ballotIdNotFoundAtFilter:
                "Non trouvé, veuillez vérifier que votre identifiant de suivi est correct et qu'il appartient à cet utilisateur.",
            filterByBallotId: "Filtrez par identifiant de suivi",
            totalBallots: "Total: {{total}}",
            idHelp: "L'identifiant de suivi court, comme FTBE-MHRX, ou l'identifiant complet",
            foundShort:
                "Trouvé grâce à son identifiant court. Pour être certain que c'est le vôtre, comparez l'identifiant de suivi complet avec votre reçu.",
            search: {
                title: "Vérifiez votre bulletin dans le registre public",
                intro: "Saisissez l'identifiant de suivi indiqué sur votre écran de confirmation ou votre reçu. Votre navigateur le recherche et vérifie lui-même les preuves.",
                find: "Rechercher",
                invalid:
                    "Un identifiant de suivi ne contient que des chiffres et les lettres de a à f.",
                chips: {
                    short: "<b>Court</b> FTBE-MHRX",
                    full: "<b>Complet</b> 16 groupes de quatre",
                    spaces: "Espaces facultatifs",
                },
                features: {
                    cast: {
                        title: "Bulletins émis",
                        text: "Vérifiez qu'il figure dans le registre public, confirmé par des témoins indépendants et ancré publiquement.",
                    },
                    audited: {
                        title: "Bulletins audités",
                        text: "Consultez les sélections qu'il contient, déchiffrées dans votre navigateur, et vérifiez qu'elles sont bien celles que vous avez faites.",
                    },
                    secret: {
                        title: "Votre vote reste secret",
                        text: "Un bulletin émis reste chiffré. Le suivi des bulletins montre qu'il a été comptabilisé, jamais comment vous avez voté.",
                    },
                },
            },
            matches: {
                title: "{{total}} bulletins partagent l'identifiant court <b>{{shortId}}</b>. Choisissez le vôtre, ou saisissez l'identifiant de suivi complet.",
                cast: "Émis le {{time}}",
                audited: "Audité le {{time}}",
            },
            audit: {
                title: "Vos sélections, décodées",
                note: "Votre navigateur a déchiffré ce bulletin audité avec la clé révélée par le portail. Si ce ne sont pas les sélections que vous avez faites, contactez l'autorité électorale.",
                failed: {
                    title: "Échec de l'audit",
                    text: "Le bulletin figurant dans le registre public ne se déchiffre pas avec la clé révélée par le portail. Ne votez pas depuis cet appareil ; informez l'autorité électorale.",
                },
                unchecked: {
                    title: "L'audit n'a pas pu être vérifié",
                    text: "Cet appareil n'a pas pu vérifier ce bulletin audité ; aucune sélection n'est donc affichée. Réessayez plus tard, ou vérifiez le contenu du bulletin dans le vérificateur de bulletins.",
                },
            },
            result: {
                receipt: "Reçu du bulletin",
                idComment:
                    "Utilisez l'identifiant court ou l'identifiant complet pour retrouver ce bulletin.",
                fullId: "Identifiant de suivi complet",
                copy: "Copier l'identifiant de suivi complet",
                copied: "Identifiant de suivi copié",
                tag: {
                    found: "Trouvé",
                    auditFailed: "Échec de l'audit",
                },
                record: {
                    title: "Le contenu de votre bulletin",
                    subtitle: "Chiffré, comme dans le registre public",
                },
                recordEntry: "Entrée du registre",
                save: "Enregistrer le reçu",
                sealLabel: "État du bulletin : {{status}}",
                check: "Le vérifier de façon indépendante",
                caption: "Vérifié par votre navigateur, avec du code que chacun peut inspecter.",
                captionVerified:
                    "Confirmé par votre navigateur, avec du code que chacun peut inspecter.",
                captionProblem:
                    "Trouvé par votre navigateur, avec du code que chacun peut inspecter.",
                late: "Cela signifie en général qu'un pôle est lent ou hors ligne ; ce n'est pas une preuve de falsification. Vérifiez à nouveau plus tard, ici ou sur un autre appareil.",
                problem:
                    "Le portail indique que ce bulletin a été émis, mais le registre public de l'élection n'a aucune entrée pour lui. Votre navigateur a conservé la preuve : la réponse du portail et le point de contrôle signé du registre. Envoyez-la à l'autorité électorale ou à un observateur.",
                problemOther:
                    "Les preuves signées de ce bulletin se contredisent. Il peut s'agir d'une panne ou d'une attaque. Votre navigateur a conservé la preuve. Envoyez-la à l'autorité électorale ou à un observateur.",
                saveEvidence: "Enregistrer la preuve",
                howToReport: "Comment le signaler",
                howToReportText:
                    "Enregistrez le fichier de la preuve et envoyez-le à l'autorité électorale ou à un observateur indépendant. Ils peuvent le vérifier sur leurs propres ordinateurs. Il ne contient rien sur la façon dont vous avez voté.",
                noReceipt:
                    "Cet appareil n'a pas de reçu pour ce bulletin. Vérifiez-le sur l'appareil avec lequel vous avez voté, ou vérifiez le fichier du reçu dans le vérificateur de bulletins.",
                receiptPending:
                    "Cet appareil conserve un reçu pour ce bulletin, mais il n'est pas complet ou n'a pas pu être vérifié à l'instant. Réessayez plus tard, ou vérifiez le fichier du reçu dans le vérificateur de bulletins.",
                status: {
                    checking: {
                        head: "Vérification…",
                        sub: "Votre navigateur recherche le bulletin dans le registre public.",
                    },
                    recorded: {
                        head: "Dans le registre public",
                        sub: "Votre navigateur a trouvé le bulletin. Vérification de ses témoins…",
                    },
                    witnessed: {
                        head: "Confirmé par des témoins",
                        sub: "Vérification des ancrages publics…",
                        subAnchored_one:
                            "Ancré dans les pôles publics. L'ancrage là où les navigateurs vérifient suivra d'ici {{count}} minute.",
                        subAnchored_other:
                            "Ancré dans les pôles publics. L'ancrage là où les navigateurs vérifient suivra d'ici {{count}} minutes.",
                    },
                    verified: {
                        head: "Vérifié",
                        sub: "Dans le registre public, confirmé par des témoins indépendants et ancré publiquement.",
                    },
                    late: {
                        head: "Plus long que prévu",
                        sub: "Dans le registre public et confirmé par des témoins, mais ses ancrages publics sont en retard.",
                    },
                    problem: {
                        head: "Absent du registre public",
                        sub: "Votre navigateur n'a pas trouvé ce bulletin dans le registre public de l'élection.",
                    },
                    otherProblem: {
                        head: "Une vérification a échoué",
                        sub: "Les preuves signées de ce bulletin se contredisent. Voyez l'étape qui a échoué.",
                    },
                },
                steps: {
                    record: {
                        title: "Ajouté au registre public",
                        done: "Entrée n° {{index}} du registre des bulletins de l'élection. Votre navigateur a vérifié sa preuve.",
                        failed: "Aucune entrée du registre des bulletins de l'élection n'a cet identifiant de suivi.",
                        missing: "Absent",
                    },
                    witnesses: {
                        title: "Confirmé par des témoins",
                        done: "Des témoins indépendants ont signé un registre qui l'inclut.",
                        count: "{{signed}} sur {{total}} · {{needed}} requis",
                        countOf: "{{signed}} sur {{total}}",
                    },
                    hubs: {
                        title_one: "Ancré dans {{count}} pôle public",
                        title_other: "Ancré dans {{count}} pôles publics",
                        titleAny: "Ancré dans des pôles publics",
                        done: "Des registres partagés conservent aussi cette entrée : {{hubs}}.",
                    },
                    browser: {
                        title: "Ancré là où les navigateurs vérifient",
                        done: "Conservé par un pôle de substitution jusqu'à ce que les navigateurs vérifient eux-mêmes les registres.",
                    },
                    checking: "Votre navigateur vérifie…",
                    scheduled_one: "D'ici {{count}} minute. Inutile d'attendre.",
                    scheduled_other: "D'ici {{count}} minutes. Inutile d'attendre.",
                    by: "Avant {{time}}",
                    late: "En retard",
                    lateHubs: "Les pôles auraient dû l'ancrer avant {{time}}.",
                    lateOther: "Cela aurait dû être fait avant {{time}}.",
                    waiting: "En attente de l'étape précédente.",
                    skipped: "Non vérifié.",
                    failedStep: "Les preuves signées de cette étape se contredisent.",
                    failedTime: "Problème",
                },
            },
            checksAvailableUntil: "Vous pouvez vérifier votre bulletin jusqu'au {{date}}.",
            checksEnded: "Les vérifications ont pris fin le {{date}}.",
            castAt: "Déposé le {{date}}",
            steps: {
                lookup: "Trouvez votre bulletin de vote",
                result: "Résultat",
            },
            titleHelpDialog: {
                title: "À propos du suivi du bulletin",
                content:
                    "Cet écran vous permet de saisir l'identifiant de suivi pour trouver votre vote et confirmer qu'il a été enregistré correctement.",
                ok: "OK",
            },
            tabs: {
                logs: "Logs",
                ballotLocator: "Suivi du bulletin",
            },
            column: {
                statement_kind: "Type",
                statement_timestamp: "Marque de temps",
                ballot_id: "Identifiant de suivi",
            },
        },
        vitrine: {
            ballotId: {
                label: "Identifiant de suivi",
                full: "Identifiant de suivi complet",
                yours: "Votre identifiant de suivi :",
                copy: "Copier l’identifiant de suivi complet",
                copied: "Identifiant de suivi copié",
                showFull: "Afficher l’identifiant de suivi complet",
            },
            status: {
                title: "État du bulletin",
                cast: "Émis le {{when}}",
                audited: "Audité le {{when}}",
                ballotCast: "Vote émis",
                aria: "État du bulletin : {{state}}. Afficher les détails",
                ariaCast: "Vote émis. État du bulletin : {{state}}. Afficher les détails",
                tooltip: "{{state}} · Afficher les détails",
                tracker: "Ouvrir dans le suivi des bulletins",
                save: "Enregistrer le justificatif",
                ok: "OK",
                tag: {
                    cast: "Émis",
                    notCast: "Non émis",
                    auditable: "Auditable",
                    audited: "Auditée",
                },
                state: {
                    notcast: "Pas encore émis",
                    checking: "Vérification…",
                    recorded: "Dans le registre public",
                    witnessed: "Confirmé par des témoins",
                    verified: "Vérifié",
                    late: "En retard",
                    problem: "Problème",
                    auditable: "Auditable",
                    auditfail: "Échec de l’audit",
                },
                head: {
                    late: "Plus long que prévu",
                    mismatch: "Le registre ne correspond pas",
                    damaged: "Le justificatif est endommagé",
                    noReceipt: "Pas de justificatif pour votre bulletin",
                    unchecked: "Votre bulletin n’a pas pu être vérifié",
                },
                sub: {
                    notcast:
                        "Ce bulletin est chiffré sur cet appareil et rien n’a été envoyé. Une fois émis, votre navigateur vérifie qu’il parvient au registre public.",
                    checking:
                        "Votre navigateur vérifie que votre bulletin figure dans le registre public.",
                    recorded:
                        "Votre navigateur a trouvé votre bulletin dans le registre public. Les témoins le confirment ensuite.",
                    witnessed:
                        "Des témoins indépendants ont confirmé le registre. Les ancrages publics suivent d’ici {{minutes}} minutes ; inutile d’attendre.",
                    verified:
                        "Votre bulletin figure dans le registre public, confirmé par des témoins indépendants et ancré publiquement.",
                    late: "Votre bulletin figure dans le registre public et confirmé par les témoins, mais ses ancrages publics sont en retard.",
                    lateStep:
                        "Une étape prend plus de temps que prévu. Ce n’est pas une preuve de falsification, et votre navigateur continue de vérifier.",
                    mismatch:
                        "L’entrée n° {{index}} du registre public n’est pas celle de ce justificatif.",
                    mismatchNoEntry:
                        "L’entrée du registre public n’est pas celle de ce justificatif.",
                    damaged:
                        "Une partie du justificatif ou du registre n’a pas pu être lue ou vérifiée. C’est une erreur de données, pas une preuve de falsification. Votre vote a été émis.",
                    noReceipt:
                        "Ce site n’a reçu aucun justificatif pour votre bulletin. Votre vote a été émis, et les observateurs de l’élection ont été informés.",
                    unchecked:
                        "Ce site n’a pas pu vérifier le justificatif. Votre vote a été émis.",
                    auditable:
                        "Vous avez audité ce bulletin ; il n’a donc jamais été émis. Il figure dans le registre public, et le suivi des bulletins le déchiffre pour afficher les sélections qu’il contient.",
                    auditfail:
                        "Le bulletin figurant dans le registre public ne se déchiffre pas avec la clé révélée par le portail. Ne votez pas depuis cet appareil ; informez l’autorité électorale.",
                },
                step: {
                    record: {
                        title: "Dans le registre public",
                        upcoming:
                            "Votre navigateur trouve le bulletin dans le registre public de l’élection.",
                    },
                    witnesses: {
                        title: "Confirmé par des témoins",
                        upcoming: "Des témoins indépendants signent un registre qui l’inclut.",
                    },
                    anchors: {
                        title: "Ancré publiquement",
                        upcoming:
                            "Des nœuds publics, et les journaux que vérifient les navigateurs, conservent ce registre.",
                    },
                },
                row: {
                    waiting: "En attente de l’étape précédente.",
                    checking: "Votre navigateur vérifie…",
                    anchoring:
                        "Nœuds publics, puis là où les navigateurs vérifient : d’ici {{minutes}} minutes.",
                    recorded: "Votre navigateur a vérifié sa preuve.",
                    recordedEntry: "Entrée n° {{index}}. Votre navigateur a vérifié sa preuve.",
                    witnessed:
                        "Assez de témoins indépendants de la politique ({{count}}) ont signé un registre qui l’inclut.",
                    anchoredWaict:
                        "Dans les nœuds publics ({{count}}) et là où les navigateurs vérifient.",
                    anchoredBootstrap:
                        "Dans les nœuds publics ({{count}}) et là où les navigateurs vérifient, par un nœud de substitution pour l’instant.",
                    anchoredHubs: "Dans les nœuds publics ({{count}}).",
                    lateNow: "Cela devrait déjà être fait.",
                    lateStep: "Cela devrait être fait à {{time}}.",
                    lateAnchors: "Les nœuds auraient dû l’ancrer à {{time}}.",
                    mismatch: "L’entrée n° {{index}} du registre diffère de votre justificatif.",
                    mismatchNoEntry: "Une entrée du registre diffère de votre justificatif.",
                    contradiction: "Ses preuves signées se contredisent.",
                    damaged: "Sa preuve est endommagée et n’a pas pu être vérifiée.",
                    noReceipt: "Ce site n’a reçu aucun justificatif.",
                    unchecked: "Ce site n’a pas pu vérifier le justificatif.",
                    skipped: "Non vérifié, car l’étape précédente n’a pas pu être achevée.",
                    skippedMismatch: "Non vérifié, car le registre ne correspond pas.",
                },
                word: {
                    withinSeconds: "Sous quelques secondes",
                    withinMinutes: "Sous {{minutes}} min",
                    checking: "Vérification…",
                    late: "En retard",
                    mismatch: "Différent",
                    damaged: "Endommagé",
                    noReceipt: "Pas de justificatif",
                    unchecked: "Non vérifié",
                    skipped: "—",
                },
                callout: {
                    late: "Cela signifie en général qu’un nœud est lent ou hors ligne ; ce n’est pas une preuve de falsification.",
                    lateKept:
                        "Cet appareil vérifie de nouveau à chaque retour, et le suivi des bulletins peut vérifier depuis n’importe quel appareil.",
                    lateTracker:
                        "Le suivi des bulletins peut vérifier depuis n’importe quel appareil.",
                    evidence:
                        "Votre navigateur a conservé les preuves : ce justificatif et l’entrée signée du registre. Envoyez-les à l’autorité électorale ou à un observateur ; tout le monde peut les vérifier.",
                    saveEvidence: "Enregistrer les preuves",
                },
                caption: {
                    notcast:
                        "Rien n’est vérifié tant que vous ne l’avez pas émis. Ensuite, cette coche montre jusqu’où votre bulletin a été vérifié.",
                    checking:
                        "Votre navigateur vérifie le registre public, avec un code que chacun peut inspecter.",
                    problem:
                        "Constaté par votre navigateur, avec un code que chacun peut inspecter.",
                    auditable:
                        "Déchiffré par votre navigateur, avec un code que chacun peut inspecter.",
                    verified:
                        "Vérifié par votre navigateur, avec un code que chacun peut inspecter.",
                    checked:
                        "Contrôlé par votre navigateur, avec un code que chacun peut inspecter.",
                    kept: "Cet appareil conserve le justificatif et le vérifie de nouveau à chaque visite.",
                },
            },
            booth: {
                title: "Votre bulletin ne peut pas encore être préparé",
                explanation:
                    "Avant de chiffrer votre bulletin, ce site vérifie que la configuration de l'élection est celle publiée dans le registre public de l'élection. Votre bulletin n'a été ni chiffré ni envoyé, et vos choix sont conservés.",
                pending:
                    "La configuration publiée n'est pas encore disponible. Veuillez réessayer dans un instant.",
                invalid:
                    "La configuration publiée n'a pas pu être lue. Veuillez réessayer dans un instant.",
                failed: "La configuration ne correspond pas à celle publiée. Les observateurs de l'élection ont été prévenus. Veuillez contacter l'assistance.",
                retry: "Réessayer",
                close: "Fermer",
            },
            record: {
                json: "JSON",
                text: "Texte",
                fields_one: "{{count}} champ",
                fields_other: "{{count}} champs",
                items_one: "{{count}} élément",
                items_other: "{{count}} éléments",
                size: {
                    bytes: "{{value}} o",
                    kilobytes: "{{value}} Ko",
                },
                copy: "Copier {{file}}",
                copied: "Copié",
                download: "Télécharger {{file}}",
            },
        },
    },
}

export default frenchTranslation
