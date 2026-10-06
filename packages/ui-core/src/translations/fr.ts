// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
        a11y: {
            closeDialog: "Fermer la boîte de dialogue",
            dismissMessage: "Masquer le message",
            ballotIdHelp: "À propos de votre identifiant de vote",
            loading: "Chargement",
            severity: {
                error: "Erreur",
                warning: "Avertissement",
                success: "Succès",
                info: "Information",
            },
            selectList: "Sélectionner toute la liste",
            preferenceLabel: "Préférence",
            writeInFor: "Nom du candidat écrit",
        },
        candidate: {
            moreInformationLink: "Plus d'informations",
            writeInsPlaceholder: "Tapez ici le candidat par écrit",
            blankVote: "Vote blanc",
            preferential: {
                position: "Position",
                none: "Aucune",
                ordinals: {
                    first: "er",
                    second: "e",
                    third: "e",
                    other: "e",
                },
            },
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
        footer: {
            poweredBy: "Propulsé par <sequent />",
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
                overVoteDisabled:
                    "Maximum atteint : Vous avez sélectionné le maximum de {{numSelected}} choix. Pour changer votre sélection, veuillez d'abord désélectionner une autre option.",
                blankVote: "Vote Blanc: 0 options sélectionnées",
                preferenceOrderWithGaps:
                    "Vote invalide! L'ordre de préférence comporte un ou plusieurs trous.",
                duplicatedPosition:
                    "Vote invalide! La même position a été sélectionnée pour deux ou plusieurs candidats.",
            },
            explicit: {
                notAllowed:
                    "Vote marqué explicitement comme invalide mais la question ne le permet pas",
                alert: "La sélection marquée sera considérée comme un vote invalide.",
            },
            configuration: {
                multipleExplicitInvalidCandidates:
                    "Configuration du bulletin invalide : le scrutin définit {{count}} candidats explicitement invalides, mais un seul est autorisé.",
                multipleExplicitBlankCandidates:
                    "Configuration du bulletin invalide : le scrutin définit {{count}} candidats de vote blanc explicite, mais un seul est autorisé.",
            },
        },
        ballotHash: "Votre Localisateur de Vote : {{ballotId}}",
        version: {
            header: "Version :",
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
            importError: "Impossible d’importer ce fichier. Veuillez réessayer.",
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
        problems: {
            noProblems: "Rien à signaler. L'importation fonctionnerait.",
            errors_one: "{{count}} erreur",
            errors_other: "{{count}} erreurs",
            errorsExplained:
                "L'importation ne se fera pas tant que chacune de ces erreurs n'est pas corrigée.",
            warnings_one: "{{count}} avertissement",
            warnings_other: "{{count}} avertissements",
            warningsExplained:
                "L'importation se fera. Chacun de ces points ne correspond probablement pas à ce qui était voulu.",
            warningsStrict:
                "Le mode strict est activé, donc ces avertissements bloquent la génération.",
            messages: {
                area: {
                    "cycle": {
                        lead: "Zones imbriquées l'une dans l'autre",
                        text: "Zones imbriquées l'une dans l'autre — la zone '{{area}}' fait partie d'une boucle de parents, ce qui bloquerait le Portail d'administration.",
                    },
                    "duplicate-name": {
                        lead: "Deux zones portent le même nom",
                        text: "Deux zones portent le même nom — '{{name}}' désigne à la fois '{{first}}' et '{{second}}'. Le CSV des électeurs identifie une zone par son nom, donc les électeurs iraient dans celle que l'importateur trouve en premier.",
                    },
                    "early-voting-unknown": {
                        lead: "Paramètre de vote anticipé inconnu",
                        text: "Paramètre de vote anticipé inconnu — '{{value}}' n'est pas valide pour une zone. La plateforme accepte {{allowed}}.",
                    },
                    "empty-ballot": {
                        lead: "Zone avec un bulletin vide",
                        text: "Zone avec un bulletin vide — la zone '{{area}}' ne vote sur aucun scrutin, ni le sien ni un scrutin hérité d'une zone qui la contient, donc ses électeurs verraient un bulletin vide.",
                    },
                    "inside-itself": {
                        lead: "Zone à l'intérieur d'elle-même",
                        text: "Zone à l'intérieur d'elle-même — une zone ne peut pas être son propre parent.",
                    },
                    "no-identifier": {
                        lead: "Zone sans identifiant",
                        text: "Zone sans identifiant — chaque zone doit en avoir un.",
                    },
                    "no-name": {
                        lead: "Zone sans nom",
                        text: "Zone sans nom — le CSV des électeurs identifie la zone d'un électeur par son nom, pas par son id, donc aucun électeur ne peut être placé dans une zone sans nom.",
                    },
                    "parent-missing": {
                        lead: "Zone parente manquante",
                        text: "Zone parente manquante — la zone se trouve dans un parent qui ne figure pas dans ce fichier.",
                    },
                    "parent-unknown": {
                        lead: "Zone parente inconnue",
                        text: "Zone parente inconnue — rien n'a l'identifiant '{{parent}}'.",
                    },
                },
                ballot: {
                    "no-elections": {
                        lead: "Aucune élection",
                        text: "Aucune élection — un événement électoral en nécessite au moins une.",
                    },
                },
                bundle: {
                    "duplicate-id": {
                        lead: "Identifiant utilisé deux fois",
                        text: "Identifiant utilisé deux fois — {{id}} dans {{kind}} est aussi utilisé par {{previous}}.",
                    },
                    "event-no-encryption": {
                        lead: "Aucun protocole de chiffrement",
                        text: "Aucun protocole de chiffrement — l'événement électoral n'indique pas comment les bulletins sont chiffrés.",
                    },
                    "event-no-id": {
                        lead: "Événement sans id",
                        text: "Événement sans id — l'événement électoral de ce fichier n'a pas d'identifiant.",
                    },
                    "no-areas": {
                        lead: "Aucune zone",
                        text: "Aucune zone — aucun électeur ne peut recevoir de bulletin tant que l'événement n'a pas au moins une zone.",
                    },
                    "no-elections": {
                        lead: "Aucune élection",
                        text: "Aucune élection — un événement électoral nécessite au moins une élection.",
                    },
                    "no-tenant": {
                        lead: "Aucun tenant",
                        text: "Aucun tenant — le fichier n'indique pas à quel tenant il appartient.",
                    },
                    "tenant-not-uuid": {
                        lead: "Le tenant n'est pas un identifiant",
                        text: "Le tenant n'est pas un identifiant — '{{value}}' n'est pas un id de tenant valide.",
                    },
                },
                candidate: {
                    "contest-missing": {
                        lead: "Scrutin du candidat manquant",
                        text: "Scrutin du candidat manquant — le candidat appartient à un scrutin qui ne figure pas dans ce fichier.",
                    },
                    "no-contest": {
                        lead: "Candidat sans scrutin",
                        text: "Candidat sans scrutin — chaque candidat doit appartenir à un scrutin.",
                    },
                    "picture-mismatch": {
                        lead: "La photo pointe ailleurs",
                        text: "La photo pointe ailleurs — le bulletin affiche '{{url}}', qui ne désigne pas le document '{{document}}' à côté. Après l'importation, les deux pointeraient vers des fichiers différents.",
                    },
                    "picture-not-shown": {
                        lead: "Photo jamais affichée",
                        text: "Photo jamais affichée — un candidat nomme une photo vers laquelle aucune entrée du bulletin ne pointe, donc elle serait téléversée sans jamais être affichée.",
                    },
                    "picture-unrecorded": {
                        lead: "Photo non enregistrée",
                        text: "Photo non enregistrée — la photo d'un candidat figure sur le bulletin et rien n'indique de quel document il s'agit, donc elle ne pourra plus être modifiée ni supprimée sur la plateforme.",
                    },
                },
                census: {
                    "column-not-declared": {
                        lead: "Colonne du registre non déclarée",
                        text: "Colonne du registre non déclarée — {{message}}",
                    },
                    "no-area-column": {
                        lead: "Le registre ne nomme aucune zone",
                        text: "Le registre ne nomme aucune zone — cette élection a des zones, mais chaque électeur recevrait le bulletin par défaut. Si le découpage doit s'appliquer, le registre a besoin d'une colonne de zone.",
                    },
                    "note": {
                        lead: "À propos du registre",
                        text: "À propos du registre — {{message}}",
                    },
                    "unreadable": {
                        lead: "Registre illisible",
                        text: "Registre illisible — {{message}}",
                    },
                    "unreadable-member": {
                        lead: "Registre illisible dans le zip",
                        text: "Registre illisible dans le zip — {{message}}",
                    },
                },
                channels: {
                    "early-voting-closed": {
                        lead: "Vote anticipé désactivé",
                        text: "Vote anticipé désactivé — {{areas}} autorisent le vote anticipé et l'événement n'ouvre pas ce canal, donc le paramètre n'a aucun effet.",
                    },
                    "early-voting-no-area": {
                        lead: "Vote anticipé sans zone",
                        text: "Vote anticipé sans zone — le vote anticipé est ouvert et aucune zone ne l'autorise, donc la période anticipée n'aurait aucun électeur.",
                    },
                    "kiosk-client": {
                        lead: "Le kiosque a besoin de son propre client",
                        text: "Le kiosque a besoin de son propre client — le vote en kiosque nécessite un client d'authentification portant le nom du client ordinaire suivi de '-kiosk', que ce fichier ne crée pas.",
                    },
                    "none-open": {
                        lead: "Aucun mode de vote",
                        text: "Aucun mode de vote — tous les canaux de vote sont désactivés, donc personne ne peut voter.",
                    },
                    "telephone-elsewhere": {
                        lead: "Vote par téléphone configuré plus tard",
                        text: "Vote par téléphone configuré plus tard — le vote par téléphone se configure dans l'onglet IVR de l'événement après l'importation ; rien de cela ne figure dans ce fichier.",
                    },
                    "unknown": {
                        lead: "Mode de vote inconnu",
                        text: "Mode de vote inconnu — rien ne lit '{{name}}'. Les modes de vote pris en compte par la plateforme sont {{allowed}}.",
                    },
                },
                contacts: {
                    none: {
                        lead: "Aucun point de contact",
                        text: "Aucun point de contact — le jour de l'élection, ce sont les personnes qu'on appelle.",
                    },
                },
                contest: {
                    "algorithm-unknown": {
                        lead: "Méthode de dépouillement inconnue",
                        text: "Méthode de dépouillement inconnue — '{{value}}' n'est pas un algorithme de dépouillement. La plateforme accepte {{allowed}}.",
                    },
                    "area-unknown": {
                        lead: "Zone du scrutin inconnue",
                        text: "Zone du scrutin inconnue — rien n'a l'identifiant '{{area}}'.",
                    },
                    "cap-invalid": {
                        lead: "Limite de sélection impossible",
                        text: "Limite de sélection impossible — {{cap}} n'est pas un nombre de sélections.",
                    },
                    "cap-never-applies": {
                        lead: "La limite de sélection ne s'applique jamais",
                        text: "La limite de sélection ne s'applique jamais — une limite de {{cap}} par type ne s'applique jamais dans un scrutin où un électeur peut en choisir {{max}} au total.",
                    },
                    "chooses-more-than-available": {
                        lead: "Plus de choix que de candidats",
                        text: "Plus de choix que de candidats — un électeur peut en choisir {{max}} parmi {{available}} candidats.",
                    },
                    "chooses-more-than-offered": {
                        lead: "Plus de choix que d'options",
                        text: "Plus de choix que d'options — un électeur peut en choisir jusqu'à {{chosen}}, mais il n'y en a que {{offered}} au choix.",
                    },
                    "columns-invalid": {
                        lead: "Mise en page impossible",
                        text: "Mise en page impossible — {{columns}} colonnes n'est pas une mise en page.",
                    },
                    "columns-too-many": {
                        lead: "Trop de colonnes",
                        text: "Trop de colonnes — {{columns}} colonnes seront illisibles sur un téléphone, et c'est ainsi que la plupart des électeurs votent.",
                    },
                    "count-missing": {
                        lead: "Nombre manquant pour le scrutin",
                        text: "Nombre manquant pour le scrutin — le scrutin a besoin de {{field}}.",
                    },
                    "count-negative": {
                        lead: "Nombre négatif",
                        text: "Nombre négatif — {{field}} vaut {{value}}, et un nombre ne peut pas être inférieur à zéro.",
                    },
                    "election-missing": {
                        lead: "Élection du scrutin manquante",
                        text: "Élection du scrutin manquante — le scrutin appartient à une élection qui ne figure pas dans ce fichier.",
                    },
                    "elects-more-than-available": {
                        lead: "Plus de sièges que de candidats",
                        text: "Plus de sièges que de candidats — le scrutin élit {{winners}} parmi {{available}} candidats.",
                    },
                    "elects-more-than-chosen": {
                        lead: "Élit plus que permis",
                        text: "Élit plus que permis — le scrutin élit {{winners}}, mais un électeur ne peut en choisir que {{chosen}}.",
                    },
                    "elects-more-than-standing": {
                        lead: "Plus de sièges que de candidats",
                        text: "Plus de sièges que de candidats — le scrutin élit {{winners}} parmi {{standing}} candidats.",
                    },
                    "elects-nobody": {
                        lead: "N'élit personne",
                        text: "N'élit personne — le scrutin n'a aucun élu.",
                    },
                    "max-votes-below-one": {
                        lead: "Rien à choisir",
                        text: "Rien à choisir — un électeur peut choisir moins d'un candidat.",
                    },
                    "min-above-max": {
                        lead: "Minimum supérieur au maximum",
                        text: "Minimum supérieur au maximum — un électeur doit en choisir au moins {{min}} mais peut en choisir au plus {{max}}.",
                    },
                    "no-candidates": {
                        lead: "Aucun candidat",
                        text: "Aucun candidat — personne ne se présente encore à ce scrutin.",
                    },
                    "no-candidates-in-bundle": {
                        lead: "Aucun candidat",
                        text: "Aucun candidat — le scrutin n'a aucun candidat, donc personne ne peut y voter.",
                    },
                    "on-no-ballot": {
                        lead: "Scrutin sur aucun bulletin",
                        text: "Scrutin sur aucun bulletin — aucun bulletin de zone n'inclut ce scrutin, donc personne ne peut y voter.",
                    },
                    "policy-not-text": {
                        lead: "La règle du bulletin n'est pas du texte",
                        text: "La règle du bulletin n'est pas du texte — {{key}} devrait être du texte, et vaut {{value}}.",
                    },
                    "policy-unknown": {
                        lead: "Règle de bulletin inconnue",
                        text: "Règle de bulletin inconnue — '{{value}}' n'est pas un {{key}} valide. La plateforme accepte {{allowed}}.",
                    },
                    "ranked-counted-unranked": {
                        lead: "Bulletin classé compté sans classement",
                        text: "Bulletin classé compté sans classement — un scrutin préférentiel est compté par '{{algorithm}}', qui ignore les classements donnés par les électeurs.",
                    },
                    "tie-breaking-unknown": {
                        lead: "Règle de départage inconnue",
                        text: "Règle de départage inconnue — '{{value}}' n'est pas une règle de départage. La plateforme accepte {{allowed}}.",
                    },
                    "unranked-counted-ranked": {
                        lead: "Bulletin simple compté par classement",
                        text: "Bulletin simple compté par classement — un scrutin non préférentiel est compté par '{{algorithm}}', qui nécessite des bulletins classés.",
                    },
                    "voting-type-unknown": {
                        lead: "Type de vote inconnu",
                        text: "Type de vote inconnu — '{{value}}' n'est pas un type de vote. La plateforme accepte {{allowed}}.",
                    },
                    "write-in-slots-not-allowed": {
                        lead: "Emplacements libres non autorisés",
                        text: "Emplacements libres non autorisés — {{count}} emplacements de candidature libre figurent sur un scrutin qui ne les autorise pas, ce qui place des options sans nom sur le bulletin.",
                    },
                    "write-ins-no-slot": {
                        lead: "Candidats libres sans espace pour écrire",
                        text: "Candidats libres sans espace pour écrire — les candidatures libres sont autorisées et le scrutin n'a aucun emplacement prévu, donc un électeur n'a nulle part où saisir un nom.",
                    },
                },
                delivery: {
                    "no-importable": {
                        lead: "Aucune archive importable",
                        text: "Aucune archive importable — ce zip contient un plan mais pas l'archive que le Portail d'administration importe, donc le registre et les fichiers qu'il nomme n'y sont pas.",
                    },
                },
                design: {
                    "no-stable-key": {
                        lead: "Conception de bulletin sans clé",
                        text: "Conception de bulletin sans clé — {{kind}} {{id}} n'a ni nom ni identifiant externe, donc ses conceptions de bulletin ne peuvent pas être reconnues après une importation.",
                    },
                    "unreadable-style": {
                        lead: "Style de bulletin illisible",
                        text: "Style de bulletin illisible — le style de bulletin de la plateforme n'a pas pu être lu pour calculer l'empreinte de sa conception : {{reason}}",
                    },
                },
                election: {
                    "channels-differ": {
                        lead: "L'élection et l'événement divergent",
                        text: "L'élection et l'événement divergent — {{channels}} diffère de celui de l'événement, donc les commandes de démarrage de cette élection ne correspondraient pas.",
                    },
                    "grace-disallowed": {
                        lead: "Délai de grâce désactivé",
                        text: "Délai de grâce désactivé — {{seconds}} secondes de grâce sont définies et aucun délai de grâce n'est autorisé, donc le vote se ferme à l'échéance.",
                    },
                    "grace-negative": {
                        lead: "Délai de grâce négatif",
                        text: "Délai de grâce négatif — {{value}} secondes n'est pas une durée.",
                    },
                    "grace-zero": {
                        lead: "Délai de grâce nul",
                        text: "Délai de grâce nul — un délai de grâce est autorisé et dure zéro seconde, donc il n'y en a pas.",
                    },
                    "no-contests": {
                        lead: "Aucun scrutin",
                        text: "Aucun scrutin — personne ne vote dans cette élection.",
                    },
                    "revotes-negative": {
                        lead: "Nombre de votes impossible",
                        text: "Nombre de votes impossible — {{value}} n'est pas un nombre de fois qu'un électeur peut voter.",
                    },
                    "setting-unknown": {
                        lead: "Paramètre d'élection inconnu",
                        text: "Paramètre d'élection inconnu — '{{value}}' n'est pas un {{key}} valide. La plateforme accepte {{allowed}}.",
                    },
                    "spoil-without-revote": {
                        lead: "Annulation sans seconde chance",
                        text: "Annulation sans seconde chance — un électeur peut annuler un bulletin déposé sans avoir de seconde tentative pour le remplacer.",
                    },
                },
                event: {
                    "no-identifier": {
                        lead: "Aucun identifiant",
                        text: "Aucun identifiant — chaque id généré en est dérivé, donc sans lui rien ne peut être généré deux fois de la même façon.",
                    },
                    "no-name": {
                        lead: "Aucun nom",
                        text: "Aucun nom — les électeurs le voient au-dessus du bulletin, et il devient le titre de la page de connexion.",
                    },
                    "setting-unknown": {
                        lead: "Paramètre d'événement inconnu",
                        text: "Paramètre d'événement inconnu — '{{value}}' n'est pas un {{key}} valide. La plateforme accepte {{allowed}}.",
                    },
                },
                file: {
                    "cannot-decrypt": {
                        lead: "Déchiffrement impossible",
                        text: "Déchiffrement impossible — le fichier est chiffré et ne s'est pas ouvert. Vérifiez le mot de passe.",
                    },
                    "checksum-mismatch": {
                        lead: "Pas le fichier attendu",
                        text: "Pas le fichier attendu — son SHA-256 ne correspond pas à celui fourni, donc ce n'est pas le fichier prévu. Vérifiez la somme de contrôle ou téléversez à nouveau le fichier.",
                    },
                    "duplicate-name": {
                        lead: "Nom de fichier utilisé deux fois",
                        text: "Nom de fichier utilisé deux fois — '{{file}}' désigne deux choses différentes. Les fichiers sont transmis par nom, donc l'un deviendrait silencieusement l'autre.",
                    },
                    "missing": {
                        lead: "Fichier manquant",
                        text: "Fichier manquant — '{{file}}' est nommé et rien ici ne le contient. Une entrée d'archive vide fait échouer l'importation plutôt que de perdre un fichier.",
                    },
                    "not-a-bundle": {
                        lead: "Pas un export d'événement électoral",
                        text: "Pas un export d'événement électoral — le fichier a été lu mais n'en a pas la structure : {{reason}}",
                    },
                    "not-json": {
                        lead: "Fichier illisible",
                        text: "Fichier illisible — ce n'est pas un export d'événement électoral que la plateforme peut lire : {{reason}}",
                    },
                    "unreadable-archive": {
                        lead: "Archive illisible",
                        text: "Archive illisible — {{reason}}",
                    },
                    "unused": {
                        lead: "Fichier non utilisé",
                        text: "Fichier non utilisé — '{{file}}' a été fourni et rien ne le nomme, donc il serait transmis avec la livraison et montré à personne.",
                    },
                    "version-incompatible": {
                        lead: "Exporté par une autre version",
                        text: "Exporté par une autre version — le fichier provient de la version {{found}}, que la version {{current}} ne peut pas importer.",
                    },
                },
                identifier: {
                    duplicate: {
                        lead: "Identifiant utilisé deux fois",
                        text: "Identifiant utilisé deux fois — '{{identifier}}' est déjà utilisé par {{first}}. Les identifiants sont uniques dans tout l'événement électoral, donc le second remplace le premier au lieu d'être ajouté.",
                    },
                },
                ivr: {
                    "language-not-spoken": {
                        lead: "Langue non disponible par téléphone",
                        text: "Langue non disponible par téléphone — l'appel ne parle qu'anglais, français et espagnol, donc {{languages}} n'est pas proposé aux appelants.",
                    },
                    "missing-prompts": {
                        lead: "Messages d'appel manquants",
                        text: "Messages d'appel manquants — {{prompts}} n'ont pas de texte en « {{language}} », et le système téléphonique refuse tous les appels tant qu'ils n'en ont pas.",
                        lead_one: "Message d'appel manquant",
                        text_one:
                            "Message d'appel manquant — {{prompts}} n'a pas de texte en « {{language}} », et le système téléphonique refuse tous les appels tant qu'il n'en a pas.",
                    },
                },
                labels: {
                    "in-use": {
                        lead: "Étiquettes d'autorisation utilisées",
                        text: "Étiquettes d'autorisation utilisées — {{labels}}. Tout élément portant une étiquette est masqué pour chaque administrateur qui ne l'a pas, donc la personne qui importe ce fichier doit en avoir une sur son propre compte, sinon le Portail d'administration lui affichera une liste vide.",
                    },
                },
                languages: {
                    "default-not-offered": {
                        lead: "Langue par défaut non proposée",
                        text: "Langue par défaut non proposée — '{{chosen}}' ne fait pas partie de {{offered}}, donc les électeurs recevraient la première à la place.",
                    },
                    "detection-unknown": {
                        lead: "Détection de langue inconnue",
                        text: "Détection de langue inconnue — '{{policy}}' n'est pas une règle connue de la plateforme.",
                    },
                    "none": {
                        lead: "Aucune langue",
                        text: "Aucune langue — le bulletin se rabat sur l'anglais, ce qui est un filet de sécurité plutôt qu'un choix.",
                    },
                },
                link: {
                    "area-missing": {
                        lead: "Lien de bulletin vers une zone manquante",
                        text: "Lien de bulletin vers une zone manquante — un scrutin est placé sur le bulletin d'une zone qui ne figure pas dans ce fichier.",
                    },
                    "contest-missing": {
                        lead: "Lien de bulletin vers un scrutin manquant",
                        text: "Lien de bulletin vers un scrutin manquant — le bulletin d'une zone liste un scrutin qui ne figure pas dans ce fichier.",
                    },
                },
                logo: {
                    "file-and-link": {
                        lead: "Deux logos",
                        text: "Deux logos — ce plan contient à la fois un logo téléversé ('{{file}}') et un lien ('{{url}}'). C'est le fichier qui est livré ; le lien est ignoré.",
                    },
                    "file-missing": {
                        lead: "Fichier du logo manquant",
                        text: "Fichier du logo manquant — '{{file}}' est désigné comme logo et n'a pas été fourni. Placez le fichier à côté du classeur sous exactement ce nom.",
                    },
                    "no-bytes": {
                        lead: "Fichier du logo vide",
                        text: "Fichier du logo vide — '{{file}}' est désigné comme logo et ne contient aucun octet. Une entrée d'archive vide fait échouer l'importation plutôt que de perdre une image.",
                    },
                },
                material: {
                    "empty-document": {
                        lead: "Document d'information vide",
                        text: "Document d'information vide — il serait importé comme un lien vers rien. Omettez complètement le document pour une ressource qui n'a pas de fichier.",
                    },
                    "file-missing": {
                        lead: "Fichier du document manquant",
                        text: "Fichier du document manquant — '{{file}}' est nommé ici et n'a pas été fourni. Placez le fichier à côté du classeur sous exactement ce nom.",
                    },
                    "file-unused": {
                        lead: "Fichier du document non utilisé",
                        text: "Fichier du document non utilisé — '{{file}}' a été fourni et aucune ligne ne le nomme, donc il serait téléversé et montré à personne.",
                    },
                    "no-identifier": {
                        lead: "Document sans identifiant",
                        text: "Document sans identifiant — l'id de son fichier en est dérivé, donc sans lui le fichier ne peut pas être associé à la ligne.",
                    },
                    "tab-off": {
                        lead: "Onglet des documents désactivé",
                        text: "Onglet des documents désactivé — {{count}} documents d'information figurent dans ce fichier et l'onglet qui les affiche est désactivé, donc les électeurs ne les verraient jamais.",
                    },
                    "wrong-event": {
                        lead: "Document d'un autre événement",
                        text: "Document d'un autre événement — ce document d'information appartient à un autre événement électoral.",
                    },
                },
                messages: {
                    "weekly-no-time": {
                        lead: "Une répétition sans heure",
                        text: "Une répétition sans heure — un message est répété chaque semaine sans préciser à quelle heure, donc la personne qui l'envoie doit choisir une heure que personne n'a notée.",
                    },
                },
                passwords: {
                    "column-already-there": {
                        lead: "Mots de passe fournis deux fois",
                        text: "Mots de passe fournis deux fois — le registre a déjà une colonne de mot de passe, donc les mots de passe générés seraient une seconde réponse à la même question. Supprimez la colonne, ou désactivez la génération.",
                    },
                    "no-characters": {
                        lead: "Mots de passe sans caractères",
                        text: "Mots de passe sans caractères — les mots de passe générés doivent pouvoir utiliser au moins un type de caractère.",
                    },
                    "no-seed": {
                        lead: "Mots de passe sans graine",
                        text: "Mots de passe sans graine — c'est la graine qui fait qu'une nouvelle génération produit les mêmes mots de passe plutôt que de nouveaux.",
                    },
                },
                package: {
                    "already-imported": {
                        lead: "Déjà importée",
                        text: "Déjà importée — la révision {{revision}} de cette configuration a déjà été importée ; importez une révision plus récente.",
                    },
                    "approval-invalid": {
                        lead: "L'approbation ne compte pas",
                        text: "L'approbation ne compte pas — l'approbation de {{name}} n'a pas pu être vérifiée : {{reason}}",
                    },
                    "approval-repeated": {
                        lead: "Approuvé deux fois par la même personne",
                        text: "Approuvé deux fois par la même personne — {{name}} a approuvé plus d'une fois, et ne compte qu'une fois.",
                    },
                    "approver-key-usage": {
                        lead: "L'approbateur ne peut pas signer",
                        text: "L'approbateur ne peut pas signer — le certificat d'un approbateur n'est pas fait pour signer.",
                    },
                    "bad-signature": {
                        lead: "La signature ne correspond pas",
                        text: "La signature ne correspond pas — la signature du paquet ne se vérifie pas, donc il a été modifié après la signature ou signé par une autre clé : {{reason}}",
                    },
                    "content-digest": {
                        lead: "L'empreinte du contenu ne correspond pas",
                        text: "L'empreinte du contenu ne correspond pas — le manifeste indique {{expected}} et son contenu donne {{actual}}.",
                    },
                    "duplicate-member": {
                        lead: "Nom de fichier utilisé deux fois",
                        text: "Nom de fichier utilisé deux fois — '{{file}}' apparaît deux fois dans {{archive}}, donc deux lecteurs pourraient prendre des fichiers différents.",
                    },
                    "file-changed": {
                        lead: "Modifié après la signature",
                        text: "Modifié après la signature — {{file}} a le SHA-256 {{actual}}, et le manifeste indique {{expected}}. Rien dans le paquet n'a été lu.",
                    },
                    "file-extra": {
                        lead: "Fichier absent du manifeste",
                        text: "Fichier absent du manifeste — {{file}} est dans le paquet mais n'a pas été signé. Rien dans le paquet n'a été lu.",
                    },
                    "file-missing": {
                        lead: "Fichier signé manquant",
                        text: "Fichier signé manquant — {{file}} est dans le manifeste et pas dans le paquet. Rien dans le paquet n'a été lu.",
                    },
                    "invalid-time": {
                        lead: "Pas une date et heure",
                        text: "Pas une date et heure — '{{value}}' dans le manifeste n'est pas une date et heure.",
                    },
                    "member-too-large": {
                        lead: "Fichier trop volumineux",
                        text: "Fichier trop volumineux — '{{file}}' dans {{archive}} occupe, une fois décompressé, plus que les {{limit}} octets permis pour un fichier.",
                    },
                    "nested-too-deep": {
                        lead: "Trop de zips imbriqués",
                        text: "Trop de zips imbriqués — '{{file}}' se trouve dans plus de zips que les {{limit}} dans lesquels un fichier peut être imbriqué.",
                    },
                    "no-importable": {
                        lead: "Rien à importer",
                        text: "Rien à importer — le paquet ne contient pas official_election_setup.zip, l'archive que lit l'importateur.",
                    },
                    "report-template-changed": {
                        lead: "Modèle de rapport modifié",
                        text: "Modèle de rapport modifié — le modèle du rapport {{report}} n'est pas celui qui a été approuvé : son empreinte est {{actual}}, et la configuration signée indique {{expected}}.",
                    },
                    "report-template-missing": {
                        lead: "Modèle de rapport manquant",
                        text: "Modèle de rapport manquant — le rapport {{report}} est produit avec le modèle '{{template}}', qui n'est pas dans la configuration, donc sa conception ne peut pas être signée.",
                    },
                    "report-unreadable": {
                        lead: "Rapport impossible à signer",
                        text: "Rapport impossible à signer — {{message}}",
                    },
                    "revoked-approver": {
                        lead: "Certificat d'approbateur révoqué",
                        text: "Certificat d'approbateur révoqué — le certificat d'un approbateur a été révoqué, donc l'approbation ne compte pas.",
                    },
                    "revoked-signer": {
                        lead: "Clé de signature révoquée",
                        text: "Clé de signature révoquée — la clé qui a signé ce paquet a été révoquée, et ses paquets sont refusés.",
                    },
                    "rollback": {
                        lead: "Pas une révision plus récente",
                        text: "Pas une révision plus récente — la révision {{revision}} n'est pas plus récente que la révision {{last}}, la dernière importée.",
                    },
                    "signed-in-the-future": {
                        lead: "Signé dans le futur",
                        text: "Signé dans le futur — le paquet indique {{at}} comme moment de la signature, et il est maintenant {{now}}.",
                    },
                    "signer-key-usage": {
                        lead: "La clé de signature ne peut pas signer",
                        text: "La clé de signature ne peut pas signer — le certificat de la clé qui a signé ce paquet n'est pas fait pour signer.",
                    },
                    "too-few-approvals": {
                        lead: "Trop peu d'approbations",
                        text: "Trop peu d'approbations — {{count}} approbations valides de personnes différentes, et il en faut {{required}}.",
                    },
                    "too-large": {
                        lead: "Paquet trop volumineux",
                        text: "Paquet trop volumineux — une fois décompressé, il occupe plus que les {{limit}} octets permis pour un paquet : '{{file}}' dans {{archive}} les dépasse.",
                    },
                    "too-many-members": {
                        lead: "Trop de fichiers",
                        text: "Trop de fichiers — {{archive}} contient plus de fichiers que les {{limit}} qu'un paquet peut contenir.",
                    },
                    "unhashable-content": {
                        lead: "Contenu impossible à hacher",
                        text: "Contenu impossible à hacher — le contenu de la configuration n'a pas pu être écrit pour être haché : {{reason}}",
                    },
                    "unknown-format": {
                        lead: "Format de manifeste inconnu",
                        text: "Format de manifeste inconnu — le manifeste est au format '{{format}}', que cette version ne sait pas lire.",
                    },
                    "unreadable-chain": {
                        lead: "Certificats du signataire illisibles",
                        text: "Certificats du signataire illisibles — la chaîne de certificats du paquet n'a pas pu être lue : {{reason}}",
                    },
                    "unreadable-manifest": {
                        lead: "Manifeste illisible",
                        text: "Manifeste illisible — le manifeste du paquet n'a pas pu être lu : {{reason}}",
                    },
                    "unreadable-revocation-list": {
                        lead: "Liste de révocation illisible",
                        text: "Liste de révocation illisible — une liste de révocation n'a pas pu être lue, donc elle ne peut pas être appliquée : {{reason}}",
                    },
                    "unreadable-trust": {
                        lead: "Certificats de confiance illisibles",
                        text: "Certificats de confiance illisibles — le paramètre {{setting}} n'a pas pu être lu : {{reason}}",
                    },
                    "unreadable-zip": {
                        lead: "Archive illisible",
                        text: "Archive illisible — {{archive}} n'a pas pu être lu comme un zip : {{reason}}",
                    },
                    "unsigned": {
                        lead: "Paquet non signé",
                        text: "Paquet non signé — il n'a pas de {{missing}}, et cette installation n'importe que des paquets signés.",
                    },
                    "untrusted-approver": {
                        lead: "Approbateur non fiable",
                        text: "Approbateur non fiable — le certificat d'un approbateur n'est pas de confiance : {{reason}}",
                    },
                    "untrusted-signer": {
                        lead: "Signataire non fiable",
                        text: "Signataire non fiable — la clé qui a signé ce paquet n'est pas une clé à laquelle cette installation fait confiance : {{reason}}",
                    },
                    "unwritable-manifest": {
                        lead: "Manifeste impossible à écrire",
                        text: "Manifeste impossible à écrire — le manifeste n'a pas pu être écrit : {{reason}}",
                    },
                },
                plan: {
                    "not-a-plan": {
                        lead: "Pas un plan électoral",
                        text: "Pas un plan électoral — le fichier n'a pas pu être lu comme tel : {{error}}",
                    },
                    "saved-by-newer-version": {
                        lead: "Enregistré par une version plus récente",
                        text: "Enregistré par une version plus récente — {{saved}} contre {{supported}}. L'ouvrir ici supprimerait silencieusement tout ce que cette version a ajouté.",
                    },
                    "unreadable": {
                        lead: "Plan illisible",
                        text: "Plan illisible — {{error}}",
                    },
                },
                reports: {
                    "duplicate": {
                        lead: "Rapport configuré deux fois",
                        text: "Rapport configuré deux fois — le rapport {{report}} est configuré plus d'une fois pour la même élection.",
                    },
                    "no-copies": {
                        lead: "Aucun exemplaire",
                        text: "Aucun exemplaire — le rapport {{report}} est configuré pour n'imprimer aucun exemplaire. Indiquez-en au moins un.",
                    },
                    "unknown-election": {
                        lead: "Élection inconnue",
                        text: "Élection inconnue — le rapport {{report}} porte sur l'élection '{{election}}', que ce plan ne contient pas.",
                    },
                    "unsupported-format": {
                        lead: "Format non disponible",
                        text: "Format non disponible — le rapport {{report}} ne peut pas être généré au format {{format}}.",
                    },
                },
                schedule: {
                    "closes-before-opens": {
                        lead: "Ferme avant d'ouvrir",
                        text: "Ferme avant d'ouvrir — le vote ne serait jamais ouvert.",
                    },
                    "crosses-daylight-saving": {
                        lead: "Traverse un changement d'heure",
                        text: "Traverse un changement d'heure — la période dure une heure de plus ou de moins que ne le laissent penser les horaires.",
                    },
                    "key-ceremony-not-first": {
                        lead: "Cérémonie des clés trop tardive",
                        text: "Cérémonie des clés trop tardive — la clé de l'élection doit exister avant qu'un vote puisse être chiffré avec elle.",
                    },
                    "tally-ceremony-too-early": {
                        lead: "Cérémonie de dépouillement trop tôt",
                        text: "Cérémonie de dépouillement trop tôt — elle compterait des votes qui n'ont pas encore été exprimés.",
                    },
                    "window-incomplete": {
                        lead: "Période de vote incomplète",
                        text: "Période de vote incomplète — la période devra être ouverte ou fermée manuellement dans le Portail d'administration.",
                    },
                },
                signing: {
                    "duplicate-action": {
                        lead: "Deux règles pour une action",
                        text: "Deux règles pour une action — '{{action}}' a plus d'une règle de signature. N'en gardez qu'une.",
                    },
                    "signatures-out-of-range": {
                        lead: "Signatures hors limites",
                        text: "Signatures hors limites — la règle de '{{action}}' doit demander entre {{min}} et {{max}} signatures.",
                    },
                    "expiry-out-of-range": {
                        lead: "Expiration hors limites",
                        text: "Expiration hors limites — une demande '{{action}}' doit expirer après 1 à 525 600 minutes (un an), ou jamais.",
                    },
                },
                threshold: {
                    "above-trustees": {
                        lead: "Seuil trop élevé",
                        text: "Seuil trop élevé — {{threshold}} dépositaires sur {{trustees}} sont requis, ce qui est impossible à atteindre. La clé serait générée et le résultat ne pourrait jamais être déchiffré.",
                    },
                    "one": {
                        lead: "Seuil de un",
                        text: "Seuil de un — n'importe quel dépositaire peut ouvrir le dépouillement seul, quoi qu'en dise la liste — la même garantie qu'avec un seul dépositaire, et aucune contre cette personne.",
                    },
                    "zero": {
                        lead: "Seuil de zéro",
                        text: "Seuil de zéro — le dépouillement pourrait être ouvert sans personne.",
                    },
                },
                trustee: {
                    "email-malformed": {
                        lead: "Adresse e-mail invalide",
                        text: "Adresse e-mail invalide — '{{email}}' ne ressemble pas à une adresse e-mail.",
                    },
                    "no-email": {
                        lead: "Dépositaire sans adresse",
                        text: "Dépositaire sans adresse — c'est ainsi qu'il est invité à la cérémonie des clés.",
                    },
                    "no-name": {
                        lead: "Dépositaire sans nom",
                        text: "Dépositaire sans nom — l'importateur associe les membres de la cérémonie des clés par nom aux dépositaires déjà configurés dans le tenant, et un nom vide devient silencieusement un membre qui n'existe pas.",
                    },
                },
                trustees: {
                    "none": {
                        lead: "Aucun dépositaire",
                        text: "Aucun dépositaire — une clé d'élection doit être détenue par au moins deux personnes. Avec une seule, cette personne peut déchiffrer chaque bulletin seule, alors que le chiffrement est justement là pour l'empêcher.",
                    },
                    "only-one": {
                        lead: "Un seul dépositaire",
                        text: "Un seul dépositaire — une clé d'élection doit être détenue par au moins deux personnes. Avec une seule, cette personne peut déchiffrer chaque bulletin seule, alors que le chiffrement est justement là pour l'empêcher.",
                    },
                },
                voter: {
                    "area-unknown": {
                        lead: "Zone de l'électeur inconnue",
                        text: "Zone de l'électeur inconnue — rien ne s'appelle '{{area}}'. Les électeurs sont associés à leur zone par nom, donc cet électeur ne recevrait aucun bulletin. Copiez le nom depuis la zone plutôt que de le retaper.",
                    },
                    "duplicate-username": {
                        lead: "Nom d'utilisateur utilisé deux fois",
                        text: "Nom d'utilisateur utilisé deux fois — '{{username}}' figure aussi à la ligne {{first}}. Deux électeurs partageant un nom d'utilisateur deviennent un seul compte, et celui-ci remplacerait l'autre sans le signaler.",
                    },
                    "no-area": {
                        lead: "Électeur sans zone",
                        text: "Électeur sans zone — la zone détermine quel bulletin reçoit un électeur, et la génération refuse une ligne du registre qui n'en a pas.",
                    },
                    "no-username": {
                        lead: "Électeur sans nom d'utilisateur",
                        text: "Électeur sans nom d'utilisateur — c'est l'identifiant de connexion et ce dont son compte est dérivé.",
                    },
                },
                voters: {
                    "duplicate-column": {
                        lead: "Colonne fournie deux fois",
                        text: "Colonne fournie deux fois — deux colonnes définissent toutes deux '{{column}}'. N'en gardez qu'une.",
                    },
                    "unreadable-row": {
                        lead: "Ligne illisible",
                        text: "Ligne illisible — la ligne {{row}} n'a pas pu être lue : {{reason}}",
                    },
                    "vote-weight-misspelled": {
                        lead: "Colonne du poids du vote mal orthographiée",
                        text: "Colonne du poids du vote mal orthographiée — '{{column}}' n'est pas reconnue. La colonne s'écrit exactement '{{expected}}', sinon chaque électeur serait compté avec un poids de 1.",
                    },
                    "vote-weight-not-a-number": {
                        lead: "Le poids du vote n'est pas un nombre",
                        text: "Le poids du vote n'est pas un nombre — '{{value}}' à la ligne {{row}} doit être un nombre entier entre 1 et {{max}}.",
                    },
                    "vote-weight-out-of-range": {
                        lead: "Poids du vote hors limites",
                        text: "Poids du vote hors limites — {{value}} à la ligne {{row}} doit être compris entre {{min}} et {{max}}.",
                    },
                },
            },
        },
    },
}

export default frenchTranslation
