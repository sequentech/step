// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const tagalogTranslation: TranslationType = {
    translations: {
        philSysID: "PhilSys ID",
        iBP: "Integrated Bar of the Philippines (IBP)",
        philippinePassport: "Pasaporte ng Pilipinas",
        driversLicense: "Lisensya sa Pagmamaneho",
        seamanBook: "Libro ng Seaman",
        loading: "Naglo-load...",
        loadingDataProvider: "Naglo-load ng tagapagbigay ng datos...",
        tallySheetImport: {
            title: "Mga import ng tally sheet",
            subtitle:
                "Mag-import ng ES&S o CSV tally sheet files, i-preview ang nabuong ballot boxes, at aprubahan ang mga ito bago gumawa ng tally sheets.",
            createTitle: "I-import ang mga tally sheet",
            detailTitle: "Import ng tally sheet",
            empty: "Wala pang mga import ng tally sheet.",
            emptyBody:
                "Magsimula sa pag-import ng ES&S Enhanced XML o canonical CSV file para sa election event na ito.",
            sourceFormat: {
                ESS_ENHANCED_XML: "ES&S Enhanced XML",
                CANONICAL_CSV: "Canonical CSV",
            },
            channel: {
                PAPER: "Papel",
                POSTAL: "Postal",
                IN_PERSON: "Personal",
            },
            table: {
                created: "Nilikha",
                createdBy: "Ginawa ni",
                file: "File",
                format: "Format",
                channel: "Channel",
                status: "Katayuan",
                labels: "Mga Label",
                annotations: "Mga Anotasyon",
                actions: "Mga Aksyon",
            },
            summary: {
                imported: "Na-import",
                changed: "Nabago",
                new: "Bago",
                unchanged: "Walang pagbabago",
                conflicted: "May Salungatan",
                errors: "Mga Error",
            },
            status: {
                PENDING_REVIEW: "Naghihintay ng Review",
                APPROVED: "Inaprubahan",
                DISAPPROVED: "Hindi Inaprubahan",
                FAILED_VALIDATION: "Nabigo ang Validation",
                CONFLICTED: "May Salungatan",
                NEW: "Bago",
                CHANGED: "Nabago",
                UNCHANGED: "Walang pagbabago",
            },
            fields: {
                format: "Format",
                channel: "Channel",
                supportedFormats: "Mga suportadong format: XML, CSV",
                generatedTallySheet: "Nabuong tally sheet",
                sourceCandidates: "Mga source candidate ID",
                none: "Wala",
            },
            actions: {
                create: "I-import ang mga tally sheet",
                review: "I-review",
                source: "Source",
                cancel: "I-cancel",
                preview: "I-preview",
                save: "I-save ang import",
                approve: "Aprubahan",
                disapprove: "Hindi aprubahan",
                close: "Isara",
                openExisting: "Buksan ang umiiral na",
            },
            notifications: {
                selectFile: "Pumili ng import file bago ito i-preview",
                duplicateSource:
                    "Ang hash ng source file na ito ay lumitaw na sa isang naunang import ng tally sheet.",
                uploadUrlError: "Hindi makagawa ng upload URL",
                uploadError: "Hindi ma-upload ang import file",
                previewEmpty: "Walang laman ang sagot ng preview",
                previewError: "Hindi ma-preview ang import",
                importEmpty: "Walang laman ang sagot ng import",
                created: "Nagawa ang import ng tally sheet",
                createError: "Hindi magawa ang import",
                reviewEmpty: "Walang laman ang sagot ng review",
                conflicted: "May lumang baseline conflicts ang import",
                approved: "Inaprubahan ang import",
                disapproved: "Hindi inaprubahan ang import",
                reviewError: "Hindi ma-review ang import",
                sourceUrlError: "Hindi makagawa ng source download URL",
                sourceDownloadError: "Hindi ma-download ang source file",
            },
            pagination: {
                range: "{{rangeStart}}-{{rangeEnd}} ng {{total}}",
                previous: "Nakaraan",
                next: "Susunod",
            },
        },
        reconciliation: {
            menuButton: "I-sync ang ext. botante",
            categories: {
                VOTED_INTERNET: "Bumoto sa pamamagitan ng Internet",
                VOTED_OTHER_CHANNEL: "Bumoto sa ibang channel",
                DISABLED_DELETE_CALL: "Na-disable ang botante",
                DELETION_REVERTED: "Na-revert ang pagtanggal",
                PROFILE_UPDATE: "Na-update ang profile",
                VOTER_ADDED: "Naidagdag ang botante",
                REENABLED: "Na-enable muli ang botante",
                VOTED_UNMARKED: "Inalis ang markang bumoto ang botante",
                ROW_FAILURE: "Nabigo ang row",
            },
            table: {
                voterId: "ID ng Botante",
                field: "Field",
                category: "Kategorya",
                currentValue: "Kasalukuyang value",
                newValue: "Bagong value",
                reason: "Dahilan",
                rowLabel: "Row",
                noDifferences: "Walang nakitang pagkakaiba - naka-sync na ang mga sistema.",
            },
            wizard: {
                title: "Pag-sync ng External Reconciliation",
                subtitle: "I-sync ang listahan ng botante sa external system",
                drop: {
                    description:
                        "I-drop ang reconciliation file na ginawa ng external system - awtomatikong kinakalkula at ipinapakita sa magkahiwalay na tables ang parehong diffs (external side at Sequent side).",
                    fileFormatLabel: "CSV file",
                    uploading: "Ina-upload ang {{fileName}} at kinakalkula ang parehong diffs...",
                },
                review: {
                    fileSummary:
                        "{{fileName}} - Sequence {{sequence}}, ginawa noong {{generatedAt}}",
                    rowFailuresWarning:
                        "Hindi ligtas na ma-reconcile ang {{count}} row(s) at hindi kasama ang mga ito sa parehong diffs - tingnan ang mga detalye sa ibaba.",
                    noDifferences: "Walang pagkakaiba - naka-sync na ang dalawang sistema.",
                    diffOnlyDifferences:
                        "Convergence check ito para sa nailapat nang Sequence. Ipinapakita ang mga pagkakaiba para sa follow-up, ngunit hindi na maaaring ilapat muli ang round na ito.",
                    externalDiffTitle: "External diff",
                    sequentDiffTitle: "Sequent diff",
                    downloadExternalPatch: "I-download ang external patch",
                    externalDiffCaption:
                        "I-download ang patch at ibigay ito sa external system sa labas ng tool na ito. Kapag na-apply na nito ang patch at nagawa ang susunod na reconciliation file, i-click ang 'Bumalik' at i-drop ang file na iyon - mag-e-enable ang 'I-apply' kapag walang laman ang table na ito.",
                    noExternalDifferences: "Walang pagkakaiba sa external side.",
                    sequentDiffCaption:
                        "I-apply ang mga pagbabago nang direkta sa Sequent sa pamamagitan ng pag-click sa 'I-apply' - walang ginagawang patch file para dito.",
                    noSequentDifferences: "Walang pagkakaiba sa Sequent side.",
                },
                applying: {
                    inProgress: "Inilalapat ang mga pagbabago sa Sequent side...",
                    rowFailures:
                        "Hindi kasama ang {{count}} row(s) sa round na ito at kailangan ng manual na follow-up - tingnan ang mga detalye sa ibaba.",
                    rowFailuresTruncated:
                        "Ipinapakita ang unang {{shown}} sa {{count}} row failure. Ayusin ang magkakaparehong sanhi at subukang muli upang makita ang mga natitirang failure.",
                    success: "Matagumpay na nailapat ang lahat ng pagbabago sa Sequent side.",
                },
                actions: {
                    cancel: "I-cancel",
                    back: "Bumalik",
                    apply: "I-apply",
                    next: "Susunod",
                    startOver: "Ulitin mula sa simula",
                    close: "Isara",
                },
                confirm: {
                    title: "Kumpirmahin ang mga pagbabago sa reconciliation",
                    categoriesNote:
                        "Ang mga kategoryang naka-highlight sa orange ({{categories}}) ay nakakaapekto sa status ng pagboto o nagdi-disable ng mga botante.",
                    applyChanges: "I-apply ang mga pagbabago",
                    continue: "Magpatuloy",
                },
                summary: {
                    votedOtherChannel:
                        "mina-mark ang {{count}} botante bilang bumoto sa ibang channel",
                    disabled: "dini-disable ang {{count}} botante",
                    reenabled: "ine-enable muli ang {{count}} botante",
                    votedUnmarked: "inaalis ang markang bumoto sa {{count}} botante",
                    profileUpdated: "ina-update ang {{count}} profile",
                    voterAdded: "nagdadagdag ng {{count}} botante",
                    prefix: "Ilalapat nito ang mga pagbabagong {{parts}}.",
                    empty: "Walang pagbabago sa Sequent side na ila-apply.",
                },
                notifications: {
                    envelopeLoadError: "Hindi na-load ang reconciliation diff - subukan muli.",
                    generateFailed:
                        "Hindi na-calculate ang reconciliation diff - tingnan ang task widget para sa detalye.",
                    applyFailed:
                        "Hindi na-apply ang mga pagbabago sa Sequent side - tingnan ang task widget para sa detalye.",
                    uploadUrlError: "Hindi makakuha ng upload URL",
                    generateTaskError: "Hindi masimulan ang reconciliation diff task",
                    uploadError: "Hindi ma-upload ang reconciliation file",
                    applyTaskError: "Hindi masimulan ang apply task",
                    applyError: "Hindi na-apply ang mga pagbabago sa reconciliation",
                },
            },
        },
        logsScreen: {
            noPermissions: "Wala kang permiso upang ma-access ang mga log.",
            title: "Mga Log",
            subtitle: "Pangkalahatang mga log ng pangunahing at IAM na database",
            actions: {
                csv: "I-export sa CSV",
                pdf: "I-export sa PDF",
            },
            exportdialog: {
                description:
                    "Pakikonpirma na nais mong isagawa ang aksyong ito, maaaring magtagal ito bago matapos.",
                title: "I-export ang mga log",
                from: "Mula",
                to: "Hanggang",
                timeZone: "Timezone",
                format: "Format",
                csv: "CSV",
                pdf: "PDF",
                zoneNote:
                    "Bawat hilera ay may oras sa UTC (ISO 8601) at sa {{abbr}}, kasama ang pangalan ng timezone. Kasama sa saklaw ng petsa ang magkabilang dulo, sa {{abbr}}.",
                zoneNotePdf:
                    "Ipinapakita ng PDF ang bawat oras sa {{abbr}}. Kasama sa saklaw ng petsa ang magkabilang dulo, sa {{abbr}}.",
                rowZones: "Ang timezone ng halalan ng bawat hilera",
                zoneNoteRows:
                    "Bawat hilera ay may oras sa UTC (ISO 8601) at sa timezone ng halalan nito, kasama ang pangalan ng timezone. Kasama sa saklaw ng petsa ang magkabilang dulo, sa {{abbr}}.",
                zoneNoteRowsPdf:
                    "Ipinapakita ng PDF ang bawat oras sa timezone ng halalan nito. Kasama sa saklaw ng petsa ang magkabilang dulo, sa {{abbr}}.",
            },
            filter: {
                createdFrom: "Nilikha mula",
                createdTo: "hanggang",
                statementTimestampFrom: "Timestamp ng pahayag mula",
                statementTimestampTo: "Timestamp ng pahayag hanggang",
                timeZone: "Timezone",
            },
            scheduledOutcome: {
                outcome: {
                    "waiting-for-initialization": "Naghihintay ng inisyalisasyon",
                    "runs": "tumatakbo",
                    "runs-unsigned": "tumatakbo nang walang lagda",
                    "refused": "tinanggihan",
                },
                check: {
                    "initialization": "Hindi pa kumpleto ang kinakailangang inisyalisasyon",
                    "voting-close":
                        "Hindi maaaring buksan ang pagboto pagkatapos ng takdang pagsasara",
                    "needs-signatures": "kailangan ng lagda",
                    "covered": "nasa nilagdaang configuration",
                    "unsigned-close": "pagsasara nang walang lagda",
                    "stricter-copy": "kasalukuyan at nailathalang setting",
                    "defaults": "wala pang nailathala",
                },
                changed: "Ngayon ay {{after}} (dati: {{before}}).",
                result: "Kinalabasan: {{outcome}}.",
                deciding: "Nagpasyang pagsusuri: {{check}}. {{value}}",
                authorizedBy: "Pinahintulutan ng configuration {{code}}.",
                nextStep: "Susunod na hakbang: {{step}}",
                reason: {
                    "ballot-box-seal-policy":
                        "Pinanatiling sarado: sa I-seal sa pagsasara, nananatiling sarado ang botohang naisara na.",
                    "never-opened-kept-open":
                        "Walang isasara ayon sa iskedyul: hindi kailanman nabuksan ang Post, kaya nananatili ito sa kalagayan nito.",
                },
            },
            column: {
                id: "ID",
                statement_kind: "Uri ng Pahayag",
                created: "Nilikha",
                statement_timestamp: "Tatak ng Panahon ng Pahayag",
                message: "Mensahe",
                user_id: "ID ng User",
                username: "Username",
                sender_pk: "Primary Key ng Nagpadala",
                log_type: "Uri ng Log",
                event_type: "Uri ng Kaganapan",
                description: "Paglalarawan",
                version: "Bersyon",
            },
            main: {
                title: "Mga Log ng Pangunahing Database",
            },
            iam: {
                title: "Mga Log ng IAM Database",
            },
            ballotBoxSeal: {
                sealHash: "Seal hash: {{hash}}",
                counted: "{{counted}} sa {{inBox}} na balota ang binilang.",
                notCounted_one:
                    "Ang isa pang balota ay pinalitan ng mas huling balota ng botante, itinapon, o ibinoto ng botanteng hindi kwalipikado.",
                notCounted_other:
                    "Ang {{count}} pang balota ay pinalitan ng mas huling balota ng botante, itinapon, o ibinoto ng botanteng hindi kwalipikado.",
                closeRequest: "Isinara ng Close voting request na {{request}}.",
                noCloseRequest:
                    "Isinara nang walang Close voting request (Ihinto ang Pagboto o ang nakatakdang pagsasara).",
                failedReason: "Dahilan: {{reason}}",
                failedLocked:
                    "Nananatiling naka-lock at hindi naka-seal ang ballot box: isang insidente.",
                verifiedCounted: "{{counted}} balota ang binilang mula sa seal.",
                tallySession: "Tally session {{session}}.",
                differs: "Ang naiiba: {{differs}}",
            },
        },
        tasksScreen: {
            noPermissions: "Wala kang pahintulot na ma-access ang mga log.",
            title: "Pagpapatupad ng Mga Gawain",
            subtitle: "Impormasyon tungkol sa mga naisagawang gawain",
            taskInformation: "Impormasyon ng Gawain",
            status: "katayuan: {{status}}",
            ok: "Sige",
            column: {
                id: "Indeks",
                name: "Pangalan ng Gawain",
                type: "Uri",
                execution_status: "Katayuan",
                start_at: "Oras ng Pagsisimula",
                end_at: "Oras ng Pagtatapos",
                executed_by_user: "Tagapagpatupad",
                annotations: "Mga Anotasyon",
                labels: "Mga Label",
                logs: "Mga Log",
            },
            tasksExecution: {
                DELETE_TENANT: "Tanggalin ang tenant",
                PUBLISH_BALLOT: "I-publish ang balota",
                VOTER_INFORMATION_LETTER: "Liham ng impormasyon para sa botante",
                EXPORT_MONITORING_DATA: "I-export ang Datos ng Pagsubaybay",
                EXPORT_ELECTION_EVENT: "I-export ang Kaganapan sa Halalan",
                CREATE_ELECTION_EVENT: "Lumikha ng Kaganapan ng Halalan",
                IMPORT_ELECTION_EVENT: "I-import ang Kaganapan sa Halalan",
                IMPORT_USERS: "I-import ang mga Gumagamit",
                EDIT_USER: "I-edit ang Botante",
                IMPORT_CANDIDATES: "I-import ang mga Kandidato",
                EXPORT_VOTERS: "I-export ang mga botante",
                CREATE_TRANSMISSION_PACKAGE: "Lumikha ng Transmission Package",
                EXPORT_BALLOT_PUBLICATION: "I-export ang Paglalathala ng Balota",
                GENERATE_TRANSMISSION_REPORT: "Bumuo ng Ulat ng Paglilipat",
                EXPORT_ACTIVITY_LOGS_REPORT: "I-export ang Ulat ng Mga Log ng Aktibidad",
                GENERATE_REPORT: "Bumuo ng ulat",
                EXPORT_TRUSTEES: "I-export ang mga Awtoridad",
                EXPORT_APPLICATION: "I-export ang Mga Aplikasyon",
                EXPORT_TENANT_CONFIG: "I-export ang Configurasyon ng Tenant",
                IMPORT_TENANT_CONFIG: "I-import ang Configurasyon ng Tenant",
                RENDER_DOCUMENT_PDF: "I-render ang dokumento bilang PDF",
                CREATE_TENANT: "Lumikha ng Tenant",
                EXPORT_TEMPLATES: "I-export ang mga Template",
                IMPORT_TEMPLATES: "I-import ang mga Template",
                DELETE_ELECTION_EVENT: "Tanggalin ang Kaganapan ng Halalan",
                DELETE_VOTERS: "Delete Voters",
                PREPARE_PUBLICATION_PREVIEW: "Ihanda ang paunang tingin ng publikasyon",
                EXPORT_TALLY_RESULTS_XLSX:
                    "I-export ang mga resulta ng pagbibilang sa format na XLSX",
                EXPORT_CERTIFICATE_AUTHORITIES: "I-export ang mga awtoridad sa sertipikasyon",
                PUBLISH_RESULTS_WEBSITE: "I-publish ang website ng mga resulta",
            },
            documentAccess: {
                title: "Pag-access sa dokumento",
                sensitivityNotice:
                    "Sensitibong impormasyon. Ibahagi ang password na ito sa nilalayong tatanggap lamang.",
                passwordLabel: "Password para buksan ang naka-encrypt na PDF",
                showPassword: "Ipakita ang password",
                copyPassword: "Kopyahin ang password",
                passwordCopied: "Nakopya ang password",
                passwordError: "Hindi nakuha ang password ng PDF",
                copyError: "Hindi makopya ang password",
                guidance:
                    "Nilo-load lamang ang password pagkatapos mong piliin ang Ipakita ang password. Kapag na-load na, lalabas dito ang read-only na field na may opsyong kumopya.",
            },
            widget: {
                taskTitle: "Gawain: {{title}}",
                viewTask: "Tingnan Ang Gawain",
                downloadDocument: "I-download ang File",
                downloadHashManifest: "Hash manifest",
            },
            exportTasksExecution: {
                success: "Matagumpay na natapos ang pag-export",
                error: "Error sa pag-export ng Tasks Execution",
            },
        },
        areas: {
            common: {
                title: "Mga Lugar",
                subTitle: "Pag-configure ng lugar.",
                deleteError: "Error sa pagtanggal ng lugar",
            },
            createAreaSuccess: "Lugar na nalikha",
            updateAreaSuccess: "Na-update ang lugar",
            createAreaError: "Hindi makalikha ng Lugar",
            sequent_backend_area_contest: "Mga Paligsahan",
            empty: {
                header: "Wala pang Lugar.",
                action: "Lumikha ng Lugar",
            },
            formImputs: {
                allowEarlyVoting: "Payagan ang Maagang Pagboto",
            },
        },
        integrationsScreen: {
            common: {
                gapiKey: "Google Calendar Service Account Key",
                gapiEmail: "Google Calendar Authentication Email",
                gapiKeyHelper:
                    "Hindi ipinapakita ang naka-save na key. Mag-paste ng bagong key para palitan ito.",
                gapiKeySaved: "Na-save ang Google Calendar Service Account Key",
            },
            errors: {
                invalidGapiKey: "Hindi wastong format ng Google Calendar Service Account Key",
                saveGapiKey: "Hindi ma-save ang Google Calendar Service Account Key",
            },
        },
        lookAndFeelScreen: {
            common: {
                helpLinks: "Mga Link ng Tulong",
                logoUrl: "URL ng Logo",
                css: "Custom CSS",
                displayName: "Ipinapakitang pangalan",
                displayNameHelp:
                    "Ang pangalan ng organisasyon sa mga mensaheng bumabanggit dito. Kapag walang laman: ang maikling pangalan ng tenant.",
            },
            errors: {
                invalidHelpLinks: "Hindi wastong format ng Mga Link ng Tulong",
            },
        },
        electionTypeScreen: {
            noPermissions: "Wala kang pahintulot na ma-access ang mga setting.",
            common: {
                title: "Uri ng Halalan",
                subtitle: "Pag-configure ng uri ng halalan",
                onlineVoting: "Online na Pagboto",
                kioskVoting: "Pagboto sa Kiosk",
                telephoneVoting: "Pagboto sa Telepono",
                settingTitle: "Mga Setting",
                settingSubtitle: "Pangkalahatang Pag-configure",
                createNew: "Lumikha ng Uri ng Halalan",
                emptyHeader: "Wala pang Uri ng Halalan.",
                emptyBody: "Gusto mo bang lumikha ng isa?",
            },
            create: {
                title: "Lumikha ng Uri ng Halalan",
            },
            edit: {
                title: "I-edit ang Uri ng Halalan",
            },
            tabs: {
                votingChannels: "MGA CHANNEL NG PAGBOTO",
                electionTypes: "URI NG HALALAN",
                languages: "WIKA",
                localization: "LOKALISASYON",
                integrations: "MGA INTEGRASYON",
                lookAndFeel: "PAGPASADYA KAN ITSURA",
                schedules: "NAISKEDYUL NA MGA KAGANAPAN",
                trustees: "TAGAPANGALAGA",
                BackupRestore: "Backup / Ibalik",
            },
        },
        trusteesSettingsScreen: {
            common: {
                emptyHeader: "Wala pang mga tagapangasiwa.",
                createNew: "Lumikha ng Tagapangasiwa",
                title: "Tagapangasiwa",
                subtitle: "Konpigurasyon ng Tagapangasiwa",
                emptyBody: "Gusto mo bang lumikha ng isa?",
            },
            create: {
                title: "Lumikha ng Tagapangasiwa",
            },
            edit: {
                title: "I-edit ang Tagapangasiwa",
            },
        },
        scheduleScreen: {
            noPermissions: "Wala kang pahintulot na ma-access ang mga setting.",
            createScheduleSuccess: "Nalikha ang iskedyul",
            createScheduleError: "Error sa paglikha ng iskedyul",
            deleteScheduleSuccess: "Naitanggal ang iskedyul",
            deleteScheduleError: "Error sa pagtanggal ng iskedyul",
            common: {
                title: "Naka-iskedyul",
                subtitle: "Pag-configure ng mga iskedyul",
                createNew: "Lumikha ng Iskedyul",
                emptyHeader: "Wala pang Iskedyul.",
                emptyBody: "Gusto mo bang lumikha ng isa?",
            },
            create: {
                title: "Lumikha ng Iskedyul",
                selectSchedule:
                    "Pumili ng isang iskedyul mula sa predefined list o magsulat ng custom na iskedyul",
            },
            edit: {
                title: "I-edit ang Iskedyul",
            },
            eventTypes: {
                SYSTEM_LOCKDOWN_FOR_INTERNET_VOTING_SETTINGS:
                    "Pag-lockdown ng sistema para sa pinal na pag-configure ng Internet voting settings",
                START_PRE_REGISTRATION_OVCS:
                    "Pagsisimula at pagtatapos ng pre-registration para sa OVCS",
                END_PRE_REGISTRATION_OVCS: "Pagtatapos ng pre-registration para sa OVCS",
                START_TEST_VOTING_PERIOD: "Pagsisimula ng test voting period",
                END_TEST_VOTING_PERIOD: "Pagtatapos ng test voting period",
                START_INTERNET_VOTING_PERIOD: "Pagsisimula ng Internet voting period",
                END_INTERNET_VOTING_PERIOD: "Pagtatapos ng Internet voting period",
                LAB_TEST: "Lab test",
                FIELD_TEST: "Field test",
                MOCK_ELECTIONS: "Mock elections",
                FTS: "FTS",
            },
        },
        dashboard: {
            voteByDay: "Boto kada araw",
            votesOverTime: "Mga boto sa paglipas ng panahon",
            timeResolution: "Resolusyon ng oras",
            timeRange: "Saklaw ng oras",
            minute: "Minuto",
            hour: "Oras",
            day: "Araw",
            votersByChannels: "Mga botante ayon sa channel",
            voterLoginURL: "URL para sa Pag-login ng Botante",
            voterEnrollURL: "URL para sa Pag-enroll ng Botante",
            voterEnrollKioskURL: "Kiosk URL para sa Pag-enroll ng Botante",
            ballotBoxes: {
                show: "Ipakita ang mga ballot box",
                loadError:
                    "Hindi mabasa ang mga seal ng mga ballot box. I-reload ang pahina, o suriin ang koneksyon sa server.",
                title: "Mga ballot box",
                sealing:
                    "Nagsara ang botohan noong {{closed}}. Sine-seal ang mga ballot box pagkatapos ng grace period, sa {{deadline}}.",
                sealed: "Nagsara ang botohan noong {{closed}}. Naka-seal na ang mga ballot box: wala nang balotang maidadagdag, mababago o mabubura.",
                failed: "Nagsara ang botohan noong {{closed}}. May ballot box na hindi na-seal: nananatili itong naka-lock, at nasa logs ang insidente.",
                closedBySignatures:
                    "Isinara nina {{names}} gamit ang kanilang mga certificate, signing code {{code}}.",
                closedByUser: "Isinara ni {{username}}.",
                closedBySchedule: "Isinara ng nakatakdang pagsasara ng botohan.",
                column: {
                    area: "Lugar",
                    status: "Katayuan",
                    inTheBox: "Nasa box",
                    counted: "Binilang",
                    sealedAt: "Na-seal",
                    sealHash: "Seal hash",
                    record: "Seal record",
                },
                status: {
                    open: "Bukas",
                    sealing: "Ise-seal sa {{time}}",
                    publishing: "Naka-seal, ipinapaskil",
                    sealed: "Naka-seal",
                    failed: "Hindi naka-seal: insidente",
                    due: "Sine-seal na",
                    overdue: "Lampas na sa oras ng pag-seal",
                },
                help: {
                    publishing:
                        "Naka-lock ang ballot box. Muling ipinapaskil ang entry nito sa bulletin board.",
                    counted:
                        "Mga balotang binibilang: ang pinakahuling balidong balota ng bawat kwalipikadong botante. Ang iba sa ballot box ay pinalitan ng mas huling balota ng botante, itinapon, o ibinoto ng botanteng hindi kwalipikado.",
                },
                copyHash: "Kopyahin ang seal hash",
                copied: "Nakopya ang seal hash",
                copyError: "Hindi makopya ang seal hash",
                notYet: "Wala pa",
                openRecord: "Buksan ang seal record ng {{area}}",
                downloadRecord: "I-download ang seal record ng {{area}}",
                recordRestricted:
                    "Limitado: hingin ito sa administrator na makakapag-download ng mga dokumento.",
                recordError: "Hindi ma-download ang seal record. Subukang muli.",
                recordMissing:
                    "Nawawala ang dokumento ng seal record: i-report ito bilang insidente.",
                beforeClose: "Sine-seal ang ballot box ng bawat area kapag nagsara ang botohan.",
                notStarted:
                    "Hindi pa nagbubukas ang botohan. Sine-seal ang ballot box ng bawat area kapag nagsara ang botohan.",
                openOn: "Bukas ang botohan sa {{channels}}. Sine-seal ang ballot box ng bawat area kapag nagsara ang botohan.",
                paused: "Naka-pause ang botohan. Sine-seal ang ballot box ng bawat area kapag nagsara ang botohan.",
                holding_one:
                    "Naka-enable at hindi pa sarado ang {{channels}}: ihinto ito para ma-seal ang mga ballot box.",
                holding_other:
                    "Naka-enable at hindi pa sarado ang {{channels}}: ihinto ang mga ito para ma-seal ang mga ballot box.",
                sealingNow:
                    "Nagsara ang botohan noong {{closed}}. Sine-seal na ang mga ballot box.",
                sealingPastGrace:
                    "Nagsara ang botohan noong {{closed}}. Natapos ang grace period noong {{deadline}}; sine-seal na ang mga ballot box.",
                why: {
                    due: "Sine-seal na: aabot ito nang hanggang isang minuto.",
                    channelOpen:
                        "Naka-enable pa at hindi sarado ang {{channel}}: ihinto ito para ma-seal ang ballot box.",
                    channelNotEnabled:
                        "Hindi sarado ang {{channel}} at hindi ito naka-enable para sa Post na ito: ihinto ito para ma-seal ang ballot box.",
                    channelHasBallots:
                        "May mga balota ng {{channel}} sa ballot box na ito at hindi pa ito sarado: ihinto ito para ma-seal ang ballot box.",
                    datafixVotes_one:
                        "May {{count}} botong isinasagawa pa sa Datafix: sine-seal ang ballot box kapag naresolba ito.",
                    datafixVotes_other:
                        "May {{count}} botong isinasagawa pa sa Datafix: sine-seal ang ballot box kapag naresolba ang mga ito.",
                    stale: "Huling sinubukan noong {{time}}: maaaring hindi tumatakbo ang sealer. Suriin ang Beat at ang seal worker.",
                    notTried:
                        "Hindi pa nasusubukan: maaaring hindi tumatakbo ang sealer. Suriin ang Beat at ang seal worker.",
                    errorCategory: {
                        board: "Hindi naabot ng huling pagtatangka ang bulletin board; inuulit ito bawat minuto.",
                        census: "Hindi nabasa ng huling pagtatangka ang listahan ng mga botante; inuulit ito bawat minuto.",
                        keystore:
                            "Hindi nakuha ng huling pagtatangka ang signing key; inuulit ito bawat minuto.",
                        storage:
                            "Hindi na-upload ng huling pagtatangka ang seal record sa file storage; inuulit ito bawat minuto.",
                        settings:
                            "Hindi nabasa ng huling pagtatangka ang mga setting ng halalan; inuulit ito bawat minuto.",
                        other: "Nabigo ang huling pagtatangka; inuulit ito bawat minuto. Nasa service log ang mga detalye.",
                        ballots:
                            "May nakitang balota ang huling pagtatangka na hindi pa mabasa o isinasagawa pa; inuulit ito bawat minuto.",
                        database:
                            "Hindi natapos sa database ang huling pagtatangka; inuulit ito bawat minuto.",
                    },
                },
                failure: {
                    ballotIdMismatch: "May balotang hindi tugma sa Ballot ID nito.",
                    missingContent: "May balotang walang laman o walang Ballot ID.",
                    unreadable: "May balotang hindi mabasa.",
                    inProgress: "May balotang isinasagawa pa.",
                    alreadyOnBoard:
                        "Nasa bulletin board na ang isang seal para sa ballot box na ito.",
                    noBoard: "Walang bulletin board ang election event.",
                    unknownChannel: "May balotang hindi kilala ang voting channel.",
                },
                incident: {
                    title_one: "{{count}} ballot box ang hindi na-seal",
                    title_other: "{{count}} ballot box ang hindi na-seal",
                    body: "Isa itong insidente: nananatiling naka-lock ang bawat ballot box na ito at hindi ito maaaring i-tally. Sundin ang runbook para sa nabigong seal.",
                    line: "{{election}}, {{area}}: {{reason}}",
                },
            },
            ipAddress: {
                emptyState: "Wala pang mga boto.",
                title: "Mga IP Address",
                ip: "IP",
                country: "Bansa",
                VoteCount: "Bilang ng boto",
                ElectionName: "Pangalan ng halalan",
                VotersId: "Mga Id ng mga botante",
            },
        },
        electionEventScreen: {
            common: {
                subtitle: "Pag-configure ng kaganapan ng halalan.",
                showMore: "Lakihan ang nakikita",
                showLess: "Bawasan ang nakikita",
                adminPortal: "Admin Portal",
                allowPublishAfterLockdown: "Only allow election event publishing after lockdown",
                reset: "I-reset ang custom na filter",
            },
            edit: {
                general: "Pangkalahatan",
                dates: "Petsa",
                customUrls: "Pasadyang Unang Bahagi ng URL",
                votingPeriod: "Panahon ng Pagboto",
                language: "Wika",
                allowed: "Pinapayagang Mga Channel ng Pagboto",
                materials: "Mga Karagdagang Materyales",
                ballotReceipts: "Mga resibo ng balota",
                ballotDesign: "Disenyo ng Balota",
                templates: "Mga plantilya",
                reorder: "I-reorder ang mga halalan",
                advancedConfigurations: "Mga Advanced na Pag-configure",
                importCandidates: "Mag-import ng mga Kandidato",
                custom_filters: "Pasadyang mga filter",
                voter_authentication: "Pag-authenticate ng Botante",
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
                login: "Pag-login",
                enrollment: "Pag-enroll",
            },
            localization: {
                emptyHeader: "Walang nakatakdang wika para sa kaganapan",
                selectLanguage: "Pumili ng Wika",
                notify: {
                    success: "Matagumpay ang pag-update ng localization",
                    error: "Bigo ang pag-update ng localization",
                    duplicateKey: "May override na para sa key at saklaw ng portal na ito.",
                    invalidDateTimeFormat:
                        "Di-wastong format ng petsa/oras. Gamitin ang mga token na yyyy, MM, dd, HH, mm, ss (hal. dd/MM/yyyy HH:mm).",
                    invalidTimeZoneText: "Dapat panatilihin ng tekstong ito ang {{placeholders}}.",
                },
                common: {
                    title: "Localization",
                    subTitle: "Pag-configure ng localization",
                },
                labels: {
                    key: "Susi",
                    scope: "Saklaw ng portal",
                    value: "Halaga",
                },
                scopes: {
                    legacy: "Legacy ({{portal}})",
                    global: "Pangkalahatan",
                    votingPortal: "Portal ng pagboto",
                    ballotVerifier: "Tagapagpatunay ng balota",
                    resultsPortal: "Portal ng mga resulta",
                    adminPortal: "Portal ng admin",
                    templates: "Mga ulat at mensahe",
                },
            },
            field: {
                passwordPolicy: {
                    minimumLength: "Pinakamababang haba",
                    maximumLength: "Pinakamataas na haba",
                    includeUppercase: "Magsama ng malalaking titik",
                    includeLowercase: "Magsama ng maliliit na titik",
                    includeDigits: "Magsama ng mga numero",
                    includeSpecialCharacters: "Magsama ng mga espesyal na karakter",
                    help: {
                        minimumLength:
                            "Ang pinakamababang bilang ng mga karakter na kailangan para sa password.",
                        maximumLength:
                            "Ang pinakamataas na bilang ng mga karakter na pinapayagan para sa password.",
                        includeUppercase:
                            "Kailangan ng hindi bababa sa isang malaking titik sa password.",
                        includeLowercase:
                            "Kailangan ng hindi bababa sa isang maliit na titik sa password.",
                        includeDigits: "Kailangan ng hindi bababa sa isang numero sa password.",
                        includeSpecialCharacters:
                            "Kailangan ng hindi bababa sa isang espesyal na karakter sa password.",
                    },
                    notConfigured:
                        "Walang naka-configure na patakaran sa password. Kapag nag-save, ilalapat ang mga default na halaga sa ibaba.",
                    errors: {
                        lengthRange:
                            "Ang mga halaga ng haba ng password ay dapat mga buong numero mula 1 hanggang 256.",
                        minimumExceedsMaximum:
                            "Hindi maaaring lumampas ang pinakamababang haba sa pinakamataas na haba.",
                        characterClassRequired:
                            "Pumili ng hindi bababa sa isang klase ng karakter para sa password.",
                    },
                },
                name: "Pangalan",
                alias: "Alias",
                description: "Paglalarawan",
                startDateTime: "Petsa at Oras ng Pagsisimula",
                endDateTime: "Petsa at Oras ng Pagtatapos",
                language: "Wika",
                votingChannels: "Mga Channel ng Pagboto",
                materialActivated: "Mga Karagdagang Materyales na Gumagana",
                supportMaterialsPolicy: {
                    label: "Patakaran sa Mga Pangsuportang Materyales",
                    helperText:
                        "Ang Kinakailangan para Bumoto ay nangangailangan sa mga botante na buksan ang bawat Pangsuportang Materyal at kumpirmahin na nabasa na nila ito bago sila makaboto.",
                    options: {
                        off: "Naka-off",
                        optional: "Opsyonal",
                        mandatory_for_voting: "Kinakailangan para Bumoto",
                    },
                },
                materialTitle: "Pamagat",
                materialSubTitle: "Subtitle",
                logoUrl: "URL ng Logo",
                userVerification:
                    "Puede kang mag-introdusir nin sarong pasadyang plantilya na gagamiton tanganing mano-manong ma-verify an mga botante",
                redirectFinishUrl: "Redirect Finish URL",
                kioskRedirectFinishUrl: "Kiosk Redirect Finish URL",
                css: "Custom CSS",
                skipElectionList: "Laktawan ang Screen ng Listahan ng Halalan",
                showUserProfile: "Ipakita ang Profile ng Gumagamit",
                ballotReceipts: {
                    checksPeriod: {
                        policyLabel: "Panahon ng pagsusuri ng mga naihulog na balota",
                        helper: "Kung gaano katagal mahahanap ng mga botante ang kanilang naihulog na balota at mai-print ang resibo nito sa Voting Portal.",
                        options: {
                            "unlimited": "Walang limitasyon",
                            "until-date": "Hanggang sa isang petsa",
                        },
                    },
                    checksAvailableUntil: "Available ang mga pagsusuri hanggang ({{timezone}})",
                    checksAvailableUntilRequired:
                        "Ilagay ang petsa at oras kung hanggang kailan masusuri ang mga balota.",
                },
                voterAccessibilitySettingsPolicy: {
                    policyLabel: "Mga setting ng accessibility ng botante",
                    options: {
                        disabled: "Itago ang mga setting ng accessibility",
                        enabled: "Ialok ang laki ng teksto, contrast, agwat at galaw",
                    },
                },
                audioInstructionsPolicy: {
                    policyLabel: "Mga tagubiling audio",
                    options: {
                        "disabled": "Walang tagubiling audio",
                        "recorded": "Mga in-upload na recording lamang",
                        "recorded-or-synthesized":
                            "Mga in-upload na recording, o ang boses ng browser kung wala",
                    },
                },
                showCastVoteLogs: {
                    policyLabel: "Patakaran sa Ipakita ng mga Log ng Pagboto",
                    options: {
                        "show-logs-tab": "Ipakita ang Tab ng mga Log ng Pagboto",
                        "hide-logs-tab": "Laktawan ang Tab ng mga Log ng Pagboto",
                    },
                },
                lockdownState: {
                    policyLabel: "Kalagayan ng Lockdown",
                    helperText:
                        "Iiskedyul ang simula o katapusan ng panahon ng lockdown upang baguhin ang kalagayang ito.",
                    options: {
                        "locked-down": "Naka-lockdown",
                        "not-locked-down": "Hindi naka-lockdown",
                    },
                },
                ballotBoxSealPolicy: {
                    policyLabel: "Patakaran sa Pag-seal ng Ballot Box",
                    helperText:
                        "Kapag nagsara ang botohan, sine-seal ang ballot box ng bawat area: ipinapadala sa bulletin board ang isang pinirmahang hash ng mga balota nito, wala nang balotang maidadagdag, mababago o mabubura, at hindi na muling masisimulan ang botohan.",
                    locked: "Hindi na ito mababago kapag nabuksan na ang botohan.",
                    options: {
                        "seal-at-close": "I-seal sa pagsasara",
                        "do-not-seal": "Huwag i-seal",
                    },
                    checking: "Sinusuri kung nabuksan na ang botohan…",
                    lockedUnknown:
                        "Naka-lock: hindi mabasa ang mga halalan, kaya hindi alam kung nabuksan na ang botohan.",
                    lockedOpened: "Naka-lock: nabuksan na ang botohan sa {{names}}.",
                    lockedEvent: "Naka-lock: nabuksan na ang botohan sa election event na ito.",
                    refused:
                        "Hindi na mababago ang Patakaran sa Pag-seal ng Ballot Box kapag nabuksan na ang botohan.",
                    settingLocked: "Sa I-seal sa pagsasara, umaasa ang seal sa setting na ito.",
                    settingRefused:
                        "Hindi na mababago ang setting na ito kapag nabuksan na ang botohan: sa I-seal sa pagsasara, umaasa rito ang seal.",
                    boardRefused:
                        "Hindi na mababago ang bulletin board ng election event kapag nabuksan na ang botohan: sa I-seal sa pagsasara, dito ipinapaskil ang mga seal.",
                },
                ballotBoxSealRecordPolicy: {
                    policyLabel: "Seal Record ng Ballot Box",
                    options: {
                        restricted: "Limitado",
                        public: "Pampubliko",
                    },
                    help: {
                        restricted:
                            "Ang mga administrator lang ang makakapag-download nito; ibahagi ito sa mga tagamasid.",
                        public: "Sinumang may mga id ng event ay makakapag-download nito nang hindi nagsa-sign in; ipinapakita nito kung paano binilang ang bawat balota.",
                    },
                },
                decodedBallots: {
                    policyLabel: "Isama ang mga na-decode na balota sa database ng mga resulta",
                    options: {"included": "Isama", "not-included": "Huwag isama"},
                },
                contestEncryptionPolicy: {
                    options: {
                        "single-contest": "Isang Paligsahan",
                        "multiple-contests": "Maraming Paligsahan",
                    },
                    policyLabel: "Patakaran sa Pag-encode ng Paligsahan",
                },
                votingPortalDateTimeFormat: {
                    policyLabel: "Format ng petsa at oras ng Voting Portal",
                    helperText:
                        'Nalalapat sa buong kaganapan. Upang i-override bawat wika, idagdag ang key na "votingPortalDateTimeFormat" sa tab na Localization gamit ang mga token na yyyy, MM, dd, HH, mm, ss (hal. dd/MM/yyyy HH:mm). Tingnan ang dokumentasyon para sa mga detalye.',
                    options: {
                        "legacy-gb-24h": "Legacy GB 24h (dd/MM/yyyy HH:mm, 24h)",
                        "iso-local": "ISO Local (yyyy-MM-dd HH:mm)",
                        "us-12h": "US 12h (MM/dd/yyyy h:mm AM/PM)",
                        "locale-medium": "Locale Medium (katamtamang petsa, maikling oras)",
                        "date-only": "Date Only (walang oras)",
                        "custom": "Pasadyang format",
                    },
                    customFormat: {
                        label: "Pasadyang format ng petsa at oras",
                        helperText:
                            "Gamitin ang mga token na yyyy, MM, dd, HH, mm, ss (hal. dd/MM/yyyy HH:mm). Ang iba pang karakter ay ipapakita nang literal.",
                        invalid:
                            "Di-wastong format. Gumamit ng hindi bababa sa isa sa mga token na yyyy, MM, dd, HH, mm, ss.",
                    },
                },
                countDownPolicyOptions: {
                    NO_COUNTDOWN: "Walang Countdown",
                    COUNTDOWN: "Countdown",
                    COUNTDOWN_WITH_ALERT: "Countdown na may alerto",
                    sectionTitle: "Portal ng Pagboto",
                    policyLabel: "Patakaran sa Countdown ng Portal ng Pagboto",
                    coundownSecondsLabel: "Segundo bago mag-expire para ipakita ang countdown",
                    alertSecondsLabel: "Segundo bago mag-expire para ipakita ang Logout alert",
                },
                voterSigningPolicy: {
                    "policyLabel": "Patakaran sa Pagpirma ng Botante",
                    "no-signature": "Walang pirma",
                    "with-signature": "May pirma",
                },
                receiptsPolicy: {
                    "policyLabel": "Mga resibong pinirmahan ng ballot box",
                    "disabled": "Hindi pinagana",
                    "signed-by-ballot-box": "Pinirmahan ng ballot box",
                    "helperText":
                        "Kapag naka-on, iniimbak at pinipirmahan ng ballot box ang bawat balota sa pagsusuri, at makikita lamang ng botante ang Ballot ID kapag natanggap na ang balota. Pinipirmahan ng mga botante ang kanilang mga balota. I-publish muli ang mga balota pagkatapos itong baguhin.",
                    "lockedHelperText": "Hindi na ito mababago kapag nagsimula na ang botohan.",
                },
                VoterCertificatePolicy: {
                    policyLabel: "Voter Digital Certificate Policy",
                    enabled: "Naka-enable",
                    disabled: "Naka-disable",
                },
                enrollment: {
                    policyLabel: "Pagpaparehistro",
                    options: {
                        enabled: "Naka-enable",
                        disabled: "Naka-disable",
                    },
                },
                otp: {
                    policyLabel: "OTP",
                    options: {
                        enabled: "Naka-enable",
                        disabled: "Naka-disable",
                    },
                },
                ceremoniesPolicy: {
                    policyLabel: "Patakaran sa mga Seremonya ng Susi/Pagbilang",
                    options: {
                        "automated-ceremonies": "Payagan ang mga Awtomatikong Seremonya",
                        "manual-ceremonies": "Mga Manu-manong Seremonya",
                    },
                },
                automaticRecountPolicy: {
                    policyLabel: "Awtomatikong pagbilang muli matapos aprubahan ang import",
                    options: {
                        enabled: "Pinagana",
                        disabled: "Hindi pinagana",
                    },
                },
                weightedVotingPolicy: {
                    policyLabel: "Patakaran sa Timbang na Pagboto",
                    options: {
                        "areas-weighted-voting": "Timbang na Pagboto ayon sa mga Lugar",
                        "voters-weighted-voting": "Timbang na Pagboto ayon sa mga Botante",
                        "disabled-weighted-voting": "Hindi Pinagana ang Timbang na Pagboto",
                    },
                    noDelegated:
                        "Ang Timbang na Pagboto ayon sa mga Botante ay hindi maaaring pagsamahin sa Delegadong Pagboto",
                    noDecodedBallots:
                        "Ang Timbang na Pagboto ayon sa mga Botante ay hindi maaaring pagsamahin sa pagsasama ng mga na-decode na balota sa mga resulta",
                },
                delegatedVotingPolicy: {
                    policyLabel: "Patakaran sa Delegadong Pagboto",
                    options: {
                        enabled: "Pinagana",
                        disabled: "Hindi pinagana",
                    },
                },
                languageDetectionPolicy: {
                    policyLabel: "Patakaran sa Pag-detect ng Wika",
                    options: {
                        "browser-detect": "Awtomatikong tuklasin mula sa browser",
                        "force-default": "Ipatupad ang default",
                    },
                },
            },
            error: {
                endDate: "Ang pagtatapos na petsa ay dapat pagkalipas ng petsa ng pagsisimula",
                startDate: "Ang petsa ng pagsisimula ay dapat nasa hinaharap",
                noResult: "Walang pang Kaganapan ng Halalan",
                endDateInvalid: "Ang petsa ng pagtatapos ay dapat nasa hinaharap",
            },
            voters: {
                title: "Mga Botante",
            },
            createElectionEventSuccess: "Nalikha ang Kaganapan ng Halalan",
            createElectionEventError: "Error sa paglikha ng kaganapan ng halalan",
            ivr: {
                tabs: {
                    config: "Configuration",
                    blacklist: "Listahan ng pag-block",
                    prompts: "Mga voice prompt",
                    emulator: "Emulator",
                },
                common: {
                    saveSuccess: "Matagumpay na na-save",
                    saveError: "Nabigong i-save",
                    deleteSuccess: "Matagumpay na natanggal",
                    deleteError: "Nabigong tanggalin",
                },
                config: {
                    configuredPhone: "Na-configure na numero ng telepono",
                    infoMsg:
                        "I-configure ang IVR flow at ang mga property nito sa ibaba. Para sa higit pang detalye, makipag-ugnayan sa Sequent.",
                },
                prompts: {
                    emptyMsg: "Wala pang nagagawang prompt",
                    infoMsg:
                        "I-configure ang mga prompt na ginagamit ng IVR. Kinakailangan ang mga announcement prompt, at maaaring i-override ang mga system prompt para sa mga gustong wika. Sinusuportahan ang SSML, kabilang ang paghahalo ng mga wika.",
                    editorTitle: "Prompt",
                    editorSubtitle: "Configuration ng prompt",
                },
                blacklist: {
                    columns: {
                        phone: "Numero ng telepono",
                        reason: "Dahilan",
                        createdAt: "Ginawa noong",
                        createdBy: "Ginawa ni",
                        createdBefore: "Ginawa bago ang",
                        createdAfter: "Ginawa pagkatapos ng",
                    },
                    emptyMsg: "Walang entry sa listahan ng pag-block",
                    infoMsg:
                        "I-configure ang blocklist para sa IVR. Ang mga tawag mula sa mga numerong ito ay awtomatikong idi-disconnect ng system.",

                    noFilterMatch: "Walang entry na tumutugma sa ibinigay na mga filter",
                    phoneRequired: "Kinakailangan ang numero ng telepono",
                },
                emulator: {
                    infoMsg:
                        "Pumili ng lugar at ng mga nais na halalan upang subukan ang IVR session.",
                    apiStatus: {
                        unavailable: "Hindi available ang emulator system sa iyong environment",
                        loading: "Nilo-load ang emulator system",
                        error: "Nagkaroon ng error sa pag-load ng emulator system",
                    },
                    hints: {
                        title: "Mga pahiwatig",
                        publishRequired:
                            "Ang anumang pagbabagong ginawa sa mga halalan, mga contest, o mga kandidato ay dapat munang i-publish para maging available. Tanging ang pinakabagong na-publish na mga ballot style para sa tumutugmang lugar ang gagamitin sa emulator.",
                        eventChangesImmediate:
                            "Ang mga pagbabagong ginawa sa election event, gaya ng IVR configuration o mga pagbabago sa prompt, ay available kaagad kapag ni-restart ang emulator session.",
                        credentials: 'Ang valid na voter ID at PIN ay "123" at "123".',
                    },
                    sendDtmf: "Magpadala ng DTMF input",
                    keypadInput: "Input sa keypad",
                    sendTimeout: "Magpadala ng timeout",
                    disconnected: "Nadiskonekta",
                    startSession: "Magsimula ng bagong session",
                    endSession: "Tapusin ang session",
                    noStylesFound:
                        "Walang nakitang na-publish na mga ballot style na tumutugma sa iyong mga pinili",
                    inputPlaceholder: "Pindutin ang {{keys}} (timeout={{timeout}} s)",
                    inputPlaceholderOr: "o",
                    inputPlaceholderAnyKeys:
                        "Maglagay ng hanggang {{maxDigits}} digit (anumang digit, timeout={{timeout}} s)",
                    blacklistCaller: "I-block ang tumatawag",
                    elections: "Mga halalan",
                    area: "Lugar",
                },
            },
            stats: {
                elegibleVoters: "Mga Kwalipikadong Botante",
                voters: "Mga Aktwal na Botante",
                elections: "Mga Halalan",
                contests: "Mga Paligsahan",
                areas: "Mga Lugar",
                sentEmails: "Mga Email na Naipadala",
                sentSMS: "Mga SMS na Naipadala",
                calendar: {
                    title: "Kalendaryo",
                    scheduled: "Naka-iskedyul",
                },
            },
            keys: {
                createNew: "Lumikha ng Seremonya ng mga Susi",
                emptyHeader: "Walang pang Seremonya ng mga Susi.",
                statusLabel: "Katayuan",
                waitingKeys: "Naghihintay sa Paglikha ng mga Susi..",
                started: "Nagsimula noong",
                actions: {
                    participate: "Lumahok sa seremonya ng mga susi",
                    view: "Tingnan ang seremonya ng mga susi",
                },
                breadCrumbs: {
                    configure: "I-configure",
                    ceremony: "Seremonya",
                    created: "Natapos",
                    start: "Simulan",
                    status: "Katayuan",
                    download: "I-download",
                    check: "Suriin",
                    success: "Natapos",
                },
                notify: {
                    participateNow:
                        "Naanyayahan kang makibahagi sa seremonya ng mga susi. Mangyaring <1>i-click ang Aksyon ng Key ng seremonya</1> upang makilahok.",
                },
            },
            tabs: {
                dashboard: "Dashboard",
                monitoring: "Pagsubaybay",
                data: "Data",
                ivr: "IVR",
                localization: "Localization",
                voters: "Mga Botante",
                areas: "Mga Lugar",
                keys: "Mga Susi",
                tally: "Tally",
                tallySheetImports: "Mga import ng tally sheet",
                publish: "I-publish",
                logs: "Mga Log",
                tasks: "Mga Gawain",
                events: "Naka-schedule na Kaganapan",
                notifications: "Mga Abiso",
                reports: "Ulat",
                approvals: "Approvals",
                cas: "Mga Sertipiko",
            },
            tally: {
                emptyHeader: "Walang pang Tally.",
                title: "Tally ng Kaganapan ng Halalan",
                elections: "Mga Halalan",
                electionNumber: "Bilang ng mga Halalan",
                trustees: "Mga Tagapangasiwa",
                status: "Katayuan",
                permissionLabels: "Mga Label ng Pahintulot",
                tallyType: {
                    label: "Uri ng Bilang",
                    ELECTORAL_RESULTS: "Mga Resulta ng Halalan",
                    INITIALIZATION_REPORT: "Mga Resulta ng Inisyal",
                },
                create: {
                    title: "Lumikha ng Tally",
                    subtitle: "Lumikha ng bagong Tally para sa Kaganapan ng Halalan na ito",
                    createTallyButton: "Simulan ang Seremonya ng Tally",
                    createInitializationReportButton: "Lumikha ng Initialization Report",
                    error: {
                        create: "Error sa paglikha ng Tally",
                    },
                    success: "Nalikha ang Tally",
                },
                logs: {
                    noLogs: "Walang magagamit na mga log",
                },
                notify: {
                    noKeysTally:
                        "Hindi maaaring magsimula ang Seremonya ng Tally hanggang ang Seremonya ng Key ay matagumpay na nakumpleto.",
                    noPublication:
                        "Ang Seremonya ng Pagbibilang ay hindi maaaring magsimula hangga't hindi ka lumilikha ng isang post sa tab na I-publish.",
                    participateNow:
                        "Naanyayahan kang makibahagi sa seremonya ng Tally. Mangyaring <1>i-click ang Aksyon ng Key ng seremonya</1> upang makilahok.",
                    startDisabled:
                        "Hindi mo maaaring ipagpatuloy ang seremonya dahil walang napiling eleksyon o ang eleksyon ay hindi pa na-publish.",
                    ceremonyDisabled:
                        "Hindi mo maaaring ipagpatuloy ang seremonya dahil ang tally session ay hindi konektado o ang pagsisimula ng seremonya ay hindi pinapayagan.",
                },
            },
            importAreas: {
                title: "Mag-import ng mga Lugar",
                subtitle: "Mag-import ng data ng mga lugar",
                areaParagraph:
                    "Mag-import ng mga lugar gamit ang isang spreadsheet file sa format na Comma Separated Values (CSV).",
                importSuccess: "Matagumpay na na-import ang mga Lugar",
                importError: "Error sa pag-import ng mga Lugar",
                upsert: "Upsert ng mga Lugar",
            },
            import: {
                eetitle: "Mag-import ng Kaganapan ng Halalan",
                eesubtitle: "Mag-import ng data ng kaganapan ng halalan",
                title: "Mag-import ng mga Botante",
                subtitle: "Mag-import ng data ng mga botante",
                voters: "Mga Botante",
                votersParagraph:
                    "Mag-import ng mga botante gamit ang isang spreadsheet file sa format na Comma Separated Values (CSV). I-download ang isang halimbawa ng import CSV file dito.",
                electionEventParagraph:
                    "Mag-import ng mga Kaganapan ng Halalan gamit ang isang JSON file.",
                elections: "Mga Halalan",
                areas: "Mga Lugar",
                sha: "Pag-check ng Integridad (SHA-256)",
                cancel: "Kanselahin",
                import: "Mag-import",
                fileUploadSuccess: "Na-upload ang file sa server - ngunit hindi pa na-import",
                fileUploadError: "Error sa pag-upload ng file",
                importVotersSuccess: "Matagumpay na na-iskedyul ang Pag-import ng mga Botante",
                importVotersError: "Error sa pag-import ng mga Botante",
                importElectionEventSuccess: "Election event imported Successfully",
                importElectionEventError: "Error importing election event",
                shaDialog: {
                    ok: "Oo, Mag-import nang walang Pag-check ng Integridad",
                    cancel: "Bumalik",
                    title: "Mag-import nang Walang Pag-check ng Integridad?",
                    description:
                        "Hindi mo ipinakita ang field para sa Pag-check ng Integridad (SHA-256). Mangyaring kumpirmahin na ini-import mo ang tamang file at nais mo itong i-import.",
                },
                passwordDialog: {
                    title: "Decryption Password",
                    description: "Ilagay ang password para i-decrypt ang file",
                    label: "Password",
                    copyPassword: "Kopyahin ang Password",
                    ok: "Ok",
                },
            },
            export: {
                title: "I-export ang Kaganapang Halalan",
                subtitle:
                    "Ang pag-export ay maaaring isang mahabang proseso. Sigurado ka bang gusto mong i-export ang mga tala?",
                encryptWithPassword: "I-encrypt gamit ang Password",
                passwordForcedNote:
                    "Mapoprotektahan pa rin ng password ang archive: palaging naka-encrypt ang mga ulat, aplikasyon at datos ng bulletin board. Lagyan ng tsek ang kahon para isama rin ang mga na-decrypt na lihim na field ng botante.",
                includeVoters: "Isama ang mga Botante",
                activityLogs: "Mga Log ng Aktibidad",
                bulletinBoard: "Bulletin Board",
                publications: "Mga Publikasyon",
                s3Files: "Mga File ng S3",
                scheduledEvents: "Nakatakdang Mga Kaganapan",
                exportSuccess: "Matagumpay na nai-export ang Kaganapang Halalan",
                exportError: "Error sa pag-export ng Kaganapang Halalan",
                passwordTitle: "Password",
                passwordDescription: "Password para i-decrypt ang file:",
                copiedSuccess: "Nakopya ang password sa clipboard",
                copiedError: "Error sa pag-copy ng password sa clipboard",
                reports: "Mga Ulat",
                applications: "Mga Aplikasyon",
                tally: "Bilang",
                certificates: "Mga Sertipiko",
            },
            taskNotification:
                "{{action}} ay nagsimula na. Maaari mong makita ang status nito sa Talahanayan ng Pagpapatupad ng Mga Gawain.",
        },
        electionScreen: {
            common: {
                title: "Halalan",
                subtitle: "Pag-configure ng halalan.",
                fileLoaded: "Na-load ang file",
                noPermission: "Wala kang pahintulot na ma-access ang halalang ito.",
            },
            edit: {
                general: "Pangkalahatan",
                dates: "Petsa",
                votingPeriod: "Panahon ng Pagboto",
                language: "Wika",
                allowed: "Pinapayagang Mga Channel ng Pagboto",
                default: "Default",
                defaultLang: "Default na wika",
                receipts: "Mga Resibo",
                image: "Larawan",
                advanced: "Advanced na Pag-configure",
                numAllowedVotes: "Bilang ng pinapayagang boto",
                reorder: "I-reorder ang mga paligsahan",
                castVoteConfirm: "Modal ng Pagkumpirma ng Pagboto",
                gracePeriodPolicy: "Patakaran sa Palugit",
                allowTallyPolicy: "Payagan si Tally",
                permissionLabel: "Label ng pahintulot",
                custom_filters: "Pasadyang mga filter",
            },
            field: {
                name: "Pangalan",
                language: "Wika",
                votingChannels: "Mga Channel ng Pagboto",
                startDateTime: "Petsa at Oras ng Pagsisimula",
                endDateTime: "Petsa at Oras ng Pagtatapos",
                startDateTimeWithTimezone: "Petsa at Oras ng Pagsisimula ({{timezone}})",
                endDateTimeWithTimezone: "Petsa at Oras ng Pagtatapos ({{timezone}})",
                scheduledOpening: "Naka-iskedyul na Pagbubukas",
                scheduledClosing: "Naka-iskedyul na Pagsasara",
                alias: "Alias",
                description: "Paglalarawan",
                securityConfirmationHtml: "HTML ng Kumpirmasyon sa Seguridad",
                ivrPrompt: "Prompt ng IVR",
                externalId: "Panlabas na ID",
            },
            securityConfirmationPolicy: {
                label: "Patakaran sa Checkbox ng Kumpirmasyon sa Seguridad",
                none: "Wala",
                mandatory: "Kinakailangan",
            },
            error: {
                fileError: "Error sa pag-upload ng file",
                fileLoaded: "Na-load ang file",
                endDate: "Ang petsa ng pagtatapos ay dapat pagkalipas ng petsa ng pagsisimula",
                startDate: "Ang petsa ng pagsisimula ay dapat nasa hinaharap",
                endDateInvalid: "Ang petsa ng pagtatapos ay dapat nasa hinaharap",
            },
            createElectionEventSuccess: "Nalikha ang Kaganapan ng Halalan",
            createElectionEventError: "Error sa paglikha ng kaganapan ng halalan",
            tabs: {
                dashboard: "Dashboard",
                monitoring: "Pagsubaybay",
                data: "Data",
                voters: "Mga Botante",
                publish: "I-publish",
                logs: "Mga Log",
                approvals: "Approvals",
                tallySheets: "Mga Talaksahan",
            },
            gracePeriodPolicy: {
                "label": "Patakaran sa Palugit",
                "no-grace-period": "Walang palugit",
                "grace-period-without-alert": "Palugit na walang babala",
                "gracePeriodSecs": "Palugit sa segundo",
            },
            allowTallyPolicy: {
                "allowed": "Tinogotan",
                "disallowed": "Hindi Pinapayagan",
                "requires-voting-period-end": "Nangangailangan ng Pagtatapos ng Panahon ng Pagboto",
            },
            initializeReportPolicy: {
                "label": "I-initialize ang Patakaran sa Ulat",
                "not-required": "Hindi Kinakailangan",
                "required": "Kinakailangan",
            },
            castVoteGoldLevelPolicy: {
                label: "Gold level Authentication Policy",
                options: {
                    "gold-level": "Gold level Authentication",
                    "no-gold-level": "No Gold level Authentication",
                },
            },
            slates: {
                title: "Mga Slate",
                configuration: "Configuration ng mga slate (JSON)",
                helper: "Mga slate na may pangalan at ang mga kandidato ng bawat isa sa bawat paligsahan. Iwanang walang laman para sa halalang walang slate.",
                loading:
                    "Nilo-load pa ang mga paligsahan at kandidato ng halalan. Subukan muli sa ilang sandali.",
                mobileCandidateLists: {
                    label: "Mga listahan ng kandidato sa mobile",
                    helper: "Kung paano nagsisimula ang listahan ng kandidato ng bawat slate sa telepono. Maaari itong buksan o isara ng botante anumang oras.",
                    options: {
                        collapsed: "Nakatiklop",
                        expanded: "Nakabukas",
                    },
                },
            },
            startScreenTitlePolicy: {
                label: "Patakaran sa Pamagat ng Pangunahing Screen",
                options: {
                    "election": "Pamagat ng halalan",
                    "election-event": "Pamagat ng kaganapan ng halalan",
                },
            },
            consolidatedReportPolicy: {
                label: "Patakaran sa Pinagsamang Ulat",
                options: {
                    "generate": "Bumuo",
                    "do-not-generate": "Huwag bumuo",
                },
            },
            declineToVotePolicy: {
                label: "Patakaran sa pagtangging bumoto",
                options: {
                    enabled: "Pinagana",
                    disabled: "Naka-disable",
                },
            },
            blankBallotsPolicy: {
                label: "Patakaran sa mga blangkong balota",
                options: {
                    enabled: "Pinagana",
                    disabled: "Naka-disable",
                },
            },
            votingScreenBackPolicy: {
                label: "Patakaran sa pindutang Bumalik ng screen ng pagboto",
                options: {
                    "election-selection-screen": "Pumunta sa screen ng pagpili ng halalan",
                    "start-screen": "Pumunta sa panimulang screen ng halalan",
                },
            },
        },
        tenantScreen: {
            common: {
                title: "Mga Nangungupahan",
            },
            new: {
                subtitle: "Lumikha ng bagong nangungupahan",
            },
            createSuccess: "Nangungupahan na nilikha",
            createError: "Error sa paglikha ng nangungupahan",
        },
        usersAndRolesScreen: {
            noPermissions: "Wala kang pahintulot na ma-access ang mga tagagamit o tungkulin.",
            common: {
                title: "Mga Tagagamit at Tungkulin",
                subtitle: "Pangkalahatang pag-configure",
                mobileNumber: "Mobile",
            },
            editPassword: {
                passwordPolicyViolation:
                    "Hindi sumusunod ang password sa Patakaran sa Password para sa kaganapang ito ng halalan. Suriin ang patakaran sa Data ng Kaganapan ng Halalan at maglagay ng wastong password.",
                passwordPolicyRules: {
                    minimumLength: "Ang pinakamababang haba ng password ay {{count}}.",
                    maximumLength: "Ang pinakamataas na haba ng password ay {{count}}.",
                    uppercase: "Kinakailangang malalaking titik: {{count}}.",
                    lowercase: "Kinakailangang maliliit na titik: {{count}}.",
                    digits: "Kinakailangang mga numero: {{count}}.",
                    specialCharacters: "Kinakailangang espesyal na mga karakter: {{count}}.",
                },
                label: "Palitan ang password",
                temporatyLabel: "Pansamantala",
                temporatyInfo:
                    "Kung naka-enable, kinakailangang palitan ng tagagamit ang password sa susunod na pag-login",
            },
            users: {
                title: "Mga Tagagamit",
                subtitle: "Tingnan at i-edit ang data ng tagagamit",
                review: {
                    title: "Suriin ang mga Pagbabago",
                    subtitle: "Kumpirmahin ang mga update na ito bago isumite.",
                    confirm: "Kumpirmahin ang mga Pagbabago",
                    noChanges: "Walang pagbabagong susuriin",
                    field: "Field",
                    currentValue: "Kasalukuyang Halaga",
                    newValue: "Bagong Halaga",
                },
                edit: {
                    title: "Data ng Tagagamit",
                    subtitle: "Tingnan at i-edit ang tagagamit",
                },
                create: {
                    title: "Tagagamit",
                    subtitle: "Lumikha ng tagagamit",
                },
                fields: {
                    "has_voted": "Nakaboto",
                    "support_materials_viewed": "Support Materials Viewed",
                    "vote-weight": "Bigat ng Boto",
                    "voted-channel": "Channel ng pagboto",
                    "disable-comment": "Komento sa pag-disable",
                    "username": "Username",
                    "first_name": "Unang Pangalan",
                    "last_name": "Huling Pangalan",
                    "email": "Email",
                    "enabled": "Pinagana",
                    "emailVerified": "Na-verify ang Email",
                    "groups": "Mga Grupo",
                    "attributes": "Mga Katangian",
                    "area": "Lugar",
                    "password": "Password",
                    "savePassword": "I-save ang password",
                    "repeatPassword": "Ulitin ang Password",
                    "passwordMismatch": "Dapat magkatugma ang mga password",
                    "passwordLengthValidate": "Ang password ay dapat hindi bababa sa 8 karakter",
                    "passwordUppercaseValidate":
                        "Ang password ay dapat maglaman ng hindi bababa sa isang malaking titik",
                    "passwordLowercaseValidate":
                        "Ang password ay dapat maglaman ng hindi bababa sa isang maliit na titik",
                    "passwordDigitValidate":
                        "Ang password ay dapat maglaman ng hindi bababa sa isang digit",
                    "passwordSpecialCharValidate":
                        "Ang password ay dapat maglaman ng hindi bababa sa isang espesyal na karakter",
                    "trustee": "Kumilos bilang trustee",
                    "permissionLabel": "Label ng pahintulot",
                    "authorized-election-ids": "Halalan",
                },
                delete: {
                    body: "Sigurado ka bang gusto mong tanggalin ang tagagamit na ito?",
                    bulkBody: "Sigurado ka bang gusto mong tanggalin ang mga napiling tagagamit?",
                    bulkBodySelected: "Delete the {{count}} selected users? This cannot be undone.",
                    bulkBodyChoose:
                        "{{count}} users are selected. You can instead delete every user matching the current filters, which may be more. This cannot be undone.",
                    okSelected: "Delete {{count}} selected",
                    okAllMatching: "Delete all matching",
                },
                notifications: {
                    exportError: "Error sa pag-export ng mga tagagamit",
                    deleteError: "Error sa pagtanggal ng tagagamit",
                    deleteSuccess: "Natanggal ang tagagamit",
                    multipleDeleteSuccess: "Natanggal ang mga tagagamit",
                },
            },
            voters: {
                voterInformationLetter: {
                    label: "Liham ng impormasyon para sa botante",
                    generate: "Bumuo",
                    confirmation:
                        "Bumuo ng Liham ng impormasyon para sa botanteng ito? Magtatalaga ng bagong password at isasama ito sa isang naka-encrypt na PDF.",
                    generationStarted: "Nagsimula na ang pagbuo ng Liham ng impormasyon",
                    generationError: "Hindi mabuo ang Liham ng impormasyon",
                    policyNotConfigured:
                        "Hindi naka-configure ang Patakaran sa Password. I-configure ito sa Data ng Kaganapan ng Halalan bago bumuo ng liham.",
                    policyMinimumLengthMissing:
                        "Dapat may pinakamababang haba ang Patakaran sa Password bago bumuo ng liham.",
                    policyCharacterClassMissing:
                        "Dapat may hindi bababa sa isang klase ng karakter ang Patakaran sa Password bago bumuo ng liham.",
                },
                title: "Mga Botante",
                subtitle: "Tingnan at i-edit ang data ng botante",
                secretAttribute: {
                    storedPlaceholder: "Nakaimbak na naka-encrypt na halaga",
                    reveal: "Ipakita",
                    hide: "Itago",
                    revealError: "Hindi maipakita ang naka-encrypt na field ng botante",
                    includeInExport: "Isama ang na-decrypt na mga lihim na field ng botante",
                    exportWarning:
                        "Sensitibong export: ang na-download na CSV ay maglalaman ng mga field na ito bilang plain text.",
                    clear: "Burahin",
                    add: "Magdagdag ng halaga",
                    remove: "Alisin ang halaga",
                },
                review: {
                    title: "Suriin ang mga Pagbabago",
                    subtitle: "Kumpirmahin ang mga update na ito bago isumite.",
                    confirm: "Kumpirmahin ang mga Pagbabago",
                    noChanges: "Walang pagbabagong susuriin",
                    field: "Field",
                    currentValue: "Kasalukuyang Halaga",
                    newValue: "Bagong Halaga",
                },
                logs: {
                    label: "Mga Log ng Gumagamit",
                },
                create: {
                    title: "Botante",
                    subtitle: "Lumikha ng Botante",
                },
                manualVerification: {
                    label: "Manwal na I-verify",
                    verify: "Manu-manong i-verify ang botante",
                    body: "Manu-manong i-verify ang botante. Makakakuha ka ng PDF na may QR Code link na nagpapahintulot sa botante na mag-login na hindi dumaan sa online KYC.",
                    noEmailOrPhone:
                        "Ang botanteng ito ay hindi maaaring mano-manong ma-verify dahil wala silang nakatalagang email address o numero ng telepono.",
                },
                emptyHeader: "Wala pang mga botante.",
                askCreate: "Gusto mo bang lumikha ng isa?",
                errors: {
                    editError: "Error sa pag-edit ng botante",
                    editErrorReason: "Error sa pag-edit ng botante: {{reason}}",
                    editSuccess: "Nai-edit ang botante",
                    createError: "Error sa paglikha ng botante",
                    createErrorReason: "Error sa paglikha ng botante: {{reason}}",
                    createSuccess: "Nalikha ang botante",
                    attribute: {
                        invalidNamed: 'Tinanggihan ang "{{field}}": {{constraint}}',
                        fieldsToCorrect: "May mga field na kailangang itama bago mag-save",
                        hintBetween: "Sa pagitan ng {{min}} at {{max}} na karakter",
                        hintMin: "Hindi bababa sa {{min}} na karakter",
                        hintMax: "Hindi hihigit sa {{max}} na karakter",
                        andMore: "at {{count}} pa",
                        invalidLength:
                            'Ang "{{field}}" ay dapat na nasa pagitan ng {{min}} at {{max}} na karakter',
                        tooShort: 'Ang "{{field}}" ay dapat na hindi bababa sa {{min}} na karakter',
                        tooLong: 'Ang "{{field}}" ay dapat na hindi hihigit sa {{max}} na karakter',
                        required: 'Ang "{{field}}" ay kinakailangan',
                        invalidEmail: 'Ang "{{field}}" ay dapat na wastong email address',
                        invalidFormat: 'Ang "{{field}}" ay walang inaasahang format',
                        invalid: 'Ang "{{field}}" ay may hindi wastong halaga',
                    },
                    createPasswordError:
                        "Nalikha ang botante, ngunit hindi maitakda ang password nito",
                    createPasswordErrorReason:
                        "Nalikha ang botante, ngunit hindi maitakda ang password nito: {{reason}}",
                },
                delete: {
                    body: "Sigurado ka bang gusto mong tanggalin ang botante na ito?",
                    bulkBody: "Sigurado ka bang gusto mong tanggalin ang mga napiling botante?",
                    bulkBodySelected:
                        "Delete the {{count}} selected voters? This cannot be undone.",
                    bulkBodyChoose:
                        "{{count}} voters are selected. You can instead delete every voter matching the current filters, which may be more. This cannot be undone.",
                    okSelected: "Delete {{count}} selected",
                    okAllMatching: "Delete all matching",
                },
                notifications: {
                    exportError: "Error sa pag-export ng mga botante",
                    deleteError: "Error sa pagtanggal ng botante",
                    deleteSuccess: "Natanggal na ang botante",
                    multipleDeleteSuccess: "Natanggal na ang mga botante",
                    manualVerificationError: "Error sa manu-manong pag-verify ng botante",
                    manualVerificationSuccess:
                        "Matagumpay na na-verify nang manu-mano ang botante, nagda-download ng PDF..",
                },
            },
            roles: {
                title: "Mga Tungkulin",
                edit: {
                    title: "Data ng Tungkulin",
                    subtitle: "Tingnan at i-edit ang tungkulin",
                },
                create: {
                    title: "Tungkulin",
                    subtitle: "Lumikha ng tungkulin",
                },
                errors: {
                    createError: "Error sa paglikha ng tungkulin",
                    createSuccess: "Nalikha ang tungkulin",
                },
                fields: {
                    name: "Pangalan",
                },
                delete: {
                    body: "Sigurado ka bang gusto mong tanggalin ang tungkulin na ito?",
                },
                notifications: {
                    deleteError: "Error sa pagtanggal ng tungkulin",
                    deleteSuccess: "Naitanggal ang tungkulin",
                    permissionEditError: "Error sa pag-edit ng pahintulot",
                    permissionEditSuccess: "Na-edit ang pahintulot",
                },
            },
            permissions: {
                "voter-information-letter": "Bumuo ng Liham ng impormasyon para sa botante",
                "admin-user": "Admin na Tagagamit",
                "admin-dashboard-view": "Tingnan ang Dashboard ng Admin",
                "monitoring-view": "Tingnan ang mga Dashboard ng Pagsubaybay",
                "monitoring-configure": "I-configure ang mga Dashboard ng Pagsubaybay",
                "election-event-signatures-tab": "Tab ng mga Pirma ng Kaganapan sa Halalan",
                "signing-rules-read": "Mga Pirma: tingnan ang mga protektadong aksyon",
                "signing-rules-write": "Mga Pirma: i-edit ang mga protektadong aksyon",
                "signing-certificates-read": "Mga Pirma: tingnan ang mga sertipiko",
                "signing-issuers-write":
                    "Mga Pirma: mag-import at mag-alis ng mga pinagkakatiwalaang issuer",
                "signing-checks-write": "Mga Pirma: i-edit ang mga pagsusuri sa sertipiko",
                "signing-certificates-register": "Mga Pirma: magrehistro ng mga sertipiko",
                "signing-certificates-revoke": "Mga Pirma: bawiin ang mga sertipiko",
                "signing-requests-read": "Mga Pirma: tingnan ang mga kahilingan",
                "signing-requests-cancel": "Mga Pirma: kanselahin ang mga kahilingan",
                "signing-requests-export": "Mga Pirma: i-export ang mga kahilingan",
                "sign-initialize-voting": "Pirmahan: i-initialize ang pagboto",
                "sign-open-voting": "Pirmahan: buksan ang pagboto",
                "sign-close-voting": "Pirmahan: isara ang pagboto",
                "sign-generate-election-returns": "Pirmahan: bumuo ng election returns",
                "sign-generate-reports": "Pirmahan: bumuo ng iba pang ulat ng halalan",
                "sign-transmit-results": "Pirmahan: i-transmit ang mga resulta",
                "sign-approve-voter": "Pirmahan: manwal na aprubahan ang isang botante",
                "sign-approve-configuration":
                    "Pirmahan: aprubahan ang isang bersyon ng configuration",
                "sign-key-ceremony": "Pirmahan: kumpirmahin ang isang piraso ng susi",
                "sign-tally-key": "Pirmahan: iambag ang isang piraso ng susi",
                "application-export": "Pag-export ng Aplikasyon",
                "application-import": "Pag-import ng Aplikasyon",
                "tenant-create": "Lumikha ng Tenant",
                "tenant-read": "Basahin ang Tenant",
                "tenant-write": "I-edit ang Tenant",
                "tenant-delete": "Tanggalin ang Tenant",
                "election-event-create": "Lumikha ng Kaganapan ng Halalan",
                "election-event-read": "Basahin ang Kaganapan ng Halalan",
                "election-event-write": "I-edit ang Kaganapan ng Halalan",
                "keycloak-realm-attributes-read": "Read Keycloak realm attributes",
                "keycloak-realm-attributes-write": "Edit Keycloak realm attributes",
                "election-event-delete": "Tanggalin ang Kaganapan ng Halalan",
                "voter-create": "Lumikha ng Botante",
                "voter-read": "Basahin ang Botante",
                "voter-write": "I-edit ang Botante",
                "voter-secret-attribute-read": "Ipakita ang mga Lihim na Field ng Botante",
                "voter-secret-attribute-write": "I-edit ang mga Lihim na Field ng Botante",
                "user-create": "Lumikha ng Tagagamit",
                "user-read": "Basahin ang Tagagamit",
                "user-write": "I-edit ang Tagagamit",
                "user-permission-create": "Lumikha ng Pahintulot ng Tagagamit",
                "user-permission-read": "Basahin ang Pahintulot ng Tagagamit",
                "user-permission-write": "I-edit ang Pahintulot ng Tagagamit",
                "role-create": "Lumikha ng Tungkulin",
                "role-read": "Basahin ang Tungkulin",
                "role-write": "I-edit ang Tungkulin",
                "role-assign": "I-assign ang Tungkulin",
                "communication-template-create": "Lumikha ng Template ng Komunikasyon",
                "communication-template-read": "Basahin ang Template ng Komunikasyon",
                "communication-template-write": "I-edit ang Template ng Komunikasyon",
                "notification-read": "Basahin ang Notipikasyon",
                "notification-write": "I-edit ang Notipikasyon",
                "notification-send": "Magpadala ng Notipikasyon",
                "area-read": "Basahin ang Area",
                "area-write": "I-edit ang Area",
                "election-state-write": "I-edit ang Estado ng Halalan",
                "election-type-create": "Lumikha ng Uri ng Halalan",
                "election-type-read": "Basahin ang Uri ng Halalan",
                "election-type-write": "I-edit ang Uri ng Halalan",
                "voting-channel-read": "Basahin ang Channel ng Pagboto",
                "voting-channel-write": "I-edit ang Channel ng Pagboto",
                "trustee-create": "Lumikha ng Tagapangasiwa",
                "trustee-read": "Basahin ang Tagapangasiwa",
                "trustee-write": "I-edit ang Tagapangasiwa",
                "tally-read": "Basahin ang Tally",
                "tally-start": "Simulan ang Tally",
                "tally-write": "I-edit ang Tally",
                "tally-results-read": "Basahin ang Mga Resulta ng Tally",
                "publish-read": "Basahin ang Publish",
                "publish-write": "I-edit ang Publish",
                "publish-results-read": "Basahin ang Results Publication",
                "publish-results-write": "I-edit ang Results Publication",
                "logs-read": "Basahin ang Logs",
                "tasks-read": "Basahin ang Pagpapatupad ng Mga Gawain",
                "keys-read": "Basahin ang Mga Susi",
                "document-upload": "Mag-upload ng Mga Dokumento",
                "document-download": "I-download ang Mga Dokumento",
                "document-password-read": "Basahin ang Mga Password ng Dokumento",
                "tally-sheet-create": "Lumikha ng Tally Sheet",
                "tally-sheet-import-create": "Lumikha ng Import ng Tally Sheet",
                "tally-sheet-import-review": "Suriin ang Import ng Tally Sheet",
                "tally-sheet-import-view": "Tingnan ang Import ng Tally Sheet",
                "tally-recount-execute": "Isagawa ang Recount ng Tally",
                "trustee-ceremony": "Seremonya ng Tagapangasiwa",
                "tally-sheet-review": "Suriin ang Tally Sheet",
                "tally-sheet-view": "Tingnan ang Tally Sheet",
                "admin-ceremony": "Seremonya ng Admin",
                "tally-sheet-delete": "Tanggalin ang Tally Sheet",
                "cast-vote-read": "Basahin ang Na-cast na Mga Boto",
                "document-read": "Basahin ang Mga Dokumento",
                "document-write": "I-edit ang Mga Dokumento",
                "support-material-read": "Basahin ang Suportang Materyal",
                "support-material-write": "I-edit ang Suportang Materyal",
                "miru-create": "Miru Lumikha",
                "miru-download": "Miru I-download",
                "miru-send": "Miru Ipadala",
                "miru-sign": "Miru Lagdaan",
                "contest-write": "I-edit ang paligsahan",
                "contest-read": "Basahin ang paligsahan",
                "candidate-write": "I-edit ang mga kandidato",
                "candidate-read": "Basahin ang mga kandidato",
                "permission-label-write": "I-edit ang label ng pahintulot",
                "scheduled-event-write": "I-edit ang Naka-schedule na Kaganapan",
                "contest-create": "Lumikha ng Paligsahan",
                "contest-delete": "Tanggalin ang Paligsahan",
                "candidate-create": "Lumikha ng Kandidato",
                "candidate-delete": "Tanggalin ang Kandidato",
                "election-create": "Lumikha ng Halalan",
                "election-read": "Basahin ang Halalan",
                "election-write": "I-edit ang Halalan",
                "election-delete": "Tanggalin ang Halalan",
                "election-event-archive": "I-archive ang Kaganapang Halalan",
                "election-data-tab": "Tingnan ang Datos ng Halalan",
                "election-event-areas-tab": "Tingnan ang mga Lugar ng Kaganapang Halalan",
                "election-event-data-tab": "Tingnan ang Datos ng Kaganapang Halalan",
                "election-event-keys-tab": "Tingnan ang mga Susi ng Kaganapang Halalan",
                "election-event-logs-tab": "Tingnan ang mga Log ng Kaganapang Halalan",
                "election-event-publish-tab": "Tingnan ang Pag-publish ng Kaganapang Halalan",
                "election-event-reports-tab": "Tingnan ang mga Ulat ng Kaganapang Halalan",
                "election-event-scheduled-tab": "Tingnan ang Iskedyul ng Kaganapang Halalan",
                "election-event-tally-tab": "Tingnan ang Tally ng Kaganapang Halalan",
                "election-event-tasks-tab": "Tingnan ang mga Gawain ng Kaganapang Halalan",
                "election-event-voters-tab": "Tingnan ang mga Botante ng Kaganapang Halalan",
                "election-publish-tab": "Tingnan ang Pag-publish ng Halalan",
                "election-voters-tab": "Tingnan ang mga Botante ng Halalan",
                "report-write": "I-edit ang Mga Ulat",
                "report-read": "Basahin ang Mga Ulat",
                "users-menu": "Tingnan ang mga user at mga role",
                "settings-menu": "Tingnan ang mga setting",
                "templates-menu": "Tingnan ang mga template",
                "settings-election-types-tab": "Tingnan ang mga setting ng mga uri ng halalan",
                "settings-voting-channels-tab": "Tingnan ang mga setting ng mga channel ng pagboto",
                "settings-templates-tab": "Tingnan ang mga setting ng mga template",
                "settings-languages-tab": "Tingnan ang mga setting ng mga wika",
                "settings-localization-tab": "Tingnan ang mga setting ng lokalisasyon",
                "settings-look-feel-tab": "Tingnan ang mga setting ng itsura",
                "settings-trustees-tab": "Tingnan ang mga setting ng mga tagapangasiwa",
                "settings-countries-tab": "Tingnan ang mga setting ng mga bansa",
                "voter-import": "I-import ang Botante",
                "ee-voters-columns": "Tingnan ang mga Kolum ng mga Botante ng Kaganapang Halalan",
                "voter-manually-verify": "Manwal na I-verify ang Botante",
                "ee-voters-logs": "Tingnan ang mga Log ng mga Botante ng Kaganapang Halalan",
                "voter-export": "I-export ang Botante",
                "ee-voters-filters": "Tingnan ang mga Filter ng mga Botante ng Kaganapang Halalan",
                "voter-delete": "Tanggalin ang Botante",
                "voter-change-password": "Palitan ang Password ng Botante",
                "election-event-localization-selector":
                    "Tagapili ng Lokalisasyon ng Kaganapang Halalan",
                "localization-create": "Lumikha ng Lokalisasyon",
                "localization-read": "Basahin ang Lokalisasyon",
                "localization-write": "I-edit ang Lokalisasyon",
                "localization-delete": "Tanggalin ang Lokalisasyon",
                "area-create": "Lumikha ng Lugar",
                "area-delete": "Tanggalin ang Lugar",
                "area-export": "I-export ang Lugar",
                "area-import": "I-import ang Lugar",
                "area-upsert": "Magpasok o I-update ang Lugar",
                "election-event-areas-columns": "Mga Kolum ng mga Lugar ng Kaganapang Halalan",
                "election-event-areas-filters": "Mga Filter ng mga Lugar ng Kaganapang Halalan",
                "election-event-tasks-back-button": "Bumalik sa mga Gawain ng Kaganapang Halalan",
                "election-event-tasks-columns": "Mga Kolum ng mga Gawain ng Kaganapang Halalan",
                "election-event-tasks-filters": "Mga Filter ng mga Gawain ng Kaganapang Halalan",
                "task-export": "I-export ang mga Gawain",
                "application-read": "Basahin ang Aplikasyon",
                "application-write": "I-edit ang Aplikasyon",
                "approval-matrix-write": "I-edit ang Matrix ng Pag-apruba",
                "logs-export": "I-export ang mga Log",
                "election-event-logs-columns": "Mga Kolum ng mga Log ng Kaganapang Halalan",
                "election-events-logs-filters": "Mga Filter ng mga Log ng Kaganapang Halalan",
                "election-event-scheduled-event-columns":
                    "Mga Kolum ng mga Naka-iskedyul na Kaganapan ng Kaganapang Halalan",
                "scheduled-event-create": "Lumikha ng Naka-iskedyul na Kaganapan",
                "scheduled-event-delete": "Tanggalin ang Naka-iskedyul na Kaganapan",
                "election-event-reports-columns": "Mga Kolum ng mga Ulat ng Kaganapang Halalan",
                "report-create": "Lumikha ng Ulat",
                "report-delete": "Tanggalin ang Ulat",
                "report-generate": "Lumikha ng Ulat",
                "report-preview": "I-preview ang Ulat",
                "monitor-authenticated-voters": "Pagmamatyag sa Mga Awtentikadong Botante",
                "monitor-all-approve-disapprove-voters":
                    "Basahin ang Pagmamatyag sa Mga Aprubado at Hindi Aprubado",
                "monitor-automatic-approve-disapprove-voters":
                    "Basahin ang Pagmamatyag sa Awtomatikong Aprubado at Hindi Aprubado",
                "monitor-manually-approve-disapprove-voters":
                    "Basahin ang Pagmamatyag sa Manu-manong Aprubado at Hindi Aprubado",
                "monitor-enrolled-overseas-voters":
                    "Basahin ang Pagmamatyag sa Mga Naka-enroll na Botante sa Ibayong Dagat",
                "monitor-posts-already-closed-voting":
                    "Basahin ang Pagmamatyag sa Mga Post na Sarado na ang Botohan",
                "monitor-posts-already-generated-election-results":
                    "Basahin ang Pagmamatyag sa Mga Post na Naglabas na ng Resulta ng Eleksyon",
                "monitor-posts-already-opened-voting":
                    "Basahin ang Pagmamatyag sa Mga Post na Binuksan na ang Botohan",
                "monitor-posts-already-started-counting-votes":
                    "Basahin ang Pagmamatyag sa Mga Post na Sinimulan na ang Pagbilang ng Boto",
                "monitor-posts-initialized-the-system":
                    "Basahin ang Pagmamatyag sa Mga Post na Inisyalisa ang Sistema",
                "monitor-posts-started-voting":
                    "Basahin ang Pagmamatyag sa Mga Post na Nagsimula na ang Botohan",
                "monitor-posts-transmitted-results":
                    "Basahin ang Pagmamatyag sa Mga Post na Nagpadala ng Resulta",
                "monitor-voters-voted-test-election":
                    "Basahin ang Pagmamatyag sa Mga Botanteng Bumoto sa Test Election",
                "monitor-voters-who-voted": "Basahin ang Pagmamatyag sa Mga Botanteng Bumoto",
                "election-event-publish-preview": "I-preview ang Publikasyon ng Kaganapang Halalan",
                "election-event-publish-back-button":
                    "Bumalik sa Publikasyon ng Kaganapang Halalan",
                "election-event-publish-columns": "Mga Kolum ng Publikasyon ng Kaganapang Halalan",
                "election-event-publish-filters": "Mga Filter ng Publikasyon ng Kaganapang Halalan",
                "publish-create": "Lumikha ng Publikasyon",
                "publish-regenerate": "Muling Lumikha ng Publikasyon",
                "publish-export": "I-export ang Publikasyon",
                "publish-start-voting": "Simulan ang Pagboto",
                "publish-pause-voting": "I-pause ang Pagboto",
                "publish-stop-voting": "Itigil ang Pagboto",
                "publish-changes": "I-publish ang mga Pagbabago",
                "election-event-publish-view": "Tingnan ang Publikasyon ng Kaganapang Halalan",
                "election-event-keys-columns": "Mga Kolum ng mga Susi ng Kaganapang Halalan",
                "create-ceremony": "Lumikha ng Seremonya",
                "export-ceremony": "I-export ang Seremonya",
                "election-event-tally-columns": "Mga Kolum ng Tally ng Kaganapang Halalan",
                "election-event-tally-back-button": "Bumalik sa Tally ng Kaganapang Halalan",
                "transmition-ceremony": "Seremonya ng Transmisyon",
                "admin-ip-address-view": "Tingnan ang IP Address",
                "election-approvals-tab": "Tingnan ang mga Pagmamatyag sa Halalan",
                "election-event-approvals-tab": "Tingnan ang mga Pagmamatyag ng Kaganapang Halalan",
                "election-ip-address-view": "Tingnan ang IP Address ng Halalan",
                "election-dashboard-tab": "Tingnan ang Dashboard ng Halalan",
                "trustees-export": "I-export ang mga Tagapangasiwa",
                "user-import": "Pag-import ng Mga Tagagamit",
                "voter-voted-edit": "I-edit ang mga botanteng bumoto",
                "voter-email-tlf-edit": "I-edit ang mga field ng email/telepono ng mga botante",
                "cloudflare-write": "I-edit ang mga patakaran sa pag-block ng bansa sa Cloudflare",
                "transmission-report-generate": "Lumikha ng Ulat ng Transmisyon",
                "google-meet-link": "Bumuo ng Google Meet Link",
                "service-account": "Service account",
                "datafix-account": "Datafix account",
                "gold": "Ginto",
                "silver": "Pilak",
                "election-event-ivr-tab": "Tingnan ang IVR ng election event",
                "election-event-cas-tab": "Tingnan ang CAS ng election event",
                "ca-read": "Basahin ang mga certificate authority",
                "ca-write": "I-edit ang mga certificate authority",
                "generate-preview": "Bumuo ng preview",
                "preview-read": "Basahin ang preview",
                "tally-resolution-submit": "Isumite ang resolusyon ng tally",
                "phone-blacklist-read": "Basahin ang blacklist ng telepono",
                "phone-blacklist-create": "Gumawa ng mga entry sa blacklist ng telepono",
                "phone-blacklist-update": "I-edit ang mga entry sa blacklist ng telepono",
                "phone-blacklist-delete": "Tanggalin ang mga entry sa blacklist ng telepono",
                "election-event-voter-list-reconciliation":
                    "I-reconcile ang listahan ng mga botante ng election event",
                "messaging-account-read": "Basahin ang mga messaging account",
                "messaging-account-write": "Pamahalaan ang mga messaging account",
                "messaging-config-write": "I-configure ang messaging ng election event",
            },
        },
        generalSettingsScreen: {
            body: "I-enable ang mga wika sa sistema. Tanging ang mga wikang pinagana dito ang magagamait para sa mga kaganapan ng halalan.",
        },
        eventsScreen: {
            title: "Naka-schedule na Kaganapan",
            subtitle:
                "Pinamamahalaan ang configuration ng awtomatikong pagpapatupad ng mga kaganapan tulad ng pagsisimula o pagtatapos ng panahon ng pagboto.",
            messages: {
                createSuccess: "Matagumpay na nalikha ang Naka-schedule na Kaganapan",
                createError: "Error sa paglikha ng Naka-schedule na Kaganapan",
                editSuccess: "Matagumpay na na-edit ang Naka-schedule na Kaganapan",
                editError: "Error sa pag-edit ng Naka-schedule na Kaganapan",
                onlineWithEarlyVoting:
                    "Hindi maaaring buksan ng iskedyul ng pagsisimula ang Online at Maagang pagboto nang sabay: ang maagang pagboto ay dapat magsimula bago ang online na pagboto.",
            },
            eventType: {
                label: "Uri",
                ALLOW_INIT_REPORT: "Allow Initialization Report",
                START_VOTING_PERIOD: "Simula ng Panahon ng Pagboto",
                END_VOTING_PERIOD: "Pagtatapos ng Panahon ng Pagboto",
                ALLOW_VOTING_PERIOD_END: "Allow Voting Period End",
                START_ENROLLMENT_PERIOD: "Simulan ang Panahon ng Pagpapatala",
                END_ENROLLMENT_PERIOD: "Panahon ng Pagpapatala",
                START_LOCKDOWN_PERIOD: "Simulan ang Lockdown Period",
                END_LOCKDOWN_PERIOD: "Tapusin ang Panahon ng Lockdown",
                ALLOW_TALLY: "Payagan ang tally",
                START_READINESS_TEST: "Simulan ang Pagsubok sa Kahandaan ng Halalan",
                END_READINESS_TEST: "Tapusin ang Pagsubok sa Kahandaan ng Halalan",
                START_FINAL_TESTING: "Simulan ang Huling Pagsubok at Lockdown",
                END_FINAL_TESTING: "Tapusin ang Huling Pagsubok at Lockdown",
                START_TEST_VOTING: "Simulan ang Pagsubok na Pagboto",
                END_TEST_VOTING: "Tapusin ang Pagsubok na Pagboto",
            },
            warning: {
                votingWindowDays:
                    "Ang botohan ng {{election}} ay sumasaklaw sa {{days}} lokal na araw ({{start_local}} hanggang {{end_local}}, {{time_zone}}); {{expected}} ang hinihingi ng patakaran.",
                finalTestingLeadTime:
                    "Magsisimula ang final testing ng {{election}} sa {{final_testing_local}}, wala pang {{minimum_days}} araw bago magbukas ang botohan sa {{voting_start_local}} ({{time_zone}}).",
                closeBeforeOpen:
                    "Nagsasara ang botohan ng {{election}} bago o sa mismong pagbubukas nito ({{start_local}} hanggang {{end_local}}, {{time_zone}}).",
                shortLastDay:
                    "Ang huling araw ng botohan ng {{election}} ay may {{hours}} oras, kulang sa {{minimum_hours}}: nagsasara ang botohan sa {{end_local}} ({{time_zone}}).",
            },
            election: {
                label: "Halalan",
            },
            empty: {
                header: "Wala pang Naka-schedule na Kaganapan.",
                body: "Gusto mo bang lumikha ng isa?",
                button: "Lumikha ng Naka-schedule na Kaganapan",
            },
            create: {
                title: "Lumikha ng Naka-schedule na Kaganapan",
                subtitle: "Lumikha ng bagong configuration ng Naka-schedule na Kaganapan.",
            },
            edit: {
                title: "I-edit ang Naka-schedule na Kaganapan",
                subtitle: "I-edit ang configuration ng Naka-schedule na Kaganapan.",
                delete: "Sigurado ka bang gusto mong tanggalin ang Naka-schedule na Kaganapan na ito?",
            },
            fields: {
                electionId: "Halalan",
                eventProcessor: "Uri",
                stoppedAt: "Huminto Noong",
                scheduledDate: "Naka-schedule Noong",
            },
        },
        reportsScreen: {
            title: "Mga Ulat",
            subtitle: "Lumikha ng mga ulat para sa mga kaganapan ng halalan",
            messages: {
                createSuccess: "Matagumpay na nalikha ang ulat",
                createError: "Nagkaroon ng error sa paglikha ng ulat",
                submitError: "Error sa pagsusumite ng Ulat",
                updateSuccess: "Matagumpay na na-update ang Ulat",
                passwordMismatch:
                    "Ang password at confirm password ay hindi magkatugma. Siguraduhing pareho ang password sa dalawang field.",
                incorectPassword: "Incorrect password",
                decryptFileTitle: "Decrypt File",
                decryptInstructions:
                    "1. '-in' : Ang path patungo sa naka-encrypt na file. \n2. '-out' : Ang path kung saan ise-save ang na-decrypt na file. \n3. '-pass' : Ang password na ginamit para i-encrypt ang file. \n",
                encryptSuccess: "Matagumpay na na-setup ang pag-encrypt ng ulat",
                encryptError: "Error sa pag-setup ng pag-encrypt ng ulat",
            },
            reportType: {
                BALLOT_RECEIPT: "Resibo ng Balota",
                ELECTORAL_RESULTS: "Mga Resulta ng Eleksyon",
                MANUAL_VERIFICATION: "Manwal na Pag-verify",
                PARTICIPATION_REPORT: "Ulat ng Pakikilahok",
                STATISTICAL_REPORT: "Ulat ng Istatistika",
                OVCS_EVENTS: "Pagsubaybay ng Botohan sa Ibang Bansa - Mga Kaganapan ng OVCS",
                AUDIT_LOGS: "Mga Talaan ng Awdit",
                ACTIVITY_LOG: "Mga Talaan ng aktibidad",
                STATUS: "Katayuan",
                OVCS_INFORMATION: "Impormasyon ng OVCS",
                OVERSEAS_VOTERS: "Listahan ng mga botanteng nasa ibang bansa",
                OV_USERS_WHO_VOTED: "Listahan ng mga Botanteng Nasa Ibang Bansa na Bumoto",
                OV_WITH_VOTING_STATUS:
                    "Listahan ng mga Botanteng Nasa Ibang Bansa na may Katayuan sa Pagboto",
                OVCS_STATISTICS: "Pagsubaybay ng Botohan sa Ibang Bansa - Estadistika ng OVCS",
                PRE_ENROLLED_OV_BUT_DISAPPROVED:
                    "Listahan ng mga Botanteng Nasa Ibang Bansa na Naka-pre-enroll pero Hindi Naaprubahan",
                PRE_ENROLLED_OV_SUBJECT_TO_MANUAL_VALIDATION:
                    "Listahan ng mga Botanteng Nasa Ibang Bansa na Naka-pre-enroll pero Kailangan ng Manwal na Pagpapatunay",
            },
            reportEncryptionPolicy: {
                title: "Patakaran sa Encryption",
                UNENCRYPTED: "Hindi naka-encrypt",
                CONFIGURED_PASSWORD: "Nakakonfig na password",
            },
            empty: {
                header: "Wala pang mga ulat.",
                body: "Gusto mo bang lumikha ng isa?",
                button: "Lumikha ng Ulat",
            },
            create: {
                title: "Lumikha ng Ulat",
                subtitle: "Lumikha ng bagong konfigurasyon ng ulat.",
            },
            edit: {
                title: "I-edit ang Ulat",
                subtitle: "I-edit ang konfigurasyon ng ulat.",
                delete: "Sigurado ka bang nais mong tanggalin ang ulat na ito?",
            },
            fields: {
                electionId: "Halalan",
                template: "Template",
                reportType: "Uri ng Ulat",
                repeatable: "Nauulit",
                cronExpression: "Cron Expression",
                emailRecipients: "Mga Tatanggap ng Email",
                emailRecipientsPlaceholder: "I-type ang email at pindutin ang Enter",
            },
            delete: {
                body: "Sigurado ka bang nais mong tanggalin ang ulat na ito?",
            },
            actions: {
                generate: "Gumawa",
                delete: "Tanggalin",
                edit: "I-edit",
                preview: "I-preview",
            },
        },
        googleMeet: {
            title: "Bumuo ng Google Meet Link",
            generateButton: "Google Meet",
            meetingTitle: "Pamagat ng Meeting",
            description: "Paglalarawan (Opsyonal)",
            startDate: "Petsa ng Simula",
            startTime: "Oras ng Simula",
            duration: "Tagal (mga minuto)",
            attendeeEmails: "Mga Email ng mga Kalahok",
            attendeeEmailHelp: "Mga email na pinaghiwalay ng kuwit para sa mga kalahok sa meeting",
            note: "Paalala: Ito ay lilikha ng calendar event sa inyong Google Calendar na may Google Meet link. Kailangan ninyong mag-sign in sa inyong Google account.",
            success: "Matagumpay na Nabuo ang Google Meet Link!",
            copy: "Kopyahin sa clipboard",
            copied: "Nakopya na ang link sa clipboard!",
            instructions:
                "Ibahagi ang link na ito sa mga kalahok para sumali sa meeting. Naidagdag na ang calendar event sa inyong Google Calendar.",
            generating: "Bumubuo...",
            generate: "Bumuo ng Meet Link",
        },
        common: {
            export: "Maaaring magtagal ang pag-export. Sigurado ka bang nais mong i-export ang mga rekord?",
            resources: {
                electionEvent: "Kaganapan ng Halalan",
                election: "Halalan",
                contest: "Paligsahan",
                candidate: "Kandidato",
                noResult: {
                    askCreate: "Gusto mo bang lumikha ng isa?",
                },
            },
            label: {
                add: "Magdagdag",
                actions: "Mga Aksyon",
                create: "Lumikha",
                delete: "Tanggalin",
                archive: "I-archive",
                unarchive: "I-unarchive",
                cancel: "I-cancel",
                edit: "I-edit",
                yes: "Oo",
                no: "Hindi",
                save: "I-save",
                close: "Isara",
                back: "Bumalik",
                next: "Susunod",
                warning: "Babala",
                json: "Preview",
                noResult: "Walang resulta",
                import: "I-import",
                export: "I-export",
                loadingData: "Naga-load ng data...",
                exportFormat: "I-export sa format na {{format}} - Mga resulta ng '{{item}}",
                allResults: "kaganapan ng halalan",
                globalAreaResults: "lahat ng lugar",
                title: "Pamagat",
                subtitle: "Subtitle",
                kind: "Uri ng file",
                filter: "Pasadyang mga filter",
                approve: "Aprubahan",
                continue: "Magpatuloy",
                logout: "Mag-logout",
                selectTenant: "Pumili ng Tenant",
                processing: "Nagpro-process...",
                tenantName: "Pangalan ng Tenant",
            },
            language: {
                es: "Espanyol",
                en: "Ingles",
                fr: "Pranses",
                cat: "Valencian",
                tl: "Tagalog",
                gl: "Galego",
                nl: "Nederlands",
                eu: "Euskera",
            },
            channel: {
                online: "Online",
                kiosk: "Kiosk",
                early_voting: "Maagang pagboto",
                telephone: "Pagboto sa Telepono",
                other: "Iba pa",
            },
            message: {
                delete: "Sigurado ka bang gusto mong tanggalin ang item na ito?",
                continueOrLogout: "Gusto mo bang magpatuloy o mag-logout?",
            },
        },
        createResource: {
            electionEvent: "Lumikha ng Kaganapan ng Halalan",
            election: "Lumikha ng Halalan",
            contest: "Lumikha ng Paligsahan",
            candidate: "Lumikha ng Kandidato",
        },
        importResource: {
            electionEvent: "Mag-import ng Kaganapan ng Halalan",
            election: "Mag-import ng Halalan",
            contest: "Mag-import ng Paligsahan",
            candidate: "Mag-import ng Kandidato",
            ImportHashMismatch: "Hashes don't match. Integrity check failure.",
        },
        sideMenu: {
            electionEvents: "Mga Kaganapan ng Halalan",
            search: "Maghanap",
            usersAndRoles: "Mga Tagagamit at Tungkulin",
            logs: "Mga Log",
            settings: "Mga Setting",
            help: "Tulong",
            templates: "Templates",
            active: "Aktibo",
            archived: "Arkilado",
            addResource: {
                electionEvent: "Lumikha ng Kaganapan ng Halalan",
                election: "Lumikha ng Halalan",
                contest: "Lumikha ng Paligsahan",
                candidate: "Lumikha ng Kandidato",
            },
            menuActions: {
                archive: {
                    electionEvent: "I-archive ang Kaganapan ng Halalan",
                },
                unarchive: {
                    electionEvent: "I-unarchive ang Kaganapan ng Halalan",
                    election: "I-unarchive ang Halalan",
                    contest: "I-unarchive ang Paligsahan",
                    candidate: "I-unarchive ang Kandidato",
                },
                remove: {
                    electionEvent: "Tanggalin ang Kaganapan ng Halalan",
                    election: "Tanggalin ang Halalan",
                    contest: "Tanggalin ang Paligsahan",
                    candidate: "Tanggalin ang Kandidato",
                },
                messages: {
                    confirm: {
                        archive: "Sigurado ka bang i-archive ang item na ito?",
                        unarchive: "Sigurado ka bang i-unarchive ang item na ito?",
                        delete: "Sigurado ka bang tanggalin ang item na ito?",
                        sealsUnknown:
                            "Hindi masuri ang mga seal ng ballot box: kung may naka-seal itong mga ballot box, tatanggihan ang pagbura.",
                    },
                    notification: {
                        success: {
                            archive: "Ang item ay na-archive na",
                            unarchive: "Ang item ay na-unarchive na",
                            delete: "Ang item ay natanggal na",
                            reloading: "Wait. Ang pahina ay na-reload na sa iba pang mga oras.",
                        },
                        error: {
                            archive: "Error habang sinusubukang i-archive ang item na ito",
                            unarchive: "Error habang sinusubukang i-unarchive ang item na ito",
                            delete: "Error habang sinusubukang tanggalin ang item na ito",
                            deleteSealedElection:
                                "May naka-seal na mga ballot box ang halalang ito at hindi ito mabubura. I-archive na lang ang election event nito.",
                            deleteSealedEvent:
                                "May naka-seal na mga ballot box ang election event na ito at hindi ito mabubura. I-archive na lang ito.",
                            deleteMaybeSealed:
                                "Nagkaroon ng error sa pagbura ng item na ito. Kung may naka-seal itong mga ballot box, hindi ito mabubura.",
                        },
                    },
                },
            },
        },
        candidateScreen: {
            common: {
                subtitle: "Pag-configure ng Kandidato",
            },
            edit: {
                externalId: "Panlabas na ID",
                general: "Pangkalahatan",
                type: "Uri",
                image: "Larawan",
                isDisabled: "Hindi Aktibo",
                isExplicitInvalid: "Di-wastong Boto",
                isExplicitBlank: "Blangkong Boto",
                isCategoryList: "Listahan ng Kategorya",
                isWriteIn: "Write-in",
            },
            field: {
                name: "Pangalan",
                alias: "Alias",
                description: "Deskripsyon",
            },
            options: {
                "candidate": "Kandidato",
                "option": "Opsyon",
                "write-in": "Write-in",
                "open-list": "Bukas na Listahan",
                "closed-list": "Saradong Listahan",
                "semi-open-list": "Semi Bukas na Listahan",
                "invalid-vote": "Di-wastong Boto",
                "blank-vote": "Blangkong Boto",
            },
            invalidVotePosition: {
                label: "Posisyon ng Di-wastong Boto",
                null: "Wala (Default)",
                top: "Taas",
                bottom: "Baba",
            },
            error: {},
            createCandidateSuccess: "Kandidato ay nalikha",
            createCandidateError: "Error sa paglikha ng kandidato",
        },
        contestScreen: {
            common: {
                subtitle: "Pag-configure ng Paligsahan",
            },
            edit: {
                externalId: "Panlabas na ID",
                general: "Pangkalahatan",
                type: "Uri",
                image: "Larawan",
                system: "Sistema ng Pagboto sa Balota",
                design: "Disenyo ng Balota",
                reorder: "I-reorder ang mga kandidato",
                policies: "Mga Patakaran",
            },
            field: {
                name: "Pangalan",
                alias: "Alias",
                description: "Deskripsyon",
            },
            options: {
                "non-preferential": "Walang Preferensyal",
                "plurality-at-large": "Pluralidad sa Lahat",
                "instant-runoff": "Instant Runoff",
                "random": "Random",
                "external-procedure": "External Procedure",
                "custom": "Pasadya",
                "alphabetical": "Alpabetikal",
            },
            tieBreakingPolicy: {
                label: "Patakaran sa Tie-Breaking",
            },
            auditButtonConfig: {
                "label": "Mga Opsyon sa Pagpahiling kan Buton nin Pag-audit",
                "show": "Ipahiling",
                "not-show": "Dai Ipahiling",
                "show-in-help": "Ipahiling an Help Dialog",
            },
            underVotePolicy: {
                "label": "Sa irarom kan Patakaran sa Pagboto",
                "allowed": "Tinogotan",
                "warn-only-in-review": "Warn in Review",
                "warn": "Patanid",
                "warn-and-alert": "Patanid asin Alerto",
                "warn-and-confirm-in-review": "Patanid asin Kumpirmahon sa Review",
            },
            invalidVotePolicy: {
                "label": "Patakaran sa walang boto",
                "allowed": "Pinapayagan",
                "warn": "Magbigay ng Babala",
                "warn-invalid-implicit-and-explicit":
                    "Magbigay ng Babala sa Di-wastong Implicit at Explicit",
                "not-allowed": "Hindi Pinapayagan",
                "allowed-with-exclusive-explicit":
                    "Pinapayagan na may Eksklusibong Di-wastong Boto",
            },
            candidatesIconCheckboxPolicy: {
                "label": "An porma kan icon kan kahon kan mga kandidato",
                "square-checkbox": "Square Checkbox",
                "round-checkbox": "Bilog na Kahon nin Tsek",
            },
            checkableListPolicy: {
                "allow-selecting-candidates-and-lists": "Mga Kandidato at Listahan",
                "allow-selecting-candidates": "Mga Kandidato Lamang",
                "allow-selecting-lists": "Mga Listahan Lamang",
                "disabled": "Hindi Aktibo",
            },
            collapsibleListsPolicy: {
                "label": "Natatiklop na mga listahan",
                "disabled": "Hindi pinagana",
                "enabled-expanded": "Pinagana (nagsisimulang nakalawak)",
                "enabled-collapsed": "Pinagana (nagsisimulang nakatiklop)",
            },
            blankVotePolicy: {
                "label": "Patakaran sa walang boto",
                "allowed": "Pinapayagan",
                "warn-only-in-review": "Magbigay ng babala sa Pagsusuri",
                "warn": "Magbigay ng babala",
                "not-allowed": "Hindi pinapayagan",
            },
            overVotePolicy: {
                "label": "Patakaran sa Sobra sa Pagboto",
                "allowed": "Tinogotan",
                "allowed-with-msg": "Tinutugot sa Mensahe nin Patanid",
                "allowed-with-msg-and-alert": "Tinutugotan sa mensahe nin Patanid asin Alerto",
                "not-allowed-with-msg-and-alert":
                    "Dai Tinotogotan an mensahe nin Patanid asin Alerto",
                "not-allowed-with-msg-and-disable":
                    "Dai Tinutugutan an mensahe nin Patanid asin I-disable an mga dugang pang pagpili",
            },
            duplicatedRankPolicy: {
                "label": "Di-wastong Boto - Patakaran sa Dobleng Ranggo",
                "allowed-warn-and-dialog":
                    "Ipakita ang babala at dialog (maaaring magpatuloy ang botante)",
                "not-allowed-warn-and-dialog":
                    "Ipakita ang babala at dialog (hindi maaaring magpatuloy ang botante)",
            },
            preferenceGapsPolicy: {
                "label": "Di-wastong Boto - Patakaran sa Nilaktawang Ranggo",
                "allowed-warn-and-dialog":
                    "Ipakita ang babala at dialog (maaaring magpatuloy ang botante)",
                "not-allowed-warn-and-dialog":
                    "Ipakita ang babala at dialog (hindi maaaring magpatuloy ang botante)",
            },
            paginationPolicy: {
                label: "Pangalan ng Pahina",
            },
            isAcclaimed: {
                label: "Napagpasyahan sa pamamagitan ng aklamasyon",
                helperText:
                    "Nakikita ng mga botante ang paligsahang ito ngunit walang mapipili, walang naitatala, at lahat ng kandidato ay iniuulat na nanalo na may zero na boto. Itakda ito bago ilathala ang mga balota: ang pagbabago pagkatapos ay magpapawalang-bisa sa mga balotang naisumite na.",
            },
            allowWriteins: {
                label: "Payagan ang Mga Manu-manong Kandidato",
            },
            maxVotes: {
                helperText:
                    "Pinakamataas na bilang ng mga kandidatong maaaring piliin ng botante (hindi-kagustuhang pagboto).",
                helperTextPreferential:
                    "Pinakamataas na posisyon ng ranggo na magagamit (hal. '5' ay nagbibigay ng mga posisyon 1–5). Dapat na hindi bababa sa bilang ng mga kandidatong irurango (kagustuhang pagboto).",
            },
            error: {},
            createContestSuccess: "Paligsahan ay nalikha",
            createContestError: "Error sa paglikha ng paligsahan",
        },
        keysGeneration: {
            configureStep: {
                create: "Lumikha ng Seremonya ng Mga Susi",
                name: "Pangalan ng Seremonya ng mga Susi",
                allElections: "Lahat ng Halalan",
                title: "Lumikha ng Seremonya ng Mga Susi para sa Kaganapan sa Halalan",
                subtitle:
                    "Sa Seremonya ng Mga Susi, bawat trustee ay lilikha at magda-download ng kanilang piraso ng pribadong susi para sa Kaganapan sa Halalan. Upang magpatuloy, mangyaring pumili ng mga trustee na lalahok sa seremonya at ang threshold, na siyang minimum na bilang ng mga trustee na kinakailangan upang magbilang.",
                threshold: "Threshold",
                trusteeList: "Mga Trustee",
                errorMinTrustees_one:
                    "Pumili ka lamang ng {{selected}} trustee, ngunit kailangan mong pumili ng hindi bababa sa {{threshold}}.",
                errorMinTrustees_other:
                    "Pumili ka lamang ng {{selected}} mga trustee, ngunit kailangan mong pumili ng hindi bababa sa {{threshold}}.",
                errorThreshold:
                    "Pumili ka ng threshold {{selected}} ngunit ito ay dapat na nasa pagitan ng {{min}} at {{max}}.",
                errorCreatingCeremony: "Error sa paglikha ng Seremonya ng Mga Susi: {{error}}",
                createCeremonySuccess: "Seremonya ng Mga Susi ay nalikha",
                confirmdDialog: {
                    ok: "Oo, Lumikha ng Seremonya ng Mga Susi",
                    cancel: "Kanselahin",
                    title: "Sigurado ka bang nais mong lumikha ng Seremonya ng Mga Susi?",
                    automaticCeremonyTitle:
                        "Sigurado ka bang gusto mong lumikha ng isang Awtomatikong Seremonya ng mga Susi?",
                    description:
                        "Ikaw ay malapit nang lumikha ng Seremonya ng Mga Susi. Ang aksyong ito ay magpapadala ng abiso sa mga Trustee upang lumahok sa paglikha at pamamahagi ng Mga Susi ng Kaganapan sa Halalan.",
                    automaticCeremonyDescription:
                        "Ikaw ay lilikha ng isang Awtomatikong Seremonya ng mga Susi. Hindi nito aabisuhan ang mga Trustee na lumahok.",
                },
                filterTrustees: "I-filter ang mga trustee",
                errorPermisionLabels:
                    "Hindi maipagawa ang Key Ceremony: may nawawala sa isa o higit pang permission label.",
                automaticCeremonyToggle: "Awtomatikong Seremonya",
            },
            ceremonyStep: {
                cancel: "Kanselahin ang Seremonya ng Mga Susi",
                progressHeader: "Pag-andar ng Seremonya ng Mga Susi",
                description:
                    "Ipinapakita ng screen na ito ang pag-andar at mga tala ng Seremonya ng Mga Susi ng Kaganapan sa Halalan. Sa Seremonya ng Mga Susi, bawat trustee ay lilikha at magda-download ng kanilang piraso ng pribadong susi para sa Kaganapan sa Halalan.",
                executionStatus: "Katayuan: {{status}}",
                confirmdDialog: {
                    ok: "Oo, Kanselahin ang Paglikha ng Seremonya ng Mga Susi",
                    cancel: "Bumalik sa Seremonya ng Mga Susi",
                    title: "Sigurado ka bang nais mong kanselahin ang Seremonya ng Mga Susi?",
                    description:
                        "Malapit mo nang ma-kansela ang Seremonya ng Mga Susi. Pagkatapos isagawa ang aksyong ito, upang magkaroon ng matagumpay na Seremonya ng Mga Susi, kailangan mong lumikha ng bago.",
                },
                header: {
                    trusteeName: "Pangalan ng Trustee",
                    fragment: "Nalikha ang Piraso ng Susi",
                    downloaded: "Nai-download na Piraso ng Pribadong Susi",
                    checked: "Nasuri ang Piraso ng Pribadong Susi",
                },
                logsHeader: {
                    title: "Mga Tala",
                    date: "Petsa",
                    entry: "Entry",
                },
                emptyLogs: "Wala pang tala.",
            },
            startStep: {
                title: "Seremonya ng Mga Susi ng Trustee",
                subtitle:
                    "Ikaw ay malapit nang lumahok sa Seremonya ng Mga Susi bilang isang Trustee (<strong>{{name}}</strong>). Kasama sa mga hakbang ang mga sumusunod:",
                one: "<strong>I-download</strong> ang iyong Encrypted Private Key.",
                two: "Lumikha ng maraming <strong>Backup</strong> ng Encrypted Private Key.",
                three: "<strong>Surii</strong> na ang mga backup ay maayos na gumagana.",
            },
            downloadStep: {
                title: "I-download ang Encrypted Private Key",
                subtitle:
                    "Upang magpatuloy, mangyaring i-download at itago ang iyong Encrypted Private Key sa hindi bababa sa dalawang magkaibang device:",
                downloadButton: "I-download ang iyong Encrypted Private Key",
                downloaded: "Matagumpay na na-download ang Encrypted Private Key.",
                errorEmptyKey: "Error sa pag-download, walang laman na file",
                unexpectedError: "Hindi ma-download ang pribadong key. Pakisubukang muli.",
                alreadyVerified: "Na-download at na-verify na ang iyong pribadong key.",
                unavailable:
                    "Hindi na maaaring i-download ang pribadong key dahil nagpatuloy na ang seremonya.",
                confirmdDialog: {
                    ok: "Kumpirmahin ang mga Backup at Magpatuloy",
                    cancel: "Bumalik",
                    title: "I-backup ang iyong Encrypted Private Key",
                    description:
                        "Mangyaring i-backup ang iyong Encrypted Private Key sa hindi bababa sa dalawang magkaibang ligtas na lokasyon at pagkatapos ay kumpirmahin ito sa ibaba:",
                    firstCopy: "Unang backup ay ligtas",
                    secondCopy: "Ikalawang backup ay ligtas",
                    confirmError:
                        "Gumawa ng mga kinakailangang backup at lagyan ng tsek ang mga kahon ng kumpirmasyon upang magpatuloy",
                },
            },
            checkStep: {
                title: "Suriin ang Iyong Encrypted Private Key Backups",
                verifyButton: "Beripikahin ang key",
                subtitle:
                    "I-upload ang isang Backup ng Encrypted Private Key upang suriin kung ito ay tama. Maaari mong subukan ng maraming beses hangga't kinakailangan, mula sa iyong iba't ibang backups:",
                errorUploading: "Di-wastong Encrypted Private Key Backup, mangyaring subukan muli",
                errorEmptyFile: "Walang laman na file o hindi natagpuan",
                verified: "Backup ay matagumpay na nasuri.",
            },
        },
        miruExport: {
            create: {
                success: "Nilikha ang Transmission Package...",
                error: "Error sa paglikha ng Transmission Package",
            },
            send: {
                success: "Ipinapadala ang Transmission Package...",
                error: "Error sa pagpapadala ng Transmission Package",
            },
        },
        tally: {
            errorUploadingSignature: "Nagkaroon ng error sa pag-upload ng pirma",
            downloadTransmissionPackage: "I-download ang pakete",
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
                title: "Pakete ng Transmisyon para sa Lugar '{{name}}' at Halalan na '{{eventName}}'",
                description:
                    "Hinahayaan kang i-export ang isang Pakete ng Transmisyon sa mga Server ng Destinasyon o i-download ito.",
                actions: {
                    sign: {
                        title: "I-regenerate",
                        dialog: {
                            title: "Nais mo bang pirmahan ang pakete ng transmisyon?",
                            description:
                                "Pakisiguro na nais mong i-regenerate ang pakete ng transmisyon para sa lugar na `{{name}}`",
                            confirm: "Pirmahan ang pakete ng transmisyon",
                            cancel: "Isara",
                            input: {
                                placeholder: "Ilagay ang iyong password",
                            },
                        },
                    },
                    send: {
                        title: "Ipadala",
                        dialog: {
                            title: "Gusto mo bang ipadala ang Pakete ng Transmisyon?",
                            description:
                                "Pakisiguro na nais mong ipadala ang Pakete ng Transmisyon para sa Lugar '{{name}}' sa mga Server ng Destinasyon.",
                            confirm: "Ipadala ang Pakete ng Transmisyon",
                            cancel: "Isara",
                        },

                        disabled:
                            "Kulang ang mga kinakailangang lagda o naipadala na ang transmission package sa lahat ng destinasyon.",
                    },
                    regenerate: {
                        title: "I-regenerate",
                        dialog: {
                            title: "Nais mo bang i-regenerate ang pakete ng transmisyon?",
                            description:
                                "Pakisiguro na nais mong i-regenerate ang pakete ng transmisyon para sa lugar na `{{name}}`",
                            confirm: "I-regenerate ang pakete ng transmisyon",
                            cancel: "Isara",
                        },
                    },
                    download: {
                        title: "I-download",
                        emlTitle: "I-download ang EML {{date}}",
                        transmissionPackageTitle: "I-download ang Pakete ng Transmisyon {{date}}",
                        transmissionReportTitle: "I-download ang Ulat ng Transmisyon",
                        dialog: {
                            title: "Gusto mo bang i-download ang Pakete ng Transmisyon?",
                            description:
                                "Pakisiguro na nais mong i-download ang Pakete ng Transmisyon para sa Lugar '{{name}}.'",
                            confirm: "I-download ang Pakete ng Transmisyon",
                            cancel: "Isara",
                        },
                    },
                },
                destinationServers: {
                    title: "Mga Server ng Destinasyon",
                    description:
                        "Ipinapakita ng talahanayan sa ibaba ang katayuan ng pagpapadala sa bawat isa sa mga Server ng Destinasyon.",
                    status: "Naipadala sa {{signed}} ng {{total}}",
                    table: {
                        serverName: "Pangalan ng Server",
                        sendStatus: "Katayuan ng Pagpapadala",
                    },
                },
                signatures: {
                    title: "Mga lagda",
                    description:
                        "Maaaring lagdaan ng mga miyembro ang transmission package. Ipinapakita ng talahanayan ang katayuan ng paglagda ng bawat miyembro.",
                    table: {
                        trusteeName: "Miyembro",
                        signed: "Napirmahan",
                    },
                    status: "{{signed}} sa {{total}} Napirmahan",
                },
            },
            sendToTransmissionPackageServers:
                "Ipadala ang pakete ng transmisyon para sa lugar '{{name}}'",
            uploadTransmissionPackage: "I-upload",
            uploadTransmissionPackageDesc:
                "I-upload ang iyong pirma para pirmahan ang pakete ng Resulta ng Halalan. Ang operasyong ito ay opsyonal.",
            exportElectionArea: "Ipadala ang pakete ng transmisyon para sa lugar '{{name}}'",
            generateReport: "Bumuo ng {{name}}",
            templateTitle: "Template ng Resulta",
            templateSubTitle: "Opsyonal na palitan ang template ng mga resulta.",
            keysCeremonyTitle: "Seremonya ng mga Susi",
            keysCeremonySubTitle: "Piliin ang Seremonya ng mga Susi para sa pagbilang na ito",
            ceremonyTitle: "Halalan para sa Pagbibilang",
            initializationTitle: "Halalan para sa Ulat ng Inisyal na Pagsisimula",
            ceremonySubTitle: "Piliin ang halalan para sa pagbibilang",
            tallyTitle: "Progreso ng Pagbibilang ng Halalan",
            logsTitle: "Mga Logs",
            resultsTitle: "Mga Resulta at Pakikilahok",
            generalInfoTitle: "Pangkalahatang Impormasyon",
            trusteeTallyTitle: "Tagapagtiwala",
            trusteeTallySubTitle: "Katayuan ng pag-import ng fragment ng key",
            ballotBoxes: {
                unavailable: "Hindi makuha ang mga seal",
                sealed: "{{sealed}} sa {{total}} ang naka-seal",
                publishing: "Naka-seal, {{published}} sa {{total}} ang nasa bulletin board",
                sealing: "Ise-seal sa {{time}}",
                notSealed: "Hindi naka-seal",
                help: "Maaaring i-tally ang isang halalan kapag naka-seal na ang bawat ballot box nito at nasa bulletin board na ang seal nito.",
                failed: "Hindi naka-seal: insidente",
                overdue: "Lampas na sa oras ng pag-seal",
                blocked: "{{name}}: {{reason}}",
                reason: {
                    "not-sealed":
                        "hindi pa nagsasara ang botohan, kaya wala pang seal ang mga ballot box nito",
                    "sealing": "sine-seal ang mga ballot box nito kapag natapos ang grace period",
                    "overdue":
                        "lampas na sa oras ang mga ballot box nito at hindi pa naka-seal (tingnan ang Dashboard nito)",
                    "publishing": "ipinapaskil pa sa bulletin board ang ilan sa mga seal nito",
                    "failed":
                        "may ballot box na hindi na-seal, isang insidente (tingnan ang Dashboard nito)",
                    "unavailable": "hindi mabasa ang mga seal ng mga ballot box nito",
                },
            },
            eligibility: {
                ballotBoxesUnavailable:
                    "Hindi mabasa ang mga seal ng mga ballot box ng isang napiling halalan, kaya hindi pa ito maaaring i-tally. I-reload ang pahina para subukang muli.",
                selectElection: "Pumili ng kahit isang halalan.",
                publishElection:
                    "I-publish ang bawat napiling halalan bago gumawa ng pagbibilang nito.",
                tallyDisallowed:
                    "Hindi pinapayagan ang pagbibilang para sa isang napiling halalan.",
                endVoting:
                    "Tapusin ang pagboto sa bawat napiling halalan at ihinto ang mga aktibong channel bago gumawa ng pagbibilang.",
                sealBallotBoxes:
                    "Maaaring i-tally ang isang halalan kapag naka-seal na ang bawat ballot box nito at nasa bulletin board na ang seal nito.",
            },
            createTallySuccess: "Pagbibilang na ginawa",
            createTallyError: "Error sa paggawa ng pagbibilang",
            startTallySuccess: "Nagsimula ang pagbibilang",
            startTallyError: "Error sa pagsisimula ng pagbibilang",
            startTallyCeremonySuccess: "Nagsimula ang seremonya ng pagbibilang",
            startTallyCeremonyError: "Hindi nasimulan ang seremonya ng pagbibilang",
            cancelTallyCeremonySuccess: "Nakansela ang seremonya ng pagbibilang",
            cancelTallyCeremonyError: "Hindi nakansela ang seremonya ng pagbibilang",
            recountTallyCeremony: "Muling pagbibilang",
            recountTallyCeremonyMessage:
                "Gagawa ito ng bagong resulta para sa natapos na sesyon ng pagbibilang.",
            recountTallyCeremonyStarting: "Sinisimulan ang muling pagbibilang...",
            recountTallyCeremonySuccess: "Nagsimula ang muling pagbibilang",
            recountTallyCeremonyError: "Hindi nasimulan ang muling pagbibilang",
            recountTallyCeremonyOk: "Muling bilangin",
            trusteeTitle: "Proseso ng tagapagtiwala",
            trusteeSubTitle: "Pakilagay ang iyong fragment ng key",
            invited: "Naanyayahan kang lumahok sa isang seremonya ng pagbibilang. Pakisuyo, ",
            click: "i-click ang aksyon ng pagbibilang",
            participate: "para lumahok.",
            breadcrumbSteps: {
                start: "Simula",
                finish: "Tapos",
                tally: "Pagbibilang",
                results: "Mga Resulta",
                ceremony: "Seremonya",
            },
            common: {
                title: "Pagbibilang",
                subTitle: "Pag-setup ng Pagbibilang.",
                cancel: "Bumalik",
                next: "Susunod",
                date: "Petsa ng Pagbibilang",
                global: "Global",
                noTrustees: "Walang mga tagapagtiwala pa",
                imported: " tagapagtiwala ang nag-import ng kanilang fragment ng key",
                needed: " tagapagtiwala ang kailangan para sa pagbibilang",
                start: "Simulan ang Pagbibilang",
                ceremony: "Simulan ang Seremonya ng Pagbibilang",
                initialization: "Simulan ang Ulat ng Inisyal na Pagsisimula",
                results: "Mga Resulta",
                dialog: {
                    ok: "Ok",
                    okTally: "Simulan ang pagbibilang",
                    okCancel: "Kanselahin ang pagbibilang",
                    cancel: "Isara",
                    title: "Sigurado ka bang gusto mong simulan ang isang seremonya?",
                    tallyTitle: "Sigurado ka bang gusto mong simulan ang pagbibilang?",
                    cancelTitle: "Sigurado ka bang gusto mong kanselahin ang pagbibilang?",
                    message:
                        "Malapit ka nang magsimula ng isang seremonya ng pagbibilang. Aabisuhan nito ang mga tagapagtiwala para i-import ang kanilang mga fragment ng key.",
                    cancelMessage:
                        "Malapit ka nang kanselahin ang seremonya ng pagbibilang. Hindi na mababawi ang aksyong ito.",
                    ceremony:
                        "Na-verify na ng lahat ng kinakailangang tagapagtiwala ang kanilang mga fragment ng key. Handa na ang lahat para simulan ang pagtanggap ng mga resulta. Nais mo bang simulan ang Pagbibilang?",
                    startAutomatedTallyMessage:
                        "Piliin ang 'Start Tally' upang patakbuhin ang proseso ng pagbilang at ipakita ang mga resulta, o 'Close' upang kanselahin.",
                },
            },
            table: {
                ballotBoxes: "Mga ballot box",
                elections: "Halalan",
                selected: "Napili",
                status: "Katayuan",
                progress: "Progreso",
                method: "Pamamaraan ng Pagbibilang",
                elegible: "Mga Karapat-dapat na Botante",
                number: "Bilang ng mga Boto",
                total: "Kabuuan",
                turnout: "%",
                candidates: "Mga Resulta ng Kandidato",
                options: "Mga Opsyon",
                global: "Buod ng pakikilahok",
                elegible_census: "Senso ng mga karapat-dapat na botante",
                cast_votes: "Bilang ng mga Boto",
                cast_votes_percent: "Porsyento ng mga Boto",
                total_votes: "Kabuuang mga botante",
                total_votes_percent: "Pakikilahok",
                total_votes_counted: "Kabuuang mga Botong Nabilang",
                total_auditable_votes: "Kabuuang Mga Botong Maaaring Ma-audit",
                total_valid_votes: "Kabuuang mga balidong boto",
                total_valid_votes_percent: "Porsyento ng balidong boto",
                total_invalid_votes: "Kabuuang mga di-balidong boto",
                total_invalid_votes_percent: "Porsyento ng mga di-balidong boto",
                explicit_invalid_votes: "Mga hayag na di-balidong boto",
                explicit_invalid_votes_percent: "Porsyento ng mga hayag na di-balidong boto",
                implicit_invalid_votes: "Mga nakatagong di-balidong boto",
                implicit_invalid_votes_percent: "Porsyento ng mga nakatagong di-balidong boto",
                blank_votes: "Mga botong walang laman",
                explicit_blank_votes: "Mga hayag na blankong boto",
                implicit_blank_votes: "Mga nakatagong blankong boto",
                blank_votes_percent: "Porsyento ng mga botong walang laman",
                number_of_votes: "Bilang ng mga boto",
                winning_position: "Panalong posisyon",
                weight: "Timbang",
                preferential: {
                    candidate: "Kandidato",
                    winner: "Nanalo",
                    eliminated: "Naalis",
                    round: "Ikot",
                },
                total_declined_to_vote: "Kabuuang Tumangging Bumoto",
                total_blank_ballots: "Kabuuang Blangkong Balota",
                participation_by_channel: "Paglahok ayon sa channel",
                channel: "Channel",
                channel_online: "Online",
                channel_kiosk: "Kiosk",
                channel_early_voting: "Maagang pagboto",
                channel_telephone: "Telepono",
                channel_paper: "Papel",
                channel_postal: "Koreo",
                channel_in_person: "Personal",
                acclamation_note:
                    "Nanalo sa pamamagitan ng aklamasyon. Ang paligsahang ito ay napagpasyahan nang walang botohan, kaya walang naitalang boto.",
            },
            pendingResolutions: {
                round: "Ikot {{round}}",
                tieResolutionRequired: "Kinakailangan ang resolusyon sa ugnayan",
                tieResolved: "Naresolba ang ugnayan",
                globalArea: "Global",
                pendingResolutionsHeader: "Mga nakabinbing resolusyon",
                pendingResolutionStatus: "Nakabinbing resolusyon",
                resolvedStatus: "Naresolba",
                resolutionTitle: "Resolusyon",
                selectContest: "Pumili ng aytem sa kaliwa upang tingnan ang mga detalye",
                selectCandidateToAdvance: "Piliin ang kandidatong itataguyod",
                undoResolution: "I-undo ang resolusyon",
                applyResolutions: "Ilapat ang mga resolusyon at muling kalkulahin",
                submitSuccess: "Mga resolusyon isinumite. Ang pagbibilang ay nagpapatuloy...",
                submitError: "Nabigo ang pag-isumite ng mga resolusyon. Pakisubukan muli.",
                filter: "I-filter",
                save: "I-save",
                pendingApplyStatus: "Nakabinbing kalkulasyon",
                filterElection: "Eleksyon",
                filterContest: "Paligsahan",
                filterArea: "Lugar",
                filterStatusLabel: "Katayuan",
                clearFilters: "I-clear ang mga filter",
                candidateWithVotes: "{{name}} ({{votes}} boto)",
                candidateWithVotesAndPercent: "{{name}} ({{votes}} boto, {{percent}}%)",
                tieInfoTitle:
                    "Ang pagbibilang ay natigil dahil sa hindi naresolubasiyong ugnayan (Ikot {{round}})",
                tieInfoBody:
                    "Mga kandidatong nagtatali ({{votes}} boto, {{percent}}%): {{candidates}}. Kinakailangan ang manu-manong pagpili para magpatuloy ang pagbibilang.",
                tallyResumedTitle:
                    "Nagpapatuloy na ang pagbibilang pagkatapos ilapat ang resolusyon",
                tallyResumedBody: "Ang ugnayan ay nalutas noong {{date}} ni {{user}}",
            },
            chart: {
                votesForCandidates: "Mga Boto para sa mga Kandidato",
                blankVotes: "Mga Blankong Boto",
                invalidVotes: "Mga Hindi Wastong Boto",
                totalVoters: "Kabuuang mga Botante",
                nonVoters: "Hindi Bumoto",
            },
            exportAllAreas:
                "I-export ang resulta ng lahat ng lugar sa format na {{format}} para kay '{{item}}'",
        },
        publish: {
            initialization: {
                countryInfo:
                    "Bumuo ng ulat para sa buong Post o isang bansa. Mananatiling naka-block ang pagboto hanggang makumpleto ang lahat ng kinakailangang inisyalisasyon para sa mga bansa at sa buong kaganapan.",
                countriesError:
                    "Hindi ma-load ang mga kwalipikadong bansa. Isara at subukang muli.",
                noCountries:
                    "Walang kwalipikadong bansa na may aktibong mga estilo ng balota ang Post na ito. Suriin ang mga lugar at publikasyon nito bago mag-inisyalisa.",
                country: "Bansa",
                entirePost: "Buong Post",
            },
            preview: {
                publicationAreas: "Piliin ang Lugar para sa Preview",
                action: "Preview",
                copy: "Kopyahin ang link",
                copy_success: "Tagumpay sa pagkopya ng link ng preview",
                copy_error: "Nabigong kopyahin ang link ng preview",
                success: "Matagumpay na pagbukas ng preview",
            },
            header: {
                change: "Mga Pagbabago na Ilalathala",
                viewChange: "Tingnan ang Paglalathala",
                history: "Kasaysayan ng Paglalathala",
            },
            action: {
                generateInitializationReport: "Gumawa ng Ulat sa Inisyal na Pagsisimula",
                startVotingPeriod: "Simulan ang Pagboto",
                startKioskVoting: "Simulan ang Pagboto sa Kiosk",
                startOnlineVoting: "Simulan ang Pagboto Online",
                startEarlyVoting: "Simulan ang Maagang Pagboto",
                startTelephoneVoting: "Simulan ang Telepono Pagboto",
                stopVotingPeriod: "Itigil ang Pagboto",
                stopOnlineVoting: "Itigil ang Pagboto Online",
                stopEarlyVoting: "Itigil ang Maagang Pagboto",
                stopTelephoneVoting: "Itigil ang Telepono Pagboto",
                stopKioskVotingPeriod: "Itigil ang Pagboto sa Kiosk",
                pauseVotingPeriod: "I-pause ang Pagboto",
                pauseKioskVoting: "I-pause ang Pagboto sa Kiosk",
                pauseOnlineVoting: "I-pause ang Pagboto Online",
                pauseEarlyVoting: "I-pause ang Maagang Pagboto",
                pauseTelephoneVoting: "I-pause ang Telepono Pagboto",
                generate: "Muling Lumikha",
                publish: "Ilathala ang Mga Pagbabago",
                back: "Bumalik",
            },
            label: {
                current: "Kasalukuyan",
                previous: "Nakaraang Paglalathala",
                publication: "Paglalathala",
                diff: "Mga Pagbabago na Ilalathala",
            },
            empty: {
                header: "Walang Paglalathala Pa.",
                action: "Lumikha ng Paglalathala",
            },
            forbidden: {
                header: "Hindi mai-publish hanggang sa makumpleto ang Key Ceremony.",
            },
            skippedElections: {
                dismiss: "Isara",
                title: "May mga halalang nananatiling sarado",
                ballotBoxSealPolicy:
                    "Nananatiling sarado ang {{name}}: nagsara na ang botohan nito at pinal ang pagsasara sa I-seal sa pagsasara.",
                other: "Hindi binago ang {{name}} ({{reason}}).",
            },
            dialog: {
                title: "Kumpirmahin ang Aksyon",
                info: "Ikaw ay nag-click sa isang sensitibong aksyon, kaya't kailangan namin ng iyong kumpirmasyon upang magpatuloy",
                initializationInfo:
                    "Malapit ka nang bumuo ng ulat ng inisyalisasyon. Sigurado ka bang gusto mong magpatuloy?",
                startInfo:
                    "Malapit mo nang simulan ang panahon ng pagboto. Sigurado ka bang nais mong magpatuloy?",
                stopInfo:
                    "Malapit mo nang itigil ang panahon ng pagboto. Sigurado ka bang nais mong magpatuloy?",
                stopSeal:
                    "Ihihinto mo na ang botohan sa {{name}}. Pagkatapos ay sine-seal ang mga ballot box nito: wala nang balotang maidadagdag, mababago o mabubura, at hindi na muling masisimulan ang botohan. Sigurado ka bang gusto mong magpatuloy?",
                stopSealEvent:
                    "Ihihinto mo na ang botohan sa lahat ng halalan. Pagkatapos ay sine-seal ang kanilang mga ballot box: wala nang balotang maidadagdag, mababago o mabubura, at hindi na muling masisimulan ang botohan. Sigurado ka bang gusto mong magpatuloy?",
                startSealNote:
                    "Sa I-seal sa pagsasara, nananatiling sarado ang botohang naisara na: hindi magbubukas ang mga halalang sarado na ang botohan.",
                channel: {
                    ONLINE: "Online",
                    KIOSK: "Kiosk",
                    EARLY_VOTING: "Maagang pagboto",
                    TELEPHONE: "Telepono",
                },
                kioskStopInfo:
                    "Malapit mo nang itigil ang panahon ng pagboto sa kiosk. Sigurado ka bang gusto mong magpatuloy?",
                pauseInfo:
                    "Malapit mo nang i-pause ang panahon ng pagboto. Sigurado ka bang nais mong magpatuloy?",
                publishInfo:
                    "Malapit ka nang gumawa ng publikasyon. Sigurado ka bang gusto mong magpatuloy?",
                ok: "Kumpirmahin",
                ko: "Kanselahin",
                error: "Error sa pag-load ng paglalathala ng balota",
                error_publish: "Error sa paglalathala ng balota",
                error_capacity: "Nabigo ang paggawa ng estilo ng balota: {{message}}",
                error_status: "Error sa pagbabago ng katayuan ng paglalathala ng balota",
                error_preview: "Error sa pag-preview ng publikasyon",
                diff: "Ang pag-render ng lahat ng mga pagbabago ay maaaring magdulot ng pagka-antala sa pahina. Sigurado ka bang nais mong magpatuloy?",
                confirmation:
                    "Ang aksyong gagawin mo ay sensitibo at nangangailangan ng kumpirmasyon. Paki-enter ang iyong password para magpatuloy sa {{action}}.",
                stopSealNotYet:
                    "Ihihinto mo ang panahon ng pagboto. {{holding}} Sigurado ka bang gusto mong magpatuloy?",
                sealHolding_one:
                    "Sa I-seal sa pagsasara, sine-seal ang mga ballot box nito kapag sarado na ang bawat naka-enable na channel: naka-enable pa at hindi sarado ang {{channels}}.",
                sealHolding_other:
                    "Sa I-seal sa pagsasara, sine-seal ang mga ballot box nito kapag sarado na ang bawat naka-enable na channel: naka-enable pa at hindi sarado ang {{channels}}.",
                sealNotEnabled:
                    "Hindi sarado ang {{channel}} at hindi ito naka-enable para sa Post na ito: ihinto ito para ma-seal ang mga ballot box.",
                sealNotEnabledPost:
                    "Sa {{post}}, hindi sarado ang {{channel}} at hindi ito naka-enable: ihinto ito sa Post na iyon para ma-seal ang mga ballot box nito.",
                sealNoChannel:
                    "Walang naka-enable na channel para sa Post na ito at walang nagbukas, kaya hindi sine-seal ang mga ballot box nito.",
                stopNeverOpened_one:
                    "Hindi kailanman nabuksan ang {{channels}}: kapag inihinto ito, hindi na ito magbubukas.",
                stopNeverOpened_other:
                    "Hindi kailanman nabuksan ang {{channels}}: kapag inihinto ang mga ito, hindi na magbubukas ang mga ito.",
                stopSealGrace_one:
                    "Ihihinto mo na ang botohan sa {{name}}. Sine-seal ang mga ballot box nito pagkatapos ng grace period, {{count}} minuto mamaya: mula noon, wala nang balotang maidadagdag, mababago o mabubura. Hindi na muling masisimulan ang botohan. Sigurado ka bang gusto mong magpatuloy?",
                stopSealGrace_other:
                    "Ihihinto mo na ang botohan sa {{name}}. Sine-seal ang mga ballot box nito pagkatapos ng grace period, {{count}} minuto mamaya: mula noon, wala nang balotang maidadagdag, mababago o mabubura. Hindi na muling masisimulan ang botohan. Sigurado ka bang gusto mong magpatuloy?",
                stopSealEventGrace_one:
                    "Ihihinto mo na ang botohan sa lahat ng halalan. Sine-seal ang kanilang mga ballot box kapag natapos ang grace period ng bawat halalan, hanggang {{count}} minuto mamaya: mula noon, wala nang balotang maidadagdag, mababago o mabubura. Hindi na muling masisimulan ang botohan. Sigurado ka bang gusto mong magpatuloy?",
                stopSealEventGrace_other:
                    "Ihihinto mo na ang botohan sa lahat ng halalan. Sine-seal ang kanilang mga ballot box kapag natapos ang grace period ng bawat halalan, hanggang {{count}} minuto mamaya: mula noon, wala nang balotang maidadagdag, mababago o mabubura. Hindi na muling masisimulan ang botohan. Sigurado ka bang gusto mong magpatuloy?",
                stopSealEventSome:
                    "Ihihinto mo na ang botohan sa lahat ng halalan. {{sealed}} {{holding}} Sigurado ka bang gusto mong magpatuloy?",
                sealedNowPart:
                    "Pagkatapos ay sine-seal ang mga ballot box ng {{names}}: wala nang balotang maidadagdag, mababago o mabubura, at hindi na muling masisimulan doon ang botohan.",
                sealedGracePart_one:
                    "Sine-seal ang mga ballot box ng {{names}} kapag natapos ang kanilang grace period, hanggang {{count}} minuto mamaya.",
                sealedGracePart_other:
                    "Sine-seal ang mga ballot box ng {{names}} kapag natapos ang kanilang grace period, hanggang {{count}} minuto mamaya.",
                holdingEventPart_one:
                    "May isa pang naka-enable at hindi saradong channel ang {{names}}: sine-seal ang mga ballot box nito kapag naisara ang channel na iyon.",
                holdingEventPart_other:
                    "May isa pang naka-enable at hindi saradong channel ang {{names}}: sine-seal ang kanilang mga ballot box kapag naisara ang mga channel na iyon.",
                noChannelEventPart_one:
                    "Walang naka-enable na channel ang {{names}} at walang nagbukas doon: hindi sine-seal ang mga ballot box nito.",
                noChannelEventPart_other:
                    "Walang naka-enable na channel ang {{names}} at walang nagbukas doon: hindi sine-seal ang kanilang mga ballot box.",
                startSealNoteList:
                    "Sa I-seal sa pagsasara, nananatiling sarado ang botohang naisara na: {{items}}.",
                startKeptChannels_one: "{{post}}: nananatiling sarado ang {{channels}}",
                startKeptChannels_other: "{{post}}: nananatiling sarado ang {{channels}}",
                startKeptSealed:
                    "nananatiling sarado ang {{post}}, dahil naka-seal na ang mga ballot box nito",
                startNotEnabledList:
                    "Sa I-seal sa pagsasara, binubuksan ng Start ang isang channel sa mga Post lamang na nag-enable nito: {{items}}.",
                startNotEnabledChannels_one:
                    "{{post}}: hindi naka-enable doon ang {{channels}} at nananatiling Hindi pa nagsisimula",
                startNotEnabledChannels_other:
                    "{{post}}: hindi naka-enable doon ang {{channels}} at nananatiling Hindi pa nagsisimula",
                stopNeverOpenedPosts_one:
                    "Hindi kailanman nabuksan ang {{names}}: isasara ito ng paghinto at ise-seal ang mga walang lamang ballot box nito.",
                stopNeverOpenedPosts_other:
                    "Hindi kailanman nabuksan ang {{names}}: isasara ang mga ito ng paghinto at ise-seal ang kanilang mga walang lamang ballot box.",
            },
            notifications: {
                generated: "Balota nilikha",
                published: "Balota inilathala",
                change_status: "Katayuan ng halalan nabago",
            },
            sealRefusals: {
                startAgain:
                    "Hindi na muling masisimulan ang botohan: sa I-seal sa pagsasara, nananatiling sarado ang botohang naisara na at naka-seal ang mga ballot box nito.",
                startDisabled:
                    "Hindi magagamit ang Simulan ang Pagboto: naka-seal o sine-seal na ang mga ballot box ng halalang ito, at sa I-seal sa pagsasara, nananatiling sarado ang botohang naisara na.",
                closedIsFinal:
                    "Hindi na mababago ang botohang naisara na: sa I-seal sa pagsasara, nananatiling sarado ang botohang naisara na.",
            },
        },
        emailEditor: {
            subject: "Paksa ng Email",
            tabs: {
                plaintext: "Simpleng Teksto",
                richtext: "Mayamang Teksto",
            },
        },
        sendCommunication: {
            send: "Ipadala",
            title: "Ipadala ang Abiso",
            subtitle: "Magpadala ng abiso sa mga botante.",
            sendButton: "Ipadala ang Abiso",
            voters: "Audience",
            schedule: "Iskedyul",
            nowInput: "Ipadala ngayon",
            dateInput: "Petsa at oras para magsimulang magpadala ng mga abiso",
            chooseDate: "Pumili ng petsa",
            languages: "Mga Wika",
            smsMessage: "Mensaheng SMS",
            errorSending: "Error sa pagpapadala ng abiso: {{error}}",
            successSending: "Nakaprograma/naipadala nang matagumpay ang abiso",
            method: "Pamamaraan ng Mga Template",
            type: "Uri ng Komunikasyon",
            alias: "Alias ng Template",
            votersSelection: {
                ALL_USERS: "Lahat",
                NOT_VOTED: "Mga hindi pa nakaboto",
                VOTED: "Mga nakaboto na",
                SELECTED: "Sa {{total}} Napiling {{voters}}",
            },
            path: {
                users: "mga tagagamit",
                voters: "mga botante",
            },
            methodTitle: "Template ng Komunikasyon",
            communicationMethod: {
                EMAIL: "Email",
                SMS: "SMS",
                WHATSAPP: "WhatsApp",
                VIBER: "Viber",
                MESSENGER: "Facebook Messenger",
            },
            communicationType: {
                CREDENTIALS: "Credentials",
                BALLOT_RECEIPT: "Resibo ng Balota",
            },
            email: {
                subject: "Paksa",
            },
        },
        tallysheet: {
            title: "Mga Kahon ng Balota",
            subtitle: "Digitalized na mga kahon ng balota ayon sa channel",
            createTallySuccess: "Na-save ang Tally Sheet",
            createTallyError: "Error sa pag-save ng Tally Sheet",
            createTallyErrorSameKindExists:
                "Ang Tally Sheet ay umiiral na para sa paligsahang ito na may parehong channel at lugar",
            allFieldsRequired: "Lahat ng mga patlang ay kinakailangan",
            header: {
                change: "Mga Pagbabago na Ipinapublish",
                viewChange: "Tingnan ang Paglalathala",
                history: "Kasaysayan ng Paglalathala",
            },
            action: {
                start: "Simulan ang Eleksyon",
                stop: "Itigil ang Eleksyon",
                pause: "Pansamantala",
                generate: "I-regenerate",
                publish: "I-publish ang mga Pagbabago",
                back: "Bumalik",
            },
            inputError: {
                totalValidDoesNotMatch:
                    "Ang mga boto ng kandidato ({{candidateVotesSum}}) ay dapat nasa pagitan ng {{lowerBound}} at {{upperBound}} ayon sa mga panuntunan sa pagboto ng paligsahang ito ({{nonBlankValidVotes}} balidong hindi blangkong boto × hanggang {{maxMarks}} marka bawat balota)",
                censusTooSmall:
                    "Ang kabuuang bilang ng mga boto ({{totalVotes}}) ay hindi dapat mas malaki kaysa sa senso ({{census}})",
                totalInvalidDoesNotMatch:
                    "Ang kabuuang bilang ng mga di-balidong boto ({{totalInvalid}}) ay dapat katumbas ng implicit na di-balidong boto ({{implicitInvalid}}) kasama ang explicit na di-balidong boto ({{explicitInvalid}})",
                totalVotesDoesNotMatch:
                    "Ang kabuuang bilang ng boto ({{totalVotes}}) ay dapat katumbas ng kabuuang balidong boto ({{totalValidVotes}}) kasama ang kabuuang di-balidong boto ({{totalInvalid}})",
                unknownCountingAlgorithm:
                    "Hindi nakikilala ang counting algorithm ng paligsahang ito ({{countingAlgorithm}}), kaya hindi matukoy ang pinapayagang bilang ng mga boto ng kandidato. Suriin ang konpigurasyon ng paligsahan.",
                blankBallotsInconsistent:
                    "Ang Blangkong Balota ay dapat magkaroon ng parehong halaga sa bawat sheet ng kontest sa kahong ito",
                blankBallotsOutOfBounds:
                    "Ang halaga ng Blangkong Balota ay wala sa hanay na ipinahihiwatig ng bilang ng blangkong boto bawat kontest sa kahong ito",
            },
            label: {
                area: "Lugar",
                channel: "Channel",
                total_votes: "Kabuuang Boto",
                total_valid_votes: "Kabuuang Valid na Boto",
                total_invalid: "Kabuuang Invalid na Boto",
                explicit_invalid: "Tahasang Invalid na Boto",
                implicit_invalid: "Implicit na Invalid na Boto",
                total_blank_votes: "Blankong Boto",
                blank_ballots: "Blangkong Balota",
                census: "Senso",
            },
            common: {
                tallyCeremony: {
                    manage: "Pamahalaan ang Seremonya ng Pagbibilang",
                    view: "Tingnan ang Seremonya ng Pagbibilang",
                    cancel: "Kanselahin ang Seremonya ng Pagbibilang",
                    addKey: "Magdagdag ng Susi ng Pagbibilang",
                },
                edit: "I-edit",
                confirm: "Kumpirmahin",
                back: "Bumalik",
                next: "Susunod",
                cancel: "Bumalik",
                data: "Data",
                title: "Tally Sheet",
                subtitle: "Pag-configure ng Tally Sheet.",
                candidates: "Mga Kandidato",
                save: "I-save",
                approve: "Aprubahan",
                disapprove: "Hindi aprubahan",
                show: "Ipakita",
                add: "Idagdag",
                versions: "Mga bersyon",
                warningDisapprove: "Sigurado ka bang hindi aprubahan ang Tally Sheet na ito?",
                warningApprove: "Sigurado ka bang aprubahan ang Tally Sheet na ito?",
            },
            empty: {
                header: "Walang Tally Sheet Pa.",
                action: "Bumuo ng Tally Sheet",
                add: "Magdagdag",
            },
            breadcrumbSteps: {
                start: "Simulan",
                edit: "I-edit",
                confirm: "Kumpirmahin",
                view: "Tingnan",
            },
            table: {
                area: "Lugar",
                contest: "Paligsahan",
                approvedVersion: "Inaprubahang bersyon",
                latestVersion: "Pinakabagong bersyon",
                labels: "Mga Label",
                annotations: "Mga Anotasyon",
            },
            versionsTable: {
                title: "Mga bersyon ng ballot box",
                version: "Bersyon",
                createdBy: "Ginawa ni",
                reviewedBy: "Na-review ni",
                createdAt: "Ginawa noong",
                reviewedAt: "Na-review noong",
                sourceImport: "Source import",
                importStatus: "Katayuan ng import",
                openImport: "Buksan ang import",
                sourceFile: "Source file",
            },
            message: {
                reviewError: "Error sa pag-review ng tally sheet",
                reviewSuccess: "Na-review ang tally sheet",
            },
        },
        application: {
            import: {
                title: "Mag-import ng Mga Aplikasyon",
                subtitle: "Mag-import ng data ng mga aplikasyon",
                paragraph:
                    "Mag-import ng mga aplikasyon gamit ang isang spreadsheet file sa format na Comma Separated Values (CSV). Mag-download ng halimbawa ng import na CSV file dito.",
                messages: {
                    success: "Matagumpay na na-import ang mga aplikasyon",
                    error: "May error sa pag-import ng mga aplikasyon",
                },
            },
            export: {
                title: "Mag-export ng Mga Aplikasyon",
                subtitle: "Mag-export ng data ng mga aplikasyon",
                button: "I-export",
                paragraph:
                    "Mag-export ng mga aplikasyon gamit ang isang spreadsheet file sa format na Comma Separated Values (CSV).",
                messages: {
                    success: "Matagumpay na na-export ang mga aplikasyon",
                    error: "May error sa pag-export ng mga aplikasyon",
                },
            },
        },
        template: {
            noPermissions: "Wala kang permiso na ma-access ang mga template.",
            title: "Mga Template",
            subtitle: "Listahan ng mga template",
            chooseMethods: "Pumili ng Mga Paraan",
            default: "Gamiton an default na plantilya",
            empty: {
                title: "Walang Template Pa",
                subtitle: "Gusto mo bang lumikha ng isa?",
            },
            action: {
                createOne: "Lumikha ng Template",
            },
            create: {
                title: "Lumikha ng Template",
                success: "Nalikha ang Template",
                error: "Error sa paglikha ng Template",
            },
            update: {
                success: "Na-update ang Template",
                error: "Error sa pag-update ng Template",
            },
            edit: {
                title: "I-edit ang Template",
            },
            form: {
                smsMessage: "SMS Message",
                document: "Dokumento",
                pdfOptions: "Mga Opsyon sa PDF",
                reportOptions: "Mga Pagpipilian sa Ulat",
                name: "Pangalan ng Template",
                alias: "Alias ng Template",
                type: "Uri ng Komunikasyon",
                communicationMethod: "Pamamaraan ng Komunikasyon",
            },
            type: {
                CREDENTIALS: "Mga Kredensyal",
                INITIALIZATION_REPORT: "Ulat ng Inisyal na Proseso",
                ELECTORAL_RESULTS: "Mga Resulta ng Halalan",
                BALLOT_IMAGES: "Mga Larawan ng Balota",
                BALLOT_RECEIPT: "Resibo ng Balota",
                ACTIVITY_LOGS: "Mga Tala ng Aktibidad",
                MANUAL_VERIFICATION: "Manwal na Pag-verify",
                PARTICIPATION_REPORT: "Ulat ng Pakikilahok",
            },
            method: {
                email: "Email",
                sms: "SMS",
                document: "Dokumento",
                whatsapp: "WhatsApp",
                viber: "Viber",
                messenger: "Facebook Messenger",
            },
            import: {
                title: "Mag-import ng Mga Template",
                subtitle: "Mag-import ng data ng mga template",
                paragraph:
                    "Mag-import ng mga template gamit ang spreadsheet file sa format na Comma Separated Values (CSV). I-download ang isang halimbawa ng import na CSV file dito.",
            },
        },
        materials: {
            audioInstructions: {
                screenLabel: "Mga tagubiling audio para sa screen",
                languageLabel: "Wika ng recording",
                none: "Hindi tagubiling audio",
                helperText:
                    "Maririnig ng mga botante ang file na ito kapag hiniling nila ang mga tagubilin sa screen na iyon.",
                screens: {
                    "election-chooser": "Listahan ng mga halalan",
                    "start": "Simula",
                    "ballot": "Balota",
                    "review": "Pagsusuri",
                    "confirmation": "Kumpirmasyon",
                    "audit": "Audit",
                    "ballot-locator": "Ballot locator",
                    "support-materials": "Mga materyal na pansuporta",
                },
            },
            createMaterialSuccess: "Nalikha ang suportang materyal",
            createMaterialError: "Error sa paglikha ng suportang materyal",
            updateMaterialSuccess: "Na-update ang suportang materyal",
            updateMaterialError: "Error sa pag-update ng suportang materyal",
            common: {
                title: "Suportang Materyal",
                subtitle: "Ilagay ang data ng suportang materyal.",
            },
            error: {
                title: "Kinakailangan ang pamagat",
                document: "Kinakailangan ang dokumento",
            },
            fields: {
                isHidden: "Nakatago",
                publicUrl: "Publikong URL",
            },
            empty: {
                header: "Wala pang support material",
                action: "Gumawa ng support material",
            },
        },
        widget: {
            logs: "Mga Log",
        },
        settings: {
            countries: {
                title: "Pag-block ng Bansa",
                votingDescription:
                    "Piliin sa ibaba ang mga bansang gusto mong i-block ang pagboto mula sa.",
                enrollmentDescription:
                    " Piliin sa ibaba ang mga bansang gusto mong i-block ang pag-eenroll mula sa.",
                error: {
                    errorSaving: "Error sa pag-save ng listahan ng mga bansa",
                },
            },
            backupRestore: {
                title: "Backup / Ibalik ang Tenant config",
                backup: {
                    label: "Backup",
                    subtitle: "Backup ng Tenant configurations",
                },
                restore: {
                    label: "Ibalik",
                    subtitle: "Ibalik ang Tenant config",
                    title: "Mag-import ng Mga Konfigurasyon ng Tenant",
                    paragraph:
                        "Mag-import ng mga konfigurasyon ng tenant, mga konfigurasyon ng Keycloak, mga role at data ng pahintulot gamit ang zip na folder.",
                    tenantConfigOption: "Mag-import ng Mga Konfigurasyon ng Tenant",
                    keycloakConfigOption: "Mag-import ng Mga Konfigurasyon ng Keycloak",
                    RolesConfigOption: "Mag-import ng Mga Role at Pahintulot na Konfigurasyon",
                },
            },
            previewScreen: {
                label: "Mga Preview",
                noContent: "Walang nahanap na mga preview",
                table: {
                    title: "Mga Panlabas na Preview",
                    description:
                        "Isang rekord ng mga preview ng estilo ng balota na ginawa sa pamamagitan ng mga panlabas na request",
                    requestedBy: "Hiniling ng",
                    document: "Dokumento",
                    url: "URL",
                },
            },
            languages: {
                default: "Default na Wika",
            },
        },
        approvalsScreen: {
            column: {
                status: "Katayuan",
                id: "ID ng aplikasyon",
                applicantId: "ID ng aplikante",
                verificationType: "Pag-verify",
                createdAt: "Nag-apply",
                verified_by: "Na-verify ni",
                voter: "Botante",
                what: "Ano ang nangyari",
                post: "Post",
                when: "Kailan",
            },
            status: {
                PENDING: "Kailangang suriin",
                ACCEPTED: "Naaprubahan",
                REJECTED: "Tinanggihan",
            },
            verification: {
                AUTOMATIC: "Awtomatiko",
                MANUAL: "Mano-mano",
            },
            time: {
                minutes_one: "{{count}} minuto",
                minutes_other: "{{count}} minuto",
                hours_one: "{{count}} oras",
                hours_other: "{{count}} oras",
                days_one: "{{count}} araw",
                days_other: "{{count}} araw",
            },
            summary: {
                join: "{{head}} at {{last}}",
                differs_one: "{{fields}} ang naiiba sa talaan",
                differs_other: "{{fields}} ang naiiba sa talaan",
                typedByHand:
                    "Mano-manong tinype ang mga detalye, hindi binasa mula sa na-scan na ID",
                needsFaceToFace: "Kailangan ng harapang pagsusuri",
                scanVerified: "Na-verify ang na-scan na ID",
                noVoter: "Walang natagpuang botante sa talaan",
                allMatch: "Tumutugma sa talaan ang lahat ng detalye",
                needsReview: "Naghihintay ng pasya ng isang tao",
                approvedBy: "Inaprubahan ni {{name}}",
                approvedAuto: "Awtomatikong naaprubahan",
                rejectedBy: "Tinanggihan ni {{name}}",
                rejectedAuto: "Awtomatikong tinanggihan",
            },
            list: {
                title: "Mga Pag-apruba",
                subtitle:
                    "Dito naghihintay ng isang tao ang mga pagpapatalang hindi mapagpasyahan ng mga panuntunan nang mag-isa.",
                search: "Maghanap",
                review: "Suriin ang pagpapatala",
                openRecord: "Buksan ang pagpapatala",
                seeRule: "Tingnan ang panuntunang nagpasya",
                unnamed: "Aplikanteng walang pangalan",
                waiting: "{{time}} nang naghihintay",
                applied: "Nag-apply noong {{date}}",
                empty: {
                    title: "Walang laman dito",
                    text: "Lalabas dito ang mga pagpapatalang may ganitong katayuan. Subukan ang ibang paghahanap o katayuan.",
                },
            },
            flow: {
                stepsLabel: "Mga hakbang ng pagsusuri",
                steps: {
                    identity: "Suriin ang pagkakakilanlan",
                    voter: "Hanapin ang botante",
                    decide: "Magpasya",
                },
                continue: "Magpatuloy",
                backToList: "Bumalik sa Mga Pag-apruba",
                identity: {
                    details: "Mga detalye sa pagpapatala",
                    confirm:
                        "Sinuri ko ang ID ng botante nang personal o sa video call, at tumutugma ito sa pagpapatalang ito.",
                    checked: "Nakumpirma ang harapang pagsusuri",
                    notChecked: "Hindi pa nakukumpirma ang harapang pagsusuri",
                },
                voter: {
                    none: "Wala sa mga ito ang botante",
                    noneHint:
                        "Kung gayon, matatanggihan lamang ang pagpapatala dahil walang tumutugmang botante.",
                    noneChosen: "Wala sa mga ito ang botante",
                    notChosen: "Wala pang napiling botante",
                },
                decide: {
                    approve: "Aprubahan",
                    reject: "Tanggihan",
                    approveText:
                        "Iugnay ang pagpapatalang ito kay {{voter}} sa talaan. Sasabihan ang botante sa email o text message at makakapag-sign in siya upang bumoto kapag nagbukas ang botohan.",
                    rejectText: "Sasabihin sa botante ang dahilan. Hindi na ito mababawi.",
                    chooseVoter:
                        "Piliin ang tumutugmang botante sa hakbang 2 upang makapag-apruba.",
                    noVoter:
                        "Wala kang nahanap na tumutugmang botante, kaya matatanggihan lamang ang pagpapatalang ito.",
                    enrolled: "Nakatala na ang napiling botante.",
                    faceToFace:
                        "Kumpirmahin ang harapang pagsusuri sa hakbang 1 upang makapag-apruba.",
                },
            },
            review: {
                loadError: "Hindi ma-load ang pagpapatala.",
                applied: "Nag-apply noong {{date}}",
                waiting: "{{time}} nang naghihintay",
                whyTitle: "Bakit kailangan nito ng isang tao",
                decisionTitle: "Paano ito napagpasyahan",
                rule: "Panuntunan {{rule}} ng bersyon {{version}} ng matrix",
                ruleLast: "Huling panuntunan ng bersyon {{version}} ng matrix",
                seeRule: "Tingnan ang panuntunan",
                why: {
                    typedByHand:
                        "Mano-manong tinype ng botante ang kanyang mga detalye sa halip na mag-scan ng ID. Hindi kailanman awtomatikong inaaprubahan ang ganitong mga pagpapatala: kinukumpirma muna ng isang opisyal kung sino siya.",
                    differs_one:
                        "Isang detalye ang hindi tumutugma sa talaan: {{details}}. Hinihiling ng mga panuntunan sa pag-apruba na suriin ng isang tao ang pagpapatalang ito.",
                    differs_other:
                        "{{count}} detalye ang hindi tumutugma sa talaan: {{details}}. Hinihiling ng mga panuntunan sa pag-apruba na suriin ng isang tao ang pagpapatalang ito.",
                    differsFields_one:
                        "Isang detalye ang hindi tumutugma sa talaan: {{fields}}. Hinihiling ng mga panuntunan sa pag-apruba na suriin ng isang tao ang pagpapatalang ito.",
                    differsFields_other:
                        "{{count}} detalye ang hindi tumutugma sa talaan: {{fields}}. Hinihiling ng mga panuntunan sa pag-apruba na suriin ng isang tao ang pagpapatalang ito.",
                    difference:
                        "ang {{field}} ay “{{enrollment}}” sa pagpapatala at “{{registry}}” sa talaan",
                    noVoter:
                        "Walang botante sa talaan na may ganitong mga detalye. Hinihiling ng mga panuntunan sa pag-apruba na suriin ng isang tao ang pagpapatalang ito.",
                    severalVoters:
                        "Higit sa isang botante sa talaan ang tumutugma sa pagpapatalang ito. Isang tao ang pipili ng tama.",
                    pending:
                        "Hinihiling ng mga panuntunan sa pag-apruba na suriin ng isang tao ang pagpapatalang ito.",
                    unknown: "Naghihintay ang pagpapatalang ito ng pasya ng isang tao.",
                    approvedAuto:
                        "Awtomatikong inaprubahan ng mga panuntunan sa pag-apruba ang pagpapatalang ito. Pumasa ang lahat ng pagsusuring hinihingi ng mga ito.",
                    approvedBy: "Inaprubahan ni {{name}} ang pagpapatalang ito noong {{date}}.",
                    rejectedAuto:
                        "Awtomatikong tinanggihan ng mga panuntunan sa pag-apruba ang pagpapatalang ito: {{reason}}.",
                    rejectedBy:
                        "Tinanggihan ni {{name}} ang pagpapatalang ito noong {{date}}: {{reason}}.",
                },
                registryHelp:
                    "Naghanap kami ng mga botanteng may parehong {{fields}}. Piliin kung kanino ang pagpapatalang ito.",
                registrySearching:
                    "Ito ang mga botante sa talaan na tumutugma sa iyong paghahanap. Piliin kung kanino ang pagpapatalang ito.",
                registrySearch: "Wala sa listahan? Maghanap sa talaan ayon sa pangalan o email",
                registryLoading: "Naghahanap sa talaan",
                registryError: "Hindi makapaghanap sa talaan.",
                noCandidates:
                    "Walang botante sa talaan na tumutugma. Subukang maghanap ayon sa pangalan o email.",
                candidates: "Mga botante sa talaan",
                alreadyEnrolled: "Nakatala na",
                bestMatch: "Pinakatugma",
                detailsMatch: "{{count}} sa {{total}} detalye ang tumutugma",
                compareTitle: "Inihambing kay {{name}} sa talaan",
                col: {
                    detail: "Detalye",
                    enrollment: "Sa pagpapatala",
                    registry: "Sa talaan",
                    result: "Resulta",
                },
                same: "Pareho",
                differs: "Magkaiba",
                compareNote:
                    "Hindi isinasaalang-alang sa mga pangalan ang malaki at maliit na titik, mga tuldik at gitling.",
                compareJoint:
                    "Para sa mga lisensya sa pagmamaneho at seafarer's book, pinagsamang inihahambing ang unang pangalan at gitnang pangalan.",
                applicationId: "ID ng aplikasyon",
                copy: "Kopyahin",
                copied: "Nakopya",
                approve: "Aprubahan ang pagpapatala",
                approveDialog: {
                    title: "Aprubahan si {{name}}?",
                    body: "Iuugnay nito ang pagpapatala sa botante sa talaan na nasa ibaba. Sasabihan ang botante sa email o text message at makakapag-sign in siya upang bumoto kapag nagbukas ang botohan.",
                    checked: "Sinuri mo nang harapan ang ID ng botante.",
                    irreversible: "Hindi na ito mababawi.",
                    confirm: "Aprubahan",
                },
                reject: "Tanggihan ang pagpapatala",
            },
            idCheck: {
                title: "Pagsusuri ng ID",
                method: {
                    VERIFIED: "Na-verify ang na-scan na ID",
                    MANUAL_ENTRY: "Mano-manong tinype",
                    UNKNOWN: "Hindi iniulat",
                },
                verified: "Na-verify ng proseso ng pagpapatala ang ID ng botante",
                typedByHand: "Mano-manong tinype ng botante ang kanyang mga detalye",
                unknown:
                    "Hindi iniulat ng proseso ng pagpapatala kung paano sinuri ang pagkakakilanlan",
                faceToFaceTitle: "Suriin siya nang harapan bago aprubahan",
                faceToFaceText:
                    "Kausapin ang botante nang personal o sa video call at ihambing ang kanyang ID sa mga detalye sa pahinang ito.",
            },
            reject: {
                rejectReason: "Dahilan ng pagtanggi",
                message: "Mensahe sa botante",
                messageRequired: "Sumulat ng mensahe para sa botante kapag ang dahilan ay Iba pa.",
                reasons: {
                    "undefined": "-",
                    "insufficient-information": "Kulang na datos",
                    "no-matching-voter": "Walang tumutugmang botante",
                    "voter-already-approved": "Naaprubahan na",
                    "other": "Iba pa",
                },
                hint: {
                    "insufficient-information": "May kulang na detalye o hindi ito mabasa.",
                    "no-matching-voter": "Wala ang tao sa talaan ng mga botante.",
                    "voter-already-approved": "Nakatala na ang botanteng ito.",
                    "other": "Sumulat ng sarili mong mensahe.",
                },
                preview: {
                    "insufficient-information":
                        "Hindi ka namin naitala dahil may kulang o hindi mabasa sa iyong mga detalye. Mangyaring magpatala muli na may kumpletong detalye.",
                    "no-matching-voter":
                        "Wala kaming nahanap na botante sa talaan na tumutugma sa iyong mga detalye. Suriin ang iyong mga detalye at magpatala muli, o makipag-ugnayan sa iyong tanggapan ng halalan.",
                    "voter-already-approved":
                        "Nakatala ka na. Makakapag-sign in ka upang bumoto kapag nagbukas ang botohan.",
                },
                previewTitle: "Makikita ng botante",
            },
            notifications: {
                approveError: "Hindi maaprubahan ang pagpapatala",
                approveSuccess: "Naaprubahan si {{name}}. Nasabihan na ang botante.",
                rejectError: "Hindi matanggihan ang pagpapatala",
                rejectSuccess: "Tinanggihan si {{name}}. Nasabihan na ang botante.",
                VoterApprovedAlready: "Nakatala na ang botanteng ito.",
            },
            export: {
                success: "Matagumpay na natapos ang pag-export ng mga aplikasyon",
                error: "Error sa pag-export ng mga aplikasyon",
            },
            matrix: {
                button: "Matrix ng pag-apruba",
                title: "Matrix ng pag-apruba",
                back: "Mga Pag-apruba",
                subtitle:
                    "Ang mga panuntunan ang nagpapasya kung ano ang mangyayari sa bawat pagpapatala. Ang unang panuntunang tumutugma ang nagpapasya.",
                versionChip: "Bersyon {{version}}",
                savedBy: "Na-save noong {{date}} ni {{user}}",
                builtIn: "Mga likas na panuntunan, ginagamit hanggang may ma-save na bersyon",
                unsaved: "May mga pagbabagong hindi pa na-save",
                viewOnly: "Pagtingin lamang",
                readOnlyTitle: "Makikita mo ang mga panuntunan ngunit hindi mo mababago",
                readOnlyText:
                    "Hilingin sa isang administrator na may pahintulot na approval-matrix-write na gawin ang mga pagbabago.",
                loadError: "Hindi ma-load ang matrix ng pag-apruba.",
                compared: "Ano ang inihahambing namin",
                comparedHelp:
                    "Ang bawat pagpapatala ay inihahambing sa botanteng natagpuan sa talaan. Hindi isinasaalang-alang sa mga pangalan ang malaki at maliit na titik, mga tuldik at gitling; para sa mga lisensya sa pagmamaneho at seafarer's book, pinagsamang inihahambing ang unang pangalan at gitnang pangalan.",
                addCompared: "Maghambing ng isa pang detalye",
                rules: "Mga panuntunan",
                rulesHelp:
                    "Sinusuri ang mga panuntunan mula sa itaas. Ang unang tumutugma ang nagpapasya; kung walang tumutugma, ang huling panuntunan ang ilalapat.",
                when: "Kapag",
                then: "Kung gayon",
                otherwise: "Kung hindi",
                noneApply: "Walang tumutugma sa mga panuntunan sa itaas",
                andWord: "at",
                and: " at ",
                appliesToExample: "Tumutugma sa iyong halimbawa",
                cameFrom: "Nagpasya sa pagpapatalang pinanggalingan mo",
                voterIsTold: "Sasabihin sa botante: “{{reason}}”.",
                sentence: "Kapag {{when}}, {{outcome}}.",
                sentenceOtherwise: "Kung walang tumutugma sa mga panuntunan sa itaas, {{outcome}}.",
                sentenceEmpty:
                    "Magdagdag ng kondisyon upang sabihin kung kailan tumutugma ang panuntunang ito.",
                addRule: "Magdagdag ng panuntunan",
                discard: "Itapon ang mga pagbabago",
                actions: {
                    edit: "I-edit ang panuntunan {{number}}",
                    editOtherwise: "I-edit ang huling panuntunan",
                    moveUp: "Itaas ang panuntunan {{number}}",
                    moveDown: "Ibaba ang panuntunan {{number}}",
                    delete: "Tanggalin ang panuntunan {{number}}",
                },
                saveBar: {
                    title: "May mga pagbabago kang hindi pa na-save",
                    fix_one: "Ayusin ang 1 panuntunan bago mag-save",
                    fix_other: "Ayusin ang {{count}} panuntunan bago mag-save",
                    more: "+{{count}} pa",
                },
                test: "Sumubok ng halimbawa",
                testHelp:
                    "Ilarawan ang isang pagpapatala upang makita kung aling panuntunan ang nagpapasya rito. Kasama ang mga pagbabago mong hindi pa na-save.",
                testDetails: "Mga detalyeng inihahambing",
                applies: "Tumutugma ang panuntunan {{number}}",
                otherwiseApplies: "Ang huling panuntunan ang ilalapat",
                testError: "Hindi masubukan ang halimbawa.",
                testInvalid: "Ayusin ang mga panuntunang ito upang makasubok ng halimbawa:",
                ruleError: "Panuntunan {{number}}: {{error}}",
                invariants: {
                    MANUAL_ENTRY_NOT_ACCEPTED:
                        "Hindi kailanman awtomatikong inaaprubahan ang pagkakakilanlang tinype, kaya ipapadala ito sa isang tao.",
                    ALREADY_ENROLLED_NOT_ACCEPTED:
                        "Hindi na muling inaaprubahan ang botanteng nakatala na.",
                    NO_VOTER_NOT_ACCEPTED: "Walang inaaprubahan kung walang botante sa talaan.",
                    OTHERWISE_NOT_ACCEPTED: "Hindi kailanman nag-aapruba ang huling panuntunan.",
                },
                dialog: {
                    editTitle: "I-edit ang panuntunan {{number}}",
                    newTitle: "Bagong panuntunan",
                    otherwiseTitle: "I-edit ang huling panuntunan",
                    summary: "Sa madaling salita",
                    whenHelp:
                        "Dapat totoo ang lahat ng ito. Huwag isama ang isang kondisyon kapag hindi ito mahalaga.",
                    otherwiseHelp: "Kung walang tumutugma sa mga panuntunan sa itaas",
                    addCondition: "Magdagdag ng kondisyon",
                    remove: "Alisin ang “{{condition}}”",
                    identity: "Pagsusuri ng pagkakakilanlan",
                    voterFound: "Botante sa talaan",
                    alreadyEnrolled: "Nakatala na",
                    validId: "Uri ng ID",
                    differing: "Mga detalyeng naiiba",
                    decision: "Pasya",
                    reason: "Ano ang sasabihin sa botante",
                    voterSees: "Makikita ng botante",
                    apply: "Ilapat",
                    close: "Isara",
                    yes: "Oo",
                    no: "Hindi",
                    notReported: "Hindi iniulat",
                },
                identity: {
                    VERIFIED: "Na-verify sa pag-scan ng ID",
                    MANUAL_ENTRY: "Mano-manong tinype",
                },
                differing: {
                    none: "Wala",
                    exactly_1: "Eksaktong 1",
                    at_most_1: "Hindi hihigit sa 1",
                    exactly_2: "Eksaktong 2",
                    at_most_2: "Hindi hihigit sa 2",
                    at_least_3: "3 o higit pa",
                },
                fieldMatch: {
                    MATCHES: "Pareho",
                    DIFFERS: "Magkaiba",
                },
                decisions: {
                    ACCEPTED: "Awtomatikong aprubahan",
                    PENDING: "Ipadala sa isang tao",
                    REJECTED: "Tanggihan",
                },
                outcomeShort: {
                    ACCEPTED: "awtomatikong aprubahan",
                    PENDING: "ipadala sa isang tao",
                    REJECTED: "tanggihan",
                },
                outcomeHelp: {
                    ACCEPTED: "Naitatala ang botante nang walang taong tumitingin dito.",
                    PENDING:
                        "Isang opisyal ang magpapasya, at sasabihin sa botante na sinusuri ang kanyang pagpapatala.",
                    REJECTED: "Sasabihin sa botante ang dahilan, at maaari siyang magpatala muli.",
                },
                outcomeSentence: {
                    ACCEPTED: "awtomatikong aaprubahan ang pagpapatala",
                    PENDING: "ipapadala ang pagpapatala sa isang tao",
                    REJECTED: "tatanggihan ang pagpapatala",
                },
                reasons: {
                    NO_VOTER: "Walang tumutugmang botante",
                    ALREADY_APPROVED: "Naaprubahan na",
                    INSUFFICIENT_INFORMATION: "Kulang na datos",
                    IDENTITY_NOT_VERIFIED: "Hindi na-verify ang pagkakakilanlan",
                    OTHER: "Iba pa",
                },
                voterText: {
                    NO_VOTER:
                        "Wala kaming nahanap na botante sa talaan na tumutugma sa iyong mga detalye. Suriin ang iyong mga detalye at magpatala muli, o makipag-ugnayan sa iyong tanggapan ng halalan.",
                    ALREADY_APPROVED:
                        "Nakatala ka na. Makakapag-sign in ka upang bumoto kapag nagbukas ang botohan.",
                    INSUFFICIENT_INFORMATION:
                        "Hindi ka namin naitala dahil may kulang o hindi mabasa sa iyong mga detalye. Mangyaring magpatala muli na may kumpletong detalye.",
                    IDENTITY_NOT_VERIFIED:
                        "Hindi namin awtomatikong na-verify ang iyong pagkakakilanlan, kaya susuriin ng isang opisyal ng halalan ang iyong pagpapatala.",
                    OTHER: "Isang opisyal ng halalan ang susulat ng mensaheng ito kapag nagpasya na siya.",
                },
                conditions: {
                    any: "Wala pang kondisyon",
                    identity: {
                        VERIFIED: "Na-verify ang pagkakakilanlan sa pag-scan ng ID",
                        MANUAL_ENTRY: "Mano-manong tinype ang pagkakakilanlan",
                    },
                    voterFound: {
                        true: "Natagpuan ang botante sa talaan",
                        false: "Walang natagpuang botante sa talaan",
                    },
                    alreadyEnrolled: {
                        true: "Nakatala na",
                        false: "Hindi pa nakatala",
                    },
                    validId: "ID: {{id}}",
                    differing: {
                        none: "Tumutugma ang lahat ng detalye",
                        exactly_1: "Eksaktong 1 detalye ang naiiba",
                        at_most_1: "Hindi hihigit sa 1 detalye ang naiiba",
                        exactly_2: "Eksaktong 2 detalye ang naiiba",
                        at_most_2: "Hindi hihigit sa 2 detalye ang naiiba",
                        at_least_3: "3 o higit pang detalye ang naiiba",
                    },
                    field: {
                        MATCHES: "Tumutugma ang {{field}}",
                        DIFFERS: "Naiiba ang {{field}}",
                    },
                },
                errors: {
                    ACCEPTS_MANUAL_ENTRY:
                        "Hindi maaaring awtomatikong aprubahan ang mga pagpapatalang mano-manong tinype ang pagkakakilanlan.",
                    ACCEPTS_ALREADY_ENROLLED:
                        "Hindi na maaaring aprubahan muli ang botanteng nakatala na.",
                    ACCEPTS_WITHOUT_VOTER:
                        "Hindi maaaring aprubahan ang pagpapatala kung walang botante sa talaan.",
                    OTHERWISE_ACCEPTS:
                        "Ang huling panuntunan ay maaaring magpadala ng mga pagpapatala sa isang tao o tanggihan ang mga ito, ngunit hindi aprubahan.",
                    MISSING_REASON: "Piliin kung ano ang sasabihin sa botante.",
                    UNEXPECTED_REASON: "Walang dahilan ang isang pag-apruba.",
                    NO_COMPARED_FIELDS: "Pumili ng kahit isang detalyeng ihahambing sa talaan.",
                    DUPLICATE_COMPARED_FIELD: "May inihahambing na detalyeng inulit.",
                    UNKNOWN_FIELD: "May panuntunang gumagamit ng detalyeng hindi inihahambing.",
                    NO_CONDITIONS:
                        "Magdagdag ng kahit isang kondisyon. Ang huling panuntunan lamang ang sumasaklaw sa lahat ng iba pa.",
                },
                change: {
                    added: "Naidagdag ang panuntunan {{number}}",
                    decision: "Panuntunan {{number}}: {{from}} → {{to}}",
                    edited: "Nabago ang panuntunan {{number}}",
                    removed: "May inalis na panuntunan ({{text}})",
                    moved: "Binago ang pagkakasunod-sunod ng mga panuntunan",
                    otherwise: "Nabago ang huling panuntunan",
                    compared: "Nabago ang mga detalyeng inihahambing",
                },
                save: {
                    button: "I-save bilang bersyon {{version}}",
                    title: "I-save bilang bersyon {{version}}?",
                    body: "Mula ngayon, pagpapasyahan ang mga bagong pagpapatala gamit ang mga panuntunang ito. Mananatili ang pasya ng mga pagpapatalang napagpasyahan na.",
                    changes: "Ano ang nagbago",
                    log: "Itinatala ang bagong bersyon sa electoral log.",
                    confirm: "I-save ang bersyon {{version}}",
                    success: "Na-save bilang bersyon {{version}}",
                    error: "Hindi ma-save ang matrix ng pag-apruba",
                },
            },
        },
        monitoring: {
            title: "Pagsubaybay",
            loading: "Nilo-load ang pagsubaybay",
            unavailableAlert:
                "Hindi ma-load ang mga dashboard ng pagsubaybay, kaya ang karaniwang dashboard ang ipinapakita.",
            noDashboards: "Walang dashboard ng pagsubaybay ang event na ito.",
            dashboardFailed: "Hindi ma-load ang dashboard ng pagsubaybay.",
            dashboardInvalid: "Hindi maipakita ang dashboard na ito: {{problem}}",
            retry: "Subukan muli",
            errors: {
                busy: "Abala ang server. Susubukan muli sa loob ng ilang segundo.",
                forbiddenScope:
                    "Hindi mo maaaring makita ang rehiyon, Post o bansang ito. Pumili ng iba.",
                snapshotPruned:
                    "Hindi na itinatago ang ipinapakitang update. Ipinapakita na ngayon ng dashboard ang pinakabagong update: mag-export muli para gamitin ito.",
                checksUnavailable:
                    "Hindi available ngayon ang serbisyo ng chart. Subukan muli mamaya.",
                lockedDown: "Naka-lock down ang election event, kaya hindi ito mababago.",
                notFound:
                    "Hindi na naka-configure ang dashboard o widget na ito. I-reload ang pahina.",
                badRequest: "Hindi tinanggap ang kahilingan. I-reload ang pahina at subukan muli.",
                conflict: "May ibang nag-save muna ng pagbabago. I-reload at subukan muli.",
                invalid: "Hindi tinatanggap ang ilang halaga ng export.",
                unknown: "May nangyaring mali. Subukan muli mamaya.",
            },
            header: {
                dashboard: "Dashboard",
                updated: "Na-update {{time}} ({{timeZone}})",
                notUpdated: "Hindi pa nabibilang",
                refresh: "bawat {{seconds}} s",
                export: "I-export",
                editDashboard: "I-edit ang dashboard",
                preset: "Preset ng dashboard",
                reload: "Tingnan ang bagong bilang",
            },
            footer: {
                dataThrough: "Datos hanggang {{time}} ({{timeZone}})",
            },
            selectors: {
                region: "Rehiyon",
                post: "Post",
                country: "Bansa",
                allRegions: "Lahat ng rehiyon",
                allPosts: "Lahat ng Post",
                allAuthorizedPosts: "Lahat ng awtorisadong Post",
                allCountries: "Lahat ng bansa",
                authorizedOnly: "{{all}} (awtorisado)",
            },
            widget: {
                menu: "Mga aksyon para sa {{widget}}",
                configure: "I-configure ang widget",
                viewData: "Tingnan ang datos",
                export: "I-export",
                duplicate: "Doblehin",
                loading: "Nilo-load ang {{widget}}",
                missing: "May widget na wala sa dashboard: {{id}}",
                updating: "Ina-update ang {{widget}}",
                updatingNote: "Ina-update: ang ipinapakitang chart ay ang nauna.",
            },
            frame: {
                title: "Tsart ng {{widget}}",
            },
            sources: {
                voter_turnout: "Dami ng bumoto",
                test_voting: "Pagsubok na pagboto",
                enrollment_decisions: "Mga desisyon sa enrollment",
                voting_credentials: "Mga kredensyal sa pagboto",
                poll_status: "Katayuan ng botohan",
                final_testing_lockdown: "Huling pagsubok at lockdown",
                counting_transmission: "Pagbilang at transmisyon",
                voting_enrollment_activity: "Aktibidad sa pagboto at enrollment",
                access_security: "Access at seguridad",
                attack_detections: "Mga natukoy na atake",
                helpdesk: "Helpdesk",
            },
            reasons: {
                TEST_ELECTION_DESIGNATION: "hindi pa maaaring markahan ang mga pagsubok na halalan",
                CREDENTIAL_ISSUED_EVENT: "hindi pa naitatala ang pag-isyu ng kredensyal",
                FINAL_TESTING_LOCKDOWN_STATE: "hindi pa naitatala ang huling pagsubok at lockdown",
                ATTACK_DETECTION_FEED: "walang nakakonektang feed ng pagtukoy ng atake",
                HELPDESK_INTEGRATION: "walang nakakonektang helpdesk system",
            },
            notices: {
                UNREGISTERED_ATTEMPTS_AT_EVENT_SCOPE_ONLY:
                    "Ang mga pagtatangka gamit ang hindi rehistradong username ay hindi kabilang sa anumang Post, kaya binibilang lamang ang mga ito para sa buong event.",
                UNREGISTERED_ATTEMPTS_EXCLUDED:
                    "Hindi kasama sa mga bilang na ito ang mga pagtatangka gamit ang hindi rehistradong username; binibilang ang mga ito para sa buong event.",
                CREDENTIALS_ISSUED_WHEN_PASSWORD_SET:
                    "Itinuturing na naibigay ang mga kredensyal kapag naitakda na ang password ng botante, hanggang maitala ng platform ang pagbibigay nito.",
                CONFIG_NEWER_THAN_SNAPSHOT:
                    "Iginuhit gamit ang pinakabagong setting; bibilangin ang mga numero gamit ito sa susunod na pagbilang.",
                CONFIG_AT_SNAPSHOT_UNAVAILABLE:
                    "Iginuhit gamit ang kasalukuyang setting: hindi na itinatago ang setting na ginamit sa pagbilang ng mga numero.",
            },
            unavailable: {
                notConnected: "Hindi nakakonekta · {{reason}}",
                notConnectedHelp: "Walang ipapakita hangga't hindi ito nakakonekta.",
                unknownReason: "hindi available ang pinagkukunan ng datos",
                noSnapshot: "Hindi pa nabibilang",
                noSnapshotHelp:
                    "Hindi pa tapos ang unang bilang. Kusang mag-a-update ang widget na ito.",
                scopePending: "Binibilang ang seleksyong ito",
                scopePendingHelp:
                    "Bibilangin ang seleksyong ito sa susunod na pagpasa, sa loob ng mga isang minuto.",
                settingsPending: "Nagbibilang gamit ang bagong mga setting…",
                settingsPendingHelp:
                    "Muling binibilang ang mga numero gamit ang na-save na mga setting, sa loob ng mga isang minuto.",
                renderFailed: "Hindi maiguhit ang tsart",
                renderFailedHelp: "Ang mga numero ang ipinapakita sa halip.",
                invalid: "Hindi maipakita ang widget na ito",
                invalidHelp: "May problema ang configuration nito.",
                requestFailed: "Hindi ma-load ang widget na ito",
                requestFailedHelp: "Susubukan muli kapag nag-update ang dashboard.",
            },
            dataTable: {
                title: "{{widget}} · datos",
                close: "Isara",
                empty: "Walang hilera",
                rowsPerPage: "Mga hilera bawat pahina:",
                shownRows: "{{from}}–{{to}} sa {{total}}",
                firstPage: "Unang pahina",
                previousPage: "Nakaraang pahina",
                nextPage: "Susunod na pahina",
                lastPage: "Huling pahina",
            },
            columns: {
                numerator: "Numerator",
                denominator: "Denominator",
                pct: "Porsyento",
                pct_label: "Porsyento gaya ng ipinakita",
                group: "Grupo",
                group_key: "Susi ng grupo",
                post: "Post",
                post_id: "ID ng Post",
                region: "Rehiyon",
                country: "Bansa",
                reason: "Dahilan",
                category: "Kategorya",
                state: "Katayuan",
                state_label: "Katayuan gaya ng ipinakita",
                bucket_start: "Simula ng panahon",
                bucket_label: "Panahon",
                bucket_utc: "Simula ng panahon (UTC)",
                measure: "Sukatan",
                label: "Label",
                value: "Halaga",
            },
            measures: {
                registered: "Nakarehistro",
                pre_enrolled: "Paunang nakatala",
                credentials_issued: "Naibigay na kredensyal",
                test_voted: "Pansubok na boto",
                voted: "Bumoto",
                voted_pre_enrolled: "Paunang nakatala na bumoto",
                applications: "Mga aplikasyon",
                pending: "Nakabinbin",
                approved: "Inaprubahan",
                disapproved: "Hindi inaprubahan",
                posts: "Mga Post",
                initialized: "Na-initialize",
                opened: "Nabuksan",
                paused: "Naka-pause",
                closed: "Sarado",
                tested: "Nasubukan",
                locked_down: "Naka-lock",
                tallied: "Nabilang",
                transmitted: "Naipadala",
                transmission_failed: "Nabigo ang pagpapadala",
                logins: "Mga pag-login",
                login_failures: "Mga nabigong pag-login",
                login_failures_valid_user: "Nabigo, wastong user",
                login_failures_unregistered: "Nabigo, hindi rehistradong user",
                password_resets: "Mga pag-reset ng password",
                password_reset_requests: "Mga kahilingang i-reset ang password",
                detections: "Mga natukoy",
                issues: "Mga isyu",
                pending_issues: "Mga nakabinbing isyu",
            },
            export: {
                title: "I-export ang datos ng pagsubaybay",
                format: "Format",
                csv: "CSV",
                sql: "SQL",
                from: "Mula",
                to: "Hanggang",
                timeZoneHelp:
                    "Ang mga oras ay nasa {{timeZone}}. Ang mga kabuuan, katayuan at grupo ay ayon sa ipinapakitang update; ang mga hilera lamang ng mga serye ng aktibidad ang limitado sa saklaw, mula sa oras ng simula hanggang sa, ngunit hindi kasama, ang oras ng pagtatapos.",
                cancel: "Kanselahin",
                export: "I-export",
                invalidRange: "Dapat mas huli ang pagtatapos kaysa sa simula.",
                problems: {
                    unknownSelector:
                        "Wala nang pagpipiliang “{{selector}}” ang {{widget}}. I-reload ang dashboard at mag-export muli.",
                    unknownOption:
                        "Hindi na iniaalok ang halagang pinili para sa “{{selector}}” sa {{widget}}. Piliin itong muli at mag-export.",
                },
            },
            editor: {
                scopeSelector: {
                    region: "Rehiyon",
                    post: "Posisyon",
                    country: "Bansa",
                },
                sources: {
                    voter_turnout: "Dami ng bumoto",
                    test_voting: "Pagsubok na pagboto",
                    enrollment_decisions: "Mga desisyon sa pagpaparehistro",
                    voting_credentials: "Mga kredensyal sa pagboto",
                    poll_status: "Katayuan ng halalan",
                    final_testing_lockdown: "Huling pagsubok at lockdown",
                    counting_transmission: "Pagbibilang at pagpapadala",
                    voting_enrollment_activity: "Aktibidad sa pagboto at pagpaparehistro",
                    access_security: "Access at seguridad",
                    attack_detections: "Mga natukoy na atake",
                    helpdesk: "Helpdesk",
                },
                templates: {
                    summary: "Buod",
                    by_group: "Ayon sa grupo",
                    by_post: "Ayon sa posisyon",
                    timeseries: "Sa paglipas ng panahon",
                    by_measure: "Ayon sa sukat",
                },
                yaml: {
                    label: "YAML",
                    readOnly:
                        "May syntax error ang YAML. Ayusin ito sa tab na YAML para magamit muli ang mga form.",
                },
                diagnostics: {
                    title: "Mga pagsusuri",
                    none: "Walang nakitang problema",
                    localUnavailable:
                        "Hindi available ang mga pagsusuri sa browser; lalabas ang mga problema pagkatapos ng preview o Patunayan.",
                    line: "Linya {{line}}",
                    engineCode: "dbt Charts {{code}}",
                    severity: {
                        ERROR: "Error",
                        WARNING: "Babala",
                    },
                    origin: {
                        SYNTAX: "YAML syntax",
                        LOCAL: "Pagsusuri ng browser",
                        SERVER: "Pagsusuri ng server",
                    },
                },
                footer: {
                    preview: "Preview",
                    valid: "Wasto",
                    errors_one: "{{count}} error",
                    errors_other: "{{count}} error",
                    noWarnings: "walang babala sa chart",
                    warnings_one: "{{count}} babala sa chart",
                    warnings_other: "{{count}} babala sa chart",
                    rendering: "Nagre-render…",
                    previewFailed: "nabigo ang preview",
                    renderedIn: "na-render sa {{ms}} ms",
                    revision: "Rebisyon {{revision}}",
                    savedBy: "na-save noong {{date}} ni {{user}}",
                    unknownUser: "isang administrator",
                    notSaved: "hindi pa na-save",
                    unsaved: "mga pagbabagong hindi pa na-save",
                    cancel: "Kanselahin",
                    validate: "Patunayan",
                },
                preview: {
                    title: "Preview",
                    empty: "Lalabas ang preview kapag wasto na ang YAML.",
                    failed: "Hindi ma-render ang preview: {{reason}}",
                    notConnectedTail: "Walang ipapakita hanggang sa ito ay konektado.",
                    queryResult: "Resulta ng query · mga unang hilera",
                    noRows: "Walang ibinalik na hilera ang query.",
                    state: {
                        RENDERED: "Na-render",
                        NOT_CONNECTED: "Hindi konektado",
                        NO_SNAPSHOT: "Wala pang data",
                        SCOPE_PENDING: "Binibilang pa ang saklaw na ito",
                        RENDER_FAILED: "Hindi nailarawan ang chart",
                        INVALID: "Hindi wasto ang configuration",
                    },
                },
                configureWidget: {
                    title: "I-configure ang widget",
                    tabs: {
                        dataQuery: "Data at query",
                        selectors: "Mga selector",
                        yaml: "YAML",
                        preview: "Preview",
                    },
                    save: "I-save ang widget",
                    loading: "Nilo-load ang widget…",
                    loadFailed: "Hindi ma-load ang widget: {{reason}}",
                    saved: "Na-save ang widget bilang rebisyon {{revision}}",
                    refused: "Hindi na-save ang widget: ayusin ang mga nakalistang problema.",
                    validated: "Wasto ang widget.",
                    invalid: "May mga problema ang widget: tingnan ang mga pagsusuri.",
                    requestFailed: "Nabigo ang kahilingan: {{reason}}",
                    discardTitle: "Itapon ang mga pagbabago?",
                    discardBody: "Hindi pa na-save ang mga pagbabago mo sa widget na ito.",
                    discard: "Itapon",
                    keepEditing: "Ituloy ang pag-edit",
                },
                dataQuery: {
                    title: "Pamagat",
                    source: "Pinagmulan ng data",
                    template: "Query",
                    measures: "Mga sukat",
                    ratio: "Numerator / denominator",
                    numerator: "Numerator",
                    denominator: "Denominator",
                    groupBy: "Igrupo ayon sa",
                    sort: "Ayusin",
                    sortOrder: "Pagkakasunod-sunod",
                    templateOrder: "Sariling pagkakasunod-sunod ng query",
                    sortBy: {
                        label: "Label",
                        value: "Halaga",
                        ratio: "Ratio",
                    },
                    order: {
                        asc: "Pataas",
                        desc: "Pababa",
                    },
                    limit: "Mga hilera",
                    noLimit: "Lahat",
                    none: "Wala",
                    fromSelector: "Mula sa selector: {{name}}",
                    follow: "Sundin ang mga selector ng dashboard",
                    followHelp: "Hindi nililimitahan ng selector na walang tsek ang widget na ito.",
                    manyQueries:
                        "Maraming query ang widget na ito; i-edit ang mga ito sa tab na YAML.",
                    notConnected: "Hindi konektado · {{reason}}",
                    sourceHelp: {
                        DISTINCT_VOTERS:
                            "Binibilang ang mga natatanging botante; ang pinagmulan ng data ang nagtatakda ng mga patakaran sa pagbibilang.",
                        DISTINCT_PRE_ENROLLED_VOTERS:
                            "Binibilang ang mga natatanging botanteng paunang nakarehistro; ang pinagmulan ng data ang nagtatakda ng mga patakaran sa pagbibilang.",
                        LATEST_DECISION_PER_VOTER:
                            "Binibilang ang pinakahuling desisyon sa bawat botante; ang pinagmulan ng data ang nagtatakda ng mga patakaran sa pagbibilang.",
                        APPROVED_VOTERS:
                            "Binibilang ang mga aprubadong botante; ang pinagmulan ng data ang nagtatakda ng mga patakaran sa pagbibilang.",
                        POSTS_IN_SCOPE:
                            "Binibilang ang mga posisyon sa saklaw; ang pinagmulan ng data ang nagtatakda ng mga patakaran sa pagbibilang.",
                        FIRST_EVENT_PER_VOTER:
                            "Binibilang ang unang wastong boto o pag-apruba ng bawat botante; ang pinagmulan ng data ang nagtatakda ng mga patakaran sa pagbibilang.",
                        ATTEMPTS:
                            "Binibilang ang mga pagtatangka, hindi ang mga tao; ang pinagmulan ng data ang nagtatakda ng mga patakaran sa pagbibilang.",
                        DETECTIONS:
                            "Binibilang ang mga natukoy; ang pinagmulan ng data ang nagtatakda ng mga patakaran sa pagbibilang.",
                        REPORTED_ISSUES:
                            "Binibilang ang mga iniulat na isyu; ang pinagmulan ng data ang nagtatakda ng mga patakaran sa pagbibilang.",
                        default:
                            "Ang pinagmulan ng data ang nagtatakda ng mga patakaran sa pagbibilang.",
                    },
                },
                selectors: {
                    help: "Lumalabas ang mga selector sa header ng widget. Ang mga halaga nila ang nagbibigay ng input sa query; ang mga selector ng dashboard (Rehiyon, Posisyon, Bansa) ay nalalapat sa bawat widget.",
                    name: "Pangalan",
                    label: "Label",
                    control: "Kontrol",
                    controls: {
                        dropdown: "Dropdown",
                        toggle: "Toggle",
                    },
                    default: "Default",
                    optionValue: "Halaga",
                    optionLabel: "Label ng opsyon",
                    addOption: "Magdagdag ng opsyon",
                    addSelector: "Magdagdag ng selector",
                    removeOption: "Alisin ang opsyong {{option}}",
                    removeSelector: "Alisin ang selector na {{name}}",
                    moveUp: "Ilipat pataas ang {{name}}",
                    moveDown: "Ilipat pababa ang {{name}}",
                    dynamic: "Galing sa data ang mga opsyon ({{source}}).",
                    none: "Walang selector ang widget na ito.",
                    newLabel: "Bagong selector",
                    newOption: "Bagong opsyon",
                    shownWhen: "Ipinapakita kapag ang {{selector}} ay {{values}}",
                },
                conflict: {
                    title: "May nauna nang nag-save",
                    body: "Na-save ni {{user}} ang rebisyon {{revision}} ({{date}}) habang nag-e-edit ka.",
                    bodyShort: "Na-save ang rebisyon {{revision}} habang nag-e-edit ka.",
                    saved: "Na-save na rebisyon",
                    mine: "Ang mga pagbabago ko",
                    reload: "I-reload",
                    copy: "Kopyahin ang YAML ko",
                    copied: "Nasa clipboard na ang YAML mo.",
                    copyFailed:
                        "Hindi available ang clipboard; piliin ang YAML at kopyahin ito nang mano-mano.",
                    keepEditing: "Ituloy ang pag-edit",
                    removed:
                        "Inalis ang dokumento habang nag-e-edit ka. Magpatuloy sa pag-edit para i-save itong muli.",
                },
                dashboard: {
                    editing: "Ine-edit ang dashboard",
                    title: "Pamagat",
                    selectors: "Mga selector ng dashboard",
                    theme: "Tema",
                    editTheme: "I-edit ang tema",
                    addWidget: "Magdagdag ng widget",
                    cancel: "Kanselahin",
                    save: "I-save ang dashboard",
                    width: "Lapad",
                    widthValue: "{{n}} sa 12",
                    moveUp: "Ilipat pataas",
                    moveDown: "Ilipat pababa",
                    remove: "Alisin",
                    duplicate: "I-duplicate",
                    configure: "I-configure ang widget",
                    empty: "Wala pang widget ang dashboard na ito.",
                    saved: "Na-save ang dashboard bilang rebisyon {{revision}}",
                    refused: "Hindi na-save ang dashboard: ayusin ang mga nakalistang problema.",
                    requestFailed: "Nabigo ang kahilingan: {{reason}}",
                    dragHandle: "I-drag para muling ayusin ang {{title}}",
                    widgets: "Mga widget",
                    duplicated: "Na-duplicate bilang {{id}}",
                    resetToPreset: "I-reset sa preset",
                    actions: "Mga aksyon para sa {{title}}",
                    discardBody: "Hindi pa naisi-save ang iyong mga pagbabago sa dashboard na ito.",
                    duplicateInvalid: "Hindi na-save ang kopya: {{problem}}",
                    layoutMalformed:
                        "May mga item sa layout na hindi widget na may lapad. Ayusin ang mga ito sa YAML tab para maisaayos muli ang mga widget.",
                },
                catalog: {
                    title: "Magdagdag ng widget",
                    search: "Maghanap ng mga widget, pinagmulan ng data o requirement",
                    add: "Idagdag",
                    empty: "Walang tumutugmang widget.",
                    onDashboard: "Nasa dashboard na ito",
                    close: "Isara",
                },
                theme: {
                    title: "Tema ng dashboard",
                    subtitle:
                        "istilo ng dbt Charts na ilalapat sa bawat widget sa dashboard na ito",
                    appliesTo_one: "nalalapat sa {{count}} widget",
                    appliesTo_other: "nalalapat sa {{count}} widget",
                    apply: "Ilapat ang tema",
                    saved: "Na-save ang tema bilang rebisyon {{revision}}",
                    discardBody: "Hindi pa na-save ang mga pagbabago mo sa temang ito.",
                },
                reset: {
                    title: "I-reset sa preset",
                    body: "Ang bawat dashboard, widget at tema ng event na ito ay papalitan ng sa preset. Mananatili sa history ang kasalukuyang configuration.",
                    preset: "Preset",
                    confirm: "I-reset",
                    cancel: "Kanselahin",
                    done: "Gumagamit na ngayon ang event ng {{title}}.",
                    failed: "Nabigo ang pag-reset: {{reason}}",
                    noPresets: "Walang available na preset.",
                    loading: "Nilo-load ang mga preset…",
                },
                lockedDown:
                    "Naka-lockdown ang event; hindi mababago ang monitoring configuration nito.",
                document: {
                    loadFailed: "Hindi ma-load ang dokumento: {{reason}}",
                    refused: "Hindi na-save: ayusin ang mga nakalistang problema.",
                    validated: "Wasto ang dokumento.",
                    invalid: "May mga problema ang dokumento: tingnan ang mga pagsusuri.",
                    requestFailed: "Nabigo ang kahilingan: {{reason}}",
                    savedWithWarnings_one: "{{count}} babala: tingnan ang mga pagsusuri.",
                    savedWithWarnings_other: "{{count}} babala: tingnan ang mga pagsusuri.",
                },
                errors: {
                    checksUnavailable:
                        "Hindi masuri ng chart engine ang pagbabago, kaya hindi ito na-save. Subukang muli mamaya.",
                    busy: "May iba pang pagbabago sa event na ito na sine-save. Subukang muli mamaya.",
                    lockedDown:
                        "Naka-lock down ang event; hindi na mababago ang configuration ng monitoring nito.",
                    forbiddenScope:
                        "Hindi mo maaaring makita ang mga bilang para sa napiling rehiyon, posisyon o bansa.",
                    badRequest:
                        "Nagpadala ang editor ng kahilingang hindi mabasa ng server. I-reload ang pahina at subukan muli.",
                },
                duplicate: {
                    copyTitle: "{{title}} (kopya)",
                    done: "Idinagdag sa dashboard ang {{id}}, isang kopya ng widget.",
                    failed: "Hindi ma-duplicate ang widget: {{reason}}",
                    notPlaced:
                        "Nai-save ang kopyang {{id}}, pero hindi ito tinanggap ng dashboard ({{reason}}). Idagdag ito gamit ang I-edit ang dashboard.",
                },
            },
        },
        monitoringDashboardScreen: {
            voters: {
                title: "Mga Botante",
                enrolledOverseasVoters: "Mga Botanteng Nakarehistro sa Ibang Bansa",
                approvalStatus: "Status ng Aprobasyon: Mga Botanteng Aprobado/Di-Aprobado",
                manuallyApproval: "Mga Botanteng Aprobado/Di-Aprobado Nang Manu-mano",
                automaticallyApproval: "Mga Botanteng Aprobado/Di-Aprobado Nang Awtomatik",
                authenticatedVoters: "Mga Autentikadong Botante",
                invalidUserErrors: "Mga Error sa Hindi Valido ng Gumagamit:",
                invalidPasswordErrors: "Mga Error sa Hindi Valido ng Password:",
            },
            polls: {
                title: "Mga Botohan",
                initializedSystems: "Mga Post na may Inisyal na Sistema",
                votingOpened: "Mga Post na may Bukas na Pagboto",
                votingClosed: "Mga Post na may Saradong Pagboto",
                votingStarted: "Mga Post na may Nagsimula nang Pagboto",
                voterTurnout: "Pagdalo ng mga Botante",
            },
            tally: {
                title: "Pagbibilang",
                activeVotesCounting: "Mga Post na may Aktibong Pagbibilang ng mga Boto",
                generatedERs: "Mga Post na may Nabuo nang ERs",
                transmittedResults: "Mga Post na may Naitagong Resulta",
            },
            testing: {
                title: "Pagsubok",
                testElectionVoterCount: "Bilang ng mga Botante sa Pagsubok na Halalan",
            },
        },
        certificateAuthorities: {
            title: "Mga Sertipiko",
            subtitle:
                "Mga pinagkakatiwalaang awtoridad sa sertipikasyon (CA) para sa kaganapang ito ng eleksyon. Ang mga na-import na CA ay ginagamit upang mapatunayan ang mga sertipiko ng botante.",
            importButton: "Mag-import ng mga sertipiko",
            type: {
                root: "Ugat",
                intermediate: "Panggitna",
            },
            expiry: {
                expired: "Nag-expire na",
                expiringSoon: "Malapit nang mag-expire",
                valid: "Wasto",
            },
            columns: {
                commonName: "Karaniwang pangalan",
                type: "Uri",
                issuerCn: "CN ng nagbigay",
                notBefore: "Wasto mula",
                notAfter: "Mag-e-expire",
                fingerprint: "SHA256 Fingerprint",
            },
            importDialog: {
                title: "Mag-import ng mga awtoridad sa sertipikasyon",
                subtitle: "Mag-import ng isa o higit pang CA na sertipiko mula sa PEM file",
                description:
                    "Pumili ng PEM file na naglalaman ng isa o higit pang sertipiko. Sinusuportahan ang mga bundle — ang bawat sertipiko ay ina-import nang paisa-isa.",
                selectFile: "Pumili ng PEM file",
                fileLoaded: "Na-load ang file ({{bytes}} bytes)",
                importButton: "I-import",
            },
            notify: {
                importSuccess: "Na-import ang {{inserted}} sertipiko.",
                importSkipped: "{{count}} nilaktawan (mayroon na).",
                importErrors: "Mga isyu sa pag-import: {{errors}}",
                importError: "Nabigo ang pag-import: {{error}}",
                deleteSuccess: "Nabura ang sertipiko.",
                deleteError: "Error sa pagbura ng sertipiko.",
                exportSuccess: "Matagumpay na na-export ang sertipiko(s).",
                exportError: "Error sa pag-export ng mga sertipiko.",
            },
            exportDialog: {
                title: "I-export ang mga awtoridad sa sertipikasyon",
                description: "Ie-export mo ang {{amount}} sertipiko.",
                all: "lahat",
            },
            deleteDialog: {
                description: "Sigurado ka bang gusto mong burahin ang {{count}} sertipiko?",
            },
            emptyHeader:
                "Walang mga awtoridad sa sertipikasyon na na-import para sa electoral event na ito.",
            fileReadError: "Nabigo ang pagbasa ng file.",
            viewDialog: {
                title: "Mga detalye ng awtoridad sa sertipikasyon",
                subject: "Paksa",
                issuer: "Nagbigay",
                serialNumber: "Serial number",
                pemContent: "Nilalaman ng PEM",
            },
            confirmDelete: "Burahin ang awtoridad sa sertipikasyon",
            confirmDeleteDescription:
                'Sigurado ka bang nais mong burahin ang sertipikong "{{name}}" (fingerprint: {{fingerprint}})?',
        },
        signing: {
            terms: {
                post: "Post",
                posts: "Mga Post",
            },
            tab: {
                title: "Mga Pirma",
                intro: "Tumatakbo lamang ang mga protektadong aksyon kapag sapat na awtorisadong tao ang pumirma sa mga ito gamit ang kanilang digital na sertipiko. Sinusuri ang bawat pirma laban sa mga pinagkakatiwalaang issuer at itinatala sa log.",
                protectedActions: "Mga protektadong aksyon",
                certificates: "Mga Sertipiko",
                requests: "Mga Kahilingan",
            },
            loadError:
                "Hindi ma-load ang mga setting ng pagpirma. I-reload ang pahina para subukang muli.",
            errors: {
                automatedCeremonies:
                    "Gumagamit ang event na ito ng mga awtomatikong seremonya ng susi. Hindi ginagawa ng mga trustee ang mga hakbang na ito, kaya hindi maaaring hingin ang kanilang mga lagda. Gumamit ng mga manwal na seremonya ng susi upang hingin ang mga lagda ng trustee.",
                forbidden: "Wala kang pahintulot para sa pagbabagong ito.",
                invalid:
                    "Tinanggihan ng server ang mga value na ito. Suriin ang mga ito at subukang muli.",
                conflict:
                    "May ibang nagbago nito habang ginagawa mo ito. I-reload ang pahina at subukang muli.",
                lockedDown:
                    "Naka-lock ang kaganapan sa halalan: nagbabago lamang ang mga patakaran sa pagpirma sa pamamagitan ng bagong bersyon ng configuration.",
                notFound: "Wala na ito. I-reload ang pahina.",
            },
            readOnly: {
                chip: "Pagbasa lamang",
                rules: "Pagbasa lamang. Kailangan ang pahintulot na “Mga Pirma: i-edit ang mga protektadong aksyon” para baguhin ang mga patakaran sa pagpirma.",
                whoCanSign:
                    "Mga tungkulin na may pahintulot na “Pirmahan: {{action}}” sa Mga Tagagamit at Tungkulin. Kailangan ng pahintulot na mag-edit ng mga tungkulin para baguhin ang mga ito.",
            },
            groups: {
                "voting": "Pagboto",
                "results-and-reports": "Mga resulta at ulat",
                "enrollment": "Pagpapatala",
                "configuration-and-keys": "Configuration at mga susi",
            },
            actions: {
                "initialize-voting": {
                    label: "I-initialize ang pagboto",
                    short: "Initialization",
                    permissionName: "i-initialize ang pagboto",
                    object: "initialization ng pagboto",
                    appliesTo: "Bawat $t(signing.terms.post)",
                    description:
                        "Sinisimulan sa I-publish. Ini-initialize ang $t(signing.terms.post) at binubuo ang Initialization Report nito.",
                },
                "open-voting": {
                    label: "Buksan ang pagboto",
                    short: "Pagbubukas",
                    permissionName: "buksan ang pagboto",
                    object: "pagbubukas ng pagboto",
                    appliesTo: "Bawat $t(signing.terms.post)",
                    description:
                        "Sinisimulan sa I-publish gamit ang Simulan ang Pagboto. Binubuksan ang pagboto sa $t(signing.terms.post).",
                },
                "close-voting": {
                    label: "Isara ang pagboto",
                    short: "Pagsasara",
                    permissionName: "isara ang pagboto",
                    object: "pagsasara ng pagboto",
                    appliesTo: "Bawat $t(signing.terms.post)",
                    description:
                        "Sinisimulan sa I-publish gamit ang Itigil ang Pagboto. Isinasara ang pagboto sa $t(signing.terms.post); itinatago sa record nito ang mga pirma ng pagsasara.",
                    descriptionSealed:
                        "Sinisimulan sa I-publish gamit ang Itigil ang Pagboto. Isinasara ang pagboto sa $t(signing.terms.post). Kapag sarado na ang bawat channel, sine-seal ang mga ballot box nito, pagkatapos ng grace period kung mayroon: wala nang balotang maidadagdag, mababago o mabubura, at hindi na muling masisimulan ang pagboto. Itinatago sa record nito ang mga pirma ng pagsasara.",
                },
                "generate-election-returns": {
                    label: "Bumuo ng election returns",
                    short: "Election returns",
                    permissionName: "bumuo ng election returns",
                    object: "election returns",
                    appliesTo: "Bawat $t(signing.terms.post) at bansa",
                    description:
                        "Sinisimulan ng tally, isang kahilingan bawat $t(signing.terms.post) at bansa. Inilalabas ang pinirmahang election returns para i-print at ipadala.",
                },
                "generate-reports": {
                    label: "Bumuo ng iba pang ulat ng halalan",
                    short: "Ulat",
                    permissionName: "bumuo ng iba pang ulat ng halalan",
                    object: "ulat",
                    appliesTo: "Bawat $t(signing.terms.post)",
                    description:
                        "Sinisimulan ng tally para sa Initialization Report at sa Mga Ulat para sa ulat ng partisipasyon. Inilalabas ang pinirmahang ulat.",
                },
                "transmit-results": {
                    label: "I-transmit ang mga resulta",
                    short: "Transmisyon",
                    permissionName: "i-transmit ang mga resulta",
                    object: "results package",
                    appliesTo: "Bawat $t(signing.terms.post) at bansa",
                    description:
                        "Sinisimulan sa Tally, Transmisyon. Binubuo ang pinirmahang results package para sa mga destinasyon nito; pinupunan ng mga pirma ang listahan ng pirma nito.",
                },
                "approve-voter": {
                    label: "Manwal na aprubahan ang isang botante",
                    short: "Pag-apruba ng botante",
                    permissionName: "manwal na aprubahan ang isang botante",
                    object: "pag-apruba ng botante",
                    appliesTo: "Ang $t(signing.terms.post) ng botante",
                    description:
                        "Sinisimulan sa Approvals. Inaaprubahan ang botante at ibinibigay ang kanyang mga credential.",
                },
                "approve-configuration": {
                    label: "Aprubahan ang isang bersyon ng configuration",
                    short: "Bersyon ng configuration",
                    permissionName: "aprubahan ang isang bersyon ng configuration",
                    object: "bersyon ng configuration",
                    appliesTo: "Ang kaganapan sa halalan",
                    description:
                        "Sinisimulan sa I-publish. Ipina-publish ang bersyon ng configuration.",
                },
                "key-ceremony": {
                    label: "Kumpirmahin ang isang piraso ng susi (seremonya ng mga susi)",
                    short: "Piraso ng susi",
                    permissionName: "kumpirmahin ang isang piraso ng susi",
                    object: "piraso ng susi",
                    appliesTo: "Bawat trustee",
                    description:
                        "Sinisimulan ng bawat trustee sa Mga Susi. Itinatala ang pirma ng trustee sa seremonya at sa bulletin board.",
                },
                "tally-key": {
                    label: "Iambag ang isang piraso ng susi (tally)",
                    short: "Ambag na piraso ng susi",
                    permissionName: "iambag ang isang piraso ng susi",
                    object: "ambag na piraso ng susi",
                    appliesTo: "Bawat trustee",
                    description:
                        "Sinisimulan ng bawat trustee sa Tally. Itinatala ang ambag ng trustee.",
                },
            },
            protectedActions: {
                intro: "Ginagawa ang bawat pirma gamit ang digital na sertipiko sa security token ng pumipirma.",
                columns: {
                    action: "Aksyon",
                    appliesTo: "Para sa",
                    whoCanSign: "Sino ang maaaring pumirma",
                    signaturesNeeded: "Kailangang pirma",
                    requestExpires: "Pag-expire ng kahilingan",
                    waiting: "Naghihintay",
                },
                off: "Naka-off",
                eachTrustee: "Bawat trustee",
                footerVersion:
                    "Bahagi ang mga patakaran sa pagpirma ng bersyon {{version}} ng configuration ng kaganapang ito.",
                footerFirstVersion:
                    "Magiging bahagi ang mga panuntunan sa pagpirma ng unang bersyon ng configuration ng event na ito kapag ito ay na-publish.",
                footerChanged: "Huling binago noong {{date}}.",
                footerChangedBy: "Huling binago noong {{date}} ni {{name}}.",
                lockedDown:
                    "Naka-lock ang kaganapan sa halalan: kabilang ang mga patakaran nito sa pagpirma sa bersyon nito ng configuration, kaya nagbabago lamang ang mga ito sa pamamagitan ng bagong bersyon ng configuration.",
                edit: "I-edit ang {{action}}",
                view: "Tingnan ang {{action}}",
                waitingCount_one: "{{count}} kahilingan ang naghihintay",
                waitingCount_other: "{{count}} kahilingan ang naghihintay",
                capacityError:
                    "Hindi ma-load kung sino ang maaaring pumirma, kaya hindi masuri ang bilang ng pirma laban sa $t(signing.terms.posts).",
            },
            expiry: {
                "30": "30 minuto",
                "60": "1 oras",
                "120": "2 oras",
                "1440": "24 oras",
                "none": "Walang limitasyon",
                "other": "{{count}} minuto",
            },
            rule: {
                needsSignatures: "Kailangan ng mga pirma",
                whoCanSign: "Sino ang maaaring pumirma",
                whoCanSignHelp:
                    "Nakakakuha ang mga tungkuling ito ng pahintulot na “Pirmahan: {{action}}” sa Mga Tagagamit at Tungkulin, para sa bawat kaganapan sa halalan. Kailangan ding may access ang mga pumipirma sa $t(signing.terms.post).",
                signaturesNeeded: "Kailangang pirma",
                signaturesNeededHelp:
                    "Ginagamit ng bawat pumipirma ang kanyang digital na sertipiko. May hindi bababa sa {{n}} taong maaaring pumirma ang bawat $t(signing.terms.post).",
                signaturesNeededShortHelp:
                    "Ginagamit ng bawat pumipirma ang kanyang digital na sertipiko.",
                requesterSigning: "Maaari ring pumirma ang taong nagsimula nito",
                expiresAfter: "Mag-e-expire ang kahilingan pagkalipas ng",
                trusteesSign: "Pumipirma ang mga trustee sa hakbang na ito",
                trusteesHelp:
                    "Pinipirmahan ng bawat trustee ang sarili niyang hakbang gamit ang kanyang digital na sertipiko. Itinatakda ng seremonya ng mga susi kung ilang trustee ang lalahok.",
                footer: "Itinatala ang mga pagbabago sa log ng kaganapan sa halalan at nagiging bahagi ng susunod na bersyon ng configuration.",
                cancel: "Kanselahin",
                save: "I-save",
                saved: "Na-save ang patakaran sa pagpirma.",
                savedShort_one:
                    "Na-save ang patakaran sa pagpirma. Hindi pa maaabot ng {{posts}} ang bilang: magdagdag ng pumipirma doon.",
                savedShort_other:
                    "Na-save ang patakaran sa pagpirma. Hindi pa maaabot ng {{posts}} ang bilang: magdagdag ng mga pumipirma doon.",
                checkedOnSave:
                    "Sinusuri ang bilang laban sa mga bagong tungkulin kapag nag-save ka.",
                savedRequesterShort:
                    "Na-save ang patakaran sa pagpirma. Hindi maaabot ng ilan sa $t(signing.terms.posts) ang bilang nang wala ang taong nagsisimula ng kahilingan.",
                saveError:
                    "Hindi ma-save ang patakaran sa pagpirma. Maaaring may ibang nagbago nito habang ginagawa mo ito; i-reload at subukang muli.",
            },
            validation: {
                atLeastOne: "Hindi bababa sa 1.",
                tooMany:
                    "Walang $t(signing.terms.post) na may {{n}} taong maaaring pumirma. Ang pinakamarami ay {{max}}.",
                tooManyEvent:
                    "{{max}} tao lamang ang maaaring pumirma nito. Pumili ng hindi hihigit sa {{max}}.",
                atMost: "Hindi hihigit sa {{max}}.",
                shortPosts_one:
                    "{{n}} tao lamang ang maaaring pumirma sa {{posts}}, kaya hindi nito maaabot ang {{required}} pirma. Magdagdag ng pumipirma doon o babaan ang bilang.",
                requesterShort_one:
                    "Kung wala ang taong nagsimula nito, {{n}} tao lamang ang maaaring pumirma sa {{posts}}, kaya hindi nito maaabot ang {{required}} pirma.",
                requesterShort_other:
                    "Kung wala ang taong nagsimula nito, {{n}} tao lamang ang maaaring pumirma sa {{posts}}, kaya hindi nila maaabot ang {{required}} pirma.",
                shortPosts_other:
                    "{{n}} tao lamang ang maaaring pumirma sa {{posts}}, kaya hindi nila maaabot ang {{required}} pirma. Magdagdag ng pumipirma doon o babaan ang bilang.",
            },
            pendingRequests_one:
                "{{count}} kahilingan ang naghihintay ng mga pirma sa ilalim ng kasalukuyang patakaran. Kakanselahin ito ng pag-save; magsisimulang muli ang taong nagsimula nito.",
            pendingRequests_other:
                "{{count}} kahilingan ang naghihintay ng mga pirma sa ilalim ng kasalukuyang patakaran. Kakanselahin ang mga ito ng pag-save; magsisimulang muli ang mga taong nagsimula ng mga ito.",
            certificates: {
                issuersIntro:
                    "Dapat naka-chain sa isa sa mga ito ang mga sertipiko ng staff. Hiwalay ang mga ito sa mga sertipikong ginagamit ng mga botante sa pag-sign in.",
                checkRevocation: "Suriin ang mga revocation list",
                crlUnavailable: {
                    "label": "Kapag hindi ma-download ang isang listahan",
                    "refuse": "Huwag tanggapin ang mga pirma",
                    "accept-unchecked": "Tanggapin at markahan ang pirma bilang hindi nasuri",
                },
                registration: {
                    "label": "Pagrehistro ng sertipiko sa isang tao",
                    "on-first-use": "Kapag unang pumirma ang may-ari nito gamit ito",
                    "security-officer-only":
                        "Kapag nirehistro lamang ito ng taong maaaring magrehistro ng mga sertipiko",
                },
                onePost:
                    "Pumipirma ang isang sertipiko para sa iisang $t(signing.terms.post) lamang",
                issuers: "Mga pinagkakatiwalaang issuer",
                import: "Mag-import ng mga sertipiko ng issuer",
                importHelp:
                    "Pumili ng PEM o CER file na may sertipiko ng issuer. Maaaring maglaman ang isang PEM file ng ilang sertipiko.",
                chooseFile: "Pumili ng file ng sertipiko",
                fileError: "Hindi mabasa ang file.",
                imported:
                    "Na-import ang {{imported}} sertipiko ng issuer; {{skipped}} ang pinagkakatiwalaan na.",
                importedWithErrors:
                    "Na-import ang {{imported}} sertipiko ng issuer, {{skipped}} ang pinagkakatiwalaan na. Tinanggihan: {{errors}}",
                importError: "Hindi ma-import ang mga sertipiko ng issuer.",
                deleteIssuer: "Alisin ang {{name}}",
                deleteIssuerConfirm:
                    "Alisin ang {{name}} sa mga pinagkakatiwalaang issuer? Hindi na makakapirma ang mga sertipikong inisyu nito.",
                deleteError: "Hindi maalis ang issuer.",
                noIssuers:
                    "Wala pang pinagkakatiwalaang issuer. Hindi makakapirma ang staff hangga't walang na-import.",
                root: "Root",
                intermediate: "Intermediate",
                columns: {
                    issuer: "Issuer",
                    type: "Uri",
                    issuedBy: "Inisyu ni",
                    validUntil: "Valid hanggang",
                    sha256: "SHA-256",
                    person: "Tao",
                    post: "$t(signing.terms.post)",
                    certificate: "Sertipiko",
                    registered: "Nakarehistro",
                    status: "Katayuan",
                },
                checks: "Mga pagsusuri",
                checksSaved: "Na-save ang mga pagsusuri sa sertipiko.",
                checksError: "Hindi ma-save ang mga pagsusuri sa sertipiko.",
                crlSchedule: "Dina-download mula sa bawat issuer kada oras.",
                crlUpdated: "{{url}}: na-update {{time}}",
                crlFailed: "{{url}}: hindi ma-download (huling subok {{time}})",
                registeredTitle: "Mga nakarehistrong sertipiko",
                search: "Maghanap ng mga tao, sertipiko o $t(signing.terms.posts)",
                status: "Katayuan",
                statusAll: "Lahat",
                statuses: {
                    "active": "Aktibo",
                    "expires-soon": "Malapit nang mag-expire",
                    "expired": "Nag-expire",
                    "revoked": "Binawi",
                },
                revokedOn: "Binawi noong {{date}}",
                allPosts: "Lahat",
                noCertificates: "Walang nakarehistrong sertipiko.",
                registeredHow: {
                    "first-use": "Sa unang pirma",
                    "security-officer": "Nirehistro ng isang administrator",
                },
                register: "Magrehistro ng sertipiko",
                registerSubmit: "Irehistro",
                registerDone: "Nairehistro ang sertipiko.",
                registerError: "Hindi mairehistro ang sertipiko.",
                person: "Tao",
                personSearchHelp: "Mag-type ng bahagi ng username para mahanap ang tao.",
                registeredBy: "Ni {{name}}",
                registerRefused:
                    "Hindi mairehistro ang sertipikong ito: tiyaking inisyu ito ng pinagkakatiwalaang issuer, valid ito ngayon at ginawa ito para sa pagpirma.",
                registeredToOther:
                    "Nakarehistro ang sertipikong ito kay {{name}}. Kung kay {{name}} din ang account na ito, i-link ito bilang kanyang pangalawang account.",
                linkAccount: "I-link bilang pangalawang account ng parehong tao",
                alreadyRegistered: "Nakarehistro na ang sertipikong ito sa taong ito.",
                pem: "Sertipiko (PEM)",
                revoke: "Bawiin",
                revokeOf: "Bawiin ang sertipiko ni {{name}}",
                revokeTitle: "Bawiin ang sertipiko ni {{name}}",
                revokeHelp:
                    "Hindi na makakapirma ang binawing sertipiko. Bilang pa rin ang mga pirmang nagawa na nito.",
                revokeReason: "Dahilan",
                revokeDone: "Nabawi ang sertipiko.",
                revokeError: "Hindi mabawi ang sertipiko.",
            },
            requests: {
                exportCsv: "I-export ang CSV",
                exportError: "Hindi ma-export ang mga kahilingan.",
                exportFileName: "signing-requests.csv",
                status: "Katayuan",
                statusAll: "Lahat",
                statusCount: "{{status}} · {{count}} sa {{total}}",
                expires: "Mag-e-expire {{time}}",
                lastSignatureBy: "{{name}}, {{time}}",
                empty: "Wala pang kahilingan sa pagpirma.",
                columns: {
                    request: "Kahilingan",
                    status: "Katayuan",
                    started: "Sinimulan",
                    by: "Ni",
                    lastSignature: "Huling pirma",
                    code: "Code",
                },
            },
            reports: {
                postRequired:
                    "Pumili ng Post upang buuin ang ulat na ito kapag kailangan ng mga lagda.",
                generateNotice:
                    "{{post}}: binubuo na ngayon ang dokumento. Maaari itong i-print at i-transmit kapag napirmahan na ito ng {{n}} tao.",
            },
            status: {
                waiting: "Naghihintay",
                completed: "Napirmahan",
                executed: "Tapos na",
                cancelled: "Kinansela",
                expired: "Nag-expire",
                failed: "Nabigo",
            },
            cancelReasons: {
                "by-requester": "Kinansela ito ng taong nagsimula nito",
                "by-operator": "Kinansela ito ng isang operator",
                "rule-changed": "Nagbago ang patakaran sa pagpirma ng aksyon",
                "payload-changed": "Nagbago ang pinipirmahan nito",
                "superseded": "Pinalitan ito ng mas bagong kahilingan",
                "certificate-revoked": "Binawi ang isang sertipikong pumirma rito",
            },
            panel: {
                rulePost:
                    "Kailangan ng {{n}} pirma mula sa mga pumipirma ng {{post}}, bawat isa gamit ang kanilang digital na sertipiko.",
                ruleEvent:
                    "Kailangan ng {{n}} pirma, bawat isa gamit ang digital na sertipiko ng pumipirma.",
                signingCode: "Code ng pagpirma",
                signers: "Mga pumipirma",
                sign: "Pirmahan",
                handover: "Susunod na miyembro ang mag-sign in",
                cancel: "Kanselahin ang kahilingan",
                signedAt: "Pinirmahan {{time}}",
                notSigned: "Hindi pa napirmahan",
                certificate: "Sertipiko {{name}}",
                you: "(ikaw)",
                expiresAt: "Mag-e-expire nang {{time}}",
                progress: "{{count}} sa {{total}}",
                openDocument: "Buksan ang dokumento",
                configurationVersion: "Bersyon ng configuration {{version}}",
                configurationChanges: "Mga pagbabago sa bersyong ito",
            },
            dialog: {
                title: "Pirmahan ang {{object}}",
                steps: {
                    check: "Suriin",
                    certificate: "Sertipiko",
                    signed: "Napirmahan",
                },
                localNote:
                    "Nangyayari ang pagpirma sa browser na ito. Hindi kailanman ipinapadala ang iyong file ng sertipiko, ang private key nito at ang password nito. Ang iyong pirma at pampublikong sertipiko lamang ang napupunta sa server.",
                check: {
                    signingAs: "Pumipirma ka bilang {{name}}",
                    titlePost: "{{title}}, {{post}}",
                    sameCode: "Iisang code ang nakikita ng lahat ng pumipirma.",
                    confirmDocument: "Nasuri ko na ang {{object}}",
                },
                certificate: {
                    intro: "Ipasok ang iyong security token at piliin ang iyong file ng sertipiko.",
                    password: "Password ng sertipiko",
                    open: "Buksan ang sertipiko",
                    chooseAnother: "Pumili ng ibang file",
                },
                checks: {
                    "passed": {
                        "trusted-issuer": "Inisyu ng pinagkakatiwalaang issuer ({{root}})",
                        "valid-now": "Valid ngayon",
                        "signing-key-usage": "Ginawa para sa pagpirma",
                        "not-revoked": "Hindi binawi (na-update ang mga listahan {{time}})",
                        "registered": "Nakarehistro sa iyo noong {{date}}",
                        "registered-to-other": "Hindi nakarehistro sa ibang tao",
                        "already-signed": "Hindi pa nagamit para sa kahilingang ito",
                        "post-binding": "Nakarehistro para sa $t(signing.terms.post) na ito",
                        "signature": "Saklaw ng pirma ang kahilingang ito",
                    },
                    "failed": {
                        "trusted-issuer": "Hindi inisyu ng pinagkakatiwalaang issuer",
                        "valid-now": "Hindi valid ngayon",
                        "signing-key-usage": "Hindi ginawa para sa pagpirma",
                        "not-revoked":
                            "Binawi, o walang kasalukuyang revocation list para suriin ito",
                        "registered": "Hindi nakarehistro sa iyo",
                        "registered-to-other": "Nakarehistro kay {{name}}",
                        "already-signed": "Nagamit na para sa kahilingang ito",
                        "post-binding": "Nakarehistro para sa ibang $t(signing.terms.post)",
                        "signature": "Hindi saklaw ng pirma ang kahilingang ito",
                    },
                    "first-use": "Unang paggamit: irerehistro ito sa iyo",
                },
                problems: {
                    wrongPassword: "Maling password. Suriin ito at subukang muli.",
                    notForYou:
                        "Hindi makakapirma ang sertipikong ito para sa iyo. Gamitin ang sertipiko sa sarili mong security token.",
                    issuerNotAccepted:
                        "Gamitin ang sertipikong nirehistro ng {{organization}} para sa iyo. Hindi tinatanggap ang mga sertipiko mula sa ibang issuer.",
                    cancelled:
                        "Kinansela ang kahilingang ito: {{reason}}. Hindi na bilang ang mga pirmang ibinigay para dito. Simulan itong muli para pirmahan ang kasalukuyang bersyon.",
                },
                signed: {
                    title: "Napirmahan",
                    withCertificate: "gamit ang sertipiko ni {{name}}",
                    count: "{{n}} sa {{total}} pirma.",
                    allIn: "Kumpleto na ang lahat ng {{total}} pirma.",
                    next: "Susunod na pipirma: {{names}}.",
                },
                handover:
                    "Mala-log out ka. Mag-sign in ang susunod na miyembro sa computer na ito at babalik sa kahilingang ito para pumirma. Bukas ang kahilingan hanggang {{time}}.",
                sign: "Pirmahan",
                back: "Bumalik",
                cancel: "Kanselahin",
            },
            widget: {
                continue: "Magpatuloy",
                done: "Tapos na",
                close: "Isara",
                retry: "Subukang muli",
                loading: "Nilo-load ang kahilingan…",
                loadError: "Hindi ma-load ang kahilingan.",
                chooseFile: "Pumili ng file ng sertipiko",
                fileInput: "File ng sertipiko",
                fileSize: "{{size}} KB",
                showPassword: "Ipakita ang password",
                hidePassword: "Itago ang password",
                opening: "Binubuksan ang sertipiko…",
                checking: "Sinusuri ang sertipiko…",
                signing: "Pumipirma…",
                certificateCard: "Inisyu ni {{issuer}} · valid hanggang {{date}} · {{algorithm}}",
                fingerprint: "SHA-256 {{fingerprint}}",
                algorithms: {
                    "rsa-pkcs1-sha256": "RSA",
                    "ecdsa-p256-sha256": "EC P-256",
                },
                document: "{{type}} · SHA-256 {{hash}}",
                documentPages: "{{type}} · {{pages}} pahina · SHA-256 {{hash}}",
                checksTitle: "Mga pagsusuri sa sertipiko",
                untrustedIssuer:
                    "Hindi pinagkakatiwalaang issuer ang {{issuer}} para sa kaganapang ito sa halalan",
                registeredToSomeoneElse: "Nakarehistro sa ibang tao",
                checkPassedNoDetail: {
                    "trusted-issuer": "Inisyu ng pinagkakatiwalaang issuer",
                    "not-revoked": "Hindi binawi",
                },
                organization: "iyong organisasyon",
                cantSign: "Hindi mapipirmahan ng sertipikong ito ang kahilingang ito.",
                checkError: "Hindi masuri ang sertipiko. Subukang muli.",
                fileErrors: {
                    UNREADABLE_FILE:
                        "Hindi file ng sertipiko (.p12 o .pfx) ang file na ito, o sira ito.",
                    UNSUPPORTED_ENCRYPTION:
                        "Hindi mabuksan ng browser na ito ang encryption na ginagamit ng file na ito.",
                    NO_PRIVATE_KEY:
                        "Walang private key ang file na ito. Piliin ang file ng sertipiko mula sa iyong security token.",
                    NO_CERTIFICATE: "Walang sertipiko ang file na ito.",
                    UNSUPPORTED_KEY:
                        "Hindi suportado ang uri ng key ng sertipikong ito. Gumamit ng RSA o EC P-256 na sertipiko.",
                    KEY_CERTIFICATE_MISMATCH:
                        "Hindi tugma ang sertipiko sa file na ito sa key nito.",
                },
                openError: "Hindi mabuksan ang sertipiko. Subukang muli.",
                signError: "Hindi maipadala ang pirma. Subukang muli.",
                refused: "Tinanggihan ng server ang pirma.",
                stale: "Nagbago ang dokumento habang pumipirma ka. Pumirmang muli.",
                mismatch:
                    "Hindi tugma sa kahilingang ito ang pipirmahan. Isara ang dialog at buksang muli ang kahilingan.",
                documentMismatch:
                    "Hindi tugma ang dokumento sa dokumentong pinipirmahan ng kahilingang ito.",
                documentError: "Hindi ma-download ang dokumento. Subukang muli.",
                alreadySigned: "Napirmahan mo na ang kahilingang ito.",
                closed: {
                    changed:
                        "Nagbago ang kahilingang ito pagkatapos mo itong buksan. Isara ang window na ito at suriin itong muli bago ka pumirma.",
                    allSigned: "Kumpleto na ang lahat ng pirma ng kahilingang ito.",
                },
                chooseCertificate: "Sertipikong gagamitin sa pagpirma",
                renderError:
                    "Hindi maipakita ang kahilingan sa pagpirma. Isara ito at buksang muli.",
                signedAt: "{{time}}",
                panel: {
                    completedAt: "Napirmahan nang {{time}}",
                    expired:
                        "Nag-expire ang kahilingang ito. Hindi na bilang ang mga pirmang ibinigay para dito. Simulan itong muli para pumirma.",
                    failed: "Kumpleto na ang lahat ng pirma, pero nabigo ang aksyon. Nasa log ang mga detalye.",
                    details: "Mga detalye",
                    close: "Isara ang panel ng kahilingan",
                },
                cancelDialog: {
                    title: "Kanselahin ang kahilingang ito?",
                    body: "Hindi na bilang ang mga pirmang ibinigay para dito. Magsisimulang muli ang taong nagsimula nito.",
                    reason: "Dahilan (opsyonal)",
                    confirm: "Kanselahin ang kahilingan",
                    back: "Panatilihin ito",
                    error: "Hindi makansela ang kahilingan. Subukang muli.",
                },
                handoverDialog: {
                    title: "Susunod na miyembro ang mag-sign in",
                    noExpiry:
                        "Mala-log out ka. Mag-sign in ang susunod na miyembro sa computer na ito at babalik sa kahilingang ito para pumirma.",
                    confirm: "Mag-sign out",
                    back: "Manatiling naka-sign in",
                    error: "Hindi maitala ang handover. Subukang muli.",
                },
            },
            details: {
                keys_ceremony_id: "Seremonya",
                tally_session_id: "Tally session",
                trustee_id: "Trustee",
                key_share_sha256: "SHA-256 ng piraso ng susi",
                channel: "Channel",
                channels: "Mga channel",
                publication_id: "Paglalathala ng balota",
                ballot_publication_id: "Paglalathala ng balota",
                digest: "SHA-256 ng configuration",
                signing_rules: "Mga patakaran sa pagpirma",
                scheduled_events: "Mga bagong nakaiskedyul na kaganapan",
                ballots_and_contests: "Mga balota at paligsahan",
                application_id: "Aplikasyon",
                applicant_registry_id: "Account sa registry",
                decision: "Desisyon",
                submitted_at: "Isinumite",
                reason: "Bakit kailangan ng tao",
                registry_record: "Record sa registry",
                status: "Katayuan ng aplikasyon",
                from: "Dating katayuan",
            },
            closed: {
                pending: "Kumpleto na ang lahat ng pirma. Magsasara ang pagboto sa ilang sandali.",
                title: "Nagsara ang pagboto nang {{time}}.",
                titleSealed: "Nagsara ang pagboto nang {{time}}. Naka-seal ang mga balota.",
                record: "Seal record",
                ballots: "Mga balota sa seal",
                sealHash: "{{algorithm}} ng seal",
                signedBy: "Pinirmahan ni",
                signatures: "Mga pirma ng pagsasara sa seal record",
                signaturesValue_one: "{{count}}, code ng pagpirma {{code}}",
                signaturesValue_other: "{{count}}, code ng pagpirma {{code}}",
                signers: "Pinirmahan ng mga miyembro",
            },
            values: {
                ballots_and_contests: {
                    "first-version": "Unang bersyon",
                    "no-changes": "Walang pagbabago",
                    "changed": "Binago",
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
                    ONLINE: "Online",
                    KIOSK: "Kiosk",
                    EARLY_VOTING: "Maagang pagboto",
                    TELEPHONE: "Telepono",
                },
                statuses: {
                    NOT_STARTED: "Hindi pa nagsisimula",
                    OPEN: "Bukas",
                    PAUSED: "Naka-pause",
                    CLOSED: "Sarado",
                },
                channelStatus: "{{channel}}: {{status}}",
                ruleChange: "{{action}}: {{rule}}",
                ruleChangeFrom: "{{action}}: {{rule}} (dati {{was}})",
                ruleNeeds: "kailangan ng {{n}}",
                ruleOff: "naka-off",
                decision: {
                    approve: "Aprubahan",
                },
            },
            results: {
                signatures: "Mga Pirma",
                needs: "Kailangan ng {{n}}",
                off: "Naka-off",
                openRequest: "Buksan ang kahilingan sa pagpirma",
                downloadSigned: "I-download ang pinirmahang PDF",
                print: "I-print",
                transmit: "I-transmit ang mga resulta",
                sendTo: "Ipadala sa {{count}} destinasyon",
                awaiting: "{{item}}: naghihintay ng mga pirma",
                transmission: {
                    title: "Mga Pirma",
                    description:
                        "Pinipirmahan ng bawat pumipirma ang mga resulta ng package gamit ang kanyang digital na sertipiko, sa browser na ito. Maaaring ipadala ang package kapag napirmahan na ito ng {{n}} tao.",
                    waiting:
                        "Maaaring ipadala ang package kapag kumpleto na ang lahat ng pirma ng kahilingan nito sa pagpirma.",
                    signed: "Dala ng package ang lahat ng pirma nito at maaari na itong ipadala.",
                    ended: "Natapos na ang kahilingan sa pagpirma ng package na ito. Gawin muli ang package para pirmahan ito.",
                },
            },
            waiting: {
                title: "Naghihintay ng aking pirma",
                buttonCount_one: "Naghihintay ng aking pirma: {{count}} kahilingang pipirmahan",
                buttonCount_other: "Naghihintay ng aking pirma: {{count}} kahilingang pipirmahan",
                intro: "Mga kahilingang naghihintay ng mga pirma para sa mga aksyong maaari mong pirmahan, sa iyong mga $t(signing.terms.posts).",
                close: "Isara ang listahan",
                empty: "Walang naghihintay ng iyong pirma.",
                loadError: "Hindi ma-load ang mga kahilingang naghihintay ng mga pirma.",
                signedByYou: "Pinirmahan mo",
            },
            notes: {
                afterApproval: "Pagkatapos ng pag-apruba",
                afterApprovalValue:
                    "Ibinibigay at ipinapadala sa botante ang kanyang mga credential",
                keyShare: "Ang iyong piraso ng susi",
                keyShareChecked: "Nasuri: ito ang iyong piraso ng susi para sa seremonyang ito",
                recordedIn: "Itinatala sa",
                recordedInCeremony: "Ang seremonya ng mga susi at ang bulletin board",
                recordedInTally: "Ang tally session",
            },
            keyShare: {
                signing:
                    "Pirmahan ang iyong piraso ng susi sa panel ng pagpirma. Itatala ito kapag napirmahan mo na.",
                record: "Itala ang aking piraso ng susi",
                failed: "Hindi maitala ang iyong pinirmahang piraso ng susi: {{error}}",
                dropAgain:
                    "I-drop muli ang file ng iyong piraso ng susi para maitala ang iyong pinirmahang piraso ng susi.",
                redo: "Naiambag ang iyong piraso ng susi nang walang pirma mo, na kailangan na ngayon ng halalang ito. Iambag itong muli at pirmahan.",
                notTaken:
                    "Hindi na tinatanggap ng seremonya ang piraso ng susi na ito. I-drop muli ang file ng iyong piraso ng susi.",
            },
        },
        lifecycle: {
            signedClose: {
                title: "Nilagdaang takdang pagsasara",
                deadline: "{{election}}: {{time}} · pinahintulutan ng configuration {{code}}.",
                explanation:
                    "Mananatiling masusunod ang nilagdaang takdang oras na ito kahit baguhin o alisin ang nae-edit na iskedyul. Isinasara ng tagaiskedyul ang mga pinahintulutang channel na bukas pa.",
                reached:
                    "Lumipas na ang nilagdaang takdang oras na ito. Suriin ang kasalukuyang kalagayan ng pagboto at ang audit log; hindi pa naitala ang pagproseso.",
                processed: "Naproseso ang nilagdaang takdang pagsasara noong {{time}}.",
                signedAt: "Nilagdaang takdang oras: {{time}}.",
                channels: "Mga channel na saklaw pa rin ng takdang oras na ito: {{channels}}.",
                result: "Tingnan ang kalagayan ng pagboto at ang audit log para sa aktuwal na mga pagbabago at kumpletong resulta.",
                unavailable:
                    "Hindi ma-load ang mga nilagdaang takdang pagsasara. Suriin ang nailathalang iskedyul at ang audit log.",
            },
            picker: {
                noMatch:
                    "Walang tumugmang timezone. Mag-type ng lungsod, bansa, zone, daglat o offset.",
            },
            input: {
                timezone: "Timezone",
                scheduledAt: "Nakaiskedyul sa",
                meetingStart: "Simula ng pulong",
                cronZone: "Tumatakbo ang iskedyul sa pangunahing timezone ng event, {{zone}}.",
                unconfiguredZone:
                    "Ang {{zone}} ay hindi isa sa mga naka-configure na timezone ng event. Pumili ng isa sa mga ito.",
            },
            schedule: {
                allElections: "Lahat ng halalan",
                outcome: "Kalalabasan",
                noOffset: "Walang timezone offset: hindi kailanman tatakbo",
                unpublished: "Hindi pa nailalathala",
                notPublished:
                    "Wala pang nailalathala: makikita ng mga botante ang iskedyul pagkatapos ng unang paglalathala.",
                unpublishedChanges_one:
                    "{{count}} nakaiskedyul na event ang nagbago mula sa huling paglalathala. Makikita ito ng mga botante pagkatapos mong maglathala.",
                unpublishedChanges_other:
                    "{{count}} nakaiskedyul na event ang nagbago mula sa huling paglalathala. Makikita ang mga ito ng mga botante pagkatapos mong maglathala.",
                offsetless_one:
                    "{{count}} nakaiskedyul na oras ang walang timezone offset, kaya hindi ito kailanman tatakbo. I-edit ito para itakda ang timezone nito.",
                offsetless_other:
                    "{{count}} nakaiskedyul na oras ang walang timezone offset, kaya hindi kailanman tatakbo ang mga ito. I-edit ang mga ito para itakda ang kanilang timezone.",
                outcomeChange:
                    "Kapag na-save, mababago ang gagawin ng nakaiskedyul na transisyong ito: {{before}} → {{after}}.",
                outcomeNew: "Kapag na-save, ang nakaiskedyul na transisyong ito: {{after}}.",
                outcomeElections: "{{count}} sa {{total}} na halalan",
                exportError: "Hindi ma-export ang iskedyul.",
                exportFileName: "schedule.csv",
                totals: {
                    refused_one:
                        "{{count}} nakaiskedyul na row ang tatanggihan ({{transitions}} transisyon ng halalan).",
                    refused_other:
                        "{{count}} nakaiskedyul na row ang tatanggihan ({{transitions}} transisyon ng halalan).",
                    runsUnsigned_one:
                        "{{count}} nakaiskedyul na pagsasara ang tatakbo nang walang lagda ({{transitions}} transisyon ng halalan).",
                    runsUnsigned_other:
                        "{{count}} nakaiskedyul na pagsasara ang tatakbo nang walang lagda ({{transitions}} transisyon ng halalan).",
                    review: "Suriin",
                    showAll: "Ipakita lahat",
                    showing: {
                        refused:
                            "Ipinapakita ang {{count}} nakaiskedyul na row na tatanggihan ({{transitions}} transisyon ng halalan).",
                        runsUnsigned:
                            "Ipinapakita ang {{count}} nakaiskedyul na pagsasara na tatakbo nang walang lagda ({{transitions}} transisyon ng halalan).",
                    },
                },
                recompute: {
                    title_one:
                        "Inililipat ng isang update sa timezone database ang {{count}} nakaiskedyul na oras sa hinaharap. Walang magbabago hangga't hindi mo ito inilalapat.",
                    title_other:
                        "Inililipat ng isang update sa timezone database ang {{count}} nakaiskedyul na oras sa hinaharap. Walang magbabago hangga't hindi mo inilalapat ang mga ito.",
                    change: "{{type}}: {{before}} → {{after}}",
                    apply: "Ilapat",
                    applied_one: "{{count}} nakaiskedyul na oras ang na-update.",
                    applied_other: "{{count}} nakaiskedyul na oras ang na-update.",
                    error: "Hindi ma-update ang mga nakaiskedyul na oras.",
                },
                outcomeChangeElections_one:
                    "Binabago ng pag-save ang kalalabasan sa {{count}} halalan:",
                outcomeChangeElections_other:
                    "Binabago ng pag-save ang kalalabasan sa {{count}} halalan:",
            },
            authorizes: {
                reportPolicyOf: "{{election}}: {{value}}",
                initializationRetained:
                    "Nananatiling kailangan ang isang ulat na kailangan sa nilagdaang configuration na ito kahit gawing hindi kailangan sa kasalukuyang setting ng Post.",
                title: "Ano ang pinahihintulutan ng pag-apruba na ito",
                schedule: "Mga nakaiskedyul na pagbubukas at pagsasara",
                noSchedule:
                    "Walang nakaiskedyul na pagbubukas o pagsasara: ang mga pumipirma ang nagbubukas at nagsasara ng pagboto.",
                opens: "Magbubukas {{time}}",
                closes: "Magsasara {{time}}",
                settings: "Mga setting",
                unsignedClose: "Nakaiskedyul na pagsasara nang walang lagda: {{value}}",
                initialization: "Inisyalisasyon: {{value}}",
                firstConfiguration:
                    "Ito ang unang naaprubahang configuration: walang maihahambing.",
                sameAsPrevious: "Pareho ang mga setting sa nakaraang naaprubahang configuration.",
                rule: {
                    openNeeds_one: "Kailangan ng {{count}} lagda para magbukas",
                    openNeeds_other: "Kailangan ng {{count}} lagda para magbukas",
                    openNoSignatures: "Hindi kailangan ng lagda para magbukas",
                    closeNeeds_one: "Kailangan ng {{count}} lagda para magsara",
                    closeNeeds_other: "Kailangan ng {{count}} lagda para magsara",
                    closeNoSignatures: "Hindi kailangan ng lagda para magsara",
                    openSetting: "Pagbubukas ng pagboto",
                    closeSetting: "Pagsasara ng pagboto",
                    signatures_one: "{{count}} lagda",
                    signatures_other: "{{count}} lagda",
                    none: "walang lagda",
                },
                diff: {
                    tightens: "Hinihigpitan: {{setting}} {{before}} → {{after}}",
                    loosens: "Niluluwagan: {{setting}} {{before}} → {{after}}",
                    mixed: "Mga pagbabago: {{setting}} {{before}} → {{after}} (mas mahigpit sa isang paraan, mas maluwag sa iba)",
                },
                comparedWith: "Kumpara sa naunang aprubadong configuration, approval {{code}}:",
                channels: "Mga voting channel bawat halalan",
                channelsOf: "{{election}}: {{channels}}",
                noChannels: "wala",
            },
            publish: {
                openedAuthorized:
                    "Nagbukas ang pagboto ayon sa iskedyul noong {{time}}, pinahintulutan ng pag-apruba ng configuration {{code}} (nilagdaan ni/nina {{names}}).",
                closedAuthorized:
                    "Nagsara ang pagboto ayon sa iskedyul noong {{time}}, pinahintulutan ng pag-apruba ng configuration {{code}} (nilagdaan ni/nina {{names}}).",
                closedUnsigned:
                    "Nagsara ang pagboto ayon sa iskedyul noong {{time}}. Walang lagda sa pagsasara: isinara ng iskedyul ang pagboto sa takdang oras nito.",
                authorizedBy: "Pinahintulutan ni/ng",
                cancelledRequest:
                    "Ang kahilingang {{code}} ay may {{n}} sa {{k}} na lagda at kinansela.",
                openedRefused: "Tinanggihan ang nakaiskedyul na pagbubukas sa {{time}}.",
                closedRefused: "Tinanggihan ang nakaiskedyul na pagsasara sa {{time}}.",
                openedNoSignaturesNeeded:
                    "Nagbukas ang botohan ayon sa iskedyul ({{time}}); walang kailangang lagda.",
                closedNoSignaturesNeeded:
                    "Nagsara ang botohan ayon sa iskedyul ({{time}}); walang kailangang lagda.",
                openedNothingToChange:
                    "Noong {{time}}, walang mabubuksan ang nakaiskedyul na pagbubukas: bukas na ang mga channel nito.",
                closedNothingToChange:
                    "Noong {{time}}, walang maisasara ang nakaiskedyul na pagsasara: sarado na ang mga channel nito.",
            },
            import: {
                title: "Mag-import ng iskedyul",
                subtitle:
                    "Isang row bawat event at halalan, sa lokal na oras. Iwanang blangko ang timezone para gamitin ang timezone ng halalan.",
                chooseFile: "Pumili ng CSV file",
                template: "I-download ang template",
                templateFileName: "schedule-template.csv",
                ready: "{{ok}} event ang handa para sa {{posts}} halalan.",
                needsAttention_one:
                    "{{ok}} event ang handa para sa {{posts}} halalan. {{count}} row ang kailangang ayusin; itama ang file at i-upload itong muli.",
                needsAttention_other:
                    "{{ok}} event ang handa para sa {{posts}} halalan. {{count}} row ang kailangang ayusin; itama ang file at i-upload itong muli.",
                preview: "Mga row na ii-import",
                row: "Row",
                asWritten: "{{local}} · {{place}}",
                moreRows: "…at {{count}} pang row",
                imported:
                    "Na-import ang iskedyul: {{created}} ang nalikha, {{updated}} ang na-update.",
                uploadError: "Hindi masuri ang file. I-upload itong muli.",
                importError: "Hindi ma-import ang iskedyul.",
                error: {
                    unknownElection: "Walang halalan na may alias na {{election}}.",
                    unknownEventType: "Ang {{type}} ay hindi uri ng nakaiskedyul na event.",
                    invalidTimeZone: "Ang {{zone}} ay hindi isang timezone.",
                    invalidDateTime: "Ang petsa at oras ay dapat nasa anyong YYYY-MM-DDTHH:MM.",
                    invalidVotingChannels:
                        "Hindi kilala ang mga channel ng pagboto, o sabay na nagbubukas ng Online at Maagang pagboto.",
                    dstGap: "Hindi umiiral ang {{dateTime}} sa {{city}} dahil umuusad ang orasan. Maglagay ng oras na umiiral.",
                    duplicate:
                        "May ibang row na nag-iiskedyul ng parehong event para sa halalang ito.",
                    other: "Hindi ma-import ang row na ito ({{code}}).",
                    ambiguousElection: "Higit sa isang halalan ang may alias na {{election}}.",
                },
            },
            settings: {
                accordion: "Wika, Petsa at Oras",
                dateAndTime: "Petsa at oras",
                configured: "Mga naka-configure na timezone",
                configuredHelp:
                    "{{count}} timezone. Pumipili ang mga halalan ng kanilang timezone mula sa listahang ito; mag-type ng lungsod o bansa para magdagdag.",
                moreZones: "+{{count}}",
                primary: "Pangunahing timezone",
                primaryHelp:
                    "Ginagamit para sa mga iskedyul ng buong event, mga ulat at mga halalang walang sariling timezone.",
                primaryInUse:
                    "Ang {{zone}} ang pangunahing timezone. Pumili muna ng ibang pangunahing timezone.",
                inUse: "Ginagamit ang {{zone}} ng {{names}}. Baguhin muna ang mga halalang iyon.",
                logs: "Mga oras sa Logs at sa mga export ng log",
                logsPrimary: "Pangunahing timezone ({{abbr}})",
                logsElection: "Timezone ng halalan ng bawat row",
                logsHelp: "Ginagamit ng mga row na walang halalan ang pangunahing timezone.",
                electionZone: "Timezone",
                electionPrimary: "Pangunahin ng event: {{zone}}",
                electionZoneHelp:
                    "Ginagamit ng mga iskedyul, screen ng botante at ulat para sa halalang ito ang timezone na ito, kasama ang bawat area sa ilalim nito. Kapag blangko, ginagamit ang pangunahing timezone ng event.",
                electionUnconfigured:
                    "Hindi na naka-configure sa event ang timezone na ito, kaya ginagamit ng halalan ang pangunahing timezone, {{zone}}. Pumili ng isa sa mga naka-configure na timezone.",
                electionUnconfiguredSave:
                    "Pumili ng isa sa mga naka-configure na timezone ng event.",
            },
            policies: {
                accordion: "Lifecycle ng pagboto",
                intro: "Bahagi ang mga setting na ito ng configuration ng election event: nilalagdaan ang mga ito ng pag-apruba ng configuration, at sinusunod ng mga nakaiskedyul na pagbubukas at pagsasara ang mas mahigpit sa kasalukuyan at sa nailathalang mga setting.",
                nothingPublished:
                    "Wala pang nailalathala: hanggang sa unang paglalathala, ginagamit ng mga nakaiskedyul na pagbubukas at pagsasara ang mga default (bawat halalan, tanggihan).",
                publishedValue: "Nailathalang configuration: {{value}}",
                changedSincePublished:
                    "Nagbago mula sa nailathalang configuration: sinusunod ng mga nakaiskedyul na pagbubukas at pagsasara ang mas mahigpit sa dalawa hanggang sa susunod na aprubadong paglalathala.",
                scope: {
                    title: "Inisyalisasyon bago magbukas ang pagboto",
                    post: {
                        label: "Bawat halalan",
                        help: "Nagbubukas ang isang halalan kapag na-initialize na ito.",
                    },
                    event: {
                        label: "Buong event",
                        help: "Walang halalang magbubukas hangga't hindi na-initialize ang bawat halalan.",
                        warning:
                            "Kapag may isang halalang hindi pa na-initialize, mananatiling sarado ang bawat halalan, kahit sa kanilang nakaiskedyul na pagbubukas.",
                    },
                    postAndCountry: {
                        label: "Bawat halalan at bansa",
                        help: "Nagbubukas ang isang halalan kapag na-initialize na ang bawat bansa (area) sa ilalim nito.",
                        warning:
                            "Mananatiling sarado ang isang halalan, kahit sa nakaiskedyul nitong pagbubukas, hangga't hindi na-initialize ang bawat bansa sa ilalim nito; ini-initialize ang bawat bansa gamit ang sarili nitong ulat.",
                    },
                },
                close: {
                    title: "Nakaiskedyul na pagsasara nang walang lagda",
                    help: "Kapag kailangan ng lagda ang pagsasara ng pagboto at wala sa nilagdaang configuration ang isang nakaiskedyul na pagsasara.",
                    refuse: {
                        label: "Tanggihan",
                        help: "Hindi tatakbo ang pagsasara; isinasara ng mga pumipirma ng halalan ang pagboto gamit ang kanilang lagda.",
                    },
                    runAsSystem: {
                        label: "Patakbuhin bilang system",
                        help: "Magsasara ang pagboto sa takdang oras, at itatala bilang isinara ng iskedyul nang walang lagda.",
                        warning:
                            "Isinasara ng mga nakaiskedyul na pagsasara na wala sa nilagdaang configuration ang pagboto nang walang lagda ng sinuman. Nakasaad ito sa log at sa mga dokumento.",
                    },
                },
                onSave: {
                    outcomes_zero:
                        "Walang nakaiskedyul na transisyon ang magbabago ng kalalabasan.",
                    outcomes_one:
                        "{{count}} nakaiskedyul na transisyon ang magbabago ng kalalabasan. Suriin ito sa Naka-schedule na Kaganapan.",
                    outcomes_other:
                        "{{count}} nakaiskedyul na transisyon ang magbabago ng kalalabasan. Suriin ang mga ito sa Naka-schedule na Kaganapan.",
                },
                saveError: "Hindi ma-save ang mga setting ng lifecycle ng pagboto.",
                publishedPerTarget: "Nailathalang configuration, bawat target: {{values}}",
                publishedCount_one: "{{value}} ({{count}} target)",
                publishedCount_other: "{{value}} ({{count}} target)",
                savedWithoutPolicies:
                    "Na-save ang election event, pero hindi ang mga setting ng voting lifecycle: {{reason}}. I-save muli ang mga ito.",
            },
        },
        scheduledOutcome: {
            chip: {
                waitingForInitialization: "Naghihintay ng inisyalisasyon",
                runs: "Tatakbo",
                runsUnsigned: "Tatakbo nang walang lagda",
                refused: "Tatanggihan",
            },
            note: {
                waitingForInitialization: "Naghihintay ng inisyalisasyon",
                authorized: "Pinahintulutan ng configuration {{code}}",
                noSignaturesNeeded: "Hindi kailangan ng lagda",
                closesUnsigned: "Magsasara nang walang lagda",
                refused: {
                    initialization: "Hindi pa kumpleto ang kinakailangang inisyalisasyon",
                    votingClose:
                        "Hindi maaaring buksan ang pagboto pagkatapos ng takdang pagsasara",
                    needsSignatures: "Kailangan ng lagda ng mga pumipirma",
                    covered: "Wala sa nilagdaang configuration",
                    unsignedClose: "Tinatanggihan ang pagsasara na walang lagda",
                    stricterCopy:
                        "Nagbago mula sa nailathalang configuration, na siya pa ring nagpapasya",
                    defaults: "Wala pang nailathala: ang mga default ang ginagamit",
                },
                refusedWithStep: "{{reason}}. {{next}}",
            },
            why: {
                button: "Bakit?",
                title: {
                    waitingForInitialization: "Bakit naghihintay ng inisyalisasyon",
                    runs: "Bakit ito tatakbo",
                    runsUnsigned: "Bakit ito tatakbo nang walang lagda",
                    refused: "Bakit ito tatanggihan",
                },
                checks: "Mga pagsusuri",
                check: "Pagsusuri",
                current: "Kasalukuyang mga setting",
                published: "Nailathalang configuration",
                verdict: "Pasya",
                allows: "Pinapayagan",
                blocks: "Hinaharangan",
                deciding: "Mapagpasyang pagsusuri",
                nextStep: "Susunod na hakbang:",
                signedBy: "Nilagdaan ni/nina {{names}}",
            },
            question: {
                initialization: "Kumpleto na ba ang kinakailangang inisyalisasyon?",
                votingClose: "Sinusunod ba ng pagbubukas na ito ang takdang pagsasara ng pagboto?",
                needsSignatures: "Kailangan ba ng lagda ang aksyong ito?",
                covered: "Nasa nilagdaang configuration ba ang eksaktong iskedyul na ito?",
                unsignedClose: "Ano ang mangyayari sa pagsasara nang walang lagda?",
                stricterCopy:
                    "Magkaiba ba ang kasalukuyan at ang nailathalang mga setting? Alin ang nagpapasya?",
                defaults: "May nailathala na ba?",
            },
            check: {
                initialization: {
                    waiting:
                        "Kailangang makumpleto ang mga inisyalisasyong hinihingi ng kasalukuyan at nailathalang mga setting.",
                },
                votingClose: {
                    passed: "Magsasara ang pagboto sa {{closes_at}}; hindi maaaring isagawa ang pagbubukas na ito sa oras na iyon o pagkatapos nito.",
                },
                needsSignatures: {
                    yes: "Oo, {{signatures}} lagda",
                    yes_one: "Oo, {{count}} lagda",
                    yes_other: "Oo, {{count}} lagda",
                    no: "Hindi",
                },
                covered: {
                    overriddenBySignedPostRow:
                        "Ginagamit ng nilagdaang configuration {{code}} ang sariling pagbubukas {{scheduled_event_id}} ng Post na ito. Hindi nalalapat ang pagbubukas para sa buong kaganapan.",
                    yes: "Oo: pag-apruba {{code}}, walang pagbabago",
                    changed: "Hindi: nagbago mula sa pag-apruba {{code}}",
                    changedBy:
                        "Hindi: in-edit noong {{edited_at}} ni {{edited_by}}, pagkatapos ng pag-apruba {{code}}",
                    notInApproval: "Hindi: hindi ito kasama sa pag-apruba {{code}}",
                    noApproval: "Wala pang naaprubahang configuration",
                    channelsChanged:
                        "Hindi: nagbago ang mga voting channel ng halalan mula sa approval {{code}}",
                    alreadyFired:
                        "Hindi: tumakbo na ang transisyong ito ng approval {{code}} noong {{fired_at}}; kailangan ng lagda para patakbuhin ito muli",
                    late: "Hindi: lampas na ng 15 minuto mula {{scheduled_date}} (approval {{code}}); kailangan ng lagda para patakbuhin ito ngayon",
                },
                unsignedClose: {
                    refuse: "Tanggihan",
                    runAsSystem: "Patakbuhin bilang system",
                },
                stricterCopy: {
                    same: "Pareho ang dalawa",
                    currentStricter:
                        "Mas mahigpit ang kasalukuyang mga setting: nalalapat na ngayon",
                    currentLooser:
                        "Mas maluwag ang kasalukuyang mga setting: malalapat ang mga ito pagkatapos ng susunod na naaprubahang paglalathala",
                    combined: "Mas mahigpit ang bawat isa sa isang halaga: parehong nalalapat",
                },
                defaults: {
                    published: "Nailathala noong {{published_at}}",
                    nothingPublished: "Walang nailathala: ang mga default ang nalalapat",
                    noSnapshot:
                        "Nailathala noong {{published_at}}, bago itinatago ng mga paglalathala ang mga setting na ito: ang mga default ang nalalapat",
                },
            },
            nextStep: {
                initialize:
                    "Kumpletuhin ang kinakailangang inisyalisasyon. Susubukan muli ng tagaiskedyul bago magsara ang pagboto.",
                closed: "Hindi isasagawa ang pagbubukas na ito pagkatapos magsara ang pagboto.",
                none: "Walang kailangang gawin.",
                publishAndApprove: "Ilathala at aprubahan ang configuration.",
                requireConfigurationApproval:
                    "Gawing nangangailangan ng lagda ang Pag-apruba ng configuration, pagkatapos ay ilathala at aprubahan ang configuration.",
                askSignersToOpen: "Hilingin sa mga pumipirma ng halalan na buksan ang pagboto.",
                askSignersToClose: "Hilingin sa mga pumipirma ng halalan na isara ang pagboto.",
            },
            applies: {
                tightens: "Nalalapat na ngayon sa mga manwal at nakaiskedyul na aksyon.",
                loosens:
                    "Nalalapat na ngayon sa mga manwal na aksyon; sa mga nakaiskedyul na pagbubukas at pagsasara pagkatapos ng susunod na naaprubahang paglalathala.",
                tightensAndLoosens:
                    "Nalalapat na ngayon ang mas mahigpit na bahagi nito sa mga manwal at nakaiskedyul na aksyon; nalalapat na ngayon ang mas maluwag na bahagi nito sa mga manwal na aksyon, at sa mga nakaiskedyul na pagbubukas at pagsasara pagkatapos ng susunod na naaprubahang paglalathala.",
            },
        },
        messagingEvent: {
            tab: "Pagmemensahe",
            intro: "Ang mga channel na mapipili ng mga botante ng event na ito para sa mga code at abiso, at ang account na pinagpapadalhan ng bawat isa. Pinamamahalaan ang mga account sa Settings > Messaging.",
            readOnly:
                "Makikita mo ang mga setting na ito. Kailangan ang pahintulot na messaging-config-write para baguhin ang mga ito.",
            savingNote:
                "Ina-update din ng pag-save ang mga channel na inaalok ng mga pahina ng enrollment para sa bawat Post.",
            save: "I-save",
            saved: "Na-save ang mga setting ng pagmemensahe.",
            saveRejected:
                "Hindi na-save ang mga setting ng pagmemensahe. Ayusin ang mga ipinakitang problema.",
            saveError: "Hindi ma-save ang mga setting ng pagmemensahe.",
            accountLabel: "Account ng {{channel}}",
            notUsed: "Hindi ginagamit",
            missingAccount: "Hindi nahanap ang account",
            noAccount: "Magdagdag muna ng account sa Settings > Messaging",
            missing: "Kulang: {{blockers}}",
            purposeSwitch: "{{channel}}: {{purpose}}",
            sections: {
                channels: "Mga channel",
                templates: "Mga aprubadong template",
                fallback: "Pagkakasunod-sunod ng fallback para sa mga abiso",
                posts: "Mga channel ayon sa Post",
                postsCount: "Mga channel ayon sa Post ({{count}} Post)",
                reply: "Sagot sa mga papasok na mensahe",
                delivery: "Katayuan ng paghahatid",
            },
            column: {
                channel: "Channel",
                account: "Ipinapadala mula sa",
                purpose: "Layunin",
                language: "Wika",
                template: "Template ng provider",
                status: "Katayuan",
                post: "Post",
                key: "Para sa mensahe",
                providerLanguage: "Wika ng provider",
            },
            outOfWindow: {
                label: "Sa labas ng window ng usapan",
                help: "Ipinapadala lamang ang mga abisong free text habang bukas ang window ng usapan: sa Messenger, sa loob ng 24 na oras mula sa huling mensahe ng botante. Piliin ang Mga utility message para makapagpadala ng mga abiso pagkatapos nito gamit ang aprubadong template. Dapat itong aprubahan ng Meta para sa Page (ang pahintulot na page_utility_messaging at isang aprubadong UTILITY template), at dapat nakaugnay ang template na iyon sa mga abiso sa ilalim ng Mga aprubadong template. Kapag Huwag ipadala, mapupunta ang abisong nasa labas ng window sa susunod na magagamit na channel ng botante.",
                DISABLED: "Huwag ipadala",
                UTILITY_MESSAGES: "Mga utility message",
                noTemplate:
                    "Wala pang template na nakaugnay sa mga abiso sa channel na ito. Magdagdag ng isa sa ilalim ng Mga aprubadong template; hanggang doon, mapupunta ang mga abisong nasa labas ng window sa susunod na magagamit na channel ng botante.",
            },
            templates: {
                empty: "Pumili ng account na nagpapadala ng mga aprubadong template, gaya ng WhatsApp, Viber o Messenger, para iugnay dito ang mga template nito.",
                help: "Sinasabi ng bawat hilera kung aling aprubadong template ang ipinapadala ng provider para sa isang mensahe. Ang Para sa mensahe ay ang alias ng isang template sa Templates, para sa isang notification, o ang message key na ipinapadala ng Keycloak, gaya ng otp; iwanang walang laman para sa template na ginagamit bilang default para sa layunin. Ang Wika ay ang wika ng botante. Ang Template ng provider ay ang pangalan o ID ng template sa provider. Ang Wika ng provider ay ang code ng provider para sa template na iyon kapag iba ito sa wika ng botante: kailangan ng WhatsApp ang eksaktong code ng aprubadong template, gaya ng en_US.",
                order: "Para sa bawat mensahe, ang pinakatiyak na hilera ang nananaig: ang hilera para sa mensahe sa wika ng botante, pagkatapos ang hilera para sa mensahe sa anumang wika, pagkatapos ang default para sa layunin sa wika ng botante, at panghuli ang anumang default para sa layunin.",
                noneRequired:
                    "Mga aprubadong template lamang ang ipinapadala ng {{channel}}. Magdagdag ng kahit isang default na template para sa bawat layuning ginagamit.",
                noneOptional:
                    "Walang template na nakaugnay para sa {{channel}}. Kailangan lamang ang mga ito para magpadala ng mga abiso sa labas ng window ng usapan.",
                row: "Template {{position}} ng {{channel}}",
                keyDefault: "Default para sa layunin",
                add: "Magdagdag ng template ng {{channel}}",
                remove: "Alisin ang template {{position}} ng {{channel}}",
                incomplete:
                    "Ilagay ang wika at ang template ng provider, o alisin ang hilerang ito.",
                approval: {
                    APPROVED: "Aprubado",
                    NOT_APPROVED: "Hindi aprubado",
                    ADMIN_CONFIRMED: "Kinumpirma ng administrator",
                    NOT_CHECKED: "Hindi pa nasusuri ang pag-apruba",
                },
            },
            fallback: {
                help: "Kapag hindi maabot ng abiso ang botante sa kanyang channel, mapupunta ito sa susunod na channel sa pagkakasunod-sunod na ito na na-verify ng botante at inaalok ng kanyang Post. Hindi kailanman muling ipinapadala nang kusa ang mga code: ang botante ang pipili ng ibang paraan.",
                empty: "I-on ang mga abiso ng isang channel para idagdag ito sa fallback.",
                earlier: "Ilipat ang {{channel}} nang mas maaga",
                later: "Ilipat ang {{channel}} nang mas huli",
            },
            posts: {
                noChannels:
                    "I-on ang mga code o abiso ng isang channel para piliin ang mga channel ng bawat Post.",
                help: "Ipinapakita ng enrollment sa mga botante ng bawat Post ang mga channel na naka-tsek dito.",
                restricted:
                    "{{count}} Post ang nag-aalok ng mas kaunti sa lahat ng {{total}} channel.",
                allChannels:
                    "Inaalok ng bawat Post ang lahat ng {{total}} channel; alisin ang tsek sa channel para sa Post kung saan hindi ito gumagana.",
                search: "Maghanap ng Post",
                cell: "{{post}}: {{channel}}",
                showing:
                    "Ipinapakita ang {{shown}} sa {{total}} Post. Maghanap para makita ang iba.",
            },
            reply: {
                help: "Ipinapadala kapag sumulat ang botante sa isa sa mga account ng event na ito, hindi hihigit sa isang beses bawat araw bawat botante.",
                label: "Sagot ({{language}})",
            },
            delivery: {
                empty: "Walang channel na ginagamit.",
                help: "Ang Tinanggap ay nangangahulugang tinanggap ng provider ang kahilingan, hindi na natanggap o na-verify ng botante ang code. Ang Hindi alam ay nangangahulugang hindi pa kumpirmado ang paghahatid. Ang provider na walang ulat ng paghahatid ay nagpapakita ng paghahatid bilang hindi available.",
            },
            error: {
                UNSUPPORTED_VERSION:
                    "Gumagamit ang configuration na ito ng bersyon {{version}}, na hindi sinusuportahan.",
                DUPLICATE_CHANNEL: "Higit sa isang beses na naka-configure ang {{channel}}.",
                UNKNOWN_ACCOUNT: "Wala na ang account ng {{channel}}. Pumili ng ibang account.",
                ACCOUNT_OF_ANOTHER_TENANT: "Pag-aari ng ibang tenant ang napiling account.",
                ACCOUNT_CHANNEL_MISMATCH:
                    "Hindi nagpapadala ng mga mensahe sa {{channel}} ang napiling account.",
                PURPOSE_NOT_READY:
                    "Hindi pa makakapagpadala ang {{channel}} ng {{purpose}}. Kulang: {{blockers}}.",
                TEMPLATE_NOT_APPROVED:
                    "Hindi aprubado ng provider ang template ng {{channel}} para sa {{purpose}} sa {{language}}.",
                OUT_OF_WINDOW_NOT_SUPPORTED:
                    "Hindi makakapagpadala ang {{channel}} sa labas ng window ng usapan gamit ang account na ito: wala itong window ng usapan, o kailangan na ng template ang mga abiso nito.",
                FALLBACK_CHANNEL_NOT_ENABLED:
                    "Nasa fallback ang {{channel}} pero hindi ito nagpapadala ng mga abiso.",
                DUPLICATE_FALLBACK_CHANNEL:
                    "Higit sa isang beses na nasa fallback ang {{channel}}.",
                ELECTION_CHANNEL_NOT_ENABLED:
                    "Inaalok ng {{election}} ang {{channel}}, na hindi ginagamit ng event na ito.",
                UNKNOWN_ELECTION: "Ang {{election}} ay hindi halalan ng event na ito.",
            },
        },
        messaging: {
            channel: {
                EMAIL: "Email",
                SMS: "SMS",
                WHATSAPP: "WhatsApp",
                VIBER: "Viber",
                MESSENGER: "Facebook Messenger",
            },
            provider: {
                AWS_SES: "Amazon SES",
                SMTP: "SMTP server",
                AWS_SNS: "Amazon SNS",
                WHATSAPP_CLOUD_API: "WhatsApp Cloud API (Meta)",
                MESSENGER_SEND_API: "Messenger Platform (Meta)",
                VIBER_INFOBIP: "Viber Business Messages (Infobip)",
                CONSOLE: "Console (pagsubok lamang, walang ipinapadala)",
                HTTP_API: "Custom na HTTP API",
            },
            purpose: {
                OTP: "Mga code",
                NOTICE: "Mga abiso",
            },
            state: {
                QUEUED: "Nakapila",
                ACCEPTED: "Tinanggap",
                DELIVERED: "Naihatid",
                FAILED: "Nabigo",
                UNKNOWN: "Hindi alam",
            },
            stateHelp: {
                QUEUED: "Naghihintay na maipasa sa provider.",
                ACCEPTED:
                    "Tinanggap ng provider ang mensahe. Hindi ito nangangahulugang natanggap ito ng botante.",
                DELIVERED: "Iniulat ng provider na naihatid ang mensahe.",
                FAILED: "Kinumpirma ng provider na hindi naihatid ang mensahe.",
                UNKNOWN: "Hindi pa kumpirmado ang paghahatid.",
            },
            blocker: {
                NOT_CONNECTED: "Hindi nakakonekta",
                UNSUPPORTED_PURPOSE: "Hindi sinusuportahan ng provider na ito",
                NEEDS_PROVIDER_APPROVAL: "Kailangan ng pag-apruba ng provider",
                NEEDS_PRODUCTION_ACCESS: "Kailangan ng production access",
                NEEDS_APPROVED_TEMPLATE: "Kailangan ng aprubadong template",
            },
            readiness: {
                connected: "Nakakonekta",
                notConnected: "Hindi nakakonekta",
                readyOtp: "Handa para sa OTP",
                readyNotice: "Handa para sa mga abiso",
                notReady: "Hindi pa handa",
                lastCheck: "Sinuri {{date}}",
                neverChecked: "Hindi pa nasusuri",
                adminConfirmed: "Kinumpirma ng administrator",
                checkNotUsed: "Hindi ginagamit ang pagsusuri",
            },
            approval: {
                PENDING: "Hinihintay ang pag-apruba ng provider",
                CONFIRMED: "Kumpirmado ang pag-apruba ng provider",
            },
            credential: {
                ACCESS_TOKEN: "Access token",
                APP_SECRET: "App secret",
                VERIFY_TOKEN: "Verify token",
                API_KEY: "API key",
                SMTP_PASSWORD: "Password",
                AWS_ACCESS_KEY_ID: "AWS access key ID",
                AWS_SECRET_ACCESS_KEY: "AWS secret access key",
                API_SECRET: "API secret",
                USERNAME: "Username",
                PASSWORD: "Password",
                WEBHOOK_SECRET: "Webhook secret",
            },
            deliveryUnavailable: "Hindi available ang paghahatid",
            templates: {
                noMethod: "Pumili ng kahit isang paraan para sa template.",
                parameters: "Mga parameter ng template",
                parametersHelp:
                    "Kung ano ang pupuno sa bawat placeholder ng aprubadong template, ayon sa pagkakasunod, gaya ng user.first_name o vote_url. Para sa template na may mga pinangalanang parameter, isulat ang @pangalan=halaga, gaya ng @first_name=user.first_name; positional ang anumang ibang entry.",
                parameter: "Parameter {{position}}",
                removeParameter: "Alisin ang parameter {{position}}",
                addParameter: "Magdagdag ng parameter",
                noAccount:
                    "Wala pang {{channel}} account. Magdagdag sa Settings > Messaging para makita kung aling mga wika ang aprubado.",
                account: "Account",
                approvalTitle: "Mga aprubadong template",
                language: "Wika",
                approvalFor: "Aprubado para sa {{purpose}}",
                approved: "Aprubado",
                notApproved: "Hindi aprubado",
                approvalHelp:
                    "Galing sa provider ang mga pag-apruba at ina-update ng connection check ng account.",
                messengerIntro:
                    "Sa loob ng 24 na oras mula sa huling mensahe ng botante, ipinapadala ng Messenger ang teksto sa ibaba.",
                messengerMessage: "Mensahe sa loob ng 24 na oras",
                messengerWindow:
                    "Ang naka-save na Messenger recipient ay hindi pahintulot na magpadala. Sa labas ng 24 na oras na window, ipinapadala ang abisong ito bilang utility message kapag pinapayagan ito ng election event at may aprubadong template na nakatakda sa ibaba o nakaugnay sa event; kung hindi, mapupunta ito sa susunod na magagamit na channel ng botante. Kailangan ng mga utility message ang pahintulot na page_utility_messaging at isang aprubadong UTILITY template sa Page.",
                intro: {
                    WHATSAPP:
                        "Nagpapadala lamang ang WhatsApp ng mga template na inaprubahan ng Meta para sa WhatsApp Business Account. Dapat tumugma ang mensahe sa aprubadong template; piliin kung ano ang pupuno sa mga parameter nito.",
                    VIBER: "Nagpapadala lamang ang Viber ng mga code at transactional na mensahe gamit ang mga template na inaprubahan ng Viber partner. Dapat tumugma ang mensahe sa aprubadong template; piliin kung ano ang pupuno sa mga parameter nito.",
                },
                approvedWording: "Aprubadong teksto",
                approvedWordingHelp:
                    "Kopya ng aprubadong template, ginagamit bilang preview. Hindi nito binabago ang ipinapadala ng provider.",
                providerTemplateTitle: "Template ng provider",
                providerTemplateHelp:
                    "Opsyonal. Ang pangalan o ID ng aprubadong template sa provider. Kapag walang laman, ginagamit ang template ng election event na nakaugnay sa alias ng template na ito, o ang default ng event para sa layunin.",
                providerTemplate: "Pangalan o ID ng template ng provider",
                providerLanguage: "Language code ng provider",
                providerLanguageHelp: {
                    WHATSAPP:
                        "Ang eksaktong language code ng aprubadong template ng WhatsApp, gaya ng en_US.",
                    VIBER: "Ang language code kung saan kilala ng provider ng Viber ang template, kapag kailangan nito.",
                    MESSENGER: "Ang language code ng aprubadong utility template, gaya ng en_US.",
                },
                approvalAdminConfirmed:
                    "Kinumpirma ng isang administrator sa provider na aprubado ang mga template ng account na ito, kaya hindi ginagamit ang mga pag-apruba mula sa pagsusuri ng koneksyon.",
            },
            send: {
                channel: "Channel",
                eachVoter: "Channel ng bawat botante",
                only: "{{channel}} lamang",
                eachVoterHelp:
                    "Ang mga kumpirmadong pagkabigo ay gagamit ng susunod na available na verified channel. Ang hindi kumpirmadong paghahatid ay ipinapakita bilang Hindi alam.",
                onlyHelp: "Ipinapadala ang abisong ito sa bawat botante sa {{channel}}.",
                channelColumn: "Channel",
                sendsFrom: "Ipinapadala mula sa",
                noAccount: "Walang account",
                missingContent: "Walang nilalaman ang abisong ito para sa {{channels}}.",
                approvedTemplateHelp:
                    "Ipinapadala gamit ang template na inaprubahan ng provider. I-edit ito sa Templates.",
                providerTemplate: "Template ng provider ng {{channel}}",
                providerTemplateHelp:
                    "Opsyonal. Kapag walang laman, ginagamit ang template ng event na nakaugnay sa alias ng napiling template, o ang default ng event para sa mga abiso.",
                providerLanguage: "Wika ng provider ng {{channel}}",
                providerLanguageHelp:
                    "Ang language code ng provider para sa template na iyon, gaya ng en_US.",
            },
            voter: {
                title: "Messaging",
                preferredChannel: "Gustong channel",
                whatsappNumber: "WhatsApp number",
                viberNumber: "Viber number",
                messengerConnected: "Nakakonekta",
                messengerNotConnected: "Hindi nakakonekta",
                verifiedChannels: "Mga verified na channel",
                noneVerified: "Walang verified na channel",
                notSet: "Hindi nakatakda",
            },
            logs: {
                channel: "Channel",
            },
            stats: {
                sent: {
                    WHATSAPP: "Mga mensahe sa WhatsApp na naipadala",
                    VIBER: "Mga mensahe sa Viber na naipadala",
                    MESSENGER: "Mga mensahe sa Messenger na naipadala",
                },
            },
            readinessPolicy: {
                PROVIDER_CHECK: "Mula sa pagsusuri ng provider",
                ADMIN_CONFIRMED: "Kinumpirma ng administrator",
            },
        },
        messagingAccounts: {
            tab: "MESSAGING",
            description:
                "Mga account na nagpapadala sa mga botante ng kanilang mga code at abiso. Pinipili ng bawat election event ang account para sa bawat channel; nagsisimula ang mga bagong event sa default na account.",
            list: {
                title: "Mga account na nagpapadala",
                add: "Magdagdag ng account",
                loading: "Nilo-load ang mga account",
                loadError: "Hindi ma-load ang mga account na nagpapadala.",
                empty: "Wala pang account na nagpapadala.",
            },
            column: {
                channel: "Channel",
                name: "Account",
                sender: "Nagpapadala bilang",
                provider: "Provider",
                default: "Default",
                isDefault: "Default na account",
                lastCheck: "Huling pagsusuri",
                actions: "Mga aksyon",
            },
            action: {
                edit: "I-edit",
                editNamed: "I-edit ang {{name}}",
                view: "Tingnan",
                viewNamed: "Tingnan ang {{name}}",
                check: "Suriin ang koneksyon",
                checkNamed: "Suriin ang koneksyon ng {{name}}",
                test: "Magpadala ng test message",
                testNamed: "Magpadala ng test message mula sa {{name}}",
                delete: "Tanggalin",
                deleteNamed: "Tanggalin ang {{name}}",
            },
            check: {
                done: "Nasuri na ang {{name}}. Na-update ang status nito.",
                error: "Hindi masuri ang {{name}}.",
            },
            delete: {
                title: "Tanggalin ang account",
                body: "Tanggalin ang {{name}}? Titigil sa pagpapadala sa channel nito ang mga election event na gumagamit nito.",
                success: "Natanggal ang account",
                error: "Hindi matanggal ang account.",
            },
            editor: {
                addTitle: "Magdagdag ng account",
                editTitle: "I-edit ang {{channel}} account",
                subtitle:
                    "Tumatanggap ang mga botante ng mga code at abiso mula sa account na ito sa mga channel na gumagamit nito.",
                channel: "Channel",
                provider: "Provider",
                save: "I-save",
                cancel: "Kanselahin",
                close: "Isara",
                channelHelp: "Hindi na mababago pagkatapos malikha ang account.",
            },
            field: {
                name: "Pangalan ng account",
                from_address: "Address ng nagpadala",
                from_name: "Pangalan ng nagpadala",
                region: "AWS region",
                notification_topic_arn: "Topic ng mga abiso sa paghahatid (SNS ARN)",
                server_url: "Server at port",
                sender_id: "Sender ID",
                origination_number: "Origination number",
                business_account_id: "WhatsApp Business Account ID",
                phone_number_id: "Phone number ID",
                display_phone_number: "Numero",
                display_name: "Display name",
                api_version: "Bersyon ng Graph API",
                page_id: "Facebook Page ID",
                page_name: "Pangalan ng Page",
                page_username: "Username ng Page",
                base_url: "Base URL ng API",
                sender: "Pangalan ng nagpadala",
                provider_approval: "Pag-apruba ng provider",
                is_default: "Default na {{channel}} account para sa mga bagong election event",
                readiness: "Kahandaan",
                api_base_url: "Base URL ng Graph API",
                label: "Nagpadalang ipinapakita sa mga botante",
            },
            fieldHelp: {
                from_address:
                    "Ang address na nakikita ng mga botante. Dapat beripikado ang domain nito sa provider.",
                notification_topic_arn:
                    "Ang SNS topic kung saan inilalathala ng SES ang mga event ng paghahatid at bounce. Tinatanggihan ang mga abiso mula sa ibang topic.",
                sender_id:
                    "Hanggang 11 titik at numero. May mga bansang nangangailangan ng rehistro.",
                origination_number:
                    "Ginagamit sa halip ng sender ID kung saan nangangailangan ang bansa ng numero.",
                phone_number_id: "Ang numerong pinagmumulan ng mga mensahe.",
                display_name: "Ang display name na inaprubahan ng Meta para sa numero.",
                page_username:
                    "Ginagamit para sa m.me link na binubuksan ng mga botante para makuha ang kanilang code.",
                api_version: "Halimbawa, v23.0.",
                base_url: "Ang base URL ng Infobip API ng account.",
                sender: "Ang aprubadong nagpadala na nakikita ng mga botante.",
                provider_approval:
                    "Pinapayagan lamang ng Meta ang pagmemensahe ng gobyerno sa WhatsApp sa pamamagitan ng aprubadong kaayusan. Piliin ang Kumpirmado ang pag-apruba ng provider kapag naaprubahan na ito ng Meta para sa account na ito; hanggang doon, hindi mapapagana ang mga OTP at abiso para dito.",
                readiness:
                    "Ginagamit ng Mula sa pagsusuri ng provider ang nakikita ng pagsusuri ng koneksyon: kung nakakonekta ang account, kung nasa production ito, at kung aling mga template ang aprubado. Ang Kinumpirma ng administrator ay para sa mga provider na hindi ito matutukoy ng pagsusuri: ito ang iyong pahayag na nakakonekta ang account, nasa production at aprubado ang mga template nito, at ito ang ginagamit sa halip ng pagsusuri.",
                api_base_url:
                    "Kapag hindi sa Meta mismo ang Graph API lamang, gaya ng endpoint ng isang Solution Provider. Kapag walang laman, ang sa Meta ang ginagamit.",
                label: "Ang pangalang nakikita ng mga botante bilang nagpadala ng account na ito.",
            },
            error: {
                REQUIRED: "Kailangan",
                NOT_A_COUNT: "Maglagay ng buong numero",
                OTP_ABOVE_TOTAL: "Hindi maaaring lumampas sa mga mensahe bawat segundo",
                INVALID_CALLING_CODE:
                    "Maglagay ng country calling code na 1 hanggang 3 numero, gaya ng 63",
                DUPLICATE_LANGUAGE: "May template na ang wikang ito para sa layuning ito",
                NOT_A_URL: "Maglagay ng address na nagsisimula sa https:// o http://",
                INVALID_HTTP_CONFIG: "Ayusin ang mga ipinakitang problema",
            },
            warning: {
                pageChange:
                    "Ang mga usapan sa Messenger ay pag-aari ng isang Page. Pagkatapos palitan ang Page, makakatanggap lamang ng code ang mga botanteng nakakonekta sa {{page}} kapag muli nilang ikinonekta ang Messenger.",
                numberChange:
                    "Manggagaling sa ibang numero ang mga mensahe. Dapat aprubado ang mga template nito sa business account na iyon bago ito makapagpadala ng mga code, at makakakita ang mga botante ng bagong chat.",
            },
            viber: {
                title: "Mga aprubadong template",
                description:
                    "Ilagay ang mga template na inaprubahan ng Viber sa pamamagitan ng partner, ayon sa layunin at wika. Hindi available ang template API ng partner, kaya mano-manong pinapanatili ang listahang ito at binabasa ito ng pagsusuri ng koneksyon.",
                purpose: "Layunin",
                language: "Wika",
                templateId: "Template ID ng partner",
                add: "Magdagdag ng template",
                remove: "Alisin ang template",
            },
            limits: {
                title: "Mga limitasyon sa pagpapadala",
                messagesPerSecond: "Mga mensahe bawat segundo",
                otpReservedPerSecond: "Nakalaan para sa OTP bawat segundo",
                otpReservedHelp: "Nakalaan para sa mga code habang may maramihang pagpapadala.",
                allowedCallingCodes: "Mga pinapayagang destinasyon (country calling codes)",
                allowedCallingCodesHelp:
                    "Pinaghihiwalay ng kuwit, halimbawa 63, 971. Kapag walang laman, pinapayagan ang anumang destinasyon.",
            },
            credentials: {
                title: "Mga kredensyal",
                description:
                    "Write-only ang mga kredensyal: pagkatapos mag-save, ang petsa lamang ng huling pagpapalit ng bawat isa ang ipinapakita.",
                set: "Naitakda · pinalitan {{date}}. Naka-encrypt itong iniimbak at hindi kailanman ipinapakita.",
                replace: "Palitan",
                replaceNamed: "Palitan ang {{name}}",
            },
            credentialHelp: {
                AWS_SES: {
                    AWS_ACCESS_KEY_ID:
                        "Opsyonal. Kung walang key, ginagamit ang sariling role ng serbisyo.",
                    AWS_SECRET_ACCESS_KEY: "Opsyonal. Itakda ito kasama ng access key ID.",
                },
                AWS_SNS: {
                    AWS_ACCESS_KEY_ID:
                        "Opsyonal. Kung walang key, ginagamit ang sariling role ng serbisyo.",
                    AWS_SECRET_ACCESS_KEY: "Opsyonal. Itakda ito kasama ng access key ID.",
                },
                SMTP: {
                    SMTP_PASSWORD: "Ang password ng SMTP server.",
                },
                WHATSAPP_CLOUD_API: {
                    ACCESS_TOKEN:
                        "Token ng isang system user sa business portfolio ng may-ari, na may whatsapp_business_messaging.",
                    APP_SECRET: "Sinusuri na galing sa Meta ang mga tawag sa webhook.",
                },
                MESSENGER_SEND_API: {
                    ACCESS_TOKEN: "Page access token na may pages_messaging.",
                    APP_SECRET: "Sinusuri na galing sa Meta ang mga tawag sa webhook.",
                },
                VIBER_INFOBIP: {
                    API_KEY: "Ang Infobip API key.",
                },
                HTTP_API: {
                    API_KEY: "Opsyonal. Ginagamit ito ng mga request bilang kredensyal na API_KEY.",
                    API_SECRET:
                        "Opsyonal. Pangalawang secret, at ang key na pumipirma sa JWT: PEM private key para sa RS256, ang shared secret para sa HS256.",
                    ACCESS_TOKEN:
                        "Opsyonal. Ginagamit ito ng mga request bilang kredensyal na ACCESS_TOKEN.",
                    USERNAME:
                        "Opsyonal. Kasama ng password, binubuo nito ang placeholder na basic_auth.",
                    PASSWORD:
                        "Opsyonal. Kasama ng username, binubuo nito ang placeholder na basic_auth.",
                    WEBHOOK_SECRET:
                        "Opsyonal. Ang shared secret na ginagamit sa pagsusuri ng mga callback ng provider.",
                },
            },
            webhook: {
                title: "Mga ulat sa paghahatid at mga sagot",
                description:
                    "Ilagay ang callback na ito sa mga setting ng webhook ng provider. Doon dumarating ang mga ulat sa paghahatid at ang mga sagot ng mga botante.",
                path: "Callback path",
                pathHelp:
                    "Idagdag ito sa pampublikong address ng mga messaging webhook ng platform na ito.",
                afterSaving: "Ipinapakita pagkatapos mag-save",
                copyPath: "Kopyahin ang callback path",
                tokenSet: "Naitakda · pinalitan {{date}}",
                tokenMissing: "Hindi pa nabubuo",
                tokenAfterSaving: "Bubuuin pagkatapos mag-save",
                generate: "Bumuo ng verify token",
                tokenTitle: "Verify token",
                tokenOnce:
                    "Ilagay na ngayon ang token na ito sa mga setting ng webhook ng Meta. Isang beses lamang ito ipinapakita.",
                copyToken: "Kopyahin ang verify token",
                tokenDone: "Tapos na",
                tokenError: "Hindi mabuo ang verify token.",
                httpHelp:
                    "Maaaring i-post ng custom na HTTP API ang mga ulat nito bilang JSON, o ipadala ang mga ito bilang GET request; binabasa noon ang mga query parameter nito bilang flat na object, na may mga pointer gaya ng /status.",
            },
            copy: {
                success: "Nakopya",
                error: "Hindi makopya",
            },
            save: {
                success: "Na-save ang account",
                error: "Hindi ma-save ang account.",
            },
            test: {
                title: "Magpadala ng test message mula sa {{name}}",
                description:
                    "Nagpapadala ng totoong mensahe para sa napiling layunin sa destinasyong ito. Ipinapakita ng resulta ang iniulat ng provider.",
                purpose: "Layunin",
                destination: {
                    EMAIL_ADDRESS: "Email address",
                    PHONE_NUMBER: "Numero ng telepono (E.164)",
                    PAGE_SCOPED_ID: "Page-scoped ID",
                },
                language: "Wika",
                send: "Magpadala ng test message",
                reason: "Dahilan: {{reason}}",
                error: "Hindi maipadala ang test message.",
                template: "Aprubadong template",
                templateHelp:
                    "Ang pangalan o ID ng template na inaprubahan ng provider para sa layunin at wikang ito.",
                viberTemplate:
                    "Ginagamit ng Viber ang template na nakalista sa account na ito bilang aprubado para sa napiling layunin at wika.",
                languageHelp:
                    "Para sa provider na nagpapadala ng mga aprubadong template, ilagay ang language code ng provider para sa template, gaya ng en_US.",
            },
            http: {
                title: "Custom na HTTP API",
                description:
                    "Inilalarawan ang isang provider ayon sa mga HTTP request nito: ibang Viber partner, sariling API ng isang WhatsApp Solution Provider, isang SMS gateway. JSON ang mga request; maaaring maglaman ang URL, mga header at body ng mga ito ng mga placeholder mula sa reference sa ibaba.",
                phoneFormat: "Format ng numero ng telepono",
                phoneFormatHelp:
                    "Kung paano isinusulat sa request ang numero ng telepono ng tatanggap.",
                phoneFormatOption: {
                    E164: "May plus sign: +639171234567",
                    DIGITS: "Mga numero lamang: 639171234567",
                },
                templateRequired: "Mga layuning nangangailangan ng aprubadong template",
                templateRequiredHelp:
                    "Ang layuning naka-tsek ay ipinapadala lamang gamit ang template na inaprubahan ng provider, na nakaugnay sa election event. Ipinapadala bilang free text ang iba pang layunin.",
                approvedLanguages: "Mga wikang may aprubadong template para sa {{purpose}}",
                approvedLanguagesHelp:
                    "Ang mga language code na may aprubadong template, ayon sa kinumpirma sa provider, na pinaghihiwalay ng kuwit: en, tl. Iniuulat ang mga ito ng pagsusuri ng koneksyon.",
                conversationWindow: "Window ng usapan (oras)",
                conversationWindowHelp:
                    "Mga oras pagkatapos ng huling mensahe ng tatanggap kung kailan maaaring magpadala ng free text. Walang laman kapag walang ganitong window ang provider.",
                messageIdPointer: "Message ID sa sagot sa pagpapadala",
                messageIdPointerHelp:
                    "JSON pointer sa message ID ng provider sa sagot sa send request, gaya ng /message_id. Dito itinutugma ang mga ulat sa paghahatid.",
                notConfigured: "Hindi naka-configure.",
                thisSection: "Ang seksyong ito",
                add: "Idagdag: {{section}}",
                remove: "Alisin: {{section}}",
                section: {
                    SEND: "Send request",
                    CHECK: "Request ng pagsusuri ng koneksyon",
                    TOKEN: "Token request",
                    JWT: "Pinirmahang token (JWT)",
                    REPORTS: "Mga ulat sa paghahatid at mga sagot",
                    RECONCILE: "Request ng paghahanap ng mensahe",
                },
                sectionHelp: {
                    SEND: "Ang request na nagpapadala ng isang mensahe: method (POST kapag hindi inilagay), url, headers at body.",
                    CHECK: "Opsyonal. Isang request na nagtatagumpay, na may sagot na 2xx, kapag gumagana ang mga kredensyal. Pinapatakbo ito ng pagsusuri ng koneksyon.",
                    TOKEN: "Opsyonal. Kumukuha ng panandaliang token bago magpadala, gaya ng OAuth client credentials: request, token_pointer (kung nasaan ang token sa sagot) at lifetime_seconds. Ginagamit ito ng mga request sa pamamagitan ng placeholder na token.",
                    JWT: "Opsyonal. Isang token na pinipirmahan para sa bawat request gamit ang kredensyal na API secret: algorithm (RS256 o HS256), claims (idinadagdag ang iat, exp at jti) at lifetime_seconds. Ginagamit ito ng mga request sa pamamagitan ng placeholder na jwt.",
                    REPORTS:
                        "Opsyonal. Kung paano babasahin ang ipinapadala ng provider sa callback: auth, items_pointer (kung nasaan ang listahan ng mga ulat; ang buong payload kapag hindi inilagay), status (message_id_pointer, state_pointer, states na nagtutugma ng bawat value ng provider sa QUEUED, ACCEPTED, DELIVERED, FAILED o UNKNOWN, at error_pointer) at inbound_from_pointer (kung nasaan ang nagpadala ng isang sagot). May kind ang auth: URL_KEY (ang lihim na address lamang ng callback), HEADER_SECRET (isang header na katumbas ng webhook secret), HMAC_SHA256 (isang header na may HMAC ng body gamit ang webhook secret, na may prefix, encoding na HEX o BASE64, at signed kapag higit pa sa body ang pinipirmahan) o JWT_HS256 (isang header na may bearer JWT na pinirmahan gamit ang webhook secret).",
                    RECONCILE:
                        "Opsyonal. Nagtatanong sa provider tungkol sa isang mensaheng hindi alam ang kinalabasan: request at status, na binabasa gaya ng status ng mga ulat sa paghahatid.",
                },
                problem: {
                    NOT_AN_OBJECT: "Dapat object ang {{path}}.",
                    MISSING_URL: "Kailangan ang {{path}}: ang address ng request.",
                    INVALID_METHOD: "Dapat HTTP method ang {{path}}, gaya ng POST o GET.",
                    INVALID_HEADERS:
                        "Dapat text ang {{path}}: ang headers ay object ng mga pangalan ng header at mga text na value.",
                    UNKNOWN_FIELD: "Hindi field ng seksyong ito ang {{path}}.",
                    UNKNOWN_PLACEHOLDER:
                        "Gumagamit ang {{path}} ng placeholder na hindi umiiral. Tingnan ang reference ng mga placeholder.",
                    INVALID_POINTER:
                        "Dapat JSON pointer na nagsisimula sa / ang {{path}}, gaya ng /data/id.",
                    INVALID_STATES:
                        "Dapat itugma ng {{path}} ang isang status value ng provider sa QUEUED, ACCEPTED, DELIVERED, FAILED o UNKNOWN; kailangan ng kahit isa.",
                    INVALID_AUTH:
                        "Hindi wasto ang {{path}}: ang kind ay URL_KEY, HEADER_SECRET, HMAC_SHA256 o JWT_HS256; kailangan ang header maliban sa URL_KEY; ang encoding ay HEX o BASE64.",
                    INVALID_LIFETIME: "Dapat buong bilang ng segundo na higit sa 0 ang {{path}}.",
                    INVALID_ALGORITHM: "Dapat RS256 o HS256 ang {{path}}.",
                    INVALID_CLAIMS: "Dapat object ang {{path}}.",
                    INVALID_HOURS: "Dapat buong bilang ng oras na higit sa 0 ang {{path}}.",
                },
                placeholders: {
                    title: "Reference ng mga placeholder",
                    help: "Isinusulat sa pagitan ng dobleng curly brace sa URL, sa value ng header o sa anumang text ng body. Pinapalitan ang bawat isa kapag ginawa ang request.",
                },
                placeholder: {
                    to: "Ang tatanggap: numero ng telepono, email address o Page-scoped ID.",
                    text: "Ang mensahe bilang plain text.",
                    subject: "Ang paksa, para sa email.",
                    html: "Ang mensahe bilang HTML, para sa email.",
                    code: "Ang one-time code, para sa mga OTP.",
                    template: "Ang template ng provider na nakaugnay sa election event.",
                    language: "Ang language code ng provider para sa template.",
                    message_id: "Ang message ID ng provider, sa request ng paghahanap ng mensahe.",
                    callback_url: "Ang pampublikong address ng callback ng account na ito.",
                    param: "Isang parameter ng template ayon sa posisyon nito: 1, 2, 3 at iba pa.",
                    credential:
                        "Isang kredensyal ng account na ito ayon sa pangalan: API_KEY, API_SECRET, ACCESS_TOKEN, USERNAME, PASSWORD o WEBHOOK_SECRET.",
                    basic_auth:
                        "Ang username at password, naka-encode para sa header na Authorization: Basic.",
                    token: "Ang token na nakuha sa token request.",
                    jwt: "Ang pinirmahang token na inilarawan sa Pinirmahang token (JWT).",
                    parameters:
                        "Kapag mag-isa bilang value sa body, nagiging listahan ito ng lahat ng parameter ng template.",
                    named_parameters:
                        "Kapag mag-isa bilang value sa body, nagiging object ito ng mga parameter na isinulat bilang @pangalan=halaga.",
                },
                example: {
                    title: "Buong halimbawa: isang Viber partner",
                    description:
                        "Tumatanggap ang partner ng JSON POST na authenticated gamit ang API key bilang bearer token, sumasagot ng ID ng mensahe sa ilalim ng message_id, at nagpo-post ng mga ulat sa paghahatid na may lihim na header. Gamitin ito bilang panimula at palitan ang address at mga pangalan ng field ng sa provider.",
                    use: "Gamitin ang halimbawang ito",
                },
            },
        },
    },
}

export default tagalogTranslation
