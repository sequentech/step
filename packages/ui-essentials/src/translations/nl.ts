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
            import: "Importeer Gegevens",
            verify: "Verifieer",
            finish: "Voltooi",
        },
        electionEventBreadcrumbSteps: {
            created: "Aangemaakt",
            keys: "Sleutels",
            publish: "Publiceer",
            started: "Gestart",
            ended: "Beëindigd",
            results: "Resultaten",
        },
        candidate: {
            moreInformationLink: "Meer informatie",
            writeInsPlaceholder: "Typ hier de naam van de kandidaat",
            blankVote: "Blanco stem",
        },
        homeScreen: {
            title: "Sequent Stembiljet Verifieerder",
            description1:
                "De stembiljetverifieerder wordt gebruikt wanneer de kiezer ervoor kiest om het stembiljet in het stemhokje te controleren. De verificatie duurt ongeveer 1-2 minuten.",
            description2:
                "De stembiljetverifieerder stelt de kiezer in staat te verzekeren dat het versleutelde stembiljet de selecties gemaakt in het stemhokje correct vastlegt. Deze controle mogelijk maken heet 'cast-as-intended' verifieerbaarheid en voorkomt fouten en kwaadwillige activiteiten tijdens de versleuteling van het stembiljet.",
            descriptionMore: "Meer weten",
            startButton: "Blader door bestanden",
            dragDropOption: "Of sleep het bestand hierheen",
            importErrorDescription:
                "Er was een probleem bij het importeren van het controleerbare stembiljet. Heeft u het juiste bestand gekozen?",
            importErrorMoreInfo: "Meer info",
            importErrorTitle: "Fout",
            useSampleText: "Heeft u geen controleerbaar stembiljet?",
            useSampleLink: "Gebruik een voorbeeld van een controleerbaar stembiljet",
        },
        confirmationScreen: {
            title: "Sequent Stembiljet Verifieerder",
            topDescription1:
                "Op basis van de informatie in het geïmporteerde Controleerbare Stembiljet, hebben we berekend dat:",
            topDescription2: "Als dit de Stembiljet-ID is die in het stemhokje wordt getoond:",
            bottomDescription1:
                "Uw stembiljet is correct versleuteld. U kunt dit venster nu sluiten en terugkeren naar het stemhokje.",
            bottomDescription2:
                "Als ze niet overeenkomen, klik hier voor meer informatie over de mogelijke redenen en welke acties u kunt ondernemen.",
            ballotChoicesDescription: "En uw stemkeuzes zijn:",
            helpAndFaq: "Veelgestelde Vragen", // FAQ is commonly understood, alternatively "Veelgestelde Vragen"
            backButton: "Terug",
            markedInvalid: "Stembiljet expliciet ongeldig gemarkeerd",
        },
        ballotSelectionsScreen: {
            statusModal: {
                title: "Status",
                content: "Het statuspaneel geeft u informatie over de uitgevoerde verificaties.",
                ok: "OK",
            },
        },
        footer: {
            poweredBy: "Aangedreven door <1></1>",
        },
        errors: {
            encoding: {
                notEnoughChoices: "Niet genoeg keuzes om te decoderen",
                writeInChoiceOutOfRange: "Ingevoerde keuze buiten bereik: {{index}}",
                writeInNotEndInZero: "Ingevoerde waarde eindigt niet op 0",
                bytesToUtf8Conversion:
                    "Fout bij het converteren van ingevoerde waarde van bytes naar UTF-8 string: {{errorMessage}}",
                ballotTooLarge: "Stembiljet groter dan verwacht",
            },
            implicit: {
                selectedMax:
                    "Aantal geselecteerde keuzes {{numSelected}} is meer dan het maximum {{max}}",
                selectedMin:
                    "Aantal geselecteerde keuzes {{numSelected}} is minder dan het minimum {{min}}",
            },
            explicit: {
                notAllowed:
                    "Stembiljet expliciet ongeldig gemarkeerd, maar vraag staat dit niet toe",
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
                title: "Bent u zeker dat u wilt uitloggen?",
                content:
                    "U staat op het punt deze applicatie te sluiten. Deze actie kan niet ongedaan worden gemaakt.",
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
            electionWebsite: "Verkiezingswebsite",
            countdown:
                "Verkiezing Begint over {{years}} jaar, {{months}} maanden, {{weeks}} weken, {{days}} dagen, {{hours}} uur, {{minutes}} minuten, {{seconds}} seconden", // Check singular/plural needs in your implementation (jaar/jaren, maand/maanden, week/weken, dag/dagen, uur/uren, minuut/minuten, seconde/seconden)
            openElection: "Open",
            closedElection: "Gesloten",
            voted: "Gestemd",
            notVoted: "Niet gestemd",
            resultsButton: "Verkiezingsresultaten",
            voteButton: "Klik om te stemmen",
            openDate: "Open: ",
            closeDate: "Sluit: ",
            ballotLocator: "Lokaliseer uw stembiljet",
        },
        header: {
            profile: "Profiel",
            welcome: "Welkom,<br><span>{{name}}</span>",
            session: {
                title: "Uw sessie gaat bijna verlopen.",
                timeLeft: "U heeft nog {{time}} om uw stem uit te brengen.", // {{time}} should contain units like "5 minuten"
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minuten en {{time}} seconden",
                timeLeftSeconds: "{{timeLeft}} seconden",
            },
        },
    },
}

// You can then export it if needed in your environment
export default dutchTranslation
