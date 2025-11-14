// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
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
        breadcrumbSteps: {
            electionList: "Kieslijst",
            ballot: "Stembiljet",
            review: "Controle",
            confirmation: "Bevestiging",
            audit: "Audit",
        },
        footer: {
            poweredBy: "Aangedreven door <sequent />",
        },
        votingScreen: {
            backButton: "Terug",
            reviewButton: "Volgende",
            clearButton: "Selectie wissen",
            ballotHelpDialog: {
                title: "Informatie: Stemscherm",
                content:
                    "Dit scherm toont de stemming(en) waarvoor u stemgerechtigd bent. U kunt uw selectie maken door het selectievakje rechts van de Kandidaat/Antwoord aan te vinken. Om uw selecties te resetten, klik op de knop “<b>Selectie wissen</b>”. Om naar de volgende stap te gaan, klik op de knop “<b>Volgende</b>” hieronder.",
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
            instructionsTitle: "Instructies",
            instructionsDescription: "Volg deze stappen om uw stem uit te brengen:",
            step1Title: "1. Selecteer uw opties",
            step1Description:
                "Kies uw voorkeurskandidaten en beantwoord de vragen op het stembiljet een voor een zoals ze verschijnen. U kunt uw stembiljet bewerken totdat u klaar bent om verder te gaan.",
            step2Title: "2. Controleer uw stembiljet",
            step2Description:
                "Zodra u tevreden bent met uw selecties, versleutelen we uw stembiljet en tonen we u een laatste overzicht van uw keuzes. U ontvangt ook een unieke tracker-ID voor uw stembiljet.",
            step3Title: "3. Breng uw stem uit",
            step3Description:
                "Breng uw stem uit: Tot slot kunt u uw stem uitbrengen zodat deze correct wordt geregistreerd. Als alternatief kunt u kiezen voor een audit om te bevestigen dat uw stembiljet correct is vastgelegd en versleuteld.",
        },
        reviewScreen: {
            title: "Controleer uw stembiljet",
            description:
                "Om wijzigingen aan te brengen in uw selecties, klik op de knop “<b>Stembiljet bewerken</b>”. Om uw selecties te bevestigen, klik op de knop “<b>Breng uw stem uit</b>” hieronder. Om uw stembiljet te auditen, klik op de knop “<b>Audit stembiljet</b>” hieronder.",
            descriptionNoAudit:
                "Om wijzigingen aan te brengen in uw selecties, klik op de knop “<b>Stembiljet bewerken</b>”. Om uw selecties te bevestigen, klik op de knop “<b>Breng uw stem uit</b>” hieronder.",
            backButton: "Stembiljet bewerken",
            castBallotButton: "Breng uw stem uit",
            auditButton: "Audit stembiljet",
            reviewScreenHelpDialog: {
                title: "Informatie: Controlescherm",
                content:
                    "Dit scherm stelt u in staat uw selecties te controleren voordat u uw stem uitbrengt.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Stem nog niet uitgebracht",
                content:
                    "<p>Dit is uw Stembiljet Tracker ID, maar <b>uw stem is nog niet uitgebracht</b>. Als u probeert het stembiljet te traceren, zult u het niet vinden.</p><p>De reden dat we de Stembiljet Tracker ID in dit stadium tonen, is om u in staat te stellen de correctheid van het versleutelde stembiljet te auditen voordat u het uitbrengt.</p>",
                ok: "Ik accepteer dat mijn stem NOG NIET is uitgebracht",
                cancel: "Annuleren",
            },
            auditBallotHelpDialog: {
                title: "Wilt u het stembiljet auditen?",
                content:
                    "<p>Let op: het auditen van uw stembiljet maakt het ongeldig, waardoor u het stemproces opnieuw moet starten. Het auditproces stelt u in staat te verifiëren dat uw stembiljet correct is gecodeerd, maar het omvat geavanceerde technische stappen. We raden aan alleen door te gaan als u zeker bent van uw technische vaardigheden. Als u gewoon uw stem wilt uitbrengen, klik dan op <u>Annuleren</u> om terug te gaan naar het controlescherm.</b></p>",
                ok: "Ja, ik wil mijn stembiljet VERWERPEN om het te auditen",
                cancel: "Annuleren",
            },
            confirmCastVoteDialog: {
                title: "Weet u zeker dat u uw stem wilt uitbrengen?",
                content: "Uw stem kan niet meer worden bewerkt nadat deze is bevestigd.",
                ok: "Ja, ik wil mijn stem UITBRENGEN",
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
                title: "Informatie: Bevestigingsscherm",
                content:
                    "Dit scherm toont dat uw stem succesvol is uitgebracht. De informatie op deze pagina stelt u in staat te verifiëren dat het stembiljet is opgeslagen in de stembus. Dit proces kan op elk moment worden uitgevoerd tijdens de stemperiode en nadat de stemming is gesloten.",
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
                title: "Informatie: Stembiljet ID",
                content:
                    "De Stembiljet ID is een code waarmee u uw stembiljet in de stembus kunt vinden. Deze ID is uniek en bevat geen informatie over uw selecties.",
                ok: "OK",
            },
            ballotIdDemoHelpDialog: {
                title: "Informatie: Stembiljet ID",
                content:
                    "<p>De Stembiljet ID is een code waarmee u uw stembiljet in de stembus kunt vinden. Deze ID is uniek en bevat geen informatie over uw selecties.</p><p><b>Let op:</b> Dit stemhokje is alleen voor demonstratiedoeleinden. Uw stem is NIET uitgebracht.</p>",
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
            title: "Audit uw Stembiljet",
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
            title: "Kieslijst",
            description: "Selecteer het stembiljet waarvoor u wilt stemmen",
            chooserHelpDialog: {
                title: "Informatie: Kieslijst",
                content:
                    "Welkom bij het Stemhokje. Dit scherm toont de lijst met stembiljetten waarvoor u een stem kunt uitbrengen. Stembiljetten die in deze lijst worden weergegeven, kunnen open zijn voor stemming, gepland zijn of gesloten zijn. U kunt alleen toegang krijgen tot het stembiljet als de stemperiode open is.",
                ok: "OK",
            },
            noResults: "Momenteel geen stembiljetten.",
            demoDialog: {
                title: "Demo stemhokje",
                content:
                    "U betreedt een demo stemhokje. <strong>Uw stem wordt NIET uitgebracht.</strong> Dit stemhokje is alleen voor demonstratiedoeleinden.",
                ok: "Ik accepteer dat mijn stem niet wordt uitgebracht",
            },
            errors: {
                noVotingArea:
                    "Kiesgebied niet toegewezen aan kiezer. Probeer het later opnieuw of neem contact op met ondersteuning voor hulp.",
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
                writeInChoiceOutOfRange: "In te vullen keuze buiten bereik: {{index}}",
                writeInNotEndInZero: "In te vullen tekst eindigt niet op 0",
                writeInCharsExceeded:
                    "In te vullen tekst overschrijdt maximum aantal tekens met {{numCharsExceeded}}. Moet worden aangepast.",
                bytesToUtf8Conversion:
                    "Fout bij het converteren van in te vullen tekst van bytes naar UTF-8 string: {{errorMessage}}",
                ballotTooLarge: "Stembiljet groter dan verwacht",
            },
            implicit: {
                selectedMax:
                    "Te veel stemmen: Aantal geselecteerde keuzes {{numSelected}} is meer dan het maximum {{max}}",
                selectedMin:
                    "Aantal geselecteerde keuzes {{numSelected}} is minder dan het minimum {{min}}",
                maxSelectionsPerType:
                    "Aantal geselecteerde keuzes {{numSelected}} voor lijst {{type}} is meer dan het maximum {{max}}",
                underVote:
                    "Te weinig stemmen: Aantal geselecteerde keuzes {{numSelected}} is minder dan het maximum {{max}}",
                overVoteDisabled:
                    "Maximum bereikt: U heeft het maximum aantal keuzes {{numSelected}} geselecteerd. Om uw selectie te wijzigen, deselecteer eerst een andere optie.",
                blankVote: "Blanco stem: 0 keuzes geselecteerd",
                preferenceOrderWithGaps: "De voorkeursvolgorde heeft een of meer hiaten.",
                duplicatedPosition:
                    "Dezelfde positie is geselecteerd voor twee of meer kandidaten.",
            },
            explicit: {
                notAllowed:
                    "Stembiljet expliciet ongeldig gemarkeerd maar vraag staat dit niet toe",
                alert: "Gemarkeerde selectie wordt als ongeldige stem beschouwd.",
            },
            page: {
                oopsWithStatus: "Oeps! {{status}}",
                oopsWithoutStatus: "Oeps! Onverwachte Fout",
                somethingWrong: "Er is iets misgegaan.",
            },
        },
        materials: {
            common: {
                label: "Ondersteunend Materiaal",
                back: "Terug naar Kieslijst",
                close: "Sluiten",
                preview: "Voorbeeld",
            },
        },
        ballotLocator: {
            title: "Lokaliseer uw Stembiljet",
            titleResult: "Resultaat van uw Stembiljet Zoekopdracht",
            description: "Verifieer dat uw stembiljet correct is ingediend",
            locate: "Lokaliseer uw Stembiljet",
            locateAgain: "Lokaliseer een ander Stembiljet",
            found: "Uw stembiljet ID {{ballotId}} is gelokaliseerd",
            notFound: "Uw stembiljet ID {{ballotId}} is niet gelokaliseerd",
            contentDesc: "Dit is de inhoud van uw stembiljet: ",
            wrongFormatBallotId: "Verkeerd formaat voor Stembiljet ID",
            ballotIdNotFoundAtFilter:
                "Niet gevonden, controleer dat uw Stembiljet ID correct is en behoort tot deze gebruiker.",
            filterByBallotId: "Filteren op Stembiljet ID",
            totalBallots: "Aantal stembiljet: {{total}}",
            steps: {
                lookup: "Lokaliseer uw Stembiljet",
                result: "Resultaat",
            },
            titleHelpDialog: {
                title: "Informatie: Stembiljet Lokaliseren Scherm",
                content:
                    "Dit scherm stelt de kiezer in staat om zijn/haar stem te vinden door de Stembiljet ID te gebruiken om deze op te halen. Deze procedure maakt het mogelijk te controleren of hun stembiljet correct is uitgebracht en of het geregistreerde stembiljet overeenkomt met het versleutelde stembiljet dat ze hebben verzonden.",
                ok: "OK",
            },
            tabs: {
                logs: "Logs",
                ballotLocator: "Lokaliseer uw Stembiljet",
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
