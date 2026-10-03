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
        a11y: {
            closeDialog: "Pechar o diálogo",
            languageSelector: "Idioma: {{language}}",
            dismissMessage: "Descartar a mensaxe",
            ballotIdHelp: "Sobre o seu ID de voto",
            loading: "Cargando",
            severity: {
                error: "Erro",
                warning: "Aviso",
                success: "Correcto",
                info: "Información",
            },
            selectList: "Seleccionar toda a lista",
            preferenceLabel: "Preferencia",
            writeInFor: "Nome do candidato escrito",
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
                writeInCharsExceeded:
                    "Supera o límite de caracteres permitidos por {{numCharsExceeded}}. Precísase corrixilo.",
                bytesToUtf8Conversion:
                    "Erro ao converter a opción escrita de bytes a unha cadea UTF-8: {{errorMessage}}",
                ballotTooLarge: "A papeleta é máis grande do esperado",
            },
            implicit: {
                selectedMax:
                    "Voto excedido: Número de opcións seleccionadas {{numSelected}} supera o máximo {{max}}",
                selectedMin:
                    "Número de opcións seleccionadas {{numSelected}} está por debaixo do mínimo {{min}}",
                maxSelectionsPerType:
                    "Número de opcións seleccionadas {{numSelected}} para a lista {{type}} supera o máximo {{max}}",
                underVote:
                    "Voto insuficiente: Número de opcións seleccionadas {{numSelected}} está por debaixo do máximo {{max}}",
                overVoteDisabled:
                    "Máximo alcanzado: Seleccionaches o máximo {{numSelected}} opcións. Para cambiar a selección, deselecciona primeiro outra opción.",
                blankVote: "Voto en branco: 0 opcións seleccionadas",
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
                    "Configuración de voto inválida: o concurso define {{count}} candidatos explicitamente inválidos, pero só se permite un.",
                multipleExplicitBlankCandidates:
                    "Configuración de voto inválida: o concurso define {{count}} candidatos de voto en branco explícito, pero só se permite un.",
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
            importError: "Non se puido importar este ficheiro. Téntao de novo.",
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
        problems: {
            noProblems: "Nada que sinalar. Isto importaríase.",
            errors_one: "{{count}} erro",
            errors_other: "{{count}} erros",
            errorsExplained:
                "Isto non se poderá importar ata que se corrixa cada un destes puntos.",
            warnings_one: "{{count}} aviso",
            warnings_other: "{{count}} avisos",
            warningsExplained:
                "Isto importarase. Cada un destes puntos é algo que probablemente non era o que se quería.",
            warningsStrict:
                "O modo estrito está activado, así que estes puntos impiden a construción.",
            messages: {
                area: {
                    "cycle": {
                        lead: "Circunscricións unha dentro doutra",
                        text: "Circunscricións unha dentro doutra — a circunscrición «{{area}}» forma parte dun ciclo de nais, que bloquearía o Portal de Administración.",
                    },
                    "duplicate-name": {
                        lead: "Dúas circunscricións co mesmo nome",
                        text: "Dúas circunscricións co mesmo nome — «{{name}}» é á vez «{{first}}» e «{{second}}». O CSV de votantes resolve a circunscrición polo nome, así que os votantes acabarían na que o importador atopase primeiro.",
                    },
                    "early-voting-unknown": {
                        lead: "Configuración de voto anticipado descoñecida",
                        text: "Configuración de voto anticipado descoñecida — «{{value}}» non é válido para unha circunscrición. A plataforma acepta {{allowed}}.",
                    },
                    "empty-ballot": {
                        lead: "Circunscrición coa papeleta baleira",
                        text: "Circunscrición coa papeleta baleira — a circunscrición «{{area}}» non vota en ningunha votación, nin propia nin herdada dunha circunscrición que a conteña, así que os seus votantes verían unha papeleta baleira.",
                    },
                    "inside-itself": {
                        lead: "Circunscrición dentro de si mesma",
                        text: "Circunscrición dentro de si mesma — unha circunscrición non pode ser a súa propia nai.",
                    },
                    "no-identifier": {
                        lead: "Circunscrición sen identificador",
                        text: "Circunscrición sen identificador — todas necesitan un.",
                    },
                    "no-name": {
                        lead: "Circunscrición sen nome",
                        text: "Circunscrición sen nome — o CSV de votantes identifica a circunscrición dun votante polo nome, non polo id, así que nunha circunscrición sen nome non se pode poñer a ninguén.",
                    },
                    "parent-missing": {
                        lead: "Falta a circunscrición nai",
                        text: "Falta a circunscrición nai — a circunscrición está dentro dunha nai que non está neste ficheiro.",
                    },
                    "parent-unknown": {
                        lead: "Circunscrición nai descoñecida",
                        text: "Circunscrición nai descoñecida — nada ten o identificador «{{parent}}».",
                    },
                },
                ballot: {
                    "no-elections": {
                        lead: "Sen eleccións",
                        text: "Sen eleccións — un evento electoral necesita polo menos unha.",
                    },
                },
                bundle: {
                    "duplicate-id": {
                        lead: "Identificador repetido",
                        text: "Identificador repetido — {{id}} en {{kind}} tamén o usa {{previous}}.",
                    },
                    "event-no-encryption": {
                        lead: "Sen protocolo de cifrado",
                        text: "Sen protocolo de cifrado — o evento electoral non di como se cifran as papeletas.",
                    },
                    "event-no-id": {
                        lead: "Evento sen id",
                        text: "Evento sen id — o evento electoral deste ficheiro non ten identificador.",
                    },
                    "no-areas": {
                        lead: "Sen circunscricións",
                        text: "Sen circunscricións — non se lle pode dar papeleta a ningún votante ata que o evento teña polo menos unha circunscrición.",
                    },
                    "no-elections": {
                        lead: "Sen eleccións",
                        text: "Sen eleccións — un evento electoral necesita polo menos unha elección.",
                    },
                    "no-tenant": {
                        lead: "Sen tenant",
                        text: "Sen tenant — o ficheiro non di a que tenant pertence.",
                    },
                    "tenant-not-uuid": {
                        lead: "O tenant non é un identificador",
                        text: "O tenant non é un identificador — «{{value}}» non é un id de tenant válido.",
                    },
                },
                candidate: {
                    "contest-missing": {
                        lead: "Falta a votación da candidatura",
                        text: "Falta a votación da candidatura — a candidatura pertence a unha votación que non está neste ficheiro.",
                    },
                    "no-contest": {
                        lead: "Candidatura sen votación",
                        text: "Candidatura sen votación — cada candidatura debe pertencer a unha votación.",
                    },
                    "picture-mismatch": {
                        lead: "A imaxe apunta a outro sitio",
                        text: "A imaxe apunta a outro sitio — a papeleta mostra «{{url}}», que non corresponde ao documento «{{document}}» que a acompaña. Despois da importación, ambos apuntarían a ficheiros distintos.",
                    },
                    "picture-not-shown": {
                        lead: "Imaxe que nunca se mostra",
                        text: "Imaxe que nunca se mostra — unha candidatura nomea unha imaxe á que non apunta ningunha entrada da papeleta, así que se subiría e nunca se mostraría.",
                    },
                    "picture-unrecorded": {
                        lead: "Imaxe non rexistrada",
                        text: "Imaxe non rexistrada — a imaxe dunha candidatura está na papeleta e nada rexistra que documento é, así que despois non se poderá cambiar nin eliminar na plataforma.",
                    },
                },
                census: {
                    "column-not-declared": {
                        lead: "Columna do censo non declarada",
                        text: "Columna do censo non declarada — {{message}}",
                    },
                    "no-area-column": {
                        lead: "O censo non nomea ningunha circunscrición",
                        text: "O censo non nomea ningunha circunscrición — esta elección ten circunscricións pero todos os votantes recibirían a papeleta por defecto. Se a división debe aplicarse, o censo necesita unha columna de circunscrición.",
                    },
                    "note": {
                        lead: "Sobre o censo",
                        text: "Sobre o censo — {{message}}",
                    },
                    "unreadable": {
                        lead: "Censo ilexible",
                        text: "Censo ilexible — {{message}}",
                    },
                    "unreadable-member": {
                        lead: "Censo do zip ilexible",
                        text: "Censo do zip ilexible — {{message}}",
                    },
                },
                channels: {
                    "early-voting-closed": {
                        lead: "Voto anticipado desactivado",
                        text: "Voto anticipado desactivado — {{areas}} permiten o voto anticipado e o evento non abre esa canle, así que a configuración non fai nada.",
                    },
                    "early-voting-no-area": {
                        lead: "Voto anticipado sen circunscrición",
                        text: "Voto anticipado sen circunscrición — o voto anticipado está aberto e ningunha circunscrición o permite, así que o período anticipado non tería votantes.",
                    },
                    "kiosk-client": {
                        lead: "O quiosco necesita o seu propio cliente",
                        text: "O quiosco necesita o seu propio cliente — o voto en quiosco necesita un cliente de autenticación co nome do ordinario rematado en '-kiosk', que este ficheiro non crea.",
                    },
                    "none-open": {
                        lead: "Ningunha forma de votar",
                        text: "Ningunha forma de votar — todas as canles de votación están desactivadas, así que ninguén pode votar.",
                    },
                    "telephone-elsewhere": {
                        lead: "Voto telefónico configurado despois",
                        text: "Voto telefónico configurado despois — o voto telefónico configúrase na lapela IVR do evento despois da importación; nada diso está neste ficheiro.",
                    },
                    "unknown": {
                        lead: "Forma de votar descoñecida",
                        text: "Forma de votar descoñecida — nada le «{{name}}». As formas de votar coas que traballa a plataforma son {{allowed}}.",
                    },
                },
                contacts: {
                    none: {
                        lead: "Sen puntos de contacto",
                        text: "Sen puntos de contacto — o día da elección é a quen se chama.",
                    },
                },
                contest: {
                    "algorithm-unknown": {
                        lead: "Método de reconto descoñecido",
                        text: "Método de reconto descoñecido — «{{value}}» non é un algoritmo de reconto. A plataforma acepta {{allowed}}.",
                    },
                    "area-unknown": {
                        lead: "Circunscrición da votación descoñecida",
                        text: "Circunscrición da votación descoñecida — nada ten o identificador «{{area}}».",
                    },
                    "cap-invalid": {
                        lead: "Límite de selección imposible",
                        text: "Límite de selección imposible — {{cap}} non é un número de seleccións.",
                    },
                    "cap-never-applies": {
                        lead: "O límite de selección nunca se aplica",
                        text: "O límite de selección nunca se aplica — un límite de {{cap}} por tipo nunca se aplica nunha votación na que un votante pode escoller {{max}} en total.",
                    },
                    "chooses-more-than-available": {
                        lead: "Máis eleccións que candidaturas",
                        text: "Máis eleccións que candidaturas — un votante pode escoller {{max}} entre {{available}} candidaturas.",
                    },
                    "chooses-more-than-offered": {
                        lead: "Máis eleccións que opcións",
                        text: "Máis eleccións que opcións — un votante pode escoller ata {{chosen}} pero só hai {{offered}} entre as que escoller.",
                    },
                    "columns-invalid": {
                        lead: "Disposición imposible",
                        text: "Disposición imposible — {{columns}} columnas non é unha disposición.",
                    },
                    "columns-too-many": {
                        lead: "Demasiadas columnas",
                        text: "Demasiadas columnas — {{columns}} columnas serán ilexibles nun móbil, que é como vota a maioría dos votantes.",
                    },
                    "count-missing": {
                        lead: "Á votación fáltalle un número",
                        text: "Á votación fáltalle un número — a votación necesita {{field}}.",
                    },
                    "count-negative": {
                        lead: "Conta negativa",
                        text: "Conta negativa — {{field}} é {{value}}, e unha conta non pode ser inferior a cero.",
                    },
                    "election-missing": {
                        lead: "Falta a elección da votación",
                        text: "Falta a elección da votación — a votación pertence a unha elección que non está neste ficheiro.",
                    },
                    "elects-more-than-available": {
                        lead: "Máis postos que candidaturas",
                        text: "Máis postos que candidaturas — a votación elixe {{winners}} entre {{available}} candidaturas.",
                    },
                    "elects-more-than-chosen": {
                        lead: "Elixe máis do permitido",
                        text: "Elixe máis do permitido — a votación elixe {{winners}} pero un votante só pode escoller {{chosen}}.",
                    },
                    "elects-more-than-standing": {
                        lead: "Máis postos que candidaturas",
                        text: "Máis postos que candidaturas — a votación elixe {{winners}} dun total de {{standing}}.",
                    },
                    "elects-nobody": {
                        lead: "Non elixe a ninguén",
                        text: "Non elixe a ninguén — a votación non ten gañadores.",
                    },
                    "max-votes-below-one": {
                        lead: "Nada que votar",
                        text: "Nada que votar — un votante pode escoller menos dunha candidatura.",
                    },
                    "min-above-max": {
                        lead: "Mínimo por riba do máximo",
                        text: "Mínimo por riba do máximo — un votante debe escoller polo menos {{min}} pero pode escoller como máximo {{max}}.",
                    },
                    "no-candidates": {
                        lead: "Sen candidaturas",
                        text: "Sen candidaturas — aínda non se presenta ninguén a esta votación.",
                    },
                    "no-candidates-in-bundle": {
                        lead: "Sen candidaturas",
                        text: "Sen candidaturas — a votación non ten candidaturas, así que ninguén pode votar nela.",
                    },
                    "on-no-ballot": {
                        lead: "Votación en ningunha papeleta",
                        text: "Votación en ningunha papeleta — a papeleta de ningunha circunscrición inclúe esta votación, así que ninguén pode votar nela.",
                    },
                    "policy-not-text": {
                        lead: "A regra da papeleta non é texto",
                        text: "A regra da papeleta non é texto — {{key}} debería ser texto, e é {{value}}.",
                    },
                    "policy-unknown": {
                        lead: "Regra de papeleta descoñecida",
                        text: "Regra de papeleta descoñecida — «{{value}}» non é un valor válido para {{key}}. A plataforma acepta {{allowed}}.",
                    },
                    "ranked-counted-unranked": {
                        lead: "Papeleta ordenada contada sen orde",
                        text: "Papeleta ordenada contada sen orde — unha votación preferencial cóntase con «{{algorithm}}», que ignora a orde que dan os votantes.",
                    },
                    "tie-breaking-unknown": {
                        lead: "Regra de desempate descoñecida",
                        text: "Regra de desempate descoñecida — «{{value}}» non é unha política de desempate. A plataforma acepta {{allowed}}.",
                    },
                    "unranked-counted-ranked": {
                        lead: "Papeleta simple contada por orde",
                        text: "Papeleta simple contada por orde — unha votación non preferencial cóntase con «{{algorithm}}», que necesita papeletas ordenadas.",
                    },
                    "voting-type-unknown": {
                        lead: "Tipo de votación descoñecido",
                        text: "Tipo de votación descoñecido — «{{value}}» non é un tipo de votación. A plataforma acepta {{allowed}}.",
                    },
                    "write-in-slots-not-allowed": {
                        lead: "Espazos de voto libre non permitidos",
                        text: "Espazos de voto libre non permitidos — hai {{count}} espazos de voto libre nunha votación que non o permite, o que pon opcións sen nome na papeleta.",
                    },
                    "write-ins-no-slot": {
                        lead: "Voto libre sen onde escribir",
                        text: "Voto libre sen onde escribir — permítense candidaturas escritas polo votante e a votación non ten ningún espazo para iso, así que un votante non ten onde escribir un nome.",
                    },
                },
                delivery: {
                    "no-importable": {
                        lead: "Sen arquivo importable",
                        text: "Sen arquivo importable — este zip ten un plan pero non o arquivo que importa o Portal de Administración, así que non contén o censo nin os ficheiros que nomea.",
                    },
                },
                election: {
                    "channels-differ": {
                        lead: "A elección e o evento non coinciden",
                        text: "A elección e o evento non coinciden — {{channels}} é distinto do do evento, así que os controis de inicio desta elección non coincidirían.",
                    },
                    "grace-disallowed": {
                        lead: "Período de graza desactivado",
                        text: "Período de graza desactivado — hai {{seconds}} segundos de graza configurados e non se permite ningún período de graza, así que a votación pecha na hora límite.",
                    },
                    "grace-negative": {
                        lead: "Período de graza negativo",
                        text: "Período de graza negativo — {{value}} segundos non é unha duración.",
                    },
                    "grace-zero": {
                        lead: "Período de graza sen duración",
                        text: "Período de graza sen duración — permítese un período de graza e dura cero segundos, así que non hai ningún.",
                    },
                    "no-contests": {
                        lead: "Sen votacións",
                        text: "Sen votacións — ninguén vota nesta elección.",
                    },
                    "revotes-negative": {
                        lead: "Número de votos imposible",
                        text: "Número de votos imposible — {{value}} non é un número de veces que un votante poida votar.",
                    },
                    "setting-unknown": {
                        lead: "Configuración da elección descoñecida",
                        text: "Configuración da elección descoñecida — «{{value}}» non é un valor válido para {{key}}. A plataforma acepta {{allowed}}.",
                    },
                    "spoil-without-revote": {
                        lead: "Anular sen segunda oportunidade",
                        text: "Anular sen segunda oportunidade — un votante pode descartar unha papeleta emitida e non ten un segundo intento para substituíla.",
                    },
                },
                event: {
                    "no-identifier": {
                        lead: "Sen identificador",
                        text: "Sen identificador — todos os id xerados derívanse del, así que sen un nada se pode construír dúas veces igual.",
                    },
                    "no-name": {
                        lead: "Sen nome",
                        text: "Sen nome — os votantes veno enriba da papeleta, e pasa a ser o título da páxina de acceso.",
                    },
                    "setting-unknown": {
                        lead: "Configuración do evento descoñecida",
                        text: "Configuración do evento descoñecida — «{{value}}» non é un valor válido para {{key}}. A plataforma acepta {{allowed}}.",
                    },
                },
                file: {
                    "cannot-decrypt": {
                        lead: "Non se puido descifrar",
                        text: "Non se puido descifrar — o ficheiro está cifrado e non se abriu. Comprobe o contrasinal.",
                    },
                    "checksum-mismatch": {
                        lead: "Non é o ficheiro esperado",
                        text: "Non é o ficheiro esperado — o seu SHA-256 non coincide co indicado, así que non é o ficheiro que se quería. Comprobe a suma de verificación ou volva subir o ficheiro.",
                    },
                    "duplicate-name": {
                        lead: "Nome de ficheiro repetido",
                        text: "Nome de ficheiro repetido — «{{file}}» nomea dúas cousas distintas. Os ficheiros viaxan identificados polo nome, así que un substituiría o outro en silencio.",
                    },
                    "missing": {
                        lead: "Falta un ficheiro",
                        text: "Falta un ficheiro — nomease «{{file}}» e aquí non hai nada que o conteña. Unha entrada de arquivo baleira fai fallar a importación en vez de perder un ficheiro.",
                    },
                    "not-a-bundle": {
                        lead: "Non é unha exportación de evento electoral",
                        text: "Non é unha exportación de evento electoral — o ficheiro leuse pero non ten a súa forma: {{reason}}",
                    },
                    "not-json": {
                        lead: "Non é un ficheiro lexible",
                        text: "Non é un ficheiro lexible — isto non é unha exportación de evento electoral que a plataforma poida ler: {{reason}}",
                    },
                    "unreadable-archive": {
                        lead: "Arquivo ilexible",
                        text: "Arquivo ilexible — {{reason}}",
                    },
                    "unused": {
                        lead: "Ficheiro non usado",
                        text: "Ficheiro non usado — achegouse «{{file}}» e nada o nomea, así que viaxaría na entrega e non se lle mostraría a ninguén.",
                    },
                    "version-incompatible": {
                        lead: "Exportado con outra versión",
                        text: "Exportado con outra versión — o ficheiro vén da versión {{found}}, que a versión {{current}} non pode importar.",
                    },
                },
                identifier: {
                    duplicate: {
                        lead: "Identificador repetido",
                        text: "Identificador repetido — «{{identifier}}» xa o usa {{first}}. Os identificadores son únicos en todo o evento electoral, así que o segundo substitúe o primeiro en vez de engadirse.",
                    },
                },
                ivr: {
                    "language-not-spoken": {
                        lead: "Idioma non dispoñible por teléfono",
                        text: "Idioma non dispoñible por teléfono — a chamada só fala inglés, francés e español, así que non se ofrece {{languages}} a quen chama.",
                    },
                    "missing-prompts": {
                        lead: "Faltan mensaxes da chamada",
                        text: "Faltan mensaxes da chamada — {{prompts}} non teñen texto en «{{language}}», e o sistema telefónico rexeita todas as chamadas ata que o teñan.",
                        lead_one: "Falta unha mensaxe da chamada",
                        text_one:
                            "Falta unha mensaxe da chamada — {{prompts}} non ten texto en «{{language}}», e o sistema telefónico rexeita todas as chamadas ata que o teña.",
                    },
                },
                labels: {
                    "in-use": {
                        lead: "Etiquetas de permiso en uso",
                        text: "Etiquetas de permiso en uso — {{labels}}. Todo o que leva unha etiqueta queda oculto para calquera administrador que non a teña, así que quen importe isto necesita unha desas etiquetas na súa propia conta ou o Portal de Administración mostraralle unha lista baleira.",
                    },
                },
                languages: {
                    "default-not-offered": {
                        lead: "Idioma por defecto non ofrecido",
                        text: "Idioma por defecto non ofrecido — «{{chosen}}» non está entre {{offered}}, así que os votantes recibirían o primeiro.",
                    },
                    "detection-unknown": {
                        lead: "Detección de idioma descoñecida",
                        text: "Detección de idioma descoñecida — «{{policy}}» non é unha política que a plataforma coñeza.",
                    },
                    "none": {
                        lead: "Sen idiomas",
                        text: "Sen idiomas — a papeleta recorre ao inglés, que é unha rede de seguridade e non unha elección.",
                    },
                },
                link: {
                    "area-missing": {
                        lead: "Ligazón de papeleta a unha circunscrición que falta",
                        text: "Ligazón de papeleta a unha circunscrición que falta — unha votación ponse na papeleta dunha circunscrición que non está neste ficheiro.",
                    },
                    "contest-missing": {
                        lead: "Ligazón de papeleta a unha votación que falta",
                        text: "Ligazón de papeleta a unha votación que falta — a papeleta dunha circunscrición inclúe unha votación que non está neste ficheiro.",
                    },
                },
                logo: {
                    "file-and-link": {
                        lead: "Dous logotipos",
                        text: "Dous logotipos — este plan leva á vez un logotipo subido («{{file}}») e unha ligazón («{{url}}»). Envíase o ficheiro; a ligazón ignórase.",
                    },
                    "file-missing": {
                        lead: "Falta o ficheiro do logotipo",
                        text: "Falta o ficheiro do logotipo — «{{file}}» indícase como logotipo e non se achegou. Poña o ficheiro xunto á folla de cálculo con exactamente ese nome.",
                    },
                    "no-bytes": {
                        lead: "Ficheiro do logotipo baleiro",
                        text: "Ficheiro do logotipo baleiro — «{{file}}» indícase como logotipo e non ten ningún byte. Unha entrada de arquivo baleira fai fallar a importación en vez de perder unha imaxe.",
                    },
                },
                material: {
                    "empty-document": {
                        lead: "Material cun documento baleiro",
                        text: "Material cun documento baleiro — importaríase como unha ligazón a nada. Se o material non ten ficheiro, deixe o documento fóra de todo.",
                    },
                    "file-missing": {
                        lead: "Falta o ficheiro do material",
                        text: "Falta o ficheiro do material — aquí nomease «{{file}}» e non se achegou. Poña o ficheiro xunto á folla de cálculo con exactamente ese nome.",
                    },
                    "file-unused": {
                        lead: "Ficheiro de material non usado",
                        text: "Ficheiro de material non usado — achegouse «{{file}}» e ningunha fila o nomea, así que se subiría e non se lle mostraría a ninguén.",
                    },
                    "no-identifier": {
                        lead: "Material sen identificador",
                        text: "Material sen identificador — o id do seu documento derívase del, así que sen un o ficheiro non se pode emparellar coa fila.",
                    },
                    "tab-off": {
                        lead: "Lapela de materiais desactivada",
                        text: "Lapela de materiais desactivada — este ficheiro contén {{count}} materiais de apoio e a lapela que os mostra está desactivada, así que os votantes nunca os verían.",
                    },
                    "wrong-event": {
                        lead: "Material doutro evento",
                        text: "Material doutro evento — este material de apoio pertence a outro evento electoral.",
                    },
                },
                messages: {
                    "weekly-no-time": {
                        lead: "Unha repetición sen hora",
                        text: "Unha repetición sen hora — unha mensaxe repítese cada semana pero non di a que hora, así que quen a envíe terá que escoller unha hora que ninguén escribiu.",
                    },
                },
                passwords: {
                    "column-already-there": {
                        lead: "Contrasinais indicados dúas veces",
                        text: "Contrasinais indicados dúas veces — o censo xa ten unha columna de contrasinal, así que os contrasinais xerados serían unha segunda resposta á mesma pregunta. Elimine a columna ou desactive a xeración.",
                    },
                    "no-characters": {
                        lead: "Contrasinais sen caracteres",
                        text: "Contrasinais sen caracteres — os contrasinais xerados necesitan polo menos un tipo de carácter do que estar feitos.",
                    },
                    "no-seed": {
                        lead: "Contrasinais sen semente",
                        text: "Contrasinais sen semente — a semente é o que fai que unha reconstrución produza os mesmos contrasinais en vez de novos.",
                    },
                },
                plan: {
                    "not-a-plan": {
                        lead: "Non é un plan electoral",
                        text: "Non é un plan electoral — o ficheiro non se puido ler como tal: {{error}}",
                    },
                    "saved-by-newer-version": {
                        lead: "Gardado cunha versión máis nova",
                        text: "Gardado cunha versión máis nova — {{saved}} fronte a {{supported}}. Abrilo aquí descartaría en silencio o que esa versión engadise.",
                    },
                    "unreadable": {
                        lead: "Plan ilexible",
                        text: "Plan ilexible — {{error}}",
                    },
                },
                schedule: {
                    "closes-before-opens": {
                        lead: "Pecha antes de abrir",
                        text: "Pecha antes de abrir — a votación nunca estaría aberta.",
                    },
                    "crosses-daylight-saving": {
                        lead: "Atravesa un cambio de hora",
                        text: "Atravesa un cambio de hora — a xanela dura unha hora máis ou unha hora menos do que indican as horas.",
                    },
                    "key-ceremony-not-first": {
                        lead: "Cerimonia de claves demasiado tarde",
                        text: "Cerimonia de claves demasiado tarde — a clave da elección ten que existir antes de que se poida cifrar un voto con ela.",
                    },
                    "tally-ceremony-too-early": {
                        lead: "Cerimonia de reconto demasiado cedo",
                        text: "Cerimonia de reconto demasiado cedo — contaría votos que aínda non se emitiron.",
                    },
                    "window-incomplete": {
                        lead: "Xanela de votación incompleta",
                        text: "Xanela de votación incompleta — haberá que abrir ou pechar o período a man no Portal de Administración.",
                    },
                },
                threshold: {
                    "above-trustees": {
                        lead: "Limiar demasiado alto",
                        text: "Limiar demasiado alto — esíxense {{threshold}} de {{trustees}} persoas depositarias, o que non se pode cumprir. A clave xeraríase e o resultado non se podería descifrar nunca.",
                    },
                    "one": {
                        lead: "Limiar de un",
                        text: "Limiar de un — calquera persoa depositaria pode abrir o reconto soa, diga o que diga a lista — a mesma garantía que ter unha soa, e ningunha fronte a esa persoa.",
                    },
                    "zero": {
                        lead: "Limiar de cero",
                        text: "Limiar de cero — o reconto poderíase abrir sen ninguén.",
                    },
                },
                trustee: {
                    "email-malformed": {
                        lead: "Non é un enderezo de correo",
                        text: "Non é un enderezo de correo — «{{email}}» non o parece.",
                    },
                    "no-email": {
                        lead: "Persoa depositaria sen enderezo",
                        text: "Persoa depositaria sen enderezo — é como se lle convida á cerimonia de claves.",
                    },
                    "no-name": {
                        lead: "Persoa depositaria sen nome",
                        text: "Persoa depositaria sen nome — o importador resolve polo nome os membros da cerimonia de claves, comparándoos coas persoas depositarias xa dadas de alta no tenant, e un nome baleiro convértese en silencio nun membro que non existe.",
                    },
                },
                trustees: {
                    "none": {
                        lead: "Sen persoas depositarias",
                        text: "Sen persoas depositarias — unha clave de elección necesita polo menos dúas persoas que a custodien. Cunha soa, esa persoa pode descifrar todas as papeletas pola súa conta, que é xusto a garantía que achega o cifrado.",
                    },
                    "only-one": {
                        lead: "Só unha persoa depositaria",
                        text: "Só unha persoa depositaria — unha clave de elección necesita polo menos dúas persoas que a custodien. Cunha soa, esa persoa pode descifrar todas as papeletas pola súa conta, que é xusto a garantía que achega o cifrado.",
                    },
                },
                voter: {
                    "area-unknown": {
                        lead: "Circunscrición do votante descoñecida",
                        text: "Circunscrición do votante descoñecida — nada se chama «{{area}}». Os votantes emparéllanse coa súa circunscrición polo nome, así que este votante non recibiría papeleta. Copie o nome da circunscrición en vez de volver escribilo.",
                    },
                    "duplicate-username": {
                        lead: "Nome de usuario repetido",
                        text: "Nome de usuario repetido — «{{username}}» tamén está na fila {{first}}. Dous votantes que comparten nome de usuario convértense nunha soa conta, e esta substituiría a outra sen dicilo.",
                    },
                    "no-area": {
                        lead: "Votante sen circunscrición",
                        text: "Votante sen circunscrición — a circunscrición decide que papeleta recibe cada votante, e a construción rexeita unha fila do censo sen ela.",
                    },
                    "no-username": {
                        lead: "Votante sen nome de usuario",
                        text: "Votante sen nome de usuario — é co que accede e do que se deriva a súa conta.",
                    },
                },
                voters: {
                    "duplicate-column": {
                        lead: "Columna indicada dúas veces",
                        text: "Columna indicada dúas veces — dúas columnas definen «{{column}}». Conserve só unha delas.",
                    },
                    "unreadable-row": {
                        lead: "Fila ilexible",
                        text: "Fila ilexible — a fila {{row}} non se puido ler: {{reason}}",
                    },
                    "vote-weight-misspelled": {
                        lead: "Columna do peso do voto mal escrita",
                        text: "Columna do peso do voto mal escrita — non se recoñece «{{column}}». A columna escríbese exactamente «{{expected}}»; se non, todos os votantes contaríanse con peso 1.",
                    },
                    "vote-weight-not-a-number": {
                        lead: "O peso do voto non é un número",
                        text: "O peso do voto non é un número — «{{value}}» na fila {{row}} debe ser un número enteiro entre 1 e {{max}}.",
                    },
                    "vote-weight-out-of-range": {
                        lead: "Peso do voto fóra de rango",
                        text: "Peso do voto fóra de rango — {{value}} na fila {{row}} debe estar entre {{min}} e {{max}}.",
                    },
                },
            },
        },
    },
}

export default galegoTranslation
