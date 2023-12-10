// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const spanishTranslation: TranslationType = {
    translations: {
        welcome: "Let's start: Import auditable ballot..",
        homeScreen: {
            step1: "Paso 1: Importa tu papeleta electoral.",
            description1:
                "Para continuar, por favor importe los datos de las boletas encriptadas proporcionados en el Portal de Votación:",
            importBallotHelpDialog: {
                title: "Información: Importa tu papeleta electoral",
                ok: "OK",
                content:
                    "Para continuar, por favor importe los datos de las boletas encriptadas proporcionados en el Portal de Votación.",
            },
            step2: "Paso 2: Inserta tu ID de Boleta.",
            description2:
                "Por favor ingrese el ID de la Boleta proporcionado en el Portal de Votación:",
            ballotIdHelpDialog: {
                title: "Information: Tu ID de Boleta",
                ok: "OK",
                content:
                    "Por favor ingrese el ID de la Boleta proporcionado en el Portal de Votación.",
            },
            startButton: "Selecciona fichero",
            dragDropOption: "O arrastre el fichero aquí",
            importErrorDescription:
                "Hubo un problema al importar el voto auditable. ¿Elegiste el archivo correcto?",
            importErrorMoreInfo: "Más información",
            importErrorTitle: "Error",
            useSampleLink: "Use voto de ejemplo",
            nextButton: "Continuar",
            ballotIdLabel: "ID de Boleta",
            ballotIdPlaceholder: "Escribe aquí tu ID de Boleta",
            fileUploaded: "Cargado",
        },
        confirmationScreen: {
            ballotIdTitle: "ID de Boleta",
            ballotIdDescription:
                "A continuación, el sistema muestra el ID de la boleta descodificada y el generado por el verificador.",
            ballotIdError: "No coincide con el ID de boleta decodificado.",
            decodedBallotId: "Id de Boleta decodificado",
            decodedBallotIdHelpDialog: {
                title: "Información: Id de Boleta decodificado",
                ok: "OK",
                content:
                    "Este es el ID de Boleta extraído del fichero de la Papeleta Auditable descodificada que proporcionaste.",
            },
            yourBallotId: "La Id de Boleta que proporcionaste",
            userBallotIdHelpDialog: {
                title: "Información: La Id de Boleta que proporcionaste",
                ok: "OK",
                content:
                    "Esta es la Id de la Boleta que escribiste en el anterior paso y que recogiste de la Cabina de Votación.",
            },
            backButton: "Atrás",
            printButton: "Imprimir",
            finishButton: "Verificado",
            verifySelectionsTitle: "Verifica tus selecciones en la boleta",
            verifySelectionsDescription:
                "Las siguientes selecciones de la boleta han sido descodificadas de la boleta que importaste. Por favor, revísalas y asegúrate de que coincidan con las selecciones que hiciste en el Portal de Votación. Si tus selecciones no coinciden, por favor, contacta a las autoridades electorales...",
            verifySelectionsHelpDialog: {
                title: "Información: Verifica tus selecciones en la boleta",
                ok: "OK",
                content:
                    "Las siguientes selecciones de la boleta han sido descodificadas de la boleta que importaste. Por favor, revísalas y asegúrate de que coincidan con las selecciones que hiciste en el Portal de Votación. Si tus selecciones no coinciden, por favor, contacta a las autoridades electorales...",
            },
            markedInvalid: "Voto explícitamente marcado inválido",
            points: "({{points}} Puntos)",
        },
        poweredBy: "Funciona con",
        errors: {
            encoding: {
                notEnoughChoices: "No hay suficientes opciones para decodificar",
                writeInChoiceOutOfRange: "Opción de voto escrita fuera de rango: {{index}}",
                writeInNotEndInZero: "Opción de voto escrita no finaliza en 0",
                bytesToUtf8Conversion:
                    "Error convirtiendo bytes de opción de voto escrita a cadena UTF-8: {{errorMessage}}",
                ballotTooLarge: "Voto más grande de lo esperado",
            },
            implicit: {
                selectedMax:
                    "El número de opciones seleccionadas {{numSelected}} es mayor que el máximo {{max}}",
                selectedMin:
                    "El número de opciones seleccionadas {{numSelected}} es menor que el máximo {{min}}",
            },
            explicit: {
                notAllowed:
                    "Voto marcado explícitamente como inválido pero la pregunta no lo permite",
            },
        },
    },
}

export default spanishTranslation
