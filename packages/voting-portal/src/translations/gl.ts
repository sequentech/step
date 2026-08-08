// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//

import {TranslationType} from "./en"

// SPDX-License-Identifier: AGPL-3.0-only
const galegoTranslation: TranslationType = {
    translations: {
        common: {
            goBack: "Volver",
            showMore: "Mostrar Máis",
            showLess: "Mostrar Menos",
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
            electionList: "Papeletas",
            ballot: "Papeleta",
            review: "Revisión",
            confirmation: "Confirmar",
            audit: "Auditoría",
        },
        footer: {
            poweredBy: "Desenvolvido por <1></1>",
        },
        votingScreen: {
            backButton: "Volver",
            reviewButton: "Seguinte",
            clearButton: "Limpar selección",
            ballotHelpDialog: {
                title: "Sobre a pantalla da papeleta",
                content:
                    "Esta pantalla mostra a pregunta na que es elixible para votar. Activa a caixa á dereita para seleccionar un Candidato/Resposta. Para restablecer, fai clic en “<b>Limpar selección</b>”; para continuar, fai clic en “<b>Seguinte</b>”.",
                ok: "Aceptar",
            },
            nonVotedDialog: {
                title: "Voto inválido ou en branco",
                content:
                    "Algunhas das túas respostas farán que a papeleta sexa inválida ou quede en branco en unha ou máis preguntas.",
                ok: "Volver e revisar",
                continue: "Continuar",
                cancel: "Cancelar",
            },
            warningDialog: {
                title: "Revisa a túa papeleta",
                content:
                    "A túa papeleta contén seleccións que poden necesitar a túa atención (como seleccionar menos opcións das permitidas). A túa papeleta é válida e contarase tal como se enviou.",
                ok: "Voltar e revisar",
                continue: "Continuar",
                cancel: "Cancelar",
            },
        },
        startScreen: {
            startButton: "Comezar a votar",
            declineToVoteButton: "Absterse de votar",
            declineToVoteDialog: {
                title: "Confirmar abstención de voto",
                content:
                    "Seguro que queres absterte de votar?<br />Irás directamente á revisión e o teu estado de participación gardarase como <b>Abstívose de votar</b>.",
                continue: "Absterse de votar",
                cancel: "Cancelar",
            },
            instructionsTitle: "Como votar",
            instructionsDescription: "Siga estes pasos para votar:",
            step1Title: "1. Selecciona as túas opcións",
            step1Description:
                "Elixe os teus candidatos e responde a cada pregunta. Podes editar a túa papeleta ata estar listo.",
            step2Title: "2. Revisa a túa papeleta",
            step2Description:
                "Encriptaremos a túa papeleta e mostrarémosche unha revisión final. Recibirás un ID de seguimento único.",
            step3Title: "3. Emite a túa papeleta",
            step3Description:
                "Emite a túa papeleta para que quede rexistrada, ou auditala para confirmar que foi encriptada correctamente.",
        },
        reviewScreen: {
            title: "Revisa a túa papeleta",
            description:
                "Fai clic en “<b>Editar papeleta</b>” para cambiar as túas seleccións, “<b>Emitir papeleta</b>” para confirmar, ou “<b>Comproba a papeleta</b>” para auditala.",
            descriptionNoAudit:
                "Fai clic en “<b>Editar papeleta</b>” para cambiar as túas seleccións, ou “<b>Emitir papeleta</b>” para confirmar.",
            backButton: "Editar papeleta",
            castBallotButton: "Emitir papeleta",
            auditButton: "Comproba a papeleta",
            reviewScreenHelpDialog: {
                title: "Sobre a pantalla de revisión",
                content:
                    "Esta pantalla permíteche revisar as túas seleccións antes de emitir a papeleta.",
                ok: "Aceptar",
            },
            ballotIdHelpDialog: {
                title: "O voto non foi emitido",
                content:
                    "<p>Este é o teu ID de seguimento da papeleta, pero <b>o teu voto aínda non foi emitido</b>. Se tentas rastrexar a papeleta, non a atoparás.</p><p>A razón pola que mostramos o ID de seguimento nesta fase é permitirche auditar a corrección da papeleta encriptada antes de emitila.</p>",
                ok: "Entendo que o meu voto non foi emitido",
                cancel: "Cancelar",
            },
            auditBallotHelpDialog: {
                title: "Queres auditar a papeleta?",
                content:
                    "<p>Ten en conta que auditar a túa papeleta anularaa, polo que terás que reiniciar o proceso de votación. O proceso de auditoría permíteche verificar que a túa papeleta está correctamente codificada, pero implica pasos técnicos avanzados. Recomendamos continuar só se estás seguro das túas habilidades técnicas. Se só queres emitir a túa papeleta, fai clic en <u>Cancelar</u> para volver á pantalla de revisión.</p>",
                ok: "Si, quero descartar a miña papeleta para auditala",
                cancel: "Cancelar",
            },
            confirmCastVoteDialog: {
                title: "Estás seguro de que queres emitir o teu voto?",
                content: "Tras confirmar, o teu voto será emitido.",
                ok: "Si, quero emitir o meu voto",
                cancel: "Cancelar",
            },
            error: {
                NETWORK_ERROR:
                    "Houbo un problema de rede. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                UNABLE_TO_FETCH_DATA:
                    "Houbo un problema ao obter os datos. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                LOAD_ELECTION_EVENT:
                    "Non se pode cargar o evento de eleccións. Inténtao de novo máis tarde.",
                CAST_VOTE:
                    "Houbo un erro descoñecido ao emitir o voto. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_AreaNotFound:
                    "Houbo un erro ao emitir o voto: área non atopada. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_CheckStatusFailed:
                    "A elección non permite emitir o voto. A elección pode estar pechada, arquivada ou podes estar intentando votar fóra do período de gracia.",
                CAST_VOTE_InternalServerError:
                    "Produciuse un erro interno ao emitir o voto. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_QueueError:
                    "Houbo un problema ao procesar o teu voto. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_Unauthorized:
                    "Non estás autorizado para emitir un voto. Contacta co soporte para obter axuda.",
                CAST_VOTE_ElectionEventNotFound:
                    "Non se puido atopar o evento de eleccións. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_ElectoralLogNotFound:
                    "Non se puido atopar o rexistro de votación. Contacta co soporte para obter axuda.",
                CAST_VOTE_CheckPreviousVotesFailed:
                    "Produciuse un erro ao comprobar o estado do teu voto. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_GetClientCredentialsFailed:
                    "Non se puideron verificar as túas credenciais. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_GetAreaIdFailed:
                    "Produciuse un erro ao verificar a túa área de votación. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_GetTransactionFailed:
                    "Produciuse un erro ao procesar o teu voto. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_DeserializeBallotFailed:
                    "Produciuse un erro ao ler a túa papeleta. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_DeserializeContestsFailed:
                    "Produciuse un erro ao ler as túas seleccións. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_PokValidationFailed:
                    "Non se puido validar o teu voto. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_UuidParseFailed:
                    "Produciuse un erro ao procesar a túa solicitude. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_unexpected:
                    "Produciuse un erro descoñecido ao emitir o voto. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CAST_VOTE_timeout:
                    "Erro de tempo de espera ao emitir o voto. Por favor, inténteo de novo máis tarde ou contacte co soporte para obter axuda.",
                CAST_VOTE_InsertFailedExceedsAllowedRevotes:
                    "Superou o límite de renovacións de voto. Por favor, inténteo de novo máis tarde ou contacte co soporte para obter axuda.",
                CAST_VOTE_CheckRevotesFailed:
                    "Superou o número permitido de renovacións de voto. Por favor, inténteo de novo máis tarde ou contacte co soporte para obter axuda.",
                CAST_VOTE_CheckVotesInOtherAreasFailed:
                    "Xa votou noutra área. Por favor, inténteo de novo máis tarde ou contacte co soporte para obter axuda.",
                CAST_VOTE_UnknownError:
                    "Produciuse un erro descoñecido ao emitir o voto. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                NO_BALLOT_SELECTION:
                    "O estado de selección para esta elección non está presente. Asegúrate de que seleccionaches as túas eleccións correctamente ou contacta co soporte.",
                NO_BALLOT_STYLE: "O estilo da papeleta non está dispoñible. Contacta co soporte.",
                NO_AUDITABLE_BALLOT: "Non hai papeleta auditable dispoñible. Contacta co soporte.",
                INCONSISTENT_HASH:
                    "Houbo un erro relacionado co proceso de hash da papeleta. O ID da Papeleta: {{ballotId}} non é consistente co hash da Papeleta Auditable: {{auditableBallotHash}}. Informa deste problema ao soporte.",
                ELECTION_EVENT_NOT_OPEN: "O evento de eleccións está pechado. Contacta co soporte.",
                PARSE_ERROR:
                    "Houbo un erro ao analizar a papeleta. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                DESERIALIZE_AUDITABLE_ERROR:
                    "Houbo un erro ao deserializar a papeleta auditable. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                DESERIALIZE_HASHABLE_ERROR:
                    "Houbo un erro ao deserializar a papeleta hashable. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                CONVERT_ERROR:
                    "Houbo un erro ao converter a papeleta. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                SERIALIZE_ERROR:
                    "Houbo un erro ao serializar a papeleta. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                UNKNOWN_ERROR:
                    "Houbo un erro. Inténtao de novo máis tarde ou contacta co soporte para obter axuda.",
                REAUTH_FAILED:
                    "A autenticación fallou. Inténtao de novo ou contacta co soporte para obter axuda.",
                SESSION_EXPIRED:
                    "A túa sesión expirou. Por favor, comeza de novo dende o principio.",
                CAST_VOTE_BallotIdMismatch:
                    "O identificador da papeleta non coincide co do voto emitido.",
                SESSION_STORAGE_ERROR:
                    "O almacenamento de sesión non está dispoñible. Por favor, inténteo de novo ou contacte co soporte.",
                PARSE_BALLOT_DATA_ERROR:
                    "Houbo un erro ao analizar os datos da papeleta. Por favor, inténteo de novo máis tarde ou contacte co soporte para obter axuda.",
                NOT_VALID_BALLOT_DATA_ERROR:
                    "Os datos da papeleta non son válidos. Por favor, inténteo de novo máis tarde ou contacte co soporte para obter axuda.",
                FETCH_DATA_TIMEOUT_ERROR:
                    "Erro de tempo de espera ao obter os datos. Por favor, inténteo de novo máis tarde ou contacte co soporte para obter axuda.",
                TO_HASHABLE_BALLOT_ERROR:
                    "Erro ao converter a papeleta hashable. Por favor, inténteo de novo máis tarde ou contacte co soporte para obter axuda.",
                INTERNAL_ERROR:
                    "Houbo un erro interno ao emitir o voto. Por favor, inténteo de novo máis tarde ou contacte co soporte para obter axuda.",
            },
            declineToVote: "Absterse de votar",
        },
        confirmationScreen: {
            title: "O teu voto foi emitido",
            description:
                "O código de confirmación abaixo verifica que <b>o teu voto foi emitido correctamente</b>. Podes usar este código para verificar que a túa papeleta foi contada.",
            ballotId: "ID da Papeleta",
            printButton: "Imprimir",
            finishButton: "Rematar",
            verifyCastTitle: "Verifica que o teu voto foi emitido",
            verifyCastDescription:
                "Podes verificar en calquera momento que o teu voto foi emitido correctamente usando o seguinte código QR:",
            confirmationHelpDialog: {
                title: "Sobre a pantalla de confirmación",
                content:
                    "Esta pantalla amosa que o teu voto foi emitido correctamente. Podes verificar que a papeleta foi almacenada na urna.",
                ok: "Aceptar",
            },
            demoPrintDialog: {
                title: "Imprimindo papeleta",
                content: "Impresión desactivada en modo de demostración",
                ok: "Aceptar",
            },
            demoBallotUrlDialog: {
                title: "ID da Papeleta",
                content: "Non se pode usar o código, desactivado en modo de demostración.",
                ok: "Aceptar",
            },
            ballotIdHelpDialog: {
                title: "Sobre o ID da Papeleta",
                content:
                    "O ID da Papeleta é un código que permite atopar a túa papeleta na urna, este ID é único e non contén información sobre as túas seleccións.",
                ok: "Aceptar",
            },
            ballotIdDemoHelpDialog: {
                title: "Sobre o ID da Papeleta",
                content:
                    "O ID da Papeleta é un código que permite atopar a túa papeleta na urna, este ID é único e non contén información sobre as túas seleccións.",
                ok: "Aceptar",
            },
            errorDialogPrintBallotReceipt: {
                title: "Erro",
                content: "Produciuse un erro, por favor inténteo de novo",
                ok: "OK",
            },
            demoQRText: "O rastrexador de papeletas está desactivado en modo de demostración",
        },
        auditScreen: {
            printButton: "Imprimir",
            restartButton: "Comezar a votar",
            title: "Comproba a túa Papeleta",
            description: "Segue os pasos abaixo para comprobar a túa papeleta:",
            step1Title: "1. Descarga ou copia a seguinte información",
            step1Description:
                "O teu <b>ID da Papeleta</b> aparece na parte superior da pantalla e a túa papeleta encriptada abaixo:",
            step1HelpDialog: {
                title: "Copia a Papeleta Encriptada",
                content:
                    "Podes descargar ou copiar a túa papeleta encriptada para auditala e verificar que o contido encriptado contén as túas seleccións.",
                ok: "Aceptar",
            },
            downloadButton: "Descargar",
            step2Title: "2. Verifica a túa papeleta",
            step2Description:
                "<VerifierLink>Accede ao verificador de papeletas</VerifierLink>, abrirase unha nova pestana no teu navegador.",
            step2HelpDialog: {
                title: "Tutorial de auditoría da papeleta",
                content:
                    "Para auditar a túa papeleta, necesitarás seguir os pasos mostrados no tutorial, o que inclúe a descarga dunha aplicación de escritorio usada para verificar a papeleta encriptada de forma independente do sitio web.",
                ok: "Aceptar",
            },
            bottomWarning:
                "Por razóns de seguridade, ao auditar a túa papeleta, esta debe ser anulada. Para continuar co proceso de votación, necesitas facer clic en ‘<b>Comezar a votar</b>’ abaixo.",
        },
        electionSelectionScreen: {
            title: "Papeletas",
            description: "Selecciona a papeleta na que queres votar",
            chooserHelpDialog: {
                title: "Sobre a lista de papeletas",
                content:
                    "Esta pantalla amosa a lista de papeletas nas que podes votar. Só poderás acceder á papeleta se o período de votación está aberto.",
                ok: "OK",
            },
            noResults: "Sen papeletas por agora.",
            resultsButton: "Ver resultados",
            demoDialog: {
                title: "Cabina de votación de demostración",
                content:
                    "Estás entrando nunha cabina de votación de demostración. <strong>O teu voto non será emitido.</strong> Só para fins de demostración.",
                ok: "Entendo que o meu voto non será emitido",
            },
            errors: {
                noVotingArea:
                    "Non estás rexistrado como votante nesta elección. Por favor, contacta co soporte.",
                networkError:
                    "Houbo un problema de rede. Inténteo de novo máis tarde ou contacte co soporte para obter asistencia.",
                unableToFetchData:
                    "Houbo un problema ao recuperar os datos. Inténteo de novo máis tarde ou contacte co soporte para obter asistencia.",
                noElectionEvent:
                    "O evento electoral non existe. Inténteo de novo máis tarde ou contacte co soporte para obter asistencia.",
                ballotStylesEmlError:
                    "Houbo un erro co estilo de papeleta publicado. Inténteo de novo máis tarde ou contacte co soporte para obter asistencia.",
                obtainingElectionFromID:
                    "Houbo un erro ao obter as eleccións asociadas cos seguintes IDs de elección: {{electionIds}}. Inténteo de novo máis tarde ou contacte co soporte para obter asistencia.",
            },
            alerts: {
                noElections:
                    "Non hai eleccións nas que poidas votar. Isto pode deberse a que a área non ten ningunha pregunta asociada. Inténteo de novo máis tarde ou contacte co soporte para obter asistencia.",
                electionEventNotPublished:
                    "O evento electoral aínda non foi publicado. Inténteo de novo máis tarde ou contacte co soporte para obter asistencia.",
            },
        },
        errors: {
            encoding: {
                notEnoughChoices: "Non hai suficientes opcións para decodificar",
                writeInChoiceOutOfRange: "Opción de escritura libre fóra de rango: {{index}}",
                writeInNotEndInZero: "A escritura libre non remata en 0",
                bytesToUtf8Conversion:
                    "Erro ao converter a escritura libre de bytes a cadea UTF-8: {{errorMessage}}",
                ballotTooLarge: "A papeleta é máis grande do esperado",
            },
            explicit: {
                notAllowed:
                    "A papeleta está marcada como explicitamente inválida, pero a pregunta non o permite",
                alert: "Esta selección contarase como un voto inválido",
            },
            page: {
                oopsWithStatus: "¡Vaia! {{status}}",
                oopsWithoutStatus: "Oops! Erro inesperado",
                somethingWrong: "Algo saiu mal.",
                certAuthFailedTitle: "Erro de Autenticación con Certificado",
                certAuthFailedMessage:
                    "Non foi posible verificar o seu certificado. Comprobe que está a usar un certificado de votante válido e ténteo de novo.",
            },
        },
        materials: {
            common: {
                label: "Materiais de apoio",
                back: "Voltar á lista de papeletas",
                close: "Pechar",
                preview: "Previsualizar",
            },
        },
        ballotLocator: {
            title: "Busca a túa Papeleta",
            titleResult: "Resultado da túa busca de Papeleta",
            description: "Verifica que a túa Papeleta foi correctamente enviada",
            locate: "Busca a túa Papeleta",
            locateAgain: "Buscar outra Papeleta",
            found: "O teu ID de papeleta {{ballotId}} foi atopado",
            notFound: "O teu ID de papeleta {{ballotId}} non foi atopado",
            ambiguous:
                "Máis dunha das túas papeletas coincide con {{ballotId}}. Usa o ID completo da papeleta.",
            contentDesc: "Este é o contido da túa Papeleta:",
            wrongFormatBallotId: "Formato incorrecto para o ID da Papeleta",
            ballotIdNotFoundAtFilter:
                "Non atopado, comprobe que o ID da Papeleta seja correcto e pertenezca a este usuario.",
            filterByBallotId: "Filtrar por ID da Papeleta",
            totalBallots: "Papeletas: {{total}}",
            steps: {
                lookup: "Busca a túa Papeleta",
                result: "Resultado",
            },
            titleHelpDialog: {
                title: "Sobre o localizador de papeleta",
                content:
                    "O localizador de papeletas permíteche introducir o ID da papeleta para atopar o teu voto e confirmar que foi rexistrado correctamente.",
                ok: "Aceptar",
            },
            tabs: {
                logs: "Logs",
                ballotLocator: "Busca de Papeleta",
            },
            column: {
                statement_kind: "Tipo",
                statement_timestamp: "Marca de tempo",
                username: "Nome de usuario",
                ballot_id: "ID da papeleta",
                message: "Mensaxe",
            },
        },
    },
}

export default galegoTranslation
