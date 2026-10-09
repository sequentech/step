// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ETranslationPresetTemplate, ITranslationPresetTemplate} from "@/types/translationPresets"

/**
 * Addresses voters as *tu* where the `cat` bundles use *vós*. Only the strings
 * whose wording depends on the register are listed.
 */
export const catalanInformalTemplate: ITranslationPresetTemplate = {
    id: ETranslationPresetTemplate.CATALAN_INFORMAL,
    language: "cat",
    overrides: {
        "global:homeScreen.startButton": "Selecciona fitxer",
        "global:homeScreen.dragDropOption": "O arrossega el fitxer aquí",
        "global:homeScreen.importErrorDescription":
            "Hi va haver un problema en importar el vot auditable. Vas triar el fitxer correcte?",
        "global:homeScreen.useSampleText": "No tens un vot verificable?",
        "global:homeScreen.useSampleLink": "Utilitza un vot verificable d'exemple",
        "global:confirmationScreen.bottomDescription1":
            "El teu vot va ser xifrat correctament. Ara pots tancar aquesta finestra i tornar a la Cabina de Votació.",
        "global:confirmationScreen.bottomDescription2":
            "Si no coincideixen, fes clic aquí per obtenir més informació sobre els possibles motius i les accions que pots prendre.",
        "global:confirmationScreen.ballotChoicesDescription": "I les teves seleccions de vot són:",
        "global:ballotSelectionsScreen.statusModal.content":
            "El panell d'estat et dóna informació sobre les verificacions realitzades.",
        "global:errors.encoding.writeInCharsExceeded_one":
            "Escurça l'escriptura lliure en {{count}} caràcter.",
        "global:errors.encoding.writeInCharsExceeded_many":
            "Escurça l'escriptura lliure en {{count}} caràcters.",
        "global:errors.encoding.writeInCharsExceeded_other":
            "Escurça l'escriptura lliure en {{count}} caràcters.",
        "global:errors.implicit.selectedMax_one": "Desmarca {{count}} candidat.",
        "global:errors.implicit.selectedMax_many": "Desmarca {{count}} candidats.",
        "global:errors.implicit.selectedMax_other": "Desmarca {{count}} candidats.",
        "global:errors.implicit.selectedMin_one": "Selecciona {{count}} candidat més.",
        "global:errors.implicit.selectedMin_many": "Selecciona {{count}} candidats més.",
        "global:errors.implicit.selectedMin_other": "Selecciona {{count}} candidats més.",
        "global:errors.implicit.maxSelectionsPerType_one":
            "Desmarca {{count}} candidat de {{type}}.",
        "global:errors.implicit.maxSelectionsPerType_many":
            "Desmarca {{count}} candidats de {{type}}.",
        "global:errors.implicit.maxSelectionsPerType_other":
            "Desmarca {{count}} candidats de {{type}}.",
        "global:errors.implicit.underVote_one": "Selecciona fins a {{count}} candidat més.",
        "global:errors.implicit.underVote_many": "Selecciona fins a {{count}} candidats més.",
        "global:errors.implicit.underVote_other": "Selecciona fins a {{count}} candidats més.",
        "global:errors.implicit.overVoteDisabled_one":
            "Has seleccionat el màxim de {{count}} candidat. Desmarca'l per triar-ne un altre.",
        "global:errors.implicit.overVoteDisabled_many":
            "Has seleccionat el màxim de {{count}} candidats. Desmarca'n un per triar-ne un altre.",
        "global:errors.implicit.overVoteDisabled_other":
            "Has seleccionat el màxim de {{count}} candidats. Desmarca'n un per triar-ne un altre.",
        "global:errors.implicit.blankVote": "No has seleccionat cap candidat.",
        "global:ballotHash": "El teu Localitzador de Vot: {{ballotId}}",
        "global:logout.buttonText": "Tanca la sessió",
        "global:logout.modal.title": "Estàs segur que vols tancar la sessió?",
        "global:logout.modal.content":
            "Estàs a punt de tancar aquesta aplicació. Aquesta acció no es pot desfer.",
        "global:logout.modal.close": "Tanca",
        "global:dragNDrop.firstLine": "Arrossega i deixa anar fitxers o",
        "global:dragNDrop.browse": "Carrega fitxer",
        "global:selectElection.voteButton": "Fes clic per votar",
        "global:selectElection.ballotLocator": "Localitza el teu vot",
        "global:header.session.title": "La teva sessió està a punt d'expirar.",
        "global:header.session.timeLeft": "Et queden {{time}} per emetre el teu vot.",
        "votingPortal:votingScreen.ballotHelpDialog.content":
            "Aquesta pantalla mostra les preguntes en les quals ets elegible per votar. Pots fer la teva selecció activant la casella a la dreta del Candidat/Resposta. Per restablir les teves seleccions, fes clic al botó “<b>Netejar seleccions</b>”, per passar al següent pas, fes clic al botó “<b>Següent</b>”.",
        "votingPortal:votingScreen.nonVotedDialog.title": "El teu vot és invàlid o en blanc",
        "votingPortal:votingScreen.nonVotedDialog.content":
            "Algunes de les teves respostes podrien fer que la papereta en una o més preguntes sigui invàlida o en blanc.",
        "votingPortal:votingScreen.warningDialog.title": "Revisa la teva papereta",
        "votingPortal:votingScreen.warningDialog.content":
            "La teva papereta conté seleccions que poden necessitar la teva atenció (com ara seleccionar menys opcions de les permeses). La teva papereta és vàlida i es comptarà tal com s'ha enviat.",
        "votingPortal:votingScreen.warningDialog.ok": "Torna i revisa",
        "votingPortal:votingScreen.warningDialog.continue": "Continua",
        "votingPortal:votingScreen.warningDialog.cancel": "Cancel·la",
        "votingPortal:votingScreen.blankBallotDialog.continue": "Continua",
        "votingPortal:votingScreen.blankBallotDialog.cancel": "Cancel·la",
        "votingPortal:startScreen.declineToVoteDialog.title": "Confirma que vols declinar votar",
        "votingPortal:startScreen.declineToVoteDialog.content":
            "Segur que vols declinar votar?<br />Aniràs directament a la revisió i el teu estat de participació es desarà com a <b>Ha declinat votar</b>.",
        "votingPortal:startScreen.instructionsDescription":
            "Segueix aquests passos per emetre el teu vot",
        "votingPortal:startScreen.step1Title": "1. Fes les teves seleccions",
        "votingPortal:startScreen.step1Description":
            "Escull els teus candidats preferits i respon cada pregunta de la papereta segons aparegui. Pots canviar les teves seleccions en qualsevol moment abans d'emetre el teu vot",
        "votingPortal:startScreen.step2Title": "2. Revisa les teves seleccions",
        "votingPortal:startScreen.step2Description":
            "Quan estiguis satisfet amb les teves seleccions, xifrem la teva papereta de forma segura i et mostrarem una revisió final. També rebràs un ID de seguiment únic com a referència",
        "votingPortal:startScreen.step3Title": "3. Emet la teva papereta",
        "votingPortal:startScreen.step3Description":
            "Quan estiguis a punt, emet la teva papereta perquè quedi registrada oficialment. O tria auditar primer per confirmar que va ser capturada i xifrada correctament",
        "votingPortal:reviewScreen.acclamation.description":
            "Revisa el que s'ha resolt per aclamació en aquesta elecció. No s'emetrà cap papereta.",
        "votingPortal:reviewScreen.title": "Revisa el teu vot",
        "votingPortal:reviewScreen.description":
            "Per fer canvis a les teves seleccions, fes clic al botó “<b>Edita el teu vot</b>”, per confirmar les teves seleccions, fes clic al botó “<b>Envia el vot</b>” a sota, i per auditar la teva papereta fes clic al botó “<b>Auditar papereta</b>” a sota.",
        "votingPortal:reviewScreen.descriptionNoAudit":
            "Per fer canvis a les teves seleccions, fes clic al botó “<b>Edita el teu vot</b>”, per confirmar les teves seleccions, fes clic al botó “<b>Envia el vot</b>” a sota.",
        "votingPortal:reviewScreen.backButton": "Edita el teu vot",
        "votingPortal:reviewScreen.castBallotButton": "Envia el vot",
        "votingPortal:reviewScreen.copyBallotId": "Copia l'ID de la papereta",
        "votingPortal:reviewScreen.reviewScreenHelpDialog.content":
            "Aquesta pantalla et permet revisar les teves seleccions abans d'emetre el vot",
        "votingPortal:reviewScreen.ballotIdHelpDialog.title": "El teu vot no ha estat emès",
        "votingPortal:reviewScreen.ballotIdHelpDialog.content":
            "<p>Aquest és el teu Localitzador del Vot, però <b>el teu vot encara no s'ha emès</b>. Si intentes buscar-lo ara, no apareixerà.</p><p>Mostrem el Localitzador del Vot en aquesta etapa perquè puguis auditar la papereta xifrada abans d'emetre-la.</p>",
        "votingPortal:reviewScreen.auditBallotHelpDialog.title": "Vols auditar la teva papereta?",
        "votingPortal:reviewScreen.auditBallotHelpDialog.content":
            "<p>Auditar la teva papereta l'invalidarà i hauràs de reiniciar el procés de votació. Continua només si et sents còmode amb els passos tècnics avançats. En cas contrari, fes clic a <u>Cancel·la</u> per tornar.</p>",
        "votingPortal:reviewScreen.confirmCastVoteDialog.title":
            "Estàs segur que vols emetre el teu vot?",
        "votingPortal:reviewScreen.confirmCastVoteDialog.content":
            "Un cop confirmis, el teu vot serà emès.",
        "votingPortal:reviewScreen.error.NETWORK_ERROR":
            "Hi ha hagut un problema de xarxa. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.UNABLE_TO_FETCH_DATA":
            "Hi ha hagut un problema en recuperar les dades. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.LOAD_ELECTION_EVENT":
            "No es pot carregar l'esdeveniment electoral. Si us plau, torna-ho a provar més tard.",
        "votingPortal:reviewScreen.error.CAST_VOTE":
            "Hi ha hagut un error desconegut en emetre el vot. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.CAST_VOTE_CheckStatusFailed":
            "L'elecció no permet emetre el vot. L'elecció pot estar tancada, arxivada o potser estàs intentant votar fora del període de gràcia.",
        "votingPortal:reviewScreen.error.CAST_VOTE_AreaNotFound":
            "Hi ha hagut un error en emetre el vot: àrea no trobada. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.CAST_VOTE_InternalServerError":
            "Hi ha hagut un error intern en emetre el vot. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.CAST_VOTE_QueueError":
            "Hi ha hagut un problema en processar el teu vot. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.CAST_VOTE_Unauthorized":
            "No estàs autoritzat per emetre un vot. Si us plau, contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.CAST_VOTE_ElectionEventNotFound":
            "No s'ha pogut trobar l'esdeveniment electoral. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.CAST_VOTE_ElectoralLogNotFound":
            "No s'ha pogut trobar el teu registre de vot. Si us plau, contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.CAST_VOTE_CheckPreviousVotesFailed":
            "Hi ha hagut un error en comprovar el teu estat de votació. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.CAST_VOTE_GetClientCredentialsFailed":
            "No s'han pogut verificar les teves credencials. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.CAST_VOTE_GetAreaIdFailed":
            "Hi ha hagut un error en verificar la teva àrea de votació. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.CAST_VOTE_GetTransactionFailed":
            "Hi ha hagut un error en processar el teu vot. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.CAST_VOTE_DeserializeBallotFailed":
            "Hi ha hagut un error en llegir la teva papereta. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.CAST_VOTE_DeserializeContestsFailed":
            "Hi ha hagut un error en llegir les teves seleccions. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.CAST_VOTE_PokValidationFailed":
            "No s'ha pogut validar el teu vot. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.CAST_VOTE_UuidParseFailed":
            "Hi ha hagut un error en processar la teva sol·licitud. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.CAST_VOTE_unexpected":
            "Hi ha hagut un error desconegut en emetre el vot. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.CAST_VOTE_timeout":
            "Error de temps d'espera en emetre el vot. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.CAST_VOTE_InsertFailedExceedsAllowedRevotes":
            "Has superat el límit de revots. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.CAST_VOTE_CheckRevotesFailed":
            "Has superat el nombre permès de revots. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.CAST_VOTE_CheckVotesInOtherAreasFailed":
            "Ja has votat en una altra àrea. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.CAST_VOTE_UnknownError":
            "Hi ha hagut un error desconegut en emetre el vot. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.NO_BALLOT_SELECTION":
            "No es troba l'estat de selecció per a aquesta elecció. Si us plau, assegura't d'haver seleccionat les teves opcions correctament o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.NO_BALLOT_STYLE":
            "L'estil de la papereta no està disponible. Si us plau, contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.NO_AUDITABLE_BALLOT":
            "No hi ha cap papereta auditable disponible. Si us plau, contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.INCONSISTENT_HASH":
            "Hi ha hagut un error relacionat amb el procés de hashing de la papereta. El BallotId: {{ballotId}} no és consistent amb el Hash de la Papereta Auditable: {{auditableBallotHash}}. Si us plau, informa d'aquest problema al servei d'assistència.",
        "votingPortal:reviewScreen.error.ELECTION_EVENT_NOT_OPEN":
            "L'esdeveniment electoral està tancat. Si us plau, contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.PARSE_ERROR":
            "Hi ha hagut un error en analitzar la papereta. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.DESERIALIZE_AUDITABLE_ERROR":
            "Hi ha hagut un error en deserialitzar la papereta auditable. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.DESERIALIZE_HASHABLE_ERROR":
            "Hi ha hagut un error en deserialitzar la papereta hashable. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.CONVERT_ERROR":
            "Hi ha hagut un error en convertir la papereta. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.SERIALIZE_ERROR":
            "Hi ha hagut un error en serialitzar la papereta. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.UNKNOWN_ERROR":
            "Hi ha hagut un error. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.REAUTH_FAILED":
            "L'autenticació ha fallat. Si us plau, torna-ho a provar o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.SESSION_EXPIRED":
            "La teva sessió ha caducat. Si us plau, torna a començar des del principi.",
        "votingPortal:reviewScreen.error.SESSION_STORAGE_ERROR":
            "L'emmagatzematge de sessió no està disponible. Si us plau, torna-ho a provar o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.PARSE_BALLOT_DATA_ERROR":
            "S'ha produït un error en analitzar les dades de la papereta. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.NOT_VALID_BALLOT_DATA_ERROR":
            "Les dades de la papereta no són vàlides. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.FETCH_DATA_TIMEOUT_ERROR":
            "Error de temps d'espera en obtenir les dades. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.TO_HASHABLE_BALLOT_ERROR":
            "Error en convertir a papereta hashable. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:reviewScreen.error.INTERNAL_ERROR":
            "S'ha produït un error intern en emetre el vot. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:confirmationScreen.title": "El teu vot ha estat emès",
        "votingPortal:confirmationScreen.description":
            "La teva papereta va ser emesa correctament. Utilitza el codi a continuació per verificar que va ser comptabilitzada",
        "votingPortal:confirmationScreen.remainingElectionsError":
            "No hem pogut comprovar si tens més eleccions en què votar. Torna-ho a provar.",
        "votingPortal:confirmationScreen.retryButton": "Torna-ho a provar",
        "votingPortal:confirmationScreen.verifyCastTitle": "Comprova que el teu vot va ser emès",
        "votingPortal:confirmationScreen.verifyCastDescription":
            "Pots verificar en qualsevol moment que la teva papereta va ser emesa correctament usant el codi QR a continuació",
        "votingPortal:confirmationScreen.confirmationHelpDialog.content":
            "Aquesta pantalla confirma que el teu vot va ser emès correctament. La informació aquí et permet verificar que la papereta va ser emmagatzemada a l'urna, tant durant el període de votació com després del seu tancament",
        "votingPortal:confirmationScreen.ballotIdHelpDialog.content":
            "El Localitzador del Vot és un codi únic que et permet trobar la teva papereta a l'urna. No conté informació sobre les teves seleccions.",
        "votingPortal:confirmationScreen.ballotIdDemoHelpDialog.content":
            "L'identificador de papereta de vot és un codi que et permet trobar la teva papereta a l'urna. Aquest identificador és únic i no conté informació sobre les teves seleccions.",
        "votingPortal:confirmationScreen.errorDialogPrintBallotReceipt.content":
            "Ha ocorregut un error. Si us plau, intenta-ho de nou.",
        "votingPortal:auditScreen.title": "Comprova la teva papereta",
        "votingPortal:auditScreen.description":
            "Per comprovar la teva papereta, segueix els passos a continuació:",
        "votingPortal:auditScreen.step1Title": "1. Desa les dades següents:",
        "votingPortal:auditScreen.step1Description":
            "el teu <b>Localitzador del Vot</b> a la part superior de la pantalla i la teva papereta encriptada a continuació",
        "votingPortal:auditScreen.step1HelpDialog.content":
            "Pots descarregar o copiar el codi de la teva papereta per verificar que reflecteix correctament les teves seleccions.",
        "votingPortal:auditScreen.step2Title": "2. Comprova la teva papereta",
        "votingPortal:auditScreen.step2Description":
            "Fes clic a <VerifierLink>Comprova el codi de la teva papereta</VerifierLink>. S'obrirà en una nova pestanya",
        "votingPortal:auditScreen.step2HelpDialog.content":
            "Per comprovar el codi de la teva papereta, segueix els passos de la guia. Inclou la descàrrega d'una aplicació d'escriptori per verificar la teva papereta de forma independent al lloc web.",
        "votingPortal:auditScreen.bottomWarning":
            "Per motius de seguretat, quan auditis la teva papereta, hauràs d'invalidar-la. Per continuar amb el procés de votació, fes clic a ‘<b>Iniciar votació</b>’.",
        "votingPortal:electionSelectionScreen.description":
            "Selecciona la papereta en la qual vols votar",
        "votingPortal:electionSelectionScreen.chooserHelpDialog.content":
            "Aquesta pantalla mostra la llista de paperetes a les quals pots accedir. Poden estar obertes, programades o tancades. Només pots votar en les que estan obertes",
        "votingPortal:electionSelectionScreen.demoDialog.content":
            "Estàs entrant en una cabina de votació de demostració. <strong>El teu vot no serà comptabilitzat.</strong> Aquesta cabina és només per a fins de demostració.",
        "votingPortal:electionSelectionScreen.errors.noVotingArea":
            "No estàs registrat com a votant en aquesta elecció. Si us plau, contacta amb el servei d'assistència.",
        "votingPortal:electionSelectionScreen.errors.networkError":
            "Hi ha hagut un problema de xarxa. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:electionSelectionScreen.errors.unableToFetchData":
            "Hi ha hagut un problema en obtenir les dades. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:electionSelectionScreen.errors.noElectionEvent":
            "L'esdeveniment electoral no existeix. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:electionSelectionScreen.errors.ballotStylesEmlError":
            "Hi ha hagut un error amb la publicació de l'estil de la papereta. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:electionSelectionScreen.errors.obtainingElectionFromID":
            "Hi ha hagut un error en obtenir les eleccions associades amb els següents IDs d'eleccions: {{electionIds}}. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:electionSelectionScreen.alerts.noElections":
            "No hi ha eleccions en les quals puguis votar. Això podria ser perquè l'àrea no té cap pregunta associada. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:electionSelectionScreen.alerts.electionEventNotPublished":
            "L'esdeveniment electoral encara no ha estat publicat. Si us plau, torna-ho a provar més tard o contacta amb el servei d'assistència.",
        "votingPortal:electionSelectionScreen.materialsGate.instructions":
            "Has de llegir <MaterialsLink>{{materialsTitle}}</MaterialsLink> abans de poder votar.",
        "votingPortal:errors.page.certAuthFailedMessage":
            "No s'ha pogut verificar el teu certificat. Comprova que estàs utilitzant un certificat de votant vàlid i torna-ho a provar.",
        "votingPortal:materials.mandatory.continueButton": "Continua",
        "votingPortal:materials.mandatory.error":
            "Hi ha hagut un problema en registrar la teva confirmació. Si us plau, torna-ho a intentar.",
        "votingPortal:ballotLocator.title": "Troba la teva papereta",
        "votingPortal:ballotLocator.titleResult": "Resultats de la cerca de la teva papereta",
        "votingPortal:ballotLocator.description":
            "Confirma que la teva papereta va ser emesa correctament",
        "votingPortal:ballotLocator.locate": "Troba la teva papereta",
        "votingPortal:ballotLocator.locateAgain": "Troba una altra papereta",
        "votingPortal:ballotLocator.found": "El teu ID de Papereta {{ballotId}} ha estat trobat",
        "votingPortal:ballotLocator.notFound":
            "El teu ID de Papereta {{ballotId}} no ha estat trobat",
        "votingPortal:ballotLocator.ambiguous":
            "Més d'una de les teves paperetes coincideix amb {{ballotId}}. Utilitza l'ID complet de la papereta.",
        "votingPortal:ballotLocator.contentDesc": "Aquest és el contingut de la teva papereta: ",
        "votingPortal:ballotLocator.ballotIdNotFoundAtFilter":
            "No trobat, comprova que l'ID de la Papereta sigui correcte i pertanyi a l'usuari actual.",
        "votingPortal:ballotLocator.steps.lookup": "Troba la teva papereta",
        "votingPortal:ballotLocator.titleHelpDialog.content":
            "El Cercador de Paperetes et permet introduir el teu ID de Papereta per localitzar el teu vot i confirmar que va ser registrat correctament.",
        "ballotVerifier:404.subtitle": "La pàgina que busques no existeix",
        "ballotVerifier:homeScreen.step1": "Pas 1: Importa la teva papereta electoral.",
        "ballotVerifier:homeScreen.description1":
            "Per continuar, si us plau importa les dades de les paperetes encriptades proporcionades al Portal de Votació:",
        "ballotVerifier:homeScreen.importBallotHelpDialog.title":
            "Informació: Importa la teva papereta electoral",
        "ballotVerifier:homeScreen.importBallotHelpDialog.content":
            "Per continuar, si us plau importa les dades de les paperetes encriptades proporcionades al Portal de Votació.",
        "ballotVerifier:homeScreen.step2": "Pas 2: Introdueix el teu ID de papereta.",
        "ballotVerifier:homeScreen.description2":
            "Si us plau, introdueix l'ID de la papereta proporcionat al Portal de Votació:",
        "ballotVerifier:homeScreen.ballotIdHelpDialog.title": "Informació: El teu ID de papereta",
        "ballotVerifier:homeScreen.ballotIdHelpDialog.content":
            "Si us plau, introdueix l'ID de la papereta proporcionat al Portal de Votació.",
        "ballotVerifier:homeScreen.startButton": "Selecciona fitxer",
        "ballotVerifier:homeScreen.dragDropOption": "O arrossega el fitxer aquí",
        "ballotVerifier:homeScreen.importErrorDescription":
            "Hi ha hagut un problema en importar el vot auditable. Has triat el fitxer correcte?",
        "ballotVerifier:homeScreen.useSampleLink": "Utilitza vot d'exemple",
        "ballotVerifier:homeScreen.ballotIdPlaceholder": "Escriu aquí el teu ID de papereta",
        "ballotVerifier:confirmationScreen.verifySelectionsTitle":
            "Verifica les teves seleccions a la papereta",
        "ballotVerifier:confirmationScreen.verifySelectionsDescription":
            "Les següents seleccions de la papereta han estat descodificades de la papereta que vas importar. Si us plau, revisa-les i assegura't que coincideixin amb les seleccions que vas fer al Portal de Votació. Si les teves seleccions no coincideixen, si us plau, contacta amb les autoritats electorals...",
        "ballotVerifier:confirmationScreen.verifySelectionsHelpDialog.title":
            "Informació: Verifica les teves seleccions a la papereta",
        "ballotVerifier:confirmationScreen.verifySelectionsHelpDialog.content":
            "Les següents seleccions de la papereta han estat descodificades de la papereta que vas importar. Si us plau, revisa-les i assegura't que coincideixin amb les seleccions que vas fer al Portal de Votació. Si les teves seleccions no coincideixen, si us plau, contacta amb les autoritats electorals...",
        "resultsPortal:logout.modal.title": "Segur que vols tancar la sessió?",
        "resultsPortal:logout.modal.content": "Estàs a punt de tancar aquesta aplicació.",
        "resultsPortal:header.session.title": "La teva sessió està a punt de caducar.",
        "resultsPortal:header.session.timeLeft": "Et queda {{time}}.",
        "resultsPortal:resultsPortal.state.loadErrorMessage":
            "No hem pogut carregar els resultats ara mateix. Torna-ho a provar d'aquí a uns minuts.",
        "resultsPortal:resultsPortal.state.signInErrorMessage":
            "No hem pogut completar l'inici de sessió per als resultats ara mateix. Torna-ho a provar d'aquí a uns minuts.",
        "resultsPortal:resultsPortal.state.signInRequiredMessage":
            "Inicia la sessió amb el teu compte de votant per veure aquests resultats.",
        "resultsPortal:resultsPortal.state.notPublishedMessage":
            "Els resultats no estan disponibles en aquest moment. Torna-ho a comprovar més tard.",
    },
}
