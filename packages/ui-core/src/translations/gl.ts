// SPDX-FileCopyrightText: 2025 Sequent Tech Legal <legal@sequentech.io>
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
                notEnoughChoices: "Non hai suficientes opcións para descifrar",
                writeInChoiceOutOfRange: "Opción escrita fóra do rango: {{index}}",
                writeInNotEndInZero: "O candidato escrito non remata en 0",
                bytesToUtf8Conversion:
                    "Erro ao converter o candidato escrito de bytes a cadea UTF-8: {{errorMessage}}",
                ballotTooLarge: "A papeleta é máis grande do esperado",
            },
            implicit: {
                selectedMax:
                    "O número de opcións seleccionadas {{numSelected}} supera o máximo {{max}}",
                selectedMin:
                    "O número de opcións seleccionadas {{numSelected}} é menor que o mínimo {{min}}",
            },
            explicit: {
                notAllowed: "Papeleta marcada como inválida pero a pregunta non o permite",
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
