// SPDX-FileCopyrightText: 2025 Enric Badia <enric@xtremis.com>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const basqueTranslation: TranslationType = {
    translations: {
        philippinePassport: "Filipinetako Pasaportea",
        seamanBook: "Marinoaren Liburua",
        philSysID: "PhilSys IDa",
        iBP: "Filipinetako Abokatu Bategindua (IBP)",
        driversLicense: "Gidabaimena",
        loading: "Kargatzen...",
        loadingDataProvider: "Datu hornitzailea kargatzen...",
        tallySheetImport: {
            title: "Eskrutinio-akten inportazioak",
            subtitle:
                "Inportatu ES&S edo CSV eskrutinio-akten fitxategiak, aurreikusi sortutako hautetsontziak eta onartu eskrutinio-aktak sortu aurretik.",
            createTitle: "Inportatu eskrutinio-aktak",
            detailTitle: "Eskrutinio-aktaren inportazioa",
            empty: "Oraindik ez dago eskrutinio-akten inportaziorik.",
            emptyBody:
                "Hasi hauteskunde ekitaldi honetarako ES&S Enhanced XML edo CSV kanoniko fitxategi bat inportatuz.",
            sourceFormat: {
                ESS_ENHANCED_XML: "ES&S Enhanced XML",
                CANONICAL_CSV: "CSV kanonikoa",
            },
            channel: {
                PAPER: "Paper",
                POSTAL: "Postaz",
                IN_PERSON: "Aurrez aurre",
            },
            table: {
                created: "Sortua",
                createdBy: "Sortzailea",
                file: "Fitxategia",
                format: "Formatua",
                channel: "Kanala",
                status: "Egoera",
                labels: "Etiketak",
                annotations: "Oharrak",
                actions: "Ekintzak",
            },
            summary: {
                imported: "Inportatuak",
                changed: "Aldatuak",
                new: "Berriak",
                unchanged: "Aldaketarik gabe",
                conflicted: "Gatazkan",
                errors: "Erroreak",
            },
            status: {
                PENDING_REVIEW: "Berrikuspenaren zain",
                APPROVED: "Onartua",
                DISAPPROVED: "Ez onartua",
                FAILED_VALIDATION: "Balidazioak huts egin du",
                CONFLICTED: "Gatazkan",
                NEW: "Berria",
                CHANGED: "Aldatua",
                UNCHANGED: "Aldaketarik gabe",
            },
            fields: {
                format: "Formatua",
                channel: "Kanala",
                supportedFormats: "Onartutako formatuak: XML, CSV",
                generatedTallySheet: "Sortutako eskrutinio-akta",
                sourceCandidates: "Jatorrizko hautagaien IDak",
                none: "Bat ere ez",
            },
            actions: {
                create: "Inportatu eskrutinio-aktak",
                review: "Berrikusi",
                source: "Iturria",
                cancel: "Ezeztatu",
                preview: "Aurreikusi",
                save: "Gorde inportazioa",
                approve: "Onartu",
                disapprove: "Ez onartu",
                close: "Itxi",
                openExisting: "Ireki dagoena",
            },
            notifications: {
                selectFile: "Hautatu inportatzeko fitxategi bat aurreikusi aurretik",
                duplicateSource:
                    "Jatorrizko fitxategiaren hash hau lehendik dagoen eskrutinio-akten inportazio batean agertzen da.",
                uploadUrlError: "Ezin izan da igotzeko URLa sortu",
                uploadError: "Ezin izan da inportazio-fitxategia igo",
                previewEmpty: "Aurreikuspenaren erantzuna hutsik zegoen",
                previewError: "Ezin izan da inportazioa aurreikusi",
                importEmpty: "Inportazioaren erantzuna hutsik zegoen",
                created: "Eskrutinio-aktaren inportazioa sortu da",
                createError: "Ezin izan da inportazioa sortu",
                reviewEmpty: "Berrikuspenaren erantzuna hutsik zegoen",
                conflicted: "Inportazioak zaharkitutako oinarri-gatazkak ditu",
                approved: "Inportazioa onartu da",
                disapproved: "Inportazioa ez da onartu",
                reviewError: "Ezin izan da inportazioa berrikusi",
                sourceUrlError: "Ezin izan da jatorrizko deskarga-URLa sortu",
                sourceDownloadError: "Ezin izan da jatorrizko fitxategia deskargatu",
            },
            pagination: {
                range: "{{rangeStart}}-{{rangeEnd}} / {{total}}",
                previous: "Aurrekoa",
                next: "Hurrengoa",
            },
        },
        reconciliation: {
            menuButton: "Kanpo bozk. sink.",
            categories: {
                VOTED_INTERNET: "Internet bidez bozkatu du",
                VOTED_OTHER_CHANNEL: "Beste kanal batetik bozkatu du",
                DISABLED_DELETE_CALL: "Bozkatzailea desgaituta",
                DELETION_REVERTED: "Ezabaketa desegina",
                PROFILE_UPDATE: "Profila eguneratuta",
                VOTER_ADDED: "Bozkatzailea gehituta",
                REENABLED: "Bozkatzailea berriz gaituta",
                VOTED_UNMARKED: "Bozkatzaileari boto-marka kendu zaio",
                ROW_FAILURE: "Errenkada-errorea",
            },
            table: {
                voterId: "Bozkatzailearen IDa",
                field: "Eremua",
                category: "Kategoria",
                currentValue: "Uneko balioa",
                newValue: "Balio berria",
                reason: "Arrazoia",
                rowLabel: "Errenkada",
                noDifferences: "Ez da desberdintasunik aurkitu - sistemak sinkronizatuta daude.",
            },
            wizard: {
                title: "Kanpoko bat-egite sinkronizazioa",
                subtitle: "Sinkronizatu bozkatzaileen zerrenda kanpoko sistemarekin",
                drop: {
                    description:
                        "Jaregin kanpoko sistemak sortutako bat-egite fitxategia - bi diffak (kanpoko aldea eta Sequent aldea) automatikoki kalkulatzen dira eta taula bereizietan erakusten dira.",
                    fileFormatLabel: "CSV fitxategia",
                    uploading: "{{fileName}} igotzen eta bi diffak kalkulatzen...",
                },
                review: {
                    fileSummary: "{{fileName}} - {{sequence}} sekuentzia, {{generatedAt}} sortua",
                    rowFailuresWarning:
                        "{{count}} errenkada ezin izan dira modu seguruan bateratu eta bi diffetatik kanpo geratzen dira - ikusi xehetasunak behean.",
                    noDifferences:
                        "Ez dago desberdintasunik - bi sistemak jada sinkronizatuta daude.",
                    diffOnlyDifferences:
                        "Dagoeneko aplikatutako sekuentzia baten konbergentzia-egiaztapena da hau. Aldeak jarraipenerako erakusten dira, baina txanda hau ezin da berriro aplikatu.",
                    externalDiffTitle: "Kanpoko diff-a",
                    sequentDiffTitle: "Sequent diff-a",
                    downloadExternalPatch: "Deskargatu kanpoko adabakia",
                    externalDiffCaption:
                        "Deskargatu adabakia eta eman ezazu kanpoko sistemari tresna honetatik kanpo. Adabakia aplikatu eta hurrengo bat-egite fitxategia sortu ondoren, sakatu 'Atzera' eta jaregin fitxategi hori - 'Aplikatu' gaitzen da taula hau hutsik dagoenean.",
                    noExternalDifferences: "Ez dago desberdintasunik kanpoko aldean.",
                    sequentDiffCaption:
                        "Aplikatu aldaketak zuzenean Sequent-i 'Aplikatu' sakatuz - hauetarako ez da adabaki-fitxategirik sortzen.",
                    noSequentDifferences: "Ez dago desberdintasunik Sequent aldean.",
                },
                applying: {
                    inProgress: "Sequent aldeko aldaketak aplikatzen...",
                    rowFailures:
                        "{{count}} errenkada bat-egite honetatik kanpo geratu dira eta eskuzko jarraipena behar dute - ikusi xehetasunak behean.",
                    rowFailuresTruncated:
                        "{{count}} errenkada-hutsegiteetatik lehen {{shown}} erakusten dira. Ebatzi kausa komuna eta saiatu berriro gainerakoak ikusteko.",
                    success: "Sequent aldeko aldaketa guztiak ondo aplikatu dira.",
                },
                actions: {
                    cancel: "Ezeztatu",
                    back: "Atzera",
                    apply: "Aplikatu",
                    next: "Hurrengoa",
                    startOver: "Hasi berriro",
                    close: "Itxi",
                },
                confirm: {
                    title: "Berretsi bat-egite aldaketak",
                    categoriesNote:
                        "Laranjaz nabarmendutako kategoriek ({{categories}}) boto-egoerari eragiten diote edo bozkatzaileak desgaitzen dituzte.",
                    applyChanges: "Aplikatu aldaketak",
                    continue: "Jarraitu",
                },
                summary: {
                    votedOtherChannel:
                        "{{count}} bozkatzaile beste kanal batetik bozkatu izana adierazten du",
                    disabled: "{{count}} bozkatzaile desgaitzen ditu",
                    reenabled: "{{count}} bozkatzaile berriz gaitzen ditu",
                    votedUnmarked: "{{count}} bozkatzaileari boto-marka kentzen dio",
                    profileUpdated: "{{count}} profil eguneratzen ditu",
                    voterAdded: "{{count}} bozkatzaile gehitzen ditu",
                    prefix: "Honek {{parts}} dituzten aldaketak aplikatuko ditu.",
                    empty: "Ez dago Sequent aldeko aldaketarik aplikatzeko.",
                },
                notifications: {
                    envelopeLoadError: "Ezin izan da bat-egite diff-a kargatu - saiatu berriro.",
                    generateFailed:
                        "Ezin izan da bat-egite diff-a kalkulatu - ikusi zereginen widget-a xehetasunetarako.",
                    applyFailed:
                        "Ezin izan dira Sequent aldeko aldaketak aplikatu - ikusi zereginen widget-a xehetasunetarako.",
                    uploadUrlError: "Ezin izan da igotzeko URLa lortu",
                    generateTaskError: "Ezin izan da bat-egite diff zeregina abiarazi",
                    uploadError: "Ezin izan da bat-egite fitxategia igo",
                    applyTaskError: "Ezin izan da aplikatzeko zeregina abiarazi",
                    applyError: "Ezin izan dira bat-egite aldaketak aplikatu",
                },
            },
        },
        logsScreen: {
            noPermissions: "Ez duzu egunkariak atzitzeko baimenik.",
            title: "Egunkariak",
            subtitle: "Datu-base nagusi eta IAMaren egunkari orokorrak",
            actions: {
                csv: "CSV formatuan esportatu",
                pdf: "PDF formatuan esportatu",
            },
            exportdialog: {
                description:
                    "Mesedez, berretsi ekintza hau exekutatu nahi duzula, denbora pixka bat hartu dezake.",
                title: "Esportatu erregistroak",
                from: "Noiztik",
                to: "Noiz arte",
                timeZone: "Ordu-eremua",
                format: "Formatua",
                csv: "CSV",
                pdf: "PDF",
                zoneNote:
                    "Errenkada bakoitzak bere ordua UTC-n (ISO 8601) eta {{abbr}}-n gordetzen du, ordu-eremuaren izenarekin. Data-tarteak bi muturrak barne hartzen ditu, {{abbr}}-n.",
                zoneNotePdf:
                    "PDFak ordu bakoitza {{abbr}}-n erakusten du. Data-tarteak bi muturrak barne hartzen ditu, {{abbr}}-n.",
                rowZones: "Errenkada bakoitzaren hauteskundearen ordu-eremua",
                zoneNoteRows:
                    "Errenkada bakoitzak bere ordua UTC-n (ISO 8601) eta bere hauteskundearen ordu-eremuan gordetzen du, ordu-eremuaren izenarekin. Data-tarteak bi muturrak barne hartzen ditu, {{abbr}}-n.",
                zoneNoteRowsPdf:
                    "PDFak ordu bakoitza bere hauteskundearen ordu-eremuan erakusten du. Data-tarteak bi muturrak barne hartzen ditu, {{abbr}}-n.",
            },
            filter: {
                createdFrom: "Sortua noiztik",
                createdTo: "noiz arte",
                statementTimestampFrom: "Adierazpenaren denbora-zigilua noiztik",
                statementTimestampTo: "Adierazpenaren denbora-zigilua noiz arte",
                timeZone: "Ordu-eremua",
            },
            scheduledOutcome: {
                outcome: {
                    "waiting-for-initialization": "Hasieratzearen zain",
                    "runs": "exekutatzen da",
                    "runs-unsigned": "sinadurarik gabe exekutatzen da",
                    "refused": "baztertzen da",
                },
                check: {
                    "initialization": "Beharrezko hasieratzea osatu gabe dago",
                    "voting-close": "Bozketa ezin da itxiera-epearen ondoren ireki",
                    "needs-signatures": "sinadurak behar dira",
                    "covered": "sinatutako konfigurazioan",
                    "unsigned-close": "sinadurarik gabeko itxiera",
                    "stricter-copy": "uneko eta argitaratutako ezarpenak",
                    "defaults": "ez da ezer argitaratu oraindik",
                },
                changed: "Orain {{after}} (lehen: {{before}}).",
                result: "Emaitza: {{outcome}}.",
                deciding: "Egiaztapen erabakigarria: {{check}}. {{value}}",
                authorizedBy: "{{code}} konfigurazioak baimendua.",
                nextStep: "Hurrengo urratsa: {{step}}",
                reason: {
                    "ballot-box-seal-policy":
                        "Itxita mantendu da: Zigilatu ixtean aukerarekin, itxi den bozketa itxita geratzen da.",
                    "never-opened-kept-open":
                        "Ez dago ezer programazioz ixteko: Postua ez da inoiz ireki, beraz dagoen bezala geratzen da.",
                },
            },
            column: {
                id: "IDa",
                statement_kind: "Adierazpen mota",
                created: "Sortua",
                statement_timestamp: "Adierazpen denbora-marka",
                message: "Mezua",
                user_id: "Erabiltzaile IDa",
                username: "Erabiltzaile izena",
                sender_pk: "Bidaltzaile Pk",
                log_type: "Egunkari Mota",
                event_type: "Gertaera Mota",
                description: "Deskribapena",
                version: "Bertsioa",
            },
            main: {
                title: "Datu-base Nagusiaren Egunkariak",
            },
            iam: {
                title: "IAM Datu-basearen Egunkariak",
            },
            ballotBoxSeal: {
                sealHash: "Zigiluaren hash-a: {{hash}}",
                counted: "Hautestontziko {{inBox}} botoetatik {{counted}} zenbatu dira.",
                notCounted_one:
                    "Beste botoa bozkatzailearen geroagoko boto batek ordezkatu zuen, baztertu egin zen, edo boto-eskubiderik ez duen bozkatzaile batek eman zuen.",
                notCounted_other:
                    "Beste {{count}} botoak bozkatzailearen geroagoko boto batek ordezkatu zituen, baztertu egin ziren, edo boto-eskubiderik ez duen bozkatzaile batek eman zituen.",
                closeRequest: "«Bozketa itxi» eskaerak itxi du: {{request}}.",
                noCloseRequest:
                    "«Bozketa itxi» eskaerarik gabe itxi da (Gelditu Bozketa edo programatutako itxiera).",
                failedReason: "Zergatia: {{reason}}",
                failedLocked:
                    "Hautestontzia blokeatuta geratzen da eta ez dago zigilatuta: gorabehera bat da.",
                verifiedCounted: "Zigilutik {{counted}} boto zenbatu dira.",
                tallySession: "Zenbaketa-saioa: {{session}}.",
                differs: "Zer da desberdina: {{differs}}",
            },
        },
        tasksScreen: {
            noPermissions: "Ez duzu egunkariak atzitzeko baimenik.",
            title: "Ataza Exekuzioa",
            subtitle: "Exekutatutako atazei buruzko informazioa",
            taskInformation: "Ataza Informazioa",
            status: "egoera: {{status}}",
            ok: "Ados",
            column: {
                id: "Indizea",
                name: "Ataza izena",
                type: "Mota",
                execution_status: "Egoera",
                start_at: "Hasiera ordua",
                end_at: "Amaiera ordua",
                executed_by_user: "Exekutatzailea",
                annotations: "Oharrak",
                labels: "Etiketak",
                logs: "Egunkariak",
            },
            tasksExecution: {
                DELETE_TENANT: "Ezabatu maizterra",
                PUBLISH_BALLOT: "Boto-papera argitaratu",
                VOTER_INFORMATION_LETTER: "Hauteslearen informazio-gutuna",
                EXPORT_MONITORING_DATA: "Esportatu Monitorizazio Datuak",
                EXPORT_ELECTION_EVENT: "Esportatu Hauteskunde Gertaera",
                CREATE_ELECTION_EVENT: "Sortu Hauteskunde Gertaera",
                IMPORT_ELECTION_EVENT: "Inportatu Hauteskunde Gertaera",
                IMPORT_USERS: "Inportatu Erabiltzaileak",
                EDIT_USER: "Editatu Hauteslea",
                IMPORT_CANDIDATES: "Inportatu Hautagaiak",
                EXPORT_VOTERS: "Esportatu Bozkatzaileak",
                CREATE_TRANSMISSION_PACKAGE: "Sortu Transmisio Paketea",
                EXPORT_BALLOT_PUBLICATION: "Esportatu Bozketa Argitalpena",
                EXPORT_ACTIVITY_LOGS_REPORT: "Esportatu Jarduera Egunkarien Txostena",
                GENERATE_REPORT: "Sortu Txostena",
                GENERATE_TRANSMISSION_REPORT: "Sortu Transmisio Txostena",
                EXPORT_TRUSTEES: "Esportatu Fideikomisarioak",
                EXPORT_APPLICATION: "Esportatu Aplikazioak",
                EXPORT_TENANT_CONFIG: "Esportatu Maizter Konfigurazioa",
                IMPORT_TENANT_CONFIG: "Inportatu Maizter Konfigurazioa",
                RENDER_DOCUMENT_PDF: "Errendatu Dokumentu PDFa",
                CREATE_TENANT: "Maizterra Sortu",
                EXPORT_TEMPLATES: "Txantiloiak Esportatu",
                IMPORT_TEMPLATES: "Txantiloiak Inportatu",
                DELETE_ELECTION_EVENT: "Ezabatu Hauteskunde Gertaera",
                DELETE_VOTERS: "Delete Voters",
                PREPARE_PUBLICATION_PREVIEW: "Argitalpenaren aurrebista prestatu",
                EXPORT_TALLY_RESULTS_XLSX: "Esportatu zenbaketa-emaitzak XLSX formatuan",
                EXPORT_CERTIFICATE_AUTHORITIES: "Ziurtagiri-agintaritzak esportatu",
                PUBLISH_RESULTS_WEBSITE: "Argitaratu emaitzen webgunea",
            },
            documentAccess: {
                title: "Dokumenturako sarbidea",
                sensitivityNotice:
                    "Informazio sentikorra. Partekatu pasahitz hau aurreikusitako hartzailearekin soilik.",
                passwordLabel: "PDF zifratua irekitzeko pasahitza",
                showPassword: "Erakutsi pasahitza",
                copyPassword: "Kopiatu pasahitza",
                passwordCopied: "Pasahitza kopiatu da",
                passwordError: "Ezin izan da PDFaren pasahitza eskuratu",
                copyError: "Ezin izan da pasahitza kopiatu",
                guidance:
                    "Pasahitza Erakutsi pasahitza aukeratu ondoren bakarrik kargatzen da. Kargatu ondoren, kopiatzeko aukera duen irakurtzeko soilik den eremu bat agertuko da hemen.",
            },
            widget: {
                taskTitle: "Ataza: {{title}}",
                viewTask: "Ikusi Ataza",
                downloadDocument: "Deskargatu Fitxategia",
                downloadHashManifest: "Hash manifestua",
            },
            exportTasksExecution: {
                success: "Esportazioa arrakastaz amaitu da",
                error: "Errorea Ataza Exekuzioa esportatzerakoan",
            },
        },
        areas: {
            common: {
                title: "Eremuak",
                subTitle: "Eremu konfigurazioa.",
                deleteError: "Errorea eremua ezabatzerakoan",
            },
            createAreaSuccess: "Eremua sortua",
            updateAreaSuccess: "Eremua aldatua",
            createAreaError: "Ezin izan da Eremua sortu",
            sequent_backend_area_contest: "Lehiaketak",
            empty: {
                header: "Ez dago Eremurik oraindik.",
                action: "Sortu Eremua",
            },
            formImputs: {
                allowEarlyVoting: "Onartu Goiztiarra Bozketa",
            },
        },
        integrationsScreen: {
            common: {
                gapiKey: "Google Calendar Zerbitzu Kontu Giltza",
                gapiEmail: "Google Calendar Autentifikazio Helbide Elektronikoa",
                gapiKeyHelper:
                    "Gordetako giltza ez da erakusten. Itsatsi giltza berri bat ordezkatzeko.",
                gapiKeySaved: "Google Calendar Zerbitzu Kontu Giltza gorde da",
            },
            errors: {
                invalidGapiKey: "Google Calendar Zerbitzu Kontu Giltza formatu baliogabea",
                saveGapiKey: "Ezin izan da Google Calendar Zerbitzu Kontu Giltza gorde",
            },
        },
        lookAndFeelScreen: {
            common: {
                helpLinks: "Laguntza Estekak",
                logoUrl: "Logo URLa",
                css: "CSS Pertsonalizatua",
                displayName: "Bistaratzeko izena",
                displayNameHelp:
                    "Erakundearen izena hura aipatzen duten mezuetan. Hutsik: maizterraren izen laburra.",
            },
            errors: {
                invalidHelpLinks: "Laguntza Esteken formatu baliogabea",
            },
        },
        electionTypeScreen: {
            noPermissions: "Ez duzu ezarpenak atzitzeko baimenik.",
            common: {
                title: "Hauteskunde Mota",
                subtitle: "Hauteskunde mota konfigurazioa",
                onlineVoting: "Lineko Bozketa",
                kioskVoting: "Kiosko Bozketa",
                telephoneVoting: "Telefono Bozketa",
                settingTitle: "Ezarpenak",
                settingSubtitle: "Konfigurazio Orokorra",
                createNew: "Sortu Hauteskunde Mota",
                emptyHeader: "Ez dago Hauteskunde Motarik oraindik.",
                emptyBody: "Bat sortu nahi duzu?",
            },
            create: {
                title: "Sortu Hauteskunde Mota",
            },
            edit: {
                title: "Editatu Hauteskunde Mota",
            },
            tabs: {
                votingChannels: "BOZKETA KANALAK",
                electionTypes: "HAUTESKUNDE MOTAK",
                languages: "HIZKUNTZAK",
                localization: "LOKALIZAZIOA",
                integrations: "Integrazioak",
                lookAndFeel: "Itxura eta Sentimendua",
                schedules: "PROGRAMATUTAKO GERTAERAK",
                trustees: "FIDEIKOMISARIOAK",
                BackupRestore: "Babeskopia / Leheneratu",
            },
        },
        trusteesSettingsScreen: {
            common: {
                emptyHeader: "Ez dago Fideikomisariorik oraindik.",
                createNew: "Sortu Fideikomisarioa",
                title: "Fideikomisarioa",
                subtitle: "Fideikomisario konfigurazioa",
                emptyBody: "Bat sortu nahi duzu?",
            },
            create: {
                title: "Sortu Fideikomisarioa",
            },
            edit: {
                title: "Editatu Fideikomisarioa",
            },
        },
        scheduleScreen: {
            noPermissions: "Ez duzu ezarpenak atzitzeko baimenik.",
            createScheduleSuccess: "Ordutegia sortua",
            createScheduleError: "Errorea ordutegia sortzerakoan",
            deleteScheduleSuccess: "Ordutegia ezabatua",
            deleteScheduleError: "Errorea ordutegia ezabatzerakoan",
            common: {
                title: "Programatua",
                subtitle: "Ordutegi konfigurazioa",
                createNew: "Sortu Ordutegia",
                emptyHeader: "Ez dago Ordutegiarik oraindik.",
                emptyBody: "Bat sortu nahi duzu?",
            },
            create: {
                title: "Sortu Ordutegia",
                selectSchedule:
                    "Hautatu ordutegi bat aurrez definitutako zerrendatik edo idatzi pertsonalizatu bat",
            },
            edit: {
                title: "Editatu Ordutegia",
            },
            eventTypes: {
                SYSTEM_LOCKDOWN_FOR_INTERNET_VOTING_SETTINGS:
                    "Sistema blokeoa Internet bozketa ezarpenen amaitzeko",
                START_PRE_REGISTRATION_OVCS: "OVCS aurre-erregistroaren hasiera eta amaiera",
                END_PRE_REGISTRATION_OVCS: "OVCS aurre-erregistroaren amaiera",
                START_TEST_VOTING_PERIOD: "Proba bozketa aldiaren hasiera",
                END_TEST_VOTING_PERIOD: "Proba bozketa aldiaren amaiera",
                START_INTERNET_VOTING_PERIOD: "Internet bozketa aldiaren hasiera",
                END_INTERNET_VOTING_PERIOD: "Internet bozketa aldiaren amaiera",
                LAB_TEST: "Laboratorio proba",
                FIELD_TEST: "Eremu proba",
                MOCK_ELECTIONS: "Hauteskunde faltsuak",
                FTS: "FTS",
            },
        },
        dashboard: {
            voteByDay: "Eguneko botoak",
            votesOverTime: "Botoak denboran zehar",
            timeResolution: "Denbora-bereizmena",
            timeRange: "Denbora-tartea",
            minute: "Minutua",
            hour: "Ordua",
            day: "Eguna",
            votersByChannels: "Kanaleko bozkatzaileak",
            voterLoginURL: "Bozkatzaile Sarrera URLa",
            voterEnrollURL: "Bozkatzaile Matrikula URLa",
            voterEnrollKioskURL: "Bozkatzaile Matrikula Kiosko URLa",
            ballotBoxes: {
                show: "Erakutsi hautestontziak",
                loadError:
                    "Ezin izan dira hautestontzien zigiluak irakurri. Kargatu berriro orria, edo egiaztatu zerbitzariarekiko konexioa.",
                title: "Hautestontziak",
                sealing:
                    "Bozketa itxi da ({{closed}}). Hautestontziak grazia-aldia amaitzean zigilatuko dira ({{deadline}}).",
                sealed: "Bozketa itxi da ({{closed}}). Hautestontziak zigilatuta daude: ezin da botorik gehitu, aldatu edo ezabatu.",
                failed: "Bozketa itxi da ({{closed}}). Hautestontzi bat ezin izan da zigilatu: blokeatuta geratzen da, eta gorabehera egunkarietan dago.",
                closedBySignatures:
                    "Itxi dutenak: {{names}}, beren ziurtagiriekin, {{code}} sinadura-kodearekin.",
                closedByUser: "Itxi duena: {{username}}.",
                closedBySchedule: "Programatutako bozketa-itxierak itxi du.",
                column: {
                    area: "Eremua",
                    status: "Egoera",
                    inTheBox: "Hautestontzian",
                    counted: "Zenbatuak",
                    sealedAt: "Zigilatua",
                    sealHash: "Zigiluaren hash-a",
                    record: "Zigiluaren erregistroa",
                },
                status: {
                    open: "Irekita",
                    sealing: "Zigilatzea: {{time}}",
                    publishing: "Zigilatuta, argitaratzen",
                    sealed: "Zigilatuta",
                    failed: "Zigilatu gabe: gorabehera",
                    due: "Zigilatzen orain",
                    overdue: "Zigilatzea berandu",
                },
                help: {
                    publishing:
                        "Hautestontzia blokeatuta dago. Iragarki-taulako bere sarrera berriro argitaratzen ari da.",
                    counted:
                        "Zenbatzen diren botoak: boto-eskubidea duen bozkatzaile bakoitzaren azken boto baliozkoa. Hautestontziko gainerakoak bozkatzailearen geroagoko boto batek ordezkatu zituen, baztertu egin ziren, edo boto-eskubiderik ez duen bozkatzaile batek eman zituen.",
                },
                copyHash: "Kopiatu zigiluaren hash-a",
                copied: "Zigiluaren hash-a kopiatu da",
                copyError: "Ezin izan da zigiluaren hash-a kopiatu",
                notYet: "Oraindik ez",
                openRecord: "Ireki zigiluaren erregistroa (eremua: {{area}})",
                downloadRecord: "Deskargatu zigiluaren erregistroa (eremua: {{area}})",
                recordRestricted:
                    "Mugatua: eskatu dokumentuak deskarga ditzakeen administratzaile bati.",
                recordError: "Ezin izan da zigiluaren erregistroa deskargatu. Saiatu berriro.",
                recordMissing:
                    "Zigiluaren erregistroaren dokumentua falta da: jakinarazi gorabehera gisa.",
                beforeClose: "Eremu bakoitzeko hautestontzia bozketa ixtean zigilatzen da.",
                notStarted:
                    "Bozketa ez da oraindik ireki. Eremu bakoitzeko hautestontzia bozketa ixtean zigilatzen da.",
                openOn: "Bozketa irekita dago kanal hauetan: {{channels}}. Eremu bakoitzeko hautestontzia bozketa ixtean zigilatzen da.",
                paused: "Bozketa pausatuta dago. Eremu bakoitzeko hautestontzia bozketa ixtean zigilatzen da.",
                holding_one:
                    "{{channels}} gaituta dago eta ez da itxi: gelditu ezazu hautestontziak zigilatzeko.",
                holding_other:
                    "{{channels}} gaituta daude eta ez dira itxi: gelditu itzazu hautestontziak zigilatzeko.",
                sealingNow: "Bozketa itxi da ({{closed}}). Hautestontziak zigilatzen ari dira.",
                sealingPastGrace:
                    "Bozketa itxi da ({{closed}}). Grazia-aldia amaitu da ({{deadline}}); hautestontziak zigilatzen ari dira.",
                why: {
                    due: "Zigilatzen ari da: minutu bat arte iraun dezake.",
                    channelOpen:
                        "{{channel}} gaituta dago oraindik eta ez da itxi: gelditu ezazu hautestontzia zigilatzeko.",
                    channelNotEnabled:
                        "{{channel}} ez dago itxita, ezta gaituta ere hauteskunde honetan: gelditu ezazu hautestontzia zigilatzeko.",
                    channelHasBallots:
                        "{{channel}} kanalak botoak ditu hautestontzi honetan eta ez da itxi: gelditu ezazu hautestontzia zigilatzeko.",
                    datafixVotes_one:
                        "{{count}} boto Datafix-en bideratzen ari da: hautestontzia ebazten denean zigilatuko da.",
                    datafixVotes_other:
                        "{{count}} boto Datafix-en bideratzen ari dira: hautestontzia ebazten direnean zigilatuko da.",
                    stale: "Azken saiakera: {{time}}. Baliteke zigilatzailea martxan ez egotea. Egiaztatu Beat eta zigilatze-langilea.",
                    notTried:
                        "Oraindik ez da saiatu: baliteke zigilatzailea martxan ez egotea. Egiaztatu Beat eta zigilatze-langilea.",
                    errorCategory: {
                        board: "Azken saiakerak ezin izan du iragarki-taulara iritsi; minuturo saiatzen da berriro.",
                        census: "Azken saiakerak ezin izan du bozkatzaile-zerrenda irakurri; minuturo saiatzen da berriro.",
                        keystore:
                            "Azken saiakerak ezin izan du sinadura-gakoa lortu; minuturo saiatzen da berriro.",
                        storage:
                            "Azken saiakerak ezin izan du zigiluaren erregistroa fitxategi-biltegira igo; minuturo saiatzen da berriro.",
                        settings:
                            "Azken saiakerak ezin izan ditu hauteskundearen ezarpenak irakurri; minuturo saiatzen da berriro.",
                        other: "Azken saiakerak huts egin du; minuturo saiatzen da berriro. Zerbitzuaren egunkarian daude xehetasunak.",
                        ballots:
                            "Azken saiakerak oraindik irakurri ezin den edo oraindik bideratzen ari den boto bat aurkitu du; minuturo saiatzen da berriro.",
                        database:
                            "Azken saiakera ezin izan da datu-basean osatu; minuturo saiatzen da berriro.",
                    },
                },
                failure: {
                    ballotIdMismatch: "Boto bat ez dator bat bere Boto IDarekin.",
                    missingContent: "Boto batek ez du edukirik edo Boto IDrik.",
                    unreadable: "Ezin da boto bat irakurri.",
                    inProgress: "Boto bat oraindik bideratzen ari da.",
                    alreadyOnBoard: "Hautestontzi honen zigilu bat badago jada iragarki-taulan.",
                    noBoard: "Hauteskunde-gertaerak ez du iragarki-taularik.",
                    unknownChannel: "Boto batek boto-kanal ezezagun bat du.",
                },
                incident: {
                    title_one: "{{count}} hautestontzi ezin izan da zigilatu",
                    title_other: "{{count}} hautestontzi ezin izan dira zigilatu",
                    body: "Gorabehera bat da: hautestontzi horietako bakoitza blokeatuta geratzen da eta ezin da zenbatu. Jarraitu zigilatze hutsetarako prozedura.",
                    line: "{{election}}, {{area}}: {{reason}}",
                },
            },
            ipAddress: {
                emptyState: "Ez dago botorik oraindik.",
                title: "IP Helbideak",
                ip: "IP",
                country: "Herrialdea",
                VoteCount: "Boto Kopurua",
                ElectionName: "Hauteskunde Izena",
                VotersId: "Bozkatzaileen IDa",
            },
        },
        electionEventScreen: {
            common: {
                subtitle: "Hauteskunde gertaera konfigurazioa.",
                showMore: "Erakutsi Gehiago",
                showLess: "Erakutsi Gutxiago",
                adminPortal: "Admin Portala",
                allowPublishAfterLockdown:
                    "Hauteskunde gertaera argitalpena blokearen ondoren soilik baimendu",
                reset: "Berrezarri iragazki pertsonalizatua",
            },
            edit: {
                general: "Orokorra",
                dates: "Datak",
                customUrls: "URL Aurrizki Pertsonalizatuak",
                votingPeriod: "Bozketa Aldia",
                language: "Hizkuntza",
                allowed: "Baimendutako Bozketa Kanalak",
                materials: "Laguntza Materialak",
                ballotReceipts: "Boto-txartelen ordezkagiriak",
                ballotDesign: "Bozketa Diseinua",
                templates: "Txantiloiak",
                reorder: "Berrantolatu hauteskundeak",
                advancedConfigurations: "Konfigurazio Aurreratuak",
                importCandidates: "Inportatu Hautagaiak",
                custom_filters: "Iragazki pertsonalizatuak",
                voter_authentication: "Bozkatzaile Autentifikazioa",
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
                login: "Sarrera",
                enrollment: "Matrikula",
            },
            localization: {
                emptyHeader: "Ez da hizkuntzarik ezarri gertaeraarako",
                selectLanguage: "Hautatu Hizkuntza",
                notify: {
                    success: "Lokalizazioa arrakastaz eguneratua",
                    error: "Lokalizazio eguneraketa huts egin du",
                    duplicateKey: "Giltza eta atari-esparru hori dituen ordezkapen bat badago.",
                    invalidDateTimeFormat:
                        "Data/orduaren formatu baliogabea. Erabili yyyy, MM, dd, HH, mm, ss tokenak (adib. dd/MM/yyyy HH:mm).",
                    invalidTimeZoneText: "Testu honek {{placeholders}} gorde behar ditu.",
                },
                common: {
                    title: "Lokalizazioa",
                    subTitle: "Lokalizazio konfigurazioa",
                },
                labels: {
                    key: "Giltza",
                    scope: "Atariaren esparrua",
                    value: "Balioa",
                },
                scopes: {
                    legacy: "Legatua ({{portal}})",
                    global: "Globala",
                    votingPortal: "Bozketa-ataria",
                    ballotVerifier: "Boto-paperen egiaztatzailea",
                    resultsPortal: "Emaitzen ataria",
                    adminPortal: "Administrazio-ataria",
                    templates: "Txostenak eta mezuak",
                },
            },
            field: {
                passwordPolicy: {
                    minimumLength: "Gutxieneko luzera",
                    maximumLength: "Gehieneko luzera",
                    includeUppercase: "Sartu letra larriak",
                    includeLowercase: "Sartu letra xeheak",
                    includeDigits: "Sartu digituak",
                    includeSpecialCharacters: "Sartu karaktere bereziak",
                    help: {
                        minimumLength: "Pasahitzak izan behar duen gutxieneko karaktere kopurua.",
                        maximumLength: "Pasahitzak izan dezakeen gehieneko karaktere kopurua.",
                        includeUppercase: "Pasahitzak gutxienez letra larri bat izan behar du.",
                        includeLowercase: "Pasahitzak gutxienez letra xehe bat izan behar du.",
                        includeDigits: "Pasahitzak gutxienez digitu bat izan behar du.",
                        includeSpecialCharacters:
                            "Pasahitzak gutxienez karaktere berezi bat izan behar du.",
                    },
                    notConfigured:
                        "Ez dago pasahitz-politikarik konfiguratuta. Gordetzean, beheko balio lehenetsiak aplikatuko dira.",
                    errors: {
                        lengthRange:
                            "Pasahitzaren luzera-balioek 1 eta 256 arteko zenbaki osoak izan behar dute.",
                        minimumExceedsMaximum:
                            "Gutxieneko luzerak ezin du gehieneko luzera gainditu.",
                        characterClassRequired:
                            "Hautatu gutxienez karaktere-klase bat pasahitzerako.",
                    },
                },
                name: "Izena",
                alias: "Ezizena",
                description: "Deskribapena",
                startDateTime: "Hasiera Data eta Ordua",
                endDateTime: "Amaiera Data eta Ordua",
                language: "Hizkuntza",
                votingChannels: "Bozketa Kanalak",
                materialActivated: "Laguntza Materialak Aktibatuta",
                supportMaterialsPolicy: {
                    label: "Laguntza Materialen Politika",
                    helperText:
                        "Bozkatzeko Derrigorrezkoa aukeratzen bada, bozkatzaileek boto bat eman ahal izan aurretik Laguntza Material bakoitza ireki eta irakurri dutela berretsi beharko dute.",
                    options: {
                        off: "Desaktibatuta",
                        optional: "Aukerakoa",
                        mandatory_for_voting: "Bozkatzeko Derrigorrezkoa",
                    },
                },
                materialTitle: "Izenburua",
                materialSubTitle: "Azpititulua",
                logoUrl: "Logo URLa",
                userVerification:
                    "Bozkatzaileak eskuz egiaztatzeko erabiliko den txantiloi pertsonalizatu bat sar dezakezu",
                redirectFinishUrl: "Berbideratu Amaiera URLa",
                kioskRedirectFinishUrl: "Kioskoaren amaierako berbideratze URLa",
                css: "CSS Pertsonalizatua",
                skipElectionList: "Saltatu Hauteskunde Zerrenda Pantaila",
                showUserProfile: "Erakutsi Erabiltzaile Profila",
                ballotReceipts: {
                    checksPeriod: {
                        policyLabel: "Emandako botoak egiaztatzeko epea",
                        helper: "Zenbat denboraz bilatu dezaketen hautesleek emandako botoa eta inprimatu haren agiria Bozketa Atarian.",
                        options: {
                            "unlimited": "Mugarik gabe",
                            "until-date": "Data batera arte",
                        },
                    },
                    checksAvailableUntil: "Egiaztapenak noiz arte ({{timezone}})",
                    checksAvailableUntilRequired:
                        "Sartu botoak noiz arte egiazta daitezkeen adierazten duen data eta ordua.",
                },
                voterAccessibilitySettingsPolicy: {
                    policyLabel: "Boto-emailearen irisgarritasun-ezarpenak",
                    options: {
                        disabled: "Ezkutatu irisgarritasun-ezarpenak",
                        enabled: "Eskaini testuaren tamaina, kontrastea, tartea eta mugimendua",
                    },
                },
                audioInstructionsPolicy: {
                    policyLabel: "Audio-argibideak",
                    options: {
                        "disabled": "Audio-argibiderik ez",
                        "recorded": "Igotako grabazioak soilik",
                        "recorded-or-synthesized":
                            "Igotako grabazioak, edo nabigatzailearen ahotsa halakorik ez dagoenean",
                    },
                },
                showCastVoteLogs: {
                    policyLabel: "Erakutsi Logs Bozketa Taba",
                    options: {
                        "show-logs-tab": "Erakutsi Logs Bozketa Taba",
                        "hide-logs-tab": "Ez Erakutsi Logs Bozketa Taba",
                    },
                },
                lockdownState: {
                    policyLabel: "Blokeo Egoera",
                    helperText:
                        "Programatu blokeo-aldiaren hasiera edo amaiera egoera hau aldatzeko.",
                    options: {
                        "locked-down": "Blokeatuta",
                        "not-locked-down": "Blokeatu gabe",
                    },
                },
                ballotBoxSealPolicy: {
                    policyLabel: "Hautestontzien Zigilu Politika",
                    helperText:
                        "Bozketa ixtean, eremu bakoitzeko hautestontzia zigilatzen da: bere botoen hash sinatu bat iragarki-taulara bidaltzen da, ezin da botorik gehitu, aldatu edo ezabatu, eta bozketa ezin da berriro hasi.",
                    locked: "Ezin da aldatu bozketa ireki ondoren.",
                    options: {
                        "seal-at-close": "Zigilatu ixtean",
                        "do-not-seal": "Ez zigilatu",
                    },
                    checking: "Bozketa ireki den egiaztatzen…",
                    lockedUnknown:
                        "Blokeatuta: ezin izan dira hauteskundeak irakurri, beraz ez dakigu bozketa ireki den.",
                    lockedOpened: "Blokeatuta: bozketa ireki da hauteskunde hauetan: {{names}}.",
                    lockedEvent: "Blokeatuta: bozketa ireki da hauteskunde-gertaera honetan.",
                    refused: "Hautestontzien Zigilu Politika ezin da aldatu bozketa ireki ondoren.",
                    settingLocked: "Zigilatu ixtean aukerarekin, zigilua ezarpen honen mende dago.",
                    settingRefused:
                        "Ezarpen hau ezin da aldatu bozketa ireki ondoren: Zigilatu ixtean aukerarekin, zigilua haren mende dago.",
                    boardRefused:
                        "Hauteskunde-gertaeraren iragarki-taula ezin da aldatu bozketa ireki ondoren: Zigilatu ixtean aukerarekin, zigiluak bertan argitaratzen dira.",
                },
                ballotBoxSealRecordPolicy: {
                    policyLabel: "Hautestontziaren zigiluaren erregistroa",
                    options: {
                        restricted: "Mugatua",
                        public: "Publikoa",
                    },
                    help: {
                        restricted:
                            "Administratzaileek soilik deskarga dezakete; partekatu behatzaileekin.",
                        public: "Gertaeraren identifikatzaileak dituen edonork deskarga dezake, saioa hasi gabe; boto bakoitza nola zenbatu den erakusten du.",
                    },
                },
                decodedBallots: {
                    policyLabel: "Deskodetutako boto-paperak emaitzen datu-basean sartu",
                    options: {"included": "Sartu", "not-included": "Ez sartu"},
                },
                contestEncryptionPolicy: {
                    options: {
                        "single-contest": "Lehiaketa Bakarra",
                        "multiple-contests": "Lehiaketa Anitzak",
                    },
                    policyLabel: "Lehiaketa zifratze politika",
                },
                votingPortalDateTimeFormat: {
                    policyLabel: "Bozketa-atariko data eta orduaren formatua",
                    helperText:
                        'Gertaera osoari aplikatzen zaio. Hizkuntza bakoitzeko gainjartzeko, gehitu "votingPortalDateTimeFormat" gakoa Lokalizazioa fitxan, yyyy, MM, dd, HH, mm, ss tokenak erabiliz (adib. dd/MM/yyyy HH:mm). Ikus dokumentazioa xehetasunetarako.',
                    options: {
                        "legacy-gb-24h": "Legacy GB 24h (dd/MM/yyyy HH:mm, 24h)",
                        "iso-local": "ISO Local (yyyy-MM-dd HH:mm)",
                        "us-12h": "US 12h (MM/dd/yyyy h:mm AM/PM)",
                        "locale-medium": "Locale Medium (data ertaina, ordu laburra)",
                        "date-only": "Date Only (ordurik gabe)",
                        "custom": "Formatu pertsonalizatua",
                    },
                    customFormat: {
                        label: "Data eta ordu formatu pertsonalizatua",
                        helperText:
                            "Erabili yyyy, MM, dd, HH, mm, ss tokenak (adib. dd/MM/yyyy HH:mm). Beste edozein karaktere literalki erakusten da.",
                        invalid:
                            "Formatu baliogabea. Erabili yyyy, MM, dd, HH, mm, ss tokenetako bat gutxienez.",
                    },
                },
                countDownPolicyOptions: {
                    NO_COUNTDOWN: "Ez dago Kontaketa Atzera",
                    COUNTDOWN: "Kontaketa Atzera",
                    COUNTDOWN_WITH_ALERT: "Kontaketa Atzera alerta batekin",
                    sectionTitle: "Bozketa Portala",
                    policyLabel: "Bozketa Portal Kontaketa Atzera politika",
                    coundownSecondsLabel:
                        "iraungitze aurretik kontaketa atzera erakusteko denbora segundotan",
                    alertSecondsLabel:
                        "iraungitze aurretik Saioa Itxi alerta erakusteko denbora segundotan",
                },
                voterSigningPolicy: {
                    "policyLabel": "Bozkatzaile Sinadura Politika",
                    "no-signature": "Sinadurarik ez",
                    "with-signature": "Sinadura batekin",
                },
                receiptsPolicy: {
                    "policyLabel": "Hautetsontziak sinatutako ordezkagiriak",
                    "disabled": "Desgaituta",
                    "signed-by-ballot-box": "Hautetsontziak sinatuta",
                    "helperText":
                        "Aktibatuta dagoenean, hautetsontziak boto-txartel bakoitza berrikuspen-pantailan gorde eta sinatzen du, eta bozkatzaileak boto-txartelaren IDa hautetsontziak jaso ondoren bakarrik ikusten du. Bozkatzaileek beren boto-txartelak sinatzen dituzte. Aldatu ondoren, argitaratu berriro boto-txartelak.",
                    "lockedHelperText": "Ezin da aldatu bozketa hasi ondoren.",
                },
                VoterCertificatePolicy: {
                    policyLabel: "Voter Digital Certificate Policy",
                    enabled: "Gaituta",
                    disabled: "Desgaituta",
                },
                enrollment: {
                    policyLabel: "Matrikula",
                    options: {
                        enabled: "Gaituta",
                        disabled: "Desgaituta",
                    },
                },
                otp: {
                    policyLabel: "OTP",
                    options: {
                        enabled: "Gaituta",
                        disabled: "Desgaituta",
                    },
                },
                ceremoniesPolicy: {
                    policyLabel: "Giltza/Zenbaketa zeremonien politika",
                    options: {
                        "automated-ceremonies": "Zeremonia automatikoak baimendu",
                        "manual-ceremonies": "Eskuzko zeremoniak",
                    },
                },
                automaticRecountPolicy: {
                    policyLabel: "Inportazioa onartu ondorengo zenbaketa automatikoa",
                    options: {
                        enabled: "Gaituta",
                        disabled: "Desgaituta",
                    },
                },
                weightedVotingPolicy: {
                    policyLabel: "Bozketa Ponderatuaren Politika",
                    options: {
                        "areas-weighted-voting": "Eremuen araberako Bozketa Ponderatua",
                        "voters-weighted-voting": "Bozkatzaileen araberako Bozketa Ponderatua",
                        "disabled-weighted-voting": "Bozketa Ponderatua Desgaituta",
                    },
                    noDelegated:
                        "Bozkatzaileen araberako Bozketa Ponderatua ezin da Boto Delegatuarekin konbinatu",
                    noDecodedBallots:
                        "Bozkatzaileen araberako Bozketa Ponderatua ezin da emaitzetan deszifratutako botoak sartzearekin konbinatu",
                },
                delegatedVotingPolicy: {
                    policyLabel: "Botoa Eskualdatzeko Politika",
                    options: {
                        enabled: "Gaituta",
                        disabled: "Desgaituta",
                    },
                },
                languageDetectionPolicy: {
                    policyLabel: "Hizkuntza detekzio politika",
                    options: {
                        "browser-detect": "Arakatzailetik detektatu",
                        "force-default": "Lehenetsia behartu",
                    },
                },
            },
            error: {
                endDate: "Amaiera data hasiera data baino beranduagokoa izan behar da",
                startDate: "Hasiera data etorkizunean egon behar da",
                noResult: "Ez dago Hauteskunde Gertaerarik oraindik",
                endDateInvalid: "Amaiera data etorkizunean egon behar da",
            },
            voters: {
                title: "Bozkatzaileak",
            },
            createElectionEventSuccess: "Hauteskunde Gertaera sortua",
            createElectionEventError: "Errorea hauteskunde gertaera sortzerakoan",
            ivr: {
                tabs: {
                    config: "Konfigurazioa",
                    blacklist: "Blokeatze-zerrenda",
                    prompts: "Ahots-mezuak",
                    emulator: "Emuladorea",
                },
                common: {
                    saveSuccess: "Behar bezala gorde da",
                    saveError: "Ezin izan da gorde",
                    deleteSuccess: "Behar bezala ezabatu da",
                    deleteError: "Ezin izan da ezabatu",
                },
                config: {
                    configuredPhone: "Konfiguratutako telefono-zenbakia",
                    infoMsg:
                        "Konfiguratu IVRaren fluxua eta haren propietateak behean. Xehetasun gehiagorako, jarri harremanetan Sequent-ekin.",
                },
                prompts: {
                    emptyMsg: "Oraindik ez da promptik sortu",
                    infoMsg:
                        "Konfiguratu IVRak erabiltzen dituen promptak. Iragarpen-promptak nahitaezkoak dira, eta sistema-promptak nahi diren hizkuntzetarako gainidatz daitezke. SSML onartzen da, hizkuntzak nahasteko ere bai.",
                    editorTitle: "Prompta",
                    editorSubtitle: "Promptaren konfigurazioa",
                },
                blacklist: {
                    columns: {
                        phone: "Telefono-zenbakia",
                        reason: "Arrazoia",
                        createdAt: "Sortze-data",
                        createdBy: "Nork sortua",
                        createdBefore: "Noiz baino lehen sortua",
                        createdAfter: "Noiz baino ondoren sortua",
                    },
                    emptyMsg: "Ez dago sarrerarik blokeatze-zerrendan",
                    infoMsg:
                        "Konfiguratu IVRaren blokeo-zerrenda. Zenbaki hauetatik datozen deiak automatikoki deskonektatuko ditu sistemak.",

                    noFilterMatch: "Ez dago emandako iragazkiekin bat datorren sarrerarik",
                    phoneRequired: "Telefono-zenbakia nahitaezkoa da",
                },
                emulator: {
                    infoMsg:
                        "Hautatu eremu bat eta nahi dituzun hauteskundeak IVR saioa probatzeko.",
                    apiStatus: {
                        unavailable: "Emuladore-sistema ez dago erabilgarri zure ingurunean",
                        loading: "Emuladore-sistema kargatzen",
                        error: "Errorea emuladore-sistema kargatzean",
                    },
                    hints: {
                        title: "Aholkuak",
                        publishRequired:
                            "Hauteskundeetan, lehiaketetan edo hautagaietan egindako edozein aldaketa lehenik argitaratu behar da erabilgarri egon dadin. Dagokion eremurako azkenik argitaratutako boto-paper estiloak soilik erabiliko dira emuladorean.",
                        eventChangesImmediate:
                            "Hauteskunde-ekitaldian egindako aldaketak, hala nola IVR konfigurazioa edo mezuen gainidazketak, berehala egongo dira erabilgarri emuladorearen saioa berrabiaraztean.",
                        credentials: 'Baliozko hautesle-IDa eta PINa "123" eta "123" dira.',
                    },
                    sendDtmf: "Bidali DTMF sarrera",
                    keypadInput: "Teklatuaren sarrera",
                    sendTimeout: "Bidali denbora-muga",
                    disconnected: "Deskonektatuta",
                    startSession: "Hasi saio berria",
                    endSession: "Amaitu saioa",
                    noStylesFound:
                        "Ez da aurkitu zure hautapenekin bat datorren argitaratutako boto-paper estilorik",
                    inputPlaceholder: "Sakatu {{keys}} (denbora-muga={{timeout}} s)",
                    inputPlaceholderOr: "edo",
                    inputPlaceholderAnyKeys:
                        "Idatzi gehienez {{maxDigits}} digitu (edozein digitu, denbora-muga={{timeout}} s)",
                    blacklistCaller: "Blokeatu deitzailea",
                    elections: "Hauteskundeak",
                    area: "Eremua",
                },
            },
            stats: {
                elegibleVoters: "Bozkatzaile Eskudunak",
                voters: "Egiazko Bozkatzaileak",
                elections: "Hauteskundeak",
                contests: "Lehiaketak",
                areas: "Eremuak",
                sentEmails: "Bidali diren emailak",
                sentSMS: "Bidali diren SMSak",
                calendar: {
                    title: "Egutegia",
                    scheduled: "Programatua",
                },
            },
            keys: {
                createNew: "Sortu Giltzen Zeremonia",
                emptyHeader: "Ez dago Giltzen Zeremoniarik oraindik.",
                statusLabel: "Egoera",
                waitingKeys: "Giltzen Sorkuntza itxoiten..",
                started: "Hasita:",
                actions: {
                    participate: "Parte hartu giltzen zeremonian",
                    view: "Ikusi giltzen zeremonia",
                },
                breadCrumbs: {
                    configure: "Konfiguratu",
                    ceremony: "Zeremonia",
                    created: "Amaituta",
                    start: "Hasi",
                    status: "Egoera",
                    download: "Deskargatu",
                    check: "Egiaztatu",
                    success: "Amaituta",
                },
                notify: {
                    participateNow:
                        "Giltzen zeremonoia batean parte hartzeko gonbidatu zaituzte. Mesedez <1>sakatu zeremoniaren Giltza Ekintzaren</1> parte hartzeko.",
                },
            },
            tabs: {
                dashboard: "Panela",
                monitoring: "Monitorizazioa",
                data: "Datuak",
                ivr: "IVR",
                localization: "Lokalizazioa",
                voters: "Bozkatzaileak",
                areas: "Eremuak",
                keys: "Giltzak",
                tally: "Zenbaketa",
                tallySheetImports: "Akten inportazioa",
                publish: "Argitaratu",
                logs: "Egunkariak",
                tasks: "Atazak",
                events: "Programatutako Gertaerak",
                notifications: "Jakinarazpenak",
                reports: "Txostenak",
                approvals: "Onespenak",
                cas: "Ziurtagiriak",
            },
            tally: {
                emptyHeader: "Ez dago Zenbaketarik oraindik.",
                title: "Hauteskunde Gertaera Zenbaketa",
                elections: "Hauteskundeak",
                electionNumber: "Hauteskunde Kopurua",
                trustees: "Fideikomisarioak",
                permissionLabels: "Baimen Etiketak",
                status: "Egoera",
                tallyType: {
                    label: "Zenbaketa Mota",
                    ELECTORAL_RESULTS: "Hauteskunde Emaitzak",
                    INITIALIZATION_REPORT: "Hasierako Emaitzak",
                },
                create: {
                    title: "Sortu Zenbaketa",
                    subtitle: "Sortu Zenbaketa berri bat Hauteskunde Gertaera honetarako",
                    createTallyButton: "Hasi Zenbaketa Zeremonia",
                    createInitializationReportButton: "Sortu Hasierako Txostena",
                    error: {
                        create: "Errorea Zenbaketa sortzerakoan",
                    },
                    success: "Zenbaketa sortua",
                },
                logs: {
                    noLogs: "Ez daude egunkaririk eskuragarri",
                },
                notify: {
                    noKeysTally:
                        "Zenbaketa Zeremonia ezin da hasi Giltzen Zeremonia arrakastaz osatu arte.",
                    noPublication:
                        "Zenbaketa Zeremonia ezin da hasi Argitaratu fitxan argitalpen bat sortu arte.",
                    participateNow:
                        "Zenbaketa zeremonoia batean parte hartzeko gonbidatu zaituzte. Mesedez <1>sakatu zeremoniaren Giltza Ekintzaren</1> parte hartzeko.",
                    startDisabled:
                        "Ezin duzu zeremoniarekin jarraitu hauteskunderik hautatu ez delako edo hauteskundeak argitaratu ez direlako.",
                    ceremonyDisabled:
                        "Ezin duzu zeremoniarekin jarraitu zenbaketa saioa konektatu ez delako edo zeremonoia hastea baimenduta ez dagoelako.",
                },
            },
            importAreas: {
                title: "Inportatu Eremuak",
                subtitle: "Inportatu eremuen datuak",
                areaParagraph:
                    "Inportatu eremuak Komaz Banandutako Balioen (CSV) formatuko kalkulu-orri fitxategi bat erabiliz.",
                importSuccess: "Eremuak Arrakastaz Inportatuak",
                importError: "Errorea Eremuak inportatzerakoan",
                upsert: "Eguneratu Eremuak",
            },
            import: {
                eetitle: "Inportatu Hauteskunde Gertaera",
                eesubtitle: "Inportatu hauteskunde gertaera datuak",
                title: "Inportatu Bozkatzaileak",
                subtitle: "Inportatu bozkatzaileen datuak",
                voters: "Bozkatzaileak",
                votersParagraph:
                    "Inportatu bozkatzaileak Komaz Banandutako Balioen (CSV) formatuko kalkulu-orri fitxategia erabiliz. Deskargatu inportazio CSV fitxategiaren adibidea hemen.",
                electionEventParagraph: "Inportatu Hauteskunde Gertaerak JSON fitxategia erabiliz.",
                elections: "Hauteskundeak",
                areas: "Eremuak",
                sha: "Osotasun Egiaztapena (SHA-256)",
                cancel: "Ezeztatu",
                import: "Inportatu",
                fileUploadSuccess:
                    "Fitxategia zerbitzarira igo da - baina ez da inportatu oraindik",
                fileUploadError: "Errorea fitxategia igotzean",
                importVotersSuccess: "Bozkatzaileen Inportazioa Arrakastaz Programatua",
                importVotersError: "Errorea Bozkatzaileak inportatzerakoan",
                importElectionEventSuccess: "Hauteskunde gertaera arrakastaz inportatua",
                importElectionEventError: "Errorea hauteskunde gertaera inportatzerakoan",
                shaDialog: {
                    ok: "Bai, Inportatu Osotasun Egiaztapenik gabe",
                    cancel: "Itzuli",
                    title: "Inportatu Osotasun Egiaztapenik gabe?",
                    description:
                        "Ez duzu Osotasun Egiaztapena (SHA-256) eremua bete. Mesedez, berretsi fitxategi egokia inportatzen ari zarela eta inportatu nahi duzula.",
                },
                passwordDialog: {
                    title: "Deszifratze Pasahitza",
                    description: "Sartu pasahitza fitxategia deszifratzeko",
                    label: "Pasahitza",
                    copyPassword: "Kopiatu Pasahitza",
                    ok: "Ados",
                },
            },
            export: {
                title: "Esportatu Hauteskunde Gertaera",
                subtitle:
                    "Esportazioa eragiketa luzea izan daiteke. Ziur zaude erregistroak esportatu nahi dituzula?",
                encryptWithPassword: "Zifratu Pasahitzarekin",
                passwordForcedNote:
                    "Artxiboa pasahitzarekin babestuko da nolanahi ere: txostenak, eskaerak eta iragarki-taulako datuak beti zifratzen dira. Markatu laukia hautesleen eremu sekretu deszifratuak ere sartzeko.",
                includeVoters: "Sartu Bozkatzaileak",
                activityLogs: "Jarduera Egunkariak",
                bulletinBoard: "Iragarki Taula",
                publications: "Argitalpenak",
                s3Files: "S3 Fitxategiak",
                scheduledEvents: "Programatutako Gertaerak",
                exportSuccess: "Hauteskunde Gertaera arrakastaz esportatua",
                exportError: "Errorea Hauteskunde Gertaera esportatzerakoan",
                passwordTitle: "Pasahitza",
                passwordDescription: "Fitxategia deszifratzeko pasahitza:",
                copiedSuccess: "Pasahitza arbelera kopiatu da",
                copiedError: "Errorea pasahitza kopiatzerakoan",
                reports: "Txostenak",
                applications: "Aplikazioak",
                tally: "Zenbaketa",
                certificates: "Ziurtagiriak",
            },
            taskNotification:
                "{{action}} hasi da. Bere egoera Ataza Exekuzio taulan ikus dezakezu.",
        },
        electionScreen: {
            common: {
                title: "Hauteskundea",
                subtitle: "Hauteskunde konfigurazioa.",
                fileLoaded: "Fitxategia kargatua",
                noPermission: "Ez duzu hauteskunde hau atzitzeko baimenik.",
            },
            edit: {
                general: "Orokorra",
                dates: "Datak",
                votingPeriod: "Bozketa Aldia",
                language: "Hizkuntza",
                allowed: "Baimendutako Bozketa Kanalak",
                default: "Lehenetsia",
                defaultLang: "Lehenetsitako hizkuntza",
                receipts: "Jaso-agiriak",
                image: "Irudia",
                advanced: "Konfigurazio Aurreratua",
                numAllowedVotes: "Baimendutako boto kopurua",
                reorder: "Berrantolatu lehiaketak",
                castVoteConfirm: "Boto Berrespena Modal",
                gracePeriodPolicy: "Grazia Aldia",
                allowTallyPolicy: "Baimendu Zenbaketa",
                permissionLabel: "Baimen Etiketa",
                custom_filters: "Iragazki pertsonalizatuak",
            },
            field: {
                name: "Izena",
                language: "Hizkuntza",
                votingChannels: "Bozketa Kanalak",
                startDateTime: "Hasiera Data eta Ordua",
                endDateTime: "Amaiera Data eta Ordua",
                startDateTimeWithTimezone: "Hasiera Data eta Ordua ({{timezone}})",
                endDateTimeWithTimezone: "Amaiera Data eta Ordua ({{timezone}})",
                scheduledOpening: "Programatutako Irekiera",
                scheduledClosing: "Programatutako Itxiera",
                alias: "Ezizena",
                description: "Deskribapena",
                securityConfirmationHtml: "Segurtasun-berrespena HTML",
                ivrPrompt: "IVR mezua",
                externalId: "Kanpoko IDa",
            },
            securityConfirmationPolicy: {
                label: "Segurtasun-berrespeneko kontrol-laukiaren politika",
                none: "Bat ere ez",
                mandatory: "Derrigorrezkoa",
            },
            error: {
                fileError: "Errorea fitxategia igotzean",
                fileLoaded: "Fitxategia kargatua",
                endDate: "Amaiera data hasiera data baino beranduagokoa izan behar da",
                startDate: "Hasiera data etorkizunean egon behar da",
                endDateInvalid: "Amaiera data etorkizunean egon behar da",
            },
            createElectionEventSuccess: "Hauteskunde Gertaera sortua",
            createElectionEventError: "Errorea hauteskunde gertaera sortzerakoan",
            tabs: {
                dashboard: "Panela",
                monitoring: "Monitorizazioa",
                data: "Datuak",
                voters: "Bozkatzaileak",
                publish: "Argitaratu",
                logs: "Egunkariak",
                approvals: "Onespenak",
                tallySheets: "Kontaketa orriak",
            },
            gracePeriodPolicy: {
                "label": "Grazia Aldi Politika",
                "no-grace-period": "Graziazko aldirik ez",
                "grace-period-without-alert": "Graziazko aldia alertarik gabe",
                "gracePeriodSecs": "Graziazko aldia segundotan",
            },
            allowTallyPolicy: {
                "allowed": "Baimenduta",
                "disallowed": "Baimendu gabe",
                "requires-voting-period-end": "Bozketa Aldi Amaiera Behar du",
            },
            initializeReportPolicy: {
                "label": "Hasierako Txosten Politika",
                "not-required": "Ez da Beharrezkoa",
                "required": "Beharrezkoa",
            },
            castVoteGoldLevelPolicy: {
                label: "Urre maila Autentifikazio Politika",
                options: {
                    "gold-level": "Urre maila Autentifikazioa",
                    "no-gold-level": "Ez da Urre maila Autentifikaziorik",
                },
            },
            slates: {
                title: "Hautagai-zerrendak",
                configuration: "Hautagai-zerrenden konfigurazioa (JSON)",
                helper: "Izendun hautagai-zerrendak eta bakoitzak lehia bakoitzean aurkezten dituen hautagaiak. Utzi hutsik hautagai-zerrendarik gabeko hauteskunde baterako.",
                loading:
                    "Hauteskundearen lehiak eta hautagaiak kargatzen ari dira oraindik. Saiatu berriro une batean.",
                mobileCandidateLists: {
                    label: "Hautagaien zerrendak mugikorrean",
                    helper: "Nola agertzen den hasieran hautagai-zerrenda bakoitzaren hautagaien zerrenda mugikorrean. Hautesleak beti ireki edo itxi dezake.",
                    options: {
                        collapsed: "Tolestuta",
                        expanded: "Zabalduta",
                    },
                },
            },
            startScreenTitlePolicy: {
                label: "Hasierako pantailaren titulu politika",
                options: {
                    "election": "Hauteskundearen titulua",
                    "election-event": "Hauteskunde-ekitaldiaren titulua",
                },
            },
            consolidatedReportPolicy: {
                label: "Txosten bateratuaren politika",
                options: {
                    "generate": "Sortu",
                    "do-not-generate": "Ez sortu",
                },
            },
            declineToVotePolicy: {
                label: "Bozkatzeari uko egiteko politika",
                options: {
                    enabled: "Gaituta",
                    disabled: "Desgaituta",
                },
            },
            blankBallotsPolicy: {
                label: "Boto-txartel zurien politika",
                options: {
                    enabled: "Gaituta",
                    disabled: "Desgaituta",
                },
            },
            votingScreenBackPolicy: {
                label: "Bozketa-pantailako atzera botoiaren politika",
                options: {
                    "election-selection-screen": "Joan hauteskundeak hautatzeko pantailara",
                    "start-screen": "Joan hauteskundearen hasierako pantailara",
                },
            },
        },
        tenantScreen: {
            common: {
                title: "Maizterrak",
            },
            new: {
                subtitle: "Sortu maizter berria",
            },
            createSuccess: "Maizterra sortua",
            createError: "Errorea maizterra sortzerakoan",
        },
        usersAndRolesScreen: {
            noPermissions: "Ez duzu erabiltzaileak edo rolak atzitzeko baimenik.",
            common: {
                title: "Erabiltzaileak eta Rolak",
                subtitle: "Konfigurazio orokorra",
                mobileNumber: "Mugikorra",
            },
            editPassword: {
                passwordPolicyViolation:
                    "Pasahitzak ez du betetzen hauteskunde-gertaera honetako Pasahitz-politika. Berrikusi politika Hauteskunde-gertaeraren datuak atalean eta idatzi baliozko pasahitz bat.",
                passwordPolicyRules: {
                    minimumLength: "Pasahitzaren gutxieneko luzera {{count}} da.",
                    maximumLength: "Pasahitzaren gehieneko luzera {{count}} da.",
                    uppercase: "Beharrezko letra larriak: {{count}}.",
                    lowercase: "Beharrezko letra xeheak: {{count}}.",
                    digits: "Beharrezko digituak: {{count}}.",
                    specialCharacters: "Beharrezko karaktere bereziak: {{count}}.",
                },
                label: "Aldatu pasahitza",
                temporatyLabel: "Behin-behinekoa",
                temporatyInfo:
                    "Gaituta badago, erabiltzaileak pasahitza aldatu beharko du hurrengo sarreran",
            },
            users: {
                title: "Erabiltzaileak",
                subtitle: "Ikusi eta editatu erabiltzaile datuak",
                review: {
                    title: "Aldaketak berrikusi",
                    subtitle: "Berretsi eguneratze hauek bidali aurretik.",
                    confirm: "Aldaketak berretsi",
                    noChanges: "Ez dago berrikusteko aldaketarik",
                    field: "Eremua",
                    currentValue: "Uneko balioa",
                    newValue: "Balio berria",
                },
                edit: {
                    title: "Erabiltzaile Datuak",
                    subtitle: "Ikusi eta editatu erabiltzailea",
                },
                create: {
                    title: "Erabiltzailea",
                    subtitle: "Sortu erabiltzailea",
                },
                fields: {
                    "has_voted": "Bozkatu du",
                    "support_materials_viewed": "Support Materials Viewed",
                    "vote-weight": "Botoaren pisua",
                    "voted-channel": "Boto-kanala",
                    "disable-comment": "Desgaitzeko iruzkina",
                    "username": "Erabiltzaile izena",
                    "first_name": "Izena",
                    "last_name": "Abizena",
                    "email": "Emaila",
                    "enabled": "Gaituta",
                    "emailVerified": "Emaila Egiaztatua",
                    "groups": "Taldeak",
                    "attributes": "Atributuak",
                    "area": "Eremua",
                    "password": "Pasahitza",
                    "repeatPassword": "Errepikatu Pasahitza",
                    "savePassword": "Gorde Pasahitza",
                    "passwordMismatch": "Pasahitzek bat egin behar dute",
                    "passwordLengthValidate": "Pasahitzak gutxienez 8 karaktere izan behar ditu",
                    "passwordUppercaseValidate": "Pasahitzak gutxienez maiuskula bat izan behar du",
                    "passwordLowercaseValidate": "Pasahitzak gutxienez minuskula bat izan behar du",
                    "passwordDigitValidate": "Pasahitzak gutxienez zenbaki bat izan behar du",
                    "passwordSpecialCharValidate":
                        "Pasahitzak gutxienez karaktere berezi bat izan behar du",
                    "trustee": "Fideikomisario gisa jardun",
                    "permissionLabel": "Baimen Etiketa",
                    "authorized-election-ids": "Hauteskundeak",
                },
                delete: {
                    body: "Ziur zaude erabiltzaile hau ezabatu nahi duzula?",
                    bulkBody: "Ziur zaude hautatutako erabiltzaileak ezabatu nahi dituzula?",
                    bulkBodySelected: "Delete the {{count}} selected users? This cannot be undone.",
                    bulkBodyChoose:
                        "{{count}} users are selected. You can instead delete every user matching the current filters, which may be more. This cannot be undone.",
                    okSelected: "Delete {{count}} selected",
                    okAllMatching: "Delete all matching",
                },
                notifications: {
                    exportError: "Errorea erabiltzaileak esportatzerakoan",
                    deleteError: "Errorea erabiltzailea ezabatzerakoan",
                    deleteSuccess: "Erabiltzailea ezabatua",
                    multipleDeleteSuccess: "Erabiltzaileak ezabatuak",
                },
            },
            voters: {
                voterInformationLetter: {
                    label: "Hauteslearen informazio-gutuna",
                    generate: "Sortu",
                    confirmation:
                        "Hautesle honentzako informazio-gutuna sortu? Pasahitz berri bat esleituko da eta PDF zifratu batean sartuko da.",
                    generationStarted: "Informazio-gutuna sortzen hasi da",
                    generationError: "Ezin izan da informazio-gutuna sortu",
                    policyNotConfigured:
                        "Pasahitz-politika ez dago konfiguratuta. Konfiguratu Hauteskunde-gertaeraren datuak atalean gutuna sortu aurretik.",
                    policyMinimumLengthMissing:
                        "Pasahitz-politikak gutxieneko luzera izan behar du gutuna sortu aurretik.",
                    policyCharacterClassMissing:
                        "Pasahitz-politikak gutxienez karaktere-klase bat izan behar du gutuna sortu aurretik.",
                },
                title: "Bozkatzaileak",
                subtitle: "Ikusi eta editatu bozkatzaile datuak",
                secretAttribute: {
                    storedPlaceholder: "Gordetako balio zifratua",
                    reveal: "Erakutsi",
                    hide: "Ezkutatu",
                    revealError: "Ezin izan da bozkatzailearen eremu zifratua erakutsi",
                    includeInExport: "Sartu deszifratutako bozkatzailearen eremu sekretuak",
                    exportWarning:
                        "Esportazio sentikorra: deskargatutako CSVak eremu hauek testu arruntean izango ditu.",
                    clear: "Garbitu",
                    add: "Gehitu balioa",
                    remove: "Kendu balioa",
                },
                review: {
                    title: "Aldaketak berrikusi",
                    subtitle: "Berretsi eguneratze hauek bidali aurretik.",
                    confirm: "Aldaketak berretsi",
                    noChanges: "Ez dago berrikusteko aldaketarik",
                    field: "Eremua",
                    currentValue: "Uneko balioa",
                    newValue: "Balio berria",
                },
                logs: {
                    label: "Erabiltzailearen Egunkariak",
                },
                create: {
                    title: "Bozkatzailea",
                    subtitle: "Sortu Bozkatzailea",
                },
                manualVerification: {
                    label: "Eskuz Egiaztatu",
                    verify: "Eskuz egiaztatu bozkatzaile hau",
                    body: "Eskuz egiaztatu bozkatzaile hau. Bozkatzaileari online KYC saltatzeko sartzen ahalbidetzen dion QR Kode esteka bat duen PDF bat lortuko duzu.",
                    noEmailOrPhone:
                        "Bozkatzaile hau ezin da eskuz egiaztatu email helbiderik edo telefono zenbakirik ez duelako.",
                },
                emptyHeader: "Ez dago bozkatzailerik oraindik.",
                askCreate: "Bat sortu nahi duzu?",
                errors: {
                    editError: "Errorea bozkatzailea editatzerakoan",
                    editErrorReason: "Errorea bozkatzailea editatzerakoan: {{reason}}",
                    editSuccess: "Bozkatzailea editatua",
                    createError: "Errorea bozkatzailea sortzerakoan",
                    createErrorReason: "Errorea bozkatzailea sortzerakoan: {{reason}}",
                    createSuccess: "Bozkatzailea sortua",
                    attribute: {
                        invalidNamed: '"{{field}}" baztertu da: {{constraint}}',
                        fieldsToCorrect: "Eremu batzuk zuzendu behar dira gorde aurretik",
                        hintBetween: "{{min}} eta {{max}} karaktere artean",
                        hintMin: "Gutxienez {{min}} karaktere",
                        hintMax: "Gehienez {{max}} karaktere",
                        andMore: "eta beste {{count}}",
                        invalidLength:
                            '"{{field}}" eremuak {{min}} eta {{max}} karaktere artean izan behar ditu',
                        tooShort: '"{{field}}" eremuak gutxienez {{min}} karaktere izan behar ditu',
                        tooLong: '"{{field}}" eremuak gehienez {{max}} karaktere izan behar ditu',
                        required: '"{{field}}" nahitaezkoa da',
                        invalidEmail: '"{{field}}" baliozko helbide elektronikoa izan behar da',
                        invalidFormat: '"{{field}}" ez dauka espero den formatua',
                        invalid: '"{{field}}" balio baliogabea dauka',
                    },
                    createPasswordError:
                        "Bozkatzailea sortua, baina ezin izan da bere pasahitza ezarri",
                    createPasswordErrorReason:
                        "Bozkatzailea sortua, baina ezin izan da bere pasahitza ezarri: {{reason}}",
                },
                delete: {
                    body: "Ziur zaude bozkatzaile hau ezabatu nahi duzula?",
                    bulkBody: "Ziur zaude hautatutako bozkatzaileak ezabatu nahi dituzula?",
                    bulkBodySelected:
                        "Delete the {{count}} selected voters? This cannot be undone.",
                    bulkBodyChoose:
                        "{{count}} voters are selected. You can instead delete every voter matching the current filters, which may be more. This cannot be undone.",
                    okSelected: "Delete {{count}} selected",
                    okAllMatching: "Delete all matching",
                },
                notifications: {
                    exportError: "Errorea bozkatzaileak esportatzerakoan",
                    deleteError: "Errorea bozkatzailea ezabatzerakoan",
                    deleteSuccess: "Bozkatzailea ezabatua",
                    multipleDeleteSuccess: "Bozkatzaileak ezabatuak",
                    manualVerificationError: "Errorea bozkatzailea eskuz egiaztatzerakoan",
                    manualVerificationSuccess:
                        "Arrakastaz egiaztatu da eskuz bozkatzailea, PDFa deskargatzen..",
                },
            },
            roles: {
                title: "Rolak",
                edit: {
                    title: "Rol Datuak",
                    subtitle: "Ikusi eta editatu rola",
                },
                create: {
                    title: "Rola",
                    subtitle: "Sortu rola",
                },
                errors: {
                    createError: "Errorea rola sortzerakoan",
                    createSuccess: "Rola sortua",
                },
                fields: {
                    name: "Izena",
                },
                delete: {
                    body: "Ziur zaude rol hau ezabatu nahi duzula?",
                },
                notifications: {
                    deleteError: "Errorea rola ezabatzerakoan",
                    deleteSuccess: "Rola ezabatua",
                    permissionEditError: "Errorea baimena editatzerakoan",
                    permissionEditSuccess: "Baimena editatua",
                },
            },
            permissions: {
                "voter-information-letter": "Hauteslearen informazio-gutuna sortu",
                "admin-user": "Admin Erabiltzailea",
                "admin-dashboard-view": "Admin Panela Ikusi",
                "monitoring-view": "Monitorizazio-panelak ikusi",
                "monitoring-configure": "Monitorizazio-panelak konfiguratu",
                "election-event-signatures-tab": "Hauteskunde Ekitaldiaren Sinadurak Fitxa",
                "signing-rules-read": "Sinadurak: ikusi ekintza babestuak",
                "signing-rules-write": "Sinadurak: editatu ekintza babestuak",
                "signing-certificates-read": "Sinadurak: ikusi ziurtagiriak",
                "signing-issuers-write": "Sinadurak: inportatu eta kendu jaulkitzaile fidagarriak",
                "signing-checks-write": "Sinadurak: editatu ziurtagirien egiaztapenak",
                "signing-certificates-register": "Sinadurak: erregistratu ziurtagiriak",
                "signing-certificates-revoke": "Sinadurak: baliogabetu ziurtagiriak",
                "signing-requests-read": "Sinadurak: ikusi eskaerak",
                "signing-requests-cancel": "Sinadurak: ezeztatu eskaerak",
                "signing-requests-export": "Sinadurak: esportatu eskaerak",
                "sign-initialize-voting": "Sinatu: bozketa hasieratu",
                "sign-open-voting": "Sinatu: bozketa ireki",
                "sign-close-voting": "Sinatu: bozketa itxi",
                "sign-generate-election-returns": "Sinatu: hauteskunde-aktak sortu",
                "sign-generate-reports": "Sinatu: beste hauteskunde-txosten batzuk sortu",
                "sign-transmit-results": "Sinatu: emaitzak transmititu",
                "sign-approve-voter": "Sinatu: hautesle bat eskuz onartu",
                "sign-approve-configuration": "Sinatu: konfigurazio-bertsio bat onartu",
                "sign-key-ceremony": "Sinatu: giltza-zati bat berretsi",
                "sign-tally-key": "Sinatu: giltza-zati bat eman",
                "application-export": "Aplikazio Esportazioa",
                "application-import": "Aplikazio Inportazioa",
                "tenant-create": "Sortu Maizterra",
                "tenant-read": "Irakurri Maizterra",
                "tenant-write": "Editatu Maizterra",
                "tenant-delete": "Ezabatu Maizterra",
                "election-event-create": "Sortu Hauteskunde Gertaera",
                "election-event-read": "Irakurri Hauteskunde Gertaera",
                "election-event-write": "Editatu Hauteskunde Gertaera",
                "keycloak-realm-attributes-read": "Read Keycloak realm attributes",
                "keycloak-realm-attributes-write": "Edit Keycloak realm attributes",
                "election-event-delete": "Ezabatu Hauteskunde Gertaera",
                "voter-create": "Sortu Bozkatzailea",
                "voter-read": "Irakurri Bozkatzailea",
                "voter-write": "Editatu Bozkatzailea",
                "voter-secret-attribute-read": "Erakutsi Bozkatzailearen Eremu Sekretuak",
                "voter-secret-attribute-write": "Editatu Bozkatzailearen Eremu Sekretuak",
                "user-create": "Sortu Erabiltzailea",
                "user-read": "Irakurri Erabiltzailea",
                "user-write": "Editatu Erabiltzailea",
                "user-permission-create": "Sortu Erabiltzaile Baimena",
                "user-permission-read": "Irakurri Erabiltzaile Baimena",
                "user-permission-write": "Editatu Erabiltzaile Baimena",
                "role-create": "Sortu Rola",
                "role-read": "Irakurri Rola",
                "role-write": "Editatu Rola",
                "role-assign": "Esleitu Rola",
                "communication-template-create": "Sortu Komunikazio Txantiloia",
                "communication-template-read": "Irakurri Komunikazio Txantiloia",
                "communication-template-write": "Editatu Komunikazio Txantiloia",
                "notification-read": "Irakurri Jakinarazpena",
                "notification-write": "Editatu Jakinarazpena",
                "notification-send": "Bidali Jakinarazpena",
                "area-read": "Irakurri Eremua",
                "area-write": "Editatu Eremua",
                "election-state-write": "Editatu Hauteskunde Egoera",
                "election-type-create": "Sortu Hauteskunde Mota",
                "election-type-read": "Irakurri Hauteskunde Mota",
                "election-type-write": "Editatu Hauteskunde Mota",
                "voting-channel-read": "Irakurri Bozketa Kanala",
                "voting-channel-write": "Editatu Bozketa Kanala",
                "trustee-create": "Sortu Fideikomisarioa",
                "trustee-read": "Irakurri Fideikomisarioa",
                "trustee-write": "Editatu Fideikomisarioa",
                "tally-read": "Irakurri Zenbaketa",
                "tally-start": "Hasi Zenbaketa",
                "tally-write": "Editatu Zenbaketa",
                "tally-results-read": "Irakurri Zenbaketa Emaitzak",
                "publish-read": "Irakurri Argitalpena",
                "publish-write": "Editatu Argitalpena",
                "publish-results-read": "Irakurri Emaitzen Argitalpena",
                "publish-results-write": "Editatu Emaitzen Argitalpena",
                "logs-read": "Irakurri Egunkariak",
                "tasks-read": "Irakurri Ataza Exekuzioa",
                "keys-read": "Irakurri Giltzak",
                "document-upload": "Igo Dokumentuak",
                "document-download": "Deskargatu Dokumentuak",
                "document-password-read": "Irakurri dokumentuen pasahitzak",
                "tally-sheet-create": "Sortu Zenbaketa Orria",
                "tally-sheet-import-create": "Sortu zenbaketa orrien inportazioa",
                "tally-sheet-import-review": "Berrikusi zenbaketa orrien inportazioa",
                "tally-sheet-import-view": "Ikusi zenbaketa orrien inportazioa",
                "tally-recount-execute": "Exekutatu emaitzen birzenbaketa",
                "trustee-ceremony": "Fideikomisario Zeremonia",
                "tally-sheet-review": "Zenbaketa-orria berrikusi",
                "tally-sheet-view": "Ikusi Zenbaketa Orria",
                "admin-ceremony": "Admin Zeremonia",
                "tally-sheet-delete": "Ezabatu Zenbaketa Orria",
                "cast-vote-read": "Irakurri Emandako Botoak",
                "document-read": "Irakurri Dokumentuak",
                "document-write": "Editatu Dokumentuak",
                "support-material-read": "Irakurri Laguntza Materialak",
                "support-material-write": "Editatu Laguntza Materialak",
                "miru-create": "Miru Sortu",
                "miru-download": "Miru Deskargatu",
                "miru-send": "Miru Bidali",
                "miru-sign": "Miru Sinatu",
                "contest-write": "Editatu Lehiaketa",
                "contest-read": "Irakurri Lehiaketa",
                "candidate-write": "Editatu Hautagaia",
                "candidate-read": "Irakurri Hautagaia",
                "permission-label-write": "Editatu Baimen Etiketa",
                "scheduled-event-write": "Editatu Programatutako Gertaerak",
                "contest-create": "Sortu Lehiaketa",
                "contest-delete": "Ezabatu Lehiaketa",
                "candidate-create": "Sortu Hautagaia",
                "candidate-delete": "Ezabatu Hautagaia",
                "election-create": "Sortu Hauteskundea",
                "election-read": "Irakurri Hauteskundea",
                "election-write": "Editatu Hauteskundea",
                "election-delete": "Ezabatu Hauteskundea",
                "election-event-archive": "Artxibatu Hauteskunde Gertaera",
                "election-data-tab": "Ikusi Hauteskunde Datuak",
                "election-event-areas-tab": "Ikusi Hauteskunde Gertaera Eremuak",
                "election-event-data-tab": "Ikusi Hauteskunde Gertaera Datuak",
                "election-event-keys-tab": "Ikusi Hauteskunde Gertaera Giltzak",
                "election-event-logs-tab": "Ikusi Hauteskunde Gertaera Egunkariak",
                "election-event-publish-tab": "Ikusi Hauteskunde Gertaera Argitalpena",
                "election-event-reports-tab": "Ikusi Hauteskunde Gertaera Txostenak",
                "election-event-scheduled-tab": "Ikusi Hauteskunde Gertaera Programatua",
                "election-event-tally-tab": "Ikusi Hauteskunde Gertaera Zenbaketa",
                "election-event-tasks-tab": "Ikusi Hauteskunde Gertaera Atazak",
                "election-event-voters-tab": "Ikusi Hauteskunde Gertaera Bozkatzaileak",
                "election-publish-tab": "Ikusi Hauteskunde Argitalpena",
                "election-voters-tab": "Ikusi Hauteskunde Bozkatzaileak",
                "report-write": "Editatu Txostenak",
                "report-read": "Irakurri Txostenak",
                "users-menu": "Ikusi erabiltzaileak eta rolak",
                "settings-menu": "Ikusi ezarpenak",
                "templates-menu": "Ikusi txantiloiak",
                "settings-election-types-tab": "Ikusi hauteskunde moten ezarpenak",
                "settings-voting-channels-tab": "Ikusi bozketa kanalen ezarpenak",
                "settings-templates-tab": "Ikusi txantiloien ezarpenak",
                "settings-languages-tab": "Ikusi hizkuntzen ezarpenak",
                "settings-localization-tab": "Ikusi lokalizazio ezarpenak",
                "settings-look-feel-tab": "Ikusi itxura eta sentimendu ezarpenak",
                "settings-trustees-tab": "Ikusi fideikomisarioen ezarpenak",
                "settings-countries-tab": "Ikusi herrialdeen ezarpenak",
                "voter-import": "Inportatu Bozkatzailea",
                "ee-voters-columns": "Ikusi Hauteskunde Gertaera Bozkatzaileen Zutabeak",
                "voter-manually-verify": "Eskuz Egiaztatu Bozkatzailea",
                "ee-voters-logs": "Ikusi Hauteskunde Gertaera Bozkatzaileen Egunkariak",
                "voter-export": "Esportatu Bozkatzailea",
                "ee-voters-filters": "Ikusi Hauteskunde Gertaera Bozkatzaileen Iragazkiak",
                "voter-delete": "Ezabatu Bozkatzailea",
                "voter-change-password": "Aldatu Bozkatzaile Pasahitza",
                "election-event-localization-selector":
                    "Hauteskunde Gertaera Lokalizazio Hautatzailea",
                "localization-create": "Sortu Lokalizazioa",
                "localization-read": "Irakurri Lokalizazioa",
                "localization-write": "Editatu Lokalizazioa",
                "localization-delete": "Ezabatu Lokalizazioa",
                "area-create": "Sortu Eremua",
                "area-delete": "Ezabatu Eremua",
                "area-export": "Esportatu Eremua",
                "area-import": "Inportatu Eremua",
                "area-upsert": "Eguneratu Eremua",
                "election-event-areas-columns": "Hauteskunde Gertaera Eremuen Zutabeak",
                "election-event-areas-filters": "Hauteskunde Gertaera Eremuen Iragazkiak",
                "election-event-tasks-back-button": "Itzuli Hauteskunde Gertaera Atazetara",
                "election-event-tasks-columns": "Hauteskunde Gertaera Atazen Zutabeak",
                "election-event-tasks-filters": "Hauteskunde Gertaera Atazen Iragazkiak",
                "task-export": "Esportatu Atazak",
                "application-read": "Irakurri Aplikazioa",
                "application-write": "Editatu Aplikazioa",
                "approval-matrix-write": "Editatu Onarpen Matrizea",
                "logs-export": "Esportatu Egunkariak",
                "election-event-logs-columns": "Hauteskunde Gertaera Egunkarien Zutabeak",
                "election-events-logs-filters": "Hauteskunde Gertaera Egunkarien Iragazkiak",
                "election-event-scheduled-event-columns":
                    "Hauteskunde Gertaera Programatutako Gertaeren Zutabeak",
                "scheduled-event-create": "Sortu Programatutako Gertaera",
                "scheduled-event-delete": "Ezabatu Programatutako Gertaera",
                "election-event-reports-columns": "Hauteskunde Gertaera Txostenen Zutabeak",
                "report-create": "Sortu Txostena",
                "report-delete": "Ezabatu Txostena",
                "report-generate": "Sortu Txostena",
                "report-preview": "Aurreikusi Txostena",
                "monitor-authenticated-voters": "Monitorizatu Autentifikatutako Bozkatzaileak",
                "monitor-all-approve-disapprove-voters":
                    "Irakurri Monitorizazio Onartu Ezarri Bozkatzaileak",
                "monitor-automatic-approve-disapprove-voters":
                    "Irakurri Monitorizazio Automatiko Onartu Ezarri Bozkatzaileak",
                "monitor-manually-approve-disapprove-voters":
                    "Irakurri Monitorizazio Eskuz Onartu Ezarri Bozkatzaileak",
                "monitor-enrolled-overseas-voters":
                    "Irakurri Monitorizazio Matrikulatutako Atzerriko Bozkatzaileak",
                "monitor-posts-already-closed-voting":
                    "Irakurri Monitorizazio Bozketa Itxi duten Postuak",
                "monitor-posts-already-generated-election-results":
                    "Irakurri Monitorizazio Hauteskunde Emaitzak Sortu dituzten Postuak",
                "monitor-posts-already-opened-voting":
                    "Irakurri Monitorizazio Bozketa Ireki duten Postuak",
                "monitor-posts-already-started-counting-votes":
                    "Irakurri Monitorizazio Botoak Zenbatzen Hasi diren Postuak",
                "monitor-posts-initialized-the-system":
                    "Irakurri Monitorizazio Sistema Hasieratu duten Postuak",
                "monitor-posts-started-voting": "Irakurri Monitorizazio Bozketa Hasi duten Postuak",
                "monitor-posts-transmitted-results":
                    "Irakurri Monitorizazio Emaitzak Transmititu dituzten Postuak",
                "monitor-voters-voted-test-election":
                    "Irakurri Monitorizazio Proba Hauteskunden Bozkatu duten Bozkatzaileak",
                "monitor-voters-who-voted": "Irakurri Monitorizazio Bozkatu duten Bozkatzaileak",
                "election-event-publish-preview": "Aurreikusi Hauteskunde Gertaera Argitalpena",
                "election-event-publish-back-button": "Itzuli Hauteskunde Gertaera Argitalpenara",
                "election-event-publish-columns": "Hauteskunde Gertaera Argitalpenaren Zutabeak",
                "election-event-publish-filters": "Hauteskunde Gertaera Argitalpenaren Iragazkiak",
                "publish-create": "Sortu Argitalpena",
                "publish-regenerate": "Bersortu Argitalpena",
                "publish-export": "Esportatu Argitalpena",
                "publish-start-voting": "Hasi Bozketa",
                "publish-pause-voting": "Gelditu Bozketa",
                "publish-stop-voting": "Gelditu Bozketa",
                "publish-changes": "Argitaratu Aldaketak",
                "election-event-publish-view": "Hauteskunde Gertaera Argitalpen Ikusi",
                "election-event-keys-columns": "Hauteskunde Gertaera Giltzen Zutabeak",
                "create-ceremony": "Sortu Zeremonia",
                "export-ceremony": "Esportatu Zeremonia",
                "election-event-tally-columns": "Hauteskunde Gertaera Zenbaketa Zutabeak",
                "election-event-tally-back-button": "Itzuli Hauteskunde Gertaera Zenbaketara",
                "transmition-ceremony": "Transmisio Zeremonia",
                "admin-ip-address-view": "Ikusi IP Helbidea",
                "election-approvals-tab": "Ikusi Hauteskunde Onespenak",
                "election-event-approvals-tab": "Ikusi Hauteskunde Gertaera Onespenak",
                "election-ip-address-view": "Ikusi Hauteskunde IP Helbidea",
                "election-dashboard-tab": "Ikusi Hauteskunde Panela",
                "trustees-export": "Esportatu Fideikomisarioak",
                "user-import": "Inportatu Erabiltzaileak",
                "voter-voted-edit": "Editatu bozkatu duten bozkatzaileak",
                "voter-email-tlf-edit": "Editatu bozkatzaileen email/telefono eremuak",
                "cloudflare-write": "Editatu Herrialde Blokeo Arauak Cloudflare-n",
                "transmission-report-generate": "Sortu Transmisio Txostena",
                "google-meet-link": "Google Meet Esteka Sortu",
                "service-account": "Zerbitzu-kontua",
                "datafix-account": "Datuak zuzentzeko kontua",
                "gold": "Urrea",
                "silver": "Zilarra",
                "election-event-ivr-tab": "Ikusi hauteskunde-gertaeraren IVRa",
                "election-event-cas-tab": "Ikusi hauteskunde-gertaeraren CASa",
                "ca-read": "Irakurri ziurtagiri-agintaritzak",
                "ca-write": "Editatu ziurtagiri-agintaritzak",
                "generate-preview": "Sortu aurrebista",
                "preview-read": "Irakurri aurrebista",
                "tally-resolution-submit": "Bidali zenbaketa-ebazpena",
                "phone-blacklist-read": "Irakurri telefonoen zerrenda beltza",
                "phone-blacklist-create": "Sortu telefonoen zerrenda beltzeko sarrerak",
                "phone-blacklist-update": "Editatu telefonoen zerrenda beltzeko sarrerak",
                "phone-blacklist-delete": "Ezabatu telefonoen zerrenda beltzeko sarrerak",
                "election-event-voter-list-reconciliation":
                    "Berradiskidetu hauteskunde-gertaeraren bozkatzaile-zerrenda",
                "messaging-account-read": "Mezularitza-kontuak ikusi",
                "messaging-account-write": "Mezularitza-kontuak kudeatu",
                "messaging-config-write": "Hauteskunde-gertaeraren mezularitza konfiguratu",
            },
        },
        generalSettingsScreen: {
            body: "Gaitu hizkuntzak sisteman. Hemen gaitutako hizkuntzak soilik egongo dira eskuragarri hauteskunde gertaeretan.",
        },
        eventsScreen: {
            title: "Programatutako Gertaerak",
            subtitle:
                "Bozketa aldiaren hasiera edo amaiera bezalako gertaeren exekuzio automatikoaren konfigurazioa kudeatzen du.",
            messages: {
                createSuccess: "Programatutako Gertaera arrakastaz sortua",
                createError: "Errorea Programatutako Gertaera sortzerakoan",
                editSuccess: "Programatutako Gertaera arrakastaz editatua",
                editError: "Errorea Programatutako Gertaera editatzerakoan",
                onlineWithEarlyVoting:
                    "Hasierako programazio batek ezin ditu aldi berean lineako botoa eta aurretiazko botoa ireki: aurretiazko botoak lineako botoa baino lehen hasi behar du.",
            },
            eventType: {
                label: "Mota",
                ALLOW_INIT_REPORT: "Baimendu Hasierako Txostena",
                START_VOTING_PERIOD: "Hasi Bozketa Aldia",
                END_VOTING_PERIOD: "Amaitu Bozketa Aldia",
                ALLOW_VOTING_PERIOD_END: "Baimendu Bozketa Aldi Amaiera",
                START_ENROLLMENT_PERIOD: "Hasi Matrikula Aldia",
                END_ENROLLMENT_PERIOD: "Amaitu Matrikula Aldia",
                START_LOCKDOWN_PERIOD: "Hasi Blokeo Aldia",
                END_LOCKDOWN_PERIOD: "Amaitu Blokeo Aldia",
                ALLOW_TALLY: "Baimendu Zenbaketa",
                START_READINESS_TEST: "Hasi hauteskundeetarako prestutasun-proba",
                END_READINESS_TEST: "Amaitu hauteskundeetarako prestutasun-proba",
                START_FINAL_TESTING: "Hasi azken probak eta blokeoa",
                END_FINAL_TESTING: "Amaitu azken probak eta blokeoa",
                START_TEST_VOTING: "Hasi proba-bozketa",
                END_TEST_VOTING: "Amaitu proba-bozketa",
            },
            warning: {
                votingWindowDays:
                    "{{election}} hauteskundearen bozketa-aldiak {{days}} egun lokal hartzen ditu ({{start_local}} - {{end_local}}, {{time_zone}}); arauak {{expected}} eskatzen ditu.",
                finalTestingLeadTime:
                    "{{election}} hauteskundearen azken probak {{final_testing_local}}(e)an hasten dira, bozketa {{voting_start_local}}(e)an ireki baino {{minimum_days}} egun baino gutxiago lehenago ({{time_zone}}).",
                closeBeforeOpen:
                    "{{election}} hauteskundearen bozketa ireki aurretik edo irekitzean bertan ixten da ({{start_local}} - {{end_local}}, {{time_zone}}).",
                shortLastDay:
                    "{{election}} hauteskundearen azken bozketa-egunak {{hours}} ordu ditu, {{minimum_hours}} baino gutxiago: bozketa {{end_local}}(e)an ixten da ({{time_zone}}).",
            },
            election: {
                label: "Hauteskundea",
            },
            empty: {
                header: "Ez dago Programatutako Gertaerarik oraindik.",
                body: "Bat sortu nahi duzu?",
                button: "Sortu Programatutako Gertaera",
            },
            create: {
                title: "Sortu Programatutako Gertaera",
                subtitle: "Sortu Programatutako Gertaera konfigurazio berri bat.",
            },
            edit: {
                title: "Editatu Programatutako Gertaera",
                subtitle: "Editatu Programatutako Gertaera konfigurazioa.",
                delete: "Ziur zaude Programatutako Gertaera hau ezabatu nahi duzula?",
            },
            fields: {
                electionId: "Hauteskundea",
                eventProcessor: "Mota",
                stoppedAt: "Gelditua:",
                scheduledDate: "Programatua:",
            },
        },
        reportsScreen: {
            title: "Txostenak",
            subtitle: "Sortu txostenak hauteskunde gertaeretarako",
            messages: {
                createSuccess: "Txostena arrakastaz sortua",
                createError: "Errorea Txostena sortzerakoan",
                submitError: "Errorea Txostena bidaltzerakoan",
                updateSuccess: "Txostena arrakastaz eguneratua",
                passwordMismatch:
                    "Pasahitza eta pasahitza berretsi ez datoz bat. Mesedez, ziurtatu eremu biek pasahitz bera dutela.",
                incorectPassword: "Pasahitz okerra",
                decryptFileTitle: "Nola deszifratzen den fitxategia",
                decryptInstructions: `1. '-in' :Zifratutako fitxategiaren bidea. \n2. '-out' :Deszifratutako fitxategia gordeko den bidea. \n3. '-pass' :Fitxategia zifratzeko erabilitako pasahitza. \n`,
                encryptSuccess: "Txostenaren enkriptatzea behar bezala konfiguratu da",
                encryptError: "Errorea txostenaren enkriptatzea konfiguratzean",
            },
            reportType: {
                BALLOT_RECEIPT: "Bozketa Jasoagiria",
                ELECTORAL_RESULTS: "Hauteskunde Emaitzak",
                MANUAL_VERIFICATION: "Eskuzko Egiaztapena",
                PARTICIPATION_REPORT: "Parte-hartze Txostena",
                STATISTICAL_REPORT: "Txosten Estatistikoa",
                OVCS_EVENTS: "Atzerriko Bozketa Monitorizazioa - OVCS Gertaerak",
                AUDIT_LOGS: "Auditoria Egunkariak",
                ACTIVITY_LOG: "Jarduera Egunkaria",
                STATUS: "Egoera",
                OVCS_INFORMATION: "OVCS Informazioa",
                OVERSEAS_VOTERS: "Atzerriko bozkatzaileen zerrenda",
                OV_USERS_WHO_VOTED: "Bozkatu duten Atzerriko Bozkatzaileen zerrenda",
                OV_WITH_VOTING_STATUS: "Bozketa Egoerarekin Atzerriko Bozkatzaileen zerrenda",
                OVCS_STATISTICS: "Atzerriko Bozketa Monitorizazioa - OVCS Estatistikak",
                PRE_ENROLLED_OV_BUT_DISAPPROVED:
                    "Aurre-matrikulatu baina Ezarri diren ABen zerrenda",
                PRE_ENROLLED_OV_SUBJECT_TO_MANUAL_VALIDATION:
                    "Aurre-matrikulatu baina Eskuzko Baliozkotzearen menpean dauden ABen zerrenda",
            },
            reportEncryptionPolicy: {
                title: "Zifratze Politika",
                UNENCRYPTED: "Zifratu gabe",
                CONFIGURED_PASSWORD: "Konfiguratutako Pasahitza",
            },
            empty: {
                header: "Ez dago Txostenik oraindik.",
                body: "Bat sortu nahi duzu?",
                button: "Sortu Txostena",
            },
            create: {
                title: "Sortu Txostena",
                subtitle: "Sortu Txosten konfigurazio berri bat.",
            },
            edit: {
                title: "Editatu Txostena",
                subtitle: "Editatu Txosten konfigurazioa.",
                delete: "Ziur zaude Txosten hau ezabatu nahi duzula?",
            },
            fields: {
                electionId: "Hauteskundea",
                template: "Txantiloia",
                reportType: "Txosten Mota",
                repeatable: "Errepika daiteke",
                cronExpression: "Cron Adierazpena",
                emailRecipients: "Email Hartzaileak",
                emailRecipientsPlaceholder: "Idatzi emaila eta sakatu Enter",
            },

            delete: {
                body: "Ziur zaude Txosten hau ezabatu nahi duzula?",
            },
            actions: {
                generate: "Sortu",
                delete: "Ezabatu",
                edit: "Editatu",
                preview: "Aurreikusi",
            },
        },
        googleMeet: {
            title: "Google Meet Esteka Sortu",
            generateButton: "Google Meet",
            meetingTitle: "Bileraren Izenburua",
            description: "Deskribapena (Aukerakoa)",
            startDate: "Hasiera Data",
            startTime: "Hasiera Ordua",
            duration: "Iraupena (minutuak)",
            attendeeEmails: "Partaideen Emailak",
            attendeeEmailHelp: "Komaz banandutako emailak bilerako partaideentzat",
            note: "Oharra: Honek zure Google Calendar-en gertaera bat sortuko du Google Meet esteka batekin. Zure Google kontuan saioa hasi beharko duzu.",
            success: "Google Meet Esteka Arrakastaz Sortua!",
            copy: "Arbelera kopiatu",
            copied: "Esteka arbelera kopiatua!",
            instructions:
                "Partekatu esteka hau partaideekin bileran parte hartzeko. Egutegi gertaera zure Google Calendar-era gehitu da.",
            generating: "Sortzen...",
            generate: "Meet Esteka Sortu",
        },
        common: {
            export: "Esportazioa eragiketa luzea izan daiteke. Ziur zaude erregistroak esportatu nahi dituzula?",
            resources: {
                electionEvent: "Hauteskunde Gertaera",
                election: "Hauteskundea",
                contest: "Lehiaketa",
                candidate: "Hautagaia",
                noResult: {
                    askCreate: "Bat sortu nahi duzu?",
                },
            },
            label: {
                add: "Gehitu",
                actions: "Ekintzak",
                create: "Sortu",
                delete: "Ezabatu",
                archive: "Artxibatu",
                unarchive: "Desartxibatu",
                cancel: "Ezeztatu",
                edit: "Editatu",
                yes: "Bai",
                no: "Ez",
                save: "Gorde",
                close: "Itxi",
                back: "Atzera",
                next: "Hurrengoa",
                warning: "Abisua",
                json: "Aurreikusi",
                noResult: "Ez dago emaitzarik",
                import: "Inportatu",
                export: "Esportatu",
                loadingData: "Datuak kargatzen ...",
                exportFormat: "{{format}} formatuan esportatu - '{{item}}' emaitzak",
                allResults: "hauteskunde gertaera",
                globalAreaResults: "eremu guztiak",
                title: "Izenburua",
                subtitle: "Azpititulua",
                kind: "Fitxategi mota",
                filter: "Iragazki Pertsonalizatuak",
                approve: "Onartu",
                continue: "Jarraitu",
                logout: "Saioa Itxi",
                selectTenant: "Hautatu Maizterra",
                processing: "Prozesatzen...",
                tenantName: "Maizter Izena",
            },
            language: {
                es: "Gaztelania",
                en: "Ingelesa",
                fr: "Frantsesa",
                cat: "Valentziera",
                tl: "Tagaloa",
                gl: "Galiziera",
                nl: "Nederlandera",
                eu: "Euskera",
            },
            channel: {
                online: "Linea",
                kiosk: "Kiosko",
                early_voting: "Aurre-botoa",
                telephone: "Telefono bozketa",
                other: "Beste batzuk",
            },
            message: {
                delete: "Ziur zaude elementu hau ezabatu nahi duzula?",
                continueOrLogout: "Maizter honetara konektatuta egon nahi duzu ala saioa itxi?",
            },
        },
        createResource: {
            electionEvent: "Sortu Hauteskunde Gertaera",
            election: "Sortu Hauteskundea",
            contest: "Sortu Lehiaketa",
            candidate: "Sortu Hautagaia",
        },
        importResource: {
            electionEvent: "Inportatu Hauteskunde Gertaera",
            election: "Inportatu Hauteskundea",
            contest: "Inportatu Lehiaketa",
            candidate: "Inportatu Hautagaia",
            ImportHashMismatch: "Hash-ak ez datoz bat. Osotasun egiaztapen hutsegitea.",
        },
        sideMenu: {
            electionEvents: "Hauteskunde Gertaerak",
            search: "Bilatu",
            usersAndRoles: "Erabiltzaileak eta Rolak",
            logs: "Egunkariak",
            settings: "Ezarpenak",
            help: "Laguntza",
            templates: "Txantiloiak",
            active: "Aktiboa",
            archived: "Artxibatua",
            addResource: {
                electionEvent: "Sortu Hauteskunde Gertaera",
                election: "Sortu Hauteskundea",
                contest: "Sortu Lehiaketa",
                candidate: "Sortu Hautagaia",
            },
            menuActions: {
                archive: {
                    electionEvent: "Artxibatu Hauteskunde Gertaera hau",
                },
                unarchive: {
                    electionEvent: "Desartxibatu Hauteskunde Gertaera hau",
                    election: "Desartxibatu hauteskunde hau",
                    contest: "Desartxibatu Lehiaketa hau",
                    candidate: "Desartxibatu Hautagaia hau",
                },
                remove: {
                    electionEvent: "Kendu Hauteskunde Gertaera hau",
                    election: "Kendu Hauteskunde hau",
                    contest: "Kendu Lehiaketa hau",
                    candidate: "Kendu Hautagaia hau",
                },
                messages: {
                    confirm: {
                        archive: "Ziur zaude elementu hau artxibatu nahi duzula?",
                        unarchive: "Ziur zaude elementu hau desartxibatu nahi duzula?",
                        delete: "Ziur zaude elementu hau ezabatu nahi duzula?",
                        sealsUnknown:
                            "Ezin izan dira hautestontzien zigiluak egiaztatu: hautestontzi zigilaturik badu, ezabaketa baztertu egingo da.",
                    },
                    notification: {
                        success: {
                            archive: "Elementua artxibatu da",
                            unarchive: "Elementua desartxibatu da",
                            delete: "Elementua ezabatu da",
                            reloading: "Itxaron. Orria une batez berriro kargatuko da.",
                        },
                        error: {
                            archive: "Errorea elementu hau artxibatzen saiatzean",
                            unarchive: "Errorea elementu hau desartxibatzen saiatzean",
                            delete: "Errorea elementu hau ezabatzen saiatzean",
                            deleteSealedElection:
                                "Hauteskunde honek hautestontzi zigilatuak ditu eta ezin da ezabatu. Artxibatu bere hauteskunde-gertaera horren ordez.",
                            deleteSealedEvent:
                                "Hauteskunde-gertaera honek hautestontzi zigilatuak ditu eta ezin da ezabatu. Artxibatu ezazu horren ordez.",
                            deleteMaybeSealed:
                                "Errorea elementu hau ezabatzen saiatzean. Hautestontzi zigilaturik badu, ezin da ezabatu.",
                        },
                    },
                },
            },
        },
        candidateScreen: {
            common: {
                subtitle: "Hautagaia konfigurazioa.",
            },
            edit: {
                externalId: "Kanpoko IDa",
                general: "Orokorra",
                type: "Mota",
                image: "Irudia",
                isDisabled: "Desgaituta",
                isExplicitInvalid: "Baliogabeko Botoa",
                isExplicitBlank: "Boto Zuria",
                isCategoryList: "Kategoria Zerrenda",
                isWriteIn: "Idatzi",
            },
            field: {
                name: "Izena",
                alias: "Ezizena",
                description: "Deskribapena",
            },
            options: {
                "candidate": "Hautagaia",
                "option": "Aukera",
                "write-in": "Idatzi",
                "open-list": "Zerrenda Irekia",
                "closed-list": "Zerrenda Itxia",
                "semi-open-list": "Zerrenda Erdi Irekia",
                "invalid-vote": "Baliogabeko Botoa",
                "blank-vote": "Boto Zuria",
            },
            invalidVotePosition: {
                label: "Baliogabeko Botoaren Posizioa",
                null: "Ezer ez (Lehenetsia)",
                top: "Goian",
                bottom: "Behean",
            },
            error: {},
            createCandidateSuccess: "Hautagaia sortua",
            createCandidateError: "Errorea hautagaia sortzerakoan",
        },
        contestScreen: {
            common: {
                subtitle: "Lehiaketa konfigurazioa.",
            },
            edit: {
                externalId: "Kanpoko IDa",
                general: "Orokorra",
                type: "Mota",
                image: "Irudia",
                system: "Bozketa Sistema",
                design: "Bozketa Diseinua",
                reorder: "Berrantolatu hautagaiak",
                policies: "Politikak",
            },
            field: {
                name: "Izena",
                alias: "Ezizena",
                description: "Deskribapena",
            },
            options: {
                "non-preferential": "Ez Lehentasuna",
                "plurality-at-large": "Pluralitatea Orokorrean",
                "instant-runoff": "Berehalako Bigarren Itzulia",
                "random": "Ausazkoa",
                "external-procedure": "Kanpoko prozedura",
                "custom": "Pertsonalizatua",
                "alphabetical": "Alfabetikoa",
            },
            tieBreakingPolicy: {
                label: "Berdinketa hausteko politika",
            },
            auditButtonConfig: {
                "label": "Auditoria Botoi Erakutsi Aukerak",
                "show": "Erakutsi",
                "not-show": "Ez Erakutsi",
                "show-in-help": "Erakutsi Laguntza Elkarrizketa-koadroan",
            },
            underVotePolicy: {
                "label": "Azpi Boto Politika",
                "allowed": "Baimenduta",
                "warn-only-in-review": "Abisatu Berrikuspena",
                "warn": "Abisatu",
                "warn-and-alert": "Abisatu eta Alerta",
                "warn-and-confirm-in-review": "Abisatu eta Berretsi Berrikuspenean",
            },
            invalidVotePolicy: {
                "label": "Baliogabeko Boto Politika",
                "allowed": "Baimenduta",
                "warn": "Abisatu",
                "warn-invalid-implicit-and-explicit":
                    "Abisatu Baliogabeko Inplizitu eta Esplizitua",
                "not-allowed": "Ez Baimenduta",
                "allowed-with-exclusive-explicit": "Baimenduta, Esplizitua Esklusiboa",
            },
            candidatesIconCheckboxPolicy: {
                "label": "Hautagaien kontrol-laukiaren ikono forma",
                "square-checkbox": "Kontrol-lauki Karratua",
                "round-checkbox": "Kontrol-lauki Biribila",
            },
            checkableListPolicy: {
                "allow-selecting-candidates-and-lists": "Hautagaiak eta Zerrendak",
                "allow-selecting-candidates": "Hautagaiak Soilik",
                "allow-selecting-lists": "Zerrendak Soilik",
                "disabled": "Desgaituta",
            },
            collapsibleListsPolicy: {
                "label": "Zerrenda tolesgarriak",
                "disabled": "Desgaituta",
                "enabled-expanded": "Gaituta (zabalik hasten da)",
                "enabled-collapsed": "Gaituta (tolestuta hasten da)",
            },
            blankVotePolicy: {
                "label": "Boto Zuri Politika",
                "allowed": "Baimenduta",
                "warn-only-in-review": "Abisatu Berrikuspena",
                "warn": "Abisatu",
                "not-allowed": "Ez Baimenduta",
            },
            overVotePolicy: {
                "label": "Gain Boto Politika",
                "allowed": "Baimenduta",
                "allowed-with-msg": "Baimenduta Abisu Mezuarekin",
                "allowed-with-msg-and-alert": "Baimenduta Abisu mezu eta Alertarekin",
                "not-allowed-with-msg-and-alert": "Ez Baimenduta Abisu mezu eta Alertarekin",
                "not-allowed-with-msg-and-disable":
                    "Ez Baimenduta Abisu mezuarekin eta Gehiago hautatzea Desgaitu",
            },
            duplicatedRankPolicy: {
                "label": "Boto baliogabea - Bikoiztutako Rankaren Politika",
                "allowed-warn-and-dialog":
                    "Erakutsi ohartarazpena eta elkarrizketa-koadroa (botatzaileak aurrera egin dezake)",
                "not-allowed-warn-and-dialog":
                    "Erakutsi ohartarazpena eta elkarrizketa-koadroa (botatzaileak ezin du aurrera egin)",
            },
            preferenceGapsPolicy: {
                "label": "Boto baliogabea - Saltatutako Ranken Politika",
                "allowed-warn-and-dialog":
                    "Erakutsi ohartarazpena eta elkarrizketa-koadroa (botatzaileak aurrera egin dezake)",
                "not-allowed-warn-and-dialog":
                    "Erakutsi ohartarazpena eta elkarrizketa-koadroa (botatzaileak ezin du aurrera egin)",
            },
            paginationPolicy: {
                label: "Orri Izena",
            },
            isAcclaimed: {
                label: "Aklamazioz erabakia",
                helperText:
                    "Botoemaileek lehiaketa hau ikusten dute baina ezin dute ezer hautatu, ez da ezer erregistratzen eta hautagai guztiak irabazle gisa jasotzen dira zero bozkarekin. Ezarri hau boto-txartelak argitaratu aurretik: ondoren aldatzeak dagoeneko emandako boto-txartelak baliogabetzen ditu.",
            },
            allowWriteins: {
                label: "Eskuzko hautagaitzak baimendu",
            },
            maxVotes: {
                helperText:
                    "Hautesleak aukeratu ahal dituen hautagai kopuru maximoa (ez-hobesentziazko botazioa).",
                helperTextPreferential:
                    "Hautesleek erabil dezaketen posizio altuena (adib. '5' 1-5 posizioak ahalbidetzen ditu). Ordenatu beharreko hautagai kopurua baino gehiago izan behar du (hobesentziazko botazioa).",
            },
            error: {},
            createContestSuccess: "Lehiaketa sortua",
            createContestError: "Errorea hautagaia sortzerakoan",
        },
        keysGeneration: {
            configureStep: {
                create: "Sortu Giltzen Zeremonia",
                name: "Giltzen Zeremonia Izena",
                allElections: "Hauteskunde Guztiak",
                title: "Sortu Hauteskunde Gertaera Giltzen Zeremonia",
                subtitle:
                    "Giltzen Zeremonoan fideikomisario bakoitzak Hauteskunde Gertaerarako giltza pribatuaren zatia sortuko eta deskargatuko du. Jarraitzeko, mesedez aukeratu zeremonoan parte hartuko duten fideikomisarioak eta atalasea, zenbaketerako behar den fideikomisario gutxieneko kopurua.",
                threshold: "Atalasea",
                trusteeList: "Fideikomisarioak",
                errorMinTrustees_one:
                    "{{selected}} fideikomisario soilik hautatu duzu, baina gutxienez {{threshold}} hautatu behar dituzu.",
                errorMinTrustees_other:
                    "{{selected}} fideikomisario soilik hautatu dituzu, baina gutxienez {{threshold}} hautatu behar dituzu.",
                errorThreshold:
                    "{{selected}} atalasea hautatu duzu baina {{min}} eta {{max}} artean egon behar da.",
                errorCreatingCeremony: "Errorea Giltzen Zeremonia sortzerakoan: {{error}}",
                createCeremonySuccess: "Giltzen Zeremonia sortua",
                confirmdDialog: {
                    ok: "Bai, Sortu Giltzen Zeremonia",
                    cancel: "Ezeztatu",
                    title: "Ziur zaude Giltzen Zeremonia sortu nahi duzula?",
                    automaticCeremonyTitle:
                        "Ziur zaude giltza-ekitaldi automatiko bat sortu nahi duzula?",
                    description:
                        "Giltzen Zeremonia sortzear zaude. Ekintza honek Fideikomisarioei jakinaraziko die Hauteskunde Gertaera Giltzen sorkuntzan eta banaketan parte har dezaten.",
                    automaticCeremonyDescription:
                        "Giltza-ekitaldi automatiko bat sortzear zaude. Honek ez die arduradunei parte hartzeko jakinaraziko.",
                },
                filterTrustees: "Iragazi Fideikomisarioak",
                errorPermisionLabels:
                    "Errorea Giltzen Zeremonia sortzerakoan: Gutxienez baimen-etiketa bat falta da.",
                automaticCeremonyToggle: "Zeremonia automatikoa",
            },
            ceremonyStep: {
                cancel: "Ezeztatu Giltzen Zeremonia",
                progressHeader: "Giltzen Zeremonia Aurrerapena",
                description:
                    "Pantaila honek Hauteskunde Gertaeraren Giltzen Zeremonaren aurrerapena eta egunkariak erakusten ditu. Giltzen Zeremonoan fideikomisario bakoitzak Hauteskunde Gertaerarako giltza pribatuaren zatia sortuko eta deskargatuko du.",
                executionStatus: "Egoera: {{status}}",
                confirmdDialog: {
                    ok: "Bai, Ezeztatu Giltzen Zeremonia Sortu",
                    cancel: "Itzuli Giltzen Zeremoniara",
                    title: "Ziur zaude Giltzen Zeremonia ezeztatu nahi duzula?",
                    description:
                        "Giltzen Zeremonia ezeztatuera zaude. Ekintza hau egin ondoren, Giltzen Zeremonia arrakastatsua izan dadin berri bat sortu beharko duzu.",
                },
                header: {
                    trusteeName: "Fideikomisario Izena",
                    fragment: "Giltza Zatia Sortua",
                    downloaded: "Giltza Pribatu Zatia Deskargatua",
                    checked: "Giltza Pribatu Zatia Egiaztatua",
                },
                logsHeader: {
                    title: "Egunkariak",
                    date: "Data",
                    entry: "Sarrera",
                },
                emptyLogs: "Ez dago egunkaririk oraindik.",
            },
            startStep: {
                title: "Fideikomisario Giltzen Zeremonia",
                subtitle:
                    "Giltzen Zeremonoan Fideikomisario gisa (<strong>{{name}}</strong>) parte hartzear zaude. Honek urrats hauek dakartza:",
                one: "<strong>Deskargatu</strong> zure Zifratutako Giltza Pribatua.",
                two: "Sortu Zifratutako Giltza Pribatuaren <strong>Babeskopia</strong> anitz.",
                three: "<strong>Egiaztatu</strong> babeskopiak ondo funtzionatzen dutela.",
            },
            downloadStep: {
                title: "Deskargatu Zifratutako Giltza Pribatua",
                subtitle:
                    "Jarraitzeko, mesedez deskargatu eta gorde zure Zifratutako Giltza Pribatua gutxienez bi gailu desberdinetan:",
                downloadButton: "Deskargatu zure Zifratutako Giltza Pribatua",
                downloaded: "Zifratutako Giltza Pribatua behar bezala deskargatu da.",
                errorEmptyKey: "Deskarga errorea, fitxategi hutsa",
                unexpectedError: "Ezin izan da giltza pribatua deskargatu. Saiatu berriro.",
                alreadyVerified: "Zure giltza pribatua deskargatuta eta egiaztatuta zegoen.",
                unavailable:
                    "Giltza pribatuaren deskarga jada ez dago erabilgarri, zeremoniak aurrera egin duelako.",
                confirmdDialog: {
                    ok: "Berretsi Babeskopiak eta Jarraitu",
                    cancel: "Itzuli",
                    title: "Egin babeskopia zure Zifratutako Giltza Pribatuari",
                    description:
                        "Mesedez, egin babeskopia zure Zifratutako Giltza Pribatuari gutxienez bi leku seguru desberdinetan eta gero berretsi behean:",
                    firstCopy: "Lehen babeskopia segurua",
                    secondCopy: "Bigarren babeskopia segurua",
                    confirmError:
                        "Sortu behar diren babeskopiak eta markatu berrespena laukiak jarraitzeko",
                },
            },
            checkStep: {
                title: "Egiaztatu zure Zifratutako Giltza Pribatu Babeskopiak",
                verifyButton: "Egiaztatu gakoa",
                subtitle:
                    "Igo Zifratutako Giltza Pribatu Babeskopia bat zuzena dela egiaztatzeko. Behar adina aldiz saia zaitezke, zure babeskopia desberdinetatik:",
                errorUploading:
                    "Zifratutako Giltza Pribatu Babeskopia baliogabea, mesedez saiatu berriro",
                errorEmptyFile: "Fitxategia hutsa edo ez da aurkitu",
                verified: "Babeskopia arrakastaz egiaztatua.",
            },
        },
        miruExport: {
            create: {
                success: "Transmisio Paketea sortzen...",
                error: "Errorea Transmisio Paketea sortzerakoan ",
            },
            send: {
                success: "Transmisio Paketea bidaltzen...",
                error: "Errorea Transmisio Paketea bidaltzerakoan ",
            },
        },
        tally: {
            errorUploadingSignature: "Errorea izan da sinadura igotzean",
            downloadTransmissionPackage: "Deskargatu Transmisio Paketea",
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
                title: "'{{name}}' Eremuaren eta '{{eventName}}' Hauteskundearen Transmisio Paketea",
                description:
                    "Transmisio Paketea Helburu Zerbitzarietara esportatzeko edo deskargatzeko aukera ematen du.",
                actions: {
                    send: {
                        title: "Bidali",
                        dialog: {
                            title: "Transmisio Paketea bidali nahi duzu?",
                            description:
                                "Mesedez, berretsi `{{name}}` Eremuaren Transmisio Paketea Helburu Zerbitzarietara bidali nahi duzula.",
                            confirm: "Bidali Transmisio Paketea",
                            cancel: "Itxi",
                        },

                        disabled:
                            "Beharrezko sinadurak falta dira, edo transmisio-paketea helmuga guztietara bidali da dagoeneko.",
                    },
                    regenerate: {
                        title: "Bersortu",
                        dialog: {
                            title: "Transmisio Paketea bersortu nahi duzu?",
                            description:
                                "Mesedez, berretsi `{{name}}` Eremuaren Transmisio Paketea bersortu nahi duzula",
                            confirm: "Bersortu Transmisio Paketea",
                            cancel: "Itxi",
                        },
                    },
                    sign: {
                        title: "Bersortu",
                        dialog: {
                            title: "Transmisio Paketea sinatu nahi duzu?",
                            description:
                                "Mesedez, berretsi `{{name}}` Eremuaren Transmisio Paketea bersortu nahi duzula",
                            confirm: "Sinatu Transmisio Paketea",
                            cancel: "Itxi",
                            input: {
                                placeholder: "Sartu zure pasahitza",
                            },
                        },
                    },
                    download: {
                        title: "Deskargatu",
                        emlTitle: "Deskargatu EML {{date}}",
                        transmissionPackageTitle: "Deskargatu Transmisio Paketea {{date}}",
                        transmissionReportTitle: "Deskargatu Transmisio Txostena",
                        dialog: {
                            title: "Transmisio Paketea deskargatu nahi duzu?",
                            description:
                                "Mesedez, berretsi `{{name}}` Eremuaren Transmisio Paketea deskargatu nahi duzula.",
                            confirm: "Deskargatu Transmisio Paketea",
                            cancel: "Itxi",
                        },
                    },
                },
                destinationServers: {
                    title: "Helburu Zerbitzariak",
                    description:
                        "Beheko taulak Helburu Zerbitzari bakoitzaren bidaltze egoera erakusten du.",
                    status: "Bidalia {{signed}}/{{total}}",
                    table: {
                        serverName: "Zerbitzari Izena",
                        sendStatus: "Bidaltze Egoera",
                    },
                },
                signatures: {
                    title: "Sinadurak",
                    description:
                        "Kideek transmisio-paketea sina dezakete. Taulak kide bakoitzaren sinadura-egoera erakusten du.",
                    status: "{{signed}}/{{total}} Sinatuak, {{minimum}} gutxienez",
                    table: {
                        trusteeName: "Kidea",
                        signed: "Sinatua du",
                    },
                },
            },
            sendToTransmissionPackageServers: "Bidali Transmisio Paketea '{{name}}' Eremu",
            uploadTransmissionPackage: "Sinatu Transmisio Paketea",
            uploadTransmissionPackageDesc:
                "Igo zure Sinadura Transmisio Paketea sinatzeko. Aukerako eragiketa da hau.",
            exportElectionArea: "Sortu Transmisio Paketea '{{name}}' Eremu",
            generateReport: "Sortu {{name}}",
            templateTitle: "Emaitzen Txantiloia",
            templateSubTitle: "Aukeran gainidatzi emaitzen txantiloia.",
            keysCeremonyTitle: "Giltzen Zeremonia",
            keysCeremonySubTitle: "Hautatu zenbaketa honetarako Giltzen Zeremonia",
            ceremonyTitle: "Zenbaketan Hauteskundeak",
            initializationTitle: "Hasierako Txostenerako Hauteskundeak",
            ceremonySubTitle: "Aukeratu zenbakatu nahi dituzun hauteskundeak",
            tallyTitle: "Hauteskunde Zenbaketa Aurrerapena",
            logsTitle: "Egunkariak",
            resultsTitle: "Emaitzak eta Parte-hartzea",
            generalInfoTitle: "Informazio Orokorra",
            trusteeTallyTitle: "Fideikomisarioak",
            trusteeTallySubTitle: "Giltza zatiaren inportazio egoera",
            ballotBoxes: {
                unavailable: "Zigiluak ez daude eskuragarri",
                sealed: "{{sealed}}/{{total}} zigilatuta",
                publishing: "Zigilatuta, {{published}}/{{total}} iragarki-taulan",
                sealing: "Zigilatzea: {{time}}",
                notSealed: "Zigilatu gabe",
                help: "Hauteskunde bat zenbatu daiteke hautestontzi guztiak zigilatuta daudenean eta haien zigiluak iragarki-taulan daudenean.",
                failed: "Zigilatu gabe: gorabehera",
                overdue: "Zigilatzea berandu",
                blocked: "{{name}}: {{reason}}",
                reason: {
                    "not-sealed":
                        "bozketa ez da itxi, beraz bere hautestontziek ez dute zigilurik oraindik",
                    "sealing": "bere hautestontziak grazia-aldia amaitzean zigilatzen dira",
                    "overdue":
                        "bere hautestontziek epea gainditu dute eta oraindik ez daude zigilatuta (ikus bere Panela)",
                    "publishing":
                        "bere zigiluetako batzuk oraindik iragarki-taulan argitaratzen ari dira",
                    "failed":
                        "hautestontzi bat ezin izan da zigilatu, gorabehera bat (ikus bere Panela)",
                    "unavailable": "ezin izan dira bere hautestontzien zigiluak irakurri",
                },
            },
            eligibility: {
                ballotBoxesUnavailable:
                    "Hautatutako hauteskunde baten hautestontzien zigiluak ezin izan dira irakurri, beraz ezin da oraindik zenbatu. Kargatu berriro orria berriro saiatzeko.",
                selectElection: "Hautatu gutxienez hauteskunde bat.",
                publishElection:
                    "Argitaratu hautatutako hauteskunde bakoitza zenbaketa sortu aurretik.",
                tallyDisallowed: "Zenbaketa desgaituta dago hautatutako hauteskunde batean.",
                endVoting:
                    "Amaitu hautatutako hauteskunde bakoitzeko bozketa eta gelditu kanal aktiboak zenbaketa sortu aurretik.",
                sealBallotBoxes:
                    "Hauteskunde bat zenbatu daiteke hautestontzi guztiak zigilatuta daudenean eta haien zigiluak iragarki-taulan daudenean.",
            },
            createTallySuccess: "Zenbaketa sortua",
            createTallyError: "Ezin izan da Zenbaketa sortu",
            startTallySuccess: "Zenbaketa hasita",
            startTallyError: "Ezin izan da Zenbaketa hasi",
            startTallyCeremonySuccess: "Zenbaketa Zeremonia hasita",
            startTallyCeremonyError: "Ezin izan da Zenbaketa Zeremonia hasi",
            cancelTallyCeremonySuccess: "Zenbaketa Zeremonia ezeztatua",
            cancelTallyCeremonyError: "Ezin izan da Zenbaketa Zeremonia ezeztatu",
            recountTallyCeremony: "Berriro zenbatu",
            recountTallyCeremonyMessage:
                "Honek emaitza-gertaera berri bat sortuko du osatutako zenbaketa-saiorako.",
            recountTallyCeremonyStarting: "Birzenbaketa hasten...",
            recountTallyCeremonySuccess: "Birzenbaketa hasita",
            recountTallyCeremonyError: "Ezin izan da birzenbaketa hasi",
            recountTallyCeremonyOk: "Berriro zenbatu",
            trusteeTitle: "Fideikomisarioen prozesua",
            trusteeSubTitle: "Mesedez igo zure giltza zatia",
            invited: "Zenbaketa zeremonoia batean parte hartzeko gonbidatu zaituzte. Mesedez ",
            click: "sakatu zenbaketa Ekintzan",
            participate: "parte hartzeko.",
            breadcrumbSteps: {
                start: "Hasi",
                finish: "Amaitu",
                tally: "Zenbaketa",
                results: "Emaitzak",
                ceremony: "Zeremonia",
            },
            common: {
                title: "Zenbaketa",
                subTitle: "Zenbaketa konfigurazioa.",
                cancel: "Atzera",
                next: "Hurrengoa",
                date: "Zenbaketa Data",
                global: "Orokorra",
                noTrustees: "Ez dago fideikomisariorik oraindik",
                imported: " fideikomisarioak giltza inportatu dute",
                needed: " fideikomisario behar",
                start: "Hasi Zenbaketa",
                ceremony: "Hasi Zenbaketa Zeremonia",
                initialization: "Hasi Hasierako Txostena",
                results: "Emaitzak",
                dialog: {
                    ok: "Ados",
                    okTally: "Hasi Zenbaketa",
                    okCancel: "Ezeztatu Zenbaketa",
                    cancel: "Itxi",
                    title: "Ziur zaude zeremonia bat hasi nahi duzula?",
                    tallyTitle: "Ziur zaude zenbaketa hasi nahi duzula?",
                    cancelTitle: "Ziur zaude zenbaketa ezeztatu nahi duzula?",
                    message:
                        "Zenbaketa zeremonia bat hasi nahian zaude. Ekintza honek fideikomisarioei jakinaraziko die beren giltza zatiak inportatzeko.",
                    cancelMessage:
                        "Zenbaketa zeremonia ezeztatuzera noa. Ekintza hau ezin da desegin.",
                    ceremony:
                        "Beharrezko fideikomisario guztiek beren giltza zatiak egiaztatu dituzte. Dena prest dago emaitzak jasotzen hasteko. Zenbaketa hasi nahi duzu?",
                    startAutomatedTallyMessage:
                        "Hautatu 'Start Tally' zenbaketa-prozesua exekutatzeko eta emaitzak bistaratzeko, edo 'Close' ezeztatzeko.",
                },
            },
            table: {
                ballotBoxes: "Hautestontziak",
                elections: "Hauteskundeak",
                selected: "Hautatua",
                status: "Egoera",
                progress: "Aurrerapena",
                method: "Zenbaketa Metodoa",
                elegible: "Bozkatzaile Eskudunak",
                number: "Boto Kopurua",
                total: "Guztira",
                turnout: "%",
                candidates: "Hautagaien Emaitzak",
                options: "Aukerak",
                global: "Parte-hartze Laburpena",
                elegible_census: "Bozkatzaile Eskudunak",
                cast_votes: "Boto Kopurua",
                cast_votes_percent: "Boto Ehunekoa",
                total_votes: "Bozkatzaile Guztiak",
                total_votes_percent: "Parte-hartzea",
                total_votes_counted: "Zenbatutako Boto Guztiak",
                total_auditable_votes: "Auditatu daitezkeen Boto Guztiak",
                total_valid_votes: "Baliozko Boto Guztiak",
                total_valid_votes_percent: "Baliozko Boto Ehunekoa",
                total_invalid_votes: "Baliogabeko Boto Guztiak",
                total_invalid_votes_percent: "Baliogabeko Boto Ehunekoa",
                explicit_invalid_votes: "Esplizituki Baliogabeko Botoak",
                explicit_invalid_votes_percent: "Esplizituki Baliogabeko Boto Ehunekoa",
                implicit_invalid_votes: "Inplizituki Baliogabeko Botoak",
                implicit_invalid_votes_percent: "Inplizituki Baliogabeko Boto Ehunekoa",
                blank_votes: "Boto Zuriak",
                explicit_blank_votes: "Boto Zuri Esplizituak",
                implicit_blank_votes: "Boto Zuri Inplizituak",
                blank_votes_percent: "Boto Zuri Ehunekoa",
                number_of_votes: "Boto Kopurua",
                winning_position: "Irabazle posizioa",
                weight: "Pisua",
                preferential: {
                    candidate: "Hautagaia",
                    winner: "Irabazlea",
                    eliminated: "Baztertua",
                    round: "Txanda",
                },
                total_declined_to_vote: "Bozkatzeari uko egindakoen guztira",
                total_blank_ballots: "Boto-txartel Zuriak Guztira",
                participation_by_channel: "Parte-hartzea kanalaren arabera",
                channel: "Kanala",
                channel_online: "Linean",
                channel_kiosk: "Kioskoa",
                channel_early_voting: "Boto aurreratua",
                channel_telephone: "Telefonoa",
                channel_paper: "Papera",
                channel_postal: "Posta",
                channel_in_person: "Aurrez aurre",
                acclamation_note:
                    "Aklamazioz hautatua. Lehiaketa hau bozketarik gabe erabaki zen, beraz, ez zen bozkarik erregistratu.",
            },
            pendingResolutions: {
                round: "Txanda {{round}}",
                tieResolutionRequired: "Berdinketa ebazpena beharrezkoa",
                tieResolved: "Berdinketa ebatzita",
                globalArea: "Global",
                pendingResolutionsHeader: "Ebazpen zain",
                pendingResolutionStatus: "Ebazpen zain",
                resolvedStatus: "Ebatzita",
                resolutionTitle: "Ebazpena",
                selectContest: "Hautatu elementu bat ezkerraldean xehetasunak ikusteko",
                selectCandidateToAdvance: "Hautatu aurreratzeko hautagaia",
                undoResolution: "Ebazpena desegin",
                applyResolutions: "Ebazpenak aplikatu eta birkalkulatu",
                submitSuccess: "Ebazpenak bidali dira. Zenbaketa berriro hasten ari da...",
                submitError: "Errorea ebazpenak bidaltzean. Saiatu berriro.",
                filter: "Iragazi",
                save: "Gorde",
                pendingApplyStatus: "Kalkulua zain",
                filterElection: "Hauteskundea",
                filterContest: "Lehiaketa",
                filterArea: "Eremua",
                filterStatusLabel: "Egoera",
                clearFilters: "Iragazkiak garbitu",
                candidateWithVotes: "{{name}} ({{votes}} boto)",
                candidateWithVotesAndPercent: "{{name}} ({{votes}} boto, {{percent}}%)",
                tieInfoTitle: "Zenbaketa etenda berdinketa konpondu gabeagatik (Txanda {{round}})",
                tieInfoBody:
                    "Berdinketako hautagaiak ({{votes}} boto, {{percent}}%): {{candidates}}. Eskuzko desempatea behar da zenbaketa jarraitzeko.",
                tallyResumedTitle: "Zenbaketa berriro hasi da ebazpena aplikatu ondoren",
                tallyResumedBody: "Berdinketa {{date}} egunean {{user}} erabiltzaileak ebatzi zuen",
            },
            chart: {
                votesForCandidates: "Hautagaientzako Botoak",
                blankVotes: "Boto Zuriak",
                invalidVotes: "Boto Baliogabeak",
                totalVoters: "Bozkatzaile Guztiak",
                nonVoters: "Ez-bozkatzaileak",
            },
            exportAllAreas:
                "Eremu guztien emaitzak {{format}} formatuan esportatu '{{item}}'-rentzat",
        },
        publish: {
            initialization: {
                countryInfo:
                    "Sortu txostena hauteskunde-postu osorako edo herrialde baterako. Bozketa blokeatuta egongo da herrialdeen eta ekitaldi osoaren beharrezko hasieratze guztiak osatu arte.",
                countriesError:
                    "Ezin izan dira herrialde hautagarriak kargatu. Itxi eta saiatu berriro.",
                noCountries:
                    "Postu honek ez du boto-paperen estilo aktiboak dituen herrialde hautagarririk. Egiaztatu eremuak eta argitalpena hasieratu aurretik.",
                country: "Herrialdea",
                entirePost: "Postu osoa",
            },
            preview: {
                publicationAreas: "Hautatu Eremua Aurreikusteko",
                action: "Aurreikusi",
                copy: "Kopiatu esteka",
                copy_success: "Arrakasta aurreikuspen esteka kopiatzen",
                copy_error: "Huts egin du aurreikuspen esteka kopiatzen",
                success: "Arrakasta aurreikuspena irekitzen",
            },
            header: {
                change: "Argitaratzeko Aldaketak",
                viewChange: "Ikusi Argitalpena",
                history: "Argitalpen Historia",
            },
            action: {
                generateInitializationReport: "Sortu Hasierako Txostena",
                startVotingPeriod: "Hasi Bozketa",
                startKioskVoting: "Hasi Kiosko Bozketa",
                startOnlineVoting: "Hasi Online Bozketa",
                startEarlyVoting: "Hasi Aurre-botoa",
                startTelephoneVoting: "Hasi Telefono Bozketa",
                stopVotingPeriod: "Gelditu Bozketa",
                stopOnlineVoting: "Gelditu Online Bozketa",
                stopEarlyVoting: "Gelditu Aurre-botoa",
                stopTelephoneVoting: "Gelditu Telefono Bozketa",
                stopKioskVotingPeriod: "Gelditu Kiosko Bozketa",
                pauseVotingPeriod: "Pausatu Bozketa",
                pauseKioskVoting: "Pausatu Kiosko Bozketa",
                pauseOnlineVoting: "Pausatu Online Bozketa",
                pauseEarlyVoting: "Pausatu Aurre-botoa",
                pauseTelephoneVoting: "Pausatu Telefono Bozketa",
                generate: "Bersortu",
                publish: "Argitaratu Aldaketak",
                back: "Atzera",
            },
            label: {
                current: "Oraingoa",
                previous: "Aurreko Argitalpena",
                publication: "Argitalpena",
                diff: "Argitaratzeko Aldaketak",
            },
            empty: {
                header: "Ez dago Argitalpen oraindik.",
                action: "Sortu Argitalpena",
            },
            forbidden: {
                header: "Ezin da Argitaratu Giltzen Zeremonia osatu arte.",
            },
            skippedElections: {
                dismiss: "Baztertu",
                title: "Hauteskunde batzuk itxita geratzen dira",
                ballotBoxSealPolicy:
                    "{{name}} itxita geratzen da: bere bozketa itxi da eta Zigilatu ixtean aukerarekin itxiera behin betikoa da.",
                other: "{{name}} zegoen bezala utzi da ({{reason}}).",
            },
            dialog: {
                title: "Berretsi Ekintza",
                info: "Ekintza sentikorr batean klikatu duzu, beraz behar dugu berretsi jarraitzeko",
                initializationInfo:
                    "Hasierako txostena sortzean noa. Ziur zaude jarraitu nahi duzula?",
                startInfo: "Bozketa aldia hasten ari naiz. Ziur zaude jarraitu nahi duzula?",
                stopInfo: "Bozketa aldia gelditzera noa. Ziur zaude jarraitu nahi duzula?",
                stopSeal:
                    "Bozketa gelditzera zoaz hauteskunde honetan: {{name}}. Ondoren bere hautestontziak zigilatuko dira: ezin izango da botorik gehitu, aldatu edo ezabatu, eta bozketa ezin izango da berriro hasi. Ziur zaude jarraitu nahi duzula?",
                stopSealEvent:
                    "Hauteskunde guztietan bozketa gelditzera zoaz. Ondoren haien hautestontziak zigilatuko dira: ezin izango da botorik gehitu, aldatu edo ezabatu, eta bozketa ezin izango da berriro hasi. Ziur zaude jarraitu nahi duzula?",
                startSealNote:
                    "Zigilatu ixtean aukerarekin, itxitako bozketa itxita geratzen da: bozketa itxita duten hauteskundeak ez dira irekiko.",
                channel: {
                    ONLINE: "Online",
                    KIOSK: "Kiosko",
                    EARLY_VOTING: "Aurre-botoa",
                    TELEPHONE: "Telefono",
                },
                kioskStopInfo:
                    "Kiosko bozketa aldia gelditzera noa. Ziur zaude jarraitu nahi duzula?",
                pauseInfo: "Bozketa aldia pausatuzera noa. Ziur zaude jarraitu nahi duzula?",
                publishInfo: "Argitalpen bat sortzera noa. Ziur zaude jarraitu nahi duzula?",
                ok: "Berretsi",
                ko: "Ezeztatu",
                error: "Errorea bozketa argitalpena kargatzerakoan",
                error_publish: "Errorea bozketa argitalpena argitaratzerakoan",
                error_capacity: "Bozketa-estiloa sortzeak huts egin du: {{message}}",
                error_status: "Errorea bozketa argitalpen egoera aldatzerakoan",
                error_preview: "Errorea argitalpena aurreikusterakoan",
                diff: "Aldaketa guztiak errendatzeak orria erantzunik gabe utzi dezake. Ziur zaude jarraitu nahi duzula?",
                confirmation:
                    "Egiteko saiatzen ari zaren ekintza sentikorra da eta berrespena behar du. Mesedez, sartu zure pasahitza {{action}} ekintzarekin jarraitzeko.",
                stopSealNotYet:
                    "Bozketa aldia gelditzera zoaz. {{holding}} Ziur zaude jarraitu nahi duzula?",
                sealHolding_one:
                    "Zigilatu ixtean aukerarekin, bere hautestontziak gaitutako kanal guztiak itxitakoan zigilatzen dira: {{channels}} gaituta dago oraindik eta ez da itxi.",
                sealHolding_other:
                    "Zigilatu ixtean aukerarekin, bere hautestontziak gaitutako kanal guztiak itxitakoan zigilatzen dira: {{channels}} gaituta daude oraindik eta ez dira itxi.",
                sealNotEnabled:
                    "{{channel}} ez dago itxita, ezta gaituta ere hauteskunde honetan: gelditu ezazu hautestontziak zigilatzeko.",
                sealNotEnabledPost:
                    "{{post}}: {{channel}} ez dago itxita, ezta gaituta ere; gelditu ezazu hauteskunde horretan bere hautestontziak zigilatzeko.",
                sealNoChannel:
                    "Hauteskunde honetan ez dago kanalik gaituta eta bat ere ez da ireki; beraz, bere hautestontziak ez dira zigilatzen.",
                stopNeverOpened_one:
                    "{{channels}} ez da inoiz ireki: gelditzen baduzu, ez da irekiko.",
                stopNeverOpened_other:
                    "{{channels}} ez dira inoiz ireki: gelditzen badituzu, ez dira irekiko.",
                stopSealGrace_one:
                    "Bozketa gelditzera zoaz hauteskunde honetan: {{name}}. Bere hautestontziak grazia-aldia amaitzean zigilatuko dira, {{count}} minutu geroago: hortik aurrera ezin izango da botorik gehitu, aldatu edo ezabatu. Bozketa ezin izango da berriro hasi. Ziur zaude jarraitu nahi duzula?",
                stopSealGrace_other:
                    "Bozketa gelditzera zoaz hauteskunde honetan: {{name}}. Bere hautestontziak grazia-aldia amaitzean zigilatuko dira, {{count}} minutu geroago: hortik aurrera ezin izango da botorik gehitu, aldatu edo ezabatu. Bozketa ezin izango da berriro hasi. Ziur zaude jarraitu nahi duzula?",
                stopSealEventGrace_one:
                    "Hauteskunde guztietan bozketa gelditzera zoaz. Haien hautestontziak hauteskunde bakoitzaren grazia-aldia amaitzean zigilatuko dira, gehienez {{count}} minutu geroago: hortik aurrera ezin izango da botorik gehitu, aldatu edo ezabatu. Bozketa ezin izango da berriro hasi. Ziur zaude jarraitu nahi duzula?",
                stopSealEventGrace_other:
                    "Hauteskunde guztietan bozketa gelditzera zoaz. Haien hautestontziak hauteskunde bakoitzaren grazia-aldia amaitzean zigilatuko dira, gehienez {{count}} minutu geroago: hortik aurrera ezin izango da botorik gehitu, aldatu edo ezabatu. Bozketa ezin izango da berriro hasi. Ziur zaude jarraitu nahi duzula?",
                stopSealEventSome:
                    "Hauteskunde guztietan bozketa gelditzera zoaz. {{sealed}} {{holding}} Ziur zaude jarraitu nahi duzula?",
                sealedNowPart:
                    "Ondoren hauteskunde hauen hautestontziak zigilatuko dira: {{names}}. Ezin izango da botorik gehitu, aldatu edo ezabatu, eta bozketa ezin izango da berriro hasi haietan.",
                sealedGracePart_one:
                    "Hauteskunde hauen hautestontziak beren grazia-aldia amaitzean zigilatuko dira, gehienez {{count}} minutu geroago: {{names}}.",
                sealedGracePart_other:
                    "Hauteskunde hauen hautestontziak beren grazia-aldia amaitzean zigilatuko dira, gehienez {{count}} minutu geroago: {{names}}.",
                holdingEventPart_one:
                    "{{names}}: beste kanal bat gaituta dago eta ez da itxi; bere hautestontziak kanal hori itxitakoan zigilatuko dira.",
                holdingEventPart_other:
                    "{{names}}: beste kanal bat gaituta dute eta ez da itxi; haien hautestontziak kanal horiek itxitakoan zigilatuko dira.",
                noChannelEventPart_one:
                    "{{names}}: ez dago kanalik gaituta eta bat ere ez da ireki; bere hautestontziak ez dira zigilatzen.",
                noChannelEventPart_other:
                    "{{names}}: ez dute kanalik gaituta eta bat ere ez da ireki; haien hautestontziak ez dira zigilatzen.",
                startSealNoteList:
                    "Zigilatu ixtean aukerarekin, itxitako bozketa itxita geratzen da: {{items}}.",
                startKeptChannels_one: "{{post}}: {{channels}} itxita geratzen da",
                startKeptChannels_other: "{{post}}: {{channels}} itxita geratzen dira",
                startKeptSealed:
                    "{{post}}: itxita geratzen da, bere hautestontziak zigilatuta daudelako",
                startNotEnabledList:
                    "Zigilatu ixtean aukerarekin, hasierak kanal bat gaitzen duten Postetan bakarrik irekitzen du: {{items}}.",
                startNotEnabledChannels_one:
                    "{{post}}: {{channels}} ez dago han gaituta eta Hasi gabe geratzen da",
                startNotEnabledChannels_other:
                    "{{post}}: {{channels}} ez daude han gaituta eta Hasi gabe geratzen dira",
                stopNeverOpenedPosts_one:
                    "{{names}}: ez da inoiz ireki; gelditzeak itxi egingo du eta bere hautestontzi hutsak zigilatuko ditu.",
                stopNeverOpenedPosts_other:
                    "{{names}}: ez dira inoiz ireki; gelditzeak itxi egingo ditu eta haien hautestontzi hutsak zigilatuko ditu.",
            },
            notifications: {
                generated: "Bozketa sortua",
                published: "Bozketa argitaratua",
                change_status: "Hauteskunde egoera aldatua",
            },
            sealRefusals: {
                startAgain:
                    "Bozketa ezin da berriro hasi: Zigilatu ixtean aukerarekin, itxi den bozketa itxita geratzen da eta bere hautestontziak zigilatuta daude.",
                startDisabled:
                    "Hasi Bozketa ez dago erabilgarri: hauteskunde honen hautestontziak zigilatuta daude edo zigilatzen ari dira, eta Zigilatu ixtean aukerarekin itxitako bozketa itxita geratzen da.",
                closedIsFinal:
                    "Itxitako bozketa ezin da aldatu: Zigilatu ixtean aukerarekin, itxitako bozketa itxita geratzen da.",
            },
        },
        emailEditor: {
            subject: "Email Gaia",
            tabs: {
                plaintext: "Testu Soilaren Gorputza",
                richtext: "Testu Aberatsaren Gorputza",
            },
        },
        sendCommunication: {
            send: "Bidali",
            title: "Bidali Jakinarazpena",
            subtitle: "Bidali jakinarazpen bat bozkatzaileei.",
            sendButton: "Bidali Jakinarazpena",
            voters: "Entzuleria",
            schedule: "Programatu",
            nowInput: "Orain bidali",
            dateInput: "Jakinarazpenak bidaltzen hasteko data eta ordua",
            chooseDate: "Mesedez aukeratu data bat",
            languages: "Hizkuntzak",
            smsMessage: "SMS Mezua",
            errorSending: "Errorea jakinarazpena bidaltzerakoan: {{error}}",
            successSending: "Jakinarazpena arrakastaz programatu/bidali da",
            method: "Txantiloi Metodoa",
            type: "Komunikazio Mota",
            alias: "Txantiloi Ezizena",
            votersSelection: {
                ALL_USERS: "Denak",
                NOT_VOTED: "Oraindik bozkatu ez dutenak",
                VOTED: "Jadanik bozkatu dutenak",
                SELECTED: "Hautatutako {{total}} {{voters}}",
            },
            path: {
                users: "erabiltzaileak",
                voters: "bozkatzaileak",
            },
            methodTitle: "Komunikazio Txantiloia",
            communicationMethod: {
                EMAIL: "Emaila",
                SMS: "SMS",
                WHATSAPP: "WhatsApp",
                VIBER: "Viber",
                MESSENGER: "Facebook Messenger",
            },
            communicationType: {
                CREDENTIALS: "Kredentzialak",
                BALLOT_RECEIPT: "Bozketa Jasoagiria",
            },
            email: {
                subject: "Gaia",
            },
        },
        tallysheet: {
            title: "Bozka-ontziak",
            subtitle: "Bozka-ontzi digitalizatuak kanalean",
            createTallySuccess: "Zenbaketa Orria gordea",
            createTallyError: "Errorea Zenbaketa Orria gordetzerakoan",
            createTallyErrorSameKindExists:
                "Kontaketa-orria dagoeneko existitzen da lehiaketa honetarako kanal eta eremu berarekin",
            allFieldsRequired: "Eremu guztiak beharrezkoak dira",
            header: {
                change: "Argitaratzeko Aldaketak",
                viewChange: "Ikusi Argitalpena",
                history: "Argitalpen Historia",
            },
            action: {
                start: "Hasi Hauteskundea",
                stop: "Gelditu Hauteskundea",
                pause: "Pausatu",
                generate: "Bersortu",
                publish: "Argitaratu Aldaketak",
                back: "Atzera",
            },
            inputError: {
                totalValidDoesNotMatch:
                    "Hautagaien botoek ({{candidateVotesSum}}) {{lowerBound}} eta {{upperBound}} artean egon behar dute lehiaketa honen bozketa-arauen arabera ({{nonBlankValidVotes}} baliozko boto ez-zuri × gehienez {{maxMarks}} marka boto-txartel bakoitzeko)",
                censusTooSmall:
                    "Boto guztien kopurua ({{totalVotes}}) ezin da erroldakoa ({{census}}) baino handiagoa izan",
                totalInvalidDoesNotMatch:
                    "Boto baliogabeen guztizkoak ({{totalInvalid}}) boto baliogabe inplizituen ({{implicitInvalid}}) eta boto baliogabe esplizituen ({{explicitInvalid}}) baturaren berdina izan behar du",
                totalVotesDoesNotMatch:
                    "Boto guztizkoak ({{totalVotes}}) boto baliodun guztizkoen ({{totalValidVotes}}) eta boto baliogabe guztizkoen ({{totalInvalid}}) baturaren berdina izan behar du",
                unknownCountingAlgorithm:
                    "Lehiaketa honen zenbaketa-algoritmoa ({{countingAlgorithm}}) ez da ezaguna, beraz ezin da zehaztu hautagaien botoen baimendutako kopurua. Egiaztatu lehiaketaren konfigurazioa.",
                blankBallotsInconsistent:
                    "Boto-txartel Zuriak balio berdina izan behar du ontzi honetako hautagaitza-orri guztietan",
                blankBallotsOutOfBounds:
                    "Boto-txartel Zuriak balioa ontzi honen hautagaitzako boto zurien kontaketek ezartzen duten tartetik kanpo dago",
            },
            label: {
                area: "Eremua",
                channel: "Kanala",
                total_votes: "Boto Guztiak",
                total_valid_votes: "Baliozko Boto Guztiak",
                total_invalid: "Baliogabeko Boto Guztiak",
                explicit_invalid: "Esplizituki Baliogabeko Botoak",
                implicit_invalid: "Inplizituki Baliogabeko Botoak",
                total_blank_votes: "Boto Zuriak",
                blank_ballots: "Boto-txartel Zuriak",
                census: "Zentso",
            },
            common: {
                tallyCeremony: {
                    manage: "Kudeatu Zenbaketa Zeremonia",
                    view: "Ikusi Zenbaketa Zeremonia",
                    cancel: "Ezeztatu Zenbaketa Zeremonia",
                    addKey: "Gehitu Zenbaketa Giltza",
                },
                edit: "Editatu",
                confirm: "Berretsi",
                back: "Atzera",
                next: "Hurrengoa",
                cancel: "Atzera",
                data: "Datuak",
                title: "Zenbaketa Orria",
                subtitle: "Zenbaketa Orri konfigurazioa.",
                candidates: "Hautagaiak",
                save: "Gorde",
                approve: "Onartu",
                disapprove: "Ez onartu",
                show: "Erakutsi",
                add: "Gehitu",
                versions: "Bertsioak",
                warningDisapprove: "Ziur zaude Kontaketa Orri hau ez onartzeko?",
                warningApprove: "Ziur zaude Kontaketa Orri hau onartzeko?",
            },
            empty: {
                header: "Ez dago Zenbaketa Orririk Oraindik.",
                action: "Sortu Zenbaketa Orria",
                add: "Gehitu",
            },
            breadcrumbSteps: {
                start: "Hasi",
                edit: "Editatu",
                confirm: "Berretsi",
                view: "Ikusi",
            },
            table: {
                area: "Eremua",
                contest: "Lehiaketa",
                approvedVersion: "Onartutako bertsioa",
                latestVersion: "Azken bertsioa",
                labels: "Etiketak",
                annotations: "Oharrak",
            },
            versionsTable: {
                title: "Hauntzaren bertsioak",
                version: "Bertsioa",
                createdBy: "Sortzailea",
                reviewedBy: "Berrikustatzailea",
                createdAt: "Sortze data",
                reviewedAt: "Berrikuste data",
                sourceImport: "Jatorrizko inportazioa",
                importStatus: "Inportazioaren egoera",
                openImport: "Ireki inportazioa",
                sourceFile: "Jatorrizko fitxategia",
            },
            message: {
                reviewError: "Errorea zenbaketa orria berrikusterakoan",
                reviewSuccess: "Zenbaketa orria berrikusia",
            },
        },
        application: {
            import: {
                title: "Inportatu aplikazioak",
                subtitle: "Inportatu aplikazioen datuak",
                paragraph:
                    "Inportatu aplikazioak Komaz Banandutako Balioen (CSV) formatuko kalkulu-orri fitxategia erabiliz. Deskargatu inportazio CSV fitxategiaren adibidea hemen.",
                messages: {
                    success: "Aplikazioak arrakastaz inportatuak",
                    error: "Errorea aplikazioak inportatzerakoan",
                },
            },
            export: {
                title: "Esportatu aplikazioak",
                subtitle: "Esportatu aplikazioen datuak",
                button: "Esportatu",
                paragraph:
                    "Esportatu aplikazioen datuak Komaz Banandutako Balioen (CSV) formatuko kalkulu-orri fitxategira.",
                messages: {
                    success: "Aplikazioak arrakastaz esportatuak",
                    error: "Errorea aplikazioak esportatzerakoan",
                },
            },
        },
        template: {
            noPermissions: "Ez duzu txantiloiak atzitzeko baimenik.",
            title: "Txantiloiak",
            subtitle: "Txantiloien zerrenda",
            chooseMethods: "Aukeratu Metodoak",
            default: "Erabili lehenetsitako txantiloia",
            empty: {
                title: "Ez dago Txantiloirik Oraindik",
                subtitle: "Bat sortu nahi duzu?",
            },
            action: {
                createOne: "Sortu Txantiloia",
            },
            create: {
                title: "Sortu Txantiloia",
                success: "Txantiloia sortua",
                error: "Errorea Txantiloia sortzerakoan",
            },
            update: {
                success: "Txantiloia eguneratua",
                error: "Errorea Txantiloia eguneratzerakoan",
            },
            edit: {
                title: "Editatu Txantiloia",
            },
            form: {
                smsMessage: "SMS Mezua",
                document: "Dokumentua",
                pdfOptions: "PDF Aukerak",
                reportOptions: "Txosten Aukerak",
                name: "Txantiloi Izena",
                alias: "Txantiloi Ezizena",
                type: "Mota",
                communicationMethod: "Metodoa",
            },
            type: {
                CREDENTIALS: "Kredentzialak",
                INITIALIZATION_REPORT: "Hasierako Txostena",
                ELECTORAL_RESULTS: "Hauteskunde Emaitzak",
                BALLOT_IMAGES: "Bozketa Irudiak",
                BALLOT_RECEIPT: "Bozketa Jasoagiria",
                ACTIVITY_LOGS: "Jarduera Egunkariak",
                MANUAL_VERIFICATION: "Eskuzko Egiaztapena",
                PARTICIPATION_REPORT: "Parte-hartze Txostena",
            },
            method: {
                email: "Emaila",
                sms: "SMS",
                document: "Dokumentua",
                whatsapp: "WhatsApp",
                viber: "Viber",
                messenger: "Facebook Messenger",
            },
            import: {
                title: "Inportatu Txantiloiak",
                subtitle: "Inportatu txantiloien datuak",
                paragraph:
                    "Inportatu txantiloiak Komaz Banandutako Balioen (CSV) formatuko kalkulu-orri fitxategia erabiliz. Deskargatu inportazio CSV fitxategiaren adibidea hemen.",
            },
        },
        materials: {
            audioInstructions: {
                screenLabel: "Audio-argibideak pantaila honetarako",
                languageLabel: "Grabazioaren hizkuntza",
                none: "Ez dira audio-argibideak",
                helperText:
                    "Boto-emaileek fitxategi hau entzuten dute pantaila horretan argibideak eskatzean.",
                screens: {
                    "election-chooser": "Hauteskundeen zerrenda",
                    "start": "Hasiera",
                    "ballot": "Boto-papera",
                    "review": "Berrikuspena",
                    "confirmation": "Baieztapena",
                    "audit": "Ikuskapena",
                    "ballot-locator": "Boto-paperen bilatzailea",
                    "support-materials": "Laguntza-materialak",
                },
            },
            createMaterialSuccess: "Laguntza materiala sortua",
            createMaterialError: "Errorea laguntza materiala sortzerakoan",
            updateMaterialSuccess: "Laguntza materiala eguneratua",
            updateMaterialError: "Errorea laguntza materiala eguneratzerakoan",
            common: {
                title: "Laguntza Materiala",
                subtitle: "Sartu laguntza material datuak.",
            },
            error: {
                title: "Izenburua beharrezkoa da",
                document: "Dokumentua beharrezkoa da",
            },
            fields: {
                isHidden: "Ezkutatuta dago",
                publicUrl: "URL Publikoa",
            },
            empty: {
                header: "Ez dago laguntza materialak",
                action: "Laguntza materiala sortu",
            },
        },
        widget: {
            logs: "Egunkariak",
        },
        settings: {
            countries: {
                title: "Herrialde Blokeoa",
                votingDescription: "Aukeratu behean bozketa blokeatu nahi dituzun herrialdeak.",
                enrollmentDescription:
                    "Aukeratu behean matrikula blokeatu nahi dituzun herrialdeak.",
                error: {
                    errorSaving: "Errorea herrialde zerrenda gordetzerakoan",
                },
            },
            backupRestore: {
                title: "Babeskopia / Leheneratu Maizter konfigurazioa",
                backup: {
                    label: "Babeskopia",
                    subtitle: "Maizter konfigurazioen babeskopia",
                },
                restore: {
                    label: "Leheneratu",
                    subtitle: "Leheneratu Maizter konfigurazioa",
                    title: "Inportatu Maizter Konfigurazioak",
                    paragraph:
                        "Inportatu maizter konfigurazioak, Keycloak konfigurazioak, rol eta baimen datuak zip karpeta erabiliz",
                    tenantConfigOption: "Inportatu Maizter Konfigurazioak",
                    keycloakConfigOption: "Inportatu Keycloak Konfigurazioak",
                    RolesConfigOption: "Inportatu Rol eta Baimen Konfigurazioak",
                },
            },
            previewScreen: {
                label: "Aurreikuspenak",
                noContent: "Ez da aurreikuspenik aurkitu",
                table: {
                    title: "Kanpoko aurrebistak",
                    description:
                        "Kanpoko eskaeren bidez sortutako hautestontzi-estiloen aurrebisten erregistroa",
                    requestedBy: "Eskatzailea",
                    document: "Dokumentua",
                    url: "URLa",
                },
            },
            languages: {
                default: "Lehenetsitako hizkuntza",
            },
        },
        approvalsScreen: {
            column: {
                status: "Egoera",
                id: "Eskaeraren IDa",
                applicantId: "Eskatzailearen IDa",
                verificationType: "Egiaztapena",
                createdAt: "Eskatua",
                verified_by: "Egiaztatzailea",
                voter: "Bozkatzailea",
                what: "Zer gertatu den",
                post: "Postua",
                when: "Noiz",
            },
            status: {
                PENDING: "Berrikusteko",
                ACCEPTED: "Onartua",
                REJECTED: "Baztertua",
            },
            verification: {
                AUTOMATIC: "Automatikoa",
                MANUAL: "Eskuzkoa",
            },
            time: {
                minutes_one: "minutu {{count}}",
                minutes_other: "{{count}} minutu",
                hours_one: "ordu {{count}}",
                hours_other: "{{count}} ordu",
                days_one: "egun {{count}}",
                days_other: "{{count}} egun",
            },
            summary: {
                join: "{{head}} eta {{last}}",
                differs_one: "{{fields}} ez dator bat erregistroarekin",
                differs_other: "{{fields}} ez datoz bat erregistroarekin",
                typedByHand:
                    "Datuak eskuz idatzita daude, ez dira eskaneatutako agiri batetik irakurri",
                needsFaceToFace: "Aurrez aurreko egiaztapena behar du",
                scanVerified: "Eskaneatutako agiria egiaztatuta",
                noVoter: "Ez da bozkatzailerik aurkitu erregistroan",
                allMatch: "Datu guztiak bat datoz erregistroarekin",
                needsReview: "Pertsona batek erabaki zain",
                approvedBy: "Onartzailea: {{name}}",
                approvedAuto: "Automatikoki onartua",
                rejectedBy: "Baztertzailea: {{name}}",
                rejectedAuto: "Automatikoki baztertua",
            },
            list: {
                title: "Onarpenak",
                subtitle:
                    "Arauek beren kabuz erabaki ezin dituzten izen-emateak hemen daude pertsona baten zain.",
                search: "Bilatu",
                review: "Berrikusi izen-ematea",
                openRecord: "Ireki izen-ematea",
                seeRule: "Ikusi erabaki zuen araua",
                unnamed: "Izenik gabeko eskatzailea",
                waiting: "Zain: {{time}}",
                applied: "Eskaera-data: {{date}}",
                empty: {
                    title: "Ez dago ezer hemen",
                    text: "Egoera hau duten izen-emateak hemen agertuko dira. Saiatu beste bilaketa edo egoera batekin.",
                },
            },
            flow: {
                stepsLabel: "Berrikuspenaren urratsak",
                steps: {
                    identity: "Egiaztatu identitatea",
                    voter: "Bilatu bozkatzailea",
                    decide: "Erabaki",
                },
                continue: "Jarraitu",
                backToList: "Itzuli Onarpenetara",
                identity: {
                    details: "Izen-emateko datuak",
                    confirm:
                        "Bozkatzailearen identifikazio agiria aurrez aurre edo bideo-deiz egiaztatu dut, eta bat dator izen-emate honekin.",
                    checked: "Aurrez aurreko egiaztapena berretsita",
                    notChecked: "Aurrez aurreko egiaztapena oraindik berretsi gabe",
                },
                voter: {
                    none: "Hauetako bat ere ez da bozkatzailea",
                    noneHint:
                        "Orduan izen-ematea baztertu baino ezin da egin, bat datorren bozkatzailerik ez dagoelako.",
                    noneChosen: "Hauetako bat ere ez da bozkatzailea",
                    notChosen: "Oraindik ez da bozkatzailerik aukeratu",
                },
                decide: {
                    approve: "Onartu",
                    reject: "Baztertu",
                    approveText:
                        "Lotu izen-emate hau erregistroko bozkatzaile honekin: {{voter}}. Bozkatzaileari posta elektronikoz edo SMS bidez jakinarazten zaio, eta bozketa irekitzen denean saioa hasi ahal izango du botoa emateko.",
                    rejectText: "Bozkatzaileari arrazoia esaten zaio. Hau ezin da desegin.",
                    chooseVoter: "Onartzeko, aukeratu bat datorren bozkatzailea 2. urratsean.",
                    noVoter:
                        "Ez duzu bat datorren bozkatzailerik aurkitu, beraz izen-emate hau baztertu baino ezin da egin.",
                    enrolled: "Aukeratutako bozkatzaileak dagoeneko izena emanda du.",
                    faceToFace: "Onartzeko, berretsi aurrez aurreko egiaztapena 1. urratsean.",
                },
            },
            review: {
                loadError: "Ezin izan da izen-ematea kargatu.",
                applied: "Eskaera-data: {{date}}",
                waiting: "Zain: {{time}}",
                whyTitle: "Zergatik behar den pertsona bat",
                decisionTitle: "Nola erabaki zen",
                rule: "Matrizearen {{version}}. bertsioko {{rule}}. araua",
                ruleLast: "Matrizearen {{version}}. bertsioko azken araua",
                seeRule: "Ikusi araua",
                why: {
                    typedByHand:
                        "Bozkatzaileak bere datuak eskuz idatzi ditu, identifikazio agiri bat eskaneatu beharrean. Horrelako izen-emateak ez dira inoiz automatikoki onartzen: lehenik funtzionario batek berresten du nor den.",
                    differs_one:
                        "Datu bat ez dator bat erregistroarekin: {{details}}. Onarpen-arauek pertsona batek izen-emate hau egiaztatzea eskatzen dute.",
                    differs_other:
                        "{{count}} datu ez datoz bat erregistroarekin: {{details}}. Onarpen-arauek pertsona batek izen-emate hau egiaztatzea eskatzen dute.",
                    differsFields_one:
                        "Datu bat ez dator bat erregistroarekin: {{fields}}. Onarpen-arauek pertsona batek izen-emate hau egiaztatzea eskatzen dute.",
                    differsFields_other:
                        "{{count}} datu ez datoz bat erregistroarekin: {{fields}}. Onarpen-arauek pertsona batek izen-emate hau egiaztatzea eskatzen dute.",
                    difference:
                        "{{field}}: izen-ematean “{{enrollment}}” dago eta erregistroan “{{registry}}”",
                    noVoter:
                        "Erregistroko bozkatzaile batek ere ez ditu datu hauek. Onarpen-arauek pertsona batek izen-emate hau egiaztatzea eskatzen dute.",
                    severalVoters:
                        "Erregistroko bozkatzaile bat baino gehiago dator bat izen-emate honekin. Pertsona batek aukeratzen du zuzena.",
                    pending:
                        "Onarpen-arauek pertsona batek izen-emate hau egiaztatzea eskatzen dute.",
                    unknown: "Izen-emate hau pertsona batek erabaki zain dago.",
                    approvedAuto:
                        "Onarpen-arauek automatikoki onartu dute izen-emate hau. Eskatzen dituzten egiaztapen guztiak gainditu dira.",
                    approvedBy: "{{name}} erabiltzaileak onartu du izen-emate hau. Data: {{date}}.",
                    rejectedAuto:
                        "Onarpen-arauek automatikoki baztertu dute izen-emate hau: {{reason}}.",
                    rejectedBy:
                        "{{name}} erabiltzaileak baztertu du izen-emate hau. Data: {{date}}. Arrazoia: {{reason}}.",
                },
                registryHelp:
                    "Datu berdinak dituzten bozkatzaileak bilatu ditugu: {{fields}}. Aukeratu izen-emate hau norena den.",
                registrySearching:
                    "Hauek dira zure bilaketarekin bat datozen erregistroko bozkatzaileak. Aukeratu izen-emate hau norena den.",
                registrySearch:
                    "Ez dago zerrendan? Bilatu erregistroan izenaren edo helbide elektronikoaren arabera",
                registryLoading: "Erregistroan bilatzen",
                registryError: "Ezin izan da erregistroan bilatu.",
                noCandidates:
                    "Erregistroko bozkatzaile bat ere ez dator bat. Saiatu izenaren edo helbide elektronikoaren arabera bilatzen.",
                candidates: "Erregistroko bozkatzaileak",
                alreadyEnrolled: "Dagoeneko izena emanda",
                bestMatch: "Bat-etortze onena",
                detailsMatch: "{{total}} datutik {{count}} bat datoz",
                compareTitle: "Erregistroko {{name}} bozkatzailearekin alderatuta",
                col: {
                    detail: "Datua",
                    enrollment: "Izen-ematean",
                    registry: "Erregistroan",
                    result: "Emaitza",
                },
                same: "Berdina",
                differs: "Desberdina",
                compareNote:
                    "Izenetan ez dira kontuan hartzen maiuskulak, azentuak eta marratxoak.",
                compareJoint:
                    "Gidabaimenetan eta itsasgizon-liburuetan, izena eta bigarren izena batera alderatzen dira.",
                applicationId: "Eskaeraren IDa",
                copy: "Kopiatu",
                copied: "Kopiatuta",
                approve: "Onartu izen-ematea",
                approveDialog: {
                    title: "{{name}} onartu?",
                    body: "Honek izen-ematea beheko erregistroko bozkatzailearekin lotzen du. Bozkatzaileari posta elektronikoz edo SMS bidez jakinarazten zaio, eta bozketa irekitzen denean saioa hasi ahal izango du botoa emateko.",
                    checked: "Bozkatzailearen identifikazio agiria aurrez aurre egiaztatu duzu.",
                    irreversible: "Hau ezin da desegin.",
                    confirm: "Onartu",
                },
                reject: "Baztertu izen-ematea",
            },
            idCheck: {
                title: "Agiriaren egiaztapena",
                method: {
                    VERIFIED: "Eskaneatutako agiria egiaztatuta",
                    MANUAL_ENTRY: "Eskuz idatzita",
                    UNKNOWN: "Ez da adierazi",
                },
                verified: "Izen-emate prozesuak bozkatzailearen identifikazio agiria egiaztatu du",
                typedByHand: "Bozkatzaileak bere datuak eskuz idatzi ditu",
                unknown: "Izen-emate prozesuak ez du adierazi identitatea nola egiaztatu den",
                faceToFaceTitle: "Egiaztatu bozkatzailea aurrez aurre onartu aurretik",
                faceToFaceText:
                    "Elkartu bozkatzailearekin aurrez aurre edo bideo-deiz, eta alderatu haren identifikazio agiria orri honetako datuekin.",
            },
            reject: {
                rejectReason: "Baztertzeko arrazoia",
                message: "Bozkatzailearentzako mezua",
                messageRequired: "Idatzi mezu bat bozkatzailearentzat arrazoia Bestelakoa denean.",
                reasons: {
                    "undefined": "-",
                    "insufficient-information": "Datuak falta dira",
                    "no-matching-voter": "Ez dago bat datorren bozkatzailerik",
                    "voter-already-approved": "Dagoeneko onartua",
                    "other": "Bestelakoa",
                },
                hint: {
                    "insufficient-information": "Datuak falta dira edo ezin dira irakurri.",
                    "no-matching-voter": "Pertsona ez dago bozkatzaileen erregistroan.",
                    "voter-already-approved": "Bozkatzaile honek dagoeneko izena emanda du.",
                    "other": "Idatzi zure mezua.",
                },
                preview: {
                    "insufficient-information":
                        "Ezin izan dugu zure izen-ematea egin, zure datu batzuk falta direlako edo ezin direlako irakurri. Eman izena berriro datu osoekin.",
                    "no-matching-voter":
                        "Ez dugu erregistroan zure datuekin bat datorren bozkatzailerik aurkitu. Egiaztatu zure datuak eta eman izena berriro, edo jarri harremanetan zure hauteskunde-bulegoarekin.",
                    "voter-already-approved":
                        "Dagoeneko izena emanda duzu. Bozketa irekitzen denean saioa hasi ahal izango duzu botoa emateko.",
                },
                previewTitle: "Bozkatzaileak hau ikusiko du",
            },
            notifications: {
                approveError: "Ezin izan da izen-ematea onartu",
                approveSuccess: "{{name}}: onartua. Bozkatzaileari jakinarazi zaio.",
                rejectError: "Ezin izan da izen-ematea baztertu",
                rejectSuccess: "{{name}}: baztertua. Bozkatzaileari jakinarazi zaio.",
                VoterApprovedAlready: "Bozkatzaile honek dagoeneko izena emanda du.",
            },
            export: {
                success: "Eskaeren esportazioa ondo amaitu da",
                error: "Errorea eskaerak esportatzean",
            },
            matrix: {
                button: "Onarpen matrizea",
                title: "Onarpen matrizea",
                back: "Onarpenak",
                subtitle:
                    "Arauek erabakitzen dute izen-emate bakoitzarekin zer gertatzen den. Betetzen den lehen arauak erabakitzen du.",
                versionChip: "{{version}}. bertsioa",
                savedBy: "{{user}} erabiltzaileak gordea, {{date}}",
                builtIn: "Arau integratuak, bertsio bat gorde arte erabiltzen dira",
                unsaved: "Gorde gabeko aldaketak",
                viewOnly: "Ikusteko soilik",
                readOnlyTitle: "Arauak ikus ditzakezu, baina ezin dituzu aldatu",
                readOnlyText:
                    "Eskatu approval-matrix-write baimena duen administratzaile bati aldaketak egiteko.",
                loadError: "Ezin izan da onarpen matrizea kargatu.",
                compared: "Zer alderatzen dugun",
                comparedHelp:
                    "Izen-emate bakoitza erregistroan aurkitutako bozkatzailearekin alderatzen da. Izenetan ez dira kontuan hartzen maiuskulak, azentuak eta marratxoak; gidabaimenetan eta itsasgizon-liburuetan, izena eta bigarren izena batera alderatzen dira.",
                addCompared: "Alderatu beste datu bat",
                rules: "Arauak",
                rulesHelp:
                    "Arauak goitik behera egiaztatzen dira. Betetzen den lehenak erabakitzen du; bat ere betetzen ez bada, azken araua aplikatzen da.",
                when: "Noiz",
                then: "Orduan",
                otherwise: "Bestela",
                noneApply: "Goiko arauetako bat ere ez da aplikatzen",
                andWord: "eta",
                and: " eta ",
                appliesToExample: "Zure adibideari aplikatzen zaio",
                cameFrom: "Zatozen izen-ematea erabaki zuen",
                voterIsTold: "Bozkatzaileari hau esaten zaio: “{{reason}}”.",
                sentence: "Baldintza hauek betetzen direnean: {{when}}; {{outcome}}.",
                sentenceOtherwise: "Goiko arauetako bat ere aplikatzen ez bada, {{outcome}}.",
                sentenceEmpty: "Gehitu baldintza bat arau hau noiz aplikatzen den adierazteko.",
                addRule: "Gehitu araua",
                discard: "Baztertu aldaketak",
                actions: {
                    edit: "Editatu {{number}}. araua",
                    editOtherwise: "Editatu azken araua",
                    moveUp: "Igo {{number}}. araua",
                    moveDown: "Jaitsi {{number}}. araua",
                    delete: "Ezabatu {{number}}. araua",
                },
                saveBar: {
                    title: "Gorde gabeko aldaketak dituzu",
                    fix_one: "Zuzendu arau 1 gorde aurretik",
                    fix_other: "Zuzendu {{count}} arau gorde aurretik",
                    more: "+{{count}} gehiago",
                },
                test: "Probatu adibide bat",
                testHelp:
                    "Deskribatu izen-emate bat zein arauk erabakitzen duen ikusteko. Gorde gabeko aldaketak ere kontuan hartzen dira.",
                testDetails: "Alderatutako datuak",
                applies: "{{number}}. araua aplikatzen da",
                otherwiseApplies: "Azken araua aplikatzen da",
                testError: "Ezin izan da adibidea probatu.",
                testInvalid: "Zuzendu arau hauek adibide bat probatzeko:",
                ruleError: "{{number}}. araua: {{error}}",
                invariants: {
                    MANUAL_ENTRY_NOT_ACCEPTED:
                        "Eskuz idatzitako identitatea ez da inoiz automatikoki onartzen, beraz hau pertsona bati bidaltzen zaio.",
                    ALREADY_ENROLLED_NOT_ACCEPTED:
                        "Dagoeneko izena emanda duen bozkatzailea ez da inoiz berriro onartzen.",
                    NO_VOTER_NOT_ACCEPTED: "Ez da inor onartzen erregistroan bozkatzailerik gabe.",
                    OTHERWISE_NOT_ACCEPTED: "Azken arauak ez du inoiz onartzen.",
                },
                dialog: {
                    editTitle: "Editatu {{number}}. araua",
                    newTitle: "Arau berria",
                    otherwiseTitle: "Editatu azken araua",
                    summary: "Laburbilduz",
                    whenHelp:
                        "Hauek guztiak bete behar dira. Utzi baldintza bat kanpoan garrantzirik ez duenean.",
                    otherwiseHelp: "Goiko arauetako bat ere aplikatzen ez bada",
                    addCondition: "Gehitu baldintza",
                    remove: "Kendu “{{condition}}”",
                    identity: "Identitatearen egiaztapena",
                    voterFound: "Bozkatzailea erregistroan",
                    alreadyEnrolled: "Dagoeneko izena emanda",
                    validId: "Agiri mota",
                    differing: "Desberdinak diren datuak",
                    decision: "Erabakia",
                    reason: "Bozkatzaileari zer esaten zaion",
                    voterSees: "Bozkatzaileak hau ikusten du",
                    apply: "Aplikatu",
                    close: "Itxi",
                    yes: "Bai",
                    no: "Ez",
                    notReported: "Ez da adierazi",
                },
                identity: {
                    VERIFIED: "Eskaneatutako agiriarekin egiaztatua",
                    MANUAL_ENTRY: "Eskuz idatzia",
                },
                differing: {
                    none: "Bat ere ez",
                    exactly_1: "Zehazki 1",
                    at_most_1: "Gehienez 1",
                    exactly_2: "Zehazki 2",
                    at_most_2: "Gehienez 2",
                    at_least_3: "3 edo gehiago",
                },
                fieldMatch: {
                    MATCHES: "Berdina",
                    DIFFERS: "Desberdina",
                },
                decisions: {
                    ACCEPTED: "Onartu automatikoki",
                    PENDING: "Bidali pertsona bati",
                    REJECTED: "Baztertu",
                },
                outcomeShort: {
                    ACCEPTED: "automatikoki onartu",
                    PENDING: "pertsona bati bidali",
                    REJECTED: "baztertu",
                },
                outcomeHelp: {
                    ACCEPTED: "Bozkatzaileak izena emanda geratzen da, inork begiratu gabe.",
                    PENDING:
                        "Funtzionario batek erabakitzen du, eta bozkatzaileari esaten zaio izen-ematea berrikusten ari direla.",
                    REJECTED: "Bozkatzaileari arrazoia esaten zaio, eta berriro eman dezake izena.",
                },
                outcomeSentence: {
                    ACCEPTED: "izen-ematea automatikoki onartzen da",
                    PENDING: "izen-ematea pertsona bati bidaltzen zaio",
                    REJECTED: "izen-ematea baztertzen da",
                },
                reasons: {
                    NO_VOTER: "Ez dago bat datorren bozkatzailerik",
                    ALREADY_APPROVED: "Dagoeneko onartua",
                    INSUFFICIENT_INFORMATION: "Datuak falta dira",
                    IDENTITY_NOT_VERIFIED: "Identitatea ez dago egiaztatuta",
                    OTHER: "Bestelakoa",
                },
                voterText: {
                    NO_VOTER:
                        "Ez dugu erregistroan zure datuekin bat datorren bozkatzailerik aurkitu. Egiaztatu zure datuak eta eman izena berriro, edo jarri harremanetan zure hauteskunde-bulegoarekin.",
                    ALREADY_APPROVED:
                        "Dagoeneko izena emanda duzu. Bozketa irekitzen denean saioa hasi ahal izango duzu botoa emateko.",
                    INSUFFICIENT_INFORMATION:
                        "Ezin izan dugu zure izen-ematea egin, zure datu batzuk falta direlako edo ezin direlako irakurri. Eman izena berriro datu osoekin.",
                    IDENTITY_NOT_VERIFIED:
                        "Ezin izan dugu zure identitatea automatikoki egiaztatu, beraz hauteskunde-funtzionario batek berrikusiko du zure izen-ematea.",
                    OTHER: "Hauteskunde-funtzionario batek idazten du mezu hau erabakitzen duenean.",
                },
                conditions: {
                    any: "Oraindik ez dago baldintzarik",
                    identity: {
                        VERIFIED: "Identitatea eskaneatutako agiriarekin egiaztatuta",
                        MANUAL_ENTRY: "Identitatea eskuz idatzita",
                    },
                    voterFound: {
                        true: "Bozkatzailea erregistroan aurkitu da",
                        false: "Ez da bozkatzailerik aurkitu erregistroan",
                    },
                    alreadyEnrolled: {
                        true: "Dagoeneko izena emanda",
                        false: "Oraindik izena eman gabe",
                    },
                    validId: "Agiria: {{id}}",
                    differing: {
                        none: "Datu guztiak bat datoz",
                        exactly_1: "Zehazki datu 1 desberdina da",
                        at_most_1: "Gehienez datu 1 desberdina da",
                        exactly_2: "Zehazki 2 datu desberdinak dira",
                        at_most_2: "Gehienez 2 datu desberdinak dira",
                        at_least_3: "3 datu edo gehiago desberdinak dira",
                    },
                    field: {
                        MATCHES: "{{field}} bat dator",
                        DIFFERS: "{{field}} desberdina da",
                    },
                },
                errors: {
                    ACCEPTS_MANUAL_ENTRY:
                        "Identitatea eskuz idatzita duten izen-emateak ezin dira automatikoki onartu.",
                    ACCEPTS_ALREADY_ENROLLED:
                        "Dagoeneko izena emanda duen bozkatzailea ezin da berriro onartu.",
                    ACCEPTS_WITHOUT_VOTER:
                        "Izen-emate bat ezin da onartu erregistroan bozkatzailerik gabe.",
                    OTHERWISE_ACCEPTS:
                        "Azken arauak izen-emateak pertsona bati bidali edo baztertu ditzake, baina ez onartu.",
                    MISSING_REASON: "Aukeratu bozkatzaileari zer esaten zaion.",
                    UNEXPECTED_REASON: "Onarpen batek ez du arrazoirik.",
                    NO_COMPARED_FIELDS: "Aukeratu gutxienez datu bat erregistroarekin alderatzeko.",
                    DUPLICATE_COMPARED_FIELD: "Alderatutako datu bat errepikatuta dago.",
                    UNKNOWN_FIELD: "Arau batek alderatzen ez den datu bat erabiltzen du.",
                    NO_CONDITIONS:
                        "Gehitu gutxienez baldintza bat. Azken araua bakarrik aplikatzen zaio gainerako guztiari.",
                },
                change: {
                    added: "{{number}}. araua gehitu da",
                    decision: "{{number}}. araua: {{from}} → {{to}}",
                    edited: "{{number}}. araua aldatu da",
                    removed: "Arau bat kendu da ({{text}})",
                    moved: "Arauen ordena aldatu da",
                    otherwise: "Azken araua aldatu da",
                    compared: "Alderatutako datuak aldatu dira",
                },
                save: {
                    button: "Gorde {{version}}. bertsio gisa",
                    title: "{{version}}. bertsio gisa gorde?",
                    body: "Hemendik aurrera izen-emate berriak arau hauekin erabakitzen dira. Dagoeneko erabakitako izen-emateek beren erabakia mantentzen dute.",
                    changes: "Zer aldatu den",
                    log: "Bertsio berria hauteskunde-egunkarian jasotzen da.",
                    confirm: "Gorde {{version}}. bertsioa",
                    success: "{{version}}. bertsio gisa gorde da",
                    error: "Ezin izan da onarpen matrizea gorde",
                },
            },
        },
        monitoring: {
            title: "Monitorizazio-panelak",
            loading: "Monitorizazioa kargatzen",
            unavailableAlert:
                "Ezin izan dira monitorizazio-panelak kargatu; panel estandarra erakusten da.",
            noDashboards: "Gertaera honek ez du monitorizazio-panelik.",
            dashboardFailed: "Ezin izan da monitorizazio-panela kargatu.",
            dashboardInvalid: "Panel hau ezin da erakutsi: {{problem}}",
            retry: "Saiatu berriro",
            errors: {
                busy: "Zerbitzaria lanpetuta dago. Segundo batzuk barru saiatuko da berriro.",
                forbiddenScope:
                    "Ezin duzu eskualde, Post edo herrialde hau ikusi. Aukeratu beste bat.",
                snapshotPruned:
                    "Erakutsitako eguneratzea ez da gehiago gordetzen. Panelak azken eguneratzea erakusten du orain: esportatu berriro hura erabiltzeko.",
                checksUnavailable: "Grafikoen zerbitzua ez dago erabilgarri orain. Saiatu geroago.",
                lockedDown: "Hauteskunde-gertaera blokeatuta dago; beraz, hau ezin da aldatu.",
                notFound: "Panel edo widget hau ez dago konfiguratuta jada. Kargatu berriro orria.",
                badRequest: "Eskaera ez da onartu. Kargatu berriro orria eta saiatu berriro.",
                conflict:
                    "Beste norbaitek aldaketa bat gorde du lehenago. Kargatu berriro eta saiatu berriro.",
                invalid: "Esportazioaren balio batzuk ez dira onartzen.",
                unknown: "Zerbaitek huts egin du. Saiatu geroago.",
            },
            header: {
                dashboard: "Panela",
                updated: "Eguneratua: {{time}} ({{timeZone}})",
                notUpdated: "Oraindik zenbatu gabe",
                refresh: "{{seconds}} s-ro",
                export: "Esportatu",
                editDashboard: "Editatu panela",
                preset: "Panelen aurrezarpena",
                reload: "Bilatu datu berriak",
            },
            footer: {
                dataThrough: "Datuak {{time}} arte ({{timeZone}})",
            },
            selectors: {
                region: "Eskualdea",
                post: "Postua",
                country: "Herrialdea",
                allRegions: "Eskualde guztiak",
                allPosts: "Postu guztiak",
                allAuthorizedPosts: "Baimendutako postu guztiak",
                allCountries: "Herrialde guztiak",
                authorizedOnly: "{{all}} (baimenduak)",
            },
            widget: {
                menu: "{{widget}} widgetaren ekintzak",
                configure: "Konfiguratu widgeta",
                viewData: "Ikusi datuak",
                export: "Esportatu",
                duplicate: "Bikoiztu",
                loading: "{{widget}} kargatzen",
                missing: "Panelak existitzen ez den widget bat aipatzen du: {{id}}",
                updating: "{{widget}} eguneratzen",
                updatingNote: "Eguneratzen: erakutsitako grafikoa aurrekoa da.",
            },
            frame: {
                title: "{{widget}} grafikoa",
            },
            sources: {
                voter_turnout: "Parte-hartzea",
                test_voting: "Proba-bozketa",
                enrollment_decisions: "Izen-emate erabakiak",
                voting_credentials: "Bozketa-kredentzialak",
                poll_status: "Bozketaren egoera",
                final_testing_lockdown: "Azken probak eta blokeoa",
                counting_transmission: "Zenbaketa eta transmisioa",
                voting_enrollment_activity: "Bozketa eta izen-emate jarduera",
                access_security: "Sarbidea eta segurtasuna",
                attack_detections: "Erasoen detekzioa",
                helpdesk: "Laguntza",
            },
            reasons: {
                TEST_ELECTION_DESIGNATION: "oraindik ezin dira proba-hauteskundeak markatu",
                CREDENTIAL_ISSUED_EVENT: "kredentzialen igorpena ez da oraindik erregistratzen",
                FINAL_TESTING_LOCKDOWN_STATE:
                    "azken probak eta blokeoa ez dira oraindik erregistratzen",
                ATTACK_DETECTION_FEED: "ez dago erasoak detektatzeko iturririk konektatuta",
                HELPDESK_INTEGRATION: "ez dago laguntza-sistemarik konektatuta",
            },
            notices: {
                UNREGISTERED_ATTEMPTS_AT_EVENT_SCOPE_ONLY:
                    "Erregistratu gabeko erabiltzaile-izenen saiakerak ez dira inongo posturenak, eta, beraz, gertaera osorako soilik zenbatzen dira.",
                UNREGISTERED_ATTEMPTS_EXCLUDED:
                    "Zifra hauek ez dituzte erregistratu gabeko erabiltzaile-izenen saiakerak barne hartzen; gertaera osorako zenbatzen dira.",
                CREDENTIALS_ISSUED_WHEN_PASSWORD_SET:
                    "Kredentzialak emandakotzat hartzen dira hauteslearen pasahitza ezartzen denean, plataformak ematea erregistratu arte.",
                CONFIG_NEWER_THAN_SNAPSHOT:
                    "Azken ezarpenekin marraztua; zifrak hurrengo zenbaketan zenbatzen dira haiekin.",
                CONFIG_AT_SNAPSHOT_UNAVAILABLE:
                    "Uneko ezarpenekin marraztua: zifrak zenbatzeko erabili ziren ezarpenak ez dira gordetzen.",
            },
            unavailable: {
                notConnected: "Konektatu gabe · {{reason}}",
                notConnectedHelp: "Ez da ezer erakusten konektatu arte.",
                unknownReason: "datu-iturria ez dago erabilgarri",
                noSnapshot: "Oraindik zenbatu gabe",
                noSnapshotHelp:
                    "Lehen zenbaketa ez da amaitu. Widget hau bere kabuz eguneratzen da.",
                scopePending: "Hautapen hau zenbatzen",
                scopePendingHelp:
                    "Hautapen hau hurrengo pasaldian zenbatzen da, minutu bat inguruan.",
                settingsPending: "Ezarpen berriekin zenbatzen…",
                settingsPendingHelp:
                    "Zifrak gordetako ezarpenekin zenbatzen dira berriro, minutu bat inguruan.",
                renderFailed: "Ezin izan da grafikoa marraztu",
                renderFailedHelp: "Horren ordez, zifrak erakusten dira.",
                invalid: "Widget hau ezin da erakutsi",
                invalidHelp: "Bere konfigurazioak arazo bat du.",
                requestFailed: "Ezin izan da widget hau kargatu",
                requestFailedHelp: "Berriro saiatzen da panela eguneratzean.",
            },
            dataTable: {
                title: "{{widget}} · datuak",
                close: "Itxi",
                empty: "Errenkadarik ez",
                rowsPerPage: "Errenkadak orriko:",
                shownRows: "{{from}}–{{to}} / {{total}}",
                firstPage: "Lehen orria",
                previousPage: "Aurreko orria",
                nextPage: "Hurrengo orria",
                lastPage: "Azken orria",
            },
            columns: {
                numerator: "Zenbakitzailea",
                denominator: "Izendatzailea",
                pct: "Ehunekoa",
                pct_label: "Erakutsitako ehunekoa",
                group: "Taldea",
                group_key: "Taldearen gakoa",
                post: "Postua",
                post_id: "Postuaren IDa",
                region: "Eskualdea",
                country: "Herrialdea",
                reason: "Arrazoia",
                category: "Kategoria",
                state: "Egoera",
                state_label: "Erakutsitako egoera",
                bucket_start: "Aldiaren hasiera",
                bucket_label: "Aldia",
                bucket_utc: "Aldiaren hasiera (UTC)",
                measure: "Neurria",
                label: "Etiketa",
                value: "Balioa",
            },
            measures: {
                registered: "Erroldatuak",
                pre_enrolled: "Aurrez izena emandakoak",
                credentials_issued: "Emandako kredentzialak",
                test_voted: "Probako botoak",
                voted: "Bozkatu dute",
                voted_pre_enrolled: "Aurrez izena emanda bozkatu dute",
                applications: "Eskaerak",
                pending: "Zain",
                approved: "Onartuak",
                disapproved: "Ukatuak",
                posts: "Postuak",
                initialized: "Hasieratuak",
                opened: "Irekiak",
                paused: "Etenda",
                closed: "Itxiak",
                tested: "Probatuak",
                locked_down: "Blokeatuak",
                tallied: "Zenbatuak",
                transmitted: "Bidaliak",
                transmission_failed: "Bidalketak huts egin du",
                logins: "Saio-hasierak",
                login_failures: "Huts egindako saio-hasierak",
                login_failures_valid_user: "Hutsak, baliozko erabiltzailea",
                login_failures_unregistered: "Hutsak, erregistratu gabeko erabiltzailea",
                password_resets: "Pasahitz-berrezarpenak",
                password_reset_requests: "Pasahitza berrezartzeko eskaerak",
                detections: "Detekzioak",
                issues: "Gorabeherak",
                pending_issues: "Zain dauden gorabeherak",
            },
            export: {
                title: "Esportatu monitorizazio-datuak",
                format: "Formatua",
                csv: "CSV",
                sql: "SQL",
                from: "Noiztik",
                to: "Noiz arte",
                timeZoneHelp:
                    "Orduak {{timeZone}} ordu-eremuan daude. Guztizkoak, egoerak eta taldeak erakutsitako eguneratzekoak dira; jarduera-serieen errenkadak soilik mugatzen dira tartera, hasiera-ordutik amaiera-ordura arte, hura barne hartu gabe.",
                cancel: "Utzi",
                export: "Esportatu",
                invalidRange: "Amaierak hasiera baino geroagokoa izan behar du.",
                problems: {
                    unknownSelector:
                        "{{widget}} widgetak ez du jada «{{selector}}» aukera. Kargatu berriro panela eta esportatu berriro.",
                    unknownOption:
                        "{{widget}} widgetean «{{selector}}» aukerarako hautatutako balioa ez dago jada eskuragarri. Hautatu berriro eta esportatu.",
                },
            },
            editor: {
                scopeSelector: {
                    region: "Eskualdea",
                    post: "Postua",
                    country: "Herrialdea",
                },
                sources: {
                    voter_turnout: "Hautesleen partaidetza",
                    test_voting: "Proba-bozketa",
                    enrollment_decisions: "Izen-emateen erabakiak",
                    voting_credentials: "Bozketa-kredentzialak",
                    poll_status: "Bozketaren egoera",
                    final_testing_lockdown: "Azken probak eta blokeoa",
                    counting_transmission: "Zenbaketa eta transmisioa",
                    voting_enrollment_activity: "Bozketa- eta izen-emate jarduera",
                    access_security: "Sarbidea eta segurtasuna",
                    attack_detections: "Erasoen detekzioak",
                    helpdesk: "Laguntza-zerbitzua",
                },
                templates: {
                    summary: "Laburpena",
                    by_group: "Taldeka",
                    by_post: "Postuka",
                    timeseries: "Denboran zehar",
                    by_measure: "Neurrika",
                },
                yaml: {
                    label: "YAML",
                    readOnly:
                        "YAMLak sintaxi-errore bat du. Zuzendu YAML fitxan, berriro formularioak erabiltzeko.",
                },
                diagnostics: {
                    title: "Egiaztapenak",
                    none: "Ez da arazorik aurkitu",
                    localUnavailable:
                        "Nabigatzailean egiten diren egiaztapenak ez daude erabilgarri; arazoak aurrebistaren edo Balidatu-ren ondoren agertzen dira.",
                    line: "{{line}}. lerroa",
                    engineCode: "dbt Charts {{code}}",
                    severity: {
                        ERROR: "Errorea",
                        WARNING: "Abisua",
                    },
                    origin: {
                        SYNTAX: "YAML sintaxia",
                        LOCAL: "Nabigatzailearen egiaztapena",
                        SERVER: "Zerbitzariaren egiaztapena",
                    },
                },
                footer: {
                    preview: "Aurrebista",
                    valid: "Baliozkoa",
                    errors_one: "{{count}} errore",
                    errors_other: "{{count}} errore",
                    noWarnings: "grafiko-abisurik ez",
                    warnings_one: "{{count}} grafiko-abisu",
                    warnings_other: "{{count}} grafiko-abisu",
                    rendering: "Sortzen…",
                    previewFailed: "aurrebistak huts egin du",
                    renderedIn: "{{ms}} ms-tan sortua",
                    revision: "{{revision}}. berrikuspena",
                    savedBy: "{{user}} erabiltzaileak gorde du {{date}}",
                    unknownUser: "administratzaile bat",
                    notSaved: "oraindik gorde gabe",
                    unsaved: "gorde gabeko aldaketak",
                    cancel: "Utzi",
                    validate: "Balidatu",
                },
                preview: {
                    title: "Aurrebista",
                    empty: "Aurrebista YAMLa baliozkoa denean agertzen da.",
                    failed: "Ezin izan da aurrebista sortu: {{reason}}",
                    notConnectedTail: "Ez da ezer erakusten konektatu arte.",
                    queryResult: "Kontsultaren emaitza · lehen errenkadak",
                    noRows: "Kontsultak ez du errenkadarik itzuli.",
                    state: {
                        RENDERED: "Sortua",
                        NOT_CONNECTED: "Konektatu gabe",
                        NO_SNAPSHOT: "Oraindik daturik ez",
                        SCOPE_PENDING: "Eremu hau zenbatzen ari da",
                        RENDER_FAILED: "Ezin izan da grafikoa marraztu",
                        INVALID: "Konfigurazioa ez da baliozkoa",
                    },
                },
                configureWidget: {
                    title: "Konfiguratu widgeta",
                    tabs: {
                        dataQuery: "Datuak eta kontsulta",
                        selectors: "Hautatzaileak",
                        yaml: "YAML",
                        preview: "Aurrebista",
                    },
                    save: "Gorde widgeta",
                    loading: "Widgeta kargatzen…",
                    loadFailed: "Ezin izan da widgeta kargatu: {{reason}}",
                    saved: "Widgeta {{revision}}. berrikuspen gisa gorde da",
                    refused: "Widgeta ez da gorde: zuzendu zerrendatutako arazoak.",
                    validated: "Widgeta baliozkoa da.",
                    invalid: "Widgetak arazoak ditu: ikusi egiaztapenak.",
                    requestFailed: "Eskaerak huts egin du: {{reason}}",
                    discardTitle: "Aldaketak baztertu?",
                    discardBody: "Widget honetan egindako aldaketak ez dira gorde.",
                    discard: "Baztertu",
                    keepEditing: "Jarraitu editatzen",
                },
                dataQuery: {
                    title: "Izenburua",
                    source: "Datu-iturria",
                    template: "Kontsulta",
                    measures: "Neurriak",
                    ratio: "Zenbakitzailea / izendatzailea",
                    numerator: "Zenbakitzailea",
                    denominator: "Izendatzailea",
                    groupBy: "Multzokatu honen arabera",
                    sort: "Ordenatu",
                    sortOrder: "Ordena",
                    templateOrder: "Kontsultaren ordena propioa",
                    sortBy: {
                        label: "Etiketa",
                        value: "Balioa",
                        ratio: "Erratioa",
                    },
                    order: {
                        asc: "Gorantz",
                        desc: "Beherantz",
                    },
                    limit: "Errenkadak",
                    noLimit: "Guztiak",
                    none: "Bat ere ez",
                    fromSelector: "Hautatzailetik: {{name}}",
                    follow: "Jarraitu panelaren hautatzaileei",
                    followHelp: "Markatu gabeko hautatzaile batek ez du widget hau murrizten.",
                    manyQueries: "Widget honek hainbat kontsulta ditu; editatu itzazu YAML fitxan.",
                    notConnected: "Konektatu gabe · {{reason}}",
                    sourceHelp: {
                        DISTINCT_VOTERS:
                            "Hautesle desberdinak zenbatzen ditu; zenbaketa-arauak datu-iturriak finkatzen ditu.",
                        DISTINCT_PRE_ENROLLED_VOTERS:
                            "Aurrez izena emandako hautesle desberdinak zenbatzen ditu; zenbaketa-arauak datu-iturriak finkatzen ditu.",
                        LATEST_DECISION_PER_VOTER:
                            "Hautesleko azken erabakia zenbatzen du; zenbaketa-arauak datu-iturriak finkatzen ditu.",
                        APPROVED_VOTERS:
                            "Onartutako hautesleak zenbatzen ditu; zenbaketa-arauak datu-iturriak finkatzen ditu.",
                        POSTS_IN_SCOPE:
                            "Eremuko postuak zenbatzen ditu; zenbaketa-arauak datu-iturriak finkatzen ditu.",
                        FIRST_EVENT_PER_VOTER:
                            "Hautesle bakoitzaren lehen boto edo onarpen baliozkoa zenbatzen du; zenbaketa-arauak datu-iturriak finkatzen ditu.",
                        ATTEMPTS:
                            "Saiakerak zenbatzen ditu, ez pertsonak; zenbaketa-arauak datu-iturriak finkatzen ditu.",
                        DETECTIONS:
                            "Detekzioak zenbatzen ditu; zenbaketa-arauak datu-iturriak finkatzen ditu.",
                        REPORTED_ISSUES:
                            "Jakinarazitako arazoak zenbatzen ditu; zenbaketa-arauak datu-iturriak finkatzen ditu.",
                        default: "Zenbaketa-arauak datu-iturriak finkatzen ditu.",
                    },
                },
                selectors: {
                    help: "Hautatzaileak widgetaren goiburuan agertzen dira. Haien balioek kontsulta elikatzen dute; panelaren hautatzaileak (Eskualdea, Postua, Herrialdea) widget guztiei aplikatzen zaizkie.",
                    name: "Izena",
                    label: "Etiketa",
                    control: "Kontrola",
                    controls: {
                        dropdown: "Goitibeherakoa",
                        toggle: "Etengailua",
                    },
                    default: "Lehenetsia",
                    optionValue: "Balioa",
                    optionLabel: "Aukeraren etiketa",
                    addOption: "Gehitu aukera",
                    addSelector: "Gehitu hautatzailea",
                    removeOption: "Kendu {{option}} aukera",
                    removeSelector: "Kendu {{name}} hautatzailea",
                    moveUp: "Igo {{name}}",
                    moveDown: "Jaitsi {{name}}",
                    dynamic: "Aukerak datuetatik datoz ({{source}}).",
                    none: "Widget honek ez du hautatzailerik.",
                    newLabel: "Hautatzaile berria",
                    newOption: "Aukera berria",
                    shownWhen: "{{selector}} hau denean erakusten da: {{values}}",
                },
                conflict: {
                    title: "Norbaitek lehenago gorde du",
                    body: "{{user}} erabiltzaileak {{revision}}. berrikuspena gorde du ({{date}}) zu editatzen ari zinen bitartean.",
                    bodyShort:
                        "{{revision}}. berrikuspena gorde da zu editatzen ari zinen bitartean.",
                    saved: "Gordetako berrikuspena",
                    mine: "Nire aldaketak",
                    reload: "Birkargatu",
                    copy: "Kopiatu nire YAMLa",
                    copied: "Zure YAMLa arbelean dago.",
                    copyFailed: "Arbela ez dago erabilgarri; hautatu YAMLa eta kopiatu eskuz.",
                    keepEditing: "Jarraitu editatzen",
                    removed:
                        "Dokumentua ezabatu egin da zu editatzen ari zinen bitartean. Jarraitu editatzen berriro gordetzeko.",
                },
                dashboard: {
                    editing: "Panela editatzen",
                    title: "Izenburua",
                    selectors: "Panelaren hautatzaileak",
                    theme: "Gaia",
                    editTheme: "Editatu gaia",
                    addWidget: "Gehitu widgeta",
                    cancel: "Utzi",
                    save: "Gorde panela",
                    width: "Zabalera",
                    widthValue: "{{n}} / 12",
                    moveUp: "Igo",
                    moveDown: "Jaitsi",
                    remove: "Kendu",
                    duplicate: "Bikoiztu",
                    configure: "Konfiguratu widgeta",
                    empty: "Panel honek ez du widgetik oraindik.",
                    saved: "Panela {{revision}}. berrikuspen gisa gorde da",
                    refused: "Panela ez da gorde: zuzendu zerrendatutako arazoak.",
                    requestFailed: "Eskaerak huts egin du: {{reason}}",
                    dragHandle: "Arrastatu {{title}} berrantolatzeko",
                    widgets: "Widgetak",
                    duplicated: "{{id}} gisa bikoiztu da",
                    resetToPreset: "Berrezarri aurrezarpenera",
                    actions: "{{title}} elementuaren ekintzak",
                    discardBody: "Panel honetan egindako aldaketak ez dira gorde.",
                    duplicateInvalid: "Kopia ez da gorde: {{problem}}",
                    layoutMalformed:
                        "Diseinuko elementu batzuk ez dira zabalera duen widget bat. Konpondu YAML fitxan widgetak berrantolatzeko.",
                },
                catalog: {
                    title: "Gehitu widgeta",
                    search: "Bilatu widgetak, datu-iturriak edo eskakizunak",
                    add: "Gehitu",
                    empty: "Ez dago widget egokirik.",
                    onDashboard: "Panel honetan",
                    close: "Itxi",
                },
                theme: {
                    title: "Panelaren gaia",
                    subtitle: "dbt Charts estiloa, panel honetako widget guztiei aplikatua",
                    appliesTo_one: "{{count}} widgeti aplikatzen zaio",
                    appliesTo_other: "{{count}} widgeti aplikatzen zaie",
                    apply: "Aplikatu gaia",
                    saved: "Gaia {{revision}}. berrikuspen gisa gorde da",
                    discardBody: "Gai honetan egindako aldaketak ez dira gorde.",
                },
                reset: {
                    title: "Berrezarri aurrezarpenera",
                    body: "Ekitaldi honetako panel, widget eta gai guztiak aurrezarpenekoekin ordezkatzen dira. Uneko konfigurazioak historian jarraitzen du.",
                    preset: "Aurrezarpena",
                    confirm: "Berrezarri",
                    cancel: "Utzi",
                    done: "Ekitaldiak orain {{title}} erabiltzen du.",
                    failed: "Berrezarpenak huts egin du: {{reason}}",
                    noPresets: "Ez dago aurrezarpenik erabilgarri.",
                    loading: "Aurrezarpenak kargatzen…",
                },
                lockedDown:
                    "Ekitaldia blokeatuta dago; haren monitorizazio-konfigurazioa ezin da aldatu.",
                document: {
                    loadFailed: "Ezin izan da dokumentua kargatu: {{reason}}",
                    refused: "Ez da gorde: konpondu zerrendako arazoak.",
                    validated: "Dokumentua baliozkoa da.",
                    invalid: "Dokumentuak arazoak ditu: ikusi egiaztapenak.",
                    requestFailed: "Eskaerak huts egin du: {{reason}}",
                    savedWithWarnings_one: "Ohar {{count}}: ikusi egiaztapenak.",
                    savedWithWarnings_other: "{{count}} ohar: ikusi egiaztapenak.",
                },
                errors: {
                    checksUnavailable:
                        "Grafikoen motorrak ezin izan du aldaketa egiaztatu, eta ez da gorde. Saiatu berriro une batean.",
                    busy: "Gertaera honen beste aldaketa batzuk gordetzen ari dira. Saiatu berriro une batean.",
                    lockedDown:
                        "Gertaera blokeatuta dago; bere monitorizazio-konfigurazioa ezin da aldatu.",
                    forbiddenScope:
                        "Ezin dituzu ikusi aukeratutako eskualdeko, postuko edo herrialdeko zifrak.",
                    badRequest:
                        "Editoreak zerbitzariak irakurri ezin duen eskaera bat bidali du. Kargatu berriro orria eta saiatu berriro.",
                },
                duplicate: {
                    copyTitle: "{{title}} (kopia)",
                    done: "{{id}} gehitu da panelera, widgetaren kopia bat.",
                    failed: "Ezin izan da widgeta bikoiztu: {{reason}}",
                    notPlaced:
                        "{{id}} kopia gorde da, baina panelak ez du hartu ({{reason}}). Gehitu Editatu panela erabiliz.",
                },
            },
        },
        monitoringDashboardScreen: {
            voters: {
                title: "Bozkatzaileak",
                enrolledOverseasVoters: "Matrikulatutako Atzerriko Bozkatzaileak",
                approvalStatus: "Onespen Egoera: Onartutako/Baztertutako Bozkatzaileak",
                manuallyApproval: "Eskuz Onartutako/Baztertutako Bozkatzaileak",
                automaticallyApproval: "Automatikoki Onartutako/Baztertutako Bozkatzaileak",
                authenticatedVoters: "Autentifikatutako Bozkatzaileak",
                invalidUserErrors: "Erabiltzaile Baliogabe Erroreak:",
                invalidPasswordErrors: "Pasahitz Baliogabe Erroreak:",
            },
            polls: {
                title: "Inkestak",
                initializedSystems: "Sistema Hasieratuak dituzten Postuak",
                votingOpened: "Lineko Bozketa Irekia duten Postuak",
                votingClosed: "Bozketa Kanal biak Itxita dituzten Postuak",
                votingStarted: "Bozketa Hasita duten Postuak",
                voterTurnout: "Bozkatzaileen Parte-hartzea",
            },
            tally: {
                title: "Zenbaketa",
                activeVotesCounting: "Boto Zenbaketa Aktiboa duten Postuak",
                generatedERs: "ERak Sortuta dituzten Postuak",
                transmittedResults: "Emaitzak Transmitituta dituzten Postuak",
            },
            testing: {
                title: "Probak",
                testElectionVoterCount: "Proba Hauteskunde Bozkatzaile Kopurua",
            },
        },
        certificateAuthorities: {
            title: "Ziurtagiriak",
            subtitle:
                "Hauteskunde-ekitaldi honetarako fidagarriak diren ziurtagiri-agintariak (CA). Inportatutako CA-k hautesleen ziurtagiriak egiaztatzeko erabiltzen dira.",
            importButton: "Ziurtagiriak inportatu",
            type: {
                root: "Erroa",
                intermediate: "Tartekoa",
            },
            expiry: {
                expired: "Iraungita",
                expiringSoon: "Laster iraungiko da",
                valid: "Baliozkoa",
            },
            columns: {
                commonName: "Izen arrunta",
                type: "Mota",
                issuerCn: "Eragilearen CN",
                notBefore: "Baliozko hasieratik",
                notAfter: "Iraungitzen da",
                fingerprint: "SHA256 Hatz-marka",
            },
            importDialog: {
                title: "Ziurtagiri-agintaritzak inportatu",
                subtitle: "PEM fitxategi batetik CA ziurtagiri bat edo gehiago inportatu",
                description:
                    "Hautatu ziurtagiri bat edo gehiago dituen PEM fitxategia. Multzoak onartzen dira — ziurtagiri bakoitza banaka inportatzen da.",
                selectFile: "PEM fitxategia hautatu",
                fileLoaded: "Fitxategia kargatuta ({{bytes}} byte)",
                importButton: "Inportatu",
            },
            notify: {
                importSuccess: "{{inserted}} ziurtagiri inportatuta.",
                importSkipped: "{{count}} saltatu (dagoeneko badaude).",
                importErrors: "Inportazio arazoak: {{errors}}",
                importError: "Inportazioa huts egin du: {{error}}",
                deleteSuccess: "Ziurtagiria ezabatuta.",
                deleteError: "Errorea ziurtagiria ezabatzean.",
                exportSuccess: "Ziurtagiri(ak) arrakastaz esportatu da(dira).",
                exportError: "Errorea ziurtagiriak esportatzean.",
            },
            exportDialog: {
                title: "Ziurtagiri-agintaritzak esportatu",
                description: "{{amount}} ziurtagiri esportatuko duzu.",
                all: "guztiak",
            },
            deleteDialog: {
                description: "Ziur zaude {{count}} ziurtagiri(ak) ezabatu nahi dituzula?",
            },
            emptyHeader:
                "Ez da ziurtagiri-agintaritzarik inportatu hauteskunde-ekitaldi honetarako.",
            fileReadError: "Fitxategia irakurtzean huts egin du.",
            viewDialog: {
                title: "Ziurtagiri-agintaritzaren xehetasunak",
                subject: "Gaia",
                issuer: "Eragilea",
                serialNumber: "Serie-zenbakia",
                pemContent: "PEM edukia",
            },
            confirmDelete: "Ziurtagiri-agintaritza ezabatu",
            confirmDeleteDescription:
                'Ziur al zaude "{{name}}" ziurtagiria (hatz-marka: {{fingerprint}}) ezabatu nahi duzula?',
        },
        signing: {
            terms: {
                post: "Postu",
                posts: "Postuak",
            },
            tab: {
                title: "Sinadurak",
                intro: "Ekintza babestuak behar adina pertsona baimendunek beren ziurtagiri digitalekin sinatu ondoren bakarrik exekutatzen dira. Sinadura bakoitza jaulkitzaile fidagarrien aurka egiaztatzen da eta erregistroan jasotzen da.",
                protectedActions: "Ekintza babestuak",
                certificates: "Ziurtagiriak",
                requests: "Eskaerak",
            },
            loadError:
                "Ezin izan da sinadura-ezarpenak kargatu. Kargatu berriro orria berriro saiatzeko.",
            errors: {
                automatedCeremonies:
                    "Ekitaldi honek gakoen zeremonia automatikoak erabiltzen ditu. Zaindariek ez dituzte urrats hauek egiten; beraz, ezin dira haien sinadurak eskatu. Zaindarien sinadurak eskatzeko, erabili eskuzko gakoen zeremoniak.",
                forbidden: "Ez duzu aldaketa hau egiteko baimenik.",
                invalid: "Zerbitzariak balio hauek baztertu ditu. Egiaztatu eta saiatu berriro.",
                conflict:
                    "Beste norbaitek aldatu du bitartean. Kargatu berriro orria eta saiatu berriro.",
                lockedDown:
                    "Hauteskunde-ekitaldia blokeatuta dago: sinadura-arauak konfigurazio-bertsio berri baten bidez bakarrik aldatzen dira.",
                notFound: "Jada ez dago. Kargatu berriro orria.",
            },
            readOnly: {
                chip: "Irakurtzeko soilik",
                rules: "Irakurtzeko soilik. Sinadura-arauak aldatzeko «Sinadurak: editatu ekintza babestuak» baimena behar da.",
                whoCanSign:
                    "Erabiltzaileak eta Rolak atalean «Sinatu: {{action}}» baimena duten rolak. Horiek aldatzeko rolak editatzeko baimena behar da.",
            },
            groups: {
                "voting": "Bozketa",
                "results-and-reports": "Emaitzak eta txostenak",
                "enrollment": "Izen-ematea",
                "configuration-and-keys": "Konfigurazioa eta giltzak",
            },
            actions: {
                "initialize-voting": {
                    label: "Bozketa hasieratu",
                    short: "Hasieratzea",
                    permissionName: "bozketa hasieratu",
                    object: "bozketaren hasieratzea",
                    appliesTo: "$t(signing.terms.post) bakoitza",
                    description:
                        "Argitaratu atalean hasten da. $t(signing.terms.post)a hasieratzen du eta haren Hasieratze Txostena sortzen du.",
                },
                "open-voting": {
                    label: "Bozketa ireki",
                    short: "Irekiera",
                    permissionName: "bozketa ireki",
                    object: "bozketaren irekiera",
                    appliesTo: "$t(signing.terms.post) bakoitza",
                    description:
                        "Argitaratu atalean hasten da, Hasi Bozketa botoiarekin. Bozketa irekitzen du $t(signing.terms.post)an.",
                },
                "close-voting": {
                    label: "Bozketa itxi",
                    short: "Itxiera",
                    permissionName: "bozketa itxi",
                    object: "bozketaren itxiera",
                    appliesTo: "$t(signing.terms.post) bakoitza",
                    description:
                        "Argitaratu atalean hasten da, Gelditu Bozketa botoiarekin. Bozketa ixten du $t(signing.terms.post)an; itxierako sinadurak haren aktan gordetzen dira.",
                    descriptionSealed:
                        "Argitaratu atalean hasten da, Gelditu Bozketa botoiarekin. Bozketa ixten du $t(signing.terms.post)an. Kanal guztiak itxita daudenean, haren hautetsontziak zigilatzen dira, grazia-epearen ondoren, baldin badago: ezin da botorik gehitu, aldatu edo ezabatu, eta bozketa ezin da berriro hasi. Itxierako sinadurak haren aktan gordetzen dira.",
                },
                "generate-election-returns": {
                    label: "Hauteskunde-aktak sortu",
                    short: "Hauteskunde-aktak",
                    permissionName: "hauteskunde-aktak sortu",
                    object: "hauteskunde-aktak",
                    appliesTo: "$t(signing.terms.post) eta herrialde bakoitza",
                    description:
                        "Zenbaketak hasten du, eskaera bat $t(signing.terms.post) eta herrialde bakoitzeko. Sinatutako hauteskunde-aktak askatzen ditu inprimatzeko eta transmititzeko.",
                },
                "generate-reports": {
                    label: "Beste hauteskunde-txosten batzuk sortu",
                    short: "Txostena",
                    permissionName: "beste hauteskunde-txosten batzuk sortu",
                    object: "txostena",
                    appliesTo: "$t(signing.terms.post) bakoitza",
                    description:
                        "Zenbaketak hasten du Hasieratze Txostenerako, eta Txostenak atalak parte-hartze txostenerako. Sinatutako txostena askatzen du.",
                },
                "transmit-results": {
                    label: "Emaitzak transmititu",
                    short: "Transmisioa",
                    permissionName: "emaitzak transmititu",
                    object: "emaitza-paketea",
                    appliesTo: "$t(signing.terms.post) eta herrialde bakoitza",
                    description:
                        "Zenbaketa atalean hasten da, Transmisioa urratsean. Sinatutako emaitza-paketea sortzen du bere helburuetarako; sinadurek haren sinadura-zerrenda betetzen dute.",
                },
                "approve-voter": {
                    label: "Hautesle bat eskuz onartu",
                    short: "Hautesle-onarpena",
                    permissionName: "hautesle bat eskuz onartu",
                    object: "hautesle-onarpena",
                    appliesTo: "Hauteslearen $t(signing.terms.post)a",
                    description:
                        "Onespenak atalean hasten da. Hauteslea onartzen du eta haren kredentzialak igortzen ditu.",
                },
                "approve-configuration": {
                    label: "Konfigurazio-bertsio bat onartu",
                    short: "Konfigurazio-bertsioa",
                    permissionName: "konfigurazio-bertsio bat onartu",
                    object: "konfigurazio-bertsioa",
                    appliesTo: "Hauteskunde-ekitaldia",
                    description:
                        "Argitaratu atalean hasten da. Konfigurazio-bertsioa argitaratzen du.",
                },
                "key-ceremony": {
                    label: "Giltza-zati bat berretsi (giltzen zeremonia)",
                    short: "Giltza-zatia",
                    permissionName: "giltza-zati bat berretsi",
                    object: "giltza-zatia",
                    appliesTo: "Fideikomisario bakoitza",
                    description:
                        "Fideikomisario bakoitzak hasten du Giltzak atalean. Fideikomisarioaren sinadura zeremonian eta iragarki-taulan jasotzen du.",
                },
                "tally-key": {
                    label: "Giltza-zati bat eman (zenbaketa)",
                    short: "Giltza-zatiaren ekarpena",
                    permissionName: "giltza-zati bat eman",
                    object: "giltza-zatiaren ekarpena",
                    appliesTo: "Fideikomisario bakoitza",
                    description:
                        "Fideikomisario bakoitzak hasten du Zenbaketa atalean. Fideikomisarioaren ekarpena jasotzen du.",
                },
            },
            protectedActions: {
                intro: "Sinadura bakoitza sinatzailearen segurtasun-tokeneko ziurtagiri digitalarekin egiten da.",
                columns: {
                    action: "Ekintza",
                    appliesTo: "Honi aplikatzen zaio",
                    whoCanSign: "Nork sina dezakeen",
                    signaturesNeeded: "Behar diren sinadurak",
                    requestExpires: "Eskaeraren iraungitzea",
                    waiting: "Zain",
                },
                off: "Desaktibatuta",
                eachTrustee: "Fideikomisario bakoitza",
                footerVersion:
                    "Sinadura-arauak ekitaldi honen {{version}} konfigurazio-bertsioaren parte dira.",
                footerFirstVersion:
                    "Sinadura-arauak gertaera honen lehen konfigurazio-bertsioaren parte izango dira argitaratzen denean.",
                footerChanged: "Azken aldaketa: {{date}}.",
                footerChangedBy: "Azken aldaketa: {{date}}, {{name}}(e)k egina.",
                lockedDown:
                    "Hauteskunde-ekitaldia blokeatuta dago: haren sinadura-arauak bere konfigurazio-bertsioaren parte dira, beraz konfigurazio-bertsio berri baten bidez bakarrik aldatzen dira.",
                edit: "Editatu: {{action}}",
                view: "Ikusi: {{action}}",
                waitingCount_one: "{{count}} eskaera zain",
                waitingCount_other: "{{count}} eskaera zain",
                capacityError:
                    "Ezin izan da kargatu nork sina dezakeen; beraz, sinadura kopurua ezin da egiaztatu $t(signing.terms.posts) kontuan hartuta.",
            },
            expiry: {
                "30": "30 minutu",
                "60": "Ordu 1",
                "120": "2 ordu",
                "1440": "24 ordu",
                "none": "Mugarik gabe",
                "other": "{{count}} minutu",
            },
            rule: {
                needsSignatures: "Sinadurak behar ditu",
                whoCanSign: "Nork sina dezakeen",
                whoCanSignHelp:
                    "Rol hauek «Sinatu: {{action}}» baimena jasotzen dute Erabiltzaileak eta Rolak atalean, hauteskunde-ekitaldi guztietarako. Sinatzaileek $t(signing.terms.post)rako sarbidea ere izan behar dute.",
                signaturesNeeded: "Behar diren sinadurak",
                signaturesNeededHelp:
                    "Sinatzaile bakoitzak bere ziurtagiri digitala erabiltzen du. $t(signing.terms.post) bakoitzak gutxienez sina dezaketen {{n}} pertsona ditu.",
                signaturesNeededShortHelp:
                    "Sinatzaile bakoitzak bere ziurtagiri digitala erabiltzen du.",
                requesterSigning: "Hasten duen pertsonak ere sina dezake",
                expiresAfter: "Eskaera bat iraungitzen da denbora hau igarotakoan:",
                trusteesSign: "Fideikomisarioek sinatzen dute urrats hau",
                trusteesHelp:
                    "Fideikomisario bakoitzak bere urratsa sinatzen du bere ziurtagiri digitalarekin. Giltzen zeremoniak zehazten du zenbat fideikomisariok hartzen duten parte.",
                footer: "Aldaketak hauteskunde-ekitaldiaren erregistroan jasotzen dira eta hurrengo konfigurazio-bertsioaren parte bihurtzen dira.",
                cancel: "Ezeztatu",
                save: "Gorde",
                saved: "Sinadura-araua gorde da.",
                savedShort_one:
                    "Sinadura-araua gorde da. {{posts}} ezin da oraindik kopurura iritsi: gehitu sinatzaile bat bertan.",
                savedShort_other:
                    "Sinadura-araua gorde da. {{posts}} ezin dira oraindik kopurura iritsi: gehitu sinatzaileak bertan.",
                checkedOnSave: "Gordetzean, kopurua rol berrien arabera egiaztatzen da.",
                savedRequesterShort:
                    "Sinadura-araua gorde da. Badira eskaera bat hasten duen pertsonarik gabe kopurura iritsi ezin diren $t(signing.terms.posts).",
                saveError:
                    "Ezin izan da sinadura-araua gorde. Baliteke beste norbaitek bitartean aldatu izana; kargatu berriro eta saiatu berriro.",
            },
            validation: {
                atLeastOne: "Gutxienez 1.",
                tooMany:
                    "Ez dago sina dezaketen {{n}} pertsona dituen $t(signing.terms.post)rik. Gehienez {{max}} dira.",
                tooManyEvent:
                    "{{max}} pertsonak bakarrik sina dezakete hau. Aukeratu gehienez {{max}}.",
                atMost: "Gehienez {{max}}.",
                shortPosts_one:
                    "{{posts}}(e)k sina dezaketen {{n}} pertsona baino ez ditu, beraz ezin da {{required}} sinadurara iritsi. Gehitu sinatzaile bat bertan edo jaitsi kopurua.",
                requesterShort_one:
                    "Hasten duen pertsonarik gabe, {{posts}}(e)k sina dezaketen {{n}} pertsona baino ez ditu, beraz ezin da {{required}} sinadurara iritsi.",
                requesterShort_other:
                    "Hasten duen pertsonarik gabe, {{posts}}(e)k sina dezaketen {{n}} pertsona baino ez dituzte, beraz ezin dira {{required}} sinadurara iritsi.",
                shortPosts_other:
                    "{{posts}}(e)k sina dezaketen {{n}} pertsona baino ez dituzte, beraz ezin dira {{required}} sinadurara iritsi. Gehitu sinatzaile bat bertan edo jaitsi kopurua.",
            },
            pendingRequests_one:
                "{{count}} eskaera sinaduren zain dago uneko arauarekin. Gordetzeak ezeztatu egingo du; hasi zuen pertsonak berriro hasi beharko du.",
            pendingRequests_other:
                "{{count}} eskaera sinaduren zain daude uneko arauarekin. Gordetzeak ezeztatu egingo ditu; hasi zituzten pertsonek berriro hasi beharko dute.",
            certificates: {
                issuersIntro:
                    "Langileen ziurtagiriek hauetako batekin kateatu behar dute. Hautesleek saioa hasteko erabiltzen dituzten ziurtagirietatik bereizita daude.",
                checkRevocation: "Egiaztatu baliogabetze-zerrendak",
                crlUnavailable: {
                    "label": "Zerrenda bat deskargatu ezin denean",
                    "refuse": "Ez onartu sinadurak",
                    "accept-unchecked": "Onartu eta markatu sinadura egiaztatu gabe gisa",
                },
                registration: {
                    "label": "Ziurtagiri bat pertsona bati erregistratzea",
                    "on-first-use": "Titularrak lehen aldiz harekin sinatzen duenean",
                    "security-officer-only":
                        "Ziurtagiriak erregistra ditzakeen norbaitek erregistratzen duenean bakarrik",
                },
                onePost: "Ziurtagiri batek $t(signing.terms.post) bakarrerako sinatzen du",
                issuers: "Jaulkitzaile fidagarriak",
                import: "Inportatu jaulkitzaileen ziurtagiriak",
                importHelp:
                    "Aukeratu jaulkitzailearen ziurtagiria duen PEM edo CER fitxategi bat. PEM fitxategi batek hainbat ziurtagiri izan ditzake.",
                chooseFile: "Aukeratu ziurtagiri-fitxategi bat",
                fileError: "Ezin izan da fitxategia irakurri.",
                imported:
                    "Jaulkitzaileen {{imported}} ziurtagiri inportatu dira; {{skipped}} fidagarriak ziren jada.",
                importedWithErrors:
                    "Jaulkitzaileen {{imported}} ziurtagiri inportatu dira, {{skipped}} fidagarriak ziren jada. Baztertuak: {{errors}}",
                importError: "Ezin izan dira jaulkitzaileen ziurtagiriak inportatu.",
                deleteIssuer: "Kendu {{name}}",
                deleteIssuerConfirm:
                    "{{name}} jaulkitzaile fidagarrietatik kendu? Hark jaulkitako ziurtagiriek ezin izango dute sinatu.",
                deleteError: "Ezin izan da jaulkitzailea kendu.",
                noIssuers:
                    "Oraindik ez dago jaulkitzaile fidagarririk. Langileek ezin dute sinatu bat inportatu arte.",
                root: "Erroa",
                intermediate: "Tartekoa",
                columns: {
                    issuer: "Jaulkitzailea",
                    type: "Mota",
                    issuedBy: "Nork jaulkia",
                    validUntil: "Baliozkoa noiz arte",
                    sha256: "SHA-256",
                    person: "Pertsona",
                    post: "$t(signing.terms.post)a",
                    certificate: "Ziurtagiria",
                    registered: "Erregistratua",
                    status: "Egoera",
                },
                checks: "Egiaztapenak",
                checksSaved: "Ziurtagirien egiaztapenak gorde dira.",
                checksError: "Ezin izan dira ziurtagirien egiaztapenak gorde.",
                crlSchedule: "Jaulkitzaile bakoitzetik orduro deskargatzen dira.",
                crlUpdated: "{{url}}: eguneratua {{time}}",
                crlFailed: "{{url}}: ezin izan da deskargatu (azken saiakera {{time}})",
                registeredTitle: "Erregistratutako ziurtagiriak",
                search: "Bilatu pertsonak, ziurtagiriak edo $t(signing.terms.posts)",
                status: "Egoera",
                statusAll: "Guztiak",
                statuses: {
                    "active": "Aktiboa",
                    "expires-soon": "Laster iraungiko da",
                    "expired": "Iraungita",
                    "revoked": "Baliogabetuta",
                },
                revokedOn: "Baliogabetua: {{date}}",
                allPosts: "Guztiak",
                noCertificates: "Ez dago erregistratutako ziurtagiririk.",
                registeredHow: {
                    "first-use": "Lehen sinaduran",
                    "security-officer": "Administratzaile batek erregistratua",
                },
                register: "Erregistratu ziurtagiri bat",
                registerSubmit: "Erregistratu",
                registerDone: "Ziurtagiria erregistratu da.",
                registerError: "Ezin izan da ziurtagiria erregistratu.",
                person: "Pertsona",
                personSearchHelp: "Idatzi erabiltzaile-izen baten zati bat pertsona aurkitzeko.",
                registeredBy: "Egilea: {{name}}",
                registerRefused:
                    "Ziurtagiri hau ezin da erregistratu: egiaztatu jaulkitzaile fidagarri batek jaulki duela, gaur baliozkoa dela eta sinatzeko egina dagoela.",
                registeredToOther:
                    "Ziurtagiri hau {{name}}(r)en izenean erregistratuta dago. Kontu hau ere {{name}}(r)ena bada, lotu bere bigarren kontu gisa.",
                linkAccount: "Lotu pertsona beraren bigarren kontu gisa",
                alreadyRegistered:
                    "Ziurtagiri hau pertsona honen izenean erregistratuta dago jada.",
                pem: "Ziurtagiria (PEM)",
                revoke: "Baliogabetu",
                revokeOf: "Baliogabetu {{name}}(r)en ziurtagiria",
                revokeTitle: "Baliogabetu {{name}}(r)en ziurtagiria",
                revokeHelp:
                    "Baliogabetutako ziurtagiri batek ezin du gehiago sinatu. Lehendik egindako sinadurek balio dute.",
                revokeReason: "Arrazoia",
                revokeDone: "Ziurtagiria baliogabetu da.",
                revokeError: "Ezin izan da ziurtagiria baliogabetu.",
            },
            requests: {
                exportCsv: "Esportatu CSV",
                exportError: "Ezin izan dira eskaerak esportatu.",
                exportFileName: "signing-requests.csv",
                status: "Egoera",
                statusAll: "Guztiak",
                statusCount: "{{status}} · {{count}} / {{total}}",
                expires: "Iraungitzea: {{time}}",
                lastSignatureBy: "{{name}}, {{time}}",
                empty: "Oraindik ez dago sinadura-eskaerarik.",
                columns: {
                    request: "Eskaera",
                    status: "Egoera",
                    started: "Hasiera",
                    by: "Egilea",
                    lastSignature: "Azken sinadura",
                    code: "Kodea",
                },
            },
            reports: {
                postRequired:
                    "Hautatu hauteskunde-postu bat txosten hau sortzeko sinadurak behar direnean.",
                generateNotice:
                    "{{post}}: dokumentua orain sortzen da. {{n}} pertsonak sinatu ondoren inprimatu eta transmititu ahal izango da.",
            },
            status: {
                waiting: "Zain",
                completed: "Sinatuta",
                executed: "Eginda",
                cancelled: "Ezeztatuta",
                expired: "Iraungita",
                failed: "Huts egin du",
            },
            cancelReasons: {
                "by-requester": "Hasi zuen pertsonak ezeztatu zuen",
                "by-operator": "Operadore batek ezeztatu zuen",
                "rule-changed": "Ekintzaren sinadura-araua aldatu zen",
                "payload-changed": "Sinatzen duena aldatu zen",
                "superseded": "Eskaera berriago batek ordezkatu zuen",
                "certificate-revoked": "Hura sinatu zuen ziurtagiri bat baliogabetu zen",
            },
            panel: {
                rulePost:
                    "{{post}}(e)ko sinatzaileen {{n}} sinadura behar ditu, bakoitza bere ziurtagiri digitalarekin.",
                ruleEvent:
                    "{{n}} sinadura behar ditu, bakoitza sinatzailearen ziurtagiri digitalarekin.",
                signingCode: "Sinadura-kodea",
                signers: "Sinatzaileak",
                sign: "Sinatu",
                handover: "Hurrengo kideak saioa hasten du",
                cancel: "Ezeztatu eskaera",
                signedAt: "Sinatua {{time}}",
                notSigned: "Sinatu gabe",
                certificate: "{{name}} ziurtagiria",
                you: "(zu)",
                expiresAt: "Iraungitzea: {{time}}",
                progress: "{{count}} / {{total}}",
                openDocument: "Ireki dokumentua",
                configurationVersion: "{{version}}. konfigurazio-bertsioa",
                configurationChanges: "Bertsio honetako aldaketak",
            },
            dialog: {
                title: "Sinatu: {{object}}",
                steps: {
                    check: "Berrikusi",
                    certificate: "Ziurtagiria",
                    signed: "Sinatuta",
                },
                localNote:
                    "Sinadura nabigatzaile honetan egiten da. Zure ziurtagiri-fitxategia, haren giltza pribatua eta pasahitza ez dira inoiz bidaltzen. Zure sinadura eta zure ziurtagiri publikoa bakarrik iristen dira zerbitzarira.",
                check: {
                    signingAs: "{{name}} gisa sinatzen ari zara",
                    titlePost: "{{title}}, {{post}}",
                    sameCode: "Sinatzen duten guztiek kode bera ikusten dute.",
                    confirmDocument: "Sinatzen dudana berrikusi dut: {{object}}",
                },
                certificate: {
                    intro: "Sartu zure segurtasun-tokena eta aukeratu zure ziurtagiri-fitxategia.",
                    password: "Ziurtagiriaren pasahitza",
                    open: "Ireki ziurtagiria",
                    chooseAnother: "Aukeratu beste fitxategi bat",
                },
                checks: {
                    "passed": {
                        "trusted-issuer": "Jaulkitzaile fidagarri batek jaulkia ({{root}})",
                        "valid-now": "Gaur baliozkoa",
                        "signing-key-usage": "Sinatzeko egina",
                        "not-revoked": "Baliogabetu gabea (zerrendak eguneratuta {{time}})",
                        "registered": "Zure izenean erregistratua {{date}}(e)an",
                        "registered-to-other": "Ez dago beste inoren izenean erregistratuta",
                        "already-signed": "Oraindik ez da eskaera honetarako erabili",
                        "post-binding": "$t(signing.terms.post) honetarako erregistratua",
                        "signature": "Sinadurak eskaera hau estaltzen du",
                    },
                    "failed": {
                        "trusted-issuer": "Ez du jaulkitzaile fidagarri batek jaulki",
                        "valid-now": "Gaur ez da baliozkoa",
                        "signing-key-usage": "Ez dago sinatzeko egina",
                        "not-revoked":
                            "Baliogabetuta, edo ez dago hura egiaztatzeko baliogabetze-zerrenda eguneraturik",
                        "registered": "Ez dago zure izenean erregistratuta",
                        "registered-to-other": "{{name}}(r)en izenean erregistratua",
                        "already-signed": "Eskaera honetarako erabili da jada",
                        "post-binding": "Beste $t(signing.terms.post) baterako erregistratua",
                        "signature": "Sinadurak ez du eskaera hau estaltzen",
                    },
                    "first-use": "Lehen erabilera: zure izenean erregistratuko da",
                },
                problems: {
                    wrongPassword: "Pasahitz okerra. Egiaztatu eta saiatu berriro.",
                    notForYou:
                        "Ziurtagiri honek ezin du zure izenean sinatu. Erabili zure segurtasun-tokeneko ziurtagiria.",
                    issuerNotAccepted:
                        "Erabili {{organization}}(e)k zuretzat erregistratu duen ziurtagiria. Ez dira beste jaulkitzaile batzuen ziurtagiriak onartzen.",
                    cancelled:
                        "Eskaera hau ezeztatu da: {{reason}}. Harentzat emandako sinadurek ez dute balio. Hasi berriro uneko bertsioa sinatzeko.",
                },
                signed: {
                    title: "Sinatuta",
                    withCertificate: "{{name}}(r)en ziurtagiriarekin",
                    count: "{{n}} / {{total}} sinadura.",
                    allIn: "{{total}} sinadurak jasota daude.",
                    next: "Hurrengo sinatzaileak: {{names}}.",
                },
                handover:
                    "Zure saioa itxiko da. Hurrengo kideak ordenagailu honetan hasiko du saioa eta eskaera honetara itzuliko da sinatzeko. Eskaera irekita egongo da {{time}} arte.",
                sign: "Sinatu",
                back: "Atzera",
                cancel: "Ezeztatu",
            },
            widget: {
                continue: "Jarraitu",
                done: "Eginda",
                close: "Itxi",
                retry: "Saiatu berriro",
                loading: "Eskaera kargatzen…",
                loadError: "Ezin izan da eskaera kargatu.",
                chooseFile: "Aukeratu ziurtagiri-fitxategia",
                fileInput: "Ziurtagiri-fitxategia",
                fileSize: "{{size}} KB",
                showPassword: "Erakutsi pasahitza",
                hidePassword: "Ezkutatu pasahitza",
                opening: "Ziurtagiria irekitzen…",
                checking: "Ziurtagiria egiaztatzen…",
                signing: "Sinatzen…",
                certificateCard:
                    "Jaulkitzailea: {{issuer}} · baliozkoa {{date}} arte · {{algorithm}}",
                fingerprint: "SHA-256 {{fingerprint}}",
                algorithms: {
                    "rsa-pkcs1-sha256": "RSA",
                    "ecdsa-p256-sha256": "EC P-256",
                },
                document: "{{type}} · SHA-256 {{hash}}",
                documentPages: "{{type}} · {{pages}} orri · SHA-256 {{hash}}",
                checksTitle: "Ziurtagiriaren egiaztapenak",
                untrustedIssuer:
                    "{{issuer}} ez da hauteskunde-ekitaldi honetarako jaulkitzaile fidagarria",
                registeredToSomeoneElse: "Beste norbaiten izenean erregistratua",
                checkPassedNoDetail: {
                    "trusted-issuer": "Jaulkitzaile fidagarri batek jaulkia",
                    "not-revoked": "Baliogabetu gabea",
                },
                organization: "zure erakundea",
                cantSign: "Ziurtagiri honek ezin du eskaera hau sinatu.",
                checkError: "Ezin izan da ziurtagiria egiaztatu. Saiatu berriro.",
                fileErrors: {
                    UNREADABLE_FILE:
                        "Fitxategi hau ez da ziurtagiri-fitxategi bat (.p12 edo .pfx), edo hondatuta dago.",
                    UNSUPPORTED_ENCRYPTION:
                        "Nabigatzaile honek ezin du fitxategi honek erabiltzen duen zifratzea ireki.",
                    NO_PRIVATE_KEY:
                        "Fitxategi honek ez du giltza pribaturik. Aukeratu zure segurtasun-tokeneko ziurtagiri-fitxategia.",
                    NO_CERTIFICATE: "Fitxategi honek ez du ziurtagiririk.",
                    UNSUPPORTED_KEY:
                        "Ziurtagiri honen giltza-mota ez da onartzen. Erabili RSA edo EC P-256 ziurtagiri bat.",
                    KEY_CERTIFICATE_MISMATCH:
                        "Fitxategi honetako ziurtagiria ez dator bat haren giltzarekin.",
                },
                openError: "Ezin izan da ziurtagiria ireki. Saiatu berriro.",
                signError: "Ezin izan da sinadura bidali. Saiatu berriro.",
                refused: "Zerbitzariak sinadura baztertu du.",
                stale: "Dokumentua aldatu egin da sinatzen ari zinen bitartean. Sinatu berriro.",
                mismatch:
                    "Sinatuko litzatekeena ez dator bat eskaera honekin. Itxi elkarrizketa-koadroa eta ireki berriro eskaera.",
                documentMismatch: "Dokumentua ez dator bat eskaera honek sinatzen duenarekin.",
                documentError: "Ezin izan da dokumentua deskargatu. Saiatu berriro.",
                alreadySigned: "Eskaera hau sinatu duzu jada.",
                closed: {
                    changed:
                        "Eskaera hau aldatu egin da ireki ondoren. Itxi leiho hau eta berrikusi berriro sinatu aurretik.",
                    allSigned: "Eskaera honek bere sinadura guztiak ditu jada.",
                },
                chooseCertificate: "Sinatzeko ziurtagiria",
                renderError: "Ezin izan da sinadura-eskaera erakutsi. Itxi eta ireki berriro.",
                signedAt: "{{time}}",
                panel: {
                    completedAt: "Sinatua {{time}}",
                    expired:
                        "Eskaera hau iraungi da. Harentzat emandako sinadurek ez dute balio. Hasi berriro sinatzeko.",
                    failed: "Sinadura guztiak jasota daude, baina ekintzak huts egin du. Erregistroan daude xehetasunak.",
                    details: "Xehetasunak",
                    close: "Itxi eskaeraren panela",
                },
                cancelDialog: {
                    title: "Eskaera hau ezeztatu?",
                    body: "Harentzat emandako sinadurek ez dute balio. Hasi zuen pertsonak berriro hasi beharko du.",
                    reason: "Arrazoia (aukerakoa)",
                    confirm: "Ezeztatu eskaera",
                    back: "Mantendu",
                    error: "Ezin izan da eskaera ezeztatu. Saiatu berriro.",
                },
                handoverDialog: {
                    title: "Hurrengo kideak saioa hasten du",
                    noExpiry:
                        "Zure saioa itxiko da. Hurrengo kideak ordenagailu honetan hasiko du saioa eta eskaera honetara itzuliko da sinatzeko.",
                    confirm: "Itxi saioa",
                    back: "Mantendu saioa",
                    error: "Ezin izan da txandakatzea erregistratu. Saiatu berriro.",
                },
            },
            details: {
                keys_ceremony_id: "Zeremonia",
                tally_session_id: "Zenbaketa-saioa",
                trustee_id: "Fideikomisarioa",
                key_share_sha256: "Giltza-zatiaren SHA-256",
                channel: "Kanala",
                channels: "Kanalak",
                publication_id: "Boto-txartelen argitalpena",
                ballot_publication_id: "Boto-txartelen argitalpena",
                digest: "Konfigurazioaren SHA-256",
                signing_rules: "Sinadura-arauak",
                scheduled_events: "Programatutako gertaera berriak",
                ballots_and_contests: "Boto-txartelak eta lehiaketak",
                application_id: "Izen-emate eskaera",
                applicant_registry_id: "Erroldako kontua",
                decision: "Erabakia",
                submitted_at: "Bidalia",
                reason: "Zergatik behar duen pertsona bat",
                registry_record: "Erroldako erregistroa",
                status: "Izen-emate eskaeraren egoera",
                from: "Aurreko egoera",
            },
            closed: {
                pending: "Sinadura guztiak jasota daude. Bozketa berehala itxiko da.",
                title: "Bozketa {{time}}(e)an itxi da.",
                titleSealed: "Bozketa {{time}}(e)an itxi da. Boto-txartelak zigilatuta.",
                record: "Zigilatze-akta",
                ballots: "Zigiluko boto-txartelak",
                sealHash: "Zigiluaren {{algorithm}}",
                signedBy: "Sinatzaileak",
                signatures: "Zigilatze-aktako itxierako sinadurak",
                signaturesValue_one: "{{count}}, sinadura-kodea {{code}}",
                signaturesValue_other: "{{count}}, sinadura-kodea {{code}}",
                signers: "Kideek sinatua",
            },
            values: {
                ballots_and_contests: {
                    "first-version": "Lehen bertsioa",
                    "no-changes": "Aldaketarik ez",
                    "changed": "Aldatuta",
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
                    ONLINE: "Linean",
                    KIOSK: "Kioskoa",
                    EARLY_VOTING: "Boto aurreratua",
                    TELEPHONE: "Telefonoa",
                },
                statuses: {
                    NOT_STARTED: "Hasi gabe",
                    OPEN: "Irekita",
                    PAUSED: "Etenda",
                    CLOSED: "Itxita",
                },
                channelStatus: "{{channel}}: {{status}}",
                ruleChange: "{{action}}: {{rule}}",
                ruleChangeFrom: "{{action}}: {{rule}} (lehen {{was}})",
                ruleNeeds: "{{n}} behar ditu",
                ruleOff: "desaktibatuta",
                decision: {
                    approve: "Onartu",
                },
            },
            results: {
                signatures: "Sinadurak",
                needs: "{{n}} behar ditu",
                off: "Desaktibatuta",
                openRequest: "Ireki sinadura-eskaera",
                downloadSigned: "Deskargatu sinatutako PDFa",
                print: "Inprimatu",
                transmit: "Transmititu emaitzak",
                sendTo: "Bidali {{count}} helburutara",
                awaiting: "{{item}}: sinaduren zain",
                transmission: {
                    title: "Sinadurak",
                    description:
                        "Sinatzaile bakoitzak paketearen emaitzak sinatzen ditu bere ziurtagiri digitalarekin, nabigatzaile honetan. Paketea bidali ahal izango da {{n}} pertsonak sinatu ondoren.",
                    waiting:
                        "Paketea bidali ahal izango da bere sinadura-eskaerak sinadura guztiak dituenean.",
                    signed: "Paketeak bere sinadura guztiak ditu eta bidal daiteke.",
                    ended: "Pakete honen sinadura-eskaera amaitu da. Sortu berriro paketea hura sinatzeko.",
                },
            },
            waiting: {
                title: "Nire sinaduraren zain",
                buttonCount_one: "Nire sinaduraren zain: sinatzeko {{count}} eskaera",
                buttonCount_other: "Nire sinaduraren zain: sinatzeko {{count}} eskaera",
                intro: "Sina ditzakezun ekintzen sinaduren zain dauden eskaerak. $t(signing.terms.posts): zureak soilik.",
                close: "Itxi zerrenda",
                empty: "Ez dago ezer zure sinaduraren zain.",
                loadError: "Ezin izan dira kargatu sinaduren zain dauden eskaerak.",
                signedByYou: "Zuk sinatua",
            },
            notes: {
                afterApproval: "Onartu ondoren",
                afterApprovalValue: "Hautesleari kredentzialak igorri eta bidaltzen zaizkio",
                keyShare: "Zure giltza-zatia",
                keyShareChecked: "Egiaztatuta: zeremonia honetarako zure giltza-zatia da",
                recordedIn: "Non jasotzen den",
                recordedInCeremony: "Giltzen zeremonia eta iragarki-taula",
                recordedInTally: "Zenbaketa-saioa",
            },
            keyShare: {
                signing:
                    "Sinatu zure giltza-zatia sinadura-panelean. Sinatu ondoren erregistratzen da.",
                record: "Erregistratu nire giltza-zatia",
                failed: "Ezin izan da zure giltza-zati sinatua erregistratu: {{error}}",
                dropAgain:
                    "Jaregin berriro zure giltza-zatiaren fitxategia zure giltza-zati sinatua erregistratzeko.",
                redo: "Zure giltza-zatia zure sinadurarik gabe eman zen, eta hauteskunde honek orain sinadura behar du. Eman berriro eta sinatu.",
                notTaken:
                    "Zeremoniak ez du giltza-zati hau onartzen jada. Jaregin berriro zure giltza-zatiaren fitxategia.",
            },
        },
        lifecycle: {
            signedClose: {
                title: "Sinatutako itxiera-epea",
                deadline: "{{election}}: {{time}} · {{code}} konfigurazioak baimenduta.",
                explanation:
                    "Sinatutako epe honek agintzen jarraitzen du egutegi editagarria aldatu edo kendu arren. Planifikatzaileak oraindik irekita dauden baimendutako kanalak ixten ditu.",
                reached:
                    "Sinatutako epe hau igaro da. Egiaztatu bozketaren uneko egoera eta auditoretza-erregistroa; prozesatzea ez da oraindik erregistratu.",
                processed: "Sinatutako itxiera-epea {{time}} unean prozesatu da.",
                signedAt: "Sinatutako epea: {{time}}.",
                channels: "Epe honek oraindik hartzen dituen kanalak: {{channels}}.",
                result: "Kontsultatu bozketaren egoera eta auditoretza-erregistroa benetako aldaketak eta emaitza osoa ezagutzeko.",
                unavailable:
                    "Ezin izan dira sinatutako itxiera-epeak kargatu. Egiaztatu argitaratutako egutegia eta auditoretza-erregistroa.",
            },
            picker: {
                noMatch:
                    "Ez dago bat datorren ordu-eremurik. Idatzi hiri, herrialde, eremu, laburdura edo desfase bat.",
            },
            input: {
                timezone: "Ordu-eremua",
                scheduledAt: "Programatutako unea",
                meetingStart: "Bileraren hasiera",
                cronZone: "Programazioa gertaeraren ordu-eremu nagusian exekutatzen da: {{zone}}.",
                unconfiguredZone:
                    "{{zone}} ez da gertaeraren ordu-eremu konfiguratuetako bat. Aukeratu horietako bat.",
            },
            schedule: {
                allElections: "Hauteskunde guztiak",
                outcome: "Emaitza",
                noOffset: "Ordu-eremuaren desfaserik gabe: ez da inoiz exekutatzen",
                unpublished: "Oraindik argitaratu gabe",
                notPublished:
                    "Oraindik ez dago ezer argitaratuta: hautesleek lehen argitalpenaren ondoren ikusiko dute programazioa.",
                unpublishedChanges_one:
                    "Programatutako gertaera {{count}} aldatu da azken argitalpenetik. Hautesleek argitaratu ondoren ikusiko dute.",
                unpublishedChanges_other:
                    "Programatutako {{count}} gertaera aldatu dira azken argitalpenetik. Hautesleek argitaratu ondoren ikusiko dituzte.",
                offsetless_one:
                    "Programatutako ordu {{count}}ek ez du ordu-eremuaren desfaserik, eta beraz ez da inoiz exekutatuko. Editatu bere ordu-eremua ezartzeko.",
                offsetless_other:
                    "Programatutako {{count}} orduk ez dute ordu-eremuaren desfaserik, eta beraz ez dira inoiz exekutatuko. Editatu itzazu beren ordu-eremua ezartzeko.",
                outcomeChange:
                    "Gordetzean, programatutako trantsizio honek egiten duena aldatuko da: {{before}} → {{after}}.",
                outcomeNew: "Gorde ondoren, programatutako trantsizio hau: {{after}}.",
                outcomeElections: "{{total}} hauteskundetatik {{count}}",
                exportError: "Ezin izan da programazioa esportatu.",
                exportFileName: "schedule.csv",
                totals: {
                    refused_one:
                        "Programatutako errenkada {{count}} baztertuko da ({{transitions}} hauteskunde-trantsizio).",
                    refused_other:
                        "Programatutako {{count}} errenkada baztertuko dira ({{transitions}} hauteskunde-trantsizio).",
                    runsUnsigned_one:
                        "Programatutako itxiera {{count}} sinadurarik gabe exekutatuko da ({{transitions}} hauteskunde-trantsizio).",
                    runsUnsigned_other:
                        "Programatutako {{count}} itxiera sinadurarik gabe exekutatuko dira ({{transitions}} hauteskunde-trantsizio).",
                    review: "Berrikusi",
                    showAll: "Erakutsi guztiak",
                    showing: {
                        refused:
                            "Baztertuko diren programatutako {{count}} errenkadak erakusten dira ({{transitions}} hauteskunde-trantsizio).",
                        runsUnsigned:
                            "Sinadurarik gabe exekutatuko diren programatutako {{count}} itxierak erakusten dira ({{transitions}} hauteskunde-trantsizio).",
                    },
                },
                recompute: {
                    title_one:
                        "Ordu-eremuen datu-basearen eguneratze batek etorkizuneko programatutako ordu {{count}} mugitzen du. Ez da ezer aldatuko aplikatzen duzun arte.",
                    title_other:
                        "Ordu-eremuen datu-basearen eguneratze batek etorkizuneko programatutako {{count}} ordu mugitzen ditu. Ez da ezer aldatuko aplikatzen dituzun arte.",
                    change: "{{type}}: {{before}} → {{after}}",
                    apply: "Aplikatu",
                    applied_one: "Programatutako ordu {{count}} eguneratu da.",
                    applied_other: "Programatutako {{count}} ordu eguneratu dira.",
                    error: "Ezin izan dira programatutako orduak eguneratu.",
                },
                outcomeChangeElections_one:
                    "Gordetzeak emaitza aldatzen du {{count}} hauteskundetan:",
                outcomeChangeElections_other:
                    "Gordetzeak emaitza aldatzen du {{count}} hauteskundetan:",
            },
            authorizes: {
                reportPolicyOf: "{{election}}: {{value}}",
                initializationRetained:
                    "Sinatutako konfigurazio honetan derrigorrezkoa den txostenak derrigorrezkoa izaten jarraitzen du, uneko postuaren ezarpena derrigorrezkoa ez izatera aldatzen bada.",
                title: "Onarpen honek baimentzen duena",
                schedule: "Programatutako irekierak eta itxierak",
                noSchedule:
                    "Ez dago programatutako irekierarik edo itxierarik: sinatzaileek irekitzen eta ixten dute bozketa.",
                opens: "Irekiera: {{time}}",
                closes: "Itxiera: {{time}}",
                settings: "Ezarpenak",
                unsignedClose: "Sinadurarik gabeko programatutako itxiera: {{value}}",
                initialization: "Hasieratzea: {{value}}",
                firstConfiguration:
                    "Hau da onartutako lehen konfigurazioa: ez dago zerekin alderatu.",
                sameAsPrevious: "Ezarpenak onartutako aurreko konfigurazioko berberak dira.",
                rule: {
                    openNeeds_one: "Irekitzeko sinadura {{count}} behar da",
                    openNeeds_other: "Irekitzeko {{count}} sinadura behar dira",
                    openNoSignatures: "Irekitzeko ez da sinadurarik behar",
                    closeNeeds_one: "Ixteko sinadura {{count}} behar da",
                    closeNeeds_other: "Ixteko {{count}} sinadura behar dira",
                    closeNoSignatures: "Ixteko ez da sinadurarik behar",
                    openSetting: "Bozketa irekitzea",
                    closeSetting: "Bozketa ixtea",
                    signatures_one: "Sinadura {{count}}",
                    signatures_other: "{{count}} sinadura",
                    none: "sinadurarik ez",
                },
                diff: {
                    tightens: "Zorrozten du: {{setting}} {{before}} → {{after}}",
                    loosens: "Malgutzen du: {{setting}} {{before}} → {{after}}",
                    mixed: "Aldaketak: {{setting}} {{before}} → {{after}} (zorrotzagoa alde batetik, malguagoa bestetik)",
                },
                comparedWith: "Aurreko konfigurazio onartuarekin alderatuta, {{code}} onarpena:",
                channels: "Bozketa-kanalak hauteskundeka",
                channelsOf: "{{election}}: {{channels}}",
                noChannels: "bat ere ez",
            },
            publish: {
                openedAuthorized:
                    "Bozketa programazioaren arabera ireki da {{time}}(e)an, {{code}} konfigurazio-onarpenak baimenduta (sinatzaileak: {{names}}).",
                closedAuthorized:
                    "Bozketa programazioaren arabera itxi da {{time}}(e)an, {{code}} konfigurazio-onarpenak baimenduta (sinatzaileak: {{names}}).",
                closedUnsigned:
                    "Bozketa programazioaren arabera itxi da {{time}}(e)an. Itxierako sinadurarik ez: programazioak itxi du bozketa epemugan.",
                authorizedBy: "Baimena eman du",
                cancelledRequest:
                    "{{code}} eskaerak {{k}} sinaduratik {{n}} zituen, eta bertan behera utzi da.",
                openedRefused: "{{time}}(e)ko irekiera programatua baztertu da.",
                closedRefused: "{{time}}(e)ko itxiera programatua baztertu da.",
                openedNoSignaturesNeeded:
                    "Bozketa programazioaren arabera ireki da ({{time}}); ez zen sinadurarik behar.",
                closedNoSignaturesNeeded:
                    "Bozketa programazioaren arabera itxi da ({{time}}); ez zen sinadurarik behar.",
                openedNothingToChange:
                    "{{time}}(e)an irekiera programatuak ez zuen ezer irekitzeko: bere kanalak irekita zeuden jada.",
                closedNothingToChange:
                    "{{time}}(e)an itxiera programatuak ez zuen ezer ixteko: bere kanalak itxita zeuden jada.",
            },
            import: {
                title: "Inportatu programazioa",
                subtitle:
                    "Errenkada bat gertaera eta hauteskunde bakoitzeko, tokiko orduan. Utzi ordu-eremua hutsik hauteskundearen ordu-eremua erabiltzeko.",
                chooseFile: "Aukeratu CSV fitxategi bat",
                template: "Deskargatu txantiloia",
                templateFileName: "schedule-template.csv",
                ready: "{{ok}} gertaera prest {{posts}} hauteskundetarako.",
                needsAttention_one:
                    "{{ok}} gertaera prest {{posts}} hauteskundetarako. Errenkada {{count}} berrikusi behar da; zuzendu fitxategia eta igo berriro.",
                needsAttention_other:
                    "{{ok}} gertaera prest {{posts}} hauteskundetarako. {{count}} errenkada berrikusi behar dira; zuzendu fitxategia eta igo berriro.",
                preview: "Inportatu beharreko errenkadak",
                row: "Errenkada",
                asWritten: "{{local}} · {{place}}",
                moreRows: "…eta beste {{count}} errenkada",
                imported:
                    "Programazioa inportatu da: {{created}} sortuta, {{updated}} eguneratuta.",
                uploadError: "Ezin izan da fitxategia egiaztatu. Igo berriro.",
                importError: "Ezin izan da programazioa inportatu.",
                error: {
                    unknownElection: "Ez dago {{election}} aliasa duen hauteskunderik.",
                    unknownEventType: "{{type}} ez da programatutako gertaera mota bat.",
                    invalidTimeZone: "{{zone}} ez da ordu-eremu bat.",
                    invalidDateTime: "Data eta orduak YYYY-MM-DDTHH:MM formatua izan behar du.",
                    invalidVotingChannels:
                        "Bozketa-kanalak ezezagunak dira, edo Online bozketa eta Aldez aurreko bozketa batera irekitzen dituzte.",
                    dstGap: "{{dateTime}} ez da existitzen {{city}}(e)n, erlojuak aurrera egiten duelako. Idatzi existitzen den ordu bat.",
                    duplicate:
                        "Beste errenkada batek gertaera bera programatzen du hauteskunde honetarako.",
                    other: "Ezin da errenkada hau inportatu ({{code}}).",
                    ambiguousElection: "Hauteskunde batek baino gehiagok du {{election}} aliasa.",
                },
            },
            settings: {
                accordion: "Hizkuntza, data eta ordua",
                dateAndTime: "Data eta ordua",
                configured: "Konfiguratutako ordu-eremuak",
                configuredHelp:
                    "{{count}} ordu-eremu. Hauteskundeek zerrenda honetatik aukeratzen dute berea; idatzi hiri edo herrialde bat ordu-eremu bat gehitzeko.",
                moreZones: "+{{count}}",
                primary: "Ordu-eremu nagusia",
                primaryHelp:
                    "Gertaera osoko programazioetarako, txostenetarako eta ordu-eremu propiorik ez duten hauteskundeetarako erabiltzen da.",
                primaryInUse:
                    "{{zone}} da ordu-eremu nagusia. Aukeratu lehenik beste ordu-eremu nagusi bat.",
                inUse: "{{names}}(e)k erabiltzen du {{zone}}. Aldatu lehenik hauteskunde horiek.",
                logs: "Orduak erregistroetan eta erregistroen esportazioetan",
                logsPrimary: "Ordu-eremu nagusia ({{abbr}})",
                logsElection: "Errenkada bakoitzeko hauteskundearen ordu-eremua",
                logsHelp: "Hauteskunderik gabeko errenkadek ordu-eremu nagusia erabiltzen dute.",
                electionZone: "Ordu-eremua",
                electionPrimary: "Gertaeraren nagusia: {{zone}}",
                electionZoneHelp:
                    "Hauteskunde honen programazioek, hautesleen pantailek eta txostenek ordu-eremu hau erabiltzen dute, haren azpiko eremu guztiak barne. Hutsik badago, gertaeraren ordu-eremu nagusia erabiltzen da.",
                electionUnconfigured:
                    "Gertaerak jada ez du ordu-eremu hau konfiguratzen; beraz, hauteskundeak ordu-eremu nagusia erabiltzen du: {{zone}}. Aukeratu konfiguratutako ordu-eremuetako bat.",
                electionUnconfiguredSave: "Aukeratu gertaeraren ordu-eremu konfiguratuetako bat.",
            },
            policies: {
                accordion: "Bozketaren bizi-zikloa",
                intro: "Ezarpen hauek hauteskunde-gertaeraren konfigurazioaren parte dira: konfigurazio-onarpenak sinatzen ditu, eta programatutako irekierek eta itxierek uneko ezarpenen eta argitaratutakoen artean zorrotzenari jarraitzen diote.",
                nothingPublished:
                    "Oraindik ez dago ezer argitaratuta: lehen argitalpenera arte, programatutako irekierek eta itxierek balio lehenetsiak erabiltzen dituzte (hauteskunde bakoitzeko, ukatu).",
                publishedValue: "Argitaratutako konfigurazioa: {{value}}",
                changedSincePublished:
                    "Argitaratutako konfiguraziotik aldatu da: programatutako irekierek eta itxierek bien artean zorrotzena jarraitzen dute hurrengo argitalpen onartura arte.",
                scope: {
                    title: "Hasieratzea bozketa ireki aurretik",
                    post: {
                        label: "Hauteskunde bakoitzeko",
                        help: "Hauteskunde bat hasieratuta dagoenean irekitzen da.",
                    },
                    event: {
                        label: "Gertaera osoa",
                        help: "Ez da hauteskunderik irekitzen hauteskunde guztiak hasieratu arte.",
                        warning:
                            "Hasieratu gabeko hauteskunde bakar batek hauteskunde guztiak itxita mantentzen ditu, baita programatutako irekieretan ere.",
                    },
                    postAndCountry: {
                        label: "Hauteskunde eta herrialde bakoitzeko",
                        help: "Hauteskunde bat irekitzen da haren azpiko herrialde (eremu) guztiak hasieratuta daudenean.",
                        warning:
                            "Hauteskunde bat itxita geratzen da, baita programatutako irekieran ere, haren azpiko herrialde guztiak hasieratu arte; herrialde bakoitza bere txostenarekin hasieratzen da.",
                    },
                },
                close: {
                    title: "Sinadurarik gabeko programatutako itxiera",
                    help: "Bozketa ixteko sinadurak behar direnean eta programatutako itxiera bat sinatutako konfigurazioan ez dagoenean.",
                    refuse: {
                        label: "Ukatu",
                        help: "Itxiera ez da exekutatzen; hauteskundearen sinatzaileek ixten dute bozketa beren sinadurekin.",
                    },
                    runAsSystem: {
                        label: "Exekutatu sistema gisa",
                        help: "Bozketa epemugan ixten da, eta programazioak sinadurarik gabe itxi duela erregistratzen da.",
                        warning:
                            "Sinatutako konfiguraziotik kanpoko programatutako itxierek inoren sinadurarik gabe ixten dute bozketa. Erregistroak eta dokumentuek hala adierazten dute.",
                    },
                },
                onSave: {
                    outcomes_zero:
                        "Programatutako trantsizio batek ere ez du bere emaitza aldatzen.",
                    outcomes_one:
                        "Programatutako trantsizio {{count}}ek bere emaitza aldatzen du. Berrikusi Programatutako Gertaerak atalean.",
                    outcomes_other:
                        "Programatutako {{count}} trantsiziok beren emaitza aldatzen dute. Berrikusi Programatutako Gertaerak atalean.",
                },
                saveError: "Ezin izan dira bozketaren bizi-zikloaren ezarpenak gorde.",
                publishedPerTarget: "Argitaratutako konfigurazioa, helburuka: {{values}}",
                publishedCount_one: "{{value}} ({{count}} helburu)",
                publishedCount_other: "{{value}} ({{count}} helburu)",
                savedWithoutPolicies:
                    "Hauteskunde-gertaera gorde da, baina bozketaren bizi-zikloaren ezarpenak ez: {{reason}}. Gorde berriro.",
            },
        },
        scheduledOutcome: {
            chip: {
                waitingForInitialization: "Hasieratzearen zain",
                runs: "Exekutatuko da",
                runsUnsigned: "Sinadurarik gabe exekutatuko da",
                refused: "Ukatuko da",
            },
            note: {
                waitingForInitialization: "Hasieratzearen zain",
                authorized: "{{code}} konfigurazioak baimenduta",
                noSignaturesNeeded: "Ez da sinadurarik behar",
                closesUnsigned: "Sinadurarik gabe ixten da",
                refused: {
                    initialization: "Beharrezko hasieratzea osatu gabe dago",
                    votingClose: "Bozketa ezin da itxiera-epearen ondoren ireki",
                    needsSignatures: "Sinatzaileen sinadurak behar ditu",
                    covered: "Ez dago sinatutako konfigurazioan",
                    unsignedClose: "Sinadurarik gabeko itxiera baztertu egiten da",
                    stricterCopy:
                        "Argitaratutako konfiguraziotik aldatu da, eta hark erabakitzen du oraindik",
                    defaults: "Oraindik ez da ezer argitaratu: balio lehenetsiak aplikatzen dira",
                },
                refusedWithStep: "{{reason}}. {{next}}",
            },
            why: {
                button: "Zergatik?",
                title: {
                    waitingForInitialization: "Zergatik dago hasieratzearen zain",
                    runs: "Zergatik exekutatuko den",
                    runsUnsigned: "Zergatik exekutatuko den sinadurarik gabe",
                    refused: "Zergatik ukatuko den",
                },
                checks: "Egiaztapenak",
                check: "Egiaztapena",
                current: "Uneko ezarpenak",
                published: "Argitaratutako konfigurazioa",
                verdict: "Ebazpena",
                allows: "Baimentzen du",
                blocks: "Blokeatzen du",
                deciding: "Egiaztapen erabakigarria",
                nextStep: "Hurrengo urratsa:",
                signedBy: "Sinatzaileak: {{names}}",
            },
            question: {
                initialization: "Beharrezko hasieratzea osatu al da?",
                votingClose: "Irekiera honek bozketaren itxiera-epea errespetatzen al du?",
                needsSignatures: "Ekintza honek sinadurak behar ditu?",
                covered: "Programazio zehatz hau sinatutako konfigurazioan dago?",
                unsignedClose: "Zer gertatzen da sinadurarik gabeko itxiera batekin?",
                stricterCopy:
                    "Uneko ezarpenak eta argitaratutakoak desberdinak dira? Zeinek erabakitzen du?",
                defaults: "Ba al dago ezer argitaratuta?",
            },
            check: {
                initialization: {
                    waiting:
                        "Uneko eta argitaratutako ezarpenek eskatutako hasieratze guztiak osatu behar dira.",
                },
                votingClose: {
                    passed: "Bozketa {{closes_at}} unean ixten da; irekiera hau ezin da une horretan edo geroago exekutatu.",
                },
                needsSignatures: {
                    yes: "Bai, {{signatures}} sinadura",
                    yes_one: "Bai, sinadura {{count}}",
                    yes_other: "Bai, {{count}} sinadura",
                    no: "Ez",
                },
                covered: {
                    overriddenBySignedPostRow:
                        "Sinatutako {{code}} konfigurazioak postu honen {{scheduled_event_id}} irekiera propioa erabiltzen du. Ekitaldi osorako irekiera ez da aplikatzen.",
                    yes: "Bai: {{code}} onarpena, aldatu gabe",
                    changed: "Ez: aldatu egin da {{code}} onarpenetik",
                    changedBy:
                        "Ez: {{edited_by}}(e)k editatu du {{edited_at}}(e)an, {{code}} onarpenaren ondoren",
                    notInApproval: "Ez: {{code}} onarpenak ez du barne hartzen",
                    noApproval: "Oraindik ez dago onartutako konfiguraziorik",
                    channelsChanged:
                        "Ez: hauteskundearen bozketa-kanalak aldatu dira {{code}} onarpenetik",
                    alreadyFired:
                        "Ez: {{code}} onarpenaren trantsizio hau {{fired_at}}(e)an exekutatu zen jada; berriro exekutatzeko sinadurak behar dira",
                    late: "Ez: 15 minutu baino gehiago igaro dira {{scheduled_date}}(e)tik ({{code}} onarpena); orain exekutatzeko sinadurak behar dira",
                },
                unsignedClose: {
                    refuse: "Ukatu",
                    runAsSystem: "Exekutatu sistema gisa",
                },
                stricterCopy: {
                    same: "Biak berdinak dira",
                    currentStricter: "Uneko ezarpenak zorrotzagoak dira: orain aplikatzen dira",
                    currentLooser:
                        "Uneko ezarpenak malguagoak dira: onartutako hurrengo argitalpenaren ondoren aplikatuko dira",
                    combined: "Bakoitza zorrotzagoa da balio batean: biak aplikatzen dira",
                },
                defaults: {
                    published: "{{published_at}}(e)an argitaratua",
                    nothingPublished:
                        "Ez dago ezer argitaratuta: balio lehenetsiak aplikatzen dira",
                    noSnapshot:
                        "{{published_at}}(e)an argitaratua, argitalpenek ezarpen hauek gorde aurretik: balio lehenetsiak aplikatzen dira",
                },
            },
            nextStep: {
                initialize:
                    "Osatu beharrezko hasieratzea. Planifikatzailea berriro saiatuko da bozketa itxi aurretik.",
                closed: "Irekiera hau ez da exekutatuko bozketa itxi ondoren.",
                none: "Ez da ekintzarik behar.",
                publishAndApprove: "Argitaratu eta onartu konfigurazioa.",
                requireConfigurationApproval:
                    "Ezarri Onartu konfigurazioa ekintzak sinadurak behar ditzan, eta ondoren argitaratu eta onartu konfigurazioa.",
                askSignersToOpen: "Eskatu hauteskundearen sinatzaileei bozketa irekitzeko.",
                askSignersToClose: "Eskatu hauteskundearen sinatzaileei bozketa ixteko.",
            },
            applies: {
                tightens: "Orain aplikatzen da eskuzko eta programatutako ekintzetan.",
                loosens:
                    "Orain aplikatzen da eskuzko ekintzetan; programatutako irekieretan eta itxieretan, onartutako hurrengo argitalpenaren ondoren.",
                tightensAndLoosens:
                    "Haren alde zorrotzagoa orain aplikatzen da eskuzko eta programatutako ekintzetan; alde malguagoa orain aplikatzen da eskuzko ekintzetan, eta programatutako irekieretan eta itxieretan onartutako hurrengo argitalpenaren ondoren.",
            },
        },
        messagingEvent: {
            tab: "Mezularitza",
            intro: "Gertaera honetako hautesleek kodeetarako eta jakinarazpenetarako aukera ditzaketen kanalak, eta bakoitzak zein kontutatik bidaltzen duen. Kontuak Ezarpenak > Mezularitza atalean kudeatzen dira.",
            readOnly:
                "Ezarpen hauek ikus ditzakezu. Aldatzeko messaging-config-write baimena behar da.",
            savingNote:
                "Gordetzean, izen-emate orriek Post bakoitzean eskaintzen dituzten kanalak ere eguneratzen dira.",
            save: "Gorde",
            saved: "Mezularitza-ezarpenak gorde dira.",
            saveRejected: "Ez dira mezularitza-ezarpenak gorde. Konpondu adierazitako arazoak.",
            saveError: "Ezin izan dira mezularitza-ezarpenak gorde.",
            accountLabel: "{{channel}} kontua",
            notUsed: "Erabili gabe",
            missingAccount: "Ez da kontua aurkitu",
            noAccount: "Gehitu lehenik kontu bat Ezarpenak > Mezularitza atalean",
            missing: "Falta da: {{blockers}}",
            purposeSwitch: "{{channel}}: {{purpose}}",
            sections: {
                channels: "Kanalak",
                templates: "Txantiloi onartuak",
                fallback: "Jakinarazpenen ordezko ordena",
                posts: "Kanalak Post-eko",
                postsCount: "Kanalak Post-eko ({{count}} Post)",
                reply: "Sarrerako mezuei erantzuna",
                delivery: "Entregaren egoera",
            },
            column: {
                channel: "Kanala",
                account: "Nondik bidaltzen du",
                purpose: "Helburua",
                language: "Hizkuntza",
                template: "Hornitzailearen txantiloia",
                status: "Egoera",
                post: "Post",
                key: "Mezu honetarako",
                providerLanguage: "Hornitzailearen hizkuntza",
            },
            outOfWindow: {
                label: "Elkarrizketa-leihotik kanpo",
                help: "Testu libreko jakinarazpenak elkarrizketa-leihoa irekita dagoen bitartean soilik bidaltzen dira: Messenger-en, hauteslearen azken mezutik 24 orduko epean. Aukeratu Utilitate-mezuak jakinarazpenak geroago txantiloi onartu batekin bidaltzeko. Meta-k onartu behar du orriarentzat (page_utility_messaging baimena eta onartutako UTILITY txantiloi bat), eta txantiloi horrek jakinarazpenei lotuta egon behar du Txantiloi onartuak atalean. Ez bidali aukerarekin, leihotik kanpoko jakinarazpena hauteslearen hurrengo kanal erabilgarrira doa.",
                DISABLED: "Ez bidali",
                UTILITY_MESSAGES: "Utilitate-mezuak",
                noTemplate:
                    "Oraindik ez dago txantiloirik lotuta kanal honetako jakinarazpenei. Gehitu bat Txantiloi onartuak atalean; bitartean, leihotik kanpoko jakinarazpenak hauteslearen hurrengo kanal erabilgarrira doaz.",
            },
            templates: {
                empty: "Aukeratu txantiloi onartuak bidaltzen dituen kontu bat, hala nola WhatsApp, Viber edo Messenger, bere txantiloiak hemen lotzeko.",
                help: "Errenkada bakoitzak adierazten du hornitzaileak zein txantiloi onartu bidaltzen duen mezu baterako. Mezu honetarako da Txantiloiak ataleko txantiloi baten aliasa, jakinarazpen baterako, edo Keycloak-ek bidaltzen duen mezu-gakoa, adibidez otp; utzi hutsik helbururako lehenespenez erabiltzen den txantiloirako. Hizkuntza hauteslearen hizkuntza da. Hornitzailearen txantiloia txantiloiak hornitzailean duen izena edo IDa da. Hornitzailearen hizkuntza hornitzaileak txantiloi horretarako duen kodea da, hauteslearen hizkuntzatik desberdina denean: WhatsApp-ek txantiloi onartuaren kode zehatza behar du, adibidez en_US.",
                order: "Mezu bakoitzerako errenkada zehatzenak irabazten du: mezuaren errenkada hauteslearen hizkuntzan, gero mezuaren errenkada edozein hizkuntzatan, gero helbururako lehenetsia hauteslearen hizkuntzan, eta azkenik helbururako edozein lehenetsi.",
                noneRequired:
                    "{{channel}} kanalak txantiloi onartuak soilik bidaltzen ditu. Gehitu gutxienez txantiloi lehenetsi bat erabiltzen den helburu bakoitzerako.",
                noneOptional:
                    "Ez dago txantiloirik lotuta {{channel}} kanalerako. Elkarrizketa-leihotik kanpo jakinarazpenak bidaltzeko soilik behar dira.",
                row: "{{channel}} txantiloia, {{position}}.",
                keyDefault: "Helbururako lehenetsia",
                add: "Gehitu {{channel}} txantiloia",
                remove: "Kendu {{channel}} txantiloia, {{position}}.",
                incomplete:
                    "Idatzi hizkuntza eta hornitzailearen txantiloia, edo kendu errenkada hau.",
                approval: {
                    APPROVED: "Onartua",
                    NOT_APPROVED: "Onartu gabea",
                    ADMIN_CONFIRMED: "Administratzaile batek berretsia",
                    NOT_CHECKED: "Onarpena egiaztatu gabe",
                },
            },
            fallback: {
                help: "Jakinarazpen batek hautesle bat bere kanalean iritsi ezin duenean, hautesleak egiaztatu duen eta bere Post-ak eskaintzen duen ordena honetako hurrengo kanalera doa. Kodeak ez dira inoiz berez berriro bidaltzen: hautesleak beste modu bat aukeratzen du.",
                empty: "Aktibatu kanal baten jakinarazpenak ordezko ordenara gehitzeko.",
                earlier: "Eraman {{channel}} aurrerago",
                later: "Eraman {{channel}} atzerago",
            },
            posts: {
                noChannels:
                    "Aktibatu kanal baten kodeak edo jakinarazpenak Post bakoitzaren kanalak aukeratzeko.",
                help: "Izen-emateak Post bakoitzeko hautesleei hemen markatutako kanalak erakusten dizkie.",
                restricted:
                    "{{count}} Post-ek {{total}} kanalak baino gutxiago eskaintzen dituzte.",
                allChannels:
                    "Post guztiek {{total}} kanalak eskaintzen dituzte; desmarkatu kanal bat funtzionatzen ez duen Post-ean.",
                search: "Bilatu Post-ak",
                cell: "{{post}}: {{channel}}",
                showing:
                    "{{total}} Post-etatik {{shown}} erakusten dira. Bilatu besteak aurkitzeko.",
            },
            reply: {
                help: "Hautesle batek gertaera honetako kontu batera idazten duenean bidaltzen da, gehienez egunean behin hautesleko.",
                label: "Erantzuna ({{language}})",
            },
            delivery: {
                empty: "Ez da kanalik erabiltzen.",
                help: "Onartua esan nahi du hornitzaileak eskaera onartu duela, ez hautesleak kodea jaso edo egiaztatu duela. Ezezaguna esan nahi du entrega ez dagoela oraindik berretsita. Entrega-txostenik gabeko hornitzaileak entrega ez erabilgarri gisa erakusten du.",
            },
            error: {
                UNSUPPORTED_VERSION:
                    "Konfigurazio honek {{version}} bertsioa erabiltzen du, eta ez da onartzen.",
                DUPLICATE_CHANNEL: "{{channel}} behin baino gehiagotan dago konfiguratuta.",
                UNKNOWN_ACCOUNT: "{{channel}} kontua ez dago jada. Aukeratu beste kontu bat.",
                ACCOUNT_OF_ANOTHER_TENANT: "Hautatutako kontua beste maizter batena da.",
                ACCOUNT_CHANNEL_MISMATCH:
                    "Hautatutako kontuak ez du {{channel}} mezurik bidaltzen.",
                PURPOSE_NOT_READY:
                    "{{channel}}-ek ezin ditu oraindik {{purpose}} bidali. Falta da: {{blockers}}.",
                TEMPLATE_NOT_APPROVED:
                    "{{channel}} txantiloia ({{purpose}}, {{language}}) ez dago hornitzaileak onartuta.",
                OUT_OF_WINDOW_NOT_SUPPORTED:
                    "{{channel}} kanalak ezin du elkarrizketa-leihotik kanpo bidali kontu honekin: ez du elkarrizketa-leihorik, edo bere jakinarazpenek dagoeneko txantiloi bat behar dute.",
                FALLBACK_CHANNEL_NOT_ENABLED:
                    "{{channel}} ordezko ordenan dago baina ez du jakinarazpenik bidaltzen.",
                DUPLICATE_FALLBACK_CHANNEL:
                    "{{channel}} behin baino gehiagotan dago ordezko ordenan.",
                ELECTION_CHANNEL_NOT_ENABLED:
                    "{{election}}-ek {{channel}} eskaintzen du, eta gertaera honek ez du erabiltzen.",
                UNKNOWN_ELECTION: "{{election}} ez da gertaera honetako hauteskunde bat.",
            },
        },
        messaging: {
            channel: {
                EMAIL: "Posta elektronikoa",
                SMS: "SMS",
                WHATSAPP: "WhatsApp",
                VIBER: "Viber",
                MESSENGER: "Facebook Messenger",
            },
            provider: {
                AWS_SES: "Amazon SES",
                SMTP: "SMTP zerbitzaria",
                AWS_SNS: "Amazon SNS",
                WHATSAPP_CLOUD_API: "WhatsApp Cloud API (Meta)",
                MESSENGER_SEND_API: "Messenger Platform (Meta)",
                VIBER_INFOBIP: "Viber Business Messages (Infobip)",
                CONSOLE: "Kontsola (probak soilik, ez da ezer bidaltzen)",
                HTTP_API: "HTTP API pertsonalizatua",
            },
            purpose: {
                OTP: "Kodeak",
                NOTICE: "Jakinarazpenak",
            },
            state: {
                QUEUED: "Ilaran",
                ACCEPTED: "Onartua",
                DELIVERED: "Entregatua",
                FAILED: "Huts egin du",
                UNKNOWN: "Ezezaguna",
            },
            stateHelp: {
                QUEUED: "Hornitzaileari emateko zain.",
                ACCEPTED:
                    "Hornitzaileak mezua onartu du. Horrek ez du esan nahi hautesleak jaso duenik.",
                DELIVERED: "Hornitzaileak mezua entregatu dela jakinarazi du.",
                FAILED: "Hornitzaileak berretsi du mezua ez dela entregatu.",
                UNKNOWN: "Entrega ez dago oraindik berretsita.",
            },
            blocker: {
                NOT_CONNECTED: "Konektatu gabe",
                UNSUPPORTED_PURPOSE: "Hornitzaile honek ez du onartzen",
                NEEDS_PROVIDER_APPROVAL: "Hornitzailearen onarpena behar du",
                NEEDS_PRODUCTION_ACCESS: "Ekoizpen-sarbidea behar du",
                NEEDS_APPROVED_TEMPLATE: "Txantiloi onartua behar du",
            },
            readiness: {
                connected: "Konektatuta",
                notConnected: "Konektatu gabe",
                readyOtp: "Kodeetarako prest",
                readyNotice: "Jakinarazpenetarako prest",
                notReady: "Ez dago prest",
                lastCheck: "Egiaztatua: {{date}}",
                neverChecked: "Oraindik egiaztatu gabe",
                adminConfirmed: "Administratzaile batek berretsia",
                checkNotUsed: "Egiaztapena ez da erabiltzen",
            },
            approval: {
                PENDING: "Hornitzailearen onarpenaren zain",
                CONFIRMED: "Hornitzailearen onarpena berretsita",
            },
            credential: {
                ACCESS_TOKEN: "Sarbide-tokena",
                APP_SECRET: "Aplikazioaren sekretua",
                VERIFY_TOKEN: "Egiaztatze-tokena",
                API_KEY: "API gakoa",
                SMTP_PASSWORD: "Pasahitza",
                AWS_ACCESS_KEY_ID: "AWS sarbide-gakoaren IDa",
                AWS_SECRET_ACCESS_KEY: "AWS sarbide-gako sekretua",
                API_SECRET: "APIaren sekretua",
                USERNAME: "Erabiltzaile-izena",
                PASSWORD: "Pasahitza",
                WEBHOOK_SECRET: "Webhookaren sekretua",
            },
            deliveryUnavailable: "Entrega ez dago erabilgarri",
            templates: {
                noMethod: "Aukeratu gutxienez metodo bat txantiloirako.",
                parameters: "Txantiloiaren parametroak",
                parametersHelp:
                    "Txantiloi onartuaren leku-marka bakoitza zerk betetzen duen, ordenan, adibidez user.first_name edo vote_url. Parametro izendunak dituen txantiloi baterako idatzi @izena=balioa, adibidez @first_name=user.first_name; beste edozein sarrera posizionala da.",
                parameter: "{{position}}. parametroa",
                removeParameter: "Kendu {{position}}. parametroa",
                addParameter: "Gehitu parametroa",
                noAccount:
                    "Oraindik ez dago {{channel}} konturik. Gehitu bat Ezarpenak > Mezularitza atalean zein hizkuntza dauden onartuta ikusteko.",
                account: "Kontua",
                approvalTitle: "Txantiloi onartuak",
                language: "Hizkuntza",
                approvalFor: "Onartua honetarako: {{purpose}}",
                approved: "Onartua",
                notApproved: "Onartu gabea",
                approvalHelp:
                    "Onarpenak hornitzailetik datoz eta kontuaren konexio-egiaztapenak eguneratzen ditu.",
                messengerIntro:
                    "Hautesleak azken mezua bidali eta 24 orduko epean, Messenger-ek beheko testua bidaltzen du.",
                messengerMessage: "24 orduko epeko mezua",
                messengerWindow:
                    "Gordetako Messenger hartzaile bat ez da bidaltzeko baimena. 24 orduko leihotik kanpo jakinarazpen hau utilitate-mezu gisa bidaltzen da, hauteskunde-gertaerak baimentzen duenean eta txantiloi onartu bat behean ezarrita edo gertaeran lotuta dagoenean; bestela, hauteslearen hurrengo kanal erabilgarrira doa. Utilitate-mezuek page_utility_messaging baimena eta onartutako UTILITY txantiloi bat behar dituzte orrian.",
                intro: {
                    WHATSAPP:
                        "WhatsApp-ek Meta-k WhatsApp Business kontuarentzat onartutako txantiloiak soilik bidaltzen ditu. Mezuak txantiloi onartuarekin bat etorri behar du; aukeratu zerk betetzen dituen parametroak.",
                    VIBER: "Viber-ek kodeak eta transakzio-mezuak Viber bazkideak onartutako txantiloiekin soilik bidaltzen ditu. Mezuak txantiloi onartuarekin bat etorri behar du; aukeratu zerk betetzen dituen parametroak.",
                },
                approvedWording: "Testu onartua",
                approvedWordingHelp:
                    "Txantiloi onartuaren kopia bat, aurrebista gisa erabilia. Hemen aldatzeak ez du aldatzen hornitzaileak bidaltzen duena.",
                providerTemplateTitle: "Hornitzailearen txantiloia",
                providerTemplateHelp:
                    "Aukerakoa. Txantiloi onartuak hornitzailean duen izena edo IDa. Hutsik badago, txantiloi honen aliasari lotutako hauteskunde-gertaeraren txantiloia erabiltzen da, edo gertaeraren helbururako lehenetsia.",
                providerTemplate: "Hornitzailearen txantiloiaren izena edo IDa",
                providerLanguage: "Hornitzailearen hizkuntza-kodea",
                providerLanguageHelp: {
                    WHATSAPP:
                        "Onartutako WhatsApp txantiloiaren hizkuntza-kode zehatza, adibidez en_US.",
                    VIBER: "Viber hornitzaileak txantiloia ezagutzeko erabiltzen duen hizkuntza-kodea, behar duenean.",
                    MESSENGER:
                        "Onartutako utilitate-txantiloiaren hizkuntza-kodea, adibidez en_US.",
                },
                approvalAdminConfirmed:
                    "Administratzaile batek hornitzailearekin berretsi du kontu honen txantiloiak onartuta daudela; beraz, konexio-egiaztapenaren onarpenak ez dira erabiltzen.",
            },
            send: {
                channel: "Kanala",
                eachVoter: "Hautesle bakoitzaren kanala",
                only: "{{channel}} soilik",
                eachVoterHelp:
                    "Berretsitako hutsegiteek hurrengo kanal egiaztatu erabilgarria erabiltzen dute. Berretsi gabeko entrega Ezezaguna gisa erakusten da.",
                onlyHelp: "Jakinarazpen hau hautesle bakoitzari {{channel}} bidez bidaltzen zaio.",
                channelColumn: "Kanala",
                sendsFrom: "Honetatik bidaltzen da",
                noAccount: "Konturik ez",
                missingContent: "Jakinarazpen honek ez du edukirik honetarako: {{channels}}.",
                approvedTemplateHelp:
                    "Hornitzaileak onartutako txantiloiarekin bidaltzen da. Editatu Txantiloiak atalean.",
                providerTemplate: "{{channel}} hornitzailearen txantiloia",
                providerTemplateHelp:
                    "Aukerakoa. Hutsik badago, aukeratutako txantiloiaren aliasari lotutako gertaeraren txantiloia erabiltzen da, edo gertaeraren jakinarazpenetarako lehenetsia.",
                providerLanguage: "{{channel}} hornitzailearen hizkuntza",
                providerLanguageHelp:
                    "Hornitzaileak txantiloi horretarako duen hizkuntza-kodea, adibidez en_US.",
            },
            voter: {
                title: "Mezularitza",
                preferredChannel: "Kanal hobetsia",
                whatsappNumber: "WhatsApp zenbakia",
                viberNumber: "Viber zenbakia",
                messengerConnected: "Konektatuta",
                messengerNotConnected: "Konektatu gabe",
                verifiedChannels: "Kanal egiaztatuak",
                noneVerified: "Ez dago kanal egiaztaturik",
                notSet: "Ezarri gabe",
            },
            logs: {
                channel: "Kanala",
            },
            stats: {
                sent: {
                    WHATSAPP: "Bidalitako WhatsApp mezuak",
                    VIBER: "Bidalitako Viber mezuak",
                    MESSENGER: "Bidalitako Messenger mezuak",
                },
            },
            readinessPolicy: {
                PROVIDER_CHECK: "Hornitzailearen egiaztapenaren arabera",
                ADMIN_CONFIRMED: "Administratzaile batek berretsia",
            },
        },
        messagingAccounts: {
            tab: "MEZULARITZA",
            description:
                "Hautesleei beren kodeak eta jakinarazpenak bidaltzen dizkieten kontuak. Hauteskunde-gertaera bakoitzak kanal bakoitzeko kontua aukeratzen du; gertaera berriak kontu lehenetsiarekin hasten dira.",
            list: {
                title: "Bidalketa-kontuak",
                add: "Gehitu kontua",
                loading: "Kontuak kargatzen",
                loadError: "Ezin izan dira bidalketa-kontuak kargatu.",
                empty: "Oraindik ez dago bidalketa-konturik.",
            },
            column: {
                channel: "Kanala",
                name: "Kontua",
                sender: "Honela bidaltzen du",
                provider: "Hornitzailea",
                default: "Lehenetsia",
                isDefault: "Kontu lehenetsia",
                lastCheck: "Azken egiaztapena",
                actions: "Ekintzak",
            },
            action: {
                edit: "Editatu",
                editNamed: "Editatu {{name}}",
                view: "Ikusi",
                viewNamed: "Ikusi {{name}}",
                check: "Egiaztatu konexioa",
                checkNamed: "Egiaztatu {{name}} kontuaren konexioa",
                test: "Bidali proba-mezua",
                testNamed: "Bidali proba-mezu bat {{name}} kontutik",
                delete: "Ezabatu",
                deleteNamed: "Ezabatu {{name}}",
            },
            check: {
                done: "{{name}} egiaztatu da. Bere egoera eguneratuta dago.",
                error: "Ezin izan da {{name}} egiaztatu.",
            },
            delete: {
                title: "Ezabatu kontua",
                body: "{{name}} ezabatu? Erabiltzen duten hauteskunde-gertaerek bere kanaletik bidaltzeari utziko diote.",
                success: "Kontua ezabatu da",
                error: "Ezin izan da kontua ezabatu.",
            },
            editor: {
                addTitle: "Gehitu kontua",
                editTitle: "Editatu {{channel}} kontua",
                subtitle:
                    "Hautesleek kontu honen kodeak eta jakinarazpenak jasotzen dituzte hura erabiltzen duten kanaletan.",
                channel: "Kanala",
                provider: "Hornitzailea",
                save: "Gorde",
                cancel: "Utzi",
                close: "Itxi",
                channelHelp: "Ezin da aldatu kontua sortu ondoren.",
            },
            field: {
                name: "Kontuaren izena",
                from_address: "Igorlearen helbidea",
                from_name: "Igorlearen izena",
                region: "AWS eskualdea",
                notification_topic_arn: "Entrega-jakinarazpenen gaia (SNS ARN)",
                server_url: "Zerbitzaria eta ataka",
                sender_id: "Igorlearen IDa",
                origination_number: "Jatorri-zenbakia",
                business_account_id: "WhatsApp Business kontuaren IDa",
                phone_number_id: "Telefono-zenbakiaren IDa",
                display_phone_number: "Zenbakia",
                display_name: "Bistaratzeko izena",
                api_version: "Graph API bertsioa",
                page_id: "Facebook orriaren IDa",
                page_name: "Orriaren izena",
                page_username: "Orriaren erabiltzaile-izena",
                base_url: "APIaren oinarrizko URLa",
                sender: "Igorlearen izena",
                provider_approval: "Hornitzailearen onarpena",
                is_default: "{{channel}} kontu lehenetsia hauteskunde-gertaera berrietarako",
                readiness: "Prestasuna",
                api_base_url: "Graph API oinarrizko URLa",
                label: "Hautesleei erakusten zaien igorlea",
            },
            fieldHelp: {
                from_address:
                    "Hautesleek ikusten duten helbidea. Bere domeinuak hornitzailearekin egiaztatuta egon behar du.",
                notification_topic_arn:
                    "SESek entrega- eta errebote-gertaerak argitaratzen dituen SNS gaia. Beste edozein gairen jakinarazpenak baztertu egiten dira.",
                sender_id:
                    "Gehienez 11 letra eta digitu. Herrialde batzuek erregistroa eskatzen dute.",
                origination_number:
                    "Igorlearen IDaren ordez erabiltzen da herrialde batek zenbaki bat eskatzen duenean.",
                phone_number_id: "Mezuak bidaltzen diren zenbakia.",
                display_name: "Metak zenbakirako onartutako bistaratzeko izena.",
                page_username:
                    "Hautesleek beren kodea lortzeko irekitzen duten m.me estekan erabiltzen da.",
                api_version: "Adibidez, v23.0.",
                base_url: "Kontuaren Infobip APIaren oinarrizko URLa.",
                sender: "Hautesleek ikusten duten igorle onartua.",
                provider_approval:
                    "Meta-k gobernuen WhatsApp mezularitza onartutako akordio baten bidez soilik baimentzen du. Aukeratu Hornitzailearen onarpena berretsita Meta-k kontu honetarako onartu duenean; bitartean, ezin dira kodeak eta jakinarazpenak gaitu kontu honetan.",
                readiness:
                    "Hornitzailearen egiaztapenaren arabera aukerak konexio-egiaztapenak aurkitzen duena erabiltzen du: kontua konektatuta dagoen, ekoizpenean dagoen eta zein txantiloi dauden onartuta. Administratzaile batek berretsia aukera egiaztapenak hori jakin ezin duen hornitzaileentzat da: zure adierazpena da kontua konektatuta dagoela, ekoizpenean dagoela eta bere txantiloiak onartuta dituela, eta egiaztapenaren ordez erabiltzen da.",
                api_base_url:
                    "Graph API Meta-rena ez denean soilik, adibidez soluzio-hornitzaile baten amaiera-puntua. Hutsik badago, Meta-rena erabiltzen da.",
                label: "Hautesleek kontu honen igorle gisa ikusten duten izena.",
            },
            error: {
                REQUIRED: "Derrigorrezkoa",
                NOT_A_COUNT: "Idatzi zenbaki oso bat",
                OTP_ABOVE_TOTAL: "Ezin du segundoko mezuen kopurua gainditu",
                INVALID_CALLING_CODE:
                    "Idatzi 1etik 3 digitura bitarteko herrialde-aurrezenbakiak, adibidez 63",
                DUPLICATE_LANGUAGE:
                    "Hizkuntza honek badu dagoeneko txantiloi bat helburu honetarako",
                NOT_A_URL: "Idatzi https:// edo http:// hasten den helbide bat",
                INVALID_HTTP_CONFIG: "Konpondu adierazitako arazoak",
            },
            warning: {
                pageChange:
                    "Messenger elkarrizketak orri batenak dira. Orria aldatu ondoren, {{page}} orriarekin konektatutako hautesleek Messenger berriro konektatu ondoren soilik jasoko dituzte kodeak.",
                numberChange:
                    "Mezuak beste zenbaki batetik iritsiko dira. Bere txantiloiek enpresa-kontu horretan onartuta egon behar dute kodeak bidali ahal izateko, eta hautesleek txat berri bat ikusiko dute.",
            },
            viber: {
                title: "Txantiloi onartuak",
                description:
                    "Idatzi Viberrek bazkidearen bidez onartutako txantiloiak, helburu eta hizkuntza bakoitzeko. Bazkidearen txantiloien APIa ez dago erabilgarri; beraz, zerrenda hau eskuz mantentzen da eta konexio-egiaztapenak irakurtzen du.",
                purpose: "Helburua",
                language: "Hizkuntza",
                templateId: "Bazkidearen txantiloi-IDa",
                add: "Gehitu txantiloia",
                remove: "Kendu txantiloia",
            },
            limits: {
                title: "Bidalketa-mugak",
                messagesPerSecond: "Mezuak segundoko",
                otpReservedPerSecond: "Kodeetarako gordeak segundoko",
                otpReservedHelp: "Kodeetarako libre gordetzen dira bidalketa masiboetan.",
                allowedCallingCodes: "Baimendutako helmugak (herrialde-aurrezenbakiak)",
                allowedCallingCodesHelp:
                    "Komaz bereizita, adibidez 63, 971. Hutsik badago, edozein helmuga baimentzen da.",
            },
            credentials: {
                title: "Kredentzialak",
                description:
                    "Kredentzialak idazteko soilik dira: gorde ondoren, bakoitza azkenekoz noiz ordeztu zen soilik erakusten da.",
                set: "Ezarrita · ordeztua {{date}}. Zifratuta gordetzen da eta ez da inoiz erakusten.",
                replace: "Ordeztu",
                replaceNamed: "Ordeztu {{name}}",
            },
            credentialHelp: {
                AWS_SES: {
                    AWS_ACCESS_KEY_ID:
                        "Aukerakoa. Gakorik gabe, zerbitzuaren rol propioa erabiltzen da.",
                    AWS_SECRET_ACCESS_KEY: "Aukerakoa. Ezarri sarbide-gakoaren IDarekin batera.",
                },
                AWS_SNS: {
                    AWS_ACCESS_KEY_ID:
                        "Aukerakoa. Gakorik gabe, zerbitzuaren rol propioa erabiltzen da.",
                    AWS_SECRET_ACCESS_KEY: "Aukerakoa. Ezarri sarbide-gakoaren IDarekin batera.",
                },
                SMTP: {
                    SMTP_PASSWORD: "SMTP zerbitzariaren pasahitza.",
                },
                WHATSAPP_CLOUD_API: {
                    ACCESS_TOKEN:
                        "Jabearen enpresa-zorroko sistema-erabiltzaile baten tokena, whatsapp_business_messaging baimenarekin.",
                    APP_SECRET: "Webhook deiak Metatik datozela egiaztatzen du.",
                },
                MESSENGER_SEND_API: {
                    ACCESS_TOKEN: "Orri-sarbiderako token bat, pages_messaging baimenarekin.",
                    APP_SECRET: "Webhook deiak Metatik datozela egiaztatzen du.",
                },
                VIBER_INFOBIP: {
                    API_KEY: "Infobip API gakoa.",
                },
                HTTP_API: {
                    API_KEY: "Aukerakoa. Eskaerek API_KEY kredentzial gisa erabiltzen dute.",
                    API_SECRET:
                        "Aukerakoa. Bigarren sekretu bat, eta JWT sinatzen duen gakoa: PEM gako pribatu bat RS256rako, sekretu partekatua HS256rako.",
                    ACCESS_TOKEN:
                        "Aukerakoa. Eskaerek ACCESS_TOKEN kredentzial gisa erabiltzen dute.",
                    USERNAME: "Aukerakoa. Pasahitzarekin batera, basic_auth leku-marka osatzen du.",
                    PASSWORD:
                        "Aukerakoa. Erabiltzaile-izenarekin batera, basic_auth leku-marka osatzen du.",
                    WEBHOOK_SECRET:
                        "Aukerakoa. Hornitzailearen itzulera-deiak egiaztatzeko erabiltzen den sekretu partekatua.",
                },
            },
            webhook: {
                title: "Entrega-txostenak eta erantzunak",
                description:
                    "Idatzi itzulera-helbide hau hornitzailearen webhook ezarpenetan. Entrega-txostenak eta hautesleen erantzunak hara iristen dira.",
                path: "Itzulera-bidea",
                pathHelp: "Gehitu plataforma honen mezularitza-webhooken helbide publikoari.",
                afterSaving: "Gorde ondoren erakusten da",
                copyPath: "Kopiatu itzulera-bidea",
                tokenSet: "Ezarrita · ordeztua {{date}}",
                tokenMissing: "Oraindik sortu gabe",
                tokenAfterSaving: "Gorde ondoren sortzen da",
                generate: "Sortu egiaztatze-tokena",
                tokenTitle: "Egiaztatze-tokena",
                tokenOnce:
                    "Idatzi orain token hau Metaren webhook ezarpenetan. Behin bakarrik erakusten da.",
                copyToken: "Kopiatu egiaztatze-tokena",
                tokenDone: "Eginda",
                tokenError: "Ezin izan da egiaztatze-tokena sortu.",
                httpHelp:
                    "HTTP API pertsonalizatu batek bere txostenak JSON gisa bidal ditzake, edo GET eskaera gisa; orduan, bere kontsulta-parametroak objektu lau gisa irakurtzen dira, /status bezalako erakusleekin.",
            },
            copy: {
                success: "Kopiatuta",
                error: "Ezin izan da kopiatu",
            },
            save: {
                success: "Kontua gorde da",
                error: "Ezin izan da kontua gorde.",
            },
            test: {
                title: "Bidali proba-mezu bat {{name}} kontutik",
                description:
                    "Aukeratutako helbururako benetako mezu bat bidaltzen du helmuga honetara. Emaitzak hornitzaileak jakinarazitakoa erakusten du.",
                purpose: "Helburua",
                destination: {
                    EMAIL_ADDRESS: "Helbide elektronikoa",
                    PHONE_NUMBER: "Telefono-zenbakia (E.164)",
                    PAGE_SCOPED_ID: "Orriaren esparruko IDa",
                },
                language: "Hizkuntza",
                send: "Bidali proba-mezua",
                reason: "Arrazoia: {{reason}}",
                error: "Ezin izan da proba-mezua bidali.",
                template: "Txantiloi onartua",
                templateHelp:
                    "Hornitzaileak helburu eta hizkuntza honetarako onartu duen txantiloiaren izena edo IDa.",
                viberTemplate:
                    "Viber-ek kontu honek aukeratutako helburu eta hizkuntzarako onartutzat duen txantiloia erabiltzen du.",
                languageHelp:
                    "Txantiloi onartuak bidaltzen dituen hornitzaile baterako, idatzi hornitzaileak txantiloirako duen hizkuntza-kodea, adibidez en_US.",
            },
            http: {
                title: "HTTP API pertsonalizatua",
                description:
                    "Hornitzaile bat bere HTTP eskaeren bidez deskribatzen du: beste Viber bazkide bat, WhatsApp soluzio-hornitzaile baten API propioa, SMS pasabide bat. Eskaerak JSON dira; haien URLak, goiburuek eta gorputzak beheko erreferentziako leku-markak izan ditzakete.",
                phoneFormat: "Telefono-zenbakiaren formatua",
                phoneFormatHelp: "Nola idazten den hartzailearen telefono-zenbakia eskaera batean.",
                phoneFormatOption: {
                    E164: "Plus ikurrarekin: +639171234567",
                    DIGITS: "Digituak soilik: 639171234567",
                },
                templateRequired: "Txantiloi onartua behar duten helburuak",
                templateRequiredHelp:
                    "Markatutako helburu bat hornitzaileak onartutako txantiloi batekin soilik bidaltzen da, hauteskunde-gertaeran lotuta. Gainerako helburuak testu libre gisa bidaltzen dira.",
                approvedLanguages: "Txantiloi onartua duten hizkuntzak: {{purpose}}",
                approvedLanguagesHelp:
                    "Txantiloi onartua duten hizkuntza-kodeak, hornitzailearekin berretsi bezala, komaz bereizita: en, tl. Konexio-egiaztapenak horien berri ematen du.",
                conversationWindow: "Elkarrizketa-leihoa (orduak)",
                conversationWindowHelp:
                    "Hartzailearen azken mezuaren ondoren testu librea bidal daitekeen orduak. Hutsik hornitzaileak horrelako leihorik ez duenean.",
                messageIdPointer: "Mezuaren IDa bidalketaren erantzunean",
                messageIdPointerHelp:
                    "Bidalketa-eskaeraren erantzunean hornitzailearen mezu-IDa non dagoen adierazten duen JSON erakuslea, adibidez /message_id. Entrega-txostenak harekin parekatzen dira.",
                notConfigured: "Konfiguratu gabe.",
                thisSection: "Atal hau",
                add: "Gehitu: {{section}}",
                remove: "Kendu: {{section}}",
                section: {
                    SEND: "Bidalketa-eskaera",
                    CHECK: "Konexio-egiaztapenaren eskaera",
                    TOKEN: "Token-eskaera",
                    JWT: "Token sinatua (JWT)",
                    REPORTS: "Entrega-txostenak eta erantzunak",
                    RECONCILE: "Mezu baten kontsulta-eskaera",
                },
                sectionHelp: {
                    SEND: "Mezu bat bidaltzen duen eskaera: method (POST, zehazten ez bada), url, headers eta body.",
                    CHECK: "Aukerakoa. Kredentzialek funtzionatzen dutenean 2xx erantzunarekin ongi amaitzen den eskaera. Konexio-egiaztapenak exekutatzen du.",
                    TOKEN: "Aukerakoa. Iraupen laburreko token bat lortzen du bidali aurretik, adibidez OAuth bezero-kredentzialekin: request, token_pointer (tokena erantzunean non dagoen) eta lifetime_seconds. Eskaerek token leku-markarekin erabiltzen dute.",
                    JWT: "Aukerakoa. Eskaera bakoitzerako APIaren sekretua kredentzialarekin sinatutako token bat: algorithm (RS256 edo HS256), claims (iat, exp eta jti gehitzen dira) eta lifetime_seconds. Eskaerek jwt leku-markarekin erabiltzen dute.",
                    REPORTS:
                        "Aukerakoa. Hornitzaileak itzulera-helbidera bidaltzen duena nola irakurri: auth, items_pointer (txostenen zerrenda non dagoen; eduki osoa, zehazten ez bada), status (message_id_pointer, state_pointer, states, hornitzailearen balio bakoitza QUEUED, ACCEPTED, DELIVERED, FAILED edo UNKNOWN balioarekin lotzen duena, eta error_pointer) eta inbound_from_pointer (erantzun baten igorlea non dagoen). auth-ek kind bat du: URL_KEY (itzulera-helbide sekretua soilik), HEADER_SECRET (webhookaren sekretuaren berdina den goiburu bat), HMAC_SHA256 (gorputzaren HMAC duen goiburu bat, webhookaren sekretuarekin kalkulatua, prefix-arekin, HEX edo BASE64 encoding-arekin, eta signed gorputza baino gehiago sinatzen denean) edo JWT_HS256 (webhookaren sekretuarekin sinatutako bearer JWT bat duen goiburu bat).",
                    RECONCILE:
                        "Aukerakoa. Emaitza ezezaguna duen mezu bati buruz galdetzen dio hornitzaileari: request eta status, entrega-txostenen status bezala irakurtzen dena.",
                },
                problem: {
                    NOT_AN_OBJECT: "{{path}} objektu bat izan behar da.",
                    MISSING_URL: "{{path}} derrigorrezkoa da: eskaeraren helbidea.",
                    INVALID_METHOD:
                        "{{path}} HTTP metodo bat izan behar da, adibidez POST edo GET.",
                    INVALID_HEADERS:
                        "{{path}} testua izan behar da: headers goiburu-izenen eta testu-balioen objektu bat da.",
                    UNKNOWN_FIELD: "{{path}} ez da atal honetako eremu bat.",
                    UNKNOWN_PLACEHOLDER:
                        "{{path}} eremuak existitzen ez den leku-marka bat erabiltzen du. Ikusi leku-marken erreferentzia.",
                    INVALID_POINTER:
                        "{{path}} / karaktereaz hasten den JSON erakusle bat izan behar da, adibidez /data/id.",
                    INVALID_STATES:
                        "{{path}} eremuak hornitzailearen egoera-balio bat QUEUED, ACCEPTED, DELIVERED, FAILED edo UNKNOWN balioarekin lotu behar du; gutxienez bat behar da.",
                    INVALID_AUTH:
                        "{{path}} ez da baliozkoa: kind URL_KEY, HEADER_SECRET, HMAC_SHA256 edo JWT_HS256 da; header derrigorrezkoa da URL_KEY kasuan izan ezik; encoding HEX edo BASE64 da.",
                    INVALID_LIFETIME:
                        "{{path}} 0 baino handiagoa den segundo kopuru oso bat izan behar da.",
                    INVALID_ALGORITHM: "{{path}} RS256 edo HS256 izan behar da.",
                    INVALID_CLAIMS: "{{path}} objektu bat izan behar da.",
                    INVALID_HOURS:
                        "{{path}} 0 baino handiagoa den ordu kopuru oso bat izan behar da.",
                },
                placeholders: {
                    title: "Leku-marken erreferentzia",
                    help: "Giltza bikoitzen artean idazten dira URLan, goiburu baten balioan edo gorputzeko edozein testutan. Bakoitza eskaera egitean ordezten da.",
                },
                placeholder: {
                    to: "Hartzailea: telefono-zenbakia, helbide elektronikoa edo orriaren esparruko IDa.",
                    text: "Mezua testu soil gisa.",
                    subject: "Gaia, posta elektronikorako.",
                    html: "Mezua HTML gisa, posta elektronikorako.",
                    code: "Erabilera bakarreko kodea, kodeetarako.",
                    template: "Hauteskunde-gertaeran lotutako hornitzailearen txantiloia.",
                    language: "Hornitzaileak txantiloirako duen hizkuntza-kodea.",
                    message_id: "Hornitzailearen mezu-IDa, mezu baten kontsulta-eskaeran.",
                    callback_url: "Kontu honen itzulera-helbide publikoa.",
                    param: "Txantiloiaren parametro bat bere posizioaren arabera: 1, 2, 3 eta abar.",
                    credential:
                        "Kontu honen kredentzial bat bere izenaren arabera: API_KEY, API_SECRET, ACCESS_TOKEN, USERNAME, PASSWORD edo WEBHOOK_SECRET.",
                    basic_auth:
                        "Erabiltzaile-izena eta pasahitza, Authorization: Basic goiburu baterako kodetuta.",
                    token: "Token-eskaerarekin lortutako tokena.",
                    jwt: "Token sinatua (JWT) atalean deskribatutako token sinatua.",
                    parameters:
                        "Gorputzeko balio gisa bakarrik dagoenean, txantiloiaren parametro guztien zerrenda bihurtzen da.",
                    named_parameters:
                        "Gorputzeko balio gisa bakarrik dagoenean, @izena=balioa gisa idatzitako parametroen objektu bat bihurtzen da.",
                },
                example: {
                    title: "Adibide osoa: Viber bazkide bat",
                    description:
                        "Bazkideak JSON POST bat jasotzen du, API gakoarekin bearer token gisa autentifikatua, mezuaren IDarekin erantzuten du message_id eremuan, eta entrega-txostenak goiburu sekretu batekin bidaltzen ditu. Erabili abiapuntu gisa, eta aldatu helbidea eta eremu-izenak hornitzailearenekin.",
                    use: "Erabili adibide hau",
                },
            },
        },
    },
}

export default basqueTranslation
