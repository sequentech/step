// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {TranslationType} from "./en"

const basqueTranslation: TranslationType = {
    translations: {
        welcome: "Kaixo <br/> <strong>Mundua</strong>",
        404: {
            title: "Orria ez da aurkitu",
            subtitle: "Bilatzen ari zaren orria ez da existitzen",
        },
        homeScreen: {
            step1: "1. urratsa: Inportatu zure boto-txartela",
            description1:
                "Jarraitzeko, inportatu Bozketa Atarian emandako boto-txartel enkriptatuaren datuak:",
            importBallotHelpDialog: {
                title: "Informazioa: Inportatu zure boto-txartela",
                ok: "Ados",
                content:
                    "Jarraitzeko, inportatu Bozketa Atarian emandako boto-txartel enkriptatuaren datuak.",
            },
            step2: "2. urratsa: Idatzi zure boto-txartelaren IDa",
            description2: "Idatzi Bozketa Atarian emandako boto-txartelaren IDa:",
            ballotIdHelpDialog: {
                title: "Informazioa: Zure boto-txartelaren IDa",
                ok: "Ados",
                content: "Idatzi Bozketa Atarian emandako boto-txartelaren IDa.",
            },
            startButton: "Bilatu fitxategia",
            dragDropOption: "Edo arrastatu eta askatu hemen",
            importErrorDescription:
                "Arazo bat egon da boto-txartel auditagarria inportatzean. Fitxategi zuzena aukeratu duzu?",
            importErrorMoreInfo: "Informazio gehiago",
            importErrorTitle: "Errorea",
            useSampleLink: "Erabili adibidezko boto-txartel bat",
            nextButton: "Hurrengoa",
            ballotIdLabel: "Boto-txartelaren IDa",
            ballotIdPlaceholder: "Idatzi zure boto-txartelaren IDa",
            fileUploaded: "Igota",
        },
        confirmationScreen: {
            ballotIdTitle: "Boto-txartelaren IDa",
            ballotIdDescription:
                "Behean sistemak deskodetutako boto-txartelaren IDa eta egiaztatzaileak sortutakoa erakusten ditu",
            ballotIdError: "Ez dator bat deskodetutako boto-txartelaren IDarekin",
            decodedBallotId: "Deskodetutako Boto-txartelaren IDa",
            decodedBallotIdHelpDialog: {
                title: "Informazioa: Deskodetutako Boto-txartelaren IDa",
                ok: "Ados",
                content:
                    "Hau da eman duzun Boto-txartel Auditagarriaren fitxategia deskodetzetik lortutako Boto-txartelaren IDa.",
            },
            yourBallotId: "Eman duzun Boto-txartelaren IDa",
            userBallotIdHelpDialog: {
                title: "Informazioa: Eman duzun Boto-txartelaren IDa",
                ok: "Ados",
                content:
                    "Hau da aurreko urratsean idatzi zenuen eta Bozketa Kabinatik jaso zenuen Boto-txartelaren IDa.",
            },
            backButton: "Atzera",
            printButton: "Inprimatu",
            finishButton: "Egiaztatuta",
            verifySelectionsTitle: "Egiaztatu zure boto-txartelaren hautaketak",
            verifySelectionsDescription:
                "Hurrengo boto-txartelaren hautaketak inportatu duzun boto-txartelatik deskodetu dira. Mesedez, berrikusi eta ziurtatu Bozketa Atarian egin zenituen hautaketekin bat datozela. Zure hautaketak bat ez badatoz, jarri harremanetan hauteskunde agintariekin...",
            verifySelectionsHelpDialog: {
                title: "Informazioa: Egiaztatu zure boto-txartelaren hautaketak",
                ok: "Ados",
                content:
                    "Hurrengo boto-txartelaren hautaketak inportatu duzun boto-txartelatik deskodetu dira. Mesedez, berrikusi eta ziurtatu Bozketa Atarian egin zenituen hautaketekin bat datozela. Zure hautaketak bat ez badatoz, jarri harremanetan hauteskunde agintariekin...",
            },
            markedInvalid: "Boto-txartela berariaz baliogabetzat markatu da",
            points: "({{points}} Puntu)",
            contestNotFound: "Hautagaitza ez da aurkitu: {{contestId}}",
            declineToVote: "Bozkatzeari uko egin dio",
            blankBallot: "Boto-txartel zuria",
        },
        footer: {
            poweredBy: "Honek bultzatuta: <1></1>",
        },
        errors: {
            encoding: {
                notEnoughChoices: "Ez dago nahikoa aukera deskodetzeko",
                writeInChoiceOutOfRange: "Eskuz idatzitako aukera barrutitik kanpo dago: {{index}}",
                writeInNotEndInZero: "Eskuz idatzitakoa ez da 0-rekin amaitzen",
                bytesToUtf8Conversion:
                    "Errorea eskuz idatzitakoa byte-etatik UTF-8 katera bihurtzean: {{errorMessage}}",
                ballotTooLarge: "Boto-txartela espero baino handiagoa da",
            },
            implicit: {
                selectedMax:
                    "Hautatutako aukera kopurua {{numSelected}} gehienezkoa {{max}} baino handiagoa da",
                selectedMin:
                    "Hautatutako aukera kopurua {{numSelected}} gutxienekoa {{min}} baino txikiagoa da",
            },
            explicit: {
                notAllowed:
                    "Boto-txartela berariaz baliogabetzat markatu da, baina galderak ez du hori onartzen",
            },
        },
    },
}

export default basqueTranslation
