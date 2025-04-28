// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {TranslationType} from "./en"

const galegoTranslation: TranslationType = {
    translations: {
        welcome: "Ola <br/> <strong>Mundo</strong>",
        404: {
            title: "Páxina non atopada",
            subtitle: "A páxina que estás buscando non existe",
        },
        homeScreen: {
            step1: "Paso 1: Importa a túa papeleta",
            description1:
                "Para continuar, importa os datos da papeleta cifrada proporcionados no Portal de Votación:",
            importBallotHelpDialog: {
                title: "Información: Importa a túa papeleta",
                ok: "Aceptar",
                content:
                    "Para continuar, importa os datos da papeleta cifrada proporcionados no Portal de Votación.",
            },
            step2: "Paso 2: Introduce o teu ID de papeleta",
            description2: "Escribe o ID da papeleta proporcionado no Portal de Votación:",
            ballotIdHelpDialog: {
                title: "Información: O teu ID de papeleta",
                ok: "Aceptar",
                content: "Escribe o ID da papeleta proporcionado no Portal de Votación.",
            },
            startButton: "Seleccionar arquivo",
            dragDropOption: "Ou arrástrao e déixao aquí",
            importErrorDescription:
                "Houbo un problema ao importar a papeleta auditábel. Escolléchelo arquivo correcto?",
            importErrorMoreInfo: "Máis información",
            importErrorTitle: "Erro",
            useSampleLink: "Usar unha papeleta de exemplo",
            nextButton: "Seguinte",
            ballotIdLabel: "ID de papeleta",
            ballotIdPlaceholder: "Escribe o teu ID de papeleta",
            fileUploaded: "Cargado",
        },
        confirmationScreen: {
            ballotIdTitle: "ID de papeleta",
            ballotIdDescription:
                "Abaixo o sistema mostra o ID da papeleta descifrada e o xerado polo verificador",
            ballotIdError: "Non coincide co ID da papeleta descifrada",
            decodedBallotId: "ID de papeleta descifrada",
            decodedBallotIdHelpDialog: {
                title: "Información: ID de papeleta descifrada",
                ok: "Aceptar",
                content:
                    "Este é o ID da papeleta lido a partir do ficheiro de papeleta auditábel que proporcionaches.",
            },
            yourBallotId: "O ID de papeleta que proporcionaches",
            userBallotIdHelpDialog: {
                title: "Información: O ID de papeleta que proporcionaches",
                ok: "Aceptar",
                content:
                    "Este é o ID de papeleta que introduciches no paso anterior e que recolliches no posto de votación.",
            },
            backButton: "Volver",
            printButton: "Imprimir",
            finishButton: "Verificado",
            verifySelectionsTitle: "Verifica as túas seleccións de papeleta",
            verifySelectionsDescription:
                "As seguintes seleccións de papeleta foron descifradas da papeleta que importaches. Revísaas e asegúrate de que coinciden coas que fixeches no Portal de Votación. Se as túas seleccións non coinciden, contacta coas autoridades electorais...",
            verifySelectionsHelpDialog: {
                title: "Información: Verifica as túas seleccións de papeleta",
                ok: "Aceptar",
                content:
                    "As seguintes seleccións de papeleta foron descifradas da papeleta que importaches. Revísaas e asegúrate de que coinciden coas que fixeches no Portal de Votación. Se as túas seleccións non coinciden, contacta coas autoridades electorais...",
            },
            markedInvalid: "Papeleta marcada explícitamente como inválida",
            points: "({{points}} Puntos)",
            contestNotFound: "Concurso non atopado: {{contestId}}",
        },
        poweredBy: "Desenvolvido por",
        errors: {
            encoding: {
                notEnoughChoices: "Non hai suficientes opcións para descodificar",
                writeInChoiceOutOfRange: "A opción escrita está fóra de rango: {{index}}",
                writeInNotEndInZero: "A opción escrita non remata en 0",
                bytesToUtf8Conversion:
                    "Erro ao converter a opción escrita de bytes a cadea UTF-8: {{errorMessage}}",
                ballotTooLarge: "A papeleta é máis grande do esperado",
            },
            implicit: {
                selectedMax:
                    "Número de opcións seleccionadas {{numSelected}} é máis do máximo permitido {{max}}",
                selectedMin:
                    "Número de opcións seleccionadas {{numSelected}} é menor do mínimo {{min}}",
            },
            explicit: {
                notAllowed: "A papeleta está marcada como inválida pero a pregunta non o permite",
            },
        },
    },
}

export default galegoTranslation
