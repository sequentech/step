// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {TranslationType} from "./en"

const galegoTranslation: TranslationType = {
    translations: {
        language: "Galego",
        welcome: "Ola <br/> <strong>Mundo</strong>",
        breadcrumbSteps: {
            select: "Seleccionar un Verificador",
            import: "Importar Datos",
            verify: "Verificar",
            finish: "Rematar",
        },
        electionEventBreadcrumbSteps: {
            created: "Creado",
            keys: "Chaves",
            publish: "Publicar",
            started: "Iniciado",
            ended: "Finalizado",
            results: "Resultados",
        },
        candidate: {
            moreInformationLink: "Máis información",
            writeInsPlaceholder: "Escribe aquí o candidato escrito",
            blankVote: "Voto en Branco",
            preferential: {
                position: "Posición",
                none: "Ningún",
                ordinals: {
                    first: "º",
                    second: "º",
                    third: "º",
                    other: "º",
                },
            },
        },
        homeScreen: {
            title: "Verificador de Papeletas Sequent",
            description1:
                "O verificador de papeletas úsase cando o votante decide auditar a papeleta no lugar de votación. A verificación debería tardar 1-2 minutos.",
            description2:
                "O verificador de papeletas permite ao votante asegurarse de que a papeleta cifrada recolle correctamente as seleccións feitas no lugar de votación. Realizar esta comprobación chámase verificabilidade como votado e prevén erros e actividades maliciosas durante o cifrado da papeleta.",
            descriptionMore: "Saber máis",
            startButton: "Explorar arquivo",
            dragDropOption: "Ou arrastrao aquí",
            importErrorDescription:
                "Houbo un problema ao importar a papeleta auditable. Escolliches o arquivo correcto?",
            importErrorMoreInfo: "Máis información",
            importErrorTitle: "Erro",
            useSampleText: "Non tes unha papeleta auditable?",
            useSampleLink: "Usar un exemplo de papeleta auditable",
        },
        confirmationScreen: {
            title: "Verificador de Papeletas Sequent",
            topDescription1:
                "Baseándose na información da Papeleta Auditable importada, calculamos que:",
            topDescription2: "Se este é o ID de Papeleta que se mostra no Lugar de Votación:",
            bottomDescription1:
                "A túa papeleta foi cifrada correctamente. Agora podes pechar esta ventá e volver ao Lugar de Votación.",
            bottomDescription2:
                "Se non coinciden, fai clic aquí para saber máis sobre os motivos potenciais e que accións podes tomar.",
            ballotChoicesDescription: "E as túas eleccións na papeleta son:",
            helpAndFaq: "Axuda e FAQ",
            backButton: "Volver",
            markedInvalid: "Papeleta marcada explicitamente como inválida",
        },
        ballotSelectionsScreen: {
            statusModal: {
                title: "Estado",
                content: "O panel de estado dálle información sobre as verificacións realizadas.",
                ok: "OK",
            },
        },
        footer: {
            poweredBy: "Desenvolvido por <sequent />",
        },
        errors: {
            encoding: {
                notEnoughChoices: "Non hai suficientes opcións para descodificar",
                writeInChoiceOutOfRange: "A opción escrita está fóra do rango: {{index}}",
                writeInNotEndInZero: "A opción escrita non remata en 0",
                writeInCharsExceeded_one: "Acurta a escritura libre en {{count}} carácter.",
                writeInCharsExceeded_other: "Acurta a escritura libre en {{count}} caracteres.",
                bytesToUtf8Conversion:
                    "Erro ao converter a opción escrita de bytes a unha cadea UTF-8: {{errorMessage}}",
                ballotTooLarge: "A papeleta é máis grande do esperado",
            },
            implicit: {
                selectedMax_one: "Desmarca {{count}} candidato.",
                selectedMax_other: "Desmarca {{count}} candidatos.",
                selectedMin_one: "Selecciona {{count}} candidato máis.",
                selectedMin_other: "Selecciona {{count}} candidatos máis.",
                maxSelectionsPerType_one: "Desmarca {{count}} candidato de {{type}}.",
                maxSelectionsPerType_other: "Desmarca {{count}} candidatos de {{type}}.",
                underVote_one: "Selecciona ata {{count}} candidato máis.",
                underVote_other: "Selecciona ata {{count}} candidatos máis.",
                overVoteDisabled_one:
                    "Seleccionaches o máximo de {{count}} candidato. Desmárcao para elixir outro.",
                overVoteDisabled_other:
                    "Seleccionaches o máximo de {{count}} candidatos. Desmarca un para elixir outro.",
                blankVote: "Non seleccionaches ningún candidato.",
                preferenceOrderWithGaps:
                    "Voto non válido! A orde de preferencia ten un ou máis ocos.",
                duplicatedPosition:
                    "Voto non válido! A mesma posición foi seleccionada para dous ou máis candidatos.",
            },
            explicit: {
                notAllowed:
                    "A papeleta está marcada como explícitamente inválida, pero a pregunta non o permite",
                alert: "A selección marcada será considerada voto inválido.",
            },
            configuration: {
                multipleExplicitInvalidCandidates:
                    "Configuración de voto inválida: a pregunta define {{count}} candidatos explicitamente inválidos, pero só se permite un.",
                multipleExplicitBlankCandidates:
                    "Configuración de voto inválida: a pregunta define {{count}} candidatos de voto en branco explícito, pero só se permite un.",
            },
        },
        ballotHash: "O teu ID de Papeleta: {{ballotId}}",
        version: {
            header: "Versión:",
        },
        hash: {
            header: "Hash:",
        },
        logout: {
            buttonText: "Pechar sesión",
            modal: {
                title: "Seguro que queres pechar a sesión?",
                content:
                    "Estás a piques de pechar esta aplicación. Esta acción non se pode desfacer.",
                ok: "OK",
                close: "Pechar",
            },
        },
        stories: {
            openDialog: "Abrir Diálogo",
        },
        dragNDrop: {
            firstLine: "Arrastra e solta arquivos ou",
            browse: "Explorar",
            format: "Formato soportado: txt",
        },
        selectElection: {
            electionWebsite: "Sitio Web da Papeleta",
            countdown:
                "A elección comeza en {{years}} anos, {{months}} meses, {{weeks}} semanas, {{days}} días, {{hours}} horas, {{minutes}} minutos, {{seconds}} segundos",
            openElection: "Aberta",
            closedElection: "Pechada",
            voted: "Votado",
            notVoted: "Non votado",
            resultsButton: "Resultados da Papeleta",
            voteButton: "Premer para Votar",
            openDate: "Apertura: ",
            closeDate: "Peche: ",
            ballotLocator: "Localiza a túa papeleta",
        },
        header: {
            profile: "Perfil",
            welcome: "Benvido,<br><span>{{name}}</span>",
            session: {
                title: "A túa sesión está a piques de expirar.",
                timeLeft: "Tes {{time}} para emitir o teu voto.",
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minutos e {{time}} segundos",
                timeLeftSeconds: "{{timeLeft}} segundos",
            },
        },
    },
}

export default galegoTranslation
