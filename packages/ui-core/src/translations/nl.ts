// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const dutchTranslation: TranslationType = {
    translations: {
        language: "Nederlands",
        welcome: "Hello <br/> <strong>World</strong>",
        breadcrumbSteps: {
            select: "Selecteer een verificateur",
            import: "Gegevens importeren",
            verify: "Verifiëren",
            finish: "Voltooien",
        },
        electionEventBreadcrumbSteps: {
            created: "Aangemaakt",
            keys: "Sleutels",
            publish: "Publiceren",
            started: "Gestart",
            ended: "Beëindigd",
            results: "Resultaten",
        },
        candidate: {
            moreInformationLink: "Meer informatie",
            writeInsPlaceholder: "Typ hier de naam van de kandidaat",
            blankVote: "Blanco stem",
            preferential: {
                position: "Positie",
                none: "Geen",
                ordinals: {
                    first: "e",
                    second: "e",
                    third: "e",
                    other: "e",
                },
            },
        },
        homeScreen: {
            title: "Sequent Stembiljetverificateur",
            description1:
                "De stembiljetverificateur wordt gebruikt wanneer de kiezer ervoor kiest het stembiljet in het stemhokje te controleren. De verificatie duurt 1 tot 2 minuten.",
            description2:
                "Met de stembiljetverificateur kan de kiezer controleren of het versleutelde stembiljet de in het stemhokje gemaakte keuzes correct weergeeft. Deze controle mogelijk maken heet cast-as-intended-verifieerbaarheid en voorkomt fouten en kwaadwillig handelen tijdens het versleutelen van het stembiljet.",
            descriptionMore: "Meer informatie",
            startButton: "Bestand kiezen",
            dragDropOption: "Of sleep het bestand hierheen",
            importErrorDescription:
                "Er is een probleem opgetreden bij het importeren van het controleerbare stembiljet. Heeft u het juiste bestand gekozen?",
            importErrorMoreInfo: "Meer informatie",
            importErrorTitle: "Fout",
            useSampleText: "Heeft u geen controleerbaar stembiljet?",
            useSampleLink: "Gebruik een voorbeeld van een controleerbaar stembiljet",
        },
        confirmationScreen: {
            title: "Sequent Stembiljetverificateur",
            topDescription1:
                "Op basis van de informatie in het geïmporteerde controleerbare stembiljet hebben wij berekend dat:",
            topDescription2: "Als dit de stembiljet-ID is die in het stemhokje wordt weergegeven:",
            bottomDescription1:
                "Uw stembiljet is correct versleuteld. U kunt dit venster nu sluiten en terugkeren naar het stemhokje.",
            bottomDescription2:
                "Als ze niet overeenkomen, klik hier voor meer informatie over de mogelijke oorzaken en wat u kunt doen.",
            ballotChoicesDescription: "En uw keuzes op het stembiljet zijn:",
            helpAndFaq: "Help en veelgestelde vragen",
            backButton: "Terug",
            markedInvalid: "Stembiljet expliciet als ongeldig gemarkeerd",
        },
        ballotSelectionsScreen: {
            statusModal: {
                title: "Status",
                content: "Het statuspaneel geeft u informatie over de uitgevoerde verificaties.",
                ok: "OK",
            },
        },
        footer: {
            poweredBy: "Aangedreven door <sequent />",
        },
        errors: {
            encoding: {
                notEnoughChoices: "Niet genoeg keuzes om te decoderen",
                writeInChoiceOutOfRange: "In te vullen keuze buiten bereik: {{index}}",
                writeInNotEndInZero: "In te vullen tekst eindigt niet op 0",
                writeInCharsExceeded_one: "Maak de ingevulde tekst {{count}} teken korter.",
                writeInCharsExceeded_other: "Maak de ingevulde tekst {{count}} tekens korter.",
                bytesToUtf8Conversion:
                    "Fout bij het converteren van in te vullen tekst van bytes naar UTF-8 string: {{errorMessage}}",
                ballotTooLarge: "Stembiljet groter dan verwacht",
            },
            implicit: {
                selectedMax_one: "Deselecteer {{count}} kandidaat.",
                selectedMax_other: "Deselecteer {{count}} kandidaten.",
                selectedMin_one: "Selecteer nog {{count}} kandidaat.",
                selectedMin_other: "Selecteer nog {{count}} kandidaten.",
                maxSelectionsPerType_one: "Deselecteer {{count}} kandidaat uit {{type}}.",
                maxSelectionsPerType_other: "Deselecteer {{count}} kandidaten uit {{type}}.",
                underVote_one: "Selecteer nog maximaal {{count}} kandidaat.",
                underVote_other: "Selecteer nog maximaal {{count}} kandidaten.",
                overVoteDisabled_one:
                    "U heeft het maximum van {{count}} kandidaat geselecteerd. Deselecteer deze om een andere te kiezen.",
                overVoteDisabled_other:
                    "U heeft het maximum van {{count}} kandidaten geselecteerd. Deselecteer er een om een andere te kiezen.",
                blankVote: "U heeft geen kandidaat geselecteerd.",
                preferenceOrderWithGaps:
                    "Ongeldige stem! De voorkeursvolgorde heeft een of meer hiaten.",
                duplicatedPosition:
                    "Ongeldige stem! Dezelfde positie is geselecteerd voor twee of meer kandidaten.",
            },
            explicit: {
                notAllowed:
                    "Stembiljet expliciet ongeldig gemarkeerd maar de stemming staat dit niet toe",
                alert: "Gemarkeerde selectie wordt als ongeldige stem beschouwd.",
            },
            configuration: {
                multipleExplicitInvalidCandidates:
                    "Ongeldige stemconfiguratie: de stemming definieert {{count}} expliciet ongeldige kandidaten, maar er is er maar één toegestaan.",
                multipleExplicitBlankCandidates:
                    "Ongeldige stemconfiguratie: de stemming definieert {{count}} expliciete blanco kandidaten, maar er is er maar één toegestaan.",
            },
        },
        ballotHash: "Uw stembiljet-ID: {{ballotId}}",
        version: {
            header: "Versie:",
        },
        hash: {
            header: "Hash:",
        },
        logout: {
            buttonText: "Afmelden",
            modal: {
                title: "Weet u zeker dat u zich wilt afmelden?",
                content:
                    "U staat op het punt deze applicatie te sluiten. Deze actie kan niet ongedaan worden gemaakt. ",
                ok: "OK",
                close: "Sluiten",
            },
        },
        stories: {
            openDialog: "Dialoogvenster openen",
        },
        dragNDrop: {
            firstLine: "Sleep bestanden hierheen of",
            browse: "Bestand kiezen",
            format: "Ondersteund formaat: txt",
        },
        selectElection: {
            electionWebsite: "Verkiezingswebsite",
            countdown:
                "De verkiezing begint over {{years}} jaar, {{months}} maanden, {{weeks}} weken, {{days}} dagen, {{hours}} uur, {{minutes}} minuten, {{seconds}} seconden",
            openElection: "Geopend",
            closedElection: "Gesloten",
            voted: "Gestemd",
            notVoted: "Niet gestemd",
            resultsButton: "Verkiezingsuitslag",
            voteButton: "Klik om te stemmen",
            openDate: "Geopend: ",
            closeDate: "Gesloten: ",
            ballotLocator: "Vind uw stembiljet",
        },
        header: {
            profile: "Profiel",
            welcome: "Welkom,<br><span>{{name}}</span>",
            session: {
                title: "Uw sessie verloopt binnenkort.",
                timeLeft: "U heeft nog {{time}} om uw stem uit te brengen.",
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minuten en {{time}} seconden",
                timeLeftSeconds: "{{timeLeft}} seconden",
            },
        },
    },
}

export default dutchTranslation
