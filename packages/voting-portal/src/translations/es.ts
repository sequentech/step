// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const spanishTranslation: TranslationType = {
    translations: {
        common: {
            goBack: "Regresar",
        },
        breadcrumbSteps: {
            electionList: "Lista de Votaciones",
            ballot: "Papeleta",
            review: "Revisión",
            confirmation: "Confirmación",
            audit: "Auditar",
        },
        votingScreen: {
            backButton: "Atrás",
            reviewButton: "Siguiente",
            clearButton: "Limpiar selección",
            ballotHelpDialog: {
                title: "Información: Pantalla de votación",
                content:
                    "Esta pantalla muestra la votación en la que usted es elegible para votar. Puede seleccionar su sección activando la casilla de la derecha Candidato/Respuesta. Para restablecer sus selecciones, haga clic en el botón “<b>Borrar selección</b>”, para pasar al siguiente paso, haga clic en el botón “<b>Siguiente</b>”.",
                ok: "OK",
            },
            nonVotedDialog: {
                title: "Voto inválido o en blanco",
                content:
                    "Algunas de sus respuestas harán que la papeleta en una o más preguntas sea inválida o en blanco.",
                ok: "Volver y revisar",
                continue: "Continuar",
                cancel: "Cancelar",
            },
        },
        startScreen: {
            startButton: "Empezar a votar",
            instructionsTitle: "Instrucciones",
            instructionsDescription: "Por favor, siga estos pasos para emitir su voto:",
            step1Title: "1. Seleccione su opción de voto",
            step1Description:
                "Seleccione sus candidatos preferidos y responda las preguntas de la elección una por una a medida que aparezcan. Puede editar su papeleta hasta que esté listo para continuar.",
            step2Title: "2. Revise su papeleta",
            step2Description:
                "Una vez que esté satisfecho con sus selecciones, encriptaremos su papeleta y le mostraremos una revisión final de sus elecciones. También recibirá un ID de seguimiento único para su papeleta.",
            step3Title: "3. Envíe su voto",
            step3Description:
                "Envía tu papeleta: Finalmente, puedes enviar tu papeleta para que se registre correctamente. Alternativamente, puedes optar por auditar y confirmar que tu papeleta fue capturada y cifrada correctamente.",
        },
        reviewScreen: {
            title: "Revisa tu voto",
            description:
                "Para realizar cambios en sus selecciones, haga clic en el botón “<b>Editar selección</b>”, para confirmar sus selecciones, haga clic en el botón “<b>Enviar tu voto</b>” debajo, y para auditar su papeleta haga clic en el botón “<b>Auditar papeleta</b>” debajo. Tenga en cuenta que una vez que envíe su papeleta, habrá votado y no se le emitirá otra papeleta para esta elección.",
            descriptionNoAudit:
                "Para realizar cambios en sus selecciones, haga clic en el botón “<b>Editar selección</b>”, para confirmar sus selecciones, haga clic en el botón “<b>Enviar tu voto</b>” debajo. Tenga en cuenta que una vez que envíe su papeleta, habrá votado y no se le emitirá otra papeleta para esta elección.",
            backButton: "Editar tu voto",
            castBallotButton: "Enviar tu voto",
            auditButton: "Auditar papeleta",
            reviewScreenHelpDialog: {
                title: "Información: Pantalla de revisión",
                content:
                    "Esta pantalla le permite revisar sus selecciones antes de emitir su voto.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Voto no emitido",
                content:
                    "<p>Está a punto de copiar el Localizador del Voto, pero <b>su voto aún no se ha emitido</b>. Si intenta buscar el Localizador del Voto, no lo encontrará.</p><p>La razón por la que mostramos el Localizador del Voto en este momento es para que pueda auditar la corrección del voto cifrado antes de emitirlo. Si esa es la razón por la que desea copiar el Localizador del Voto, proceda a copiarlo y luego audite su voto.</p>",
                ok: "Acepto que mi voto NO ha sido emitido",
                cancel: "Cancelq4",
            },
            auditBallotHelpDialog: {
                title: "¿Realmente quieres Auditar tu papeleta?",
                content:
                    "<p>La auditoría de la papeleta lo invalidará y tendrás que iniciar el proceso de votación de nuevo si deseas emitir tu voto. El proceso de auditoría de la papeleta permite verificar que está codificada correctamente. Hacer este proceso requiere que unos conocimientos técnicos importantes, por lo que no se recomienda si no sabes lo que estás haciendo.</p><p><b>Si lo que desea es emitir su voto, en <u>Cancelar</u> para volver a la pantalla de revisión de votación.</b></p>",
                ok: "Si, quiero INVALIDAR mi papeleta para AUDITARLA",
                cancel: "Cancelar",
            },
        },
        confirmationScreen: {
            title: "Su voto ha sido emitido",
            description:
                "El código de confirmación que aparece a continuación verifica que <b>su voto se ha emitido correctamente</b>. Puede utilizar este código para verificar que su voto ha sido contabilizado.",
            ballotId: "Localizador del Voto",
            printButton: "Imprimir",
            finishButton: "Finalizar",
            verifyCastTitle: "Compruebe que su voto ha sido emitido",
            verifyCastDescription:
                "Puede comprobar en todo momento que su papeleta se ha emitido correctamente utilizando el siguiente código QR:",
            confirmationHelpDialog: {
                title: "Información: Pantalla de confirmación",
                content:
                    "Esta pantalla muestra que su voto se ha emitido correctamente. La información proporcionada en esta página le permite verificar que la papeleta ha sido almacenada en la urna , este proceso puede ser ejecutado en cualquier momento durante el periodo de votación y después de que la elección haya sido cerrada.",
                ok: "OK",
            },
            demoPrintDialog: {
                title: "Impresión de la papeleta de votación",
                content: "La impresión está desactivada en modo de demostración",
                ok: "Aceptar",
            },
            ballotIdHelpDialog: {
                title: "Información: Localizador del Voto",
                content:
                    "El Localizador del Voto de papeleta es un código que le permite encontrar su papeleta en la urna, este Localizador es único y no contiene información sobre sus selecciones.",
                ok: "OK",
            },
            ballotIdDemoHelpDialog: {
                title: "Información: Identificación de la papeleta",
                content:
                    "<p>La identificación de la papeleta es un código que te permite encontrar tu papeleta en la urna. Este identificador es único y no contiene información sobre tus selecciones.</p><p><b>Aviso:</b> Esta cabina de votación es solo para fines de demostración. Tu voto NO ha sido emitido.</p>",
                ok: "Aceptar",
            },
            errorDialogPrintVoteReceipt: {
                title: "Error",
                content: "Ha ocurrido un error, por favor intenta de nuevo",
                ok: "Aceptar",
            },
            demoQRText: "El rastreador de boletas está deshabilitado en modo de demostración",
        },
        auditScreen: {
            printButton: "Imprimir",
            restartButton: "Iniciar votación",
            title: "Audite su Papeleta",
            description: "Para verificar su papeleta deberá seguir los siguientes pasos:",
            step1Title: "1. Descargue o copie la siguiente información",
            step1Description:
                "Tu <b>Localizador del Voto</b> que aparece en la parte superior de la pantalla y tu papeleta encriptada a continuación:",
            step1HelpDialog: {
                title: "Copiar el Voto Cifrado",
                content:
                    "Puede descargar o copiar su Voto Cifrado para auditarlo y verificar que el contenido encriptado contiene sus selecciones.",
                ok: "OK",
            },
            downloadButton: "Descargar",
            step2Title: "2. Verifica tu papeleta",
            step2Description:
                "<a class=\"link\" href='{{linkToBallotVerifier}}' target='_blank'>Accede al verificador del voto</a>, que se abrirá una nueva pestaña en tu navegador.",
            step2HelpDialog: {
                title: "Tutorial sobre la Auditoría del Voto",
                content:
                    "Para auditar su voto deberá seguir los pasos indicados en el tutorial, que incluyen la descarga de una aplicación de escritorio utilizada para verificar el voto cifrado independientemente del sitio web.",
                ok: "OK",
            },
            bottomWarning:
                "Por motivos de seguridad, cuando audite su papeleta, deberá invalidarla. Para continuar con el proceso de votación, haga clic en ‘<b>Iniciar votación</b>’.",
        },
        electionSelectionScreen: {
            title: "Lista de Votaciones",
            description: "Seleccione la votación que desea votar",
            chooserHelpDialog: {
                title: "Información: Lista de Votaciones",
                content:
                    "Bienvenido a la cabina de votación, esta pantalla muestra la lista de elecciones en las que puede emitir su voto. Las elecciones que aparecen en esta lista pueden estar abiertas a votación, programadas o cerradas. Sólo podrá acceder a la votación si el periodo de votación está abierto.",
                ok: "OK",
            },
            noResults: "No hay elecciones por ahora.",
            demoDialog: {
                title: "Cabina de votación de demostración",
                content:
                    "Está entrando en una cabina de votación de demostración. <strong>Su voto NO será registrado.</strong> Esta cabina de votación es solo para fines de demostración.",
                ok: "Acepto que mi voto NO será registrado",
            },
            noVotingAreaError:
                "Área de votación no asignada al votante. Por favor, contacte con su administrador para obtener asistencia.",
        },
        errors: {
            encoding: {
                notEnoughChoices: "No hay suficientes opciones para decodificar",
                writeInChoiceOutOfRange: "Opción de voto escrita fuera de rango: {{index}}",
                writeInNotEndInZero: "Opción de voto escrita no finaliza en 0",
                writeInCharsExceeded:
                    "Opción de voto escrita excede el número de caracters por {{numCharsExceeded}} caracteres. Requiere arreglo.",
                bytesToUtf8Conversion:
                    "Error convirtiendo bytes de opción de voto escrita a cadena UTF-8: {{errorMessage}}",
                ballotTooLarge: "Voto más grande de lo esperado",
            },
            implicit: {
                selectedMax:
                    "Sobrevoto: El número de opciones seleccionadas {{numSelected}} es mayor que el máximo {{max}}",
                selectedMin:
                    "El número de opciones seleccionadas {{numSelected}} es menor que el máximo {{min}}",
                maxSelectionsPerType:
                    "El número de opciones seleccionadas {{numSelected}} para la lista {{type}} es mayor que el máximo {{max}}",
                underVote:
                    "Subvoto: El número de opciones seleccionadas {{numSelected}} es menor que el máximo permitido de {{max}}",
            },
            explicit: {
                notAllowed:
                    "Voto marcado explícitamente como inválido pero la pregunta no lo permite",
            },
            page: {
                oopsWithStatus: "¡Vaya! {{status}}",
                oopsWithoutStatus: "¡Vaya! Error Inesperado",
                somethingWrong: "Algo salió mal.",
            },
        },
        materials: {
            common: {
                label: "Materiales de Soporte",
                back: "Volver a la Lista de Votaciones",
                close: "Cerrar",
                preview: "Vista previa",
            },
        },
        ballotLocator: {
            title: "Localiza tu Papeleta",
            titleResult: "Resultado de la búsqueda de tu Papeleta",
            description: "Verifique que su Papeleta ha sido emitida correctamente",
            locate: "Localiza tu Papeleta",
            locateAgain: "Localiza otra Papeleta",
            found: "Tu ID de Papeleta {{ballotId}} ha sido localizada",
            notFound: "Tu ID de Papeleta {{ballotId}} no ha sido localizada",
            contentDesc: "Este es el contenido de tu Papeleta: ",
            wrongFormatBallotId: "Formato incorrecto para el ID de la Papeleta",
            steps: {
                lookup: "Localiza tu Papeleta",
                result: "Resultado",
            },
            titleHelpDialog: {
                title: "Información: pantalla de Localización de tu Papeleta",
                content:
                    "Esta pantalla le permite al votante encontrar su Papeleta utilizando el ID de la Papeleta para recuperarlo. Este procedimiento permite comprobar que su voto fue emitido correctamente y que el voto registrado coincide con el voto cifrado que emitió.",
                ok: "OK",
            },
        },
    },
}

export default spanishTranslation
