// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const catalanTranslation: TranslationType = {
    translations: {
        "welcome": "Comencem: Importa la papereta auditable...",
        "404": {
            title: "Pàgina no trobada",
            subtitle: "La pàgina que busques no existeix",
        },
        "homeScreen": {
            step1: "Pas 1: Importa la teva papereta electoral.",
            description1:
                "Per continuar, si us plau importa les dades de les paperetes encriptades proporcionades al Portal de Votació:",
            importBallotHelpDialog: {
                title: "Informació: Importa la teva papereta electoral",
                ok: "D'acord",
                content:
                    "Per continuar, si us plau importa les dades de les paperetes encriptades proporcionades al Portal de Votació.",
            },
            step2: "Pas 2: Insereix el teu ID de papereta.",
            description2:
                "Si us plau introdueix l'ID de la papereta proporcionat al Portal de Votació:",
            ballotIdHelpDialog: {
                title: "Informació: El teu ID de papereta",
                ok: "D'acord",
                content:
                    "Si us plau introdueix l'ID de la papereta proporcionat al Portal de Votació.",
            },
            startButton: "Selecciona fitxer",
            dragDropOption: "O arrossega el fitxer aquí",
            importErrorDescription:
                "Hi ha hagut un problema en importar el vot auditable. Has triat el fitxer correcte?",
            importErrorMoreInfo: "Més informació",
            importErrorTitle: "Error",
            useSampleLink: "Utilitza vot d'exemple",
            nextButton: "Continuar",
            ballotIdLabel: "ID de papereta",
            ballotIdPlaceholder: "Escriu aquí el teu ID de papereta",
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
            verifySelectionsTitle: "Verifica les teves seleccions a la papereta",
            verifySelectionsDescription:
                "Les següents seleccions de la papereta han estat descodificades de la papereta que vas importar. Si us plau, revisa-les i assegura't que coincideixin amb les seleccions que vas fer al Portal de Votació. Si les teves seleccions no coincideixen, si us plau, contacta amb les autoritats electorals...",
            verifySelectionsHelpDialog: {
                title: "Informació: Verifica les teves seleccions a la papereta",
                ok: "D'acord",
                content:
                    "Les següents seleccions de la papereta han estat descodificades de la papereta que vas importar. Si us plau, revisa-les i assegura't que coincideixin amb les seleccions que vas fer al Portal de Votació. Si les teves seleccions no coincideixen, si us plau, contacta amb les autoritats electorals...",
            },
            markedInvalid: "Vot explícitament marcat invàlid",
            points: "({{points}} Punts)",
            contestNotFound: "Pregunta no trobada: {{contestId}}",
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
            implicit: {
                selectedMax:
                    "El nombre d'opcions seleccionades {{numSelected}} és major que el màxim {{max}}",
                selectedMin:
                    "El nombre d'opcions seleccionades {{numSelected}} és menor que el mínim {{min}}",
            },
            explicit: {
                notAllowed: "Vot marcat explícitament com a invàlid però la pregunta no ho permet",
            },
        },
        "footer": {
            poweredBy: "Funciona amb <sequent />",
        },
    },
}

export default catalanTranslation
