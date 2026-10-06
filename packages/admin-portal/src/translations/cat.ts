// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const catalanTranslation: TranslationType = {
    translations: {
        philippinePassport: "Passaport filipí",
        seamanBook: "Llibreta de mariner",
        philSysID: "Identificació PhilSys",
        iBP: "Col·legi d'Advocats Integrat de Filipines (IBP)",
        driversLicense: "Permís de conduir",
        loading: "Carregant...",
        loadingDataProvider: "Carregant proveïdor de dades...",
        tallySheetImport: {
            title: "Importacions d'actes d'escrutini",
            subtitle:
                "Importa fitxers d'actes d'escrutini ES&S o CSV, previsualitza les urnes generades i aprova-les abans de crear actes d'escrutini.",
            createTitle: "Importar actes d'escrutini",
            detailTitle: "Importació d'acta d'escrutini",
            empty: "Encara no hi ha importacions d'actes d'escrutini.",
            emptyBody:
                "Comença important un fitxer ES&S Enhanced XML o CSV canònic per a aquest esdeveniment electoral.",
            sourceFormat: {
                ESS_ENHANCED_XML: "ES&S Enhanced XML",
                CANONICAL_CSV: "CSV canònic",
            },
            channel: {
                PAPER: "Paper",
                POSTAL: "Postal",
                IN_PERSON: "Presencial",
            },
            table: {
                created: "Creat",
                createdBy: "Creat per",
                file: "Fitxer",
                format: "Format",
                channel: "Canal",
                status: "Estat",
                labels: "Etiquetes",
                annotations: "Anotacions",
                actions: "Accions",
            },
            summary: {
                imported: "Importades",
                changed: "Modificades",
                new: "Noves",
                unchanged: "Sense canvis",
                conflicted: "En conflicte",
                errors: "Errors",
            },
            status: {
                PENDING_REVIEW: "Pendent de revisió",
                APPROVED: "Aprovada",
                DISAPPROVED: "Desaprovada",
                FAILED_VALIDATION: "Validació fallida",
                CONFLICTED: "En conflicte",
                NEW: "Nova",
                CHANGED: "Modificada",
                UNCHANGED: "Sense canvis",
            },
            fields: {
                format: "Format",
                channel: "Canal",
                supportedFormats: "Formats admesos: XML, CSV",
                generatedTallySheet: "Acta d'escrutini generada",
                sourceCandidates: "IDs de candidats d'origen",
                none: "Cap",
            },
            actions: {
                create: "Importar actes d'escrutini",
                review: "Revisar",
                source: "Origen",
                cancel: "Cancel·lar",
                preview: "Previsualitzar",
                save: "Desar importació",
                approve: "Aprovar",
                disapprove: "Desaprovar",
                close: "Tancar",
                openExisting: "Obrir existent",
            },
            notifications: {
                selectFile: "Selecciona un fitxer d'importació abans de previsualitzar-lo",
                duplicateSource:
                    "Aquest hash de fitxer d'origen ja apareix en una importació d'acta d'escrutini anterior.",
                uploadUrlError: "No s'ha pogut crear la URL de pujada",
                uploadError: "No s'ha pogut pujar el fitxer d'importació",
                previewEmpty: "La resposta de la previsualització estava buida",
                previewError: "No s'ha pogut previsualitzar la importació",
                importEmpty: "La resposta de la importació estava buida",
                created: "S'ha creat la importació de l'acta d'escrutini",
                createError: "No s'ha pogut crear la importació",
                reviewEmpty: "La resposta de la revisió estava buida",
                conflicted: "La importació té conflictes de línia base desactualitzats",
                approved: "S'ha aprovat la importació",
                disapproved: "S'ha desaprovat la importació",
                reviewError: "No s'ha pogut revisar la importació",
                sourceUrlError: "No s'ha pogut crear la URL de descàrrega d'origen",
                sourceDownloadError: "No s'ha pogut descarregar el fitxer d'origen",
            },
            pagination: {
                range: "{{rangeStart}}-{{rangeEnd}} de {{total}}",
                previous: "Anterior",
                next: "Següent",
            },
        },
        reconciliation: {
            menuButton: "Sincr. votants ext.",
            categories: {
                VOTED_INTERNET: "Ha votat per Internet",
                VOTED_OTHER_CHANNEL: "Ha votat per un altre canal",
                DISABLED_DELETE_CALL: "Votant deshabilitat",
                DELETION_REVERTED: "Eliminació revertida",
                PROFILE_UPDATE: "Perfil actualitzat",
                VOTER_ADDED: "Votant afegit",
                REENABLED: "Votant rehabilitat",
                VOTED_UNMARKED: "Votant desmarcat com a votat",
                ROW_FAILURE: "Error de fila",
            },
            table: {
                voterId: "ID de Votant",
                field: "Camp",
                category: "Categoria",
                currentValue: "Valor actual",
                newValue: "Valor nou",
                reason: "Motiu",
                rowLabel: "Fila",
                noDifferences: "No s'han trobat diferències - els sistemes estan sincronitzats.",
            },
            wizard: {
                title: "Sincronització de reconciliació externa",
                subtitle: "Sincronitza la llista de votants amb el sistema extern",
                drop: {
                    description:
                        "Deixa anar el fitxer de reconciliació generat pel sistema extern - ambdós diffs (costat extern i costat Sequent) es calculen automàticament i es mostren en taules separades.",
                    fileFormatLabel: "Fitxer CSV",
                    uploading: "Pujant {{fileName}} i calculant ambdós diffs...",
                },
                review: {
                    fileSummary: "{{fileName}} - Seqüència {{sequence}}, generat {{generatedAt}}",
                    rowFailuresWarning:
                        "{{count}} fila(es) no s'han pogut reconciliar de manera segura i queden excloses d'ambdós diffs - vegeu els detalls a continuació.",
                    noDifferences: "No hi ha diferències - ambdós sistemes ja estan sincronitzats.",
                    diffOnlyDifferences:
                        "Aquesta és una comprovació de convergència d'una seqüència ja aplicada. Les diferències es mostren per fer-ne seguiment, però aquesta ronda no es pot tornar a aplicar.",
                    externalDiffTitle: "Diff extern",
                    sequentDiffTitle: "Diff de Sequent",
                    downloadExternalPatch: "Descarregar pedaç extern",
                    externalDiffCaption:
                        "Descarrega el pedaç i lliura'l al sistema extern fora d'aquesta eina. Un cop l'apliqui i generi el següent fitxer de reconciliació, fes clic a 'Enrere' i deixa anar aquest fitxer - 'Aplicar' s'habilita quan aquesta taula estigui buida.",
                    noExternalDifferences: "No hi ha diferències del costat extern.",
                    sequentDiffCaption:
                        "Aplica els canvis directament a Sequent fent clic a 'Aplicar' - no es genera cap fitxer de pedaç per aquests.",
                    noSequentDifferences: "No hi ha diferències del costat de Sequent.",
                },
                applying: {
                    inProgress: "Aplicant els canvis del costat de Sequent...",
                    rowFailures:
                        "{{count}} fila(es) han quedat excloses d'aquesta ronda i necessiten seguiment manual - vegeu els detalls a continuació.",
                    rowFailuresTruncated:
                        "Es mostren les primeres {{shown}} de {{count}} errades de fila. Resoleu la causa comuna i torneu-ho a provar per veure les errades restants.",
                    success: "Tots els canvis del costat de Sequent s'han aplicat correctament.",
                },
                actions: {
                    cancel: "Cancel·lar",
                    back: "Enrere",
                    apply: "Aplicar",
                    next: "Següent",
                    startOver: "Tornar a començar",
                    close: "Tancar",
                },
                confirm: {
                    title: "Confirmar canvis de reconciliació",
                    categoriesNote:
                        "Les categories ressaltades en taronja ({{categories}}) afecten l'estat de vot o deshabiliten votants.",
                    applyChanges: "Aplicar canvis",
                    continue: "Continuar",
                },
                summary: {
                    votedOtherChannel:
                        "marca {{count}} votant(s) com a votat(s) per un altre canal",
                    disabled: "deshabilita {{count}} votant(s)",
                    reenabled: "rehabilita {{count}} votant(s)",
                    votedUnmarked: "desmarca {{count}} votant(s) com a votat",
                    profileUpdated: "actualitza {{count}} perfil(s)",
                    voterAdded: "afegeix {{count}} votant(s)",
                    prefix: "Això aplicarà canvis que {{parts}}.",
                    empty: "No hi ha canvis del costat de Sequent per aplicar.",
                },
                notifications: {
                    envelopeLoadError:
                        "No s'ha pogut carregar el diff de reconciliació - torna-ho a provar.",
                    generateFailed:
                        "No s'ha pogut calcular el diff de reconciliació - consulta el widget de tasques per a més detalls.",
                    applyFailed:
                        "No s'han pogut aplicar els canvis del costat de Sequent - consulta el widget de tasques per a més detalls.",
                    uploadUrlError: "No s'ha pogut obtenir una URL de pujada",
                    generateTaskError: "No s'ha pogut iniciar la tasca de diff de reconciliació",
                    uploadError: "No s'ha pogut pujar el fitxer de reconciliació",
                    applyTaskError: "No s'ha pogut iniciar la tasca d'aplicació",
                    applyError: "No s'han pogut aplicar els canvis de reconciliació",
                },
            },
        },
        tasksScreen: {
            noPermissions: "No tens permís per accedir als registres.",
            title: "Execució de tasques",
            subtitle: "Informació sobre les tasques executades",
            taskInformation: "Informació de la tasca",
            status: "estat: {{status}}",
            ok: "D'acord",
            column: {
                id: "Índex",
                name: "Nom de la tasca",
                type: "Tipus",
                execution_status: "Estat",
                start_at: "Hora d'inici",
                end_at: "Hora de finalització",
                executed_by_user: "Executor",
                annotations: "Annotations",
                labels: "Etiquetes",
                logs: "Registres",
            },
            tasksExecution: {
                DELETE_TENANT: "Suprimir llogater",
                PUBLISH_BALLOT: "Publicar papereta",
                VOTER_INFORMATION_LETTER: "Carta d'informació per al votant",
                EXPORT_MONITORING_DATA: "Exportar Dades de Monitorització",
                EXPORT_ELECTION_EVENT: "Exportar esdeveniment electoral",
                CREATE_ELECTION_EVENT: "Crear Esdeveniment Electoral",
                IMPORT_ELECTION_EVENT: "Importar esdeveniment electoral",
                IMPORT_USERS: "Importar usuaris",
                EDIT_USER: "Editar votant",
                IMPORT_CANDIDATES: "Importar candidats",
                EXPORT_VOTERS: "Exportar votants",
                CREATE_TRANSMISSION_PACKAGE: "Crear paquet de transmissió",
                EXPORT_BALLOT_PUBLICATION: "Exporta Publicació de Butlleta",
                EXPORT_ACTIVITY_LOGS_REPORT: "Exportar Informe de Registres d'Activitat",
                GENERATE_REPORT: "Generar informe",
                GENERATE_TRANSMISSION_REPORT: "Generar informe de transmissió",
                EXPORT_TRUSTEES: "Exportar Autoritats",
                EXPORT_APPLICATION: "Exportar Sol·licituds",
                EXPORT_TENANT_CONFIG: "Exporta la Configuració del Llogater",
                IMPORT_TENANT_CONFIG: "Importa la Configuració del Llogater",
                RENDER_DOCUMENT_PDF: "Generar el document PDF",
                CREATE_TENANT: "Crear Llogater",
                EXPORT_TEMPLATES: "Exportar plantilles",
                IMPORT_TEMPLATES: "Importar plantilles",
                DELETE_ELECTION_EVENT: "Esborrar esdeveniment electoral",
                DELETE_VOTERS: "Delete Voters",
                PREPARE_PUBLICATION_PREVIEW: "Preparar la vista prèvia de la publicació",
                EXPORT_TALLY_RESULTS_XLSX: "Exporta els resultats del recompte en format XLSX",
                EXPORT_CERTIFICATE_AUTHORITIES: "Exportar autoritats de certificació",
                PUBLISH_RESULTS_WEBSITE: "Publicar el lloc web de resultats",
            },
            documentAccess: {
                title: "Accés al document",
                sensitivityNotice:
                    "Informació sensible. Compartiu aquesta contrasenya només amb el destinatari previst.",
                passwordLabel: "Contrasenya per obrir el PDF xifrat",
                showPassword: "Mostra la contrasenya",
                copyPassword: "Copia la contrasenya",
                passwordCopied: "Contrasenya copiada",
                passwordError: "No s'ha pogut recuperar la contrasenya del PDF",
                copyError: "No s'ha pogut copiar la contrasenya",
                guidance:
                    "La contrasenya només es carrega després de triar Mostra la contrasenya. Després de carregar-la, aquí apareix un camp de només lectura amb l'opció de copiar.",
            },
            widget: {
                taskTitle: "Tasca: {{title}}",
                viewTask: "Veure Tasca",
                downloadDocument: "Descarregar Fitxer",
                downloadHashManifest: "Manifest de hashes",
            },
            exportTasksExecution: {
                success: "L'exportació s'ha completat amb èxit",
                error: "Error en exportar l'Execució de Tasques",
            },
        },
        logsScreen: {
            noPermissions: "No tens permís per accedir a les bitàcoles.",
            title: "Bitàcoles",
            subtitle: "Bitàcoles generals de les bases de dades principal i de IAM.",
            actions: {
                csv: "Exportar en CSV",
                pdf: "Exportar en PDF",
            },
            exportdialog: {
                description:
                    "Si us plau, confirmeu que voleu executar aquesta acció; pot trigar una estona a completar-se.",
                title: "Exporta els registres",
                from: "Des de",
                to: "Fins a",
                timeZone: "Fus horari",
                format: "Format",
                csv: "CSV",
                pdf: "PDF",
                zoneNote:
                    "Cada fila conserva la seva hora en UTC (ISO 8601) i en {{abbr}}, amb el nom del fus horari. L'interval de dates inclou els dos extrems, en {{abbr}}.",
                zoneNotePdf:
                    "El PDF mostra cada hora en {{abbr}}. L'interval de dates inclou els dos extrems, en {{abbr}}.",
                rowZones: "El fus horari de l'elecció de cada fila",
                zoneNoteRows:
                    "Cada fila conserva la seva hora en UTC (ISO 8601) i en el fus horari de la seva elecció, amb el nom del fus horari. L'interval de dates inclou els dos extrems, en {{abbr}}.",
                zoneNoteRowsPdf:
                    "El PDF mostra cada hora en el fus horari de la seva elecció. L'interval de dates inclou els dos extrems, en {{abbr}}.",
            },
            filter: {
                createdFrom: "Creat des de",
                createdTo: "fins a",
                statementTimestampFrom: "Marca de temps de la declaració des de",
                statementTimestampTo: "Marca de temps de la declaració fins a",
                timeZone: "Fus horari",
            },
            scheduledOutcome: {
                outcome: {
                    "waiting-for-initialization": "Esperant la inicialització",
                    "runs": "s'executa",
                    "runs-unsigned": "s'executa sense signatures",
                    "refused": "es rebutja",
                },
                check: {
                    "initialization": "La inicialització requerida és incompleta",
                    "voting-close": "La votació no es pot obrir després del termini de tancament",
                    "needs-signatures": "signatures necessàries",
                    "covered": "a la configuració signada",
                    "unsigned-close": "tancament sense signatures",
                    "stricter-copy": "configuració actual o publicada",
                    "defaults": "encara no s'ha publicat res",
                },
                changed: "Ara {{after}} (abans: {{before}}).",
                result: "Resultat: {{outcome}}.",
                deciding: "Comprovació decisiva: {{check}}. {{value}}",
                authorizedBy: "Autoritzat per la configuració {{code}}.",
                nextStep: "Pas següent: {{step}}",
            },
            column: {
                id: "ID",
                statement_kind: "Tipus de declaració",
                created: "Creat",
                statement_timestamp: "Marca de temps de declaració",
                message: "Missatge",
                user_id: "ID d'usuari",
                username: "Nom d'Usuari",
                sender_pk: "Clau primària de l'emissor",
                log_type: "Tipus de registre",
                event_type: "Tipus d'esdeveniment",
                description: "Descripció",
                version: "Versió",
            },
            main: {
                title: "Bitàcola de Base de Dades Principal",
            },
            iam: {
                title: "Bitàcola de Base de Dades de IAM",
            },
        },
        areas: {
            common: {
                title: "Àrees",
                subTitle: "Configuració d'Àrea.",
                deleteError: "Error esborrant Àrea",
            },
            createAreaSuccess: "Àrea creada",
            updateAreaSuccess: "Àrea actualizada",
            createAreaError: "Error creant àrea",
            sequent_backend_area_contest: "Preguntes de l'Àrea",
            empty: {
                header: "No hi ha Àrees encara.",
                action: "Crear una Àrea",
            },
            formImputs: {
                allowEarlyVoting: "Permetre Votació Anticipada",
            },
        },
        integrationsScreen: {
            common: {
                gapiKey: "Clau de Compte de Servei de Google Calendar",
                gapiEmail: "Correu d'Autenticació de Google Calendar",
            },
            errors: {
                invalidGapiKey: "Format de Clau de Compte de Servei de Google Calendar invàlid",
            },
        },
        lookAndFeelScreen: {
            common: {
                helpLinks: "Enllaços d'Ajuda",
                logoUrl: "URL del Logotip",
                css: "CSS Personalitzat",
                displayName: "Nom visible",
                displayNameHelp:
                    "El nom de l'organització en els missatges que l'esmenten. Buit: el nom curt del llogater.",
            },
            errors: {
                invalidHelpLinks: "Format d'Enllaços d'Ajuda invàlid",
            },
        },
        electionTypeScreen: {
            noPermissions: "No tens permís per accedir a la configuració.",
            common: {
                title: "Tipus d'Elecció",
                subtitle: "Configuració del Tipus d'Elecció",
                onlineVoting: "Votació en Línia",
                kioskVoting: "Votació en Quiosc",
                telephoneVoting: "Votació Telefònica",
                settingTitle: "Configuració",
                settingSubtitle: "Ajustos generals",
                sms: "SMS",
                mail: "Correus",
                createNew: "Crear un Tipus d'Elecció",
                emptyHeader: "No hi ha Tipus d'Elecció encara.",
                emptyBody: "Vols crear-ne un?",
            },
            create: {
                title: "Crear Tipus d'Elecció",
            },
            edit: {
                title: "Editar Tipus d'Elecció",
            },
            tabs: {
                votingChannels: "CANALS DE VOTACIÓ",
                electionTypes: "TIPUS D'ELECCIÓ",
                templates: "PLANTILLES",
                localization: "LOCALITZACIÓ",
                languages: "IDIOMES",
                integrations: "INTEGRACIONS",
                lookAndFeel: "PERSONALITZACIÓ D'APARENÇA",
                schedules: "ESDEVENIMENTS PROGRAMATS",
                trustees: "AUTORITATS",
                BackupRestore: "SAUVEGARDE / RESTAURATION",
            },
        },
        trusteesSettingsScreen: {
            common: {
                emptyHeader: "Encara no hi ha fiduciàries.",
                createNew: "Crear fiduciari",
                title: "Fiduciari",
                subtitle: "Configuració del fiduciari",
                emptyBody: "Vols crear-ne un?",
            },
            create: {
                title: "Crear fiduciari",
            },
            edit: {
                title: "Editar fiduciari",
            },
        },
        scheduleScreen: {
            noPermissions: "No tens permís per accedir a la configuració.",
            createScheduleSuccess: "Data creada",
            createScheduleError: "Error creant data",
            deleteScheduleSuccess: "Data esborrada",
            deleteScheduleError: "Error esborrant data",
            common: {
                title: "Programat",
                subtitle: "Configuració de dates",
                createNew: "Crear data",
                emptyHeader: "Encara no hi ha dates.",
                emptyBody: "Vols crear-ne una?",
            },
            create: {
                title: "Crear data",
                selectSchedule:
                    "Seleccioneu un horari de la llista predefinida o escriviu-ne un de personalitzat",
            },
            edit: {
                title: "Editar data",
            },
            eventTypes: {
                SYSTEM_LOCKDOWN_FOR_INTERNET_VOTING_SETTINGS:
                    "Bloqueig del sistema per a la finalització de la configuració de votació per Internet",
                START_PRE_REGISTRATION_OVCS: "Inici de la preinscripció per a OVCS",
                END_PRE_REGISTRATION_OVCS: "Fi de la preinscripció per a OVCS",
                START_TEST_VOTING_PERIOD: "Inici del període de votació de prova",
                END_TEST_VOTING_PERIOD: "Fi del període de votació de prova",
                START_INTERNET_VOTING_PERIOD: "Inici del període de votació per Internet",
                END_INTERNET_VOTING_PERIOD: "Fi del període de votació per Internet",
                LAB_TEST: "Prova de laboratori",
                FIELD_TEST: "Prova de camp",
                MOCK_ELECTIONS: "Eleccions simulades",
                FTS: "FTS",
            },
        },
        dashboard: {
            voteByDay: "Vots per dia",
            votesOverTime: "Vots al llarg del temps",
            timeResolution: "Resolució temporal",
            timeRange: "Interval de temps",
            minute: "Minut",
            hour: "Hora",
            day: "Dia",
            votersByChannels: "Votants per canals",
            voterLoginURL: "URL d'inici de sessió dels votants",
            voterEnrollURL: "URL d'inscripció de votants",
            voterEnrollKioskURL: "Kiosk URL d'inscripció de votants",
            ipAddress: {
                emptyState: "Encara no hi ha vots.",
                title: "IP Addresses",
                ip: "IP",
                country: "País",
                VoteCount: "Nombre de vots",
                ElectionName: "Nom de l'elecció",
                VotersId: "Identificadors dels votants",
            },
        },
        electionEventScreen: {
            common: {
                subtitle: "Configuració de l'Esdeveniment Electoral.",
                showMore: "Mostra'n més",
                showLess: "Mostra'n menys",
                adminPortal: "Portal d'administració",
                allowPublishAfterLockdown: "Only allow election event publishing after lockdown",
                reset: "Restablir filtre personalitzat",
            },
            edit: {
                general: "General",
                dates: "Dates",
                customUrls: "Prefix d'URL personalitzats",
                language: "Idiomes",
                votingPeriod: "Període de votació",
                allowed: "Canals de Vot Permesos",
                materials: "Materials de Suport",
                ballotDesign: "Disseny de la Papereta",
                templates: "Plantillas",
                reorder: "Reordenar eleccions",
                advancedConfigurations: "Voting Portal Countdown Policy",
                importCandidates: "Importar Candidats",
                custom_filters: "Filtres personalitzats",
                voter_authentication: "Autenticació del votant",
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
                login: "Inici de sessió",
                enrollment: "Inscripció",
            },
            localization: {
                emptyHeader: "No s'han establert idiomes per a l'esdeveniment",
                selectLanguage: "Selecciona idioma",
                notify: {
                    success: "La localització s'ha actualitzat correctament",
                    error: "La actualització de la localització ha fallat",
                    duplicateKey:
                        "Ja existeix una substitució amb aquesta clau i àmbit del portal.",
                    invalidDateTimeFormat:
                        "Format de data/hora no vàlid. Utilitza els tokens yyyy, MM, dd, HH, mm, ss (p. ex. dd/MM/yyyy HH:mm).",
                    invalidTimeZoneText: "Aquest text ha de conservar {{placeholders}}.",
                },
                common: {
                    title: "Localització",
                    subTitle: "Configuració de la localització",
                },
                labels: {
                    key: "Clau",
                    scope: "Àmbit del portal",
                    value: "Valor",
                },
                scopes: {
                    legacy: "Anterior ({{portal}})",
                    global: "Global",
                    votingPortal: "Portal de votació",
                    ballotVerifier: "Verificador de paperetes",
                    resultsPortal: "Portal de resultats",
                    adminPortal: "Portal d'administració",
                    templates: "Informes i missatges",
                },
            },
            field: {
                passwordPolicy: {
                    minimumLength: "Longitud mínima",
                    maximumLength: "Longitud màxima",
                    includeUppercase: "Incloure lletres majúscules",
                    includeLowercase: "Incloure lletres minúscules",
                    includeDigits: "Incloure dígits",
                    includeSpecialCharacters: "Incloure caràcters especials",
                    help: {
                        minimumLength:
                            "El nombre mínim de caràcters necessaris per a la contrasenya.",
                        maximumLength:
                            "El nombre màxim de caràcters permesos per a la contrasenya.",
                        includeUppercase:
                            "La contrasenya ha d'incloure almenys una lletra majúscula.",
                        includeLowercase:
                            "La contrasenya ha d'incloure almenys una lletra minúscula.",
                        includeDigits: "La contrasenya ha d'incloure almenys un dígit.",
                        includeSpecialCharacters:
                            "La contrasenya ha d'incloure almenys un caràcter especial.",
                    },
                    notConfigured:
                        "No hi ha cap política de contrasenyes configurada. En desar, s'aplicaran els valors predeterminats següents.",
                    errors: {
                        lengthRange:
                            "Les longituds de la contrasenya han de ser nombres enters entre 1 i 256.",
                        minimumExceedsMaximum:
                            "La longitud mínima no pot superar la longitud màxima.",
                        characterClassRequired:
                            "Seleccioneu almenys una classe de caràcters per a la contrasenya.",
                    },
                },
                name: "Nom",
                alias: "Àlies",
                description: "Descripció",
                startDateTime: "Data i hora d'inici",
                endDateTime: "Data i hora de finalització",
                language: "Idioma",
                votingChannels: "Canals de Vot",
                materialActivated: "Materials de Suport activats",
                supportMaterialsPolicy: {
                    label: "Política de Materials de Suport",
                    helperText:
                        "L'opció Obligatòria per Votar requereix que els votants obrin cada Material de Suport i confirmin que l'han llegit abans de poder votar.",
                    options: {
                        off: "Desactivada",
                        optional: "Opcional",
                        mandatory_for_voting: "Obligatòria per Votar",
                    },
                },
                materialTitle: "Títol",
                materialSubTitle: "Subtítol",
                logoUrl: "URL del Logotip",
                userVerification:
                    "Podeu introduir una plantilla personalitzada que s'utilitzarà per verificar manualment els votants",
                redirectFinishUrl: "URL de redirecció en finalitzar",
                kioskRedirectFinishUrl: "URL de redirecció en finalitzar del quiosc",
                css: "CSS personalitzat",
                skipElectionList: "Saltar pantalla per escollir elecció",
                showUserProfile: "Mostra el perfil de l'usuari",
                showCastVoteLogs: {
                    policyLabel: "Mostra els registres de votació",
                    options: {
                        "show-logs-tab": "Mostra els registres de votació",
                        "hide-logs-tab": "Amaga els registres de votació",
                    },
                },
                lockdownState: {
                    policyLabel: "Estat de Confinament",
                    helperText:
                        "Programeu l’inici o el final del període de bloqueig per canviar aquest estat.",
                    options: {
                        "locked-down": "Confinat",
                        "not-locked-down": "No Confinat",
                    },
                },
                decodedBallots: {
                    policyLabel:
                        "Inclou les paperetes descodificades a la base de dades de resultats",
                    options: {"included": "Inclou", "not-included": "No incloguis"},
                },
                contestEncryptionPolicy: {
                    options: {
                        "single-contest": "Concurs únic",
                        "multiple-contests": "Diversos concursos",
                    },
                    policyLabel: "Política de xifrat de concurs",
                },
                votingPortalDateTimeFormat: {
                    policyLabel: "Format de data i hora del portal de votació",
                    helperText:
                        "S'aplica a tot l'esdeveniment. Per substituir-ho per idioma, afegeix la clau \"votingPortalDateTimeFormat\" a la pestanya Localització amb els tokens yyyy, MM, dd, HH, mm, ss (p. ex. dd/MM/yyyy HH:mm). Consulta la documentació per a més detalls.",
                    options: {
                        "legacy-gb-24h": "Legacy GB 24h (dd/MM/yyyy HH:mm, 24h)",
                        "iso-local": "ISO Local (yyyy-MM-dd HH:mm)",
                        "us-12h": "US 12h (MM/dd/yyyy h:mm AM/PM)",
                        "locale-medium": "Locale Medium (data mitjana, hora curta)",
                        "date-only": "Date Only (sense hora)",
                        "custom": "Format personalitzat",
                    },
                    customFormat: {
                        label: "Format de data i hora personalitzat",
                        helperText:
                            "Utilitza els tokens yyyy, MM, dd, HH, mm, ss (p. ex. dd/MM/yyyy HH:mm). Qualsevol altre caràcter es mostra literalment.",
                        invalid:
                            "Format no vàlid. Utilitza almenys un dels tokens yyyy, MM, dd, HH, mm, ss.",
                    },
                },
                countDownPolicyOptions: {
                    NO_COUNTDOWN: "Sense compte enrere",
                    COUNTDOWN: "Compte enrere",
                    COUNTDOWN_WITH_ALERT: "Compte enrere amb avís",
                    sectionTitle: "Portal de votació",
                    policyLabel: "Política de compte enrere del portal de votació",
                    coundownSecondsLabel:
                        "temps en segons abans de la caducitat per mostrar el compte enrere",
                    alertSecondsLabel:
                        "temps en segons abans de la caducitat per mostrar l'avís de tancament de sessió",
                },
                voterSigningPolicy: {
                    "policyLabel": "Política de Signatura de Votants",
                    "no-signature": "Sense signatura",
                    "with-signature": "Amb signatura",
                },
                VoterCertificatePolicy: {
                    policyLabel: "Voter Digital Certificate Policy",
                    enabled: "Habilitat",
                    disabled: "Deshabilitat",
                },
                enrollment: {
                    policyLabel: "Inscripció",
                    options: {
                        enabled: "Habilitat",
                        disabled: "Deshabilitat",
                    },
                },
                otp: {
                    policyLabel: "OTP",
                    options: {
                        enabled: "Habilitat",
                        disabled: "Deshabilitat",
                    },
                },
                ceremoniesPolicy: {
                    policyLabel: "Política de cerimònies de claus/recompte",
                    options: {
                        "automated-ceremonies": "Permetre cerimònies automàtiques",
                        "manual-ceremonies": "Cerimònies manuals",
                    },
                },
                automaticRecountPolicy: {
                    policyLabel: "Recompte automàtic després d'aprovar una importació",
                    options: {
                        enabled: "Activat",
                        disabled: "Desactivat",
                    },
                },
                weightedVotingPolicy: {
                    policyLabel: "Política de Votació Ponderada",
                    options: {
                        "areas-weighted-voting": "Votació Ponderada per Àrees",
                        "voters-weighted-voting": "Votació Ponderada per Votants",
                        "disabled-weighted-voting": "Votació Ponderada Desactivada",
                    },
                    noDelegated:
                        "La Votació Ponderada per Votants no es pot combinar amb el Vot Delegat",
                    noDecodedBallots:
                        "La Votació Ponderada per Votants no es pot combinar amb la inclusió de paperetes desxifrades als resultats",
                },
                delegatedVotingPolicy: {
                    policyLabel: "Política de Votació Delegada",
                    options: {
                        enabled: "Activada",
                        disabled: "Desactivada",
                    },
                },
                languageDetectionPolicy: {
                    policyLabel: "Política de detecció de llengua",
                    options: {
                        "browser-detect": "Detectar del navegador",
                        "force-default": "Forçar per defecte",
                    },
                },
            },
            error: {
                endDate: "La data de finalització ha de ser posterior a la data d'inici",
                startDate: "La data d'inici ha de ser en el futur",
                noResult: "Encara no hi ha Esdeveniment Electoral",
                endDateInvalid: "La data de finalització ha de ser en el futur",
            },
            voters: {
                title: "Votants",
            },
            createElectionEventSuccess: "Esdeveniment Electoral creat",
            createElectionEventError: "Error creant Esdeveniment Electoral",
            ivr: {
                tabs: {
                    config: "Configuració",
                    blacklist: "Llista de bloqueig",
                    prompts: "Locucions",
                    emulator: "Emulador",
                },
                common: {
                    saveSuccess: "S'ha desat correctament",
                    saveError: "No s'ha pogut desar",
                    deleteSuccess: "S'ha eliminat correctament",
                    deleteError: "No s'ha pogut eliminar",
                },
                config: {
                    configuredPhone: "Número de telèfon configurat",
                    infoMsg:
                        "Configureu el flux de l’IVR i les seves propietats a continuació. Per a més informació, poseu-vos en contacte amb Sequent.",
                },
                prompts: {
                    emptyMsg: "Encara no s’ha creat cap missatge",
                    infoMsg:
                        "Configureu els missatges utilitzats per l’IVR. Els missatges d’anunci són obligatoris, i els missatges del sistema es poden sobreescriure per als idiomes desitjats. S’admet SSML, també per barrejar idiomes.",
                    editorTitle: "Missatge",
                    editorSubtitle: "Configuració del missatge",
                },
                blacklist: {
                    columns: {
                        phone: "Número de telèfon",
                        reason: "Motiu",
                        createdAt: "Creat el",
                        createdBy: "Creat per",
                        createdBefore: "Creat abans",
                        createdAfter: "Creat després",
                    },
                    emptyMsg: "No hi ha cap entrada a la llista de bloqueig",
                    infoMsg:
                        "Configureu la llista de bloqueig de l’IVR. Les trucades d’aquests números seran desconnectades automàticament pel sistema.",

                    noFilterMatch: "Cap entrada no coincideix amb els filtres indicats",
                    phoneRequired: "El número de telèfon és obligatori",
                },
                emulator: {
                    infoMsg:
                        "Seleccioneu una àrea i les eleccions desitjades per provar la sessió IVR.",
                    apiStatus: {
                        unavailable: "El sistema de l'emulador no està disponible al vostre entorn",
                        loading: "S'està carregant el sistema de l'emulador",
                        error: "S'ha produït un error en carregar el sistema de l'emulador",
                    },
                    hints: {
                        title: "Consells",
                        publishRequired:
                            "Qualsevol canvi fet a les eleccions, les conteses o els candidats s'ha de publicar primer perquè estigui disponible. A l'emulador només s'utilitzaran els estils de papereta publicats més recentment per a l'àrea corresponent.",
                        eventChangesImmediate:
                            "Els canvis fets a l'esdeveniment electoral, com ara la configuració IVR o les substitucions dels missatges, estan disponibles immediatament en reiniciar la sessió de l'emulador.",
                        credentials:
                            'L\'identificador de votant i el PIN vàlids són "123" i "123".',
                    },
                    sendDtmf: "Envia una entrada DTMF",
                    keypadInput: "Entrada del teclat",
                    sendTimeout: "Envia el temps d'espera",
                    disconnected: "Desconnectat",
                    startSession: "Inicia una sessió nova",
                    endSession: "Finalitza la sessió",
                    noStylesFound:
                        "No s'ha trobat cap estil de papereta publicat que coincideixi amb les vostres seleccions",
                    inputPlaceholder: "Premeu {{keys}} (temps d'espera={{timeout}} s)",
                    inputPlaceholderOr: "o",
                    inputPlaceholderAnyKeys:
                        "Introduïu fins a {{maxDigits}} dígits (qualsevol dígit, temps d'espera={{timeout}} s)",
                    blacklistCaller: "Bloqueja la persona que truca",
                    elections: "Eleccions",
                    area: "Àrea",
                },
            },
            stats: {
                elegibleVoters: "Electors",
                voters: "Votants",
                elections: "Eleccions",
                contests: "Preguntes",
                areas: "Àrees",
                sentEmails: "Correus enviats",
                sentSMS: "SMS enviats",
                calendar: {
                    title: "Calendari",
                    scheduled: "Programat",
                },
            },
            keys: {
                createNew: "Crear Cerimònia de Claus",
                emptyHeader: "Encara no hi ha Cerimònia de Claus.",
                statusLabel: "Estat",
                waitingKeys: "Esperant a la Generació de Claus..",
                started: "Iniciada en",
                actions: {
                    participate: "Participa en la cerimònia de claus",
                    view: "Mostra la cerimònia de claus",
                },
                breadCrumbs: {
                    configure: "Configurar",
                    ceremony: "Cerimònia",
                    created: "Acabat",
                    start: "Començament",
                    status: "Estat",
                    download: "Descarregar",
                    check: "Comprovar",
                    success: "Finalitzat",
                },
                notify: {
                    participateNow:
                        "Ha estat convidat a participar a una Cerimònia de Claus. Si us plau <1>feu clic a continuació en l'acció de clau de la cerimònia</1> per participar.",
                },
            },
            tabs: {
                dashboard: "Tauler de Control",
                monitoring: "Monitoratge",
                data: "Dades",
                ivr: "IVR",
                localization: "Localització",
                voters: "Votants",
                areas: "Àrees",
                keys: "Claus",
                tally: "Recompte",
                tallySheetImports: "Importació d'actes",
                publish: "Publicar",
                logs: "Registres",
                tasks: "Tasques",
                events: "Esdeveniment Programat",
                notifications: "Notificacions",
                reports: "Informe",
                approvals: "Aprovacions",
                cas: "Certificats",
            },
            tally: {
                emptyHeader: "Encara no hi ha Recompte.",
                title: "Recompte de l'Esdeveniment Electoral",
                elections: "Eleccions",
                electionNumber: "Número d'Eleccions",
                trustees: "Trustees",
                permissionLabels: "Etiquetes de Permís",
                status: "Estat",
                tallyType: {
                    label: "Tipus de Recompte",
                    ELECTORAL_RESULTS: "Resultats Electorals",
                    INITIALIZATION_REPORT: "Resultats d'Inicialització",
                },
                create: {
                    title: "Crear Recompte",
                    subtitle: "Crear un nou Recompte per a aquest Esdeveniment Electoral",
                    createTallyButton: "Iniciar la Cerimònia de Recompte",
                    createInitializationReportButton: "Crear Informe d'Inicialització",
                    error: {
                        create: "Error creant Recompte",
                    },
                    success: "Recompte creat",
                },
                logs: {
                    noLogs: "No hi ha registres disponibles",
                },
                notify: {
                    noKeysTally:
                        "La Cerimònia del Compte no pot començar fins que la Cerimònia de Fideïcomissaris no s'hagi completat amb èxit.",
                    noPublication:
                        "La Cerimònia de Còmput no pot començar fins que no creïs una publicació a la pestanya Publicar.",
                    participateNow:
                        "Ha estat convidat a participar a una Cerimònia de Recompte. Si us plau <1>feu clic a continuació en l'acció de recompte de la cerimònia</1> per participar.",
                    startDisabled:
                        "No podeu continuar amb la cerimònia perquè no s'ha seleccionat cap elecció o les eleccions no estan publicades.",
                    ceremonyDisabled:
                        "No podeu continuar amb la cerimònia perquè la sessió de recompte no està connectada o l'inici de la cerimònia no està permès.",
                },
            },
            importAreas: {
                title: "Importar Àrees",
                subtitle: "Importar dades d'àrees",
                areaParagraph:
                    "Importar àrees utilitzant un fitxer de full de càlcul en format de valors separats per comes (CSV).",
                importSuccess: "Àrees Importades amb Èxit",
                importError: "Error en importar Àrees",
                upsert: "Upsert Areas",
            },
            import: {
                eetitle: "Importar Esdeveniment Electoral",
                eesubtitle: "Importar dades de l'Esdeveniment Electoral",
                title: "Importar votants",
                subtitle: "Importar votants a l'Esdeveniment Electoral",
                voters: "Votants",
                votersParagraph:
                    "Importa votants utilitzant una fulla de càlcul en format Comma Separated Values (CSV). Descarregueu un exemple de fitxer d'importació CSV aquí.",
                electionEventParagraph:
                    "Importa Esdeveniments Electorals utilitzant un fitxer JSON.",
                elections: "Eleccions",
                areas: "Àrees",
                sha: "Verificació d'Integritat (SHA 256)",
                cancel: "Cancel·lar",
                import: "Importar",
                fileUploadSuccess: "Fitxer pujat al servidor - però no importat encara",
                fileUploadError: "Error pujant el fitxer",
                importVotersSuccess: "Importació de Votants llançada en segon pla amb èxit.",
                importVotersError: "Error Important Votants.",
                importElectionEventSuccess: "Election event imported Successfully",
                importElectionEventError: "Error importing election event",
                shaDialog: {
                    ok: "Sí, Importar Sense Verificació d'Integritat",
                    cancel: "Tornar",
                    title: "Importar Sense Verificació d'Integritat?",
                    description:
                        "No va introduir el camp Verificació d'integritat (SHA-256). Confirmeu que està importanr el fitxer correcte i que desitja importar-lo.",
                },
                passwordDialog: {
                    title: "Contrasenya de Desxifrat",
                    description: "Introdueix la contrasenya per desxifrar l'arxiu",
                    label: "Contrasenya",
                    copyPassword: "Copiar Contrasenya",
                    ok: "D'acord",
                },
            },
            export: {
                title: "Exportar Esdeveniment Electoral",
                subtitle:
                    "L'exportació pot ser una operació llarga. Estàs segur que vols exportar els registres?",
                encryptWithPassword: "Xifrar amb Contrasenya",
                passwordForcedNote:
                    "L'arxiu es protegirà amb contrasenya igualment: els informes, les sol·licituds i les dades del tauler sempre es xifren. Marca la casella per incloure també els camps secrets de votant desxifrats.",
                includeVoters: "Incloure Votants",
                activityLogs: "Registres d'Activitat",
                bulletinBoard: "Tauler d'Anuncis",
                publications: "Publicacions",
                s3Files: "Fitxers S3",
                scheduledEvents: "Esdeveniments Programats",
                exportSuccess: "Esdeveniment Electoral exportat amb èxit",
                exportError: "Error en exportar l'Esdeveniment Electoral",
                passwordTitle: "Contrasenya",
                passwordDescription: "Contrasenya per desxifrar el fitxer:",
                copiedSuccess: "Contrasenya copiada al porta-retalls",
                copiedError: "Error copiant la contrasenya",
                reports: "Informes",
                applications: "Aplicacions",
                tally: "Recompte",
                certificates: "Certificats",
            },
            taskNotification:
                "{{action}} ha començat. Podeu veure el seu estat a la taula d'Execució de Tasques.",
        },
        electionScreen: {
            common: {
                title: "Elecció",
                subtitle: "Configuració de l'elecció.",
                fileLoaded: "Fitxer carregat",
                noPermission: "No tens permís per accedir a aquesta elecció.",
            },
            edit: {
                general: "General",
                dates: "Dates",
                votingPeriod: "Període de votació",
                language: "Idioma",
                allowed: "Canals de Vot Permesos",
                default: "Per defecte",
                defaultLang: "Idioma per defecte",
                receipts: "Rebuts",
                image: "Imatge",
                advanced: "Configuració Avançada",
                numAllowedVotes: "Número de vots permesos",
                reorder: "Reordenar concursos",
                castVoteConfirm: "Modal de Confirmació de Vot",
                gracePeriodPolicy: "Política de període de gràcia",
                allowTallyPolicy: "Permetre recompte",
                permissionLabel: "Etiqueta de permís",
                custom_filters: "Filtres personalitzats",
            },
            field: {
                name: "Nom",
                language: "Idioma",
                votingChannels: "Canals de Vot",
                startDateTime: "Data i hora d'inici",
                endDateTime: "Data i hora de finalització",
                startDateTimeWithTimezone: "Data i hora d'inici ({{timezone}})",
                endDateTimeWithTimezone: "Data i hora de finalització ({{timezone}})",
                scheduledOpening: "Obertura Programada",
                scheduledClosing: "Tancament Programat",
                alias: "Àlies",
                description: "Descripció",
                securityConfirmationHtml: "Confirmació de seguretat HTML",
                ivrPrompt: "Missatge IVR",
                externalId: "ID extern",
            },
            securityConfirmationPolicy: {
                label: "Política de la casella de confirmació de seguretat",
                none: "Cap",
                mandatory: "Obligatori",
            },
            error: {
                fileError: "Error al carregar el fitxer",
                fileLoaded: "Fitxer carregat",
                endDate: "La data de finalització ha de ser posterior a la data d'inici",
                startDate: "La data d'inici ha de ser en el futur",
                endDateInvalid: "La data de finalització ha de ser en el futur",
            },
            createElectionEventSuccess: "Creada l'elecció",
            createElectionEventError: "Error Creant l'elecció",
            tabs: {
                dashboard: "Tauler de Control",
                monitoring: "Monitoratge",
                data: "Dades",
                voters: "Votants",
                publish: "Publicar",
                logs: "Registres",
                approvals: "Aprovacions",
                tallySheets: "Fulls de recompte",
            },
            gracePeriodPolicy: {
                "label": "Política de període de gràcia",
                "no-grace-period": "Sense període de gràcia",
                "grace-period-without-alert": "Període de gràcia sense avís",
                "gracePeriodSecs": "Període de gràcia (segons)",
            },
            allowTallyPolicy: {
                "allowed": "Permès",
                "disallowed": "No Permès",
                "requires-voting-period-end": "Requereix el final del període de votació",
            },
            initializeReportPolicy: {
                "label": "Inicialitzar política d'informes",
                "not-required": "No requereix",
                "required": "Requerit",
            },
            castVoteGoldLevelPolicy: {
                label: "Gold level Authentication Policy",
                options: {
                    "gold-level": "Gold level Authentication",
                    "no-gold-level": "No Gold level Authentication",
                },
            },
            startScreenTitlePolicy: {
                label: "Política de títol de la pantalla d'inici",
                options: {
                    "election": "Títol de l'elecció",
                    "election-event": "Títol de l'esdeveniment electoral",
                },
            },
            consolidatedReportPolicy: {
                label: "Política d'informe consolidat",
                options: {
                    "generate": "Generar",
                    "do-not-generate": "No generar",
                },
            },
            declineToVotePolicy: {
                label: "Política de declinació de vot",
                options: {
                    enabled: "Habilitat",
                    disabled: "Desactivat",
                },
            },
            blankBallotsPolicy: {
                label: "Política de paperetes en blanc",
                options: {
                    enabled: "Habilitat",
                    disabled: "Desactivat",
                },
            },
            votingScreenBackPolicy: {
                label: "Política del botó Enrere de la pantalla de votació",
                options: {
                    "election-selection-screen": "Vés a la pantalla de selecció d'eleccions",
                    "start-screen": "Vés a la pantalla d'inici de l'elecció",
                },
            },
        },
        tenantScreen: {
            common: {
                title: "Llogaters",
            },
            new: {
                subtitle: "Crear nou llogater",
            },
            createSuccess: "Llogater creat",
            createError: "Error en crear el llogater",
        },
        usersAndRolesScreen: {
            noPermissions: "No tens permís per accedir als usuaris o rols.",
            common: {
                title: "Usuaris i Rols",
                subtitle: "Configuració general",
                mobileNumber: "Mòbil",
            },
            editPassword: {
                passwordPolicyViolation:
                    "La contrasenya no compleix la Política de contrasenyes d'aquest esdeveniment electoral. Reviseu la política a Dades de l'esdeveniment electoral i introduïu una contrasenya vàlida.",
                passwordPolicyRules: {
                    minimumLength: "La longitud mínima de la contrasenya és {{count}}.",
                    maximumLength: "La longitud màxima de la contrasenya és {{count}}.",
                    uppercase: "Caràcters en majúscula necessaris: {{count}}.",
                    lowercase: "Caràcters en minúscula necessaris: {{count}}.",
                    digits: "Dígits necessaris: {{count}}.",
                    specialCharacters: "Caràcters especials necessaris: {{count}}.",
                },
                label: "Canviar contrasenya",
                temporatyLabel: "Temporal",
                temporatyInfo:
                    "Si està habilitat, l'usuari haurà de canviar la contrasenya en el pròxim inici de sessió.",
            },
            users: {
                title: "Usuaris",
                subtitle: "Veure i editar dades de l'usuari",
                review: {
                    title: "Revisar canvis",
                    subtitle: "Confirma aquestes actualitzacions abans d'enviar-les.",
                    confirm: "Confirmar canvis",
                    noChanges: "No hi ha canvis per revisar",
                    field: "Camp",
                    currentValue: "Valor actual",
                    newValue: "Valor nou",
                },
                edit: {
                    title: "Informació de l'Usuari",
                    subtitle: "Veure i editar Usuari",
                },
                create: {
                    title: "Usuari",
                    subtitle: "Crear usuari",
                },
                fields: {
                    "has_voted": "Ha votat",
                    "support_materials_viewed": "Support Materials Viewed",
                    "vote-weight": "Pes del vot",
                    "voted-channel": "Canal de vot",
                    "disable-comment": "Comentari de desactivació",
                    "username": "Nom d'Usuari",
                    "first_name": "Nom",
                    "last_name": "Cognom",
                    "email": "Correu",
                    "enabled": "Habilitat",
                    "emailVerified": "Correu Verificat",
                    "groups": "Grups",
                    "attributes": "Atributs",
                    "area": "Àrea",
                    "password": "Contrasenya",
                    "savePassword": "Desa la contrasenya",
                    "repeatPassword": "Repetir Contrasenya",
                    "passwordMismatch": "Les contrasenyes han de coincidir",
                    "passwordLengthValidate": "La contrasenya ha de tenir almenys 8 caràcters",
                    "passwordUppercaseValidate":
                        "La contrasenya ha de contenir almenys una lletra majúscula",
                    "passwordLowercaseValidate":
                        "La contrasenya ha de contenir almenys una lletra minúscula",
                    "passwordDigitValidate": "La contrasenya ha de contenir almenys un dígit",
                    "passwordSpecialCharValidate":
                        "La contrasenya ha de contenir almenys un caràcter especial",
                    "trustee": "Actuar com a fideïcomissari",
                    "permissionLabel": "Etiqueta de permís",
                    "authorized-election-ids": "Eleccions",
                },
                delete: {
                    body: "Estàs segur que vols esborrar aquest usuari?",
                    bulkBody: "Estàs segur que vols esborrar els usuaris seleccionats?",
                    bulkBodySelected: "Delete the {{count}} selected users? This cannot be undone.",
                    bulkBodyChoose:
                        "{{count}} users are selected. You can instead delete every user matching the current filters, which may be more. This cannot be undone.",
                    okSelected: "Delete {{count}} selected",
                    okAllMatching: "Delete all matching",
                },
                notifications: {
                    exportError: "Error exportant usuaris",
                    deleteError: "Error esborrant usuari",
                    deleteSuccess: "Usuari esborrat",
                    multipleDeleteSuccess: "Usuaris esborrats",
                },
            },
            voters: {
                voterInformationLetter: {
                    label: "Carta d'informació per al votant",
                    generate: "Genera",
                    confirmation:
                        "Voleu generar una Carta d'informació per a aquest votant? S'assignarà una contrasenya nova i s'inclourà en un PDF xifrat.",
                    generationStarted: "S'ha iniciat la generació de la Carta d'informació",
                    generationError: "No s'ha pogut generar la Carta d'informació",
                    policyNotConfigured:
                        "La Política de contrasenyes no està configurada. Configureu-la a Dades de l'esdeveniment electoral abans de generar una carta.",
                    policyMinimumLengthMissing:
                        "La Política de contrasenyes ha d'incloure una longitud mínima abans de generar una carta.",
                    policyCharacterClassMissing:
                        "La Política de contrasenyes ha d'incloure almenys una classe de caràcters abans de generar una carta.",
                },
                title: "Votants",
                subtitle: "Veure i editar dades del votant",
                secretAttribute: {
                    storedPlaceholder: "Valor xifrat emmagatzemat",
                    reveal: "Mostra",
                    hide: "Amaga",
                    revealError: "No s'ha pogut mostrar el camp xifrat del votant",
                    includeInExport: "Inclou els camps secrets desxifrats del votant",
                    exportWarning:
                        "Exportació sensible: el CSV descarregat contindrà aquests camps en text pla.",
                    clear: "Esborra",
                    add: "Afegeix un valor",
                    remove: "Elimina el valor",
                },
                review: {
                    title: "Revisar canvis",
                    subtitle: "Confirma aquestes actualitzacions abans d'enviar-les.",
                    confirm: "Confirmar canvis",
                    noChanges: "No hi ha canvis per revisar",
                    field: "Camp",
                    currentValue: "Valor actual",
                    newValue: "Valor nou",
                },
                logs: {
                    label: "Registres de l'usuari",
                },
                emptyHeader: "Encara no hi ha votant.",
                askCreate: "Vols crear-ne un?",
                create: {
                    title: "Votant",
                    subtitle: "Crear votant",
                },
                manualVerification: {
                    label: "Verificar manualment",
                    verify: "Verificar manualment al votant",
                    body: "Verifiqueu manualment a aquest votant. Obtindrà un PDF amb un enllaç de codi QR que permet al votant iniciar sessió ometent el KYC en línia.",
                    noEmailOrPhone:
                        "Aquest votant no pot ser verificat manualment perquè no té una adreça de correu electrònic o un número de telèfon assignat.",
                },
                errors: {
                    editError: "Error editant votant",
                    editErrorReason: "Error editant votant: {{reason}}",
                    editSuccess: "Votant editat",
                    createError: "Error creant votant",
                    createErrorReason: "Error creant votant: {{reason}}",
                    createSuccess: "Votant creat",
                    attribute: {
                        invalidNamed: 'S\'ha rebutjat "{{field}}": {{constraint}}',
                        fieldsToCorrect: "Alguns camps s'han de corregir abans de desar",
                        hintBetween: "Entre {{min}} i {{max}} caràcters",
                        hintMin: "Com a mínim {{min}} caràcters",
                        hintMax: "Com a màxim {{max}} caràcters",
                        andMore: "i {{count}} més",
                        invalidLength: '"{{field}}" ha de tenir entre {{min}} i {{max}} caràcters',
                        tooShort: '"{{field}}" ha de tenir com a mínim {{min}} caràcters',
                        tooLong: '"{{field}}" ha de tenir com a màxim {{max}} caràcters',
                        required: '"{{field}}" és obligatori',
                        invalidEmail:
                            '"{{field}}" ha de ser una adreça de correu electrònic vàlida',
                        invalidFormat: '"{{field}}" no té el format esperat',
                        invalid: '"{{field}}" té un valor no vàlid',
                    },
                    createPasswordError:
                        "Votant creat, però no s'ha pogut establir la seva contrasenya",
                    createPasswordErrorReason:
                        "Votant creat, però no s'ha pogut establir la seva contrasenya: {{reason}}",
                },
                delete: {
                    body: "Estàs segur que vols esborrar aquest votant?",
                    bulkBody: "Estàs segur que vols esborrar els votants seleccionats?",
                    bulkBodySelected:
                        "Delete the {{count}} selected voters? This cannot be undone.",
                    bulkBodyChoose:
                        "{{count}} voters are selected. You can instead delete every voter matching the current filters, which may be more. This cannot be undone.",
                    okSelected: "Delete {{count}} selected",
                    okAllMatching: "Delete all matching",
                },
                notifications: {
                    exportError: "Error exportant votants",
                    deleteError: "Error esborrant votant",
                    deleteSuccess: "Votant esborrat",
                    multipleDeleteSuccess: "Votants esborrats",
                    manualVerificationError: "Error verificant manualment al votant",
                    manualVerificationSuccess:
                        "Verificat amb èxit manualment al votant, descarregar PDF..",
                },
            },
            roles: {
                title: "Rols",
                edit: {
                    title: "Informació del Rol",
                    subtitle: "Veure i editar Rol",
                },
                create: {
                    title: "Rol",
                    subtitle: "Crear rol",
                },
                errors: {
                    createError: "Error creant rol",
                    createSuccess: "Rol creat",
                },
                fields: {
                    name: "Nom",
                },
                delete: {
                    body: "Estàs segur que vols esborrar aquest rol?",
                },
                notifications: {
                    deleteError: "Error esborrant rol",
                    deleteSuccess: "Rol esborrat",
                    permissionEditError: "Error editant permís",
                    permissionEditSuccess: "Permís editat",
                },
            },
            permissions: {
                "voter-information-letter": "Genera una Carta d'informació per al votant",
                "admin-user": "Administració",
                "admin-dashboard-view": "Vista del Tauler d'Administració",
                "monitoring-view": "Veure Taulers de Monitoratge",
                "monitoring-configure": "Configurar Taulers de Monitoratge",
                "election-event-signatures-tab": "Pestanya Signatures de l'Esdeveniment Electoral",
                "signing-rules-read": "Signatures: veure accions protegides",
                "signing-rules-write": "Signatures: editar accions protegides",
                "signing-certificates-read": "Signatures: veure certificats",
                "signing-issuers-write": "Signatures: importar i suprimir emissors de confiança",
                "signing-checks-write": "Signatures: editar comprovacions de certificats",
                "signing-certificates-register": "Signatures: registrar certificats",
                "signing-certificates-revoke": "Signatures: revocar certificats",
                "signing-requests-read": "Signatures: veure sol·licituds",
                "signing-requests-cancel": "Signatures: cancel·lar sol·licituds",
                "signing-requests-export": "Signatures: exportar sol·licituds",
                "sign-initialize-voting": "Signar: inicialitzar la votació",
                "sign-open-voting": "Signar: obrir la votació",
                "sign-close-voting": "Signar: tancar la votació",
                "sign-generate-election-returns": "Signar: generar actes electorals",
                "sign-generate-reports": "Signar: generar altres informes electorals",
                "sign-transmit-results": "Signar: transmetre resultats",
                "sign-approve-voter": "Signar: aprovar manualment un votant",
                "sign-approve-configuration": "Signar: aprovar una versió de configuració",
                "sign-key-ceremony": "Signar: confirmar un fragment de clau",
                "sign-tally-key": "Signar: aportar un fragment de clau",
                "application-export": "Exportació d'Aplicacions",
                "application-import": "Importació d'Aplicacions",
                "tenant-create": "Crear Inquilí",
                "tenant-read": "Llegir Inquilí",
                "tenant-write": "Editar Inquilí",
                "tenant-delete": "Esborrar llogater",
                "election-event-create": "Crear Esdeveniment Electoral",
                "election-event-read": "Llegir Esdeveniment Electoral",
                "election-event-write": "Editar Esdeveniment Electoral",
                "keycloak-realm-attributes-read": "Read Keycloak realm attributes",
                "keycloak-realm-attributes-write": "Edit Keycloak realm attributes",
                "election-event-delete": "Esborrar Esdeveniment Electoral",
                "voter-create": "Crear Votant",
                "voter-read": "Llegir Votant",
                "voter-write": "Editar Votant",
                "voter-secret-attribute-read": "Mostrar Camps Secrets del Votant",
                "voter-secret-attribute-write": "Editar Camps Secrets del Votant",
                "user-create": "Crear Usuari",
                "user-read": "Llegir Usuari",
                "user-write": "Editar Usuari",
                "user-permission-create": "Crear Permís d'Usuari",
                "user-permission-read": "Llegir Permís d'Usuari",
                "user-permission-write": "Editar Permís d'Usuari",
                "role-create": "Crear Rol",
                "role-read": "Llegir Rol",
                "role-write": "Editar Rol",
                "role-assign": "Assignar Rol",
                "communication-template-create": "Crear Plantilla de Comunicació",
                "communication-template-read": "Llegir Plantilla de Comunicació",
                "communication-template-write": "Editar Plantilla de Comunicació",
                "notification-read": "Llegir Notificació",
                "notification-write": "Editar Notificació",
                "notification-send": "Enviar Notificació",
                "area-read": "Llegir Àrea",
                "area-write": "Editar Àrea",
                "election-state-write": "Editar Estat d'Elecció",
                "election-type-create": "Crear Tipus d'Elecció",
                "election-type-read": "Llegir Tipus d'Elecció",
                "election-type-write": "Editar Tipus d'Elecció",
                "voting-channel-read": "Llegir Canal de Votació",
                "voting-channel-write": "Editar Canal de Votació",
                "trustee-create": "Crear Fideïcomissari",
                "trustee-read": "Llegir Fideïcomissari",
                "trustee-write": "Editar Fideïcomissari",
                "tally-read": "Llegir Recompte",
                "tally-start": "Iniciar Recompte",
                "tally-write": "Editar Recompte",
                "tally-results-read": "Llegir Resultats de Recompte",
                "publish-read": "Llegir Publicació",
                "publish-write": "Editar Publicació",
                "publish-results-read": "Llegir Publicació de Resultats",
                "publish-results-write": "Editar Publicació de Resultats",
                "logs-read": "Llegir Registres",
                "tasks-read": "Llegir l'Execució de Tasques",
                "keys-read": "Llegir Claus",
                "document-upload": "Pujar Documents",
                "document-download": "Descarregar Documents",
                "document-password-read": "Llegir contrasenyes de documents",
                "tally-sheet-create": "Crear Acta de Recompte",
                "tally-sheet-import-create": "Crear importació d'actes de recompte",
                "tally-sheet-import-review": "Revisar importació d'actes de recompte",
                "tally-sheet-import-view": "Veure importació d'actes de recompte",
                "tally-recount-execute": "Executar recompte de resultats",
                "trustee-ceremony": "Cerimònia de Fideïcomissari",
                "tally-sheet-review": "Revisar full de recompte",
                "tally-sheet-view": "Veure Acta de Recompte",
                "admin-ceremony": "Administrar Cerimònia de Claus",
                "tally-sheet-delete": "Esborrar Acta de Recompte",
                "cast-vote-read": "Llegir Vots Emissos",
                "document-read": "Llegir Documents",
                "document-write": "Editar Documents",
                "support-material-read": "Llegir Materials de Suport",
                "support-material-write": "Editar Materials de Suport",
                "miru-create": "Miru Create",
                "miru-download": "Miru Download",
                "miru-send": "Miru Send",
                "miru-sign": "Miru Sign",
                "contest-write": "Editar Concurs",
                "contest-read": "Llegir Concurs",
                "candidate-write": "Editar Candidats",
                "candidate-read": "Llegir Candidats",
                "permission-label-write": "Edita l'etiqueta de permís",
                "scheduled-event-write": "Editar Esdeveniments Programats",
                "contest-create": "Crear Concurs",
                "contest-delete": "Esborrar Concurs",
                "candidate-create": "Crear Candidat",
                "candidate-delete": "Esborrar Candidat",
                "election-create": "Crear Election",
                "election-read": "Llegir Elecció",
                "election-write": "Editar Elecció",
                "election-delete": "Esborrar Elecció",
                "election-event-archive": "Arxivar Event Electoral",
                "election-data-tab": "Veure Dades de l'Elecció",
                "election-event-areas-tab": "Veure Àrees de l'Esdeveniment Electoral",
                "election-event-data-tab": "Veure Dades de l'Esdeveniment Electoral",
                "election-event-keys-tab": "Veure Claus de l'Esdeveniment Electoral",
                "election-event-logs-tab": "Veure Registres de l'Esdeveniment Electoral",
                "election-event-publish-tab": "Veure Publicació de l'Esdeveniment Electoral",
                "election-event-reports-tab": "Veure Informes de l'Esdeveniment Electoral",
                "election-event-scheduled-tab": "Veure Programació de l'Esdeveniment Electoral",
                "election-event-tally-tab": "Veure Recompte de l'Esdeveniment Electoral",
                "election-event-tasks-tab": "Veure Tasques de l'Esdeveniment Electoral",
                "election-event-voters-tab": "Veure Votants de l'Esdeveniment Electoral",
                "election-publish-tab": "Veure Publicació de l'Elecció",
                "election-voters-tab": "Veure Votants de l'Elecció",
                "report-write": "Editar Informes",
                "report-read": "Llegir Informes",
                "users-menu": "Veure usuaris i rols",
                "settings-menu": "Veure configuració",
                "templates-menu": "Veure plantilles",
                "settings-election-types-tab": "Veure configuració de tipus d'elecció",
                "settings-voting-channels-tab": "Veure configuració de canals de votació",
                "settings-templates-tab": "Veure configuració de plantilles",
                "settings-languages-tab": "Veure configuració d'idiomes",
                "settings-localization-tab": "Veure configuració de localització",
                "settings-look-feel-tab": "Veure configuració d'aparença",
                "settings-trustees-tab": "Veure configuració de fideïcomissaris",
                "settings-countries-tab": "Veure configuració de països",
                "voter-import": "Importar Votant",
                "ee-voters-columns": "Veure Columnes de Votants de l'Esdeveniment Electoral",
                "voter-manually-verify": "Verificar Votant Manualment",
                "ee-voters-logs": "Veure Registres de Votants de l'Esdeveniment Electoral",
                "voter-export": "Exportar Votant",
                "ee-voters-filters": "Veure Filtres de Votants de l'Esdeveniment Electoral",
                "voter-delete": "Eliminar Votant",
                "voter-change-password": "Canviar la Contrasenya del Votant",
                "election-event-localization-selector":
                    "Selector de Localització de l'Esdeveniment Electoral",
                "localization-create": "Crear Localització",
                "localization-read": "Llegir Localització",
                "localization-write": "Editar Localització",
                "localization-delete": "Eliminar Localització",
                "area-create": "Crear Àrea",
                "area-delete": "Eliminar Àrea",
                "area-export": "Exportar Àrea",
                "area-import": "Importar Àrea",
                "area-upsert": "Inserir o Actualitzar Àrea",
                "election-event-areas-columns": "Columnes de les Àrees de l'Esdeveniment Electoral",
                "election-event-areas-filters": "Filtres de les Àrees de l'Esdeveniment Electoral",
                "election-event-tasks-back-button":
                    "Tornar a les Tasques de l'Esdeveniment Electoral",
                "election-event-tasks-columns":
                    "Columnes de les Tasques de l'Esdeveniment Electoral",
                "election-event-tasks-filters":
                    "Filtres de les Tasques de l'Esdeveniment Electoral",
                "task-export": "Exportar Tasques",
                "application-read": "Llegir Aplicació",
                "application-write": "Editar Aplicació",
                "logs-export": "Exportar Registres",
                "election-event-logs-columns":
                    "Columnes dels Registres de l'Esdeveniment Electoral",
                "election-events-logs-filters":
                    "Filtres dels Registres de l'Esdeveniment Electoral",
                "election-event-scheduled-event-columns":
                    "Columnes dels Esdeveniments Programats de l'Esdeveniment Electoral",
                "scheduled-event-create": "Crear Esdeveniment Programat",
                "scheduled-event-delete": "Eliminar Esdeveniment Programat",
                "election-event-reports-columns":
                    "Columnes dels Informes de l'Esdeveniment Electoral",
                "report-create": "Crear Informe",
                "report-delete": "Eliminar Informe",
                "report-generate": "Generar Informe",
                "report-preview": "Vista Prèvia de l'Informe",
                "monitor-authenticated-voters": "Monitoreig de Votants Autenticats",
                "monitor-all-approve-disapprove-voters":
                    "Llegir Monitoreig de Votants Aprovat i Rebutjat",
                "monitor-automatic-approve-disapprove-voters":
                    "Llegir Monitoreig d'Aprovacions i Rebutjos Automàtics",
                "monitor-manually-approve-disapprove-voters":
                    "Llegir Monitoreig d'Aprovacions i Rebutjos Manuals",
                "monitor-enrolled-overseas-voters":
                    "Llegir Monitoreig de Votants Registrats a l'Estranger",
                "monitor-posts-already-closed-voting":
                    "Llegir Monitoreig de Publicacions amb Votació Tancada",
                "monitor-posts-already-generated-election-results":
                    "Llegir Monitoreig de Publicacions amb Resultats Generats",
                "monitor-posts-already-opened-voting":
                    "Llegir Monitoreig de Publicacions amb Votació Oberta",
                "monitor-posts-already-started-counting-votes":
                    "Llegir Monitoreig de Publicacions que Han Començat a Comptar Vots",
                "monitor-posts-initialized-the-system":
                    "Llegir Monitoreig de Publicacions que Han Inicialitzat el Sistema",
                "monitor-posts-started-voting":
                    "Llegir Monitoreig de Publicacions que Han Començat a Votar",
                "monitor-posts-transmitted-results":
                    "Llegir Monitoreig de Publicacions que Han Transmès Resultats",
                "monitor-voters-voted-test-election":
                    "Llegir Monitoreig de Votants a l'Elecció de Prova",
                "monitor-voters-who-voted": "Llegir Monitoreig de Votants que Han Votat",
                "election-event-publish-preview":
                    "Vista Prèvia de la Publicació de l'Esdeveniment Electoral",
                "election-event-publish-back-button":
                    "Tornar a la Publicació de l'Esdeveniment Electoral",
                "election-event-publish-columns":
                    "Columnes de la Publicació de l'Esdeveniment Electoral",
                "election-event-publish-filters":
                    "Filtres de la Publicació de l'Esdeveniment Electoral",
                "publish-create": "Crear Publicació",
                "publish-regenerate": "Regenerar Publicació",
                "publish-export": "Exportar Publicació",
                "publish-start-voting": "Iniciar Votació",
                "publish-pause-voting": "Pausar Votació",
                "publish-stop-voting": "Aturar Votació",
                "publish-changes": "Publicar Canvis",
                "election-event-publish-view": "Veure la Publicació de l'Esdeveniment Electoral",
                "election-event-keys-columns": "Columnes de les Claus de l'Esdeveniment Electoral",
                "create-ceremony": "Crear Cerimònia",
                "export-ceremony": "Exportar Cerimònia",
                "election-event-tally-columns": "Columnes del Recompte de l'Esdeveniment Electoral",
                "election-event-tally-back-button":
                    "Tornar al Recompte de l'Esdeveniment Electoral",
                "transmition-ceremony": "Cerimònia de Transmissió",
                "admin-ip-address-view": "Veure Adreça IP",
                "election-approvals-tab": "Veure Aprovacions d'Elecció",
                "election-event-approvals-tab": "Veure Aprovacions de l'Esdeveniment Electoral",
                "election-ip-address-view": "Veure Adreça IP d'Elecció",
                "election-dashboard-tab": "Veure Panell de Monitoreig d'Elecció",
                "trustees-export": "Exportar Fideïcomissaris",
                "user-import": "Importar Usuaris",
                "voter-voted-edit": "Edita els votants que han votat",
                "voter-email-tlf-edit": "Edita els camps de correu electrònic/telèfon dels votants",
                "cloudflare-write": "Edita les regles de bloqueig per país a Cloudflare",
                "transmission-report-generate": "Generar Informe de Transmissió",
                "google-meet-link": "Generar Enllaç de Google Meet",
                "service-account": "Compte de servei",
                "datafix-account": "Compte de correcció de dades",
                "gold": "Or",
                "silver": "Plata",
                "election-event-ivr-tab": "Mostra l’IVR de l’esdeveniment electoral",
                "election-event-cas-tab": "Mostra el CAS de l’esdeveniment electoral",
                "ca-read": "Consulta les autoritats de certificació",
                "ca-write": "Edita les autoritats de certificació",
                "generate-preview": "Genera la previsualització",
                "preview-read": "Consulta la previsualització",
                "tally-resolution-submit": "Envia la resolució del recompte",
                "phone-blacklist-read": "Consulta la llista negra de telèfons",
                "phone-blacklist-create": "Crea entrades a la llista negra de telèfons",
                "phone-blacklist-update": "Edita entrades de la llista negra de telèfons",
                "phone-blacklist-delete": "Suprimeix entrades de la llista negra de telèfons",
                "election-event-voter-list-reconciliation":
                    "Concilia la llista de votants de l’esdeveniment electoral",
            },
        },
        generalSettingsScreen: {
            body: "Activeu els idiomes al sistema. Només els idiomes activats aquí estaran disponibles per a esdeveniments electorals.",
        },
        eventsScreen: {
            title: "Esdeveniments Programats",
            subtitle:
                "Gestiona la configuració de l'execució automàtica d'esdeveniments com l'inici o el final del període de votació.",
            messages: {
                createSuccess: "Esdeveniment Programat creat amb èxit",
                createError: "Error en crear l'Esdeveniment Programat",
                editSuccess: "Esdeveniment Programat editat amb èxit",
                editError: "Error en editar l'Esdeveniment Programat",
                onlineWithEarlyVoting:
                    "Una programació d'inici no pot obrir alhora el vot en línia i el vot anticipat: el vot anticipat ha de començar abans que el vot en línia.",
            },
            eventType: {
                label: "Tipus",
                ALLOW_INIT_REPORT: "Allow Initialization Report",
                START_VOTING_PERIOD: "Inici del Període de Votació",
                END_VOTING_PERIOD: "Final del Període de Votació",
                ALLOW_VOTING_PERIOD_END: "Allow Voting Period End",
                START_ENROLLMENT_PERIOD: "Inici del període de matrícula",
                END_ENROLLMENT_PERIOD: "Finalització del període de matrícula",
                START_LOCKDOWN_PERIOD: "Inici del Período de Bloc de Dades Censals",
                END_LOCKDOWN_PERIOD: "Final del Período de Bloc de Dades Censals",
                ALLOW_TALLY: "Permetre el recompte",
                START_READINESS_TEST: "Iniciar la prova de preparació electoral",
                END_READINESS_TEST: "Finalitzar la prova de preparació electoral",
                START_FINAL_TESTING: "Iniciar les proves finals i el bloqueig",
                END_FINAL_TESTING: "Finalitzar les proves finals i el bloqueig",
                START_TEST_VOTING: "Iniciar la votació de prova",
                END_TEST_VOTING: "Finalitzar la votació de prova",
            },
            warning: {
                votingWindowDays:
                    "El període de votació de {{election}} abasta {{days}} dies locals (del {{start_local}} al {{end_local}}, {{time_zone}}); la regla demana {{expected}}.",
                finalTestingLeadTime:
                    "Les proves finals de {{election}} comencen el {{final_testing_local}}, menys de {{minimum_days}} dies abans que s'obri la votació el {{voting_start_local}} ({{time_zone}}).",
                closeBeforeOpen:
                    "La votació de {{election}} es tanca abans o en el moment d'obrir-se ({{start_local}} a {{end_local}}, {{time_zone}}).",
                shortLastDay:
                    "L'últim dia de votació de {{election}} té {{hours}} hores, menys de {{minimum_hours}}: la votació es tanca el {{end_local}} ({{time_zone}}).",
            },
            election: {
                label: "Elecció",
            },
            empty: {
                header: "Encara no hi ha Esdeveniments Programats.",
                body: "Voleu crear-ne un?",
                button: "Crear Esdeveniment Programat",
            },
            create: {
                title: "Crear Esdeveniment Programat",
                subtitle: "Crea una nova configuració d'Esdeveniment Programat.",
            },
            edit: {
                title: "Editar Esdeveniment Programat",
                subtitle: "Edita la configuració de l'Esdeveniment Programat.",
                delete: "Esteu segur que voleu suprimir aquest Esdeveniment Programat?",
            },
            fields: {
                electionId: "Elecció",
                eventProcessor: "Tipus",
                stoppedAt: "Aturat A",
                scheduledDate: "Programat A",
            },
        },
        reportsScreen: {
            title: "Informes",
            subtitle: "Generar informes per als esdeveniments electorals",
            messages: {
                createSuccess: "Informe creat amb èxit",
                createError: "Error en crear l'informe",
                submitError: "Error en enviar l'Informe",
                updateSuccess: "Informe actualitzat amb èxit",
                passwordMismatch:
                    "La contrasenya i la confirmació no coincideixen. Assegura't que tots dos camps continguin la mateixa contrasenya.",
                incorectPassword: "Contrasenya incorrecta",
                decryptFileTitle: "Desxifrar arxiu",
                decryptInstructions:
                    "1. '-in': La ruta al fitxer xifrat. \n2. '-out': La ruta on es desarà el fitxer desxifrat. \n3. '-pass': La contrasenya utilitzada per xifrar el fitxer. \n",
                encryptSuccess: "S'ha configurat correctament l'encriptació de l'informe",
                encryptError: "Error en configurar l'encriptació de l'informe",
            },
            reportType: {
                BALLOT_RECEIPT: "Rebut de la Papereta",
                ELECTORAL_RESULTS: "Resultats Electorals",
                MANUAL_VERIFICATION: "Verificació Manual",
                PARTICIPATION_REPORT: "Informe de Participació",
                STATISTICAL_REPORT: "Informe Estadístic",
                OVCS_EVENTS: "Seguiment del Vot a l'Estranger - Esdeveniments OVCS",
                AUDIT_LOGS: "Registres d'Auditoria",
                ACTIVITY_LOG: "Registres d'Activitat",
                STATUS: "Estat",
                OVCS_INFORMATION: "Informació de l'OVCS",
                OVERSEAS_VOTERS: "Llista de Votants a l'Estranger",
                OV_USERS_WHO_VOTED: "Llista de Votants a l'Estranger que han Votat",
                OV_WITH_VOTING_STATUS: "Llista de Votants a l'Estranger amb Estat de Votació",
                OVCS_STATISTICS: "Seguiment del Vot a l'Estranger - Estadístiques de l'OVCS",
                PRE_ENROLLED_OV_BUT_DISAPPROVED:
                    "Llista de Votants a l'Estranger Preinscrits però Desaprovats",
                PRE_ENROLLED_OV_SUBJECT_TO_MANUAL_VALIDATION:
                    "Llista de Votants a l'Estranger Preinscrits però Sotmesos a Validació Manual",
            },
            reportEncryptionPolicy: {
                title: "Política de xifrat",
                UNENCRYPTED: "Sense xifrar",
                CONFIGURED_PASSWORD: "Contrasenya configurada",
            },
            empty: {
                header: "Encara no hi ha informes.",
                body: "Vols crear-ne un?",
                button: "Crear informe",
            },
            create: {
                title: "Crear informe",
                subtitle: "Crear una nova configuració d'informe.",
            },
            edit: {
                title: "Editar informe",
                subtitle: "Editar la configuració de l'informe.",
                delete: "Estàs segur que vols eliminar aquest informe?",
            },
            fields: {
                electionId: "Elecció",
                template: "Plantilla",
                reportType: "Tipus d'informe",
                repeatable: "Repetible",
                cronExpression: "Expressió Cron",
                emailRecipients: "Destinataris de correu electrònic",
                emailRecipientsPlaceholder: "Escriviu el correu electrònic i premeu Enter",
            },
            delete: {
                body: "Estàs segur que vols eliminar aquest informe?",
            },
            actions: {
                generate: "Generar",
                delete: "Eliminar",
                edit: "Editar",
                preview: "Previsualitzar",
            },
        },
        googleMeet: {
            title: "Generar Enllaç de Google Meet",
            generateButton: "Google Meet",
            meetingTitle: "Títol de la Reunió",
            description: "Descripció (Opcional)",
            startDate: "Data d'Inici",
            startTime: "Hora d'Inici",
            duration: "Durada (minuts)",
            attendeeEmails: "Emails dels Participants",
            attendeeEmailHelp: "Emails separats per comes per als participants de la reunió",
            note: "Nota: Això crearà un esdeveniment de calendari al teu Google Calendar amb un enllaç de Google Meet. Necessitaràs iniciar sessió al teu compte de Google.",
            success: "Enllaç de Google Meet Generat Correctament!",
            copy: "Copiar al porta-retalls",
            copied: "Enllaç copiat al porta-retalls!",
            instructions:
                "Comparteix aquest enllaç amb els participants per unir-se a la reunió. L'esdeveniment de calendari s'ha afegit al teu Google Calendar.",
            generating: "Generant...",
            generate: "Generar Enllaç de Meet",
        },
        common: {
            export: "L'exportació pot ser un procés llarg. Estàs segur que vols exportar?",
            resources: {
                electionEvent: "Esdeveniment Electoral",
                election: "Elecció",
                contest: "Concurs",
                candidate: "Candidat",
                noResult: {
                    askCreate: "Vols crear-ne una?",
                },
            },
            label: {
                add: "Afegir",
                actions: "Accions",
                create: "Crear",
                delete: "Esborrar",
                archive: "Arxivar",
                unarchive: "Desarxivar",
                cancel: "Cancel·lar",
                edit: "Editar",
                yes: "Sí",
                no: "No",
                save: "Guardar",
                close: "Tancar",
                back: "Enrere",
                next: "Següent",
                warning: "Avis",
                json: "Vista prèvia",
                noResult: "Sense resultats",
                import: "Importar",
                export: "Exportar",
                loadingData: "Carregant dades...",
                exportFormat: "Exporta en format {{format}} - Resultats de '{{item}}'",
                allResults: "de l'esdeveniment electoral",
                globalAreaResults: "de totes les àrees",
                title: "Títol",
                subtitle: "Subtítol",
                kind: "Tipus d'arxiu",
                filter: "Filtres Personalizats",
                approve: "Aprovar",
                continue: "Continuar",
                logout: "Sortir",
                selectTenant: "Seleccionar Llogater",
                processing: "Processant...",
                tenantName: "Nom del Llogater",
            },
            language: {
                es: "Espanyol",
                en: "Anglès",
                fr: "Francès",
                cat: "Valencià",
                tl: "Tagal",
                gl: "Galego",
                nl: "Holandés",
                eu: "Euskera",
            },
            channel: {
                online: "En línia",
                kiosk: "Quiosc",
                early_voting: "Votació anticipada",
                telephone: "Votació telefònica",
                other: "Altres",
            },
            message: {
                delete: "Estàs segur que vols esborrar aquest element?",
                continueOrLogout: "Vols continuar conectat o sortir?",
            },
        },
        createResource: {
            electionEvent: "Crear un Esdeveniment Electoral",
            election: "Crear una Elecció",
            contest: "Crear un Concurs",
            candidate: "Crear un Candidat",
        },
        importResource: {
            electionEvent: "Importar un Esdeveniment Electoral",
            election: "Importar una Elecció",
            contest: "Importar un Concurs",
            candidate: "Importar un Candidat",
            ImportHashMismatch: "Hashes don't match. Integrity check failure.",
        },
        sideMenu: {
            electionEvents: "Processos Electorals",
            search: "Cercar",
            usersAndRoles: "Usuaris i Rols",
            logs: "Bitàcora",
            settings: "Configuració",
            help: "Ajuda",
            templates: "Plantilles",
            active: "Actius",
            archived: "Arxivats",
            addResource: {
                electionEvent: "Crear un Esdeveniment Electoral",
                election: "Crear una Elecció",
                contest: "Crear un Concurs",
                candidate: "Crear un Candidat",
            },
            menuActions: {
                archive: {
                    electionEvent: "Arxivar aquest Esdeveniment Electoral",
                },
                unarchive: {
                    electionEvent: "Desarxivar aquest Esdeveniment Electoral",
                    election: "Desarxivar aquesta Elecció",
                    contest: "Desarxivar aquest Concurs",
                    candidate: "Desarxivar aquest Candidat",
                },
                remove: {
                    electionEvent: "Eliminar aquest Esdeveniment Electoral",
                    election: "Eliminar aquesta Elecció",
                    contest: "Eliminar aquest Concurs",
                    candidate: "Eliminar aquest Candidat",
                },
                messages: {
                    confirm: {
                        archive: "Estàs segur de que vols arxivar aquest element?",
                        unarchive: "Estàs segur de que vols desarxivar aquest element?",
                        delete: "Estàs segur de que vols eliminar aquest element?",
                    },
                    notification: {
                        success: {
                            archive: "L'element ha estat arxivat",
                            unarchive: "L'element ha estat desarxivat",
                            delete: "L'element ha estat eliminat",
                            reloading: "Espera. La pàgina serà recarregada en alguns moments",
                        },
                        error: {
                            archive: "Error intentant arxivar aquest element",
                            unarchive: "Error intentant desarxivar aquest element",
                            delete: "Error intentant eliminar aquest element",
                        },
                    },
                },
            },
        },
        candidateScreen: {
            common: {
                subtitle: "Configuració de candidats.",
            },
            edit: {
                externalId: "ID extern",
                general: "General",
                type: "Tipus",
                image: "Imatge",
                isDisabled: "Deshabilitat",
                isExplicitInvalid: "Vot Invàlid",
                isExplicitBlank: "Vot en Blanc",
                isCategoryList: "Llista",
                isWriteIn: "Vot per escrit",
            },
            field: {
                name: "Nom",
                alias: "Àlies",
                description: "Descripció",
            },
            options: {
                "candidate": "Candidat",
                "option": "Opció",
                "write-in": "Vot per escrit",
                "open-list": "Llista oberta",
                "closed-list": "Llista tancada",
                "semi-open-list": "Llista semioberta",
                "invalid-vote": "Vot Invàlid",
                "blank-vote": "Vot en blanc",
            },
            invalidVotePosition: {
                label: "Posició del Vot Invàlid",
                null: "Cap (Per defecte)",
                top: "Superior",
                bottom: "Inferior",
            },
            error: {},
            createCandidateSuccess: "Candidat creat",
            createCandidateError: "Error creant candidat",
        },
        contestScreen: {
            common: {
                subtitle: "Configuració de pregunta.",
            },
            edit: {
                externalId: "ID extern",
                general: "General",
                type: "Tipus",
                image: "Imatge",
                system: "Sistema de votació de paperetes",
                design: "Disseny de la papereta",
                reorder: "Reordenar candidats",
                policies: "Polítiques",
            },
            field: {
                name: "Nom",
                alias: "Àlies",
                description: "Descripció",
            },
            options: {
                "non-preferential": "Sense Preferència",
                "plurality-at-large": "Majoria Plural",
                "instant-runoff": "Segona Volta Instantània",
                "random": "Aleatòries",
                "external-procedure": "Procediment extern",
                "custom": "Personalitzat",
                "alphabetical": "Alfabètic",
            },
            tieBreakingPolicy: {
                label: "Política de desempat",
            },
            auditButtonConfig: {
                "label": "Opció de visualització del botó d'auditoria",
                "show": "Mostrar",
                "not-show": "No mostrar",
                "show-in-help": "Mostra al diàleg d'ajuda",
            },
            underVotePolicy: {
                "label": "Política de Votació Inferior",
                "allowed": "Permès",
                "warn-only-in-review": "Advertir en Revisió",
                "warn": "Advertir",
                "warn-and-alert": "Advertir i Alertar",
            },
            invalidVotePolicy: {
                "label": "Política de vot invàlid",
                "allowed": "Permesa",
                "warn": "Advertència",
                "warn-invalid-implicit-and-explicit": "Advertir Invàlids Implícits i Explícits",
                "not-allowed": "No Permesa",
                "allowed-with-exclusive-explicit": "Permesa amb Vot Invàlid Exclusiu",
            },
            candidatesIconCheckboxPolicy: {
                "label": "Forma de la icona de la casella de verificació dels candidats",
                "square-checkbox": "Caixa de verificació quadrada",
                "round-checkbox": "Caixa de verificació rodona",
            },
            checkableListPolicy: {
                "allow-selecting-candidates-and-lists": "Candidats I Llistes",
                "allow-selecting-candidates": "Només Candidats",
                "allow-selecting-lists": "Només Llistes",
                "disabled": "Deshabilitat",
            },
            collapsibleListsPolicy: {
                "label": "Llistes plegables",
                "disabled": "Desactivat",
                "enabled-expanded": "Activat (comença expandit)",
                "enabled-collapsed": "Activat (comença contret)",
            },
            blankVotePolicy: {
                "label": "Política de vot en blanc",
                "allowed": "Permès",
                "warn-only-in-review": "Advertir en Revisió",
                "warn": "Advertir",
                "not-allowed": "No permès",
            },
            overVotePolicy: {
                "label": "Política de vot excessiva",
                "allowed": "Permès",
                "allowed-with-msg": "Permès amb missatge d'avís",
                "allowed-with-msg-and-alert": "Permès amb missatge d'avís i alerta",
                "not-allowed-with-msg-and-alert": "No es permet amb missatge d'avís i alerta",
                "not-allowed-with-msg-and-disable":
                    "No es permet amb missatge d'avís i desactiva més seleccions",
            },
            duplicatedRankPolicy: {
                "label": "Vot invàlid - Política de rang duplicat",
                "allowed-warn-and-dialog": "Mostrar advertiment i diàleg (el votant pot continuar)",
                "not-allowed-warn-and-dialog":
                    "Mostrar advertiment i diàleg (el votant no pot continuar)",
            },
            preferenceGapsPolicy: {
                "label": "Vot invàlid - Política de rangs omesos",
                "allowed-warn-and-dialog": "Mostrar advertiment i diàleg (el votant pot continuar)",
                "not-allowed-warn-and-dialog":
                    "Mostrar advertiment i diàleg (el votant no pot continuar)",
            },
            paginationPolicy: {
                label: "Nom de la pàgina",
            },
            isAcclaimed: {
                label: "Resolt per aclamació",
                helperText:
                    "Els votants veuen aquesta votació però no poden seleccionar res, no es registra res i totes les candidatures es declaren guanyadores amb zero vots. Configureu-ho abans de publicar les paperetes: canviar-ho després invalida les paperetes ja emeses.",
            },
            allowWriteins: {
                label: "Permetre candidatures manuals",
            },
            maxVotes: {
                helperText:
                    "Nombre màxim de candidats que un votant pot seleccionar (votació no preferencial).",
                helperTextPreferential:
                    "Posició de rang més alta disponible per als votants (p.ex. '5' significa posicions 1–5). Ha de ser almenys igual al nombre de candidats a ordenar (votació preferencial).",
            },
            error: {},
            createContestSuccess: "Pregunta creada",
            createContestError: "Error creant pregunta",
        },
        keysGeneration: {
            configureStep: {
                create: "Crear Cerimònia de Claus",
                name: "Nom de la Cerimònia de Claus",
                allElections: "Totes les Eleccions",
                title: "Crear Cerimònia de Claus de l'Esdeveniment Electoral",
                subtitle:
                    "En aquesta cerimònia cada autoritat generarà i descarregarà la seva part de les claus privades per a l'Esdeveniment Electoral. Per continuar, trieu les autoritats que participaran en la cerimònia i el llindar, que és el nombre mínim d'autoritats necessaris per comptar.",
                trusteeList: "Autoritats",
                threshold: "Llindar",
                errorMinTrustees_one:
                    "Has seleccionat només {{selected}} autoritat, però has de seleccionar almenys {{threshold}}.",
                errorMinTrustees_other:
                    "Has seleccionat només {{selected}} autoritats, però has de seleccionar almenys {{threshold}}.",
                errorThreshold:
                    "Has seleccionat un llindar de {{selected}} però ha d'estar entre {{min}} i {{max}}.",
                errorCreatingCeremony: "Error creant Cerimònia de Claus: {{error}}",
                createCeremonySuccess: "Cerimònia de Claus creada",
                confirmdDialog: {
                    ok: "Sí, Crear Cerimònia de Claus",
                    cancel: "Cancel·lar",
                    title: "Estàs segur de que vols Crear una Cerimònia de Claus?",
                    automaticCeremonyTitle:
                        "Estàs segur que vols crear una cerimònia de claus automàtica?",
                    description:
                        "Estàs a punt de Crear una Cerimònia de Claus. Aquesta acció notificarà a les Autoritats per participar en la creació i distribució de les Claus de l'Esdeveniment Electoral.",
                    automaticCeremonyDescription:
                        "Estàs a punt de crear una cerimònia de claus automàtica. Això no notificarà als fideïcomissaris que hi participin.",
                },
                filterTrustees: "Filtrar Autoritats",
                errorPermisionLabels:
                    "No es pot crear la cerimònia de claus: falta almenys una etiqueta de permís.",
                automaticCeremonyToggle: "Cerimònia automàtica",
            },
            ceremonyStep: {
                cancel: "Cancel·lar Cerimònia de Claus",
                progressHeader: "Progrés de Cerimònia de Claus",
                description:
                    "Aquesta pantalla mostra el progrés i els registres de la Cerimònia de Claus de l'Esdeveniment Electoral. En la Cerimònia de Claus, cada autoritat generarà i descarregarà el seu fragment de la clau privada per a l'Esdeveniment Electoral.",
                executionStatus: "Estat: {{status}}",
                confirmdDialog: {
                    ok: "Sí, Cancel·lar Creació de Cerimònia de Claus",
                    cancel: "Tornar a la Cerimònia de Claus",
                    title: "Estàs segur de que vols Cancel·lar la Cerimònia de Claus?",
                    description:
                        "Estàs a punt de Cancel·lar la Cerimònia de Claus. Després de realitzar aquesta acció, per tenir una Cerimònia de Claus exitosa hauràs de Crear una nova.",
                },
                header: {
                    trusteeName: "Nom d'Autoritat",
                    fragment: "Fragment de Clau Generat",
                    downloaded: "Fragment Privat de Clau Descarregat",
                    checked: "Fragment Privat de Clau Comprovat",
                },
                logsHeader: {
                    title: "Registres",
                    date: "Data",
                    entry: "Entrada",
                },
                emptyLogs: "Sense logs encara.",
            },
            startStep: {
                title: "Cerimònia de Claus d'Autoritat",
                subtitle:
                    "Estàs a punt de participar en la Cerimònia de Claus com l'Autoritat (<strong>{{name}}</strong>). Això implica els següents passos:",
                one: "<strong>Descarregar</strong> la teva Clau Privada Encriptada.",
                two: "Crear múltiples <strong>Còpies de seguretat</strong> de la teva Clau Privada Encriptada.",
                three: "<strong>Verificar</strong> que les còpies de seguretat funcionen correctament.",
            },
            downloadStep: {
                title: "Descarregar Clau Privada Encriptada",
                subtitle:
                    "Per continuar, si us plau descarrega i guarda la teva Clau Privada Encriptada en almenys dos dispositius diferents:",
                downloadButton: "Descarregar la teva Clau Privada Encriptada",
                downloaded: "Clau Privada Encriptada descarregada correctament.",
                errorEmptyKey: "Error de descàrrega, fitxer buit",
                unexpectedError: "No s'ha pogut descarregar la clau privada. Torna-ho a provar.",
                alreadyVerified: "La teva clau privada ja s'havia descarregat i verificat.",
                unavailable:
                    "La descàrrega de la clau privada ja no està disponible perquè la cerimònia ha avançat.",
                confirmdDialog: {
                    ok: "Confirmar còpies de seguretat i Continuar",
                    cancel: "Tornar",
                    title: "Còpia de seguretat de la teva Clau Privada Encriptada",
                    description:
                        "Si us plau, realitza una còpia de seguretat de la teva Clau Privada Encriptada en almenys dues ubicacions segures diferents i després confirma-ho a continuació:",
                    firstCopy: "Primera còpia de seguretat realitzada",
                    secondCopy: "Segona còpia de seguretat realitzada",
                    confirmError:
                        "Creeu les còpies de seguretat necessàries i marqueu les caselles de confirmació per continuar",
                },
            },
            checkStep: {
                title: "Verifica les teves Còpies de Seguretat de la teva Clau Privada Encriptada",
                verifyButton: "Verifica la clau",
                subtitle:
                    "Puja la Còpia de Seguretat de la teva Clau Privada Encriptada per verificar que sigui correcta. Pots intentar-ho tantes vegades com sigui necessari, des de les teves diferents còpies de seguretat:",
                errorUploading:
                    "Còpia de Seguretat de la Clau Encriptada Privada invàlida, si us plau intenta-ho de nou",
                errorEmptyFile: "Fitxer buit o no trobat",
                verified: "Còpia de seguretat verificada correctament.",
            },
        },
        miruExport: {
            create: {
                success: "Paquet de Transmissió Creat",
                error: "Error en crear el Paquet de Transmissió",
            },
            send: {
                success: "Paquet de Transmissió Enviat",
                error: "Error en enviar el Paquet de Transmissió",
            },
        },
        tally: {
            errorUploadingSignature: "S'ha produït un error en carregar la signatura",
            downloadTransmissionPackage: "Descarregar paquet",
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
                title: "Paquet de Transmissió per a l'Àrea '{{name}}' y Elección '{{eventName}}'",
                description:
                    "Et permet exportar un Paquet de Transmissió als Servidors de Destinació o descarregar-lo.",
                actions: {
                    sign: {
                        title: "Regenerar",
                        dialog: {
                            title: "Vols signar el paquet de transmissió?",
                            description:
                                "Si us plau, confirma que vols regenerar el paquet de transmissió per a l'àrea `{{name}}`",
                            confirm: "Signar el paquet de transmissió",
                            cancel: "Tancar",
                            input: {
                                placeholder: "Introdueix la teva contrasenya",
                            },
                        },
                    },
                    send: {
                        title: "Enviar",
                        dialog: {
                            title: "Vols enviar el Paquet de Transmissió?",
                            description:
                                "Si us plau, confirma que vols enviar el Paquet de Transmissió per a l'Àrea '{{name}}' als Servidors de Destinació.",
                            confirm: "Enviar Paquet de Transmissió",
                            cancel: "Tancar",
                        },

                        disabled:
                            "Falten les signatures necessàries o el paquet de transmissió ja s’ha enviat a totes les destinacions.",
                    },
                    regenerate: {
                        title: "Regenerar",
                        dialog: {
                            title: "Vol regenerar el paquet de transmissió?",
                            description:
                                "Si us plau, confirmi que vol regenerar el paquet de transmissió per a l'àrea `{{name}}`",
                            confirm: "Regenerar paquet de transmissió",
                            cancel: "Tancar",
                        },
                    },
                    download: {
                        title: "Descarregar",
                        emlTitle: "Download EML {{date}}",
                        transmissionPackageTitle: "Descarregar el Paquet de Transmissió {{date}}",
                        transmissionReportTitle: "Descarrega l'informe de transmissió",
                        dialog: {
                            title: "Vols descarregar el Paquet de Transmissió?",
                            description:
                                "Si us plau, confirma que vols descarregar el Paquet de Transmissió per a l'Àrea '{{name}}.'",
                            confirm: "Descarregar Paquet de Transmissió",
                            cancel: "Tancar",
                        },
                    },
                },
                destinationServers: {
                    title: "Servidors de Destinació",
                    description:
                        "La taula a continuació mostra l'estat d'enviament de cadascun dels Servidors de Destinació.",
                    status: "Enviat a {{signed}} de {{total}}",
                    table: {
                        serverName: "Nom del Servidor",
                        sendStatus: "Estat d'Enviament",
                    },
                },
                signatures: {
                    title: "Signatures",
                    description:
                        "Els membres poden signar el paquet de transmissió. La taula mostra l’estat de signatura de cada membre.",
                    table: {
                        trusteeName: "Membre",
                        signed: "Ha Signat",
                    },
                    status: "{{signed}} de {{total}} Han Signat",
                },
            },
            sendToTransmissionPackageServers:
                "Enviar paquet de transmissió per a l'àrea '{{name}}'",
            uploadTransmissionPackage: "Carregar",
            uploadTransmissionPackageDesc:
                "Carrega la teva signatura per signar el paquet de Resultats Electorals. Aquesta operació és opcional.",
            exportElectionArea: "Envia paquet de transmissió per a l'àrea '{{name}}'",
            generateReport: "Genera {{name}}",
            templateTitle: "Plantilla de Resultats",
            initializationTitle: "Eleccions per a l'informe d'inicialització",
            templateSubTitle: "Opcionalment sobreescriure la plantilla de resultats.",
            keysCeremonyTitle: "Cerimònia de Claus",
            keysCeremonySubTitle: "Selecciona la Cerimònia de Claus per a aquest recompte",
            ceremonyTitle: "Eleccions per al Recompte",
            ceremonySubTitle: "Seleccioneu les eleccions per al recompte",
            tallyTitle: "Progrés del Recompte d'Eleccions",
            logsTitle: "Registres",
            resultsTitle: "Resultats & Participació",
            generalInfoTitle: "Informació General",
            trusteeTallyTitle: "Trustee",
            trusteeTallySubTitle: "Estat d'importació del fragment de clau",
            eligibility: {
                selectElection: "Selecciona almenys una elecció.",
                publishElection: "Publica cada elecció seleccionada abans de crear-ne el recompte.",
                tallyDisallowed: "El recompte està deshabilitat per a una elecció seleccionada.",
                endVoting:
                    "Finalitza la votació de cada elecció seleccionada i atura els canals actius abans de crear el recompte.",
            },
            createTallySuccess: "Recompte creat",
            createTallyError: "Error creant recompte",
            startTallySuccess: "Recompte iniciat",
            startTallyError: "Error iniciant recompte",
            startTallyCeremonySuccess: "Iniciada la cerimònia del recompte",
            startTallyCeremonyError: "No s'ha pogut iniciar la cerimònia del recompte",
            cancelTallyCeremonySuccess: "Cancel·lada la cerimònia del recompte",
            cancelTallyCeremonyError: "No s'ha pogut cancel·lar la cerimònia del recompte",
            recountTallyCeremony: "Repetir el recompte",
            recountTallyCeremonyMessage:
                "Això generarà un nou esdeveniment de resultats per a la sessió de recompte completada.",
            recountTallyCeremonyStarting: "S'està iniciant el recompte...",
            recountTallyCeremonySuccess: "Recompte iniciat",
            recountTallyCeremonyError: "No s'ha pogut iniciar el recompte",
            recountTallyCeremonyOk: "Repetir el recompte",
            trusteeTitle: "Proces del trustee",
            trusteeSubTitle: "Si us plau, importeu el vostre fragment de clau",
            invited: "Has estat convidat a participar en una cerimònia de recompte. Si us plau, ",
            click: "feu clic en l'acció de recompte",
            participate: "per participar.",
            breadcrumbSteps: {
                start: "Inici",
                finish: "Final",
                tally: "Recompte",
                results: "Resultats",
                ceremony: "Cerimònia",
            },
            common: {
                title: "Recompte",
                subTitle: "Configuració del Recompte.",
                cancel: "Enrere",
                next: "Següent",
                date: "Data de Recompte",
                global: "Global",
                noTrustees: "Encara no hi ha trustees",
                imported: " trustees han importat el seu fragment de clau",
                needed: " trustees necessaris per al recompte",
                start: "Iniciar Recompte",
                ceremony: "Iniciar Cerimònia de Recompte",
                initialization: "Inicia l'informe d'inicialització",
                results: "Resultats",
                dialog: {
                    ok: "D'acord",
                    okTally: "Iniciar recompte",
                    okCancel: "Cancel·lar recompte",
                    cancel: "Tancar",
                    title: "Estàs segur de que vols iniciar una cerimònia?",
                    tallyTitle: "Estàs segur de que vols iniciar el recompte?",
                    cancelTitle: "Estàs segur de que vols cancel·lar el recompte?",
                    message:
                        "Estàs a punt d'iniciar una cerimònia de recompte. Aquesta acció notificarà als trustees per importar els seus fragments de clau.",
                    cancelMessage:
                        "Estàs a punt de cancel·lar la cerimònia de recompte. Aquesta acció no es pot desfer.",
                    ceremony:
                        "Tots els trustees requerits han verificat els seus fragments de clau. Tot està a punt per començar a rebre resultats. Voleu iniciar el Recompte?",
                    startAutomatedTallyMessage:
                        "Seleccioneu 'Start Tally' per executar el procés de recompte i mostrar els resultats, o 'Close' per cancel·lar.",
                },
            },
            table: {
                elections: "Eleccions",
                selected: "Seleccionades",
                status: "Estat",
                progress: "Progrés",
                method: "Mètode de Recompte",
                elegible: "Votants Elegibles",
                number: "Número de Vots",
                total: "Total",
                turnout: "%",
                candidates: "Resultats de Candidats",
                options: "Opcions",
                global: "Resum de participació",
                elegible_census: "Cens de votants elegibles",
                cast_votes: "Número de Vots",
                cast_votes_percent: "Percentatges de Vots",
                total_votes: "Total de votants",
                total_votes_percent: "Participació",
                total_votes_counted: "Total de Vots Comptats",
                total_auditable_votes: "Total de Vots Auditables",
                total_valid_votes: "Total de vots vàlids",
                total_valid_votes_percent: "Percentatge de vots vàlids",
                total_invalid_votes: "Total de vots invàlids",
                total_invalid_votes_percent: "Percentatge de vots invàlids",
                explicit_invalid_votes: "Vots explícitament invàlids",
                explicit_invalid_votes_percent: "Percentatge de vots explícitament invàlids",
                implicit_invalid_votes: "Vots implícitament invàlids",
                implicit_invalid_votes_percent: "Percentatge de vots implícitament invàlids",
                blank_votes: "Vots en blanc",
                explicit_blank_votes: "Vots en blanc explícits",
                implicit_blank_votes: "Vots en blanc implícits",
                blank_votes_percent: "Percentatge de vots en blanc",
                number_of_votes: "Número de vots",
                winning_position: "Posició guanyadora",
                weight: "Pes",
                preferential: {
                    candidate: "Candidat",
                    winner: "Guanyador",
                    eliminated: "Eliminat",
                    round: "Ronda",
                },
                total_declined_to_vote: "Total de vots de renúncia",
                total_blank_ballots: "Total de Paperetes en Blanc",
                participation_by_channel: "Participació per canal",
                channel: "Canal",
                channel_online: "En línia",
                channel_kiosk: "Quiosc",
                channel_early_voting: "Votació anticipada",
                channel_telephone: "Telèfon",
                channel_paper: "Paper",
                channel_postal: "Postal",
                channel_in_person: "Presencial",
                acclamation_note:
                    "Elegit per aclamació. Aquesta votació es va resoldre sense votació, per la qual cosa no es va registrar cap vot.",
            },
            pendingResolutions: {
                round: "Ronda {{round}}",
                tieResolutionRequired: "Cal resolució d'empat",
                tieResolved: "Empat resolt",
                globalArea: "Global",
                pendingResolutionsHeader: "Resolucions pendents",
                pendingResolutionStatus: "Resolució pendent",
                resolvedStatus: "Resolta",
                resolutionTitle: "Resolució",
                selectContest: "Seleccioneu un element a l'esquerra per veure els detalls",
                selectCandidateToAdvance: "Seleccioneu el candidat a avançar",
                undoResolution: "Desfer la resolució",
                applyResolutions: "Aplicar resolucions i recalcular",
                submitSuccess: "Resolucions enviades. El recompte s'està reprenent...",
                submitError: "Error en enviar les resolucions. Torneu-ho a intentar.",
                filter: "Filtra",
                save: "Desa",
                pendingApplyStatus: "Càlcul pendent",
                filterElection: "Elecció",
                filterContest: "Concurs",
                filterArea: "Àrea",
                filterStatusLabel: "Estat",
                clearFilters: "Esborrar filtres",
                candidateWithVotes: "{{name}} ({{votes}} vots)",
                candidateWithVotesAndPercent: "{{name}} ({{votes}} vots, {{percent}}%)",
                tieInfoTitle: "Recompte pausat per empat sense resoldre (Ronda {{round}})",
                tieInfoBody:
                    "Candidats empatats ({{votes}} vots, {{percent}}%): {{candidates}}. Cal un desempat manual per continuar el recompte.",
                tallyResumedTitle: "Recompte reprès després d'aplicar la resolució",
                tallyResumedBody: "L'empat va ser resolt el {{date}} per {{user}}",
            },
            chart: {
                votesForCandidates: "Vots per Candidats",
                blankVotes: "Vots en Blanc",
                invalidVotes: "Vots Invàlids",
                totalVoters: "Total de Votants",
                nonVoters: "No Votants",
            },
            exportAllAreas:
                "Exporta els resultats de totes les àrees en format {{format}} per a '{{item}}'",
        },
        publish: {
            initialization: {
                countryInfo:
                    "Genereu l’informe per a tot el lloc de votació o per a un país. La votació continua bloquejada fins que es completi tota la inicialització requerida per país i per a l’esdeveniment.",
                countriesError:
                    "No s’han pogut carregar els països elegibles. Tanqueu i torneu-ho a intentar.",
                noCountries:
                    "Aquest lloc de votació no té països elegibles amb estils de papereta actius. Comproveu-ne les àrees i la publicació abans d’inicialitzar.",
                country: "País",
                entirePost: "Tot el lloc de votació",
            },
            preview: {
                publicationAreas: "Selecciona l'àrea per a la vista prèvia",
                action: "Vista prèvia",
                copy: "Copia l'enllaç",
                copy_success: "Copia correctament l'enllaç de previsualització",
                copy_error: "No s'ha pogut copiar l'enllaç de previsualització",
                success: "Previsualització oberta amb èxit",
            },
            header: {
                change: "Canvis a Publicar",
                viewChange: "Veure Publicació",
                history: "Històric de Canvis",
            },
            action: {
                generateInitializationReport: "Genera l'Informe d'Inicialització",
                startVotingPeriod: "Començar el període de votació",
                startKioskVoting: "Començar Votació al Quiosc",
                startOnlineVoting: "Començar Votació en Línia",
                startEarlyVoting: "Començar Votació Anticipada",
                startTelephoneVoting: "Començar Votació Telefònica",
                stopVotingPeriod: "Detenir el període de votació",
                stopOnlineVoting: "Detenir la Votació en Línia",
                stopEarlyVoting: "Detenir la Votació Anticipada",
                stopTelephoneVoting: "Detenir la Votació Telefònica",
                stopKioskVotingPeriod: "Aturar la Votació al Quiosc",
                pauseVotingPeriod: "Pausar el període de votació",
                pauseKioskVoting: "Pausar la Votació al Quiosc",
                pauseOnlineVoting: "Pausar la Votació en Línia",
                pauseEarlyVoting: "Pausar la Votació Anticipada",
                pauseTelephoneVoting: "Pausar la Votació Telefònica",
                generate: "Regenerar",
                publish: "Publicar Canvis",
                back: "Enrere",
            },
            empty: {
                header: "Encara no hi ha Publicació.",
                action: "Generar Publicació",
            },
            forbidden: {
                header: "No es pot publicar fins que s'hagi completat la cerimònia de claus.",
            },
            dialog: {
                title: "Confirmar Acció",
                info: "Has fet clic en una acció sensible, per la qual cosa necessitem que la confirmis per poder continuar.",
                initializationInfo:
                    "Esteu a punt de generar l'informe d'inicialització. Esteu segur que voleu continuar?",
                startInfo:
                    "Està a punt de començar el període de votació. Està segur que vol continuar?",
                stopInfo:
                    "Està a punt de detenir el període de votació. Està segur que vol continuar?",
                kioskStopInfo:
                    "Esteu a punt d'aturar el període de votació del quiosc. Esteu segur que voleu continuar?",
                pauseInfo:
                    "Està a punt de pausar el període de votació. Està segur que vol continuar?",
                publishInfo:
                    "You are about to generate a publication. Are you sure you want to continue?",
                ok: "Confirmar",
                ko: "Cancel·lar",
                error: "Error carregant les paperetes publicades",
                error_publish: "Error publicant la papereta",
                error_capacity: "Error en generar l'estil de papereta: {{message}}",
                error_status: "Error canviant l'estat de la publicació",
                error_preview: "S'ha produït un error en visualitzar la publicació",
                diff: "Renderitzar tots els canvis podria fer que la pàgina no respongui. Esteu segur que voleu continuar?",
                confirmation:
                    "L'acció que esteu a punt de realitzar és sensible i requereix confirmació. Introduïu la vostra contrasenya per continuar amb {{action}}.",
            },
            label: {
                current: "Actual",
                previous: "Publicació Anterior",
                diff: "Canvis a publicar",
                publication: "Publicació",
            },
            notifications: {
                generated: "Papereta generada",
                published: "Papereta publicada",
                change_status: "Votació canviada d'estat",
            },
        },
        emailEditor: {
            subject: "Assumpte de l'Email",
            tabs: {
                plaintext: "Cos de Text Pla",
                richtext: "Cos de Text Enriquit",
            },
        },
        sendCommunication: {
            send: "Enviar",
            title: "Enviar Notificació",
            subtitle: "Enviar una notificació a usuaris/votants.",
            sendButton: "Enviar Notificació",
            voters: "Audiència",
            schedule: "Calendari",
            nowInput: "Enviar ara",
            dateInput: "Data i hora de començament d'enviament",
            chooseDate: "Si us plau trieu una data",
            languages: "Idiomes",
            smsMessage: "Missatge SMS",
            errorSending: "Error enviant la notificació: {{error}}",
            successSending: "Notificació programada/enviada amb èxit",
            method: "Mètode de Plantilles",
            type: "Tipus de Comunicació",
            alias: "Àlies de la Plantilla",
            votersSelection: {
                ALL_USERS: "Tots",
                NOT_VOTED: "Els que no han votat",
                VOTED: "Els que ja han votat",
                SELECTED: "A {{total}} Votants seleccionats",
            },
            path: {
                users: "usuaris",
                voters: "votants",
            },
            methodTitle: "Mètode de Comunicació",
            communicationMethod: {
                EMAIL: "Email",
                SMS: "SMS",
            },
            communicationType: {
                CREDENTIALS: "Credencials",
                BALLOT_RECEIPT: "Comprovant de Votació",
            },
            email: {
                subject: "Assumpte",
            },
        },
        tallysheet: {
            title: "Urnes",
            subtitle: "Urnes digitalitzades per canal",
            createTallySuccess: "Acta de Recompte creada",
            createTallyError: "Error creant Acta de Recompte",
            createTallyErrorSameKindExists:
                "El full de recompte ja existeix per a aquest concurs amb el mateix canal i àrea",
            allFieldsRequired: "Tots els camps són obligatoris",
            header: {
                change: "Canvis a Publicar",
                viewChange: "Veure Publicació",
                history: "Historial de Publicació",
            },
            action: {
                start: "Començar Elecció",
                stop: "Aturar Elecció",
                pause: "Pausar",
                generate: "Regenerar",
                publish: "Publicar Canvis",
                back: "Enrere",
            },
            inputError: {
                totalValidDoesNotMatch:
                    "Els vots de candidats ({{candidateVotesSum}}) han d'estar entre {{lowerBound}} i {{upperBound}} segons les regles de votació d'aquesta contesa ({{nonBlankValidVotes}} vots vàlids no en blanc × fins a {{maxMarks}} marques per papereta)",
                censusTooSmall:
                    "El total de vots ({{totalVotes}}) no pot ser major que el cens ({{census}})",
                totalInvalidDoesNotMatch:
                    "El total de vots invàlids ({{totalInvalid}}) ha de ser igual als vots invàlids implícits ({{implicitInvalid}}) més els vots invàlids explícits ({{explicitInvalid}})",
                totalVotesDoesNotMatch:
                    "El total de vots ({{totalVotes}}) ha de ser igual al total de vots vàlids ({{totalValidVotes}}) més el total de vots invàlids ({{totalInvalid}})",
                unknownCountingAlgorithm:
                    "L'algorisme de recompte d'aquesta contesa ({{countingAlgorithm}}) no es reconeix, de manera que no es pot determinar el nombre permès de vots de candidats. Reviseu la configuració de la contesa.",
                blankBallotsInconsistent:
                    "Les Paperetes en Blanc han de tenir el mateix valor a tots els fulls de contesa d'aquesta urna",
                blankBallotsOutOfBounds:
                    "El valor de Paperetes en Blanc està fora del rang que impliquen els recomptes de vots en blanc per contesa d'aquesta urna",
            },
            label: {
                area: "Àrea",
                channel: "Canal",
                total_votes: "Vots Totals",
                total_valid_votes: "Vots Vàlids Totals",
                total_invalid: "Vots Invàlids Totals",
                explicit_invalid: "Vots Explícitament Invàlids",
                implicit_invalid: "Vots Implícitament Invàlids",
                total_blank_votes: "Vots en Blanc Totals",
                blank_ballots: "Paperetes en Blanc",
                census: "Cens",
            },
            common: {
                tallyCeremony: {
                    manage: "Gestionar la Cerimònia de Còmput",
                    view: "Veure la Cerimònia de Còmput",
                    cancel: "Cancel·lar la Cerimònia de Còmput",
                    addKey: "Afegir Clau de Còmput",
                },
                edit: "Editar",
                confirm: "Confirmar",
                back: "Enrere",
                next: "Següent",
                cancel: "Enrere",
                data: "Dades",
                title: "Acta de Recompte",
                subtitle: "Configuració de l'Acta de Recompte.",
                candidates: "Candidats",
                save: "Guardar",
                approve: "Aprovar",
                disapprove: "Desaprovar",
                show: "Mostrar",
                add: "Afegir",
                versions: "Versions",
                warningDisapprove: "Estàs segur de desaprovar aquest Full de Recompte?",
                warningApprove: "Estàs segur d'aprovar aquest Full de Recompte?",
            },
            empty: {
                header: "No hi ha Actes de Recompte.",
                action: "Generar Acta de Recompte",
                add: "Afegir",
            },
            breadcrumbSteps: {
                start: "Inici",
                edit: "Editar",
                confirm: "Confirmar",
                view: "Veure",
            },
            table: {
                area: "Àrea",
                contest: "Contesa",
                approvedVersion: "Versió aprovada",
                latestVersion: "Última versió",
                labels: "Etiquetes",
                annotations: "Anotacions",
            },
            versionsTable: {
                title: "Versions de l'urna",
                version: "Versió",
                createdBy: "Creat per",
                reviewedBy: "Revisat per",
                createdAt: "Creat el",
                reviewedAt: "Revisat el",
                sourceImport: "Importació d'origen",
                importStatus: "Estat de la importació",
                openImport: "Obrir importació",
                sourceFile: "Fitxer d'origen",
            },
            message: {
                reviewError: "Error revisant l'Acta de Recompte",
                reviewSuccess: "Acta de Recompte revisada",
            },
        },
        application: {
            import: {
                title: "Importar Aplicacions",
                subtitle: "Importar dades d'aplicacions",
                paragraph:
                    "Importa aplicacions utilitzant un fitxer de full de càlcul en format de valors separats per comes (CSV). Descarrega un fitxer CSV d'exemple aquí.",
                messages: {
                    success: "Aplicacions importades amb èxit",
                    error: "Error en importar les aplicacions",
                },
            },
            export: {
                title: "Exportar Aplicacions",
                subtitle: "Exportar dades d'aplicacions",
                button: "Exportar",
                paragraph:
                    "Exporta aplicacions utilitzant un fitxer de full de càlcul en format de valors separats per comes (CSV).",
                messages: {
                    success: "Aplicacions exportades amb èxit",
                    error: "Error en exportar les aplicacions",
                },
            },
        },
        template: {
            noPermissions: "No tens permisos per accedir a les Plantilles.",
            title: "Plantilles",
            subtitle: "Llistat de plantilles",
            chooseMethods: "Trieu Mètodes",
            default: "Utilitzeu la plantilla predeterminada",
            empty: {
                title: "No hi ha plantilles",
                subtitle: "Vols crear-ne una de nova?",
            },
            action: {
                createOne: "Crear Plantilla",
            },
            create: {
                title: "Crear una Plantilla",
                success: "Plantilla creada",
                error: "Error creant plantilla",
            },
            update: {
                success: "Plantilla actualitzada",
                error: "Error actualitzant plantilla",
            },
            edit: {
                title: "Editar una Plantilla",
            },
            form: {
                smsMessage: "Missatge SMS",
                document: "Document",
                pdfOptions: "Opcions PDF",
                reportOptions: "Opcions Report",
                name: "Nom de la Plantilla",
                alias: "Àlies de la Plantilla",
                type: "Tipus",
                communicationMethod: "Mètode",
            },
            type: {
                CREDENTIALS: "Credencials",
                INITIALIZATION_REPORT: "Informe d'Inicialització",
                ELECTORAL_RESULTS: "Resultats Electorals",
                BALLOT_IMAGES: "Imatges de Butlleta",
                BALLOT_RECEIPT: "Rebut de Vot",
                ACTIVITY_LOGS: "Registres d'Activitats",
                MANUAL_VERIFICATION: "Verificació Manual",
                PARTICIPATION_REPORT: "Informe de Participació",
            },
            method: {
                email: "Email",
                sms: "SMS",
                document: "Document",
            },
            import: {
                title: "Importar Plantilles",
                subtitle: "Importar dades de plantilles",
                paragraph:
                    "Importa plantilles utilitzant un fitxer de full de càlcul en format de valors separats per comes (CSV). Descarrega un fitxer CSV d'exemple aquí.",
            },
        },
        materials: {
            createMaterialSuccess: "Material de suport creat",
            createMaterialError: "Error creant material de suport",
            updateMaterialSuccess: "Material de suport actualitzat",
            updateMaterialError: "Error actualitzant material de suport",
            common: {
                title: "Materials de Suport",
                subtitle: "Introduir dades del material de suport.",
            },
            error: {
                title: "El títol és obligatori",
                document: "El document és obligatori",
            },
            fields: {
                isHidden: "Ocult",
                publicUrl: "Enllaç públic",
            },
            empty: {
                header: "Encara no hi ha material de suport",
                action: "Genera material de suport",
            },
        },
        widget: {
            logs: "Registres",
        },
        settings: {
            countries: {
                title: "Bloqueig de Països",
                votingDescription:
                    "Trieu a continuació els països dels quals voleu bloquejar les votacions.",
                enrollmentDescription:
                    "Trieu a continuació els països dels quals voleu bloquejar la preinscripció.",
                error: {
                    errorSaving: "Error en desar la llista de països",
                },
            },
            backupRestore: {
                title: "Còpia de seguretat / Restaurar la configuració del llogater",
                backup: {
                    label: "còpia de seguretat",
                    subtitle: "Còpia de seguretat de les configuracions del llogater",
                },
                restore: {
                    label: "Restaurar",
                    subtitle: "Restaura la configuració del llogater",
                    title: "Importa les configuracions del llogater",
                    paragraph:
                        "Importa configuracions de llogater, configuracions de Keycloak, rols i dades de permisos utilitzant una carpeta comprimida.",
                    tenantConfigOption: "Importa les configuracions del llogater",
                    keycloakConfigOption: "Importa les configuracions de Keycloak",
                    RolesConfigOption: "Importa les configuracions de rols i permisos",
                },
            },
            previewScreen: {
                label: "Prèvies",
                noContent: "No s'han trobat prèvies",
                table: {
                    title: "Vistes prèvies externes",
                    description:
                        "Un registre de les vistes prèvies d'estils de papereta generades mitjançant peticions externes",
                    requestedBy: "Sol·licitat per",
                    document: "Document",
                    url: "URL",
                },
            },
            languages: {
                default: "Llengua per defecte",
            },
        },
        approvalsScreen: {
            column: {
                status: "Estat",
                id: "ID",
                applicantId: "ID del Sol·licitant",
                verificationType: "Tipus de Verificació",
                createdAt: "Creat El",
                updatedAt: "Actualitzat El",
                verified_by: "Verificat Per",
            },
            approvalRequest: "Sol·licitud d'Aprovació",
            taskInformation: "Informació de la tasca",
            ok: "D'acord",
            title: "Votants",
            subtitle: "Cercar votants coincidents",
            approve: {
                body: "Estàs segur que vols aprovar aquest votant? Aquesta acció no es pot desfer.",
            },
            reject: {
                label: "Rebutja la sol·licitud",
                confirm:
                    "Esteu segur que voleu rebutjar aquest votant? Aquesta acció no es pot revertir.",
                message: "Escriviu aquí el motiu del rebuig",
                rejectReason: "Motiu del rebuig",
                messageRequired: "Es requereix un missatge de rebuig per a l'opció 'Altres'",
                reasons: {
                    "undefined": "-",
                    "insufficient-information": "Données Manquantes",
                    "no-matching-voter": "Votant Non Trouvé",
                    "voter-already-approved": "Déjà Approuvé",
                    "other": "Autre",
                },
            },
            notifications: {
                approveError: "Error en aprovar el votant",
                approveSuccess: "Votant aprovat",
                rejectError: "Error en rebutjar el votant",
                rejectSuccess: "Votant rebutjat",
                VoterApprovedAlready: "El votant ja està aprovat.",
            },
            export: {
                success: "L'exportació d'aplicacions s'ha completat amb èxit",
                error: "Error en exportar les aplicacions",
            },
        },
        monitoring: {
            title: "Monitoratge",
            loading: "Carregant el monitoratge",
            unavailableAlert:
                "No s'han pogut carregar els taulers de monitoratge, així que es mostra el tauler estàndard.",
            noDashboards: "Aquest esdeveniment no té taulers de monitoratge.",
            dashboardFailed: "No s'ha pogut carregar el tauler de monitoratge.",
            dashboardInvalid: "Aquest tauler no es pot mostrar: {{problem}}",
            retry: "Torna-ho a provar",
            errors: {
                busy: "El servidor està ocupat. Es tornarà a provar d'aquí a uns segons.",
                forbiddenScope: "No podeu veure aquesta regió, Post o país. Trieu-ne un altre.",
                snapshotPruned:
                    "L'actualització mostrada ja no es conserva. El tauler mostra ara l'última actualització: torneu a exportar per fer-la servir.",
                checksUnavailable:
                    "El servei de gràfics no està disponible ara. Torneu-ho a provar més tard.",
                lockedDown:
                    "L'esdeveniment electoral està bloquejat, així que això no es pot canviar.",
                notFound:
                    "Aquest tauler o giny ja no està configurat. Torneu a carregar la pàgina.",
                badRequest:
                    "No s'ha acceptat la sol·licitud. Torneu a carregar la pàgina i torneu-ho a provar.",
                conflict:
                    "Una altra persona ha desat un canvi abans. Torneu a carregar i torneu-ho a provar.",
                invalid: "Alguns valors de l'exportació no s'accepten.",
                unknown: "Alguna cosa ha fallat. Torneu-ho a provar més tard.",
            },
            header: {
                dashboard: "Tauler",
                updated: "Actualitzat {{time}} ({{timeZone}})",
                notUpdated: "Encara sense recompte",
                refresh: "cada {{seconds}} s",
                export: "Exportar",
                editDashboard: "Editar el tauler",
                preset: "Valor predefinit de taulers",
                reload: "Cerca xifres noves",
            },
            footer: {
                dataThrough: "Dades fins a {{time}} ({{timeZone}})",
            },
            selectors: {
                region: "Regió",
                post: "Lloc",
                country: "País",
                allRegions: "Totes les regions",
                allPosts: "Tots els llocs",
                allAuthorizedPosts: "Tots els llocs autoritzats",
                allCountries: "Tots els països",
                authorizedOnly: "{{all}} (autoritzats)",
            },
            widget: {
                menu: "Accions de {{widget}}",
                configure: "Configurar el giny",
                viewData: "Veure les dades",
                export: "Exportar",
                duplicate: "Duplicar",
                loading: "Carregant {{widget}}",
                missing: "El tauler anomena un giny que no existeix: {{id}}",
                updating: "S'està actualitzant {{widget}}",
                updatingNote: "S'està actualitzant: el gràfic mostrat és l'anterior.",
            },
            frame: {
                title: "Gràfic de {{widget}}",
            },
            sources: {
                voter_turnout: "Participació",
                test_voting: "Votació de prova",
                enrollment_decisions: "Decisions d'inscripció",
                voting_credentials: "Credencials de vot",
                poll_status: "Estat de la votació",
                final_testing_lockdown: "Proves finals i bloqueig",
                counting_transmission: "Recompte i transmissió",
                voting_enrollment_activity: "Activitat de vot i inscripció",
                access_security: "Accés i seguretat",
                attack_detections: "Detecció d'atacs",
                helpdesk: "Suport",
            },
            reasons: {
                TEST_ELECTION_DESIGNATION: "encara no es poden marcar les eleccions de prova",
                CREDENTIAL_ISSUED_EVENT: "encara no es registra l'emissió de credencials",
                FINAL_TESTING_LOCKDOWN_STATE:
                    "encara no es registren les proves finals i el bloqueig",
                ATTACK_DETECTION_FEED: "no hi ha cap font de detecció d'atacs connectada",
                HELPDESK_INTEGRATION: "no hi ha cap sistema de suport connectat",
            },
            notices: {
                UNREGISTERED_ATTEMPTS_AT_EVENT_SCOPE_ONLY:
                    "Els intents amb noms d'usuari no registrats no pertanyen a cap lloc, de manera que només es compten per a tot l'esdeveniment.",
                UNREGISTERED_ATTEMPTS_EXCLUDED:
                    "Aquestes xifres no inclouen els intents amb noms d'usuari no registrats; es compten per a tot l'esdeveniment.",
                CREDENTIALS_ISSUED_WHEN_PASSWORD_SET:
                    "Les credencials compten com a emeses quan s'estableix la contrasenya del votant, fins que la plataforma en registri l'emissió.",
                CONFIG_NEWER_THAN_SNAPSHOT:
                    "Dibuixat amb la configuració més recent; les xifres es compten amb ella a la propera passada.",
                CONFIG_AT_SNAPSHOT_UNAVAILABLE:
                    "Dibuixat amb la configuració actual: la configuració amb què es van comptar les xifres ja no es conserva.",
            },
            unavailable: {
                notConnected: "No connectat · {{reason}}",
                notConnectedHelp: "No es mostra res fins que ho estigui.",
                unknownReason: "font de dades no disponible",
                noSnapshot: "Encara sense recompte",
                noSnapshotHelp: "El primer recompte no ha acabat. Aquest giny s'actualitza sol.",
                scopePending: "Comptant aquesta selecció",
                scopePendingHelp:
                    "Aquesta selecció es compta en la passada següent, en aproximadament un minut.",
                settingsPending: "Comptant amb la nova configuració…",
                settingsPendingHelp:
                    "Les xifres es tornen a comptar amb la configuració desada, en un minut aproximadament.",
                renderFailed: "No s'ha pogut dibuixar el gràfic",
                renderFailedHelp: "En el seu lloc es mostren les xifres.",
                invalid: "Aquest giny no es pot mostrar",
                invalidHelp: "La seva configuració té un problema.",
                requestFailed: "No s'ha pogut carregar aquest giny",
                requestFailedHelp: "Es torna a provar quan el tauler s'actualitza.",
            },
            dataTable: {
                title: "{{widget}} · dades",
                close: "Tancar",
                empty: "Sense files",
                rowsPerPage: "Files per pàgina:",
                shownRows: "{{from}}–{{to}} de {{total}}",
                firstPage: "Primera pàgina",
                previousPage: "Pàgina anterior",
                nextPage: "Pàgina següent",
                lastPage: "Última pàgina",
            },
            columns: {
                numerator: "Numerador",
                denominator: "Denominador",
                pct: "Percentatge",
                pct_label: "Percentatge mostrat",
                group: "Grup",
                group_key: "Clau del grup",
                post: "Lloc",
                post_id: "ID del lloc",
                region: "Regió",
                country: "País",
                reason: "Motiu",
                category: "Categoria",
                state: "Estat",
                state_label: "Estat mostrat",
                bucket_start: "Inici del període",
                bucket_label: "Període",
                bucket_utc: "Inici del període (UTC)",
                measure: "Mesura",
                label: "Etiqueta",
                value: "Valor",
            },
            measures: {
                registered: "Registrats",
                pre_enrolled: "Preinscrits",
                credentials_issued: "Credencials emeses",
                test_voted: "Vots de prova",
                voted: "Han votat",
                voted_pre_enrolled: "Preinscrits que han votat",
                applications: "Sol·licituds",
                pending: "Pendents",
                approved: "Aprovades",
                disapproved: "Rebutjades",
                posts: "Llocs",
                initialized: "Inicialitzats",
                opened: "Oberts",
                paused: "En pausa",
                closed: "Tancats",
                tested: "Provats",
                locked_down: "Bloquejats",
                tallied: "Escrutats",
                transmitted: "Transmesos",
                transmission_failed: "Transmissió fallida",
                logins: "Inicis de sessió",
                login_failures: "Inicis de sessió fallits",
                login_failures_valid_user: "Fallits, usuari vàlid",
                login_failures_unregistered: "Fallits, usuari no registrat",
                password_resets: "Restabliments de contrasenya",
                password_reset_requests: "Sol·licituds de restabliment de contrasenya",
                detections: "Deteccions",
                issues: "Incidències",
                pending_issues: "Incidències pendents",
            },
            export: {
                title: "Exportar dades de monitoratge",
                format: "Format",
                csv: "CSV",
                sql: "SQL",
                from: "Des de",
                to: "Fins a",
                timeZoneHelp:
                    "Les hores són en {{timeZone}}. Els totals, els estats i els grups són els de l'actualització mostrada; només les files de les sèries d'activitat es limiten a l'interval, des de l'hora d'inici fins a l'hora de fi, sense incloure-la.",
                cancel: "Cancel·lar",
                export: "Exportar",
                invalidRange: "La fi ha de ser posterior a l'inici.",
                problems: {
                    unknownSelector:
                        "{{widget}} ja no té l'opció «{{selector}}». Torneu a carregar el tauler i exporteu de nou.",
                    unknownOption:
                        "El valor triat per a «{{selector}}» a {{widget}} ja no s'ofereix. Torneu-lo a triar i exporteu.",
                },
            },
            editor: {
                scopeSelector: {
                    region: "Regió",
                    post: "Lloc",
                    country: "País",
                },
                sources: {
                    voter_turnout: "Participació de votants",
                    test_voting: "Votació de prova",
                    enrollment_decisions: "Decisions d'inscripció",
                    voting_credentials: "Credencials de votació",
                    poll_status: "Estat de la mesa",
                    final_testing_lockdown: "Proves finals i bloqueig",
                    counting_transmission: "Recompte i transmissió",
                    voting_enrollment_activity: "Activitat de votació i inscripció",
                    access_security: "Accés i seguretat",
                    attack_detections: "Deteccions d'atacs",
                    helpdesk: "Assistència tècnica",
                },
                templates: {
                    summary: "Resum",
                    by_group: "Per grup",
                    by_post: "Per lloc",
                    timeseries: "Al llarg del temps",
                    by_measure: "Per mesura",
                },
                yaml: {
                    label: "YAML",
                    readOnly:
                        "El YAML té un error de sintaxi. Corregiu-lo a la pestanya YAML per tornar a usar els formularis.",
                },
                diagnostics: {
                    title: "Comprovacions",
                    none: "No s'ha trobat cap problema",
                    localUnavailable:
                        "Les comprovacions al navegador no estan disponibles; els problemes apareixen després de la vista prèvia o de Valida.",
                    line: "Línia {{line}}",
                    engineCode: "dbt Charts {{code}}",
                    severity: {
                        ERROR: "Error",
                        WARNING: "Advertència",
                    },
                    origin: {
                        SYNTAX: "Sintaxi YAML",
                        LOCAL: "Comprovació del navegador",
                        SERVER: "Comprovació del servidor",
                    },
                },
                footer: {
                    preview: "Vista prèvia",
                    valid: "Vàlid",
                    errors_one: "{{count}} error",
                    errors_other: "{{count}} errors",
                    noWarnings: "sense advertències de gràfics",
                    warnings_one: "{{count}} advertència de gràfic",
                    warnings_other: "{{count}} advertències de gràfics",
                    rendering: "Generant…",
                    previewFailed: "ha fallat la vista prèvia",
                    renderedIn: "generat en {{ms}} ms",
                    revision: "Revisió {{revision}}",
                    savedBy: "desat {{date}} per {{user}}",
                    unknownUser: "un administrador",
                    notSaved: "encara sense desar",
                    unsaved: "canvis sense desar",
                    cancel: "Cancel·la",
                    validate: "Valida",
                },
                preview: {
                    title: "Vista prèvia",
                    empty: "La vista prèvia apareix quan el YAML és vàlid.",
                    failed: "No s'ha pogut generar la vista prèvia: {{reason}}",
                    notConnectedTail: "No es mostra res fins que ho estigui.",
                    queryResult: "Resultat de la consulta · primeres files",
                    noRows: "La consulta no ha retornat cap fila.",
                    state: {
                        RENDERED: "Generat",
                        NOT_CONNECTED: "No connectat",
                        NO_SNAPSHOT: "Encara sense dades",
                        SCOPE_PENDING: "S'està comptant aquest àmbit",
                        RENDER_FAILED: "No s'ha pogut dibuixar el gràfic",
                        INVALID: "La configuració no és vàlida",
                    },
                },
                configureWidget: {
                    title: "Configura el widget",
                    tabs: {
                        dataQuery: "Dades i consulta",
                        selectors: "Selectors",
                        yaml: "YAML",
                        preview: "Previsualització",
                    },
                    save: "Desa el widget",
                    loading: "Carregant el widget…",
                    loadFailed: "No s'ha pogut carregar el widget: {{reason}}",
                    saved: "Widget desat com a revisió {{revision}}",
                    refused: "No s'ha desat el widget: corregiu els problemes indicats.",
                    validated: "El widget és vàlid.",
                    invalid: "El widget té problemes: consulteu les comprovacions.",
                    requestFailed: "La sol·licitud ha fallat: {{reason}}",
                    discardTitle: "Voleu descartar els canvis?",
                    discardBody: "Els canvis en aquest widget no s'han desat.",
                    discard: "Descarta",
                    keepEditing: "Continua editant",
                },
                dataQuery: {
                    title: "Títol",
                    source: "Font de dades",
                    template: "Consulta",
                    measures: "Mesures",
                    ratio: "Numerador / denominador",
                    numerator: "Numerador",
                    denominator: "Denominador",
                    groupBy: "Agrupa per",
                    sort: "Ordena",
                    sortOrder: "Ordre",
                    templateOrder: "L'ordre propi de la consulta",
                    sortBy: {
                        label: "Etiqueta",
                        value: "Valor",
                        ratio: "Proporció",
                    },
                    order: {
                        asc: "Ascendent",
                        desc: "Descendent",
                    },
                    limit: "Files",
                    noLimit: "Totes",
                    none: "Cap",
                    fromSelector: "Des del selector: {{name}}",
                    follow: "Segueix els selectors del tauler",
                    followHelp: "Un selector sense marcar no limita aquest widget.",
                    manyQueries:
                        "Aquest widget té diverses consultes; editeu-les a la pestanya YAML.",
                    notConnected: "No connectat · {{reason}}",
                    sourceHelp: {
                        DISTINCT_VOTERS:
                            "Compta votants diferents; les regles de recompte les fixa la font de dades.",
                        DISTINCT_PRE_ENROLLED_VOTERS:
                            "Compta votants preinscrits diferents; les regles de recompte les fixa la font de dades.",
                        LATEST_DECISION_PER_VOTER:
                            "Compta l'última decisió per votant; les regles de recompte les fixa la font de dades.",
                        APPROVED_VOTERS:
                            "Compta votants aprovats; les regles de recompte les fixa la font de dades.",
                        POSTS_IN_SCOPE:
                            "Compta els llocs de l'àmbit; les regles de recompte les fixa la font de dades.",
                        FIRST_EVENT_PER_VOTER:
                            "Compta el primer vot o aprovació vàlid de cada votant; les regles de recompte les fixa la font de dades.",
                        ATTEMPTS:
                            "Compta intents, no persones; les regles de recompte les fixa la font de dades.",
                        DETECTIONS:
                            "Compta deteccions; les regles de recompte les fixa la font de dades.",
                        REPORTED_ISSUES:
                            "Compta incidències notificades; les regles de recompte les fixa la font de dades.",
                        default: "Les regles de recompte les fixa la font de dades.",
                    },
                },
                selectors: {
                    help: "Els selectors apareixen a la capçalera del widget. Els seus valors alimenten la consulta; els selectors del tauler (Regió, Lloc, País) s'apliquen a tots els widgets.",
                    name: "Nom",
                    label: "Etiqueta",
                    control: "Control",
                    controls: {
                        dropdown: "Llista desplegable",
                        toggle: "Interruptor",
                    },
                    default: "Per defecte",
                    optionValue: "Valor",
                    optionLabel: "Etiqueta de l'opció",
                    addOption: "Afegeix una opció",
                    addSelector: "Afegeix un selector",
                    removeOption: "Elimina l'opció {{option}}",
                    removeSelector: "Elimina el selector {{name}}",
                    moveUp: "Puja {{name}}",
                    moveDown: "Baixa {{name}}",
                    dynamic: "Les opcions provenen de les dades ({{source}}).",
                    none: "Aquest widget no té selectors.",
                    newLabel: "Selector nou",
                    newOption: "Opció nova",
                    shownWhen: "Es mostra quan {{selector}} és {{values}}",
                },
                conflict: {
                    title: "Algú ha desat primer",
                    body: "{{user}} ha desat la revisió {{revision}} ({{date}}) mentre editàveu.",
                    bodyShort: "S'ha desat la revisió {{revision}} mentre editàveu.",
                    saved: "Revisió desada",
                    mine: "Els meus canvis",
                    reload: "Torna a carregar",
                    copy: "Copia el meu YAML",
                    copied: "El vostre YAML és al porta-retalls.",
                    copyFailed:
                        "El porta-retalls no està disponible; seleccioneu el YAML i copieu-lo a mà.",
                    keepEditing: "Continua editant",
                    removed:
                        "El document s'ha eliminat mentre editàveu. Continueu editant per tornar-lo a desar.",
                },
                dashboard: {
                    editing: "Editant el tauler",
                    title: "Títol",
                    selectors: "Selectors del tauler",
                    theme: "Tema",
                    editTheme: "Edita el tema",
                    addWidget: "Afegeix un widget",
                    cancel: "Cancel·la",
                    save: "Desa el tauler",
                    width: "Amplada",
                    widthValue: "{{n}} de 12",
                    moveUp: "Puja",
                    moveDown: "Baixa",
                    remove: "Elimina",
                    duplicate: "Duplica",
                    configure: "Configura el widget",
                    empty: "Aquest tauler encara no té widgets.",
                    saved: "Tauler desat com a revisió {{revision}}",
                    refused: "No s'ha desat el tauler: corregiu els problemes indicats.",
                    requestFailed: "La sol·licitud ha fallat: {{reason}}",
                    dragHandle: "Arrossegueu per reordenar {{title}}",
                    widgets: "Widgets",
                    duplicated: "Duplicat com a {{id}}",
                    resetToPreset: "Restableix al valor predefinit",
                    actions: "Accions de {{title}}",
                    discardBody: "Els teus canvis en aquest tauler no s'han desat.",
                    duplicateInvalid: "La còpia no s'ha desat: {{problem}}",
                    layoutMalformed:
                        "Alguns elements del disseny no són un widget amb una amplada. Corregeix-los a la pestanya YAML per reordenar els widgets.",
                },
                catalog: {
                    title: "Afegeix un widget",
                    search: "Cerca widgets, fonts de dades o requisits",
                    add: "Afegeix",
                    empty: "Cap widget coincideix.",
                    onDashboard: "En aquest tauler",
                    close: "Tanca",
                },
                theme: {
                    title: "Tema del tauler",
                    subtitle: "estil de dbt Charts aplicat a tots els widgets d'aquest tauler",
                    appliesTo_one: "s'aplica a {{count}} widget",
                    appliesTo_other: "s'aplica a {{count}} widgets",
                    apply: "Aplica el tema",
                    saved: "Tema desat com a revisió {{revision}}",
                    discardBody: "Els canvis en aquest tema no s'han desat.",
                },
                reset: {
                    title: "Restableix al valor predefinit",
                    body: "Tots els taulers, widgets i temes d'aquest esdeveniment se substitueixen pels del valor predefinit. La configuració actual es conserva a l'historial.",
                    preset: "Valor predefinit",
                    confirm: "Restableix",
                    cancel: "Cancel·la",
                    done: "L'esdeveniment ara usa {{title}}.",
                    failed: "El restabliment ha fallat: {{reason}}",
                    noPresets: "No hi ha valors predefinits disponibles.",
                    loading: "Carregant els valors predefinits…",
                },
                lockedDown:
                    "L'esdeveniment està bloquejat; la seva configuració de monitoratge no es pot canviar.",
                document: {
                    loadFailed: "No s'ha pogut carregar el document: {{reason}}",
                    refused: "No s'ha desat: corregeix els problemes indicats.",
                    validated: "El document és vàlid.",
                    invalid: "El document té problemes: consulta les comprovacions.",
                    requestFailed: "La sol·licitud ha fallat: {{reason}}",
                    savedWithWarnings_one: "{{count}} avís: consulta les comprovacions.",
                    savedWithWarnings_other: "{{count}} avisos: consulta les comprovacions.",
                },
                errors: {
                    checksUnavailable:
                        "El motor de gràfics no ha pogut comprovar el canvi, així que no s'ha desat. Torna-ho a provar d'aquí a un moment.",
                    busy: "S'estan desant altres canvis d'aquest esdeveniment. Torna-ho a provar d'aquí a un moment.",
                    lockedDown:
                        "L'esdeveniment està bloquejat; la seva configuració de monitoratge no pot canviar.",
                    forbiddenScope: "No pots veure xifres de la regió, el lloc o el país triats.",
                    badRequest:
                        "L'editor ha enviat una sol·licitud que el servidor no ha pogut llegir. Recarregueu la pàgina i torneu-ho a provar.",
                },
                duplicate: {
                    copyTitle: "{{title}} (còpia)",
                    done: "S'ha afegit {{id}}, una còpia del widget, al tauler.",
                    failed: "No s'ha pogut duplicar el widget: {{reason}}",
                    notPlaced:
                        "La còpia {{id}} s'ha desat, però el tauler no l'ha inclosa ({{reason}}). Afegiu-la amb Edita el tauler.",
                },
            },
        },
        monitoringDashboardScreen: {
            voters: {
                title: "Votants",
                enrolledOverseasVoters: "Votants Inscrits a l'Estranger",
                approvalStatus: "Estat d'Aprovació: Votants Aprovats/Desaprovarats",
                manuallyApproval: "Votants Aprovats/Desaprovarats Manualment",
                automaticallyApproval: "Votants Aprovats/Desaprovarats Automàticament",
                authenticatedVoters: "Votants Autenticats",
                invalidUserErrors: "Errors d'Usuari Invàlid:",
                invalidPasswordErrors: "Errors de Contrasenya Invàlida:",
            },
            polls: {
                title: "Enquestes",
                initializedSystems: "Publicacions amb Sistemes Inicialitzats",
                votingOpened: "Publicacions amb Votació Oberta",
                votingClosed: "Publicacions amb Votació Tancada",
                votingStarted: "Publicacions amb Votació Iniciada",
                voterTurnout: "Participació dels Votants",
            },
            tally: {
                title: "Compte",
                activeVotesCounting: "Publicacions amb Comptatge de Vots Actiu",
                generatedERs: "Publicacions amb ERs Generats",
                transmittedResults: "Publicacions amb Resultats Transmesos",
            },
            testing: {
                title: "Proves",
                testElectionVoterCount: "Comptatge de Votants a l'Elecció de Prova",
            },
        },
        certificateAuthorities: {
            title: "Certificats",
            subtitle:
                "Autoritats de certificació (CA) de confiança per a aquest esdeveniment electoral. Les CA importades s'utilitzen per validar els certificats dels votants.",
            importButton: "Importar certificats",
            type: {
                root: "Arrel",
                intermediate: "Intermedi",
            },
            expiry: {
                expired: "Caducat",
                expiringSoon: "Pròxim a caducar",
                valid: "Vàlid",
            },
            columns: {
                commonName: "Nom comú",
                type: "Tipus",
                issuerCn: "CN de l'emissor",
                notBefore: "Vàlid des de",
                notAfter: "Caduca",
                fingerprint: "Empremta SHA256",
            },
            importDialog: {
                title: "Importar autoritats de certificació",
                subtitle: "Importar un o més certificats CA des d'un fitxer PEM",
                description:
                    "Seleccioneu un fitxer PEM que contingui un o més certificats. S'admeten paquets — cada certificat s'importa individualment.",
                selectFile: "Seleccionar fitxer PEM",
                fileLoaded: "Fitxer carregat ({{bytes}} bytes)",
                importButton: "Importar",
            },
            notify: {
                importSuccess: "S'han importat {{inserted}} certificat(s).",
                importSkipped: "{{count}} omès(os) (ja presents).",
                importErrors: "Problemes en la importació: {{errors}}",
                importError: "Error en la importació: {{error}}",
                deleteSuccess: "Certificat eliminat.",
                deleteError: "Error en eliminar el certificat.",
                exportSuccess: "Certificat(s) exportat(s) correctament.",
                exportError: "Error en exportar els certificats.",
            },
            exportDialog: {
                title: "Exportar autoritats de certificació",
                description: "Esteu a punt d'exportar {{amount}} certificat(s).",
                all: "tots",
            },
            deleteDialog: {
                description: "Esteu segur que voleu suprimir {{count}} certificat(s)?",
            },
            emptyHeader:
                "No s'han importat autoritats de certificació per a aquest esdeveniment electoral.",
            fileReadError: "Error en llegir el fitxer.",
            viewDialog: {
                title: "Detalls de l'autoritat de certificació",
                subject: "Assumpte",
                issuer: "Emissor",
                serialNumber: "Número de sèrie",
                pemContent: "Contingut PEM",
            },
            confirmDelete: "Eliminar autoritat de certificació",
            confirmDeleteDescription:
                'Esteu segurs que voleu eliminar el certificat "{{name}}" (empremta: {{fingerprint}})?',
        },
        signing: {
            terms: {
                post: "Lloc",
                posts: "Llocs",
            },
            tab: {
                title: "Signatures",
                intro: "Les accions protegides només s'executen quan prou persones autoritzades les signen amb els seus certificats digitals. Cada signatura es comprova amb els emissors de confiança i queda anotada al registre.",
                protectedActions: "Accions protegides",
                certificates: "Certificats",
                requests: "Sol·licituds",
            },
            loadError:
                "No s'ha pogut carregar la configuració de signatures. Recarregueu la pàgina per tornar-ho a provar.",
            errors: {
                automatedCeremonies:
                    "Aquest esdeveniment utilitza cerimònies de claus automàtiques. Els custodis no duen a terme aquests passos, per tant no es poden exigir les seves signatures. Per exigir les signatures dels custodis, utilitza cerimònies de claus manuals.",
                forbidden: "No teniu permís per fer aquest canvi.",
                invalid:
                    "El servidor ha rebutjat aquests valors. Reviseu-los i torneu-ho a provar.",
                conflict:
                    "Una altra persona ho ha canviat mentrestant. Recarregueu la pàgina i torneu-ho a provar.",
                lockedDown:
                    "L'esdeveniment electoral està bloquejat: les regles de signatura només canvien mitjançant una nova versió de configuració.",
                notFound: "Ja no existeix. Recarregueu la pàgina.",
            },
            readOnly: {
                chip: "Només lectura",
                rules: "Només lectura. Per canviar les regles de signatura cal el permís «Signatures: editar accions protegides».",
                whoCanSign:
                    "Rols amb el permís «Signar: {{action}}» a Usuaris i Rols. Per canviar-los cal permís per editar rols.",
            },
            groups: {
                "voting": "Votació",
                "results-and-reports": "Resultats i informes",
                "enrollment": "Inscripció",
                "configuration-and-keys": "Configuració i claus",
            },
            actions: {
                "initialize-voting": {
                    label: "Inicialitzar la votació",
                    short: "Inicialització",
                    permissionName: "inicialitzar la votació",
                    object: "inicialització de la votació",
                    appliesTo: "Cada $t(signing.terms.post)",
                    description:
                        "S'inicia a Publicar. Inicialitza el $t(signing.terms.post) i genera el seu Informe d'Inicialització.",
                },
                "open-voting": {
                    label: "Obrir la votació",
                    short: "Obertura",
                    permissionName: "obrir la votació",
                    object: "obertura de la votació",
                    appliesTo: "Cada $t(signing.terms.post)",
                    description:
                        "S'inicia a Publicar amb Començar el període de votació. Obre la votació al $t(signing.terms.post).",
                },
                "close-voting": {
                    label: "Tancar la votació",
                    short: "Tancament",
                    permissionName: "tancar la votació",
                    object: "tancament de la votació",
                    appliesTo: "Cada $t(signing.terms.post)",
                    description:
                        "S'inicia a Publicar amb Detenir el període de votació. Tanca la votació al $t(signing.terms.post); les signatures de tancament es conserven a la seva acta.",
                },
                "generate-election-returns": {
                    label: "Generar actes electorals",
                    short: "Actes electorals",
                    permissionName: "generar actes electorals",
                    object: "actes electorals",
                    appliesTo: "Cada $t(signing.terms.post) i país",
                    description:
                        "L'inicia el recompte, una sol·licitud per $t(signing.terms.post) i país. Allibera les actes electorals signades per imprimir-les i transmetre-les.",
                },
                "generate-reports": {
                    label: "Generar altres informes electorals",
                    short: "Informe",
                    permissionName: "generar altres informes electorals",
                    object: "informe",
                    appliesTo: "Cada $t(signing.terms.post)",
                    description:
                        "L'inicia el recompte per a l'Informe d'Inicialització i Informes per a l'informe de participació. Allibera l'informe signat.",
                },
                "transmit-results": {
                    label: "Transmetre resultats",
                    short: "Transmissió",
                    permissionName: "transmetre resultats",
                    object: "paquet de resultats",
                    appliesTo: "Cada $t(signing.terms.post) i país",
                    description:
                        "S'inicia a Recompte, Transmissió. Genera el paquet de resultats signat per als seus destins; les signatures completen la seva llista de signatures.",
                },
                "approve-voter": {
                    label: "Aprovar manualment un votant",
                    short: "Aprovació de votant",
                    permissionName: "aprovar manualment un votant",
                    object: "aprovació de votant",
                    appliesTo: "El $t(signing.terms.post) del votant",
                    description:
                        "S'inicia a Aprovacions. Aprova el votant i n'emet les credencials.",
                },
                "approve-configuration": {
                    label: "Aprovar una versió de configuració",
                    short: "Versió de configuració",
                    permissionName: "aprovar una versió de configuració",
                    object: "versió de configuració",
                    appliesTo: "L'esdeveniment electoral",
                    description: "S'inicia a Publicar. Publica la versió de configuració.",
                },
                "key-ceremony": {
                    label: "Confirmar un fragment de clau (cerimònia de claus)",
                    short: "Fragment de clau",
                    permissionName: "confirmar un fragment de clau",
                    object: "fragment de clau",
                    appliesTo: "Cada autoritat",
                    description:
                        "L'inicia cada autoritat a Claus. Anota la signatura de l'autoritat a la cerimònia i al tauler d'anuncis.",
                },
                "tally-key": {
                    label: "Aportar un fragment de clau (recompte)",
                    short: "Aportació de fragment de clau",
                    permissionName: "aportar un fragment de clau",
                    object: "aportació de fragment de clau",
                    appliesTo: "Cada autoritat",
                    description:
                        "L'inicia cada autoritat a Recompte. Anota l'aportació de l'autoritat.",
                },
            },
            protectedActions: {
                intro: "Cada signatura es fa amb el certificat digital del testimoni de seguretat del signant.",
                columns: {
                    action: "Acció",
                    appliesTo: "S'aplica a",
                    whoCanSign: "Qui pot signar",
                    signaturesNeeded: "Signatures necessàries",
                    requestExpires: "Caducitat de la sol·licitud",
                    waiting: "En espera",
                },
                off: "Desactivada",
                eachTrustee: "Cada autoritat",
                footerVersion:
                    "Les regles de signatura formen part de la versió de configuració {{version}} d'aquest esdeveniment.",
                footerFirstVersion:
                    "Les regles de signatura passaran a formar part de la primera versió de configuració d'aquest esdeveniment quan es publiqui.",
                footerChanged: "Darrer canvi: {{date}}.",
                footerChangedBy: "Darrer canvi: {{date}}, per {{name}}.",
                lockedDown:
                    "L'esdeveniment electoral està bloquejat: les seves regles de signatura pertanyen a la seva versió de configuració, de manera que només canvien mitjançant una nova versió de configuració.",
                edit: "Editar {{action}}",
                view: "Veure {{action}}",
                waitingCount_one: "{{count}} sol·licitud en espera",
                waitingCount_other: "{{count}} sol·licituds en espera",
                capacityError:
                    "No s'ha pogut carregar qui pot signar, de manera que el nombre de signatures no es pot comprovar amb els $t(signing.terms.posts).",
            },
            expiry: {
                "30": "30 minuts",
                "60": "1 hora",
                "120": "2 hores",
                "1440": "24 hores",
                "none": "Sense límit",
                "other": "{{count}} minuts",
            },
            rule: {
                needsSignatures: "Requereix signatures",
                whoCanSign: "Qui pot signar",
                whoCanSignHelp:
                    "Aquests rols reben el permís «Signar: {{action}}» a Usuaris i Rols, per a tots els esdeveniments electorals. Els signants també han de tenir accés al $t(signing.terms.post).",
                signaturesNeeded: "Signatures necessàries",
                signaturesNeededHelp:
                    "Cada signant fa servir el seu certificat digital. Cada $t(signing.terms.post) té almenys {{n}} persones que poden signar.",
                signaturesNeededShortHelp: "Cada signant fa servir el seu certificat digital.",
                requesterSigning: "La persona que la inicia també pot signar",
                expiresAfter: "Una sol·licitud caduca al cap de",
                trusteesSign: "Les autoritats signen aquest pas",
                trusteesHelp:
                    "Cada autoritat signa el seu propi pas amb el seu certificat digital. La cerimònia de claus determina quantes autoritats hi participen.",
                footer: "Els canvis queden anotats al registre de l'esdeveniment electoral i passen a formar part de la versió de configuració següent.",
                cancel: "Cancel·lar",
                save: "Desar",
                saved: "S'ha desat la regla de signatura.",
                savedShort_one:
                    "S'ha desat la regla de signatura. {{posts}} encara no pot arribar al nombre: afegiu-hi un signant.",
                savedShort_other:
                    "S'ha desat la regla de signatura. {{posts}} encara no poden arribar al nombre: afegiu-hi signants.",
                checkedOnSave: "El nombre es comprova amb els nous rols en desar.",
                savedRequesterShort:
                    "S'ha desat la regla de signatura. Alguns $t(signing.terms.posts) no poden arribar al nombre sense la persona que inicia una sol·licitud.",
                saveError:
                    "No s'ha pogut desar la regla de signatura. Potser una altra persona l'ha canviada mentrestant; recarregueu i torneu-ho a provar.",
            },
            validation: {
                atLeastOne: "Almenys 1.",
                tooMany:
                    "Cap $t(signing.terms.post) no té {{n}} persones que puguin signar. El màxim és {{max}}.",
                tooManyEvent:
                    "Només {{max}} persones poden signar això. Trieu-ne com a màxim {{max}}.",
                atMost: "Com a màxim {{max}}.",
                shortPosts_one:
                    "{{posts}} només té {{n}} persones que poden signar, de manera que no pot arribar a {{required}} signatures. Afegiu-hi un signant o reduïu el nombre.",
                requesterShort_one:
                    "Sense la persona que la inicia, {{posts}} només té {{n}} persones que poden signar, de manera que no pot arribar a {{required}} signatures.",
                requesterShort_other:
                    "Sense la persona que la inicia, {{posts}} només tenen {{n}} persones que poden signar, de manera que no poden arribar a {{required}} signatures.",
                shortPosts_other:
                    "{{posts}} només tenen {{n}} persones que poden signar, de manera que no poden arribar a {{required}} signatures. Afegiu-hi un signant o reduïu el nombre.",
            },
            pendingRequests_one:
                "{{count}} sol·licitud espera signatures amb la regla actual. En desar es cancel·la; la persona que la va iniciar haurà de tornar a començar.",
            pendingRequests_other:
                "{{count}} sol·licituds esperen signatures amb la regla actual. En desar es cancel·len; les persones que les van iniciar hauran de tornar a començar.",
            certificates: {
                issuersIntro:
                    "Els certificats del personal han d'encadenar amb un d'aquests. Són diferents dels certificats amb què inicien sessió els votants.",
                checkRevocation: "Comprovar les llistes de revocació",
                crlUnavailable: {
                    "label": "Quan no es pot baixar una llista",
                    "refuse": "No acceptar signatures",
                    "accept-unchecked": "Acceptar i marcar la signatura com a no comprovada",
                },
                registration: {
                    "label": "Registre d'un certificat a nom d'una persona",
                    "on-first-use": "Quan el seu titular hi signa per primera vegada",
                    "security-officer-only":
                        "Només quan el registra algú que pot registrar certificats",
                },
                onePost: "Un certificat només signa per a un $t(signing.terms.post)",
                issuers: "Emissors de confiança",
                import: "Importar certificats d'emissors",
                importHelp:
                    "Trieu un fitxer PEM o CER amb el certificat de l'emissor. Un fitxer PEM pot contenir diversos certificats.",
                chooseFile: "Triar un fitxer de certificat",
                fileError: "No s'ha pogut llegir el fitxer.",
                imported:
                    "{{imported}} certificats d'emissors importats; {{skipped}} ja eren de confiança.",
                importedWithErrors:
                    "{{imported}} certificats d'emissors importats, {{skipped}} ja eren de confiança. Rebutjats: {{errors}}",
                importError: "No s'han pogut importar els certificats d'emissors.",
                deleteIssuer: "Suprimir {{name}}",
                deleteIssuerConfirm:
                    "Voleu suprimir {{name}} dels emissors de confiança? Els certificats que ha emès ja no podran signar.",
                deleteError: "No s'ha pogut suprimir l'emissor.",
                noIssuers:
                    "Encara no hi ha emissors de confiança. El personal no pot signar fins que se n'importi un.",
                root: "Arrel",
                intermediate: "Intermedi",
                columns: {
                    issuer: "Emissor",
                    type: "Tipus",
                    issuedBy: "Emès per",
                    validUntil: "Vàlid fins a",
                    sha256: "SHA-256",
                    person: "Persona",
                    post: "$t(signing.terms.post)",
                    certificate: "Certificat",
                    registered: "Registrat",
                    status: "Estat",
                },
                checks: "Comprovacions",
                checksSaved: "S'han desat les comprovacions de certificats.",
                checksError: "No s'han pogut desar les comprovacions de certificats.",
                crlSchedule: "Es baixen de cada emissor cada hora.",
                crlUpdated: "{{url}}: actualitzada {{time}}",
                crlFailed: "{{url}}: no s'ha pogut baixar (darrer intent {{time}})",
                registeredTitle: "Certificats registrats",
                search: "Cercar persones, certificats o $t(signing.terms.posts)",
                status: "Estat",
                statusAll: "Tots",
                statuses: {
                    "active": "Actiu",
                    "expires-soon": "Caduca aviat",
                    "expired": "Caducat",
                    "revoked": "Revocat",
                },
                revokedOn: "Revocat el {{date}}",
                allPosts: "Tots",
                noCertificates: "No hi ha certificats registrats.",
                registeredHow: {
                    "first-use": "En la primera signatura",
                    "security-officer": "Registrat per un administrador",
                },
                register: "Registrar un certificat",
                registerSubmit: "Registrar",
                registerDone: "S'ha registrat el certificat.",
                registerError: "No s'ha pogut registrar el certificat.",
                person: "Persona",
                personSearchHelp: "Escriviu part d'un nom d'usuari per trobar la persona.",
                registeredBy: "Per {{name}}",
                registerRefused:
                    "Aquest certificat no es pot registrar: comproveu que l'ha emès un emissor de confiança, que és vàlid avui i que està destinat a signar.",
                registeredToOther:
                    "Aquest certificat està registrat a nom de {{name}}. Si aquest compte també és de {{name}}, vinculeu-lo com el seu segon compte.",
                linkAccount: "Vincular com a segon compte de la mateixa persona",
                alreadyRegistered: "Aquest certificat ja està registrat a nom d'aquesta persona.",
                pem: "Certificat (PEM)",
                revoke: "Revocar",
                revokeOf: "Revocar el certificat de {{name}}",
                revokeTitle: "Revocar el certificat de {{name}}",
                revokeHelp:
                    "Un certificat revocat ja no pot signar. Les signatures que ja ha fet continuen sent vàlides.",
                revokeReason: "Motiu",
                revokeDone: "S'ha revocat el certificat.",
                revokeError: "No s'ha pogut revocar el certificat.",
            },
            requests: {
                exportCsv: "Exportar CSV",
                exportError: "No s'han pogut exportar les sol·licituds.",
                exportFileName: "signing-requests.csv",
                status: "Estat",
                statusAll: "Totes",
                statusCount: "{{status}} · {{count}} de {{total}}",
                expires: "Caduca {{time}}",
                lastSignatureBy: "{{name}}, {{time}}",
                empty: "Encara no hi ha sol·licituds de signatura.",
                columns: {
                    request: "Sol·licitud",
                    status: "Estat",
                    started: "Iniciada",
                    by: "Per",
                    lastSignature: "Darrera signatura",
                    code: "Codi",
                },
            },
            reports: {
                postRequired:
                    "Selecciona un lloc per generar aquest informe quan es requereixen signatures.",
                generateNotice:
                    "{{post}}: el document es genera ara. Es podrà imprimir i transmetre quan l'hagin signat {{n}} persones.",
            },
            status: {
                waiting: "En espera",
                completed: "Signada",
                executed: "Feta",
                cancelled: "Cancel·lada",
                expired: "Caducada",
                failed: "Fallida",
            },
            cancelReasons: {
                "by-requester": "La persona que la va iniciar l'ha cancel·lada",
                "by-operator": "Un operador l'ha cancel·lada",
                "rule-changed": "Ha canviat la regla de signatura de l'acció",
                "payload-changed": "Ha canviat el que se signa",
                "superseded": "Una sol·licitud més recent l'ha substituïda",
                "certificate-revoked": "S'ha revocat un certificat que l'havia signada",
            },
            panel: {
                rulePost:
                    "Requereix {{n}} signatures dels signants de {{post}}, cadascuna amb el seu certificat digital.",
                ruleEvent:
                    "Requereix {{n}} signatures, cadascuna amb el certificat digital del signant.",
                signingCode: "Codi de signatura",
                signers: "Signants",
                sign: "Signar",
                handover: "El següent membre inicia la sessió",
                cancel: "Cancel·lar la sol·licitud",
                signedAt: "Signat {{time}}",
                notSigned: "Sense signar",
                certificate: "Certificat {{name}}",
                you: "(vós)",
                expiresAt: "Caduca a les {{time}}",
                progress: "{{count}} de {{total}}",
                openDocument: "Obrir el document",
                configurationVersion: "Versió de configuració {{version}}",
                configurationChanges: "Canvis en aquesta versió",
            },
            dialog: {
                title: "Signar {{object}}",
                steps: {
                    check: "Revisar",
                    certificate: "Certificat",
                    signed: "Signat",
                },
                localNote:
                    "La signatura es fa en aquest navegador. El vostre fitxer de certificat, la seva clau privada i la contrasenya no s'envien mai. Només la vostra signatura i el vostre certificat públic arriben al servidor.",
                check: {
                    signingAs: "Esteu signant com a {{name}}",
                    titlePost: "{{title}}, {{post}}",
                    sameCode: "Totes les persones que signen veuen el mateix codi.",
                    confirmDocument: "He revisat el que signo: {{object}}",
                },
                certificate: {
                    intro: "Inseriu el vostre testimoni de seguretat i trieu el vostre fitxer de certificat.",
                    password: "Contrasenya del certificat",
                    open: "Obrir el certificat",
                    chooseAnother: "Triar un altre fitxer",
                },
                checks: {
                    "passed": {
                        "trusted-issuer": "Emès per un emissor de confiança ({{root}})",
                        "valid-now": "Vàlid avui",
                        "signing-key-usage": "Destinat a signar",
                        "not-revoked": "No revocat (llistes actualitzades {{time}})",
                        "registered": "Registrat a nom vostre el {{date}}",
                        "registered-to-other": "No registrat a nom de cap altra persona",
                        "already-signed": "Encara no s'ha fet servir per a aquesta sol·licitud",
                        "post-binding": "Registrat per a aquest $t(signing.terms.post)",
                        "signature": "La signatura cobreix aquesta sol·licitud",
                    },
                    "failed": {
                        "trusted-issuer": "No emès per un emissor de confiança",
                        "valid-now": "No vàlid avui",
                        "signing-key-usage": "No destinat a signar",
                        "not-revoked":
                            "Revocat, o no hi ha cap llista de revocació vigent per comprovar-ho",
                        "registered": "No registrat a nom vostre",
                        "registered-to-other": "Registrat a nom de {{name}}",
                        "already-signed": "Ja s'ha fet servir per a aquesta sol·licitud",
                        "post-binding": "Registrat per a un altre $t(signing.terms.post)",
                        "signature": "La signatura no cobreix aquesta sol·licitud",
                    },
                    "first-use": "Primer ús: es registrarà a nom vostre",
                },
                problems: {
                    wrongPassword: "Contrasenya incorrecta. Reviseu-la i torneu-ho a provar.",
                    notForYou:
                        "Aquest certificat no pot signar per vós. Feu servir el certificat del vostre propi testimoni de seguretat.",
                    issuerNotAccepted:
                        "Feu servir el certificat que {{organization}} us ha registrat. No s'accepten certificats d'altres emissors.",
                    cancelled:
                        "Aquesta sol·licitud s'ha cancel·lat: {{reason}}. Les signatures que s'hi van donar ja no compten. Torneu-la a iniciar per signar la versió actual.",
                },
                signed: {
                    title: "Signat",
                    withCertificate: "amb el certificat de {{name}}",
                    count: "{{n}} de {{total}} signatures.",
                    allIn: "Ja hi són totes les {{total}} signatures.",
                    next: "A continuació signen: {{names}}.",
                },
                handover:
                    "Es tancarà la vostra sessió. El següent membre inicia la sessió en aquest ordinador i torna a aquesta sol·licitud per signar. La sol·licitud continua oberta fins a les {{time}}.",
                sign: "Signar",
                back: "Enrere",
                cancel: "Cancel·lar",
            },
            widget: {
                continue: "Continuar",
                done: "Fet",
                close: "Tancar",
                retry: "Tornar-ho a provar",
                loading: "S'està carregant la sol·licitud…",
                loadError: "No s'ha pogut carregar la sol·licitud.",
                chooseFile: "Triar el fitxer de certificat",
                fileInput: "Fitxer de certificat",
                fileSize: "{{size}} KB",
                showPassword: "Mostrar la contrasenya",
                hidePassword: "Amagar la contrasenya",
                opening: "S'està obrint el certificat…",
                checking: "S'està comprovant el certificat…",
                signing: "S'està signant…",
                certificateCard: "Emès per {{issuer}} · vàlid fins a {{date}} · {{algorithm}}",
                fingerprint: "SHA-256 {{fingerprint}}",
                algorithms: {
                    "rsa-pkcs1-sha256": "RSA",
                    "ecdsa-p256-sha256": "EC P-256",
                },
                document: "{{type}} · SHA-256 {{hash}}",
                documentPages: "{{type}} · {{pages}} pàgines · SHA-256 {{hash}}",
                checksTitle: "Comprovacions del certificat",
                untrustedIssuer:
                    "{{issuer}} no és un emissor de confiança per a aquest esdeveniment electoral",
                registeredToSomeoneElse: "Registrat a nom d'una altra persona",
                checkPassedNoDetail: {
                    "trusted-issuer": "Emès per un emissor de confiança",
                    "not-revoked": "No revocat",
                },
                organization: "la vostra organització",
                cantSign: "Aquest certificat no pot signar aquesta sol·licitud.",
                checkError: "No s'ha pogut comprovar el certificat. Torneu-ho a provar.",
                fileErrors: {
                    UNREADABLE_FILE:
                        "Aquest fitxer no és un fitxer de certificat (.p12 o .pfx), o està malmès.",
                    UNSUPPORTED_ENCRYPTION:
                        "Aquest navegador no pot obrir el xifratge que fa servir aquest fitxer.",
                    NO_PRIVATE_KEY:
                        "Aquest fitxer no té clau privada. Trieu el fitxer de certificat del vostre testimoni de seguretat.",
                    NO_CERTIFICATE: "Aquest fitxer no té cap certificat.",
                    UNSUPPORTED_KEY:
                        "El tipus de clau d'aquest certificat no és compatible. Feu servir un certificat RSA o EC P-256.",
                    KEY_CERTIFICATE_MISMATCH:
                        "El certificat d'aquest fitxer no coincideix amb la seva clau.",
                },
                openError: "No s'ha pogut obrir el certificat. Torneu-ho a provar.",
                signError: "No s'ha pogut enviar la signatura. Torneu-ho a provar.",
                refused: "El servidor ha rebutjat la signatura.",
                stale: "El document ha canviat mentre signàveu. Torneu a signar.",
                mismatch:
                    "El que se signaria no coincideix amb aquesta sol·licitud. Tanqueu el diàleg i torneu a obrir la sol·licitud.",
                documentMismatch: "El document no coincideix amb el que signa aquesta sol·licitud.",
                documentError: "No s'ha pogut baixar el document. Torneu-ho a provar.",
                alreadySigned: "Ja heu signat aquesta sol·licitud.",
                closed: {
                    changed:
                        "Aquesta sol·licitud ha canviat després que l'obríssiu. Tanqueu aquesta finestra i torneu-la a revisar abans de signar.",
                    allSigned: "Aquesta sol·licitud ja té totes les seves signatures.",
                },
                chooseCertificate: "Certificat amb què signar",
                renderError:
                    "No s'ha pogut mostrar la sol·licitud de signatura. Tanqueu-la i torneu-la a obrir.",
                signedAt: "{{time}}",
                panel: {
                    completedAt: "Signada a les {{time}}",
                    expired:
                        "Aquesta sol·licitud ha caducat. Les signatures que s'hi van donar ja no compten. Torneu-la a iniciar per signar.",
                    failed: "Hi són totes les signatures, però l'acció ha fallat. El registre en té els detalls.",
                    details: "Detalls",
                    close: "Tancar el tauler de la sol·licitud",
                },
                cancelDialog: {
                    title: "Voleu cancel·lar aquesta sol·licitud?",
                    body: "Les signatures que s'hi van donar ja no compten. La persona que la va iniciar haurà de tornar a començar.",
                    reason: "Motiu (opcional)",
                    confirm: "Cancel·lar la sol·licitud",
                    back: "Mantenir-la",
                    error: "No s'ha pogut cancel·lar la sol·licitud. Torneu-ho a provar.",
                },
                handoverDialog: {
                    title: "El següent membre inicia la sessió",
                    noExpiry:
                        "Es tancarà la vostra sessió. El següent membre inicia la sessió en aquest ordinador i torna a aquesta sol·licitud per signar.",
                    confirm: "Tancar la sessió",
                    back: "Mantenir la sessió",
                    error: "No s'ha pogut registrar el relleu. Torneu-ho a provar.",
                },
            },
            details: {
                keys_ceremony_id: "Cerimònia",
                tally_session_id: "Sessió de recompte",
                trustee_id: "Autoritat",
                key_share_sha256: "SHA-256 del fragment de clau",
                channel: "Canal",
                channels: "Canals",
                publication_id: "Publicació de paperetes",
                ballot_publication_id: "Publicació de paperetes",
                digest: "SHA-256 de la configuració",
                signing_rules: "Regles de signatura",
                scheduled_events: "Nous esdeveniments programats",
                ballots_and_contests: "Paperetes i concursos",
                application_id: "Sol·licitud d'inscripció",
                applicant_registry_id: "Compte del registre",
                decision: "Decisió",
                submitted_at: "Enviada",
                reason: "Per què requereix una persona",
                registry_record: "Registre del cens",
                status: "Estat de la sol·licitud d'inscripció",
                from: "Estat anterior",
            },
            closed: {
                pending: "Ja hi són totes les signatures. La votació es tanca d'aquí a un moment.",
                title: "La votació s'ha tancat a les {{time}}.",
                titleSealed: "La votació s'ha tancat a les {{time}}. Paperetes segellades.",
                record: "Acta de segellat",
                ballots: "Paperetes al segell",
                sealHash: "{{algorithm}} del segell",
                signedBy: "Signat per",
                signatures: "Signatures de tancament a l'acta de segellat",
                signaturesValue_one: "{{count}}, codi de signatura {{code}}",
                signaturesValue_other: "{{count}}, codi de signatura {{code}}",
                signers: "Signat pels membres",
            },
            values: {
                ballots_and_contests: {
                    "first-version": "Primera versió",
                    "no-changes": "Sense canvis",
                    "changed": "Amb canvis",
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
                    ONLINE: "En línia",
                    KIOSK: "Quiosc",
                    EARLY_VOTING: "Votació anticipada",
                    TELEPHONE: "Telèfon",
                },
                statuses: {
                    NOT_STARTED: "No iniciada",
                    OPEN: "Oberta",
                    PAUSED: "En pausa",
                    CLOSED: "Tancada",
                },
                channelStatus: "{{channel}}: {{status}}",
                ruleChange: "{{action}}: {{rule}}",
                ruleChangeFrom: "{{action}}: {{rule}} (abans {{was}})",
                ruleNeeds: "necessita {{n}}",
                ruleOff: "desactivada",
                decision: {
                    approve: "Aprovar",
                },
            },
            results: {
                signatures: "Signatures",
                needs: "Requereix {{n}}",
                off: "Desactivada",
                openRequest: "Obrir la sol·licitud de signatura",
                downloadSigned: "Baixar el PDF signat",
                print: "Imprimir",
                transmit: "Transmetre resultats",
                sendTo: "Enviar a {{count}} destins",
                awaiting: "{{item}}: esperant signatures",
                transmission: {
                    title: "Signatures",
                    description:
                        "Cada signant signa els resultats del paquet amb el seu certificat digital, en aquest navegador. El paquet es pot enviar quan l'hagin signat {{n}} persones.",
                    waiting:
                        "El paquet es pot enviar quan la seva sol·licitud de signatura tingui totes les signatures.",
                    signed: "El paquet porta totes les seves signatures i es pot enviar.",
                    ended: "La sol·licitud de signatura d'aquest paquet ha acabat. Torneu a crear el paquet per signar-lo.",
                },
            },
            waiting: {
                title: "Pendent de la meva signatura",
                buttonCount_one: "Pendent de la meva signatura: {{count}} sol·licitud per signar",
                buttonCount_other:
                    "Pendent de la meva signatura: {{count}} sol·licituds per signar",
                intro: "Sol·licituds que esperen les signatures de les accions que podeu signar, als vostres $t(signing.terms.posts).",
                close: "Tanca la llista",
                empty: "No hi ha res pendent de la vostra signatura.",
                loadError: "No s'han pogut carregar les sol·licituds que esperen signatures.",
                signedByYou: "Signada per vós",
            },
            notes: {
                afterApproval: "Després de l'aprovació",
                afterApprovalValue: "S'emeten les credencials del votant i se li envien",
                keyShare: "El vostre fragment de clau",
                keyShareChecked: "Comprovat: és el vostre fragment de clau per a aquesta cerimònia",
                recordedIn: "S'anota a",
                recordedInCeremony: "La cerimònia de claus i el tauler d'anuncis",
                recordedInTally: "La sessió de recompte",
            },
            keyShare: {
                signing:
                    "Signeu el vostre fragment de clau al tauler de signatura. Queda anotat un cop l'hàgiu signat.",
                record: "Anotar el meu fragment de clau",
                failed: "No s'ha pogut anotar el vostre fragment de clau signat: {{error}}",
                dropAgain:
                    "Torneu a deixar anar el vostre fitxer de fragment de clau per anotar el fragment de clau signat.",
                redo: "El vostre fragment de clau es va aportar sense la vostra signatura, que ara aquesta elecció requereix. Torneu-lo a aportar i signeu-lo.",
                notTaken:
                    "La cerimònia ja no accepta aquest fragment de clau. Torneu a deixar anar el vostre fitxer de fragment de clau.",
            },
        },
        lifecycle: {
            signedClose: {
                title: "Termini de tancament signat",
                deadline: "{{election}}: {{time}} · autoritzat per la configuració {{code}}.",
                explanation:
                    "Aquest termini signat continua sent vinculant encara que es canviï o s’elimini el calendari editable. El planificador tanca els canals autoritzats que encara estiguin oberts.",
                reached:
                    "Aquest termini signat ja ha vençut. Comproveu l’estat actual de la votació i el registre d’auditoria; encara no se n’ha registrat el processament.",
                processed: "Termini de tancament signat processat a les {{time}}.",
                signedAt: "Termini signat: {{time}}.",
                channels: "Canals que continuen coberts per aquest termini: {{channels}}.",
                result: "Consulteu l’estat de la votació i el registre d’auditoria per conèixer els canvis reals i el resultat complet.",
                unavailable:
                    "No s’han pogut carregar els terminis de tancament signats. Comproveu el calendari publicat i el registre d’auditoria.",
            },
            picker: {
                noMatch:
                    "Cap fus horari coincideix. Escriu una ciutat, un país, una zona, una abreviatura o un desplaçament.",
            },
            input: {
                timezone: "Fus horari",
                scheduledAt: "Programat per a",
                meetingStart: "Inici de la reunió",
                cronZone:
                    "La programació s'executa en el fus horari principal de l'esdeveniment, {{zone}}.",
                unconfiguredZone:
                    "{{zone}} no és un dels fusos horaris configurats de l'esdeveniment. Trieu-ne un.",
            },
            schedule: {
                allElections: "Totes les eleccions",
                outcome: "Resultat",
                noOffset: "Sense desplaçament horari: no s'executa mai",
                unpublished: "Encara no publicat",
                notPublished:
                    "Encara no hi ha res publicat: els votants veuen la programació després de la primera publicació.",
                unpublishedChanges_one:
                    "{{count}} esdeveniment programat ha canviat des de l'última publicació. Els votants el veuran quan publiquis.",
                unpublishedChanges_other:
                    "{{count}} esdeveniments programats han canviat des de l'última publicació. Els votants els veuran quan publiquis.",
                offsetless_one:
                    "{{count}} hora programada no té desplaçament horari, així que no s'executa mai. Edita-la per fixar-ne el fus horari.",
                offsetless_other:
                    "{{count}} hores programades no tenen desplaçament horari, així que no s'executen mai. Edita-les per fixar-ne el fus horari.",
                outcomeChange:
                    "En desar canvia el que fa aquesta transició programada: {{before}} → {{after}}.",
                outcomeNew: "Un cop desada, aquesta transició programada: {{after}}.",
                outcomeElections: "{{count}} de {{total}} eleccions",
                exportError: "No s'ha pogut exportar la programació.",
                exportFileName: "schedule.csv",
                totals: {
                    refused_one:
                        "{{count}} fila programada es rebutjarà ({{transitions}} transicions d'eleccions).",
                    refused_other:
                        "{{count}} files programades es rebutjaran ({{transitions}} transicions d'eleccions).",
                    runsUnsigned_one:
                        "{{count}} tancament programat s'executarà sense signatures ({{transitions}} transicions d'eleccions).",
                    runsUnsigned_other:
                        "{{count}} tancaments programats s'executaran sense signatures ({{transitions}} transicions d'eleccions).",
                    review: "Revisar",
                    showAll: "Mostrar-ho tot",
                    showing: {
                        refused:
                            "Es mostren les {{count}} files programades que es rebutjaran ({{transitions}} transicions d'eleccions).",
                        runsUnsigned:
                            "Es mostren els {{count}} tancaments programats que s'executaran sense signatures ({{transitions}} transicions d'eleccions).",
                    },
                },
                recompute: {
                    title_one:
                        "Una actualització de la base de dades de fusos horaris mou {{count}} hora programada futura. No canvia res fins que l'apliquis.",
                    title_other:
                        "Una actualització de la base de dades de fusos horaris mou {{count}} hores programades futures. No canvia res fins que les apliquis.",
                    change: "{{type}}: {{before}} → {{after}}",
                    apply: "Aplicar",
                    applied_one: "{{count}} hora programada actualitzada.",
                    applied_other: "{{count}} hores programades actualitzades.",
                    error: "No s'han pogut actualitzar les hores programades.",
                },
                outcomeChangeElections_one: "Desar canvia el resultat a {{count}} elecció:",
                outcomeChangeElections_other: "Desar canvia el resultat a {{count}} eleccions:",
            },
            authorizes: {
                reportPolicyOf: "{{election}}: {{value}}",
                initializationRetained:
                    "Un informe obligatori en aquesta configuració signada continua sent obligatori si la configuració actual del lloc canvia a no obligatori.",
                title: "Què autoritza aquesta aprovació",
                schedule: "Obertures i tancaments programats",
                noSchedule:
                    "No hi ha obertures ni tancaments programats: els signants obren i tanquen la votació.",
                opens: "S'obre {{time}}",
                closes: "Es tanca {{time}}",
                settings: "Configuració",
                unsignedClose: "Tancament programat sense signatures: {{value}}",
                initialization: "Inicialització: {{value}}",
                firstConfiguration:
                    "És la primera configuració aprovada: no hi ha res amb què comparar.",
                sameAsPrevious:
                    "La configuració és la mateixa que en la configuració aprovada anterior.",
                rule: {
                    openNeeds_one: "Obrir requereix {{count}} signatura",
                    openNeeds_other: "Obrir requereix {{count}} signatures",
                    openNoSignatures: "Obrir no requereix signatures",
                    closeNeeds_one: "Tancar requereix {{count}} signatura",
                    closeNeeds_other: "Tancar requereix {{count}} signatures",
                    closeNoSignatures: "Tancar no requereix signatures",
                    openSetting: "Obertura de la votació",
                    closeSetting: "Tancament de la votació",
                    signatures_one: "{{count}} signatura",
                    signatures_other: "{{count}} signatures",
                    none: "sense signatures",
                },
                diff: {
                    tightens: "Endureix: {{setting}} {{before}} → {{after}}",
                    loosens: "Relaxa: {{setting}} {{before}} → {{after}}",
                    mixed: "Canvia: {{setting}} {{before}} → {{after}} (més estricte en un aspecte i menys en un altre)",
                },
                comparedWith: "Comparat amb la configuració aprovada anterior, aprovació {{code}}:",
                channels: "Canals de votació per elecció",
                channelsOf: "{{election}}: {{channels}}",
                noChannels: "cap",
            },
            publish: {
                openedAuthorized:
                    "La votació es va obrir segons la programació ({{time}}), autoritzada per l'aprovació de configuració {{code}} (signada per {{names}}).",
                closedAuthorized:
                    "La votació es va tancar segons la programació ({{time}}), autoritzada per l'aprovació de configuració {{code}} (signada per {{names}}).",
                closedUnsigned:
                    "La votació es va tancar segons la programació ({{time}}). Sense signatures de tancament: la programació va tancar la votació a l'hora límit.",
                authorizedBy: "Autoritzat per",
                cancelledRequest:
                    "La sol·licitud {{code}} tenia {{n}} de {{k}} signatures i es va cancel·lar.",
                openedRefused: "L'obertura programada de {{time}} s'ha rebutjat.",
                closedRefused: "El tancament programat de {{time}} s'ha rebutjat.",
                openedNoSignaturesNeeded:
                    "La votació s'ha obert segons la programació ({{time}}); no calien signatures.",
                closedNoSignaturesNeeded:
                    "La votació s'ha tancat segons la programació ({{time}}); no calien signatures.",
                openedNothingToChange:
                    "A les {{time}} l'obertura programada no tenia res a obrir: els seus canals ja eren oberts.",
                closedNothingToChange:
                    "A les {{time}} el tancament programat no tenia res a tancar: els seus canals ja eren tancats.",
            },
            import: {
                title: "Importar la programació",
                subtitle:
                    "Una fila per esdeveniment i elecció, en hora local. Deixa el fus horari buit per utilitzar el fus horari de l'elecció.",
                chooseFile: "Tria un fitxer CSV",
                template: "Baixar la plantilla",
                templateFileName: "schedule-template.csv",
                ready: "{{ok}} esdeveniments a punt per a {{posts}} eleccions.",
                needsAttention_one:
                    "{{ok}} esdeveniments a punt per a {{posts}} eleccions. {{count}} fila requereix atenció; corregeix el fitxer i torna'l a pujar.",
                needsAttention_other:
                    "{{ok}} esdeveniments a punt per a {{posts}} eleccions. {{count}} files requereixen atenció; corregeix el fitxer i torna'l a pujar.",
                preview: "Files que s'importaran",
                row: "Fila",
                asWritten: "{{local}} · {{place}}",
                moreRows: "…i {{count}} files més",
                imported: "Programació importada: {{created}} creats, {{updated}} actualitzats.",
                uploadError: "No s'ha pogut comprovar el fitxer. Torna'l a pujar.",
                importError: "No s'ha pogut importar la programació.",
                error: {
                    unknownElection: "Cap elecció no té l'àlies {{election}}.",
                    unknownEventType: "{{type}} no és un tipus d'esdeveniment programat.",
                    invalidTimeZone: "{{zone}} no és un fus horari.",
                    invalidDateTime: "La data i l'hora han de tenir el format YYYY-MM-DDTHH:MM.",
                    invalidVotingChannels:
                        "Els canals de votació són desconeguts o obren alhora la votació en línia i l'anticipada.",
                    dstGap: "{{dateTime}} no existeix a {{city}} perquè s'avancen els rellotges. Escriu una hora que existeixi.",
                    duplicate:
                        "Una altra fila programa el mateix esdeveniment per a aquesta elecció.",
                    other: "Aquesta fila no es pot importar ({{code}}).",
                    ambiguousElection: "Més d'una elecció té l'àlies {{election}}.",
                },
            },
            settings: {
                accordion: "Idioma, data i hora",
                dateAndTime: "Data i hora",
                configured: "Fusos horaris configurats",
                configuredHelp:
                    "{{count}} fusos horaris. Les eleccions trien el seu d'aquesta llista; escriu una ciutat o un país per afegir-ne un.",
                moreZones: "+{{count}}",
                primary: "Fus horari principal",
                primaryHelp:
                    "S'utilitza per a les programacions de tot l'esdeveniment, els informes i les eleccions sense fus horari propi.",
                primaryInUse:
                    "{{zone}} és el fus horari principal. Tria abans un altre fus horari principal.",
                inUse: "{{zone}} l'utilitzen {{names}}. Canvia abans aquestes eleccions.",
                logs: "Hores als registres i a les seves exportacions",
                logsPrimary: "Fus horari principal ({{abbr}})",
                logsElection: "El fus horari de l'elecció de cada fila",
                logsHelp: "Les files sense elecció utilitzen el fus horari principal.",
                electionZone: "Fus horari",
                electionPrimary: "Principal de l'esdeveniment: {{zone}}",
                electionZoneHelp:
                    "Les programacions, les pantalles dels votants i els informes d'aquesta elecció utilitzen aquest fus horari, també en totes les seves àrees. Buit utilitza el fus horari principal de l'esdeveniment.",
                electionUnconfigured:
                    "L'esdeveniment ja no configura aquest fus horari, així que l'elecció utilitza el fus horari principal, {{zone}}. Tria un dels fusos horaris configurats.",
                electionUnconfiguredSave:
                    "Trieu un dels fusos horaris configurats de l'esdeveniment.",
            },
            policies: {
                accordion: "Cicle de la votació",
                intro: "Aquesta configuració forma part de la configuració de l'esdeveniment electoral: l'aprovació de la configuració la signa, i les obertures i els tancaments programats segueixen la més estricta entre la configuració actual i la publicada.",
                nothingPublished:
                    "Encara no hi ha res publicat: fins a la primera publicació, les obertures i els tancaments programats utilitzen els valors per defecte (per elecció, rebutjar).",
                publishedValue: "Configuració publicada: {{value}}",
                changedSincePublished:
                    "Ha canviat des de la configuració publicada: les obertures i els tancaments programats segueixen la més estricta de les dues fins a la propera publicació aprovada.",
                scope: {
                    title: "Inicialització abans d'obrir la votació",
                    post: {
                        label: "Per elecció",
                        help: "Una elecció s'obre quan està inicialitzada.",
                    },
                    event: {
                        label: "Tot l'esdeveniment",
                        help: "Cap elecció no s'obre fins que totes estiguin inicialitzades.",
                        warning:
                            "Una elecció sense inicialitzar manté tancades totes les eleccions, també a les seves obertures programades.",
                    },
                    postAndCountry: {
                        label: "Per elecció i país",
                        help: "Una elecció s'obre quan tots els seus països (àrees) estan inicialitzats.",
                        warning:
                            "Una elecció continua tancada, també a la seva obertura programada, fins que tots els seus països estan inicialitzats; cada país s'inicialitza amb el seu propi informe.",
                    },
                },
                close: {
                    title: "Tancament programat sense signatures",
                    help: "Quan tancar la votació requereix signatures i un tancament programat no és a la configuració signada.",
                    refuse: {
                        label: "Rebutjar",
                        help: "El tancament no s'executa; els signants de l'elecció tanquen la votació amb les seves signatures.",
                    },
                    runAsSystem: {
                        label: "Executar com a sistema",
                        help: "La votació es tanca a l'hora límit i queda registrada com a tancada per la programació sense signatures.",
                        warning:
                            "Els tancaments programats fora de la configuració signada tanquen la votació sense la signatura de ningú. El registre i els documents ho indiquen.",
                    },
                },
                onSave: {
                    outcomes_zero: "Cap transició programada no canvia de resultat.",
                    outcomes_one:
                        "{{count}} transició programada canvia de resultat. Revisa-la a Esdeveniments Programats.",
                    outcomes_other:
                        "{{count}} transicions programades canvien de resultat. Revisa-les a Esdeveniments Programats.",
                },
                saveError: "No s'ha pogut desar la configuració del cicle de la votació.",
                publishedPerTarget: "Configuració publicada, per destinació: {{values}}",
                publishedCount_one: "{{value}} ({{count}} destinació)",
                publishedCount_other: "{{value}} ({{count}} destinacions)",
                savedWithoutPolicies:
                    "L'esdeveniment electoral s'ha desat, però la configuració del cicle de la votació no: {{reason}}. Torneu-la a desar.",
            },
        },
        scheduledOutcome: {
            chip: {
                waitingForInitialization: "Esperant la inicialització",
                runs: "S'executarà",
                runsUnsigned: "S'executarà sense signatures",
                refused: "Es rebutjarà",
            },
            note: {
                waitingForInitialization: "Esperant la inicialització",
                authorized: "Autoritzat per la configuració {{code}}",
                noSignaturesNeeded: "No necessita signatures",
                closesUnsigned: "Es tanca sense signatures",
                refused: {
                    initialization: "La inicialització requerida és incompleta",
                    votingClose: "La votació no es pot obrir després del termini de tancament",
                    needsSignatures: "Necessita les signatures dels signants",
                    covered: "No és a la configuració signada",
                    unsignedClose: "Un tancament sense signatures es rebutja",
                    stricterCopy:
                        "Ha canviat des de la configuració publicada, que encara decideix",
                    defaults: "Encara no s'ha publicat res: s'apliquen els valors per defecte",
                },
                refusedWithStep: "{{reason}}. {{next}}",
            },
            why: {
                button: "Per què?",
                title: {
                    waitingForInitialization: "Per què espera la inicialització",
                    runs: "Per què s'executarà",
                    runsUnsigned: "Per què s'executarà sense signatures",
                    refused: "Per què es rebutjarà",
                },
                checks: "Comprovacions",
                check: "Comprovació",
                current: "Configuració actual",
                published: "Configuració publicada",
                verdict: "Resultat",
                allows: "Permet",
                blocks: "Bloqueja",
                deciding: "Comprovació decisiva",
                nextStep: "Següent pas:",
                signedBy: "Signat per {{names}}",
            },
            question: {
                initialization: "S’ha completat la inicialització requerida?",
                votingClose: "Aquesta obertura respecta el termini de tancament de la votació?",
                needsSignatures: "Aquesta acció necessita signatures?",
                covered: "Aquesta programació exacta és a la configuració signada?",
                unsignedClose: "Què passa amb un tancament sense signatures?",
                stricterCopy: "Difereixen la configuració actual i la publicada? Quina decideix?",
                defaults: "Ja hi ha alguna cosa publicada?",
            },
            check: {
                initialization: {
                    waiting:
                        "Cal completar les inicialitzacions exigides per la configuració actual i la publicada.",
                },
                votingClose: {
                    passed: "La votació es tanca a les {{closes_at}}; aquesta obertura no es pot executar en aquell moment ni després.",
                },
                needsSignatures: {
                    yes: "Sí, {{signatures}} signatures",
                    yes_one: "Sí, {{count}} signatura",
                    yes_other: "Sí, {{count}} signatures",
                    no: "No",
                },
                covered: {
                    overriddenBySignedPostRow:
                        "La configuració signada {{code}} utilitza l’obertura pròpia d’aquest lloc de votació, {{scheduled_event_id}}. L’obertura per a tot l’esdeveniment no s’aplica.",
                    yes: "Sí: aprovació {{code}}, sense canvis",
                    changed: "No: ha canviat des de l'aprovació {{code}}",
                    changedBy:
                        "No: editat el {{edited_at}} per {{edited_by}}, després de l'aprovació {{code}}",
                    notInApproval: "No: l'aprovació {{code}} no ho inclou",
                    noApproval: "Encara no hi ha cap configuració aprovada",
                    channelsChanged:
                        "No: els canals de votació de l'elecció han canviat des de l'aprovació {{code}}",
                    alreadyFired:
                        "No: aquesta transició de l'aprovació {{code}} ja s'ha executat el {{fired_at}}; tornar-la a executar necessita signatures",
                    late: "No: han passat més de 15 minuts des de {{scheduled_date}} (aprovació {{code}}); executar-la ara necessita signatures",
                },
                unsignedClose: {
                    refuse: "Rebutjar",
                    runAsSystem: "Executar com a sistema",
                },
                stricterCopy: {
                    same: "Totes dues són iguals",
                    currentStricter: "La configuració actual és més estricta: s'aplica ja",
                    currentLooser:
                        "La configuració actual és menys estricta: s'aplica després de la propera publicació aprovada",
                    combined: "Cadascuna és més estricta en un valor: s'apliquen totes dues",
                },
                defaults: {
                    published: "Publicat el {{published_at}}",
                    nothingPublished: "Res publicat: s'apliquen els valors per defecte",
                    noSnapshot:
                        "Publicat el {{published_at}}, abans que les publicacions desessin aquesta configuració: s'apliquen els valors per defecte",
                },
            },
            nextStep: {
                initialize:
                    "Completeu la inicialització requerida. El planificador ho tornarà a intentar abans del tancament de la votació.",
                closed: "Aquesta obertura no s’executarà després del tancament de la votació.",
                none: "No cal fer res.",
                publishAndApprove: "Publica i aprova la configuració.",
                requireConfigurationApproval:
                    "Fes que Aprovar la configuració requereixi signatures i, després, publica i aprova la configuració.",
                askSignersToOpen: "Demana als signants de l'elecció que obrin la votació.",
                askSignersToClose: "Demana als signants de l'elecció que tanquin la votació.",
            },
            applies: {
                tightens: "S'aplica ja a les accions manuals i programades.",
                loosens:
                    "S'aplica ja a les accions manuals; a les obertures i els tancaments programats, després de la propera publicació aprovada.",
                tightensAndLoosens:
                    "La seva part més estricta s'aplica ja a les accions manuals i programades; la seva part menys estricta s'aplica ja a les accions manuals i, a les obertures i els tancaments programats, després de la propera publicació aprovada.",
            },
        },
    },
}

export default catalanTranslation
