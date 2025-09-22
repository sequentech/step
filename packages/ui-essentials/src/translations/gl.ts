// SPDX-FileCopyrightText: 2025 Sequent Tech Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const galegoTranslation = {
    translations: {
        language: "Galego",
        welcome: "Ola <br/> <strong>Mundo</strong>",
        breadcrumbSteps: {
            select: "Seleccionar un Verificador",
            import: "Importar Datos",
            verify: "Verificar",
            finish: "Finalizar",
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
            writeInsPlaceholder: "Escriba aquí o candidato por escrito",
            blankVote: "Voto en Branco",
        },
        homeScreen: {
            title: "Verificador de Papeletas Sequent",
            description1:
                "O verificador de papeletas úsase cando o votante decide auditar a papeleta no posto de votación. A verificación debería levar 1-2 minutos.",
            description2:
                "O verificador de papeletas permítelle ao votante asegurarse de que a papeleta cifrada captura correctamente as seleccións realizadas no posto de votación. Permitir realizar esta comprobación chámase verificabilidade de emitido-como-intencionado e prevén erros e actividades maliciosas durante o cifrado da papeleta.",
            descriptionMore: "Aprender máis",
            startButton: "Buscar ficheiro",
            dragDropOption: "Ou arrástreo e déixeo aquí",
            importErrorDescription:
                "Houbo un problema ao importar a papeleta auditábel. ¿Elixiu o ficheiro correcto?",
            importErrorMoreInfo: "Máis información",
            importErrorTitle: "Erro",
            useSampleText: "¿Non ten unha papeleta auditábel?",
            useSampleLink: "Usar unha papeleta auditábel de mostra",
        },
        confirmationScreen: {
            title: "Verificador de Papeletas Sequent",
            topDescription1:
                "Baseándonos na información da Papeleta Auditábel importada, calculamos que:",
            topDescription2: "Se este é o ID da Papeleta que se mostra no Posto de Votación:",
            bottomDescription1:
                "A súa papeleta foi cifrada correctamente. Agora pode pechar esta ventá e volver ao Posto de Votación.",
            bottomDescription2:
                "Se non coinciden, faga clic aquí para aprender máis sobre as posibles razóns e que accións pode tomar.",
            ballotChoicesDescription: "E as súas opcións de voto son:",
            helpAndFaq: "Axuda e Preguntas Frequentes",
            backButton: "Volver",
            markedInvalid: "Papeleta marcada explícitamente como inválida",
        },
        ballotSelectionsScreen: {
            statusModal: {
                title: "Estado",
                content: "O panel de estado dálle información sobre as verificacións realizadas.",
                ok: "Aceptar",
            },
        },
        footer: {
            poweredBy: "Desenvolvido por <sequent />",
        },
        errors: {
            encoding: {
                notEnoughChoices: "Non hai suficientes opcións para descodificar",
                writeInChoiceOutOfRange: "Opción por escrito fóra de rango: {{index}}",
                writeInNotEndInZero: "A opción por escrito non remata en 0",
                bytesToUtf8Conversion:
                    "Erro ao converter a opción por escrito de bytes a cadea UTF-8: {{errorMessage}}",
                ballotTooLarge: "A papeleta é máis grande do esperado",
            },
            implicit: {
                selectedMax:
                    "O número de opcións seleccionadas {{numSelected}} é máis que o máximo {{max}}",
                selectedMin:
                    "O número de opcións seleccionadas {{numSelected}} é menor que o mínimo {{min}}",
            },
            explicit: {
                notAllowed:
                    "Papeleta marcada explícitamente como inválida pero a pregunta non o permite",
            },
        },
        ballotHash: "O seu ID de Papeleta: {{ballotId}}",
        version: {
            header: "Versión:",
        },
        hash: {
            header: "Hash:",
        },
        logout: {
            buttonText: "Pechar sesión",
            modal: {
                title: "¿Está seguro de que quere pechar sesión?",
                content:
                    "Está a piques de pechar esta aplicación. Esta acción non se pode desfacer.",
                ok: "Aceptar",
                close: "Pechar",
            },
        },
        stories: {
            openDialog: "Abrir Diálogo",
        },
        dragNDrop: {
            firstLine: "Arrastrar e soltar ficheiros ou",
            browse: "Buscar",
            format: "Formato soportado: txt",
        },
        selectElection: {
            electionWebsite: "Sitio Web da Papeleta",
            countdown:
                "A elección comeza en {{years}} anos, {{months}} meses, {{weeks}} semanas, {{days}} días, {{hours}} horas, {{minutes}} minutos, {{seconds}} segundos",
            openElection: "Aberto",
            closedElection: "Pechado",
            voted: "Votado",
            notVoted: "Non votado",
            resultsButton: "Resultados da Papeleta",
            voteButton: "Faga clic para Votar",
            openDate: "Apertura: ",
            closeDate: "Peche: ",
            ballotLocator: "Localice a súa papeleta",
        },
        header: {
            profile: "Perfil",
            welcome: "Benvido,<br><span>{{name}}</span>",
            session: {
                title: "A súa sesión está a piques de expirar.",
                timeLeft: "Ten {{time}} para emitir o seu voto.",
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minutos e {{time}} segundos",
                timeLeftSeconds: "{{timeLeft}} segundos",
            },
        },
    },
}

export default galegoTranslation
