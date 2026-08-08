// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const catalanTranslation: TranslationType = {
    translations: {
        "404": {
            title: "Pàgina no trobada",
            subtitle: "La pàgina que busqueu no existeix",
        },
        "homeScreen": {
            step1: "Pas 1: Importeu la vostra papereta electoral.",
            description1:
                "Per continuar, si us plau importeu les dades de les paperetes encriptades proporcionades al Portal de Votació:",
            importBallotHelpDialog: {
                title: "Informació: Importeu la vostra papereta electoral",
                ok: "D'acord",
                content:
                    "Per continuar, si us plau importeu les dades de les paperetes encriptades proporcionades al Portal de Votació.",
            },
            step2: "Pas 2: Introduïu el vostre ID de papereta.",
            description2:
                "Si us plau, introduïu l'ID de la papereta proporcionat al Portal de Votació:",
            ballotIdHelpDialog: {
                title: "Informació: El vostre ID de papereta",
                ok: "D'acord",
                content:
                    "Si us plau, introduïu l'ID de la papereta proporcionat al Portal de Votació.",
            },
            startButton: "Seleccioneu fitxer",
            dragDropOption: "O arrossegueu el fitxer aquí",
            importErrorDescription:
                "Hi ha hagut un problema en importar el vot auditable. Heu triat el fitxer correcte?",
            importErrorMoreInfo: "Més informació",
            importErrorTitle: "Error",
            useSampleLink: "Utilitzeu vot d'exemple",
            nextButton: "Continuar",
            ballotIdLabel: "ID de papereta",
            ballotIdPlaceholder: "Escriviu aquí el vostre ID de papereta",
            fileUploaded: "Carregat",
        },
        "confirmationScreen": {
            ballotIdTitle: "ID de papereta",
            ballotIdDescription:
                "A continuació, el sistema mostra l'ID de la papereta descodificada i el generat pel verificador.",
            ballotIdError: "No coincideix amb l'ID de papereta descodificat.",
            decodedBallotId: "ID de papereta descodificat",
            decodedBallotIdHelpDialog: {
                title: "Informació: ID de papereta descodificat",
                ok: "D'acord",
                content:
                    "Aquest és l'ID de la papereta extret del fitxer de la Papereta Auditable descodificada que vas proporcionar.",
            },
            yourBallotId: "L'ID de papereta que vas proporcionar",
            userBallotIdHelpDialog: {
                title: "Informació: L'ID de papereta que vas proporcionar",
                ok: "D'acord",
                content:
                    "Aquesta és l'ID de papereta que vas escriure en l'anterior pas i que vas recollir de la Cabina de Votació.",
            },
            backButton: "Enrere",
            printButton: "Imprimir",
            finishButton: "Verificat",
            verifySelectionsTitle: "Verifiqueu les vostres seleccions a la papereta",
            verifySelectionsDescription:
                "Les següents seleccions de la papereta han estat descodificades de la papereta que vau importar. Si us plau, reviseu-les i assegureu-vos que coincideixin amb les seleccions que vau fer al Portal de Votació. Si les vostres seleccions no coincideixen, si us plau, contacteu amb les autoritats electorals...",
            verifySelectionsHelpDialog: {
                title: "Informació: Verifiqueu les vostres seleccions a la papereta",
                ok: "D'acord",
                content:
                    "Les següents seleccions de la papereta han estat descodificades de la papereta que vau importar. Si us plau, reviseu-les i assegureu-vos que coincideixin amb les seleccions que vau fer al Portal de Votació. Si les vostres seleccions no coincideixen, si us plau, contacteu amb les autoritats electorals...",
            },
            markedInvalid: "Vot explícitament marcat invàlid",
            points_one: "({{count}} Punt)",
            points_many: "({{count}} Punts)",
            points_other: "({{count}} Punts)",
            contestNotFound: "Pregunta no trobada: {{contestId}}",
            declineToVote: "Vot no emès",
        },
        "errors": {
            encoding: {
                notEnoughChoices: "No hi ha prou opcions per a descodificar",
                writeInChoiceOutOfRange: "Opció de vot escrita fora de rang: {{index}}",
                writeInNotEndInZero: "Opció de vot escrita no finalitza en 0",
                bytesToUtf8Conversion:
                    "Error convertint bytes de l'opció de vot escrita a cadena UTF-8: {{errorMessage}}",
                ballotTooLarge: "Vot més gran de l'esperat",
            },
            explicit: {
                notAllowed: "Vot marcat explícitament com a invàlid però la pregunta no ho permet",
            },
        },
        "footer": {
            poweredBy: "Funciona amb <1></1>",
        },
    },
}

export default catalanTranslation
