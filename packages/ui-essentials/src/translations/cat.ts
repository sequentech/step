// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const catalanTranslation: TranslationType = {
    translations: {
        language: "Valencià",
        welcome: "Comencem: Importa el vot auditable..",
        breadcrumbSteps: {
            select: "Seleccionar un Verificador",
            import: "Importar Dades",
            verify: "Verificar",
            finish: "Acabar",
        },
        electionEventBreadcrumbSteps: {
            created: "Creat",
            keys: "Claus",
            publish: "Publicar",
            started: "Iniciat",
            ended: "Finalitzat",
            results: "Resultats",
        },
        candidate: {
            moreInformationLink: "Més informació",
            writeInsPlaceholder: "Tecleja aquí el candidat per escrit",
            blankVote: "Vot en blanc",
            preferential: {
                position: "Posició",
                none: "Cap",
                ordinals: {
                    first: "º",
                    second: "º",
                    third: "º",
                    other: "º",
                },
            },
        },
        homeScreen: {
            title: "Verificador de Vot Sequent",
            description1:
                "El verificador de vot s'utilitza quan el votant tria auditar la butlleta a la cabina de votació. La verificació ha de durar d'1 a 2 minuts.",
            description2:
                "El verificador de vot permet al votant assegurar-se que el vot xifrat capturi correctament les seleccions fetes a la cabina de votació. Permetre realitzar aquesta verificació es denomina verificabilitat de transmissió segons el previst i evita errors i activitats malicioses durant el xifratge del vot.",
            descriptionMore: "Més informació",
            startButton: "Selecciona fitxer",
            dragDropOption: "O arrossega el fitxer aquí",
            importErrorDescription:
                "Hi va haver un problema en importar el vot auditable. Vas triar el fitxer correcte?",
            importErrorMoreInfo: "Més informació",
            importErrorTitle: "Error",
            useSampleText: "No tens un vot verificable?",
            useSampleLink: "Utilitza un vot verificable d'exemple",
        },
        confirmationScreen: {
            title: "Verificador de Vot Sequent",
            topDescription1: "Basat en la informació del vot auditable importat, calculem que:",
            topDescription2: "Si aquest ID de vot és mostrat a la Cabina de Votació:",
            bottomDescription1:
                "El teu vot va ser xifrat correctament. Ara pots tancar aquesta finestra i tornar a la Cabina de Votació.",
            bottomDescription2:
                "Si no coincideixen, fes clic aquí per obtenir més informació sobre els possibles motius i les accions que pots prendre.",
            ballotChoicesDescription: "I les teves seleccions de vot són:",
            helpAndFaq: "Ajuda i Preguntes Freqüents",
            backButton: "Enrere",
            markedInvalid: "Vot explícitament marcat invàlid",
        },
        ballotSelectionsScreen: {
            statusModal: {
                title: "Estat",
                content:
                    "El panell d'estat et dóna informació sobre les verificacions realitzades.",
                ok: "OK",
            },
        },
        footer: {
            poweredBy: "Funciona amb <1></1>",
        },
        errors: {
            encoding: {
                notEnoughChoices: "No hi ha prou opcions per a decodificar",
                writeInChoiceOutOfRange: "Opció de vot escrita fora de rang: {{index}}",
                writeInNotEndInZero: "Opció de vot escrita no finalitza en 0",
                bytesToUtf8Conversion:
                    "Error convertint bytes d'opció de vot escrita a cadena UTF-8: {{errorMessage}}",
                ballotTooLarge: "Vot més gran del que s'esperava",
            },
            implicit: {
                selectedMax:
                    "El nombre d'opcions seleccionades {{numSelected}} és major que el màxim {{max}}",
                selectedMin:
                    "El nombre d'opcions seleccionades {{numSelected}} és menor que el màxim {{min}}",
            },
            explicit: {
                notAllowed: "Vot marcat explícitament com a invàlid però la pregunta no ho permet",
            },
        },
        ballotHash: "El teu Localitzador de Vot: {{ballotId}}",
        version: {
            header: "Versió:",
        },
        hash: {
            header: "Hash:",
        },
        logout: {
            buttonText: "Tanca sessió",
            modal: {
                title: "Estàs segur que vols tancar sessió?",
                content:
                    "Estàs a punt de tancar aquesta aplicació. Aquesta acció no es pot desfer.",
                ok: "OK",
                close: "Tanca",
            },
        },
        stories: {
            openDialog: "Obrir Diàleg",
        },
        dragNDrop: {
            firstLine: "Arrossega i deixa anar fitxers o",
            browse: "Carrega fitxer",
            format: "Formats suportats: txt",
        },
        selectElection: {
            electionWebsite: "Lloc web electoral",
            countdown:
                "L’elecció comença en {{years}} anys, {{months}} mesos, {{weeks}} setmanes, {{days}} dies, {{hours}} hores, {{minutes}} minuts, {{seconds}} segons",
            openElection: "Oberta",
            closedElection: "Tancada",
            voted: "Votat",
            notVoted: "No votat",
            resultsButton: "Resultats de la Votació",
            voteButton: "Fes clic per Votar",
            openDate: "Oberta: ",
            closeDate: "Tancada: ",
            ballotLocator: "Localitza el teu vot",
        },
        header: {
            profile: "Perfil",
            welcome: "Benvingut/da,<br><span>{{name}}</span>",
            session: {
                title: "La seva sessió està a punt d'expirar.",
                timeLeft: "Li queden {{time}} per emetre el seu vot.",
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minuts i {{time}} segons",
                timeLeftSeconds: "{{timeLeft}} segons",
            },
        },
    },
}

export default catalanTranslation
