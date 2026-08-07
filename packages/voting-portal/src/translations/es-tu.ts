// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const spanishInformalTranslation: TranslationType = {
    translations: {
        common: {
            goBack: "Regresar",
            showMore: "Mostrar más",
            showLess: "Mostrar menos",
        },
        candidatesList: {
            collapseToggle: "Alternar lista {{listTitle}}",
            showCandidates: "Mostrar candidatos",
            hideCandidates: "Ocultar candidatos",
            selectedCandidates_one: "{{count}} candidato seleccionado",
            selectedCandidates_other: "{{count}} candidatos seleccionados",
            expandAll: "Expandir todo",
            collapseAll: "Contraer todo",
        },
        breadcrumbSteps: {
            electionList: "Votaciones",
            ballot: "Papeleta",
            review: "Revisión",
            confirmation: "Confirmar",
            audit: "Auditar",
        },
        footer: {
            poweredBy: "Funciona con <1></1>",
        },
        votingScreen: {
            backButton: "Atrás",
            reviewButton: "Siguiente",
            clearButton: "Limpiar selecciones",
            ballotHelpDialog: {
                title: "Sobre esta pantalla",
                content:
                    "Esta pantalla muestra las contiendas en las que eres elegible para votar. Puedes hacer tu selección activando la casilla a la derecha del Candidato/Respuesta. Para restablecer tus selecciones, haz clic en el botón “<b>Limpiar selecciones</b>”, para pasar al siguiente paso, haz clic en el botón “<b>Siguiente</b>”.",
                ok: "OK",
            },
            nonVotedDialog: {
                title: "Tu voto es inválido o está en blanco",
                content:
                    "Algunas de tus respuestas harán que la papeleta en una o más contiendas sea inválida o en blanco.",
                ok: "Revisar selección",
                continue: "Continuar",
                cancel: "Cancelar",
            },
            warningDialog: {
                title: "Revisa tu papeleta",
                content:
                    "Tu papeleta contiene selecciones que pueden necesitar tu atención (como seleccionar menos opciones de las permitidas). Tu papeleta es válida y se contará tal como se ha enviado.",
                ok: "Volver y revisar",
                continue: "Continuar",
                cancel: "Cancelar",
            },
        },
        startScreen: {
            startButton: "Empezar a votar",
            declineToVoteButton: "Declinar votar",
            declineToVoteDialog: {
                title: "Confirmar declinación de voto",
                content:
                    "¿Estás seguro de que deseas declinar votar?<br />Irás directamente a la revisión y tu estado de participación se guardará como <b>Ha declinado votar</b>.",
                continue: "Declinar votar",
                cancel: "Cancelar",
            },
            instructionsTitle: "Cómo votar",
            instructionsDescription: "Sigue estos pasos para emitir tu voto",
            step1Title: "1. Haz tus selecciones",
            step1Description:
                "Elige a tus candidatos preferidos y responde cada contienda de la papeleta según aparezca. Puedes cambiar tus selecciones en cualquier momento antes de emitir tu voto",
            step2Title: "2. Revisa tus selecciones",
            step2Description:
                "Cuando estés satisfecho con tus selecciones, cifraremos tu papeleta de forma segura y te mostraremos una revisión final. También recibirás un ID de seguimiento único como referencia",
            step3Title: "3. Emite tu papeleta",
            step3Description:
                "Cuando estés listo, emite tu papeleta para que quede registrada oficialmente. O elige auditar primero para confirmar que fue correctamente capturada y cifrada",
        },
        reviewScreen: {
            title: "Revisa tu voto",
            description:
                "Para realizar cambios en tus selecciones, haz clic en el botón “<b>Editar selección</b>”, para confirmar tus selecciones, haz clic en el botón “<b>Enviar tu voto</b>” debajo, y para auditar tu papeleta haz clic en el botón “<b>Auditar papeleta</b>” debajo.",
            descriptionNoAudit:
                "Para realizar cambios en tus selecciones, haz clic en el botón “<b>Editar selección</b>”, para confirmar tus selecciones, haz clic en el botón “<b>Enviar tu voto</b>” debajo.",
            backButton: "Editar tu voto",
            castBallotButton: "Enviar voto",
            auditButton: "Auditar papeleta",
            reviewScreenHelpDialog: {
                title: "Sobre la pantalla de revisión",
                content: "Esta pantalla te permite revisar tus selecciones antes de emitir tu voto",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Tu voto no ha sido emitido",
                content:
                    "<p>Este es tu Localizador del Voto, pero <b>tu voto aún no ha sido emitido</b>. Si intentas buscarlo ahora, no aparecerá.</p><p>Mostramos el Localizador del Voto en esta etapa para que puedas auditar la papeleta cifrada antes de emitirla.</p>",
                ok: "Entiendo que mi voto no ha sido emitido",
                cancel: "Cancelar",
            },
            auditBallotHelpDialog: {
                title: "¿Quieres auditar tu papeleta?",
                content:
                    "<p>Auditar tu papeleta la invalidará y tendrás que reiniciar el proceso de votación. Continúa solo si te sientes cómodo con los pasos técnicos avanzados. De lo contrario, haz clic en <u>Cancelar</u> para volver.</p>",
                ok: "Sí, descartar mi papeleta para auditarla",
                cancel: "Cancelar",
            },
            confirmCastVoteDialog: {
                title: "¿Estás seguro de que quieres emitir tu voto?",
                content: "Una vez que confirmes, tu voto será emitido.",
                ok: "Sí, quiero emitir mi voto",
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
                    "Ha ocurrido un error desconocido al emitir el voto. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                CAST_VOTE_CheckStatusFailed:
                    "La elección no permite emitir el voto. La elección puede estar cerrada, archivada o quizá estés intentando votar fuera del período de gracia.",
                CAST_VOTE_AreaNotFound:
                    "Ha ocurrido un error al emitir el voto: área no encontrada. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                CAST_VOTE_InternalServerError:
                    "Ha ocurrido un error interno al emitir el voto. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                CAST_VOTE_QueueError:
                    "Ha ocurrido un problema al procesar tu voto. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                CAST_VOTE_Unauthorized:
                    "No estás autorizado para emitir un voto. Por favor, contacta con el soporte para obtener ayuda.",
                CAST_VOTE_ElectionEventNotFound:
                    "No se pudo encontrar el evento electoral. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                CAST_VOTE_ElectoralLogNotFound:
                    "No se pudo encontrar tu registro de votación. Por favor, contacta con el soporte para obtener ayuda.",
                CAST_VOTE_CheckPreviousVotesFailed:
                    "Ha ocurrido un error al verificar tu estado de votación. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                CAST_VOTE_GetClientCredentialsFailed:
                    "No se pudieron verificar tus credenciales. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                CAST_VOTE_GetAreaIdFailed:
                    "Ha ocurrido un error al verificar tu área de votación. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                CAST_VOTE_GetTransactionFailed:
                    "Ha ocurrido un error al procesar tu voto. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                CAST_VOTE_DeserializeBallotFailed:
                    "Ha ocurrido un error al leer tu papeleta. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                CAST_VOTE_DeserializeContestsFailed:
                    "Ha ocurrido un error al leer tus selecciones. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                CAST_VOTE_PokValidationFailed:
                    "No se pudo validar tu voto. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                CAST_VOTE_UuidParseFailed:
                    "Ha ocurrido un error al procesar tu solicitud. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                CAST_VOTE_unexpected:
                    "Ha ocurrido un error desconocido al emitir el voto. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                CAST_VOTE_timeout:
                    "Error de tiempo de espera al emitir el voto. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                CAST_VOTE_InsertFailedExceedsAllowedRevotes:
                    "Has superado el límite de revotos. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                CAST_VOTE_CheckRevotesFailed:
                    "Has superado el número permitido de revotos. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                CAST_VOTE_CheckVotesInOtherAreasFailed:
                    "Ya has votado en otra área. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                CAST_VOTE_UnknownError:
                    "Ha ocurrido un error desconocido al emitir el voto. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
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
                    "Tu sesión ha expirado. Por favor, inténtalo de nuevo desde el principio.",
                CAST_VOTE_BallotIdMismatch:
                    "El identificador de la papeleta no coincide con el del voto emitido.",
                SESSION_STORAGE_ERROR:
                    "El almacenamiento de sesión no está disponible. Por favor, inténtalo de nuevo o contacta con el soporte.",
                PARSE_BALLOT_DATA_ERROR:
                    "Hubo un error al analizar los datos de la papeleta. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                NOT_VALID_BALLOT_DATA_ERROR:
                    "Los datos de la papeleta no son válidos. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                FETCH_DATA_TIMEOUT_ERROR:
                    "Error de tiempo de espera al obtener los datos. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                TO_HASHABLE_BALLOT_ERROR:
                    "Error al convertir a papeleta hashable. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                INTERNAL_ERROR:
                    "Hubo un error interno al emitir el voto. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
            },
            declineToVote: "Declinar votar",
        },
        confirmationScreen: {
            title: "Tu voto ha sido emitido",
            description:
                "Tu papeleta fue emitida correctamente. Usa el código a continuación para verificar que fue contabilizada",
            ballotId: "Localizador del Voto",
            printButton: "Imprimir",
            finishButton: "Finalizar",
            verifyCastTitle: "Comprueba que tu voto fue emitido",
            verifyCastDescription:
                "Puedes verificar en cualquier momento que tu papeleta fue emitida correctamente usando el código QR a continuación",
            confirmationHelpDialog: {
                title: "Sobre la pantalla de confirmación",
                content:
                    "Esta pantalla confirma que tu voto fue emitido correctamente. La información aquí te permite verificar que tu papeleta fue almacenada en la urna, tanto durante el periodo de votación como después de su cierre",
                ok: "OK",
            },
            demoPrintDialog: {
                title: "Impresión de la papeleta de votación",
                content: "La impresión está desactivada en modo de demostración",
                ok: "Aceptar",
            },
            demoBallotUrlDialog: {
                title: "Rastreador de Papeletas",
                content: "No se puede usar el código, deshabilitado en modo de demostración.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Sobre el Localizador del Voto",
                content:
                    "El Localizador del Voto es un código único que te permite encontrar tu papeleta en la urna. No contiene información sobre tus selecciones.",
                ok: "OK",
            },
            ballotIdDemoHelpDialog: {
                title: "Sobre el Localizador del Voto",
                content:
                    "La identificación de la papeleta es un código que te permite encontrar tu papeleta en la urna. Este identificador es único y no contiene información sobre tus selecciones.",
                ok: "Aceptar",
            },
            errorDialogPrintBallotReceipt: {
                title: "Error",
                content: "Ha ocurrido un error, por favor inténtalo de nuevo",
                ok: "Aceptar",
            },
            demoQRText: "El rastreador de papeletas está deshabilitado en modo de demostración",
        },
        auditScreen: {
            printButton: "Imprimir",
            restartButton: "Iniciar votación",
            title: "Comprueba tu papeleta",
            description: "Para comprobar tu papeleta, sigue los pasos a continuación:",
            step1Title: "1. Guarda los siguientes datos:",
            step1Description:
                "tu <b>Localizador del Voto</b> en la parte superior de la pantalla y tu papeleta encriptada a continuación",
            step1HelpDialog: {
                title: "Copiar código de la papeleta",
                content:
                    "Puedes descargar o copiar el código de tu papeleta para verificar que refleja correctamente tus selecciones.",
                ok: "OK",
            },
            downloadButton: "Descargar",
            step2Title: "2. Comprueba tu papeleta",
            step2Description:
                "Haz clic en <VerifierLink>Comprueba el código de tu papeleta</VerifierLink>. Se abrirá en una nueva pestaña",
            step2HelpDialog: {
                title: "Cómo comprobar el código de la papeleta",
                content:
                    "Para comprobar el código de tu papeleta, sigue los pasos de la guía. Incluye la descarga de una aplicación de escritorio para verificar tu papeleta de forma independiente al sitio web.",
                ok: "OK",
            },
            bottomWarning:
                "Por motivos de seguridad, cuando audites tu papeleta, deberás invalidarla. Para continuar con el proceso de votación, haz clic en ‘<b>Iniciar votación</b>’.",
        },
        electionSelectionScreen: {
            title: "Lista de Votaciones",
            description: "Selecciona la papeleta en la que deseas votar",
            chooserHelpDialog: {
                title: "Sobre la Lista de Votaciones",
                content:
                    "Esta pantalla muestra la lista de papeletas a las que puedes acceder. Pueden estar abiertas, programadas o cerradas. Solo puedes votar en las que están abiertas",
                ok: "OK",
            },
            noResults: "No hay elecciones por ahora.",
            resultsButton: "Ver resultados",
            demoDialog: {
                title: "Cabina de votación de demostración",
                content:
                    "Estás entrando en una cabina de votación de demostración. <strong>Tu voto no será registrado.</strong> Esta cabina es solo para demostración.",
                ok: "Entiendo que mi voto no será registrado",
            },
            errors: {
                noVotingArea:
                    "No estás registrado como votante en esta elección. Por favor, contacta con el soporte.",
                networkError:
                    "Hubo un problema de red. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                unableToFetchData:
                    "Hubo un problema al obtener los datos. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                noElectionEvent:
                    "El evento electoral no existe. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                ballotStylesEmlError:
                    "Hubo un error con la publicación del estilo de la papeleta. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                obtainingElectionFromID:
                    "Hubo un error al obtener las elecciones asociadas con los siguientes IDs de elecciones: {{electionIds}}. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
            },
            alerts: {
                noElections:
                    "No hay elecciones en las que puedas votar. Esto podría deberse a que el área no tiene ninguna contienda asociada. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
                electionEventNotPublished:
                    "El evento electoral aún no ha sido publicado. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
            },
        },
        errors: {
            encoding: {
                notEnoughChoices: "No hay suficientes opciones para decodificar",
                writeInChoiceOutOfRange: "Opción de escritura libre fuera de rango: {{index}}",
                writeInNotEndInZero: "La escritura libre no termina en 0",
                bytesToUtf8Conversion:
                    "Error al convertir la escritura libre de bytes a cadena UTF-8: {{errorMessage}}",
                ballotTooLarge: "La papeleta es más grande de lo esperado",
            },
            explicit: {
                notAllowed:
                    "La papeleta está marcada como explícitamente inválida, pero la contienda no lo permite",
                alert: "Esta selección se contará como un voto inválido",
            },
            page: {
                oopsWithStatus: "¡Vaya! {{status}}",
                oopsWithoutStatus: "¡Vaya! Error Inesperado",
                somethingWrong: "Algo salió mal.",
                certAuthFailedTitle: "Error de Autenticación con Certificado",
                certAuthFailedMessage:
                    "No se ha podido verificar tu certificado. Comprueba que estás usando un certificado de votante válido e inténtalo de nuevo.",
            },
        },
        materials: {
            common: {
                label: "Materiales de Soporte",
                back: "Volver a la lista de votaciones",
                close: "Cerrar",
                preview: "Vista previa",
            },
        },
        ballotLocator: {
            title: "Encuentra tu papeleta",
            titleResult: "Resultados de tu búsqueda de Papeleta",
            description: "Confirma que tu papeleta fue emitida correctamente",
            locate: "Encuentra tu papeleta",
            locateAgain: "Encuentra otra papeleta",
            found: "Tu ID de Papeleta {{ballotId}} ha sido encontrado",
            notFound: "Tu ID de Papeleta {{ballotId}} no fue encontrado",
            ambiguous:
                "Más de una de tus papeletas coincide con {{ballotId}}. Usa el ID de papeleta completo.",
            contentDesc: "Este es el contenido de tu papeleta: ",
            wrongFormatBallotId: "Formato incorrecto para el ID de la Papeleta",
            ballotIdNotFoundAtFilter:
                "No encontrado, comprueba que el ID de la papeleta sea correcto y pertenezca a este usuario.",
            filterByBallotId: "Filtrar por ID de Papeleta",
            totalBallots: "Papeletas: {{total}}",
            steps: {
                lookup: "Encuentra tu papeleta",
                result: "Resultado",
            },
            titleHelpDialog: {
                title: "Sobre el Buscador de Papeletas",
                content:
                    "El Buscador de Papeletas te permite introducir tu ID de Papeleta para localizar tu voto y confirmar que fue registrado correctamente.",
                ok: "OK",
            },
            tabs: {
                logs: "Logs",
                ballotLocator: "Localizador de Papeletas",
            },
            column: {
                statement_kind: "Tipo",
                statement_timestamp: "Marca de tiempo",
                username: "Usuario",
                ballot_id: "ID de Papeleta",
                message: "Mensaje",
            },
        },
    },
}

export default spanishInformalTranslation
