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
        a11y: {
            skipToContent: "Ir ao contido principal",
            helpAbout: "Axuda sobre {{topic}}",
            previewMaterial: "Vista previa de {{title}}",
            ballotsTable: "Papeletas",
            ballotLocatorTabs: "Seccións do localizador de papeletas",
            ballotIdLabel: "ID de voto",
            votingProgress: "Progreso da votación",
            stepOf: "Paso {{current}} de {{total}}",
            selectUpTo_one: "Seleccione ata {{count}} opción",
            selectUpTo_many: "Seleccione ata {{count}} opcións",
            selectUpTo_other: "Seleccione ata {{count}} opcións",
            selectExactly_one: "Seleccione {{count}} opción",
            selectExactly_many: "Seleccione {{count}} opcións",
            selectExactly_other: "Seleccione {{count}} opcións",
            selectBetween: "Seleccione entre {{min}} e {{max}} opcións",
        },
        candidatesList: {
            collapseToggle: "Alternar lista {{listTitle}}",
            showCandidates: "Mostrar candidatos",
            hideCandidates: "Ocultar candidatos",
            selectedCandidates_one: "{{count}} candidato seleccionado",
            selectedCandidates_many: "{{count}} candidatos seleccionados",
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
        contest: {
            acclamation: {
                description:
                    "Esta pregunta resolveuse por aclamación. As súas candidaturas resultan elixidas sen votación, polo que non se pode seleccionar ningunha opción nin se rexistra ningún voto.",
            },
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
            blankBallotDialog: {
                title: "Non seleccionou ningún candidato",
                content:
                    "Non fixo ningunha selección. A súa papeleta emitirase como papeleta en branco, o cal é unha elección válida e deliberada e contabilizarase como tal.",
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
            acclamation: {
                title: "Resolto por aclamación",
                helpDialog: {
                    title: "Información: Aclamación",
                    content:
                        "Esta pantalla mostra o que se resolveu por aclamación. Como non se puido seleccionar ningunha opción, non se emite ningunha papeleta e non hai nada que verificar despois.",
                    ok: "Aceptar",
                },
                description:
                    "Revisa o que se resolveu por aclamación nesta elección. Non se emitirá ningunha papeleta.",
                finishButton: "Rematar",
            },
            title: "Revisa a túa papeleta",
            description:
                "Fai clic en “<b>Editar papeleta</b>” para cambiar as túas seleccións, “<b>Emitir papeleta</b>” para confirmar, ou “<b>Comproba a papeleta</b>” para auditala.",
            descriptionNoAudit:
                "Fai clic en “<b>Editar papeleta</b>” para cambiar as túas seleccións, ou “<b>Emitir papeleta</b>” para confirmar.",
            backButton: "Editar papeleta",
            castBallotButton: "Emitir papeleta",
            auditButton: "Comproba a papeleta",
            copyBallotId: "Copiar o ID da papeleta",
            ballotIdCopied: "ID da papeleta copiado",
            ballotIdCopyError: "Non se puido copiar o ID da papeleta",
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
            confirmCastBlankBallotDialog: {
                title: "Está seguro de que quere emitir unha papeleta en branco?",
                content:
                    "Non seleccionou ningún candidato. Unha vez confirme, a súa papeleta emitirase en branco.",
                ok: "Si, quero emitir a miña papeleta en branco",
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
            blankBallot: "Papeleta en branco",
        },
        confirmationScreen: {
            acclamation: {
                title: "Resolto por aclamación",
                description:
                    "Todas as preguntas desta elección resolvéronse por aclamación, polo que non se emitiu ningunha papeleta e non hai identificador de papeleta que consultar.",
                helpDialog: {
                    title: "Información: Aclamación",
                    content:
                        "Todas as preguntas desta elección resolvéronse por aclamación: as súas candidaturas resultan elixidas sen votación. Como non se emitiu ningunha papeleta, non hai identificador de papeleta, comprobante nin código QR que verificar.",
                    ok: "Aceptar",
                },
            },
            title: "O teu voto foi emitido",
            description:
                "O código de confirmación abaixo verifica que <b>o teu voto foi emitido correctamente</b>. Podes usar este código para verificar que a túa papeleta foi contada.",
            blankBallot: {
                description:
                    "A súa papeleta emitiuse en branco, o cal é unha elección válida e deliberada.",
            },
            ballotId: "ID da Papeleta",
            printButton: "Imprimir",
            finishButton: "Rematar",
            remainingElectionsError:
                "Non puidemos comprobar se tes máis eleccións nas que votar. Téntao de novo.",
            retryButton: "Tentar de novo",
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
            materialsGate: {
                instructions:
                    "Debes ler <MaterialsLink>{{materialsTitle}}</MaterialsLink> antes de poder votar.",
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
                oopsWithStatus: "Vaia! {{status}}",
                oopsWithoutStatus: "Vaia! Erro inesperado",
                somethingWrong: "Algo saiu mal.",
                invalidLoginHintParametersTitle: "Ligazón de votación non válida",
                invalidLoginHintParametersMessage:
                    "Esta ligazón de votación contén información de acceso non válida. Solicite unha nova ligazón e ténteo de novo.",
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
                download: "Descargar",
            },
            mandatory: {
                checkboxLabel: "Lin os Materiais de apoio",
                continueButton: "Continuar",
                error: "Houbo un problema ao rexistrar a túa confirmación. Inténtao de novo.",
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
            contentDesc: "Este é o contido da túa Papeleta:",
            wrongFormatBallotId: "Formato incorrecto para o ID da Papeleta",
            ballotIdNotFoundAtFilter:
                "Non atopado, comprobe que o ID da Papeleta seja correcto e pertenezca a este usuario.",
            filterByBallotId: "Filtrar por ID da Papeleta",
            totalBallots: "Papeletas: {{total}}",
            idHelp: "O ID de papeleta curto, como 30ff d314, ou o completo",
            foundShort:
                "Atopada polo seu ID curto. Para estares seguro de que é a túa, compara o ID de papeleta completo co teu recibo.",
            matches: {
                title: "{{total}} papeletas teñen o ID curto {{shortId}}. Escolle a túa ou introduce o ID de papeleta completo.",
                cast: "Emitida o {{time}}",
            },
            result: {
                receipt: "Recibo da papeleta",
                idComment: "Usa o ID curto ou o completo para volver atopar esta papeleta.",
                fullId: "ID de papeleta completo",
                copy: "Copiar o ID de papeleta completo",
                copied: "ID de papeleta copiado",
                copiedLabel: "Copiado",
                recordEntry: "Entrada do rexistro",
                save: "Gardar o recibo",
                sealLabel: "Estado da papeleta: {{status}}",
                check: "Comprobalo de forma independente",
                caption:
                    "Comprobado polo teu navegador, con código que calquera pode inspeccionar.",
                captionVerified:
                    "Verificado polo teu navegador, con código que calquera pode inspeccionar.",
                captionProblem:
                    "Atopado polo teu navegador, con código que calquera pode inspeccionar.",
                late: "Normalmente significa que un nodo vai lento ou está sen conexión; non é proba de manipulación. Volve comprobalo máis tarde, aquí ou noutro dispositivo.",
                problem:
                    "O portal indica que esta papeleta foi emitida, pero o rexistro público da elección non ten ningunha entrada para ela. O teu navegador gardou a proba: a resposta do portal e o punto de control asinado do rexistro. Envíaa á autoridade electoral ou a un observador.",
                problemOther:
                    "As probas asinadas desta papeleta contradínse entre si. Pode ser un fallo ou un ataque. O teu navegador gardou a proba. Envíaa á autoridade electoral ou a un observador.",
                saveEvidence: "Gardar a proba",
                howToReport: "Como denuncialo",
                howToReportText:
                    "Garda o ficheiro da proba e envíao á autoridade electoral ou a un observador independente. Poden comprobalo nos seus propios ordenadores. Non contén nada sobre como votaches.",
                noReceipt:
                    "Este dispositivo non ten o recibo desta papeleta. Compróbaa no dispositivo co que votaches, ou comproba o ficheiro do recibo no verificador de papeletas.",
                receiptPending:
                    "Este dispositivo garda un recibo desta papeleta, pero non está completo ou non se puido comprobar agora. Téntao de novo máis tarde, ou comproba o ficheiro do recibo no verificador de papeletas.",
                status: {
                    checking: {
                        head: "Comprobando…",
                        sub: "O teu navegador está buscando a papeleta no rexistro público.",
                    },
                    recorded: {
                        head: "No rexistro público",
                        sub: "O teu navegador atopou a papeleta. Comprobando as súas testemuñas…",
                    },
                    witnessed: {
                        head: "Confirmada por testemuñas",
                        sub: "Comprobando os ancoraxes públicos…",
                        subAnchored_one:
                            "Ancorada en nodos públicos. Onde comproban os navegadores chegará en {{count}} minuto.",
                        subAnchored_other:
                            "Ancorada en nodos públicos. Onde comproban os navegadores chegará en {{count}} minutos.",
                    },
                    verified: {
                        head: "Verificada",
                        sub: "No rexistro público, confirmada por testemuñas independentes e ancorada publicamente.",
                    },
                    late: {
                        head: "Tarda máis do esperado",
                        sub: "No rexistro público e confirmada por testemuñas, pero os seus ancoraxes públicos van tarde.",
                    },
                    problem: {
                        head: "Non está no rexistro público",
                        sub: "O teu navegador non puido atopar esta papeleta no rexistro público da elección.",
                    },
                    otherProblem: {
                        head: "Fallou unha comprobación",
                        sub: "As probas asinadas desta papeleta contradínse entre si. Mira o paso que fallou.",
                    },
                },
                steps: {
                    record: {
                        title: "Engadida ao rexistro público",
                        done: "Entrada n.º {{index}} do rexistro de papeletas da elección. O teu navegador comprobou a súa proba.",
                        failed: "Ningunha entrada do rexistro de papeletas da elección ten este ID de papeleta.",
                        missing: "Falta",
                    },
                    witnesses: {
                        title: "Confirmada por testemuñas",
                        done: "Testemuñas independentes asinaron un rexistro que a inclúe.",
                        count: "{{signed}} de {{total}} · fan falta {{needed}}",
                        countOf: "{{signed}} de {{total}}",
                    },
                    hubs: {
                        title_one: "Ancorada en {{count}} nodo público",
                        title_other: "Ancorada en {{count}} nodos públicos",
                        titleAny: "Ancorada en nodos públicos",
                        done: "Rexistros compartidos tamén gardan esta entrada: {{hubs}}.",
                    },
                    browser: {
                        title: "Ancorada onde comproban os navegadores",
                        done: "Gardada por un nodo provisional ata que os navegadores comproben os rexistros por si mesmos.",
                    },
                    checking: "O teu navegador está comprobando…",
                    scheduled_one: "En {{count}} minuto. Non fai falta esperar.",
                    scheduled_other: "En {{count}} minutos. Non fai falta esperar.",
                    by: "Antes de {{time}}",
                    late: "Tarde",
                    lateHubs: "Os nodos deberían telo ancorado antes de {{time}}.",
                    lateOther: "Isto debería terse feito antes de {{time}}.",
                    waiting: "Agardando o paso anterior.",
                    skipped: "Sen comprobar.",
                    failedStep: "As probas asinadas deste paso contradínse entre si.",
                    failedTime: "Problema",
                },
            },
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
                ballot_id: "ID da papeleta",
            },
        },
        vitrine: {
            ballotId: {
                label: "ID da Papeleta",
                full: "ID da Papeleta completo",
                yours: "O teu ID de Papeleta:",
                copy: "Copiar o ID completo",
                copied: "ID copiado",
                copiedShort: "Copiado",
            },
            status: {
                title: "Estado da papeleta",
                cast: "Emitida o {{when}}",
                ballotCast: "Voto emitido",
                aria: "Estado da papeleta: {{state}}. Mostrar detalles",
                ariaCast: "Voto emitido. Estado da papeleta: {{state}}. Mostrar detalles",
                tooltip: "{{state}} · Mostrar detalles",
                tracker: "Abrir no rastrexador de papeletas",
                save: "Gardar o xustificante",
                ok: "Aceptar",
                state: {
                    notcast: "Aínda sen emitir",
                    checking: "Comprobando…",
                    recorded: "No rexistro público",
                    witnessed: "Confirmada por testemuñas",
                    verified: "Verificada",
                    late: "Con atraso",
                    problem: "Problema",
                },
                head: {
                    late: "Tarda máis do previsto",
                    mismatch: "O rexistro non coincide",
                    damaged: "O xustificante está danado",
                    noReceipt: "Sen xustificante da túa papeleta",
                    unchecked: "Non se puido comprobar a túa papeleta",
                },
                sub: {
                    notcast:
                        "Esta papeleta está cifrada neste dispositivo e non se enviou nada. Cando a emitas, o teu navegador comprobará que chega ao rexistro público.",
                    checking:
                        "O teu navegador está a comprobar que a túa papeleta está no rexistro público.",
                    recorded:
                        "O teu navegador atopou a túa papeleta no rexistro público. Agora confírmana as testemuñas.",
                    witnessed:
                        "Testemuñas independentes confirmaron o rexistro. As ancoraxes públicas chegan en {{minutes}} minutos; non hai que agardar.",
                    verified:
                        "A túa papeleta está no rexistro público, confirmada por testemuñas independentes e ancorada publicamente.",
                    late: "A túa papeleta está no rexistro público e confirmada polas testemuñas, pero as súas ancoraxes públicas van con atraso.",
                    lateStep:
                        "Un paso tarda máis do previsto. Non é proba de manipulación, e o teu navegador segue comprobando.",
                    mismatch:
                        "A entrada n.º {{index}} do rexistro público non é a deste xustificante.",
                    mismatchNoEntry: "A entrada do rexistro público non é a deste xustificante.",
                    damaged:
                        "Non se puido ler ou verificar unha parte do xustificante ou do rexistro. É un fallo de datos, non unha proba de manipulación. O teu voto emitiuse.",
                    noReceipt:
                        "Este sitio web non recibiu ningún xustificante da túa papeleta. O teu voto emitiuse e avisouse aos observadores da elección.",
                    unchecked:
                        "Este sitio web non puido comprobar o xustificante. O teu voto emitiuse.",
                },
                step: {
                    record: {
                        title: "No rexistro público",
                        upcoming:
                            "O teu navegador atopa a papeleta no rexistro público da elección.",
                    },
                    witnesses: {
                        title: "Confirmada por testemuñas",
                        upcoming: "Testemuñas independentes asinan un rexistro que a inclúe.",
                    },
                    anchors: {
                        title: "Ancorada publicamente",
                        upcoming:
                            "Os nodos públicos, e os rexistros que comproban os navegadores, gardan ese rexistro.",
                    },
                },
                row: {
                    waiting: "Agardando polo paso anterior.",
                    checking: "O teu navegador está a comprobar…",
                    anchoring:
                        "Nodos públicos e, despois, onde comproban os navegadores: en {{minutes}} minutos.",
                    recorded: "O teu navegador comprobou a súa proba.",
                    recordedEntry: "Entrada n.º {{index}}. O teu navegador comprobou a súa proba.",
                    witnessed:
                        "Un número suficiente das testemuñas independentes da política ({{count}}) asinou un rexistro que a inclúe.",
                    anchoredWaict:
                        "Nos nodos públicos ({{count}}) e onde comproban os navegadores.",
                    anchoredBootstrap:
                        "Nos nodos públicos ({{count}}) e onde comproban os navegadores, por agora mediante un nodo substituto.",
                    anchoredHubs: "Nos nodos públicos ({{count}}).",
                    lateNow: "Xa debería estar feito.",
                    lateStep: "Debería estar feito ás {{time}}.",
                    lateAnchors: "Os nodos deberían telo ancorado ás {{time}}.",
                    mismatch: "A entrada n.º {{index}} do rexistro é distinta do teu xustificante.",
                    mismatchNoEntry: "Unha entrada do rexistro é distinta do teu xustificante.",
                    contradiction: "As súas probas asinadas contradicíense.",
                    damaged: "A súa proba está danada e non se puido verificar.",
                    noReceipt: "Este sitio web non recibiu ningún xustificante.",
                    unchecked: "Este sitio web non puido comprobar o xustificante.",
                    skipped: "Non comprobado, porque non se puido completar o paso anterior.",
                    skippedMismatch: "Non comprobado, porque o rexistro non coincide.",
                },
                word: {
                    withinSeconds: "En poucos segundos",
                    withinMinutes: "En {{minutes}} min",
                    checking: "Comprobando…",
                    late: "Con atraso",
                    mismatch: "Non coincide",
                    damaged: "Danado",
                    noReceipt: "Sen xustificante",
                    unchecked: "Non comprobado",
                    skipped: "—",
                },
                callout: {
                    late: "Adoita significar que un nodo vai lento ou está fóra de liña; non é proba de manipulación.",
                    lateKept:
                        "Este dispositivo volve comprobalo cada vez que regresas, e o rastrexador de papeletas pode comprobalo desde calquera dispositivo.",
                    lateTracker:
                        "O rastrexador de papeletas pode comprobalo desde calquera dispositivo.",
                    evidence:
                        "O teu navegador gardou a evidencia: este xustificante e a entrada asinada do rexistro. Envíaa á autoridade electoral ou a un observador; calquera pode comprobala.",
                    saveEvidence: "Gardar a evidencia",
                },
                caption: {
                    notcast:
                        "Non se comproba nada ata que a emitas. Despois, esta marca mostra ata onde se verificou a túa papeleta.",
                    checking:
                        "O teu navegador está a comprobar o rexistro público, con código que calquera pode inspeccionar.",
                    problem:
                        "Detectado polo teu navegador, con código que calquera pode inspeccionar.",
                    verified:
                        "Verificado polo teu navegador, con código que calquera pode inspeccionar.",
                    checked:
                        "Comprobado polo teu navegador, con código que calquera pode inspeccionar.",
                    kept: "Este dispositivo garda o xustificante e volve comprobalo en cada visita.",
                },
            },
            booth: {
                title: "Aínda non se pode preparar a súa papeleta",
                explanation:
                    "Antes de cifrar a súa papeleta, este sitio web comproba que a configuración da elección é a publicada no rexistro público da elección. A súa papeleta non se cifrou nin se enviou, e as súas opcións mantéñense.",
                pending:
                    "A configuración publicada aínda non está dispoñible. Ténteo de novo nun momento.",
                invalid: "Non se puido ler a configuración publicada. Ténteo de novo nun momento.",
                failed: "A configuración non coincide coa publicada. Avisouse aos observadores da elección. Póñase en contacto co soporte.",
                retry: "Tentar de novo",
                close: "Pechar",
            },
        },
    },
}

export default galegoTranslation
