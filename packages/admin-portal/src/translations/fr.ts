// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const frenchTranslation: TranslationType = {
    translations: {
        philippinePassport: "Passeport philippin",
        seamanBook: "Livret de marin",
        philSysID: "Carte d'identité PhilSys",
        iBP: "Barreau intégré des Philippines (IBP)",
        driversLicense: "Permis de conduire",
        loading: "Chargement...",
        loadingDataProvider: "Chargement du fournisseur de données...",
        tallySheetImport: {
            title: "Importations de procès-verbaux de dépouillement",
            subtitle:
                "Importez des fichiers de procès-verbaux de dépouillement ES&S ou CSV, prévisualisez les urnes générées et approuvez-les avant de créer des procès-verbaux de dépouillement.",
            createTitle: "Importer des procès-verbaux de dépouillement",
            detailTitle: "Importation de procès-verbal de dépouillement",
            empty: "Aucune importation de procès-verbal de dépouillement pour le moment.",
            emptyBody:
                "Commencez par importer un fichier ES&S Enhanced XML ou CSV canonique pour cet événement électoral.",
            sourceFormat: {
                ESS_ENHANCED_XML: "ES&S Enhanced XML",
                CANONICAL_CSV: "CSV canonique",
            },
            channel: {
                PAPER: "Papier",
                POSTAL: "Postal",
                IN_PERSON: "En personne",
            },
            table: {
                created: "Créé",
                createdBy: "Créé par",
                file: "Fichier",
                format: "Format",
                channel: "Canal",
                status: "Statut",
                labels: "Étiquettes",
                annotations: "Annotations",
                actions: "Actions",
            },
            summary: {
                imported: "Importées",
                changed: "Modifiées",
                new: "Nouvelles",
                unchanged: "Inchangées",
                conflicted: "En conflit",
                errors: "Erreurs",
            },
            status: {
                PENDING_REVIEW: "En attente de révision",
                APPROVED: "Approuvée",
                DISAPPROVED: "Désapprouvée",
                FAILED_VALIDATION: "Échec de la validation",
                CONFLICTED: "En conflit",
                NEW: "Nouvelle",
                CHANGED: "Modifiée",
                UNCHANGED: "Inchangée",
            },
            fields: {
                format: "Format",
                channel: "Canal",
                supportedFormats: "Formats pris en charge : XML, CSV",
                generatedTallySheet: "Procès-verbal de dépouillement généré",
                sourceCandidates: "IDs des candidats source",
                none: "Aucun",
            },
            actions: {
                create: "Importer des procès-verbaux de dépouillement",
                review: "Réviser",
                source: "Source",
                cancel: "Annuler",
                preview: "Prévisualiser",
                save: "Enregistrer l'importation",
                approve: "Approuver",
                disapprove: "Désapprouver",
                close: "Fermer",
                openExisting: "Ouvrir l'existante",
            },
            notifications: {
                selectFile: "Sélectionnez un fichier d'importation avant de le prévisualiser",
                duplicateSource:
                    "Ce hash de fichier source apparaît déjà dans une importation de procès-verbal de dépouillement précédent.",
                uploadUrlError: "Impossible de créer l'URL de téléversement",
                uploadError: "Impossible de téléverser le fichier d'importation",
                previewEmpty: "La réponse de la prévisualisation était vide",
                previewError: "Impossible de prévisualiser l'importation",
                importEmpty: "La réponse de l'importation était vide",
                created: "Importation de procès-verbal de dépouillement créé",
                createError: "Impossible de créer l'importation",
                reviewEmpty: "La réponse de la révision était vide",
                conflicted: "L'importation présente des conflits de référence obsolètes",
                approved: "Importation approuvée",
                disapproved: "Importation désapprouvée",
                reviewError: "Impossible de réviser l'importation",
                sourceUrlError: "Impossible de créer l'URL de téléchargement source",
                sourceDownloadError: "Impossible de télécharger le fichier source",
            },
            pagination: {
                range: "{{rangeStart}}-{{rangeEnd}} sur {{total}}",
                previous: "Précédent",
                next: "Suivant",
            },
        },
        reconciliation: {
            menuButton: "Sync. élect. ext.",
            categories: {
                VOTED_INTERNET: "A voté par Internet",
                VOTED_OTHER_CHANNEL: "A voté par un autre canal",
                DISABLED_DELETE_CALL: "Électeur désactivé",
                DELETION_REVERTED: "Suppression annulée",
                PROFILE_UPDATE: "Profil mis à jour",
                VOTER_ADDED: "Électeur ajouté",
                REENABLED: "Électeur réactivé",
                VOTED_UNMARKED: "Vote de l’électeur annulé",
                ROW_FAILURE: "Échec de ligne",
            },
            table: {
                voterId: "ID Électeur",
                field: "Champ",
                category: "Catégorie",
                currentValue: "Valeur actuelle",
                newValue: "Nouvelle valeur",
                reason: "Motif",
                rowLabel: "Ligne",
                noDifferences: "Aucune différence trouvée - les systèmes sont synchronisés.",
            },
            wizard: {
                title: "Synchronisation de réconciliation externe",
                subtitle: "Synchronisez la liste des électeurs avec le système externe",
                drop: {
                    description:
                        "Déposez le fichier de réconciliation généré par le système externe - les deux diffs (côté externe et côté Sequent) sont calculés automatiquement et affichés dans des tableaux séparés.",
                    fileFormatLabel: "Fichier CSV",
                    uploading: "Téléversement de {{fileName}} et calcul des deux diffs...",
                },
                review: {
                    fileSummary: "{{fileName}} - Séquence {{sequence}}, généré {{generatedAt}}",
                    rowFailuresWarning:
                        "{{count}} ligne(s) n'ont pas pu être réconciliées en toute sécurité et sont exclues des deux diffs - voir les détails ci-dessous.",
                    noDifferences: "Aucune différence - les deux systèmes sont déjà synchronisés.",
                    diffOnlyDifferences:
                        "Il s'agit d'un contrôle de convergence pour une séquence déjà appliquée. Les différences sont affichées pour suivi, mais ce cycle ne peut pas être appliqué à nouveau.",
                    externalDiffTitle: "Diff externe",
                    sequentDiffTitle: "Diff Sequent",
                    downloadExternalPatch: "Télécharger le correctif externe",
                    externalDiffCaption:
                        "Téléchargez le correctif et remettez-le au système externe en dehors de cet outil. Une fois qu'il l'applique et produit le fichier de réconciliation suivant, cliquez sur 'Retour' et déposez ce fichier - 'Appliquer' se débloque une fois ce tableau vide.",
                    noExternalDifferences: "Aucune différence côté externe.",
                    sequentDiffCaption:
                        "Appliquez les changements directement à Sequent en cliquant sur 'Appliquer' - aucun fichier de correctif n'est généré pour ceux-ci.",
                    noSequentDifferences: "Aucune différence côté Sequent.",
                },
                applying: {
                    inProgress: "Application des changements côté Sequent...",
                    rowFailures:
                        "{{count}} ligne(s) ont été exclues de ce cycle et nécessitent un suivi manuel - voir les détails ci-dessous.",
                    rowFailuresTruncated:
                        "Affichage des {{shown}} premiers échecs de ligne sur {{count}}. Corrigez la cause commune et réessayez pour voir les échecs restants.",
                    success: "Tous les changements côté Sequent ont été appliqués avec succès.",
                },
                actions: {
                    cancel: "Annuler",
                    back: "Retour",
                    apply: "Appliquer",
                    next: "Suivant",
                    startOver: "Recommencer",
                    close: "Fermer",
                },
                confirm: {
                    title: "Confirmer les changements de réconciliation",
                    categoriesNote:
                        "Les catégories surlignées en orange ({{categories}}) affectent le statut de vote ou désactivent des électeurs.",
                    applyChanges: "Appliquer les changements",
                    continue: "Continuer",
                },
                summary: {
                    votedOtherChannel:
                        "marque {{count}} électeur(s) comme ayant voté par un autre canal",
                    disabled: "désactive {{count}} électeur(s)",
                    reenabled: "réactive {{count}} électeur(s)",
                    votedUnmarked: "annule le vote de {{count}} électeur(s)",
                    profileUpdated: "met à jour {{count}} profil(s)",
                    voterAdded: "ajoute {{count}} électeur(s)",
                    prefix: "Ceci appliquera des changements qui {{parts}}.",
                    empty: "Il n'y a aucun changement côté Sequent à appliquer.",
                },
                notifications: {
                    envelopeLoadError:
                        "Impossible de charger le diff de réconciliation - veuillez réessayer.",
                    generateFailed:
                        "Impossible de calculer le diff de réconciliation - consultez le widget de tâches pour plus de détails.",
                    applyFailed:
                        "Impossible d'appliquer les changements côté Sequent - consultez le widget de tâches pour plus de détails.",
                    uploadUrlError: "Impossible d'obtenir une URL de téléversement",
                    generateTaskError: "Impossible de démarrer la tâche de diff de réconciliation",
                    uploadError: "Impossible de téléverser le fichier de réconciliation",
                    applyTaskError: "Impossible de démarrer la tâche d'application",
                    applyError: "Impossible d'appliquer les changements de réconciliation",
                },
            },
        },
        logsScreen: {
            noPermissions: "Vous n'avez pas la permission d'accéder aux journaux.",
            title: "Journaux",
            subtitle: "Journaux généraux des bases de données principale et IAM.",
            actions: {
                csv: "Exporter en CSV",
                pdf: "Exporter en PDF",
            },
            exportdialog: {
                description:
                    "Veuillez confirmer que vous souhaitez exécuter cette action, cela pourrait prendre un certain temps.",
                title: "Exporter les journaux",
                from: "Du",
                to: "Au",
                timeZone: "Fuseau horaire",
                format: "Format",
                csv: "CSV",
                pdf: "PDF",
                zoneNote:
                    "Chaque ligne conserve son heure en UTC (ISO 8601) et en {{abbr}}, avec le nom du fuseau horaire. La plage de dates inclut les deux bornes, en {{abbr}}.",
                zoneNotePdf:
                    "Le PDF affiche chaque heure en {{abbr}}. La plage de dates inclut les deux bornes, en {{abbr}}.",
                rowZones: "Le fuseau horaire de l'élection de chaque ligne",
                zoneNoteRows:
                    "Chaque ligne conserve son heure en UTC (ISO 8601) et dans le fuseau horaire de son élection, avec le nom du fuseau horaire. La plage de dates inclut les deux bornes, en {{abbr}}.",
                zoneNoteRowsPdf:
                    "Le PDF affiche chaque heure dans le fuseau horaire de son élection. La plage de dates inclut les deux bornes, en {{abbr}}.",
            },
            filter: {
                createdFrom: "Créé du",
                createdTo: "au",
                statementTimestampFrom: "Horodatage de la déclaration du",
                statementTimestampTo: "Horodatage de la déclaration au",
                timeZone: "Fuseau horaire",
            },
            scheduledOutcome: {
                outcome: {
                    "waiting-for-initialization": "En attente d’initialisation",
                    "runs": "s'exécute",
                    "runs-unsigned": "s'exécute sans signatures",
                    "refused": "est refusée",
                },
                check: {
                    "initialization": "L’initialisation requise est incomplète",
                    "voting-close": "Le vote ne peut pas ouvrir après sa date limite de clôture",
                    "needs-signatures": "signatures requises",
                    "covered": "dans la configuration signée",
                    "unsigned-close": "fermeture sans signatures",
                    "stricter-copy": "paramètres actuels ou publiés",
                    "defaults": "rien n'est encore publié",
                },
                changed: "Désormais {{after}} (avant : {{before}}).",
                result: "Résultat : {{outcome}}.",
                deciding: "Vérification décisive : {{check}}. {{value}}",
                authorizedBy: "Autorisée par la configuration {{code}}.",
                nextStep: "Étape suivante : {{step}}",
                reason: {
                    "ballot-box-seal-policy":
                        "Maintenu clos : avec Sceller à la clôture, un vote clôturé reste clos.",
                    "never-opened-kept-open":
                        "Rien à clôturer selon le programme : le Poste n'a jamais ouvert, il reste donc en l'état.",
                },
            },
            column: {
                id: "ID",
                statement_kind: "Type de déclaration",
                created: "Créé",
                statement_timestamp: "Horodatage de déclaration",
                message: "Message",
                user_id: "ID utilisateur",
                username: "Nom d'Utilisateur",
                sender_pk: "Clé primaire de l'expéditeur",
                log_type: "Type de journal",
                event_type: "Type d'événement",
                description: "Description",
                version: "Version",
            },
            main: {
                title: "Journal de la Base de Données Principale",
            },
            iam: {
                title: "Journal de la Base de Données IAM",
            },
            ballotBoxSeal: {
                sealHash: "Hash du scellé : {{hash}}",
                counted: "{{counted}} bulletins comptés sur {{inBox}}.",
                notCounted_one:
                    "L'autre bulletin a été remplacé par un bulletin ultérieur de l'électeur, écarté, ou déposé par un électeur non éligible.",
                notCounted_other:
                    "Les {{count}} autres bulletins ont été remplacés par un bulletin ultérieur de l'électeur, écartés, ou déposés par un électeur non éligible.",
                closeRequest: "Clôturé par la demande de clôture du vote {{request}}.",
                noCloseRequest:
                    "Clôturé sans demande de clôture du vote (Arrêter la période de vote ou fermeture programmée).",
                failedReason: "Motif : {{reason}}",
                failedLocked: "L'urne reste verrouillée et n'est pas scellée : c'est un incident.",
                verifiedCounted: "{{counted}} bulletins comptés à partir du scellé.",
                tallySession: "Session de dépouillement {{session}}.",
                differs: "Différences : {{differs}}",
            },
        },
        tasksScreen: {
            noPermissions: "Vous n'avez pas la permission d'accéder aux journaux.",
            title: "Exécution des tâches",
            subtitle: "Informations sur les tâches exécutées et les journaux de progression",
            taskInformation: "Informations sur la tâche",
            status: "statut : {{status}}",
            ok: "D'accord",
            column: {
                id: "Index",
                name: "Nom de la tâche",
                type: "Type",
                execution_status: "Statut",
                start_at: "Heure de début",
                end_at: "Heure de fin",
                executed_by_user: "Exécutant",
                annotations: "Annotations",
                labels: "Étiquettes",
                logs: "Journaux",
            },
            tasksExecution: {
                DELETE_TENANT: "Supprimer le locataire",
                PUBLISH_BALLOT: "Publier le bulletin",
                VOTER_INFORMATION_LETTER: "Lettre d'information de l'électeur",
                EXPORT_MONITORING_DATA: "Exporter les Données de Suivi",
                EXPORT_ELECTION_EVENT: "Exporter l'événement électoral",
                CREATE_ELECTION_EVENT: "Créer Événement Électoral",
                IMPORT_ELECTION_EVENT: "Importer l'événement électoral",
                IMPORT_USERS: "Importer des utilisateurs",
                EDIT_USER: "Modifier l'électeur",
                IMPORT_CANDIDATES: "Importer des candidats",
                EXPORT_VOTERS: "Exporter les électeurs",
                CREATE_TRANSMISSION_PACKAGE: "Créer un paquet de transmission",
                EXPORT_BALLOT_PUBLICATION: "Exporter Publication de Bulletin",
                EXPORT_ACTIVITY_LOGS_REPORT: "Exporter le Rapport des Journaux d'Activité",
                GENERATE_REPORT: "Générer un rapport",
                GENERATE_TRANSMISSION_REPORT: "Générer un rapport de transmission",
                EXPORT_TRUSTEES: "Exporter les Autorités",
                EXPORT_APPLICATION: "Exporter les Demandes",
                EXPORT_TENANT_CONFIG: "Exporter la Configuration du Locataire",
                IMPORT_TENANT_CONFIG: "Importer la Configuration du Locataire",
                RENDER_DOCUMENT_PDF: "Générer le document PDF",
                CREATE_TENANT: "Créer un locataire",
                EXPORT_TEMPLATES: "Exporter les Modèles",
                IMPORT_TEMPLATES: "Importer les Modèles",
                DELETE_ELECTION_EVENT: "Supprimer l'événement électoral",
                DELETE_VOTERS: "Delete Voters",
                PREPARE_PUBLICATION_PREVIEW: "Préparer l'aperçu de la publication",
                EXPORT_TALLY_RESULTS_XLSX: "Exporter les résultats du dépouillement au format XLSX",
                EXPORT_CERTIFICATE_AUTHORITIES: "Exporter les autorités de certification",
                PUBLISH_RESULTS_WEBSITE: "Publier le site des résultats",
            },
            documentAccess: {
                title: "Accès au document",
                sensitivityNotice:
                    "Informations sensibles. Ne partagez ce mot de passe qu'avec le destinataire prévu.",
                passwordLabel: "Mot de passe pour ouvrir le PDF chiffré",
                showPassword: "Afficher le mot de passe",
                copyPassword: "Copier le mot de passe",
                passwordCopied: "Mot de passe copié",
                passwordError: "Impossible de récupérer le mot de passe du PDF",
                copyError: "Impossible de copier le mot de passe",
                guidance:
                    "Le mot de passe n'est chargé qu'après avoir choisi Afficher le mot de passe. Une fois chargé, un champ en lecture seule avec une option de copie apparaît ici.",
            },
            widget: {
                taskTitle: "Tâche: {{title}}",
                viewTask: "Voir Tâche",
                downloadDocument: "Télécharger le Fichier",
                downloadHashManifest: "Manifeste des empreintes",
            },
            exportTasksExecution: {
                success: "L'exportation s'est terminée avec succès",
                error: "Erreur lors de l'exportation de l'exécution des tâches",
            },
        },
        areas: {
            common: {
                title: "Zones",
                subTitle: "Configuration de Zone.",
                deleteError: "Erreur lors de la suppression de la Zone",
            },
            createAreaSuccess: "Zone créée",
            updateAreaSuccess: "Zone mise à jour",
            createAreaError: "Erreur lors de la création de la zone",
            sequent_backend_area_contest: "Questions de la Zone",
            empty: {
                header: "Aucune Zone pour l'instant.",
                action: "Créer une Zone",
            },
            formImputs: {
                allowEarlyVoting: "Autoriser le Vote Anticipé",
            },
        },
        integrationsScreen: {
            common: {
                gapiKey: "Clé de Compte de Service Google Calendar",
                gapiEmail: "Email d'Authentification Google Calendar",
                gapiKeyHelper:
                    "La clé enregistrée n'est pas affichée. Collez une nouvelle clé pour la remplacer.",
                gapiKeySaved: "Clé de Compte de Service Google Calendar enregistrée",
            },
            errors: {
                invalidGapiKey: "Format de Clé de Compte de Service Google Calendar invalide",
                saveGapiKey: "Impossible d'enregistrer la Clé de Compte de Service Google Calendar",
            },
        },
        lookAndFeelScreen: {
            common: {
                helpLinks: "Liens d'Aide",
                logoUrl: "URL du Logo",
                css: "CSS Personnalisé",
                displayName: "Nom affiché",
                displayNameHelp:
                    "Le nom de l'organisation dans les messages qui la mentionnent. Vide : le nom court du locataire.",
            },
            errors: {
                invalidHelpLinks: "Format des Liens d'Aide invalide",
            },
        },
        electionTypeScreen: {
            noPermissions: "Vous n'avez pas la permission d'accéder à la configuration.",
            common: {
                title: "Type d'Élection",
                subtitle: "Configuration du Type d'Élection",
                onlineVoting: "Vote en Ligne",
                kioskVoting: "Vote au Kiosque",
                telephoneVoting: "Vote par Téléphone",
                settingTitle: "Configuration",
                settingSubtitle: "Paramètres généraux",
                createNew: "Créer un Type d'Élection",
                emptyHeader: "Aucun Type d'Élection pour l'instant.",
                emptyBody: "Voulez-vous en créer un ?",
            },
            create: {
                title: "Créer un Type d'Élection",
            },
            edit: {
                title: "Éditer un Type d'Élection",
            },
            tabs: {
                votingChannels: "CANAUX DE VOTE",
                electionTypes: "TYPES D'ÉLECTION",
                languages: "LANGUES",
                localization: "LOCALISATION",
                integrations: "INTÉGRATIONS",
                lookAndFeel: "PERSONNALISATION DE L'APPARENCE",
                schedules: "ÉVÉNEMENTS PROGRAMMÉS",
                trustees: "AUTORITÉS",
                BackupRestore: "SAUVEGARDE / RESTAURATION",
            },
        },
        trusteesSettingsScreen: {
            common: {
                emptyHeader: "Pas encore de fiduciaires.",
                createNew: "Créer un fiduciaire",
                title: "Fiduciaire",
                subtitle: "Configuration du fiduciaire",
                emptyBody: "Voulez-vous en créer un?",
            },
            create: {
                title: "Créer un fiduciaire",
            },
            edit: {
                title: "Modifier le fiduciaire",
            },
        },
        scheduleScreen: {
            noPermissions: "Vous n'avez pas la permission d'accéder à la configuration.",
            createScheduleSuccess: "Date créée",
            createScheduleError: "Erreur lors de la création de la date",
            deleteScheduleSuccess: "Date supprimée",
            deleteScheduleError: "Erreur lors de la suppression de la date",
            common: {
                title: "Programmé",
                subtitle: "Configuration des dates",
                createNew: "Créer une date",
                emptyHeader: "Aucune date pour l'instant.",
                emptyBody: "Voulez-vous en créer une ?",
            },
            create: {
                title: "Créer une date",
                selectSchedule:
                    "Sélectionnez un horaire de la liste prédéfinie ou écrivez-en un personnalisé",
            },
            edit: {
                title: "Éditer une date",
            },
            eventTypes: {
                SYSTEM_LOCKDOWN_FOR_INTERNET_VOTING_SETTINGS:
                    "Verrouillage du système pour la finalisation de la configuration de vote par Internet",
                START_PRE_REGISTRATION_OVCS: "Début de la pré-inscription pour OVCS",
                END_PRE_REGISTRATION_OVCS: "Fin de la pré-inscription pour OVCS",
                START_TEST_VOTING_PERIOD: "Début de la période de vote de test",
                END_TEST_VOTING_PERIOD: "Fin de la période de vote de test",
                START_INTERNET_VOTING_PERIOD: "Début de la période de vote par Internet",
                END_INTERNET_VOTING_PERIOD: "Fin de la période de vote par Internet",
                LAB_TEST: "Test de laboratoire",
                FIELD_TEST: "Test sur le terrain",
                MOCK_ELECTIONS: "Élections simulées",
                FTS: "FTS",
            },
        },
        dashboard: {
            voteByDay: "Votes par jour",
            votesOverTime: "Votes au fil du temps",
            timeResolution: "Résolution temporelle",
            timeRange: "Plage de temps",
            minute: "Minute",
            hour: "Heure",
            day: "Jour",
            votersByChannels: "Votants par canaux",
            voterLoginURL: "URL de connexion des électeurs",
            voterEnrollURL: "URL d'inscription des électeurs",
            voterEnrollKioskURL: "Kiosk URL d'inscription des électeurs",
            ballotBoxes: {
                show: "Afficher les urnes",
                loadError:
                    "Les scellés des urnes n'ont pas pu être lus. Rechargez la page ou vérifiez la connexion au serveur.",
                title: "Urnes",
                sealing:
                    "Le vote a été clôturé à {{closed}}. Les urnes sont scellées à la fin du délai de grâce, à {{deadline}}.",
                sealed: "Le vote a été clôturé à {{closed}}. Les urnes sont scellées : aucun bulletin ne peut être ajouté, modifié ni supprimé.",
                failed: "Le vote a été clôturé à {{closed}}. Une urne n'a pas pu être scellée : elle reste verrouillée, et l'incident figure dans les journaux.",
                closedBySignatures:
                    "Clôturé par {{names}} avec leurs certificats, code de signature {{code}}.",
                closedByUser: "Clôturé par {{username}}.",
                closedBySchedule: "Clôturé par la fermeture programmée du scrutin.",
                column: {
                    area: "Zone",
                    status: "Statut",
                    inTheBox: "Dans l'urne",
                    counted: "Comptés",
                    sealedAt: "Scellée",
                    sealHash: "Hash du scellé",
                    record: "Procès-verbal de scellement",
                },
                status: {
                    open: "Ouverte",
                    sealing: "Scellement à {{time}}",
                    publishing: "Scellée, publication en cours",
                    sealed: "Scellée",
                    failed: "Non scellée : incident",
                    due: "Scellement en cours",
                    overdue: "Scellement en retard",
                },
                help: {
                    publishing:
                        "L'urne est verrouillée. Son entrée est en cours de republication sur le tableau d'affichage.",
                    counted:
                        "Bulletins comptés : le dernier bulletin valide de chaque électeur éligible. Les autres bulletins de l'urne ont été remplacés par un bulletin ultérieur de l'électeur, écartés, ou déposés par un électeur non éligible.",
                },
                copyHash: "Copier le hash du scellé",
                copied: "Hash du scellé copié",
                copyError: "Impossible de copier le hash du scellé",
                notYet: "Pas encore",
                openRecord: "Ouvrir le procès-verbal de scellement de {{area}}",
                downloadRecord: "Télécharger le procès-verbal de scellement de {{area}}",
                recordRestricted:
                    "Restreint : demandez-le à un administrateur autorisé à télécharger des documents.",
                recordError:
                    "Le procès-verbal de scellement n'a pas pu être téléchargé. Réessayez.",
                recordMissing:
                    "Le document du procès-verbal de scellement est manquant : signalez-le comme incident.",
                beforeClose: "L'urne de chaque zone est scellée à la clôture du vote.",
                notStarted:
                    "Le vote n'a pas encore ouvert. L'urne de chaque zone est scellée à la clôture du vote.",
                openOn: "Le vote est ouvert sur {{channels}}. L'urne de chaque zone est scellée à la clôture du vote.",
                paused: "Le vote est en pause. L'urne de chaque zone est scellée à la clôture du vote.",
                holding_one:
                    "{{channels}} est activé et non clôturé : arrêtez-le pour sceller les urnes.",
                holding_other:
                    "{{channels}} sont activés et non clôturés : arrêtez-les pour sceller les urnes.",
                sealingNow:
                    "Le vote a été clôturé à {{closed}}. Les urnes sont en cours de scellement.",
                sealingPastGrace:
                    "Le vote a été clôturé à {{closed}}. Le délai de grâce a pris fin à {{deadline}} ; les urnes sont en cours de scellement.",
                why: {
                    due: "Scellement en cours : cela peut prendre jusqu'à une minute.",
                    channelOpen:
                        "{{channel}} est encore activé et non clôturé : arrêtez-le pour sceller l'urne.",
                    channelNotEnabled:
                        "{{channel}} n'est pas clôturé et n'est pas activé pour cette élection : arrêtez-le pour sceller l'urne.",
                    channelHasBallots:
                        "{{channel}} a des bulletins dans cette urne et n'est pas clôturé : arrêtez-le pour sceller l'urne.",
                    datafixVotes_one:
                        "{{count}} vote est en cours dans Datafix : l'urne est scellée une fois qu'il est résolu.",
                    datafixVotes_other:
                        "{{count}} votes sont en cours dans Datafix : l'urne est scellée une fois qu'ils sont résolus.",
                    stale: "Dernière tentative à {{time}} : le service de scellement n'est peut-être pas en cours d'exécution. Vérifiez Beat et le worker de scellement.",
                    notTried:
                        "Aucune tentative pour l'instant : le service de scellement n'est peut-être pas en cours d'exécution. Vérifiez Beat et le worker de scellement.",
                    errorCategory: {
                        board: "La dernière tentative n'a pas pu joindre le tableau d'affichage ; une nouvelle tentative a lieu chaque minute.",
                        census: "La dernière tentative n'a pas pu lire la liste des électeurs ; une nouvelle tentative a lieu chaque minute.",
                        keystore:
                            "La dernière tentative n'a pas pu obtenir la clé de signature ; une nouvelle tentative a lieu chaque minute.",
                        storage:
                            "La dernière tentative n'a pas pu téléverser le procès-verbal de scellement vers le stockage de fichiers ; une nouvelle tentative a lieu chaque minute.",
                        settings:
                            "La dernière tentative n'a pas pu lire les paramètres de l'élection ; une nouvelle tentative a lieu chaque minute.",
                        other: "La dernière tentative a échoué ; une nouvelle tentative a lieu chaque minute. Le journal du service contient les détails.",
                        ballots:
                            "La dernière tentative a trouvé un bulletin qui ne peut pas encore être lu ou qui est encore en cours ; une nouvelle tentative a lieu chaque minute.",
                        database:
                            "La dernière tentative n'a pas pu aboutir dans la base de données ; une nouvelle tentative a lieu chaque minute.",
                    },
                },
                failure: {
                    ballotIdMismatch: "Un bulletin ne correspond pas à son ID de bulletin.",
                    missingContent: "Un bulletin n'a pas de contenu ou pas d'ID de bulletin.",
                    unreadable: "Un bulletin ne peut pas être lu.",
                    inProgress: "Un bulletin est encore en cours.",
                    alreadyOnBoard:
                        "Un scellé de cette urne figure déjà sur le tableau d'affichage.",
                    noBoard: "L'événement électoral n'a pas de tableau d'affichage.",
                    unknownChannel: "Un bulletin a un canal de vote inconnu.",
                },
                incident: {
                    title_one: "{{count}} urne n'a pas pu être scellée",
                    title_other: "{{count}} urnes n'ont pas pu être scellées",
                    body: "Il s'agit d'un incident : chacune de ces urnes reste verrouillée et ne peut pas être dépouillée. Suivez la procédure prévue en cas d'échec du scellement.",
                    line: "{{election}}, {{area}} : {{reason}}",
                },
            },
            ipAddress: {
                emptyState: "Pas encore de votes.",
                title: "IP Addresses",
                ip: "IP",
                country: "Pays",
                VoteCount: "Nombre de votes",
                ElectionName: "Nom de l'élection",
                VotersId: "Identifiants des votants",
            },
        },
        electionEventScreen: {
            common: {
                subtitle: "Configuration de l'Événement Électoral.",
                showMore: "Afficher plus",
                showLess: "Afficher moins",
                adminPortal: "Portail d'administration",
                allowPublishAfterLockdown: "Only allow election event publishing after lockdown",
                reset: "Réinitialiser le filtre personnalisé",
            },
            edit: {
                general: "Général",
                dates: "Dates",
                customUrls: "Préfixer les URL personnalisées",
                votingPeriod: "Période de vote",
                language: "Langues",
                allowed: "Canaux de Vote Permis",
                materials: "Matériaux de Support",
                ballotReceipts: "Reçus de bulletin",
                ballotDesign: "Design du Bulletin",
                templates: "Modèles",
                reorder: "Réorganiser les élections",
                advancedConfigurations: "Voting Portal Countdown Policy",
                importCandidates: "Importer des Candidats",
                custom_filters: "Filtres personnalisés",
                voter_authentication: "Authentification des électeurs",
                realm_attributes: "Keycloak realm attributes",
                realm_attributes_load_error: "Error loading Keycloak realm attributes",
                realm_attributes_update_error: "Error updating Keycloak realm attributes",
                realm_attributes_not_loaded:
                    "Keycloak realm attributes have not loaded, changes were not saved",
                password_policy: "Password Policy",
                password_policy_load_error: "Error loading Keycloak password policy",
                password_policy_update_error: "Error updating Keycloak password policy",
                password_policy_not_loaded:
                    "Keycloak password policy has not loaded, changes were not saved",
            },
            customUrls: {
                login: "Connexion",
                enrollment: "Inscription",
            },
            localization: {
                emptyHeader: "Aucune langue n'a été définie pour l'événement",
                selectLanguage: "Sélectionnez la langue",
                notify: {
                    success: "Localisation mise à jour avec succès",
                    error: "Échec de la mise à jour de la localisation",
                    duplicateKey: "Un remplacement avec cette clé et cette portée existe déjà.",
                    invalidDateTimeFormat:
                        "Format de date/heure non valide. Utilisez les jetons yyyy, MM, dd, HH, mm, ss (ex. dd/MM/yyyy HH:mm).",
                    invalidTimeZoneText: "Ce texte doit conserver {{placeholders}}.",
                },
                common: {
                    title: "Localisation",
                    subTitle: "Configuration de la localisation",
                },
                labels: {
                    key: "Clé",
                    scope: "Portée du portail",
                    value: "Valeur",
                },
                scopes: {
                    legacy: "Hérité ({{portal}})",
                    global: "Global",
                    votingPortal: "Portail de vote",
                    ballotVerifier: "Vérificateur de bulletins",
                    resultsPortal: "Portail des résultats",
                    adminPortal: "Portail d'administration",
                    templates: "Rapports et messages",
                },
            },
            field: {
                passwordPolicy: {
                    minimumLength: "Longueur minimale",
                    maximumLength: "Longueur maximale",
                    includeUppercase: "Inclure des lettres majuscules",
                    includeLowercase: "Inclure des lettres minuscules",
                    includeDigits: "Inclure des chiffres",
                    includeSpecialCharacters: "Inclure des caractères spéciaux",
                    help: {
                        minimumLength:
                            "Le nombre minimal de caractères requis pour le mot de passe.",
                        maximumLength:
                            "Le nombre maximal de caractères autorisés dans le mot de passe.",
                        includeUppercase:
                            "Le mot de passe doit contenir au moins une lettre majuscule.",
                        includeLowercase:
                            "Le mot de passe doit contenir au moins une lettre minuscule.",
                        includeDigits: "Le mot de passe doit contenir au moins un chiffre.",
                        includeSpecialCharacters:
                            "Le mot de passe doit contenir au moins un caractère spécial.",
                    },
                    notConfigured:
                        "Aucune politique de mot de passe n'est configurée. L'enregistrement appliquera les valeurs par défaut ci-dessous.",
                    errors: {
                        lengthRange:
                            "Les longueurs du mot de passe doivent être des nombres entiers compris entre 1 et 256.",
                        minimumExceedsMaximum:
                            "La longueur minimale ne peut pas dépasser la longueur maximale.",
                        characterClassRequired:
                            "Sélectionnez au moins une classe de caractères pour le mot de passe.",
                    },
                },
                name: "Nom",
                alias: "Alias",
                description: "Description",
                startDateTime: "Date et heure de début",
                endDateTime: "Date et heure de fin",
                language: "Langue",
                votingChannels: "Canaux de Vote",
                materialActivated: "Matériaux de Support activés",
                supportMaterialsPolicy: {
                    label: "Politique des Matériaux de Support",
                    helperText:
                        "L'option Obligatoire pour Voter exige que les électeurs ouvrent chaque Matériau de Support et confirment qu'ils l'ont lu avant de pouvoir voter.",
                    options: {
                        off: "Désactivé",
                        optional: "Facultatif",
                        mandatory_for_voting: "Obligatoire pour Voter",
                    },
                },
                materialTitle: "Titre",
                materialSubTitle: "Sous-titre",
                logoUrl: "URL du Logo",
                userVerification:
                    "Vous pouvez introduire un modèle personnalisé qui sera utilisé pour vérifier manuellement les électeurs",
                redirectFinishUrl: "URL de redirection à la fin",
                kioskRedirectFinishUrl: "URL de redirection à la fin du vote en kiosque",
                css: "CSS personnalisé",
                skipElectionList: "Passer l'écran pour choisir l'élection",
                showUserProfile: "Afficher le profil utilisateur",
                ballotReceipts: {
                    checksPeriod: {
                        policyLabel: "Période de vérification des bulletins déposés",
                        helper: "Durée pendant laquelle les électeurs peuvent retrouver leur bulletin déposé et imprimer son reçu dans le Portail de vote.",
                        options: {
                            "unlimited": "Sans limite",
                            "until-date": "Jusqu'à une date",
                        },
                    },
                    checksAvailableUntil: "Vérifications disponibles jusqu'au ({{timezone}})",
                    checksAvailableUntilRequired:
                        "Saisissez la date et l'heure jusqu'auxquelles les bulletins peuvent être vérifiés.",
                },
                voterAccessibilitySettingsPolicy: {
                    policyLabel: "Paramètres d'accessibilité de l'électeur",
                    options: {
                        disabled: "Masquer les paramètres d'accessibilité",
                        enabled:
                            "Proposer la taille du texte, le contraste, l'espacement et les animations",
                    },
                },
                audioInstructionsPolicy: {
                    policyLabel: "Instructions audio",
                    options: {
                        "disabled": "Pas d'instructions audio",
                        "recorded": "Enregistrements téléversés uniquement",
                        "recorded-or-synthesized":
                            "Enregistrements téléversés, ou la voix du navigateur à défaut",
                    },
                },
                showCastVoteLogs: {
                    policyLabel: "Afficher les logs de vote",
                    options: {
                        "show-logs-tab": "Afficher l'onglet des logs de vote",
                        "hide-logs-tab": "Ne pas afficher l'onglet des logs de vote",
                    },
                },
                lockdownState: {
                    policyLabel: "État de Confinement",
                    helperText:
                        "Programmez le début ou la fin de la période de verrouillage pour modifier cet état.",
                    options: {
                        "locked-down": "Confiné",
                        "not-locked-down": "Non Confiné",
                    },
                },
                ballotBoxSealPolicy: {
                    policyLabel: "Politique de scellement des urnes",
                    helperText:
                        "À la clôture du vote, l'urne de chaque zone est scellée : un hash signé de ses bulletins est publié sur le tableau d'affichage, aucun bulletin ne peut être ajouté, modifié ni supprimé, et le vote ne peut pas reprendre.",
                    locked: "Elle ne peut plus être modifiée une fois le vote ouvert.",
                    options: {
                        "seal-at-close": "Sceller à la clôture",
                        "do-not-seal": "Ne pas sceller",
                    },
                    checking: "Vérification de l'ouverture du vote…",
                    lockedUnknown:
                        "Verrouillée : les élections n'ont pas pu être lues, on ne sait donc pas si le vote a ouvert.",
                    lockedOpened: "Verrouillée : le vote a ouvert dans {{names}}.",
                    lockedEvent: "Verrouillée : le vote a ouvert dans cet événement électoral.",
                    refused:
                        "La Politique de scellement des urnes ne peut plus être modifiée une fois le vote ouvert.",
                    settingLocked: "Avec Sceller à la clôture, le scellé dépend de ce paramètre.",
                    settingRefused:
                        "Ce paramètre ne peut plus être modifié une fois le vote ouvert : avec Sceller à la clôture, le scellé en dépend.",
                    boardRefused:
                        "Le tableau d'affichage de l'événement électoral ne peut plus changer une fois le vote ouvert : avec Sceller à la clôture, les scellés y sont publiés.",
                },
                ballotBoxSealRecordPolicy: {
                    policyLabel: "Procès-verbal de scellement des urnes",
                    options: {
                        restricted: "Restreint",
                        public: "Public",
                    },
                    help: {
                        restricted:
                            "Seuls les administrateurs peuvent le télécharger ; partagez-le avec les observateurs.",
                        public: "Toute personne connaissant les identifiants de l'événement peut le télécharger, sans se connecter ; il montre comment chaque bulletin a été compté.",
                    },
                },
                decodedBallots: {
                    policyLabel:
                        "Inclure les bulletins décodés dans la base de données de résultats",
                    options: {"included": "Inclure", "not-included": "Ne pas inclure"},
                },
                contestEncryptionPolicy: {
                    options: {
                        "single-contest": "Concours unique",
                        "multiple-contests": "Plusieurs concours",
                    },
                    policyLabel: "Politique de chiffrement de concours",
                },
                votingPortalDateTimeFormat: {
                    policyLabel: "Format de date et d'heure du portail de vote",
                    helperText:
                        "S'applique à tout l'événement. Pour le remplacer par langue, ajoutez la clé \"votingPortalDateTimeFormat\" dans l'onglet Localisation avec les jetons yyyy, MM, dd, HH, mm, ss (ex. dd/MM/yyyy HH:mm). Consultez la documentation pour plus de détails.",
                    options: {
                        "legacy-gb-24h": "Legacy GB 24h (dd/MM/yyyy HH:mm, 24h)",
                        "iso-local": "ISO Local (yyyy-MM-dd HH:mm)",
                        "us-12h": "US 12h (MM/dd/yyyy h:mm AM/PM)",
                        "locale-medium": "Locale Medium (date moyenne, heure courte)",
                        "date-only": "Date Only (sans heure)",
                        "custom": "Format personnalisé",
                    },
                    customFormat: {
                        label: "Format de date et d'heure personnalisé",
                        helperText:
                            "Utilisez les jetons yyyy, MM, dd, HH, mm, ss (ex. dd/MM/yyyy HH:mm). Tout autre caractère est affiché tel quel.",
                        invalid:
                            "Format invalide. Utilisez au moins un des jetons yyyy, MM, dd, HH, mm, ss.",
                    },
                },
                countDownPolicyOptions: {
                    NO_COUNTDOWN: "Pas de compte à rebours",
                    COUNTDOWN: "Compte à rebours",
                    COUNTDOWN_WITH_ALERT: "Compte à rebours avec alerte",
                    sectionTitle: "Portail de vote",
                    policyLabel: "Politique de compte à rebours du portail de vote",
                    coundownSecondsLabel:
                        "temps en secondes avant expiration pour afficher le compte à rebours",
                    alertSecondsLabel:
                        "temps en secondes avant expiration pour afficher l'alerte de déconnexion",
                },
                voterSigningPolicy: {
                    "policyLabel": "Politique de Signature des Électeurs",
                    "no-signature": "Sans signature",
                    "with-signature": "Avec signature",
                },
                receiptsPolicy: {
                    "policyLabel": "Reçus signés par l'urne",
                    "disabled": "Désactivé",
                    "signed-by-ballot-box": "Signés par l'urne",
                    "helperText":
                        "Lorsque cette option est activée, l'urne enregistre et signe chaque bulletin à l'étape de vérification, et l'électeur ne voit un identifiant de bulletin qu'une fois le bulletin reçu. Les électeurs signent leurs bulletins. Publiez à nouveau les bulletins après l'avoir modifiée.",
                    "lockedHelperText":
                        "Ce réglage ne peut plus être modifié une fois le vote commencé.",
                },
                VoterCertificatePolicy: {
                    policyLabel: "Voter Digital Certificate Policy",
                    enabled: "Activé",
                    disabled: "Désactivé",
                },
                enrollment: {
                    policyLabel: "Inscription",
                    options: {
                        enabled: "Activé",
                        disabled: "Désactivé",
                    },
                },
                otp: {
                    policyLabel: "OTP",
                    options: {
                        enabled: "Activé",
                        disabled: "Désactivé",
                    },
                },
                ceremoniesPolicy: {
                    policyLabel: "Politique des cérémonies de clés/décompte",
                    options: {
                        "automated-ceremonies": "Autoriser les cérémonies automatiques",
                        "manual-ceremonies": "Cérémonies manuelles",
                    },
                },
                automaticRecountPolicy: {
                    policyLabel: "Recomptage automatique après approbation de l'import",
                    options: {
                        enabled: "Activé",
                        disabled: "Désactivé",
                    },
                },
                weightedVotingPolicy: {
                    policyLabel: "Politique de Vote Pondéré",
                    options: {
                        "areas-weighted-voting": "Vote Pondéré par Zones",
                        "voters-weighted-voting": "Vote Pondéré par Électeurs",
                        "disabled-weighted-voting": "Vote Pondéré Désactivé",
                    },
                    noDelegated:
                        "Le Vote Pondéré par Électeurs ne peut pas être combiné avec le Vote Délégué",
                    noDecodedBallots:
                        "Le Vote Pondéré par Électeurs ne peut pas être combiné avec l'inclusion des bulletins déchiffrés dans les résultats",
                },
                delegatedVotingPolicy: {
                    policyLabel: "Politique de vote délégué",
                    options: {
                        enabled: "Activé",
                        disabled: "Désactivé",
                    },
                },
                languageDetectionPolicy: {
                    policyLabel: "Politique de détection de la langue",
                    options: {
                        "browser-detect": "Détection par le navigateur",
                        "force-default": "Forcer par défaut",
                    },
                },
            },
            error: {
                endDate: "La date de fin doit être postérieure à la date de début",
                noResult: "Pas encore d'Événement Électoral",
                startDate: "La date de début doit être dans le futur",
                endDateInvalid: "La date de fin doit être dans le futur",
            },
            voters: {
                title: "Électeurs",
            },
            createElectionEventSuccess: "Événement Électoral créé",
            createElectionEventError: "Erreur lors de la création de l'Événement Électoral",
            ivr: {
                tabs: {
                    config: "Configuration",
                    blacklist: "Liste de blocage",
                    prompts: "Messages vocaux",
                    emulator: "Émulateur",
                },
                common: {
                    saveSuccess: "Enregistré avec succès",
                    saveError: "Échec de l’enregistrement",
                    deleteSuccess: "Supprimé avec succès",
                    deleteError: "Échec de la suppression",
                },
                config: {
                    configuredPhone: "Numéro de téléphone configuré",
                    infoMsg:
                        "Configurez le flux du SVI et ses propriétés ci-dessous. Pour plus de détails, veuillez contacter Sequent.",
                },
                prompts: {
                    emptyMsg: "Aucun message n’a encore été créé",
                    infoMsg:
                        "Configurez les messages utilisés par le SVI. Les messages d’annonce sont obligatoires, et les messages système peuvent être remplacés pour les langues souhaitées. SSML est pris en charge, y compris pour mélanger les langues.",
                    editorTitle: "Message",
                    editorSubtitle: "Configuration du message",
                },
                blacklist: {
                    columns: {
                        phone: "Numéro de téléphone",
                        reason: "Motif",
                        createdAt: "Créé le",
                        createdBy: "Créé par",
                        createdBefore: "Créé avant",
                        createdAfter: "Créé après",
                    },
                    emptyMsg: "Il n’y a aucune entrée dans la liste de blocage",
                    infoMsg:
                        "Configurez la liste de blocage du SVI. Les appels provenant de ces numéros seront automatiquement déconnectés par le système.",

                    noFilterMatch: "Aucune entrée ne correspond aux filtres indiqués",
                    phoneRequired: "Le numéro de téléphone est obligatoire",
                },
                emulator: {
                    infoMsg:
                        "Sélectionnez une zone et les élections souhaitées pour tester la session IVR.",
                    apiStatus: {
                        unavailable:
                            "Le système d'émulation n'est pas disponible dans votre environnement",
                        loading: "Chargement du système d'émulation",
                        error: "Erreur lors du chargement du système d'émulation",
                    },
                    hints: {
                        title: "Conseils",
                        publishRequired:
                            "Toute modification apportée aux élections, aux scrutins ou aux candidats doit d'abord être publiée pour être disponible. Seuls les styles de bulletin publiés les plus récents correspondant à la zone seront utilisés dans l'émulateur.",
                        eventChangesImmediate:
                            "Les modifications apportées à l'événement électoral, telles que la configuration IVR ou la personnalisation des messages, sont disponibles immédiatement après le redémarrage de la session de l'émulateur.",
                        credentials:
                            'L\'identifiant d\'électeur et le code PIN valides sont "123" et "123".',
                    },
                    sendDtmf: "Envoyer une entrée DTMF",
                    keypadInput: "Saisie au clavier",
                    sendTimeout: "Envoyer l'expiration du délai",
                    disconnected: "Déconnecté",
                    startSession: "Démarrer une nouvelle session",
                    endSession: "Terminer la session",
                    noStylesFound: "Aucun style de bulletin publié ne correspond à vos sélections",
                    inputPlaceholder: "Appuyez sur {{keys}} (délai d'expiration={{timeout}} s)",
                    inputPlaceholderOr: "ou",
                    inputPlaceholderAnyKeys:
                        "Saisissez jusqu'à {{maxDigits}} chiffres (n'importe quels chiffres, délai d'expiration={{timeout}} s)",
                    blacklistCaller: "Bloquer l'appelant",
                    elections: "Élections",
                    area: "Zone",
                },
            },
            stats: {
                elegibleVoters: "Électeurs",
                voters: "Votants",
                elections: "Élections",
                contests: "Questions",
                areas: "Zones",
                sentEmails: "Emails Envoyés",
                sentSMS: "SMS Envoyés",
                calendar: {
                    title: "Calendrier",
                    scheduled: "Programmé",
                },
            },
            keys: {
                createNew: "Créer une Cérémonie de Clés",
                emptyHeader: "Aucune Cérémonie de Clés pour l'instant.",
                statusLabel: "État",
                waitingKeys: "En attente de la Génération de Clés..",
                started: "Commencée à",
                actions: {
                    participate: "Participer à la cérémonie des clés",
                    view: "Voir la cérémonie des clés",
                },
                breadCrumbs: {
                    configure: "Configurer",
                    ceremony: "Cérémonie",
                    created: "Terminé",
                    start: "Début",
                    status: "État",
                    download: "Télécharger",
                    check: "Vérifier",
                    success: "Finalisé",
                },
                notify: {
                    participateNow:
                        "Vous avez été invité à participer à une Cérémonie de Clés. Veuillez <1>cliquer ci-dessous sur l'action de clé de la cérémonie</1> pour participer.",
                },
            },
            tabs: {
                dashboard: "Tableau de Bord",
                monitoring: "Surveillance",
                data: "Données",
                ivr: "IVR",
                localization: "Localisation",
                voters: "Électeurs",
                areas: "Zones",
                keys: "Clés",
                tally: "Comptage",
                tallySheetImports: "Importation des procès-verbaux de dépouillement",
                publish: "Publier",
                logs: "Journaux",
                tasks: "Tâches",
                events: "Événement Planifié",
                notifications: "Notifications",
                reports: "Rapport",
                approvals: "Approvals",
                cas: "Certificats",
            },
            tally: {
                emptyHeader: "Aucun Comptage pour l'instant.",
                title: "Comptage de l'Événement Électoral",
                elections: "Élections",
                electionNumber: "Nombre d'Élections",
                trustees: "Autorités",
                status: "État",
                permissionLabels: "Étiquettes d’Autorisation",
                tallyType: {
                    label: "Type de Décompte",
                    ELECTORAL_RESULTS: "Résultats Électoraux",
                    INITIALIZATION_REPORT: "Résultats de l'Initialisation",
                },
                create: {
                    title: "Créer un Comptage",
                    subtitle: "Créer un nouveau Comptage pour cet Événement Électoral",
                    createTallyButton: "Lancer la Cérémonie de Comptage",
                    createInitializationReportButton: "Créer un Rapport d'Initialisation",
                    error: {
                        create: "Erreur lors de la création du Comptage",
                    },
                    success: "Comptage créé",
                },
                logs: {
                    noLogs: "Aucun journal disponible",
                },
                notify: {
                    noKeysTally:
                        "La Cérémonie de Comptage ne peut pas commencer tant que la Cérémonie de Fideicomisarios n'a pas été réalisée avec succès.",
                    noPublication:
                        "La Cérémonie de Dépouillement ne peut pas commencer tant que vous n'avez pas créé une publication dans l'onglet Publier.",
                    participateNow:
                        "Vous avez été invité à participer à une Cérémonie de Comptage. Veuillez <1>cliquer ci-dessous sur l'action de clé de la cérémonie</1> pour participer.",
                    startDisabled:
                        "Vous ne pouvez pas continuer la cérémonie car aucune élection n'est sélectionnée ou les élections ne sont pas publiées.",
                    ceremonyDisabled:
                        "Vous ne pouvez pas continuer la cérémonie car la session de dépouillement n'est pas connectée ou le début de la cérémonie n'est pas autorisé.",
                },
            },
            importAreas: {
                title: "Importer des Zones",
                subtitle: "Importer des données de zones",
                areaParagraph:
                    "Importer des zones en utilisant un fichier tableur au format Valeurs Séparées par des Virgules (CSV).",
                importSuccess: "Zones Importées avec Succès",
                importError: "Erreur lors de l'importation des Zones",
                upsert: "Upsert Areas",
            },
            import: {
                eetitle: "Importer un Événement Électoral",
                eesubtitle: "Importer des données de l'Événement Électoral",
                title: "Importer des électeurs",
                subtitle: "Importer des électeurs à l'Événement Électoral",
                voters: "Électeurs",
                votersParagraph:
                    "Importez des électeurs en utilisant une feuille de calcul au format Valeurs Séparées par Comma (CSV). Téléchargez un exemple de fichier d'importation CSV ici.",
                electionEventParagraph:
                    "Importez des Événements Électoraux en utilisant un fichier JSON.",
                elections: "Élections",
                areas: "Zones",
                sha: "Vérification de l'Intégrité (SHA 256)",
                cancel: "Annuler",
                import: "Importer",
                fileUploadSuccess: "Fichier chargé sur le serveur - mais pas encore importé",
                fileUploadError: "Erreur lors du chargement du fichier",
                importVotersSuccess:
                    "Importation des Électeurs lancée en arrière-plan avec succès.",
                importVotersError: "Erreur lors de l'Importation des Électeurs.",
                importElectionEventSuccess: "Election event imported Successfully",
                importElectionEventError: "Error importing election event",
                shaDialog: {
                    ok: "Oui, Importer Sans Vérification de l'Intégrité",
                    cancel: "Retour",
                    title: "Importer Sans Vérification de l'Intégrité ?",
                    description:
                        "Vous n'avez pas entré le champ de Vérification de l'Intégrité (SHA-256). Confirmez que vous importez le fichier correct et que vous souhaitez l'importer.",
                },
                passwordDialog: {
                    title: "Mot de passe de déchiffrement",
                    description: "Entrez le mot de passe pour déchiffrer le fichier",
                    label: "Mot de passe",
                    copyPassword: "Copier le Mot de Passe",
                    ok: "OK",
                },
            },
            export: {
                title: "Exporter l'Événement Électoral",
                subtitle:
                    "L'exportation peut être une opération longue. Êtes-vous sûr de vouloir exporter les enregistrements ?",
                encryptWithPassword: "Chiffrer avec Mot de Passe",
                passwordForcedNote:
                    "L'archive sera protégée par mot de passe de toute façon : les rapports, les demandes et les données du tableau d'affichage sont toujours chiffrés. Cochez la case pour inclure aussi les champs secrets des électeurs déchiffrés.",
                includeVoters: "Inclure les Électeurs",
                activityLogs: "Journaux d'Activité",
                bulletinBoard: "Tableau d'Affichage",
                publications: "Publications",
                s3Files: "Fichiers S3",
                scheduledEvents: "Événements Planifiés",
                exportSuccess: "Événement Électoral exporté avec succès",
                exportError: "Erreur lors de l'exportation de l'Événement Électoral",
                passwordTitle: "Mot de Passe",
                passwordDescription: "Mot de passe pour déchiffrer le fichier :",
                copiedSuccess: "Mot de passe copié dans le presse-papiers",
                copiedError: "Erreur lors de la copie",
                reports: "Rapports",
                applications: "Applications",
                tally: "Décompte",
                certificates: "Certificats",
            },
            taskNotification:
                "{{action}} a commencé. Vous pouvez voir son statut dans le tableau d'Exécution des Tâches.",
        },
        electionScreen: {
            common: {
                title: "Élection",
                subtitle: "Configuration de l'élection.",
                fileLoaded: "Fichier chargé",
                noPermission: "Vous n'avez pas la permission d'accéder à cette élection.",
            },
            edit: {
                general: "Général",
                dates: "Dates",
                votingPeriod: "Période de vote",
                language: "Langue",
                allowed: "Canaux de Vote Permis",
                default: "Par défaut",
                defaultLang: "Langue par défaut",
                receipts: "Reçus",
                image: "Image",
                advanced: "Configuration Avancée",
                numAllowedVotes: "Nombre de votes permis",
                reorder: "Réorganiser les concours",
                castVoteConfirm: "Modal de Confirmation de Vote",
                gracePeriodPolicy: "Politique de période de grâce",
                allowTallyPolicy: "Autoriser le décompte",
                permissionLabel: "Étiquette de permission",
                custom_filters: "Filtres personnalisés",
            },
            field: {
                name: "Nom",
                language: "Langue",
                votingChannels: "Canaux de Vote",
                startDateTime: "Date et heure de début",
                endDateTime: "Date et heure de fin",
                startDateTimeWithTimezone: "Date et heure de début ({{timezone}})",
                endDateTimeWithTimezone: "Date et heure de fin ({{timezone}})",
                scheduledOpening: "Ouverture Prévue",
                scheduledClosing: "Fermeture Prévue",
                alias: "Alias",
                description: "Description",
                securityConfirmationHtml: "Confirmation de sécurité HTML",
                ivrPrompt: "Message IVR",
                externalId: "ID externe",
            },
            securityConfirmationPolicy: {
                label: "Politique de la case à cocher de confirmation de sécurité",
                none: "Aucun",
                mandatory: "Obligatoire",
            },
            error: {
                endDate: "La date de fin doit être postérieure à la date de début",
                fileError: "Erreur lors du chargement du fichier",
                fileLoaded: "Fichier chargé",
                startDate: "La date de début doit être dans le futur",
                endDateInvalid: "La date de fin doit être dans le futur",
            },
            createElectionEventSuccess: "Élection créée",
            createElectionEventError: "Erreur lors de la création de l'élection",
            tabs: {
                dashboard: "Tableau de Bord",
                monitoring: "Surveillance",
                data: "Données",
                voters: "Électeurs",
                publish: "Publier",
                logs: "Journaux",
                approvals: "Approvals",
                tallySheets: "Feuilles de Comptage",
            },
            gracePeriodPolicy: {
                "label": "Politique de période de grâce",
                "no-grace-period": "Pas de période de grâce",
                "grace-period-without-alert": "Période de grâce sans alerte",
                "gracePeriodSecs": "Période de grâce en secondes",
            },
            allowTallyPolicy: {
                "allowed": "Autorisé",
                "disallowed": "Non autorisé",
                "requires-voting-period-end": "Nécessite la fin de la période de vote",
            },
            initializeReportPolicy: {
                "label": "Initialiser la Politique de Rapport",
                "not-required": "Non Requis",
                "required": "Requis",
            },
            castVoteGoldLevelPolicy: {
                label: "Gold level Authentication Policy",
                options: {
                    "gold-level": "Gold level Authentication",
                    "no-gold-level": "No Gold level Authentication",
                },
            },
            slates: {
                title: "Listes",
                configuration: "Configuration des listes (JSON)",
                helper: "Listes nommées et les candidats que chacune présente dans chaque scrutin. Laissez vide pour une élection sans listes.",
                loading:
                    "Les scrutins et les candidats de l'élection sont encore en cours de chargement. Réessayez dans un instant.",
                mobileCandidateLists: {
                    label: "Listes de candidats sur mobile",
                    helper: "État initial de la liste de candidats de chaque liste sur mobile. L'électeur peut toujours l'ouvrir ou la fermer.",
                    options: {
                        collapsed: "Repliées",
                        expanded: "Dépliées",
                    },
                },
            },
            startScreenTitlePolicy: {
                label: "Politique de titre de l'écran d'accueil",
                options: {
                    "election": "Titre de l'élection",
                    "election-event": "Titre de l'événement électoral",
                },
            },
            consolidatedReportPolicy: {
                label: "Politique de rapport consolidé",
                options: {
                    "generate": "Générer",
                    "do-not-generate": "Ne pas générer",
                },
            },
            declineToVotePolicy: {
                label: "Politique d’abstention de vote",
                options: {
                    enabled: "Activé",
                    disabled: "Désactivé",
                },
            },
            blankBallotsPolicy: {
                label: "Politique des bulletins blancs",
                options: {
                    enabled: "Activé",
                    disabled: "Désactivé",
                },
            },
            votingScreenBackPolicy: {
                label: "Politique du bouton Retour de l’écran de vote",
                options: {
                    "election-selection-screen": "Aller à l’écran de sélection des élections",
                    "start-screen": "Aller à l’écran d’accueil de l’élection",
                },
            },
        },
        tenantScreen: {
            common: {
                title: "Locataires",
            },
            new: {
                subtitle: "Créer un nouveau locataire",
            },
            createSuccess: "Locataire créé",
            createError: "Erreur lors de la création du locataire",
        },
        usersAndRolesScreen: {
            noPermissions: "Vous n'avez pas la permission d'accéder aux utilisateurs ou rôles.",
            common: {
                title: "Utilisateurs et Rôles",
                subtitle: "Configuration générale",
                mobileNumber: "Mobile",
            },
            editPassword: {
                passwordPolicyViolation:
                    "Le mot de passe ne respecte pas la Politique de mot de passe de cet événement électoral. Consultez la politique dans Données de l'événement électoral et saisissez un mot de passe conforme.",
                passwordPolicyRules: {
                    minimumLength: "La longueur minimale du mot de passe est de {{count}}.",
                    maximumLength: "La longueur maximale du mot de passe est de {{count}}.",
                    uppercase: "Caractères majuscules requis : {{count}}.",
                    lowercase: "Caractères minuscules requis : {{count}}.",
                    digits: "Chiffres requis : {{count}}.",
                    specialCharacters: "Caractères spéciaux requis : {{count}}.",
                },
                label: "Changer le mot de passe",
                temporatyLabel: "Temporaire",
                temporatyInfo:
                    "Si activé, l'utilisateur devra changer le mot de passe lors de la prochaine connexion.",
            },
            users: {
                title: "Utilisateurs",
                subtitle: "Voir et éditer les données de l'utilisateur",
                review: {
                    title: "Vérifier les modifications",
                    subtitle: "Confirmez ces mises à jour avant de les soumettre.",
                    confirm: "Confirmer les modifications",
                    noChanges: "Aucune modification à vérifier",
                    field: "Champ",
                    currentValue: "Valeur actuelle",
                    newValue: "Nouvelle valeur",
                },
                edit: {
                    title: "Informations de l'Utilisateur",
                    subtitle: "Voir et éditer l'Utilisateur",
                },
                create: {
                    title: "Utilisateur",
                    subtitle: "Créer utilisateur",
                },
                fields: {
                    "has_voted": "A voté",
                    "support_materials_viewed": "Support Materials Viewed",
                    "vote-weight": "Poids du vote",
                    "voted-channel": "Canal de vote",
                    "disable-comment": "Commentaire de désactivation",
                    "username": "Nom d'Utilisateur",
                    "first_name": "Prénom",
                    "last_name": "Nom",
                    "email": "Email",
                    "enabled": "Activé",
                    "emailVerified": "Email Vérifié",
                    "groups": "Groupes",
                    "attributes": "Attributs",
                    "area": "Zone",
                    "password": "Mot de passe",
                    "savePassword": "Sauvegarder le mot de passe",
                    "repeatPassword": "Répéter le Mot de Passe",
                    "passwordMismatch": "Les mots de passe doivent correspondre",
                    "passwordLengthValidate": "Le mot de passe doit avoir au moins 8 caractères",
                    "passwordUppercaseValidate":
                        "Le mot de passe doit contenir au moins une lettre majuscule",
                    "passwordLowercaseValidate":
                        "Le mot de passe doit contenir au moins une lettre minuscule",
                    "passwordDigitValidate": "Le mot de passe doit contenir au moins un chiffre",
                    "passwordSpecialCharValidate":
                        "Le mot de passe doit contenir au moins un caractère spécial",
                    "trustee": "Agir en tant que fiduciaire",
                    "permissionLabel": "Libellé d'autorisation",
                    "authorized-election-ids": "Élections",
                },
                delete: {
                    body: "Êtes-vous sûr de vouloir supprimer cet utilisateur ?",
                    bulkBody: "Êtes-vous sûr de vouloir supprimer les utilisateurs sélectionnés ?",
                    bulkBodySelected: "Delete the {{count}} selected users? This cannot be undone.",
                    bulkBodyChoose:
                        "{{count}} users are selected. You can instead delete every user matching the current filters, which may be more. This cannot be undone.",
                    okSelected: "Delete {{count}} selected",
                    okAllMatching: "Delete all matching",
                },
                notifications: {
                    exportError: "Erreur lors de l'exportation des utilisateurs",
                    deleteError: "Erreur lors de la suppression de l'utilisateur",
                    deleteSuccess: "Utilisateur supprimé",
                    multipleDeleteSuccess: "Utilisateurs supprimés",
                },
            },
            voters: {
                voterInformationLetter: {
                    label: "Lettre d'information de l'électeur",
                    generate: "Générer",
                    confirmation:
                        "Générer une Lettre d'information pour cet électeur ? Un nouveau mot de passe sera attribué et inclus dans un PDF chiffré.",
                    generationStarted: "La génération de la Lettre d'information a commencé",
                    generationError: "La Lettre d'information n'a pas pu être générée",
                    policyNotConfigured:
                        "La Politique de mot de passe n'est pas configurée. Configurez-la dans Données de l'événement électoral avant de générer une lettre.",
                    policyMinimumLengthMissing:
                        "La Politique de mot de passe doit inclure une longueur minimale avant de générer une lettre.",
                    policyCharacterClassMissing:
                        "La Politique de mot de passe doit inclure au moins une classe de caractères avant de générer une lettre.",
                },
                title: "Électeurs",
                subtitle: "Voir et éditer les données de l'électeur",
                secretAttribute: {
                    storedPlaceholder: "Valeur chiffrée enregistrée",
                    reveal: "Afficher",
                    hide: "Masquer",
                    revealError: "Le champ chiffré de l'électeur n'a pas pu être affiché",
                    includeInExport: "Inclure les champs secrets déchiffrés de l'électeur",
                    exportWarning:
                        "Exportation sensible : le CSV téléchargé contiendra ces champs en texte clair.",
                    clear: "Effacer",
                    add: "Ajouter une valeur",
                    remove: "Supprimer la valeur",
                },
                review: {
                    title: "Vérifier les modifications",
                    subtitle: "Confirmez ces mises à jour avant de les soumettre.",
                    confirm: "Confirmer les modifications",
                    noChanges: "Aucune modification à vérifier",
                    field: "Champ",
                    currentValue: "Valeur actuelle",
                    newValue: "Nouvelle valeur",
                },
                logs: {
                    label: "Journaux de l'utilisateur",
                },
                emptyHeader: "Aucun électeur pour l'instant.",
                askCreate: "Voulez-vous en créer un ?",
                create: {
                    title: "Électeur",
                    subtitle: "Créer électeur",
                },
                manualVerification: {
                    label: "Vérifier manuellement",
                    verify: "Vérifier manuellement l'électeur",
                    body: "Vérifiez manuellement cet électeur. Vous obtiendrez un PDF avec un lien de code QR qui permettra à l'électeur de se connecter en omettant le KYC en ligne.",
                    noEmailOrPhone:
                        "Cet électeur ne peut pas être vérifié manuellement car il n'a pas d'adresse e-mail ou de numéro de téléphone attribué",
                },
                errors: {
                    editError: "Erreur lors de l'édition de l'électeur",
                    editErrorReason: "Erreur lors de l'édition de l'électeur : {{reason}}",
                    editSuccess: "Électeur édité",
                    createError: "Erreur lors de la création de l'électeur",
                    createErrorReason: "Erreur lors de la création de l'électeur : {{reason}}",
                    createSuccess: "Électeur créé",
                    attribute: {
                        invalidNamed: '"{{field}}" a été refusé : {{constraint}}',
                        fieldsToCorrect:
                            "Certains champs doivent être corrigés avant d'enregistrer",
                        hintBetween: "Entre {{min}} et {{max}} caractères",
                        hintMin: "Au moins {{min}} caractères",
                        hintMax: "Au maximum {{max}} caractères",
                        andMore: "et {{count}} de plus",
                        invalidLength:
                            '"{{field}}" doit contenir entre {{min}} et {{max}} caractères',
                        tooShort: '"{{field}}" doit contenir au moins {{min}} caractères',
                        tooLong: '"{{field}}" doit contenir au maximum {{max}} caractères',
                        required: '"{{field}}" est obligatoire',
                        invalidEmail: '"{{field}}" doit être une adresse e-mail valide',
                        invalidFormat: '"{{field}}" n\'a pas le format attendu',
                        invalid: '"{{field}}" a une valeur non valide',
                    },
                    createPasswordError:
                        "Électeur créé, mais son mot de passe n'a pas pu être défini",
                    createPasswordErrorReason:
                        "Électeur créé, mais son mot de passe n'a pas pu être défini : {{reason}}",
                },
                delete: {
                    body: "Êtes-vous sûr de vouloir supprimer cet électeur ?",
                    bulkBody: "Êtes-vous sûr de vouloir supprimer les électeurs sélectionnés ?",
                    bulkBodySelected:
                        "Delete the {{count}} selected voters? This cannot be undone.",
                    bulkBodyChoose:
                        "{{count}} voters are selected. You can instead delete every voter matching the current filters, which may be more. This cannot be undone.",
                    okSelected: "Delete {{count}} selected",
                    okAllMatching: "Delete all matching",
                },
                notifications: {
                    exportError: "Erreur lors de l'exportation des électeurs",
                    deleteError: "Erreur lors de la suppression de l'électeur",
                    deleteSuccess: "Électeur supprimé",
                    multipleDeleteSuccess: "Électeurs supprimés",
                    manualVerificationError:
                        "Erreur lors de la vérification manuelle de l'électeur",
                    manualVerificationSuccess:
                        "Électeur vérifié manuellement avec succès, télécharger PDF..",
                },
            },
            roles: {
                title: "Rôles",
                edit: {
                    title: "Informations sur le Rôle",
                    subtitle: "Voir et éditer le Rôle",
                },
                create: {
                    title: "Rôle",
                    subtitle: "Créer rôle",
                },
                errors: {
                    createError: "Erreur lors de la création du rôle",
                    createSuccess: "Rôle créé",
                },
                fields: {
                    name: "Nom",
                },
                delete: {
                    body: "Êtes-vous sûr de vouloir supprimer ce rôle ?",
                },
                notifications: {
                    deleteError: "Erreur lors de la suppression du rôle",
                    deleteSuccess: "Rôle supprimé",
                    permissionEditError: "Erreur lors de l'édition de la permission",
                    permissionEditSuccess: "Permission éditée",
                },
            },
            permissions: {
                "voter-information-letter": "Générer une Lettre d'information de l'électeur",
                "admin-user": "Administration",
                "admin-dashboard-view": "Vue du Tableau de Bord d'Administration",
                "monitoring-view": "Voir les Tableaux de Bord de Suivi",
                "monitoring-configure": "Configurer les Tableaux de Bord de Suivi",
                "election-event-signatures-tab": "Onglet Signatures de l'Événement Électoral",
                "signing-rules-read": "Signatures : voir les actions protégées",
                "signing-rules-write": "Signatures : modifier les actions protégées",
                "signing-certificates-read": "Signatures : voir les certificats",
                "signing-issuers-write":
                    "Signatures : importer et supprimer des émetteurs de confiance",
                "signing-checks-write": "Signatures : modifier les vérifications des certificats",
                "signing-certificates-register": "Signatures : enregistrer des certificats",
                "signing-certificates-revoke": "Signatures : révoquer des certificats",
                "signing-requests-read": "Signatures : voir les demandes",
                "signing-requests-cancel": "Signatures : annuler des demandes",
                "signing-requests-export": "Signatures : exporter les demandes",
                "sign-initialize-voting": "Signer : initialiser le vote",
                "sign-open-voting": "Signer : ouvrir le vote",
                "sign-close-voting": "Signer : clôturer le vote",
                "sign-generate-election-returns": "Signer : générer les procès-verbaux électoraux",
                "sign-generate-reports": "Signer : générer d'autres rapports électoraux",
                "sign-transmit-results": "Signer : transmettre les résultats",
                "sign-approve-voter": "Signer : approuver manuellement un électeur",
                "sign-approve-configuration": "Signer : approuver une version de configuration",
                "sign-key-ceremony": "Signer : confirmer un fragment de clé",
                "sign-tally-key": "Signer : apporter un fragment de clé",
                "application-export": "Exportation d'Applications",
                "application-import": "Importation d'Applications",
                "tenant-create": "Créer Locataire",
                "tenant-read": "Lire Locataire",
                "tenant-write": "Éditer Locataire",
                "tenant-delete": "Supprimer un locataire",
                "election-event-create": "Créer Événement Électoral",
                "election-event-read": "Lire Événement Électoral",
                "election-event-write": "Éditer Événement Électoral",
                "keycloak-realm-attributes-read": "Read Keycloak realm attributes",
                "keycloak-realm-attributes-write": "Edit Keycloak realm attributes",
                "election-event-delete": "Supprimer Événement Électoral",
                "voter-create": "Créer Électeur",
                "voter-read": "Lire Électeur",
                "voter-write": "Éditer Électeur",
                "voter-secret-attribute-read": "Afficher les Champs Secrets de l'Électeur",
                "voter-secret-attribute-write": "Éditer les Champs Secrets de l'Électeur",
                "user-create": "Créer Utilisateur",
                "user-read": "Lire Utilisateur",
                "user-write": "Éditer Utilisateur",
                "user-permission-create": "Créer Permission d'Utilisateur",
                "user-permission-read": "Lire Permission d'Utilisateur",
                "user-permission-write": "Éditer Permission d'Utilisateur",
                "role-create": "Créer Rôle",
                "role-read": "Lire Rôle",
                "role-write": "Éditer Rôle",
                "role-assign": "Assigner Rôle",
                "communication-template-create": "Créer Modèle de Communication",
                "communication-template-read": "Lire Modèle de Communication",
                "communication-template-write": "Éditer Modèle de Communication",
                "notification-read": "Lire Notification",
                "notification-write": "Éditer Notification",
                "notification-send": "Envoyer Notification",
                "area-read": "Lire Zone",
                "area-write": "Éditer Zone",
                "election-state-write": "Éditer État de l'Élection",
                "election-type-create": "Créer Type d'Élection",
                "election-type-read": "Lire Type d'Élection",
                "election-type-write": "Éditer Type d'Élection",
                "voting-channel-read": "Lire Canal de Vote",
                "voting-channel-write": "Éditer Canal de Vote",
                "trustee-create": "Créer Fideicomisario",
                "trustee-read": "Lire Fideicomisario",
                "trustee-write": "Éditer Fideicomisario",
                "tally-read": "Lire Comptage",
                "tally-start": "Commencer Comptage",
                "tally-write": "Éditer Comptage",
                "tally-results-read": "Lire Résultats de Comptage",
                "publish-read": "Lire Publication",
                "publish-write": "Éditer Publication",
                "publish-results-read": "Lire Publication des Résultats",
                "publish-results-write": "Éditer Publication des Résultats",
                "logs-read": "Lire Journaux",
                "tasks-read": "Lire l'Exécution des Tâches",
                "keys-read": "Lire Clés",
                "document-upload": "Télécharger Documents",
                "document-download": "Télécharger Documents",
                "document-password-read": "Lire les mots de passe des documents",
                "tally-sheet-create": "Créer Acte de Comptage",
                "tally-sheet-import-create": "Créer une importation d'actes de comptage",
                "tally-sheet-import-review": "Examiner une importation d'actes de comptage",
                "tally-sheet-import-view": "Voir une importation d'actes de comptage",
                "tally-recount-execute": "Exécuter un recomptage des résultats",
                "trustee-ceremony": "Cérémonie de Fideicomisario",
                "tally-sheet-review": "Examiner la feuille de décompte",
                "tally-sheet-view": "Voir Acte de Comptage",
                "admin-ceremony": "Administrer Cérémonie de Clés",
                "tally-sheet-delete": "Supprimer Acte de Comptage",
                "cast-vote-read": "Lire Votes Émis",
                "document-read": "Lire Documents",
                "document-write": "Éditer Documents",
                "support-material-read": "Lire Matériaux de Support",
                "support-material-write": "Éditer Matériaux de Support",
                "miru-create": "Miru Create",
                "miru-download": "Miru Download",
                "miru-send": "Miru Send",
                "miru-sign": "Miru Sign",
                "contest-write": "Éditer Concours",
                "contest-read": "Lire le Concours",
                "candidate-write": "Éditer le candidats",
                "candidate-read": "Lire le candidats",
                "permission-label-write": "Modifier l'étiquette de permission",
                "scheduled-event-write": "Modifier des Événements Planifiés",
                "contest-create": "Create Contest",
                "contest-delete": "Delete Contest",
                "candidate-create": "Create Candidate",
                "candidate-delete": "Delete Candidate",
                "election-create": "Create Election",
                "election-read": "Read Election",
                "election-write": "Edit Election",
                "election-delete": "Delete Election",
                "election-event-archive": "Archive Election Event",
                "election-data-tab": "Voir les Données de l'Élection",
                "election-event-areas-tab": "Voir les Zones de l'Événement Électoral",
                "election-event-data-tab": "Voir les Données de l'Événement Électoral",
                "election-event-keys-tab": "Voir les Clés de l'Événement Électoral",
                "election-event-logs-tab": "Voir les Journaux de l'Événement Électoral",
                "election-event-publish-tab": "Voir la Publication de l'Événement Électoral",
                "election-event-reports-tab": "Voir les Rapports de l'Événement Électoral",
                "election-event-scheduled-tab": "Voir le Programme de l'Événement Électoral",
                "election-event-tally-tab": "Voir le Décompte de l'Événement Électoral",
                "election-event-tasks-tab": "Voir les Tâches de l'Événement Électoral",
                "election-event-voters-tab": "Voir les Électeurs de l'Événement Électoral",
                "election-publish-tab": "Voir la Publication de l'Élection",
                "election-voters-tab": "Voir les Électeurs de l'Élection",
                "report-write": "Modifier les Rapports",
                "report-read": "Lire les Rapports",
                "users-menu": "Voir les utilisateurs et les rôles",
                "settings-menu": "Voir les paramètres",
                "templates-menu": "Voir les modèles",
                "settings-election-types-tab": "Voir les paramètres des types d'élection",
                "settings-voting-channels-tab": "Voir les paramètres des canaux de vote",
                "settings-templates-tab": "Voir les paramètres des modèles",
                "settings-languages-tab": "Voir les paramètres des langues",
                "settings-localization-tab": "Voir les paramètres de localisation",
                "settings-look-feel-tab": "Voir les paramètres d'apparence",
                "settings-trustees-tab": "Voir les paramètres des fiduciaires",
                "settings-countries-tab": "Voir les paramètres des pays",
                "voter-import": "Importer un Électeur",
                "ee-voters-columns": "Voir les Colonnes des Électeurs de l'Événement Électoral",
                "voter-manually-verify": "Vérifier un Électeur Manuellement",
                "ee-voters-logs": "Voir les Journaux des Électeurs de l'Événement Électoral",
                "voter-export": "Exporter un Électeur",
                "ee-voters-filters": "Voir les Filtres des Électeurs de l'Événement Électoral",
                "voter-delete": "Supprimer un Électeur",
                "voter-change-password": "Changer le Mot de Passe de l'Électeur",
                "election-event-localization-selector":
                    "Sélecteur de Localisation de l'Événement Électoral",
                "localization-create": "Créer une Localisation",
                "localization-read": "Lire une Localisation",
                "localization-write": "Modifier une Localisation",
                "localization-delete": "Supprimer une Localisation",
                "area-create": "Créer une Zone",
                "area-delete": "Supprimer une Zone",
                "area-export": "Exporter une Zone",
                "area-import": "Importer une Zone",
                "area-upsert": "Insérer ou Mettre à Jour une Zone",
                "election-event-areas-columns": "Colonnes des Zones de l'Événement Électoral",
                "election-event-areas-filters": "Filtres des Zones de l'Événement Électoral",
                "election-event-tasks-back-button": "Retour aux Tâches de l'Événement Électoral",
                "election-event-tasks-columns": "Colonnes des Tâches de l'Événement Électoral",
                "election-event-tasks-filters": "Filtres des Tâches de l'Événement Électoral",
                "task-export": "Exporter les Tâches",
                "application-read": "Lire l'Application",
                "application-write": "Modifier l'Application",
                "approval-matrix-write": "Modifier la Matrice d'Approbation",
                "logs-export": "Exporter les Journaux",
                "election-event-logs-columns": "Colonnes des Journaux de l'Événement Électoral",
                "election-events-logs-filters": "Filtres des Journaux de l'Événement Électoral",
                "election-event-scheduled-event-columns":
                    "Colonnes des Événements Programmés de l'Événement Électoral",
                "scheduled-event-create": "Créer un Événement Programmé",
                "scheduled-event-delete": "Supprimer un Événement Programmé",
                "election-event-reports-columns": "Colonnes des Rapports de l'Événement Électoral",
                "report-create": "Créer un Rapport",
                "report-delete": "Supprimer un Rapport",
                "report-generate": "Générer un Rapport",
                "report-preview": "Aperçu du Rapport",
                "monitor-authenticated-voters": "Surveillance des Électeurs Authentifiés",
                "monitor-all-approve-disapprove-voters":
                    "Lire la Surveillance des Électeurs Approuvés et Refusés",
                "monitor-automatic-approve-disapprove-voters":
                    "Lire la Surveillance des Approbations et Refus Automatiques",
                "monitor-manually-approve-disapprove-voters":
                    "Lire la Surveillance des Approbations et Refus Manuels",
                "monitor-enrolled-overseas-voters":
                    "Lire la Surveillance des Électeurs Inscrits à l'Étranger",
                "monitor-posts-already-closed-voting":
                    "Lire la Surveillance des Publications avec Vote Clos",
                "monitor-posts-already-generated-election-results":
                    "Lire la Surveillance des Publications avec Résultats Générés",
                "monitor-posts-already-opened-voting":
                    "Lire la Surveillance des Publications avec Vote Ouvert",
                "monitor-posts-already-started-counting-votes":
                    "Lire la Surveillance des Publications qui ont Commencé le Décompte des Votes",
                "monitor-posts-initialized-the-system":
                    "Lire la Surveillance des Publications qui ont Initialisé le Système",
                "monitor-posts-started-voting":
                    "Lire la Surveillance des Publications qui ont Commencé à Voter",
                "monitor-posts-transmitted-results":
                    "Lire la Surveillance des Publications qui ont Transmis des Résultats",
                "monitor-voters-voted-test-election":
                    "Lire la Surveillance des Électeurs lors de l'Élection de Test",
                "monitor-voters-who-voted": "Lire la Surveillance des Électeurs qui ont Voté",
                "election-event-publish-preview":
                    "Aperçu de la Publication de l'Événement Électoral",
                "election-event-publish-back-button":
                    "Retour à la Publication de l'Événement Électoral",
                "election-event-publish-columns":
                    "Colonnes de la Publication de l'Événement Électoral",
                "election-event-publish-filters":
                    "Filtres de la Publication de l'Événement Électoral",
                "publish-create": "Créer une Publication",
                "publish-regenerate": "Régénérer une Publication",
                "publish-export": "Exporter une Publication",
                "publish-start-voting": "Commencer le Vote",
                "publish-pause-voting": "Mettre en Pause le Vote",
                "publish-stop-voting": "Arrêter le Vote",
                "publish-changes": "Publier les Modifications",
                "election-event-publish-view": "Voir la Publication de l'Événement Électoral",
                "election-event-keys-columns": "Colonnes des Clés de l'Événement Électoral",
                "create-ceremony": "Créer une Cérémonie",
                "export-ceremony": "Exporter une Cérémonie",
                "election-event-tally-columns": "Colonnes du Décompte de l'Événement Électoral",
                "election-event-tally-back-button": "Retour au Décompte de l'Événement Électoral",
                "transmition-ceremony": "Cérémonie de Transmission",
                "admin-ip-address-view": "Voir l'adresse IP",
                "election-approvals-tab": "Voir les Approbations de l'Élection",
                "election-event-approvals-tab": "Voir les Approbations de l'Événement Électoral",
                "election-ip-address-view": "Voir l'adresse IP de l'Élection",
                "election-dashboard-tab": "Voir le Tableau de Bord de l'Élection",
                "trustees-export": "Exporter les Fiduciaires",
                "user-import": "Importer des Utilisateurs",
                "voter-voted-edit": "Modifier les électeurs qui ont voté",
                "voter-email-tlf-edit": "Modifier les champs e-mail/téléphone des électeurs",
                "cloudflare-write": "Modifier les règles de blocage par pays dans Cloudflare",
                "transmission-report-generate": "Générer un rapport de transmission",
                "google-meet-link": "Générer un Lien Google Meet",
                "service-account": "Compte de service",
                "datafix-account": "Compte de correction des données",
                "gold": "Or",
                "silver": "Argent",
                "election-event-ivr-tab": "Afficher l’IVR de l’événement électoral",
                "election-event-cas-tab": "Afficher le CAS de l’événement électoral",
                "ca-read": "Consulter les autorités de certification",
                "ca-write": "Modifier les autorités de certification",
                "generate-preview": "Générer l’aperçu",
                "preview-read": "Consulter l’aperçu",
                "tally-resolution-submit": "Soumettre la résolution du dépouillement",
                "phone-blacklist-read": "Consulter la liste noire téléphonique",
                "phone-blacklist-create": "Créer des entrées dans la liste noire téléphonique",
                "phone-blacklist-update": "Modifier des entrées de la liste noire téléphonique",
                "phone-blacklist-delete": "Supprimer des entrées de la liste noire téléphonique",
                "election-event-voter-list-reconciliation":
                    "Rapprocher la liste électorale de l’événement",
                "messaging-account-read": "Voir les comptes de messagerie",
                "messaging-account-write": "Gérer les comptes de messagerie",
                "messaging-config-write": "Configurer la messagerie de l'événement électoral",
            },
        },
        generalSettingsScreen: {
            body: "Activez les langues dans le système. Seules les langues activées ici seront disponibles pour les événements électoraux.",
        },
        eventsScreen: {
            title: "Événements Planifiés",
            subtitle:
                "Gère la configuration de l'exécution automatique des événements tels que le début ou la fin de la période de vote.",
            messages: {
                createSuccess: "Événement Planifié créé avec succès",
                createError: "Erreur lors de la création de l'Événement Planifié",
                editSuccess: "Événement Planifié modifié avec succès",
                editError: "Erreur lors de la modification de l'Événement Planifié",
                onlineWithEarlyVoting:
                    "Une planification de début ne peut pas ouvrir à la fois le vote en ligne et le vote anticipé : le vote anticipé doit commencer avant le vote en ligne.",
            },
            eventType: {
                label: "Type",
                ALLOW_INIT_REPORT: "Allow Initialization Report",
                START_VOTING_PERIOD: "Début de la Période de Vote",
                END_VOTING_PERIOD: "Fin de la Période de Vote",
                ALLOW_VOTING_PERIOD_END: "Allow Voting Period End",
                START_ENROLLMENT_PERIOD: "Début de la période d'inscription",
                END_ENROLLMENT_PERIOD: "Fin de la période d'inscription",
                START_LOCKDOWN_PERIOD: "Début de la période de blocage des données du recensement",
                END_LOCKDOWN_PERIOD: "Fin de la période de blocage des données du recensement",
                ALLOW_TALLY: "Autoriser le décompte",
                START_READINESS_TEST: "Démarrer le test de préparation électorale",
                END_READINESS_TEST: "Terminer le test de préparation électorale",
                START_FINAL_TESTING: "Démarrer les tests finaux et le verrouillage",
                END_FINAL_TESTING: "Terminer les tests finaux et le verrouillage",
                START_TEST_VOTING: "Démarrer le vote de test",
                END_TEST_VOTING: "Terminer le vote de test",
            },
            warning: {
                votingWindowDays:
                    "La période de vote de {{election}} couvre {{days}} jours locaux (du {{start_local}} au {{end_local}}, {{time_zone}}) ; la règle en demande {{expected}}.",
                finalTestingLeadTime:
                    "Les tests finaux de {{election}} commencent le {{final_testing_local}}, moins de {{minimum_days}} jours avant l'ouverture du vote le {{voting_start_local}} ({{time_zone}}).",
                closeBeforeOpen:
                    "Le vote de {{election}} se clôture au moment de son ouverture ou avant ({{start_local}} à {{end_local}}, {{time_zone}}).",
                shortLastDay:
                    "Le dernier jour de vote de {{election}} compte {{hours}} heures, moins de {{minimum_hours}} : le vote se clôture le {{end_local}} ({{time_zone}}).",
            },
            election: {
                label: "Élection",
            },
            empty: {
                header: "Pas encore d'Événements Planifiés.",
                body: "Voulez-vous en créer un ?",
                button: "Créer un Événement Planifié",
            },
            create: {
                title: "Créer un Événement Planifié",
                subtitle: "Créer une nouvelle configuration d'Événement Planifié.",
            },
            edit: {
                title: "Modifier l'Événement Planifié",
                subtitle: "Modifier la configuration de l'Événement Planifié.",
                delete: "Êtes-vous sûr de vouloir supprimer cet Événement Planifié ?",
            },
            fields: {
                electionId: "Élection",
                eventProcessor: "Type",
                stoppedAt: "Arrêté Le",
                scheduledDate: "Planifié Le",
            },
        },
        reportsScreen: {
            title: "Rapports",
            subtitle: "Générer des rapports pour les événements électoraux",
            messages: {
                createSuccess: "Rapport créé avec succès",
                createError: "Erreur lors de la création du rapport",
                submitError: "Erreur lors de la soumission du Rapport",
                updateSuccess: "Rapport mis à jour avec succès",
                passwordMismatch:
                    "Le mot de passe et la confirmation ne correspondent pas. Veuillez vous assurer que les deux champs contiennent le même mot de passe.",
                incorectPassword: "Le mot de passe est incorrect",
                decryptFileTitle: "Déchiffrer le fichier",
                decryptInstructions:
                    "1. '-in' : Le chemin vers le fichier chiffré. \n2. '-out' : Le chemin où le fichier déchiffré sera enregistré. \n3. '-pass' : Le mot de passe utilisé pour chiffrer le fichier. \n",
                encryptSuccess: "Configuration du chiffrement du rapport réussie",
                encryptError: "Erreur lors de la configuration du chiffrement du rapport",
            },
            reportType: {
                BALLOT_RECEIPT: "Reçu de Bulletin",
                ELECTORAL_RESULTS: "Résultats Électoraux",
                MANUAL_VERIFICATION: "Vérification Manuelle",
                PARTICIPATION_REPORT: "Rapport de Participation",
                STATISTICAL_REPORT: "Rapport Statistique",
                OVCS_EVENTS: "Suivi du Vote à l'Étranger - Événements OVCS",
                AUDIT_LOGS: "Journaux d'Audit",
                ACTIVITY_LOG: "Journaux d'Activité",
                STATUS: "Statut",
                OVCS_INFORMATION: "Informations OVCS",
                OVERSEAS_VOTERS: "Liste des Électeurs Résidant à l'Étranger",
                OV_USERS_WHO_VOTED: "Liste des Électeurs Résidant à l'Étranger ayant Voté",
                OV_WITH_VOTING_STATUS:
                    "Liste des Électeurs Résidant à l'Étranger avec Statut de Vote",
                OVCS_STATISTICS: "Suivi du Vote à l'Étranger - Statistiques OVCS",
                PRE_ENROLLED_OV_BUT_DISAPPROVED:
                    "Liste des Électeurs Résidant à l'Étranger Pré-inscrits mais Refusés",
                PRE_ENROLLED_OV_SUBJECT_TO_MANUAL_VALIDATION:
                    "Liste des Électeurs Résidant à l'Étranger Pré-inscrits mais Soumis à Validation Manuelle",
            },
            reportEncryptionPolicy: {
                title: "Politique de chiffrement",
                UNENCRYPTED: "Non chiffré",
                CONFIGURED_PASSWORD: "Mot de passe configuré",
            },
            empty: {
                header: "Pas encore de rapports.",
                body: "Voulez-vous en créer un?",
                button: "Créer un rapport",
            },
            create: {
                title: "Créer un rapport",
                subtitle: "Créer une nouvelle configuration de rapport.",
            },
            edit: {
                title: "Modifier le rapport",
                subtitle: "Modifier la configuration du rapport.",
                delete: "Êtes-vous sûr de vouloir supprimer ce rapport?",
            },
            fields: {
                electionId: "Élection",
                template: "Modèle",
                reportType: "Type de rapport",
                repeatable: "Répétable",
                cronExpression: "Expression Cron",
                emailRecipients: "Destinataires de courriel",
                emailRecipientsPlaceholder: "Tapez l'email et appuyez sur Entrée",
            },
            delete: {
                body: "Êtes-vous sûr de vouloir supprimer ce rapport?",
            },
            actions: {
                generate: "Générer",
                delete: "Supprimer",
                edit: "Modifier",
                preview: "Aperçu",
            },
        },
        googleMeet: {
            title: "Générer un Lien Google Meet",
            generateButton: "Google Meet",
            meetingTitle: "Titre de la Réunion",
            description: "Description (Optionnel)",
            startDate: "Date de Début",
            startTime: "Heure de Début",
            duration: "Durée (minutes)",
            attendeeEmails: "Emails des Participants",
            attendeeEmailHelp:
                "Emails séparés par des virgules pour les participants de la réunion",
            note: "Note : Cela créera un événement de calendrier dans votre Google Calendar avec un lien Google Meet. Vous devrez vous connecter à votre compte Google.",
            success: "Lien Google Meet Généré avec Succès !",
            copy: "Copier dans le presse-papiers",
            copied: "Lien copié dans le presse-papiers !",
            instructions:
                "Partagez ce lien avec les participants pour rejoindre la réunion. L'événement de calendrier a été ajouté à votre Google Calendar.",
            generating: "Génération...",
            generate: "Générer le Lien Meet",
        },
        common: {
            export: "L'exportation peut être un processus long. Êtes-vous sûr de vouloir exporter ?",
            resources: {
                electionEvent: "Événement Électoral",
                election: "Élection",
                contest: "Concours",
                candidate: "Candidat",
                noResult: {
                    askCreate: "Voulez-vous en créer un ?",
                },
            },
            label: {
                add: "Ajouter",
                actions: "Actions",
                create: "Créer",
                delete: "Supprimer",
                archive: "Archiver",
                unarchive: "Désarchiver",
                cancel: "Annuler",
                edit: "Éditer",
                yes: "Oui",
                no: "Non",
                save: "Sauvegarder",
                close: "Fermer",
                back: "Retour",
                next: "Suivant",
                warning: "Avertissement",
                json: "Aperçu JSON",
                noResult: "Aucun résultat",
                import: "Importer",
                export: "Exporter",
                loadingData: "Chargement des données ...",
                exportFormat: "Exporter au format {{format}} - Résultats de '{{item}}",
                allResults: "de l'événement électoral",
                globalAreaResults: "de toutes les zones",
                title: "Titre",
                subtitle: "Sous-titre",
                kind: "Type de fichier",
                filter: "Filtres personnalisés",
                approve: "Approuver",
                continue: "Continuer",
                logout: "Déconnexion",
                selectTenant: "Sélectionner un locataire",
                processing: "Traitement en cours...",
                tenantName: "Nom du locataire",
            },
            language: {
                es: "Espagnol",
                en: "Anglais",
                fr: "Français",
                cat: "Valencien",
                tl: "Tagalog",
                gl: "Galego",
                nl: "Néerlandais",
                eu: "Euskera",
            },
            channel: {
                online: "En ligne",
                kiosk: "Kiosque",
                early_voting: "Vote anticipé",
                telephone: "Vote par téléphone",
                other: "Autre",
            },
            message: {
                delete: "Êtes-vous sûr de vouloir supprimer cet élément ?",
                continueOrLogout: "Voulez-vous continuer ou vous déconnecter ?",
            },
        },
        createResource: {
            electionEvent: "Créer un Événement Électoral",
            election: "Créer une Élection",
            contest: "Créer un Concours",
            candidate: "Créer un Candidat",
        },
        importResource: {
            electionEvent: "Importer un Événement Électoral",
            election: "Importer une Élection",
            contest: "Importer un Concours",
            candidate: "Importer un Candidat",
            ImportHashMismatch: "Hashes don't match. Integrity check failure.",
        },
        sideMenu: {
            electionEvents: "Processus Électoraux",
            search: "Chercher",
            usersAndRoles: "Utilisateurs et Rôles",
            logs: "Journaux",
            settings: "Configuration",
            help: "Aide",
            templates: "Modèles",
            active: "Actifs",
            archived: "Archivés",
            addResource: {
                electionEvent: "Créer un Événement Électoral",
                election: "Créer une Élection",
                contest: "Créer un Concours",
                candidate: "Créer un Candidat",
            },
            menuActions: {
                archive: {
                    electionEvent: "Archiver cet Événement Électoral",
                },
                unarchive: {
                    electionEvent: "Désarchiver cet Événement Électoral",
                    election: "Désarchiver cette Élection",
                    contest: "Désarchiver ce Concours",
                    candidate: "Désarchiver ce Candidat",
                },
                remove: {
                    electionEvent: "Supprimer cet Événement Électoral",
                    election: "Supprimer cette Élection",
                    contest: "Supprimer ce Concours",
                    candidate: "Supprimer ce Candidat",
                },
                messages: {
                    confirm: {
                        archive: "Êtes-vous sûr de vouloir archiver cet élément ?",
                        unarchive: "Êtes-vous sûr de vouloir désarchiver cet élément ?",
                        delete: "Êtes-vous sûr de vouloir supprimer cet élément ?",
                        sealsUnknown:
                            "Les scellés des urnes n'ont pas pu être vérifiés : si l'élément a des urnes scellées, la suppression est refusée.",
                    },
                    notification: {
                        success: {
                            archive: "L'élément a été archivé",
                            unarchive: "L'élément a été désarchivé",
                            delete: "L'élément a été supprimé",
                            reloading:
                                "Veuillez patienter. La page va se recharger dans quelques instants.",
                        },
                        error: {
                            archive: "Erreur lors de la tentative d'archivage de cet élément",
                            unarchive: "Erreur lors de la tentative de désarchivage de cet élément",
                            delete: "Erreur lors de la tentative de suppression de cet élément",
                            deleteSealedElection:
                                "Cette élection a des urnes scellées et ne peut pas être supprimée. Archivez plutôt son événement électoral.",
                            deleteSealedEvent:
                                "Cet événement électoral a des urnes scellées et ne peut pas être supprimé. Archivez-le plutôt.",
                            deleteMaybeSealed:
                                "Erreur lors de la suppression de cet élément. S'il a des urnes scellées, il ne peut pas être supprimé.",
                        },
                    },
                },
            },
        },
        candidateScreen: {
            common: {
                subtitle: "Configuration des candidats.",
            },
            edit: {
                externalId: "ID externe",
                general: "Général",
                type: "Type",
                image: "Image",
                isDisabled: "Désactivé",
                isExplicitInvalid: "Vote Inválido",
                isExplicitBlank: "Vote Blanc",
                isCategoryList: "Liste",
                isWriteIn: "Vote par Écrit",
            },
            field: {
                name: "Nom",
                alias: "Alias",
                description: "Description",
            },
            options: {
                "candidate": "Candidat",
                "option": "Option",
                "write-in": "Vote par Écrit",
                "open-list": "Liste Ouverte",
                "closed-list": "Liste Fermée",
                "semi-open-list": "Liste Semi-ouverte",
                "invalid-vote": "Vote Inválido",
                "blank-vote": "Vote Blanc",
            },
            invalidVotePosition: {
                label: "Position du Vote Invalide",
                null: "Aucune (Par défaut)",
                top: "Haut",
                bottom: "Bas",
            },
            error: {},
            createCandidateSuccess: "Candidat créé",
            createCandidateError: "Erreur lors de la création du candidat",
        },
        contestScreen: {
            common: {
                subtitle: "Configuration des questions.",
            },
            edit: {
                externalId: "ID externe",
                general: "Général",
                type: "Type",
                image: "Image",
                system: "Système de vote de bulletins",
                design: "Design du bulletin",
                reorder: "Réorganiser les candidats",
                policies: "Politiques",
            },
            field: {
                name: "Nom",
                alias: "Alias",
                description: "Description",
            },
            options: {
                "non-preferential": "Sans Préférence",
                "plurality-at-large": "Pluralité Générale",
                "instant-runoff": "Vote à Second Tour Instantané",
                "random": "Aléatoire",
                "external-procedure": "Procédure externe",
                "custom": "Personnalisé",
                "alphabetical": "Alphabétique",
            },
            tieBreakingPolicy: {
                label: "Politique de départage",
            },
            auditButtonConfig: {
                "label": "Options d'affichage du bouton d'audit",
                "show": "Afficher",
                "not-show": "Ne pas afficher",
                "show-in-help": "Afficher dans la boîte de dialogue d'aide",
            },
            underVotePolicy: {
                "label": "Politique de Sous-Vote",
                "allowed": "Autorisé",
                "warn-only-in-review": "Avertir en Révision",
                "warn": "Avertir",
                "warn-and-alert": "Avertir et Alerter",
                "warn-and-confirm-in-review": "Avertir et Confirmer en Révision",
            },
            invalidVotePolicy: {
                "label": "Politique de vote invalide",
                "allowed": "Permis",
                "warn": "Avertissement",
                "warn-invalid-implicit-and-explicit": "Avertir Inválidos Implicites et Explicites",
                "not-allowed": "Non Permis",
                "allowed-with-exclusive-explicit": "Permis avec Vote Invalide Explicite Exclusif",
            },
            candidatesIconCheckboxPolicy: {
                "label": "Forme d’icône de case à cocher pour les candidats",
                "square-checkbox": "Case à cocher carrée",
                "round-checkbox": "Case à cocher ronde",
            },
            checkableListPolicy: {
                "allow-selecting-candidates-and-lists": "Candidats Et Listes",
                "allow-selecting-candidates": "Seulement Candidats",
                "allow-selecting-lists": "Seulement Listes",
                "disabled": "Désactivé",
            },
            collapsibleListsPolicy: {
                "label": "Listes repliables",
                "disabled": "Désactivé",
                "enabled-expanded": "Activé (commence déplié)",
                "enabled-collapsed": "Activé (commence replié)",
            },
            blankVotePolicy: {
                "label": "Politique de vote blanc",
                "allowed": "Autorisé",
                "warn-only-in-review": "Avertir en Révision",
                "warn": "Avertir",
                "not-allowed": "Non autorisé",
            },
            overVotePolicy: {
                "label": "Politique de vote excessive",
                "allowed": "Autorisé",
                "allowed-with-msg": "Autorisé avec un message d'avertissement",
                "allowed-with-msg-and-alert":
                    "Autorisé avec un message d'avertissement et d'alerte",
                "not-allowed-with-msg-and-alert":
                    "Non autorisé avec un message d'avertissement et d'alerte",
                "not-allowed-with-msg-and-disable":
                    "Non autorisé avec un message d'avertissement et désactiver d'autres sélections",
            },
            duplicatedRankPolicy: {
                "label": "Vote invalide - Politique de rang dupliqué",
                "allowed-warn-and-dialog":
                    "Afficher avertissement et dialogue (l'électeur peut continuer)",
                "not-allowed-warn-and-dialog":
                    "Afficher avertissement et dialogue (l'électeur n'est pas autorisé à continuer)",
            },
            preferenceGapsPolicy: {
                "label": "Vote invalide - Politique de rangs ignorés",
                "allowed-warn-and-dialog":
                    "Afficher avertissement et dialogue (l'électeur peut continuer)",
                "not-allowed-warn-and-dialog":
                    "Afficher avertissement et dialogue (l'électeur n'est pas autorisé à continuer)",
            },
            paginationPolicy: {
                label: "Nom de la page",
            },
            isAcclaimed: {
                label: "Acquis par acclamation",
                helperText:
                    "Les électeurs voient ce vote mais ne peuvent rien sélectionner, rien n'est enregistré et tous les candidats sont déclarés élus avec zéro voix. À définir avant la publication des bulletins : le modifier ensuite invalide les bulletins déjà déposés.",
            },
            allowWriteins: {
                label: "Autoriser les candidatures manuscrites",
            },
            maxVotes: {
                helperText:
                    "Nombre maximum de candidats qu'un électeur peut sélectionner (vote non préférentiel).",
                helperTextPreferential:
                    "Position de classement la plus haute disponible (ex. '5' signifie positions 1 à 5). Doit être au moins égal au nombre de candidats à classer (vote préférentiel).",
            },
            error: {},
            createContestSuccess: "Question créée",
            createContestError: "Erreur lors de la création de la question",
        },
        keysGeneration: {
            configureStep: {
                create: "Créer une Cérémonie de Clés",
                name: "Nom de la Cérémonie des Clés",
                allElections: "Toutes les Élections",
                title: "Créer une Cérémonie de Clés de l'Événement Électoral",
                subtitle:
                    "Dans cette cérémonie, chaque autorité générera et téléchargera sa part des clés privées pour l'Événement Électoral. Pour continuer, choisissez les autorités qui participeront à la cérémonie et le seuil, qui est le nombre minimum d'autorités nécessaires pour compter.",
                trusteeList: "Autorités",
                threshold: "Seuil",
                errorMinTrustees_one:
                    "Vous avez sélectionné seulement {{selected}} autorité, mais vous devez en sélectionner au moins {{threshold}}.",
                errorMinTrustees_other:
                    "Vous avez sélectionné seulement {{selected}} autorités, mais vous devez en sélectionner au moins {{threshold}}.",
                errorThreshold:
                    "Vous avez sélectionné un seuil de {{selected}} mais il doit être entre {{min}} et {{max}}.",
                errorCreatingCeremony:
                    "Erreur lors de la création de la Cérémonie de Clés : {{error}}",
                createCeremonySuccess: "Cérémonie de Clés créée",
                confirmdDialog: {
                    ok: "Oui, Créer une Cérémonie de Clés",
                    cancel: "Annuler",
                    title: "Êtes-vous sûr de vouloir Créer une Cérémonie de Clés ?",
                    automaticCeremonyTitle:
                        "Êtes-vous sûr de vouloir créer une cérémonie de clés automatique ?",
                    description:
                        "Vous êtes sur le point de Créer une Cérémonie de Clés. Cette action notifiera aux Autorités de participer à la création et distribution des Clés de l'Événement Électoral.",
                    automaticCeremonyDescription:
                        "Vous êtes sur le point de créer une cérémonie de clés automatique. Cela n'informera pas les fiduciaires de leur participation.",
                },
                filterTrustees: "Filtre des Autorités",
                errorPermisionLabels:
                    "Impossible de créer la cérémonie de clés : une ou plusieurs étiquettes d’autorisations sont manquantes.",
                automaticCeremonyToggle: "Cérémonie automatique",
            },
            ceremonyStep: {
                cancel: "Annuler la Cérémonie de Clés",
                progressHeader: "Progrès de la Cérémonie de Clés",
                description:
                    "Cet écran montre le progrès et les journaux de la Cérémonie de Clés de l'Événement Électoral. Dans la Cérémonie de Clés, chaque autorité générera et téléchargera son fragment de la clé privée pour l'Événement Électoral.",
                executionStatus: "État : {{status}}",
                confirmdDialog: {
                    ok: "Oui, Annuler la Création de la Cérémonie de Clés",
                    cancel: "Revenir à la Cérémonie de Clés",
                    title: "Êtes-vous sûr de vouloir Annuler la Cérémonie de Clés ?",
                    description:
                        "Vous êtes sur le point d'Annuler la Cérémonie de Clés. Après avoir effectué cette action, pour avoir une Cérémonie de Clés réussie, vous devrez en Créer une nouvelle.",
                },
                header: {
                    trusteeName: "Nom de l'Autorité",
                    fragment: "Fragment de Clé Généré",
                    downloaded: "Fragment Privé de Clé Téléchargé",
                    checked: "Fragment Privé de Clé Vérifié",
                },
                logsHeader: {
                    title: "Journaux",
                    date: "Date",
                    entry: "Entrée",
                },
                emptyLogs: "Pas de journaux pour l'instant.",
            },
            startStep: {
                title: "Cérémonie de Clés de l'Autorité",
                subtitle:
                    "Vous êtes sur le point de participer à la Cérémonie de Clés en tant qu'Autorité (<strong>{{name}}</strong>). Cela implique les étapes suivantes :",
                one: "<strong>Télécharger</strong> votre Clé Privée Cryptée.",
                two: "Créer plusieurs <strong>Copies de sauvegarde</strong> de votre Clé Privée Cryptée.",
                three: "<strong>Vérifier</strong> que les copies de sauvegarde fonctionnent correctement.",
            },
            downloadStep: {
                title: "Télécharger Clé Privée Cryptée",
                subtitle:
                    "Pour continuer, veuillez télécharger et sauvegarder votre Clé Privée Cryptée sur au moins deux appareils différents :",
                downloadButton: "Télécharger votre Clé Privée Cryptée",
                downloaded: "Clé Privée Cryptée téléchargée avec succès.",
                errorEmptyKey: "Erreur de téléchargement, fichier vide",
                unexpectedError: "La clé privée n'a pas pu être téléchargée. Veuillez réessayer.",
                alreadyVerified: "Votre clé privée a déjà été téléchargée et vérifiée.",
                unavailable:
                    "Le téléchargement de la clé privée n'est plus disponible car la cérémonie a progressé.",
                confirmdDialog: {
                    ok: "Confirmer les copies de sauvegarde et Continuer",
                    cancel: "Revenir",
                    title: "Copie de sauvegarde de votre Clé Privée Cryptée",
                    description:
                        "Veuillez effectuer une copie de sauvegarde de votre Clé Privée Cryptée à au moins deux emplacements sécurisés différents et confirmez-le ensuite :",
                    firstCopy: "Première copie de sauvegarde réalisée",
                    secondCopy: "Deuxième copie de sauvegarde réalisée",
                    confirmError:
                        "Créez les sauvegardes requises et cochez les cases de confirmation pour continuer",
                },
            },
            checkStep: {
                title: "Vérifiez vos Copies de Sauvegarde de votre Clé Privée Cryptée",
                verifyButton: "Vérifier la clé",
                subtitle:
                    "Chargez la Copie de Sauvegarde de votre Clé Privée Cryptée pour vérifier qu'elle est correcte. Vous pouvez essayer autant de fois que nécessaire, depuis vos différentes copies de sauvegarde :",
                errorUploading:
                    "Copa de Sauvegarde de la Clé Privée Cryptée invalide, veuillez réessayer",
                errorEmptyFile: "Fichier vide ou non trouvé",
                verified: "Copie de sauvegarde vérifiée avec succès.",
            },
        },
        miruExport: {
            create: {
                success: "",
                error: "",
            },
            send: {
                success: "",
                error: "",
            },
        },
        tally: {
            errorUploadingSignature:
                "Une erreur s'est produite lors du téléchargement de la signature",
            downloadTransmissionPackage: "Télécharger le paquet",
            resultsPublication: {
                sectionTitle: "Publish to results website",
                policyTitle: "Results Website",
                policyAccess: "Results Website Access",
                policyVisibility: "Results Website Visibility",
                enabled: "Enabled",
                disabled: "Disabled",
                fullEvent: "Full event",
                areaBased: "Area based",
                publishStarted: "Results publication started",
                publishError: "Could not start results publication",
                revoked: "Results publication revoked",
                revokeError: "Could not revoke results publication",
                waitingForTally: "Results can be published after this tally has completed.",
                writePermissionRequired:
                    "You need publish-results-write permission to publish or revoke results.",
                readPermissionRequired:
                    "You need publish-results-read permission to view publication history.",
                disabledPolicy:
                    "Results website publishing is disabled for this election event. Enable it in the election event data before publishing results.",
                loadingElectionContext: "Results publication is loading election context.",
                route: "Route",
                eventResults: "Event results",
                electionResults: "Election results",
                election: "Election",
                access: "Access",
                publicAccess: "Public access",
                authenticatedAccess: "Authenticated access",
                visibility: "Visibility",
                fullPublishedScope: "Full published scope",
                personalVisibility: "Personal visibility",
                contests: "Contests",
                noTalliedContests: "No tallied contests available.",
                publishSelectedContests: "Publish selected contests",
                selectedContestCount: "{{count}} contest selected",
                selectedContestCount_plural: "{{count}} contests selected",
                history: "Publication history",
                version: "Version",
                status: "Status",
                published: "Published",
                revokedAt: "Revoked",
                actions: "Actions",
                open: "Open",
                revoke: "Revoke",
                noPublications: "No publications yet.",
                confirmTitle: "Start publish to results website?",
                confirmDescription:
                    "This will create a new publication from the current tally execution. The existing voter-facing results stay active until this publish task succeeds.",
                close: "Close",
            },
            transmissionPackage: {
                title: "Paquet de Transmission pour la Zone '{{name}}' y Elección '{{eventName}}'",
                description:
                    "Vous permet d'exporter un Paquet de Transmission vers des Serveurs de Destination ou de le télécharger.",
                actions: {
                    sign: {
                        title: "Régénérer",
                        dialog: {
                            title: "Voulez-vous signer le paquet de transmission ?",
                            description:
                                "Veuillez confirmer que vous souhaitez régénérer le paquet de transmission pour la zone `{{name}}`",
                            confirm: "Signer le paquet de transmission",
                            cancel: "Fermer",
                            input: {
                                placeholder: "Entrez votre mot de passe",
                            },
                        },
                    },
                    send: {
                        title: "Envoyer",
                        dialog: {
                            title: "Voulez-vous envoyer le Paquet de Transmission?",
                            description:
                                "Veuillez confirmer que vous souhaitez envoyer le Paquet de Transmission pour la Zone '{{name}}' aux Serveurs de Destination.",
                            confirm: "Envoyer le Paquet de Transmission",
                            cancel: "Fermer",
                        },

                        disabled:
                            "Les signatures requises manquent ou le paquet de transmission a déjà été envoyé à toutes les destinations.",
                    },
                    regenerate: {
                        title: "Régénérer",
                        dialog: {
                            title: "Voulez-vous régénérer le paquet de transmission?",
                            description:
                                "Veuillez confirmer que vous souhaitez régénérer le paquet de transmission pour la zone `{{name}}`",
                            confirm: "Régénérer le paquet de transmission",
                            cancel: "Fermer",
                        },
                    },
                    download: {
                        title: "Télécharger",
                        emlTitle: "Download EML {{date}}",
                        transmissionPackageTitle: "Télécharger le Paquet de Transmission {{date}}",
                        transmissionReportTitle: "Télécharger le rapport de transmission",
                        dialog: {
                            title: "Voulez-vous télécharger le Paquet de Transmission?",
                            description:
                                "Veuillez confirmer que vous souhaitez télécharger le Paquet de Transmission pour la Zone '{{name}}.'",
                            confirm: "Télécharger le Paquet de Transmission",
                            cancel: "Fermer",
                        },
                    },
                },
                destinationServers: {
                    title: "Serveurs de Destination",
                    description:
                        "Le tableau ci-dessous montre l'état d'envoi de chacun des Serveurs de Destination.",
                    status: "Envoyé à {{signed}} sur {{total}}",
                    table: {
                        serverName: "Nom du Serveur",
                        sendStatus: "État de l'Envoi",
                    },
                },
                signatures: {
                    title: "Signatures",
                    description:
                        "Les membres peuvent signer le paquet de transmission. Le tableau indique le statut de signature de chaque membre.",
                    table: {
                        trusteeName: "Membre",
                        signed: "A Signé",
                    },
                    status: "{{signed}} sur {{total}} Ont Signé",
                },
            },
            sendToTransmissionPackageServers:
                "Envoyer le paquet de transmission pour la zone '{{name}}'",
            uploadTransmissionPackage: "Télécharger",
            uploadTransmissionPackageDesc:
                "Téléchargez votre signature pour signer le paquet des Résultats Électoraux. Cette opération est optionnelle.",
            exportElectionArea: "Envoyer le paquet de transmission pour la zone '{{name}}'",
            generateReport: "Générer {{name}}",
            templateTitle: "Modèle de Résultats",
            templateSubTitle: "Éventuellement écraser le modèle de résultats.",
            keysCeremonyTitle: "Cérémonie des Clés",
            keysCeremonySubTitle: "Sélectionnez la Cérémonie des Clés pour ce dépouillement",
            ceremonyTitle: "Élections pour le Comptage",
            initializationTitle: "Élections pour le rapport d'initialisation",
            ceremonySubTitle: "Sélectionnez les élections pour le comptage",
            tallyTitle: "Progrès du Comptage des Élections",
            logsTitle: "Journaux",
            resultsTitle: "Résultats & Participation",
            generalInfoTitle: "Informations Générales",
            trusteeTallyTitle: "Trustee",
            trusteeTallySubTitle: "État d'importation du fragment de clé",
            ballotBoxes: {
                unavailable: "Scellés indisponibles",
                sealed: "{{sealed}} sur {{total}} scellées",
                publishing: "Scellées, {{published}} sur {{total}} sur le tableau d'affichage",
                sealing: "Scellement à {{time}}",
                notSealed: "Non scellées",
                help: "Une élection peut être dépouillée une fois que chaque urne est scellée et que son scellé figure sur le tableau d'affichage.",
                failed: "Non scellée : incident",
                overdue: "Scellement en retard",
                blocked: "{{name}} : {{reason}}",
                reason: {
                    "not-sealed":
                        "le vote n'est pas clôturé, ses urnes n'ont donc pas encore de scellés",
                    "sealing": "ses urnes sont scellées à la fin du délai de grâce",
                    "overdue":
                        "ses urnes ont dépassé leur échéance et ne sont pas encore scellées (voir son Tableau de Bord)",
                    "publishing":
                        "certains de ses scellés sont encore en cours de publication sur le tableau d'affichage",
                    "failed":
                        "une urne n'a pas pu être scellée, c'est un incident (voir son Tableau de Bord)",
                    "unavailable": "les scellés de ses urnes n'ont pas pu être lus",
                },
            },
            eligibility: {
                ballotBoxesUnavailable:
                    "Les scellés des urnes d'une élection sélectionnée n'ont pas pu être lus, elle ne peut donc pas encore être dépouillée. Rechargez la page pour réessayer.",
                selectElection: "Sélectionnez au moins une élection.",
                publishElection:
                    "Publiez chaque élection sélectionnée avant de créer son dépouillement.",
                tallyDisallowed: "Le dépouillement est désactivé pour une élection sélectionnée.",
                endVoting:
                    "Terminez le vote de chaque élection sélectionnée et arrêtez ses canaux actifs avant de créer le dépouillement.",
                sealBallotBoxes:
                    "Une élection peut être dépouillée une fois que chaque urne est scellée et que son scellé figure sur le tableau d'affichage.",
            },
            createTallySuccess: "Comptage créé",
            createTallyError: "Erreur lors de la création du comptage",
            startTallySuccess: "Comptage commencé",
            startTallyError: "Erreur lors du démarrage du comptage",
            startTallyCeremonySuccess: "Cérémonie de comptage commencée",
            startTallyCeremonyError: "Impossible de démarrer la cérémonie de comptage",
            cancelTallyCeremonySuccess: "Cérémonie de comptage annulée",
            cancelTallyCeremonyError: "Impossible d'annuler la cérémonie de comptage",
            recountTallyCeremony: "Recompter",
            recountTallyCeremonyMessage:
                "Cela générera un nouvel événement de résultats pour la session de comptage terminée.",
            recountTallyCeremonyStarting: "Démarrage du recomptage...",
            recountTallyCeremonySuccess: "Recomptage commencé",
            recountTallyCeremonyError: "Impossible de démarrer le recomptage",
            recountTallyCeremonyOk: "Recompter",
            trusteeTitle: "Processus du trustee",
            trusteeSubTitle: "Veuillez importer votre fragment de clé",
            invited: "Vous avez été invité à participer à une cérémonie de comptage. Veuillez, ",
            click: "cliquez sur l'action de comptage",
            participate: "pour participer.",
            breadcrumbSteps: {
                start: "Début",
                finish: "Fin",
                tally: "Comptage",
                results: "Résultats",
                ceremony: "Cérémonie",
            },
            common: {
                title: "Comptage",
                subTitle: "Configuration du Comptage.",
                cancel: "Arrière",
                next: "Suivant",
                date: "Date de Comptage",
                global: "Global",
                noTrustees: "Aucun trustee pour l'instant",
                imported: " trustees ont importé leur fragment de clé",
                needed: " trustees nécessaires pour le comptage",
                start: "Commencer Comptage",
                ceremony: "Commencer Cérémonie de Comptage",
                initialization: "Démarrer le rapport d'initialisation",
                results: "Résultats",
                dialog: {
                    ok: "Ok",
                    okTally: "Commencer comptage",
                    okCancel: "Annuler comptage",
                    cancel: "Fermer",
                    title: "Êtes-vous sûr de vouloir commencer une cérémonie ?",
                    tallyTitle: "Êtes-vous sûr de vouloir commencer le comptage ?",
                    cancelTitle: "Êtes-vous sûr de vouloir annuler le comptage ?",
                    message:
                        "Vous êtes sur le point de commencer une cérémonie de comptage. Cette action notifiera aux trustees d'importer leurs fragments de clé.",
                    cancelMessage:
                        "Vous êtes sur le point d'annuler la cérémonie de comptage. Cette action ne peut pas être annulée.",
                    ceremony:
                        "Tous les trustees requis ont vérifié leurs fragments de clé. Tout est prêt pour commencer à recevoir les résultats. Voulez-vous commencer le Comptage ?",
                    startAutomatedTallyMessage:
                        "Sélectionnez 'Start Tally' pour lancer le processus de décompte et afficher les résultats, ou 'Close' pour annuler.",
                },
            },
            table: {
                ballotBoxes: "Urnes",
                elections: "Élections",
                selected: "Sélectionnées",
                status: "État",
                progress: "Progrès",
                method: "Méthode de Comptage",
                elegible: "Votants Éligibles",
                number: "Nombre de Votes",
                total: "Total",
                turnout: "%",
                candidates: "Résultats des Candidats",
                options: "Options",
                global: "Résumé de participation",
                elegible_census: "Recensement des votants éligibles",
                cast_votes: "Nombre de Votes",
                cast_votes_percent: "Pourcentages de Votes",
                total_votes: "Total de votants",
                total_votes_percent: "Participation",
                total_votes_counted: "Total des Votes Comptés",
                total_auditable_votes: "Total des Votes Contrôlables",
                total_valid_votes: "Total de votes valides",
                total_valid_votes_percent: "Pourcentage de votes valides",
                total_invalid_votes: "Total de votes invalides",
                total_invalid_votes_percent: "Pourcentage de votes invalides",
                explicit_invalid_votes: "Votes explicitement invalides",
                explicit_invalid_votes_percent: "Pourcentage de votes explicitement invalides",
                implicit_invalid_votes: "Votes implicitement invalides",
                implicit_invalid_votes_percent: "Pourcentage de votes implicitement invalides",
                blank_votes: "Votes blancs",
                explicit_blank_votes: "Votes blancs explicites",
                implicit_blank_votes: "Votes blancs implicites",
                blank_votes_percent: "Pourcentage de votes blancs",
                number_of_votes: "Nombre de votes",
                winning_position: "Position gagnante",
                weight: "Poids",
                preferential: {
                    candidate: "Candidat",
                    winner: "Gagnant",
                    eliminated: "Éliminé",
                    round: "Tour",
                },
                total_declined_to_vote: "Total des refus de vote",
                total_blank_ballots: "Total des Bulletins Blancs",
                participation_by_channel: "Participation par canal",
                channel: "Canal",
                channel_online: "En ligne",
                channel_kiosk: "Kiosque",
                channel_early_voting: "Vote anticipé",
                channel_telephone: "Téléphone",
                channel_paper: "Papier",
                channel_postal: "Postal",
                channel_in_person: "En personne",
                acclamation_note:
                    "Élu par acclamation. Ce vote a été acquis sans scrutin : aucune voix n'a été enregistrée.",
            },
            pendingResolutions: {
                round: "Tour {{round}}",
                tieResolutionRequired: "Résolution de partage requise",
                tieResolved: "Partage résolu",
                globalArea: "Global",
                pendingResolutionsHeader: "Résolutions en attente",
                pendingResolutionStatus: "Résolution en attente",
                resolvedStatus: "Résolue",
                resolutionTitle: "Résolution",
                selectContest: "Sélectionnez un élément à gauche pour voir les détails",
                selectCandidateToAdvance: "Sélectionnez le candidat à faire avancer",
                undoResolution: "Annuler la résolution",
                applyResolutions: "Appliquer les résolutions et recalculer",
                submitSuccess: "Résolutions soumises. Le comptage reprend...",
                submitError: "Échec de la soumission des résolutions. Veuillez réessayer.",
                filter: "Filtrer",
                save: "Enregistrer",
                pendingApplyStatus: "Calcul en attente",
                filterElection: "Élection",
                filterContest: "Concours",
                filterArea: "Zone",
                filterStatusLabel: "Statut",
                clearFilters: "Effacer les filtres",
                candidateWithVotes: "{{name}} ({{votes}} voix)",
                candidateWithVotesAndPercent: "{{name}} ({{votes}} voix, {{percent}}%)",
                tieInfoTitle: "Décompte pausé en raison d'une égalité non résolue (Tour {{round}})",
                tieInfoBody:
                    "Candidats à égalité ({{votes}} voix, {{percent}}%) : {{candidates}}. Un départage manuel est requis pour continuer le décompte.",
                tallyResumedTitle: "Décompte repris après application de la résolution",
                tallyResumedBody: "L'égalité a été résolue le {{date}} par {{user}}",
            },
            chart: {
                votesForCandidates: "Votes pour les Candidats",
                blankVotes: "Votes Blancs",
                invalidVotes: "Votes Invalides",
                totalVoters: "Total des Électeurs",
                nonVoters: "Non-Électeurs",
            },
            exportAllAreas:
                "Exporter les résultats de toutes les zones au format {{format}} pour '{{item}}'",
        },
        publish: {
            initialization: {
                countryInfo:
                    "Générez le rapport pour tout le poste ou pour un pays. Le vote reste bloqué tant que toutes les initialisations requises par pays et pour l’événement entier ne sont pas terminées.",
                countriesError: "Impossible de charger les pays admissibles. Fermez et réessayez.",
                noCountries:
                    "Ce poste n’a aucun pays admissible avec des modèles de bulletin actifs. Vérifiez ses zones et sa publication avant l’initialisation.",
                country: "Pays",
                entirePost: "Poste entier",
            },
            preview: {
                publicationAreas: "Sélectionnez la zone pour l'aperçu",
                action: "Aperçu",
                copy: "Copier le lien",
                copy_success: "Copie réussie du lien d'aperçu",
                copy_error: "Échec de la copie du lien d'aperçu",
                success: "Succès lors de l'ouverture de l'aperçu",
            },
            header: {
                change: "Changements à Publier",
                viewChange: "Voir Publication",
                history: "Historique des Changements",
            },
            action: {
                generateInitializationReport: "Générer le Rapport d'Initialisation",
                startVotingPeriod: "Commencer la période de vote",
                startKioskVoting: "Commencer Vote au Kiosque",
                startOnlineVoting: "Commencer Vote en Ligne",
                startEarlyVoting: "Commencer Vote Anticipé",
                startTelephoneVoting: "Commencer Vote Téléphone",
                stopVotingPeriod: "Arrêter la période de vote",
                stopOnlineVoting: "Arrêter le Vote en Ligne",
                stopEarlyVoting: "Arrêter le Vote Anticipé",
                stopTelephoneVoting: "Arrêter le Vote Téléphone",
                stopKioskVotingPeriod: "Arrêter le Vote au Kiosque",
                pauseVotingPeriod: "Mettre en pause la période de vote",
                pauseKioskVoting: "Mettre en pause le Vote au Kiosque",
                pauseOnlineVoting: "Mettre en pause le Vote en Ligne",
                pauseEarlyVoting: "Mettre en pause le Vote Anticipé",
                pauseTelephoneVoting: "Mettre en pause le Vote Téléphone",
                generate: "régénérer",
                publish: "Publier Changements",
                back: "Arrière",
            },
            empty: {
                header: "Aucune Publication pour l'instant.",
                action: "Générer Publication",
            },
            forbidden: {
                header: "Impossible de publier tant que la cérémonie des clés n'est pas terminée.",
            },
            skippedElections: {
                dismiss: "Fermer",
                title: "Certaines élections restent closes",
                ballotBoxSealPolicy:
                    "{{name}} reste close : son vote est clôturé et Sceller à la clôture rend la clôture définitive.",
                other: "{{name}} a été laissée en l'état ({{reason}}).",
            },
            dialog: {
                title: "Confirmer Action",
                info: "Vous avez cliqué sur une action sensible, nous avons donc besoin que vous la confirmiez pour pouvoir continuer.",
                initializationInfo:
                    "Vous êtes sur le point de générer le rapport d'initialisation. Êtes-vous sûr de vouloir continuer?",
                startInfo:
                    "Vous êtes sur le point de commencer la période de vote. Êtes-vous sûr de vouloir continuer?",
                stopInfo:
                    "Vous êtes sur le point d'arrêter la période de vote. Êtes-vous sûr de vouloir continuer?",
                stopSeal:
                    "Vous êtes sur le point d'arrêter le vote dans {{name}}. Ses urnes seront alors scellées : aucun bulletin ne pourra être ajouté, modifié ni supprimé, et le vote ne pourra pas reprendre. Êtes-vous sûr de vouloir continuer ?",
                stopSealEvent:
                    "Vous êtes sur le point d'arrêter le vote dans toutes les élections. Leurs urnes seront alors scellées : aucun bulletin ne pourra être ajouté, modifié ni supprimé, et le vote ne pourra pas reprendre. Êtes-vous sûr de vouloir continuer ?",
                startSealNote:
                    "Avec Sceller à la clôture, un vote clôturé reste clos : les élections dont le vote est clôturé n'ouvriront pas.",
                channel: {
                    ONLINE: "Vote en ligne",
                    KIOSK: "Vote au kiosque",
                    EARLY_VOTING: "Vote anticipé",
                    TELEPHONE: "Vote par téléphone",
                },
                kioskStopInfo:
                    "Vous êtes sur le point d'arrêter la période de vote au kiosque. Êtes-vous sûr de vouloir continuer ?",
                pauseInfo:
                    "Vous êtes sur le point de mettre en pause la période de vote. Êtes-vous sûr de vouloir continuer?",
                publishInfo:
                    "Vous êtes sur le point de générer une publication. Êtes-vous sûr de vouloir continuer ?",
                ok: "Confirmer",
                ko: "Annuler",
                error: "Erreur lors du chargement des bulletins publiés",
                error_publish: "Erreur lors de la publication du bulletin",
                error_capacity: "Échec de la génération du style de bulletin : {{message}}",
                error_status: "Erreur lors du changement d'état de la publication",
                error_preview: "Erreur lors de l'aperçu de la publication",
                diff: "Afficher tous les changements pourrait rendre la page non réactive. Êtes-vous sûr de vouloir continuer ?",
                confirmation:
                    "L'action que vous êtes sur le point d'effectuer est sensible et nécessite une confirmation. Veuillez entrer votre mot de passe pour continuer avec {{action}}.",
                stopSealNotYet:
                    "Vous êtes sur le point d'arrêter la période de vote. {{holding}} Êtes-vous sûr de vouloir continuer ?",
                sealHolding_one:
                    "Avec Sceller à la clôture, ses urnes sont scellées une fois tous les canaux activés clôturés : {{channels}} est encore activé et non clôturé.",
                sealHolding_other:
                    "Avec Sceller à la clôture, ses urnes sont scellées une fois tous les canaux activés clôturés : {{channels}} sont encore activés et non clôturés.",
                sealNotEnabled:
                    "{{channel}} n'est pas clôturé et n'est pas activé pour cette élection : arrêtez-le pour sceller les urnes.",
                sealNotEnabledPost:
                    "Dans {{post}}, {{channel}} n'est pas clôturé et n'est pas activé : arrêtez-le pour cette élection afin de sceller ses urnes.",
                sealNoChannel:
                    "Aucun canal n'est activé pour cette élection et aucun n'a été ouvert : ses urnes ne sont donc pas scellées.",
                stopNeverOpened_one:
                    "{{channels}} n'a jamais ouvert : l'arrêter signifie qu'il n'ouvrira pas.",
                stopNeverOpened_other:
                    "{{channels}} n'ont jamais ouvert : les arrêter signifie qu'ils n'ouvriront pas.",
                stopSealGrace_one:
                    "Vous êtes sur le point d'arrêter le vote dans {{name}}. Ses urnes seront scellées à la fin du délai de grâce, {{count}} minute plus tard : dès lors, aucun bulletin ne pourra être ajouté, modifié ni supprimé. Le vote ne pourra pas reprendre. Êtes-vous sûr de vouloir continuer ?",
                stopSealGrace_other:
                    "Vous êtes sur le point d'arrêter le vote dans {{name}}. Ses urnes seront scellées à la fin du délai de grâce, {{count}} minutes plus tard : dès lors, aucun bulletin ne pourra être ajouté, modifié ni supprimé. Le vote ne pourra pas reprendre. Êtes-vous sûr de vouloir continuer ?",
                stopSealEventGrace_one:
                    "Vous êtes sur le point d'arrêter le vote dans toutes les élections. Leurs urnes seront scellées à la fin du délai de grâce de chaque élection, jusqu'à {{count}} minute plus tard : dès lors, aucun bulletin ne pourra être ajouté, modifié ni supprimé. Le vote ne pourra pas reprendre. Êtes-vous sûr de vouloir continuer ?",
                stopSealEventGrace_other:
                    "Vous êtes sur le point d'arrêter le vote dans toutes les élections. Leurs urnes seront scellées à la fin du délai de grâce de chaque élection, jusqu'à {{count}} minutes plus tard : dès lors, aucun bulletin ne pourra être ajouté, modifié ni supprimé. Le vote ne pourra pas reprendre. Êtes-vous sûr de vouloir continuer ?",
                stopSealEventSome:
                    "Vous êtes sur le point d'arrêter le vote dans toutes les élections. {{sealed}} {{holding}} Êtes-vous sûr de vouloir continuer ?",
                sealedNowPart:
                    "Les urnes de {{names}} seront alors scellées : aucun bulletin ne pourra être ajouté, modifié ni supprimé, et le vote ne pourra pas y reprendre.",
                sealedGracePart_one:
                    "Les urnes de {{names}} seront scellées à la fin de leur délai de grâce, jusqu'à {{count}} minute plus tard.",
                sealedGracePart_other:
                    "Les urnes de {{names}} seront scellées à la fin de leur délai de grâce, jusqu'à {{count}} minutes plus tard.",
                holdingEventPart_one:
                    "{{names}} garde un autre canal activé et non clôturé : ses urnes seront scellées une fois ce canal clôturé.",
                holdingEventPart_other:
                    "{{names}} gardent un autre canal activé et non clôturé : leurs urnes seront scellées une fois ces canaux clôturés.",
                noChannelEventPart_one:
                    "{{names}} n'active aucun canal et aucun n'y a été ouvert : ses urnes ne sont pas scellées.",
                noChannelEventPart_other:
                    "{{names}} n'activent aucun canal et aucun n'y a été ouvert : leurs urnes ne sont pas scellées.",
                startSealNoteList:
                    "Avec Sceller à la clôture, un vote clôturé reste clos : {{items}}.",
                startKeptChannels_one: "{{post}} : {{channels}} reste clos",
                startKeptChannels_other: "{{post}} : {{channels}} restent clos",
                startKeptSealed: "{{post}} reste clos, car ses urnes sont scellées",
                startNotEnabledList:
                    "Avec Sceller à la clôture, un démarrage n'ouvre un canal que dans les Posts qui l'activent : {{items}}.",
                startNotEnabledChannels_one:
                    "{{post}} : {{channels}} n'y est pas activé et reste Non commencé",
                startNotEnabledChannels_other:
                    "{{post}} : {{channels}} n'y sont pas activés et restent Non commencés",
                stopNeverOpenedPosts_one:
                    "{{names}} n'a jamais ouvert : l'arrêter le clôture et scelle ses urnes vides.",
                stopNeverOpenedPosts_other:
                    "{{names}} n'ont jamais ouvert : les arrêter les clôture et scelle leurs urnes vides.",
            },
            label: {
                current: "Actuel",
                previous: "Publication Précédente",
                diff: "Changements à publier",
                publication: "Publication",
            },
            notifications: {
                generated: "Bulletin généré",
                published: "Bulletin publié",
                change_status: "État de votation changé",
            },
            sealRefusals: {
                startAgain:
                    "Le vote ne peut pas reprendre : avec Sceller à la clôture, un vote clôturé reste clos et ses urnes sont scellées.",
                startDisabled:
                    "Commencer la période de vote n'est pas disponible : les urnes de cette élection sont scellées ou en cours de scellement, et avec Sceller à la clôture un vote clôturé reste clos.",
                closedIsFinal:
                    "Un vote clôturé ne peut pas changer : avec Sceller à la clôture, un vote clôturé reste clos.",
            },
        },
        emailEditor: {
            subject: "Sujet de l'Email",
            tabs: {
                plaintext: "Corps de Texte Plano",
                richtext: "Corps de Texte Enrichi",
            },
        },
        sendCommunication: {
            send: "Envoyer",
            title: "Envoyer Notification",
            subtitle: "Envoyer une notification aux utilisateurs/électeurs.",
            sendButton: "Envoyer Notification",
            voters: "Audience",
            schedule: "Calendrier",
            nowInput: "Envoyer maintenant",
            dateInput: "Date et heure de début d'envoi",
            chooseDate: "Veuillez choisir une date",
            languages: "Langues",
            smsMessage: "Message SMS",
            errorSending: "Erreur lors de l'envoi de la notification : {{error}}",
            successSending: "Notification programmée/envoyée avec succès",
            method: "Méthode de Modèles",
            type: "Type de Communication",
            alias: "Alias de la Modèle",
            votersSelection: {
                ALL_USERS: "Tous",
                NOT_VOTED: "Ceux qui n'ont pas voté",
                VOTED: "Ceux qui ont déjà voté",
                SELECTED: "À {{total}} Électeurs sélectionnés",
            },
            path: {
                users: "utilisateurs",
                voters: "électeurs",
            },
            methodTitle: "Méthode de Communication",
            communicationMethod: {
                EMAIL: "Email",
                SMS: "SMS",
                WHATSAPP: "WhatsApp",
                VIBER: "Viber",
                MESSENGER: "Facebook Messenger",
            },
            communicationType: {
                CREDENTIALS: "Identifiants",
                BALLOT_RECEIPT: "Reçu de Vote",
            },
            email: {
                subject: "Sujet",
            },
        },
        tallysheet: {
            title: "Urnes",
            subtitle: "Urnes numérisées par canal",
            createTallySuccess: "Feuille de Comptage créée",
            createTallyError: "Erreur lors de la création de la Feuille de Comptage",
            createTallyErrorSameKindExists:
                "La feuille de décompte existe déjà pour ce scrutin avec le même canal et la même zone",
            allFieldsRequired: "Tous les champs sont obligatoires",
            header: {
                change: "Changements à Publier",
                viewChange: "Voir Publication",
                history: "Historique de Publication",
            },
            action: {
                start: "Commencer Élection",
                stop: "Arrêter Élection",
                pause: "Pause",
                generate: "Régénérer",
                publish: "Publier Changements",
                back: "Arrière",
            },
            inputError: {
                totalValidDoesNotMatch:
                    "Les votes des candidats ({{candidateVotesSum}}) doivent être compris entre {{lowerBound}} et {{upperBound}} selon les règles de vote de ce scrutin ({{nonBlankValidVotes}} votes valides non blancs × jusqu'à {{maxMarks}} marques par bulletin)",
                censusTooSmall:
                    "Le total des votes ({{totalVotes}}) ne doit pas être supérieur au recensement ({{census}})",
                totalInvalidDoesNotMatch:
                    "Le total des votes invalides ({{totalInvalid}}) doit être égal aux votes invalides implicites ({{implicitInvalid}}) plus les votes invalides explicites ({{explicitInvalid}})",
                totalVotesDoesNotMatch:
                    "Le total des votes ({{totalVotes}}) doit être égal au total des votes valides ({{totalValidVotes}}) plus le total des votes invalides ({{totalInvalid}})",
                unknownCountingAlgorithm:
                    "L'algorithme de dépouillement de ce scrutin ({{countingAlgorithm}}) n'est pas reconnu, le nombre autorisé de votes de candidats ne peut donc pas être déterminé. Vérifiez la configuration du scrutin.",
                blankBallotsInconsistent:
                    "Les Bulletins Blancs doivent avoir la même valeur sur toutes les feuilles de dépouillement de cette urne",
                blankBallotsOutOfBounds:
                    "La valeur des Bulletins Blancs est en dehors de la plage impliquée par les décomptes de votes blancs par candidature de cette urne",
            },
            label: {
                area: "Zone",
                channel: "Canal",
                total_votes: "Votes Totales",
                total_valid_votes: "Votes Válidos Totales",
                total_invalid: "Votes Inválidos Totales",
                explicit_invalid: "Votes Explícitement Inválidos",
                implicit_invalid: "Votes Implicitement Inválidos",
                total_blank_votes: "Votes en Blanc Totales",
                blank_ballots: "Bulletins Blancs",
                census: "Recensement",
            },
            common: {
                tallyCeremony: {
                    manage: "Gérer la Cérémonie de Décompte",
                    view: "Voir la Cérémonie de Décompte",
                    cancel: "Annuler la Cérémonie de Décompte",
                    addKey: "Ajouter une Clé de Décompte",
                },
                edit: "Éditer",
                confirm: "Confirmer",
                back: "Arrière",
                next: "Suivant",
                cancel: "Arrière",
                data: "Données",
                title: "Feuille de Comptage",
                subtitle: "Configuration de la Feuille de Comptage.",
                candidates: "Candidats",
                save: "Sauvegarder",
                approve: "Approuver",
                disapprove: "Désapprouver",
                show: "Afficher",
                add: "Ajouter",
                versions: "Versions",
                warningDisapprove:
                    "Êtes-vous sûr de vouloir désapprouver cette Feuille de Décompte?",
                warningApprove: "Êtes-vous sûr de vouloir approuver cette Feuille de Décompte?",
            },
            empty: {
                header: "Aucune Feuille de Comptage.",
                action: "Générer Feuille de Comptage",
                add: "Ajouter",
            },
            breadcrumbSteps: {
                start: "Début",
                edit: "Éditer",
                confirm: "Confirmer",
                view: "Voir",
            },
            table: {
                area: "Zone",
                contest: "Cotienda",
                approvedVersion: "Version approuvée",
                latestVersion: "Dernière version",
                labels: "Étiquettes",
                annotations: "Annotations",
            },
            versionsTable: {
                title: "Versions de l'urne",
                version: "Version",
                createdBy: "Créé par",
                reviewedBy: "Révisé par",
                createdAt: "Créé le",
                reviewedAt: "Révisé le",
                sourceImport: "Import source",
                importStatus: "Statut de l'import",
                openImport: "Ouvrir l'import",
                sourceFile: "Fichier source",
            },
            message: {
                reviewError: "Erreur lors de la révision de la Feuille de Comptage",
                reviewSuccess: "Feuille de Comptage révisée",
            },
        },
        application: {
            import: {
                title: "Importer des Applications",
                subtitle: "Importer des données d'applications",
                paragraph:
                    "Importez des applications en utilisant une feuille de calcul au format Valeurs Séparées par Comma (CSV). Téléchargez un exemple de fichier d'importation CSV ici.",
                messages: {
                    success: "Applications importées avec succès",
                    error: "Erreur lors de l'importation des applications",
                },
            },
            export: {
                title: "Exporter des Applications",
                subtitle: "Exporter des données d'applications",
                button: "Exporter",
                paragraph:
                    "Exportez des applications en utilisant une feuille de calcul au format Valeurs Séparées par Comma (CSV).",
                messages: {
                    success: "Applications exportées avec succès",
                    error: "Erreur lors de l'exportation des applications",
                },
            },
        },
        template: {
            noPermissions: "Vous n'avez pas la permission d'accéder aux Modèles.",
            title: "Modèles",
            subtitle: "Liste des modèles",
            chooseMethods: "Choisir les méthodes",
            default: "Utiliser le modèle par défaut",
            empty: {
                title: "Aucun modèle",
                subtitle: "Voulez-vous créer un nouveau ?",
            },
            action: {
                createOne: "Créer Modèle",
            },
            create: {
                title: "Créer un Modèle",
                success: "Modèle créé",
                error: "Erreur lors de la création du modèle",
            },
            update: {
                success: "Modèle mis à jour",
                error: "Erreur lors de la mise à jour du modèle",
            },
            edit: {
                title: "Éditer un Modèle",
            },
            form: {
                smsMessage: "Message SMS",
                document: "Document",
                pdfOptions: "Options PDF",
                reportOptions: "Options de Rapport",
                name: "Nom du Modèle",
                alias: "Alias du Modèle",
                type: "Type",
                communicationMethod: "Méthode",
            },
            type: {
                CREDENTIALS: "Identifiants",
                INITIALIZATION_REPORT: "Rapport d'Initialisation",
                ELECTORAL_RESULTS: "Résultats Électoraux",
                BALLOT_IMAGES: "Images des Bulletins",
                BALLOT_RECEIPT: "Reçu de Vote",
                ACTIVITY_LOGS: "Journaux d'Activité",
                MANUAL_VERIFICATION: "Vérification Manuelle",
                PARTICIPATION_REPORT: "Rapport de Participation",
            },
            method: {
                email: "Email",
                sms: "SMS",
                document: "Document",
                whatsapp: "WhatsApp",
                viber: "Viber",
                messenger: "Facebook Messenger",
            },
            import: {
                title: "Importer des Modèles",
                subtitle: "Importer des données de modèles",
                paragraph:
                    "Importez des modèles en utilisant une feuille de calcul au format Valeurs Séparées par Comma (CSV). Téléchargez un exemple de fichier d'importation CSV ici.",
            },
        },
        materials: {
            audioInstructions: {
                screenLabel: "Instructions audio pour l'écran",
                languageLabel: "Langue de l'enregistrement",
                none: "Pas des instructions audio",
                helperText:
                    "Les électeurs entendent ce fichier lorsqu'ils demandent les instructions sur cet écran.",
                screens: {
                    "election-chooser": "Liste des élections",
                    "start": "Début",
                    "ballot": "Bulletin",
                    "review": "Vérification",
                    "confirmation": "Confirmation",
                    "audit": "Audit",
                    "ballot-locator": "Localisateur de bulletins",
                    "support-materials": "Documents d'aide",
                },
            },
            createMaterialSuccess: "Matériel de support créé",
            createMaterialError: "Erreur lors de la création du matériel de support",
            updateMaterialSuccess: "Matériel de support mis à jour",
            updateMaterialError: "Erreur lors de la mise à jour du matériel de support",
            common: {
                title: "Matériaux de Support",
                subtitle: "Entrer données du matériel de support.",
            },
            error: {
                title: "Le titre est obligatoire",
                document: "Le document est obligatoire",
            },
            fields: {
                isHidden: "Caché",
                publicUrl: "Lien public",
            },
            empty: {
                header: "Pas encore de matériel de support",
                action: "Générer du matériel de support",
            },
        },
        widget: {
            logs: "Journaux",
        },
        settings: {
            countries: {
                title: "Blocage des Pays",
                votingDescription:
                    "Choisissez ci-dessous les pays depuis lesquels vous souhaitez bloquer les votes.",
                enrollmentDescription:
                    "Choisissez ci-dessous les pays depuis lesquels vous souhaitez bloquer les inscriptions.",
                error: {
                    errorSaving: "Erreur lors de l'enregistrement de la liste des pays",
                },
            },
            backupRestore: {
                title: "Sauvegarde / Restauration de la configuration du locataire",
                backup: {
                    label: "Sauvegarde",
                    subtitle: "Sauvegarde des configurations du locataire",
                },
                restore: {
                    label: "Restaurer",
                    subtitle: "Restaurer la configuration du locataire",
                    title: "Importer les configurations du locataire",
                    paragraph:
                        "Importer les configurations du locataire, les configurations Keycloak, les rôles et les données de permissions à l'aide d'un dossier compressé.",
                    tenantConfigOption: "Importer les configurations du locataire",
                    keycloakConfigOption: "Importer les configurations Keycloak",
                    RolesConfigOption: "Importer les configurations des rôles et des permissions",
                },
            },
            previewScreen: {
                label: "Aperçus",
                noContent: "Aucun aperçu trouvé",
                table: {
                    title: "Aperçus externes",
                    description:
                        "Un registre des aperçus de styles de bulletins de vote générés via des requêtes externes",
                    requestedBy: "Demandé par",
                    document: "Document",
                    url: "URL",
                },
            },
            languages: {
                default: "Langue par défaut",
            },
        },
        approvalsScreen: {
            column: {
                status: "Statut",
                id: "ID de la demande",
                applicantId: "ID du demandeur",
                verificationType: "Vérification",
                createdAt: "Demandée",
                verified_by: "Vérifiée par",
                voter: "Électeur",
                what: "Ce qui s'est passé",
                post: "Poste",
                when: "Quand",
            },
            status: {
                PENDING: "À examiner",
                ACCEPTED: "Approuvée",
                REJECTED: "Rejetée",
            },
            verification: {
                AUTOMATIC: "Automatique",
                MANUAL: "Manuelle",
            },
            time: {
                minutes_one: "{{count}} minute",
                minutes_other: "{{count}} minutes",
                hours_one: "{{count}} heure",
                hours_other: "{{count}} heures",
                days_one: "{{count}} jour",
                days_other: "{{count}} jours",
            },
            summary: {
                join: "{{head}} et {{last}}",
                differs_one: "{{fields}} ne correspond pas au registre",
                differs_other: "{{fields}} ne correspondent pas au registre",
                typedByHand: "Données saisies à la main, non lues sur une pièce d'identité scannée",
                needsFaceToFace: "Nécessite une vérification en face à face",
                scanVerified: "Pièce d'identité scannée et vérifiée",
                noVoter: "Aucun électeur trouvé dans le registre",
                allMatch: "Toutes les données correspondent au registre",
                needsReview: "En attente de la décision d'une personne",
                approvedBy: "Approuvée par {{name}}",
                approvedAuto: "Approuvée automatiquement",
                rejectedBy: "Rejetée par {{name}}",
                rejectedAuto: "Rejetée automatiquement",
            },
            list: {
                title: "Approbations",
                subtitle:
                    "Les inscriptions que les règles ne peuvent pas décider seules attendent ici une personne.",
                search: "Rechercher",
                review: "Examiner l'inscription",
                openRecord: "Ouvrir l'inscription",
                seeRule: "Voir la règle qui a décidé",
                unnamed: "Demandeur sans nom",
                waiting: "En attente depuis {{time}}",
                applied: "Demandée le {{date}}",
                empty: {
                    title: "Rien ici",
                    text: "Les inscriptions ayant ce statut apparaîtront ici. Essayez une autre recherche ou un autre statut.",
                },
            },
            flow: {
                stepsLabel: "Étapes de l'examen",
                steps: {
                    identity: "Vérifier l'identité",
                    voter: "Trouver l'électeur",
                    decide: "Décider",
                },
                continue: "Continuer",
                backToList: "Retour aux Approbations",
                identity: {
                    details: "Données de l'inscription",
                    confirm:
                        "J'ai vérifié la pièce d'identité de l'électeur en personne ou par appel vidéo, et elle correspond à cette inscription.",
                    checked: "Vérification en face à face confirmée",
                    notChecked: "Vérification en face à face pas encore confirmée",
                },
                voter: {
                    none: "Aucun de ceux-ci n'est l'électeur",
                    noneHint:
                        "L'inscription ne peut alors qu'être rejetée, faute d'électeur correspondant.",
                    noneChosen: "Aucun de ceux-ci n'est l'électeur",
                    notChosen: "Aucun électeur choisi pour l'instant",
                },
                decide: {
                    approve: "Approuver",
                    reject: "Rejeter",
                    approveText:
                        "Associe cette inscription à {{voter}} dans le registre. L'électeur est prévenu par e-mail ou SMS et pourra se connecter pour voter à l'ouverture du vote.",
                    rejectText:
                        "Le motif est communiqué à l'électeur. Cette action est irréversible.",
                    chooseVoter: "Choisissez l'électeur correspondant à l'étape 2 pour approuver.",
                    noVoter:
                        "Vous n'avez trouvé aucun électeur correspondant : cette inscription ne peut donc qu'être rejetée.",
                    enrolled: "L'électeur choisi est déjà inscrit.",
                    faceToFace:
                        "Confirmez la vérification en face à face à l'étape 1 pour approuver.",
                },
            },
            review: {
                loadError: "L'inscription n'a pas pu être chargée.",
                applied: "Demandée le {{date}}",
                waiting: "En attente depuis {{time}}",
                whyTitle: "Pourquoi une personne doit intervenir",
                decisionTitle: "Comment la décision a été prise",
                rule: "Règle {{rule}} de la version {{version}} de la matrice",
                ruleLast: "Dernière règle de la version {{version}} de la matrice",
                seeRule: "Voir la règle",
                why: {
                    typedByHand:
                        "L'électeur a saisi ses données à la main au lieu de scanner une pièce d'identité. Ces inscriptions ne sont jamais approuvées automatiquement : un agent confirme d'abord son identité.",
                    differs_one:
                        "Une donnée ne correspond pas au registre : {{details}}. Les règles d'approbation demandent qu'une personne examine cette inscription.",
                    differs_other:
                        "{{count}} données ne correspondent pas au registre : {{details}}. Les règles d'approbation demandent qu'une personne examine cette inscription.",
                    differsFields_one:
                        "Une donnée ne correspond pas au registre : {{fields}}. Les règles d'approbation demandent qu'une personne examine cette inscription.",
                    differsFields_other:
                        "{{count}} données ne correspondent pas au registre : {{fields}}. Les règles d'approbation demandent qu'une personne examine cette inscription.",
                    difference:
                        "pour {{field}}, l'inscription indique “{{enrollment}}” et le registre indique “{{registry}}”",
                    noVoter:
                        "Aucun électeur du registre n'a ces données. Les règles d'approbation demandent qu'une personne examine cette inscription.",
                    severalVoters:
                        "Plusieurs électeurs du registre correspondent à cette inscription. Une personne choisit le bon.",
                    pending:
                        "Les règles d'approbation demandent qu'une personne examine cette inscription.",
                    unknown: "Cette inscription attend la décision d'une personne.",
                    approvedAuto:
                        "Les règles d'approbation ont approuvé cette inscription automatiquement. Toutes les vérifications qu'elles exigent ont réussi.",
                    approvedBy: "{{name}} a approuvé cette inscription le {{date}}.",
                    rejectedAuto:
                        "Les règles d'approbation ont rejeté cette inscription automatiquement : {{reason}}.",
                    rejectedBy: "{{name}} a rejeté cette inscription le {{date}} : {{reason}}.",
                },
                registryHelp:
                    "Nous avons cherché les électeurs ayant les mêmes données : {{fields}}. Choisissez celui à qui appartient cette inscription.",
                registrySearching:
                    "Voici les électeurs du registre qui correspondent à votre recherche. Choisissez celui à qui appartient cette inscription.",
                registrySearch: "Absent de la liste ? Cherchez dans le registre par nom ou e-mail",
                registryLoading: "Recherche dans le registre",
                registryError: "La recherche dans le registre a échoué.",
                noCandidates:
                    "Aucun électeur du registre ne correspond. Essayez de chercher par nom ou e-mail.",
                candidates: "Électeurs du registre",
                alreadyEnrolled: "Déjà inscrit",
                bestMatch: "Meilleure correspondance",
                detailsMatch: "{{count}} données sur {{total}} correspondent",
                compareTitle: "Comparaison avec {{name}} dans le registre",
                col: {
                    detail: "Donnée",
                    enrollment: "Dans l'inscription",
                    registry: "Dans le registre",
                    result: "Résultat",
                },
                same: "Identique",
                differs: "Différent",
                compareNote:
                    "Pour les noms, la casse, les accents et les traits d'union sont ignorés.",
                compareJoint:
                    "Pour les permis de conduire et les livrets de marin, le prénom et le deuxième prénom sont comparés ensemble.",
                applicationId: "ID de la demande",
                copy: "Copier",
                copied: "Copié",
                approve: "Approuver l'inscription",
                approveDialog: {
                    title: "Approuver {{name}} ?",
                    body: "L'inscription sera associée à l'électeur du registre ci-dessous. L'électeur est prévenu par e-mail ou SMS et pourra se connecter pour voter à l'ouverture du vote.",
                    checked: "Vous avez vérifié la pièce d'identité de l'électeur en face à face.",
                    irreversible: "Cette action est irréversible.",
                    confirm: "Approuver",
                },
                reject: "Rejeter l'inscription",
            },
            idCheck: {
                title: "Vérification de la pièce d'identité",
                method: {
                    VERIFIED: "Pièce d'identité scannée et vérifiée",
                    MANUAL_ENTRY: "Saisie à la main",
                    UNKNOWN: "Non indiqué",
                },
                verified: "Le parcours d'inscription a vérifié la pièce d'identité de l'électeur",
                typedByHand: "L'électeur a saisi ses données à la main",
                unknown:
                    "Le parcours d'inscription n'a pas indiqué comment l'identité a été vérifiée",
                faceToFaceTitle: "Vérifiez son identité en face à face avant d'approuver",
                faceToFaceText:
                    "Rencontrez l'électeur en personne ou par appel vidéo et comparez sa pièce d'identité avec les données de cette page.",
            },
            reject: {
                rejectReason: "Motif du rejet",
                message: "Message à l'électeur",
                messageRequired: "Écrivez un message pour l'électeur lorsque le motif est Autre.",
                reasons: {
                    "undefined": "-",
                    "insufficient-information": "Données manquantes",
                    "no-matching-voter": "Aucun électeur correspondant",
                    "voter-already-approved": "Déjà approuvé",
                    "other": "Autre",
                },
                hint: {
                    "insufficient-information": "Des données manquent ou sont illisibles.",
                    "no-matching-voter":
                        "La personne ne figure pas dans le registre des électeurs.",
                    "voter-already-approved": "Cet électeur est déjà inscrit.",
                    "other": "Écrivez votre propre message.",
                },
                preview: {
                    "insufficient-information":
                        "Nous n'avons pas pu vous inscrire, car certaines de vos données manquent ou sont illisibles. Veuillez vous inscrire à nouveau avec des données complètes.",
                    "no-matching-voter":
                        "Nous n'avons trouvé dans le registre aucun électeur correspondant à vos données. Vérifiez vos données et inscrivez-vous à nouveau, ou contactez votre bureau électoral.",
                    "voter-already-approved":
                        "Vous êtes déjà inscrit. Vous pourrez vous connecter pour voter à l'ouverture du vote.",
                },
                previewTitle: "L'électeur verra",
            },
            notifications: {
                approveError: "L'inscription n'a pas pu être approuvée",
                approveSuccess: "Inscription de {{name}} approuvée. L'électeur a été prévenu.",
                rejectError: "L'inscription n'a pas pu être rejetée",
                rejectSuccess: "Inscription de {{name}} rejetée. L'électeur a été prévenu.",
                VoterApprovedAlready: "Cet électeur est déjà inscrit.",
            },
            export: {
                success: "L'exportation des demandes s'est terminée avec succès",
                error: "Erreur lors de l'exportation des demandes",
            },
            matrix: {
                button: "Matrice d'approbation",
                title: "Matrice d'approbation",
                back: "Approbations",
                subtitle:
                    "Les règles décident du sort de chaque inscription. La première règle qui s'applique décide.",
                versionChip: "Version {{version}}",
                savedBy: "Enregistrée le {{date}} par {{user}}",
                builtIn: "Règles intégrées, utilisées jusqu'à l'enregistrement d'une version",
                unsaved: "Modifications non enregistrées",
                viewOnly: "Lecture seule",
                readOnlyTitle: "Vous pouvez voir les règles, mais pas les modifier",
                readOnlyText:
                    "Demandez à un administrateur disposant de la permission approval-matrix-write de faire les modifications.",
                loadError: "La matrice d'approbation n'a pas pu être chargée.",
                compared: "Ce que nous comparons",
                comparedHelp:
                    "Chaque inscription est comparée avec l'électeur trouvé dans le registre. Pour les noms, la casse, les accents et les traits d'union sont ignorés ; pour les permis de conduire et les livrets de marin, le prénom et le deuxième prénom sont comparés ensemble.",
                addCompared: "Comparer une autre donnée",
                rules: "Règles",
                rulesHelp:
                    "Les règles sont vérifiées à partir du haut. La première qui s'applique décide ; si aucune ne s'applique, la dernière règle s'applique.",
                when: "Quand",
                then: "Alors",
                otherwise: "Sinon",
                noneApply: "Aucune des règles ci-dessus ne s'applique",
                andWord: "et",
                and: " et ",
                appliesToExample: "S'applique à votre exemple",
                cameFrom: "A décidé l'inscription d'où vous venez",
                voterIsTold: "L'électeur reçoit ce message : “{{reason}}”.",
                sentence: "Quand {{when}}, {{outcome}}.",
                sentenceOtherwise: "Si aucune des règles ci-dessus ne s'applique, {{outcome}}.",
                sentenceEmpty: "Ajoutez une condition pour indiquer quand cette règle s'applique.",
                addRule: "Ajouter une règle",
                discard: "Annuler les modifications",
                actions: {
                    edit: "Modifier la règle {{number}}",
                    editOtherwise: "Modifier la dernière règle",
                    moveUp: "Monter la règle {{number}}",
                    moveDown: "Descendre la règle {{number}}",
                    delete: "Supprimer la règle {{number}}",
                },
                saveBar: {
                    title: "Vous avez des modifications non enregistrées",
                    fix_one: "Corrigez 1 règle avant d'enregistrer",
                    fix_other: "Corrigez {{count}} règles avant d'enregistrer",
                    more: "+{{count}} de plus",
                },
                test: "Essayer un exemple",
                testHelp:
                    "Décrivez une inscription pour voir quelle règle en décide. Vos modifications non enregistrées comptent.",
                testDetails: "Données comparées",
                applies: "La règle {{number}} s'applique",
                otherwiseApplies: "La dernière règle s'applique",
                testError: "L'exemple n'a pas pu être essayé.",
                testInvalid: "Corrigez ces règles pour essayer un exemple :",
                ruleError: "Règle {{number}} : {{error}}",
                invariants: {
                    MANUAL_ENTRY_NOT_ACCEPTED:
                        "Une identité saisie à la main n'est jamais approuvée automatiquement : ce cas est donc envoyé à une personne.",
                    ALREADY_ENROLLED_NOT_ACCEPTED:
                        "Un électeur déjà inscrit n'est jamais approuvé de nouveau.",
                    NO_VOTER_NOT_ACCEPTED:
                        "Personne n'est approuvé sans électeur dans le registre.",
                    OTHERWISE_NOT_ACCEPTED: "La dernière règle n'approuve jamais.",
                },
                dialog: {
                    editTitle: "Modifier la règle {{number}}",
                    newTitle: "Nouvelle règle",
                    otherwiseTitle: "Modifier la dernière règle",
                    summary: "En bref",
                    whenHelp:
                        "Toutes doivent être vraies. Laissez une condition de côté quand elle n'a pas d'importance.",
                    otherwiseHelp: "Si aucune des règles ci-dessus ne s'applique",
                    addCondition: "Ajouter une condition",
                    remove: "Retirer “{{condition}}”",
                    identity: "Vérification d'identité",
                    voterFound: "Électeur dans le registre",
                    alreadyEnrolled: "Déjà inscrit",
                    validId: "Type de pièce d'identité",
                    differing: "Données qui diffèrent",
                    decision: "Décision",
                    reason: "Ce qui est dit à l'électeur",
                    voterSees: "L'électeur voit",
                    apply: "Appliquer",
                    close: "Fermer",
                    yes: "Oui",
                    no: "Non",
                    notReported: "Non indiqué",
                },
                identity: {
                    VERIFIED: "Vérifiée par pièce d'identité scannée",
                    MANUAL_ENTRY: "Saisie à la main",
                },
                differing: {
                    none: "Aucune",
                    exactly_1: "Exactement 1",
                    at_most_1: "Au plus 1",
                    exactly_2: "Exactement 2",
                    at_most_2: "Au plus 2",
                    at_least_3: "3 ou plus",
                },
                fieldMatch: {
                    MATCHES: "Identique",
                    DIFFERS: "Différent",
                },
                decisions: {
                    ACCEPTED: "Approuver automatiquement",
                    PENDING: "Envoyer à une personne",
                    REJECTED: "Rejeter",
                },
                outcomeShort: {
                    ACCEPTED: "approuver automatiquement",
                    PENDING: "envoyer à une personne",
                    REJECTED: "rejeter",
                },
                outcomeHelp: {
                    ACCEPTED: "L'électeur est inscrit sans que personne n'examine la demande.",
                    PENDING:
                        "Un agent décide, et l'électeur est informé que son inscription est en cours d'examen.",
                    REJECTED:
                        "Le motif est communiqué à l'électeur, qui peut s'inscrire à nouveau.",
                },
                outcomeSentence: {
                    ACCEPTED: "l'inscription est approuvée automatiquement",
                    PENDING: "l'inscription est envoyée à une personne",
                    REJECTED: "l'inscription est rejetée",
                },
                reasons: {
                    NO_VOTER: "Aucun électeur correspondant",
                    ALREADY_APPROVED: "Déjà approuvé",
                    INSUFFICIENT_INFORMATION: "Données manquantes",
                    IDENTITY_NOT_VERIFIED: "Identité non vérifiée",
                    OTHER: "Autre",
                },
                voterText: {
                    NO_VOTER:
                        "Nous n'avons trouvé dans le registre aucun électeur correspondant à vos données. Vérifiez vos données et inscrivez-vous à nouveau, ou contactez votre bureau électoral.",
                    ALREADY_APPROVED:
                        "Vous êtes déjà inscrit. Vous pourrez vous connecter pour voter à l'ouverture du vote.",
                    INSUFFICIENT_INFORMATION:
                        "Nous n'avons pas pu vous inscrire, car certaines de vos données manquent ou sont illisibles. Veuillez vous inscrire à nouveau avec des données complètes.",
                    IDENTITY_NOT_VERIFIED:
                        "Nous n'avons pas pu vérifier votre identité automatiquement : un agent électoral examinera donc votre inscription.",
                    OTHER: "Un agent électoral écrit ce message au moment de décider.",
                },
                conditions: {
                    any: "Aucune condition pour l'instant",
                    identity: {
                        VERIFIED: "Identité vérifiée par pièce d'identité scannée",
                        MANUAL_ENTRY: "Identité saisie à la main",
                    },
                    voterFound: {
                        true: "Électeur trouvé dans le registre",
                        false: "Aucun électeur trouvé dans le registre",
                    },
                    alreadyEnrolled: {
                        true: "Déjà inscrit",
                        false: "Pas encore inscrit",
                    },
                    validId: "Pièce d'identité : {{id}}",
                    differing: {
                        none: "Toutes les données correspondent",
                        exactly_1: "Exactement 1 donnée diffère",
                        at_most_1: "Au plus 1 donnée diffère",
                        exactly_2: "Exactement 2 données diffèrent",
                        at_most_2: "Au plus 2 données diffèrent",
                        at_least_3: "3 données ou plus diffèrent",
                    },
                    field: {
                        MATCHES: "{{field}} correspond",
                        DIFFERS: "{{field}} diffère",
                    },
                },
                errors: {
                    ACCEPTS_MANUAL_ENTRY:
                        "Les inscriptions dont l'identité a été saisie à la main ne peuvent pas être approuvées automatiquement.",
                    ACCEPTS_ALREADY_ENROLLED:
                        "Un électeur déjà inscrit ne peut pas être approuvé de nouveau.",
                    ACCEPTS_WITHOUT_VOTER:
                        "Une inscription ne peut pas être approuvée sans électeur dans le registre.",
                    OTHERWISE_ACCEPTS:
                        "La dernière règle peut envoyer les inscriptions à une personne ou les rejeter, mais pas les approuver.",
                    MISSING_REASON: "Choisissez ce qui est dit à l'électeur.",
                    UNEXPECTED_REASON: "Une approbation n'a pas de motif.",
                    NO_COMPARED_FIELDS:
                        "Choisissez au moins une donnée à comparer avec le registre.",
                    DUPLICATE_COMPARED_FIELD: "Une donnée comparée est répétée.",
                    UNKNOWN_FIELD: "Une règle utilise une donnée qui n'est pas comparée.",
                    NO_CONDITIONS:
                        "Ajoutez au moins une condition. Seule la dernière règle s'applique à tout le reste.",
                },
                change: {
                    added: "Règle {{number}} ajoutée",
                    decision: "Règle {{number}} : {{from}} → {{to}}",
                    edited: "Règle {{number}} modifiée",
                    removed: "Une règle a été supprimée ({{text}})",
                    moved: "Les règles ont été réordonnées",
                    otherwise: "La dernière règle a changé",
                    compared: "Les données comparées ont changé",
                },
                save: {
                    button: "Enregistrer comme version {{version}}",
                    title: "Enregistrer comme version {{version}} ?",
                    body: "Les nouvelles inscriptions sont désormais décidées avec ces règles. Les inscriptions déjà décidées conservent leur décision.",
                    changes: "Ce qui a changé",
                    log: "La nouvelle version est consignée dans le journal électoral.",
                    confirm: "Enregistrer la version {{version}}",
                    success: "Enregistrée comme version {{version}}",
                    error: "La matrice d'approbation n'a pas pu être enregistrée",
                },
            },
        },
        monitoring: {
            title: "Suivi",
            loading: "Chargement du suivi",
            unavailableAlert:
                "Les tableaux de bord de suivi n'ont pas pu être chargés ; le tableau de bord standard est affiché.",
            noDashboards: "Cet événement n'a aucun tableau de bord de suivi.",
            dashboardFailed: "Le tableau de bord de suivi n'a pas pu être chargé.",
            dashboardInvalid: "Ce tableau de bord ne peut pas être affiché : {{problem}}",
            retry: "Réessayer",
            errors: {
                busy: "Le serveur est occupé. Nouvel essai dans quelques secondes.",
                forbiddenScope:
                    "Vous ne pouvez pas voir cette région, ce Post ou ce pays. Choisissez-en un autre.",
                snapshotPruned:
                    "La mise à jour affichée n'est plus conservée. Le tableau de bord affiche maintenant la dernière mise à jour : exportez de nouveau pour l'utiliser.",
                checksUnavailable:
                    "Le service de graphiques n'est pas disponible pour le moment. Réessayez plus tard.",
                lockedDown: "L'événement électoral est verrouillé : ceci ne peut pas être modifié.",
                notFound:
                    "Ce tableau de bord ou ce widget n'est plus configuré. Rechargez la page.",
                badRequest: "La demande n'a pas été acceptée. Rechargez la page et réessayez.",
                conflict:
                    "Quelqu'un d'autre a enregistré une modification avant vous. Rechargez et réessayez.",
                invalid: "Certaines valeurs de l'export ne sont pas acceptées.",
                unknown: "Une erreur s'est produite. Réessayez plus tard.",
            },
            header: {
                dashboard: "Tableau de bord",
                updated: "Mis à jour {{time}} ({{timeZone}})",
                notUpdated: "Pas encore compté",
                refresh: "toutes les {{seconds}} s",
                export: "Exporter",
                editDashboard: "Modifier le tableau de bord",
                preset: "Préréglage de tableaux de bord",
                reload: "Chercher de nouveaux chiffres",
            },
            footer: {
                dataThrough: "Données jusqu'au {{time}} ({{timeZone}})",
            },
            selectors: {
                region: "Région",
                post: "Poste",
                country: "Pays",
                allRegions: "Toutes les régions",
                allPosts: "Tous les postes",
                allAuthorizedPosts: "Tous les postes autorisés",
                allCountries: "Tous les pays",
                authorizedOnly: "{{all}} (autorisés)",
            },
            widget: {
                menu: "Actions pour {{widget}}",
                configure: "Configurer le widget",
                viewData: "Voir les données",
                export: "Exporter",
                duplicate: "Dupliquer",
                loading: "Chargement de {{widget}}",
                missing: "Le tableau de bord nomme un widget qui n'existe pas : {{id}}",
                updating: "Mise à jour de {{widget}}",
                updatingNote: "Mise à jour : le graphique affiché est le précédent.",
            },
            frame: {
                title: "Graphique {{widget}}",
            },
            sources: {
                voter_turnout: "Participation",
                test_voting: "Vote de test",
                enrollment_decisions: "Décisions d'inscription",
                voting_credentials: "Identifiants de vote",
                poll_status: "État du scrutin",
                final_testing_lockdown: "Tests finaux et verrouillage",
                counting_transmission: "Dépouillement et transmission",
                voting_enrollment_activity: "Activité de vote et d'inscription",
                access_security: "Accès et sécurité",
                attack_detections: "Détection d'attaques",
                helpdesk: "Assistance",
            },
            reasons: {
                TEST_ELECTION_DESIGNATION:
                    "les élections de test ne peuvent pas encore être signalées",
                CREDENTIAL_ISSUED_EVENT: "l'émission des identifiants n'est pas encore enregistrée",
                FINAL_TESTING_LOCKDOWN_STATE:
                    "les tests finaux et le verrouillage ne sont pas encore enregistrés",
                ATTACK_DETECTION_FEED: "aucun flux de détection d'attaques n'est connecté",
                HELPDESK_INTEGRATION: "aucun système d'assistance n'est connecté",
            },
            notices: {
                UNREGISTERED_ATTEMPTS_AT_EVENT_SCOPE_ONLY:
                    "Les tentatives avec des noms d'utilisateur non enregistrés n'appartiennent à aucun poste ; elles ne sont donc comptées que pour l'ensemble de l'événement.",
                UNREGISTERED_ATTEMPTS_EXCLUDED:
                    "Ces chiffres excluent les tentatives avec des noms d'utilisateur non enregistrés ; elles sont comptées pour l'ensemble de l'événement.",
                CREDENTIALS_ISSUED_WHEN_PASSWORD_SET:
                    "Les identifiants comptent comme délivrés lorsque le mot de passe de l'électeur est défini, jusqu'à ce que la plateforme enregistre leur délivrance.",
                CONFIG_NEWER_THAN_SNAPSHOT:
                    "Dessiné avec les derniers réglages ; les chiffres sont comptés avec eux au prochain passage.",
                CONFIG_AT_SNAPSHOT_UNAVAILABLE:
                    "Dessiné avec les réglages actuels : ceux avec lesquels les chiffres ont été comptés ne sont plus conservés.",
            },
            unavailable: {
                notConnected: "Non connecté · {{reason}}",
                notConnectedHelp: "Rien n'est affiché tant que ce n'est pas le cas.",
                unknownReason: "source de données indisponible",
                noSnapshot: "Pas encore compté",
                noSnapshotHelp:
                    "Le premier comptage n'est pas terminé. Ce widget se met à jour tout seul.",
                scopePending: "Comptage de cette sélection",
                scopePendingHelp:
                    "Cette sélection est comptée au prochain passage, dans environ une minute.",
                settingsPending: "Comptage avec les nouveaux paramètres…",
                settingsPendingHelp:
                    "Les chiffres sont recomptés avec les paramètres enregistrés, en une minute environ.",
                renderFailed: "Le graphique n'a pas pu être dessiné",
                renderFailedHelp: "Les chiffres sont affichés à la place.",
                invalid: "Ce widget ne peut pas être affiché",
                invalidHelp: "Sa configuration pose un problème.",
                requestFailed: "Ce widget n'a pas pu être chargé",
                requestFailedHelp: "Il est réessayé à la prochaine mise à jour du tableau de bord.",
            },
            dataTable: {
                title: "{{widget}} · données",
                close: "Fermer",
                empty: "Aucune ligne",
                rowsPerPage: "Lignes par page :",
                shownRows: "{{from}}–{{to}} sur {{total}}",
                firstPage: "Première page",
                previousPage: "Page précédente",
                nextPage: "Page suivante",
                lastPage: "Dernière page",
            },
            columns: {
                numerator: "Numérateur",
                denominator: "Dénominateur",
                pct: "Pourcentage",
                pct_label: "Pourcentage affiché",
                group: "Groupe",
                group_key: "Clé du groupe",
                post: "Poste",
                post_id: "ID du poste",
                region: "Région",
                country: "Pays",
                reason: "Motif",
                category: "Catégorie",
                state: "État",
                state_label: "État affiché",
                bucket_start: "Début de la période",
                bucket_label: "Période",
                bucket_utc: "Début de la période (UTC)",
                measure: "Mesure",
                label: "Libellé",
                value: "Valeur",
            },
            measures: {
                registered: "Inscrits",
                pre_enrolled: "Préinscrits",
                credentials_issued: "Identifiants délivrés",
                test_voted: "Votes de test",
                voted: "Ont voté",
                voted_pre_enrolled: "Pré-inscrits ayant voté",
                applications: "Demandes",
                pending: "En attente",
                approved: "Approuvées",
                disapproved: "Refusées",
                posts: "Postes",
                initialized: "Initialisés",
                opened: "Ouverts",
                paused: "En pause",
                closed: "Fermés",
                tested: "Testés",
                locked_down: "Verrouillés",
                tallied: "Dépouillés",
                transmitted: "Transmis",
                transmission_failed: "Échec de transmission",
                logins: "Connexions",
                login_failures: "Échecs de connexion",
                login_failures_valid_user: "Échecs, utilisateur valide",
                login_failures_unregistered: "Échecs, utilisateur non inscrit",
                password_resets: "Réinitialisations de mot de passe",
                password_reset_requests: "Demandes de réinitialisation du mot de passe",
                detections: "Détections",
                issues: "Incidents",
                pending_issues: "Incidents en attente",
            },
            export: {
                title: "Exporter les données de suivi",
                format: "Format",
                csv: "CSV",
                sql: "SQL",
                from: "Du",
                to: "Au",
                timeZoneHelp:
                    "Les heures sont en {{timeZone}}. Les totaux, les états et les groupes sont ceux de la mise à jour affichée ; seules les lignes des séries d'activité sont limitées à la période, de l'heure de début jusqu'à l'heure de fin, non comprise.",
                cancel: "Annuler",
                export: "Exporter",
                invalidRange: "La fin doit être postérieure au début.",
                problems: {
                    unknownSelector:
                        "{{widget}} n'a plus le choix « {{selector}} ». Rechargez le tableau de bord et exportez à nouveau.",
                    unknownOption:
                        "La valeur choisie pour « {{selector}} » dans {{widget}} n'est plus proposée. Choisissez-la à nouveau et exportez.",
                },
            },
            editor: {
                scopeSelector: {
                    region: "Région",
                    post: "Poste",
                    country: "Pays",
                },
                sources: {
                    voter_turnout: "Participation des électeurs",
                    test_voting: "Vote de test",
                    enrollment_decisions: "Décisions d'inscription",
                    voting_credentials: "Identifiants de vote",
                    poll_status: "État du scrutin",
                    final_testing_lockdown: "Tests finaux et verrouillage",
                    counting_transmission: "Dépouillement et transmission",
                    voting_enrollment_activity: "Activité de vote et d'inscription",
                    access_security: "Accès et sécurité",
                    attack_detections: "Détections d'attaques",
                    helpdesk: "Assistance",
                },
                templates: {
                    summary: "Résumé",
                    by_group: "Par groupe",
                    by_post: "Par poste",
                    timeseries: "Dans le temps",
                    by_measure: "Par mesure",
                },
                yaml: {
                    label: "YAML",
                    readOnly:
                        "Le YAML contient une erreur de syntaxe. Corrigez-la dans l'onglet YAML pour réutiliser les formulaires.",
                },
                diagnostics: {
                    title: "Vérifications",
                    none: "Aucun problème détecté",
                    localUnavailable:
                        "Les vérifications dans le navigateur sont indisponibles ; les problèmes apparaissent après l'aperçu ou Valider.",
                    line: "Ligne {{line}}",
                    engineCode: "dbt Charts {{code}}",
                    severity: {
                        ERROR: "Erreur",
                        WARNING: "Avertissement",
                    },
                    origin: {
                        SYNTAX: "Syntaxe YAML",
                        LOCAL: "Vérification du navigateur",
                        SERVER: "Vérification du serveur",
                    },
                },
                footer: {
                    preview: "Aperçu",
                    valid: "Valide",
                    errors_one: "{{count}} erreur",
                    errors_other: "{{count}} erreurs",
                    noWarnings: "aucun avertissement de graphique",
                    warnings_one: "{{count}} avertissement de graphique",
                    warnings_other: "{{count}} avertissements de graphiques",
                    rendering: "Génération…",
                    previewFailed: "échec de l'aperçu",
                    renderedIn: "généré en {{ms}} ms",
                    revision: "Révision {{revision}}",
                    savedBy: "enregistré {{date}} par {{user}}",
                    unknownUser: "un administrateur",
                    notSaved: "pas encore enregistré",
                    unsaved: "modifications non enregistrées",
                    cancel: "Annuler",
                    validate: "Valider",
                },
                preview: {
                    title: "Aperçu",
                    empty: "L'aperçu apparaît dès que le YAML est valide.",
                    failed: "L'aperçu n'a pas pu être généré : {{reason}}",
                    notConnectedTail: "Rien n'est affiché tant qu'il ne l'est pas.",
                    queryResult: "Résultat de la requête · premières lignes",
                    noRows: "La requête n'a renvoyé aucune ligne.",
                    state: {
                        RENDERED: "Généré",
                        NOT_CONNECTED: "Non connecté",
                        NO_SNAPSHOT: "Pas encore de données",
                        SCOPE_PENDING: "Ce périmètre est en cours de comptage",
                        RENDER_FAILED: "Le graphique n'a pas pu être dessiné",
                        INVALID: "La configuration n'est pas valide",
                    },
                },
                configureWidget: {
                    title: "Configurer le widget",
                    tabs: {
                        dataQuery: "Données et requête",
                        selectors: "Sélecteurs",
                        yaml: "YAML",
                        preview: "Aperçu",
                    },
                    save: "Enregistrer le widget",
                    loading: "Chargement du widget…",
                    loadFailed: "Le widget n'a pas pu être chargé : {{reason}}",
                    saved: "Widget enregistré en tant que révision {{revision}}",
                    refused: "Le widget n'a pas été enregistré : corrigez les problèmes indiqués.",
                    validated: "Le widget est valide.",
                    invalid: "Le widget présente des problèmes : consultez les vérifications.",
                    requestFailed: "La requête a échoué : {{reason}}",
                    discardTitle: "Abandonner les modifications ?",
                    discardBody: "Vos modifications de ce widget n'ont pas été enregistrées.",
                    discard: "Abandonner",
                    keepEditing: "Continuer à modifier",
                },
                dataQuery: {
                    title: "Titre",
                    source: "Source de données",
                    template: "Requête",
                    measures: "Mesures",
                    ratio: "Numérateur / dénominateur",
                    numerator: "Numérateur",
                    denominator: "Dénominateur",
                    groupBy: "Grouper par",
                    sort: "Trier",
                    sortOrder: "Ordre",
                    templateOrder: "L'ordre propre à la requête",
                    sortBy: {
                        label: "Libellé",
                        value: "Valeur",
                        ratio: "Ratio",
                    },
                    order: {
                        asc: "Croissant",
                        desc: "Décroissant",
                    },
                    limit: "Lignes",
                    noLimit: "Toutes",
                    none: "Aucun",
                    fromSelector: "Depuis le sélecteur : {{name}}",
                    follow: "Suivre les sélecteurs du tableau de bord",
                    followHelp: "Un sélecteur non coché ne restreint pas ce widget.",
                    manyQueries:
                        "Ce widget comporte plusieurs requêtes ; modifiez-les dans l'onglet YAML.",
                    notConnected: "Non connecté · {{reason}}",
                    sourceHelp: {
                        DISTINCT_VOTERS:
                            "Compte les électeurs distincts ; les règles de comptage sont fixées par la source de données.",
                        DISTINCT_PRE_ENROLLED_VOTERS:
                            "Compte les électeurs préinscrits distincts ; les règles de comptage sont fixées par la source de données.",
                        LATEST_DECISION_PER_VOTER:
                            "Compte la dernière décision par électeur ; les règles de comptage sont fixées par la source de données.",
                        APPROVED_VOTERS:
                            "Compte les électeurs approuvés ; les règles de comptage sont fixées par la source de données.",
                        POSTS_IN_SCOPE:
                            "Compte les postes du périmètre ; les règles de comptage sont fixées par la source de données.",
                        FIRST_EVENT_PER_VOTER:
                            "Compte le premier vote ou la première approbation valide de chaque électeur ; les règles de comptage sont fixées par la source de données.",
                        ATTEMPTS:
                            "Compte les tentatives, pas les personnes ; les règles de comptage sont fixées par la source de données.",
                        DETECTIONS:
                            "Compte les détections ; les règles de comptage sont fixées par la source de données.",
                        REPORTED_ISSUES:
                            "Compte les problèmes signalés ; les règles de comptage sont fixées par la source de données.",
                        default: "Les règles de comptage sont fixées par la source de données.",
                    },
                },
                selectors: {
                    help: "Les sélecteurs apparaissent dans l'en-tête du widget. Leurs valeurs alimentent la requête ; les sélecteurs du tableau de bord (Région, Poste, Pays) s'appliquent à tous les widgets.",
                    name: "Nom",
                    label: "Libellé",
                    control: "Contrôle",
                    controls: {
                        dropdown: "Liste déroulante",
                        toggle: "Interrupteur",
                    },
                    default: "Par défaut",
                    optionValue: "Valeur",
                    optionLabel: "Libellé de l'option",
                    addOption: "Ajouter une option",
                    addSelector: "Ajouter un sélecteur",
                    removeOption: "Supprimer l'option {{option}}",
                    removeSelector: "Supprimer le sélecteur {{name}}",
                    moveUp: "Monter {{name}}",
                    moveDown: "Descendre {{name}}",
                    dynamic: "Les options proviennent des données ({{source}}).",
                    none: "Ce widget n'a aucun sélecteur.",
                    newLabel: "Nouveau sélecteur",
                    newOption: "Nouvelle option",
                    shownWhen: "Affiché lorsque {{selector}} vaut {{values}}",
                },
                conflict: {
                    title: "Quelqu'un a enregistré avant vous",
                    body: "{{user}} a enregistré la révision {{revision}} ({{date}}) pendant que vous modifiiez.",
                    bodyShort:
                        "La révision {{revision}} a été enregistrée pendant que vous modifiiez.",
                    saved: "Révision enregistrée",
                    mine: "Mes modifications",
                    reload: "Recharger",
                    copy: "Copier mon YAML",
                    copied: "Votre YAML est dans le presse-papiers.",
                    copyFailed:
                        "Le presse-papiers est indisponible ; sélectionnez le YAML et copiez-le à la main.",
                    keepEditing: "Continuer à modifier",
                    removed:
                        "Le document a été supprimé pendant que vous le modifiiez. Continuez à modifier pour l'enregistrer à nouveau.",
                },
                dashboard: {
                    editing: "Modification du tableau de bord",
                    title: "Titre",
                    selectors: "Sélecteurs du tableau de bord",
                    theme: "Thème",
                    editTheme: "Modifier le thème",
                    addWidget: "Ajouter un widget",
                    cancel: "Annuler",
                    save: "Enregistrer le tableau de bord",
                    width: "Largeur",
                    widthValue: "{{n}} sur 12",
                    moveUp: "Monter",
                    moveDown: "Descendre",
                    remove: "Supprimer",
                    duplicate: "Dupliquer",
                    configure: "Configurer le widget",
                    empty: "Ce tableau de bord ne contient pas encore de widgets.",
                    saved: "Tableau de bord enregistré en tant que révision {{revision}}",
                    refused:
                        "Le tableau de bord n'a pas été enregistré : corrigez les problèmes indiqués.",
                    requestFailed: "La requête a échoué : {{reason}}",
                    dragHandle: "Faire glisser pour réorganiser {{title}}",
                    widgets: "Widgets",
                    duplicated: "Dupliqué sous {{id}}",
                    resetToPreset: "Rétablir le préréglage",
                    actions: "Actions pour {{title}}",
                    discardBody:
                        "Vos modifications de ce tableau de bord n'ont pas été enregistrées.",
                    duplicateInvalid: "La copie n'a pas été enregistrée : {{problem}}",
                    layoutMalformed:
                        "Certains éléments de la disposition ne sont pas un widget avec une largeur. Corrigez-les dans l'onglet YAML pour réordonner les widgets.",
                },
                catalog: {
                    title: "Ajouter un widget",
                    search: "Rechercher des widgets, des sources de données ou des exigences",
                    add: "Ajouter",
                    empty: "Aucun widget ne correspond.",
                    onDashboard: "Sur ce tableau de bord",
                    close: "Fermer",
                },
                theme: {
                    title: "Thème du tableau de bord",
                    subtitle: "style dbt Charts appliqué à tous les widgets de ce tableau de bord",
                    appliesTo_one: "s'applique à {{count}} widget",
                    appliesTo_other: "s'applique à {{count}} widgets",
                    apply: "Appliquer le thème",
                    saved: "Thème enregistré en tant que révision {{revision}}",
                    discardBody: "Vos modifications de ce thème n'ont pas été enregistrées.",
                },
                reset: {
                    title: "Rétablir le préréglage",
                    body: "Tous les tableaux de bord, widgets et thèmes de cet événement sont remplacés par ceux du préréglage. La configuration actuelle reste dans l'historique.",
                    preset: "Préréglage",
                    confirm: "Rétablir",
                    cancel: "Annuler",
                    done: "L'événement utilise désormais {{title}}.",
                    failed: "Le rétablissement a échoué : {{reason}}",
                    noPresets: "Aucun préréglage n'est disponible.",
                    loading: "Chargement des préréglages…",
                },
                lockedDown:
                    "L'événement est verrouillé ; sa configuration de supervision ne peut pas être modifiée.",
                document: {
                    loadFailed: "Le document n'a pas pu être chargé : {{reason}}",
                    refused: "Non enregistré : corrigez les problèmes indiqués.",
                    validated: "Le document est valide.",
                    invalid: "Le document comporte des problèmes : voir les vérifications.",
                    requestFailed: "La requête a échoué : {{reason}}",
                    savedWithWarnings_one: "{{count}} avertissement : voir les vérifications.",
                    savedWithWarnings_other: "{{count}} avertissements : voir les vérifications.",
                },
                errors: {
                    checksUnavailable:
                        "Le moteur de graphiques n'a pas pu vérifier la modification, qui n'a donc pas été enregistrée. Réessayez dans un instant.",
                    busy: "D'autres modifications de cet événement sont en cours d'enregistrement. Réessayez dans un instant.",
                    lockedDown:
                        "L'événement est verrouillé ; sa configuration de supervision ne peut pas changer.",
                    forbiddenScope:
                        "Vous ne pouvez pas voir les chiffres de la région, du poste ou du pays choisis.",
                    badRequest:
                        "L'éditeur a envoyé une demande que le serveur n'a pas pu lire. Rechargez la page et réessayez.",
                },
                duplicate: {
                    copyTitle: "{{title}} (copie)",
                    done: "{{id}}, une copie du widget, a été ajouté au tableau de bord.",
                    failed: "Le widget n'a pas pu être dupliqué : {{reason}}",
                    notPlaced:
                        "La copie {{id}} a été enregistrée, mais le tableau de bord ne l'a pas prise ({{reason}}). Ajoutez-la avec Modifier le tableau de bord.",
                },
            },
        },
        monitoringDashboardScreen: {
            voters: {
                title: "Électeurs",
                enrolledOverseasVoters: "Électeurs Inscrits à l'Étranger",
                approvalStatus: "Statut d'Approbation: Électeurs Approuvés/Désapprouvés",
                manuallyApproval: "Électeurs Approuvés/Désapprouvés Manuellement",
                automaticallyApproval: "Électeurs Approuvés/Désapprouvés Automatiquement",
                authenticatedVoters: "Électeurs Authentifiés",
                invalidUserErrors: "Erreurs d'Utilisateur Invalide:",
                invalidPasswordErrors: "Erreurs de Mot de Passe Invalide:",
            },
            polls: {
                title: "Sondages",
                initializedSystems: "Publications avec Systèmes Initialisés",
                votingOpened: "Publications avec Votations Ouvertes",
                votingClosed: "Publications avec Votations Closes",
                votingStarted: "Publications avec Votations Commencées",
                voterTurnout: "Taux de Participation des Électeurs",
            },
            tally: {
                title: "Comptage",
                activeVotesCounting: "Publications avec Comptage des Votes Actif",
                generatedERs: "Publications avec ERs Générés",
                transmittedResults: "Publications avec Résultats Transmis",
            },
            testing: {
                title: "Tests",
                testElectionVoterCount: "Comptage des Votants lors de l'Élection de Test",
            },
        },
        certificateAuthorities: {
            title: "Certificats",
            subtitle:
                "Autorités de certification (CA) de confiance pour cet événement électoral. Les CA importées sont utilisées pour valider les certificats des électeurs.",
            importButton: "Importer des certificats",
            type: {
                root: "Racine",
                intermediate: "Intermédiaire",
            },
            expiry: {
                expired: "Expiré",
                expiringSoon: "Expire bientôt",
                valid: "Valide",
            },
            columns: {
                commonName: "Nom commun",
                type: "Type",
                issuerCn: "CN de l'émetteur",
                notBefore: "Valide à partir de",
                notAfter: "Expire le",
                fingerprint: "Empreinte SHA256",
            },
            importDialog: {
                title: "Importer des autorités de certification",
                subtitle: "Importer un ou plusieurs certificats CA depuis un fichier PEM",
                description:
                    "Sélectionnez un fichier PEM contenant un ou plusieurs certificats. Les paquets sont pris en charge — chaque certificat est importé individuellement.",
                selectFile: "Sélectionner un fichier PEM",
                fileLoaded: "Fichier chargé ({{bytes}} octets)",
                importButton: "Importer",
            },
            notify: {
                importSuccess: "{{inserted}} certificat(s) importé(s).",
                importSkipped: "{{count}} ignoré(s) (déjà présent(s)).",
                importErrors: "Problèmes d'importation : {{errors}}",
                importError: "Échec de l'importation : {{error}}",
                deleteSuccess: "Certificat supprimé.",
                deleteError: "Erreur lors de la suppression du certificat.",
                exportSuccess: "Certificat(s) exporté(s) avec succès.",
                exportError: "Erreur lors de l'exportation des certificats.",
            },
            exportDialog: {
                title: "Exporter les autorités de certification",
                description: "Vous êtes sur le point d'exporter {{amount}} certificat(s).",
                all: "tous",
            },
            deleteDialog: {
                description: "Êtes-vous sûr de vouloir supprimer {{count}} certificat(s) ?",
            },
            emptyHeader:
                "Aucune autorité de certification n'a été importée pour cet événement électoral.",
            fileReadError: "Échec de la lecture du fichier.",
            viewDialog: {
                title: "Détails de l'autorité de certification",
                subject: "Sujet",
                issuer: "Émetteur",
                serialNumber: "Numéro de série",
                pemContent: "Contenu PEM",
            },
            confirmDelete: "Supprimer l'autorité de certification",
            confirmDeleteDescription:
                'Êtes-vous sûr de vouloir supprimer le certificat "{{name}}" (empreinte : {{fingerprint}}) ?',
        },
        signing: {
            terms: {
                post: "Poste",
                posts: "Postes",
            },
            tab: {
                title: "Signatures",
                intro: "Les actions protégées ne s'exécutent qu'une fois signées par suffisamment de personnes autorisées avec leurs certificats numériques. Chaque signature est vérifiée auprès des émetteurs de confiance et consignée dans le journal.",
                protectedActions: "Actions protégées",
                certificates: "Certificats",
                requests: "Demandes",
            },
            loadError:
                "Les paramètres de signature n'ont pas pu être chargés. Rechargez la page pour réessayer.",
            errors: {
                automatedCeremonies:
                    "Cet événement utilise des cérémonies de clés automatiques. Les dépositaires ne réalisent pas ces étapes, leurs signatures ne peuvent donc pas être exigées. Pour exiger leurs signatures, utilisez des cérémonies de clés manuelles.",
                forbidden: "Vous n'avez pas l'autorisation d'effectuer cette modification.",
                invalid: "Le serveur a refusé ces valeurs. Vérifiez-les et réessayez.",
                conflict:
                    "Quelqu'un d'autre l'a modifié entre-temps. Rechargez la page et réessayez.",
                lockedDown:
                    "L'événement électoral est verrouillé : les règles de signature ne changent que par une nouvelle version de configuration.",
                notFound: "Cet élément n'existe plus. Rechargez la page.",
            },
            readOnly: {
                chip: "Lecture seule",
                rules: "Lecture seule. Modifier les règles de signature nécessite l'autorisation « Signatures : modifier les actions protégées ».",
                whoCanSign:
                    "Rôles disposant de l'autorisation « Signer : {{action}} » dans Utilisateurs et Rôles. Les modifier nécessite l'autorisation de modifier les rôles.",
            },
            groups: {
                "voting": "Vote",
                "results-and-reports": "Résultats et rapports",
                "enrollment": "Inscription",
                "configuration-and-keys": "Configuration et clés",
            },
            actions: {
                "initialize-voting": {
                    label: "Initialiser le vote",
                    short: "Initialisation",
                    permissionName: "initialiser le vote",
                    object: "initialisation du vote",
                    appliesTo: "Chaque $t(signing.terms.post)",
                    description:
                        "Lancée dans Publier. Initialise le $t(signing.terms.post) et génère son Rapport d'Initialisation.",
                },
                "open-voting": {
                    label: "Ouvrir le vote",
                    short: "Ouverture",
                    permissionName: "ouvrir le vote",
                    object: "ouverture du vote",
                    appliesTo: "Chaque $t(signing.terms.post)",
                    description:
                        "Lancée dans Publier avec Commencer la période de vote. Ouvre le vote au $t(signing.terms.post).",
                },
                "close-voting": {
                    label: "Clôturer le vote",
                    short: "Clôture",
                    permissionName: "clôturer le vote",
                    object: "clôture du vote",
                    appliesTo: "Chaque $t(signing.terms.post)",
                    description:
                        "Lancée dans Publier avec Arrêter la période de vote. Clôture le vote au $t(signing.terms.post) ; les signatures de clôture sont conservées dans son procès-verbal.",
                    descriptionSealed:
                        "Lancée dans Publier avec Arrêter la période de vote. Clôture le vote au $t(signing.terms.post). Une fois tous les canaux clos, ses urnes sont scellées, après le délai de grâce s'il y en a un : aucun bulletin ne peut être ajouté, modifié ni supprimé, et le vote ne peut pas reprendre. Les signatures de clôture sont conservées dans son procès-verbal.",
                },
                "generate-election-returns": {
                    label: "Générer les procès-verbaux électoraux",
                    short: "Procès-verbaux électoraux",
                    permissionName: "générer les procès-verbaux électoraux",
                    object: "procès-verbaux électoraux",
                    appliesTo: "Chaque $t(signing.terms.post) et pays",
                    description:
                        "Lancée par le dépouillement, une demande par $t(signing.terms.post) et pays. Libère les procès-verbaux signés pour impression et transmission.",
                },
                "generate-reports": {
                    label: "Générer d'autres rapports électoraux",
                    short: "Rapport",
                    permissionName: "générer d'autres rapports électoraux",
                    object: "rapport",
                    appliesTo: "Chaque $t(signing.terms.post)",
                    description:
                        "Lancée par le dépouillement pour le Rapport d'Initialisation et dans Rapports pour le rapport de participation. Libère le rapport signé.",
                },
                "transmit-results": {
                    label: "Transmettre les résultats",
                    short: "Transmission",
                    permissionName: "transmettre les résultats",
                    object: "paquet de résultats",
                    appliesTo: "Chaque $t(signing.terms.post) et pays",
                    description:
                        "Lancée dans Comptage, Transmission. Construit le paquet de résultats signé pour ses destinations ; les signatures remplissent sa liste de signatures.",
                },
                "approve-voter": {
                    label: "Approuver manuellement un électeur",
                    short: "Approbation d'électeur",
                    permissionName: "approuver manuellement un électeur",
                    object: "approbation d'électeur",
                    appliesTo: "Le $t(signing.terms.post) de l'électeur",
                    description:
                        "Lancée dans Approvals. Approuve l'électeur et lui délivre ses identifiants.",
                },
                "approve-configuration": {
                    label: "Approuver une version de configuration",
                    short: "Version de configuration",
                    permissionName: "approuver une version de configuration",
                    object: "version de configuration",
                    appliesTo: "L'événement électoral",
                    description: "Lancée dans Publier. Publie la version de configuration.",
                },
                "key-ceremony": {
                    label: "Confirmer un fragment de clé (cérémonie des clés)",
                    short: "Fragment de clé",
                    permissionName: "confirmer un fragment de clé",
                    object: "fragment de clé",
                    appliesTo: "Chaque autorité",
                    description:
                        "Lancée dans Clés par chaque autorité. Consigne la signature de l'autorité auprès de la cérémonie et du tableau d'affichage.",
                },
                "tally-key": {
                    label: "Apporter un fragment de clé (dépouillement)",
                    short: "Apport de fragment de clé",
                    permissionName: "apporter un fragment de clé",
                    object: "apport de fragment de clé",
                    appliesTo: "Chaque autorité",
                    description:
                        "Lancée dans Comptage par chaque autorité. Consigne l'apport de l'autorité.",
                },
            },
            protectedActions: {
                intro: "Chaque signature est réalisée avec le certificat numérique du jeton de sécurité du signataire.",
                columns: {
                    action: "Action",
                    appliesTo: "S'applique à",
                    whoCanSign: "Qui peut signer",
                    signaturesNeeded: "Signatures requises",
                    requestExpires: "Expiration de la demande",
                    waiting: "En attente",
                },
                off: "Désactivée",
                eachTrustee: "Chaque autorité",
                footerVersion:
                    "Les règles de signature font partie de la version de configuration {{version}} de cet événement.",
                footerFirstVersion:
                    "Les règles de signature feront partie de la première version de configuration de cet événement lors de sa publication.",
                footerChanged: "Dernière modification : {{date}}.",
                footerChangedBy: "Dernière modification : {{date}}, par {{name}}.",
                lockedDown:
                    "L'événement électoral est verrouillé : ses règles de signature appartiennent à sa version de configuration et ne changent donc que par une nouvelle version de configuration.",
                edit: "Modifier {{action}}",
                view: "Voir {{action}}",
                waitingCount_one: "{{count}} demande en attente",
                waitingCount_other: "{{count}} demandes en attente",
                capacityError:
                    "Impossible de charger qui peut signer : le nombre de signatures ne peut donc pas être vérifié par rapport aux $t(signing.terms.posts).",
            },
            expiry: {
                "30": "30 minutes",
                "60": "1 heure",
                "120": "2 heures",
                "1440": "24 heures",
                "none": "Sans limite",
                "other": "{{count}} minutes",
            },
            rule: {
                needsSignatures: "Requiert des signatures",
                whoCanSign: "Qui peut signer",
                whoCanSignHelp:
                    "Ces rôles reçoivent l'autorisation « Signer : {{action}} » dans Utilisateurs et Rôles, pour tous les événements électoraux. Les signataires doivent aussi avoir accès au $t(signing.terms.post).",
                signaturesNeeded: "Signatures requises",
                signaturesNeededHelp:
                    "Chaque signataire utilise son certificat numérique. Chaque $t(signing.terms.post) compte au moins {{n}} personnes pouvant signer.",
                signaturesNeededShortHelp: "Chaque signataire utilise son certificat numérique.",
                requesterSigning: "La personne qui la lance peut aussi signer",
                expiresAfter: "Une demande expire après",
                trusteesSign: "Les autorités signent cette étape",
                trusteesHelp:
                    "Chaque autorité signe sa propre étape avec son certificat numérique. La cérémonie des clés détermine combien d'autorités y participent.",
                footer: "Les modifications sont consignées dans le journal de l'événement électoral et font partie de la prochaine version de configuration.",
                cancel: "Annuler",
                save: "Enregistrer",
                saved: "La règle de signature a été enregistrée.",
                savedShort_one:
                    "La règle de signature a été enregistrée. {{posts}} ne peut pas encore atteindre ce nombre : ajoutez-y un signataire.",
                savedShort_other:
                    "La règle de signature a été enregistrée. {{posts}} ne peuvent pas encore atteindre ce nombre : ajoutez-y des signataires.",
                checkedOnSave:
                    "Le nombre est vérifié par rapport aux nouveaux rôles lors de l'enregistrement.",
                savedRequesterShort:
                    "La règle de signature a été enregistrée. Certains $t(signing.terms.posts) ne peuvent pas atteindre ce nombre sans la personne qui lance une demande.",
                saveError:
                    "La règle de signature n'a pas pu être enregistrée. Quelqu'un l'a peut-être modifiée entre-temps ; rechargez et réessayez.",
            },
            validation: {
                atLeastOne: "Au moins 1.",
                tooMany:
                    "Aucun $t(signing.terms.post) ne compte {{n}} personnes pouvant signer. Le maximum est {{max}}.",
                tooManyEvent:
                    "Seules {{max}} personnes peuvent signer ceci. Choisissez au plus {{max}}.",
                atMost: "Au plus {{max}}.",
                shortPosts_one:
                    "{{posts}} ne compte que {{n}} personnes pouvant signer et ne peut donc pas atteindre {{required}} signatures. Ajoutez-y un signataire ou réduisez le nombre.",
                requesterShort_one:
                    "Sans la personne qui la lance, {{posts}} ne compte que {{n}} personnes pouvant signer et ne peut donc pas atteindre {{required}} signatures.",
                requesterShort_other:
                    "Sans la personne qui la lance, {{posts}} ne comptent que {{n}} personnes pouvant signer et ne peuvent donc pas atteindre {{required}} signatures.",
                shortPosts_other:
                    "{{posts}} ne comptent que {{n}} personnes pouvant signer et ne peuvent donc pas atteindre {{required}} signatures. Ajoutez-y un signataire ou réduisez le nombre.",
            },
            pendingRequests_one:
                "{{count}} demande attend des signatures selon la règle actuelle. L'enregistrement l'annule ; la personne qui l'a lancée devra recommencer.",
            pendingRequests_other:
                "{{count}} demandes attendent des signatures selon la règle actuelle. L'enregistrement les annule ; les personnes qui les ont lancées devront recommencer.",
            certificates: {
                issuersIntro:
                    "Les certificats du personnel doivent remonter à l'un d'eux. Ils sont distincts des certificats avec lesquels les électeurs se connectent.",
                checkRevocation: "Vérifier les listes de révocation",
                crlUnavailable: {
                    "label": "Lorsqu'une liste ne peut pas être téléchargée",
                    "refuse": "Ne pas accepter les signatures",
                    "accept-unchecked": "Accepter et marquer la signature comme non vérifiée",
                },
                registration: {
                    "label": "Enregistrement d'un certificat au nom d'une personne",
                    "on-first-use": "Lorsque son titulaire signe avec pour la première fois",
                    "security-officer-only":
                        "Uniquement lorsqu'une personne autorisée à enregistrer des certificats l'enregistre",
                },
                onePost: "Un certificat ne signe que pour un seul $t(signing.terms.post)",
                issuers: "Émetteurs de confiance",
                import: "Importer des certificats d'émetteurs",
                importHelp:
                    "Choisissez un fichier PEM ou CER contenant le certificat de l'émetteur. Un fichier PEM peut contenir plusieurs certificats.",
                chooseFile: "Choisir un fichier de certificat",
                fileError: "Le fichier n'a pas pu être lu.",
                imported:
                    "{{imported}} certificats d'émetteurs importés ; {{skipped}} étaient déjà de confiance.",
                importedWithErrors:
                    "{{imported}} certificats d'émetteurs importés, {{skipped}} déjà de confiance. Refusés : {{errors}}",
                importError: "Les certificats d'émetteurs n'ont pas pu être importés.",
                deleteIssuer: "Supprimer {{name}}",
                deleteIssuerConfirm:
                    "Supprimer {{name}} des émetteurs de confiance ? Les certificats qu'il a émis ne pourront plus signer.",
                deleteError: "L'émetteur n'a pas pu être supprimé.",
                noIssuers:
                    "Aucun émetteur de confiance pour l'instant. Le personnel ne peut pas signer tant qu'aucun n'est importé.",
                root: "Racine",
                intermediate: "Intermédiaire",
                columns: {
                    issuer: "Émetteur",
                    type: "Type",
                    issuedBy: "Émis par",
                    validUntil: "Valide jusqu'au",
                    sha256: "SHA-256",
                    person: "Personne",
                    post: "$t(signing.terms.post)",
                    certificate: "Certificat",
                    registered: "Enregistré",
                    status: "Statut",
                },
                checks: "Vérifications",
                checksSaved: "Les vérifications des certificats ont été enregistrées.",
                checksError: "Les vérifications des certificats n'ont pas pu être enregistrées.",
                crlSchedule: "Téléchargées depuis chaque émetteur toutes les heures.",
                crlUpdated: "{{url}} : mise à jour {{time}}",
                crlFailed: "{{url}} : téléchargement impossible (dernier essai {{time}})",
                registeredTitle: "Certificats enregistrés",
                search: "Rechercher des personnes, des certificats ou des $t(signing.terms.posts)",
                status: "Statut",
                statusAll: "Tous",
                statuses: {
                    "active": "Actif",
                    "expires-soon": "Expire bientôt",
                    "expired": "Expiré",
                    "revoked": "Révoqué",
                },
                revokedOn: "Révoqué le {{date}}",
                allPosts: "Tous",
                noCertificates: "Aucun certificat enregistré.",
                registeredHow: {
                    "first-use": "À la première signature",
                    "security-officer": "Enregistré par un administrateur",
                },
                register: "Enregistrer un certificat",
                registerSubmit: "Enregistrer",
                registerDone: "Le certificat a été enregistré.",
                registerError: "Le certificat n'a pas pu être enregistré.",
                person: "Personne",
                personSearchHelp:
                    "Saisissez une partie d'un nom d'utilisateur pour trouver la personne.",
                registeredBy: "Par {{name}}",
                registerRefused:
                    "Ce certificat ne peut pas être enregistré : vérifiez qu'il a été émis par un émetteur de confiance, qu'il est valide aujourd'hui et qu'il est destiné à la signature.",
                registeredToOther:
                    "Ce certificat est enregistré au nom de {{name}}. Si ce compte appartient aussi à {{name}}, associez-le comme son second compte.",
                linkAccount: "Associer comme second compte de la même personne",
                alreadyRegistered: "Ce certificat est déjà enregistré au nom de cette personne.",
                pem: "Certificat (PEM)",
                revoke: "Révoquer",
                revokeOf: "Révoquer le certificat de {{name}}",
                revokeTitle: "Révoquer le certificat de {{name}}",
                revokeHelp:
                    "Un certificat révoqué ne peut plus signer. Les signatures qu'il a déjà produites restent valables.",
                revokeReason: "Motif",
                revokeDone: "Le certificat a été révoqué.",
                revokeError: "Le certificat n'a pas pu être révoqué.",
            },
            requests: {
                exportCsv: "Exporter en CSV",
                exportError: "Les demandes n'ont pas pu être exportées.",
                exportFileName: "signing-requests.csv",
                status: "Statut",
                statusAll: "Toutes",
                statusCount: "{{status}} · {{count}} sur {{total}}",
                expires: "Expire {{time}}",
                lastSignatureBy: "{{name}}, {{time}}",
                empty: "Aucune demande de signature pour l'instant.",
                columns: {
                    request: "Demande",
                    status: "Statut",
                    started: "Lancée",
                    by: "Par",
                    lastSignature: "Dernière signature",
                    code: "Code",
                },
            },
            reports: {
                postRequired:
                    "Sélectionnez un poste pour générer ce rapport lorsque des signatures sont requises.",
                generateNotice:
                    "{{post}} : le document est généré maintenant. Il pourra être imprimé et transmis une fois signé par {{n}} personnes.",
            },
            status: {
                waiting: "En attente",
                completed: "Signée",
                executed: "Terminée",
                cancelled: "Annulée",
                expired: "Expirée",
                failed: "Échouée",
            },
            cancelReasons: {
                "by-requester": "La personne qui l'a lancée l'a annulée",
                "by-operator": "Un opérateur l'a annulée",
                "rule-changed": "La règle de signature de l'action a changé",
                "payload-changed": "Ce qu'elle signe a changé",
                "superseded": "Une demande plus récente l'a remplacée",
                "certificate-revoked": "Un certificat qui l'a signée a été révoqué",
            },
            panel: {
                rulePost:
                    "Requiert {{n}} signatures des signataires de {{post}}, chacune avec son certificat numérique.",
                ruleEvent:
                    "Requiert {{n}} signatures, chacune avec le certificat numérique du signataire.",
                signingCode: "Code de signature",
                signers: "Signataires",
                sign: "Signer",
                handover: "Le membre suivant se connecte",
                cancel: "Annuler la demande",
                signedAt: "Signé {{time}}",
                notSigned: "Non signé",
                certificate: "Certificat {{name}}",
                you: "(vous)",
                expiresAt: "Expire à {{time}}",
                progress: "{{count}} sur {{total}}",
                openDocument: "Ouvrir le document",
                configurationVersion: "Version de configuration {{version}}",
                configurationChanges: "Modifications de cette version",
            },
            dialog: {
                title: "Signature : {{object}}",
                steps: {
                    check: "Vérifier",
                    certificate: "Certificat",
                    signed: "Signé",
                },
                localNote:
                    "La signature a lieu dans ce navigateur. Votre fichier de certificat, sa clé privée et son mot de passe ne sont jamais envoyés. Seuls votre signature et votre certificat public sont transmis au serveur.",
                check: {
                    signingAs: "Vous signez en tant que {{name}}",
                    titlePost: "{{title}}, {{post}}",
                    sameCode: "Toutes les personnes qui signent voient le même code.",
                    confirmDocument: "J'ai vérifié ce que je signe : {{object}}",
                },
                certificate: {
                    intro: "Insérez votre jeton de sécurité et choisissez votre fichier de certificat.",
                    password: "Mot de passe du certificat",
                    open: "Ouvrir le certificat",
                    chooseAnother: "Choisir un autre fichier",
                },
                checks: {
                    "passed": {
                        "trusted-issuer": "Émis par un émetteur de confiance ({{root}})",
                        "valid-now": "Valide aujourd'hui",
                        "signing-key-usage": "Destiné à la signature",
                        "not-revoked": "Non révoqué (listes mises à jour {{time}})",
                        "registered": "Enregistré à votre nom le {{date}}",
                        "registered-to-other": "Non enregistré au nom d'une autre personne",
                        "already-signed": "Pas encore utilisé pour cette demande",
                        "post-binding": "Enregistré pour ce $t(signing.terms.post)",
                        "signature": "La signature couvre cette demande",
                    },
                    "failed": {
                        "trusted-issuer": "Non émis par un émetteur de confiance",
                        "valid-now": "Non valide aujourd'hui",
                        "signing-key-usage": "Non destiné à la signature",
                        "not-revoked":
                            "Révoqué, ou aucune liste de révocation à jour pour le vérifier",
                        "registered": "Non enregistré à votre nom",
                        "registered-to-other": "Enregistré au nom de {{name}}",
                        "already-signed": "Déjà utilisé pour cette demande",
                        "post-binding": "Enregistré pour un autre $t(signing.terms.post)",
                        "signature": "La signature ne couvre pas cette demande",
                    },
                    "first-use": "Première utilisation : il sera enregistré à votre nom",
                },
                problems: {
                    wrongPassword: "Mot de passe incorrect. Vérifiez-le et réessayez.",
                    notForYou:
                        "Ce certificat ne peut pas signer pour vous. Utilisez le certificat de votre propre jeton de sécurité.",
                    issuerNotAccepted:
                        "Utilisez le certificat que {{organization}} a enregistré pour vous. Les certificats d'autres émetteurs ne sont pas acceptés.",
                    cancelled:
                        "Cette demande a été annulée : {{reason}}. Les signatures données pour elle ne comptent plus. Relancez-la pour signer la version actuelle.",
                },
                signed: {
                    title: "Signé",
                    withCertificate: "avec le certificat de {{name}}",
                    count: "{{n}} signatures sur {{total}}.",
                    allIn: "Les {{total}} signatures sont réunies.",
                    next: "Prochains signataires : {{names}}.",
                },
                handover:
                    "Vous allez être déconnecté. Le membre suivant se connecte sur cet ordinateur et revient à cette demande pour signer. La demande reste ouverte jusqu'à {{time}}.",
                sign: "Signer",
                back: "Retour",
                cancel: "Annuler",
            },
            widget: {
                continue: "Continuer",
                done: "Terminé",
                close: "Fermer",
                retry: "Réessayer",
                loading: "Chargement de la demande…",
                loadError: "La demande n'a pas pu être chargée.",
                chooseFile: "Choisir le fichier de certificat",
                fileInput: "Fichier de certificat",
                fileSize: "{{size}} Ko",
                showPassword: "Afficher le mot de passe",
                hidePassword: "Masquer le mot de passe",
                opening: "Ouverture du certificat…",
                checking: "Vérification du certificat…",
                signing: "Signature en cours…",
                certificateCard: "Émis par {{issuer}} · valide jusqu'au {{date}} · {{algorithm}}",
                fingerprint: "SHA-256 {{fingerprint}}",
                algorithms: {
                    "rsa-pkcs1-sha256": "RSA",
                    "ecdsa-p256-sha256": "EC P-256",
                },
                document: "{{type}} · SHA-256 {{hash}}",
                documentPages: "{{type}} · {{pages}} pages · SHA-256 {{hash}}",
                checksTitle: "Vérifications du certificat",
                untrustedIssuer:
                    "{{issuer}} n'est pas un émetteur de confiance pour cet événement électoral",
                registeredToSomeoneElse: "Enregistré au nom d'une autre personne",
                checkPassedNoDetail: {
                    "trusted-issuer": "Émis par un émetteur de confiance",
                    "not-revoked": "Non révoqué",
                },
                organization: "votre organisation",
                cantSign: "Ce certificat ne peut pas signer cette demande.",
                checkError: "Le certificat n'a pas pu être vérifié. Réessayez.",
                fileErrors: {
                    UNREADABLE_FILE:
                        "Ce fichier n'est pas un fichier de certificat (.p12 ou .pfx), ou il est endommagé.",
                    UNSUPPORTED_ENCRYPTION:
                        "Ce navigateur ne peut pas ouvrir le chiffrement utilisé par ce fichier.",
                    NO_PRIVATE_KEY:
                        "Ce fichier ne contient pas de clé privée. Choisissez le fichier de certificat de votre jeton de sécurité.",
                    NO_CERTIFICATE: "Ce fichier ne contient aucun certificat.",
                    UNSUPPORTED_KEY:
                        "Le type de clé de ce certificat n'est pas pris en charge. Utilisez un certificat RSA ou EC P-256.",
                    KEY_CERTIFICATE_MISMATCH:
                        "Le certificat de ce fichier ne correspond pas à sa clé.",
                },
                openError: "Le certificat n'a pas pu être ouvert. Réessayez.",
                signError: "La signature n'a pas pu être envoyée. Réessayez.",
                refused: "Le serveur a refusé la signature.",
                stale: "Le document a changé pendant que vous signiez. Signez à nouveau.",
                mismatch:
                    "Ce qui serait signé ne correspond pas à cette demande. Fermez la fenêtre et rouvrez la demande.",
                documentMismatch: "Le document ne correspond pas à celui que signe cette demande.",
                documentError: "Le document n'a pas pu être téléchargé. Réessayez.",
                alreadySigned: "Vous avez déjà signé cette demande.",
                closed: {
                    changed:
                        "Cette demande a changé après son ouverture. Fermez cette fenêtre et vérifiez-la à nouveau avant de signer.",
                    allSigned: "Cette demande a déjà toutes ses signatures.",
                },
                chooseCertificate: "Certificat de signature",
                renderError:
                    "La demande de signature n'a pas pu être affichée. Fermez-la et rouvrez-la.",
                signedAt: "{{time}}",
                panel: {
                    completedAt: "Signée à {{time}}",
                    expired:
                        "Cette demande a expiré. Les signatures données pour elle ne comptent plus. Relancez-la pour signer.",
                    failed: "Toutes les signatures sont réunies, mais l'action a échoué. Le journal contient les détails.",
                    details: "Détails",
                    close: "Fermer le panneau de la demande",
                },
                cancelDialog: {
                    title: "Annuler cette demande ?",
                    body: "Les signatures données pour elle ne comptent plus. La personne qui l'a lancée devra recommencer.",
                    reason: "Motif (facultatif)",
                    confirm: "Annuler la demande",
                    back: "La conserver",
                    error: "La demande n'a pas pu être annulée. Réessayez.",
                },
                handoverDialog: {
                    title: "Le membre suivant se connecte",
                    noExpiry:
                        "Vous allez être déconnecté. Le membre suivant se connecte sur cet ordinateur et revient à cette demande pour signer.",
                    confirm: "Se déconnecter",
                    back: "Rester connecté",
                    error: "Le relais n'a pas pu être enregistré. Réessayez.",
                },
            },
            details: {
                keys_ceremony_id: "Cérémonie",
                tally_session_id: "Session de dépouillement",
                trustee_id: "Autorité",
                key_share_sha256: "SHA-256 du fragment de clé",
                channel: "Canal",
                channels: "Canaux",
                publication_id: "Publication des bulletins",
                ballot_publication_id: "Publication des bulletins",
                digest: "SHA-256 de la configuration",
                signing_rules: "Règles de signature",
                scheduled_events: "Nouveaux événements programmés",
                ballots_and_contests: "Bulletins et concours",
                application_id: "Demande d'inscription",
                applicant_registry_id: "Compte du registre",
                decision: "Décision",
                submitted_at: "Soumise",
                reason: "Pourquoi une personne est nécessaire",
                registry_record: "Fiche du registre",
                status: "Statut de la demande d'inscription",
                from: "Statut précédent",
            },
            closed: {
                pending: "Toutes les signatures sont réunies. Le vote se clôture dans un instant.",
                title: "Le vote a été clôturé à {{time}}.",
                titleSealed: "Le vote a été clôturé à {{time}}. Bulletins scellés.",
                record: "Procès-verbal de scellement",
                ballots: "Bulletins dans le scellé",
                sealHash: "{{algorithm}} du scellé",
                signedBy: "Signé par",
                signatures: "Signatures de clôture dans le procès-verbal de scellement",
                signaturesValue_one: "{{count}}, code de signature {{code}}",
                signaturesValue_other: "{{count}}, code de signature {{code}}",
                signers: "Signé par les membres",
            },
            values: {
                ballots_and_contests: {
                    "first-version": "Première version",
                    "no-changes": "Aucun changement",
                    "changed": "Modifiés",
                },
                signing_rules: {
                    "initialize-voting": "$t(signing.actions.initialize-voting.label)",
                    "open-voting": "$t(signing.actions.open-voting.label)",
                    "close-voting": "$t(signing.actions.close-voting.label)",
                    "generate-election-returns":
                        "$t(signing.actions.generate-election-returns.label)",
                    "generate-reports": "$t(signing.actions.generate-reports.label)",
                    "transmit-results": "$t(signing.actions.transmit-results.label)",
                    "approve-voter": "$t(signing.actions.approve-voter.label)",
                    "approve-configuration": "$t(signing.actions.approve-configuration.label)",
                    "key-ceremony": "$t(signing.actions.key-ceremony.label)",
                    "tally-key": "$t(signing.actions.tally-key.label)",
                },
                channels: {
                    ONLINE: "En ligne",
                    KIOSK: "Kiosque",
                    EARLY_VOTING: "Vote anticipé",
                    TELEPHONE: "Téléphone",
                },
                statuses: {
                    NOT_STARTED: "Non commencé",
                    OPEN: "Ouvert",
                    PAUSED: "En pause",
                    CLOSED: "Fermé",
                },
                channelStatus: "{{channel}}: {{status}}",
                ruleChange: "{{action}} : {{rule}}",
                ruleChangeFrom: "{{action}} : {{rule}} (avant {{was}})",
                ruleNeeds: "requiert {{n}}",
                ruleOff: "désactivée",
                decision: {
                    approve: "Approuver",
                },
            },
            results: {
                signatures: "Signatures",
                needs: "Requiert {{n}}",
                off: "Désactivée",
                openRequest: "Ouvrir la demande de signature",
                downloadSigned: "Télécharger le PDF signé",
                print: "Imprimer",
                transmit: "Transmettre les résultats",
                sendTo: "Envoyer à {{count}} destinations",
                awaiting: "{{item}} : en attente de signatures",
                transmission: {
                    title: "Signatures",
                    description:
                        "Chaque signataire signe les résultats du paquet avec son certificat numérique, dans ce navigateur. Le paquet peut être envoyé une fois signé par {{n}} personnes.",
                    waiting:
                        "Le paquet peut être envoyé une fois que sa demande de signature a toutes ses signatures.",
                    signed: "Le paquet porte toutes ses signatures et peut être envoyé.",
                    ended: "La demande de signature de ce paquet est terminée. Recréez le paquet pour le signer.",
                },
            },
            waiting: {
                title: "En attente de ma signature",
                buttonCount_one: "En attente de ma signature : {{count}} demande à signer",
                buttonCount_other: "En attente de ma signature : {{count}} demandes à signer",
                intro: "Les demandes qui attendent les signatures des actions que vous pouvez signer, dans vos $t(signing.terms.posts).",
                close: "Fermer la liste",
                empty: "Rien n'attend votre signature.",
                loadError: "Les demandes en attente de signatures n'ont pas pu être chargées.",
                signedByYou: "Signée par vous",
            },
            notes: {
                afterApproval: "Après l'approbation",
                afterApprovalValue:
                    "Les identifiants de l'électeur sont délivrés et lui sont envoyés",
                keyShare: "Votre fragment de clé",
                keyShareChecked: "Vérifié : c'est votre fragment de clé pour cette cérémonie",
                recordedIn: "Consigné dans",
                recordedInCeremony: "La cérémonie des clés et le tableau d'affichage",
                recordedInTally: "La session de dépouillement",
            },
            keyShare: {
                signing:
                    "Signez votre fragment de clé dans le panneau de signature. Il est consigné une fois signé.",
                record: "Consigner mon fragment de clé",
                failed: "Votre fragment de clé signé n'a pas pu être consigné : {{error}}",
                dropAgain:
                    "Déposez à nouveau votre fichier de fragment de clé pour consigner votre fragment de clé signé.",
                redo: "Votre fragment de clé a été apporté sans votre signature, que cette élection exige désormais. Apportez-le à nouveau et signez-le.",
                notTaken:
                    "La cérémonie n'accepte plus ce fragment de clé. Déposez à nouveau votre fichier de fragment de clé.",
            },
        },
        lifecycle: {
            signedClose: {
                title: "Échéance de clôture signée",
                deadline: "{{election}} : {{time}} · autorisée par la configuration {{code}}.",
                explanation:
                    "Cette échéance signée reste contraignante même si le calendrier modifiable est changé ou supprimé. Le planificateur ferme les canaux autorisés encore ouverts.",
                reached:
                    "Cette échéance signée est passée. Vérifiez l’état actuel du vote et le journal d’audit ; son traitement n’a pas encore été enregistré.",
                processed: "Échéance de clôture signée traitée à {{time}}.",
                signedAt: "Échéance signée : {{time}}.",
                channels: "Canaux encore couverts par cette échéance : {{channels}}.",
                result: "Consultez l’état du vote et le journal d’audit pour connaître les changements réels et le résultat complet.",
                unavailable:
                    "Impossible de charger les échéances de clôture signées. Vérifiez le calendrier publié et le journal d’audit.",
            },
            picker: {
                noMatch:
                    "Aucun fuseau horaire ne correspond. Saisissez une ville, un pays, une zone, une abréviation ou un décalage.",
            },
            input: {
                timezone: "Fuseau horaire",
                scheduledAt: "Prévu le",
                meetingStart: "Début de la réunion",
                cronZone:
                    "La planification s'exécute dans le fuseau horaire principal de l'événement, {{zone}}.",
                unconfiguredZone:
                    "{{zone}} ne fait pas partie des fuseaux horaires configurés de l'événement. Choisissez-en un.",
            },
            schedule: {
                allElections: "Toutes les élections",
                outcome: "Résultat",
                noOffset: "Pas de décalage horaire : ne s'exécute jamais",
                unpublished: "Pas encore publié",
                notPublished:
                    "Rien n'est encore publié : les électeurs voient le calendrier après la première publication.",
                unpublishedChanges_one:
                    "{{count}} événement planifié a changé depuis la dernière publication. Les électeurs le voient après votre publication.",
                unpublishedChanges_other:
                    "{{count}} événements planifiés ont changé depuis la dernière publication. Les électeurs les voient après votre publication.",
                offsetless_one:
                    "{{count}} horaire planifié n'a pas de décalage horaire et ne s'exécute donc jamais. Modifiez-le pour définir son fuseau horaire.",
                offsetless_other:
                    "{{count}} horaires planifiés n'ont pas de décalage horaire et ne s'exécutent donc jamais. Modifiez-les pour définir leur fuseau horaire.",
                outcomeChange:
                    "L'enregistrement modifie l'effet de cette transition planifiée : {{before}} → {{after}}.",
                outcomeNew: "Une fois enregistrée, cette transition planifiée : {{after}}.",
                outcomeElections: "{{count}} élections sur {{total}}",
                exportError: "Le calendrier n'a pas pu être exporté.",
                exportFileName: "schedule.csv",
                totals: {
                    refused_one:
                        "{{count}} ligne planifiée sera refusée ({{transitions}} transitions d'élections).",
                    refused_other:
                        "{{count}} lignes planifiées seront refusées ({{transitions}} transitions d'élections).",
                    runsUnsigned_one:
                        "{{count}} clôture planifiée s'exécutera sans signatures ({{transitions}} transitions d'élections).",
                    runsUnsigned_other:
                        "{{count}} clôtures planifiées s'exécuteront sans signatures ({{transitions}} transitions d'élections).",
                    review: "Examiner",
                    showAll: "Tout afficher",
                    showing: {
                        refused:
                            "Affichage des {{count}} lignes planifiées qui seront refusées ({{transitions}} transitions d'élections).",
                        runsUnsigned:
                            "Affichage des {{count}} clôtures planifiées qui s'exécuteront sans signatures ({{transitions}} transitions d'élections).",
                    },
                },
                recompute: {
                    title_one:
                        "Une mise à jour de la base de données des fuseaux horaires déplace {{count}} horaire planifié à venir. Rien ne change tant que vous ne l'appliquez pas.",
                    title_other:
                        "Une mise à jour de la base de données des fuseaux horaires déplace {{count}} horaires planifiés à venir. Rien ne change tant que vous ne les appliquez pas.",
                    change: "{{type}} : {{before}} → {{after}}",
                    apply: "Appliquer",
                    applied_one: "{{count}} horaire planifié mis à jour.",
                    applied_other: "{{count}} horaires planifiés mis à jour.",
                    error: "Les horaires planifiés n'ont pas pu être mis à jour.",
                },
                outcomeChangeElections_one:
                    "Enregistrer change le résultat pour {{count}} élection :",
                outcomeChangeElections_other:
                    "Enregistrer change le résultat pour {{count}} élections :",
            },
            authorizes: {
                reportPolicyOf: "{{election}} : {{value}}",
                initializationRetained:
                    "Un rapport obligatoire dans cette configuration signée reste obligatoire si le paramètre actuel du poste devient non obligatoire.",
                title: "Ce que cette approbation autorise",
                schedule: "Ouvertures et clôtures planifiées",
                noSchedule:
                    "Aucune ouverture ni clôture planifiée : les signataires ouvrent et clôturent le vote.",
                opens: "Ouverture : {{time}}",
                closes: "Clôture : {{time}}",
                settings: "Paramètres",
                unsignedClose: "Clôture planifiée sans signatures : {{value}}",
                initialization: "Initialisation : {{value}}",
                firstConfiguration: "C'est la première configuration approuvée : rien à comparer.",
                sameAsPrevious:
                    "Les paramètres sont identiques à ceux de la configuration approuvée précédente.",
                rule: {
                    openNeeds_one: "L'ouverture nécessite {{count}} signature",
                    openNeeds_other: "L'ouverture nécessite {{count}} signatures",
                    openNoSignatures: "L'ouverture ne nécessite aucune signature",
                    closeNeeds_one: "La clôture nécessite {{count}} signature",
                    closeNeeds_other: "La clôture nécessite {{count}} signatures",
                    closeNoSignatures: "La clôture ne nécessite aucune signature",
                    openSetting: "Ouverture du vote",
                    closeSetting: "Clôture du vote",
                    signatures_one: "{{count}} signature",
                    signatures_other: "{{count}} signatures",
                    none: "aucune signature",
                },
                diff: {
                    tightens: "Renforce : {{setting}} {{before}} → {{after}}",
                    loosens: "Assouplit : {{setting}} {{before}} → {{after}}",
                    mixed: "Modifie : {{setting}} {{before}} → {{after}} (plus strict sur un point, plus souple sur un autre)",
                },
                comparedWith:
                    "Par rapport à la configuration approuvée précédente, approbation {{code}} :",
                channels: "Canaux de vote par élection",
                channelsOf: "{{election}} : {{channels}}",
                noChannels: "aucun",
            },
            publish: {
                openedAuthorized:
                    "Vote ouvert comme prévu à {{time}}, autorisé par l'approbation de configuration {{code}} (signée par {{names}}).",
                closedAuthorized:
                    "Vote clôturé comme prévu à {{time}}, autorisé par l'approbation de configuration {{code}} (signée par {{names}}).",
                closedUnsigned:
                    "Vote clôturé comme prévu à {{time}}. Aucune signature de clôture : le calendrier a clôturé le vote à son échéance.",
                authorizedBy: "Autorisé par",
                cancelledRequest:
                    "La demande {{code}} avait {{n}} signatures sur {{k}} et a été annulée.",
                openedRefused: "L'ouverture planifiée du {{time}} a été refusée.",
                closedRefused: "La clôture planifiée du {{time}} a été refusée.",
                openedNoSignaturesNeeded:
                    "Le vote s'est ouvert comme prévu ({{time}}) ; aucune signature n'était nécessaire.",
                closedNoSignaturesNeeded:
                    "Le vote s'est clôturé comme prévu ({{time}}) ; aucune signature n'était nécessaire.",
                openedNothingToChange:
                    "À {{time}}, l'ouverture planifiée n'avait rien à ouvrir : ses canaux étaient déjà ouverts.",
                closedNothingToChange:
                    "À {{time}}, la clôture planifiée n'avait rien à clôturer : ses canaux étaient déjà fermés.",
            },
            import: {
                title: "Importer le calendrier",
                subtitle:
                    "Une ligne par événement et par élection, en heure locale. Laissez le fuseau horaire vide pour utiliser celui de l'élection.",
                chooseFile: "Choisir un fichier CSV",
                template: "Télécharger le modèle",
                templateFileName: "schedule-template.csv",
                ready: "{{ok}} événements prêts pour {{posts}} élections.",
                needsAttention_one:
                    "{{ok}} événements prêts pour {{posts}} élections. {{count}} ligne nécessite votre attention ; corrigez le fichier et téléversez-le à nouveau.",
                needsAttention_other:
                    "{{ok}} événements prêts pour {{posts}} élections. {{count}} lignes nécessitent votre attention ; corrigez le fichier et téléversez-le à nouveau.",
                preview: "Lignes à importer",
                row: "Ligne",
                asWritten: "{{local}} · {{place}}",
                moreRows: "…et {{count}} lignes de plus",
                imported: "Calendrier importé : {{created}} créés, {{updated}} mis à jour.",
                uploadError: "Le fichier n'a pas pu être vérifié. Téléversez-le à nouveau.",
                importError: "Le calendrier n'a pas pu être importé.",
                error: {
                    unknownElection: "Aucune élection n'a l'alias {{election}}.",
                    unknownEventType: "{{type}} n'est pas un type d'événement planifié.",
                    invalidTimeZone: "{{zone}} n'est pas un fuseau horaire.",
                    invalidDateTime: "La date et l'heure doivent être au format YYYY-MM-DDTHH:MM.",
                    invalidVotingChannels:
                        "Les canaux de vote sont inconnus, ou ouvrent ensemble le vote en ligne et le vote anticipé.",
                    dstGap: "{{dateTime}} n'existe pas à {{city}} car les horloges avancent. Indiquez une heure qui existe.",
                    duplicate: "Une autre ligne planifie le même événement pour cette élection.",
                    other: "Cette ligne ne peut pas être importée ({{code}}).",
                    ambiguousElection: "Plusieurs élections ont l'alias {{election}}.",
                },
            },
            settings: {
                accordion: "Langue, date et heure",
                dateAndTime: "Date et heure",
                configured: "Fuseaux horaires configurés",
                configuredHelp:
                    "{{count}} fuseaux horaires. Les élections choisissent le leur dans cette liste ; saisissez une ville ou un pays pour en ajouter un.",
                moreZones: "+{{count}}",
                primary: "Fuseau horaire principal",
                primaryHelp:
                    "Utilisé pour les planifications de tout l'événement, les rapports et les élections sans fuseau horaire propre.",
                primaryInUse:
                    "{{zone}} est le fuseau horaire principal. Choisissez d'abord un autre fuseau horaire principal.",
                inUse: "{{zone}} est utilisé par {{names}}. Modifiez d'abord ces élections.",
                logs: "Heures dans les journaux et leurs exports",
                logsPrimary: "Fuseau horaire principal ({{abbr}})",
                logsElection: "Le fuseau horaire de l'élection de chaque ligne",
                logsHelp: "Les lignes sans élection utilisent le fuseau horaire principal.",
                electionZone: "Fuseau horaire",
                electionPrimary: "Principal de l'événement : {{zone}}",
                electionZoneHelp:
                    "Les planifications, les écrans des électeurs et les rapports de cette élection utilisent ce fuseau horaire, y compris pour toutes ses zones. Vide, le fuseau horaire principal de l'événement s'applique.",
                electionUnconfigured:
                    "L'événement ne configure plus ce fuseau horaire : l'élection utilise donc le fuseau horaire principal, {{zone}}. Choisissez l'un des fuseaux horaires configurés.",
                electionUnconfiguredSave:
                    "Choisissez l'un des fuseaux horaires configurés de l'événement.",
            },
            policies: {
                accordion: "Cycle de vie du vote",
                intro: "Ces paramètres font partie de la configuration de l'événement électoral : l'approbation de configuration les signe, et les ouvertures et clôtures planifiées suivent le plus strict des paramètres actuels et publiés.",
                nothingPublished:
                    "Rien n'est encore publié : jusqu'à la première publication, les ouvertures et clôtures planifiées utilisent les valeurs par défaut (par élection, refuser).",
                publishedValue: "Configuration publiée : {{value}}",
                changedSincePublished:
                    "Modifié depuis la configuration publiée : les ouvertures et clôtures planifiées suivent la plus stricte des deux jusqu'à la prochaine publication approuvée.",
                scope: {
                    title: "Initialisation avant l'ouverture du vote",
                    post: {
                        label: "Par élection",
                        help: "Une élection s'ouvre dès qu'elle est initialisée.",
                    },
                    event: {
                        label: "Événement entier",
                        help: "Aucune élection ne s'ouvre tant que toutes les élections ne sont pas initialisées.",
                        warning:
                            "Une seule élection non initialisée maintient toutes les élections fermées, y compris à leur ouverture planifiée.",
                    },
                    postAndCountry: {
                        label: "Par élection et par pays",
                        help: "Une élection s'ouvre dès que chaque pays (zone) qui en dépend est initialisé.",
                        warning:
                            "Une élection reste fermée, même à son ouverture planifiée, tant que chaque pays qui en dépend n'est pas initialisé ; chaque pays est initialisé avec son propre rapport.",
                    },
                },
                close: {
                    title: "Clôture planifiée sans signatures",
                    help: "Lorsque la clôture du vote nécessite des signatures et qu'une clôture planifiée ne figure pas dans la configuration signée.",
                    refuse: {
                        label: "Refuser",
                        help: "La clôture ne s'exécute pas ; les signataires de l'élection clôturent le vote avec leurs signatures.",
                    },
                    runAsSystem: {
                        label: "Exécuter en tant que système",
                        help: "Le vote est clôturé à l'échéance, enregistré comme clôturé par le calendrier sans signatures.",
                        warning:
                            "Les clôtures planifiées hors de la configuration signée clôturent le vote sans la signature de quiconque. Le journal et les documents l'indiquent.",
                    },
                },
                onSave: {
                    outcomes_zero: "Aucune transition planifiée ne change de résultat.",
                    outcomes_one:
                        "{{count}} transition planifiée change de résultat. Examinez-la dans Événements Planifiés.",
                    outcomes_other:
                        "{{count}} transitions planifiées changent de résultat. Examinez-les dans Événements Planifiés.",
                },
                saveError: "Les paramètres du cycle de vie du vote n'ont pas pu être enregistrés.",
                publishedPerTarget: "Configuration publiée, par cible : {{values}}",
                publishedCount_one: "{{value}} ({{count}} cible)",
                publishedCount_other: "{{value}} ({{count}} cibles)",
                savedWithoutPolicies:
                    "L'événement électoral a été enregistré, mais pas les paramètres du cycle de vote : {{reason}}. Enregistrez-les à nouveau.",
            },
        },
        scheduledOutcome: {
            chip: {
                waitingForInitialization: "En attente d’initialisation",
                runs: "S'exécutera",
                runsUnsigned: "S'exécutera sans signatures",
                refused: "Sera refusée",
            },
            note: {
                waitingForInitialization: "En attente d’initialisation",
                authorized: "Autorisée par la configuration {{code}}",
                noSignaturesNeeded: "Aucune signature nécessaire",
                closesUnsigned: "Clôture sans signatures",
                refused: {
                    initialization: "L’initialisation requise est incomplète",
                    votingClose: "Le vote ne peut pas ouvrir après sa date limite de clôture",
                    needsSignatures: "Nécessite les signatures des signataires",
                    covered: "Absent de la configuration signée",
                    unsignedClose: "Une clôture sans signatures est refusée",
                    stricterCopy: "Modifié depuis la configuration publiée, qui décide encore",
                    defaults: "Rien n'est encore publié : les valeurs par défaut s'appliquent",
                },
                refusedWithStep: "{{reason}}. {{next}}",
            },
            why: {
                button: "Pourquoi ?",
                title: {
                    waitingForInitialization: "Pourquoi l’initialisation est attendue",
                    runs: "Pourquoi elle s'exécutera",
                    runsUnsigned: "Pourquoi elle s'exécutera sans signatures",
                    refused: "Pourquoi elle sera refusée",
                },
                checks: "Vérifications",
                check: "Vérification",
                current: "Paramètres actuels",
                published: "Configuration publiée",
                verdict: "Verdict",
                allows: "Autorise",
                blocks: "Bloque",
                deciding: "Vérification décisive",
                nextStep: "Prochaine étape :",
                signedBy: "Signée par {{names}}",
            },
            question: {
                initialization: "L’initialisation requise est-elle terminée ?",
                votingClose: "Cette ouverture respecte-t-elle la date limite de clôture du vote ?",
                needsSignatures: "Cette action nécessite-t-elle des signatures ?",
                covered: "Ce calendrier exact figure-t-il dans la configuration signée ?",
                unsignedClose: "Que se passe-t-il pour une clôture sans signatures ?",
                stricterCopy:
                    "Les paramètres actuels et publiés diffèrent-ils ? Lesquels décident ?",
                defaults: "Quelque chose est-il déjà publié ?",
            },
            check: {
                initialization: {
                    waiting:
                        "Les initialisations exigées par les paramètres actuels et publiés doivent toutes être terminées.",
                },
                votingClose: {
                    passed: "Le vote ferme à {{closes_at}} ; cette ouverture ne peut pas être exécutée à cette échéance ou après.",
                },
                needsSignatures: {
                    yes: "Oui, {{signatures}} signatures",
                    yes_one: "Oui, {{count}} signature",
                    yes_other: "Oui, {{count}} signatures",
                    no: "Non",
                },
                covered: {
                    overriddenBySignedPostRow:
                        "La configuration signée {{code}} utilise l’ouverture propre à ce poste, {{scheduled_event_id}}. L’ouverture de l’événement entier ne s’applique pas.",
                    yes: "Oui : approbation {{code}}, inchangée",
                    changed: "Non : modifiée depuis l'approbation {{code}}",
                    changedBy:
                        "Non : modifiée le {{edited_at}} par {{edited_by}}, après l'approbation {{code}}",
                    notInApproval: "Non : l'approbation {{code}} ne l'inclut pas",
                    noApproval: "Aucune configuration approuvée pour l'instant",
                    channelsChanged:
                        "Non : les canaux de vote de l'élection ont changé depuis l'approbation {{code}}",
                    alreadyFired:
                        "Non : cette transition de l'approbation {{code}} a déjà eu lieu le {{fired_at}} ; la relancer nécessite des signatures",
                    late: "Non : plus de 15 minutes se sont écoulées depuis {{scheduled_date}} (approbation {{code}}) ; l'exécuter maintenant nécessite des signatures",
                },
                unsignedClose: {
                    refuse: "Refuser",
                    runAsSystem: "Exécuter en tant que système",
                },
                stricterCopy: {
                    same: "Ils sont identiques",
                    currentStricter:
                        "Les paramètres actuels sont plus stricts : appliqués dès maintenant",
                    currentLooser:
                        "Les paramètres actuels sont plus souples : ils s'appliquent après la prochaine publication approuvée",
                    combined: "Chacun est plus strict sur une valeur : les deux s'appliquent",
                },
                defaults: {
                    published: "Publié le {{published_at}}",
                    nothingPublished: "Rien n'est publié : les valeurs par défaut s'appliquent",
                    noSnapshot:
                        "Publié le {{published_at}}, avant que les publications ne conservent ces paramètres : les valeurs par défaut s'appliquent",
                },
            },
            nextStep: {
                initialize:
                    "Terminez l’initialisation requise. Le planificateur réessaiera avant la clôture du vote.",
                closed: "Cette ouverture ne sera pas exécutée après la clôture du vote.",
                none: "Aucune action nécessaire.",
                publishAndApprove: "Publiez et approuvez la configuration.",
                requireConfigurationApproval:
                    "Faites en sorte que Approuver la configuration nécessite des signatures, puis publiez et approuvez la configuration.",
                askSignersToOpen: "Demandez aux signataires de l'élection d'ouvrir le vote.",
                askSignersToClose: "Demandez aux signataires de l'élection de clôturer le vote.",
            },
            applies: {
                tightens: "S'applique dès maintenant aux actions manuelles et planifiées.",
                loosens:
                    "S'applique dès maintenant aux actions manuelles ; aux ouvertures et clôtures planifiées après la prochaine publication approuvée.",
                tightensAndLoosens:
                    "Sa partie plus stricte s'applique dès maintenant aux actions manuelles et planifiées ; sa partie plus souple s'applique dès maintenant aux actions manuelles, et aux ouvertures et clôtures planifiées après la prochaine publication approuvée.",
            },
        },
        messagingEvent: {
            tab: "Messagerie",
            intro: "Les canaux que les électeurs de cet événement peuvent choisir pour les codes et les avis, et le compte depuis lequel chacun envoie. Les comptes se gèrent dans Paramètres > Messagerie.",
            readOnly:
                "Vous pouvez consulter ces paramètres. Les modifier nécessite l'autorisation messaging-config-write.",
            savingNote:
                "L'enregistrement met aussi à jour les canaux que les pages d'inscription proposent pour chaque Post.",
            save: "Enregistrer",
            saved: "Paramètres de messagerie enregistrés.",
            saveRejected:
                "Les paramètres de messagerie n'ont pas été enregistrés. Corrigez les problèmes indiqués.",
            saveError: "Impossible d'enregistrer les paramètres de messagerie.",
            accountLabel: "Compte {{channel}}",
            notUsed: "Non utilisé",
            missingAccount: "Compte introuvable",
            noAccount: "Ajoutez d'abord un compte dans Paramètres > Messagerie",
            missing: "Manque : {{blockers}}",
            purposeSwitch: "{{channel}} : {{purpose}}",
            sections: {
                channels: "Canaux",
                templates: "Modèles approuvés",
                fallback: "Ordre de repli pour les avis",
                posts: "Canaux par Post",
                postsCount: "Canaux par Post ({{count}} Posts)",
                reply: "Réponse aux messages entrants",
                delivery: "État de la remise",
            },
            column: {
                channel: "Canal",
                account: "Envoie depuis",
                purpose: "Usage",
                language: "Langue",
                template: "Modèle du fournisseur",
                status: "État",
                post: "Post",
                key: "Pour le message",
                providerLanguage: "Langue du fournisseur",
            },
            outOfWindow: {
                label: "Hors de la fenêtre de conversation",
                help: "Les avis en texte libre ne sont envoyés que tant que la fenêtre de conversation est ouverte : sur Messenger, dans les 24 heures suivant le dernier message de l'électeur. Choisissez Messages utilitaires pour envoyer des avis après ce délai avec un modèle approuvé. Meta doit l'approuver pour la Page (l'autorisation page_utility_messaging et un modèle UTILITY approuvé), et ce modèle doit être lié aux avis dans Modèles approuvés. Avec Ne pas envoyer, un avis hors de la fenêtre passe au canal disponible suivant de l'électeur.",
                DISABLED: "Ne pas envoyer",
                UTILITY_MESSAGES: "Messages utilitaires",
                noTemplate:
                    "Aucun modèle n'est encore lié aux avis sur ce canal. Ajoutez-en un dans Modèles approuvés ; d'ici là, les avis hors de la fenêtre passent au canal disponible suivant de l'électeur.",
            },
            templates: {
                empty: "Choisissez un compte qui envoie des modèles approuvés, comme WhatsApp, Viber ou Messenger, pour lier ses modèles ici.",
                help: "Chaque ligne indique quel modèle approuvé le fournisseur envoie pour un message. Pour le message est l'alias d'un modèle de Modèles, pour une notification, ou la clé de message qu'envoie Keycloak, comme otp ; laissez-le vide pour le modèle utilisé par défaut pour l'usage. Langue est la langue de l'électeur. Modèle du fournisseur est le nom ou l'ID du modèle chez le fournisseur. Langue du fournisseur est le code du fournisseur pour ce modèle lorsqu'il diffère de la langue de l'électeur : WhatsApp exige le code exact du modèle approuvé, comme en_US.",
                order: "Pour chaque message, la ligne la plus précise l'emporte : la ligne du message dans la langue de l'électeur, puis la ligne du message dans n'importe quelle langue, puis le modèle par défaut de l'usage dans la langue de l'électeur, puis n'importe quel modèle par défaut de l'usage.",
                noneRequired:
                    "{{channel}} n'envoie que des modèles approuvés. Ajoutez au moins un modèle par défaut pour chaque usage utilisé.",
                noneOptional:
                    "Aucun modèle n'est lié pour {{channel}}. Ils ne sont nécessaires que pour envoyer des avis hors de la fenêtre de conversation.",
                row: "Modèle {{channel}} {{position}}",
                keyDefault: "Par défaut pour l'usage",
                add: "Ajouter un modèle {{channel}}",
                remove: "Retirer le modèle {{channel}} {{position}}",
                incomplete:
                    "Saisissez la langue et le modèle du fournisseur, ou retirez cette ligne.",
                approval: {
                    APPROVED: "Approuvé",
                    NOT_APPROVED: "Non approuvé",
                    ADMIN_CONFIRMED: "Confirmé par un administrateur",
                    NOT_CHECKED: "Approbation non vérifiée",
                },
            },
            fallback: {
                help: "Lorsqu'un avis ne peut pas atteindre un électeur sur son canal, il passe au canal suivant de cet ordre que l'électeur a vérifié et que son Post propose. Les codes ne sont jamais renvoyés d'eux-mêmes : l'électeur choisit un autre moyen.",
                empty: "Activez les avis d'un canal pour l'ajouter à l'ordre de repli.",
                earlier: "Déplacer {{channel}} plus tôt",
                later: "Déplacer {{channel}} plus tard",
            },
            posts: {
                noChannels:
                    "Activez les codes ou les avis d'un canal pour choisir les canaux de chaque Post.",
                help: "L'inscription montre aux électeurs de chaque Post les canaux cochés ici.",
                restricted: "{{count}} Posts proposent moins que les {{total}} canaux.",
                allChannels:
                    "Chaque Post propose les {{total}} canaux ; décochez un canal pour un Post où il ne fonctionne pas.",
                search: "Rechercher des Posts",
                cell: "{{post}} : {{channel}}",
                showing:
                    "{{shown}} Posts affichés sur {{total}}. Recherchez pour trouver les autres.",
            },
            reply: {
                help: "Envoyée lorsqu'un électeur écrit à l'un des comptes de cet événement, au plus une fois par jour et par électeur.",
                label: "Réponse ({{language}})",
            },
            delivery: {
                empty: "Aucun canal n'est utilisé.",
                help: "Accepté signifie que le fournisseur a accepté la demande, pas que l'électeur a reçu ou vérifié le code. Inconnu signifie que la remise n'est pas encore confirmée. Un fournisseur sans rapports de remise affiche la remise comme non disponible.",
            },
            error: {
                UNSUPPORTED_VERSION:
                    "Cette configuration utilise la version {{version}}, qui n'est pas prise en charge.",
                DUPLICATE_CHANNEL: "{{channel}} est configuré plusieurs fois.",
                UNKNOWN_ACCOUNT: "Le compte {{channel}} n'existe plus. Choisissez un autre compte.",
                ACCOUNT_OF_ANOTHER_TENANT: "Le compte sélectionné appartient à un autre locataire.",
                ACCOUNT_CHANNEL_MISMATCH:
                    "Le compte sélectionné n'envoie pas de messages {{channel}}.",
                PURPOSE_NOT_READY:
                    "{{channel}} ne peut pas encore envoyer de {{purpose}}. Manque : {{blockers}}.",
                TEMPLATE_NOT_APPROVED:
                    "Le modèle {{channel}} pour {{purpose}} en {{language}} n'est pas approuvé par le fournisseur.",
                OUT_OF_WINDOW_NOT_SUPPORTED:
                    "{{channel}} ne peut pas envoyer hors d'une fenêtre de conversation avec ce compte : il n'a pas de fenêtre de conversation, ou ses avis nécessitent déjà un modèle.",
                FALLBACK_CHANNEL_NOT_ENABLED:
                    "{{channel}} est dans l'ordre de repli mais n'envoie pas d'avis.",
                DUPLICATE_FALLBACK_CHANNEL:
                    "{{channel}} figure plusieurs fois dans l'ordre de repli.",
                ELECTION_CHANNEL_NOT_ENABLED:
                    "{{election}} propose {{channel}}, que cet événement n'utilise pas.",
                UNKNOWN_ELECTION: "{{election}} n'est pas une élection de cet événement.",
            },
        },
        messaging: {
            channel: {
                EMAIL: "E-mail",
                SMS: "SMS",
                WHATSAPP: "WhatsApp",
                VIBER: "Viber",
                MESSENGER: "Facebook Messenger",
            },
            provider: {
                AWS_SES: "Amazon SES",
                SMTP: "Serveur SMTP",
                AWS_SNS: "Amazon SNS",
                WHATSAPP_CLOUD_API: "WhatsApp Cloud API (Meta)",
                MESSENGER_SEND_API: "Messenger Platform (Meta)",
                VIBER_INFOBIP: "Viber Business Messages (Infobip)",
                CONSOLE: "Console (test uniquement, rien n'est envoyé)",
                HTTP_API: "API HTTP personnalisée",
            },
            purpose: {
                OTP: "Codes",
                NOTICE: "Avis",
            },
            state: {
                QUEUED: "En file",
                ACCEPTED: "Accepté",
                DELIVERED: "Remis",
                FAILED: "Échec",
                UNKNOWN: "Inconnu",
            },
            stateHelp: {
                QUEUED: "En attente de transmission au fournisseur.",
                ACCEPTED:
                    "Le fournisseur a accepté le message. Cela ne signifie pas que l'électeur l'a reçu.",
                DELIVERED: "Le fournisseur a signalé le message comme remis.",
                FAILED: "Le fournisseur a confirmé que le message n'a pas été remis.",
                UNKNOWN: "La remise n'est pas encore confirmée.",
            },
            blocker: {
                NOT_CONNECTED: "Non connecté",
                UNSUPPORTED_PURPOSE: "Non pris en charge par ce fournisseur",
                NEEDS_PROVIDER_APPROVAL: "Nécessite l'approbation du fournisseur",
                NEEDS_PRODUCTION_ACCESS: "Nécessite un accès de production",
                NEEDS_APPROVED_TEMPLATE: "Nécessite un modèle approuvé",
            },
            readiness: {
                connected: "Connecté",
                notConnected: "Non connecté",
                readyOtp: "Prêt pour les codes",
                readyNotice: "Prêt pour les avis",
                notReady: "Pas prêt",
                lastCheck: "Vérifié {{date}}",
                neverChecked: "Pas encore vérifié",
                adminConfirmed: "Confirmé par un administrateur",
                checkNotUsed: "Vérification non utilisée",
            },
            approval: {
                PENDING: "Approbation du fournisseur en attente",
                CONFIRMED: "Approbation du fournisseur confirmée",
            },
            credential: {
                ACCESS_TOKEN: "Jeton d'accès",
                APP_SECRET: "Secret de l'application",
                VERIFY_TOKEN: "Jeton de vérification",
                API_KEY: "Clé d'API",
                SMTP_PASSWORD: "Mot de passe",
                AWS_ACCESS_KEY_ID: "ID de clé d'accès AWS",
                AWS_SECRET_ACCESS_KEY: "Clé d'accès secrète AWS",
                API_SECRET: "Secret de l'API",
                USERNAME: "Nom d'utilisateur",
                PASSWORD: "Mot de passe",
                WEBHOOK_SECRET: "Secret du webhook",
            },
            deliveryUnavailable: "Remise non disponible",
            templates: {
                noMethod: "Choisissez au moins une méthode pour le modèle.",
                parameters: "Paramètres du modèle",
                parametersHelp:
                    "Ce qui remplit chaque espace réservé du modèle approuvé, dans l'ordre, par exemple user.first_name ou vote_url. Pour un modèle à paramètres nommés, écrivez @nom=valeur, par exemple @first_name=user.first_name ; toute autre entrée est positionnelle.",
                parameter: "Paramètre {{position}}",
                removeParameter: "Retirer le paramètre {{position}}",
                addParameter: "Ajouter un paramètre",
                noAccount:
                    "Il n'y a pas encore de compte {{channel}}. Ajoutez-en un dans Paramètres > Messagerie pour voir quelles langues sont approuvées.",
                account: "Compte",
                approvalTitle: "Modèles approuvés",
                language: "Langue",
                approvalFor: "Approuvé pour {{purpose}}",
                approved: "Approuvé",
                notApproved: "Non approuvé",
                approvalHelp:
                    "Les approbations viennent du fournisseur et sont mises à jour par la vérification de connexion du compte.",
                messengerIntro:
                    "Dans les 24 heures qui suivent le dernier message de l'électeur, Messenger envoie le texte ci-dessous.",
                messengerMessage: "Message dans les 24 heures",
                messengerWindow:
                    "Un destinataire Messenger enregistré n'est pas une autorisation d'envoi. En dehors de la fenêtre de 24 heures, cet avis est envoyé comme message utilitaire lorsque l'événement électoral le permet et qu'un modèle approuvé est indiqué ci-dessous ou lié dans l'événement ; sinon, il passe au canal disponible suivant de l'électeur. Les messages utilitaires nécessitent l'autorisation page_utility_messaging et un modèle UTILITY approuvé sur la Page.",
                intro: {
                    WHATSAPP:
                        "WhatsApp n'envoie que les modèles approuvés par Meta pour le compte WhatsApp Business. Le message doit correspondre au modèle approuvé ; choisissez ce qui remplit ses paramètres.",
                    VIBER: "Viber n'envoie les codes et messages transactionnels qu'avec des modèles approuvés par le partenaire Viber. Le message doit correspondre au modèle approuvé ; choisissez ce qui remplit ses paramètres.",
                },
                approvedWording: "Texte approuvé",
                approvedWordingHelp:
                    "Une copie du modèle approuvé, utilisée comme aperçu. La modifier ici ne change pas ce que le fournisseur envoie.",
                providerTemplateTitle: "Modèle du fournisseur",
                providerTemplateHelp:
                    "Facultatif. Le nom ou l'ID du modèle approuvé chez le fournisseur. S'il est vide, le modèle de l'événement électoral lié à l'alias de ce modèle est utilisé, ou le modèle par défaut de l'événement pour l'usage.",
                providerTemplate: "Nom ou ID du modèle du fournisseur",
                providerLanguage: "Code de langue du fournisseur",
                providerLanguageHelp: {
                    WHATSAPP: "Le code de langue exact du modèle WhatsApp approuvé, comme en_US.",
                    VIBER: "Le code de langue sous lequel le fournisseur Viber connaît le modèle, lorsqu'il en faut un.",
                    MESSENGER: "Le code de langue du modèle utilitaire approuvé, comme en_US.",
                },
                approvalAdminConfirmed:
                    "Un administrateur a confirmé auprès du fournisseur que les modèles de ce compte sont approuvés ; les approbations de la vérification de connexion ne sont donc pas utilisées.",
            },
            send: {
                channel: "Canal",
                eachVoter: "Le canal de chaque électeur",
                only: "{{channel}} uniquement",
                eachVoterHelp:
                    "Les échecs confirmés utilisent le canal vérifié disponible suivant. Une remise non confirmée est affichée comme Inconnue.",
                onlyHelp: "Cette notification est envoyée à chaque électeur par {{channel}}.",
                channelColumn: "Canal",
                sendsFrom: "Envoyé depuis",
                noAccount: "Aucun compte",
                missingContent: "Cette notification n'a pas de contenu pour {{channels}}.",
                approvedTemplateHelp:
                    "Envoyé avec le modèle approuvé par le fournisseur. Modifiez-le dans Modèles.",
                providerTemplate: "Modèle du fournisseur {{channel}}",
                providerTemplateHelp:
                    "Facultatif. S'il est vide, le modèle de l'événement lié à l'alias du modèle choisi est utilisé, ou le modèle par défaut de l'événement pour les avis.",
                providerLanguage: "Langue du fournisseur {{channel}}",
                providerLanguageHelp:
                    "Le code de langue du fournisseur pour ce modèle, comme en_US.",
            },
            voter: {
                title: "Messagerie",
                preferredChannel: "Canal préféré",
                whatsappNumber: "Numéro WhatsApp",
                viberNumber: "Numéro Viber",
                messengerConnected: "Connecté",
                messengerNotConnected: "Non connecté",
                verifiedChannels: "Canaux vérifiés",
                noneVerified: "Aucun canal vérifié",
                notSet: "Non défini",
            },
            logs: {
                channel: "Canal",
            },
            stats: {
                sent: {
                    WHATSAPP: "Messages WhatsApp envoyés",
                    VIBER: "Messages Viber envoyés",
                    MESSENGER: "Messages Messenger envoyés",
                },
            },
            readinessPolicy: {
                PROVIDER_CHECK: "D'après la vérification du fournisseur",
                ADMIN_CONFIRMED: "Confirmé par un administrateur",
            },
        },
        messagingAccounts: {
            tab: "MESSAGERIE",
            description:
                "Comptes qui envoient aux électeurs leurs codes et avis. Chaque événement électoral choisit le compte de chaque canal ; les nouveaux événements commencent avec le compte par défaut.",
            list: {
                title: "Comptes d'envoi",
                add: "Ajouter un compte",
                loading: "Chargement des comptes",
                loadError: "Les comptes d'envoi n'ont pas pu être chargés.",
                empty: "Aucun compte d'envoi pour le moment.",
            },
            column: {
                channel: "Canal",
                name: "Compte",
                sender: "Envoie en tant que",
                provider: "Fournisseur",
                default: "Par défaut",
                isDefault: "Compte par défaut",
                lastCheck: "Dernière vérification",
                actions: "Actions",
            },
            action: {
                edit: "Modifier",
                editNamed: "Modifier {{name}}",
                view: "Afficher",
                viewNamed: "Afficher {{name}}",
                check: "Vérifier la connexion",
                checkNamed: "Vérifier la connexion de {{name}}",
                test: "Envoyer un message de test",
                testNamed: "Envoyer un message de test depuis {{name}}",
                delete: "Supprimer",
                deleteNamed: "Supprimer {{name}}",
            },
            check: {
                done: "{{name}} a été vérifié. Son état est à jour.",
                error: "{{name}} n'a pas pu être vérifié.",
            },
            delete: {
                title: "Supprimer le compte",
                body: "Supprimer {{name}} ? Les événements électoraux qui l'utilisent cesseront d'envoyer sur son canal.",
                success: "Compte supprimé",
                error: "Le compte n'a pas pu être supprimé.",
            },
            editor: {
                addTitle: "Ajouter un compte",
                editTitle: "Modifier le compte {{channel}}",
                subtitle:
                    "Les électeurs reçoivent les codes et avis de ce compte sur les canaux qui l'utilisent.",
                channel: "Canal",
                provider: "Fournisseur",
                save: "Enregistrer",
                cancel: "Annuler",
                close: "Fermer",
                channelHelp: "Ne peut pas être modifié après la création du compte.",
            },
            field: {
                name: "Nom du compte",
                from_address: "Adresse de l'expéditeur",
                from_name: "Nom de l'expéditeur",
                region: "Région AWS",
                notification_topic_arn: "Rubrique des notifications de remise (ARN SNS)",
                server_url: "Serveur et port",
                sender_id: "ID d'expéditeur",
                origination_number: "Numéro d'origine",
                business_account_id: "ID du compte WhatsApp Business",
                phone_number_id: "ID du numéro de téléphone",
                display_phone_number: "Numéro",
                display_name: "Nom affiché",
                api_version: "Version de Graph API",
                page_id: "ID de la page Facebook",
                page_name: "Nom de la page",
                page_username: "Nom d'utilisateur de la page",
                base_url: "URL de base de l'API",
                sender: "Nom de l'expéditeur",
                provider_approval: "Approbation du fournisseur",
                is_default: "Compte {{channel}} par défaut pour les nouveaux événements électoraux",
                readiness: "Disponibilité",
                api_base_url: "URL de base de Graph API",
                label: "Expéditeur affiché aux électeurs",
            },
            fieldHelp: {
                from_address:
                    "L'adresse que voient les électeurs. Son domaine doit être vérifié auprès du fournisseur.",
                notification_topic_arn:
                    "La rubrique SNS où SES publie les événements de remise et de rebond. Les notifications de toute autre rubrique sont refusées.",
                sender_id:
                    "Jusqu'à 11 lettres et chiffres. Certains pays exigent un enregistrement.",
                origination_number:
                    "Utilisé à la place de l'ID d'expéditeur lorsqu'un pays exige un numéro.",
                phone_number_id: "Le numéro depuis lequel les messages sont envoyés.",
                display_name: "Le nom affiché que Meta a approuvé pour le numéro.",
                page_username:
                    "Utilisé pour le lien m.me que les électeurs ouvrent pour obtenir leur code.",
                api_version: "Par exemple v23.0.",
                base_url: "L'URL de base de l'API Infobip du compte.",
                sender: "L'expéditeur approuvé que voient les électeurs.",
                provider_approval:
                    "Meta n'autorise les messages WhatsApp des gouvernements que dans le cadre d'un accord approuvé. Choisissez Approbation du fournisseur confirmée une fois que Meta l'a approuvé pour ce compte ; d'ici là, les codes et les avis ne peuvent pas être activés pour ce compte.",
                readiness:
                    "D'après la vérification du fournisseur utilise ce que trouve la vérification de connexion : si le compte est connecté, en production, et quels modèles sont approuvés. Confirmé par un administrateur est destiné aux fournisseurs dont la vérification ne peut pas le déterminer : c'est votre déclaration que le compte est connecté, en production et que ses modèles sont approuvés, et elle est utilisée à la place de la vérification.",
                api_base_url:
                    "Uniquement lorsque Graph API n'est pas celle de Meta, comme le point d'accès d'un fournisseur de solutions. Vide utilise celle de Meta.",
                label: "Le nom que les électeurs voient comme expéditeur de ce compte.",
            },
            error: {
                REQUIRED: "Obligatoire",
                NOT_A_COUNT: "Saisissez un nombre entier",
                OTP_ABOVE_TOTAL: "Ne peut pas dépasser les messages par seconde",
                INVALID_CALLING_CODE:
                    "Saisissez des indicatifs téléphoniques de pays de 1 à 3 chiffres, comme 63",
                DUPLICATE_LANGUAGE: "Cette langue a déjà un modèle pour cet usage",
                NOT_A_URL: "Saisissez une adresse commençant par https:// ou http://",
                INVALID_HTTP_CONFIG: "Corrigez les problèmes indiqués",
            },
            warning: {
                pageChange:
                    "Les conversations Messenger appartiennent à une page. Après le changement de page, les électeurs connectés à {{page}} ne recevront des codes qu'après avoir reconnecté Messenger.",
                numberChange:
                    "Les messages viendront d'un autre numéro. Ses modèles doivent être approuvés dans ce compte professionnel avant de pouvoir envoyer des codes, et les électeurs verront une nouvelle discussion.",
            },
            viber: {
                title: "Modèles approuvés",
                description:
                    "Saisissez les modèles que Viber a approuvés par l'intermédiaire du partenaire, par usage et par langue. L'API de modèles du partenaire n'est pas disponible : cette liste est tenue à la main et la vérification de connexion la lit.",
                purpose: "Usage",
                language: "Langue",
                templateId: "ID du modèle chez le partenaire",
                add: "Ajouter un modèle",
                remove: "Retirer le modèle",
            },
            limits: {
                title: "Limites d'envoi",
                messagesPerSecond: "Messages par seconde",
                otpReservedPerSecond: "Réservés aux codes par seconde",
                otpReservedHelp: "Gardés libres pour les codes pendant les envois en masse.",
                allowedCallingCodes: "Destinations autorisées (indicatifs téléphoniques de pays)",
                allowedCallingCodesHelp:
                    "Séparés par des virgules, par exemple 63, 971. Vide autorise toute destination.",
            },
            credentials: {
                title: "Identifiants",
                description:
                    "Les identifiants sont en écriture seule : après l'enregistrement, seule la date de leur dernier remplacement est affichée.",
                set: "Défini · remplacé {{date}}. Il est stocké chiffré et n'est jamais affiché.",
                replace: "Remplacer",
                replaceNamed: "Remplacer {{name}}",
            },
            credentialHelp: {
                AWS_SES: {
                    AWS_ACCESS_KEY_ID:
                        "Facultatif. Sans clés, le rôle propre du service est utilisé.",
                    AWS_SECRET_ACCESS_KEY: "Facultatif. À définir avec l'ID de clé d'accès.",
                },
                AWS_SNS: {
                    AWS_ACCESS_KEY_ID:
                        "Facultatif. Sans clés, le rôle propre du service est utilisé.",
                    AWS_SECRET_ACCESS_KEY: "Facultatif. À définir avec l'ID de clé d'accès.",
                },
                SMTP: {
                    SMTP_PASSWORD: "Le mot de passe du serveur SMTP.",
                },
                WHATSAPP_CLOUD_API: {
                    ACCESS_TOKEN:
                        "Un jeton d'un utilisateur système du portefeuille professionnel du propriétaire, avec whatsapp_business_messaging.",
                    APP_SECRET: "Vérifie que les appels du webhook proviennent de Meta.",
                },
                MESSENGER_SEND_API: {
                    ACCESS_TOKEN: "Un jeton d'accès de page avec pages_messaging.",
                    APP_SECRET: "Vérifie que les appels du webhook proviennent de Meta.",
                },
                VIBER_INFOBIP: {
                    API_KEY: "La clé d'API Infobip.",
                },
                HTTP_API: {
                    API_KEY: "Facultatif. Les requêtes l'utilisent comme identifiant API_KEY.",
                    API_SECRET:
                        "Facultatif. Un second secret, et la clé qui signe le JWT : une clé privée PEM pour RS256, le secret partagé pour HS256.",
                    ACCESS_TOKEN:
                        "Facultatif. Les requêtes l'utilisent comme identifiant ACCESS_TOKEN.",
                    USERNAME:
                        "Facultatif. Avec le mot de passe, il forme l'espace réservé basic_auth.",
                    PASSWORD:
                        "Facultatif. Avec le nom d'utilisateur, il forme l'espace réservé basic_auth.",
                    WEBHOOK_SECRET:
                        "Facultatif. Le secret partagé avec lequel les rappels du fournisseur sont vérifiés.",
                },
            },
            webhook: {
                title: "Rapports de remise et réponses",
                description:
                    "Saisissez ce rappel dans les paramètres de webhook du fournisseur. Les rapports de remise et les réponses des électeurs y arrivent.",
                path: "Chemin de rappel",
                pathHelp:
                    "Ajoutez-le à l'adresse publique des webhooks de messagerie de cette plateforme.",
                afterSaving: "Affiché après l'enregistrement",
                copyPath: "Copier le chemin de rappel",
                tokenSet: "Défini · remplacé {{date}}",
                tokenMissing: "Pas encore généré",
                tokenAfterSaving: "Généré après l'enregistrement",
                generate: "Générer un jeton de vérification",
                tokenTitle: "Jeton de vérification",
                tokenOnce:
                    "Saisissez maintenant ce jeton dans les paramètres de webhook de Meta. Il n'est affiché qu'une fois.",
                copyToken: "Copier le jeton de vérification",
                tokenDone: "Terminé",
                tokenError: "Le jeton de vérification n'a pas pu être généré.",
                httpHelp:
                    "Une API HTTP personnalisée peut publier ses rapports en JSON, ou les envoyer par une requête GET ; ses paramètres de requête sont alors lus comme un objet plat, avec des pointeurs comme /status.",
            },
            copy: {
                success: "Copié",
                error: "Impossible de copier",
            },
            save: {
                success: "Compte enregistré",
                error: "Le compte n'a pas pu être enregistré.",
            },
            test: {
                title: "Envoyer un message de test depuis {{name}}",
                description:
                    "Envoie un vrai message pour l'usage choisi à cette destination. Le résultat montre ce que le fournisseur a signalé.",
                purpose: "Usage",
                destination: {
                    EMAIL_ADDRESS: "Adresse e-mail",
                    PHONE_NUMBER: "Numéro de téléphone (E.164)",
                    PAGE_SCOPED_ID: "ID propre à la page",
                },
                language: "Langue",
                send: "Envoyer un message de test",
                reason: "Motif : {{reason}}",
                error: "Le message de test n'a pas pu être envoyé.",
                template: "Modèle approuvé",
                templateHelp:
                    "Le nom ou l'ID du modèle que le fournisseur a approuvé pour cet usage et cette langue.",
                viberTemplate:
                    "Viber utilise le modèle que ce compte indique comme approuvé pour l'usage et la langue choisis.",
                languageHelp:
                    "Pour un fournisseur qui envoie des modèles approuvés, saisissez le code de langue du fournisseur pour le modèle, comme en_US.",
            },
            http: {
                title: "API HTTP personnalisée",
                description:
                    "Décrit un fournisseur par ses requêtes HTTP : un autre partenaire Viber, l'API propre d'un fournisseur de solutions WhatsApp, une passerelle SMS. Les requêtes sont en JSON ; leur URL, leurs en-têtes et leur corps peuvent contenir les espaces réservés de la référence ci-dessous.",
                phoneFormat: "Format du numéro de téléphone",
                phoneFormatHelp:
                    "Comment le numéro de téléphone du destinataire est écrit dans une requête.",
                phoneFormatOption: {
                    E164: "Avec le signe plus : +639171234567",
                    DIGITS: "Chiffres uniquement : 639171234567",
                },
                templateRequired: "Usages qui nécessitent un modèle approuvé",
                templateRequiredHelp:
                    "Un usage coché n'est envoyé qu'avec un modèle approuvé par le fournisseur, lié dans l'événement électoral. Les autres usages sont envoyés en texte libre.",
                approvedLanguages: "Langues avec un modèle approuvé pour {{purpose}}",
                approvedLanguagesHelp:
                    "Les codes de langue ayant un modèle approuvé, tels que confirmés auprès du fournisseur, séparés par des virgules : en, tl. La vérification de connexion les signale.",
                conversationWindow: "Fenêtre de conversation (heures)",
                conversationWindowHelp:
                    "Heures après le dernier message du destinataire pendant lesquelles du texte libre peut être envoyé. Vide lorsque le fournisseur n'a pas de telle fenêtre.",
                messageIdPointer: "ID du message dans la réponse d'envoi",
                messageIdPointerHelp:
                    "Un pointeur JSON vers l'ID de message du fournisseur dans la réponse à la requête d'envoi, comme /message_id. Il sert à associer les rapports de remise.",
                notConfigured: "Non configuré.",
                thisSection: "Cette section",
                add: "Ajouter : {{section}}",
                remove: "Retirer : {{section}}",
                section: {
                    SEND: "Requête d'envoi",
                    CHECK: "Requête de vérification de connexion",
                    TOKEN: "Requête de jeton",
                    JWT: "Jeton signé (JWT)",
                    REPORTS: "Rapports de remise et réponses",
                    RECONCILE: "Requête de recherche d'un message",
                },
                sectionHelp: {
                    SEND: "La requête qui envoie un message : method (POST si omis), url, headers et body.",
                    CHECK: "Facultatif. Une requête qui réussit, avec une réponse 2xx, lorsque les identifiants fonctionnent. La vérification de connexion l'exécute.",
                    TOKEN: "Facultatif. Obtient un jeton de courte durée avant l'envoi, comme les identifiants client OAuth : request, token_pointer (où se trouve le jeton dans la réponse) et lifetime_seconds. Les requêtes l'utilisent avec l'espace réservé token.",
                    JWT: "Facultatif. Un jeton signé pour chaque requête avec l'identifiant Secret de l'API : algorithm (RS256 ou HS256), claims (iat, exp et jti sont ajoutés) et lifetime_seconds. Les requêtes l'utilisent avec l'espace réservé jwt.",
                    REPORTS:
                        "Facultatif. Comment lire ce que le fournisseur envoie au rappel : auth, items_pointer (où se trouve la liste des rapports ; tout le contenu si omis), status (message_id_pointer, state_pointer, states, qui associe chaque valeur du fournisseur à QUEUED, ACCEPTED, DELIVERED, FAILED ou UNKNOWN, et error_pointer) et inbound_from_pointer (où se trouve l'expéditeur d'une réponse). auth a un kind : URL_KEY (uniquement l'adresse secrète du rappel), HEADER_SECRET (un en-tête égal au secret du webhook), HMAC_SHA256 (un en-tête avec le HMAC du corps calculé avec le secret du webhook, avec prefix, encoding HEX ou BASE64, et signed lorsque la signature couvre plus que le corps) ou JWT_HS256 (un en-tête avec un JWT bearer signé avec le secret du webhook).",
                    RECONCILE:
                        "Facultatif. Interroge le fournisseur sur un message dont le résultat est inconnu : request et status, qui se lit comme le status des rapports de remise.",
                },
                problem: {
                    NOT_AN_OBJECT: "{{path}} doit être un objet.",
                    MISSING_URL: "{{path}} est obligatoire : l'adresse de la requête.",
                    INVALID_METHOD: "{{path}} doit être une méthode HTTP, comme POST ou GET.",
                    INVALID_HEADERS:
                        "{{path}} doit être du texte : headers est un objet de noms d'en-tête et de valeurs textuelles.",
                    UNKNOWN_FIELD: "{{path}} n'est pas un champ de cette section.",
                    UNKNOWN_PLACEHOLDER:
                        "{{path}} utilise un espace réservé qui n'existe pas. Consultez la référence des espaces réservés.",
                    INVALID_POINTER:
                        "{{path}} doit être un pointeur JSON commençant par /, comme /data/id.",
                    INVALID_STATES:
                        "{{path}} doit associer une valeur d'état du fournisseur à QUEUED, ACCEPTED, DELIVERED, FAILED ou UNKNOWN ; il en faut au moins une.",
                    INVALID_AUTH:
                        "{{path}} n'est pas valide : kind est URL_KEY, HEADER_SECRET, HMAC_SHA256 ou JWT_HS256 ; header est obligatoire sauf pour URL_KEY ; encoding est HEX ou BASE64.",
                    INVALID_LIFETIME:
                        "{{path}} doit être un nombre entier de secondes supérieur à 0.",
                    INVALID_ALGORITHM: "{{path}} doit être RS256 ou HS256.",
                    INVALID_CLAIMS: "{{path}} doit être un objet.",
                    INVALID_HOURS: "{{path}} doit être un nombre entier d'heures supérieur à 0.",
                },
                placeholders: {
                    title: "Référence des espaces réservés",
                    help: "Écrits entre doubles accolades dans l'URL, dans la valeur d'un en-tête ou dans tout texte du corps. Chacun est remplacé au moment de la requête.",
                },
                placeholder: {
                    to: "Le destinataire : numéro de téléphone, adresse e-mail ou ID spécifique à la Page.",
                    text: "Le message en texte brut.",
                    subject: "L'objet, pour l'e-mail.",
                    html: "Le message en HTML, pour l'e-mail.",
                    code: "Le code à usage unique, pour les codes.",
                    template: "Le modèle du fournisseur lié dans l'événement électoral.",
                    language: "Le code de langue du fournisseur pour le modèle.",
                    message_id:
                        "L'ID de message du fournisseur, dans une requête de recherche d'un message.",
                    callback_url: "L'adresse publique du rappel de ce compte.",
                    param: "Un paramètre du modèle selon sa position : 1, 2, 3, etc.",
                    credential:
                        "Un identifiant de ce compte par son nom : API_KEY, API_SECRET, ACCESS_TOKEN, USERNAME, PASSWORD ou WEBHOOK_SECRET.",
                    basic_auth:
                        "Le nom d'utilisateur et le mot de passe, encodés pour un en-tête Authorization: Basic.",
                    token: "Le jeton obtenu avec la requête de jeton.",
                    jwt: "Le jeton signé décrit dans Jeton signé (JWT).",
                    parameters:
                        "Seul comme valeur du corps, il devient la liste de tous les paramètres du modèle.",
                    named_parameters:
                        "Seul comme valeur du corps, il devient un objet des paramètres écrits sous la forme @nom=valeur.",
                },
                example: {
                    title: "Exemple complet : un partenaire Viber",
                    description:
                        "Le partenaire reçoit un POST JSON authentifié avec la clé d'API comme jeton bearer, répond avec l'ID du message dans message_id et publie les rapports de remise avec un en-tête secret. Utilisez-le comme point de départ et remplacez l'adresse et les noms de champ par ceux du fournisseur.",
                    use: "Utiliser cet exemple",
                },
            },
        },
    },
}

export default frenchTranslation
