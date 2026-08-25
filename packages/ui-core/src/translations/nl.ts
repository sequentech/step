// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const dutchTranslation: TranslationType = {
    translations: {
        language: "Nederlands",
        welcome: "Hallo <br/> <strong>Wereld</strong>",
        breadcrumbSteps: {
            select: "Selecteer een Verifieerder",
            import: "Gegevens Importeren",
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
            writeInsPlaceholder: "Typ hier de handmatig ingevoerde kandidaat",
            blankVote: "Blanco Stem",
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
            title: "Sequent Stembiljet Verifieerder",
            description1:
                "De stembiljetverifieerder wordt gebruikt wanneer de kiezer ervoor kiest het stembiljet te controleren in het stemhokje. De verificatie duurt 1-2 minuten.",
            description2:
                "De stembiljetverifieerder stelt de kiezer in staat om te controleren of het versleutelde stembiljet de in het stemhokje gemaakte selecties correct vastlegt. Deze controle wordt cast-as-intended-verifieerbaarheid genoemd en voorkomt fouten en kwaadwillige activiteiten tijdens het versleutelen van het stembiljet.",
            descriptionMore: "Meer weten",
            startButton: "Bestand kiezen",
            dragDropOption: "Of sleep het hierheen",
            importErrorDescription:
                "Er is een probleem opgetreden bij het importeren van het auditeerbare stembiljet. Heeft u het juiste bestand gekozen?",
            importErrorMoreInfo: "Meer info",
            importErrorTitle: "Fout",
            useSampleText: "Heeft u geen auditeerbaar stembiljet?",
            useSampleLink: "Gebruik een voorbeeld auditeerbaar stembiljet",
        },
        confirmationScreen: {
            title: "Sequent Stembiljet Verifieerder",
            topDescription1:
                "Op basis van de informatie in het geïmporteerde Auditeerbare Stembiljet hebben we berekend dat:",
            topDescription2: "Als dit het Stembiljet-ID is dat in het Stemhokje wordt getoond:",
            bottomDescription1:
                "Uw stembiljet is correct versleuteld. U kunt dit venster nu sluiten en terugkeren naar het Stemhokje.",
            bottomDescription2:
                "Als ze niet overeenkomen, klik hier om meer te weten te komen over de mogelijke redenen en welke acties u kunt ondernemen.",
            ballotChoicesDescription: "En uw stembiljetkeuzes zijn:",
            helpAndFaq: "Help & Veelgestelde vragen",
            backButton: "Terug",
            markedInvalid: "Stembiljet expliciet als ongeldig gemarkeerd",
        },
        ballotSelectionsScreen: {
            statusModal: {
                title: "Status",
                content:
                    "Het statuspaneel geeft u informatie over de uitgevoerde verificaties.",
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
                preferenceOrderWithGaps:
                    "Ongeldige stem! De voorkeursvolgorde heeft een of meer hiaten.",
                duplicatedPosition:
                    "Ongeldige stem! Dezelfde positie is geselecteerd voor twee of meer kandidaten.",
            },
            explicit: {
                notAllowed:
                    "Stembiljet expliciet ongeldig gemarkeerd maar vraag staat dit niet toe",
                alert: "Gemarkeerde selectie wordt als ongeldige stem beschouwd.",
            },
            configuration: {
                multipleExplicitInvalidCandidates:
                    "Ongeldige stemconfiguratie: de verkiezing definieert {{count}} expliciet ongeldige kandidaten, maar er is er maar één toegestaan.",
                multipleExplicitBlankCandidates:
                    "Ongeldige stemconfiguratie: de verkiezing definieert {{count}} expliciete blanco kandidaten, maar er is er maar één toegestaan.",
            },
        },
        ballotHash: "Uw Stembiljet-ID: {{ballotId}}",
        version: {
            header: "Versie:",
        },
        hash: {
            header: "Hash:",
        },
        logout: {
            buttonText: "Uitloggen",
            modal: {
                title: "Weet u zeker dat u wilt uitloggen?",
                content: "U staat op het punt deze applicatie te sluiten. Deze actie kan niet ongedaan worden gemaakt. ",
                ok: "OK",
                close: "Sluiten",
            },
        },
        stories: {
            openDialog: "Dialoogvenster openen",
        },
        dragNDrop: {
            firstLine: "Sleep bestanden hierheen of",
            browse: "Bladeren",
            format: "Ondersteund formaat: txt",
        },
        selectElection: {
            electionWebsite: "Stembiljet Website",
            countdown:
                "Verkiezing begint over {{years}} jaar, {{months}} maanden, {{weeks}} weken, {{days}} dagen, {{hours}} uur, {{minutes}} minuten, {{seconds}} seconden",
            openElection: "Open",
            closedElection: "Gesloten",
            voted: "Gestemd",
            notVoted: "Niet gestemd",
            resultsButton: "Stembiljet Resultaten",
            voteButton: "Klik om te Stemmen",
            openDate: "Open: ",
            closeDate: "Sluit: ",
            ballotLocator: "Zoek uw stembiljet",
        },
        header: {
            profile: "Profiel",
            welcome: "Welkom,<br><span>{{name}}</span>",
            session: {
                title: "Uw sessie staat op het punt te verlopen.",
                timeLeft: "U heeft nog {{time}} om uw stem uit te brengen.",
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minuten en {{time}} seconden",
                timeLeftSeconds: "{{timeLeft}} seconden",
            },
        },
    },
}

export default dutchTranslation
