// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ETranslationPresetTemplate, ITranslationPresetTemplate} from "@/types/translationPresets"

/**
 * Addresses voters as *tú* where the `es` bundles use *usted*. Only the strings
 * whose wording depends on the register are listed.
 */
export const spanishInformalTemplate: ITranslationPresetTemplate = {
    id: ETranslationPresetTemplate.SPANISH_INFORMAL,
    language: "es",
    overrides: {
        "global:homeScreen.startButton": "Selecciona fichero",
        "global:homeScreen.dragDropOption": "O arrastra el fichero aquí",
        "global:homeScreen.importErrorDescription":
            "Hubo un problema al importar el voto auditable. ¿Elegiste el archivo correcto?",
        "global:homeScreen.useSampleText": "¿No tienes un voto verificable?",
        "global:homeScreen.useSampleLink": "Usa un voto verificable de ejemplo",
        "global:confirmationScreen.bottomDescription1":
            "Tu voto fue cifrado correctamente. Ahora puedes cerrar esta ventana y volver a la Cabina de Votación.",
        "global:confirmationScreen.bottomDescription2":
            "Si no coinciden, haz clic aquí para obtener más información sobre los posibles motivos y las acciones que puedes tomar.",
        "global:confirmationScreen.ballotChoicesDescription": "Y tus selecciones de voto son:",
        "global:ballotSelectionsScreen.statusModal.content":
            "El panel de estado te da información sobre las verificaciones realizadas.",
        "global:errors.encoding.writeInCharsExceeded_one":
            "Acorta la escritura libre en {{count}} carácter.",
        "global:errors.encoding.writeInCharsExceeded_many":
            "Acorta la escritura libre en {{count}} caracteres.",
        "global:errors.encoding.writeInCharsExceeded_other":
            "Acorta la escritura libre en {{count}} caracteres.",
        "global:errors.implicit.selectedMax_one": "Desmarca {{count}} candidato.",
        "global:errors.implicit.selectedMax_many": "Desmarca {{count}} candidatos.",
        "global:errors.implicit.selectedMax_other": "Desmarca {{count}} candidatos.",
        "global:errors.implicit.selectedMin_one": "Selecciona {{count}} candidato más.",
        "global:errors.implicit.selectedMin_many": "Selecciona {{count}} candidatos más.",
        "global:errors.implicit.selectedMin_other": "Selecciona {{count}} candidatos más.",
        "global:errors.implicit.maxSelectionsPerType_one":
            "Desmarca {{count}} candidato de {{type}}.",
        "global:errors.implicit.maxSelectionsPerType_many":
            "Desmarca {{count}} candidatos de {{type}}.",
        "global:errors.implicit.maxSelectionsPerType_other":
            "Desmarca {{count}} candidatos de {{type}}.",
        "global:errors.implicit.underVote_one": "Selecciona hasta {{count}} candidato más.",
        "global:errors.implicit.underVote_many": "Selecciona hasta {{count}} candidatos más.",
        "global:errors.implicit.underVote_other": "Selecciona hasta {{count}} candidatos más.",
        "global:errors.implicit.overVoteDisabled_one":
            "Has seleccionado el máximo de {{count}} candidato. Desmárcalo para elegir otro.",
        "global:errors.implicit.overVoteDisabled_many":
            "Has seleccionado el máximo de {{count}} candidatos. Desmarca uno para elegir otro.",
        "global:errors.implicit.overVoteDisabled_other":
            "Has seleccionado el máximo de {{count}} candidatos. Desmarca uno para elegir otro.",
        "global:errors.implicit.blankVote": "No has seleccionado ningún candidato.",
        "global:ballotHash": "Tu Localizador de Voto: {{ballotId}}",
        "global:logout.modal.title": "¿Estás seguro de que quieres cerrar sesión?",
        "global:logout.modal.content":
            "Estás a punto de cerrar esta aplicación. Esta acción no se puede deshacer.",
        "global:selectElection.voteButton": "Haz clic para votar",
        "global:selectElection.ballotLocator": "Localiza tu voto",
        "global:header.session.title": "Tu sesión está a punto de expirar.",
        "global:header.session.timeLeft": "Te quedan {{time}} para emitir tu voto.",
        "votingPortal:votingScreen.ballotHelpDialog.content":
            "Esta pantalla muestra las preguntas en las que eres elegible para votar. Puedes hacer tu selección activando la casilla a la derecha del Candidato/Respuesta. Para restablecer tus selecciones, haz clic en el botón “<b>Limpiar selecciones</b>”, para pasar al siguiente paso, haz clic en el botón “<b>Siguiente</b>”.",
        "votingPortal:votingScreen.nonVotedDialog.title": "Tu voto es inválido o está en blanco",
        "votingPortal:votingScreen.nonVotedDialog.content":
            "Algunas de tus respuestas harán que la papeleta en una o más preguntas sea inválida o en blanco.",
        "votingPortal:votingScreen.warningDialog.title": "Revisa tu papeleta",
        "votingPortal:votingScreen.warningDialog.content":
            "Tu papeleta contiene selecciones que pueden necesitar tu atención (como seleccionar menos opciones de las permitidas). Tu papeleta es válida y se contará tal como se ha enviado.",
        "votingPortal:startScreen.declineToVoteDialog.content":
            "¿Estás seguro de que deseas declinar votar?<br />Irás directamente a la revisión y tu estado de participación se guardará como <b>Ha declinado votar</b>.",
        "votingPortal:startScreen.instructionsDescription": "Sigue estos pasos para emitir tu voto",
        "votingPortal:startScreen.step1Title": "1. Haz tus selecciones",
        "votingPortal:startScreen.step1Description":
            "Elige a tus candidatos preferidos y responde cada pregunta de la papeleta según aparezca. Puedes cambiar tus selecciones en cualquier momento antes de emitir tu voto",
        "votingPortal:startScreen.step2Title": "2. Revisa tus selecciones",
        "votingPortal:startScreen.step2Description":
            "Cuando estés satisfecho con tus selecciones, cifraremos tu papeleta de forma segura y te mostraremos una revisión final. También recibirás un ID de seguimiento único como referencia",
        "votingPortal:startScreen.step3Title": "3. Emite tu papeleta",
        "votingPortal:startScreen.step3Description":
            "Cuando estés listo, emite tu papeleta para que quede registrada oficialmente. O elige auditar primero para confirmar que fue correctamente capturada y cifrada",
        "votingPortal:reviewScreen.title": "Revisa tu voto",
        "votingPortal:reviewScreen.description":
            "Para realizar cambios en tus selecciones, haz clic en el botón “<b>Editar selección</b>”, para confirmar tus selecciones, haz clic en el botón “<b>Enviar tu voto</b>” debajo, y para auditar tu papeleta haz clic en el botón “<b>Auditar papeleta</b>” debajo.",
        "votingPortal:reviewScreen.descriptionNoAudit":
            "Para realizar cambios en tus selecciones, haz clic en el botón “<b>Editar selección</b>”, para confirmar tus selecciones, haz clic en el botón “<b>Enviar tu voto</b>” debajo.",
        "votingPortal:reviewScreen.backButton": "Editar tu voto",
        "votingPortal:reviewScreen.reviewScreenHelpDialog.content":
            "Esta pantalla te permite revisar tus selecciones antes de emitir tu voto",
        "votingPortal:reviewScreen.ballotIdHelpDialog.title": "Tu voto no ha sido emitido",
        "votingPortal:reviewScreen.ballotIdHelpDialog.content":
            "<p>Este es tu Localizador del Voto, pero <b>tu voto aún no ha sido emitido</b>. Si intentas buscarlo ahora, no aparecerá.</p><p>Mostramos el Localizador del Voto en esta etapa para que puedas auditar la papeleta cifrada antes de emitirla.</p>",
        "votingPortal:reviewScreen.auditBallotHelpDialog.title": "¿Quieres auditar tu papeleta?",
        "votingPortal:reviewScreen.auditBallotHelpDialog.content":
            "<p>Auditar tu papeleta la invalidará y tendrás que reiniciar el proceso de votación. Continúa solo si te sientes cómodo con los pasos técnicos avanzados. De lo contrario, haz clic en <u>Cancelar</u> para volver.</p>",
        "votingPortal:reviewScreen.confirmCastVoteDialog.title":
            "¿Estás seguro de que quieres emitir tu voto?",
        "votingPortal:reviewScreen.confirmCastVoteDialog.content":
            "Una vez que confirmes, tu voto será emitido.",
        "votingPortal:reviewScreen.error.NETWORK_ERROR":
            "Hubo un problema de red. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.UNABLE_TO_FETCH_DATA":
            "Hubo un problema al recuperar los datos. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.LOAD_ELECTION_EVENT":
            "No se puede cargar el evento electoral. Por favor, inténtalo de nuevo más tarde.",
        "votingPortal:reviewScreen.error.CAST_VOTE":
            "Ha ocurrido un error desconocido al emitir el voto. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.CAST_VOTE_CheckStatusFailed":
            "La elección no permite emitir el voto. La elección puede estar cerrada, archivada o quizá estés intentando votar fuera del período de gracia.",
        "votingPortal:reviewScreen.error.CAST_VOTE_AreaNotFound":
            "Ha ocurrido un error al emitir el voto: área no encontrada. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.CAST_VOTE_InternalServerError":
            "Ha ocurrido un error interno al emitir el voto. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.CAST_VOTE_QueueError":
            "Ha ocurrido un problema al procesar tu voto. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.CAST_VOTE_Unauthorized":
            "No estás autorizado para emitir un voto. Por favor, contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.CAST_VOTE_ElectionEventNotFound":
            "No se pudo encontrar el evento electoral. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.CAST_VOTE_ElectoralLogNotFound":
            "No se pudo encontrar tu registro de votación. Por favor, contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.CAST_VOTE_CheckPreviousVotesFailed":
            "Ha ocurrido un error al verificar tu estado de votación. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.CAST_VOTE_GetClientCredentialsFailed":
            "No se pudieron verificar tus credenciales. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.CAST_VOTE_GetAreaIdFailed":
            "Ha ocurrido un error al verificar tu área de votación. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.CAST_VOTE_GetTransactionFailed":
            "Ha ocurrido un error al procesar tu voto. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.CAST_VOTE_DeserializeBallotFailed":
            "Ha ocurrido un error al leer tu papeleta. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.CAST_VOTE_DeserializeContestsFailed":
            "Ha ocurrido un error al leer tus selecciones. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.CAST_VOTE_PokValidationFailed":
            "No se pudo validar tu voto. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.CAST_VOTE_UuidParseFailed":
            "Ha ocurrido un error al procesar tu solicitud. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.CAST_VOTE_unexpected":
            "Ha ocurrido un error desconocido al emitir el voto. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.CAST_VOTE_timeout":
            "Error de tiempo de espera al emitir el voto. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.CAST_VOTE_InsertFailedExceedsAllowedRevotes":
            "Has superado el límite de revotos. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.CAST_VOTE_CheckRevotesFailed":
            "Has superado el número permitido de revotos. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.CAST_VOTE_CheckVotesInOtherAreasFailed":
            "Ya has votado en otra área. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.CAST_VOTE_UnknownError":
            "Ha ocurrido un error desconocido al emitir el voto. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.NO_BALLOT_SELECTION":
            "El estado de selección para esta elección no está presente. Asegúrate de haber seleccionado correctamente tus opciones o contacta con el soporte.",
        "votingPortal:reviewScreen.error.NO_BALLOT_STYLE":
            "El estilo de la papeleta no está disponible. Por favor, contacta con el soporte.",
        "votingPortal:reviewScreen.error.NO_AUDITABLE_BALLOT":
            "No hay una papeleta verificable disponible. Por favor, contacta con el soporte.",
        "votingPortal:reviewScreen.error.INCONSISTENT_HASH":
            "Hubo un error relacionado con el proceso de hash de la papeleta. El BallotId: {{ballotId}} no es coherente con el Hash de la Papeleta Verificable: {{auditableBallotHash}}. Por favor, informa de este problema al soporte.",
        "votingPortal:reviewScreen.error.ELECTION_EVENT_NOT_OPEN":
            "El evento electoral está cerrado. Por favor, contacta con el soporte.",
        "votingPortal:reviewScreen.error.PARSE_ERROR":
            "Hubo un error al analizar la papeleta. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.DESERIALIZE_AUDITABLE_ERROR":
            "Hubo un error al deserializar la papeleta verificable. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.DESERIALIZE_HASHABLE_ERROR":
            "Hubo un error al deserializar la papeleta hashable. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.CONVERT_ERROR":
            "Hubo un error al convertir la papeleta. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.SERIALIZE_ERROR":
            "Hubo un error al serializar la papeleta. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.UNKNOWN_ERROR":
            "Hubo un error. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.REAUTH_FAILED":
            "La autenticación ha fallado. Por favor, inténtalo de nuevo o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.SESSION_EXPIRED":
            "Tu sesión ha expirado. Por favor, inténtalo de nuevo desde el principio.",
        "votingPortal:reviewScreen.error.SESSION_STORAGE_ERROR":
            "El almacenamiento de sesión no está disponible. Por favor, inténtalo de nuevo o contacta con el soporte.",
        "votingPortal:reviewScreen.error.PARSE_BALLOT_DATA_ERROR":
            "Hubo un error al analizar los datos de la papeleta. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.NOT_VALID_BALLOT_DATA_ERROR":
            "Los datos de la papeleta no son válidos. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.FETCH_DATA_TIMEOUT_ERROR":
            "Error de tiempo de espera al obtener los datos. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.TO_HASHABLE_BALLOT_ERROR":
            "Error al convertir a papeleta hashable. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:reviewScreen.error.INTERNAL_ERROR":
            "Hubo un error interno al emitir el voto. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:confirmationScreen.title": "Tu voto ha sido emitido",
        "votingPortal:confirmationScreen.description":
            "Tu papeleta fue emitida correctamente. Usa el código a continuación para verificar que fue contabilizada",
        "votingPortal:confirmationScreen.remainingElectionsError":
            "No pudimos comprobar si tienes más elecciones en las que votar. Vuelve a intentarlo.",
        "votingPortal:confirmationScreen.verifyCastTitle": "Comprueba que tu voto fue emitido",
        "votingPortal:confirmationScreen.verifyCastDescription":
            "Puedes verificar en cualquier momento que tu papeleta fue emitida correctamente usando el código QR a continuación",
        "votingPortal:confirmationScreen.confirmationHelpDialog.content":
            "Esta pantalla confirma que tu voto fue emitido correctamente. La información aquí te permite verificar que tu papeleta fue almacenada en la urna, tanto durante el periodo de votación como después de su cierre",
        "votingPortal:confirmationScreen.ballotIdHelpDialog.content":
            "El Localizador del Voto es un código único que te permite encontrar tu papeleta en la urna. No contiene información sobre tus selecciones.",
        "votingPortal:confirmationScreen.ballotIdDemoHelpDialog.content":
            "La identificación de la papeleta es un código que te permite encontrar tu papeleta en la urna. Este identificador es único y no contiene información sobre tus selecciones.",
        "votingPortal:confirmationScreen.errorDialogPrintBallotReceipt.content":
            "Ha ocurrido un error. Por favor, inténtalo de nuevo.",
        "votingPortal:auditScreen.title": "Comprueba tu papeleta",
        "votingPortal:auditScreen.description":
            "Para comprobar tu papeleta, sigue los pasos a continuación:",
        "votingPortal:auditScreen.step1Title": "1. Guarda los siguientes datos:",
        "votingPortal:auditScreen.step1Description":
            "tu <b>Localizador del Voto</b> en la parte superior de la pantalla y tu papeleta encriptada a continuación",
        "votingPortal:auditScreen.step1HelpDialog.content":
            "Puedes descargar o copiar el código de tu papeleta para verificar que refleja correctamente tus selecciones.",
        "votingPortal:auditScreen.step2Title": "2. Comprueba tu papeleta",
        "votingPortal:auditScreen.step2Description":
            "Haz clic en <VerifierLink>Comprueba el código de tu papeleta</VerifierLink>. Se abrirá en una nueva pestaña",
        "votingPortal:auditScreen.step2HelpDialog.content":
            "Para comprobar el código de tu papeleta, sigue los pasos de la guía. Incluye la descarga de una aplicación de escritorio para verificar tu papeleta de forma independiente al sitio web.",
        "votingPortal:auditScreen.bottomWarning":
            "Por motivos de seguridad, cuando audites tu papeleta, deberás invalidarla. Para continuar con el proceso de votación, haz clic en ‘<b>Iniciar votación</b>’.",
        "votingPortal:electionSelectionScreen.description":
            "Selecciona la papeleta en la que deseas votar",
        "votingPortal:electionSelectionScreen.chooserHelpDialog.content":
            "Esta pantalla muestra la lista de papeletas a las que puedes acceder. Pueden estar abiertas, programadas o cerradas. Solo puedes votar en las que están abiertas",
        "votingPortal:electionSelectionScreen.demoDialog.content":
            "Estás entrando en una cabina de votación de demostración. <strong>Tu voto no será registrado.</strong> Esta cabina es solo para demostración.",
        "votingPortal:electionSelectionScreen.errors.noVotingArea":
            "No estás registrado como votante en esta elección. Por favor, contacta con el soporte.",
        "votingPortal:electionSelectionScreen.errors.networkError":
            "Hubo un problema de red. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:electionSelectionScreen.errors.unableToFetchData":
            "Hubo un problema al obtener los datos. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:electionSelectionScreen.errors.noElectionEvent":
            "El evento electoral no existe. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:electionSelectionScreen.errors.ballotStylesEmlError":
            "Hubo un error con la publicación del estilo de la papeleta. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:electionSelectionScreen.errors.obtainingElectionFromID":
            "Hubo un error al obtener las elecciones asociadas con los siguientes IDs de elecciones: {{electionIds}}. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:electionSelectionScreen.alerts.noElections":
            "No hay elecciones en las que puedas votar. Esto podría deberse a que el área no tiene ninguna pregunta asociada. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:electionSelectionScreen.alerts.electionEventNotPublished":
            "El evento electoral aún no ha sido publicado. Por favor, inténtalo de nuevo más tarde o contacta con el soporte para obtener ayuda.",
        "votingPortal:electionSelectionScreen.materialsGate.instructions":
            "Debes leer <MaterialsLink>{{materialsTitle}}</MaterialsLink> antes de poder votar.",
        "votingPortal:errors.page.certAuthFailedMessage":
            "No se ha podido verificar tu certificado. Comprueba que estás usando un certificado de votante válido e inténtalo de nuevo.",
        "votingPortal:materials.mandatory.error":
            "Hubo un problema al registrar tu confirmación. Por favor, inténtalo de nuevo.",
        "votingPortal:ballotLocator.title": "Encuentra tu papeleta",
        "votingPortal:ballotLocator.titleResult": "Resultados de tu búsqueda de Papeleta",
        "votingPortal:ballotLocator.description":
            "Confirma que tu papeleta fue emitida correctamente",
        "votingPortal:ballotLocator.locate": "Encuentra tu papeleta",
        "votingPortal:ballotLocator.locateAgain": "Encuentra otra papeleta",
        "votingPortal:ballotLocator.found": "Tu ID de Papeleta {{ballotId}} ha sido encontrado",
        "votingPortal:ballotLocator.notFound": "Tu ID de Papeleta {{ballotId}} no fue encontrado",
        "votingPortal:ballotLocator.ambiguous":
            "Más de una de tus papeletas coincide con {{ballotId}}. Usa el ID de papeleta completo.",
        "votingPortal:ballotLocator.contentDesc": "Este es el contenido de tu papeleta: ",
        "votingPortal:ballotLocator.ballotIdNotFoundAtFilter":
            "No encontrado, comprueba que el ID de la papeleta sea correcto y pertenezca a este usuario.",
        "votingPortal:ballotLocator.steps.lookup": "Encuentra tu papeleta",
        "votingPortal:ballotLocator.titleHelpDialog.content":
            "El Buscador de Papeletas te permite introducir tu ID de Papeleta para localizar tu voto y confirmar que fue registrado correctamente.",
        "ballotVerifier:404.subtitle": "La página que buscas no existe",
        "ballotVerifier:homeScreen.step1": "Paso 1: Importa tu papeleta electoral.",
        "ballotVerifier:homeScreen.description1":
            "Para continuar, por favor importa los datos de las papeletas encriptadas proporcionados en el Portal de Votación:",
        "ballotVerifier:homeScreen.importBallotHelpDialog.title":
            "Información: Importa tu papeleta electoral",
        "ballotVerifier:homeScreen.importBallotHelpDialog.content":
            "Para continuar, por favor importa los datos de las papeletas encriptadas proporcionados en el Portal de Votación.",
        "ballotVerifier:homeScreen.step2": "Paso 2: Inserta tu ID de papeleta.",
        "ballotVerifier:homeScreen.description2":
            "Por favor ingresa el ID de la papeleta proporcionado en el Portal de Votación:",
        "ballotVerifier:homeScreen.ballotIdHelpDialog.title": "Información: Tu ID de papeleta",
        "ballotVerifier:homeScreen.ballotIdHelpDialog.content":
            "Por favor ingresa el ID de la papeleta proporcionado en el Portal de Votación.",
        "ballotVerifier:homeScreen.startButton": "Selecciona fichero",
        "ballotVerifier:homeScreen.dragDropOption": "O arrastra el fichero aquí",
        "ballotVerifier:homeScreen.importErrorDescription":
            "Hubo un problema al importar el voto auditable. ¿Elegiste el archivo correcto?",
        "ballotVerifier:homeScreen.useSampleLink": "Usa voto de ejemplo",
        "ballotVerifier:homeScreen.ballotIdPlaceholder": "Escribe aquí tu ID de papeleta",
        "ballotVerifier:confirmationScreen.verifySelectionsTitle":
            "Verifica tus selecciones en la papeleta",
        "ballotVerifier:confirmationScreen.verifySelectionsDescription":
            "Las siguientes selecciones de la papeleta han sido descodificadas de la papeleta que importaste. Por favor, revísalas y asegúrate de que coincidan con las selecciones que hiciste en el Portal de Votación. Si tus selecciones no coinciden, por favor, contacta con las autoridades electorales...",
        "ballotVerifier:confirmationScreen.verifySelectionsHelpDialog.title":
            "Información: Verifica tus selecciones en la papeleta",
        "ballotVerifier:confirmationScreen.verifySelectionsHelpDialog.content":
            "Las siguientes selecciones de la papeleta han sido descodificadas de la papeleta que importaste. Por favor, revísalas y asegúrate de que coincidan con las selecciones que hiciste en el Portal de Votación. Si tus selecciones no coinciden, por favor, contacta con las autoridades electorales...",
        "resultsPortal:logout.modal.title": "¿Seguro que quieres cerrar sesión?",
        "resultsPortal:logout.modal.content": "Estás a punto de cerrar esta aplicación.",
        "resultsPortal:header.session.title": "Tu sesión va a caducar.",
        "resultsPortal:header.session.timeLeft": "Te queda {{time}}.",
        "resultsPortal:resultsPortal.state.loadErrorMessage":
            "No hemos podido cargar los resultados ahora mismo. Inténtalo de nuevo en unos minutos.",
        "resultsPortal:resultsPortal.state.signInErrorMessage":
            "No hemos podido completar el inicio de sesión para los resultados ahora mismo. Inténtalo de nuevo en unos minutos.",
        "resultsPortal:resultsPortal.state.signInRequiredMessage":
            "Inicia sesión con tu cuenta de votante para ver estos resultados.",
        "resultsPortal:resultsPortal.state.notPublishedMessage":
            "Los resultados no están disponibles en este momento. Vuelve a comprobarlo más tarde.",
    },
}
