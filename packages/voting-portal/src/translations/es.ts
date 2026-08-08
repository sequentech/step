// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const spanishTranslation: TranslationType = {
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
            clearButton: "Limpiar opciones",
            ballotHelpDialog: {
                title: "Sobre esta pantalla",
                content:
                    "Esta pantalla muestra la votación en la que usted es elegible para votar. Puede seleccionar su sección activando la casilla de la derecha Candidato/Respuesta. Para restablecer sus selecciones, haga clic en el botón “<b>Borrar selección</b>”, para pasar al siguiente paso, haga clic en el botón “<b>Siguiente</b>”.",
                ok: "OK",
            },
            nonVotedDialog: {
                title: "Tu voto es inválido o está en blanco",
                content:
                    "Algunas de sus respuestas harán que la papeleta en una o más preguntas sea inválida o en blanco.",
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
            instructionsDescription: "Siga estos pasos para emitir su voto",
            step1Title: "1. Elija sus opciones",
            step1Description:
                "Elija a sus candidatos preferidos y responda cada pregunta de la papeleta según aparezca. Puede cambiar sus opciones en cualquier momento antes de emitir su voto",
            step2Title: "2. Revise sus elecciones",
            step2Description:
                "Cuando esté satisfecho con sus selecciones, cifraremos su papeleta de forma segura y le mostraremos una revisión final. También recibirá un ID de seguimiento único como referencia",
            step3Title: "3. Envíe su voto",
            step3Description:
                "Cuando esté listo, emita su papeleta para que quede registrada oficialmente. O elija auditar primero para confirmar que fue correctamente capturada y cifrada",
        },
        reviewScreen: {
            title: "Revisa tu voto",
            description:
                "Para realizar cambios en sus selecciones, haga clic en el botón “<b>Editar selección</b>”, para confirmar sus selecciones, haga clic en el botón “<b>Enviar tu voto</b>” debajo, y para auditar su papeleta haga clic en el botón “<b>Auditar papeleta</b>” debajo.",
            descriptionNoAudit:
                "Para realizar cambios en sus selecciones, haga clic en el botón “<b>Editar selección</b>”, para confirmar sus selecciones, haga clic en el botón “<b>Enviar tu voto</b>” debajo.",
            backButton: "Editar tu voto",
            castBallotButton: "Enviar voto",
            auditButton: "Auditar papeleta",
            reviewScreenHelpDialog: {
                title: "Sobre la pantalla de revisión",
                content: "Esta pantalla le permite revisar sus selecciones antes de emitir su voto",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Tu voto no ha sido emitido",
                content:
                    "<p>Este es su Localizador del Voto, pero <b>su voto aún no ha sido emitido</b>. Si intenta buscarlo ahora, no aparecerá.</p><p>Mostramos el Localizador del Voto en esta etapa para que pueda auditar la papeleta cifrada antes de emitirla.</p>",
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
                title: "¿Está seguro de que quiere emitir su voto?",
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
                CAST_VOTE_timeout:
                    "Error de tiempo de espera para emitir el voto. Inténtalo de nuevo más tarde o contacta con el soporte técnico.",
                CAST_VOTE_InsertFailedExceedsAllowedRevotes:
                    "Has superado el límite de revotos. Inténtalo de nuevo más tarde o contacta con el soporte técnico.",
                CAST_VOTE_CheckRevotesFailed:
                    "Has superado el número permitido de revotos. Inténtalo de nuevo más tarde o contacta con el soporte técnico.",
                CAST_VOTE_CheckVotesInOtherAreasFailed:
                    "Ya has votado en otra área. Inténtalo de nuevo más tarde o contacta con el soporte técnico.",
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
                SESSION_STORAGE_ERROR:
                    "El almacenamiento de sesión no está disponible. Por favor, inténtelo de nuevo o contacte con soporte.",
                PARSE_BALLOT_DATA_ERROR:
                    "Hubo un error al analizar los datos de la papeleta. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
                NOT_VALID_BALLOT_DATA_ERROR:
                    "Los datos de la papeleta no son válidos. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
                FETCH_DATA_TIMEOUT_ERROR:
                    "Error de tiempo de espera al obtener los datos. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
                TO_HASHABLE_BALLOT_ERROR:
                    "Error al convertir a papeleta hashable. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
                INTERNAL_ERROR:
                    "Hubo un error interno al emitir el voto. Por favor, inténtelo de nuevo más tarde o contacte con soporte para obtener ayuda.",
            },
            declineToVote: "Declinar votar",
        },
        confirmationScreen: {
            title: "Su voto ha sido emitido",
            description:
                "Su papeleta fue emitida correctamente. Use el código a continuación para verificar que fue contabilizada",
            ballotId: "Localizador del Voto",
            printButton: "Imprimir",
            finishButton: "Finalizar",
            verifyCastTitle: "Compruebe que su voto fue emitido",
            verifyCastDescription:
                "Puede verificar en cualquier momento que su papeleta fue emitida correctamente usando el código QR a continuación",
            confirmationHelpDialog: {
                title: "Sobre la pantalla de confirmación",
                content:
                    "Esta pantalla confirma que su voto fue emitido correctamente. La información aquí le permite verificar que su papeleta fue almacenada en la urna, tanto durante el periodo de votación como después de su cierre",
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
                    "El Localizador del Voto es un código único que le permite encontrar su papeleta en la urna. No contiene información sobre sus selecciones.",
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
                content: "Ha ocurrido un error, por favor intenta de nuevo",
                ok: "Aceptar",
            },
            demoQRText: "El rastreador de papeletas está deshabilitado en modo de demostración",
        },
        auditScreen: {
            printButton: "Imprimir",
            restartButton: "Iniciar votación",
            title: "Compruebe su papeleta",
            description: "Para comprobar su papeleta, siga los pasos a continuación:",
            step1Title: "1. Guarde los siguientes datos:",
            step1Description:
                "su <b>Localizador del Voto</b> en la parte superior de la pantalla y su papeleta encriptada a continuación",
            step1HelpDialog: {
                title: "Copiar código de la papeleta",
                content:
                    "Puede descargar o copiar el código de su papeleta para verificar que refleja correctamente sus selecciones.",
                ok: "OK",
            },
            downloadButton: "Descargar",
            step2Title: "2. Comprueba tu papeleta",
            step2Description:
                "Haz clic en <VerifierLink>Comprueba el código de tu papeleta</VerifierLink>. Se abrirá en una nueva pestaña",
            step2HelpDialog: {
                title: "Cómo comprobar el código de la papeleta",
                content:
                    "Para comprobar el código de su papeleta, siga los pasos de la guía. Incluye la descarga de una aplicación de escritorio para verificar su papeleta de forma independiente al sitio web.",
                ok: "OK",
            },
            bottomWarning:
                "Por motivos de seguridad, cuando audite su papeleta, deberá invalidarla. Para continuar con el proceso de votación, haga clic en ‘<b>Iniciar votación</b>’.",
        },
        electionSelectionScreen: {
            title: "Lista de Votaciones",
            description: "Seleccione la papeleta en la que desea votar",
            chooserHelpDialog: {
                title: "Sobre la Lista de Votaciones",
                content:
                    "Esta pantalla muestra la lista de papeletas a las que puede acceder. Pueden estar abiertas, programadas o cerradas. Solo puede votar en las que están abiertas",
                ok: "OK",
            },
            noResults: "No hay elecciones por ahora.",
            resultsButton: "Ver resultados",
            demoDialog: {
                title: "Cabina de votación de demostración",
                content:
                    "Está entrando en una cabina de votación de demostración. <strong>Su voto no será registrado.</strong> Esta cabina es solo para demostración.",
                ok: "Entiendo que mi voto no será registrado",
            },
            errors: {
                noVotingArea:
                    "No estás registrado como votante en esta elección. Por favor, contacta con el soporte.",
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
                    "No hay elecciones en las que pueda votar. Esto podría deberse a que el área no tiene ninguna pregunta asociada. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
                electionEventNotPublished:
                    "El evento electoral aún no ha sido publicado. Por favor, inténtelo de nuevo más tarde o contacte con el soporte para obtener ayuda.",
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
                    "La papeleta está marcada como explícitamente inválida, pero la pregunta no lo permite",
                alert: "Esta selección se contará como un voto inválido",
            },
            page: {
                oopsWithStatus: "¡Vaya! {{status}}",
                oopsWithoutStatus: "¡Vaya! Error Inesperado",
                somethingWrong: "Algo salió mal.",
                certAuthFailedTitle: "Error de Autenticación con Certificado",
                certAuthFailedMessage:
                    "No se ha podido verificar su certificado. Compruebe que está usando un certificado de votante válido e inténtelo de nuevo.",
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
            title: "Encuentra tu Papeleta",
            titleResult: "Resultados de tu búsqueda de Papeleta",
            description: "Confirme que su papeleta fue emitida correctamente",
            locate: "Encuentra tu Papeleta",
            locateAgain: "Encuentra otra Papeleta",
            found: "Tu ID de Papeleta {{ballotId}} ha sido encontrada",
            notFound: "Tu ID de Papeleta {{ballotId}} no fue encontrada",
            ambiguous:
                "Más de una de tus papeletas coincide con {{ballotId}}. Usa el ID de papeleta completo.",
            contentDesc: "Este es el contenido de tu Papeleta: ",
            wrongFormatBallotId: "Formato incorrecto para el ID de la Papeleta",
            ballotIdNotFoundAtFilter:
                "No encontrado, compruebe que el ID de la Papeleta sea correcto y pertenezca a este usuario.",
            filterByBallotId: "Filtrar por ID de Papeleta",
            totalBallots: "Papeletas: {{total}}",
            steps: {
                lookup: "Encuentra tu Papeleta",
                result: "Resultado",
            },
            titleHelpDialog: {
                title: "Sobre el Buscador de Papeletas",
                content:
                    "El Buscador de Papeletas le permite introducir su ID de Papeleta para localizar su voto y confirmar que fue registrado correctamente.",
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

export default spanishTranslation
