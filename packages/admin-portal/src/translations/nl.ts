// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import englishTranslation, {TranslationType} from "./en"

const dutchTranslation: TranslationType = {
    translations: {
        configurationPackage: englishTranslation.translations.configurationPackage,
        ballotPrivacy: englishTranslation.translations.ballotPrivacy,
        readiness: {
            close: "Sluiten",
            action: "Gereedheid controleren",
            title: "Verkiezingsgereedheid",
            description:
                "Vergelijk dit officiële evenement met een voltooide repetitie. Een geslaagde controle legt ondertekend bewijs vast in het logboek van het officiële evenement.",
            testedEvent: "Getest evenement",
            archived: "Gearchiveerd",
            noEvents: "Geen andere evenementen beschikbaar",
            retry: "Opnieuw proberen",
            eventsError: "Verkiezingsevenementen konden niet worden geladen.",
            run: "Controle uitvoeren",
            running: "Gereedheid wordt gecontroleerd…",
            error: "De controle kon niet worden voltooid. Bekijk het logboek voordat u het opnieuw probeert; het antwoord kan verloren zijn gegaan nadat het bewijs was vastgelegd.",
            downloadError: "Het rapport kon niet worden gedownload. Probeer het opnieuw.",
            success:
                "Gereedheid bevestigd. Ondertekend bewijs is vastgelegd in het logboek van het officiële evenement.",
            failed: "De controle is mislukt. Verhelp de mislukte controles en voer deze opnieuw uit. Er is geen gereedheidsvermelding vastgelegd.",
            download: "JSON-rapport downloaden",
            snapshot:
                "Dit rapport is een momentopname. Herhaal na relevante wijzigingen; het bevriest het evenement niet en vervangt geen operationele goedkeuring.",
            results: "Controleresultaten",
            condition: "Voorwaarde die de server heeft gecontroleerd",
            noDetail: "De server gaf geen verdere details voor deze controle.",
            reasons: "Waarom het mislukte",
            noArea: "geen of een onbekend gebied",
            eventRoles: {
                official: "officiële",
                tested: "geteste",
                both: "beide",
            },
            scope: {
                event: "Het {{event}} evenement",
                election: "Verkiezing {{election}}",
            },
            certification: {
                declared_sha256_missing: "er is geen SHA-256 opgegeven",
                declared_sha256_mismatch: "de opgegeven SHA-256 verschilt van die van het bestand",
                no_counts: "het heeft geen kiezers per gebied",
                blank_area: "sommige kiezers hebben geen gebied",
                invalid_sha256: "de SHA-256 is ongeldig",
            },
            findings: {
                same_event: "Het officiële en het geteste evenement zijn hetzelfde evenement.",
                configuration_missing:
                    "Het {{event}} evenement is niet geïmporteerd uit een ondertekend configuratiepakket.",
                configuration_incomplete:
                    "De ondertekende configuratie van het officiële evenement heeft geen geldige waarde voor ‘{{field}}’.",
                external_id_mismatch:
                    "De evenementen komen uit verschillende configuraties: {{official}} (officieel) en {{tested}} (getest).",
                content_mismatch:
                    "De ondertekende inhoud verschilt: SHA-256 {{official}} (officieel) en {{tested}} (getest).",
                approvers_mismatch:
                    "De geverifieerde goedkeurders verschillen: {{only_official}} alleen bij het officiële evenement en {{only_tested}} alleen bij het geteste evenement.",
                areas_mismatch:
                    "{{area}} staat alleen in de ondertekende inhoud van het {{event}} evenement.",
                voter_import_missing:
                    "Het {{event}} evenement heeft geen gecertificeerde kiezersimport.",
                voter_import_in_progress:
                    "Er loopt nog een kiezersimport bij het {{event}} evenement.",
                voter_file_not_certified:
                    "De laatste kiezersimport van het {{event}} evenement is niet gecertificeerd: {{reason}}.",
                voter_receipt_not_signed:
                    "Het logboek van het {{event}} evenement heeft geen overeenkomende ondertekende vermelding van kiezersimport {{import_id}}.",
                registry_count_changed:
                    "Het {{event}} evenement heeft nu {{current}} kiezers in {{area}}, maar de gecertificeerde import heeft er {{imported}}.",
                voter_area_not_configured:
                    "De gecertificeerde kiezers van het {{event}} evenement bevatten {{area}}, dat niet in de ondertekende inhoud staat.",
                voter_file_mismatch:
                    "De gecertificeerde kiezersbestanden verschillen: SHA-256 {{official}} (officieel) en {{tested}} (getest).",
                voter_count_mismatch:
                    "De gecertificeerde bestanden verschillen in {{area}}: {{official}} kiezers (officieel) en {{tested}} (getest).",
                ballots_present:
                    "Het officiële evenement heeft {{count}} opgeslagen stembiljetten.",
                received_ballots_present:
                    "Het officiële evenement heeft {{count}} stembiljetten ontvangen ter controle.",
                tally_sessions_present: "Het officiële evenement heeft {{count}} telsessies.",
                status_missing: "{{scope}} heeft geen stemstatus.",
                voting_started: "{{scope}}: het kanaal {{channel}} is {{status}}.",
                voting_dates_recorded:
                    "{{scope}}: het kanaal {{channel}} heeft vastgelegde stemdatums.",
                no_elections: "Het {{event}} evenement heeft geen verkiezingen.",
                publication_missing: "Het {{event}} evenement heeft geen actieve publicatie.",
                publication_mismatch:
                    "De {{design_count}} gepubliceerde stembiljetontwerpen van het {{event}} evenement komen niet overeen met alle ontwerpen van de ondertekende configuratie.",
                publication_empty:
                    "De gepubliceerde stembiljetontwerpen van het {{event}} evenement konden niet worden gecontroleerd aan de hand van een ondertekende configuratie.",
                channels_open:
                    "Het geteste evenement heeft nog stemkanalen die niet gesloten zijn.",
                seal_at_close_off:
                    "Het geteste evenement verzegelt de stembussen niet bij sluiting.",
                no_expected_boxes:
                    "De publicatie van het geteste evenement verwacht geen stembussen.",
                boxes_unsealed:
                    "Verkiezing {{election}} van het geteste evenement heeft {{unsealed}} onverzegelde stembussen.",
            },
        },
        plugins: {
            title: "Plug-ins",
            description:
                "Plug-ins zijn meegeleverd met deze release. Een plug-in inschakelen voegt het menu-item en de rechten toe voor deze tenant; uitschakelen verbergt de plug-in en bewaart de gegevens en rechten.",
            enabled: "Aan",
            disabled: "Uit",
            compatible: "Compatibel",
            incompatible: "Opgeslagen gegevens vereisen een nieuwere release",
            api: "Plug-in-API {{major}}",
            permissions: "Rechten",
            adminGroup: "toegekend aan de groep admin bij inschakelen",
            enable: "Inschakelen",
            disable: "Uitschakelen",
            confirmEnableTitle: "{{name}} inschakelen?",
            confirmDisableTitle: "{{name}} uitschakelen?",
            confirmEnable:
                "{{name}} verschijnt in het menu voor gebruikers met de bijbehorende rechten. De rechten worden in deze tenant aangemaakt en toegekend aan de groep admin.",
            confirmDisable:
                "{{name}} wordt verborgen en verzoeken worden geweigerd. De gegevens, geschiedenis en rechten blijven bewaard en verder verandert er niets in de tenant.",
            confirm: "Bevestigen",
            noPermission: "Plug-ins wijzigen vereist het recht tenant-write.",
            retry: "Opnieuw proberen",
            loading: "Plug-ins laden…",
            signIn: "Meld je aan bij de geselecteerde tenant om de plug-ins te gebruiken.",
            unavailable: "Deze plug-in maakt geen deel uit van deze release.",
            incompatibleRoute:
                "De opgeslagen gegevens van deze plug-in vereisen een nieuwere release. De gegevens zijn ongewijzigd.",
            disabledRoute:
                "{{name}} is niet ingeschakeld voor deze tenant. Een tenantbeheerder kan het inschakelen via Instellingen → Plug-ins.",
            forbidden: "Je hebt geen recht om {{name}} te gebruiken.",
        },
        projects: {
            title: "Projecten",
            subtitle:
                "Plan het werk van de tenant op een tijdlijn. Projecten wijzigen nooit verkiezingsschema's.",
            cancel: "Annuleren",
            retry: "Opnieuw proberen",
            loading: "Laden…",
            list: {
                create: "Nieuw project",
                import: "Importeren",
                emptyTitle: "Nog geen projecten",
                emptyHelp:
                    "Maak een project of importeer een CSV-bestand dat uit een ander project is geëxporteerd.",
                emptyViewer: "Er zijn geen projecten aangemaakt in deze tenant.",
                active_one: "Actief ({{count}})",
                active_other: "Actief ({{count}})",
                archived_one: "Gearchiveerd ({{count}})",
                archived_other: "Gearchiveerd ({{count}})",
                name: "Naam",
                dates: "Datums",
                tasks: "Taken",
                timezone: "Tijdzone",
                updated: "Laatste wijziging",
                updatedBy: "{{at}} · {{actor}}",
                archivedChip: "Gearchiveerd",
                noneInTab: "Hier zijn geen projecten.",
            },
            create: {
                title: "Nieuw project",
                name: "Projectnaam",
                timezone: "Tijdzone",
                timezoneHelp:
                    "Alle datums in het project worden in deze IANA-tijdzone ingevoerd en getoond.",
                submit: "Project maken",
            },
            import: {
                title: "Een project importeren",
                help: "Importeren maakt altijd een nieuw project. Gebruik een UTF-8-CSV-bestand met de kolommen {{columns}}, zoals geëxporteerd uit een project, met een UTC-afwijking bij elke datum. Er wordt niets opgeslagen tot je het project maakt.",
                choose: "CSV-bestand kiezen",
                noFile: "Geen bestand gekozen",
                fileLimit: "Het bestand mag maximaal 1 MiB zijn.",
                invalidFile: "Het bestand is geen geldige UTF-8-tekst.",
                valid_one: "{{count}} taak is klaar om te importeren.",
                valid_other: "{{count}} taken zijn klaar om te importeren.",
                invalid_one:
                    "{{count}} van {{total}} rijen bevat fouten. Corrigeer het bestand en kies het opnieuw.",
                invalid_other:
                    "{{count}} van {{total}} rijen bevatten fouten. Corrigeer het bestand en kies het opnieuw.",
                previewTable: "Voorbeeld van de import",
                line: "Regel",
                result: "Controle",
                rowValid: "Klaar",
                commit: "Project maken",
            },
            detail: {
                timezone: "Alle datums en tijden zijn in {{timezone}}.",
                export: "Exporteren",
                archive: "Archiveren",
                restore: "Herstellen",
                addTask: "Taak toevoegen",
                viewOnly: "Alleen lezen. Taken wijzigen vereist het recht projects-write.",
                archivedNotice: "Dit project is gearchiveerd. Herstel het om de taken te wijzigen.",
            },
            notices: {
                saved: "“{{title}}” opgeslagen.",
                moved: "“{{title}}” opgeslagen: {{start}} – {{end}}.",
                deleted: "“{{title}}” verwijderd.",
                archived: "Project gearchiveerd.",
                restored: "Project hersteld.",
            },
            panel: {
                details: "Taakgegevens",
                edit: "Taak bewerken",
                add: "Taak toevoegen",
                close: "Taakgegevens sluiten",
                hint: "Selecteer een taak op de tijdlijn om de gegevens hier te zien.",
                noTasks: "Dit project heeft nog geen taken.",
            },
            conflict: {
                body: "Iemand anders heeft deze taak gewijzigd ({{actor}}, {{at}}). Je wijzigingen staan er nog. Bekijk hun versie voordat je opnieuw opslaat.",
                load: "Hun versie laden",
            },
            unsaved: {
                body: "Je hebt niet-opgeslagen wijzigingen. Sla ze op, of verwerp ze om {{action}}.",
                keep: "Verder bewerken",
                discard: "Wijzigingen verwerpen",
                actions: {
                    open: "“{{title}}” te openen",
                    add: "een nieuwe taak toe te voegen",
                    leave: "terug te gaan naar de projecten",
                    close: "de taak te sluiten",
                    state: "het project te wijzigen",
                },
            },
            deleteDialog: {
                title: "Taak verwijderen?",
                body: "“{{title}}” wordt uit het project verwijderd. De wijzigingsgeschiedenis blijft bewaard.",
                confirm: "Verwijderen",
            },
            history: {
                task: "Wijzigingsgeschiedenis",
                project: "Projectgeschiedenis",
                empty: "Nog geen wijzigingen vastgelegd.",
                more: "Oudere wijzigingen tonen",
                by: "{{actor}} · {{at}}",
                actions: {
                    create: "Aangemaakt {{subject}}",
                    update: "Gewijzigd {{subject}}",
                    delete: "Verwijderd {{subject}}",
                    import_one: "{{count}} taak geïmporteerd",
                    import_other: "{{count}} taken geïmporteerd",
                    export: "Geëxporteerd als CSV",
                    other: "{{action}}",
                },
            },
            editor: {
                title: "Titel",
                type: "Type",
                status: "Status",
                start: "Begin",
                end: "Einde",
                at: "Datum en tijd",
                notes: "Notities",
                order: "Volgorde",
                orderHelp: "Taken worden gesorteerd op volgorde en daarna op begin.",
                timezoneNote: "Tijden zijn in {{timezone}}.",
                offset: "UTC{{offset}}",
                localTime: {
                    invalid: "Voer een datum en tijd in tussen 1900 en 2200.",
                    skipped:
                        "Deze tijd bestaat niet in {{timezone}}: de klok slaat hem over. Kies een andere tijd.",
                    repeated:
                        "Deze tijd komt twee keer voor in {{timezone}}. Kies welke je bedoelt.",
                },
                chooseInstant: "Welke keer",
                instantChoice: "Op UTC{{offset}}",
                saving: "Opslaan…",
                toFix_one: "{{count}} veld te corrigeren",
                toFix_other: "{{count}} velden te corrigeren",
                save: "Opslaan",
                delete: "Verwijderen",
            },
            validation: {
                title: "Voer een titel in van maximaal 200 tekens.",
                offset: "Voer geldige datums in.",
                range: "Het einde moet na het begin liggen.",
                milestone: "Een mijlpaal heeft geen einde.",
                notes: "Notities mogen maximaal 16.000 tekens bevatten.",
                order: "De volgorde moet een geheel getal van 0 tot 1.000.000 zijn.",
            },
            status: {
                not_started: "Niet begonnen",
                in_progress: "Bezig",
                done: "Klaar",
            },
            type: {
                task: "Taak",
                milestone: "Mijlpaal",
            },
            units: {
                days_one: "{{count}} dag",
                days_other: "{{count}} dagen",
                hours_one: "{{count}} u",
                hours_other: "{{count}} u",
                minutes_one: "{{count}} min",
                minutes_other: "{{count}} min",
            },
            timeline: {
                title: "Tijdlijn",
                tasks_one: "{{count}} taak",
                tasks_other: "{{count}} taken",
                empty: "Nog geen taken.",
                editHelp:
                    "Wijs een taak aan of geef die focus voor een samenvatting; selecteer haar om eronder te bewerken. Sleep een balk of gebruik ← → om haar 24 uur te verplaatsen; voeg Shift toe om het einde te wijzigen.",
                viewHelp:
                    "Wijs een taak aan of geef die focus voor een samenvatting. Selecteer haar om de gegevens eronder te zien.",
                touchEditHelp:
                    "Tik op een taak om haar eronder te bewerken, of sleep een balk om haar hele dagen te verplaatsen.",
                touchViewHelp: "Tik op een taak om de gegevens eronder te zien.",
                taskLabel: "{{title}}, van {{start}} tot {{end}}, {{status}}",
                milestoneLabel: "{{title}}, mijlpaal op {{at}}, {{status}}",
                previewEdit: "Klik of druk op Enter om te bewerken",
                previewView: "Klik of druk op Enter voor de gegevens",
            },
        },
        philippinePassport: "Filipijns Paspoort",
        seamanBook: "Zeemansboek",
        philSysID: "PhilSys ID",
        iBP: "Integrated Bar of the Philippines (IBP)",
        driversLicense: "Rijbewijs",
        loading: "Laden...",
        loadingDataProvider: "Gegevensprovider laden...",
        tallySheetImport: {
            title: "Importen van processen-verbaal",
            subtitle:
                "Importeer ES&S- of CSV-bestanden van processen-verbaal, bekijk de gegenereerde stembussen vooraf en keur ze goed voordat ze processen-verbaal aanmaken.",
            createTitle: "Processen-verbaal importeren",
            detailTitle: "Import van proces-verbaal",
            empty: "Nog geen imports van processen-verbaal.",
            emptyBody:
                "Begin met het importeren van een ES&S Enhanced XML- of canoniek CSV-bestand voor deze verkiezingsgebeurtenis.",
            sourceFormat: {
                ESS_ENHANCED_XML: "ES&S Enhanced XML",
                CANONICAL_CSV: "Canonieke CSV",
            },
            channel: {
                PAPER: "Papier",
                POSTAL: "Post",
                IN_PERSON: "Persoonlijk",
            },
            table: {
                created: "Aangemaakt",
                createdBy: "Gemaakt door",
                file: "Bestand",
                format: "Formaat",
                channel: "Kanaal",
                status: "Status",
                labels: "Labels",
                annotations: "Annotaties",
                actions: "Acties",
            },
            summary: {
                imported: "Geïmporteerd",
                changed: "Gewijzigd",
                new: "Nieuw",
                unchanged: "Ongewijzigd",
                conflicted: "In conflict",
                errors: "Fouten",
            },
            status: {
                PENDING_REVIEW: "Wacht op beoordeling",
                APPROVED: "Goedgekeurd",
                DISAPPROVED: "Afgekeurd",
                FAILED_VALIDATION: "Validatie mislukt",
                CONFLICTED: "In conflict",
                NEW: "Nieuw",
                CHANGED: "Gewijzigd",
                UNCHANGED: "Ongewijzigd",
            },
            fields: {
                format: "Formaat",
                channel: "Kanaal",
                supportedFormats: "Ondersteunde formaten: XML, CSV",
                generatedTallySheet: "Gegenereerd proces-verbaal",
                sourceCandidates: "Bron-kandidaat-ID's",
                none: "Geen",
            },
            actions: {
                create: "Processen-verbaal importeren",
                review: "Beoordelen",
                source: "Bron",
                cancel: "Annuleren",
                preview: "Voorbeeld bekijken",
                save: "Import opslaan",
                approve: "Goedkeuren",
                disapprove: "Afkeuren",
                close: "Sluiten",
                openExisting: "Bestaande openen",
            },
            notifications: {
                selectFile: "Selecteer een importbestand voordat u het bekijkt",
                duplicateSource:
                    "Deze hash van het bronbestand komt al voor in een eerdere import van proces-verbaal.",
                uploadUrlError: "Kon upload-URL niet aanmaken",
                uploadError: "Kon importbestand niet uploaden",
                previewEmpty: "Het voorbeeldantwoord was leeg",
                previewError: "Kon import niet bekijken",
                importEmpty: "Het importantwoord was leeg",
                created: "Import van proces-verbaal aangemaakt",
                createError: "Kon import niet aanmaken",
                reviewEmpty: "Het beoordelingsantwoord was leeg",
                conflicted: "Import heeft verouderde basislijnconflicten",
                approved: "Import goedgekeurd",
                disapproved: "Import afgekeurd",
                reviewError: "Kon import niet beoordelen",
                sourceUrlError: "Kon bron-downloadURL niet aanmaken",
                sourceDownloadError: "Kon bronbestand niet downloaden",
            },
            pagination: {
                range: "{{rangeStart}}-{{rangeEnd}} van {{total}}",
                previous: "Vorige",
                next: "Volgende",
            },
        },
        reconciliation: {
            menuButton: "Ext. kiezerssync",
            categories: {
                VOTED_INTERNET: "Gestemd via Internet",
                VOTED_OTHER_CHANNEL: "Gestemd via ander kanaal",
                DISABLED_DELETE_CALL: "Kiezer gedeactiveerd",
                DELETION_REVERTED: "Verwijdering teruggedraaid",
                PROFILE_UPDATE: "Profiel bijgewerkt",
                VOTER_ADDED: "Kiezer toegevoegd",
                REENABLED: "Kiezer heractiveerd",
                VOTED_UNMARKED: "Kiezer niet langer gemarkeerd als gestemd",
                ROW_FAILURE: "Rijfout",
            },
            table: {
                voterId: "Kiezer-ID",
                field: "Veld",
                category: "Categorie",
                currentValue: "Huidige waarde",
                newValue: "Nieuwe waarde",
                reason: "Reden",
                rowLabel: "Rij",
                noDifferences: "Geen verschillen gevonden - de systemen zijn gesynchroniseerd.",
            },
            wizard: {
                title: "Externe reconciliatiesynchronisatie",
                subtitle: "Synchroniseer de kiezerslijst met het externe systeem",
                drop: {
                    description:
                        "Sleep het door het externe systeem gegenereerde reconciliatiebestand hierheen - beide diffs (externe zijde en Sequent-zijde) worden automatisch berekend en in aparte tabellen weergegeven.",
                    fileFormatLabel: "CSV-bestand",
                    uploading: "{{fileName}} wordt geüpload en beide diffs worden berekend...",
                },
                review: {
                    fileSummary:
                        "{{fileName}} - Sequentie {{sequence}}, gegenereerd {{generatedAt}}",
                    rowFailuresWarning:
                        "{{count}} rij(en) konden niet veilig worden gereconcilieerd en zijn uitgesloten van beide diffs - zie details hieronder.",
                    noDifferences: "Geen verschillen - beide systemen zijn al gesynchroniseerd.",
                    diffOnlyDifferences:
                        "Dit is een convergentiecontrole voor een reeds toegepaste reeks. Verschillen worden ter opvolging getoond, maar deze ronde kan niet opnieuw worden toegepast.",
                    externalDiffTitle: "Externe diff",
                    sequentDiffTitle: "Sequent-diff",
                    downloadExternalPatch: "Externe patch downloaden",
                    externalDiffCaption:
                        "Download de patch en lever deze buiten deze tool aan het externe systeem. Zodra het de patch toepast en het volgende reconciliatiebestand produceert, klik op 'Terug' en sleep dat bestand hierheen - 'Toepassen' wordt geactiveerd zodra deze tabel leeg is.",
                    noExternalDifferences: "Geen verschillen aan de externe zijde.",
                    sequentDiffCaption:
                        "Pas wijzigingen direct toe op Sequent door op 'Toepassen' te klikken - hiervoor wordt geen patchbestand gegenereerd.",
                    noSequentDifferences: "Geen verschillen aan de Sequent-zijde.",
                },
                applying: {
                    inProgress: "Wijzigingen aan de Sequent-zijde worden toegepast...",
                    rowFailures:
                        "{{count}} rij(en) zijn uitgesloten van deze ronde en vereisen handmatige opvolging - zie details hieronder.",
                    rowFailuresTruncated:
                        "De eerste {{shown}} van {{count}} rijfouten worden getoond. Los de gemeenschappelijke oorzaak op en probeer opnieuw om de resterende fouten te zien.",
                    success: "Alle wijzigingen aan de Sequent-zijde zijn succesvol toegepast.",
                },
                actions: {
                    cancel: "Annuleren",
                    back: "Terug",
                    apply: "Toepassen",
                    next: "Volgende",
                    startOver: "Opnieuw beginnen",
                    close: "Sluiten",
                },
                confirm: {
                    title: "Reconciliatiewijzigingen bevestigen",
                    categoriesNote:
                        "Categorieën die oranje zijn omlijnd ({{categories}}) hebben invloed op de stemstatus of deactiveren kiezers.",
                    applyChanges: "Wijzigingen toepassen",
                    continue: "Doorgaan",
                },
                summary: {
                    votedOtherChannel:
                        "markeert {{count}} kiezer(s) als gestemd via een ander kanaal",
                    disabled: "deactiveert {{count}} kiezer(s)",
                    reenabled: "heractiveert {{count}} kiezer(s)",
                    votedUnmarked: "verwijdert de stemmarkering van {{count}} kiezer(s)",
                    profileUpdated: "werkt {{count}} profiel(en) bij",
                    voterAdded: "voegt {{count}} kiezer(s) toe",
                    prefix: "Dit past wijzigingen toe die {{parts}}.",
                    empty: "Er zijn geen wijzigingen aan de Sequent-zijde om toe te passen.",
                },
                notifications: {
                    envelopeLoadError:
                        "Kon de reconciliatie-diff niet laden - probeer het opnieuw.",
                    generateFailed:
                        "Kon de reconciliatie-diff niet berekenen - zie de taakwidget voor details.",
                    applyFailed:
                        "Kon de wijzigingen aan de Sequent-zijde niet toepassen - zie de taakwidget voor details.",
                    uploadUrlError: "Kon geen upload-URL verkrijgen",
                    generateTaskError: "Kon de reconciliatie-difftaak niet starten",
                    uploadError: "Kon het reconciliatiebestand niet uploaden",
                    applyTaskError: "Kon de toepassingstaak niet starten",
                    applyError: "Kon de reconciliatiewijzigingen niet toepassen",
                },
            },
        },
        tenantLogs: {
            explanation: {
                action: "Actie: {{action}}",
                outcome: {
                    succeeded: "Geaccepteerd (HTTP {{status}})",
                    denied: "Geweigerd (HTTP {{status}})",
                    failed: "Mislukt (HTTP {{status}})",
                },
                by: "Gevraagd door {{username}} ({{userId}})",
                detail: "{{name}}: {{value}}",
                board: "Log: {{board}}",
                publicKey: "Publieke sleutel van het log: {{key}}",
            },
        },
        logsScreen: {
            configurationPackage: englishTranslation.translations.logsScreen.configurationPackage,
            readiness: {
                passed: "Geslaagd",
                failed: "Mislukt",
                checkResult: "{{outcome}}: {{description}}",
                checkResultWithMessage:
                    "{{outcome}}: {{description}} Vastgelegde voorwaarde: ‘{{message}}’",
                description: {
                    VotersImported: "Kiezersbestand geïmporteerd.",
                    ReadinessVerified:
                        "Gereedheid van het officiële evenement geverifieerd met het geteste evenement.",
                },
                checks: {
                    configuration:
                        "Beide evenementen hebben dezelfde goedgekeurde inhoud, geverifieerde goedkeurders en gebieden.",
                    certified_voters:
                        "Beide evenementen komen overeen met hetzelfde gecertificeerde kiezersbestand en de huidige aantallen per gebied.",
                    official_zero:
                        "Het officiële evenement is nooit geopend voor stemmen en bevat geen stembiljetten of telsessies.",
                    publication:
                        "De officiële publicatie komt overeen met de ondertekende configuratie.",
                    tested_finished:
                        "De stemkanalen van het geteste evenement zijn gesloten en alle verwachte stembussen zijn verzegeld.",
                },
                fields: {
                    report: "Gereedheidsrapport",
                    passed: "Geslaagd",
                    checked_at: "Gecontroleerd op",
                    evidence: "Bewijs",
                    official: "Officieel evenement",
                    tested: "Getest evenement",
                    official_event_id: "ID officieel evenement",
                    tested_event_id: "ID getest evenement",
                    event_id: "Evenement-ID",
                    election_id: "Verkiezings-ID",
                    configuration: "Configuratie",
                    external_id: "Externe ID",
                    revision: "Revisie",
                    content_sha256: "SHA-256 inhoud",
                    manifest_sha256: "SHA-256 manifest",
                    approvers: "Geverifieerde goedkeurders",
                    areas: "Gebieden",
                    voters: "Kiezersimport",
                    import_id: "Import-ID",
                    document_id: "Document-ID",
                    file_name: "Bestandsnaam",
                    sha256: "SHA-256 bestand",
                    declared_sha256: "Opgegeven SHA-256",
                    counts_by_area: "Kiezers per gebied",
                    imported_at: "Geïmporteerd op",
                    voter_log_delivered: "Ondertekende import vastgelegd",
                    current_voter_counts: "Huidige kiezers per gebied",
                    voter_import_in_progress: "Kiezersimport actief",
                    status: "Status",
                    elections: "Verkiezingen",
                    ballot_count: "Opgeslagen stembiljetten",
                    received_ballot_count: "Ontvangen stembiljetten bij controle",
                    tally_session_count: "Telsessies",
                    publication: "Publicatie",
                    publication_id: "Publicatie-ID",
                    publication_ids: "Publicatie-ID’s",
                    matches_manifest: "Komt overeen met ondertekend manifest",
                    design_count: "Gepubliceerde ontwerpen",
                    seal_at_close: "Verzegelen bij sluiting",
                    expected_boxes: "Verwachte stembussen",
                    sealed_boxes: "Verzegelde stembussen",
                    all_channels_closed: "Alle stemkanalen gesloten",
                    locked_down: "Vergrendeld",
                    log_entry_id: "Logboekvermelding-ID",
                },
            },
            noPermissions: "U hebt geen toestemming om toegang te krijgen tot de logs.",
            title: "Logs",
            subtitle:
                "Alles wat in dit verkiezingsevenement is gedaan. Aanmeldingen en wijzigingen daarbuiten staan in Instellingen > Logs.",
            empty: "Geen vermeldingen gevonden.",
            tenant: {
                subtitle:
                    "Aanmeldingen, medewerkersaccounts en rollen, instellingen en verkiezingsevenementen van deze organisatie. Elk verkiezingsevenement houdt zijn eigen log bij in zijn tabblad Logs.",
                noPermissions: "U hebt geen toestemming om dit log te bekijken.",
            },
            entry: {
                title: "Logvermelding {{id}}",
                statementKind: "Soort verklaring",
                eventType: "Eventtype",
                logType: "Logtype",
                when: "Wanneer",
                utc: "{{time}} UTC",
                who: "Wie",
                userId: "Gebruikers-ID {{id}}",
                role: "Rol {{role}}",
                unknownAccount: "Onbekend account (de getypte naam wordt niet vastgelegd)",
                madeIn: "Gedaan in",
                changed: "Gewijzigd",
                objectId: "ID {{id}}",
                actionLabel: "Actie",
                details: "Details",
                origin: {
                    "hasura": "Beheerportaal (via Hasura)",
                    "harvest": "Beheerportaal (via Harvest)",
                    "windmill": "Beheerportaal (een Windmill-taak)",
                    "keycloak-console": "Beheerconsole van Keycloak, buiten het beheerportaal",
                    "service-account":
                        "Serviceaccount van Step, voor een wijziging in het beheerportaal",
                },
                action: {
                    create: "Aangemaakt",
                    update: "Bijgewerkt",
                    delete: "Verwijderd",
                    import: "Geïmporteerd",
                    export: "Geëxporteerd",
                    archive: "Gearchiveerd",
                    unarchive: "Uit archief gehaald",
                    enable: "Ingeschakeld",
                    disable: "Uitgeschakeld",
                    reset_password: "Wachtwoord opnieuw ingesteld",
                    grant: "Toegekend",
                    revoke: "Ingetrokken",
                },
                objectType: {
                    tenant: "Tenantinstellingen",
                    tenant_log: "Tenantlog",
                    staff_account: "Medewerkersaccount",
                    role: "Rol",
                    template: "Sjabloon",
                    trustee: "Trustee",
                    election_type: "Verkiezingstype",
                    election_event: "Verkiezingsevenement",
                    election: "Verkiezing",
                    contest: "Stemming",
                    candidate: "Kandidaat",
                    area: "Gebied",
                },
                values: {
                    title: "Oude en nieuwe waarden",
                    field: "Veld",
                    old: "Oude waarde",
                    new: "Nieuwe waarde",
                    none: "(geen)",
                    secret: "Gewijzigd. Geheime waarden worden nooit vastgelegd.",
                    yes: "Ja",
                    no: "Nee",
                    sha: "SHA-256 {{hash}}",
                },
                field: {
                    tenant: {
                        slug: "Korte naam",
                        isActive: "Actief",
                        logsTimezone: "Tijdzone voor het tenantlog (Talen)",
                        languages: "Talen",
                        enabledLanguages: "Ingeschakelde talen (Talen)",
                        defaultLanguage: "Standaardtaal (Talen)",
                        languageDetection: "Taaldetectie (Talen)",
                        translations: "Vertalingen (Lokalisatie)",
                        sms: "Sms",
                        mail: "E-mail",
                        gapiKey: "Google API-sleutel (Integraties)",
                        gapiEmail: "Google API-e-mailadres (Integraties)",
                        helpLinks: "Hulplinks (Uiterlijk)",
                        displayName: "Naam van de organisatie (Uiterlijk)",
                        votingCountries: "Landen geblokkeerd voor stemmen (Landen)",
                        enrollCountries: "Landen geblokkeerd voor inschrijving (Landen)",
                        logoUrl: "Logo (Uiterlijk)",
                        css: "Aangepaste CSS (Uiterlijk)",
                        votingChannels: "Stemkanalen",
                        onlineVoting: "Online stemmen (Stemkanalen)",
                        kioskVoting: "Stemmen via kiosk (Stemkanalen)",
                        telephoneVoting: "Telefonisch stemmen (Stemkanalen)",
                        earlyVoting: "Vervroegd stemmen (Stemkanalen)",
                        paperVoting: "Stemmen op papier (Stemkanalen)",
                    },
                },
                signIn: {
                    title: "Aanmelding en accountbeveiliging",
                    event: "Keycloak-gebeurtenis",
                    error: "Fout",
                    authentication: "Authenticatie",
                    credential: "Inloggegeven",
                    failures: "Mislukte pogingen op rij",
                    client: "Client",
                    ipAddress: "IP-adres",
                },
                keycloak: {
                    title: "Wijziging in Keycloak",
                    account: "Account",
                    operation: "Bewerking",
                    appliesTo: "Van toepassing op",
                    resource: "Resource",
                    newValue: "Nieuwe waarde",
                    error: "Fout",
                },
                importedFile: {
                    title: "Het geïmporteerde bestand",
                    declared: "Opgegeven controlesom",
                    sha256: "SHA-256 van het bestand",
                    declaredSha256: "Opgegeven SHA-256",
                    document: "Document",
                    checksum: {
                        undeclared: "Niet opgegeven",
                        matches: "Komt overeen met het bestand",
                        mismatch: "Komt niet overeen met het bestand",
                    },
                    counts: {
                        count: "Rijen",
                        elections: "Verkiezingen",
                        contests: "Kiesrondes",
                        candidates: "Kandidaten",
                        areas: "Gebieden",
                        area_contests: "Gebieden per kiesronde",
                        sections: "Configuratiesecties",
                    },
                },
                deletedEvent: {
                    title: "Het log van het verwijderde verkiezingsevenement",
                    lastEntry: "Laatste vermelding",
                    stateHash: "Statushash",
                    logDatabase: "Logdatabase",
                    status: "Status",
                    retired: "Ontladen en bewaard totdat het volgens het bewaarbeleid wordt gewist",
                    task: "Taak",
                    systemKey: "De systeemsleutel van het log",
                },
                exported: {
                    title: "Export",
                    entries: "Vermeldingen",
                    sha256: "SHA-256 van het bestand",
                    document: "Document",
                    scope: {
                        TENANT_LOG: "Het hele tenantlog",
                        ACCOUNT: "Eén account",
                        ACCOUNT_WITH_EVENTS:
                            "Eén account, met zijn vermeldingen in elk verkiezingsevenement",
                        ALL: "Het hele verkiezingsevenement",
                        ELECTION: "Eén verkiezing",
                        AREA: "Eén gebied",
                    },
                },
                writer: {
                    title: "Logschrijver",
                    release: "Release",
                    lastEntry: "Laatst geplaatste vermelding",
                },
                signatures: {
                    title: "Log en handtekeningen",
                    log: "Log",
                    tenantLog: "Tenantlog",
                    eventLog: "Log van het verkiezingsevenement",
                    signedBy: "Ondertekend door",
                    adminKey: "Beheersleutel van {{name}}",
                    systemOnly: "Alleen de systeemsleutel van het log",
                    systemSignature: "Systeemhandtekening",
                    publicKey: "Gepubliceerde beheersleutel",
                },
            },
            actions: {
                csv: "Exporteren als CSV",
                pdf: "Exporteren als PDF",
                audit: "Controleren",
            },
            auditDialog: {
                title: "Verkiezingslog controleren",
                confirm: "Controleren",
                description:
                    "De controle vergelijkt elke logregel met het Merkle-log en verifieert de gepubliceerde controlepunten. Het resultaat wordt als taak vastgelegd.",
            },
            exportdialog: {
                description: "Bevestig dat u deze actie wilt uitvoeren. Dit kan even duren.",
                title: "Logboeken exporteren",
                from: "Van",
                to: "Tot",
                timeZone: "Tijdzone",
                format: "Formaat",
                csv: "CSV",
                pdf: "PDF",
                sql: "SQL",
                scope: {
                    label: "Bereik",
                    tenant: {
                        TENANT_LOG: "Het hele tenantlog",
                        ACCOUNT: "Eén account",
                    },
                    event: {
                        ALL: "Het hele verkiezingsevenement",
                        ELECTION: "Eén verkiezing",
                        AREA: "Eén gebied",
                        ACCOUNT: "Eén account",
                    },
                },
                username: "Gebruikersnaam",
                election: "Verkiezing",
                area: "Gebied",
                withElectionEvents:
                    "De vermeldingen van dit account in elk verkiezingsevenement opnemen",
                rowsNameLog: "Rijen uit verkiezingsevenementen vermelden hun log.",
                recorded: "De export zelf wordt in dit log vastgelegd.",
                zoneNote:
                    "Elke rij bewaart de tijd in UTC (ISO 8601) en in {{abbr}}, met de naam van de tijdzone. Het datumbereik omvat beide grenzen, in {{abbr}}.",
                zoneNotePdf:
                    "De PDF toont elke tijd in {{abbr}}. Het datumbereik omvat beide grenzen, in {{abbr}}.",
                rowZones: "De tijdzone van de verkiezing van elke rij",
                zoneNoteRows:
                    "Elke rij bewaart de tijd in UTC (ISO 8601) en in de tijdzone van de verkiezing, met de naam van de tijdzone. Het datumbereik omvat beide grenzen, in {{abbr}}.",
                zoneNoteRowsPdf:
                    "De PDF toont elke tijd in de tijdzone van de verkiezing. Het datumbereik omvat beide grenzen, in {{abbr}}.",
            },
            filter: {
                createdFrom: "Aangemaakt van",
                createdTo: "tot",
                statementTimestampFrom: "Tijdstempel van de verklaring van",
                statementTimestampTo: "Tijdstempel van de verklaring tot",
                timeZone: "Tijdzone",
            },
            scheduledOutcome: {
                outcome: {
                    "waiting-for-initialization": "Wacht op initialisatie",
                    "runs": "wordt uitgevoerd",
                    "runs-unsigned": "wordt zonder handtekeningen uitgevoerd",
                    "refused": "wordt geweigerd",
                },
                check: {
                    "initialization": "De vereiste initialisatie is niet voltooid",
                    "voting-close": "Stemmen kan niet openen na de sluitingstijd",
                    "needs-signatures": "handtekeningen nodig",
                    "covered": "in de ondertekende configuratie",
                    "unsigned-close": "sluiten zonder handtekeningen",
                    "stricter-copy": "huidige of gepubliceerde instellingen",
                    "defaults": "nog niets gepubliceerd",
                },
                changed: "Nu {{after}} (was: {{before}}).",
                result: "Resultaat: {{outcome}}.",
                deciding: "Beslissende controle: {{check}}. {{value}}",
                authorizedBy: "Goedgekeurd door configuratie {{code}}.",
                nextStep: "Volgende stap: {{step}}",
                reason: {
                    "ballot-box-seal-policy":
                        "Gesloten gehouden: met Verzegelen bij sluiting blijft een gesloten stemming gesloten.",
                    "never-opened-kept-open":
                        "Niets te sluiten volgens planning: de Post is nooit geopend en blijft zoals hij is.",
                },
            },
            column: {
                id: "Id",
                statement_kind: "Soort verklaring",
                created: "Aangemaakt",
                statement_timestamp: "Tijdstempel verklaring",
                message: "Bericht",
                user_id: "Gebruikers-ID",
                username: "Gebruikersnaam",
                sender_pk: "Afzender PK",
                log_type: "Logtype",
                event_type: "Eventtype",
                description: "Beschrijving",
                version: "Versie",
            },
            main: {
                title: "Logs Hoofddatabase",
            },
            iam: {
                title: "Logs IAM-database",
            },
            ballotBoxSeal: {
                sealHash: "Zegelhash: {{hash}}",
                counted: "{{counted}} van {{inBox}} stembiljetten geteld.",
                notCounted_one:
                    "Het andere stembiljet is vervangen door een later stembiljet van de kiezer, afgekeurd, of uitgebracht door een kiezer die niet stemgerechtigd is.",
                notCounted_other:
                    "De andere {{count}} stembiljetten zijn vervangen door een later stembiljet van de kiezer, afgekeurd, of uitgebracht door een kiezer die niet stemgerechtigd is.",
                closeRequest: "Gesloten door het verzoek Stemming sluiten {{request}}.",
                noCloseRequest:
                    "Gesloten zonder verzoek Stemming sluiten (Stemperiode Stoppen of de geplande sluiting).",
                failedReason: "Reden: {{reason}}",
                failedLocked: "De stembus blijft vergrendeld en is niet verzegeld: een incident.",
                verifiedCounted: "{{counted}} stembiljetten geteld op basis van het zegel.",
                tallySession: "Telsessie {{session}}.",
                differs: "Wat verschilt: {{differs}}",
            },
        },
        tasksScreen: {
            noPermissions: "U hebt geen toestemming om toegang te krijgen tot logs.",
            title: "Uitvoering van Taken",
            subtitle: "Informatie over de uitgevoerde taken",
            taskInformation: "Taakinformatie",
            status: "Status: {{status}}",
            ok: "Ok",
            column: {
                id: "Index",
                name: "Taaknaam",
                type: "Type",
                execution_status: "Status",
                start_at: "Starttijd",
                end_at: "Eindtijd",
                executed_by_user: "Uitgevoerd door",
                annotations: "Annotaties",
                labels: "Labels",
                logs: "Logs",
            },
            tasksExecution: {
                DELETE_TENANT: "Tenant verwijderen",
                PUBLISH_BALLOT: "Stembiljet publiceren",
                VOTER_INFORMATION_LETTER: "Kiezersinformatiebrief",
                EXPORT_MONITORING_DATA: "Monitoringgegevens Exporteren",
                AUDIT_ELECTORAL_LOG: "Verkiezingslog controleren",
                EXPORT_ELECTION_EVENT: "Verkiezingsevenement Exporteren",
                CREATE_ELECTION_EVENT: "Verkiezingsevenement Aanmaken",
                IMPORT_ELECTION_EVENT: "Verkiezingsevenement Importeren",
                IMPORT_USERS: "Gebruikers Importeren",
                EDIT_USER: "Kiezer Bewerken",
                IMPORT_CANDIDATES: "Kandidaten Importeren",
                EXPORT_VOTERS: "Kiezers Exporteren",
                CREATE_TRANSMISSION_PACKAGE: "Transmissiepakket Aanmaken",
                EXPORT_BALLOT_PUBLICATION: "Publicatie Stembiljet Exporteren",
                EXPORT_ACTIVITY_LOGS_REPORT: "Activiteitenlogboek Rapport Exporteren",
                EXPORT_TENANT_LOGS: "Tenantlog Exporteren",
                GENERATE_REPORT: "Rapport Genereren",
                GENERATE_TRANSMISSION_REPORT: "Transmissierapport Genereren",
                EXPORT_TRUSTEES: "Trustees Exporteren",
                EXPORT_APPLICATION: "Applicaties Exporteren",
                EXPORT_TENANT_CONFIG: "Tenantconfiguratie Exporteren",
                IMPORT_TENANT_CONFIG: "Tenantconfiguratie Importeren",
                APPLY_CONFIGURATION_PACKAGE: "Configuratiepakket Toepassen",
                RENDER_DOCUMENT_PDF: "Document als PDF renderen",
                CREATE_TENANT: "Tenant Aanmaken",
                EXPORT_TEMPLATES: "Sjablonen Exporteren",
                IMPORT_TEMPLATES: "Sjablonen Importeren",
                DELETE_ELECTION_EVENT: "Verkiezingsevenement Verwijderen",
                DELETE_VOTERS: "Kiezers verwijderen",
                PREPARE_PUBLICATION_PREVIEW: "De publicatievoorbeeldweergave voorbereiden",
                EXPORT_TALLY_RESULTS_XLSX: "Exporteer de telresultaten in XLSX-indeling",
                EXPORT_CERTIFICATE_AUTHORITIES: "Certificaatautoriteiten exporteren",
                PUBLISH_RESULTS_WEBSITE: "Resultatenwebsite publiceren",
            },
            documentAccess: {
                title: "Documenttoegang",
                sensitivityNotice:
                    "Gevoelige informatie. Deel dit wachtwoord alleen met de beoogde ontvanger.",
                passwordLabel: "Wachtwoord om de versleutelde PDF te openen",
                showPassword: "Wachtwoord tonen",
                copyPassword: "Wachtwoord kopiëren",
                passwordCopied: "Wachtwoord gekopieerd",
                passwordError: "Het PDF-wachtwoord kon niet worden opgehaald",
                copyError: "Het wachtwoord kon niet worden gekopieerd",
                guidance:
                    "Het wachtwoord wordt pas geladen nadat u Wachtwoord tonen kiest. Daarna verschijnt hier een alleen-lezen veld met een kopieeroptie.",
            },
            widget: {
                taskTitle: "Taak: {{title}}",
                viewTask: "Taak Bekijken",
                downloadDocument: "Bestand Downloaden",
                downloadHashManifest: "Hashmanifest",
            },
            exportTasksExecution: {
                success: "Export succesvol voltooid",
                error: "Fout bij exporteren van taakuitvoering",
            },
        },
        areas: {
            common: {
                title: "Gebieden",
                subTitle: "Configuratie van gebieden.",
                deleteError: "Fout bij verwijderen van gebied",
            },
            createAreaSuccess: "Gebied aangemaakt",
            updateAreaSuccess: "Gebied bijgewerkt",
            createAreaError: "Kon gebied niet aanmaken",
            sequent_backend_area_contest: "Stemmingen",
            empty: {
                header: "Nog geen gebieden.",
                action: "Gebied aanmaken",
            },
            formImputs: {
                allowEarlyVoting: "Vroeg Stemmen Toestaan",
            },
        },
        integrationsScreen: {
            common: {
                gapiKey: "Google Calendar Service Account Sleutel",
                gapiEmail: "Google Calendar Authenticatie E-mail",
                gapiKeyHelper:
                    "De opgeslagen sleutel wordt niet getoond. Plak een nieuwe sleutel om deze te vervangen.",
                gapiKeySaved: "Google Calendar Service Account Sleutel opgeslagen",
            },
            errors: {
                invalidGapiKey: "Ongeldig formaat voor Google Calendar Service Account Sleutel",
                saveGapiKey: "Kon de Google Calendar Service Account Sleutel niet opslaan",
            },
        },
        lookAndFeelScreen: {
            common: {
                helpLinks: "Helplinks",
                logoUrl: "Logo-URL",
                css: "Aangepaste CSS",
                displayName: "Weergavenaam",
                displayNameHelp:
                    "De naam van de organisatie in berichten die haar noemen. Leeg: de korte naam van de tenant.",
            },
            errors: {
                invalidHelpLinks: "Ongeldig formaat voor Help Links",
            },
        },
        electionTypeScreen: {
            noPermissions: "U hebt geen toestemming om toegang te krijgen tot instellingen.",
            common: {
                title: "Verkiezingstype",
                subtitle: "Configuratie verkiezingstype",
                onlineVoting: "Online Stemmen",
                kioskVoting: "Kiosk Stemmen",
                telephoneVoting: "Telefonisch Stemmen",
                settingTitle: "Instellingen",
                settingSubtitle: "Algemene Configuratie",
                createNew: "Verkiezingstype aanmaken",
                emptyHeader: "Nog geen verkiezingstypes.",
                emptyBody: "Wilt u er een aanmaken?",
            },
            create: {
                title: "Verkiezingstype aanmaken",
            },
            edit: {
                title: "Verkiezingstype bewerken",
            },
            tabs: {
                votingChannels: "STEMKANALEN",
                electionTypes: "VERKIEZINGSTYPES",
                languages: "TALEN",
                localization: "LOCALISATIE",
                integrations: "INTEGRATIES",
                lookAndFeel: "LOOK & FEEL",
                schedules: "GEPLANDE GEBEURTENISSEN",
                trustees: "TRUSTEES",
                BackupRestore: "BACK-UP / HERSTELLEN",
                logs: "LOGS",
            },
        },
        trusteesSettingsScreen: {
            common: {
                emptyHeader: "Nog geen trustees.",
                createNew: "Trustee aanmaken",
                title: "Trustee",
                subtitle: "Configuratie trustee",
                emptyBody: "Wilt u er een aanmaken?",
            },
            create: {
                title: "Trustee aanmaken",
            },
            edit: {
                title: "Trustee bewerken",
            },
        },
        scheduleScreen: {
            noPermissions: "U hebt geen toestemming om toegang te krijgen tot instellingen.",
            createScheduleSuccess: "Schema aangemaakt",
            createScheduleError: "Fout bij aanmaken van schema",
            deleteScheduleSuccess: "Schema verwijderd",
            deleteScheduleError: "Fout bij verwijderen van schema",
            common: {
                title: "Schema's",
                subtitle: "Configuratie schema's",
                createNew: "Schema aanmaken",
                emptyHeader: "Nog geen schema's.",
                emptyBody: "Wilt u er een aanmaken?",
            },
            create: {
                title: "Schema aanmaken",
                selectSchedule:
                    "Selecteer een schema uit de vooraf gedefinieerde lijst of maak een aangepast schema aan",
            },
            edit: {
                title: "Schema bewerken",
            },
            eventTypes: {
                SYSTEM_LOCKDOWN_FOR_INTERNET_VOTING_SETTINGS:
                    "Systeemvergrendeling voor finalisatie van instellingen voor online stemmen",
                START_PRE_REGISTRATION_OVCS: "Start en einde van pre-registratie voor OVCS",
                END_PRE_REGISTRATION_OVCS: "Einde pre-registratie voor OVCS",
                START_TEST_VOTING_PERIOD: "Start teststemperiode",
                END_TEST_VOTING_PERIOD: "Einde teststemperiode",
                START_INTERNET_VOTING_PERIOD: "Start online stemperiode",
                END_INTERNET_VOTING_PERIOD: "Einde online stemperiode",
                LAB_TEST: "Labotest",
                FIELD_TEST: "Veldtest",
                MOCK_ELECTIONS: "Proefverkiezingen",
                FTS: "FTS",
            },
        },
        dashboard: {
            voteByDay: "Stemmen per dag",
            votesOverTime: "Stemmen in de tijd",
            timeResolution: "Tijdsresolutie",
            timeRange: "Tijdsbereik",
            minute: "Minuut",
            hour: "Uur",
            day: "Dag",
            votersByChannels: "Kiezers per kanaal",
            voterLoginURL: "Login URL kiezer",
            voterEnrollURL: "Inschrijvings-URL kiezer",
            voterEnrollKioskURL: "Inschrijvings-URL kiosk kiezer",
            ballotBoxes: {
                show: "Stembussen tonen",
                loadError:
                    "De zegels van de stembussen konden niet worden gelezen. Laad de pagina opnieuw of controleer de verbinding met de server.",
                title: "Stembussen",
                sealing:
                    "De stemming is gesloten om {{closed}}. De stembussen worden verzegeld wanneer de respijttermijn eindigt, om {{deadline}}.",
                sealed: "De stemming is gesloten om {{closed}}. De stembussen zijn verzegeld: er kan geen stembiljet meer worden toegevoegd, gewijzigd of verwijderd.",
                failed: "De stemming is gesloten om {{closed}}. Een stembus kon niet worden verzegeld: deze blijft vergrendeld en het incident staat in de logs.",
                closedBySignatures:
                    "Gesloten door {{names}} met hun certificaten, ondertekeningscode {{code}}.",
                closedByUser: "Gesloten door {{username}}.",
                closedBySchedule: "Gesloten door de geplande sluiting van de stemming.",
                column: {
                    area: "Gebied",
                    status: "Status",
                    inTheBox: "In de stembus",
                    counted: "Geteld",
                    sealedAt: "Verzegeld",
                    sealHash: "Zegelhash",
                    record: "Zegelrecord",
                },
                status: {
                    open: "Open",
                    sealing: "Verzegeling om {{time}}",
                    publishing: "Verzegeld, wordt gepubliceerd",
                    sealed: "Verzegeld",
                    failed: "Niet verzegeld: incident",
                    due: "Wordt nu verzegeld",
                    overdue: "Verzegeling achterstallig",
                },
                help: {
                    publishing:
                        "De stembus is vergrendeld en het zegel staat op het prikbord. Het zegelrecord wordt gepubliceerd.",
                    counted:
                        "Stembiljetten die tellen: het laatste geldige stembiljet van elke stemgerechtigde kiezer. De overige in de stembus zijn vervangen door een later stembiljet van de kiezer, afgekeurd, of uitgebracht door een kiezer die niet stemgerechtigd is.",
                },
                copyHash: "Zegelhash kopiëren",
                copied: "Zegelhash gekopieerd",
                copyError: "Kon de zegelhash niet kopiëren",
                notYet: "Nog niet",
                openRecord: "Het zegelrecord van {{area}} openen",
                downloadRecord: "Het zegelrecord van {{area}} downloaden",
                recordRestricted:
                    "Beperkt: vraag het aan een beheerder die documenten mag downloaden.",
                recordError: "Het zegelrecord kon niet worden gedownload. Probeer het opnieuw.",
                recordMissing: "Het document van het zegelrecord ontbreekt: meld het als incident.",
                beforeClose: "De stembus van elk gebied wordt verzegeld wanneer de stemming sluit.",
                notStarted:
                    "De stemming is nog niet geopend. De stembus van elk gebied wordt verzegeld wanneer de stemming sluit.",
                openOn: "De stemming is open via {{channels}}. De stembus van elk gebied wordt verzegeld wanneer de stemming sluit.",
                paused: "De stemming is gepauzeerd. De stembus van elk gebied wordt verzegeld wanneer de stemming sluit.",
                holding_one:
                    "{{channels}} is ingeschakeld en niet gesloten: stop dit kanaal om de stembussen te verzegelen.",
                holding_other:
                    "{{channels}} zijn ingeschakeld en niet gesloten: stop deze kanalen om de stembussen te verzegelen.",
                sealingNow:
                    "De stemming is gesloten om {{closed}}. De stembussen worden nu verzegeld.",
                sealingPastGrace:
                    "De stemming is gesloten om {{closed}}. De respijttermijn is geëindigd om {{deadline}}; de stembussen worden nu verzegeld.",
                why: {
                    due: "Wordt verzegeld: dit duurt maximaal een minuut.",
                    channelOpen:
                        "{{channel}} is nog ingeschakeld en niet gesloten: stop dit kanaal om de stembus te verzegelen.",
                    channelNotEnabled:
                        "{{channel}} is niet gesloten en niet ingeschakeld voor deze verkiezing: stop het om de stembus te verzegelen.",
                    channelHasBallots:
                        "{{channel}} heeft stembiljetten in deze stembus en is niet gesloten: stop dit kanaal om de stembus te verzegelen.",
                    datafixVotes_one:
                        "{{count}} stem is in behandeling in Datafix: de stembus wordt verzegeld zodra deze is afgehandeld.",
                    datafixVotes_other:
                        "{{count}} stemmen zijn in behandeling in Datafix: de stembus wordt verzegeld zodra deze zijn afgehandeld.",
                    sequencer_one:
                        "{{count}} stembiljet staat nog niet op het prikbord: de stembus wordt verzegeld zodra de sequencer het heeft toegevoegd.",
                    sequencer_other:
                        "{{count}} stembiljetten staan nog niet op het prikbord: de stembus wordt verzegeld zodra de sequencer ze heeft toegevoegd.",
                    stale: "Laatste poging om {{time}}: de verzegelaar draait mogelijk niet. Controleer Beat en de seal-worker.",
                    notTried:
                        "Nog niet geprobeerd: de verzegelaar draait mogelijk niet. Controleer Beat en de seal-worker.",
                    errorCategory: {
                        board: "De laatste poging kon het prikbord niet bereiken; er wordt elke minuut opnieuw geprobeerd.",
                        census: "De laatste poging kon de kiezerslijst niet lezen; er wordt elke minuut opnieuw geprobeerd.",
                        keystore:
                            "De laatste poging kon de ondertekeningssleutel niet ophalen; er wordt elke minuut opnieuw geprobeerd.",
                        storage:
                            "De laatste poging kon het zegelrecord niet uploaden naar de bestandsopslag; er wordt elke minuut opnieuw geprobeerd.",
                        settings:
                            "De laatste poging kon de instellingen van de verkiezing niet lezen; er wordt elke minuut opnieuw geprobeerd.",
                        other: "De laatste poging is mislukt; er wordt elke minuut opnieuw geprobeerd. Het servicelogboek bevat de details.",
                        ballots:
                            "De laatste poging vond een stembiljet dat nog niet kan worden gelezen of nog in behandeling is; er wordt elke minuut opnieuw geprobeerd.",
                        database:
                            "De laatste poging kon niet worden voltooid in de database; er wordt elke minuut opnieuw geprobeerd.",
                    },
                },
                failure: {
                    ballotIdMismatch: "Een stembiljet komt niet overeen met zijn stembiljet-ID.",
                    missingContent: "Een stembiljet heeft geen inhoud of geen stembiljet-ID.",
                    unreadable: "Een stembiljet kan niet worden gelezen.",
                    inProgress: "Een stembiljet is nog in behandeling.",
                    alreadyOnBoard: "Er staat al een zegel voor deze stembus op het prikbord.",
                    noBoard: "Het verkiezingsevenement heeft geen prikbord.",
                    unknownChannel: "Een stembiljet heeft een onbekend stemkanaal.",
                    unrecorded: "Een stembiljet heeft geen stemregistratie op het prikbord.",
                },
                incident: {
                    title_one: "{{count}} stembus kon niet worden verzegeld",
                    title_other: "{{count}} stembussen konden niet worden verzegeld",
                    body: "Dit is een incident: elk van deze stembussen blijft vergrendeld en kan niet worden geteld. Volg het runbook voor een mislukte verzegeling.",
                    line: "{{election}}, {{area}}: {{reason}}",
                },
            },
            ipAddress: {
                emptyState: "Nog geen stemmen.",
                title: "IP-adressen",
                ip: "IP",
                country: "Land",
                VoteCount: "Aantal stemmen",
                ElectionName: "Naam verkiezing",
                VotersId: "Kiezer Id",
            },
        },
        electionEventScreen: {
            dataUnavailable: "Gegevens zijn niet beschikbaar voor dit verkiezingsevenement.",
            noAccessibleTabs:
                "Je hebt geen toegang tot beschikbare tabbladen voor dit verkiezingsevenement.",

            common: {
                subtitle: "Configuratie verkiezingsevenement.",
                showMore: "Toon meer",
                showLess: "Toon minder",
                adminPortal: "Beheerdersportaal",
                allowPublishAfterLockdown:
                    "Publicatie verkiezingsevenement enkel toestaan na vergrendeling",
                reset: "Aangepaste filter resetten",
            },
            edit: {
                general: "Algemeen",
                dates: "Datums",
                customUrls: "Aangepaste URL-prefix",
                votingPeriod: "Stemperiode",
                language: "Taal",
                allowed: "Toegestane stemkanalen",
                materials: "Ondersteunend materiaal",
                ballotReceipts: "Ontvangstbewijzen van stembiljetten",
                ballotDesign: "Ontwerp stembiljet",
                templates: "Sjablonen",
                reorder: "Volgorde verkiezingen wijzigen",
                advancedConfigurations: "Geavanceerde configuraties",
                importCandidates: "Kandidaten importeren",
                custom_filters: "Aangepaste filters",
                voter_authentication: "Kiezersauthenticatie",
                realm_attributes: "Keycloak realm-attributen",
                realm_attributes_load_error: "Fout bij het laden van Keycloak realm-attributen",
                realm_attributes_update_error:
                    "Fout bij het bijwerken van Keycloak realm-attributen",
                realm_attributes_not_loaded:
                    "Keycloak realm-attributen zijn niet geladen; wijzigingen zijn niet opgeslagen",
                password_policy: "Wachtwoordbeleid",
                password_policy_load_error: "Fout bij het laden van het Keycloak-wachtwoordbeleid",
                password_policy_update_error:
                    "Fout bij het bijwerken van het Keycloak-wachtwoordbeleid",
                password_policy_not_loaded:
                    "Het Keycloak-wachtwoordbeleid is niet geladen; wijzigingen zijn niet opgeslagen",
            },
            customUrls: {
                login: "Inloggen",
                enrollment: "Inschrijving",
            },
            localization: {
                emptyHeader: "Er zijn geen talen ingesteld voor het evenement",
                selectLanguage: "Selecteer Taal",
                notify: {
                    success: "Localisatie succesvol bijgewerkt",
                    error: "Bijwerken lokalisatie mislukt",
                    duplicateKey:
                        "Er bestaat al een overschrijving met deze sleutel en dit bereik.",
                    invalidDateTimeFormat:
                        "Ongeldige datum-/tijdnotatie. Gebruik de tokens yyyy, MM, dd, HH, mm, ss (bijv. dd/MM/yyyy HH:mm).",
                    invalidTimeZoneText: "Deze tekst moet {{placeholders}} behouden.",
                },
                common: {
                    title: "Localisatie",
                    subTitle: "Configuratie lokalisatie",
                },
                labels: {
                    key: "Sleutel",
                    scope: "Portalbereik",
                    value: "Waarde",
                },
                scopes: {
                    legacy: "Verouderd ({{portal}})",
                    global: "Globaal",
                    votingPortal: "Stemportaal",
                    ballotVerifier: "Stembiljetverificatie",
                    resultsPortal: "Resultatenportaal",
                    adminPortal: "Beheerportaal",
                    templates: "Rapporten en berichten",
                },
            },
            field: {
                passwordPolicy: {
                    minimumLength: "Minimale lengte",
                    maximumLength: "Maximale lengte",
                    includeUppercase: "Hoofdletters opnemen",
                    includeLowercase: "Kleine letters opnemen",
                    includeDigits: "Cijfers opnemen",
                    includeSpecialCharacters: "Speciale tekens opnemen",
                    help: {
                        minimumLength:
                            "Het minimale aantal tekens dat voor het wachtwoord vereist is.",
                        maximumLength:
                            "Het maximale aantal tekens dat voor het wachtwoord is toegestaan.",
                        includeUppercase:
                            "Het wachtwoord moet ten minste één hoofdletter bevatten.",
                        includeLowercase:
                            "Het wachtwoord moet ten minste één kleine letter bevatten.",
                        includeDigits: "Het wachtwoord moet ten minste één cijfer bevatten.",
                        includeSpecialCharacters:
                            "Het wachtwoord moet ten minste één speciaal teken bevatten.",
                    },
                    notConfigured:
                        "Er is geen wachtwoordbeleid geconfigureerd. Bij het opslaan worden de onderstaande standaardwaarden toegepast.",
                    errors: {
                        lengthRange:
                            "De waarden voor de wachtwoordlengte moeten gehele getallen tussen 1 en 256 zijn.",
                        minimumExceedsMaximum:
                            "De minimale lengte mag niet groter zijn dan de maximale lengte.",
                        characterClassRequired:
                            "Selecteer ten minste één tekenklasse voor het wachtwoord.",
                    },
                },
                name: "Naam",
                alias: "Alias",
                description: "Beschrijving",
                startDateTime: "Startdatum en -tijd",
                endDateTime: "Einddatum en -tijd",
                language: "Taal",
                votingChannels: "Stemkanalen",
                materialActivated: "Ondersteunend materiaal geactiveerd",
                supportMaterialsPolicy: {
                    label: "Beleid voor Ondersteunend Materiaal",
                    helperText:
                        "Verplicht voor Stemmen vereist dat kiezers elk Ondersteunend Materiaal openen en bevestigen dat ze het hebben gelezen voordat ze kunnen stemmen.",
                    options: {
                        off: "Uit",
                        optional: "Optioneel",
                        mandatory_for_voting: "Verplicht voor Stemmen",
                    },
                },
                materialTitle: "Titel",
                materialSubTitle: "Ondertitel",
                logoUrl: "Logo-URL",
                userVerification:
                    "U kunt een aangepast sjabloon invoeren dat zal worden gebruikt om kiezers handmatig te verifiëren",
                redirectFinishUrl: "Doorschakel-URL na voltooiing",
                kioskRedirectFinishUrl: "Doorstuur-URL na afronding van kioskstemmen",
                css: "Aangepaste CSS",
                skipElectionList: "Scherm verkiezingslijst overslaan",
                showUserProfile: "Gebruikersprofiel tonen",
                ballotReceipts: {
                    checksPeriod: {
                        policyLabel: "Periode voor het controleren van uitgebrachte stemmen",
                        helper: "Hoe lang kiezers hun uitgebrachte stem kunnen opzoeken en het bewijs ervan kunnen afdrukken in het Stemportaal.",
                        options: {
                            "unlimited": "Geen limiet",
                            "until-date": "Tot een datum",
                        },
                    },
                    checksAvailableUntil: "Controles beschikbaar tot ({{timezone}})",
                    checksAvailableUntilRequired:
                        "Voer de datum en tijd in tot wanneer stemmen gecontroleerd kunnen worden.",
                },
                voterAccessibilitySettingsPolicy: {
                    policyLabel: "Toegankelijkheidsinstellingen voor de kiezer",
                    options: {
                        disabled: "Toegankelijkheidsinstellingen verbergen",
                        enabled: "Tekstgrootte, contrast, tekstafstand en beweging aanbieden",
                    },
                },
                audioInstructionsPolicy: {
                    policyLabel: "Gesproken instructies",
                    options: {
                        "disabled": "Geen gesproken instructies",
                        "recorded": "Alleen geüploade opnamen",
                        "recorded-or-synthesized":
                            "Geüploade opnamen, of de stem van de browser waar er geen is",
                    },
                },
                showCastVoteLogs: {
                    policyLabel: "Logboeken stemmen tonen",
                    options: {
                        "show-logs-tab": "Tab log stemmen tonen",
                        "hide-logs-tab": "Tab log stemmen verbergen",
                    },
                },
                lockdownState: {
                    since: "Vergrendeld sinds {{date}}.",
                    scheduledEnd: "Einde van de vergrendeling gepland voor {{date}}.",
                    endScheduled: "Er is een einde van de vergrendeling gepland.",
                    noScheduledEnd: "Er is geen einde van de vergrendeling gepland.",
                    banner: "Dit verkiezingsevenement is vergrendeld.",
                    policyLabel: "Vergrendelingsstatus",
                    helperText:
                        "Plan het begin of einde van de vergrendelingsperiode om deze status te wijzigen.",
                    options: {
                        "locked-down": "Vergrendeld",
                        "not-locked-down": "Niet vergrendeld",
                    },
                },
                ballotBoxSealPolicy: {
                    policyLabel: "Beleid Stembusverzegeling",
                    helperText:
                        "Wanneer de stemming sluit, wordt de stembus van elk gebied verzegeld: een ondertekende hash van de stembiljetten gaat naar het prikbord, er kan geen stembiljet meer worden toegevoegd, gewijzigd of verwijderd, en de stemming kan niet opnieuw starten.",
                    locked: "Dit kan niet meer worden gewijzigd nadat de stemming is geopend.",
                    options: {
                        "seal-at-close": "Verzegelen bij sluiting",
                        "do-not-seal": "Niet verzegelen",
                    },
                    checking: "Controleren of de stemming is geopend…",
                    lockedUnknown:
                        "Vergrendeld: de verkiezingen konden niet worden gelezen, dus het is onbekend of de stemming is geopend.",
                    lockedOpened: "Vergrendeld: de stemming is geopend in {{names}}.",
                    lockedEvent: "Vergrendeld: de stemming is geopend in dit verkiezingsevenement.",
                    refused:
                        "Het Beleid Stembusverzegeling kan niet meer worden gewijzigd nadat de stemming is geopend.",
                    settingLocked:
                        "Met Verzegelen bij sluiting is het zegel afhankelijk van deze instelling.",
                    settingRefused:
                        "Deze instelling kan niet meer worden gewijzigd nadat de stemming is geopend: met Verzegelen bij sluiting is het zegel ervan afhankelijk.",
                    boardRefused:
                        "Het prikbord van het verkiezingsevenement kan niet meer veranderen nadat de stemming is geopend: met Verzegelen bij sluiting worden de zegels daarop geplaatst.",
                },
                ballotBoxSealRecordPolicy: {
                    policyLabel: "Zegelrecord van de stembus",
                    options: {
                        restricted: "Beperkt",
                        public: "Openbaar",
                    },
                    help: {
                        restricted:
                            "Alleen beheerders kunnen het downloaden; deel het met waarnemers.",
                        public: "Iedereen met de id's van het evenement kan het downloaden, zonder in te loggen; het toont hoe elk stembiljet is geteld.",
                    },
                },
                decodedBallots: {
                    policyLabel: "Gedecodeerde stembiljetten opnemen in de resultaten database",
                    options: {"included": "Opnemen", "not-included": "Niet opnemen"},
                },
                fileIntegrityPolicy: {
                    policyLabel: "Integriteit van het kiezersbestand",
                    helperText:
                        "Vereis de onafhankelijk gecertificeerde SHA-256 voordat kiezers worden geïmporteerd.",
                    options: {
                        OPTIONAL: "Optionele SHA-256",
                        REQUIRED_SHA256: "Verplichte SHA-256",
                    },
                },
                contestEncryptionPolicy: {
                    options: {
                        "single-contest": "Enkele Stemming",
                        "multiple-contests": "Meerdere Stemmingen",
                    },
                    policyLabel: "Encryptiebeleid stemmingen",
                },
                votingPortalDateTimeFormat: {
                    policyLabel: "Datum- en tijdnotatie van stemportaal",
                    helperText:
                        'Geldt voor het hele evenement. Om per taal te overschrijven, voeg de sleutel "votingPortalDateTimeFormat" toe op het tabblad Localisatie met de tokens yyyy, MM, dd, HH, mm, ss (bijv. dd/MM/yyyy HH:mm). Zie de documentatie voor details.',
                    options: {
                        "legacy-gb-24h": "Legacy GB 24h (dd/MM/yyyy HH:mm, 24u)",
                        "iso-local": "ISO lokaal (yyyy-MM-dd HH:mm)",
                        "us-12h": "VS 12 u (MM/dd/yyyy h:mm AM/PM)",
                        "locale-medium": "Locale Medium (middellange datum, korte tijd)",
                        "date-only": "Date Only (geen tijd)",
                        "custom": "Aangepast formaat",
                    },
                    customFormat: {
                        label: "Aangepast datum- en tijdformaat",
                        helperText:
                            "Gebruik de tokens yyyy, MM, dd, HH, mm, ss (bijv. dd/MM/yyyy HH:mm). Andere tekens worden letterlijk weergegeven.",
                        invalid:
                            "Ongeldig formaat. Gebruik ten minste een van de tokens yyyy, MM, dd, HH, mm, ss.",
                    },
                },
                countDownPolicyOptions: {
                    NO_COUNTDOWN: "Geen aftelling",
                    COUNTDOWN: "Aftellen",
                    COUNTDOWN_WITH_ALERT: "Aftellen met waarschuwing",
                    sectionTitle: "Stemmingsportaal",
                    policyLabel: "Aftelbeleid stemmingsportaal",
                    coundownSecondsLabel: "tijd in seconden voor vervaldatum om af te tellen",
                    alertSecondsLabel:
                        "tijd in seconden voor vervaldatum om uitlogwaarschuwing te tonen",
                },
                voterSigningPolicy: {
                    "policyLabel": "Ondertekeningsbeleid kiezer",
                    "no-signature": "Geen handtekening",
                    "with-signature": "Met handtekening",
                },
                receiptsPolicy: {
                    "policyLabel": "Ontvangstbewijzen ondertekend door de stembus",
                    "disabled": "Uitgeschakeld",
                    "signed-by-ballot-box": "Ondertekend door de stembus",
                    "helperText":
                        "Indien ingeschakeld slaat de stembus elk stembiljet bij de controle op en ondertekent het, en ziet de kiezer pas een stembiljet-ID nadat het stembiljet is ontvangen. Kiezers ondertekenen hun stembiljetten. Publiceer de stembiljetten opnieuw na een wijziging.",
                    "lockedHelperText":
                        "Dit kan niet meer worden gewijzigd nadat het stemmen is begonnen.",
                },
                VoterCertificatePolicy: {
                    policyLabel: "Beleid voor digitaal kiezerscertificaat",
                    enabled: "Ingeschakeld",
                    disabled: "Uitgeschakeld",
                },
                enrollment: {
                    policyLabel: "Inschrijving",
                    options: {
                        enabled: "Ingeschakeld",
                        disabled: "Uitgeschakeld",
                    },
                },
                otp: {
                    policyLabel: "OTP",
                    options: {
                        enabled: "Ingeschakeld",
                        disabled: "Uitgeschakeld",
                    },
                },
                ceremoniesPolicy: {
                    policyLabel: "Sleutel-/Telceremoniebeleid",
                    options: {
                        "automated-ceremonies": "Automatische ceremonies toestaan",
                        "manual-ceremonies": "Handmatige ceremonies",
                    },
                },
                automaticRecountPolicy: {
                    policyLabel: "Automatische hertelling na importgoedkeuring",
                    options: {
                        enabled: "Ingeschakeld",
                        disabled: "Uitgeschakeld",
                    },
                },
                weightedVotingPolicy: {
                    policyLabel: "Beleid voor Gewogen Stemmen",
                    options: {
                        "areas-weighted-voting": "Gewogen Stemmen per Gebied",
                        "voters-weighted-voting": "Gewogen Stemmen per Kiezer",
                        "disabled-weighted-voting": "Gewogen Stemmen Uitgeschakeld",
                    },
                    noDelegated:
                        "Gewogen Stemmen per Kiezer kan niet worden gecombineerd met Gedelegeerd Stemmen",
                    noDecodedBallots:
                        "Gewogen Stemmen per Kiezer kan niet worden gecombineerd met het opnemen van ontsleutelde stembiljetten in de resultaten",
                },
                delegatedVotingPolicy: {
                    policyLabel: "Gedelegeerd stembeleid",
                    options: {
                        enabled: "Ingeschakeld",
                        disabled: "Uitgeschakeld",
                    },
                },
                batchAnonymityPolicy: {
                    policyLabel: "Beleid voor Anonimiteit van Batches",
                    helperText:
                        "Weigeren stopt de telling wanneer de stembiljettenbatch van een gebied minder dan vijf kiezers heeft, of verschilt van de batch die een andere telling al voor hetzelfde gebied en dezelfde verkiezing heeft gepubliceerd.",
                    options: {
                        warn: "Waarschuwen",
                        refuse: "Weigeren",
                    },
                },
                languageDetectionPolicy: {
                    policyLabel: "Taaldetectiebeleid",
                    options: {
                        "browser-detect": "Detecteren via browser",
                        "force-default": "Standaard afdwingen",
                    },
                },
            },
            error: {
                endDate: "Einddatum moet na startdatum liggen",
                startDate: "Startdatum moet in de toekomst liggen",
                noResult: "Nog geen verkiezingsevenement",
                endDateInvalid: "Einddatum moet in de toekomst liggen",
            },
            voters: {
                title: "Kiezers",
            },
            createElectionEventSuccess: "Verkiezingsevenement aangemaakt",
            createElectionEventError: "Fout bij aanmaken verkiezingsevenement",
            ivr: {
                tabs: {
                    config: "Configuratie",
                    blacklist: "Blokkeerlijst",
                    prompts: "Spraakprompts",
                    emulator: "Emulator",
                },
                common: {
                    saveSuccess: "Succesvol opgeslagen",
                    saveError: "Opslaan mislukt",
                    deleteSuccess: "Succesvol verwijderd",
                    deleteError: "Verwijderen mislukt",
                },
                config: {
                    configuredPhone: "Geconfigureerd telefoonnummer",
                    infoMsg:
                        "Configureer hieronder de IVR-flow en de bijbehorende eigenschappen. Neem voor meer informatie contact op met Sequent.",
                },
                prompts: {
                    emptyMsg: "Nog geen prompts aangemaakt",
                    infoMsg:
                        "Configureer de prompts die door de IVR worden gebruikt. Aankondigingsprompts zijn verplicht, en systeemprompts kunnen voor de gewenste talen worden overschreven. SSML wordt ondersteund, ook voor het mengen van talen.",
                    editorTitle: "Prompt",
                    editorSubtitle: "Promptconfiguratie",
                },
                blacklist: {
                    columns: {
                        phone: "Telefoonnummer",
                        reason: "Reden",
                        createdAt: "Aangemaakt op",
                        createdBy: "Aangemaakt door",
                        createdBefore: "Aangemaakt vóór",
                        createdAfter: "Aangemaakt na",
                    },
                    emptyMsg: "Er zijn geen vermeldingen in de blokkeerlijst",
                    infoMsg:
                        "Configureer de blokkeerlijst voor de IVR. Oproepen van deze nummers worden automatisch door het systeem verbroken.",

                    noFilterMatch: "Geen vermeldingen komen overeen met de opgegeven filters",
                    phoneRequired: "Telefoonnummer is verplicht",
                },
                emulator: {
                    infoMsg:
                        "Selecteer een gebied en de gewenste verkiezingen om de IVR-sessie te doorlopen.",
                    apiStatus: {
                        unavailable: "Het emulatorsysteem is niet beschikbaar in uw omgeving",
                        loading: "Het emulatorsysteem wordt geladen",
                        error: "Fout bij het laden van het emulatorsysteem",
                    },
                    hints: {
                        title: "Tips",
                        publishRequired:
                            "Wijzigingen aan verkiezingen, stemmingen of kandidaten moeten eerst worden gepubliceerd voordat ze beschikbaar zijn. Alleen de meest recent gepubliceerde stembiljetstijlen voor het overeenkomstige gebied worden in de emulator gebruikt.",
                        eventChangesImmediate:
                            "Wijzigingen aan het verkiezingsevenement, zoals de IVR-configuratie of aangepaste prompts, zijn direct beschikbaar nadat de emulatorsessie opnieuw is gestart.",
                        credentials: 'De geldige kiezer-ID en pincode zijn "123" en "123".',
                    },
                    sendDtmf: "DTMF-invoer verzenden",
                    keypadInput: "Toetsenbordinvoer",
                    sendTimeout: "Time-out verzenden",
                    disconnected: "Verbinding verbroken",
                    startSession: "Nieuwe sessie starten",
                    endSession: "Sessie beëindigen",
                    noStylesFound:
                        "Geen gepubliceerde stembiljetstijlen gevonden die overeenkomen met uw selecties",
                    inputPlaceholder: "Druk op {{keys}} (time-out={{timeout}} sec.)",
                    inputPlaceholderOr: "of",
                    inputPlaceholderAnyKeys:
                        "Voer maximaal {{maxDigits}} cijfers in (willekeurige cijfers, time-out={{timeout}} sec.)",
                    blacklistCaller: "Beller blokkeren",
                    elections: "Verkiezingen",
                    area: "Gebied",
                },
            },
            stats: {
                elegibleVoters: "Stemgerechtigde Kiezers",
                voters: "Effectieve Kiezers",
                elections: "Verkiezingen",
                contests: "Stemmingen",
                areas: "Gebieden",
                sentEmails: "Verzonden e-mails",
                sentSMS: "Verzonden SMS'en",
                calendar: {
                    title: "Kalender",
                    scheduled: "Gepland",
                },
            },
            keys: {
                createNew: "Sleutelceremonie aanmaken",
                emptyHeader: "Nog geen sleutelceremonie.",
                statusLabel: "Status",
                waitingKeys: "Wachten op sleutelgeneratie...",
                started: "Gestart op",
                actions: {
                    participate: "Deelnemen aan sleutelceremonie",
                    view: "Sleutelceremonie bekijken",
                },
                breadCrumbs: {
                    configure: "Configureren",
                    ceremony: "Ceremonie",
                    created: "Voltooid",
                    start: "Starten",
                    status: "Status",
                    download: "Downloaden",
                    check: "Controleren",
                    success: "Voltooid",
                },
                notify: {
                    participateNow:
                        "U bent uitgenodigd om deel te nemen aan een sleutelceremonie. Gelieve op de Sleutelactie van de ceremonie te klikken om deel te nemen.",
                },
            },
            tabs: {
                dashboard: "Dashboard",
                monitoring: "Monitoring",
                data: "Gegevens",
                ivr: "IVR",
                localization: "Localisatie",
                voters: "Kiezers",
                areas: "Gebieden",
                keys: "Sleutels",
                tally: "Telling",
                tallySheetImports: "Imports van processen-verbaal",
                publish: "Publiceren",
                logs: "Logs",
                tasks: "Taken",
                events: "Geplande Gebeurtenissen",
                notifications: "Meldingen",
                reports: "Rapporten",
                approvals: "Goedkeuringen",
                cas: "Certificaten",
                transparency: "Transparantie",
            },
            tally: {
                emptyHeader: "Nog geen telling.",
                title: "Telling Verkiezingsevenement",
                elections: "Verkiezingen",
                electionNumber: "Aantal Verkiezingen",
                trustees: "Trustees",
                permissionLabels: "Machtigingslabels",
                status: "Status",
                tallyType: {
                    label: "Type Telling",
                    ELECTORAL_RESULTS: "Verkiezingsresultaten",
                    INITIALIZATION_REPORT: "Initialisatierapport",
                },
                create: {
                    title: "Telling aanmaken",
                    subtitle: "Een nieuwe telling aanmaken voor dit verkiezingsevenement",
                    createTallyButton: "Telceremonie Starten",
                    createInitializationReportButton: "Initialisatierapport Genereren",
                    error: {
                        create: "Fout bij aanmaken telling",
                    },
                    success: "Telling aangemaakt",
                },
                logs: {
                    noLogs: "Geen logs beschikbaar",
                },
                notify: {
                    noKeysTally:
                        "De telceremonie kan niet starten voordat de sleutelceremonie succesvol is voltooid.",
                    noPublication:
                        "De telceremonie kan niet starten voordat u een publicatie aanmaakt in het tabblad Publiceren.",
                    participateNow:
                        "U bent uitgenodigd om deel te nemen aan een telceremonie. Gelieve op de Sleutelactie van de ceremonie te klikken om deel te nemen.",
                    startDisabled:
                        "U kunt de ceremonie niet voortzetten omdat er geen verkiezingen zijn geselecteerd of de verkiezingen niet zijn gepubliceerd.",
                    ceremonyDisabled:
                        "U kunt de ceremonie niet voortzetten omdat de telsessie niet is verbonden of de start van de ceremonie niet is toegestaan.",
                },
            },
            importAreas: {
                title: "Gebieden Importeren",
                subtitle: "Gegevens gebieden importeren",
                areaParagraph:
                    "Importeer gebieden met een spreadsheetbestand in CSV-formaat (Comma Separated Values).",
                importSuccess: "Gebieden succesvol geïmporteerd",
                importError: "Fout bij importeren van gebieden",
                upsert: "Gebieden 'Upserten'",
            },
            import: {
                eetitle: "Verkiezingsevenement Importeren",
                eesubtitle: "Gegevens verkiezingsevenement importeren",
                title: "Kiezers Importeren",
                subtitle: "Gegevens kiezers importeren",
                voters: "Kiezers",
                votersParagraph:
                    "Importeer kiezers met een spreadsheetbestand in CSV-formaat (Comma Separated Values). Download hier een voorbeeld CSV-importbestand.",
                electionEventParagraph: "Importeer Verkiezingsevenementen met een JSON-bestand.",
                elections: "Verkiezingen",
                areas: "Gebieden",
                sha: "Integriteitscontrole (SHA-256)",
                cancel: "Annuleren",
                import: "Importeren",
                fileUploadSuccess: "Bestand geüpload naar server - maar nog niet geïmporteerd",
                fileUploadError: "Fout bij uploaden bestand",
                importVotersSuccess: "Import kiezers succesvol ingepland",
                importVotersError: "Fout bij importeren van kiezers",
                importElectionEventSuccess: "Verkiezingsevenement succesvol geïmporteerd",
                importElectionEventError: "Fout bij importeren verkiezingsevenement",
                shaRequiredHelp:
                    "Voer de gecertificeerde hexadecimale SHA-256 van 64 tekens in. Deze is verplicht voor de import.",
                shaDialog: {
                    ok: "Ja, importeren zonder integriteitscontrole",
                    cancel: "Terug",
                    title: "Importeren zonder integriteitscontrole?",
                    description:
                        "U hebt het veld Integriteitscontrole (SHA-256) niet ingevuld. Bevestig dat u het juiste bestand importeert en dat u het wilt importeren.",
                },
                passwordDialog: {
                    title: "Ontsleutelingswachtwoord",
                    description: "Voer het wachtwoord in om het bestand te ontsleutelen",
                    label: "Wachtwoord",
                    copyPassword: "Wachtwoord Kopiëren",
                    ok: "Ok",
                },
            },
            export: {
                title: "Verkiezingsevenement Exporteren",
                subtitle: "Exporteren kan lang duren. Weet u zeker dat u records wilt exporteren?",
                encryptWithPassword: "Versleutelen met wachtwoord",
                passwordForcedNote:
                    "Het archief wordt hoe dan ook met een wachtwoord beveiligd: rapporten, aanvragen en bulletinboardgegevens worden altijd versleuteld. Vink het vakje aan om ook ontsleutelde geheime kiezersvelden op te nemen.",
                includeVoters: "Kiezers Opnemen",
                activityLogs: "Activiteitenlogs",
                bulletinBoard: "Prikbord",
                publications: "Publicaties",
                s3Files: "S3-bestanden",
                scheduledEvents: "Geplande Gebeurtenissen",
                exportSuccess: "Verkiezingsevenement succesvol geëxporteerd",
                exportError: "Fout bij exporteren verkiezingsevenement",
                passwordTitle: "Wachtwoord",
                passwordDescription: "Wachtwoord om het bestand te ontsleutelen:",
                copiedSuccess: "Wachtwoord gekopieerd naar klembord",
                copiedError: "Fout bij kopiëren wachtwoord",
                reports: "Rapporten",
                applications: "Applicaties",
                tally: "Telling",
                certificates: "Certificaten",
            },
            taskNotification:
                "{{action}} is gestart. U kunt de status ervan zien in de tabel Taakuitvoering.",
        },
        electionScreen: {
            common: {
                title: "Verkiezing",
                subtitle: "Configuratie verkiezing.",
                fileLoaded: "Bestand geladen",
                noPermission: "U hebt geen toestemming om toegang te krijgen tot deze verkiezing.",
            },
            edit: {
                general: "Algemeen",
                dates: "Datums",
                votingPeriod: "Stemperiode",
                language: "Taal",
                allowed: "Toegestane stemkanalen",
                default: "Standaard",
                defaultLang: "Standaardtaal",
                receipts: "Ontvangstbewijzen",
                image: "Afbeelding",
                advanced: "Geavanceerde Configuratie",
                numAllowedVotes: "Aantal toegestane stemmen",
                reorder: "Volgorde stemmingen wijzigen",
                castVoteConfirm: "Bevestigingsvenster stem uitbrengen",
                gracePeriodPolicy: "Respijttermijn",
                allowTallyPolicy: "Telling Toestaan",
                permissionLabel: "Machtigingslabel",
                custom_filters: "Aangepaste filters",
            },
            field: {
                name: "Naam",
                language: "Taal",
                votingChannels: "Stemkanalen",
                startDateTime: "Startdatum en -tijd",
                endDateTime: "Einddatum en -tijd",
                startDateTimeWithTimezone: "Startdatum en -tijd ({{timezone}})",
                endDateTimeWithTimezone: "Einddatum en -tijd ({{timezone}})",
                scheduledOpening: "Geplande Opening",
                scheduledClosing: "Geplande Sluiting",
                alias: "Alias",
                description: "Beschrijving",
                securityConfirmationHtml: "Beveiligingsbevestiging HTML",
                ivrPrompt: "IVR-bericht",
                externalId: "Externe ID",
            },
            securityConfirmationPolicy: {
                label: "Beleid voor selectievakje van beveiligingsbevestiging",
                none: "Geen",
                mandatory: "Verplicht",
            },
            error: {
                fileError: "Fout bij uploaden bestand",
                fileLoaded: "Bestand geladen",
                endDate: "Einddatum moet na startdatum liggen",
                startDate: "Startdatum moet in de toekomst liggen",
                endDateInvalid: "Einddatum moet in de toekomst liggen",
            },
            createElectionEventSuccess: "Verkiezingsevenement aangemaakt",
            createElectionEventError: "Fout bij aanmaken verkiezingsevenement",
            tabs: {
                dashboard: "Dashboard",
                monitoring: "Monitoring",
                data: "Gegevens",
                voters: "Kiezers",
                publish: "Publiceren",
                logs: "Logs",
                transparency: "Transparantie",
                approvals: "Goedkeuringen",
                tallySheets: "Tellerschema's",
            },
            gracePeriodPolicy: {
                "label": "Respijttermijnbeleid",
                "no-grace-period": "Geen respijttermijn",
                "grace-period-without-alert": "Respijttermijn zonder waarschuwing",
                "gracePeriodSecs": "Respijttermijn in seconden",
            },
            allowTallyPolicy: {
                "allowed": "Toegestaan",
                "disallowed": "Niet toegestaan",
                "requires-voting-period-end": "Vereist einde stemperiode",
            },
            initializeReportPolicy: {
                "label": "Initialisatierapportbeleid",
                "not-required": "Niet vereist",
                "required": "Vereist",
            },
            castVoteGoldLevelPolicy: {
                label: "Authenticatiebeleid 'Gold level'",
                options: {
                    "gold-level": "'Gold level' Authenticatie",
                    "no-gold-level": "Geen 'Gold level' Authenticatie",
                },
            },
            slates: {
                title: "Lijsten",
                configuration: "Lijstconfiguratie (JSON)",
                helper: "Benoemde lijsten en de kandidaten die elke lijst per verkiezingsonderdeel voordraagt. Laat leeg voor een verkiezing zonder lijsten.",
                loading:
                    "De verkiezingsonderdelen en kandidaten van de verkiezing worden nog geladen. Probeer het zo opnieuw.",
                mobileCandidateLists: {
                    label: "Kandidatenlijsten op mobiel",
                    helper: "Hoe de kandidatenlijst van elke lijst op een telefoon begint. De kiezer kan deze altijd openen of sluiten.",
                    options: {
                        collapsed: "Ingeklapt",
                        expanded: "Uitgeklapt",
                    },
                },
            },
            startScreenTitlePolicy: {
                label: "Titelbeleid startscherm",
                options: {
                    "election": "Verkiezingstitel",
                    "election-event": "Verkiezingsevenementtitel",
                },
            },
            consolidatedReportPolicy: {
                label: "Beleid voor geconsolideerd rapport",
                options: {
                    "generate": "Genereren",
                    "do-not-generate": "Niet genereren",
                },
            },
            declineToVotePolicy: {
                label: "Beleid voor afzien van stemmen",
                options: {
                    enabled: "Ingeschakeld",
                    disabled: "Uitgeschakeld",
                },
            },
            blankBallotsPolicy: {
                label: "Beleid voor blanco stembiljetten",
                options: {
                    enabled: "Ingeschakeld",
                    disabled: "Uitgeschakeld",
                },
            },
            votingScreenBackPolicy: {
                label: "Beleid voor de terugknop van het stemscherm",
                options: {
                    "election-selection-screen": "Ga naar het verkiezingskeuzescherm",
                    "start-screen": "Ga naar het startscherm van de verkiezing",
                },
            },
        },
        tenantScreen: {
            common: {
                title: "Tenants",
            },
            new: {
                subtitle: "Nieuwe tenant aanmaken",
            },
            createSuccess: "Tenant aangemaakt",
            createError: "Fout bij aanmaken tenant",
        },
        usersAndRolesScreen: {
            noPermissions:
                "U hebt geen toestemming om toegang te krijgen tot gebruikers of rollen.",
            common: {
                title: "Gebruikers en Rollen",
                subtitle: "Algemene configuratie",
                mobileNumber: "Mobiel nummer",
            },
            editPassword: {
                passwordPolicyViolation:
                    "Het wachtwoord voldoet niet aan het Wachtwoordbeleid voor deze verkiezingsgebeurtenis. Controleer het beleid onder Gegevens verkiezingsgebeurtenis en voer een geldig wachtwoord in.",
                passwordPolicyRules: {
                    minimumLength: "De minimale wachtwoordlengte is {{count}}.",
                    maximumLength: "De maximale wachtwoordlengte is {{count}}.",
                    uppercase: "Vereiste hoofdletters: {{count}}.",
                    lowercase: "Vereiste kleine letters: {{count}}.",
                    digits: "Vereiste cijfers: {{count}}.",
                    specialCharacters: "Vereiste speciale tekens: {{count}}.",
                },
                label: "Wachtwoord wijzigen",
                temporatyLabel: "Tijdelijk",
                temporatyInfo:
                    "Indien ingeschakeld, moet de gebruiker het wachtwoord wijzigen bij de volgende login",
            },
            users: {
                title: "Gebruikers",
                subtitle: "Gebruikersgegevens bekijken en bewerken",
                review: {
                    title: "Wijzigingen controleren",
                    subtitle: "Bevestig deze updates voordat u ze indient.",
                    confirm: "Wijzigingen bevestigen",
                    noChanges: "Geen wijzigingen om te controleren",
                    field: "Veld",
                    currentValue: "Huidige waarde",
                    newValue: "Nieuwe waarde",
                },
                edit: {
                    title: "Gebruikersgegevens",
                    subtitle: "Gebruiker bekijken en bewerken",
                },
                create: {
                    title: "Gebruiker",
                    subtitle: "Gebruiker aanmaken",
                },
                fields: {
                    "has_voted": "Gestemd",
                    "support_materials_viewed": "Ondersteunend materiaal bekeken",
                    "vote-weight": "Stemgewicht",
                    "voted-channel": "Stemkanaal",
                    "disable-comment": "Deactiveringsopmerking",
                    "username": "Gebruikersnaam",
                    "first_name": "Voornaam",
                    "last_name": "Achternaam",
                    "email": "E-mail",
                    "enabled": "Ingeschakeld",
                    "emailVerified": "E-mail geverifieerd",
                    "groups": "Groepen",
                    "attributes": "Attributen",
                    "area": "Gebied",
                    "password": "Wachtwoord",
                    "repeatPassword": "Wachtwoord herhalen",
                    "savePassword": "Wachtwoord opslaan",
                    "passwordMismatch": "Wachtwoorden moeten overeenkomen",
                    "passwordLengthValidate": "Wachtwoord moet minstens 8 tekens lang zijn",
                    "passwordUppercaseValidate":
                        "Wachtwoord moet minstens één hoofdletter bevatten",
                    "passwordLowercaseValidate":
                        "Wachtwoord moet minstens één kleine letter bevatten",
                    "passwordDigitValidate": "Wachtwoord moet minstens één cijfer bevatten",
                    "passwordSpecialCharValidate":
                        "Wachtwoord moet minstens één speciaal teken bevatten",
                    "trustee": "Optreden als Trustee",
                    "permissionLabel": "Machtigingslabel",
                    "authorized-election-ids": "Verkiezingen",
                },
                delete: {
                    body: "Weet u zeker dat u deze gebruiker wilt verwijderen?",
                    bulkBody: "Weet u zeker dat u de geselecteerde gebruikers wilt verwijderen?",
                    bulkBodySelected:
                        "De {{count}} geselecteerde gebruikers verwijderen? Dit kan niet ongedaan worden gemaakt.",
                    bulkBodyChoose:
                        "Er zijn {{count}} gebruikers geselecteerd. U kunt in plaats daarvan alle gebruikers verwijderen die aan de huidige filters voldoen; dat kunnen er meer zijn. Dit kan niet ongedaan worden gemaakt.",
                    okSelected: "{{count}} geselecteerde verwijderen",
                    okAllMatching: "Alle overeenkomende verwijderen",
                },
                notifications: {
                    exportError: "Fout bij exporteren gebruikers",
                    deleteError: "Fout bij verwijderen gebruiker",
                    deleteSuccess: "Gebruiker verwijderd",
                    multipleDeleteSuccess: "Gebruikers verwijderd",
                },
            },
            voters: {
                voterInformationLetter: {
                    label: "Kiezersinformatiebrief",
                    generate: "Genereren",
                    confirmation:
                        "Een Kiezersinformatiebrief voor deze kiezer genereren? Er wordt een nieuw wachtwoord toegewezen en opgenomen in een versleutelde PDF.",
                    generationStarted: "Het genereren van de Kiezersinformatiebrief is gestart",
                    generationError: "De Kiezersinformatiebrief kon niet worden gegenereerd",
                    policyNotConfigured:
                        "Het Wachtwoordbeleid is niet geconfigureerd. Stel het in onder Gegevens verkiezingsgebeurtenis voordat u een brief genereert.",
                    policyMinimumLengthMissing:
                        "Het Wachtwoordbeleid moet een minimale lengte bevatten voordat u een brief genereert.",
                    policyCharacterClassMissing:
                        "Het Wachtwoordbeleid moet ten minste één tekenklasse bevatten voordat u een brief genereert.",
                },
                title: "Kiezers",
                subtitle: "Kiezersgegevens bekijken en bewerken",
                secretAttribute: {
                    storedPlaceholder: "Opgeslagen versleutelde waarde",
                    reveal: "Tonen",
                    hide: "Verbergen",
                    revealError: "Het versleutelde kiezersveld kon niet worden getoond",
                    includeInExport: "Ontsleutelde geheime kiezersvelden opnemen",
                    exportWarning:
                        "Gevoelige export: de gedownloade CSV bevat deze velden als platte tekst.",
                    clear: "Wissen",
                    add: "Waarde toevoegen",
                    remove: "Waarde verwijderen",
                },
                review: {
                    title: "Wijzigingen controleren",
                    subtitle: "Bevestig deze updates voordat u ze indient.",
                    confirm: "Wijzigingen bevestigen",
                    noChanges: "Geen wijzigingen om te controleren",
                    field: "Veld",
                    currentValue: "Huidige waarde",
                    newValue: "Nieuwe waarde",
                },
                logs: {
                    label: "Logs van kiezer",
                },
                create: {
                    title: "Kiezer",
                    subtitle: "Kiezer aanmaken",
                },
                manualVerification: {
                    label: "Handmatig Verifiëren",
                    verify: "Deze kiezer handmatig verifiëren",
                    body: "Verifieer deze kiezer handmatig. U ontvangt een PDF met een QR-code link waarmee de kiezer kan inloggen zonder online KYC.",
                    noEmailOrPhone:
                        "Deze kiezer kan niet handmatig worden geverifieerd omdat er geen e-mailadres of telefoonnummer aan hen is toegewezen.",
                },
                emptyHeader: "Nog geen kiezers.",
                askCreate: "Wilt u er een aanmaken?",
                errors: {
                    editError: "Fout bij bewerken kiezer",
                    editErrorReason: "Fout bij bewerken kiezer: {{reason}}",
                    editSuccess: "Kiezer bewerkt",
                    createError: "Fout bij aanmaken kiezer",
                    createErrorReason: "Fout bij aanmaken kiezer: {{reason}}",
                    createSuccess: "Kiezer aangemaakt",
                    attribute: {
                        invalidNamed: '"{{field}}" is geweigerd: {{constraint}}',
                        fieldsToCorrect:
                            "Sommige velden moeten worden gecorrigeerd voordat u opslaat",
                        hintBetween: "Tussen {{min}} en {{max}} tekens",
                        hintMin: "Minimaal {{min}} tekens",
                        hintMax: "Maximaal {{max}} tekens",
                        andMore: "en nog {{count}}",
                        invalidLength: '"{{field}}" moet tussen {{min}} en {{max}} tekens bevatten',
                        tooShort: '"{{field}}" moet minimaal {{min}} tekens bevatten',
                        tooLong: '"{{field}}" mag maximaal {{max}} tekens bevatten',
                        required: '"{{field}}" is verplicht',
                        invalidEmail: '"{{field}}" moet een geldig e-mailadres zijn',
                        invalidFormat: '"{{field}}" heeft niet de verwachte indeling',
                        invalid: '"{{field}}" heeft een ongeldige waarde',
                    },
                    createPasswordError:
                        "Kiezer aangemaakt, maar het wachtwoord kon niet worden ingesteld",
                    createPasswordErrorReason:
                        "Kiezer aangemaakt, maar het wachtwoord kon niet worden ingesteld: {{reason}}",
                },
                delete: {
                    body: "Weet u zeker dat u deze kiezer wilt verwijderen?",
                    bulkBody: "Weet u zeker dat u de geselecteerde kiezers wilt verwijderen?",
                    bulkBodySelected:
                        "De {{count}} geselecteerde kiezers verwijderen? Dit kan niet ongedaan worden gemaakt.",
                    bulkBodyChoose:
                        "Er zijn {{count}} kiezers geselecteerd. U kunt in plaats daarvan alle kiezers verwijderen die aan de huidige filters voldoen; dat kunnen er meer zijn. Dit kan niet ongedaan worden gemaakt.",
                    okSelected: "{{count}} geselecteerde verwijderen",
                    okAllMatching: "Alle overeenkomende verwijderen",
                },
                notifications: {
                    exportError: "Fout bij exporteren kiezers",
                    deleteError: "Fout bij verwijderen kiezer",
                    deleteSuccess: "Kiezer verwijderd",
                    multipleDeleteSuccess: "Kiezers verwijderd",
                    manualVerificationError: "Fout bij handmatig verifiëren van de kiezer",

                    manualVerificationSuccess:
                        "Kiezer succesvol handmatig geverifieerd, PDF downloaden...",
                },
            },
            roles: {
                title: "Rollen",
                edit: {
                    title: "Rolgegevens",
                    subtitle: "Rol bekijken en bewerken",
                },
                create: {
                    title: "Rol",
                    subtitle: "Rol aanmaken",
                },
                errors: {
                    createError: "Fout bij aanmaken rol",
                    createSuccess: "Rol aangemaakt",
                },
                fields: {
                    name: "Naam",
                },
                delete: {
                    body: "Weet u zeker dat u deze rol wilt verwijderen?",
                },
                notifications: {
                    deleteError: "Fout bij verwijderen rol",
                    deleteSuccess: "Rol verwijderd",
                    permissionEditError: "Fout bij bewerken machtiging",
                    permissionEditSuccess: "Machtiging bewerkt",
                },
            },
            permissions: {
                "voter-information-letter": "Kiezersinformatiebrief genereren",
                "admin-user": "Beheerder Gebruiker",
                "admin-dashboard-view": "Beheerdersdashboard Bekijken",
                "monitoring-view": "Monitoringdashboards Bekijken",
                "monitoring-configure": "Monitoringdashboards Configureren",
                "election-event-signatures-tab": "Tabblad Handtekeningen Verkiezingsevenement",
                "signing-rules-read": "Handtekeningen: beschermde acties bekijken",
                "signing-rules-write": "Handtekeningen: beschermde acties bewerken",
                "signing-certificates-read": "Handtekeningen: certificaten bekijken",
                "signing-issuers-write":
                    "Handtekeningen: vertrouwde uitgevers importeren en verwijderen",
                "signing-checks-write": "Handtekeningen: certificaatcontroles bewerken",
                "signing-certificates-register": "Handtekeningen: certificaten registreren",
                "signing-certificates-revoke": "Handtekeningen: certificaten intrekken",
                "signing-requests-read": "Handtekeningen: verzoeken bekijken",
                "signing-requests-cancel": "Handtekeningen: verzoeken annuleren",
                "signing-requests-export": "Handtekeningen: verzoeken exporteren",
                "sign-initialize-voting": "Ondertekenen: stemming initialiseren",
                "sign-open-voting": "Ondertekenen: stemming openen",
                "sign-close-voting": "Ondertekenen: stemming sluiten",
                "sign-generate-election-returns": "Ondertekenen: processen-verbaal genereren",
                "sign-generate-reports": "Ondertekenen: andere verkiezingsrapporten genereren",
                "sign-transmit-results": "Ondertekenen: resultaten verzenden",
                "sign-approve-voter": "Ondertekenen: een kiezer handmatig goedkeuren",
                "sign-approve-configuration": "Ondertekenen: een configuratieversie goedkeuren",
                "sign-key-ceremony": "Ondertekenen: een sleutelfragment bevestigen",
                "sign-tally-key": "Ondertekenen: een sleutelfragment bijdragen",
                "application-export": "Applicatie Exporteren",
                "application-import": "Applicatie Importeren",
                "tenant-create": "Tenant Aanmaken",
                "tenant-read": "Tenant Lezen",
                "tenant-write": "Tenant Bewerken",
                "tenant-delete": "Tenant Verwijderen",
                "election-event-create": "Verkiezingsevenement Aanmaken",
                "election-event-read": "Verkiezingsevenement Lezen",
                "election-event-write": "Verkiezingsevenement Bewerken",
                "keycloak-realm-attributes-read": "Keycloak realm-attributen lezen",
                "keycloak-realm-attributes-write": "Keycloak realm-attributen bewerken",
                "election-event-delete": "Verkiezingsevenement Verwijderen",
                "voter-create": "Kiezer Aanmaken",
                "voter-read": "Kiezer Lezen",
                "voter-write": "Kiezer Bewerken",
                "voter-secret-attribute-read": "Geheime Kiezersvelden Tonen",
                "voter-secret-attribute-write": "Geheime Kiezersvelden Bewerken",
                "user-create": "Gebruiker Aanmaken",
                "user-read": "Gebruiker Lezen",
                "user-write": "Gebruiker Bewerken",
                "user-permission-create": "Gebruikersmachtiging Aanmaken",
                "user-permission-read": "Gebruikersmachtiging Lezen",
                "user-permission-write": "Gebruikersmachtiging Bewerken",
                "role-create": "Rol Aanmaken",
                "role-read": "Rol Lezen",
                "role-write": "Rol Bewerken",
                "role-assign": "Rol Toewijzen",
                "communication-template-create": "Communicatiesjabloon Aanmaken",
                "communication-template-read": "Communicatiesjabloon Lezen",
                "communication-template-write": "Communicatiesjabloon Bewerken",
                "notification-read": "Melding Lezen",
                "notification-write": "Melding Bewerken",
                "notification-send": "Melding Verzenden",
                "area-read": "Gebied Lezen",
                "area-write": "Gebied Bewerken",
                "election-state-write": "Verkiezingsstatus Bewerken",
                "election-type-create": "Verkiezingstype Aanmaken",
                "election-type-read": "Verkiezingstype Lezen",
                "election-type-write": "Verkiezingstype Bewerken",
                "voting-channel-read": "Stemkanaal Lezen",
                "voting-channel-write": "Stemkanaal Bewerken",
                "trustee-create": "Trustee Aanmaken",
                "trustee-read": "Trustee Lezen",
                "trustee-write": "Trustee Bewerken",
                "tally-read": "Telling Lezen",
                "tally-start": "Telling Starten",
                "tally-write": "Telling Bewerken",
                "tally-results-read": "Telresultaten Lezen",
                "publish-read": "Publicatie Lezen",
                "publish-write": "Publicatie Bewerken",
                "publish-results-read": "Resultatenpublicatie Lezen",
                "publish-results-write": "Resultatenpublicatie Bewerken",
                "logs-read": "Logs Lezen",
                "electoral-log-audit": "Verkiezingslog Controleren",
                "electoral-log-console-read": "Verkiezingslog Bekijken",
                "electoral-log-console-query": "Verkiezingslog Bevragen",
                "electoral-log-personal-data-read": "Persoonsgegevens van Verkiezingslog Lezen",
                "task-queues-read": "Taakwachtrijen Lezen",
                "task-queues-write": "Taakwachtrijen Beheren",
                "task-queues-query": "Taakwachtrijen Bevragen",
                "tasks-read": "Taakuitvoering Lezen",
                "keys-read": "Sleutels Lezen",
                "document-upload": "Documenten Uploaden",
                "document-download": "Documenten Downloaden",
                "document-password-read": "Documentwachtwoorden Lezen",
                "tally-sheet-create": "Telblad Aanmaken",
                "tally-sheet-import-create": "Telformulierimport aanmaken",
                "tally-sheet-import-review": "Telformulierimport beoordelen",
                "tally-sheet-import-view": "Telformulierimport bekijken",
                "tally-recount-execute": "Resultaten hertellen uitvoeren",
                "trustee-ceremony": "Trustee Ceremonie",
                "tally-sheet-review": "Telformulier beoordelen",
                "tally-sheet-view": "Telblad Bekijken",
                "admin-ceremony": "Beheerdersceremonie",
                "tally-sheet-delete": "Telblad Verwijderen",
                "document-read": "Documenten Lezen",
                "document-write": "Documenten Bewerken",
                "support-material-read": "Ondersteunend Materiaal Lezen",
                "support-material-write": "Ondersteunend Materiaal Bewerken",
                "miru-create": "Miru Aanmaken",
                "miru-download": "Miru Downloaden",
                "miru-send": "Miru Verzenden",
                "miru-sign": "Miru Ondertekenen",
                "contest-write": "Stemming Bewerken",
                "contest-read": "Stemming Lezen",
                "candidate-write": "Kandidaat Bewerken",
                "candidate-read": "Kandidaat Lezen",
                "permission-label-write": "Machtigingslabel Bewerken",
                "scheduled-event-write": "Geplande Gebeurtenissen Bewerken",
                "contest-create": "Stemming Aanmaken",
                "contest-delete": "Stemming Verwijderen",
                "candidate-create": "Kandidaat Aanmaken",
                "candidate-delete": "Kandidaat Verwijderen",
                "election-create": "Verkiezing Aanmaken",
                "election-read": "Verkiezing Lezen",
                "election-write": "Verkiezing Bewerken",
                "election-delete": "Verkiezing Verwijderen",
                "election-event-archive": "Verkiezingsevenement Archiveren",
                "election-data-tab": "Tabblad Verkiezingsgegevens Bekijken",
                "election-event-areas-tab": "Tabblad Gebieden Verkiezingsevenement Bekijken",
                "election-event-data-tab": "Tabblad Gegevens Verkiezingsevenement Bekijken",
                "election-event-keys-tab": "Tabblad Sleutels Verkiezingsevenement Bekijken",
                "election-event-logs-tab": "Tabblad Logs Verkiezingsevenement Bekijken",
                "election-event-transparency-tab":
                    "Tabblad Transparantie Verkiezingsevenement Bekijken",
                "election-event-publish-tab": "Tabblad Publicatie Verkiezingsevenement Bekijken",
                "election-event-reports-tab": "Tabblad Rapporten Verkiezingsevenement Bekijken",
                "election-event-scheduled-tab": "Tabblad Planning Verkiezingsevenement Bekijken",
                "election-event-tally-tab": "Tabblad Telling Verkiezingsevenement Bekijken",
                "election-event-tasks-tab": "Tabblad Taken Verkiezingsevenement Bekijken",
                "election-event-voters-tab": "Tabblad Kiezers Verkiezingsevenement Bekijken",
                "election-publish-tab": "Tabblad Verkiezingspublicatie Bekijken",
                "election-voters-tab": "Tabblad Kiezers Verkiezing Bekijken",
                "report-write": "Rapporten Bewerken",
                "report-read": "Rapporten Lezen",
                "users-menu": "Menu Gebruikers en Rollen Bekijken",
                "settings-menu": "Menu Instellingen Bekijken",
                "templates-menu": "Menu Sjablonen Bekijken",
                "settings-election-types-tab": "Tabblad Instellingen Verkiezingstypes Bekijken",
                "settings-voting-channels-tab": "Tabblad Instellingen Stemkanalen Bekijken",
                "settings-templates-tab": "Tabblad Instellingen Sjablonen Bekijken",
                "settings-languages-tab": "Tabblad Instellingen Talen Bekijken",
                "settings-localization-tab": "Tabblad Instellingen Localisatie Bekijken",
                "settings-look-feel-tab": "Tabblad Instellingen Look & Feel Bekijken",
                "settings-trustees-tab": "Tabblad Instellingen Trustees Bekijken",
                "settings-countries-tab": "Tabblad Instellingen Landen Bekijken",
                "voter-import": "Kiezer Importeren",
                "ee-voters-columns": "Kolommen Kiezers Verkiezingsevenement Bekijken",
                "voter-manually-verify": "Kiezer Handmatig Verifiëren",
                "ee-voters-logs": "Logs Kiezers Verkiezingsevenement Bekijken",
                "voter-export": "Kiezer Exporteren",
                "ee-voters-filters": "Filters Kiezers Verkiezingsevenement Bekijken",
                "voter-delete": "Kiezer Verwijderen",
                "voter-change-password": "Wachtwoord Kiezer Wijzigen",
                "election-event-localization-selector": "Localisatieselectie Verkiezingsevenement",
                "localization-create": "Localisatie Aanmaken",
                "localization-read": "Localisatie Lezen",
                "localization-write": "Localisatie Bewerken",
                "localization-delete": "Localisatie Verwijderen",
                "area-create": "Gebied Aanmaken",
                "area-delete": "Gebied Verwijderen",
                "area-export": "Gebied Exporteren",
                "area-import": "Gebied Importeren",
                "area-upsert": "Gebied 'Upserten'",
                "election-event-areas-columns": "Kolommen Gebieden Verkiezingsevenement",
                "election-event-areas-filters": "Filters Gebieden Verkiezingsevenement",
                "election-event-tasks-back-button": "Terug naar Taken Verkiezingsevenement",
                "election-event-tasks-columns": "Kolommen Taken Verkiezingsevenement",
                "election-event-tasks-filters": "Filters Taken Verkiezingsevenement",
                "task-export": "Taken Exporteren",
                "application-read": "Applicatie Lezen",
                "application-write": "Applicatie Bewerken",
                "approval-matrix-write": "Goedkeuringsmatrix Bewerken",
                "readiness-check": "Verkiezingsgereedheid controleren",
                "forensic-read": "Geïsoleerde forensische netwerkgegevens lezen",
                "voter-pseudonym-resolve":
                    "Een kiezerspseudoniem herleiden met een gecontroleerde reden",
                "logs-export": "Logs Exporteren",
                "tenant-logs-read": "Tenantlog Lezen",
                "tenant-logs-export": "Tenantlog Exporteren",
                "election-event-logs-columns": "Kolommen Logs Verkiezingsevenement",
                "election-events-logs-filters": "Filters Logs Verkiezingsevenement",
                "election-event-scheduled-event-columns":
                    "Kolommen Geplande Gebeurtenissen Verkiezingsevenement",
                "scheduled-event-create": "Geplande Gebeurtenis Aanmaken",
                "scheduled-event-delete": "Geplande Gebeurtenis Verwijderen",
                "election-event-reports-columns": "Kolommen Rapporten Verkiezingsevenement",
                "report-create": "Rapport Aanmaken",
                "report-delete": "Rapport Verwijderen",
                "report-generate": "Rapport Genereren",
                "report-preview": "Rapport Voorbeeldweergave",
                "monitor-authenticated-voters": "Monitoring Geauthenticeerde Kiezers",
                "monitor-all-approve-disapprove-voters":
                    "Monitoring Goedgekeurde/Afgekeurde Kiezers Lezen",
                "monitor-automatic-approve-disapprove-voters":
                    "Monitoring Automatisch Goedgekeurde/Afgekeurde Kiezers Lezen",
                "monitor-manually-approve-disapprove-voters":
                    "Monitoring Handmatig Goedgekeurde/Afgekeurde Kiezers Lezen",
                "monitor-enrolled-overseas-voters":
                    "Monitoring Ingeschreven Buitenlandse Kiezers Lezen",
                "monitor-posts-already-closed-voting":
                    "Monitoring Posten Reeds Gesloten Stemming Lezen",
                "monitor-posts-already-generated-election-results":
                    "Monitoring Posten Reeds Gegenereerde Verkiezingsresultaten Lezen",
                "monitor-posts-already-opened-voting":
                    "Monitoring Posten Reeds Geopende Stemming Lezen",
                "monitor-posts-already-started-counting-votes":
                    "Monitoring Posten Reeds Gestart Stemmen Tellen Lezen",
                "monitor-posts-initialized-the-system":
                    "Monitoring Posten Geïnitialiseerd Systeem Lezen",
                "monitor-posts-started-voting": "Monitoring Posten Gestart Stemmen Lezen",
                "monitor-posts-transmitted-results": "Monitoring Posten Verzonden Resultaten Lezen",
                "monitor-voters-voted-test-election":
                    "Monitoring Kiezers Gestemd Testverkiezing Lezen",
                "monitor-voters-who-voted": "Monitoring Kiezers Die Gestemd Hebben Lezen",
                "election-event-publish-preview":
                    "Voorbeeldweergave Publicatie Verkiezingsevenement",
                "election-event-publish-back-button": "Terug naar Publicatie Verkiezingsevenement",
                "election-event-publish-columns": "Kolommen Publicatie Verkiezingsevenement",
                "election-event-publish-filters": "Filters Publicatie Verkiezingsevenement",
                "publish-create": "Publicatie Aanmaken",
                "publish-regenerate": "Publicatie Regenereren",
                "publish-export": "Publicatie Exporteren",
                "publish-start-voting": "Stemmen Starten",
                "publish-pause-voting": "Stemmen Pauzeren",
                "publish-stop-voting": "Stemmen Stoppen",
                "publish-changes": "Wijzigingen Publiceren",
                "election-event-publish-view": "Publicatie Verkiezingsevenement Bekijken",
                "election-event-keys-columns": "Kolommen Sleutels Verkiezingsevenement",
                "create-ceremony": "Ceremonie Aanmaken",
                "export-ceremony": "Ceremonie Exporteren",
                "election-event-tally-columns": "Kolommen Telling Verkiezingsevenement",
                "election-event-tally-back-button": "Terug naar Telling Verkiezingsevenement",
                "transmition-ceremony": "Transmissieceremonie",
                "admin-ip-address-view": "IP-adres Bekijken (Admin)",
                "election-approvals-tab": "Tabblad Goedkeuringen Verkiezing Bekijken",
                "election-event-approvals-tab":
                    "Tabblad Goedkeuringen Verkiezingsevenement Bekijken",
                "election-ip-address-view": "IP-adres Verkiezing Bekijken",
                "election-dashboard-tab": "Tabblad Dashboard Verkiezing Bekijken",
                "trustees-export": "Trustees Exporteren",
                "user-import": "Gebruikers Importeren",
                "voter-voted-edit": "Kiezers die gestemd hebben bewerken",
                "voter-email-tlf-edit": "E-mail/telefoonvelden kiezers bewerken",
                "cloudflare-write": "Regels Landblokkering in Cloudflare Bewerken",
                "transmission-report-generate": "Transmissierapport Genereren",
                "google-meet-link": "Google Meet Link Genereren",
                "service-account": "Serviceaccount",
                "datafix-account": "Datafix-account",
                "gold": "Goud",
                "silver": "Zilver",
                "election-event-ivr-tab": "IVR van verkiezingsevenement bekijken",
                "election-event-cas-tab": "CAS van verkiezingsevenement bekijken",
                "ca-read": "Certificeringsinstanties lezen",
                "ca-write": "Certificeringsinstanties bewerken",
                "generate-preview": "Voorbeeld genereren",
                "preview-read": "Voorbeeld lezen",
                "tally-resolution-submit": "Tellingbesluit indienen",
                "phone-blacklist-read": "Telefoonblokkadelijst lezen",
                "phone-blacklist-create": "Vermeldingen aan de telefoonblokkadelijst toevoegen",
                "phone-blacklist-update": "Vermeldingen in de telefoonblokkadelijst bewerken",
                "phone-blacklist-delete": "Vermeldingen uit de telefoonblokkadelijst verwijderen",
                "election-event-voter-list-reconciliation":
                    "Kiezerslijst van verkiezingsevenement afstemmen",
                "messaging-account-read": "Berichtenaccounts bekijken",
                "messaging-account-write": "Berichtenaccounts beheren",
                "messaging-config-write": "Berichten van het verkiezingsevenement instellen",
                "messaging-config-read": "Berichten van het verkiezingsevenement bekijken",
            },
        },
        generalSettingsScreen: {
            body: "Schakel talen in het systeem in. Alleen talen die hier zijn ingeschakeld, zijn beschikbaar voor verkiezingsevenementen.",
        },
        eventsScreen: {
            title: "Geplande Gebeurtenissen",
            subtitle:
                "Beheert de configuratie van de automatische uitvoering van gebeurtenissen zoals de start of het einde van de stemperiode.",
            messages: {
                createSuccess: "Geplande gebeurtenis succesvol aangemaakt",
                createError: "Fout bij aanmaken geplande gebeurtenis",
                editSuccess: "Geplande gebeurtenis succesvol bewerkt",
                editError: "Fout bij bewerken geplande gebeurtenis",
                onlineWithEarlyVoting:
                    "Een startplanning kan online stemmen en vervroegd stemmen niet tegelijk openen: vervroegd stemmen moet vóór online stemmen beginnen.",
            },
            eventType: {
                label: "Type",
                ALLOW_INIT_REPORT: "Initialisatierapport Toestaan",
                START_VOTING_PERIOD: "Start Stemperiode",
                END_VOTING_PERIOD: "Einde Stemperiode",
                ALLOW_VOTING_PERIOD_END: "Einde Stemperiode Toestaan",
                START_ENROLLMENT_PERIOD: "Start Inschrijvingsperiode",
                END_ENROLLMENT_PERIOD: "Einde Inschrijvingsperiode",
                START_LOCKDOWN_PERIOD: "Start Vergrendelingsperiode",
                END_LOCKDOWN_PERIOD: "Einde Vergrendelingsperiode",
                ALLOW_TALLY: "Telling Toestaan",
                START_READINESS_TEST: "Verkiezingsgereedheidstest starten",
                END_READINESS_TEST: "Verkiezingsgereedheidstest beëindigen",
                START_FINAL_TESTING: "Eindtests en vergrendeling starten",
                END_FINAL_TESTING: "Eindtests en vergrendeling beëindigen",
                START_TEST_VOTING: "Teststemming starten",
                END_TEST_VOTING: "Teststemming beëindigen",
            },
            warning: {
                votingWindowDays:
                    "De stemperiode van {{election}} beslaat {{days}} lokale dagen ({{start_local}} tot {{end_local}}, {{time_zone}}); de regel vraagt {{expected}}.",
                finalTestingLeadTime:
                    "De eindtests van {{election}} beginnen op {{final_testing_local}}, minder dan {{minimum_days}} dagen voordat de stemming opent op {{voting_start_local}} ({{time_zone}}).",
                closeBeforeOpen:
                    "De stemming van {{election}} sluit op of vóór het moment van openen ({{start_local}} tot {{end_local}}, {{time_zone}}).",
                shortLastDay:
                    "De laatste stemdag van {{election}} heeft {{hours}} uur, minder dan {{minimum_hours}}: de stemming sluit op {{end_local}} ({{time_zone}}).",
            },
            election: {
                label: "Verkiezing",
            },
            empty: {
                header: "Nog geen geplande gebeurtenissen.",
                body: "Wilt u er een aanmaken?",
                button: "Geplande Gebeurtenis Aanmaken",
            },
            create: {
                title: "Geplande Gebeurtenis Aanmaken",
                subtitle: "Een nieuwe configuratie voor geplande gebeurtenissen aanmaken.",
            },
            edit: {
                title: "Geplande Gebeurtenis Bewerken",
                subtitle: "Configuratie geplande gebeurtenis bewerken.",
                delete: "Weet u zeker dat u deze geplande gebeurtenis wilt verwijderen?",
            },
            fields: {
                electionId: "Verkiezing",
                eventProcessor: "Type",
                stoppedAt: "Gestopt op",
                scheduledDate: "Gepland op",
            },
        },
        reportsScreen: {
            title: "Rapporten",
            subtitle: "Rapporten genereren voor de verkiezingsevenementen",
            messages: {
                createSuccess: "Rapport succesvol aangemaakt",
                createError: "Fout bij aanmaken rapport",
                submitError: "Fout bij indienen rapport",
                updateSuccess: "Rapport succesvol bijgewerkt",
                passwordMismatch:
                    "Wachtwoord en bevestig wachtwoord komen niet overeen. Zorg ervoor dat beide velden hetzelfde wachtwoord bevatten.",
                incorectPassword: "Incorrect wachtwoord",
                decryptFileTitle: "Hoe het bestand te ontsleutelen",
                decryptInstructions: `1. '-in': Het pad naar het versleutelde bestand.\n2. '-out': Het pad waar het ontsleutelde bestand wordt opgeslagen.\n3. '-pass': Het wachtwoord dat is gebruikt om het bestand te versleutelen.\n`,
                encryptSuccess: "Rapportversleuteling succesvol ingesteld",
                encryptError: "Fout bij het instellen van rapportversleuteling",
            },
            reportType: {
                BALLOT_RECEIPT: "Ontvangstbewijs Stembiljet",
                ELECTORAL_RESULTS: "Verkiezingsresultaten",
                MANUAL_VERIFICATION: "Handmatige Verificatie",
                PARTICIPATION_REPORT: "Participatierapport",
                STATISTICAL_REPORT: "Statistisch Rapport",
                OVCS_EVENTS: "Monitoring Stemmen Buitenland - OVCS Gebeurtenissen",
                AUDIT_LOGS: "Auditlogs",
                ACTIVITY_LOG: "Activiteitenlog",
                STATUS: "Status",
                OVCS_INFORMATION: "OVCS Informatie",
                OVERSEAS_VOTERS: "Lijst van buitenlandse kiezers",
                OV_USERS_WHO_VOTED: "Lijst van buitenlandse kiezers die gestemd hebben",
                OV_WITH_VOTING_STATUS: "Lijst van buitenlandse kiezers met stemstatus",
                OVCS_STATISTICS: "Monitoring Stemmen Buitenland - OVCS Statistieken",
                PRE_ENROLLED_OV_BUT_DISAPPROVED:
                    "Lijst van buitenlandse kiezers die vooraf zijn ingeschreven maar afgekeurd",
                PRE_ENROLLED_OV_SUBJECT_TO_MANUAL_VALIDATION:
                    "Lijst van buitenlandse kiezers die vooraf zijn ingeschreven maar onderhevig aan handmatige validatie",
            },
            reportEncryptionPolicy: {
                title: "Versleutelingsbeleid",
                UNENCRYPTED: "Niet versleuteld",
                CONFIGURED_PASSWORD: "Geconfigureerd Wachtwoord",
            },
            empty: {
                header: "Nog geen rapporten.",
                body: "Wilt u er een aanmaken?",
                button: "Rapport Aanmaken",
            },
            create: {
                title: "Rapport Aanmaken",
                subtitle: "Een nieuwe rapportconfiguratie aanmaken.",
            },
            edit: {
                title: "Rapport Bewerken",
                subtitle: "Rapportconfiguratie bewerken.",
                delete: "Weet u zeker dat u dit rapport wilt verwijderen?",
            },
            fields: {
                electionId: "Verkiezing",
                template: "Sjabloon",
                reportType: "Rapporttype",
                repeatable: "Herhaalbaar",
                cronExpression: "Cron Expressie",
                emailRecipients: "E-mailontvangers",
                emailRecipientsPlaceholder: "Typ e-mailadres en druk op Enter",
            },

            delete: {
                body: "Weet u zeker dat u dit rapport wilt verwijderen?",
            },
            actions: {
                generate: "Genereren",
                delete: "Verwijderen",
                edit: "Bewerken",
                preview: "Voorbeeldweergave",
            },
        },
        googleMeet: {
            title: "Google Meet Link Genereren",
            generateButton: "Meet Genereren",
            meetingTitle: "Vergadertitel",
            description: "Beschrijving (Optioneel)",
            startDate: "Startdatum",
            startTime: "Starttijd",
            duration: "Duur (minuten)",
            attendeeEmails: "Emails Deelnemers",
            attendeeEmailHelp: "Komma-gescheiden emails voor vergaderdeelnemers",
            note: "Opmerking: Dit zal een agenda-evenement aanmaken in uw Google Calendar met een Google Meet link. U moet inloggen op uw Google account.",
            success: "Google Meet Link Succesvol Gegenereerd!",
            copy: "Kopiëren naar klembord",
            copied: "Link gekopieerd naar klembord!",
            instructions:
                "Deel deze link met deelnemers om deel te nemen aan de vergadering. Het agenda-evenement is toegevoegd aan uw Google Calendar.",
            generating: "Genereren...",
            generate: "Meet Link Genereren",
        },
        common: {
            export: "Exporteren kan lang duren. Weet u zeker dat u records wilt exporteren?",
            resources: {
                electionEvent: "Verkiezingsevenement",
                election: "Verkiezing",
                contest: "Stemming",
                candidate: "Kandidaat",
                noResult: {
                    askCreate: "Wilt u er een aanmaken?",
                },
            },
            label: {
                add: "Toevoegen",
                actions: "Acties",
                create: "Aanmaken",
                delete: "Verwijderen",
                archive: "Archiveren",
                unarchive: "Dearchiveren",
                cancel: "Annuleren",
                edit: "Bewerken",
                yes: "Ja",
                no: "Nee",
                save: "Opslaan",
                close: "Sluiten",
                back: "Terug",
                next: "Volgende",
                warning: "Waarschuwing",
                json: "Voorbeeldweergave",
                noResult: "Geen resultaat",
                import: "Importeren",
                export: "Exporteren",
                loadingData: "Gegevens laden...",
                exportFormat: "Exporteren in {{format}}-formaat - Resultaten van '{{item}}",
                allResults: "verkiezingsevenement",
                globalAreaResults: "alle gebieden",
                title: "Titel",
                subtitle: "Ondertitel",
                kind: "Bestandstype",
                filter: "Aangepaste Filters",
                approve: "Goedkeuren",
                continue: "Doorgaan",
                logout: "Uitloggen",
                selectTenant: "Huurder selecteren",
                processing: "Bezig met verwerken...",
                tenantName: "Naam van de huurder",
            },
            language: {
                es: "Spaans",
                en: "Engels",
                fr: "Frans",
                cat: "Valenciaans",
                tl: "Tagalog",
                gl: "Galicisch",
                nl: "Nederlands",
                eu: "Euskera",
            },
            channel: {
                online: "Online",
                kiosk: "Kiosk",
                early_voting: "Vroeg stemmen",
                telephone: "Telefonisch stemmen",
                other: "Overig",
            },
            message: {
                delete: "Weet u zeker dat u dit item wilt verwijderen?",
                continueOrLogout: "Wil je doorgaan of uitloggen?",
            },
        },
        createResource: {
            electionEvent: "Een Verkiezingsevenement aanmaken",
            election: "Een Verkiezing aanmaken",
            contest: "Een Stemming aanmaken",
            candidate: "Een Kandidaat aanmaken",
        },
        importResource: {
            electionEvent: "Een Verkiezingsevenement importeren",
            election: "Een Verkiezing importeren",
            contest: "Een Stemming importeren",
            candidate: "Een Kandidaat importeren",
            ImportHashMismatch: "Hashes komen niet overeen. Integriteitscontrole mislukt.",
        },
        sideMenu: {
            electionEvents: "Verkiezingsevenementen",
            search: "Zoeken",
            usersAndRoles: "Gebruikers en Rollen",
            electoralLogConsole: "Verkiezingslog",
            taskQueues: "Taakwachtrijen",
            logs: "Logs",
            settings: "Instellingen",
            help: "Help",
            templates: "Sjablonen",
            active: "Actief",
            archived: "Gearchiveerd",
            addResource: {
                electionEvent: "Een Verkiezingsevenement aanmaken",
                election: "Een Verkiezing aanmaken",
                contest: "Een Stemming aanmaken",
                candidate: "Een Kandidaat aanmaken",
            },
            menuActions: {
                archive: {
                    electionEvent: "Dit Verkiezingsevenement archiveren",
                },
                unarchive: {
                    electionEvent: "Dit Verkiezingsevenement dearchiveren",
                    election: "Deze verkiezing dearchiveren",
                    contest: "Deze Stemming dearchiveren",
                    candidate: "Deze Kandidaat dearchiveren",
                },
                remove: {
                    electionEvent: "Dit Verkiezingsevenement verwijderen",
                    election: "Deze Verkiezing verwijderen",
                    contest: "Deze Stemming verwijderen",
                    candidate: "Deze Kandidaat verwijderen",
                },
                messages: {
                    confirm: {
                        archive: "Weet u zeker dat u dit item wilt archiveren?",
                        unarchive: "Weet u zeker dat u dit item wilt dearchiveren?",
                        delete: "Weet u zeker dat u dit item wilt verwijderen?",
                        sealsUnknown:
                            "De zegels van de stembussen konden niet worden gecontroleerd: als er verzegelde stembussen zijn, wordt het verwijderen geweigerd.",
                    },
                    notification: {
                        success: {
                            archive: "Het item is gearchiveerd",
                            unarchive: "Het item is gedearchiveerd",
                            delete: "Het item is verwijderd",
                            reloading:
                                "Wacht. De pagina wordt binnen enkele ogenblikken opnieuw geladen.",
                        },
                        error: {
                            archive: "Fout bij het archiveren van dit item",
                            unarchive: "Fout bij het dearchiveren van dit item",
                            delete: "Fout bij het verwijderen van dit item",
                            deleteSealedElection:
                                "Deze verkiezing heeft verzegelde stembussen en kan niet worden verwijderd. Archiveer in plaats daarvan het verkiezingsevenement.",
                            deleteSealedEvent:
                                "Dit verkiezingsevenement heeft verzegelde stembussen en kan niet worden verwijderd. Archiveer het in plaats daarvan.",
                            deleteMaybeSealed:
                                "Fout bij het verwijderen van dit item. Als het verzegelde stembussen heeft, kan het niet worden verwijderd.",
                        },
                    },
                },
            },
        },
        candidateScreen: {
            common: {
                subtitle: "Configuratie kandidaat.",
            },
            edit: {
                externalId: "Externe ID",
                general: "Algemeen",
                type: "Type",
                image: "Afbeelding",
                isDisabled: "Uitgeschakeld",
                isExplicitInvalid: "Ongeldige Stem",
                isExplicitBlank: "Blanco Stem",
                isCategoryList: "Categorielijst",
                isWriteIn: "Inschrijf",
            },
            field: {
                name: "Naam",
                alias: "Alias",
                description: "Beschrijving",
            },
            options: {
                "candidate": "Kandidaat",
                "option": "Optie",
                "write-in": "Inschrijf",
                "open-list": "Open Lijst",
                "closed-list": "Gesloten Lijst",
                "semi-open-list": "Semi-open Lijst",
                "invalid-vote": "Ongeldige Stem",
                "blank-vote": "Blanco Stem",
            },
            invalidVotePosition: {
                label: "Positie van Ongeldige Stem",
                null: "Geen (Standaard)",
                top: "Boven",
                bottom: "Onder",
            },
            error: {},
            createCandidateSuccess: "Kandidaat aangemaakt",
            createCandidateError: "Fout bij aanmaken kandidaat",
        },
        contestScreen: {
            common: {
                subtitle: "Configuratie stemming.",
            },
            edit: {
                externalId: "Externe ID",
                general: "Algemeen",
                type: "Type",
                image: "Afbeelding",
                system: "Stemsysteem Stembiljet",
                design: "Ontwerp Stembiljet",
                reorder: "Volgorde kandidaten wijzigen",
                policies: "Beleidsregels",
            },
            field: {
                name: "Naam",
                alias: "Alias",
                description: "Beschrijving",
            },
            options: {
                "non-preferential": "Niet-preferentieel",
                "plurality-at-large": "Meerderheidsstelsel (meervoudig)",
                "instant-runoff": "Instant-runoff stemmen",
                "random": "Willekeurig",
                "external-procedure": "Externe procedure",
                "custom": "Aangepast",
                "alphabetical": "Alfabetisch",
            },
            tieBreakingPolicy: {
                label: "Beleid voor gelijkspel",
            },
            auditButtonConfig: {
                "label": "Weergaveopties Auditknop",
                "show": "Tonen",
                "not-show": "Niet Tonen",
                "show-in-help": "Tonen in Helpvenster",
            },
            underVotePolicy: {
                "label": "Beleid Onderstemmen",
                "allowed": "Toegestaan",
                "warn-only-in-review": "Waarschuwen bij Controle",
                "warn": "Waarschuwen",
                "warn-and-alert": "Waarschuwen en Melden",
                "warn-and-confirm-in-review": "Waarschuwen en Bevestigen bij Controle",
            },
            invalidVotePolicy: {
                "label": "Beleid Ongeldige Stem",
                "allowed": "Toegestaan",
                "warn": "Waarschuwen",
                "warn-invalid-implicit-and-explicit":
                    "Waarschuwen bij Impliciet en Expliciet Ongeldig",
                "not-allowed": "Niet Toegestaan",
                "allowed-with-exclusive-explicit": "Toegestaan met Exclusief Expliciet Ongeldig",
            },
            candidatesIconCheckboxPolicy: {
                "label": "Vorm icoon selectievakje kandidaten",
                "square-checkbox": "Vierkant Selectievakje",
                "round-checkbox": "Rond Selectievakje",
            },
            checkableListPolicy: {
                "allow-selecting-candidates-and-lists": "Kandidaten en Lijsten",
                "allow-selecting-candidates": "Alleen Kandidaten",
                "allow-selecting-lists": "Alleen Lijsten",
                "disabled": "Uitgeschakeld",
            },
            collapsibleListsPolicy: {
                "label": "Inklapbare lijsten",
                "disabled": "Uitgeschakeld",
                "enabled-expanded": "Ingeschakeld (start uitgevouwen)",
                "enabled-collapsed": "Ingeschakeld (start ingeklapt)",
            },
            blankVotePolicy: {
                "label": "Beleid Blanco Stem",
                "allowed": "Toegestaan",
                "warn-only-in-review": "Waarschuwen bij Controle",
                "warn": "Waarschuwen",
                "not-allowed": "Niet Toegestaan",
            },
            overVotePolicy: {
                "label": "Beleid Overstemmen",
                "allowed": "Toegestaan",
                "allowed-with-msg": "Toegestaan met Waarschuwing",
                "allowed-with-msg-and-alert": "Toegestaan met Waarschuwing en Melding",
                "not-allowed-with-msg-and-alert": "Niet Toegestaan met Waarschuwing en Melding",
                "not-allowed-with-msg-and-disable":
                    "Niet Toegestaan met Waarschuwing en Verdere Selecties Uitschakelen",
            },
            duplicatedRankPolicy: {
                "label": "Ongeldige stem - Beleid Gedupliceerde Rang",
                "allowed-warn-and-dialog": "Toon waarschuwing en dialoog (kiezer kan doorgaan)",
                "not-allowed-warn-and-dialog":
                    "Toon waarschuwing en dialoog (kiezer mag niet doorgaan)",
            },
            preferenceGapsPolicy: {
                "label": "Ongeldige stem - Beleid Overgeslagen Rangen",
                "allowed-warn-and-dialog": "Toon waarschuwing en dialoog (kiezer kan doorgaan)",
                "not-allowed-warn-and-dialog":
                    "Toon waarschuwing en dialoog (kiezer mag niet doorgaan)",
            },
            paginationPolicy: {
                label: "Paginanaam",
            },
            isAcclaimed: {
                label: "Bij acclamatie beslist",
                helperText:
                    "Kiezers zien deze stemming maar kunnen niets selecteren, er wordt niets geregistreerd en alle kandidaten worden als winnaar met nul stemmen gerapporteerd. Stel dit in voordat stembiljetten worden gepubliceerd: het later wijzigen maakt reeds uitgebrachte stemmen ongeldig.",
            },
            allowWriteins: {
                label: "Handgeschreven kandidaten toestaan",
            },
            maxVotes: {
                helperText:
                    "Maximaal aantal kandidaten dat een kiezer kan selecteren (niet-preferentieel stemmen).",
                helperTextPreferential:
                    "Hoogste beschikbare rangpositie voor kiezers (bijv. '5' staat rangen 1–5 toe). Moet minimaal gelijk zijn aan het aantal te rangschikken kandidaten (preferentieel stemmen).",
            },
            error: {},
            createContestSuccess: "Stemming aangemaakt",
            createContestError: "Fout bij het aanmaken van de stemming",
        },
        keysGeneration: {
            ...englishTranslation.translations.keysGeneration,
            downloadStep: {
                ...englishTranslation.translations.keysGeneration.downloadStep,
                title: "Bewaringsbestand downloaden",
                downloadButton: "Bewaringsbestand downloaden",
                confirmdDialog: {
                    ...englishTranslation.translations.keysGeneration.downloadStep.confirmdDialog,
                    title: "Bewaringsbestand opslaan",
                },
            },
            checkStep: {
                ...englishTranslation.translations.keysGeneration.checkStep,
                subtitle:
                    "Upload uw opgeslagen bewaringsbestand of een ondertekend bewijs van bewaring:",
                required: "Bevestiging van bewaring vereist",
                reconfirmButton: "Bewaring opnieuw bevestigen (optioneel)",
                reconfirmDescription:
                    "Uw bewaring is al bevestigd. Deze aanvullende bevestiging is optioneel.",
            },
            serverCustody: {
                recoveryInstructions:
                    "Als u deze verkiezing hebt geïmporteerd of het opgeslagen bestand kwijt bent, download dan hier het huidige bewaringsbestand. Sla het op en upload het hieronder.",
                fileFormat: "Ondersteund formaat: JSON",
                waiting: "Wachten tot uw trusteeserver een bewijs van sleutelbezit maakt…",
                unavailable:
                    "De trusteeserver heeft niet gereageerd. Controleer of deze actief is en upload het bewaringsbestand opnieuw.",
                failed: "De trusteeserver kon het bewijs niet maken: {{error}}",
                invalidReply: "De trusteeserver gaf een onvolledig antwoord. Probeer het opnieuw.",
                cancelled:
                    "Het verzoek is geannuleerd. Upload het bestand opnieuw om verder te gaan.",
                downloadInstructions:
                    "Download en bewaar dit bewaringsbestand. Upload hetzelfde bestand bij de volgende stap en bij toekomstige tellingen. Het is geen back-up van uw privésleutels.",
                uploadInstructions:
                    "Upload het gedownloade bewaringsbestand. Uw trusteeserver maakt het bewijs. U kunt ook een bewijs uploaden dat door een offline trustee of browsertrustee is gemaakt.",
                tallyInstructions:
                    "Upload het bestand dat u tijdens de sleutelceremonie hebt bewaard. Uw trusteeserver maakt een nieuw bewijs voor deze telling. Bewijzen van offline trustees en browsertrustees worden ook geaccepteerd.",
            },
            configureStep: {
                create: "Sleutelceremonie Aanmaken",
                name: "Naam Sleutelceremonie",
                allElections: "Alle Verkiezingen",
                title: "Sleutelceremonie Verkiezingsevenement Aanmaken",
                subtitle:
                    "Elke beheerder genereert en bewaart de sleutels lokaal en bevestigt de bewaring. Kies de beheerders en het minimumaantal dat nodig is voor de telling.",
                threshold: "Drempelwaarde",
                trusteeList: "Trustees",
                errorMinTrustees_one:
                    "U selecteerde slechts {{selected}} trustee, maar u moet er minstens {{threshold}} selecteren.",
                errorMinTrustees_many:
                    "U selecteerde slechts {{selected}} trustees, maar u moet er minstens {{threshold}} selecteren.",
                errorMinTrustees_other:
                    "U selecteerde slechts {{selected}} trustees, maar u moet er minstens {{threshold}} selecteren.",
                errorThreshold:
                    "U selecteerde drempelwaarde {{selected}}, maar deze moet tussen {{min}} en {{max}} liggen.",
                errorCreatingCeremony: "Fout bij aanmaken sleutelceremonie: {{error}}",
                createCeremonySuccess: "Sleutelceremonie aangemaakt",
                confirmdDialog: {
                    ok: "Ja, Sleutelceremonie Aanmaken",
                    cancel: "Annuleren",
                    title: "Weet u zeker dat u de Sleutelceremonie wilt aanmaken?",
                    automaticCeremonyTitle:
                        "Weet u zeker dat u een automatische sleutelceremonie wilt creëren?",
                    description:
                        "U staat op het punt een Sleutelceremonie aan te maken. Deze actie zal de Trustees informeren om deel te nemen aan de aanmaak en distributie van de Sleutels van het Verkiezingsevenement.",
                    automaticCeremonyDescription:
                        "Automatische ceremonies slaan handmatige bevestigingen van bewaring over. De uitvoerders van de beheerders moeten de verkiezingssleutels nog steeds genereren en verifiëren.",
                },
                filterTrustees: "Trustees filteren",
                errorPermisionLabels:
                    "Sleutelceremonie kan niet worden aangemaakt: een of meer toestemmingslabels ontbreken.",
                automaticCeremonyToggle: "Automatische ceremonie",
            },
            // Native custody instructions use the English fallback until translated.
            ceremonyStep: {
                ...englishTranslation.translations.keysGeneration.ceremonyStep,
                custodyConfirmed: "Bewaring bevestigd",
                custodyComplete: "Uw deel is voltooid. U hoeft niets meer te doen.",
                waitingForTrustees:
                    "Wachten totdat de overige beheerders hun bewaring en eventuele vereiste goedkeuring hebben afgerond.",
                moreActions: "Meer acties",
                cancel: "Sleutelceremonie Annuleren",
                progressHeader: "Voortgang Sleutelceremonie",
                executionStatus: "Status: {{status}}",
                confirmdDialog: {
                    ok: "Ja, Sleutelceremonie Annuleren",
                    cancel: "Terug naar Sleutelceremonie",
                    title: "Weet u zeker dat u de Sleutelceremonie wilt annuleren?",
                    description:
                        "U staat op het punt de Sleutelceremonie te annuleren. Na deze actie moet u een nieuwe aanmaken voor een succesvolle Sleutelceremonie.",
                },
                header: {
                    ...englishTranslation.translations.keysGeneration.ceremonyStep.header,
                    trusteeName: "Naam Trustee",
                    fragment: "Sleutelfragment Gegenereerd",
                },
                logsHeader: {
                    title: "Logs",
                    date: "Datum",
                    entry: "Logbericht",
                },
                emptyLogs: "Nog geen logs.",
            },
        },
        miruExport: {
            create: {
                success: "Transmissiepakket aanmaken...",
                error: "Fout bij aanmaken Transmissiepakket ",
            },
            send: {
                success: "Transmissiepakket verzenden...",
                error: "Fout bij verzenden Transmissiepakket ",
            },
        },
        tally: {
            errorUploadingSignature: "Er was een fout bij het uploaden van de handtekening",
            downloadTransmissionPackage: "Transmissiepakket Downloaden",
            resultsPublication: {
                sectionTitle: "Publiceren op de resultatenwebsite",
                policyTitle: "Resultatenwebsite",
                policyAccess: "Toegang tot resultatenwebsite",
                policyVisibility: "Zichtbaarheid van resultatenwebsite",
                enabled: "Ingeschakeld",
                disabled: "Uitgeschakeld",
                fullEvent: "Volledig evenement",
                areaBased: "Per gebied",
                publishStarted: "Publicatie van resultaten gestart",
                publishError: "Kon de publicatie van resultaten niet starten",
                revoked: "Publicatie van resultaten ingetrokken",
                revokeError: "Kon de publicatie van resultaten niet intrekken",
                waitingForTally:
                    "Resultaten kunnen worden gepubliceerd nadat deze telling is voltooid.",
                writePermissionRequired:
                    "U hebt de permissie publish-results-write nodig om resultaten te publiceren of in te trekken.",
                readPermissionRequired:
                    "U hebt de permissie publish-results-read nodig om de publicatiegeschiedenis te bekijken.",
                disabledPolicy:
                    "Publiceren op de resultatenwebsite is uitgeschakeld voor dit verkiezingsevenement. Schakel het in bij de gegevens van het verkiezingsevenement voordat u resultaten publiceert.",
                loadingElectionContext: "De resultatenpublicatie laadt de verkiezingscontext.",
                route: "Route",
                eventResults: "Resultaten van het evenement",
                electionResults: "Verkiezingsuitslagen",
                election: "Verkiezing",
                access: "Toegang",
                publicAccess: "Openbare toegang",
                authenticatedAccess: "Geverifieerde toegang",
                visibility: "Zichtbaarheid",
                fullPublishedScope: "Volledige gepubliceerde reikwijdte",
                personalVisibility: "Persoonlijke zichtbaarheid",
                contests: "Stemmingen",
                noTalliedContests: "Geen getelde stemmingen beschikbaar.",
                publishSelectedContests: "Geselecteerde stemmingen publiceren",
                selectedContestCount_one: "{{count}} stemming geselecteerd",
                selectedContestCount_many: "{{count}} stemmingen geselecteerd",
                selectedContestCount_other: "{{count}} stemmingen geselecteerd",
                history: "Publicatiegeschiedenis",
                version: "Versie",
                status: "Status",
                published: "Gepubliceerd",
                revokedAt: "Ingetrokken",
                actions: "Acties",
                open: "Openen",
                revoke: "Intrekken",
                noPublications: "Nog geen publicaties.",
                confirmTitle: "Publiceren op de resultatenwebsite starten?",
                confirmDescription:
                    "Hiermee wordt een nieuwe publicatie gemaakt op basis van de huidige telling. De bestaande resultaten voor kiezers blijven actief totdat deze publicatietaak is geslaagd.",
                close: "Sluiten",
            },
            transmissionPackage: {
                title: "Transmissiepakket voor Gebied '{{name}}' en Verkiezing '{{eventName}}'",
                description:
                    "Hiermee kunt u een Transmissiepakket exporteren naar Bestemmingsservers of downloaden.",
                erFileSha256: "ER-bestand SHA-256:",
                actions: {
                    send: {
                        title: "Verzenden",
                        dialog: {
                            title: "Wilt u het Transmissiepakket Verzenden?",
                            description:
                                "Bevestig dat u het Transmissiepakket voor Gebied `{{name}}` wilt verzenden naar de Bestemmingsservers.",
                            confirm: "Transmissiepakket Verzenden",
                            cancel: "Sluiten",
                        },

                        disabled:
                            "De vereiste handtekeningen ontbreken, of het transmissiepakket is al naar alle bestemmingen verzonden.",
                    },
                    regenerate: {
                        title: "Regenereren",
                        dialog: {
                            title: "Wilt u het Transmissiepakket regenereren?",
                            description:
                                "Bevestig dat u het Transmissiepakket voor Gebied `{{name}}` wilt regenereren",
                            confirm: "Transmissiepakket Regenereren",
                            cancel: "Sluiten",
                        },
                    },
                    sign: {
                        title: "Ondertekenen",
                        dialog: {
                            title: "Wilt u het Transmissiepakket ondertekenen?",
                            description:
                                "Bevestig dat u het Transmissiepakket voor Gebied `{{name}}` wilt ondertekenen",
                            confirm: "Transmissiepakket Ondertekenen",
                            cancel: "Sluiten",
                            input: {
                                placeholder: "Voer uw wachtwoord in",
                            },
                        },
                    },
                    download: {
                        title: "Downloaden",
                        emlTitle: "EML Downloaden {{date}}",
                        transmissionPackageTitle: "Transmissiepakket Downloaden {{date}}",
                        transmissionReportTitle: "Transmissierapport Downloaden",
                        csvTitle: "Resultaten-CSV downloaden {{date}}",
                        ovcsPublicKeyTitle: "Publieke sleutel van de OVCS downloaden",
                        recordError:
                            "De download kon niet worden vastgelegd en is daarom niet gestart. Probeer het opnieuw.",
                        dialog: {
                            title: "Wilt u het Transmissiepakket Downloaden?",
                            description:
                                "Bevestig dat u het Transmissiepakket voor Gebied `{{name}}` wilt downloaden.",
                            confirm: "Transmissiepakket Downloaden",
                            cancel: "Sluiten",
                        },
                    },
                },
                destinationServers: {
                    title: "Bestemmingsservers",
                    description:
                        "De onderstaande tabel toont de verzendstatus van elk van de Bestemmingsservers.",
                    status: "Verzonden naar {{signed}} van {{total}}",
                    manualReceipt: "Handmatige ontvangst",
                    handOver: "Download het pakket en overhandig het",
                    sealedFor: "Verzegeld voor de sleutel, SHA-256 {{key}}…",
                    table: {
                        serverName: "Servernaam",
                        sendStatus: "Verzendstatus",
                    },
                },
                signatures: {
                    title: "Handtekeningen",
                    description:
                        "Leden kunnen het transmissiepakket ondertekenen. De tabel toont de ondertekeningsstatus van elk lid.",
                    status: "{{signed}} van {{total}}, {{minimum}} nodig",
                    statusNoMinimum: "{{signed}} van {{total}}",
                    table: {
                        trusteeName: "Lid",
                        signed: "Heeft Ondertekend",
                    },
                },
            },
            sendToTransmissionPackageServers: "Verzend Transmissiepakket voor Gebied '{{name}}'",
            uploadTransmissionPackage: "Transmissiepakket Ondertekenen",
            uploadTransmissionPackageDesc:
                "Upload uw Handtekening om het Transmissiepakket te ondertekenen. Dit is een optionele handeling.",
            exportElectionArea: "Genereer Transmissiepakket voor Gebied '{{name}}'",
            generateReport: "Genereer {{name}}",
            templateTitle: "Resultatensjabloon",
            templateSubTitle: "Overschrijf optioneel het resultatensjabloon.",
            keysCeremonyTitle: "Sleutelceremonie",
            keysCeremonySubTitle: "Selecteer de Sleutelceremonie voor deze telling",
            ceremonyTitle: "Te Tellen Verkiezingen",
            initializationTitle: "Verkiezingen voor Initialisatierapport",
            ceremonySubTitle: "Kies de verkiezingen die u wilt tellen",
            tallyTitle: "Voortgang Telling Verkiezingen",
            logsTitle: "Logs",
            resultsTitle: "Resultaten & Deelname",
            generalInfoTitle: "Algemene Informatie",
            sessionId: "ID van de telsessie",
            selectedQuorum: "Geselecteerd quorum",
            trusteeTallyTitle: "Trustees",
            trusteeTallySubTitle: "Status van bevestigingen van bewaring",
            ballotBoxes: {
                unavailable: "Zegels niet beschikbaar",
                sealed: "{{sealed}} van {{total}} verzegeld",
                publishing: "Verzegeld, {{published}} van {{total}} zegelrecords gepubliceerd",
                sealing: "Verzegeling om {{time}}",
                notSealed: "Niet verzegeld",
                help: "Een verkiezing kan worden geteld zodra elke stembus is verzegeld en het zegelrecord ervan is gepubliceerd.",
                failed: "Niet verzegeld: incident",
                overdue: "Verzegeling achterstallig",
                blocked: "{{name}}: {{reason}}",
                reason: {
                    "not-sealed":
                        "de stemming is niet gesloten, dus de stembussen hebben nog geen zegels",
                    "sealing": "de stembussen worden verzegeld wanneer de respijttermijn eindigt",
                    "overdue":
                        "de stembussen zijn over hun deadline en nog niet verzegeld (zie het Dashboard)",
                    "publishing": "de zegelrecords van sommige stembussen worden nog gepubliceerd",
                    "failed":
                        "een stembus kon niet worden verzegeld, een incident (zie het Dashboard)",
                    "unavailable": "de zegels van de stembussen konden niet worden gelezen",
                },
            },
            eligibility: {
                ballotBoxesUnavailable:
                    "De zegels van de stembussen van een geselecteerde verkiezing konden niet worden gelezen, dus deze kan nog niet worden geteld. Laad de pagina opnieuw om het nogmaals te proberen.",
                selectElection: "Selecteer ten minste één verkiezing.",
                publishElection:
                    "Publiceer elke geselecteerde verkiezing voordat u de telling aanmaakt.",
                tallyDisallowed: "Tellen is uitgeschakeld voor een geselecteerde verkiezing.",
                endVoting:
                    "Beëindig het stemmen in elke geselecteerde verkiezing en stop de actieve stemkanalen voordat u de telling aanmaakt.",
                sealBallotBoxes:
                    "Een verkiezing kan worden geteld zodra elke stembus is verzegeld en het zegelrecord ervan is gepubliceerd.",
            },
            createTallySuccess: "Telling aangemaakt",
            createTallyError: "Kon telling niet aanmaken",
            startTallySuccess: "Telling gestart",
            startTallyError: "Kon telling niet starten",
            startTallyCeremonySuccess: "Telceremonie gestart",
            startTallyCeremonyError: "Kon telceremonie niet starten",
            cancelTallyCeremonySuccess: "Telceremonie geannuleerd",
            cancelTallyCeremonyError: "Kon telceremonie niet annuleren",
            recountTallyCeremony: "Hertelling",
            recountTallyCeremonyMessage:
                "Dit genereert een nieuw resultatenevenement voor de voltooide telsessie.",
            recountTallyCeremonyStarting: "Hertelling wordt gestart...",
            recountTallyCeremonySuccess: "Hertelling gestart",
            recountTallyCeremonyError: "Kon hertelling niet starten",
            recountTallyCeremonyOk: "Hertellen",
            trusteeTitle: "Proces Trustees",
            trusteeSubTitle:
                "Upload uw opgeslagen bewaringsbestand of een ondertekend bewijs van bewaring:",
            invited: "U bent uitgenodigd om deel te nemen aan een Telceremonie. Gelieve ",
            click: "op de Telactie te klikken",
            participate: "om deel te nemen.",
            breadcrumbSteps: {
                start: "Starten",
                finish: "Voltooien",
                tally: "Tellen",
                results: "Resultaten",
                ceremony: "Ceremonie",
            },
            common: {
                title: "Telling",
                subTitle: "Configuratie telling.",
                cancel: "Terug",
                next: "Volgende",
                date: "Datum Telling",
                global: "Globaal",
                noTrustees: "Nog geen trustees",
                imported: " beheerders hebben bewaring bevestigd",
                needed: " trustees nodig",
                start: "Start Telling",
                ceremony: "Start Telceremonie",
                initialization: "Start Initialisatierapport",
                results: "Resultaten",
                dialog: {
                    ok: "Ok",
                    okTally: "Start Telling",
                    okCancel: "Annuleer Telling",
                    cancel: "Sluiten",
                    title: "Weet u zeker dat u een ceremonie wilt starten?",
                    tallyTitle: "Weet u zeker dat u de telling wilt starten?",
                    cancelTitle: "Weet u zeker dat u de telling wilt annuleren?",
                    message:
                        "Deze telceremonie vraagt beheerders om openbare bewijzen van bewaring. Houd hun uitvoerders actief met hun lokale protocolgegevens.",
                    cancelMessage:
                        "U staat op het punt de telceremonie te annuleren. Deze actie kan niet ongedaan worden gemaakt.",
                    ceremony:
                        "De benodigde beheerders hebben bewaring bevestigd. De telling starten met hun uitvoerders actief?",
                    startAutomatedTallyMessage:
                        "Selecteer 'Start Tally' om het telproces uit te voeren en de resultaten weer te geven, of 'Close' om te annuleren.",
                },
            },
            table: {
                ballotBoxes: "Stembussen",
                elections: "Verkiezingen",
                selected: "Geselecteerd",
                status: "Status",
                progress: "Voortgang",
                method: "Telmethode",
                elegible: "Stemgerechtigde Kiezers",
                number: "Aantal Stemmen",
                total: "Totaal",
                turnout: "%",
                candidates: "Resultaten Kandidaten",
                options: "Opties",
                global: "Overzicht Deelname",
                elegible_census: "Stemgerechtigde Kiezers",
                cast_votes: "Aantal Uitgebrachte Stemmen",
                cast_votes_percent: "Percentage Uitgebrachte Stemmen",
                total_votes: "Totaal Aantal Kiezers",
                total_votes_percent: "Deelnamepercentage",
                total_votes_counted: "Totaal Getelde Stemmen",
                total_auditable_votes: "Totaal Controleerbare Stemmen",
                total_valid_votes: "Totaal Geldige Stemmen",
                total_valid_votes_percent: "Percentage Geldige Stemmen",
                total_invalid_votes: "Totaal Ongeldige Stemmen",
                total_invalid_votes_percent: "Percentage Ongeldige Stemmen",
                explicit_invalid_votes: "Expliciet Ongeldige Stemmen",
                explicit_invalid_votes_percent: "Percentage Expliciet Ongeldige Stemmen",
                implicit_invalid_votes: "Impliciet Ongeldige Stemmen",
                implicit_invalid_votes_percent: "Percentage Impliciet Ongeldige Stemmen",
                blank_votes: "Blanco Stemmen",
                explicit_blank_votes: "Expliciete Blanco Stemmen",
                implicit_blank_votes: "Impliciete Blanco Stemmen",
                blank_votes_percent: "Percentage Blanco Stemmen",
                number_of_votes: "Aantal Stemmen",
                winning_position: "Winnende positie",
                weight: "Gewicht",
                preferential: {
                    candidate: "Kandidaat",
                    winner: "Winnaar",
                    eliminated: "Geëlimineerd",
                    round: "Ronde",
                },
                total_declined_to_vote: "Totaal aantal weigeringen om te stemmen",
                total_blank_ballots: "Totaal Aantal Blanco Stembiljetten",
                participation_by_channel: "Deelname per kanaal",
                channel: "Kanaal",
                channel_online: "Online",
                channel_kiosk: "Kiosk",
                channel_early_voting: "Vervroegd stemmen",
                channel_telephone: "Telefoon",
                channel_paper: "Papier",
                channel_postal: "Post",
                channel_in_person: "Persoonlijk",
                acclamation_note:
                    "Bij acclamatie gekozen. Deze stemming is zonder stemming beslist, dus er zijn geen stemmen geregistreerd.",
            },
            pendingResolutions: {
                round: "Ronde {{round}}",
                tieResolutionRequired: "Stemgelijkheid vereist oplossing",
                tieResolved: "Stemgelijkheid opgelost",
                globalArea: "Globaal",
                pendingResolutionsHeader: "Openstaande resoluties",
                pendingResolutionStatus: "Resolutie in behandeling",
                resolvedStatus: "Opgelost",
                resolutionTitle: "Beslissing",
                selectContest: "Selecteer een item aan de linkerkant om de details te bekijken",
                selectCandidateToAdvance: "Selecteer kandidaat om door te laten gaan",
                undoResolution: "Beslissing ongedaan maken",
                applyResolutions: "Beslissingen toepassen en herberekenen",
                submitSuccess: "Beslissingen ingediend. De telling wordt hervat...",
                submitError: "Fout bij het indienen van beslissingen. Probeer het opnieuw.",
                filter: "Filteren",
                save: "Opslaan",
                pendingApplyStatus: "Berekening in afwachting",
                filterElection: "Verkiezing",
                filterContest: "Stemming",
                filterArea: "Gebied",
                filterStatusLabel: "Status",
                clearFilters: "Filters wissen",
                candidateWithVotes: "{{name}} ({{votes}} stemmen)",
                candidateWithVotesAndPercent: "{{name}} ({{votes}} stemmen, {{percent}}%)",
                tieInfoTitle: "Telling gepauzeerd wegens onopgelost gelijkspel (Ronde {{round}})",
                tieInfoBody:
                    "Kandidaten gelijk ({{votes}} stemmen, {{percent}}%): {{candidates}}. Handmatige tiebreak vereist om de telling voort te zetten.",
                tallyResumedTitle: "Telling hervat na toepassing van de resolutie",
                tallyResumedBody: "De gelijkstand werd opgelost op {{date}} door {{user}}",
            },
            chart: {
                votesForCandidates: "Stemmen voor Kandidaten",
                blankVotes: "Blanco Stemmen",
                invalidVotes: "Ongeldige Stemmen",
                totalVoters: "Totaal Kiezers",
                nonVoters: "Niet-kiezers",
            },
            exportAllAreas:
                "Exporteer de resultaten van alle gebieden in {{format}}-formaat voor '{{item}}'",
        },
        publish: {
            initialization: {
                countryInfo:
                    "Genereer het rapport voor de hele post of één land. Stemmen blijft geblokkeerd totdat alle vereiste initialisaties per land en voor het hele evenement voltooid zijn.",
                countriesError:
                    "De beschikbare landen konden niet worden geladen. Sluit en probeer opnieuw.",
                noCountries:
                    "Deze post heeft geen beschikbare landen met actieve stembiljetmodellen. Controleer de gebieden en publicatie voordat u initialiseert.",
                country: "Land",
                entirePost: "Hele post",
            },
            preview: {
                publicationAreas: "Selecteer Gebied voor Voorbeeldweergave",
                action: "Voorbeeldweergave",
                copy: "Link kopiëren",
                copy_success: "Voorbeeldlink succesvol gekopieerd",
                copy_error: "Kopiëren voorbeeldlink mislukt",
                success: "Voorbeeldweergave succesvol geopend",
            },
            header: {
                change: "Te Publiceren Wijzigingen",
                viewChange: "Publicatie Bekijken",
                history: "Publicatiegeschiedenis",
            },
            action: {
                generateInitializationReport: "Initialisatierapport Genereren",
                startVotingPeriod: "Stemperiode Starten",
                startKioskVoting: "Kioskstemmen Starten",
                startOnlineVoting: "Online Stemmen Starten",
                startEarlyVoting: "Vroeg Stemmen Starten",
                startTelephoneVoting: "Telefonisch Stemmen Starten",
                stopVotingPeriod: "Stemperiode Stoppen",
                stopOnlineVoting: "Online Stemmen Stoppen",
                stopEarlyVoting: "Vroeg Stemmen Stoppen",
                stopTelephoneVoting: "Telefonisch Stemmen Stoppen",
                stopKioskVotingPeriod: "Kiosk Stemmen Stoppen",
                pauseVotingPeriod: "Stemperiode Pauzeren",
                pauseKioskVoting: "Kioskstemmen Pauzeren",
                pauseOnlineVoting: "Online Stemmen Pauzeren",
                pauseEarlyVoting: "Vroeg Stemmen Pauzeren",
                pauseTelephoneVoting: "Telefonisch Stemmen Pauzeren",
                generate: "Regenereren",
                publish: "Wijzigingen Publiceren",
                back: "Terug",
            },
            label: {
                current: "Huidige",
                previous: "Vorige Publicatie",
                publication: "Publicatie",
                diff: "Te Publiceren Wijzigingen",
            },
            empty: {
                header: "Nog geen publicatie.",
                action: "Publicatie Genereren",
            },
            forbidden: {
                header: "Publiceren niet mogelijk totdat de Sleutelceremonie is voltooid.",
            },
            skippedElections: {
                dismiss: "Sluiten",
                title: "Sommige verkiezingen blijven gesloten",
                ballotBoxSealPolicy:
                    "{{name}} blijft gesloten: de stemming is gesloten en met Verzegelen bij sluiting is sluiten definitief.",
                other: "{{name}} is ongewijzigd gelaten ({{reason}}).",
            },
            dialog: {
                title: "Actie Bevestigen",
                info: "U hebt op een gevoelige actie geklikt, bevestig alstublieft om door te gaan",
                initializationInfo:
                    "U staat op het punt een initialisatierapport te genereren. Weet u zeker dat u wilt doorgaan?",
                startInfo:
                    "U staat op het punt de stemperiode te starten. Weet u zeker dat u wilt doorgaan?",
                stopInfo:
                    "U staat op het punt de stemperiode te stoppen. Weet u zeker dat u wilt doorgaan?",
                stopSeal:
                    "U staat op het punt de stemming in {{name}} te stoppen. De stembussen worden dan verzegeld: er kan geen stembiljet meer worden toegevoegd, gewijzigd of verwijderd, en de stemming kan niet opnieuw starten. Weet u zeker dat u wilt doorgaan?",
                stopSealEvent:
                    "U staat op het punt de stemming in elke verkiezing te stoppen. De stembussen worden dan verzegeld: er kan geen stembiljet meer worden toegevoegd, gewijzigd of verwijderd, en de stemming kan niet opnieuw starten. Weet u zeker dat u wilt doorgaan?",
                startSealNote:
                    "Met Verzegelen bij sluiting blijft een gesloten stemming gesloten: verkiezingen waarvan de stemming is gesloten, gaan niet open.",
                channel: {
                    ONLINE: "Online",
                    KIOSK: "Kiosk",
                    EARLY_VOTING: "Vroeg stemmen",
                    TELEPHONE: "Telefoon",
                },
                kioskStopInfo:
                    "U staat op het punt de kiosk stemperiode te stoppen. Weet u zeker dat u wilt doorgaan?",
                pauseInfo:
                    "U staat op het punt de stemperiode te pauzeren. Weet u zeker dat u wilt doorgaan?",
                publishInfo:
                    "U staat op het punt een publicatie te genereren. Weet u zeker dat u wilt doorgaan?",
                ok: "Bevestigen",
                ko: "Annuleren",
                error: "Fout bij laden publicatie stembiljet",
                error_publish: "Fout bij publiceren publicatie stembiljet",
                error_capacity: "Genereren van stembiljetstijl mislukt: {{message}}",
                error_status: "Fout bij wijzigen status publicatie stembiljet",
                error_preview: "Fout bij voorbeeldweergave publicatie",
                diff: "Het weergeven van alle wijzigingen kan de pagina traag maken. Weet u zeker dat u wilt doorgaan?",
                confirmation:
                    "De actie die u gaat uitvoeren is gevoelig en vereist bevestiging. Voer uw wachtwoord in om door te gaan met {{action}}.",
                stopSealNotYet:
                    "U staat op het punt de stemperiode te stoppen. {{holding}} Weet u zeker dat u wilt doorgaan?",
                sealHolding_one:
                    "Met Verzegelen bij sluiting worden de stembussen verzegeld zodra elk ingeschakeld kanaal is gesloten: {{channels}} is nog ingeschakeld en niet gesloten.",
                sealHolding_other:
                    "Met Verzegelen bij sluiting worden de stembussen verzegeld zodra elk ingeschakeld kanaal is gesloten: {{channels}} zijn nog ingeschakeld en niet gesloten.",
                sealNotEnabled:
                    "{{channel}} is niet gesloten en niet ingeschakeld voor deze verkiezing: stop het om de stembussen te verzegelen.",
                sealNotEnabledPost:
                    "In {{post}} is {{channel}} niet gesloten en niet ingeschakeld: stop het daar om de stembussen te verzegelen.",
                sealNoChannel:
                    "Er is geen kanaal ingeschakeld voor deze verkiezing en er is er geen geopend, dus de stembussen worden niet verzegeld.",
                stopNeverOpened_one:
                    "{{channels}} is nooit geopend: door het te stoppen gaat het niet meer open.",
                stopNeverOpened_other:
                    "{{channels}} zijn nooit geopend: door ze te stoppen gaan ze niet meer open.",
                stopSealGrace_one:
                    "U staat op het punt de stemming in {{name}} te stoppen. De stembussen worden verzegeld wanneer de respijttermijn eindigt, {{count}} minuut later: vanaf dan kan geen stembiljet meer worden toegevoegd, gewijzigd of verwijderd. De stemming kan niet opnieuw starten. Weet u zeker dat u wilt doorgaan?",
                stopSealGrace_other:
                    "U staat op het punt de stemming in {{name}} te stoppen. De stembussen worden verzegeld wanneer de respijttermijn eindigt, {{count}} minuten later: vanaf dan kan geen stembiljet meer worden toegevoegd, gewijzigd of verwijderd. De stemming kan niet opnieuw starten. Weet u zeker dat u wilt doorgaan?",
                stopSealEventGrace_one:
                    "U staat op het punt de stemming in elke verkiezing te stoppen. De stembussen worden verzegeld wanneer de respijttermijn van elke verkiezing eindigt, tot {{count}} minuut later: vanaf dan kan geen stembiljet meer worden toegevoegd, gewijzigd of verwijderd. De stemming kan niet opnieuw starten. Weet u zeker dat u wilt doorgaan?",
                stopSealEventGrace_other:
                    "U staat op het punt de stemming in elke verkiezing te stoppen. De stembussen worden verzegeld wanneer de respijttermijn van elke verkiezing eindigt, tot {{count}} minuten later: vanaf dan kan geen stembiljet meer worden toegevoegd, gewijzigd of verwijderd. De stemming kan niet opnieuw starten. Weet u zeker dat u wilt doorgaan?",
                stopSealEventSome:
                    "U staat op het punt de stemming in elke verkiezing te stoppen. {{sealed}} {{holding}} Weet u zeker dat u wilt doorgaan?",
                sealedNowPart:
                    "De stembussen van {{names}} worden dan verzegeld: er kan geen stembiljet meer worden toegevoegd, gewijzigd of verwijderd, en de stemming kan daar niet opnieuw starten.",
                sealedGracePart_one:
                    "De stembussen van {{names}} worden verzegeld wanneer hun respijttermijn eindigt, tot {{count}} minuut later.",
                sealedGracePart_other:
                    "De stembussen van {{names}} worden verzegeld wanneer hun respijttermijn eindigt, tot {{count}} minuten later.",
                holdingEventPart_one:
                    "{{names}} houdt een ander kanaal ingeschakeld en niet gesloten: de stembussen worden verzegeld zodra dat kanaal is gesloten.",
                holdingEventPart_other:
                    "{{names}} houden een ander kanaal ingeschakeld en niet gesloten: hun stembussen worden verzegeld zodra die kanalen zijn gesloten.",
                noChannelEventPart_one:
                    "{{names}} schakelt geen kanaal in en er is er daar geen geopend: de stembussen worden niet verzegeld.",
                noChannelEventPart_other:
                    "{{names}} schakelen geen kanaal in en er is er daar geen geopend: hun stembussen worden niet verzegeld.",
                startSealNoteList:
                    "Met Verzegelen bij sluiting blijft een gesloten stemming gesloten: {{items}}.",
                startKeptChannels_one: "{{post}}: {{channels}} blijft gesloten",
                startKeptChannels_other: "{{post}}: {{channels}} blijven gesloten",
                startKeptSealed: "{{post}} blijft gesloten, omdat de stembussen zijn verzegeld",
                startNotEnabledList:
                    "Met Verzegelen bij sluiting opent een start een kanaal alleen bij de Posts die het inschakelen: {{items}}.",
                startNotEnabledChannels_one:
                    "{{post}}: {{channels}} is daar niet ingeschakeld en blijft Niet gestart",
                startNotEnabledChannels_other:
                    "{{post}}: {{channels}} zijn daar niet ingeschakeld en blijven Niet gestart",
                stopNeverOpenedPosts_one:
                    "{{names}} is nooit geopend: stoppen sluit de Post en verzegelt de lege stembussen.",
                stopNeverOpenedPosts_other:
                    "{{names}} zijn nooit geopend: stoppen sluit ze en verzegelt hun lege stembussen.",
            },
            notifications: {
                generated: "Stembiljet gegenereerd",
                published: "Stembiljet gepubliceerd",
                change_status: "Verkiezingsstatus gewijzigd",
            },
            sealRefusals: {
                startAgain:
                    "De stemming kan niet opnieuw starten: met Verzegelen bij sluiting blijft een gesloten stemming gesloten en zijn de stembussen verzegeld.",
                startDisabled:
                    "Stemperiode Starten is niet beschikbaar: de stembussen van deze verkiezing zijn verzegeld of worden verzegeld, en met Verzegelen bij sluiting blijft een gesloten stemming gesloten.",
                closedIsFinal:
                    "Een gesloten stemming kan niet veranderen: met Verzegelen bij sluiting blijft een gesloten stemming gesloten.",
            },
        },
        emailEditor: {
            subject: "Onderwerp e-mail",
            tabs: {
                plaintext: "Platte Tekst Body",
                richtext: "Opgemaakte tekst",
            },
        },
        sendCommunication: {
            send: "Verzenden",
            title: "Melding Verzenden",
            subtitle: "Een melding verzenden naar kiezers.",
            sendButton: "Melding Verzenden",
            voters: "Doelgroep",
            schedule: "Planning",
            nowInput: "Nu verzenden",
            dateInput: "Datum en tijd om meldingen te verzenden",
            chooseDate: "Kies een datum",
            languages: "Talen",
            smsMessage: "SMS-bericht",
            errorSending: "Fout bij verzenden melding: {{error}}",
            successSending: "Melding succesvol ingepland",
            method: "Methode Sjabloon",
            type: "Communicatietype",
            alias: "Alias Sjabloon",
            votersSelection: {
                ALL_USERS: "Iedereen",
                NOT_VOTED: "Degenen die nog niet gestemd hebben",
                VOTED: "Degenen die al gestemd hebben",
                SELECTED: "Naar {{total}} Geselecteerde {{voters}}",
            },
            path: {
                users: "gebruikers",
                voters: "kiezers",
            },
            methodTitle: "Communicatiesjabloon",
            communicationMethod: {
                EMAIL: "E-mail",
                SMS: "SMS",
                WHATSAPP: "WhatsApp",
                VIBER: "Viber",
                MESSENGER: "Facebook Messenger",
            },
            communicationType: {
                CREDENTIALS: "Inloggegevens",
                BALLOT_RECEIPT: "Ontvangstbewijs Stembiljet",
            },
            email: {
                subject: "Onderwerp",
            },
        },
        tallysheet: {
            title: "Stembussen",
            subtitle: "Digitaal gestemde stembussen per kanaal",
            createTallySuccess: "Telblad opgeslagen",
            createTallyError: "Fout bij opslaan telblad",
            createTallyErrorSameKindExists:
                "Telformulier bestaat al voor deze stemming met hetzelfde kanaal en gebied",
            allFieldsRequired: "Alle velden zijn verplicht",
            header: {
                change: "Te Publiceren Wijzigingen",
                viewChange: "Publicatie Bekijken",
                history: "Publicatiegeschiedenis",
            },
            action: {
                start: "Start Verkiezing",
                stop: "Stop Verkiezing",
                pause: "Pauzeren",
                generate: "Regenereren",
                publish: "Wijzigingen Publiceren",
                back: "Terug",
            },
            inputError: {
                totalValidDoesNotMatch:
                    "Kandidaatstemmen ({{candidateVotesSum}}) moeten tussen {{lowerBound}} en {{upperBound}} liggen volgens de stemregels van deze stemming ({{nonBlankValidVotes}} geldige niet-blanco stemmen × maximaal {{maxMarks}} markeringen per stembiljet)",
                censusTooSmall:
                    "Het totaal aantal stemmen ({{totalVotes}}) mag niet groter zijn dan de census ({{census}})",
                totalInvalidDoesNotMatch:
                    "Het totaal aantal ongeldige stemmen ({{totalInvalid}}) moet gelijk zijn aan impliciet ongeldige stemmen ({{implicitInvalid}}) plus expliciet ongeldige stemmen ({{explicitInvalid}})",
                totalVotesDoesNotMatch:
                    "Het totaal aantal stemmen ({{totalVotes}}) moet gelijk zijn aan het totaal aantal geldige stemmen ({{totalValidVotes}}) plus het totaal aantal ongeldige stemmen ({{totalInvalid}})",
                unknownCountingAlgorithm:
                    "Het telalgoritme van deze stemming ({{countingAlgorithm}}) wordt niet herkend, waardoor het toegestane aantal kandidaatstemmen niet kan worden bepaald. Controleer de configuratie van de stemming.",
                blankBallotsInconsistent:
                    "Blanco Stembiljetten moeten dezelfde waarde hebben op elk telformulier van deze stembus",
                blankBallotsOutOfBounds:
                    "De waarde van Blanco Stembiljetten valt buiten het bereik dat wordt geïmpliceerd door de blanco-stemtellingen per stemming van deze stembus",
            },
            label: {
                area: "Gebied",
                channel: "Kanaal",
                total_votes: "Totaal Stemmen",
                total_valid_votes: "Totaal Geldige Stemmen",
                total_invalid: "Totaal Ongeldige Stemmen",
                explicit_invalid: "Expliciet Ongeldige Stemmen",
                implicit_invalid: "Impliciet Ongeldige Stemmen",
                total_blank_votes: "Blanco Stemmen",
                blank_ballots: "Blanco Stembiljetten",
                census: "Kieserslijst",
            },
            common: {
                tallyCeremony: {
                    manage: "Telceremonie Beheren",
                    view: "Telceremonie Bekijken",
                    cancel: "Telceremonie Annuleren",
                    addKey: "Telsleutel Toevoegen",
                },
                edit: "Bewerken",
                confirm: "Bevestigen",
                back: "Terug",
                next: "Volgende",
                cancel: "Terug",
                data: "Gegevens",
                title: "Telblad",
                subtitle: "Configuratie telblad.",
                candidates: "Kandidaten",
                save: "Opslaan",
                approve: "Goedkeuren",
                disapprove: "Afkeuren",
                show: "Tonen",
                add: "Toevoegen",
                versions: "Versies",
                warningDisapprove: "Weet u zeker dat u dit Telformulier wilt afkeuren?",
                warningApprove: "Weet u zeker dat u dit Telformulier wilt goedkeuren?",
            },
            empty: {
                header: "Nog geen telblad.",
                action: "Telblad Genereren",
                add: "Toevoegen",
            },
            breadcrumbSteps: {
                start: "Starten",
                edit: "Bewerken",
                confirm: "Bevestigen",
                view: "Bekijken",
            },
            table: {
                area: "Gebied",
                contest: "Stemming",
                approvedVersion: "Goedgekeurde versie",
                latestVersion: "Nieuwste versie",
                labels: "Labels",
                annotations: "Annotaties",
            },
            versionsTable: {
                title: "Versies van de stembus",
                version: "Versie",
                createdBy: "Gemaakt door",
                reviewedBy: "Beoordeeld door",
                createdAt: "Gemaakt op",
                reviewedAt: "Beoordeeld op",
                sourceImport: "Bronimport",
                importStatus: "Importstatus",
                openImport: "Import openen",
                sourceFile: "Bronbestand",
            },
            message: {
                reviewError: "Fout bij beoordelen telblad",
                reviewSuccess: "Telblad beoordeeld",
            },
        },
        application: {
            import: {
                title: "Applicaties Importeren",
                subtitle: "Gegevens applicaties importeren",
                paragraph:
                    "Importeer applicaties met een spreadsheetbestand in CSV-formaat (Comma Separated Values). Download hier een voorbeeld CSV-importbestand.",
                messages: {
                    success: "Applicaties succesvol geïmporteerd",
                    error: "Fout bij importeren applicaties",
                },
            },
            export: {
                title: "Applicaties Exporteren",
                subtitle: "Gegevens applicaties exporteren",
                button: "Exporteren",
                paragraph:
                    "Exporteer applicatiegegevens naar een spreadsheetbestand in CSV-formaat (Comma Separated Values).",
                messages: {
                    success: "Applicaties succesvol geëxporteerd",
                    error: "Fout bij exporteren applicaties",
                },
            },
        },
        template: {
            noPermissions: "U hebt geen toestemming om toegang te krijgen tot sjablonen.",
            title: "Sjablonen",
            subtitle: "Lijst van sjablonen",
            chooseMethods: "Kies Methoden",
            default: "Standaardsjabloon gebruiken",
            empty: {
                title: "Nog geen sjabloon",
                subtitle: "Wilt u er een aanmaken?",
            },
            action: {
                createOne: "Sjabloon Aanmaken",
            },
            create: {
                title: "Een Sjabloon Aanmaken",
                success: "Sjabloon aangemaakt",
                error: "Fout bij aanmaken sjabloon",
            },
            update: {
                success: "Sjabloon bijgewerkt",
                error: "Fout bij bijwerken sjabloon",
            },
            edit: {
                title: "Een Sjabloon Bewerken",
            },
            form: {
                smsMessage: "SMS-bericht",
                document: "Document",
                pdfOptions: "PDF Opties",
                reportOptions: "Rapport Opties",
                name: "Naam Sjabloon",
                alias: "Alias Sjabloon",
                type: "Type",
                communicationMethod: "Methode",
            },
            type: {
                CREDENTIALS: "Inloggegevens",
                INITIALIZATION_REPORT: "Initialisatierapport",
                ELECTORAL_RESULTS: "Verkiezingsresultaten",
                BALLOT_IMAGES: "Afbeeldingen Stembiljetten",
                BALLOT_RECEIPT: "Ontvangstbewijs Stembiljet",
                ACTIVITY_LOGS: "Activiteitenlogs",
                MANUAL_VERIFICATION: "Handmatige Verificatie",
                PARTICIPATION_REPORT: "Participatierapport",
            },
            method: {
                email: "E-mail",
                sms: "SMS",
                document: "Document",
                whatsapp: "WhatsApp",
                viber: "Viber",
                messenger: "Facebook Messenger",
            },
            import: {
                title: "Sjablonen Importeren",
                subtitle: "Gegevens sjablonen importeren",
                paragraph:
                    "Importeer sjablonen met een spreadsheetbestand in CSV-formaat (Comma Separated Values). Download hier een voorbeeld CSV-importbestand.",
            },
        },
        materials: {
            audioInstructions: {
                screenLabel: "Gesproken instructies voor scherm",
                languageLabel: "Taal van de opname",
                none: "Geen gesproken instructies",
                helperText:
                    "Kiezers horen dit bestand wanneer ze op dat scherm om de instructies vragen.",
                screens: {
                    "election-chooser": "Lijst met verkiezingen",
                    "start": "Start",
                    "ballot": "Stembiljet",
                    "review": "Controle",
                    "confirmation": "Bevestiging",
                    "audit": "Audit",
                    "ballot-locator": "Stembiljetzoeker",
                    "support-materials": "Ondersteunend materiaal",
                },
            },
            createMaterialSuccess: "Ondersteunend materiaal aangemaakt",
            createMaterialError: "Fout bij aanmaken ondersteunend materiaal",
            updateMaterialSuccess: "Ondersteunend materiaal bijgewerkt",
            updateMaterialError: "Fout bij bijwerken ondersteunend materiaal",
            common: {
                title: "Ondersteunend Materiaal",
                subtitle: "Voer gegevens voor ondersteunend materiaal in.",
            },
            error: {
                title: "Titel is verplicht",
                document: "Document is verplicht",
            },
            fields: {
                isHidden: "Is Verborgen",
                publicUrl: "Publieke URL",
            },
            empty: {
                header: "Nog geen ondersteunend materiaal",
                action: "Ondersteunend materiaal genereren",
            },
        },
        widget: {
            logs: "Logs",
        },
        settings: {
            countries: {
                title: "Landenblokkering",
                votingDescription: "Kies hieronder de landen waaruit u stemmen wilt blokkeren.",
                enrollmentDescription:
                    "Kies hieronder de landen waaruit u inschrijvingen wilt blokkeren.",
                error: {
                    errorSaving: "Fout bij opslaan landenlijst",
                },
            },
            backupRestore: {
                title: "Back-up / Herstellen Tenantconfiguratie",
                backup: {
                    label: "Back-up",
                    subtitle: "Back-up maken van Tenantconfiguraties",
                },
                restore: {
                    label: "Herstellen",
                    subtitle: "Tenantconfiguratie herstellen",
                    title: "Tenantconfiguraties Importeren",
                    paragraph:
                        "Importeer tenantconfiguraties, Keycloak-configuraties, rollen & machtigingengegevens met een zip-map",
                    tenantConfigOption: "Tenantconfiguraties Importeren",
                    keycloakConfigOption: "Keycloak-configuraties Importeren",
                    RolesConfigOption: "Configuraties Rollen & Machtigingen Importeren",
                },
            },
            previewScreen: {
                label: "Voorbeeldweergaven",
                noContent: "Geen voorbeeldweergaven gevonden",
                table: {
                    title: "Externe voorvertoningen",
                    description:
                        "Een overzicht van voorvertoningen van stembiljetstijlen die zijn gegenereerd via externe verzoeken",
                    requestedBy: "Aangevraagd door",
                    document: "Document",
                    url: "URL",
                },
            },
            languages: {
                logsTimezone: "Tijdzone voor het tenantlog",
                default: "Standaardtaal",
            },
        },
        approvalsScreen: {
            column: {
                status: "Status",
                id: "Aanvraag-ID",
                applicantId: "ID aanvrager",
                verificationType: "Verificatie",
                createdAt: "Aangevraagd",
                verified_by: "Geverifieerd door",
                voter: "Kiezer",
                what: "Wat er is gebeurd",
                post: "Post",
                when: "Wanneer",
            },
            status: {
                PENDING: "Te beoordelen",
                ACCEPTED: "Goedgekeurd",
                REJECTED: "Afgewezen",
            },
            verification: {
                AUTOMATIC: "Automatisch",
                MANUAL: "Handmatig",
            },
            time: {
                minutes_one: "{{count}} minuut",
                minutes_other: "{{count}} minuten",
                hours_one: "{{count}} uur",
                hours_other: "{{count}} uur",
                days_one: "{{count}} dag",
                days_other: "{{count}} dagen",
            },
            summary: {
                join: "{{head}} en {{last}}",
                differs_one: "{{fields}} wijkt af van het register",
                differs_other: "{{fields}} wijken af van het register",
                typedByHand:
                    "Gegevens met de hand getypt, niet gelezen van een gescand identiteitsbewijs",
                needsFaceToFace: "Controle in een persoonlijk gesprek nodig",
                scanVerified: "Gescand identiteitsbewijs geverifieerd",
                noVoter: "Geen kiezer gevonden in het register",
                allMatch: "Alle gegevens komen overeen met het register",
                needsReview: "Wacht op de beslissing van een persoon",
                approvedBy: "Goedgekeurd door {{name}}",
                approvedAuto: "Automatisch goedgekeurd",
                rejectedBy: "Afgewezen door {{name}}",
                rejectedAuto: "Automatisch afgewezen",
            },
            list: {
                title: "Goedkeuringen",
                subtitle:
                    "Inschrijvingen waarover de regels niet zelf kunnen beslissen, wachten hier op een persoon.",
                search: "Zoeken",
                review: "Inschrijving beoordelen",
                openRecord: "Inschrijving openen",
                seeRule: "De regel bekijken die besliste",
                unnamed: "Aanvrager zonder naam",
                waiting: "Wacht al {{time}}",
                applied: "Aangevraagd op {{date}}",
                empty: {
                    title: "Hier staat niets",
                    text: "Inschrijvingen met deze status verschijnen hier. Probeer een andere zoekopdracht of status.",
                },
                searchHelp:
                    "Een naam, e-mailadres of identiteitsnummer, of de ondersteuningsreferentie die de kiezer kreeg, zoals Q5KW-8RFX.",
                referenceError: "De ondersteuningsreferentie kon niet worden opgezocht.",
            },
            flow: {
                stepsLabel: "Stappen van de beoordeling",
                steps: {
                    identity: "De identiteit controleren",
                    voter: "De kiezer zoeken",
                    decide: "Beslissen",
                },
                continue: "Doorgaan",
                backToList: "Terug naar Goedkeuringen",
                identity: {
                    details: "Gegevens op de inschrijving",
                    confirm:
                        "Ik heb het identiteitsbewijs van de kiezer persoonlijk of via een videogesprek gecontroleerd, en het komt overeen met deze inschrijving.",
                    checked: "Controle in een persoonlijk gesprek bevestigd",
                    notChecked: "Controle in een persoonlijk gesprek nog niet bevestigd",
                },
                voter: {
                    none: "Geen van deze is de kiezer",
                    noneHint:
                        "De inschrijving kan dan alleen worden afgewezen, omdat er geen overeenkomende kiezer is.",
                    noneChosen: "Geen van deze is de kiezer",
                    notChosen: "Nog geen kiezer gekozen",
                },
                decide: {
                    approve: "Goedkeuren",
                    reject: "Afwijzen",
                    approveText:
                        "Koppel deze inschrijving aan {{voter}} in het register. De kiezer krijgt bericht per e-mail of sms en kan inloggen om te stemmen zodra de stemming opent.",
                    rejectText:
                        "De kiezer krijgt te horen waarom. Dit kan niet ongedaan worden gemaakt.",
                    chooseVoter: "Kies in stap 2 de overeenkomende kiezer om goed te keuren.",
                    noVoter:
                        "U hebt geen overeenkomende kiezer gevonden, dus deze inschrijving kan alleen worden afgewezen.",
                    enrolled: "De gekozen kiezer is al ingeschreven.",
                    faceToFace:
                        "Bevestig in stap 1 de controle in een persoonlijk gesprek om goed te keuren.",
                },
            },
            review: {
                loadError: "De inschrijving kon niet worden geladen.",
                applied: "Aangevraagd op {{date}}",
                waiting: "Wacht al {{time}}",
                whyTitle: "Waarom hier een persoon voor nodig is",
                decisionTitle: "Hoe hierover is beslist",
                rule: "Regel {{rule}} van matrixversie {{version}}",
                ruleLast: "Laatste regel van matrixversie {{version}}",
                seeRule: "De regel bekijken",
                why: {
                    typedByHand:
                        "De kiezer heeft zijn gegevens met de hand getypt in plaats van een identiteitsbewijs te scannen. Zulke inschrijvingen worden nooit automatisch goedgekeurd: een ambtenaar bevestigt eerst wie de kiezer is.",
                    differs_one:
                        "Eén gegeven komt niet overeen met het register: {{details}}. De goedkeuringsregels vragen een persoon om deze inschrijving te controleren.",
                    differs_other:
                        "{{count}} gegevens komen niet overeen met het register: {{details}}. De goedkeuringsregels vragen een persoon om deze inschrijving te controleren.",
                    differsFields_one:
                        "Eén gegeven komt niet overeen met het register: {{fields}}. De goedkeuringsregels vragen een persoon om deze inschrijving te controleren.",
                    differsFields_other:
                        "{{count}} gegevens komen niet overeen met het register: {{fields}}. De goedkeuringsregels vragen een persoon om deze inschrijving te controleren.",
                    difference:
                        "bij {{field}} staat “{{enrollment}}” op de inschrijving en “{{registry}}” in het register",
                    noVoter:
                        "Geen kiezer in het register heeft deze gegevens. De goedkeuringsregels vragen een persoon om deze inschrijving te controleren.",
                    severalVoters:
                        "Meer dan één kiezer in het register past bij deze inschrijving. Een persoon kiest de juiste.",
                    pending:
                        "De goedkeuringsregels vragen een persoon om deze inschrijving te controleren.",
                    unknown: "Deze inschrijving wacht op de beslissing van een persoon.",
                    approvedAuto:
                        "De goedkeuringsregels hebben deze inschrijving automatisch goedgekeurd. Alle controles die ze vereisen zijn geslaagd.",
                    approvedBy: "{{name}} heeft deze inschrijving goedgekeurd op {{date}}.",
                    rejectedAuto:
                        "De goedkeuringsregels hebben deze inschrijving automatisch afgewezen: {{reason}}.",
                    rejectedBy:
                        "{{name}} heeft deze inschrijving afgewezen op {{date}}: {{reason}}.",
                },
                registryHelp:
                    "We hebben gezocht naar kiezers met dezelfde gegevens: {{fields}}. Kies de kiezer bij wie deze inschrijving hoort.",
                registrySearching:
                    "Dit zijn de kiezers in het register die bij uw zoekopdracht passen. Kies de kiezer bij wie deze inschrijving hoort.",
                registrySearch: "Niet in de lijst? Zoek in het register op naam of e-mailadres",
                registryLoading: "Bezig met zoeken in het register",
                registryError: "Er kon niet in het register worden gezocht.",
                noCandidates:
                    "Geen kiezer in het register komt overeen. Probeer te zoeken op naam of e-mailadres.",
                candidates: "Kiezers in het register",
                alreadyEnrolled: "Al ingeschreven",
                bestMatch: "Beste overeenkomst",
                detailsMatch: "{{count}} van {{total}} gegevens komen overeen",
                compareTitle: "Vergeleken met {{name}} in het register",
                col: {
                    detail: "Gegeven",
                    enrollment: "Op de inschrijving",
                    registry: "In het register",
                    result: "Resultaat",
                },
                same: "Gelijk",
                differs: "Anders",
                compareNote: "Bij namen tellen hoofdletters, accenten en koppeltekens niet mee.",
                compareJoint:
                    "Bij rijbewijzen en zeemansboekjes worden voornaam en tweede naam samen vergeleken.",
                applicationId: "Aanvraag-ID",
                copy: "Kopiëren",
                copied: "Gekopieerd",
                approve: "Inschrijving goedkeuren",
                approveDialog: {
                    title: "{{name}} goedkeuren?",
                    body: "Hiermee wordt de inschrijving gekoppeld aan de kiezer uit het register hieronder. De kiezer krijgt bericht per e-mail of sms en kan inloggen om te stemmen zodra de stemming opent.",
                    checked:
                        "U hebt het identiteitsbewijs van de kiezer in een persoonlijk gesprek gecontroleerd.",
                    irreversible: "Dit kan niet ongedaan worden gemaakt.",
                    confirm: "Goedkeuren",
                },
                reject: "Inschrijving afwijzen",
            },
            idCheck: {
                title: "Controle van het identiteitsbewijs",
                method: {
                    VERIFIED: "Gescand identiteitsbewijs geverifieerd",
                    MANUAL_ENTRY: "Met de hand getypt",
                    UNKNOWN: "Niet gemeld",
                },
                verified:
                    "Het inschrijfproces heeft het identiteitsbewijs van de kiezer geverifieerd",
                typedByHand: "De kiezer heeft zijn gegevens met de hand getypt",
                unknown: "Het inschrijfproces heeft niet gemeld hoe de identiteit is gecontroleerd",
                faceToFaceTitle:
                    "Controleer de kiezer in een persoonlijk gesprek voordat u goedkeurt",
                faceToFaceText:
                    "Spreek de kiezer persoonlijk of via een videogesprek en vergelijk het identiteitsbewijs met de gegevens op deze pagina.",
            },
            reject: {
                rejectReason: "Reden van afwijzing",
                message: "Bericht aan de kiezer",
                messageRequired: "Schrijf een bericht voor de kiezer als de reden Anders is.",
                reasons: {
                    "undefined": "-",
                    "insufficient-information": "Ontbrekende gegevens",
                    "no-matching-voter": "Geen overeenkomende kiezer",
                    "voter-already-approved": "Al goedgekeurd",
                    "other": "Anders",
                },
                hint: {
                    "insufficient-information": "Er ontbreken gegevens of ze zijn onleesbaar.",
                    "no-matching-voter": "De persoon staat niet in het kiezersregister.",
                    "voter-already-approved": "Deze kiezer is al ingeschreven.",
                    "other": "Schrijf uw eigen bericht.",
                },
                preview: {
                    "insufficient-information":
                        "We konden u niet inschrijven omdat een deel van uw gegevens ontbreekt of onleesbaar is. Schrijf u opnieuw in met volledige gegevens.",
                    "no-matching-voter":
                        "We konden in het register geen kiezer vinden die bij uw gegevens past. Controleer uw gegevens en schrijf u opnieuw in, of neem contact op met uw verkiezingskantoor.",
                    "voter-already-approved":
                        "U bent al ingeschreven. U kunt inloggen om te stemmen zodra de stemming opent.",
                },
                previewTitle: "De kiezer ziet",
            },
            notifications: {
                approveError: "De inschrijving kon niet worden goedgekeurd",
                approveSuccess: "{{name}} goedgekeurd. De kiezer heeft bericht gekregen.",
                rejectError: "De inschrijving kon niet worden afgewezen",
                rejectSuccess: "{{name}} afgewezen. De kiezer heeft bericht gekregen.",
                VoterApprovedAlready: "Deze kiezer is al ingeschreven.",
            },
            export: {
                success: "Export van aanvragen succesvol voltooid",
                error: "Fout bij het exporteren van aanvragen",
            },
            matrix: {
                button: "Goedkeuringsmatrix",
                title: "Goedkeuringsmatrix",
                back: "Goedkeuringen",
                subtitle:
                    "Regels bepalen wat er met elke inschrijving gebeurt. De eerste regel die van toepassing is, beslist.",
                versionChip: "Versie {{version}}",
                savedBy: "Opgeslagen op {{date}} door {{user}}",
                builtIn: "Ingebouwde regels, in gebruik totdat een versie is opgeslagen",
                unsaved: "Niet-opgeslagen wijzigingen",
                viewOnly: "Alleen bekijken",
                readOnlyTitle: "U kunt de regels bekijken, maar niet wijzigen",
                readOnlyText:
                    "Vraag een beheerder met de machtiging approval-matrix-write om wijzigingen aan te brengen.",
                loadError: "De goedkeuringsmatrix kon niet worden geladen.",
                compared: "Wat we vergelijken",
                comparedHelp:
                    "Elke inschrijving wordt vergeleken met de kiezer die in het register is gevonden. Bij namen tellen hoofdletters, accenten en koppeltekens niet mee; bij rijbewijzen en zeemansboekjes worden voornaam en tweede naam samen vergeleken.",
                addCompared: "Nog een gegeven vergelijken",
                rules: "Regels",
                rulesHelp:
                    "Regels worden van boven naar beneden gecontroleerd. De eerste die van toepassing is, beslist; als er geen van toepassing is, geldt de laatste regel.",
                when: "Wanneer",
                then: "Dan",
                otherwise: "Anders",
                noneApply: "Geen van de bovenstaande regels is van toepassing",
                andWord: "en",
                and: " en ",
                appliesToExample: "Van toepassing op uw voorbeeld",
                cameFrom: "Besliste over de inschrijving waar u vandaan komt",
                voterIsTold: "De kiezer krijgt te horen: “{{reason}}”.",
                sentence: "Als geldt: {{when}}, dan {{outcome}}.",
                sentenceOtherwise:
                    "Als geen van de bovenstaande regels van toepassing is, {{outcome}}.",
                sentenceEmpty:
                    "Voeg een voorwaarde toe om aan te geven wanneer deze regel van toepassing is.",
                addRule: "Regel toevoegen",
                discard: "Wijzigingen verwerpen",
                actions: {
                    edit: "Regel {{number}} bewerken",
                    editOtherwise: "De laatste regel bewerken",
                    moveUp: "Regel {{number}} omhoog verplaatsen",
                    moveDown: "Regel {{number}} omlaag verplaatsen",
                    delete: "Regel {{number}} verwijderen",
                },
                saveBar: {
                    title: "U hebt niet-opgeslagen wijzigingen",
                    fix_one: "Corrigeer 1 regel voordat u opslaat",
                    fix_other: "Corrigeer {{count}} regels voordat u opslaat",
                    more: "+{{count}} meer",
                },
                test: "Een voorbeeld proberen",
                testHelp:
                    "Beschrijf een inschrijving om te zien welke regel erover beslist. Uw niet-opgeslagen wijzigingen tellen mee.",
                testDetails: "Vergeleken gegevens",
                applies: "Regel {{number}} is van toepassing",
                otherwiseApplies: "De laatste regel is van toepassing",
                testError: "Het voorbeeld kon niet worden geprobeerd.",
                testInvalid: "Corrigeer deze regels om een voorbeeld te proberen:",
                ruleError: "Regel {{number}}: {{error}}",
                invariants: {
                    MANUAL_ENTRY_NOT_ACCEPTED:
                        "Een getypte identiteit wordt nooit automatisch goedgekeurd, dus dit gaat naar een persoon.",
                    ALREADY_ENROLLED_NOT_ACCEPTED:
                        "Een kiezer die al is ingeschreven wordt nooit opnieuw goedgekeurd.",
                    NO_VOTER_NOT_ACCEPTED:
                        "Niemand wordt goedgekeurd zonder kiezer in het register.",
                    OTHERWISE_NOT_ACCEPTED: "De laatste regel keurt nooit goed.",
                },
                dialog: {
                    editTitle: "Regel {{number}} bewerken",
                    newTitle: "Nieuwe regel",
                    otherwiseTitle: "De laatste regel bewerken",
                    summary: "In het kort",
                    whenHelp:
                        "Deze moeten allemaal waar zijn. Laat een voorwaarde weg als die er niet toe doet.",
                    otherwiseHelp: "Als geen van de bovenstaande regels van toepassing is",
                    addCondition: "Voorwaarde toevoegen",
                    remove: "“{{condition}}” verwijderen",
                    identity: "Identiteitscontrole",
                    voterFound: "Kiezer in het register",
                    alreadyEnrolled: "Al ingeschreven",
                    validId: "Type identiteitsbewijs",
                    differing: "Gegevens die afwijken",
                    decision: "Beslissing",
                    reason: "Wat de kiezer te horen krijgt",
                    voterSees: "De kiezer ziet",
                    apply: "Toepassen",
                    close: "Sluiten",
                    yes: "Ja",
                    no: "Nee",
                    notReported: "Niet gemeld",
                },
                identity: {
                    VERIFIED: "Geverifieerd met gescand identiteitsbewijs",
                    MANUAL_ENTRY: "Met de hand getypt",
                },
                differing: {
                    none: "Geen",
                    exactly_1: "Precies 1",
                    at_most_1: "Hoogstens 1",
                    exactly_2: "Precies 2",
                    at_most_2: "Hoogstens 2",
                    at_least_3: "3 of meer",
                },
                fieldMatch: {
                    MATCHES: "Gelijk",
                    DIFFERS: "Anders",
                },
                decisions: {
                    ACCEPTED: "Automatisch goedkeuren",
                    PENDING: "Naar een persoon sturen",
                    REJECTED: "Afwijzen",
                },
                outcomeShort: {
                    ACCEPTED: "automatisch goedkeuren",
                    PENDING: "naar een persoon sturen",
                    REJECTED: "afwijzen",
                },
                outcomeHelp: {
                    ACCEPTED: "De kiezer wordt ingeschreven zonder dat iemand ernaar kijkt.",
                    PENDING:
                        "Een ambtenaar beslist, en de kiezer krijgt te horen dat de inschrijving wordt beoordeeld.",
                    REJECTED: "De kiezer krijgt te horen waarom, en kan zich opnieuw inschrijven.",
                },
                outcomeSentence: {
                    ACCEPTED: "wordt de inschrijving automatisch goedgekeurd",
                    PENDING: "wordt de inschrijving naar een persoon gestuurd",
                    REJECTED: "wordt de inschrijving afgewezen",
                },
                reasons: {
                    NO_VOTER: "Geen overeenkomende kiezer",
                    ALREADY_APPROVED: "Al goedgekeurd",
                    INSUFFICIENT_INFORMATION: "Ontbrekende gegevens",
                    IDENTITY_NOT_VERIFIED: "Identiteit niet geverifieerd",
                    OTHER: "Anders",
                },
                voterText: {
                    NO_VOTER:
                        "We konden in het register geen kiezer vinden die bij uw gegevens past. Controleer uw gegevens en schrijf u opnieuw in, of neem contact op met uw verkiezingskantoor.",
                    ALREADY_APPROVED:
                        "U bent al ingeschreven. U kunt inloggen om te stemmen zodra de stemming opent.",
                    INSUFFICIENT_INFORMATION:
                        "We konden u niet inschrijven omdat een deel van uw gegevens ontbreekt of onleesbaar is. Schrijf u opnieuw in met volledige gegevens.",
                    IDENTITY_NOT_VERIFIED:
                        "We konden uw identiteit niet automatisch verifiëren, dus een verkiezingsambtenaar zal uw inschrijving beoordelen.",
                    OTHER: "Een verkiezingsambtenaar schrijft dit bericht bij het nemen van de beslissing.",
                },
                conditions: {
                    any: "Nog geen voorwaarden",
                    identity: {
                        VERIFIED: "Identiteit geverifieerd met gescand identiteitsbewijs",
                        MANUAL_ENTRY: "Identiteit met de hand getypt",
                    },
                    voterFound: {
                        true: "Kiezer gevonden in het register",
                        false: "Geen kiezer gevonden in het register",
                    },
                    alreadyEnrolled: {
                        true: "Al ingeschreven",
                        false: "Nog niet ingeschreven",
                    },
                    validId: "Identiteitsbewijs: {{id}}",
                    differing: {
                        none: "Alle gegevens komen overeen",
                        exactly_1: "Precies 1 gegeven wijkt af",
                        at_most_1: "Hoogstens 1 gegeven wijkt af",
                        exactly_2: "Precies 2 gegevens wijken af",
                        at_most_2: "Hoogstens 2 gegevens wijken af",
                        at_least_3: "3 of meer gegevens wijken af",
                    },
                    field: {
                        MATCHES: "{{field}} komt overeen",
                        DIFFERS: "{{field}} wijkt af",
                    },
                },
                errors: {
                    ACCEPTS_MANUAL_ENTRY:
                        "Inschrijvingen waarvan de identiteit met de hand is getypt kunnen niet automatisch worden goedgekeurd.",
                    ACCEPTS_ALREADY_ENROLLED:
                        "Een kiezer die al is ingeschreven kan niet opnieuw worden goedgekeurd.",
                    ACCEPTS_WITHOUT_VOTER:
                        "Een inschrijving kan niet worden goedgekeurd zonder kiezer in het register.",
                    OTHERWISE_ACCEPTS:
                        "De laatste regel kan inschrijvingen naar een persoon sturen of afwijzen, maar niet goedkeuren.",
                    MISSING_REASON: "Kies wat de kiezer te horen krijgt.",
                    UNEXPECTED_REASON: "Een goedkeuring heeft geen reden.",
                    NO_COMPARED_FIELDS:
                        "Kies ten minste één gegeven om met het register te vergelijken.",
                    DUPLICATE_COMPARED_FIELD: "Een vergeleken gegeven komt twee keer voor.",
                    UNKNOWN_FIELD: "Een regel gebruikt een gegeven dat niet wordt vergeleken.",
                    NO_CONDITIONS:
                        "Voeg ten minste één voorwaarde toe. Alleen de laatste regel geldt voor al het andere.",
                },
                change: {
                    added: "Regel {{number}} toegevoegd",
                    decision: "Regel {{number}}: {{from}} → {{to}}",
                    edited: "Regel {{number}} gewijzigd",
                    removed: "Een regel is verwijderd ({{text}})",
                    moved: "De volgorde van de regels is gewijzigd",
                    otherwise: "De laatste regel is gewijzigd",
                    compared: "De vergeleken gegevens zijn gewijzigd",
                },
                save: {
                    button: "Opslaan als versie {{version}}",
                    title: "Opslaan als versie {{version}}?",
                    body: "Over nieuwe inschrijvingen wordt vanaf nu met deze regels beslist. Inschrijvingen waarover al is beslist behouden hun beslissing.",
                    changes: "Wat er is gewijzigd",
                    log: "De nieuwe versie wordt vastgelegd in het verkiezingslogboek.",
                    confirm: "Versie {{version}} opslaan",
                    success: "Opgeslagen als versie {{version}}",
                    error: "De goedkeuringsmatrix kon niet worden opgeslagen",
                },
            },
        },
        monitoring: {
            configuredDashboards: "Configureerbare dashboards",
            modeChangeError: "De dashboardmodus kon niet worden gewijzigd. Probeer het opnieuw.",
            title: "Monitoring",
            loading: "Monitoring laden",
            unavailableAlert:
                "De monitoringdashboards konden niet worden geladen; het standaarddashboard wordt getoond.",
            noDashboards: "Dit evenement heeft geen monitoringdashboards.",
            dashboardFailed: "Het monitoringdashboard kon niet worden geladen.",
            dashboardInvalid: "Dit dashboard kan niet worden getoond: {{problem}}",
            retry: "Opnieuw proberen",
            errors: {
                busy: "De server is bezet. Over enkele seconden wordt het opnieuw geprobeerd.",
                forbiddenScope:
                    "U mag deze regio, deze Post of dit land niet zien. Kies een andere.",
                snapshotPruned:
                    "De getoonde update wordt niet meer bewaard. Het dashboard toont nu de laatste update: exporteer opnieuw om die te gebruiken.",
                checksUnavailable:
                    "De grafiekdienst is nu niet beschikbaar. Probeer het later opnieuw.",
                lockedDown:
                    "Het verkiezingsevenement is vergrendeld, dus dit kan niet worden gewijzigd.",
                notFound:
                    "Dit dashboard of deze widget is niet meer geconfigureerd. Laad de pagina opnieuw.",
                badRequest:
                    "Het verzoek is niet geaccepteerd. Laad de pagina opnieuw en probeer het nogmaals.",
                conflict:
                    "Iemand anders heeft eerst een wijziging opgeslagen. Laad opnieuw en probeer het nog eens.",
                invalid: "Sommige waarden van de export worden niet geaccepteerd.",
                unknown: "Er ging iets mis. Probeer het later opnieuw.",
            },
            header: {
                dashboard: "Dashboard",
                updated: "Bijgewerkt {{time}} ({{timeZone}})",
                notUpdated: "Nog niet geteld",
                refresh: "elke {{seconds}} s",
                export: "Exporteren",
                editDashboard: "Dashboard bewerken",
                preset: "Dashboardvoorinstelling",
                reload: "Nieuwe cijfers ophalen",
            },
            footer: {
                dataThrough: "Gegevens tot {{time}} ({{timeZone}})",
            },
            selectors: {
                region: "Regio",
                post: "Post",
                country: "Land",
                allRegions: "Alle regio's",
                allPosts: "Alle posten",
                allAuthorizedPosts: "Alle toegestane posten",
                allCountries: "Alle landen",
                authorizedOnly: "{{all}} (toegestaan)",
            },
            widget: {
                menu: "Acties voor {{widget}}",
                configure: "Widget configureren",
                viewData: "Gegevens bekijken",
                export: "Exporteren",
                duplicate: "Dupliceren",
                loading: "{{widget}} laden",
                missing: "Het dashboard noemt een widget die niet bestaat: {{id}}",
                updating: "{{widget}} wordt bijgewerkt",
                updatingNote: "Bijwerken: de getoonde grafiek is de vorige.",
            },
            frame: {
                title: "Grafiek {{widget}}",
            },
            sources: {
                voter_turnout: "Opkomst",
                test_voting: "Proefstemming",
                enrollment_decisions: "Inschrijvingsbeslissingen",
                voting_credentials: "Steminloggegevens",
                poll_status: "Status van de stemming",
                final_testing_lockdown: "Eindtest en vergrendeling",
                counting_transmission: "Telling en verzending",
                voting_enrollment_activity: "Stem- en inschrijvingsactiviteit",
                access_security: "Toegang en beveiliging",
                attack_detections: "Aanvalsdetecties",
                helpdesk: "Helpdesk",
            },
            reasons: {
                TEST_ELECTION_DESIGNATION: "proefverkiezingen kunnen nog niet worden gemarkeerd",
                CREDENTIAL_ISSUED_EVENT: "het uitgeven van inloggegevens wordt nog niet vastgelegd",
                FINAL_TESTING_LOCKDOWN_STATE:
                    "eindtest en vergrendeling worden nog niet vastgelegd",
                ATTACK_DETECTION_FEED: "er is geen bron voor aanvalsdetectie gekoppeld",
                HELPDESK_INTEGRATION: "er is geen helpdesksysteem gekoppeld",
            },
            notices: {
                UNREGISTERED_ATTEMPTS_AT_EVENT_SCOPE_ONLY:
                    "Pogingen met niet-geregistreerde gebruikersnamen horen bij geen enkele post en worden daarom alleen voor het hele evenement geteld.",
                UNREGISTERED_ATTEMPTS_EXCLUDED:
                    "Deze cijfers laten pogingen met niet-geregistreerde gebruikersnamen buiten beschouwing; die worden voor het hele evenement geteld.",
                CREDENTIALS_ISSUED_WHEN_PASSWORD_SET:
                    "Inloggegevens tellen als uitgegeven zodra het wachtwoord van de kiezer is ingesteld, totdat het platform de uitgifte registreert.",
                CONFIG_NEWER_THAN_SNAPSHOT:
                    "Getekend met de nieuwste instellingen; de cijfers worden er bij de volgende telling mee geteld.",
                CONFIG_AT_SNAPSHOT_UNAVAILABLE:
                    "Getekend met de huidige instellingen: de instellingen waarmee de cijfers zijn geteld, worden niet meer bewaard.",
            },
            unavailable: {
                notConnected: "Niet gekoppeld · {{reason}}",
                notConnectedHelp: "Er wordt niets getoond totdat dat zo is.",
                unknownReason: "gegevensbron niet beschikbaar",
                noSnapshot: "Nog niet geteld",
                noSnapshotHelp:
                    "De eerste telling is nog niet klaar. Deze widget werkt zichzelf bij.",
                scopePending: "Deze selectie wordt geteld",
                scopePendingHelp:
                    "Deze selectie wordt in de volgende ronde geteld, binnen ongeveer een minuut.",
                settingsPending: "Tellen met de nieuwe instellingen…",
                settingsPendingHelp:
                    "De cijfers worden binnen ongeveer een minuut opnieuw geteld met de opgeslagen instellingen.",
                renderFailed: "De grafiek kon niet worden getekend",
                renderFailedHelp: "In plaats daarvan worden de cijfers getoond.",
                invalid: "Deze widget kan niet worden getoond",
                invalidHelp: "De configuratie ervan bevat een probleem.",
                requestFailed: "Deze widget kon niet worden geladen",
                requestFailedHelp:
                    "Het wordt opnieuw geprobeerd wanneer het dashboard wordt bijgewerkt.",
            },
            dataTable: {
                title: "{{widget}} · gegevens",
                close: "Sluiten",
                empty: "Geen rijen",
                rowsPerPage: "Rijen per pagina:",
                shownRows: "{{from}}–{{to}} van {{total}}",
                firstPage: "Eerste pagina",
                previousPage: "Vorige pagina",
                nextPage: "Volgende pagina",
                lastPage: "Laatste pagina",
            },
            columns: {
                numerator: "Teller",
                denominator: "Noemer",
                pct: "Percentage",
                pct_label: "Percentage zoals getoond",
                group: "Groep",
                group_key: "Groepssleutel",
                post: "Post",
                post_id: "Post-ID",
                region: "Regio",
                country: "Land",
                reason: "Reden",
                category: "Categorie",
                state: "Status",
                state_label: "Status zoals getoond",
                bucket_start: "Begin van de periode",
                bucket_label: "Periode",
                bucket_utc: "Begin van de periode (UTC)",
                measure: "Meetwaarde",
                label: "Label",
                value: "Waarde",
            },
            measures: {
                registered: "Geregistreerd",
                pre_enrolled: "Voorlopig ingeschreven",
                credentials_issued: "Inloggegevens uitgegeven",
                test_voted: "Testgestemd",
                voted: "Gestemd",
                voted_pre_enrolled: "Vooraf ingeschreven en gestemd",
                applications: "Aanvragen",
                pending: "In behandeling",
                approved: "Goedgekeurd",
                disapproved: "Afgekeurd",
                posts: "Posten",
                initialized: "Geïnitialiseerd",
                opened: "Geopend",
                paused: "Gepauzeerd",
                closed: "Gesloten",
                tested: "Getest",
                locked_down: "Vergrendeld",
                tallied: "Geteld",
                transmitted: "Verzonden",
                transmission_failed: "Verzending mislukt",
                logins: "Aanmeldingen",
                login_failures: "Mislukte aanmeldingen",
                login_failures_valid_user: "Mislukt, geldige gebruiker",
                login_failures_unregistered: "Mislukt, niet-geregistreerde gebruiker",
                password_resets: "Wachtwoordherstel",
                password_reset_requests: "Aanvragen om wachtwoordherstel",
                detections: "Detecties",
                issues: "Meldingen",
                pending_issues: "Openstaande meldingen",
            },
            export: {
                title: "Monitoringgegevens exporteren",
                format: "Formaat",
                csv: "CSV",
                sql: "SQL",
                from: "Van",
                to: "Tot",
                timeZoneHelp:
                    "Tijden zijn in {{timeZone}}. Totalen, statussen en groepen zijn die van de getoonde update; alleen de rijen van activiteitsreeksen zijn beperkt tot de periode, vanaf de begintijd tot, maar niet met, de eindtijd.",
                cancel: "Annuleren",
                export: "Exporteren",
                invalidRange: "Het einde moet na het begin liggen.",
                problems: {
                    unknownSelector:
                        "{{widget}} heeft de keuze ‘{{selector}}’ niet meer. Laad het dashboard opnieuw en exporteer opnieuw.",
                    unknownOption:
                        "De gekozen waarde voor ‘{{selector}}’ in {{widget}} wordt niet meer aangeboden. Kies deze opnieuw en exporteer.",
                },
            },
            editor: {
                scopeSelector: {
                    region: "Regio",
                    post: "Functie",
                    country: "Land",
                },
                sources: {
                    voter_turnout: "Opkomst van kiezers",
                    test_voting: "Teststemming",
                    enrollment_decisions: "Inschrijvingsbeslissingen",
                    voting_credentials: "Stemgegevens",
                    poll_status: "Status van de stemming",
                    final_testing_lockdown: "Eindtests en vergrendeling",
                    counting_transmission: "Tellen en doorgeven",
                    voting_enrollment_activity: "Stem- en inschrijvingsactiviteit",
                    access_security: "Toegang en beveiliging",
                    attack_detections: "Aanvalsdetecties",
                    helpdesk: "Helpdesk",
                },
                templates: {
                    summary: "Samenvatting",
                    by_group: "Per groep",
                    by_post: "Per functie",
                    timeseries: "In de tijd",
                    by_measure: "Per maat",
                },
                yaml: {
                    label: "YAML",
                    readOnly:
                        "De YAML bevat een syntaxisfout. Herstel deze op het tabblad YAML om de formulieren weer te gebruiken.",
                },
                diagnostics: {
                    title: "Controles",
                    none: "Geen problemen gevonden",
                    localUnavailable:
                        "Controles in de browser zijn niet beschikbaar; problemen verschijnen na de voorvertoning of Valideren.",
                    line: "Regel {{line}}",
                    engineCode: "dbt Charts {{code}}",
                    severity: {
                        ERROR: "Fout",
                        WARNING: "Waarschuwing",
                    },
                    origin: {
                        SYNTAX: "YAML-syntaxis",
                        LOCAL: "Browsercontrole",
                        SERVER: "Servercontrole",
                    },
                },
                footer: {
                    preview: "Voorvertoning",
                    valid: "Geldig",
                    errors_one: "{{count}} fout",
                    errors_other: "{{count}} fouten",
                    noWarnings: "geen grafiekwaarschuwingen",
                    warnings_one: "{{count}} grafiekwaarschuwing",
                    warnings_other: "{{count}} grafiekwaarschuwingen",
                    rendering: "Genereren…",
                    previewFailed: "voorvertoning mislukt",
                    renderedIn: "gegenereerd in {{ms}} ms",
                    revision: "Revisie {{revision}}",
                    savedBy: "opgeslagen {{date}} door {{user}}",
                    unknownUser: "een beheerder",
                    notSaved: "nog niet opgeslagen",
                    unsaved: "niet-opgeslagen wijzigingen",
                    cancel: "Annuleren",
                    validate: "Valideren",
                },
                preview: {
                    title: "Voorvertoning",
                    empty: "De voorvertoning verschijnt zodra de YAML geldig is.",
                    failed: "De voorvertoning kon niet worden gegenereerd: {{reason}}",
                    notConnectedTail: "Er wordt niets getoond totdat dat zo is.",
                    queryResult: "Queryresultaat · eerste rijen",
                    noRows: "De query gaf geen rijen terug.",
                    state: {
                        RENDERED: "Gegenereerd",
                        NOT_CONNECTED: "Niet verbonden",
                        NO_SNAPSHOT: "Nog geen gegevens",
                        SCOPE_PENDING: "Dit bereik wordt geteld",
                        RENDER_FAILED: "De grafiek kon niet worden getekend",
                        INVALID: "De configuratie is ongeldig",
                    },
                },
                configureWidget: {
                    title: "Widget configureren",
                    tabs: {
                        dataQuery: "Gegevens en query",
                        selectors: "Selectors",
                        yaml: "YAML",
                        preview: "Voorbeeld",
                    },
                    save: "Widget opslaan",
                    loading: "Widget laden…",
                    loadFailed: "De widget kon niet worden geladen: {{reason}}",
                    saved: "Widget opgeslagen als revisie {{revision}}",
                    refused: "De widget is niet opgeslagen: los de vermelde problemen op.",
                    validated: "De widget is geldig.",
                    invalid: "De widget heeft problemen: zie de controles.",
                    requestFailed: "Het verzoek is mislukt: {{reason}}",
                    discardTitle: "Wijzigingen negeren?",
                    discardBody: "Je wijzigingen aan deze widget zijn niet opgeslagen.",
                    discard: "Negeren",
                    keepEditing: "Verder bewerken",
                },
                dataQuery: {
                    title: "Titel",
                    source: "Gegevensbron",
                    template: "Query",
                    measures: "Maten",
                    ratio: "Teller / noemer",
                    numerator: "Teller",
                    denominator: "Noemer",
                    groupBy: "Groeperen op",
                    sort: "Sorteren",
                    sortOrder: "Volgorde",
                    templateOrder: "De eigen volgorde van de query",
                    sortBy: {
                        label: "Label",
                        value: "Waarde",
                        ratio: "Verhouding",
                    },
                    order: {
                        asc: "Oplopend",
                        desc: "Aflopend",
                    },
                    limit: "Rijen",
                    noLimit: "Alle",
                    none: "Geen",
                    fromSelector: "Van selector: {{name}}",
                    follow: "Dashboardselectors volgen",
                    followHelp: "Een selector die niet is aangevinkt, beperkt deze widget niet.",
                    manyQueries:
                        "Deze widget heeft meerdere queries; bewerk ze op het tabblad YAML.",
                    notConnected: "Niet verbonden · {{reason}}",
                    sourceHelp: {
                        DISTINCT_VOTERS:
                            "Telt unieke kiezers; de telregels liggen vast in de gegevensbron.",
                        DISTINCT_PRE_ENROLLED_VOTERS:
                            "Telt unieke vooraf ingeschreven kiezers; de telregels liggen vast in de gegevensbron.",
                        LATEST_DECISION_PER_VOTER:
                            "Telt de laatste beslissing per kiezer; de telregels liggen vast in de gegevensbron.",
                        APPROVED_VOTERS:
                            "Telt goedgekeurde kiezers; de telregels liggen vast in de gegevensbron.",
                        POSTS_IN_SCOPE:
                            "Telt de functies binnen het bereik; de telregels liggen vast in de gegevensbron.",
                        FIRST_EVENT_PER_VOTER:
                            "Telt de eerste geldige stem of goedkeuring van elke kiezer; de telregels liggen vast in de gegevensbron.",
                        ATTEMPTS:
                            "Telt pogingen, geen personen; de telregels liggen vast in de gegevensbron.",
                        DETECTIONS: "Telt detecties; de telregels liggen vast in de gegevensbron.",
                        REPORTED_ISSUES:
                            "Telt gemelde problemen; de telregels liggen vast in de gegevensbron.",
                        default: "De telregels liggen vast in de gegevensbron.",
                    },
                },
                selectors: {
                    help: "Selectors verschijnen in de koptekst van de widget. Hun waarden voeden de query; dashboardselectors (Regio, Functie, Land) gelden voor elke widget.",
                    name: "Naam",
                    label: "Label",
                    control: "Bedieningselement",
                    controls: {
                        dropdown: "Keuzelijst",
                        toggle: "Schakelaar",
                    },
                    default: "Standaard",
                    optionValue: "Waarde",
                    optionLabel: "Optielabel",
                    addOption: "Optie toevoegen",
                    addSelector: "Selector toevoegen",
                    removeOption: "Optie {{option}} verwijderen",
                    removeSelector: "Selector {{name}} verwijderen",
                    moveUp: "{{name}} omhoog",
                    moveDown: "{{name}} omlaag",
                    dynamic: "De opties komen uit de gegevens ({{source}}).",
                    none: "Deze widget heeft geen selectors.",
                    newLabel: "Nieuwe selector",
                    newOption: "Nieuwe optie",
                    shownWhen: "Getoond wanneer {{selector}} {{values}} is",
                },
                conflict: {
                    title: "Iemand was je voor",
                    body: "{{user}} heeft revisie {{revision}} opgeslagen ({{date}}) terwijl je aan het bewerken was.",
                    bodyShort:
                        "Revisie {{revision}} is opgeslagen terwijl je aan het bewerken was.",
                    saved: "Opgeslagen revisie",
                    mine: "Mijn wijzigingen",
                    reload: "Opnieuw laden",
                    copy: "Mijn YAML kopiëren",
                    copied: "Je YAML staat op het klembord.",
                    copyFailed:
                        "Het klembord is niet beschikbaar; selecteer de YAML en kopieer hem handmatig.",
                    keepEditing: "Verder bewerken",
                    removed:
                        "Het document is verwijderd terwijl je aan het bewerken was. Bewerk verder om het opnieuw op te slaan.",
                },
                dashboard: {
                    editing: "Dashboard bewerken",
                    title: "Titel",
                    selectors: "Dashboardselectors",
                    theme: "Thema",
                    editTheme: "Thema bewerken",
                    addWidget: "Widget toevoegen",
                    cancel: "Annuleren",
                    save: "Dashboard opslaan",
                    width: "Breedte",
                    widthValue: "{{n}} van 12",
                    moveUp: "Omhoog",
                    moveDown: "Omlaag",
                    remove: "Verwijderen",
                    duplicate: "Dupliceren",
                    configure: "Widget configureren",
                    empty: "Dit dashboard heeft nog geen widgets.",
                    saved: "Dashboard opgeslagen als revisie {{revision}}",
                    refused: "Het dashboard is niet opgeslagen: los de vermelde problemen op.",
                    requestFailed: "Het verzoek is mislukt: {{reason}}",
                    dragHandle: "Sleep om {{title}} te herordenen",
                    widgets: "Widgets",
                    duplicated: "Gedupliceerd als {{id}}",
                    resetToPreset: "Terugzetten naar voorinstelling",
                    actions: "Acties voor {{title}}",
                    discardBody: "Je wijzigingen aan dit dashboard zijn niet opgeslagen.",
                    duplicateInvalid: "De kopie is niet opgeslagen: {{problem}}",
                    layoutMalformed:
                        "Sommige lay-outitems zijn geen widget met een breedte. Herstel ze op het YAML-tabblad om de widgets te herschikken.",
                },
                catalog: {
                    title: "Widget toevoegen",
                    search: "Zoek widgets, gegevensbronnen of vereisten",
                    add: "Toevoegen",
                    empty: "Geen widget gevonden.",
                    onDashboard: "Op dit dashboard",
                    close: "Sluiten",
                },
                theme: {
                    title: "Dashboardthema",
                    subtitle:
                        "dbt Charts-stijl die op elke widget van dit dashboard wordt toegepast",
                    appliesTo_one: "geldt voor {{count}} widget",
                    appliesTo_other: "geldt voor {{count}} widgets",
                    apply: "Thema toepassen",
                    saved: "Thema opgeslagen als revisie {{revision}}",
                    discardBody: "Je wijzigingen aan dit thema zijn niet opgeslagen.",
                },
                reset: {
                    title: "Terugzetten naar voorinstelling",
                    body: "Alle dashboards, widgets en thema's van dit evenement worden vervangen door die van de voorinstelling. De huidige configuratie blijft in de geschiedenis.",
                    preset: "Voorinstelling",
                    confirm: "Terugzetten",
                    cancel: "Annuleren",
                    done: "Het evenement gebruikt nu {{title}}.",
                    failed: "Het terugzetten is mislukt: {{reason}}",
                    noPresets: "Er zijn geen voorinstellingen beschikbaar.",
                    loading: "Voorinstellingen laden…",
                },
                lockedDown:
                    "Het evenement is vergrendeld; de monitoringconfiguratie kan niet worden gewijzigd.",
                document: {
                    loadFailed: "Het document kon niet worden geladen: {{reason}}",
                    refused: "Niet opgeslagen: los de genoemde problemen op.",
                    validated: "Het document is geldig.",
                    invalid: "Het document bevat problemen: zie de controles.",
                    requestFailed: "Het verzoek is mislukt: {{reason}}",
                    savedWithWarnings_one: "{{count}} waarschuwing: zie de controles.",
                    savedWithWarnings_other: "{{count}} waarschuwingen: zie de controles.",
                },
                errors: {
                    checksUnavailable:
                        "De grafiekengine kon de wijziging niet controleren, dus die is niet opgeslagen. Probeer het zo meteen opnieuw.",
                    busy: "Andere wijzigingen aan dit evenement worden opgeslagen. Probeer het zo meteen opnieuw.",
                    lockedDown:
                        "Het evenement is vergrendeld; de monitoringconfiguratie kan niet meer veranderen.",
                    forbiddenScope:
                        "Je mag de cijfers van de gekozen regio, functie of het gekozen land niet zien.",
                    badRequest:
                        "De editor stuurde een verzoek dat de server niet kon lezen. Laad de pagina opnieuw en probeer het nog eens.",
                },
                duplicate: {
                    copyTitle: "{{title}} (kopie)",
                    done: "{{id}}, een kopie van de widget, is aan het dashboard toegevoegd.",
                    failed: "De widget kon niet worden gedupliceerd: {{reason}}",
                    notPlaced:
                        "De kopie {{id}} is opgeslagen, maar het dashboard heeft haar niet opgenomen ({{reason}}). Voeg haar toe met Dashboard bewerken.",
                },
            },
        },
        monitoringDashboardScreen: {
            voters: {
                title: "Kiezers",
                enrolledOverseasVoters: "Ingeschreven Buitenlandse Kiezers",
                approvalStatus: "Goedkeuringsstatus: Goedgekeurde/Afgekeurde Kiezers",
                manuallyApproval: "Handmatig Goedgekeurde/Afgekeurde Kiezers",
                automaticallyApproval: "Automatisch Goedgekeurde/Afgekeurde Kiezers",
                authenticatedVoters: "Geauthenticeerde Kiezers",
                invalidUserErrors: "Fouten Ongeldige Gebruiker:",
                invalidPasswordErrors: "Fouten Ongeldig Wachtwoord:",
            },
            polls: {
                title: "Stembureaus",
                initializedSystems: "Posten met Geïnitialiseerde Systemen",
                votingOpened: "Posten met Online Stemmen Geopend",
                votingClosed: "Posten met Beide Stemkanalen Gesloten",
                votingStarted: "Posten met Stemmen Gestart",
                voterTurnout: "Opkomst Kiezers",
            },
            tally: {
                title: "Telling",
                activeVotesCounting: "Posten met Actieve Stemtelling",
                generatedERs: "Posten met Gegenereerde Verkiezingsresultaten",
                transmittedResults: "Posten met Verzonden Resultaten",
            },
            testing: {
                title: "Testen",
                testElectionVoterCount: "Aantal Kiezers Testverkiezing",
            },
        },
        certificateAuthorities: {
            title: "Certificaten",
            subtitle:
                "Vertrouwde certificeringsinstanties (CA's) voor dit verkiezingsevenement. Geïmporteerde CA's worden gebruikt om kiezercertificaten te valideren.",
            importButton: "Certificaten importeren",
            type: {
                root: "Wortel",
                intermediate: "Tussenliggend",
            },
            expiry: {
                expired: "Verlopen",
                expiringSoon: "Verloopt binnenkort",
                valid: "Geldig",
            },
            columns: {
                commonName: "Gewone naam",
                type: "Type",
                issuerCn: "CN van uitgever",
                notBefore: "Geldig vanaf",
                notAfter: "Verloopt",
                fingerprint: "SHA256 Vingerafdruk",
            },
            importDialog: {
                title: "Certificaatautoriteiten importeren",
                subtitle: "Importeer een of meer CA-certificaten vanuit een PEM-bestand",
                description:
                    "Selecteer een PEM-bestand met een of meer certificaten. Bundels worden ondersteund — elk certificaat wordt afzonderlijk geïmporteerd.",
                selectFile: "PEM-bestand selecteren",
                fileLoaded: "Bestand geladen ({{bytes}} bytes)",
                importButton: "Importeren",
            },
            notify: {
                importSuccess: "{{inserted}} certificaat(-en) geïmporteerd.",
                importSkipped: "{{count}} overgeslagen (al aanwezig).",
                importErrors: "Importproblemen: {{errors}}",
                importError: "Importeren mislukt: {{error}}",
                deleteSuccess: "Certificaat verwijderd.",
                deleteError: "Fout bij verwijderen van certificaat.",
                exportSuccess: "Certificaat(-en) succesvol geëxporteerd.",
                exportError: "Fout bij exporteren van certificaten.",
            },
            exportDialog: {
                title: "Certificaatautoriteiten exporteren",
                description: "U staat op het punt {{amount}} certificaat(-en) te exporteren.",
                all: "alle",
            },
            deleteDialog: {
                description: "Weet u zeker dat u {{count}} certificaat(-en) wilt verwijderen?",
            },
            emptyHeader:
                "Er zijn geen certificaatautoriteiten geïmporteerd voor dit verkiezingsevenement.",
            fileReadError: "Bestand lezen mislukt.",
            viewDialog: {
                title: "Details van certificaatautoriteit",
                subject: "Onderwerp",
                issuer: "Uitgever",
                serialNumber: "Serienummer",
                pemContent: "PEM-inhoud",
            },
            confirmDelete: "Certificaatautoriteit verwijderen",
            confirmDeleteDescription:
                'Weet u zeker dat u het certificaat "{{name}}" (vingerafdruk: {{fingerprint}}) wilt verwijderen?',
        },
        signing: {
            terms: {
                post: "Post",
                posts: "Posts",
            },
            tab: {
                title: "Handtekeningen",
                intro: "Beschermde acties worden pas uitgevoerd nadat genoeg bevoegde personen ze met hun digitale certificaat hebben ondertekend. Elke handtekening wordt gecontroleerd tegen de vertrouwde uitgevers en vastgelegd in het logboek.",
                protectedActions: "Beschermde acties",
                certificates: "Certificaten",
                requests: "Verzoeken",
            },
            loadError:
                "De ondertekeningsinstellingen konden niet worden geladen. Laad de pagina opnieuw om het nogmaals te proberen.",
            errors: {
                automatedCeremonies:
                    "Dit evenement gebruikt automatische sleutelceremonies. Beheerders voeren deze stappen niet uit, dus hun handtekeningen kunnen niet worden vereist. Gebruik handmatige sleutelceremonies om hun handtekeningen te vereisen.",
                forbidden: "U hebt geen toestemming voor deze wijziging.",
                invalid:
                    "De server heeft deze waarden geweigerd. Controleer ze en probeer het opnieuw.",
                conflict:
                    "Iemand anders heeft dit intussen gewijzigd. Laad de pagina opnieuw en probeer het nogmaals.",
                lockedDown:
                    "Het verkiezingsevenement is vergrendeld: ondertekeningsregels veranderen alleen via een nieuwe configuratieversie.",
                notFound: "Het bestaat niet meer. Laad de pagina opnieuw.",
            },
            readOnly: {
                chip: "Alleen lezen",
                rules: "Alleen lezen. Voor het wijzigen van ondertekeningsregels is de machtiging “Handtekeningen: beschermde acties bewerken” nodig.",
                whoCanSign:
                    "Rollen met de machtiging “Ondertekenen: {{action}}” in Gebruikers en Rollen. Voor het wijzigen ervan is de machtiging om rollen te bewerken nodig.",
            },
            groups: {
                "voting": "Stemmen",
                "results-and-reports": "Resultaten en rapporten",
                "enrollment": "Inschrijving",
                "configuration-and-keys": "Configuratie en sleutels",
            },
            actions: {
                "initialize-voting": {
                    label: "Stemming initialiseren",
                    short: "Initialisatie",
                    permissionName: "stemming initialiseren",
                    object: "initialisatie van de stemming",
                    appliesTo: "Elke $t(signing.terms.post)",
                    description:
                        "Gestart in Publiceren. Initialiseert de $t(signing.terms.post) en genereert het Initialisatierapport.",
                },
                "open-voting": {
                    label: "Stemming openen",
                    short: "Opening",
                    permissionName: "stemming openen",
                    object: "opening van de stemming",
                    appliesTo: "Elke $t(signing.terms.post)",
                    description:
                        "Gestart in Publiceren met Stemperiode Starten. Opent de stemming op de $t(signing.terms.post).",
                },
                "close-voting": {
                    label: "Stemming sluiten",
                    short: "Sluiting",
                    permissionName: "stemming sluiten",
                    object: "sluiting van de stemming",
                    appliesTo: "Elke $t(signing.terms.post)",
                    description:
                        "Gestart in Publiceren met Stemperiode Stoppen. Sluit de stemming op de $t(signing.terms.post); de sluitingshandtekeningen worden in het record ervan bewaard.",
                    descriptionSealed:
                        "Gestart in Publiceren met Stemperiode Stoppen. Sluit de stemming op de $t(signing.terms.post). Zodra elk kanaal gesloten is, worden de stembussen ervan verzegeld, na de respijtperiode als die er is: er kan geen stembiljet worden toegevoegd, gewijzigd of verwijderd, en de stemming kan niet opnieuw beginnen. De sluitingshandtekeningen worden in het record ervan bewaard.",
                },
                "generate-election-returns": {
                    label: "Processen-verbaal genereren",
                    short: "Processen-verbaal",
                    permissionName: "processen-verbaal genereren",
                    object: "processen-verbaal",
                    appliesTo: "Elke $t(signing.terms.post) en elk land",
                    description:
                        "Gestart door de telling, één verzoek per $t(signing.terms.post) en land. Geeft de ondertekende processen-verbaal vrij om te printen en te verzenden.",
                },
                "generate-reports": {
                    label: "Andere verkiezingsrapporten genereren",
                    short: "Rapport",
                    permissionName: "andere verkiezingsrapporten genereren",
                    object: "rapport",
                    appliesTo: "Elke $t(signing.terms.post)",
                    description:
                        "Gestart door de telling voor het Initialisatierapport en in Rapporten voor het deelnamerapport. Geeft het ondertekende rapport vrij.",
                },
                "transmit-results": {
                    label: "Resultaten verzenden",
                    short: "Verzending",
                    permissionName: "resultaten verzenden",
                    object: "resultatenpakket",
                    appliesTo: "Elke $t(signing.terms.post) en elk land",
                    description:
                        "Gestart in Telling, Verzending. Stelt het ondertekende resultatenpakket samen voor de bestemmingen; de handtekeningen vullen de handtekeningenlijst.",
                },
                "approve-voter": {
                    label: "Een kiezer handmatig goedkeuren",
                    short: "Goedkeuring kiezer",
                    permissionName: "een kiezer handmatig goedkeuren",
                    object: "goedkeuring van de kiezer",
                    appliesTo: "De $t(signing.terms.post) van de kiezer",
                    description:
                        "Gestart in Goedkeuringen. Keurt de kiezer goed en geeft de inloggegevens uit.",
                },
                "approve-configuration": {
                    label: "Een configuratieversie goedkeuren",
                    short: "Configuratieversie",
                    permissionName: "een configuratieversie goedkeuren",
                    object: "configuratieversie",
                    appliesTo: "Het verkiezingsevenement",
                    description: "Gestart in Publiceren. Publiceert de configuratieversie.",
                },
                "key-ceremony": {
                    label: "Een sleutelfragment bevestigen (sleutelceremonie)",
                    short: "Sleutelfragment",
                    permissionName: "een sleutelfragment bevestigen",
                    object: "sleutelfragment",
                    appliesTo: "Elke trustee",
                    description:
                        "Door elke trustee gestart in Sleutels. Legt de handtekening van de trustee vast bij de ceremonie en op het bulletinboard.",
                },
                "tally-key": {
                    label: "Een sleutelfragment bijdragen (telling)",
                    short: "Bijdrage sleutelfragment",
                    permissionName: "een sleutelfragment bijdragen",
                    object: "bijdrage van het sleutelfragment",
                    appliesTo: "Elke trustee",
                    description:
                        "Door elke trustee gestart in Telling. Legt de bijdrage van de trustee vast.",
                },
            },
            protectedActions: {
                intro: "Elke handtekening wordt gezet met het digitale certificaat op het beveiligingstoken van de ondertekenaar.",
                columns: {
                    action: "Actie",
                    appliesTo: "Geldt voor",
                    whoCanSign: "Wie kan ondertekenen",
                    signaturesNeeded: "Benodigde handtekeningen",
                    requestExpires: "Verzoek verloopt",
                    waiting: "Wachtend",
                },
                off: "Uit",
                eachTrustee: "Elke trustee",
                footerVersion:
                    "Ondertekeningsregels maken deel uit van configuratieversie {{version}} van dit evenement.",
                footerFirstVersion:
                    "De ondertekeningsregels worden deel van de eerste configuratieversie van dit evenement wanneer die wordt gepubliceerd.",
                footerChanged: "Laatst gewijzigd op {{date}}.",
                footerChangedBy: "Laatst gewijzigd op {{date}} door {{name}}.",
                lockedDown:
                    "Het verkiezingsevenement is vergrendeld: de ondertekeningsregels horen bij de configuratieversie en veranderen dus alleen via een nieuwe configuratieversie.",
                edit: "{{action}} bewerken",
                view: "{{action}} bekijken",
                waitingCount_one: "{{count}} verzoek wacht",
                waitingCount_other: "{{count}} verzoeken wachten",
                capacityError:
                    "Wie kan ondertekenen kon niet worden geladen, dus het aantal handtekeningen kan niet worden gecontroleerd tegen de $t(signing.terms.posts).",
            },
            expiry: {
                "30": "30 minuten",
                "60": "1 uur",
                "120": "2 uur",
                "1440": "24 uur",
                "none": "Geen limiet",
                "other": "{{count}} minuten",
            },
            rule: {
                needsSignatures: "Vereist handtekeningen",
                whoCanSign: "Wie kan ondertekenen",
                whoCanSignHelp:
                    "Deze rollen krijgen de machtiging “Ondertekenen: {{action}}” in Gebruikers en Rollen, voor elk verkiezingsevenement. Ondertekenaars moeten ook toegang hebben tot de $t(signing.terms.post).",
                signaturesNeeded: "Benodigde handtekeningen",
                signaturesNeededHelp:
                    "Elke ondertekenaar gebruikt zijn digitale certificaat. Elke $t(signing.terms.post) heeft minstens {{n}} personen die kunnen ondertekenen.",
                signaturesNeededShortHelp: "Elke ondertekenaar gebruikt zijn digitale certificaat.",
                requesterSigning: "De persoon die het start, mag ook ondertekenen",
                expiresAfter: "Een verzoek verloopt na",
                trusteesSign: "Trustees ondertekenen deze stap",
                trusteesHelp:
                    "Elke trustee ondertekent zijn eigen stap met zijn digitale certificaat. De sleutelceremonie bepaalt hoeveel trustees deelnemen.",
                footer: "Wijzigingen worden vastgelegd in het logboek van het verkiezingsevenement en worden onderdeel van de volgende configuratieversie.",
                cancel: "Annuleren",
                save: "Opslaan",
                saved: "De ondertekeningsregel is opgeslagen.",
                savedShort_one:
                    "De ondertekeningsregel is opgeslagen. {{posts}} kan het aantal nog niet halen: voeg daar een ondertekenaar toe.",
                savedShort_other:
                    "De ondertekeningsregel is opgeslagen. {{posts}} kunnen het aantal nog niet halen: voeg daar ondertekenaars toe.",
                checkedOnSave:
                    "Het aantal wordt bij het opslaan gecontroleerd tegen de nieuwe rollen.",
                savedRequesterShort:
                    "De ondertekeningsregel is opgeslagen. Sommige $t(signing.terms.posts) kunnen het aantal niet halen zonder de persoon die een verzoek start.",
                saveError:
                    "De ondertekeningsregel kon niet worden opgeslagen. Mogelijk heeft iemand hem intussen gewijzigd; laad opnieuw en probeer het nogmaals.",
            },
            validation: {
                atLeastOne: "Minstens 1.",
                tooMany:
                    "Geen enkele $t(signing.terms.post) heeft {{n}} personen die kunnen ondertekenen. Het maximum is {{max}}.",
                tooManyEvent:
                    "Slechts {{max}} personen kunnen dit ondertekenen. Kies maximaal {{max}}.",
                atMost: "Maximaal {{max}}.",
                shortPosts_one:
                    "{{posts}} heeft slechts {{n}} personen die kunnen ondertekenen en kan dus geen {{required}} handtekeningen halen. Voeg daar een ondertekenaar toe of verlaag het aantal.",
                requesterShort_one:
                    "Zonder de persoon die het start, heeft {{posts}} slechts {{n}} personen die kunnen ondertekenen en kan dus geen {{required}} handtekeningen halen.",
                requesterShort_other:
                    "Zonder de persoon die het start, hebben {{posts}} slechts {{n}} personen die kunnen ondertekenen en kunnen dus geen {{required}} handtekeningen halen.",
                shortPosts_other:
                    "{{posts}} hebben slechts {{n}} personen die kunnen ondertekenen en kunnen dus geen {{required}} handtekeningen halen. Voeg daar een ondertekenaar toe of verlaag het aantal.",
            },
            pendingRequests_one:
                "{{count}} verzoek wacht op handtekeningen onder de huidige regel. Opslaan annuleert het; de persoon die het startte, begint opnieuw.",
            pendingRequests_other:
                "{{count}} verzoeken wachten op handtekeningen onder de huidige regel. Opslaan annuleert ze; de personen die ze startten, beginnen opnieuw.",
            certificates: {
                issuersIntro:
                    "Certificaten van medewerkers moeten naar een van deze uitgevers leiden. Ze staan los van de certificaten waarmee kiezers inloggen.",
                checkRevocation: "Intrekkingslijsten controleren",
                crlUnavailable: {
                    "label": "Als een lijst niet kan worden gedownload",
                    "refuse": "Geen handtekeningen accepteren",
                    "accept-unchecked":
                        "Accepteren en de handtekening als ongecontroleerd markeren",
                },
                registration: {
                    "label": "Een certificaat aan een persoon registreren",
                    "on-first-use": "Wanneer de houder er voor het eerst mee ondertekent",
                    "security-officer-only":
                        "Alleen wanneer iemand die certificaten mag registreren het registreert",
                },
                onePost: "Een certificaat ondertekent voor slechts één $t(signing.terms.post)",
                issuers: "Vertrouwde uitgevers",
                import: "Uitgeverscertificaten importeren",
                importHelp:
                    "Kies een PEM- of CER-bestand met het certificaat van de uitgever. Een PEM-bestand kan meerdere certificaten bevatten.",
                chooseFile: "Kies een certificaatbestand",
                fileError: "Het bestand kon niet worden gelezen.",
                imported:
                    "{{imported}} uitgeverscertificaten geïmporteerd; {{skipped}} waren al vertrouwd.",
                importedWithErrors:
                    "{{imported}} uitgeverscertificaten geïmporteerd, {{skipped}} al vertrouwd. Geweigerd: {{errors}}",
                importError: "De uitgeverscertificaten konden niet worden geïmporteerd.",
                deleteIssuer: "{{name}} verwijderen",
                deleteIssuerConfirm:
                    "{{name}} uit de vertrouwde uitgevers verwijderen? Certificaten die deze heeft uitgegeven, kunnen dan niet meer ondertekenen.",
                deleteError: "De uitgever kon niet worden verwijderd.",
                noIssuers:
                    "Nog geen vertrouwde uitgevers. Medewerkers kunnen pas ondertekenen als er een is geïmporteerd.",
                root: "Root",
                intermediate: "Tussenliggend",
                columns: {
                    issuer: "Uitgever",
                    type: "Type",
                    issuedBy: "Uitgegeven door",
                    validUntil: "Geldig tot",
                    sha256: "SHA-256",
                    person: "Persoon",
                    post: "$t(signing.terms.post)",
                    certificate: "Certificaat",
                    registered: "Geregistreerd",
                    status: "Status",
                },
                checks: "Controles",
                checksSaved: "De certificaatcontroles zijn opgeslagen.",
                checksError: "De certificaatcontroles konden niet worden opgeslagen.",
                crlSchedule: "Elk uur bij elke uitgever gedownload.",
                crlUpdated: "{{url}}: bijgewerkt {{time}}",
                crlFailed: "{{url}}: kon niet worden gedownload (laatste poging {{time}})",
                registeredTitle: "Geregistreerde certificaten",
                search: "Zoek personen, certificaten of $t(signing.terms.posts)",
                status: "Status",
                statusAll: "Alle",
                statuses: {
                    "active": "Actief",
                    "expires-soon": "Verloopt binnenkort",
                    "expired": "Verlopen",
                    "revoked": "Ingetrokken",
                },
                revokedOn: "Ingetrokken op {{date}}",
                allPosts: "Alle",
                noCertificates: "Geen geregistreerde certificaten.",
                registeredHow: {
                    "first-use": "Bij de eerste handtekening",
                    "security-officer": "Geregistreerd door een beheerder",
                },
                register: "Een certificaat registreren",
                registerSubmit: "Registreren",
                registerDone: "Het certificaat is geregistreerd.",
                registerError: "Het certificaat kon niet worden geregistreerd.",
                person: "Persoon",
                personSearchHelp: "Typ een deel van een gebruikersnaam om de persoon te vinden.",
                registeredBy: "Door {{name}}",
                registerRefused:
                    "Dit certificaat kan niet worden geregistreerd: controleer of een vertrouwde uitgever het heeft uitgegeven, of het vandaag geldig is en of het bedoeld is voor ondertekening.",
                registeredToOther:
                    "Dit certificaat is geregistreerd op naam van {{name}}. Als dit account ook van {{name}} is, koppel het dan als diens tweede account.",
                linkAccount: "Koppelen als tweede account van dezelfde persoon",
                alreadyRegistered: "Dit certificaat is al geregistreerd op naam van deze persoon.",
                pem: "Certificaat (PEM)",
                revoke: "Intrekken",
                revokeOf: "Het certificaat van {{name}} intrekken",
                revokeTitle: "Het certificaat van {{name}} intrekken",
                revokeHelp:
                    "Een ingetrokken certificaat kan niet meer ondertekenen. Handtekeningen die het al heeft gezet, blijven geldig.",
                revokeReason: "Reden",
                revokeDone: "Het certificaat is ingetrokken.",
                revokeError: "Het certificaat kon niet worden ingetrokken.",
            },
            requests: {
                exportCsv: "CSV exporteren",
                exportError: "De verzoeken konden niet worden geëxporteerd.",
                exportFileName: "signing-requests.csv",
                status: "Status",
                statusAll: "Alle",
                statusCount: "{{status}} · {{count}} van {{total}}",
                expires: "Verloopt {{time}}",
                lastSignatureBy: "{{name}}, {{time}}",
                empty: "Nog geen ondertekeningsverzoeken.",
                columns: {
                    request: "Verzoek",
                    status: "Status",
                    started: "Gestart",
                    by: "Door",
                    lastSignature: "Laatste handtekening",
                    code: "Code",
                },
            },
            reports: {
                postRequired:
                    "Selecteer een post om dit rapport te genereren wanneer handtekeningen vereist zijn.",
                generateNotice:
                    "{{post}}: het document wordt nu gegenereerd. Het kan worden afgedrukt en verzonden zodra {{n}} personen het hebben ondertekend.",
            },
            status: {
                waiting: "Wachtend",
                completed: "Ondertekend",
                executed: "Uitgevoerd",
                cancelled: "Geannuleerd",
                expired: "Verlopen",
                failed: "Mislukt",
            },
            cancelReasons: {
                "by-requester": "De persoon die het startte, heeft het geannuleerd",
                "by-operator": "Een operator heeft het geannuleerd",
                "rule-changed": "De ondertekeningsregel van de actie is gewijzigd",
                "payload-changed": "Wat het ondertekent, is gewijzigd",
                "superseded": "Een nieuwer verzoek heeft het vervangen",
                "certificate-revoked": "Een certificaat waarmee het is ondertekend, is ingetrokken",
            },
            panel: {
                rulePost:
                    "Vereist {{n}} handtekeningen van de ondertekenaars van {{post}}, elk met hun digitale certificaat.",
                ruleEvent:
                    "Vereist {{n}} handtekeningen, elk met het digitale certificaat van de ondertekenaar.",
                signingCode: "Ondertekeningscode",
                signers: "Ondertekenaars",
                sign: "Ondertekenen",
                handover: "Volgend lid logt in",
                cancel: "Verzoek annuleren",
                signedAt: "Ondertekend {{time}}",
                notSigned: "Niet ondertekend",
                certificate: "Certificaat {{name}}",
                you: "(u)",
                expiresAt: "Verloopt om {{time}}",
                progress: "{{count}} van {{total}}",
                openDocument: "Het document openen",
                configurationVersion: "Configuratieversie {{version}}",
                configurationChanges: "Wijzigingen in deze versie",
            },
            dialog: {
                title: "Ondertekenen: {{object}}",
                steps: {
                    check: "Controleren",
                    certificate: "Certificaat",
                    signed: "Ondertekend",
                },
                localNote:
                    "Het ondertekenen gebeurt in deze browser. Uw certificaatbestand, de privésleutel en het wachtwoord worden nooit verzonden. Alleen uw handtekening en uw openbare certificaat gaan naar de server.",
                check: {
                    signingAs: "U ondertekent als {{name}}",
                    titlePost: "{{title}}, {{post}}",
                    sameCode: "Iedereen die ondertekent, ziet dezelfde code.",
                    confirmDocument: "Ik heb gecontroleerd wat ik onderteken: {{object}}",
                },
                certificate: {
                    intro: "Plaats uw beveiligingstoken en kies uw certificaatbestand.",
                    password: "Certificaatwachtwoord",
                    open: "Certificaat openen",
                    chooseAnother: "Een ander bestand kiezen",
                },
                checks: {
                    "passed": {
                        "trusted-issuer": "Uitgegeven door een vertrouwde uitgever ({{root}})",
                        "valid-now": "Vandaag geldig",
                        "signing-key-usage": "Bedoeld voor ondertekening",
                        "not-revoked": "Niet ingetrokken (lijsten bijgewerkt {{time}})",
                        "registered": "Op {{date}} op uw naam geregistreerd",
                        "registered-to-other": "Niet op naam van iemand anders geregistreerd",
                        "already-signed": "Nog niet gebruikt voor dit verzoek",
                        "post-binding": "Geregistreerd voor deze $t(signing.terms.post)",
                        "signature": "De handtekening dekt dit verzoek",
                    },
                    "failed": {
                        "trusted-issuer": "Niet uitgegeven door een vertrouwde uitgever",
                        "valid-now": "Vandaag niet geldig",
                        "signing-key-usage": "Niet bedoeld voor ondertekening",
                        "not-revoked":
                            "Ingetrokken, of geen actuele intrekkingslijst om het te controleren",
                        "registered": "Niet op uw naam geregistreerd",
                        "registered-to-other": "Geregistreerd op naam van {{name}}",
                        "already-signed": "Al gebruikt voor dit verzoek",
                        "post-binding": "Geregistreerd voor een andere $t(signing.terms.post)",
                        "signature": "De handtekening dekt dit verzoek niet",
                    },
                    "first-use": "Eerste gebruik: het wordt op uw naam geregistreerd",
                },
                problems: {
                    wrongPassword: "Onjuist wachtwoord. Controleer het en probeer het opnieuw.",
                    notForYou:
                        "Dit certificaat kan niet voor u ondertekenen. Gebruik het certificaat op uw eigen beveiligingstoken.",
                    issuerNotAccepted:
                        "Gebruik het certificaat dat {{organization}} voor u heeft geregistreerd. Certificaten van andere uitgevers worden niet geaccepteerd.",
                    cancelled:
                        "Dit verzoek is geannuleerd: {{reason}}. Handtekeningen die ervoor zijn gezet, tellen niet meer. Start het opnieuw om de huidige versie te ondertekenen.",
                },
                signed: {
                    title: "Ondertekend",
                    withCertificate: "met het certificaat van {{name}}",
                    count: "{{n}} van {{total}} handtekeningen.",
                    allIn: "Alle {{total}} handtekeningen zijn binnen.",
                    next: "Volgende ondertekenaars: {{names}}.",
                },
                handover:
                    "U wordt uitgelogd. Het volgende lid logt in op deze computer en keert terug naar dit verzoek om te ondertekenen. Het verzoek blijft open tot {{time}}.",
                sign: "Ondertekenen",
                back: "Terug",
                cancel: "Annuleren",
            },
            widget: {
                continue: "Doorgaan",
                done: "Klaar",
                close: "Sluiten",
                retry: "Opnieuw proberen",
                loading: "Het verzoek wordt geladen…",
                loadError: "Het verzoek kon niet worden geladen.",
                chooseFile: "Certificaatbestand kiezen",
                fileInput: "Certificaatbestand",
                fileSize: "{{size}} KB",
                showPassword: "Wachtwoord tonen",
                hidePassword: "Wachtwoord verbergen",
                opening: "Het certificaat wordt geopend…",
                checking: "Het certificaat wordt gecontroleerd…",
                signing: "Bezig met ondertekenen…",
                certificateCard: "Uitgegeven door {{issuer}} · geldig tot {{date}} · {{algorithm}}",
                fingerprint: "SHA-256 {{fingerprint}}",
                algorithms: {
                    "rsa-pkcs1-sha256": "RSA",
                    "ecdsa-p256-sha256": "EC P-256",
                },
                document: "{{type}} · SHA-256 {{hash}}",
                documentPages: "{{type}} · {{pages}} pagina's · SHA-256 {{hash}}",
                checksTitle: "Certificaatcontroles",
                untrustedIssuer:
                    "{{issuer}} is geen vertrouwde uitgever voor dit verkiezingsevenement",
                registeredToSomeoneElse: "Geregistreerd op naam van iemand anders",
                checkPassedNoDetail: {
                    "trusted-issuer": "Uitgegeven door een vertrouwde uitgever",
                    "not-revoked": "Niet ingetrokken",
                },
                organization: "uw organisatie",
                cantSign: "Dit certificaat kan dit verzoek niet ondertekenen.",
                checkError: "Het certificaat kon niet worden gecontroleerd. Probeer het opnieuw.",
                fileErrors: {
                    UNREADABLE_FILE:
                        "Dit bestand is geen certificaatbestand (.p12 of .pfx), of het is beschadigd.",
                    UNSUPPORTED_ENCRYPTION:
                        "Deze browser kan de versleuteling van dit bestand niet openen.",
                    NO_PRIVATE_KEY:
                        "Dit bestand bevat geen privésleutel. Kies het certificaatbestand van uw beveiligingstoken.",
                    NO_CERTIFICATE: "Dit bestand bevat geen certificaat.",
                    UNSUPPORTED_KEY:
                        "Het sleuteltype van dit certificaat wordt niet ondersteund. Gebruik een RSA- of EC P-256-certificaat.",
                    KEY_CERTIFICATE_MISMATCH:
                        "Het certificaat in dit bestand komt niet overeen met de sleutel.",
                },
                openError: "Het certificaat kon niet worden geopend. Probeer het opnieuw.",
                signError: "De handtekening kon niet worden verzonden. Probeer het opnieuw.",
                refused: "De server heeft de handtekening geweigerd.",
                stale: "Het document is gewijzigd terwijl u ondertekende. Onderteken opnieuw.",
                mismatch:
                    "Wat zou worden ondertekend, komt niet overeen met dit verzoek. Sluit het venster en open het verzoek opnieuw.",
                documentMismatch:
                    "Het document komt niet overeen met het document dat dit verzoek ondertekent.",
                documentError: "Het document kon niet worden gedownload. Probeer het opnieuw.",
                alreadySigned: "U hebt dit verzoek al ondertekend.",
                closed: {
                    changed:
                        "Dit verzoek is gewijzigd nadat u het opende. Sluit dit venster en controleer het opnieuw voordat u ondertekent.",
                    allSigned: "Dit verzoek heeft al alle handtekeningen.",
                },
                chooseCertificate: "Certificaat om mee te ondertekenen",
                renderError:
                    "Het ondertekeningsverzoek kon niet worden weergegeven. Sluit het en open het opnieuw.",
                signedAt: "{{time}}",
                panel: {
                    completedAt: "Ondertekend om {{time}}",
                    expired:
                        "Dit verzoek is verlopen. Handtekeningen die ervoor zijn gezet, tellen niet meer. Start het opnieuw om te ondertekenen.",
                    failed: "Alle handtekeningen zijn binnen, maar de actie is mislukt. Het logboek bevat de details.",
                    details: "Details",
                    close: "Het verzoekpaneel sluiten",
                },
                cancelDialog: {
                    title: "Dit verzoek annuleren?",
                    body: "Handtekeningen die ervoor zijn gezet, tellen niet meer. De persoon die het startte, begint opnieuw.",
                    reason: "Reden (optioneel)",
                    confirm: "Verzoek annuleren",
                    back: "Behouden",
                    error: "Het verzoek kon niet worden geannuleerd. Probeer het opnieuw.",
                },
                handoverDialog: {
                    title: "Volgend lid logt in",
                    noExpiry:
                        "U wordt uitgelogd. Het volgende lid logt in op deze computer en keert terug naar dit verzoek om te ondertekenen.",
                    confirm: "Uitloggen",
                    back: "Ingelogd blijven",
                    error: "De overdracht kon niet worden vastgelegd. Probeer het opnieuw.",
                },
            },
            details: {
                keys_ceremony_id: "Ceremonie",
                tally_session_id: "Telsessie",
                trustee_id: "Trustee",
                custody_proof_sha256: "SHA-256 van het bewijs van bewaring",
                channel: "Kanaal",
                channels: "Kanalen",
                publication_id: "Publicatie stembiljet",
                ballot_publication_id: "Publicatie stembiljet",
                digest: "SHA-256 van de configuratie",
                signing_rules: "Ondertekeningsregels",
                scheduled_events: "Nieuwe geplande gebeurtenissen",
                ballots_and_contests: "Stembiljetten en verkiezingen",
                application_id: "Aanvraag",
                applicant_registry_id: "Registeraccount",
                decision: "Besluit",
                submitted_at: "Ingediend",
                reason: "Waarom een persoon nodig is",
                registry_record: "Registerrecord",
                status: "Status van de aanvraag",
                from: "Vorige status",
            },
            closed: {
                pending: "Alle handtekeningen zijn binnen. De stemming sluit zo meteen.",
                title: "De stemming is gesloten om {{time}}.",
                titleSealed: "De stemming is gesloten om {{time}}. Stembiljetten verzegeld.",
                record: "Verzegelingsrecord",
                ballots: "Stembiljetten in de verzegeling",
                sealHash: "{{algorithm}} van de verzegeling",
                signedBy: "Ondertekend door",
                signatures: "Sluitingshandtekeningen in het verzegelingsrecord",
                signaturesValue_one: "{{count}}, ondertekeningscode {{code}}",
                signaturesValue_other: "{{count}}, ondertekeningscode {{code}}",
                signers: "Ondertekend door de leden",
            },
            values: {
                ballots_and_contests: {
                    "first-version": "Eerste versie",
                    "no-changes": "Geen wijzigingen",
                    "changed": "Gewijzigd",
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
                    EARLY_VOTING: "Vervroegd stemmen",
                    TELEPHONE: "Telefoon",
                },
                statuses: {
                    NOT_STARTED: "Niet gestart",
                    OPEN: "Open",
                    PAUSED: "Gepauzeerd",
                    CLOSED: "Gesloten",
                },
                channelStatus: "{{channel}}: {{status}}",
                ruleChange: "{{action}}: {{rule}}",
                ruleChangeFrom: "{{action}}: {{rule}} (was {{was}})",
                ruleNeeds: "vereist {{n}}",
                ruleOff: "uit",
                decision: {
                    approve: "Goedkeuren",
                },
            },
            results: {
                signatures: "Handtekeningen",
                needs: "Vereist {{n}}",
                off: "Uit",
                openRequest: "Het ondertekeningsverzoek openen",
                downloadSigned: "Ondertekende pdf downloaden",
                print: "Afdrukken",
                transmit: "Resultaten verzenden",
                sendTo: "Verzenden naar {{count}} bestemmingen",
                awaiting: "{{item}}: wacht op handtekeningen",
                transmission: {
                    title: "Handtekeningen",
                    description:
                        "Elke ondertekenaar ondertekent de resultaten van het pakket met zijn digitale certificaat, in deze browser. Het pakket kan worden verzonden zodra {{n}} personen het hebben ondertekend.",
                    waiting:
                        "Het pakket kan worden verzonden zodra het ondertekeningsverzoek alle handtekeningen heeft.",
                    signed: "Het pakket bevat alle handtekeningen en kan worden verzonden.",
                    ended: "Het ondertekeningsverzoek van dit pakket is beëindigd. Maak het pakket opnieuw aan om het te ondertekenen.",
                },
            },
            waiting: {
                title: "Wacht op mijn handtekening",
                buttonCount_one: "Wacht op mijn handtekening: {{count}} verzoek te ondertekenen",
                buttonCount_other:
                    "Wacht op mijn handtekening: {{count}} verzoeken te ondertekenen",
                intro: "Verzoeken die wachten op handtekeningen voor de acties die u mag ondertekenen, in uw $t(signing.terms.posts).",
                close: "De lijst sluiten",
                empty: "Er wacht niets op uw handtekening.",
                loadError:
                    "De verzoeken die op handtekeningen wachten, konden niet worden geladen.",
                signedByYou: "Door u ondertekend",
            },
            notes: {
                afterApproval: "Na goedkeuring",
                afterApprovalValue: "De inloggegevens van de kiezer worden uitgegeven en verzonden",
                custodyProof: "Uw bewijs van bewaring",
                custodyProofChecked:
                    "Gecontroleerd: het is uw bewijs van bewaring voor deze ceremonie",
                recordedIn: "Vastgelegd in",
                recordedInCeremony: "De sleutelceremonie en het bulletinboard",
                recordedInTally: "De telsessie",
            },
            custodyProof: {
                signing:
                    "Onderteken uw bewijs van bewaring in het ondertekeningspaneel. Het wordt vastgelegd zodra u het hebt ondertekend.",
                record: "Mijn bewijs van bewaring vastleggen",
                failed: "Uw ondertekende bewijs van bewaring kon niet worden vastgelegd: {{error}}",
                dropAgain:
                    "Sleep uw bewijs van bewaringbestand opnieuw hierheen om uw ondertekende bewijs van bewaring vast te leggen. (signed-custody-proof.json)",
                redo: "Uw bewijs van bewaring is bijgedragen zonder uw handtekening, die deze verkiezing nu vereist. Draag het opnieuw bij en onderteken het.",
                notTaken:
                    "De ceremonie accepteert dit bewijs van bewaring niet meer. Sleep uw bewijs van bewaringbestand opnieuw hierheen. (signed-custody-proof.json)",
            },
        },
        lifecycle: {
            signedClose: {
                title: "Ondertekende sluitingstijd",
                deadline: "{{election}}: {{time}} · geautoriseerd door configuratie {{code}}.",
                explanation:
                    "Deze ondertekende sluitingstijd blijft bindend als het bewerkbare schema wordt gewijzigd of verwijderd. De planner sluit de geautoriseerde kanalen die nog open zijn.",
                reached:
                    "Deze ondertekende sluitingstijd is verstreken. Controleer de huidige stemstatus en het auditlog; de verwerking is nog niet geregistreerd.",
                processed: "Ondertekende sluitingstijd verwerkt om {{time}}.",
                signedAt: "Ondertekende sluitingstijd: {{time}}.",
                channels: "Kanalen die nog onder deze sluitingstijd vallen: {{channels}}.",
                result: "Raadpleeg de stemstatus en het auditlog voor de daadwerkelijke wijzigingen en het volledige resultaat.",
                unavailable:
                    "Ondertekende sluitingstijden konden niet worden geladen. Controleer het gepubliceerde schema en het auditlog.",
            },
            picker: {
                noMatch:
                    "Geen tijdzone gevonden. Typ een stad, land, zone, afkorting of verschuiving.",
            },
            input: {
                timezone: "Tijdzone",
                scheduledAt: "Gepland op",
                meetingStart: "Begin van de vergadering",
                cronZone: "De planning loopt in de primaire tijdzone van het evenement, {{zone}}.",
                unconfiguredZone:
                    "{{zone}} is geen van de geconfigureerde tijdzones van het evenement. Kies er een.",
            },
            schedule: {
                allElections: "Alle verkiezingen",
                outcome: "Uitkomst",
                noOffset: "Geen tijdzoneverschuiving: wordt nooit uitgevoerd",
                unpublished: "Nog niet gepubliceerd",
                notPublished:
                    "Er is nog niets gepubliceerd: kiezers zien de planning na de eerste publicatie.",
                unpublishedChanges_one:
                    "{{count}} gepland evenement is gewijzigd sinds de laatste publicatie. Kiezers zien het nadat u publiceert.",
                unpublishedChanges_other:
                    "{{count}} geplande evenementen zijn gewijzigd sinds de laatste publicatie. Kiezers zien ze nadat u publiceert.",
                offsetless_one:
                    "{{count}} gepland tijdstip heeft geen tijdzoneverschuiving en wordt dus nooit uitgevoerd. Bewerk het om de tijdzone in te stellen.",
                offsetless_other:
                    "{{count}} geplande tijdstippen hebben geen tijdzoneverschuiving en worden dus nooit uitgevoerd. Bewerk ze om hun tijdzone in te stellen.",
                outcomeChange:
                    "Opslaan wijzigt wat deze geplande overgang doet: {{before}} → {{after}}.",
                outcomeNew: "Na opslaan doet deze geplande overgang het volgende: {{after}}.",
                outcomeElections: "{{count}} van {{total}} verkiezingen",
                exportError: "De planning kon niet worden geëxporteerd.",
                exportFileName: "schedule.csv",
                totals: {
                    refused_one:
                        "{{count}} geplande rij wordt geweigerd ({{transitions}} verkiezingsovergangen).",
                    refused_other:
                        "{{count}} geplande rijen worden geweigerd ({{transitions}} verkiezingsovergangen).",
                    runsUnsigned_one:
                        "{{count}} geplande sluiting wordt zonder handtekeningen uitgevoerd ({{transitions}} verkiezingsovergangen).",
                    runsUnsigned_other:
                        "{{count}} geplande sluitingen worden zonder handtekeningen uitgevoerd ({{transitions}} verkiezingsovergangen).",
                    review: "Bekijken",
                    showAll: "Alles tonen",
                    showing: {
                        refused:
                            "De {{count}} geplande rijen die worden geweigerd ({{transitions}} verkiezingsovergangen).",
                        runsUnsigned:
                            "De {{count}} geplande sluitingen die zonder handtekeningen worden uitgevoerd ({{transitions}} verkiezingsovergangen).",
                    },
                },
                recompute: {
                    title_one:
                        "Een update van de tijdzonedatabase verschuift {{count}} toekomstig gepland tijdstip. Er verandert niets totdat u het toepast.",
                    title_other:
                        "Een update van de tijdzonedatabase verschuift {{count}} toekomstige geplande tijdstippen. Er verandert niets totdat u ze toepast.",
                    change: "{{type}}: {{before}} → {{after}}",
                    apply: "Toepassen",
                    applied_one: "{{count}} gepland tijdstip bijgewerkt.",
                    applied_other: "{{count}} geplande tijdstippen bijgewerkt.",
                    error: "De geplande tijdstippen konden niet worden bijgewerkt.",
                },
                outcomeChangeElections_one: "Opslaan wijzigt de uitkomst bij {{count}} verkiezing:",
                outcomeChangeElections_other:
                    "Opslaan wijzigt de uitkomst bij {{count}} verkiezingen:",
            },
            authorizes: {
                reportPolicyOf: "{{election}}: {{value}}",
                initializationRetained:
                    "Een verplicht rapport in deze ondertekende configuratie blijft verplicht als de huidige instelling van de post wordt gewijzigd naar niet verplicht.",
                title: "Wat deze goedkeuring toestaat",
                schedule: "Geplande openingen en sluitingen",
                noSchedule:
                    "Geen geplande openingen of sluitingen: de ondertekenaars openen en sluiten de stemming.",
                opens: "Opent {{time}}",
                closes: "Sluit {{time}}",
                settings: "Instellingen",
                unsignedClose: "Geplande sluiting zonder handtekeningen: {{value}}",
                initialization: "Initialisatie: {{value}}",
                firstConfiguration:
                    "Dit is de eerste goedgekeurde configuratie: er is niets om mee te vergelijken.",
                sameAsPrevious:
                    "De instellingen zijn dezelfde als in de vorige goedgekeurde configuratie.",
                rule: {
                    openNeeds_one: "Openen vereist {{count}} handtekening",
                    openNeeds_other: "Openen vereist {{count}} handtekeningen",
                    openNoSignatures: "Openen vereist geen handtekeningen",
                    closeNeeds_one: "Sluiten vereist {{count}} handtekening",
                    closeNeeds_other: "Sluiten vereist {{count}} handtekeningen",
                    closeNoSignatures: "Sluiten vereist geen handtekeningen",
                    openSetting: "Stemming openen",
                    closeSetting: "Stemming sluiten",
                    signatures_one: "{{count}} handtekening",
                    signatures_other: "{{count}} handtekeningen",
                    none: "geen handtekeningen",
                },
                diff: {
                    tightens: "Strenger: {{setting}} {{before}} → {{after}}",
                    loosens: "Soepeler: {{setting}} {{before}} → {{after}}",
                    mixed: "Wijzigt: {{setting}} {{before}} → {{after}} (op het ene punt strenger, op het andere soepeler)",
                },
                comparedWith:
                    "Vergeleken met de vorige goedgekeurde configuratie, goedkeuring {{code}}:",
                channels: "Stemkanalen per verkiezing",
                channelsOf: "{{election}}: {{channels}}",
                noChannels: "geen",
            },
            publish: {
                openedAuthorized:
                    "Stemming volgens planning geopend om {{time}}, toegestaan door configuratiegoedkeuring {{code}} (ondertekend door {{names}}).",
                closedAuthorized:
                    "Stemming volgens planning gesloten om {{time}}, toegestaan door configuratiegoedkeuring {{code}} (ondertekend door {{names}}).",
                closedUnsigned:
                    "Stemming volgens planning gesloten om {{time}}. Geen sluitingshandtekeningen: de planning heeft de stemming op de deadline gesloten.",
                authorizedBy: "Toegestaan door",
                cancelledRequest:
                    "Verzoek {{code}} had {{n}} van {{k}} handtekeningen en is geannuleerd.",
                openedRefused: "De geplande opening van {{time}} is geweigerd.",
                closedRefused: "De geplande sluiting van {{time}} is geweigerd.",
                openedNoSignaturesNeeded:
                    "De stemming is volgens planning geopend ({{time}}); er waren geen handtekeningen nodig.",
                closedNoSignaturesNeeded:
                    "De stemming is volgens planning gesloten ({{time}}); er waren geen handtekeningen nodig.",
                openedNothingToChange:
                    "Om {{time}} had de geplande opening niets te openen: de kanalen waren al open.",
                closedNothingToChange:
                    "Om {{time}} had de geplande sluiting niets te sluiten: de kanalen waren al gesloten.",
            },
            import: {
                title: "Planning importeren",
                subtitle:
                    "Eén rij per evenement en verkiezing, in lokale tijd. Laat de tijdzone leeg om de tijdzone van de verkiezing te gebruiken.",
                chooseFile: "Kies een CSV-bestand",
                template: "Sjabloon downloaden",
                templateFileName: "schedule-template.csv",
                ready: "{{ok}} evenementen klaar voor {{posts}} verkiezingen.",
                needsAttention_one:
                    "{{ok}} evenementen klaar voor {{posts}} verkiezingen. {{count}} rij vereist aandacht; corrigeer het bestand en upload het opnieuw.",
                needsAttention_other:
                    "{{ok}} evenementen klaar voor {{posts}} verkiezingen. {{count}} rijen vereisen aandacht; corrigeer het bestand en upload het opnieuw.",
                preview: "Te importeren rijen",
                row: "Rij",
                asWritten: "{{local}} · {{place}}",
                moreRows: "…en nog {{count}} rijen",
                imported: "Planning geïmporteerd: {{created}} aangemaakt, {{updated}} bijgewerkt.",
                uploadError: "Het bestand kon niet worden gecontroleerd. Upload het opnieuw.",
                importError: "De planning kon niet worden geïmporteerd.",
                error: {
                    unknownElection: "Geen enkele verkiezing heeft de alias {{election}}.",
                    unknownEventType: "{{type}} is geen type gepland evenement.",
                    invalidTimeZone: "{{zone}} is geen tijdzone.",
                    invalidDateTime: "De datum en tijd moeten de vorm YYYY-MM-DDTHH:MM hebben.",
                    invalidVotingChannels:
                        "De stemkanalen zijn onbekend, of openen online en vervroegd stemmen tegelijk.",
                    dstGap: "{{dateTime}} bestaat niet in {{city}} omdat de klok vooruit gaat. Vul een tijd in die bestaat.",
                    duplicate: "Een andere rij plant hetzelfde evenement voor deze verkiezing.",
                    other: "Deze rij kan niet worden geïmporteerd ({{code}}).",
                    ambiguousElection: "Meer dan één verkiezing heeft de alias {{election}}.",
                },
            },
            settings: {
                accordion: "Taal, datum en tijd",
                dateAndTime: "Datum en tijd",
                configured: "Geconfigureerde tijdzones",
                configuredHelp:
                    "{{count}} tijdzones. Verkiezingen kiezen hun tijdzone uit deze lijst; typ een stad of land om er een toe te voegen.",
                moreZones: "+{{count}}",
                primary: "Primaire tijdzone",
                primaryHelp:
                    "Gebruikt voor planningen van het hele evenement, rapporten en verkiezingen zonder eigen tijdzone.",
                primaryInUse:
                    "{{zone}} is de primaire tijdzone. Kies eerst een andere primaire tijdzone.",
                inUse: "{{zone}} wordt gebruikt door {{names}}. Wijzig eerst die verkiezingen.",
                logs: "Tijden in logboeken en logboekexports",
                logsPrimary: "Primaire tijdzone ({{abbr}})",
                logsElection: "De tijdzone van de verkiezing van elke rij",
                logsHelp: "Rijen zonder verkiezing gebruiken de primaire tijdzone.",
                electionZone: "Tijdzone",
                electionPrimary: "Primair voor het evenement: {{zone}}",
                electionZoneHelp:
                    "Planningen, kiezersschermen en rapporten van deze verkiezing gebruiken deze tijdzone, ook voor elk gebied eronder. Leeg gebruikt de primaire tijdzone van het evenement.",
                electionUnconfigured:
                    "Het evenement configureert deze tijdzone niet meer, dus de verkiezing gebruikt de primaire tijdzone, {{zone}}. Kies een van de geconfigureerde tijdzones.",
                electionUnconfiguredSave:
                    "Kies een van de geconfigureerde tijdzones van het evenement.",
            },
            policies: {
                accordion: "Levenscyclus van de stemming",
                intro: "Deze instellingen maken deel uit van de configuratie van het verkiezingsevenement: de configuratiegoedkeuring ondertekent ze, en geplande openingen en sluitingen volgen de strengste van de huidige en de gepubliceerde instellingen.",
                nothingPublished:
                    "Nog niets gepubliceerd: tot de eerste publicatie gebruiken geplande openingen en sluitingen de standaardwaarden (per verkiezing, weigeren).",
                publishedValue: "Gepubliceerde configuratie: {{value}}",
                changedSincePublished:
                    "Gewijzigd sinds de gepubliceerde configuratie: geplande openingen en sluitingen volgen de strengste van de twee tot de volgende goedgekeurde publicatie.",
                scope: {
                    title: "Initialisatie voordat de stemming opent",
                    post: {
                        label: "Per verkiezing",
                        help: "Een verkiezing opent zodra ze is geïnitialiseerd.",
                    },
                    event: {
                        label: "Hele evenement",
                        help: "Geen enkele verkiezing opent totdat elke verkiezing is geïnitialiseerd.",
                        warning:
                            "Eén verkiezing die niet is geïnitialiseerd, houdt alle verkiezingen gesloten, ook op hun geplande opening.",
                    },
                    postAndCountry: {
                        label: "Per verkiezing en land",
                        help: "Een verkiezing opent zodra elk land (gebied) eronder is geïnitialiseerd.",
                        warning:
                            "Een verkiezing blijft gesloten, ook op haar geplande opening, totdat elk land eronder is geïnitialiseerd; elk land wordt met een eigen rapport geïnitialiseerd.",
                    },
                },
                close: {
                    title: "Geplande sluiting zonder handtekeningen",
                    help: "Wanneer het sluiten van de stemming handtekeningen vereist en een geplande sluiting niet in de ondertekende configuratie staat.",
                    refuse: {
                        label: "Weigeren",
                        help: "De sluiting wordt niet uitgevoerd; de ondertekenaars van de verkiezing sluiten de stemming met hun handtekeningen.",
                    },
                    runAsSystem: {
                        label: "Uitvoeren als systeem",
                        help: "De stemming sluit op de deadline en wordt geregistreerd als gesloten door de planning, zonder handtekeningen.",
                        warning:
                            "Geplande sluitingen buiten de ondertekende configuratie sluiten de stemming zonder handtekening van wie dan ook. Het logboek en de documenten vermelden dit.",
                    },
                },
                onSave: {
                    outcomes_zero: "Geen enkele geplande overgang wijzigt van uitkomst.",
                    outcomes_one:
                        "{{count}} geplande overgang wijzigt van uitkomst. Bekijk deze in Geplande Gebeurtenissen.",
                    outcomes_other:
                        "{{count}} geplande overgangen wijzigen van uitkomst. Bekijk ze in Geplande Gebeurtenissen.",
                },
                saveError:
                    "De instellingen voor de levenscyclus van de stemming konden niet worden opgeslagen.",
                publishedPerTarget: "Gepubliceerde configuratie, per doel: {{values}}",
                publishedCount_one: "{{value}} ({{count}} doel)",
                publishedCount_other: "{{value}} ({{count}} doelen)",
                savedWithoutPolicies:
                    "Het verkiezingsevenement is opgeslagen, maar de instellingen van de stemlevenscyclus niet: {{reason}}. Sla ze opnieuw op.",
            },
        },
        scheduledOutcome: {
            chip: {
                waitingForInitialization: "Wacht op initialisatie",
                runs: "Wordt uitgevoerd",
                runsUnsigned: "Wordt zonder handtekeningen uitgevoerd",
                refused: "Wordt geweigerd",
            },
            note: {
                waitingForInitialization: "Wacht op initialisatie",
                authorized: "Toegestaan door configuratie {{code}}",
                noSignaturesNeeded: "Geen handtekeningen nodig",
                closesUnsigned: "Sluit zonder handtekeningen",
                refused: {
                    initialization: "De vereiste initialisatie is niet voltooid",
                    votingClose: "Stemmen kan niet openen na de sluitingstijd",
                    needsSignatures: "Heeft de handtekeningen van de ondertekenaars nodig",
                    covered: "Niet in de ondertekende configuratie",
                    unsignedClose: "Een sluiting zonder handtekeningen wordt geweigerd",
                    stricterCopy: "Gewijzigd sinds de gepubliceerde configuratie, die nog beslist",
                    defaults: "Nog niets gepubliceerd: de standaardwaarden gelden",
                },
                refusedWithStep: "{{reason}}. {{next}}",
            },
            why: {
                button: "Waarom?",
                title: {
                    waitingForInitialization: "Waarom op initialisatie wordt gewacht",
                    runs: "Waarom deze wordt uitgevoerd",
                    runsUnsigned: "Waarom deze zonder handtekeningen wordt uitgevoerd",
                    refused: "Waarom deze wordt geweigerd",
                },
                checks: "Controles",
                check: "Controle",
                current: "Huidige instellingen",
                published: "Gepubliceerde configuratie",
                verdict: "Oordeel",
                allows: "Staat toe",
                blocks: "Blokkeert",
                deciding: "Beslissende controle",
                nextStep: "Volgende stap:",
                signedBy: "Ondertekend door {{names}}",
            },
            question: {
                initialization: "Is de vereiste initialisatie voltooid?",
                votingClose: "Respecteert deze opening de sluitingstijd van het stemmen?",
                needsSignatures: "Vereist deze actie handtekeningen?",
                covered: "Staat precies deze planning in de ondertekende configuratie?",
                unsignedClose: "Wat gebeurt er met een sluiting zonder handtekeningen?",
                stricterCopy:
                    "Verschillen de huidige en de gepubliceerde instellingen? Welke beslist?",
                defaults: "Is er al iets gepubliceerd?",
            },
            check: {
                initialization: {
                    waiting:
                        "De initialisaties die de huidige en gepubliceerde instellingen vereisen, moeten beide voltooid zijn.",
                },
                votingClose: {
                    passed: "Stemmen sluit om {{closes_at}}; deze opening kan niet op of na die sluitingstijd worden uitgevoerd.",
                },
                needsSignatures: {
                    yes: "Ja, {{signatures}} handtekeningen",
                    yes_one: "Ja, {{count}} handtekening",
                    yes_other: "Ja, {{count}} handtekeningen",
                    no: "Nee",
                },
                covered: {
                    overriddenBySignedPostRow:
                        "De ondertekende configuratie {{code}} gebruikt de eigen opening {{scheduled_event_id}} van deze post. De opening voor het hele evenement is niet van toepassing.",
                    yes: "Ja: goedkeuring {{code}}, ongewijzigd",
                    changed: "Nee: gewijzigd sinds goedkeuring {{code}}",
                    changedBy:
                        "Nee: bewerkt op {{edited_at}} door {{edited_by}}, na goedkeuring {{code}}",
                    notInApproval: "Nee: goedkeuring {{code}} bevat dit niet",
                    noApproval: "Nog geen goedgekeurde configuratie",
                    channelsChanged:
                        "Nee: de stemkanalen van de verkiezing zijn gewijzigd sinds goedkeuring {{code}}",
                    alreadyFired:
                        "Nee: deze overgang van goedkeuring {{code}} is al uitgevoerd op {{fired_at}}; opnieuw uitvoeren vereist handtekeningen",
                    late: "Nee: het is meer dan 15 minuten na {{scheduled_date}} (goedkeuring {{code}}); nu uitvoeren vereist handtekeningen",
                },
                unsignedClose: {
                    refuse: "Weigeren",
                    runAsSystem: "Uitvoeren als systeem",
                },
                stricterCopy: {
                    same: "Beide zijn hetzelfde",
                    currentStricter: "De huidige instellingen zijn strenger: nu toegepast",
                    currentLooser:
                        "De huidige instellingen zijn soepeler: ze gelden na de volgende goedgekeurde publicatie",
                    combined: "Elk is strenger in één waarde: beide gelden",
                },
                defaults: {
                    published: "Gepubliceerd op {{published_at}}",
                    nothingPublished: "Niets gepubliceerd: de standaardwaarden gelden",
                    noSnapshot:
                        "Gepubliceerd op {{published_at}}, voordat publicaties deze instellingen bewaarden: de standaardwaarden gelden",
                },
            },
            nextStep: {
                initialize:
                    "Voltooi de vereiste initialisatie. De planner probeert het opnieuw voordat het stemmen sluit.",
                closed: "Deze opening wordt niet uitgevoerd nadat het stemmen sluit.",
                none: "Geen actie nodig.",
                publishAndApprove: "Publiceer de configuratie en keur deze goed.",
                requireConfigurationApproval:
                    "Laat Configuratie goedkeuren handtekeningen vereisen, publiceer daarna de configuratie en keur deze goed.",
                askSignersToOpen:
                    "Vraag de ondertekenaars van de verkiezing om de stemming te openen.",
                askSignersToClose:
                    "Vraag de ondertekenaars van de verkiezing om de stemming te sluiten.",
            },
            applies: {
                tightens: "Geldt nu voor handmatige en geplande acties.",
                loosens:
                    "Geldt nu voor handmatige acties; voor geplande openingen en sluitingen na de volgende goedgekeurde publicatie.",
                tightensAndLoosens:
                    "Het strengere deel geldt nu voor handmatige en geplande acties; het soepelere deel geldt nu voor handmatige acties, en voor geplande openingen en sluitingen na de volgende goedgekeurde publicatie.",
            },
        },
        messagingEvent: {
            tab: "Berichten",
            intro: "De kanalen die kiezers van dit evenement kunnen kiezen voor codes en berichten, en het account waarvandaan elk kanaal verzendt. Accounts beheert u in Instellingen > Berichten.",
            readOnly:
                "U kunt deze instellingen bekijken. Wijzigen vereist de machtiging messaging-config-write.",
            savingNote:
                "Opslaan werkt ook de kanalen bij die de inschrijvingspagina's per Post aanbieden.",
            save: "Opslaan",
            saved: "Berichteninstellingen opgeslagen.",
            saveRejected:
                "De berichteninstellingen zijn niet opgeslagen. Los de getoonde problemen op.",
            saveError: "De berichteninstellingen konden niet worden opgeslagen.",
            accountLabel: "{{channel}}-account",
            notUsed: "Niet gebruikt",
            missingAccount: "Account niet gevonden",
            noAccount: "Voeg eerst een account toe in Instellingen > Berichten",
            missing: "Ontbreekt: {{blockers}}",
            purposeSwitch: "{{channel}}: {{purpose}}",
            sections: {
                channels: "Kanalen",
                templates: "Goedgekeurde sjablonen",
                posts: "Kanalen per Post",
                postsCount: "Kanalen per Post ({{count}} Posts)",
                delivery: "Afleverstatus",
            },
            column: {
                channel: "Kanaal",
                account: "Verzendt vanaf",
                purpose: "Doel",
                language: "Taal",
                template: "Sjabloon van de provider",
                status: "Status",
                post: "Post",
                key: "Berichttype of sjabloon",
                providerLanguage: "Taal van de provider",
                voters: "Kiezers die het gebruiken",
                sent24h: "Verzonden, laatste 24 u",
                sent24hHelp:
                    "Elk bericht waarvan de laatste poging in de afgelopen 24 uur was, in elke status, ook mislukte. De statuskolommen tellen alle berichten van het evenement.",
            },
            outOfWindow: {
                utility:
                    "Buiten het venster van 24 uur verzendt {{channel}} berichten met de goedgekeurde hulpprogrammasjabloon die onder Goedgekeurde sjablonen aan berichten is gekoppeld.",
            },
            messageTypes: {
                otp: "Verificatiecode",
                otl: "Aanmeldlink",
                enrollmentAccepted: "Registratie geaccepteerd",
                enrollmentAcceptedKiosk: "Registratie geaccepteerd (kiosk)",
                enrollmentPending: "Registratie wacht op goedkeuring",
                enrollmentRejected: "Registratie afgewezen",
                passwordReset: "Wachtwoord herstellen",
                temporaryPassword: "Tijdelijk wachtwoord",
                contactChanged: "Contactmethode gewijzigd",
            },
            templates: {
                empty: "Kies een account dat goedgekeurde sjablonen verzendt, zoals WhatsApp, Viber of Messenger, om de sjablonen hier te koppelen.",
                help: "Kies een systeemberichttype of een alias uit Sjablonen. Laat dit leeg om het goedgekeurde sjabloon standaard voor dit doel te gebruiken. Taal is de taal van de kiezer. Sjabloon van de provider is de naam of ID van de sjabloon bij de provider. Taal van de provider is de code van de provider voor die sjabloon wanneer die afwijkt van de taal van de kiezer: WhatsApp heeft de exacte code van de goedgekeurde sjabloon nodig, zoals en_US.",
                order: "Voor elk bericht wint de meest specifieke rij: de rij voor het bericht in de taal van de kiezer, dan de rij voor het bericht in een willekeurige taal, dan de standaard voor het doel in de taal van de kiezer, dan een willekeurige standaard voor het doel.",
                noneRequired:
                    "{{channel}} verzendt alleen goedgekeurde sjablonen. Voeg minstens een standaardsjabloon toe voor elk doel dat in gebruik is.",
                noneOptional:
                    "Er zijn geen sjablonen gekoppeld voor {{channel}}. Ze zijn alleen nodig om berichten buiten het gespreksvenster te verzenden.",
                row: "{{channel}}-sjabloon {{position}}",
                keyDefault: "Standaard voor dit doel",
                add: "{{channel}}-sjabloon toevoegen",
                remove: "{{channel}}-sjabloon {{position}} verwijderen",
                incomplete:
                    "Voer de taal en de sjabloon van de provider in, of verwijder deze rij.",
                approval: {
                    APPROVED: "Goedgekeurd",
                    NOT_APPROVED: "Niet goedgekeurd",
                    NOT_CHECKED: "Goedkeuring niet gecontroleerd",
                },
            },
            posts: {
                noChannels:
                    "Zet codes of berichten aan voor een kanaal om de kanalen per Post te kiezen.",
                help: "De inschrijving toont de kiezers van elke Post de kanalen die hier zijn aangevinkt.",
                restricted: "{{count}} Posts bieden minder dan alle {{total}} kanalen aan.",
                allChannels:
                    "Elke Post biedt alle {{total}} kanalen aan; vink een kanaal uit voor een Post waar het niet werkt.",
                search: "Posts zoeken",
                cell: "{{post}}: {{channel}}",
                showing: "{{shown}} van {{total}} Posts getoond. Zoek om andere te vinden.",
            },
            delivery: {
                notConfigured: "Niet aangeboden door dit evenement",
                empty: "Er wordt geen kanaal gebruikt.",
                help: "De tellingen zijn per bericht, volgens de status van de laatste poging. Geaccepteerd betekent dat de provider het bericht heeft geaccepteerd, niet dat de kiezer het heeft ontvangen. Onbekend betekent dat de aflevering nog niet is bevestigd; na 24 uur wordt dit getoond als Onopgelost, nooit als mislukt. Een account zonder afleverrapporten toont de aflevering als niet beschikbaar.",
                loadError: "De berichttellingen konden niet worden geladen.",
                noReceipts:
                    "Dit account ontvangt geen afleverrapporten, dus de aflevering kan niet worden getoond.",
                showFailed: "De mislukte {{channel}}-berichten tonen in Logs",
                noChannelHelp:
                    "Kiezers zonder geregistreerd contactkanaal. Hun berichten mislukken en worden nooit onder een kanaal geteld.",
                showFailedNoChannel:
                    "Toon in Logs de mislukte berichten van kiezers zonder contactkanaal",
            },
            error: {
                UNSUPPORTED_VERSION:
                    "Deze configuratie gebruikt versie {{version}}, die niet wordt ondersteund.",
                DUPLICATE_CHANNEL: "{{channel}} is meer dan eens geconfigureerd.",
                UNKNOWN_ACCOUNT:
                    "Het {{channel}}-account bestaat niet meer. Kies een ander account.",
                ACCOUNT_OF_ANOTHER_TENANT: "Het gekozen account hoort bij een andere tenant.",
                ACCOUNT_CHANNEL_MISMATCH:
                    "Het gekozen account verzendt geen {{channel}}-berichten.",
                PURPOSE_NOT_READY:
                    "{{channel}} kan nog geen {{purpose}} verzenden. Ontbreekt: {{blockers}}.",
                TEMPLATE_NOT_APPROVED:
                    "De {{channel}}-sjabloon voor {{purpose}} in {{language}} is niet goedgekeurd door de provider.",
                OUT_OF_WINDOW_NOT_SUPPORTED:
                    "{{channel}} kan met dit account niet buiten een gespreksvenster verzenden: het heeft geen gespreksvenster, of de berichten hebben al een sjabloon nodig.",
                ELECTION_CHANNEL_NOT_ENABLED:
                    "{{election}} biedt {{channel}} aan, dat dit evenement niet gebruikt.",
                UNKNOWN_ELECTION: "{{election}} is geen verkiezing van dit evenement.",
                UTILITY_TEMPLATE_MISSING:
                    "Berichten via {{channel}} buiten het venster van 24 uur hebben de goedgekeurde hulpprogrammasjabloon nodig. Koppel die onder Goedgekeurde sjablonen aan berichten.",
            },
            noReadPermission:
                "Voor het bekijken van deze instellingen is de machtiging messaging-config-read nodig.",
            chooseAccount: "Kies eerst een account.",
            needsSavingAgain: "De configuratie moet opnieuw worden opgeslagen.",
            needsSavingAgainHelp:
                "Ze is gekopieerd uit de eerdere opslag en doorstond de controles van een opslag niet. Los de getoonde problemen op en sla ze daarna opnieuw op.",
        },
        messaging: {
            channel: {
                EMAIL: "E-mail",
                SMS: "Sms",
                WHATSAPP: "WhatsApp",
                VIBER: "Viber",
                MESSENGER: "Facebook Messenger",
            },
            provider: {
                AWS_SES: "Amazon SES",
                SMTP: "SMTP-server",
                AWS_SNS: "Amazon SNS",
                WHATSAPP_CLOUD_API: "WhatsApp Cloud API (Meta)",
                MESSENGER_SEND_API: "Messenger Platform (Meta)",
                VIBER_INFOBIP: "Viber Business Messages (Infobip)",
                CONSOLE: "Console (niet meer ondersteund)",
                HTTP_API: "Aangepaste HTTP-API (niet meer ondersteund)",
            },
            purpose: {
                OTP: "Codes",
                NOTICE: "Berichten",
            },
            state: {
                QUEUED: "In wachtrij",
                ACCEPTED: "Geaccepteerd",
                DELIVERED: "Afgeleverd",
                FAILED: "Mislukt",
                UNKNOWN: "Onbekend",
                UNRESOLVED: "Onopgelost",
            },
            stateHelp: {
                QUEUED: "Wacht op overdracht aan de provider.",
                ACCEPTED:
                    "De provider heeft het bericht geaccepteerd. Dat betekent niet dat de kiezer het heeft ontvangen.",
                DELIVERED: "De provider meldde het bericht als afgeleverd.",
                FAILED: "De provider bevestigde dat het bericht niet is afgeleverd.",
                UNKNOWN: "De aflevering is nog niet bevestigd.",
                UNRESOLVED:
                    "De aflevering kon niet binnen 24 uur worden bevestigd. Dat betekent niet dat ze mislukt is.",
            },
            blocker: {
                UNCHECKED:
                    "Niet gecontroleerd voor het huidige nummer, de huidige pagina of afzender",
                NOT_CONNECTED: "Niet verbonden",
                UNSUPPORTED_PURPOSE: "Niet ondersteund door deze provider",
                NEEDS_PROVIDER_APPROVAL: "Goedkeuring van de provider nodig",
                NEEDS_PRODUCTION_ACCESS: "Productietoegang nodig",
                NEEDS_APPROVED_TEMPLATE: "Goedgekeurde sjabloon nodig",
                UNSUPPORTED_PROVIDER: "Niet-ondersteunde provider: vervang dit account",
                CALLBACK_NOT_CONFIGURED: "Callbackadres niet ingesteld",
                REPORT_CREDENTIALS_MISSING: "Inloggegevens voor rapporten niet gegenereerd",
            },
            readiness: {
                connected: "Verbonden",
                notConnected: "Niet verbonden",
                readyOtp: "Klaar voor codes",
                readyNotice: "Klaar voor berichten",
                notReady: "Niet klaar",
                lastCheck: "Gecontroleerd {{date}}",
                neverChecked: "Nog niet gecontroleerd",
            },
            approval: {
                PENDING: "Wacht op goedkeuring van de provider",
                CONFIRMED: "Goedkeuring van de provider bevestigd",
            },
            credential: {
                ACCESS_TOKEN: "Toegangstoken",
                APP_SECRET: "App-geheim",
                VERIFY_TOKEN: "Verificatietoken",
                API_KEY: "API-sleutel",
                SMTP_PASSWORD: "Wachtwoord",
                AWS_ACCESS_KEY_ID: "AWS-toegangssleutel-ID",
                AWS_SECRET_ACCESS_KEY: "Geheime AWS-toegangssleutel",
                REPORT_USERNAME: "Gebruikersnaam voor rapporten",
                REPORT_PASSWORD: "Wachtwoord voor rapporten",
            },
            deliveryUnavailable: "Aflevering niet beschikbaar",
            templates: {
                noMethod: "Kies minstens één methode voor de sjabloon.",
                parameters: "Sjabloonparameters",
                parametersHelp:
                    "Wat elke plaatshouder van de goedgekeurde sjabloon vult, op volgorde, zoals user.first_name of vote_url. Schrijf voor een sjabloon met benoemde parameters @naam=waarde, zoals @first_name=user.first_name; elke andere invoer is positioneel.",
                parameter: "Parameter {{position}}",
                removeParameter: "Parameter {{position}} verwijderen",
                addParameter: "Parameter toevoegen",
                noAccount:
                    "Er is nog geen {{channel}}-account. Voeg er een toe in Instellingen > Berichten om te zien welke talen zijn goedgekeurd.",
                account: "Account",
                approvalTitle: "Goedgekeurde sjablonen",
                language: "Taal",
                approvalFor: "Goedgekeurd voor {{purpose}}",
                approved: "Goedgekeurd",
                notApproved: "Niet goedgekeurd",
                approvalHelp:
                    "Goedkeuringen komen van de provider en worden bijgewerkt door de verbindingscontrole van het account.",
                messengerIntro:
                    "Binnen 24 uur na het laatste bericht van de kiezer stuurt Messenger de onderstaande tekst.",
                messengerMessage: "Bericht binnen 24 uur",
                messengerWindow:
                    "Een opgeslagen Messenger-ontvanger is geen toestemming om te verzenden. Buiten het venster van 24 uur wordt dit bericht verzonden met de goedgekeurde hulpprogrammasjabloon die hieronder is ingesteld of in het verkiezingsevenement is gekoppeld; zonder sjabloon wordt het niet verzonden en wordt de mislukking met de reden getoond. Hulpprogrammaberichten vereisen de machtiging page_utility_messaging en een goedgekeurde UTILITY-sjabloon op de pagina.",
                intro: {
                    WHATSAPP:
                        "WhatsApp verzendt alleen sjablonen die Meta heeft goedgekeurd voor het WhatsApp Business-account. Het bericht moet overeenkomen met de goedgekeurde sjabloon; kies wat de parameters vult.",
                    VIBER: "Viber verzendt codes en transactionele berichten alleen met sjablonen die de Viber-partner heeft goedgekeurd. Het bericht moet overeenkomen met de goedgekeurde sjabloon; kies wat de parameters vult.",
                },
                approvedWording: "Goedgekeurde tekst",
                approvedWordingHelp:
                    "Een kopie van de goedgekeurde sjabloon, gebruikt als voorbeeld. Hier wijzigen verandert niet wat de provider verzendt.",
                providerTemplateTitle: "Sjabloon van de provider",
                providerTemplateHelp:
                    "Optioneel. De naam of ID van de goedgekeurde sjabloon bij de provider. Indien leeg wordt de sjabloon van het verkiezingsevenement gebruikt die aan de alias van deze sjabloon is gekoppeld, of de standaard van het evenement voor het doel.",
                providerTemplate: "Naam of ID van de sjabloon van de provider",
                providerLanguage: "Taalcode van de provider",
                providerLanguageHelp: {
                    WHATSAPP:
                        "Gebruik de exacte taalcode van het goedgekeurde WhatsApp-sjabloon, zoals en of en_US. Een regiocode is alleen nodig als het sjabloon daarmee is goedgekeurd.",
                    VIBER: "De taalcode waaronder de Viber-provider de sjabloon kent, wanneer er een nodig is.",
                    MESSENGER:
                        "De taalcode van de goedgekeurde hulpprogrammasjabloon, zoals en_US.",
                },
                noReadPermission:
                    "Voor het bekijken van de accounts is de machtiging messaging-config-read nodig.",
            },
            send: {
                eachVoter: "Het kanaal van elke kiezer",
                eachVoterHelp:
                    "Elke kiezer krijgt één bericht via het eigen kanaal. Als de aflevering mislukt, wordt het opnieuw geprobeerd via dat kanaal en verschijnt de mislukking op het tabblad Berichten van het evenement. Onbevestigde aflevering wordt getoond als Onbekend.",
                channelColumn: "Kanaal",
                sendsFrom: "Verzonden vanaf",
                noAccount: "Geen account",
                missingContent: "Deze melding heeft geen inhoud voor {{channels}}.",
                approvedTemplateHelp:
                    "Omdat {{channel}}-sjablonen door de provider zijn goedgekeurd, kunnen de tekst, het providersjabloon en de taal hier niet worden gewijzigd. Kies een ander opgeslagen sjabloon om andere goedgekeurde inhoud te gebruiken.",
                voters: "Kiezers",
                environment: "De standaardafzender van de installatie",
                noNoticeChannels:
                    "De berichteninstellingen van dit evenement zetten berichten voor geen enkel kanaal aan, dus geen kiezer zou deze melding krijgen. Zet berichten aan voor een kanaal op het tabblad Berichten van het evenement.",
                noReadPermission:
                    "Voor het tonen van het kanaal van elke kiezer is de machtiging messaging-config-read nodig. Elke kiezer krijgt de melding toch via zijn eigen kanaal.",
            },
            audience: {
                notOfferedRow:
                    "Geen enkele Post van deze kiezers biedt dit kanaal aan voor berichten, dus niemand van hen ontvangt het: {{reason}}.",
                willSend: "Ontvangen de melding: {{willSend}}",
                selectedTimedOut:
                    "Het tellen van de geselecteerde kiezers duurde te lang, dus er worden geen aantallen getoond. Elke kiezer ontvangt de melding toch via het eigen kanaal.",
                notSent: "Niet verzonden",
                deletedAccount: "Verwijderd account",
                notOffered: "{{notOffered}} van hen ontvangen het niet: {{reason}}.",
                noAccountHelp:
                    "Geen account verzendt via dit kanaal, dus de berichten aan deze kiezers mislukken: {{reason}}.",
                noChannelHelp:
                    "Deze kiezers hebben geen contactkanaal geregistreerd. Ze ontvangen niets, en elk staat als mislukt: {{reason}}.",
                total: "Kiezers in dit publiek: {{total}}",
                loadError:
                    "Het aantal kiezers kon niet worden geladen. Elke kiezer ontvangt de melding toch via het eigen kanaal.",
            },
            voterCount: {
                countedAt: "Geteld: {{time}}.",
                never: "Nog niet geteld.",
                pending: "Er is een nieuwe telling onderweg.",
                timedOut: "De laatste telling duurde te lang en is gestopt.",
                failed: "De laatste telling is mislukt.",
                previous: "De getoonde aantallen zijn van de telling daarvoor.",
                recount: "Opnieuw tellen",
                recountAsked:
                    "Nieuwe telling aangevraagd. De aantallen worden bijgewerkt als die klaar is.",
                recountAlreadyAsked: "Er is al een nieuwe telling onderweg.",
                recountError: "De nieuwe telling kon niet worden aangevraagd.",
                loadError: "De aantallen kiezers konden niet worden geladen.",
            },
            volumes: {
                title: "Maandvolumes",
                intro: "Pogingen die elke maand aan een provider zijn overgedragen, per kanaal, verzendaccount, land van bestemming en factuurcategorie, om de facturen van de providers te controleren. Elke poging telt in de maand waarin ze is gedaan, dus een nieuwe poging telt opnieuw. De factuurcategorie is die die bij het verzenden werd verwacht, of die die de provider heeft gemeld; of een poging factureerbaar is, is wat de provider heeft gemeld. Er worden geen prijzen getoond.",
                month: "Maand",
                monthHelp: "Maanden worden geteld in UTC.",
                loading: "De maandvolumes worden geladen",
                loadError: "De volumes konden niet worden geladen.",
                empty: "In {{month}} is geen poging aan een provider overgedragen.",
                noPermission:
                    "Voor het tonen van de volumes is de machtiging messaging-account-read of messaging-config-read nodig.",
                notReported: "Niet gemeld",
                deploymentOrDeleted: "Afzender van de installatie of een verwijderd account",
                category: {
                    AUTHENTICATION: "Authenticatie",
                    UTILITY: "Service",
                    SERVICE: "Servicegesprek",
                    TRANSACTIONAL: "Transactioneel",
                    EMAIL: "E-mail",
                },
                billableYes: "Factureerbaar",
                billableNo: "Niet factureerbaar",
                total: "Alle pogingen",
                column: {
                    account: "Verzonden vanaf",
                    country: "Land van bestemming",
                    category: "Factuurcategorie",
                    billable: "Factureerbaar",
                    messages: "Pogingen",
                },
            },
            voter: {
                listChannel: "Berichtenkanaal",
                title: "Berichten",
                notSet: "Niet ingesteld",
                channel: "Kanaal",
                destination: "Verzonden naar",
                messengerPage: "Messenger-pagina",
                consent: "Toestemming",
                lastFailure: "Laatste mislukking",
                noFailure: "Geen",
                failureNeedsPermission: "Zichtbaar met de machtiging messaging-config-read.",
                noChannel: "Geen contact geregistreerd",
                notConfirmed: "Aflevering niet bevestigd",
                notConfirmedHelp:
                    "Het laatste bericht waarvan de aflevering nog niet is bevestigd. Na 24 uur wordt het als Onopgelost getoond; dat betekent niet dat het mislukt is.",
            },
            logs: {
                channel: "Kanaal",
                status: "Status",
                statusAsLogged:
                    "De status toen de vermelding werd geschreven; voor de huidige status is de machtiging messaging-config-read nodig.",
            },
            stats: {
                sent: {
                    WHATSAPP: "Verzonden WhatsApp-berichten",
                    VIBER: "Verzonden Viber-berichten",
                    MESSENGER: "Verzonden Messenger-berichten",
                },
            },
            reason: {
                CANDIDATE_REFUSED:
                    "De nieuwe inloggegevens maakten geen verbinding: de vorige blijven in gebruik",
                ACCOUNT_IN_USE:
                    "Gebruikt door de berichtinstellingen van een verkiezingsevenement: verwijder het daar eerst",
                NUMBER_CHANGED:
                    "Een kiezer heeft een ander WhatsApp-nummer: vraag hem zijn contactgegevens opnieuw te verifiëren",
                IDENTITY_CHANGED:
                    "De verzendidentiteit van het account is gewijzigd: het wordt opnieuw gecontroleerd",
                TEMPLATES_NOT_READ: "De sjablonen van de provider konden niet worden gelezen",
                PARTNER_LOGIN_REFUSED: "Infobip heeft de API-sleutel van het account geweigerd",
                NO_PAGES_MESSAGING: "Het paginatoken heeft geen pages_messaging-machtiging",
                NOT_SUBSCRIBED: "De app is niet geabonneerd op de pagina",
                NUMBER_STATUS_NOT_READ: "De status van het nummer kon niet worden gelezen",
                CALLBACK_NOT_CONFIGURED: "Callbackadres niet ingesteld",
                UNSUPPORTED_PROVIDER: "Niet-ondersteunde provider: vervang dit account",
                CREDENTIALS_FOLLOW_ENDPOINT:
                    "Voer de inloggegevens opnieuw in voor het nieuwe adres",
                NO_CONTACT_CHANNEL: "Geen contactkanaal",
                NOT_RETRYABLE_NO_TEMPLATE:
                    "Kan niet opnieuw worden geprobeerd: geen opgeslagen sjabloon",
                CHANNEL_NOT_OFFERED_FOR_NOTICES: "Kanaal niet aangeboden voor berichten",
                CHANNEL_NOT_OFFERED_FOR_CODES: "Kanaal niet aangeboden voor codes",
                NO_CONTENT_FOR_CHANNEL: "Geen inhoud voor het kanaal",
                NO_ACCOUNT_FOR_CHANNEL: "Geen verzendaccount voor het kanaal",
                INVALID_DESTINATION: "Ongeldige bestemming voor het kanaal",
                NOT_RETRYABLE_PROTECTED_FIELDS:
                    "Kan niet opnieuw worden geprobeerd: bevat beveiligde velden",
                REPORTED_TOO_LATE: "Te laat gemeld om opnieuw te proberen",
                CHANNEL_CHANGED: "Het kanaal van de kiezer is gewijzigd: niet opnieuw geprobeerd",
                WORKER_STOPPED_BEFORE_SENDING: "Het proces stopte vóór het verzenden",
                WORKER_STOPPED_AFTER_SENDING:
                    "Het proces stopte voordat de uitkomst was vastgelegd",
                CODE_EXPIRED_BEFORE_SENDING: "De code verliep voordat hij kon worden verzonden",
                CAPACITY_BUSY: "Niet verzonden: het account zat te lang aan zijn verzendlimiet",
                PROVIDER_ANSWER_LATE:
                    "De provider antwoordde niet op tijd: het bericht is mogelijk verzonden",
                UNRESOLVED: "Aflevering kon niet worden bevestigd",
                SELECTED_WITHOUT_VOTERS: "Selecteer ten minste één kiezer",
                WRONG_RECIPIENT_KIND:
                    "De bestemming is niet van het soort waarnaar dit kanaal verzendt",
                UNSUPPORTED_PURPOSE: "De provider verzendt dit soort bericht niet",
                MISSING_APPROVED_TEMPLATE: "Geen goedgekeurde sjabloon voor dit bericht",
                OUTSIDE_CONVERSATION_WINDOW: "Buiten het gespreksvenster",
                DESTINATION_NOT_ALLOWED:
                    "Dit account verzendt niet naar het land van de bestemming",
                CODE_EXPIRED: "De code is verlopen",
                MISSING_CODE: "Geen code om te verzenden",
                ENDPOINT_NOT_HTTPS: "Het adres van de provider moet beginnen met https://",
                PROVIDER_CODE: "Foutcode van de provider {{code}}",
                CHECK_DROPPED:
                    "Het account is gewijzigd tijdens de controle: controleer het opnieuw",
                ACCOUNT_NOT_READY: "Niet verzonden: het account is niet klaar ({{blocker}})",
                PROVIDER_UNREACHABLE: "De provider was niet bereikbaar",
                PROVIDER_STATUS: "De provider antwoordde met status {{status}}",
                MISSING_CREDENTIAL: "Ontbrekende referentie: {{credential}}",
                PROVIDER_CHECK_FAILED: "De controle van de provider is mislukt: {{detail}}",
                NUMBER_STATUS: "De status van het nummer is {{status}}",
                NUMBER_NAME_STATUS: "De weergavenaam van het nummer is {{status}}",
                NUMBER_IN_SANDBOX: "Het nummer staat in sandboxmodus",
                NUMBER_NOT_VERIFIED: "De code van het nummer is niet geverifieerd",
                SMS_SANDBOX: "Het account staat in de sms-sandbox",
                SES_SANDBOX: "Het account staat in de SES-sandbox",
                SES_SENDING_DISABLED: "Verzenden is uitgeschakeld voor dit account",
                SMTP_REFUSED: "SMTP-server onbereikbaar of referenties geweigerd",
                TOKEN_INVALID: "Het toegangstoken is niet geldig of verlopen",
                TOKEN_CANNOT_SEND: "Het toegangstoken heeft geen toestemming om te verzenden",
                TOKEN_NOT_CHECKED: "Het toegangstoken kon niet worden gecontroleerd",
                APP_SECRET_REFUSED: "Meta heeft het app-geheim geweigerd",
                APP_SECRET_NOT_CHECKED: "Het app-geheim kon niet worden gecontroleerd",
                RETRY_NOT_PREPARED:
                    "Kan niet opnieuw worden geprobeerd: het bericht kon niet opnieuw worden voorbereid",
                NOT_THE_VOTERS_CHANNEL: "Niet het kanaal van de kiezer",
                REPORT_CREDENTIALS_MISSING:
                    "Genereer de inloggegevens voor rapporten en maak het Infobip-abonnement aan",
                CALLBACK_NOT_HTTPS: "Het callbackadres moet https zijn",
                UNKNOWN_ACCOUNT:
                    "Dit account bestaat niet meer: het is mogelijk in een ander tabblad verwijderd. Laad de lijst opnieuw.",
                INTERNAL_ERROR:
                    "De server kon het verzoek niet voltooien. Probeer het later opnieuw.",
                VERIFY_TOKEN_GENERATED: "Het verificatietoken wordt gegenereerd, niet ingevoerd",
                CREDENTIAL_GENERATED: "{{credential}} wordt gegenereerd, niet ingevoerd",
                CREDENTIAL_NOT_USED: "{{provider}}-accounts gebruiken geen {{credential}}",
                CREDENTIAL_EMPTY: "{{credential}} mag niet leeg zijn",
                NO_REPORT_CREDENTIALS:
                    "{{provider}}-accounts hebben geen referenties voor rapporten",
                UNKNOWN_AUDIENCE: "Onbekende selectie van ontvangers",
                INVALID_MONTH: "De maand moet worden geschreven als JJJJ-MM",
                INVALID_SUPPORT_REFERENCE: "Geen ondersteuningsreferentie",
                MESSENGER_PAGE_CHANGED:
                    "Niet verzonden: de kiezer is verbonden met een andere Messenger-pagina",
            },
        },
        messagingAccounts: {
            status: {
                title: "Status bij de provider",
                daily: "Accounts worden één keer per dag gecontroleerd en telkens wanneer u de verbinding controleert.",
                unchecked:
                    "Het nummer, de pagina of de afzender is sinds de laatste controle gewijzigd. Tot het account opnieuw is gecontroleerd, mislukken codes en meldingen via dit account omdat het niet klaar is; het testbericht wordt wel verzonden.",
                notReported: "Niet gemeld",
                yes: "Ja",
                no: "Nee",
                token: "Toegangstoken",
                tokenExpires: "Geldig · verloopt op {{date}}",
                tokenNoExpiry: "Geldig · verloopt niet",
                tokenInvalid: "Niet geldig",
                missingScopes: "Ontbrekende machtigingen: {{scopes}}",
                numberStatus: "Status van het nummer",
                nameStatus: "Weergavenaam: {{status}}",
                qualityRating: "Kwaliteitsbeoordeling",
                quality: {
                    GREEN: "Hoog",
                    YELLOW: "Gemiddeld",
                    RED: "Laag",
                    UNKNOWN: "Onbekend",
                },
                messagingLimit: "Berichtenlimiet",
                pagesMessaging: "Machtiging om te verzenden (pages_messaging)",
                subscribed: "App geabonneerd op de pagina",
                getStarted: "Knop Aan de slag",
                getStartedMissing:
                    "Zonder de knop kunnen kiezers die de pagina nooit een bericht stuurden het gesprek dat hun code brengt niet starten.",
                partnerLogin: "Aanmelding bij de partner",
                appSecret: "App-geheim",
                notChecked: "Kon niet worden gecontroleerd",
                numberQualityWarning:
                    "Een kwaliteitswaarschuwing van Meta: het nummer verzendt nog, maar een lage kwaliteit kan de berichtenlimiet verlagen.",
                accepted: "Geaccepteerd",
                refused: "Geweigerd",
                callback: "Callbackadres",
                callbackReady: "Ingesteld",
                approvalFromCheck: "Gemeld door de controle van de provider",
                approvalEntered:
                    "Ingevoerd door een beheerder: de controle van de provider meldde het niet",
                approvalEnteredHelp:
                    "Wordt alleen gebruikt totdat de controle van de provider de goedkeuring meldt.",
                templates: {
                    title: "Sjablonen bij de provider",
                    none: "Geen sjablonen",
                    name: "Sjabloon",
                    language: "Taal",
                    category: "Categorie",
                    status: "Toestand",
                    entered:
                        "Viber-sjablonen worden door een beheerder ingevoerd onder Goedgekeurde sjablonen: de partner heeft geen API voor sjabloongoedkeuring.",
                },
                templateCategory: {
                    AUTHENTICATION: "Authenticatie",
                    UTILITY: "Service",
                    MARKETING: "Marketing",
                },
                templateStatus: {
                    APPROVED: "Goedgekeurd",
                    PENDING: "In beoordeling",
                    REJECTED: "Afgewezen",
                    PAUSED: "Gepauzeerd",
                    DISABLED: "Uitgeschakeld",
                },
                numberChanges: {
                    title: "Kiezers die hun WhatsApp-nummer hebben gewijzigd",
                    description:
                        "WhatsApp heeft deze nummerwijzigingen gemeld. Vraag deze kiezers hun contactgegevens opnieuw te verifiëren; er is niets automatisch gewijzigd.",
                    item: "{{old}} is gewijzigd in {{new}} op {{date}}.",
                    voters: "Kiezers: {{voters}}",
                    noVoters:
                        "Geen enkele kiezer ontving berichten van dit account op het oude nummer.",
                },
            },
            tab: "BERICHTEN",
            description:
                "Accounts die kiezers hun codes en berichten sturen. Elk verkiezingsevenement kiest het account per kanaal; nieuwe evenementen beginnen met het standaardaccount.",
            list: {
                title: "Verzendaccounts",
                add: "Account toevoegen",
                loading: "Accounts laden",
                loadError: "De verzendaccounts konden niet worden geladen.",
                empty: "Nog geen verzendaccounts.",
            },
            column: {
                channel: "Kanaal",
                name: "Account",
                sender: "Verzendt als",
                provider: "Provider",
                default: "Standaard",
                isDefault: "Standaardaccount",
                lastCheck: "Laatste controle",
                actions: "Acties",
            },
            action: {
                edit: "Bewerken",
                editNamed: "{{name}} bewerken",
                view: "Bekijken",
                viewNamed: "{{name}} bekijken",
                check: "Verbinding controleren",
                checkNamed: "De verbinding van {{name}} controleren",
                test: "Testbericht versturen",
                testNamed: "Een testbericht versturen vanaf {{name}}",
                delete: "Verwijderen",
                deleteNamed: "{{name}} verwijderen",
            },
            check: {
                done: "{{name}} is gecontroleerd. De status is bijgewerkt.",
                error: "{{name}} kon niet worden gecontroleerd.",
            },
            delete: {
                refused: "Het account is niet verwijderd: {{reason}}",
                title: "Account verwijderen",
                body: "{{name}} verwijderen? Verkiezingsevenementen die het gebruiken, versturen dan niets meer via dat kanaal.",
                success: "Account verwijderd",
                error: "Het account kon niet worden verwijderd.",
            },
            editor: {
                addTitle: "Account toevoegen",
                editTitle: "{{channel}}-account bewerken",
                subtitle:
                    "Kiezers ontvangen codes en berichten van dit account via de kanalen die het gebruiken.",
                channel: "Kanaal",
                provider: "Provider",
                save: "Opslaan",
                cancel: "Annuleren",
                close: "Sluiten",
                channelHelp: "Kan niet worden gewijzigd nadat het account is aangemaakt.",
            },
            field: {
                name: "Accountnaam",
                from_address: "Afzenderadres",
                from_name: "Afzendernaam",
                region: "AWS-regio",
                notification_topic_arn: "Onderwerp voor afleveringsmeldingen (SNS-ARN)",
                server_url: "Server en poort",
                sender_id: "Afzender-ID",
                origination_number: "Afzendernummer",
                business_account_id: "WhatsApp Business-account-ID",
                phone_number_id: "Telefoonnummer-ID",
                display_phone_number: "Nummer",
                display_name: "Weergavenaam",
                api_version: "Graph API-versie",
                page_id: "Facebook-pagina-ID",
                page_name: "Paginanaam",
                page_username: "Gebruikersnaam van de pagina",
                base_url: "Basis-URL van de API",
                sender: "Afzendernaam",
                provider_approval: "Goedkeuring van de provider",
                is_default: "Standaard {{channel}}-account voor nieuwe verkiezingsevenementen",
                api_base_url: "Basis-URL van de Graph API",
            },
            fieldHelp: {
                from_address:
                    "Het adres dat kiezers zien. Het domein moet bij de provider geverifieerd zijn.",
                notification_topic_arn:
                    "Het SNS-onderwerp waarop SES aflever- en bounce-meldingen publiceert. Meldingen van een ander onderwerp worden geweigerd.",
                sender_id: "Maximaal 11 letters en cijfers. Sommige landen vereisen registratie.",
                origination_number:
                    "Gebruikt in plaats van de afzender-ID waar een land een nummer vereist.",
                phone_number_id: "Het nummer waarvandaan berichten worden verzonden.",
                display_name: "De weergavenaam die Meta voor het nummer heeft goedgekeurd.",
                page_username:
                    "Gebruikt voor de m.me-link die kiezers openen om hun code te krijgen.",
                api_version: "Bijvoorbeeld v23.0.",
                base_url: "De basis-URL van de Infobip-API van het account.",
                sender: "De goedgekeurde afzender die kiezers zien.",
                provider_approval:
                    "Meta staat berichten van overheden via WhatsApp alleen toe via een goedgekeurde regeling. Kies Goedkeuring van de provider bevestigd zodra Meta dit voor dit account heeft goedgekeurd; tot die tijd kunnen codes en berichten voor dit account niet worden ingeschakeld.",
                api_base_url:
                    "Alleen wanneer de Graph API niet die van Meta zelf is, zoals het eindpunt van een Solution Provider. Leeg gebruikt die van Meta.",
            },
            error: {
                REQUIRED: "Verplicht",
                NOT_A_COUNT: "Voer een geheel getal in",
                AT_LEAST_ONE: "Voer 1 of meer in",
                OTP_ABOVE_TOTAL: "Mag niet hoger zijn dan het aantal berichten per seconde",
                INVALID_CALLING_CODE: "Voer landnummers van 1 tot 3 cijfers in, zoals 63",
                DUPLICATE_LANGUAGE: "Deze taal heeft al een sjabloon voor dit doel",
                NOT_A_URL: "Voer een adres in dat begint met https://",
                UNSUPPORTED_PROVIDER: "Niet-ondersteunde provider: vervang dit account",
                CREDENTIALS_FOLLOW_ENDPOINT:
                    "Voer de inloggegevens opnieuw in voor het nieuwe adres",
            },
            warning: {
                pageChangeCount:
                    "Kiezers verbonden met {{page}} die opnieuw moeten verbinden: {{count}}.",
                pageChangeCounting: "Kiezers verbonden met de pagina worden geteld…",
                pageChangeCountError:
                    "Het aantal kiezers verbonden met de pagina kon niet worden gelezen.",
                recheck:
                    "Bij opslaan wordt het account op de achtergrond opnieuw gecontroleerd. Tot die controle slaagt, mislukken codes en meldingen via dit account omdat het niet klaar is; het testbericht wordt wel verzonden.",
                pageChange:
                    "Messenger-gesprekken horen bij een pagina. Na het wijzigen van de pagina worden er geen codes of berichten verzonden aan de kiezers die met {{page}} verbonden zijn, totdat ze opnieuw verbinden. Een kiezer verbindt opnieuw bij de volgende keer aanmelden, na het invoeren van het wachtwoord: het aanmelden toont de verbindingsstap voor de nieuwe pagina, de kiezer tikt in Messenger op Aan de slag en ontvangt daar de code. Een kiezer die zich niet met een wachtwoord kan aanmelden, bijvoorbeeld omdat hij het vergeten is, heeft medewerkers nodig om zijn kanaal te wijzigen.",
                numberChange:
                    "Berichten komen van een ander nummer. De sjablonen moeten in dat bedrijfsaccount zijn goedgekeurd voordat het codes kan versturen, en kiezers zien een nieuwe chat.",
            },
            viber: {
                title: "Goedgekeurde sjablonen",
                description:
                    "Voer de sjablonen in die Viber via de partner heeft goedgekeurd, per doel en taal. De sjabloon-API van de partner is niet beschikbaar, dus deze lijst wordt handmatig bijgehouden en de verbindingscontrole leest hem.",
                purpose: "Doel",
                language: "Taal",
                templateId: "Sjabloon-ID bij de partner",
                add: "Sjabloon toevoegen",
                remove: "Sjabloon verwijderen",
            },
            limits: {
                title: "Verzendlimieten",
                messagesPerSecond: "Berichten per seconde",
                messagesPerSecondHelp: "Leeg: berichten worden niet beperkt.",
                otpReservedPerSecond: "Gereserveerd voor codes per seconde",
                otpReservedHelp:
                    "Een ondergrens in het ene gedeelde verzendbudget van het account: meldingen stoppen daar en codes gaan voor. Als er geen codes worden verzonden, kan een bulkverzending het volledige tempo gebruiken.",
                otpConcurrency: "Codes tegelijk verzonden",
                otpConcurrencyHelp:
                    "Leeg: 8. Codes en de berichten van Keycloak (resetlinks, tijdelijke wachtwoorden) bezetten elk een plaats terwijl ze worden verzonden. Geteld per serverproces; verdere verzendingen wachten op hun beurt.",
                allowedCallingCodes: "Toegestane bestemmingen (landnummers)",
                allowedCallingCodesHelp:
                    "Gescheiden door komma's, bijvoorbeeld 63, 971. Leeg staat elke bestemming toe.",
            },
            credentials: {
                title: "Inloggegevens",
                description:
                    "Inloggegevens zijn alleen-schrijven: na het opslaan wordt alleen de datum van de laatste vervanging getoond.",
                set: "Ingesteld · vervangen {{date}}. Het wordt versleuteld opgeslagen en nooit getoond.",
                replace: "Vervangen",
                replaceNamed: "{{name}} vervangen",
                reenter: "Het adres is gewijzigd: voer dit inloggegeven opnieuw in.",
            },
            credentialHelp: {
                AWS_SES: {
                    AWS_ACCESS_KEY_ID:
                        "Optioneel. Zonder sleutels wordt de eigen rol van de dienst gebruikt.",
                    AWS_SECRET_ACCESS_KEY:
                        "Optioneel. Stel het samen met de toegangssleutel-ID in.",
                },
                AWS_SNS: {
                    AWS_ACCESS_KEY_ID:
                        "Optioneel. Zonder sleutels wordt de eigen rol van de dienst gebruikt.",
                    AWS_SECRET_ACCESS_KEY:
                        "Optioneel. Stel het samen met de toegangssleutel-ID in.",
                },
                SMTP: {
                    SMTP_PASSWORD: "Het wachtwoord van de SMTP-server.",
                },
                WHATSAPP_CLOUD_API: {
                    ACCESS_TOKEN:
                        "Een token van een systeemgebruiker in het bedrijfsportfolio van de eigenaar, met whatsapp_business_messaging.",
                    APP_SECRET: "Controleert dat webhookaanroepen van Meta komen.",
                },
                MESSENGER_SEND_API: {
                    ACCESS_TOKEN: "Een paginatoegangstoken met pages_messaging.",
                    APP_SECRET: "Controleert dat webhookaanroepen van Meta komen.",
                },
                VIBER_INFOBIP: {
                    API_KEY: "De Infobip-API-sleutel.",
                },
            },
            webhook: {
                title: "Afleverrapporten",
                description: {
                    WHATSAPP_CLOUD_API:
                        "Voer deze callback-URL in bij de webhookinstellingen van Meta. Afleverrapporten komen hier binnen en worden gecontroleerd met het app-geheim.",
                    MESSENGER_SEND_API:
                        "Voer deze callback-URL in bij de webhookinstellingen van Meta. Afleverrapporten en de tik van de kiezer op Aan de slag komen hier binnen en worden gecontroleerd met het app-geheim.",
                    VIBER_INFOBIP:
                        "Infobip verstuurt Viber-afleverrapporten naar deze callback-URL via een abonnement met Basic-authenticatie.",
                    AWS_SES:
                        "Abonneer deze callback-URL (HTTPS) op het onderwerp voor afleverberichten. Aflever- en bounce-gebeurtenissen komen hier binnen en worden gecontroleerd met de handtekening van Amazon.",
                },
                afterSaving: "Getoond na het opslaan",
                tokenSet: "Ingesteld · vervangen {{date}}",
                tokenMissing: "Nog niet gegenereerd",
                tokenAfterSaving: "Wordt na het opslaan gegenereerd",
                generate: "Verificatietoken genereren",
                tokenTitle: "Verificatietoken",
                tokenOnce:
                    "Voer dit token nu in bij de webhookinstellingen van Meta. Het wordt maar één keer getoond.",
                copyToken: "Verificatietoken kopiëren",
                tokenDone: "Klaar",
                tokenError: "Het verificatietoken kon niet worden gegenereerd.",
                url: "Callback-URL",
                copyUrl: "Callback-URL kopiëren",
                needsWrite: "Zichtbaar voor beheerders die berichtaccounts kunnen beheren.",
                loading: "Laden…",
                unavailable: "Niet beschikbaar",
                loadError: "De callback-URL kon niet worden geladen.",
            },
            copy: {
                success: "Gekopieerd",
                error: "Kopiëren mislukt",
            },
            save: {
                success: "Account opgeslagen",
                error: "Het account kon niet worden opgeslagen.",
                refused: "Het account is niet opgeslagen: {{reason}}",
            },
            test: {
                title: "Een testbericht versturen vanaf {{name}}",
                description:
                    "Verstuurt een echt bericht voor het gekozen doel naar deze bestemming. Het resultaat toont wat de provider meldde.",
                purpose: "Doel",
                destination: {
                    EMAIL_ADDRESS: "E-mailadres",
                    PHONE_NUMBER: "Telefoonnummer (E.164)",
                    PAGE_SCOPED_ID: "Paginagebonden ID",
                },
                language: "Taal",
                send: "Testbericht versturen",
                reason: "Reden: {{reason}}",
                error: "Het testbericht kon niet worden verstuurd.",
                template: "Goedgekeurde sjabloon",
                templateHelp:
                    "De naam of ID van de sjabloon die de provider voor dit doel en deze taal heeft goedgekeurd.",
                viberTemplate:
                    "Viber gebruikt de sjabloon die dit account als goedgekeurd vermeldt voor het gekozen doel en de gekozen taal.",
                languageHelp:
                    "Voer voor een provider die goedgekeurde sjablonen verzendt de taalcode van de provider voor de sjabloon in, zoals en_US.",
            },
            unsupported:
                "Niet-ondersteunde provider: vervang dit account. De provider wordt niet meer ondersteund, dus het account kan niet verzenden. Voeg voor dit kanaal een account toe met een ondersteunde provider, kies dat in de verkiezingsevenementen die dit account gebruiken en verwijder daarna dit account.",
            reports: {
                label: "Inloggegevens voor rapporten",
                missing: "Nog niet gegenereerd",
                generate: "Inloggegevens voor rapporten genereren",
                title: "Inloggegevens voor rapporten",
                once: "Voer ze nu in bij het Infobip-abonnement. Ze worden maar één keer getoond; nieuwe genereren vervangt ze.",
                copyUsername: "Gebruikersnaam voor rapporten kopiëren",
                copyPassword: "Wachtwoord voor rapporten kopiëren",
                error: "De inloggegevens voor rapporten konden niet worden gegenereerd.",
                step: {
                    subscription: "Maak in Infobip een abonnement aan voor Viber-afleverrapporten.",
                    url: "Stel de URL in op de callback-URL van dit account.",
                    auth: "Kies Basic-authenticatie en voer de gebruikersnaam en het wachtwoord voor rapporten in.",
                },
                confirmTitle: "Inloggegevens voor rapporten vervangen?",
                confirmBody:
                    "De huidige gebruikersnaam en het wachtwoord voor rapporten werken meteen niet meer. Tot het Infobip-abonnement de nieuwe heeft, worden Viber-afleverrapporten geweigerd en blijven berichten op Geaccepteerd staan.",
                confirm: "Vervangen",
            },
        },
        sentMessages: {
            showMore: "Meer tonen",
            showLess: "Minder tonen",
            tab: "Verzonden berichten",
            accountsTab: "Verzendaccounts",
            tabsLabel: "Onderdelen van berichten",
            description:
                "Berichten die de demoprovider van deze ontwikkelomgeving ontving, zoals de apps van de kiezers ze zouden tonen. Geen ervan heeft een echte kiezer bereikt. Dit tabblad verschijnt alleen waar de routes van de demoprovider zijn ingeschakeld.",
            modeLabel: "Berichten tonen van",
            all: "Alle",
            refresh: "Vernieuwen",
            refreshing: "Bezig met vernieuwen…",
            refreshError:
                "De berichten konden niet worden vernieuwd. De getoonde lijst is de laatst gelezen lijst.",
            empty: "Nog geen berichten.",
            emptyChannel: "Nog geen {{channel}}-berichten.",
            column: {
                time: "Tijd",
                channel: "Kanaal",
                from: "Van",
                to: "Aan",
                message: "Bericht",
                report: "Afleverrapport",
            },
            report: {
                accepted: "Rapport geaccepteerd door het webhookonderdeel (HTTP 200)",
                refused: "Rapport geweigerd: HTTP {{status}}",
                pending: "Rapport nog niet verzonden",
                notSent: "Rapport niet verzonden: {{reason}}",
                noAnswer: "Het rapport kreeg geen antwoord: {{reason}}",
                noReason: "geen reden opgegeven",
                unavailable: "Geen afleverrapporten op dit kanaal",
            },
            content: {
                demoCode:
                    "{{code}} is uw verificatiecode. Deel deze code voor uw veiligheid met niemand.",
                demoNotice:
                    "Stemherinnering\n\nHallo {{name}}, de stemming voor {{event}} is geopend.\n\nMeld u aan bij het stemportaal om uw stembiljet te bekijken en uw stem uit te brengen:\n{{url}}\n\nDeel uw aanmeldcode met niemand. Als u al hebt gestemd, hoeft u niets meer te doen.",
                demoLegacyNotice:
                    "Stemherinnering\n\nHallo {{name}}, de stemming is geopend. Meld u aan bij het stemportaal om uw stem uit te brengen:\n{{url}}\n\nDeel uw aanmeldcode met niemand.",

                template: "Sjabloon {{name}} ({{language}})",
                noText: "(geen tekst)",
                code: "Code",
                copyCode: "Code kopiëren",
                copied: "Code gekopieerd",
                copyFailed: "De code kon niet worden gekopieerd",
                truncated:
                    "Ingekort: de demoprovider bewaart alleen de eerste 64 KiB van elk veld.",
            },
            chat: {
                phone: "{{channel}} op een telefoon",
                chats: "{{channel}}-chats",
                open: "Chat met {{recipient}} vanaf {{sender}} openen",
                back: "Terug naar de chats",
                from: "Van {{sender}}",
                messages: "Berichten aan {{recipient}}",
            },
            getStarted: {
                title: "Aan de slag",
                help: "Plak de Messenger-link die de inschrijvingspagina toont en druk op Aan de slag, zoals de kiezer in Messenger zou doen. Daarna komt de code in de chat.",
                link: "Messenger-link",
                submit: "Aan de slag",
                accepted: "Het webhookonderdeel antwoordde 200. De code komt in de chat.",
                answered: "Het webhookonderdeel antwoordde {{status}}.",
                failed: "Aan de slag is mislukt: {{reason}}",
            },
            email: {
                inbox: "Postvak IN",
                reading: "Geselecteerde e-mail",
                noSubject: "(geen onderwerp)",
                from: "Van",
                to: "Aan",
                view: "E-mail tonen als",
                html: "HTML",
                text: "Platte tekst",
                htmlFrame: "Inhoud van de e-mail",
                noContent: "Deze e-mail heeft geen inhoud.",
                loading: "E-mail laden…",
                loadError:
                    "De HTML van de e-mail kon niet worden gelezen: de demoprovider bewaart hem misschien niet meer.",
            },
            reason: {
                SEVERAL_ACCOUNTS: "Meerdere accounts gebruiken deze afzender",
                ACCOUNTS_UNREADABLE: "De accounts konden niet worden gelezen",
                UNREACHABLE: "Het webhookonderdeel antwoordde niet",
                LINK_UNREADABLE: "Plak de m.me-link van de inschrijvingspagina",
                LINK_WITHOUT_REF: "De link heeft geen ref: kopieer de hele link",
                NO_PAGE_ACCOUNT: "Geen Messenger-account van deze organisatie gebruikt deze pagina",
                SEVERAL_PAGE_ACCOUNTS:
                    "Meerdere Messenger-accounts van deze organisatie gebruiken deze pagina",
            },
        },
        electoralLogConsole: {
            title: "Verkiezingslog",
            subtitle:
                "Bekijk het verkiezingslog en de stembus van een verkiezingsevenement, of bevraag de database van zijn verkiezingslog. Niets op deze pagina wijzigt gegevens.",
            notAllowed:
                "U hebt de machtiging electoral-log-console-read nodig om het verkiezingslog te openen.",
            tabs: {
                tables: "Tabellen",
                query: "Query",
            },
            electionEvent: "Verkiezingsevenement",
            tenant: "Tenant",
            table: "Tabel",
            tables: {
                records: "Records",
                ballots: "Stembiljetten",
                voters: "Kiezers",
                queue: "Wachtrij van de Sequencer",
            },
            order: {
                "label": "Volgorde",
                "newest-first": "Nieuwste eerst",
                "oldest-first": "Oudste eerst",
            },
            filters: {
                statement_kind: "Soort",
                election_id: "Verkiezing",
                area_id: "Gebieds-ID",
                user_id: "Gebruikers- of Kiezers-ID",
                ballot_id: "Stembiljet-ID",
                status: "Status",
                created_after: "Vanaf",
                created_before: "Tot",
                any: "Alle",
                apply: "Filters Toepassen",
                clear: "Wissen",
                invalid:
                    "Controleer de gemarkeerde filters: datums moeten geldig zijn en gebieds-ID's van de stembus moeten UUID's zijn.",
            },
            statuses: {
                valid: "Geldig",
                pending: "In behandeling",
                rejected: "Afgewezen",
            },
            estimatedRows: "Ongeveer {{rows}} rijen in deze tabel vóór filtering.",
            personalDataHidden:
                "Gebruikersnamen, IP-adressen en landen zijn verborgen: om ze te zien is de machtiging electoral-log-personal-data-read nodig.",
            loadError: "Het verkiezingslog kon niet worden gelezen.",
            record: {
                title: "Record {{position}}",
                view: "Record bekijken",
                message: "Bericht (gedecodeerd)",
                copy: "JSON Kopiëren",
                close: "Sluiten",
                loadError: "Het record kon niet worden gelezen.",
            },
            query: {
                help: "Query's worden uitgevoerd op de verkiezingslogdatabase van het verkiezingsevenement in een alleen-lezen transactie. De tabellen zijn electoral_log_messages, ballot_box_ballot, ballot_box_voter, ballot_box_pending en ballot_box_sequencer; trellis_logs beschrijft de Merkle-logs.",
                scope: "Elk verkiezingsevenement heeft een eigen database, dus een query leest één evenement.",
                chooseEvent: "Kies het verkiezingsevenement dat u wilt bevragen.",
                placeholder: "SELECT …",
                limits: "Alleen-lezen · tot 1.000 rijen · limiet van 30 s · elke query wordt met uw gebruiker vastgelegd in de serverlogs",
                run: "Query Uitvoeren",
                summary: "Rijen: {{rows}} · {{ms}} ms",
                truncated: "Alleen de eerste {{rows}} rijen worden getoond.",
                error: "De query kon niet worden uitgevoerd.",
            },
        },
        taskQueues: {
            title: "Taakwachtrijen",
            subtitle:
                "De taakwachtrijen van de omgeving: wat wacht, wat is verwerkt en hoe lang dat duurde. De wachtrijlijsten tonen nooit taakargumenten.",
            notAllowed:
                "U hebt de machtiging task-queues-read van de superbeheerder-tenant nodig om de taakwachtrijen te openen.",
            live: "Live",
            updated: "Bijgewerkt om {{time}}",
            error: "De taakwachtrijen konden niet worden geladen.",
            columns: {
                queue: "Wachtrij",
                ready: "Klaar",
                runningOrScheduled: "Bezig of gepland",
                oldest: "Oudste",
                processed: "Verwerkt (laatste uur)",
                lastHour: "Resultaten (laatste uur)",
                sent: "Verzonden (totaal)",
            },
            outcomes: {
                succeeded: "Geslaagd",
                failed: "Mislukt",
                expired: "Verlopen",
                rejected: "Geweigerd",
                discarded: "Verworpen",
                unknown: "Onbekend",
            },
            graphs: {
                title: "Doorvoer",
                period: "Periode",
                periods: {
                    hour: "Laatste uur",
                    sixHours: "Laatste 6 uur",
                    day: "Laatste 24 uur",
                    week: "Laatste 7 dagen",
                },
                outcomes: "Verwerkte berichten per resultaat",
                durations: "Gemiddelde wacht- en verwerkingstijd",
                wait: "Wachten",
                processing: "Verwerken",
                seconds: "Seconden",
                empty: "In deze periode zijn geen berichten van deze wachtrij verwerkt.",
            },
            messages: {
                title: "Berichten",
                states: {
                    queued: "In wachtrij",
                    archived: "Gearchiveerd",
                },
                argumentsHidden:
                    "Taakargumenten worden nooit getoond, omdat ze gegevens van kiezers kunnen bevatten.",
                refresh: "Vernieuwen",
                newest: "Nieuwste",
                newer: "Nieuwer",
                older: "Ouder",
                empty: "Geen berichten",
                unreadable: "Onleesbaar bericht",
                columns: {
                    id: "ID",
                    task: "Taak",
                    outcome: "Resultaat",
                    event: "Gebeurtenis",
                    error: "Fout",
                    enqueued: "In wachtrij gezet",
                    reads: "Keren gelezen",
                    retries: "Herhalingen",
                    archived: "Gearchiveerd",
                    visible: "Zichtbaar vanaf",
                    size: "Grootte",
                    taskId: "Taak-ID",
                },
            },
            deadLetters: {
                help: "Deze gebeurtenissen van het verkiezingslog konden niet worden verwerkt. Selecteer gebeurtenissen om ze opnieuw naar de gebeurteniswachtrij te sturen of om ze te verwerpen; verworpen gebeurtenissen blijven in het archief.",
                noWrite:
                    "U hebt de machtiging task-queues-write nodig om deze gebeurtenissen opnieuw te sturen of te verwerpen.",
                replay: "Opnieuw sturen",
                discard: "Verwerpen",
                cancel: "Annuleren",
                confirmTitle: {
                    replay: "Gebeurtenissen opnieuw sturen?",
                    discard: "Gebeurtenissen verwerpen?",
                },
                confirmBody: {
                    replay_one:
                        "{{count}} gebeurtenis wordt opnieuw naar de gebeurteniswachtrij van het verkiezingslog gestuurd. Als die opnieuw mislukt, komt die hier terug.",
                    replay_many:
                        "{{count}} gebeurtenissen worden opnieuw naar de gebeurteniswachtrij van het verkiezingslog gestuurd. Gebeurtenissen die opnieuw mislukken, komen hier terug.",
                    replay_other:
                        "{{count}} gebeurtenissen worden opnieuw naar de gebeurteniswachtrij van het verkiezingslog gestuurd. Gebeurtenissen die opnieuw mislukken, komen hier terug.",
                    discard_one:
                        "{{count}} gebeurtenis wordt naar het archief verplaatst en niet aan het verkiezingslog toegevoegd.",
                    discard_many:
                        "{{count}} gebeurtenissen worden naar het archief verplaatst en niet aan het verkiezingslog toegevoegd.",
                    discard_other:
                        "{{count}} gebeurtenissen worden naar het archief verplaatst en niet aan het verkiezingslog toegevoegd.",
                },
                queued: {
                    replay_one:
                        "Opnieuw sturen van {{count}} gebeurtenis in de wachtrij gezet als taak {{taskId}}.",
                    replay_many:
                        "Opnieuw sturen van {{count}} gebeurtenissen in de wachtrij gezet als taak {{taskId}}.",
                    replay_other:
                        "Opnieuw sturen van {{count}} gebeurtenissen in de wachtrij gezet als taak {{taskId}}.",
                    discard_one:
                        "Verwerpen van {{count}} gebeurtenis in de wachtrij gezet als taak {{taskId}}.",
                    discard_many:
                        "Verwerpen van {{count}} gebeurtenissen in de wachtrij gezet als taak {{taskId}}.",
                    discard_other:
                        "Verwerpen van {{count}} gebeurtenissen in de wachtrij gezet als taak {{taskId}}.",
                },
                failed: "De bewerking kon niet in de wachtrij worden gezet.",
            },
            tabs: {
                queues: "Wachtrijen",
                query: "Query",
            },
            query: {
                help: "Query's worden uitgevoerd op de taakwachtrijdatabase van de omgeving in een alleen-lezen transactie, met haar leesrol. Elke wachtrij heeft een tabel met wachtende berichten, pgmq.q_<wachtrij>, en een met verwerkte berichten, pgmq.a_<wachtrij>.",
                arguments:
                    "Query's lezen de berichten zoals ze zijn opgeslagen, met de argumenten van hun taken, die gegevens van kiezers kunnen bevatten.",
                placeholder: "SELECT …",
                limits: "Alleen-lezen · tot 1.000 rijen · limiet van 30 s · elke query wordt met uw gebruiker vastgelegd in de serverlogs",
                run: "Query Uitvoeren",
                summary: "Rijen: {{rows}} · {{ms}} ms",
                truncated: "Alleen de eerste {{rows}} rijen worden getoond.",
                error: "De query kon niet worden uitgevoerd.",
            },
        },
        transparencyTab: {
            title: "Transparantie",
            help: "Met Vitrine komen de stembiljetten en mijlpalen van het evenement in logs waaraan alleen iets kan worden toegevoegd en die onafhankelijke getuigen ondertekenen, zodat kiezers en waarnemers kunnen controleren dat er niets is verwijderd of gewijzigd.",
            loading: "Transparantie van het evenement laden...",
            loadError: "De transparantie van het evenement kon niet worden geladen: {{reason}}",
            unknownError: "onbekende fout",
            retry: "Opnieuw proberen",
            cancel: "Annuleren",
            copy: "Kopiëren",
            copied: "Gekopieerd",
            copyFailed: "Kopiëren mislukt",
            sdkLoading: "De Vitrine-verificatie laden...",
            sdkError:
                "De Vitrine-verificatie kon in deze browser niet starten, dus het vertrouwensbeleid kan hier niet worden gecontroleerd.",
            overview: {
                notReported: "Niet gemeld",
                ago: "{{duration}} geleden",
                runBy: "Beheerd door {{party}}",
                named: "{{name}} ({{party}})",
                health: {
                    title: {
                        healthyPublic: "Het openbare register is in orde",
                        healthyPrivate: "Het verkiezingsregister is in orde",
                        attentionPublic: "Het openbare register vraagt aandacht",
                        attentionPrivate: "Het verkiezingsregister vraagt aandacht",
                        unreported: "Het register heeft zijn toestand nog niet gemeld",
                        noPolicy: "Het register wacht op zijn vertrouwensbeleid",
                        checking: "Het vertrouwensbeleid wordt gecontroleerd",
                        unchecked: "Het vertrouwensbeleid kan hier niet worden gecontroleerd",
                        disabled: "Transparantie staat uit voor dit evenement",
                    },
                    noPolicyHelp:
                        "Upload het beleid dat de Kiescommissie heeft verzegeld en ondertekend, in het onderdeel over het vertrouwensbeleid hieronder.",
                    public: "Openbaar register",
                    private: "Privéregister",
                    quorum: "getuigenbeleid {{k}} van {{n}}",
                    hubs_one: "{{count}} ankerhub, {{independent}} onafhankelijk",
                    hubs_other: "{{count}} ankerhubs, {{independent}} onafhankelijk",
                    mmd: "maximale samenvoegvertraging {{duration}}",
                    locked: "vergrendeld sinds het openen van de stemming",
                    explorerPublic: "Openbare verkenner openen",
                    explorerPrivate: "Verkenner openen",
                    download: "Vertrouwensbeleid downloaden",
                },
                issues: {
                    storedPolicyInvalid:
                        "Het vertrouwensbeleid dat bij het evenement is opgeslagen, is niet geldig.",
                    policyIdMismatch:
                        "De logs melden een ander beleids-ID dan het beleid van het evenement.",
                    logOtherPolicy: "De log {{log}} draait onder een ander beleid.",
                    logAlert: "De log {{log}} meldt een waarschuwing: {{alert}}",
                    witnessAlert: "Getuige {{witness}} meldt een waarschuwing bij de log {{log}}.",
                },
                figures: {
                    title: "Cijfers van het register",
                    accepted: "Nu geaccepteerd",
                    record: "In het register",
                    sequencerLag: "Vertraging van de sequencer",
                    receiptTime: "Tijd tot ontvangstbewijs",
                    hubAnchor: "Anker in de hubs",
                    browserLayer: "Browserlaag",
                    overduePromises: "Verlopen beloftes",
                    evidenceReports: "Bewijsmeldingen",
                    notMeasured: "Niet gemeten in deze versie",
                    unreportedSub: "De logs hebben het nog niet gemeld",
                    recordSub_one: "items in {{count}} log, zoals door getuigen bevestigd",
                    recordSub_other: "items in {{count}} logs, zoals door getuigen bevestigd",
                    recordNone: "Nog geen door getuigen bevestigd controlepunt",
                    sequencerLagSub_one: "{{count}} item wacht",
                    sequencerLagSub_other: "{{count}} items wachten",
                    hubAnchorSub_one: "{{anchored}} van {{count}} hub, elke {{period}}",
                    hubAnchorSub_other: "{{anchored}} van {{count}} hubs, elke {{period}}",
                    noHubs: "Het vertrouwensbeleid noemt geen ankerhub",
                    browserLayerSub: "De hubs van de app verankeren in {{hubs}}",
                    browserLayerNone: "Het vertrouwensbeleid noemt geen bootstrap-hub",
                    mergeDelay: "samenvoegvertraging {{duration}}",
                    noIntake: "Niet verzameld in deze versie",
                },
                witnesses: {
                    title: "Getuigen",
                    sub: "sleutels gepubliceerd met de verkiezingsmelding",
                    signing: "Ondertekent",
                    lagging: "Loopt {{duration}} achter",
                    laggingUnknown: "Loopt achter",
                    alert: "Waarschuwing",
                },
                anchoring: {
                    title: "Verankering",
                    sub: "hubs elke {{hubs}} · browsers elke {{browsers}}",
                    independent: "Beheerd door {{party}}, onafhankelijk van {{operator}}",
                    noAnchor: "Nog geen anker",
                    browserLayer: "Browserlaag",
                    browserLayerSub: "De hubs van de app verankeren in {{hubs}}",
                },
                booth: {
                    title: "Code van het stemhokje",
                    release: "Versie niet gemeld aan dit tabblad",
                    help: "Zodra de stemming opent, komt de configuratie van elke verkiezing in het register, samen met de versie van het stemhokje. Voordat een stem wordt versleuteld, controleert het stemhokje of zijn sleutel, stemformulierstijlen, vertrouwensbeleid en versie daarmee overeenkomen.",
                    note: "Een stemhokje dat niet overeenkomt met het register weigert te versleutelen en vertelt de kiezer waarom.",
                    explorer: "Bekijk het in de verkenner",
                },
                milestones: {
                    title: "Mijlpalen in het register",
                    unreported:
                        "Dit tabblad ontvangt nog niet welke mijlpalen het register bevat: de verkenner toont ze.",
                    configuration: "Configuratie",
                    configurationSub:
                        "Sleutel, stemformulierstijlen en versie van het stemhokje, bij het openen van de stemming",
                    opened: "Stemming geopend",
                    openedSub: "Vastgelegd zodra de stemming opent",
                    anchored: "Voortdurend verankerd",
                    anchoredSub: "Hubs elke {{hubs}}, browsers elke {{browsers}}",
                    closed: "Stemming gesloten",
                    closedSub: "Zodra de laatste stem in het register staat",
                    tally: "Telling voltooid",
                    tallySub: "Eén item per telsessie",
                    results: "Resultaten gepubliceerd",
                    resultsSub: "Eén item per publicatie",
                },
                visibility: {
                    title: "Zichtbaarheid",
                    group: "Zichtbaarheid van het register",
                    locked: "Vergrendeld sinds het openen van de stemming",
                    fixed: "Vastgelegd bij het aanmaken van de logs",
                },
                redactions: {
                    title: "Gegevens achterhouden",
                    approvers_one:
                        "Het achterhouden van gegevens vereist een melding, ondertekend door {{names}}, die in het register komt. Wat al is achtergehouden, wordt niet aan dit tabblad gemeld.",
                    approvers_other:
                        "Het achterhouden van gegevens vereist een melding, ondertekend door {{k}} van de {{count}} goedkeurders ({{names}}), die in het register komt. Wat al is achtergehouden, wordt niet aan dit tabblad gemeld.",
                    none: "Dit vertrouwensbeleid staat het achterhouden van gegevens niet toe.",
                },
                monitors: {
                    title: "Onafhankelijke monitors",
                    list_one:
                        "Het vertrouwensbeleid noemt één monitor: {{names}}. Verificatiesystemen, ook de browsers van kiezers, sturen die gossip, beloftes en bewijs.",
                    list_other:
                        "Het vertrouwensbeleid noemt {{count}} monitors: {{names}}. Verificatiesystemen, ook de browsers van kiezers, sturen hun gossip, beloftes en bewijs, en een storing wordt getoond zodra {{quorum}} van hen, van verschillende partijen, die melden.",
                    none: "Het vertrouwensbeleid noemt geen monitor.",
                },
                evidence: {
                    title: "Bewijsmeldingen",
                    help: "Bewijs gaat naar de monitors van het vertrouwensbeleid. Dit tabblad ontvangt in deze versie geen kopieën.",
                },
            },
            duration: {
                seconds: "{{n}} s",
                minutes: "{{n}} min",
                hours: "{{n}} u",
            },
            mode: {
                title: "Modus",
                disabled: "Uitgeschakeld",
                public: "Openbaar",
                private: "Privé",
                disabledHelp: "Vitrine legt niets vast voor dit evenement.",
                publicHelp: "Iedereen kan de logs lezen en elk ontvangstbewijs controleren.",
                privateHelp:
                    "Alleen de auditors van het beleid kunnen de logs lezen. Kiezers controleren hun eigen ontvangstbewijzen via deze installatie.",
                locked: "Het stemmen is begonnen, dus de modus en het vertrouwensbeleid kunnen niet meer veranderen.",
                choose: "Modus om in te schakelen",
                enable: "Transparantie inschakelen",
                confirmTitle: "Transparantie inschakelen ({{mode}})?",
                confirmText:
                    "De logs van het evenement worden met deze zichtbaarheid aangemaakt, die daarna niet meer kan veranderen. Daarna stelt Harvest het concept van het vertrouwensbeleid op, dat de Kiescommissie verzegelt en ondertekent.",
                enabled:
                    "Transparantie is ingeschakeld. Download het conceptbeleid voor de Kiescommissie.",
                enableError: "Transparantie kon niet worden ingeschakeld: {{reason}}",
                refusalTitle: "Openbare transparantie is niet beschikbaar voor dit evenement",
                refusals: {
                    "public-needs-unlimited-revotes":
                        "Met openbare ontvangstbewijzen kan iedereen controleren dat er een stem is uitgebracht, dus een kiezer die onder druk staat, moet later opnieuw kunnen stemmen, zo vaak als nodig.",
                },
                refusalUnknown: "Harvest weigerde dit met de code {{code}}.",
                refusalOptions:
                    "Zet, om dit toe te staan, ‘Aantal toegestane stemmen’ op 0 (onbeperkt) in elke verkiezing van het evenement. Kies anders privétransparantie of laat transparantie uitgeschakeld.",
            },
            policy: {
                title: "Vertrouwensbeleid",
                none: "Harvest heeft het vertrouwensbeleid nog niet opgesteld.",
                draftTitle: "Concept voor de Kiescommissie",
                draftHelp:
                    "Download het concept en geef het aan de Kiescommissie. De commissie controleert, verzegelt en ondertekent het met de opdrachtregeltool van Vitrine, en laat het ook ondertekenen door elke andere instantie die de sign-regel noemt. Upload daarna hier het ondertekende beleid.",
                draftWords: "De acht woorden van het concept",
                draftWordsHelp:
                    "De commissie controleert dat ‘vitrine policy words’ deze woorden toont voor het bestand dat ze heeft ontvangen.",
                download: "Concept downloaden",
                commands: "De opdrachten van de commissie",
                invalidDraft: "Het conceptbeleid is niet geldig: {{reason}}",
                changedDraft:
                    "Het concept van het evenement is gewijzigd nadat dit beleid werd verzegeld. De commissie moet het nieuwe concept verzegelen en ondertekenen, dat daarna als opvolger wordt geüpload.",
                sealedTitle: "Het beleid van het evenement",
                sealedHelp:
                    "Publiceer de beleids-ID en de zes woorden in de officiële verkiezingsaankondiging, zodat kiezers en waarnemers het beleid herkennen waartegen hun ontvangstbewijzen worden gecontroleerd.",
                idMismatch: "De logs melden de beleids-ID {{id}}, die niet van dit beleid is.",
                invalidStored: "Het opgeslagen beleid van het evenement is niet geldig: {{reason}}",
            },
            upload: {
                title: "Ondertekend beleid uploaden",
                successorTitle: "Opvolgend beleid uploaden",
                help: "Kies het bestand met het ondertekende beleid dat de Kiescommissie terugstuurde, of plak de tekst ervan. Het wordt hier gecontroleerd voordat het wordt verzonden.",
                chooseFile: "Bestand kiezen",
                text: "Ondertekend beleid",
                readError: "Het bestand kon niet worden gelezen.",
                tooLarge: "Dit is groter dan een vertrouwensbeleid kan zijn.",
                draft: "Dit is een onverzegeld concept, met de woorden {{words}}. De Kiescommissie moet het eerst verzegelen en ondertekenen.",
                invalid: "Dit is geen geldig ondertekend beleid: {{reason}}",
                preview:
                    "Controleer deze gegevens met de Kiescommissie voordat u het beleid uploadt.",
                otherEvent: "Dit beleid is voor een ander verkiezingsevenement.",
                otherVisibility:
                    "Dit beleid is {{policy}}, maar de transparantie van het evenement is {{event}}.",
                notFromDraft:
                    "Dit beleid is niet verzegeld vanuit het huidige concept van het evenement.",
                submit: "Beleid uploaden",
                uploaded:
                    "Het beleid is geüpload. Publiceer de ID en de zes woorden in de officiële verkiezingsaankondiging.",
                error: "Het beleid kon niet worden geüpload: {{reason}}",
            },
            logs: {
                title: "Logs",
                explorer: "Transparantieverkenner openen",
                unreported: "De logs hebben hun status nog niet gemeld.",
                none: "Geen",
                unknown: "Onbekend",
                origin: "Oorsprong",
                size: "Items",
                checkpointSize: "Items in het laatste checkpoint",
                checkpointTime: "Tijd van het laatste checkpoint",
                noCheckpoint: "Nog geen checkpoint",
                policy: "Beleid",
                policyCurrent: "Het beleid van het evenement",
                policyOther: "Een ander beleid",
                policyNone: "Nog geen",
                frozen: "Bevroren",
                alert: "Waarschuwing: {{alert}}",
                queueRows: "Items in de wachtrij",
                queueOldest: "Oudste item in de wachtrij",
                witnesses: "Getuigen",
                noWitnesses: "Nog geen getuige heeft zich gemeld.",
                witness: "Getuige",
                witnessState: "Status",
                witnessLag: "Items achter",
                witnessCosigned: "Laatste handtekening",
                witnessError: "Laatste fout",
                witnessCurrent: "Bijgewerkt",
                witnessBehind: "Achter",
                witnessAlert: "Waarschuwing",
                ago: "{{duration}} geleden",
                hubs: "Ankerhubs",
                noHubs: "Geen ankerhubs.",
                hub: "Hub",
                hubSize: "Verankerde grootte",
                hubTime: "Verankerd op",
                hubNoAnchor: "Nog niet verankerd",
            },
            enrolments: {
                title: "Aanmelding van getuigen",
                help: "Stuur de aanmeldingsnotitie van elke log naar de beheerders van de getuigen van het beleid, zodat ze de checkpoints ervan kunnen gaan ondertekenen.",
                log: "Log: {{log}}",
                copy: "Notitie kopiëren",
            },
        },
    },
}

export default dutchTranslation
