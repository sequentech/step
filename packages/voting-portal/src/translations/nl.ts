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
        candidatesList: {
            collapseToggle: "Lijst {{listTitle}} in-/uitvouwen",
            showCandidates: "Kandidaten tonen",
            hideCandidates: "Kandidaten verbergen",
            selectedCandidate: "{{count}} kandidaat geselecteerd",
            selectedCandidates: "{{count}} kandidaten geselecteerd",
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
                    "Sommige van uw antwoorden maken het stembiljet voor een of meer vragen ongeldig of blanco.",
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
                "Kies uw kandidaten en beantwoord de vragen. Bewerk uw stembiljet totdat u klaar bent.",
            step2Title: "2. Controleer uw stembiljet",
            step2Description:
                "We versleutelen uw stembiljet en tonen een laatste overzicht. U ontvangt een unieke tracker-ID.",
            step3Title: "3. Breng uw stem uit",
            step3Description:
                "Breng uw stem uit zodat deze correct wordt geregistreerd, of kies voor audit om te bevestigen dat het stembiljet correct is versleuteld.",
        },
        reviewScreen: {
            title: "Controleer uw stembiljet",
            description:
                "Klik op “<b>Stembiljet bewerken</b>” voor wijzigingen, “<b>Stem uitbrengen</b>” om te bevestigen, of “<b>Controleer stembiljet</b>” voor audit.",
            descriptionNoAudit:
                "Klik op “<b>Stembiljet bewerken</b>” voor wijzigingen, of “<b>Stem uitbrengen</b>” om te bevestigen.",
            backButton: "Stembiljet bewerken",
            castBallotButton: "Stem uitbrengen",
            auditButton: "Controleer stembiljet",
            reviewScreenHelpDialog: {
                title: "Over het controlescherm",
                content:
                    "Dit scherm stelt u in staat uw selecties te controleren voordat u uw stem uitbrengt.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Stem nog niet uitgebracht",
                content:
                    "<p>Dit is uw Stembiljet Tracker ID, maar <b>uw stem is nog niet uitgebracht</b>. Als u probeert het stembiljet te traceren, zult u het niet vinden.</p><p>De reden dat we de Stembiljet Tracker ID in dit stadium tonen, is om u in staat te stellen de correctheid van het versleutelde stembiljet te auditen voordat u het uitbrengt.</p>",
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
                    "Er was een fout met het hash-proces van het stembiljet. BallotId: {{ballotId}} is niet consistent met de auditeerbare Ballot Hash: {{auditableBallotHash}}. Meld dit probleem bij de ondersteuning.",
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
                    "De stembiljet-ID komt niet overeen met de uitgebrachte stem.",
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
        },
        confirmationScreen: {
            title: "Uw stem is uitgebracht",
            description:
                "De onderstaande bevestigingscode verifieert dat <b>uw stembiljet succesvol is uitgebracht</b>. U kunt deze code gebruiken om te controleren of uw stembiljet is geteld.",
            ballotId: "Stembiljet ID",
            printButton: "Afdrukken",
            finishButton: "Voltooien",
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
                title: "Stembiljet ID",
                content: "Kan code niet gebruiken, uitgeschakeld in demo-modus.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Over het Stembiljet ID",
                content:
                    "De Stembiljet ID is een code waarmee u uw stembiljet in de stembus kunt vinden. Deze ID is uniek en bevat geen informatie over uw selecties.",
                ok: "OK",
            },
            ballotIdDemoHelpDialog: {
                title: "Over het Stembiljet ID",
                content:
                    "De Stembiljet ID is een code waarmee u uw stembiljet in de stembus kunt vinden. Deze ID is uniek en bevat geen informatie over uw selecties.",
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
                "Uw <b>Stembiljet ID</b> die bovenaan het scherm verschijnt en uw versleutelde stembiljet hieronder:",
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
                noVotingArea: "Kiesgebied niet toegewezen. Probeer het later opnieuw.",
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
        },
        errors: {
            encoding: {
                notEnoughChoices: "Niet genoeg keuzes om te decoderen",
                writeInChoiceOutOfRange: "Ingevulde keuze buiten bereik: {{index}}",
                writeInNotEndInZero: "De ingevulde tekst eindigt niet op 0",
                writeInCharsExceeded:
                    "De ingevulde tekst overschrijdt de maximale lengte met {{numCharsExceeded}} tekens. Verkorting alstublieft.",
                bytesToUtf8Conversion:
                    "Fout bij het converteren van de ingevulde tekst van bytes naar UTF-8-tekenreeks: {{errorMessage}}",
                ballotTooLarge: "Stembiljet groter dan verwacht",
            },
            implicit: {
                selectedMax:
                    "Overstemming: het aantal geselecteerde keuzes {{numSelected}} is groter dan het maximum {{max}}",
                selectedMin:
                    "Het aantal geselecteerde keuzes {{numSelected}} is kleiner dan het minimum {{min}}",
                maxSelectionsPerType:
                    "Het aantal geselecteerde keuzes {{numSelected}} voor lijst {{type}} is groter dan het maximum {{max}}",
                underVote:
                    "Onderstemming: het aantal geselecteerde keuzes {{numSelected}} is kleiner dan het maximum {{max}}",
                overVoteDisabled:
                    "Maximum bereikt: u heeft het maximum van {{numSelected}} keuzes geselecteerd. Om uw selectie te wijzigen, deselecteert u eerst een andere optie.",
                blankVote: "Blanco stem: 0 keuzes geselecteerd",
            },
            explicit: {
                notAllowed:
                    "Stembiljet expliciet als ongeldig gemarkeerd, maar de vraag staat dit niet toe",
                alert: "Deze selectie wordt geteld als een ongeldige stem",
            },
            page: {
                oopsWithStatus: "Oeps! {{status}}",
                oopsWithoutStatus: "Oeps! Onverwachte Fout",
                somethingWrong: "Er is iets misgegaan.",
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
            },
        },
        ballotLocator: {
            title: "Zoek uw Stembiljet",
            titleResult: "Resultaat van uw Stembiljet Zoekopdracht",
            description: "Verifieer dat uw stembiljet correct is ingediend",
            locate: "Zoek uw Stembiljet",
            locateAgain: "Zoek een ander Stembiljet",
            found: "Uw stembiljet ID {{ballotId}} is gevonden",
            notFound: "Uw stembiljet ID {{ballotId}} is niet gevonden",
            ambiguous:
                "Meer dan één van uw stembiljetten komt overeen met {{ballotId}}. Gebruik de volledige stembiljet-ID.",
            contentDesc: "Dit is de inhoud van uw stembiljet: ",
            wrongFormatBallotId: "Verkeerd formaat voor Stembiljet ID",
            ballotIdNotFoundAtFilter:
                "Niet gevonden, controleer dat uw Stembiljet ID correct is en behoort tot deze gebruiker.",
            filterByBallotId: "Filteren op Stembiljet ID",
            totalBallots: "Aantal stembiljet: {{total}}",
            steps: {
                lookup: "Zoek uw Stembiljet",
                result: "Resultaat",
            },
            titleHelpDialog: {
                title: "Over de stembiljet zoeker",
                content:
                    "Met de stembiljet zoeker kunt u de Stembiljet ID invoeren om uw stem te vinden en te bevestigen dat deze correct is geregistreerd.",
                ok: "OK",
            },
            tabs: {
                logs: "Logs",
                ballotLocator: "Stembiljet zoeken",
            },
            column: {
                statement_kind: "Type",
                statement_timestamp: "Tijdstip",
                username: "Gebruikersnaam",
                ballot_id: "Stembiljet ID",
                message: "Bericht",
            },
        },
    },
}

export default dutchTranslation
