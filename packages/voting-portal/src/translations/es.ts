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
                "Para realizar cambios en sus selecciones, haga clic en el botón “<b>Editar selección</b>”, para confirmar sus selecciones, haga clic en el botón “<b>Enviar tu voto</b>” debajo, y para auditar su papeleta haga clic en el botón “<b>Auditar papeleta</b>” debajo.",
            descriptionNoAudit:
                "Para realizar cambios en sus selecciones, haga clic en el botón “<b>Editar selección</b>”, para confirmar sus selecciones, haga clic en el botón “<b>Enviar tu voto</b>” debajo.",
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
            confirmCastVoteDialog: {
                title: "¿Está seguro de que quiere emitir su voto?",
                content: "Su voto no se podrá editar una vez confirmado.",
                ok: "Sí, quiero EMITIR mi voto",
                cancel: "Cancelar",
            },
            error: {
                NETWORK_ERROR:
                    "Hubo un problema de red. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                UNABLE_TO_FETCH_DATA:
                    "Hubo un problema al recuperar los datos. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                LOAD_ELECTION_EVENT:
                    "No se puede cargar el evento electoral. Por favor, inténtalo de nuevo más tarde.",
                CAST_VOTE:
                    "Ha ocurrido un error desconocido al emitir el voto. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
                CAST_VOTE_CheckStatusFailed:
                    "La elección no permite emitir el voto. La elección puede estar cerrada, archivada o tal vez estés intentando votar fuera del período de gracia.",
                CAST_VOTE_AreaNotFound:
                    "Ha ocurrido un error al emitir el voto: Área no encontrada. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
                CAST_VOTE_InternalServerError:
                    "Ha ocurrido un error interno al emitir el voto. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
                CAST_VOTE_QueueError:
                    "Ha ocurrido un problema al procesar su voto. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
                CAST_VOTE_Unauthorized:
                    "No está autorizado para emitir un voto. Por favor, contacte con soporte para obtener ayuda.",
                CAST_VOTE_ElectionEventNotFound:
                    "No se pudo encontrar el evento electoral. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
                CAST_VOTE_ElectoralLogNotFound:
                    "No se pudo encontrar su registro de votación. Por favor, contacte con soporte para obtener ayuda.",
                CAST_VOTE_CheckPreviousVotesFailed:
                    "Ha ocurrido un error al verificar su estado de votación. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
                CAST_VOTE_GetClientCredentialsFailed:
                    "No se pudieron verificar sus credenciales. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
                CAST_VOTE_GetAreaIdFailed:
                    "Ha ocurrido un error al verificar su área de votación. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
                CAST_VOTE_GetTransactionFailed:
                    "Ha ocurrido un error al procesar su voto. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
                CAST_VOTE_DeserializeBallotFailed:
                    "Ha ocurrido un error al leer su papeleta. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
                CAST_VOTE_DeserializeContestsFailed:
                    "Ha ocurrido un error al leer sus selecciones. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
                CAST_VOTE_PokValidationFailed:
                    "No se pudo validar su voto. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
                CAST_VOTE_UuidParseFailed:
                    "Ha ocurrido un error al procesar su solicitud. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
                CAST_VOTE_unexpected:
                    "Ha ocurrido un error desconocido al emitir el voto. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
                CAST_VOTE_UnknownError:
                    "Ha ocurrido un error desconocido al emitir el voto. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
                NO_BALLOT_SELECTION:
                    "El estado de selección para esta elección no está presente. Asegúrate de haber seleccionado correctamente tus opciones o contacta con el soporte.",
                NO_BALLOT_STYLE:
                    "El estilo de la papeleta no está disponible. Por favor, contacta con el soporte.",
                NO_AUDITABLE_BALLOT:
                    "No hay una papeleta verificable disponible. Por favor, contacta con el soporte.",
                INCONSISTENT_HASH:
                    "Hubo un error relacionado con el proceso de hash de la papeleta. El BallotId: {{ballotId}} no es coherente con el Hash de la Papeleta Verificable: {{auditableBallotHash}}. Por favor, informa de este problema al soporte.",
                ELECTION_EVENT_NOT_OPEN:
                    "El evento electoral está cerrado. Por favor, contacta con el soporte.",
                PARSE_ERROR:
                    "Hubo un error al analizar la papeleta. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                DESERIALIZE_AUDITABLE_ERROR:
                    "Hubo un error al deserializar la papeleta verificable. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                DESERIALIZE_HASHABLE_ERROR:
                    "Hubo un error al deserializar la papeleta hashable. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                CONVERT_ERROR:
                    "Hubo un error al convertir la papeleta. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                SERIALIZE_ERROR:
                    "Hubo un error al serializar la papeleta. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                UNKNOWN_ERROR:
                    "Hubo un error. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                REAUTH_FAILED:
                    "La autenticación ha fallado. Por favor, inténtalo de nuevo o contacta con el soporte para obtener ayuda.",
                SESSION_EXPIRED:
                    "Tu sesión ha expirado. Por favor, intenta de nuevo desde el principio.",
                CAST_VOTE_BallotIdMismatch:
                    "El identificador de la papeleta no coincide con el del voto emitido.",
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
            demoBallotUrlDialog: {
                title: "Rastreador de Boletas",
                content: "No se puede usar el código, deshabilitado en modo de demostración.",
                ok: "OK",
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
            errorDialogPrintBallotReceipt: {
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
                "<VerifierLink>Accede al verificador del voto</VerifierLink>, que se abrirá una nueva pestaña en tu navegador.",
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
            errors: {
                noVotingArea:
                    "Área electoral no asignada al votante. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
                networkError:
                    "Hubo un problema de red. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
                unableToFetchData:
                    "Hubo un problema al obtener los datos. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
                noElectionEvent:
                    "El evento electoral no existe. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
                ballotStylesEmlError:
                    "Hubo un error con la publicación del estilo de la papeleta. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
                obtainingElectionFromID:
                    "Hubo un error al obtener las elecciones asociadas con los siguientes IDs de elecciones: {{electionIds}}. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
            },
            alerts: {
                noElections:
                    "No hay elecciones en las que pueda votar. Esto podría deberse a que el área no tiene ningún concurso asociado. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                electionEventNotPublished:
                    "El evento electoral aún no ha sido publicado. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
            },
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
                overVoteDisabled:
                    "Máximo alcanzado: Has seleccionado el máximo de {{numSelected}} opciones. Para cambiar tu selección, por favor, desmarca primero otra opción.",
                blankVote: "Voto en Blanco: 0 opciones seleccionadas",
            },
            explicit: {
                notAllowed:
                    "Voto marcado explícitamente como inválido pero la pregunta no lo permite",
                alert: "La selección marcada será considerada voto inválido.",
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
