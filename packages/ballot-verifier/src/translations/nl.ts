// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {TranslationType} from "./en"

const dutchTranslation: TranslationType = {
    translations: {
        welcome: "Hallo <br/> <strong>Wereld</strong>",
        404: {
            title: "Pagina niet gevonden",
            subtitle: "De pagina die u zoekt bestaat niet",
        },
        homeScreen: {
            step1: "Stap 1: Importeer uw stembiljet",
            description1:
                "Om door te gaan, importeer de versleutelde stembiljetgegevens die zijn verstrekt in het Stemportaal:",
            importBallotHelpDialog: {
                title: "Informatie: Importeer uw stembiljet",
                ok: "OK",
                content:
                    "Om door te gaan, importeer de versleutelde stembiljetgegevens die zijn verstrekt in het Stemportaal.",
            },
            step2: "Stap 2: Voer uw stembiljet-ID in",
            description2: "Voer het stembiljet-ID in dat is verstrekt in het Stemportaal:",
            ballotIdHelpDialog: {
                title: "Informatie: Uw stembiljet-ID",
                ok: "OK",
                content: "Voer het stembiljet-ID in dat is verstrekt in het Stemportaal.",
            },
            startButton: "Bestand kiezen",
            dragDropOption: "Of sleep het hierheen",
            importErrorDescription:
                "Er is een probleem opgetreden bij het importeren van het auditeerbare stembiljet. Heeft u het juiste bestand gekozen?",
            importErrorMoreInfo: "Meer informatie",
            importErrorTitle: "Fout",
            useSampleLink: "Gebruik een voorbeeldstembiljet",
            nextButton: "Volgende",
            ballotIdLabel: "Stembiljet-ID",
            ballotIdPlaceholder: "Voer uw stembiljet-ID in",
            fileUploaded: "Geüpload",
        },
        confirmationScreen: {
            ballotIdTitle: "Stembiljet-ID",
            ballotIdDescription:
                "Hieronder toont het systeem het stembiljet-ID van het gedecodeerde stembiljet en het door de verificator gegenereerde ID",
            ballotIdError: "Komt niet overeen met het gedecodeerde stembiljet-ID",
            decodedBallotId: "Gedecodeerd Stembiljet-ID",
            decodedBallotIdHelpDialog: {
                title: "Informatie: Gedecodeerd Stembiljet-ID",
                ok: "OK",
                content:
                    "Dit is het stembiljet-ID dat is afgeleid bij het decoderen van het auditeerbare stembiljetbestand dat u heeft verstrekt.",
            },
            yourBallotId: "Het stembiljet-ID dat u heeft opgegeven",
            userBallotIdHelpDialog: {
                title: "Informatie: Het stembiljet-ID dat u heeft opgegeven",
                ok: "OK",
                content:
                    "Dit is het stembiljet-ID dat u in de vorige stap heeft ingevoerd en dat u heeft verzameld in het stemhokje.",
            },
            backButton: "Terug",
            printButton: "Afdrukken",
            finishButton: "Geverifieerd",
            verifySelectionsTitle: "Verifieer uw stembiljetselecties",
            verifySelectionsDescription:
                "De volgende stembiljetselecties zijn gedecodeerd uit het stembiljet dat u heeft geïmporteerd. Controleer ze en zorg ervoor dat ze overeenkomen met de selecties die u in het Stemportaal heeft gemaakt. Als uw selecties niet overeenkomen, neem dan contact op met de verkiezingsautoriteiten...",
            verifySelectionsHelpDialog: {
                title: "Informatie: Verifieer uw stembiljetselecties",
                ok: "OK",
                content:
                    "De volgende stembiljetselecties zijn gedecodeerd uit het stembiljet dat u heeft geïmporteerd. Controleer ze en zorg ervoor dat ze overeenkomen met de selecties die u in het Stemportaal heeft gemaakt. Als uw selecties niet overeenkomen, neem dan contact op met de verkiezingsautoriteiten...",
            },
            markedInvalid: "Stembiljet expliciet als ongeldig gemarkeerd",
            points: "({{points}} Punten)",
            contestNotFound: "Verkiezing niet gevonden: {{contestId}}",
            declineToVote: "Afgezien van stemmen",
            blankBallot: "Blanco stembiljet",
        },
        footer: {
            poweredBy: "Mogelijk gemaakt door <1></1>",
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
            implicit: {
                selectedMax:
                    "Aantal geselecteerde keuzes {{numSelected}} is meer dan het maximum {{max}}",
                selectedMin:
                    "Aantal geselecteerde keuzes {{numSelected}} is minder dan het minimum {{min}}",
            },
            explicit: {
                notAllowed:
                    "Stembiljet expliciet als ongeldig gemarkeerd, maar de vraag staat dit niet toe",
            },
        },
    },
}

export default dutchTranslation
