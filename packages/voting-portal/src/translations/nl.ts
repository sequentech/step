// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const dutchTranslation: TranslationType = {
    translations: {
        common: {
            goBack: "Ga terug",
            showMore: "Toon meer",
            showLess: "Toon minder",
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
                helpDialog: {
                    title: "Informatie: Acclamatie",
                    content:
                        "Alle stemmingen in deze verkiezing zijn bij acclamatie beslist: de kandidaten worden zonder stemming gekozen. Omdat er geen stem is uitgebracht, is er geen Tracking-ID, ontvangstbewijs of QR-code om te verifiëren.",
                    ok: "OK",
                },
            },
            title: "Uw stem is uitgebracht",
            description:
                "De onderstaande bevestigingscode verifieert dat <b>uw stembiljet succesvol is uitgebracht</b>. U kunt deze code gebruiken om te controleren of uw stembiljet is geteld.",
            blankBallot: {
                description:
                    "Uw stembiljet is blanco uitgebracht, wat een geldige, weloverwogen keuze is.",
            },
            ballotId: "Tracking-ID",
            printButton: "Afdrukken",
            finishButton: "Voltooien",
            remainingElectionsError:
                "We konden niet controleren of u nog aan andere verkiezingen kunt deelnemen. Probeer het opnieuw.",
            retryButton: "Opnieuw proberen",
            verifyCastTitle: "Verifieer dat uw stembiljet is uitgebracht",
            verifyCastDescription:
                "U kunt op elk moment verifiëren dat uw stembiljet correct is uitgebracht met behulp van de volgende QR-code:",
            confirmationHelpDialog: {
                title: "Over het bevestigingsscherm",
                content:
                    "Dit scherm toont dat uw stem succesvol is uitgebracht. U kunt verifiëren dat het stembiljet is opgeslagen in de stembus.",
                ok: "OK",
            },
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
            ballotIdNotFoundAtFilter:
                "Niet gevonden, controleer of uw Tracking-ID correct is en tot deze gebruiker behoort.",
            filterByBallotId: "Filteren op Tracking-ID",
            totalBallots: "Aantal stembiljet: {{total}}",
            idHelp: "De korte Tracking-ID, zoals 30ff d314, of de volledige",
            foundShort:
                "Gevonden via de korte ID. Vergelijk de volledige Tracking-ID met uw ontvangstbewijs om zeker te weten dat het uw stembiljet is.",
            search: {
                title: "Controleer uw stembiljet in het openbare register",
                intro: "Voer de Tracking-ID van uw bevestigingsscherm of ontvangstbewijs in. Uw browser zoekt het stembiljet op en controleert zelf de bewijzen.",
                find: "Zoeken",
                invalid: "Een Tracking-ID bestaat alleen uit cijfers en de letters a tot en met f.",
                chips: {
                    short: "<b>Kort</b> 30ff d314",
                    full: "<b>Volledig</b> 16 groepen van vier",
                    spaces: "Spaties optioneel",
                },
                features: {
                    cast: {
                        title: "Uitgebrachte stembiljetten",
                        text: "Controleer of het in het openbare register staat, bevestigd door onafhankelijke getuigen en openbaar verankerd.",
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
                title: "{{total}} stembiljetten delen de korte ID <b>{{shortId}}</b>. Kies het uwe, of voer de volledige Tracking-ID in.",
                cast: "Uitgebracht op {{time}}",
                audited: "Geauditeerd op {{time}}",
            },
            audit: {
                title: "Uw selecties, gedecodeerd",
                note: "Uw browser heeft dit geauditeerde stembiljet ontsleuteld met de sleutel die de portal heeft onthuld. Als dit niet de selecties zijn die u hebt gemaakt, neem dan contact op met de verkiezingsautoriteit.",
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
                    "Sla het bewijsbestand op en stuur het naar de verkiezingsautoriteit of een onafhankelijke waarnemer. Zij kunnen het op hun eigen computers controleren. Het bevat niets over hoe u hebt gestemd.",
                noReceipt:
                    "Dit apparaat heeft geen ontvangstbewijs voor dit stembiljet. Controleer het op het apparaat waarmee u hebt gestemd, of controleer het bestand van het ontvangstbewijs in de stembiljetverificatie.",
                receiptPending:
                    "Dit apparaat bewaart een ontvangstbewijs voor dit stembiljet, maar het is niet compleet of kon nu niet worden gecontroleerd. Probeer het later opnieuw, of controleer het bestand van het ontvangstbewijs in de stembiljetverificatie.",
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
                        sub: "In het openbare register, bevestigd door onafhankelijke getuigen en openbaar verankerd.",
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
                        done: "Onafhankelijke getuigen hebben een register ondertekend dat het bevat.",
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
            ballotId: {
                label: "Tracking-ID",
                full: "Volledige Tracking-ID",
                yours: "Uw Tracking-ID:",
                copy: "De volledige Tracking-ID kopiëren",
                copied: "Tracking-ID gekopieerd",
                showFull: "De volledige Tracking-ID tonen",
            },
            status: {
                title: "Status van het stembiljet",
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
                        "Onafhankelijke getuigen hebben het register bevestigd. Openbare verankeringen volgen binnen {{minutes}} minuten; wachten is niet nodig.",
                    verified:
                        "Uw stembiljet staat in het openbare register, bevestigd door onafhankelijke getuigen en openbaar verankerd.",
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
                        upcoming:
                            "Onafhankelijke getuigen ondertekenen een register dat het bevat.",
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
                        "Genoeg van de onafhankelijke getuigen van het beleid ({{count}}) hebben een register ondertekend dat het bevat.",
                    anchoredWaict: "In de openbare hubs ({{count}}) en waar browsers controleren.",
                    anchoredBootstrap:
                        "In de openbare hubs ({{count}}) en waar browsers controleren, voorlopig via een vervangende hub.",
                    anchoredHubs: "In de openbare hubs ({{count}}).",
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
                    late: "Dit betekent meestal dat een hub traag of offline is; het is geen bewijs van knoeien.",
                    lateKept:
                        "Dit apparaat controleert opnieuw wanneer u terugkomt, en de stembiljettracker kan vanaf elk apparaat controleren.",
                    lateTracker: "De stembiljettracker kan vanaf elk apparaat controleren.",
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
                    kept: "Dit apparaat bewaart het ontvangstbewijs en controleert het bij elk bezoek opnieuw.",
                },
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
