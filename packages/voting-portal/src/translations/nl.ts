// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import englishTranslation, {TranslationType} from "./en"

const dutchTranslation: TranslationType = {
    translations: {
        ballotPrivacy: {
            ...englishTranslation.translations.ballotPrivacy,
            title: "Alle stembiljetten",
            subtitle:
                "Elk stembiljet dat in deze verkiezing is verzonden, één keer, bij zijn laatste stap. Kiezers staan onder een pseudoniem vermeld in plaats van een naam.",
            own: "Uw stembiljet staat in het logboek",
            copy: "Trackingcode kopiëren",
            ballotIdHelp: "Over uw trackingcode",
            stageTime: "{{stage}} {{time}}",
            noMatchHelp:
                "Controleer op typefouten. Een trackingcode bevat een koppelteken en minstens 8 tekens, zoals FTBE-MHRX. Een pseudoniem bevat een dubbele punt, zoals K7QM:2XWD.",
            copied: "Trackingcode gekopieerd",
            copyFailed:
                "De trackingcode kon niet worden gekopieerd. Selecteer de code om hem te kopiëren.",
            search: "Zoek een trackingcode of pseudoniem",
            placeholder: "FTBE-MHRX of K7QM:2XWD",
            matchesBoth:
                "{{tracking}} komen overeen met de trackingcode · {{pseudonym}} met het pseudoniem",
            ballotId: "Trackingcode",
            lastStep: "Laatste stap",
            time: "Tijd",
            trackingId: "Trackingcode",
            trackingIdHelp: "Berekend uit het versleutelde stembiljet en de sleutel van de kiezer.",
            noMatch: "Geen trackingcode of pseudoniem begint met {{prefix}}",
            noBallots: "Er zijn nog geen stembiljetten verzonden.",
            invalidPrefix:
                "Voer minstens 8 tekens van een trackingcode met koppelteken in, zoals FTBE-MHRX, of maximaal 8 van een pseudoniem met dubbele punt, zoals K7QM:2XWD. De codes gebruiken de cijfers 0 tot 9 en hoofdletters, zonder I, L, O en U.",
            policyId:
                "Dit is de beleids-ID van deze verkiezing. De Tracking-ID van een stembiljet staat op uw ontvangstbewijs.",
            stages: {
                received: {
                    label: "Ontvangen",
                    title: "Stembiljet ontvangen",
                    help: "De stembus heeft een ontvangstbewijs voor dit stembiljet ondertekend en het openbare logboek heeft beloofd het toe te voegen. Het wordt niet meegeteld tenzij de kiezer het uitbrengt.",
                    castState: "Nog niet uitgebracht",
                },
                cast: {
                    label: "Uitgebracht",
                    title: "Stembiljet uitgebracht",
                    help: "De stembus heeft bevestigd dat de kiezer dit stembiljet heeft uitgebracht.",
                    castState: "Geslaagd",
                },
                audited: {
                    label: "Gecontroleerd",
                    title: "Stembiljet gecontroleerd",
                    help: "De kiezer heeft het geopend om het te controleren. Een gecontroleerd stembiljet wordt nooit uitgebracht en nooit meegeteld.",
                    castState: "Nooit uitgebracht",
                },
            },
            checkLabels: {
                ...englishTranslation.translations.ballotPrivacy.checkLabels,
                ballotId: "Trackingcode",
            },
            checkDescriptions: {
                ...englishTranslation.translations.ballotPrivacy.checkDescriptions,
                ballotId:
                    "De trackingcode wordt berekend uit het versleutelde stembiljet en de sleutel van de kiezer.",
            },
        },
        startup: {
            error: "Het stemportaal kon niet worden gestart. Probeer het opnieuw.",
            retry: "Opnieuw proberen",
        },
        contactChannel: {
            change: "Contactmethode",
            updated: "Uw contactgegeven is gewijzigd. Codes en berichten gaan nu daarheen.",
            cancelled: "Uw contactgegeven is niet gewijzigd.",
            failed: "We konden niet wijzigen hoe we contact met u opnemen. Probeer het later opnieuw of neem contact op met de ondersteuning.",
            close: "Sluiten",
        },
        common: {
            goBack: "Ga terug",
            showMore: "Toon meer",
            showLess: "Toon minder",
        },
        audioInstructions: {
            screens: {
                "election-chooser":
                    "Deze pagina toont de verkiezingen waarin u kunt stemmen. Gebruik de Tab-toets om van de ene verkiezing naar de volgende te gaan. Elke verkiezing heeft een knop om te beginnen met stemmen. Druk op Enter op die knop om het stembiljet te openen.",
                "start":
                    "Deze pagina legt uit hoe u stemt. Eerst kiest u uw opties, daarna controleert u ze en ten slotte brengt u uw stem uit. Gebruik de Tab-toets om naar de knop te gaan waarmee het stemmen begint en druk op Enter. Als er een verklaring wordt getoond, vink dan eerst het selectievakje aan met de spatiebalk.",
                "ballot":
                    "Dit is uw stembiljet. Elke vraag is een groep opties, en de vraag vermeldt hoeveel opties u kunt kiezen. Gebruik de Tab-toets om tussen de opties te bewegen en de spatiebalk om er een te selecteren of te wissen. Ga als u klaar bent naar de knop om verder te gaan, onderaan de pagina, en druk op Enter om uw keuzes te controleren. Er is ook een knop om al uw keuzes te wissen. Er wordt niets uitgebracht totdat u op de controlepagina bevestigt.",
                "review":
                    "Deze pagina toont de keuzes die u hebt gemaakt. Controleer ze zorgvuldig. Gebruik de knop om uw stembiljet te bewerken als u iets wilt wijzigen. Ga naar de knop om uw stem uit te brengen en druk op Enter om te stemmen. Uw stembiljet-ID staat bovenaan de pagina. U kunt het kopiëren om uw stembiljet later terug te vinden.",
                "confirmation":
                    "Uw stem is uitgebracht. Deze pagina toont uw stembiljet-ID, waarmee u kunt controleren of uw stembiljet is geregistreerd. U kunt het kopiëren of uw ontvangstbewijs als pdf opslaan. Gebruik als u klaar bent de knop om af te sluiten, direct na de knop die het ontvangstbewijs opslaat.",
                "audit":
                    "Op deze pagina kunt u uw stembiljet controleren in plaats van het uit te brengen. De pagina toont de informatie die u met de stembiljetverificatie kunt nagaan. Een gecontroleerd stembiljet wordt niet uitgebracht; ga daarna dus terug naar uw stembiljet om te stemmen.",
                "ballot-locator":
                    "Op deze pagina kunt u nagaan of een stembiljet is geregistreerd. Typ of plak een stembiljet-ID in het tekstveld en druk op Enter. Het resultaat verschijnt onder het veld.",
                "support-materials":
                    "Deze pagina toont documenten die u helpen bij het stemmen. Gebruik de Tab-toets om ertussen te bewegen en druk op Enter op de knop van een document om het te openen. Gebruik de knop om terug te gaan naar de lijst met verkiezingen.",
            },
        },
        a11y: {
            skipToContent: "Ga naar hoofdinhoud",
            helpAbout: "Hulp over {{topic}}",
            previewMaterial: "Voorbeeld van {{title}}",
            ballotsTable: "Stembiljetten",
            ballotLocatorTabs: "Onderdelen van stembiljet volgen",
            ballotIdLabel: "Tracking-ID",
            votingProgress: "Voortgang van het stemmen",
            stepOf: "Stap {{current}} van {{total}}",
            selectUpTo_one: "Kies maximaal {{count}} optie",
            selectUpTo_many: "Kies maximaal {{count}} opties",
            selectUpTo_other: "Kies maximaal {{count}} opties",
            selectExactly_one: "Kies {{count}} optie",
            selectExactly_many: "Kies {{count}} opties",
            selectExactly_other: "Kies {{count}} opties",
            selectBetween: "Kies tussen {{min}} en {{max}} opties",
        },
        candidatesList: {
            collapseToggle: "Lijst {{listTitle}} in-/uitvouwen",
            showCandidates: "Kandidaten tonen",
            hideCandidates: "Kandidaten verbergen",
            selectedCandidates_one: "{{count}} kandidaat geselecteerd",
            selectedCandidates_many: "{{count}} kandidaten geselecteerd",
            selectedCandidates_other: "{{count}} kandidaten geselecteerd",
            expandAll: "Alles uitvouwen",
            collapseAll: "Alles inklappen",
        },
        breadcrumbSteps: {
            electionList: "Stembiljetten",
            ballot: "Stembiljet",
            review: "Controle",
            confirmation: "Bevestigen",
            audit: "Audit",
        },
        footer: {
            poweredBy: "Aangedreven door <1></1>",
        },
        contest: {
            acclamation: {
                description:
                    "Deze stemming is bij acclamatie beslist. De kandidaten worden zonder stemming gekozen, dus er kan niets worden geselecteerd en er wordt geen stem geregistreerd.",
            },
        },
        votingScreen: {
            backButton: "Terug",
            reviewButton: "Volgende",
            clearButton: "Selectie wissen",
            ballotHelpDialog: {
                title: "Over het stemscherm",
                content:
                    "Dit scherm toont de stemming(en) waarvoor u stemgerechtigd bent. Vink het selectievakje rechts aan om een Kandidaat/Antwoord te kiezen. Reset via “<b>Selectie wissen</b>”, ga verder via “<b>Volgende</b>”.",
                ok: "OK",
            },
            nonVotedDialog: {
                title: "Ongeldige of blanco stem",
                content:
                    "Sommige van uw antwoorden maken het stembiljet voor een of meer stemmingen ongeldig of blanco.",
                ok: "Terug en controleren",
                continue: "Doorgaan",
                cancel: "Annuleren",
            },
            warningDialog: {
                title: "Controleer uw stembiljet",
                content:
                    "Uw stembiljet bevat keuzes die mogelijk uw aandacht nodig hebben (zoals het selecteren van minder opties dan toegestaan). Uw stembiljet is geldig en zal worden geteld zoals ingediend.",
                ok: "Terug en controleren",
                continue: "Doorgaan",
                cancel: "Annuleren",
            },
            blankBallotDialog: {
                title: "U heeft geen kandidaten geselecteerd",
                content:
                    "U heeft geen selecties gemaakt. Uw stembiljet wordt uitgebracht als blanco stembiljet, wat een geldige, weloverwogen keuze is en als zodanig zal worden geteld.",
                continue: "Doorgaan",
                cancel: "Annuleren",
            },
        },
        startScreen: {
            startButton: "Begin met stemmen",
            declineToVoteButton: "Afzien van stemmen",
            declineToVoteDialog: {
                title: "Bevestig afzien van stemmen",
                content:
                    "Weet je zeker dat je wilt afzien van stemmen?<br />Je gaat direct naar de controlepagina en je deelnamestatus wordt opgeslagen als <b>Heeft afgezien van stemmen</b>.",
                continue: "Afzien van stemmen",
                cancel: "Annuleren",
            },
            instructionsTitle: "Hoe te stemmen",
            instructionsDescription: "Volg deze stappen om te stemmen:",
            step1Title: "1. Selecteer uw opties",
            step1Description:
                "Kies uw kandidaten en maak uw keuzes in elke stemming. Bewerk uw stembiljet totdat u klaar bent.",
            step2Title: "2. Controleer uw stembiljet",
            step2Description:
                "We versleutelen uw stembiljet en tonen een laatste overzicht. U ontvangt een unieke Tracking-ID.",
            step3Title: "3. Breng uw stem uit",
            step3Description:
                "Breng uw stem uit zodat deze correct wordt geregistreerd, of kies voor audit om te bevestigen dat het stembiljet correct is versleuteld.",
        },
        reviewScreen: {
            reloaded: {
                title: "Uw stembiljet is niet uitgebracht",
                text: "Door de pagina te herladen is de eenmalige sleutel gewist waarmee het wordt uitgebracht. Controleer uw keuzes en verzend uw stembiljet opnieuw. Het stembiljet dat u verzond blijft in het register staan als ontvangen en wordt nooit meegeteld.",
                back: "Terug naar mijn stembiljet",
            },
            castRecovery: {
                uncertain:
                    "Het resultaat van het uitbrengen is niet bevestigd. U kunt hetzelfde verzoek opnieuw proberen of uw trackingcode controleren voordat u opnieuw begint.",
                checkBallot: "Stembiljet controleren",
            },
            acclamation: {
                title: "Bij acclamatie beslist",
                helpDialog: {
                    title: "Informatie: Acclamatie",
                    content:
                        "Dit scherm toont wat bij acclamatie is beslist. Omdat er niets kon worden geselecteerd, wordt er geen stem uitgebracht en valt er achteraf niets te verifiëren.",
                    ok: "OK",
                },
                description:
                    "Bekijk wat bij acclamatie is beslist in deze verkiezing. Er wordt geen stem uitgebracht.",
                finishButton: "Afronden",
            },
            title: "Controleer uw stembiljet",
            description:
                "Klik op “<b>Stembiljet bewerken</b>” voor wijzigingen, “<b>Stem uitbrengen</b>” om te bevestigen, of “<b>Controleer stembiljet</b>” voor audit.",
            descriptionNoAudit:
                "Klik op “<b>Stembiljet bewerken</b>” voor wijzigingen, of “<b>Stem uitbrengen</b>” om te bevestigen.",
            backButton: "Stembiljet bewerken",
            castBallotButton: "Stem uitbrengen",
            auditButton: "Controleer stembiljet",
            copyBallotId: "Tracking-ID kopiëren",
            ballotIdCopied: "Tracking-ID gekopieerd",
            ballotIdCopyError: "Tracking-ID kon niet worden gekopieerd",
            reviewScreenHelpDialog: {
                title: "Over het controlescherm",
                content:
                    "Dit scherm stelt u in staat uw selecties te controleren voordat u uw stem uitbrengt.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Stem nog niet uitgebracht",
                content:
                    "<p>Dit is uw Tracking-ID, maar <b>uw stem is nog niet uitgebracht</b>. Als u probeert het stembiljet nu te volgen, zult u het niet vinden.</p><p>De reden dat we de Tracking-ID in dit stadium tonen, is om u in staat te stellen de correctheid van het versleutelde stembiljet te auditen voordat u het uitbrengt.</p>",
                ok: "Ik begrijp dat mijn stem nog niet is uitgebracht",
                cancel: "Annuleren",
            },
            auditBallotHelpDialog: {
                title: "Wilt u het stembiljet auditen?",
                content:
                    "<p>Let op: het auditen van uw stembiljet maakt het ongeldig, waardoor u het stemproces opnieuw moet starten. Het auditproces stelt u in staat te verifiëren dat uw stembiljet correct is gecodeerd, maar het omvat geavanceerde technische stappen. We raden aan alleen door te gaan als u zeker bent van uw technische vaardigheden. Als u gewoon uw stem wilt uitbrengen, klik dan op <u>Annuleren</u> om terug te gaan naar het controlescherm.</p>",
                ok: "Ja, ik wil mijn stembiljet verwerpen om het te auditen",
                cancel: "Annuleren",
            },
            auditConfirm: {
                title: "Dit stembiljet auditeren in plaats van uitbrengen?",
                text: "De portal publiceert hoe hij <Id>{{id}}</Id> heeft versleuteld, zodat u op een hub kunt controleren dat het bevat wat u koos.",
                never: "Het wordt nooit uitgebracht of geteld: het geheim zou laten zien hoe u stemde.",
                again: "Daarna stemt u opnieuw, met een nieuwe Tracking-ID.",
                cancel: "Annuleren",
                audit: "Stem controleren",
            },
            confirmCastVoteDialog: {
                title: "Weet u zeker dat u uw stem wilt uitbrengen?",
                content: "Na bevestiging wordt uw stem uitgebracht.",
                ok: "Ja, ik wil mijn stem uitbrengen",
                cancel: "Annuleren",
            },
            confirmCastBlankBallotDialog: {
                title: "Weet u zeker dat u een blanco stembiljet wilt uitbrengen?",
                content:
                    "U heeft geen kandidaten geselecteerd. Na bevestiging wordt uw stembiljet blanco uitgebracht.",
                ok: "Ja, ik wil mijn blanco stembiljet uitbrengen",
                cancel: "Annuleren",
            },
            unfilledContestsDialog: {
                title: "Sommige keuzes zijn niet ingevuld",
                content:
                    "U mag een keuze leeg laten of minder opties kiezen dan toegestaan. Controleer het volgende voordat u doorgaat.",
                selected: "{{selected}} van {{max}} geselecteerd",
                nothingSelected: "Niets geselecteerd",
                ok: "Doorgaan met deze keuzes",
                cancel: "Keuzes controleren",
            },
            error: {
                NETWORK_ERROR:
                    "Er was een netwerkprobleem. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                UNABLE_TO_FETCH_DATA:
                    "Er was een probleem bij het ophalen van de gegevens. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                LOAD_ELECTION_EVENT: "Kan kiesgebeurtenis niet laden. Probeer het later opnieuw.",
                CAST_VOTE:
                    "Er is een onbekende fout opgetreden bij het uitbrengen van de stem. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                CAST_VOTE_AreaNotFound:
                    "Er is een fout opgetreden bij het uitbrengen van de stem: Gebied niet gevonden. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                CAST_VOTE_CheckStatusFailed:
                    "Verkiezing staat het uitbrengen van de stem niet toe. De verkiezing is mogelijk gesloten, gearchiveerd of u probeert mogelijk buiten de respijtperiode te stemmen.",
                CAST_VOTE_InternalServerError:
                    "Er is een interne fout opgetreden bij het uitbrengen van de stem. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                CAST_VOTE_QueueError:
                    "Er was een probleem bij het verwerken van uw stem. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                CAST_VOTE_Unauthorized:
                    "U bent niet gemachtigd om een stem uit te brengen. Neem contact op met ondersteuning voor hulp.",
                CAST_VOTE_ElectionEventNotFound:
                    "De kiesgebeurtenis kon niet worden gevonden. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                CAST_VOTE_ElectoralLogNotFound:
                    "Uw stemregistratie kon niet worden gevonden. Neem contact op met ondersteuning voor hulp.",
                CAST_VOTE_CheckPreviousVotesFailed:
                    "Er is een fout opgetreden bij het controleren van uw stemstatus. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                CAST_VOTE_GetClientCredentialsFailed:
                    "Het verifiëren van uw gegevens is mislukt. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                CAST_VOTE_GetAreaIdFailed:
                    "Er is een fout opgetreden bij het verifiëren van uw stemgebied. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                CAST_VOTE_GetTransactionFailed:
                    "Er is een fout opgetreden bij het verwerken van uw stem. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                CAST_VOTE_DeserializeBallotFailed:
                    "Er is een fout opgetreden bij het lezen van uw stembiljet. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                CAST_VOTE_DeserializeContestsFailed:
                    "Er is een fout opgetreden bij het lezen van uw selecties. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                CAST_VOTE_PokValidationFailed:
                    "Het valideren van uw stem is mislukt. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                CAST_VOTE_UuidParseFailed:
                    "Er is een fout opgetreden bij het verwerken van uw verzoek. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                CAST_VOTE_unexpected:
                    "Er is een onbekende fout opgetreden bij het uitbrengen van de stem. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                CAST_VOTE_timeout:
                    "Time-out fout bij het uitbrengen van de stem. Probeer het later opnieuw of neem contact op met de ondersteuning voor hulp.",
                CAST_VOTE_InsertFailedExceedsAllowedRevotes:
                    "U heeft de limiet voor herstemmen overschreden. Probeer het later opnieuw of neem contact op met de ondersteuning voor hulp.",
                CAST_VOTE_CheckRevotesFailed:
                    "U heeft het toegestane aantal herstemmen overschreden. Probeer het later opnieuw of neem contact op met de ondersteuning voor hulp.",
                CAST_VOTE_CheckVotesInOtherAreasFailed:
                    "U heeft al in een ander gebied gestemd. Probeer het later opnieuw of neem contact op met de ondersteuning voor hulp.",
                CAST_VOTE_UnknownError:
                    "Er is een onbekende fout opgetreden bij het uitbrengen van de stem. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                NO_BALLOT_SELECTION:
                    "De selectiestatus voor deze verkiezing is niet aanwezig. Zorg ervoor dat u uw keuzes correct hebt geselecteerd of neem contact op met ondersteuning.",
                NO_BALLOT_STYLE:
                    "De stembiljetstijl is niet beschikbaar. Neem contact op met ondersteuning.",
                NO_AUDITABLE_BALLOT:
                    "Er is geen auditeerbaar stembiljet beschikbaar. Neem contact op met ondersteuning.",
                INCONSISTENT_HASH:
                    "Er was een fout met het hash-proces van het stembiljet. Tracking-ID: {{ballotId}} is niet consistent met de auditeerbare Ballot Hash: {{auditableBallotHash}}. Meld dit probleem bij de ondersteuning.",
                NOT_RECEIVED:
                    "Het openbare register bevat uw stembiljet nog niet, dus het is niet verzonden. Ga terug naar uw stembiljet en probeer het opnieuw.",
                ELECTION_EVENT_NOT_OPEN:
                    "De kiesgebeurtenis is gesloten. Neem contact op met ondersteuning.",
                PARSE_ERROR:
                    "Er was een fout bij het parseren van het stembiljet. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                DESERIALIZE_AUDITABLE_ERROR:
                    "Er was een fout bij het deserialiseren van het auditeerbare stembiljet. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                DESERIALIZE_HASHABLE_ERROR:
                    "Er was een fout bij het deserialiseren van het hashbare stembiljet. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                CONVERT_ERROR:
                    "Er was een fout bij het converteren van het stembiljet. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                SERIALIZE_ERROR:
                    "Er was een fout bij het serialiseren van het stembiljet. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                UNKNOWN_ERROR:
                    "Er is een fout opgetreden. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                REAUTH_FAILED:
                    "Authenticatie is mislukt. Probeer het opnieuw of neem contact op met ondersteuning voor hulp.",
                SESSION_EXPIRED: "Uw sessie is verlopen. Begin opnieuw vanaf het begin.",
                CAST_VOTE_BallotIdMismatch:
                    "De Tracking-ID komt niet overeen met de uitgebrachte stem.",
                SESSION_STORAGE_ERROR:
                    "Sessie-opslag is niet beschikbaar. Probeer het opnieuw of neem contact op met de ondersteuning.",
                PARSE_BALLOT_DATA_ERROR:
                    "Er is een fout opgetreden bij het verwerken van de stemgegevens. Probeer het later opnieuw of neem contact op met de ondersteuning voor hulp.",
                NOT_VALID_BALLOT_DATA_ERROR:
                    "Stemgegevens zijn niet geldig. Probeer het later opnieuw of neem contact op met de ondersteuning voor hulp.",
                FETCH_DATA_TIMEOUT_ERROR:
                    "Time-out fout bij het ophalen van de gegevens. Probeer het later opnieuw of neem contact op met de ondersteuning voor hulp.",
                TO_HASHABLE_BALLOT_ERROR:
                    "Fout bij het converteren naar hashbare stem. Probeer het later opnieuw of neem contact op met de ondersteuning voor hulp.",
                INTERNAL_ERROR:
                    "Er is een interne fout opgetreden tijdens het uitbrengen van de stem. Probeer het later opnieuw of neem contact op met de ondersteuning voor hulp.",
            },
            declineToVote: "Afzien van stemmen",
            blankBallot: "Blanco stembiljet",
        },
        confirmationScreen: {
            acclamation: {
                title: "Bij acclamatie beslist",
                description:
                    "Alle stemmingen in deze verkiezing zijn bij acclamatie beslist, dus er is geen stem uitgebracht en er is geen Tracking-ID.",
            },
            title: "Uw stem is uitgebracht",
            blankBallot: {
                description:
                    "Uw stembiljet is blanco uitgebracht, wat een geldige, weloverwogen keuze is.",
            },
            ballotId: "Tracking-ID",
            finishButton: "Voltooien",
            saveReceiptButton: "Ontvangstbewijs opslaan (pdf)",
            receipt: "Ontvangstbewijs",
            keepTitle: "Bewaar uw ontvangstbewijs",
            keepLead:
                "Deze website bewaart geen kopie. Sla het nu op om uw stembiljet later te controleren, op elk apparaat.",
            castEmailNote: "Er is ook een kopie naar u gemaild.",
            qrCaption: "Scan om het op een ander apparaat te controleren",
            remainingElectionsError:
                "We konden niet controleren of u nog aan andere verkiezingen kunt deelnemen. Probeer het opnieuw.",
            retryButton: "Opnieuw proberen",
            demoPrintDialog: {
                title: "Stembiljet afdrukken",
                content: "Afdrukken uitgeschakeld in demo-modus",
                ok: "OK",
            },
            demoBallotUrlDialog: {
                title: "Tracking-ID",
                content: "Kan code niet gebruiken, uitgeschakeld in demo-modus.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Over de Tracking-ID",
                content:
                    "De Tracking-ID is een code waarmee u uw stembiljet in de stembus kunt vinden. Deze ID is uniek en bevat geen informatie over uw selecties.",
                ok: "OK",
            },
            ballotIdDemoHelpDialog: {
                title: "Over de Tracking-ID",
                content:
                    "De Tracking-ID is een code waarmee u uw stembiljet in de stembus kunt vinden. Deze ID is uniek en bevat geen informatie over uw selecties.",
                ok: "OK",
            },
            errorDialogPrintBallotReceipt: {
                title: "Fout",
                content: "Er is een fout opgetreden, probeer het opnieuw.",
                ok: "OK",
            },
            demoQRText: "Stembiljet tracker is uitgeschakeld in demo-modus",
        },
        auditScreen: {
            confirmingAudit: "Bevestigen dat dit stembiljet niet meer kan worden uitgebracht…",
            auditUnavailable:
                "Het stembiljet kon niet worden gecontroleerd. De inhoud is niet onthuld. Probeer het opnieuw.",
            printButton: "Afdrukken",
            restartButton: "Begin met stemmen",
            title: "Controleer uw Stembiljet",
            description: "Volg de onderstaande stappen om uw stembiljet te verifiëren:",
            step1Title: "1. Download of kopieer de volgende informatie",
            step1Description:
                "Uw <b>Tracking-ID</b> die bovenaan het scherm verschijnt en uw versleutelde stembiljet hieronder:",
            step1HelpDialog: {
                title: "Kopieer het Versleutelde Stembiljet",
                content:
                    "U kunt uw versleutelde stembiljet downloaden of kopiëren om het stembiljet te auditen en te verifiëren dat de versleutelde inhoud uw selecties bevat.",
                ok: "OK",
            },
            downloadButton: "Downloaden",
            step2Title: "2. Verifieer uw stembiljet",
            step2Description:
                "<VerifierLink>Toegang tot de stembiljetverificateur</VerifierLink>, een nieuw tabblad wordt geopend in uw browser.",
            step2HelpDialog: {
                title: "Handleiding stembiljet audit",
                content:
                    "Om uw stembiljet te auditen, moet u de stappen volgen die in de handleiding worden getoond. Dit omvat het downloaden van een desktopapplicatie die wordt gebruikt om het versleutelde stembiljet onafhankelijk van de website te verifiëren.",
                ok: "OK",
            },
            bottomWarning:
                "Om veiligheidsredenen moet uw stembiljet ongeldig worden gemaakt wanneer u het audit. Om door te gaan met het stemproces, moet u hieronder op ‘<b>Begin met stemmen</b>’ klikken.",
            voteAgain: "Opnieuw stemmen",
            check: {
                title: "Controleer uw geauditeerde stembiljet",
                lead: "Dit stembiljet is geopend in plaats van uitgebracht, dus het wordt nooit geteld. Controleer op een hub dat het bevat wat u koos en stem daarna opnieuw.",
                label: "Een geauditeerd stembiljet controleren",
                encrypted: {
                    title: "Versleuteld op dit apparaat",
                    text: "Met de Tracking-ID, voordat u besloot het te auditeren.",
                },
                opened: {
                    title: "Geopend in het openbare register",
                    text: "De portal publiceerde het geheim waarmee hij het versleutelde. Het register markeert het stembiljet als Geauditeerd.",
                    publishingText: "De portal publiceert het geheim waarmee hij het versleutelde.",
                    failedTitle: "Niet geopend in het openbare register",
                    failedText:
                        "De portal kon het geheim waarmee hij het versleutelde niet publiceren, dus geen hub kan dit stembiljet nog controleren. Het stembiljet blijft geauditeerd en wordt niet geteld. Probeer het opnieuw, of stem opnieuw.",
                },
                compare: {
                    title: "Vergelijk het op een hub",
                    text: "De hub ontsleutelt het in uw browser en toont de keuzes die het bevat. Alleen u weet wat u koos.",
                },
                entry: "item #{{index}}",
                hubsLabel: "Hubs die stembiljetten controleren, uit het beleid van de verkiezing",
                runBy: "Beheerd door {{party}}",
                hubLabel:
                    "{{name}}, beheerd door {{party}}: controleer dit stembiljet op de eigen site (opent in een nieuw tabblad)",
                authorityName: "Stembiljetverificatie van de verkiezing",
                runByAuthority: "Beheerd door de verkiezingsautoriteit",
                authorityLabel:
                    "Stembiljetverificatie van de verkiezing, beheerd door de verkiezingsautoriteit: controleer dit stembiljet op de eigen site (opent in een nieuw tabblad)",
                footnote:
                    "Elke hub in het beleid van de verkiezing die stembiljetten controleert, telkens in een willekeurige volgorde. Elke hub opent op de eigen site met de Tracking-ID in de link.",
                footnoteAuthority:
                    "Nog geen hub in het beleid van de verkiezing controleert stembiljetten, dus dit is de stembiljetverificatie van de verkiezingsautoriteit zelf. Die opent met de Tracking-ID in de link.",
                trackerText:
                    "Nog geen hub in het beleid van de verkiezing controleert stembiljetten. De stembiljettracker kan het tonen: die ontsleutelt het stembiljet in uw browser.",
                trackerLink: "Open het in de stembiljettracker",
                tip: "Het sterkst op een ander apparaat, zoals uw telefoon: open de site van een hub en plak de Tracking-ID.",
                tipTracker:
                    "Het sterkst op een ander apparaat, zoals uw telefoon: open daar de stembiljettracker en plak de Tracking-ID.",
                why: "Waarom auditeren?",
                faq: {
                    why: {
                        question: "Waarom auditeren?",
                        answer: "Uw stem wordt op uw apparaat versleuteld, dus niemand kan hem lezen, ook u niet. Met auditeren controleert u of de portal heeft versleuteld wat u koos.",
                    },
                    catch: {
                        question: "Waarom betrapt dit fraude?",
                        answer: "De portal versleutelt voordat u besluit te auditeren, dus hij kan niet weten welke stembiljetten worden gecontroleerd. Als hij stemmen veranderde, zou iedereen die auditeert dat kunnen ontdekken.",
                    },
                    spent: {
                        question: "Waarom kan dit stembiljet niet meetellen?",
                        answer: "Met het geheim gepubliceerd laat het stembiljet zien hoe u stemde. Om uw stem geheim te houden, wordt een geauditeerd stembiljet nooit uitgebracht.",
                    },
                    hub: {
                        question: "Waarom op een hub?",
                        answer: "Een portal die uw stem veranderde, kan u ook een valse controle tonen. Een hub is een andere site; het stembiljet en het geheim komen uit het openbare register, niet van deze pagina.",
                    },
                },
                helpDialog: {
                    title: "Controleer uw geauditeerde stembiljet",
                    content:
                        "<p>Auditeren opent een stembiljet in plaats van het uit te brengen. De portal publiceert het geheim waarmee hij uw keuzes versleutelde, zodat iedereen het stembiljet kan ontsleutelen en zien wat het bevat.</p><p>Controleer op een hub dat het bevat wat u koos. Stem daarna opnieuw: uw nieuwe stembiljet krijgt een nieuwe Tracking-ID, en dat is het stembiljet dat telt.</p>",
                    ok: "OK",
                },
            },
        },
        electionSelectionScreen: {
            title: "Stembiljetten",
            description: "Selecteer het stembiljet waarvoor u wilt stemmen",
            chooserHelpDialog: {
                title: "Over de kieslijst",
                content:
                    "Dit scherm toont de lijst met stembiljetten waarvoor u kunt stemmen. U kunt alleen toegang krijgen als de stemperiode open is.",
                ok: "OK",
            },
            noResults: "Momenteel geen stembiljetten.",
            resultsButton: "Resultaten bekijken",
            demoDialog: {
                title: "Demo stemhokje",
                content:
                    "U betreedt een demo stemhokje. <strong>Uw stem wordt niet uitgebracht.</strong> Alleen voor demonstratiedoeleinden.",
                ok: "Ik begrijp dat mijn stem niet wordt uitgebracht",
            },
            errors: {
                noVotingArea:
                    "U staat niet geregistreerd als kiezer voor deze verkiezing. Neem contact op met de ondersteuning.",
                networkError:
                    "Er was een netwerkprobleem. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                unableToFetchData:
                    "Er was een probleem bij het ophalen van de gegevens. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                noElectionEvent:
                    "Kiesgebeurtenis bestaat niet. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                ballotStylesEmlError:
                    "Er was een fout met de gepubliceerde stembiljetstijl. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                obtainingElectionFromID:
                    "Er was een fout bij het ophalen van verkiezingen geassocieerd met de volgende verkiezings-ID's: {{electionIds}}. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
            },
            alerts: {
                noElections:
                    "Er zijn geen verkiezingen waarvoor u kunt stemmen. Dit kan zijn omdat het gebied geen bijbehorende stemmingen heeft. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
                electionEventNotPublished:
                    "De kiesgebeurtenis is nog niet gepubliceerd. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
            },
            materialsGate: {
                instructions:
                    "U moet <MaterialsLink>{{materialsTitle}}</MaterialsLink> lezen voordat u kunt stemmen.",
            },
            votingClosedAt: "De stemming is gesloten op {{close}} ({{localClose}}).",
            votingClosedOn: "De stemming is gesloten op {{close}}.",
        },
        errors: {
            encoding: {
                notEnoughChoices: "Niet genoeg keuzes om te decoderen",
                writeInChoiceOutOfRange: "Ingevulde keuze buiten bereik: {{index}}",
                writeInNotEndInZero: "De ingevulde tekst eindigt niet op 0",
                bytesToUtf8Conversion:
                    "Fout bij het converteren van de ingevulde tekst van bytes naar UTF-8-tekenreeks: {{errorMessage}}",
                ballotTooLarge: "Stembiljet groter dan verwacht",
            },
            explicit: {
                notAllowed:
                    "Stembiljet expliciet als ongeldig gemarkeerd, maar de stemming staat dit niet toe",
                alert: "Deze selectie wordt geteld als een ongeldige stem",
            },
            page: {
                oopsWithStatus: "Oeps! {{status}}",
                oopsWithoutStatus: "Oeps! Onverwachte Fout",
                somethingWrong: "Er is iets misgegaan.",
                invalidLoginHintParametersTitle: "Ongeldige stemlink",
                invalidLoginHintParametersMessage:
                    "Deze stemlink bevat ongeldige inloggegevens. Vraag een nieuwe link aan en probeer het opnieuw.",
                certAuthFailedTitle: "Certificaatauthenticatie Mislukt",
                certAuthFailedMessage:
                    "Uw certificaat kon niet worden geverifieerd. Controleer of u een geldig kiezercertificaat gebruikt en probeer het opnieuw.",
            },
        },
        materials: {
            common: {
                label: "Ondersteunend Materiaal",
                back: "Terug naar kieslijst",
                close: "Sluiten",
                preview: "Voorbeeld",
                download: "Downloaden",
            },
            mandatory: {
                checkboxLabel: "Ik heb het Ondersteunend Materiaal gelezen",
                continueButton: "Doorgaan",
                error: "Er was een probleem bij het registreren van uw bevestiging. Probeer het opnieuw.",
            },
        },
        slates: {
            title: "Lijsten",
            description:
                "Een lijst is een groep kandidaten die zich samen verkiesbaar stellen. Bij elke kandidaat staat de lijst waartoe die behoort.",
            independent: "Onafhankelijk",
            contestMembers: "Kandidaten van {{slate}} voor {{contest}}",
            noCandidate: "Geen kandidaat",
            coverage: {
                full: "Volledige lijst",
                singleContest: "Alleen {{contest}}",
                partial: "Gedeeltelijke lijst",
                candidates_one: "{{count}} kandidaat",
                candidates_other: "{{count}} kandidaten",
                offices_one: "{{count}} functie",
                offices_other: "{{count}} functies",
            },
            candidateList: {
                show: "Kandidaten tonen",
                hide: "Kandidaten verbergen",
            },
            tabs: {
                label: "Manieren om uw stembiljet in te vullen",
                slates: "Kies een lijst",
                candidates: "Individuele kandidaten",
            },
            review: {
                title: "Uw selecties",
                total: "Geselecteerde kandidaten: {{selected}} van {{seats}}",
                slate: "{{slate}}: {{status}}",
                independent: "Geselecteerde onafhankelijke kandidaten: {{count}}",
                note: "Uw stem wordt geregistreerd voor elke geselecteerde kandidaat. Een lijst is op zichzelf geen stem.",
                contestCount: "{{selected}} van {{max}} geselecteerd",
                edit: "Bewerken",
                editLabel: "{{contest}} bewerken",
            },
            selection: {
                all: "Alle {{total}} geselecteerd",
                mixed: "Gemengd · {{selected}} van {{total}} geselecteerd",
                partly: "Gedeeltelijk geselecteerd · {{selected}} van {{total}}",
                selected: "Geselecteerd",
                edit: "Selectie bewerken",
            },
            apply: {
                button: "Deze lijst kiezen",
                buttonLabel: "Lijst {{slate}} kiezen",
                chosen: "{{slate}} gekozen. Geselecteerde kandidaten: {{candidates}}. Verkiezingen: {{contests}}.",
                overMaximum:
                    "{{slate}} kan niet worden gekozen: de lijst heeft {{candidates}} kandidaten voor {{contest}}, waar {{max}} is toegestaan. U kunt nog steeds afzonderlijke kandidaten kiezen.",
                unavailable:
                    "{{slate}} kan op dit stembiljet niet worden gekozen. U kunt nog steeds afzonderlijke kandidaten kiezen.",
                replaceDialog: {
                    title: "Uw huidige keuzes vervangen?",
                    content:
                        "Als u {{slate}} kiest, worden uw keuzes in de onderstaande verkiezingen vervangen. Uw andere keuzes blijven ongewijzigd.",
                    removed: "Verwijderd:",
                    added: "In plaats daarvan geselecteerd:",
                    ok: "Keuzes vervangen",
                    cancel: "Mijn keuzes behouden",
                },
            },
        },
        ballotLocator: {
            title: "Stembiljet volgen",
            titleResult: "Stembiljet volgen",
            description: "Verifieer dat uw stembiljet correct is ingediend",
            descriptionAudited:
                "Dit stembiljet is geauditeerd en is dus nooit uitgebracht. Controleer of de hieronder gedecodeerde selecties overeenkomen met de selecties die u hebt gemaakt.",
            descriptionAuditedBrief: "Dit stembiljet is geauditeerd en is dus nooit uitgebracht.",
            locate: "Zoek uw Stembiljet",
            locateAgain: "Zoek een ander Stembiljet",
            found: "Uw Tracking-ID {{ballotId}} is gevonden",
            notFound: "Uw Tracking-ID {{ballotId}} is niet gevonden",
            contentDesc: "Dit is de inhoud van uw stembiljet: ",
            wrongFormatBallotId: "Verkeerd formaat voor Tracking-ID",
            ambiguous: "Meerdere van uw stembiljetten beginnen met {{ballotId}}. Kies er een:",
            ambiguousMany:
                "Meerdere van uw stembiljetten beginnen met {{ballotId}}. Typ meer van de Tracking-ID.",
            ballotIdNotFoundAtFilter:
                "Niet gevonden, controleer of uw Tracking-ID correct is en tot deze gebruiker behoort.",
            filterByBallotId: "Filteren op Tracking-ID",
            totalBallots: "Aantal stembiljet: {{total}}",
            idHelp: "De korte Tracking-ID, zoals FTBE-MHRX, of de volledige",
            policyId:
                "Dit is de beleids-ID van deze verkiezing. De Tracking-ID van een stembiljet staat op uw ontvangstbewijs.",
            foundShort:
                "Gevonden via de korte ID. Vergelijk de volledige Tracking-ID met uw ontvangstbewijs om zeker te weten dat het uw stembiljet is.",
            search: {
                title: "Controleer uw stembiljet in het openbare register",
                intro: "Voer de Tracking-ID van uw bevestigingsscherm of ontvangstbewijs in. Uw browser zoekt het stembiljet op en controleert zelf de bewijzen.",
                find: "Zoeken",
                invalid: "Een Tracking-ID bestaat alleen uit cijfers en letters, nooit een U.",
                chips: {
                    short: "<b>Kort</b> FTBE-MHRX",
                    full: "<b>Volledig</b> 13 groepen van vier",
                    spaces: "Spaties optioneel",
                },
                features: {
                    cast: {
                        title: "Uitgebrachte stembiljetten",
                        text: "Controleer of het in het openbare register staat, bevestigd door getuigen en openbaar verankerd.",
                    },
                    audited: {
                        title: "Geauditeerde stembiljetten",
                        text: "Bekijk de selecties die het bevat, ontsleuteld in uw browser, en controleer of dit de selecties zijn die u hebt gemaakt.",
                    },
                    secret: {
                        title: "Uw stem blijft geheim",
                        text: "Een uitgebracht stembiljet blijft versleuteld. De stembiljettracker toont dat het is geteld, nooit hoe u hebt gestemd.",
                    },
                },
            },
            matches: {
                cast: "Uitgebracht op {{time}}",
                audited: "Geauditeerd op {{time}}",
            },
            shortId: {
                head: {
                    one: "1 stembiljet begint met {{start}}",
                    other: "{{count}} stembiljetten beginnen met {{start}}",
                    atLeast: "{{count}} of meer stembiljetten beginnen met {{start}}",
                },
                sub: {
                    few: "Welke is van u? Vergelijk de gemarkeerde groep met uw bewijs, of typ die hieronder.",
                    one: "Open het en vergelijk de volledige Tracking-ID met uw bewijs.",
                    many: "Dat zijn er meer dan we tonen. Typ de derde groep van uw bewijs om het uwe te vinden.",
                },
                why: {
                    few: "Een korte ID vindt stembiljetten. Alleen de volledige Tracking-ID op uw bewijs zegt welke van u is.",
                    many: "Zoveel betekent meestal dat iemand met opzet stembiljetten heeft gemaakt die bij deze korte ID passen. Het uwe staat nog in het register: ook drie groepen laten kloppen kost ongeveer een miljard miljard pogingen.",
                },
                third: "Derde groep van uw bewijs",
                thirdOr: "Of typ de derde groep van uw bewijs",
                find: "Zoeken",
                serverSees:
                    "De servers van het register konden dit niet opzoeken zonder te weten welk stembiljet u wilt, dus de server van de verkiezing kreeg deze exacte ID te zien.",
            },
            audit: {
                kicker: "Stembiljetaudit",
                title: "Controleer of uw stembiljet is versleuteld zoals u koos",
                note: "Dit is het stembiljet dat u hebt geauditeerd, gedecodeerd door uw browser. Alleen u weet wat u hebt gekozen, dus alleen u kunt deze controle uitvoeren.",
                steps: {
                    label: "Hoe de controle werkt",
                    encrypted: {
                        title: "Versleuteld op uw apparaat",
                        text: "De portal versleutelde uw keuzes en gaf het stembiljet zijn Tracking-ID, voordat u besloot het te auditeren.",
                    },
                    decoded: {
                        title: "Hier gedecodeerd",
                        text: "Bij het auditeren onthulde de portal zijn geheim. Uw browser decodeerde het stembiljet ermee en vergeleek het met het openbare register.",
                    },
                    compare: {
                        title: "U vergelijkt",
                        text: "Bekijk de keuzes hieronder. Zijn dit de keuzes die u hebt gemaakt?",
                    },
                },
                choices: "Uit dit stembiljet gedecodeerde keuzes",
                question: "Komen deze overeen met wat u koos?",
                yes: "Ja, ze komen overeen",
                no: "Nee, er klopt iets niet",
                change: "Mijn antwoord wijzigen",
                match: {
                    title: "Uw stembiljet is versleuteld zoals u koos",
                    text: "Dit geauditeerde stembiljet is nu verbruikt: het is nooit uitgebracht en wordt nooit geteld. Ga terug en stem. Uw stem krijgt een nieuw Tracking-ID, en u kunt eerst opnieuw auditeren als u wilt.",
                    vote: "Terug en stemmen",
                },
                mismatch: {
                    title: "Stem niet op dit apparaat",
                    text: "Wat is versleuteld, is niet wat u koos. Dit apparaat of de stemportal kan dus zijn gemanipuleerd. Uw browser heeft het bewijs bewaard: het versleutelde stembiljet uit het openbare register, het geheim dat de portal onthulde en wat het decodeert.",
                    report: "Meld het bij de verkiezingsautoriteit",
                    reportNoContact: "Meld het bij de verkiezingsautoriteit.",
                    fromPolicy:
                        "Uit het gepubliceerde beleid van de verkiezing, niet van deze pagina.",
                    fromSettings: "Uit de instellingen van de verkiezing.",
                    then: "Stem daarna vanaf een ander apparaat.",
                    save: "Bewijs bewaren",
                    email: "De autoriteit mailen",
                    emailSubject: "Mijn geauditeerde stembiljet decodeert niet naar mijn keuzes",
                    emailBody:
                        "Ik heb een stembiljet in deze verkiezing geauditeerd en de keuzes die mijn browser decodeerde zijn niet de keuzes die ik maakte.\n\nTracking-ID: {{id}}\n\nIk heb het bewijs als bestand bewaard en kan het sturen.",
                },
                how: "Hoe werkt dit?",
                faq: {
                    why: {
                        question: "Waarom auditeren?",
                        answer: "Uw stem wordt op uw apparaat versleuteld, dus niemand kan hem lezen, ook u niet. Met auditeren controleert u of de portal heeft versleuteld wat u koos, voordat u echt stemt.",
                    },
                    catch: {
                        question: "Waarom betrapt dit fraude?",
                        answer: "De portal versleutelt voordat u besluit te auditeren, dus hij kan niet weten welke stembiljetten worden gecontroleerd. Als hij stemmen veranderde, zou elke kiezer die auditeert dat kunnen ontdekken.",
                    },
                    spent: {
                        question: "Waarom kan dit stembiljet niet meetellen?",
                        answer: "Met het geheim onthuld laat het stembiljet zien hoe u stemde. Om uw stem geheim te houden, wordt een geauditeerd stembiljet nooit uitgebracht.",
                    },
                    differ: {
                        question: "Wat als ze niet overeenkomen?",
                        answer: "Stem niet op dat apparaat. Bewaar het bewijs en meld het bij de verkiezingsautoriteit, waarvan het contact uit het beleid van de verkiezing komt. Stem daarna vanaf een ander apparaat.",
                    },
                    tip: "Voor de sterkste controle opent u deze pagina op een ander apparaat dan waarmee u stemde, bijvoorbeeld uw telefoon. Een apparaat dat uw stembiljet veranderde, kan ook veranderen wat het u hier toont.",
                },
                failed: {
                    title: "Audit mislukt",
                    text: "Het stembiljet in het openbare register kan niet worden ontsleuteld met de sleutel die de portal heeft onthuld. Breng geen stem uit vanaf dit apparaat; breng de verkiezingsautoriteit op de hoogte.",
                },
                unchecked: {
                    title: "De audit kon niet worden gecontroleerd",
                    text: "Dit apparaat kon dit geauditeerde stembiljet niet controleren, dus er worden geen selecties getoond. Probeer het later opnieuw, of controleer de inhoud van het stembiljet in de stembiljetverificatie.",
                },
            },
            result: {
                receipt: "Ontvangstbewijs van het stembiljet",
                idComment:
                    "Gebruik de korte of de volledige ID om dit stembiljet opnieuw te vinden.",
                fullId: "Volledige Tracking-ID",
                copy: "De volledige Tracking-ID kopiëren",
                copied: "Tracking-ID gekopieerd",
                tag: {
                    found: "Gevonden",
                    auditFailed: "Audit mislukt",
                },
                record: {
                    title: "De inhoud van uw stembiljet",
                    subtitle: "Versleuteld, zoals in het openbare register",
                },
                recordEntry: "Registeritem",
                save: "Ontvangstbewijs opslaan",
                sealLabel: "Status van het stembiljet: {{status}}",
                check: "Onafhankelijk controleren",
                caption: "Gecontroleerd door uw browser, met code die iedereen kan inspecteren.",
                captionVerified:
                    "Geverifieerd door uw browser, met code die iedereen kan inspecteren.",
                captionProblem: "Gevonden door uw browser, met code die iedereen kan inspecteren.",
                late: "Dit betekent meestal dat een hub traag of offline is; het is geen bewijs van manipulatie. Controleer later opnieuw, hier of op een ander apparaat.",
                problem:
                    "De portal meldt dit stembiljet als uitgebracht, maar het openbare register van de verkiezing heeft er geen item voor. Uw browser heeft het bewijs bewaard: het antwoord van de portal en het ondertekende controlepunt van het register. Stuur het naar de verkiezingsautoriteit of een waarnemer.",
                problemOther:
                    "De ondertekende bewijzen van dit stembiljet spreken elkaar tegen. Het kan een fout of een aanval zijn. Uw browser heeft het bewijs bewaard. Stuur het naar de verkiezingsautoriteit of een waarnemer.",
                saveEvidence: "Bewijs opslaan",
                howToReport: "Hoe meldt u het",
                howToReportText:
                    "Sla het bewijsbestand op en stuur het naar de verkiezingsautoriteit of een waarnemer. Zij kunnen het op hun eigen computers controleren. Het bevat niets over hoe u hebt gestemd.",
                noReceipt:
                    "Hier kon geen ontvangstbewijs voor dit stembiljet worden gecontroleerd. Controleer het bestand van het ontvangstbewijs dat u hebt bewaard in de stembiljetverificatie, of probeer het later opnieuw.",
                status: {
                    checking: {
                        head: "Controleren…",
                        sub: "Uw browser zoekt het stembiljet op in het openbare register.",
                    },
                    recorded: {
                        head: "In het openbare register",
                        sub: "Uw browser heeft het stembiljet gevonden. De getuigen worden gecontroleerd…",
                    },
                    witnessed: {
                        head: "Bevestigd door getuigen",
                        sub: "De openbare verankeringen worden gecontroleerd…",
                        subAnchored_one:
                            "Verankerd in openbare hubs. Waar browsers controleren volgt binnen {{count}} minuut.",
                        subAnchored_other:
                            "Verankerd in openbare hubs. Waar browsers controleren volgt binnen {{count}} minuten.",
                    },
                    verified: {
                        head: "Geverifieerd",
                        sub: "In het openbare register, bevestigd door getuigen en openbaar verankerd.",
                    },
                    late: {
                        head: "Duurt langer dan verwacht",
                        sub: "In het openbare register en bevestigd door getuigen, maar de openbare verankeringen zijn te laat.",
                    },
                    problem: {
                        head: "Niet in het openbare register",
                        sub: "Uw browser kon dit stembiljet niet vinden in het openbare register van de verkiezing.",
                    },
                    otherProblem: {
                        head: "Een controle is mislukt",
                        sub: "De ondertekende bewijzen van dit stembiljet spreken elkaar tegen. Bekijk de stap die mislukte.",
                    },
                },
                steps: {
                    record: {
                        title: "Toegevoegd aan het openbare register",
                        done: "Item #{{index}} in het stembiljetlogboek van de verkiezing. Uw browser heeft het bewijs ervan gecontroleerd.",
                        failed: "Geen enkel item in het stembiljetlogboek van de verkiezing heeft deze Tracking-ID.",
                        missing: "Ontbreekt",
                    },
                    witnesses: {
                        title: "Bevestigd door getuigen",
                        done: "Getuigen hebben een register ondertekend dat het bevat.",
                        count: "{{signed}} van {{total}} · {{needed}} nodig",
                        countOf: "{{signed}} van {{total}}",
                    },
                    hubs: {
                        title_one: "Verankerd in {{count}} openbare hub",
                        title_other: "Verankerd in {{count}} openbare hubs",
                        titleAny: "Verankerd in openbare hubs",
                        done: "Gedeelde logboeken bevatten dit register ook: {{hubs}}.",
                    },
                    browser: {
                        title: "Verankerd waar browsers controleren",
                        done: "Bewaard door een tijdelijke hub totdat browsers zelf logboeken controleren.",
                    },
                    checking: "Uw browser controleert…",
                    scheduled_one: "Binnen {{count}} minuut. U hoeft niet te wachten.",
                    scheduled_other: "Binnen {{count}} minuten. U hoeft niet te wachten.",
                    by: "Vóór {{time}}",
                    late: "Te laat",
                    lateHubs: "De hubs hadden het vóór {{time}} moeten verankeren.",
                    lateOther: "Dit had vóór {{time}} moeten gebeuren.",
                    waiting: "Wacht op de stap hierboven.",
                    skipped: "Niet gecontroleerd.",
                    failedStep: "De ondertekende bewijzen van deze stap spreken elkaar tegen.",
                    failedTime: "Probleem",
                },
            },
            checksAvailableUntil: "U kunt uw stembiljet controleren tot {{date}}.",
            checksEnded: "De controles zijn beëindigd op {{date}}.",
            castAt: "Uitgebracht op {{date}}",
            steps: {
                lookup: "Zoek uw Stembiljet",
                result: "Resultaat",
            },
            titleHelpDialog: {
                title: "Over het volgen van stembiljetten",
                content:
                    "Met dit scherm kunt u de Tracking-ID invoeren om uw stem te vinden en te bevestigen dat deze correct is geregistreerd.",
                ok: "OK",
            },
            tabs: {
                logs: "Logs",
                ballotLocator: "Stembiljet volgen",
            },
            column: {
                statement_kind: "Type",
                statement_timestamp: "Tijdstip",
                ballot_id: "Tracking-ID",
            },
        },
        vitrine: {
            hubProblems: {
                title: {
                    one: "Een hub meldt een probleem voor deze verkiezing",
                    other: "Er worden {{count}} problemen gemeld voor deze verkiezing",
                },
                listedBy: "Gemeld door {{hubs}}",
                proof: "Bekijk het bewijs",
                note: "Deze melding is een gemak van deze site, en een vermelding is slechts een verwijzing: open het bewijs om het in uw browser te controleren. De hubs laten het u ook weten, per e-mail, op hun eigen sites en in de extensie, of deze site dat nu doet of niet.",
                kind: {
                    splitView: "Twee versies van het register",
                    brokenPromise: "Een beloofde invoer ontbreekt",
                    breaksRules: "Een invoer overtreedt de regels",
                    twoPolicies: "Twee beleidsstukken",
                    unloggedCode: "Code die niet is vastgelegd",
                    other: "Een probleem vastgelegd door een hub",
                },
            },
            ballotId: {
                label: "Tracking-ID",
                full: "Volledige Tracking-ID",
                yours: "Uw Tracking-ID:",
                copy: "De volledige Tracking-ID kopiëren",
                copied: "Tracking-ID gekopieerd",
                copyText: "Kopiëren",
                copiedText: "Gekopieerd",
                showFull: "De volledige Tracking-ID tonen",
                selected: "Geselecteerd. Druk op Ctrl+C, of ⌘C op een Mac, om te kopiëren.",
                copyByHand:
                    "Je browser kopieerde het niet. Open de volledige Tracking-ID om hem te selecteren.",
            },
            elsewhere: {
                title: "Elders opzoeken",
                count_one: "{{count}} verkenner",
                count_other: "{{count}} verkenners",
                introBallot:
                    "Verkenners van anderen lezen zelf het openbare register. Elk zou je stem moeten vinden, met dezelfde controles, zonder deze website.",
                introEntry:
                    "Verkenners van anderen lezen zelf het openbare register. Elk zou deze invoer precies zo moeten tonen als hier.",
                foot: "De lijst komt uit het beleid van de verkiezing. Elke Vitrine-verkenner kan hetzelfde register openen: geef hem de volledige Tracking-ID.",
                name: "Verkenner van {{name}}",
                operatorName: "Verkenner van de verkiezing",
                newTab: "Opent in een nieuw tabblad",
                who: {
                    monitor:
                        "Beheerd door {{party}}, een monitor die in het beleid van de verkiezing staat",
                    hub: "Beheerd door {{party}}, een ankerhub die in het beleid van de verkiezing staat",
                    operator: "Beheerd door de verkiezingsautoriteit, die deze verkiezing beheert",
                },
            },
            status: {
                title: "Status van het stembiljet",
                audit: "Stem controleren",
                auditNote:
                    "Niet zeker of het portaal je keuzes goed heeft versleuteld? Controleer deze stem: het portaal laat zien hoe hij is versleuteld, zodat je het kunt nagaan. Een gecontroleerde stem kan nooit worden uitgebracht, dus je stemt daarna opnieuw.",
                cast: "Uitgebracht op {{when}}",
                audited: "Geauditeerd op {{when}}",
                ballotCast: "Stem uitgebracht",
                aria: "Status van het stembiljet: {{state}}. Details tonen",
                ariaCast: "Stem uitgebracht. Status van het stembiljet: {{state}}. Details tonen",
                tooltip: "{{state}} · Details tonen",
                tracker: "Openen in de stembiljettracker",
                save: "Ontvangstbewijs opslaan",
                ok: "OK",
                tag: {
                    cast: "Uitgebracht",
                    notCast: "Niet uitgebracht",
                    auditable: "Auditeerbaar",
                    audited: "Gecontroleerd",
                },
                state: {
                    notcast: "Nog niet uitgebracht",
                    checking: "Controleren…",
                    recorded: "In het openbare register",
                    witnessed: "Bevestigd door getuigen",
                    verified: "Geverifieerd",
                    late: "Vertraagd",
                    problem: "Probleem",
                    auditable: "Auditeerbaar",
                    auditfail: "Audit mislukt",
                },
                head: {
                    late: "Duurt langer dan verwacht",
                    mismatch: "Het register komt niet overeen",
                    damaged: "Het ontvangstbewijs is beschadigd",
                    noReceipt: "Geen ontvangstbewijs voor uw stembiljet",
                    unchecked: "Uw stembiljet kon niet worden gecontroleerd",
                },
                sub: {
                    notcast:
                        "Dit stembiljet is op dit apparaat versleuteld en er is niets verzonden. Zodra u het uitbrengt, controleert uw browser of het in het openbare register terechtkomt.",
                    checking:
                        "Uw browser controleert of uw stembiljet in het openbare register staat.",
                    recorded:
                        "Uw browser heeft uw stembiljet in het openbare register gevonden. Daarna bevestigen getuigen het.",
                    witnessed:
                        "Getuigen hebben het register bevestigd. Openbare verankeringen volgen binnen {{minutes}} minuten; wachten is niet nodig.",
                    verified:
                        "Uw stembiljet staat in het openbare register, bevestigd door getuigen en openbaar verankerd.",
                    late: "Uw stembiljet staat in het openbare register en is bevestigd door getuigen, maar de openbare verankeringen zijn vertraagd.",
                    lateStep:
                        "Een stap duurt langer dan verwacht. Dat is geen bewijs van knoeien, en uw browser blijft controleren.",
                    mismatch:
                        "Item #{{index}} in het openbare register is niet het item in dit ontvangstbewijs.",
                    mismatchNoEntry:
                        "Het item in het openbare register is niet het item in dit ontvangstbewijs.",
                    damaged:
                        "Een deel van het ontvangstbewijs of het register kon niet worden gelezen of geverifieerd. Dat is een gegevensfout, geen bewijs van knoeien. Uw stem is uitgebracht.",
                    noReceipt:
                        "Deze website heeft geen ontvangstbewijs voor uw stembiljet gekregen. Uw stem is uitgebracht en de waarnemers van de verkiezing zijn op de hoogte gebracht.",
                    unchecked:
                        "Deze website kon het ontvangstbewijs niet controleren. Uw stem is uitgebracht.",
                    auditable:
                        "U hebt dit stembiljet geauditeerd, dus het is nooit uitgebracht. Het staat in het openbare register en de stembiljettracker ontsleutelt het om de selecties te tonen die het bevat.",
                    auditfail:
                        "Het stembiljet in het openbare register kan niet worden ontsleuteld met de sleutel die de portal heeft onthuld. Breng geen stem uit vanaf dit apparaat; breng de verkiezingsautoriteit op de hoogte.",
                },
                step: {
                    record: {
                        title: "In het openbare register",
                        upcoming:
                            "Uw browser vindt het stembiljet in het openbare register van de verkiezing.",
                    },
                    witnesses: {
                        title: "Bevestigd door getuigen",
                        upcoming: "Getuigen ondertekenen een register dat het bevat.",
                    },
                    anchors: {
                        title: "Openbaar verankerd",
                        upcoming:
                            "Openbare hubs en de logboeken die browsers controleren, bewaren dat register.",
                    },
                },
                row: {
                    waiting: "Wacht op de vorige stap.",
                    checking: "Uw browser controleert…",
                    anchoring:
                        "Openbare hubs, daarna waar browsers controleren: binnen {{minutes}} minuten.",
                    recorded: "Uw browser heeft het bewijs gecontroleerd.",
                    recordedEntry: "Item #{{index}}. Uw browser heeft het bewijs gecontroleerd.",
                    witnessed:
                        "Genoeg van de getuigen van het beleid ({{count}}) hebben een register ondertekend dat het bevat.",
                    anchoredWaict: "In de openbare hubs ({{count}}) en waar browsers controleren.",
                    anchoredBootstrap:
                        "In de openbare hubs ({{count}}) en waar browsers controleren, voorlopig via een vervangende hub.",
                    anchoredHubs: "In de openbare hubs ({{count}}).",
                    anchoredOneHub: "Verankerd in 1 hub, beheerd door {{party}}.",
                    anchoredOneHubPlatform: "Verankerd in 1 hub, beheerd door het platform.",
                    lateNow: "Het had nu klaar moeten zijn.",
                    lateStep: "Het had om {{time}} klaar moeten zijn.",
                    lateAnchors: "De hubs hadden het om {{time}} moeten verankeren.",
                    mismatch: "Item #{{index}} in het register wijkt af van uw ontvangstbewijs.",
                    mismatchNoEntry: "Een item in het register wijkt af van uw ontvangstbewijs.",
                    contradiction: "De ondertekende bewijzen spreken elkaar tegen.",
                    damaged: "Het bewijs is beschadigd en kon niet worden geverifieerd.",
                    noReceipt: "Deze website heeft geen ontvangstbewijs gekregen.",
                    unchecked: "Deze website kon het ontvangstbewijs niet controleren.",
                    skipped: "Niet gecontroleerd, omdat de vorige stap niet kon worden voltooid.",
                    skippedMismatch: "Niet gecontroleerd, omdat het register niet overeenkomt.",
                },
                word: {
                    withinSeconds: "Binnen enkele seconden",
                    withinMinutes: "Binnen {{minutes}} min",
                    checking: "Controleren…",
                    late: "Vertraagd",
                    mismatch: "Komt niet overeen",
                    damaged: "Beschadigd",
                    noReceipt: "Geen bewijs",
                    unchecked: "Niet gecontroleerd",
                    skipped: "—",
                },
                callout: {
                    late: "Dit betekent meestal dat een hub traag of offline is; het is geen bewijs van knoeien. Controleer later opnieuw met uw ontvangstbewijs, in de stembiljettracker of op een hub, vanaf elk apparaat.",
                    evidence:
                        "Uw browser heeft het bewijs bewaard: dit ontvangstbewijs en het ondertekende item van het register. Stuur het naar de verkiezingsautoriteit of een waarnemer; iedereen kan het controleren.",
                    saveEvidence: "Bewijs opslaan",
                },
                caption: {
                    notcast:
                        "Er wordt niets gecontroleerd totdat u uw stem uitbrengt. Daarna laat dit vinkje zien hoe ver uw stembiljet is geverifieerd.",
                    checking:
                        "Uw browser controleert het openbare register, met code die iedereen kan inspecteren.",
                    problem: "Gevonden door uw browser, met code die iedereen kan inspecteren.",
                    auditable:
                        "Ontsleuteld door uw browser, met code die iedereen kan inspecteren.",
                    verified:
                        "Geverifieerd door uw browser, met code die iedereen kan inspecteren.",
                    checked:
                        "Gecontroleerd door uw browser, met code die iedereen kan inspecteren.",
                    keep: "Bewaar uw ontvangstbewijs om het later opnieuw te controleren, in de stembiljettracker of op een hub.",
                },
            },
            signers: {
                witnesses: {open: "Wie heeft getekend", close: "Verberg wie heeft getekend"},
                hubs: {open: "Welke hubs", close: "Verberg welke hubs"},
                runBy: "Beheerd door {{party}}",
                runByAlso: "Beheerd door {{party}}, dat ook {{hubs}} beheert",
                signed: "Getekend om {{time}}",
                signedNoTime: "Getekend",
                notYet: "Nog niet",
                late: "Vertraagd",
                anchored: "Verankerd om {{time}}",
                anchoredNoTime: "Verankerd",
                within: "Binnen {{minutes}} min",
                check: "Controleer op zijn site",
                checkWitness:
                    "Controleer op zijn site: getuige {{name}} (opent in een nieuw tabblad)",
                checkHub: "Controleer op zijn site: hub {{name}} (opent in een nieuw tabblad)",
                linksNote:
                    "“Controleer op zijn site” opent de eigen verkenner van de ondertekenaar, die die handtekening daar controleert.",
                standIn: {name: "Waar browsers controleren", who: "Voorlopig een vervangende hub"},
                rule: {
                    witnesses:
                        "Het beleid vereist dat {{needed}} van zijn {{total}} getuigen tekenen. {{signed}} hebben getekend.",
                    witnessesAlone:
                        "Het beleid vereist dat {{needed}} van zijn {{total}} getuigen tekenen, en geen enkele partij beheert er zoveel. {{signed}} hebben getekend.",
                    witnessesCount:
                        "{{signed}} van de {{total}} getuigen van het beleid hebben getekend.",
                    hubs: "Alle {{count}} hubs moeten het register verankeren, en minstens {{independent}} moeten worden beheerd door iemand anders dan de verkiezingsautoriteit.",
                    hubsPlatform:
                        "Alle {{count}} hubs moeten het register verankeren, en minstens {{independent}} moeten worden beheerd door iemand die niet de verkiezingsautoriteit of het platform is.",
                    hubsAll: "Alle {{count}} hubs moeten het register verankeren.",
                    oneHub: "Het beleid noemt één hub, beheerd door {{party}}.",
                    oneHubPlatform: "Het beleid noemt één hub, beheerd door het platform.",
                },
            },
            updates: {
                optional: "Optioneel",
                title: "Ontvang e-mailupdates over dit stembiljet",
                description:
                    "Wanneer het is bevestigd en wanneer het is geteld, of als er een probleem wordt vastgelegd. Kies een willekeurige hub: elke hub controleert het register zelf.",
                listLabel: "Hubs in het beleid van deze verkiezing",
                runBy: "Beheerd door {{party}}",
                hubLabel:
                    "{{name}}, beheerd door {{party}}: e-mailupdates ontvangen op zijn eigen site (opent in een nieuw tabblad)",
                footnote:
                    "De hubs die het beleid van de verkiezing noemt, telkens in willekeurige volgorde. Elke hub opent op zijn eigen site met uw ontvangstbewijs in de link; er wordt niets gedeeld totdat u daar kiest wat u wilt volgen.",
                footnoteNoReceipt:
                    "De hubs die het beleid van de verkiezing noemt, telkens in willekeurige volgorde. Elke hub opent op zijn eigen site; er wordt niets gedeeld totdat u daar kiest wat u wilt volgen.",
            },
            booth: {
                title: "Uw stembiljet kan nog niet worden voorbereid",
                explanation:
                    "Voordat deze website uw stembiljet versleutelt, controleert ze of de instellingen van de verkiezing dezelfde zijn als die in het openbare register van de verkiezing. Uw stembiljet is niet versleuteld of verzonden, en uw keuzes blijven bewaard.",
                pending:
                    "De gepubliceerde instellingen zijn nog niet beschikbaar. Probeer het zo meteen opnieuw.",
                invalid:
                    "De gepubliceerde instellingen konden niet worden gelezen. Probeer het zo meteen opnieuw.",
                failed: "De instellingen komen niet overeen met de gepubliceerde. De waarnemers van de verkiezing zijn op de hoogte gebracht. Neem contact op met de ondersteuning.",
                retry: "Opnieuw proberen",
                close: "Sluiten",
            },
            record: {
                json: "JSON",
                text: "Tekst",
                fields_one: "{{count}} veld",
                fields_other: "{{count}} velden",
                items_one: "{{count}} item",
                items_other: "{{count}} items",
                size: {
                    bytes: "{{value}} B",
                    kilobytes: "{{value}} kB",
                },
                copy: "{{file}} kopiëren",
                copied: "Gekopieerd",
                download: "{{file}} downloaden",
            },
        },
    },
}

export default dutchTranslation
