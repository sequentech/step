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
        a11y: {
            closeDialog: "Tancar el diàleg",
            dismissMessage: "Descartar el missatge",
            ballotIdHelp: "Sobre el vostre ID de vot",
            loading: "Carregant",
            severity: {
                error: "Error",
                warning: "Advertiment",
                success: "Correcte",
                info: "Informació",
            },
            selectList: "Seleccionar tota la llista",
            preferenceLabel: "Preferència",
            writeInFor: "Nom del candidat per escrit",
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
            poweredBy: "Funciona amb <sequent />",
        },
        errors: {
            encoding: {
                notEnoughChoices: "No hi ha prou opcions per desxifrar",
                writeInChoiceOutOfRange: "Opció de vot escrita fora de rang: {{index}}",
                writeInNotEndInZero: "Opció de vot escrita no finalitza en 0",
                writeInCharsExceeded:
                    "Opció de vot escrita excedeix el nombre de caràcters per {{numCharsExceeded}} caràcters. Requereix arranjament.",
                bytesToUtf8Conversion:
                    "Error convertint bytes d'opció de vot escrita a cadena UTF-8: {{errorMessage}}",
                ballotTooLarge: "Vot més gran de l'esperat",
            },
            implicit: {
                selectedMax:
                    "Sobrevot: El nombre d'opcions seleccionades {{numSelected}} és major que el màxim {{max}}",
                selectedMin:
                    "El nombre d'opcions seleccionades {{numSelected}} és menor que el mínim {{min}}",
                maxSelectionsPerType:
                    "El nombre d'opcions seleccionades {{numSelected}} per a la llista {{type}} és major que el màxim {{max}}",
                underVote:
                    "Subvot: El nombre d'opcions seleccionades {{numSelected}} és inferior al màxim permès de {{max}}",
                overVoteDisabled:
                    "Màxim assolit: Has seleccionat el màxim de {{numSelected}} opcions. Per canviar la teva selecció, si us plau, desmarca primer una altra opció.",
                blankVote: "Vot en Blanc: 0 opcions seleccionades",
                preferenceOrderWithGaps: "Vot invàlid! L'ordre de preferència té un o més buits.",
                duplicatedPosition:
                    "Vot invàlid! La mateixa posició va ser seleccionada per a dos o més candidats.",
            },
            explicit: {
                notAllowed: "Vot marcat explícitament com a invàlid però la pregunta no ho permet",
                alert: "La selecció marcada es considerarà vot invàlid.",
            },
            configuration: {
                multipleExplicitInvalidCandidates:
                    "Configuració de vot invàlida: el concurs defineix {{count}} candidats explícitament invàlids, però només se'n permet un.",
                multipleExplicitBlankCandidates:
                    "Configuració de vot invàlida: el concurs defineix {{count}} candidats de vot en blanc explícit, però només se'n permet un.",
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
            importError: "No s’ha pogut importar aquest fitxer. Torneu-ho a provar.",
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
        problems: {
            noProblems: "Res a assenyalar. Això s'importaria.",
            errors_one: "{{count}} error",
            errors_other: "{{count}} errors",
            errorsExplained:
                "Això no es podrà importar fins que es corregeixi cadascun d'aquests punts.",
            warnings_one: "{{count}} avís",
            warnings_other: "{{count}} avisos",
            warningsExplained:
                "Això s'importarà. Cadascun d'aquests punts és una cosa que probablement no era el que es volia.",
            warningsStrict:
                "El mode estricte està activat, així que aquests punts impedeixen la construcció.",
            messages: {
                area: {
                    "cycle": {
                        lead: "Circumscripcions l'una dins de l'altra",
                        text: "Circumscripcions l'una dins de l'altra — la circumscripció «{{area}}» forma part d'un cicle de mares, que bloquejaria el Portal d'Administració.",
                    },
                    "duplicate-name": {
                        lead: "Dues circumscripcions amb el mateix nom",
                        text: "Dues circumscripcions amb el mateix nom — «{{name}}» és alhora «{{first}}» i «{{second}}». El CSV de votants resol la circumscripció pel nom, així que els votants anirien a parar a la que l'importador trobés primer.",
                    },
                    "early-voting-unknown": {
                        lead: "Configuració de vot anticipat desconeguda",
                        text: "Configuració de vot anticipat desconeguda — «{{value}}» no és vàlid per a una circumscripció. La plataforma accepta {{allowed}}.",
                    },
                    "empty-ballot": {
                        lead: "Circumscripció amb la papereta buida",
                        text: "Circumscripció amb la papereta buida — la circumscripció «{{area}}» no vota en cap votació, ni pròpia ni heretada d'una circumscripció que la contingui, així que els seus votants veurien una papereta buida.",
                    },
                    "inside-itself": {
                        lead: "Circumscripció dins de si mateixa",
                        text: "Circumscripció dins de si mateixa — una circumscripció no pot ser la seva pròpia mare.",
                    },
                    "no-identifier": {
                        lead: "Circumscripció sense identificador",
                        text: "Circumscripció sense identificador — totes en necessiten un.",
                    },
                    "no-name": {
                        lead: "Circumscripció sense nom",
                        text: "Circumscripció sense nom — el CSV de votants identifica la circumscripció d'un votant pel nom, no per l'id, així que en una circumscripció sense nom no s'hi pot posar ningú.",
                    },
                    "parent-missing": {
                        lead: "Falta la circumscripció mare",
                        text: "Falta la circumscripció mare — la circumscripció és dins d'una mare que no és en aquest fitxer.",
                    },
                    "parent-unknown": {
                        lead: "Circumscripció mare desconeguda",
                        text: "Circumscripció mare desconeguda — res no té l'identificador «{{parent}}».",
                    },
                },
                ballot: {
                    "no-elections": {
                        lead: "Sense eleccions",
                        text: "Sense eleccions — un esdeveniment electoral en necessita almenys una.",
                    },
                },
                bundle: {
                    "duplicate-id": {
                        lead: "Identificador repetit",
                        text: "Identificador repetit — {{id}} a {{kind}} també l'utilitza {{previous}}.",
                    },
                    "event-no-encryption": {
                        lead: "Sense protocol de xifratge",
                        text: "Sense protocol de xifratge — l'esdeveniment electoral no diu com es xifren les paperetes.",
                    },
                    "event-no-id": {
                        lead: "Esdeveniment sense id",
                        text: "Esdeveniment sense id — l'esdeveniment electoral d'aquest fitxer no té identificador.",
                    },
                    "no-areas": {
                        lead: "Sense circumscripcions",
                        text: "Sense circumscripcions — no es pot donar cap papereta a cap votant fins que l'esdeveniment no tingui almenys una circumscripció.",
                    },
                    "no-elections": {
                        lead: "Sense eleccions",
                        text: "Sense eleccions — un esdeveniment electoral necessita almenys una elecció.",
                    },
                    "no-tenant": {
                        lead: "Sense tenant",
                        text: "Sense tenant — el fitxer no diu a quin tenant pertany.",
                    },
                    "tenant-not-uuid": {
                        lead: "El tenant no és un identificador",
                        text: "El tenant no és un identificador — «{{value}}» no és un id de tenant vàlid.",
                    },
                },
                candidate: {
                    "contest-missing": {
                        lead: "Falta la votació de la candidatura",
                        text: "Falta la votació de la candidatura — la candidatura pertany a una votació que no és en aquest fitxer.",
                    },
                    "no-contest": {
                        lead: "Candidatura sense votació",
                        text: "Candidatura sense votació — cada candidatura ha de pertànyer a una votació.",
                    },
                    "picture-mismatch": {
                        lead: "La imatge apunta a un altre lloc",
                        text: "La imatge apunta a un altre lloc — la papereta mostra «{{url}}», que no correspon al document «{{document}}» que l'acompanya. Després de la importació, tots dos apuntarien a fitxers diferents.",
                    },
                    "picture-not-shown": {
                        lead: "Imatge que no es mostra mai",
                        text: "Imatge que no es mostra mai — una candidatura esmenta una imatge a la qual no apunta cap entrada de la papereta, així que es pujaria i no es mostraria mai.",
                    },
                    "picture-unrecorded": {
                        lead: "Imatge no registrada",
                        text: "Imatge no registrada — la imatge d'una candidatura és a la papereta i res no registra quin document és, així que després no es podrà canviar ni eliminar a la plataforma.",
                    },
                },
                census: {
                    "column-not-declared": {
                        lead: "Columna del cens no declarada",
                        text: "Columna del cens no declarada — {{message}}",
                    },
                    "no-area-column": {
                        lead: "El cens no esmenta cap circumscripció",
                        text: "El cens no esmenta cap circumscripció — aquesta elecció té circumscripcions però tots els votants rebrien la papereta per defecte. Si la divisió s'ha d'aplicar, el cens necessita una columna de circumscripció.",
                    },
                    "note": {
                        lead: "Sobre el cens",
                        text: "Sobre el cens — {{message}}",
                    },
                    "unreadable": {
                        lead: "Cens il·legible",
                        text: "Cens il·legible — {{message}}",
                    },
                    "unreadable-member": {
                        lead: "Cens del zip il·legible",
                        text: "Cens del zip il·legible — {{message}}",
                    },
                },
                channels: {
                    "early-voting-closed": {
                        lead: "Vot anticipat desactivat",
                        text: "Vot anticipat desactivat — {{areas}} permeten el vot anticipat i l'esdeveniment no obre aquest canal, així que la configuració no fa res.",
                    },
                    "early-voting-no-area": {
                        lead: "Vot anticipat sense circumscripció",
                        text: "Vot anticipat sense circumscripció — el vot anticipat està obert i cap circumscripció no el permet, així que el període anticipat no tindria votants.",
                    },
                    "kiosk-client": {
                        lead: "El quiosc necessita el seu propi client",
                        text: "El quiosc necessita el seu propi client — el vot en quiosc necessita un client d'autenticació amb el nom de l'ordinari acabat en '-kiosk', que aquest fitxer no crea.",
                    },
                    "none-open": {
                        lead: "Cap forma de votar",
                        text: "Cap forma de votar — tots els canals de votació estan desactivats, així que ningú no pot votar.",
                    },
                    "telephone-elsewhere": {
                        lead: "Vot telefònic configurat després",
                        text: "Vot telefònic configurat després — el vot telefònic es configura a la pestanya IVR de l'esdeveniment després de la importació; res d'això no és en aquest fitxer.",
                    },
                    "unknown": {
                        lead: "Forma de votar desconeguda",
                        text: "Forma de votar desconeguda — res no llegeix «{{name}}». Les formes de votar amb què treballa la plataforma són {{allowed}}.",
                    },
                },
                contacts: {
                    none: {
                        lead: "Sense punts de contacte",
                        text: "Sense punts de contacte — el dia de l'elecció és a qui es truca.",
                    },
                },
                contest: {
                    "algorithm-unknown": {
                        lead: "Mètode de recompte desconegut",
                        text: "Mètode de recompte desconegut — «{{value}}» no és un algorisme de recompte. La plataforma accepta {{allowed}}.",
                    },
                    "area-unknown": {
                        lead: "Circumscripció de la votació desconeguda",
                        text: "Circumscripció de la votació desconeguda — res no té l'identificador «{{area}}».",
                    },
                    "cap-invalid": {
                        lead: "Límit de selecció impossible",
                        text: "Límit de selecció impossible — {{cap}} no és un nombre de seleccions.",
                    },
                    "cap-never-applies": {
                        lead: "El límit de selecció no s'aplica mai",
                        text: "El límit de selecció no s'aplica mai — un límit de {{cap}} per tipus no s'aplica mai en una votació on un votant pot triar-ne {{max}} en total.",
                    },
                    "chooses-more-than-available": {
                        lead: "Més opcions triables que candidatures",
                        text: "Més opcions triables que candidatures — un votant pot triar-ne {{max}} d'entre {{available}} candidatures.",
                    },
                    "chooses-more-than-offered": {
                        lead: "Més opcions triables que opcions",
                        text: "Més opcions triables que opcions — un votant pot triar fins a {{chosen}} però només n'hi ha {{offered}} per triar.",
                    },
                    "columns-invalid": {
                        lead: "Disposició impossible",
                        text: "Disposició impossible — {{columns}} columnes no és una disposició.",
                    },
                    "columns-too-many": {
                        lead: "Massa columnes",
                        text: "Massa columnes — {{columns}} columnes seran il·legibles en un mòbil, que és com vota la majoria de votants.",
                    },
                    "count-missing": {
                        lead: "A la votació li falta un nombre",
                        text: "A la votació li falta un nombre — la votació necessita {{field}}.",
                    },
                    "count-negative": {
                        lead: "Recompte negatiu",
                        text: "Recompte negatiu — {{field}} és {{value}}, i un recompte no pot ser inferior a zero.",
                    },
                    "election-missing": {
                        lead: "Falta l'elecció de la votació",
                        text: "Falta l'elecció de la votació — la votació pertany a una elecció que no és en aquest fitxer.",
                    },
                    "elects-more-than-available": {
                        lead: "Més llocs que candidatures",
                        text: "Més llocs que candidatures — la votació elegeix {{winners}} d'entre {{available}} candidatures.",
                    },
                    "elects-more-than-chosen": {
                        lead: "Elegeix més del permès",
                        text: "Elegeix més del permès — la votació elegeix {{winners}} però un votant només en pot triar {{chosen}}.",
                    },
                    "elects-more-than-standing": {
                        lead: "Més llocs que candidatures",
                        text: "Més llocs que candidatures — la votació elegeix {{winners}} d'un total de {{standing}}.",
                    },
                    "elects-nobody": {
                        lead: "No elegeix ningú",
                        text: "No elegeix ningú — la votació no té guanyadors.",
                    },
                    "max-votes-below-one": {
                        lead: "Res a votar",
                        text: "Res a votar — un votant pot triar menys d'una candidatura.",
                    },
                    "min-above-max": {
                        lead: "Mínim per sobre del màxim",
                        text: "Mínim per sobre del màxim — un votant n'ha de triar almenys {{min}} però en pot triar com a màxim {{max}}.",
                    },
                    "no-candidates": {
                        lead: "Sense candidatures",
                        text: "Sense candidatures — encara no es presenta ningú a aquesta votació.",
                    },
                    "no-candidates-in-bundle": {
                        lead: "Sense candidatures",
                        text: "Sense candidatures — la votació no té candidatures, així que ningú no hi pot votar.",
                    },
                    "on-no-ballot": {
                        lead: "Votació en cap papereta",
                        text: "Votació en cap papereta — la papereta de cap circumscripció no inclou aquesta votació, així que ningú no hi pot votar.",
                    },
                    "policy-not-text": {
                        lead: "La regla de la papereta no és text",
                        text: "La regla de la papereta no és text — {{key}} hauria de ser text, i és {{value}}.",
                    },
                    "policy-unknown": {
                        lead: "Regla de papereta desconeguda",
                        text: "Regla de papereta desconeguda — «{{value}}» no és un valor vàlid per a {{key}}. La plataforma accepta {{allowed}}.",
                    },
                    "ranked-counted-unranked": {
                        lead: "Papereta ordenada comptada sense ordre",
                        text: "Papereta ordenada comptada sense ordre — una votació preferencial es compta amb «{{algorithm}}», que ignora l'ordre que donen els votants.",
                    },
                    "tie-breaking-unknown": {
                        lead: "Regla de desempat desconeguda",
                        text: "Regla de desempat desconeguda — «{{value}}» no és una política de desempat. La plataforma accepta {{allowed}}.",
                    },
                    "unranked-counted-ranked": {
                        lead: "Papereta simple comptada per ordre",
                        text: "Papereta simple comptada per ordre — una votació no preferencial es compta amb «{{algorithm}}», que necessita paperetes ordenades.",
                    },
                    "voting-type-unknown": {
                        lead: "Tipus de votació desconegut",
                        text: "Tipus de votació desconegut — «{{value}}» no és un tipus de votació. La plataforma accepta {{allowed}}.",
                    },
                    "write-in-slots-not-allowed": {
                        lead: "Espais de vot lliure no permesos",
                        text: "Espais de vot lliure no permesos — hi ha {{count}} espais de vot lliure en una votació que no el permet, cosa que posa opcions sense nom a la papereta.",
                    },
                    "write-ins-no-slot": {
                        lead: "Vot lliure sense on escriure",
                        text: "Vot lliure sense on escriure — es permeten candidatures escrites pel votant i la votació no té cap espai per fer-ho, així que un votant no té on escriure un nom.",
                    },
                },
                delivery: {
                    "no-importable": {
                        lead: "Sense arxiu importable",
                        text: "Sense arxiu importable — aquest zip té un pla però no l'arxiu que importa el Portal d'Administració, així que no conté el cens ni els fitxers que esmenta.",
                    },
                },
                design: {
                    "no-stable-key": {
                        lead: "Disseny de papereta sense clau",
                        text: "Disseny de papereta sense clau — {{kind}} {{id}} no té nom ni identificador extern, així que els seus dissenys de papereta no es poden reconèixer després d'una importació.",
                    },
                    "unreadable-style": {
                        lead: "Estil de papereta il·legible",
                        text: "Estil de papereta il·legible — no s'ha pogut llegir l'estil de papereta de la plataforma per calcular l'empremta del seu disseny: {{reason}}",
                    },
                },
                election: {
                    "channels-differ": {
                        lead: "L'elecció i l'esdeveniment no coincideixen",
                        text: "L'elecció i l'esdeveniment no coincideixen — {{channels}} és diferent del de l'esdeveniment, així que els controls d'inici d'aquesta elecció no hi coincidirien.",
                    },
                    "grace-disallowed": {
                        lead: "Període de gràcia desactivat",
                        text: "Període de gràcia desactivat — hi ha {{seconds}} segons de gràcia configurats i no es permet cap període de gràcia, així que la votació tanca a l'hora límit.",
                    },
                    "grace-negative": {
                        lead: "Període de gràcia negatiu",
                        text: "Període de gràcia negatiu — {{value}} segons no és una durada.",
                    },
                    "grace-zero": {
                        lead: "Període de gràcia sense durada",
                        text: "Període de gràcia sense durada — es permet un període de gràcia i dura zero segons, així que no n'hi ha cap.",
                    },
                    "no-contests": {
                        lead: "Sense votacions",
                        text: "Sense votacions — ningú no vota en aquesta elecció.",
                    },
                    "revotes-negative": {
                        lead: "Nombre de vots impossible",
                        text: "Nombre de vots impossible — {{value}} no és un nombre de vegades que un votant pugui votar.",
                    },
                    "setting-unknown": {
                        lead: "Configuració de l'elecció desconeguda",
                        text: "Configuració de l'elecció desconeguda — «{{value}}» no és un valor vàlid per a {{key}}. La plataforma accepta {{allowed}}.",
                    },
                    "spoil-without-revote": {
                        lead: "Anul·lar sense segona oportunitat",
                        text: "Anul·lar sense segona oportunitat — un votant pot descartar una papereta emesa i no té cap segon intent per substituir-la.",
                    },
                },
                event: {
                    "no-identifier": {
                        lead: "Sense identificador",
                        text: "Sense identificador — tots els id generats se'n deriven, així que sense un res no es pot construir dues vegades igual.",
                    },
                    "no-name": {
                        lead: "Sense nom",
                        text: "Sense nom — els votants el veuen damunt de la papereta, i passa a ser el títol de la pàgina d'accés.",
                    },
                    "setting-unknown": {
                        lead: "Configuració de l'esdeveniment desconeguda",
                        text: "Configuració de l'esdeveniment desconeguda — «{{value}}» no és un valor vàlid per a {{key}}. La plataforma accepta {{allowed}}.",
                    },
                },
                file: {
                    "cannot-decrypt": {
                        lead: "No s'ha pogut desxifrar",
                        text: "No s'ha pogut desxifrar — el fitxer està xifrat i no s'ha obert. Comproveu la contrasenya.",
                    },
                    "checksum-mismatch": {
                        lead: "No és el fitxer esperat",
                        text: "No és el fitxer esperat — el seu SHA-256 no coincideix amb l'indicat, així que no és el fitxer que es volia. Comproveu la suma de verificació o torneu a pujar el fitxer.",
                    },
                    "duplicate-name": {
                        lead: "Nom de fitxer repetit",
                        text: "Nom de fitxer repetit — «{{file}}» anomena dues coses diferents. Els fitxers viatgen identificats pel nom, així que un substituiria l'altre en silenci.",
                    },
                    "missing": {
                        lead: "Falta un fitxer",
                        text: "Falta un fitxer — s'esmenta «{{file}}» i aquí no hi ha res que el contingui. Una entrada d'arxiu buida fa fallar la importació en lloc de perdre un fitxer.",
                    },
                    "not-a-bundle": {
                        lead: "No és una exportació d'esdeveniment electoral",
                        text: "No és una exportació d'esdeveniment electoral — el fitxer s'ha llegit però no en té la forma: {{reason}}",
                    },
                    "not-json": {
                        lead: "No és un fitxer llegible",
                        text: "No és un fitxer llegible — això no és una exportació d'esdeveniment electoral que la plataforma pugui llegir: {{reason}}",
                    },
                    "unreadable-archive": {
                        lead: "Arxiu il·legible",
                        text: "Arxiu il·legible — {{reason}}",
                    },
                    "unused": {
                        lead: "Fitxer no utilitzat",
                        text: "Fitxer no utilitzat — s'ha proporcionat «{{file}}» i res no l'esmenta, així que viatjaria a l'entrega i no es mostraria a ningú.",
                    },
                    "version-incompatible": {
                        lead: "Exportat amb una altra versió",
                        text: "Exportat amb una altra versió — el fitxer ve de la versió {{found}}, que la versió {{current}} no pot importar.",
                    },
                },
                identifier: {
                    duplicate: {
                        lead: "Identificador repetit",
                        text: "Identificador repetit — «{{identifier}}» ja l'utilitza {{first}}. Els identificadors són únics en tot l'esdeveniment electoral, així que el segon substitueix el primer en lloc d'afegir-se.",
                    },
                },
                ivr: {
                    "language-not-spoken": {
                        lead: "Idioma no disponible per telèfon",
                        text: "Idioma no disponible per telèfon — la trucada només parla anglès, francès i castellà, així que no s'ofereix {{languages}} a qui truca.",
                    },
                    "missing-prompts": {
                        lead: "Falten missatges de la trucada",
                        text: "Falten missatges de la trucada — {{prompts}} no tenen text en «{{language}}», i el sistema telefònic rebutja totes les trucades fins que en tinguin.",
                        lead_one: "Falta un missatge de la trucada",
                        text_one:
                            "Falta un missatge de la trucada — {{prompts}} no té text en «{{language}}», i el sistema telefònic rebutja totes les trucades fins que en tingui.",
                    },
                },
                labels: {
                    "in-use": {
                        lead: "Etiquetes de permís en ús",
                        text: "Etiquetes de permís en ús — {{labels}}. Tot el que porta una etiqueta queda ocult per a qualsevol administrador que no la tingui, així que qui importi això necessita una d'aquestes etiquetes al seu propi compte o el Portal d'Administració li mostrarà una llista buida.",
                    },
                },
                languages: {
                    "default-not-offered": {
                        lead: "Idioma per defecte no oferit",
                        text: "Idioma per defecte no oferit — «{{chosen}}» no és entre {{offered}}, així que els votants rebrien el primer.",
                    },
                    "detection-unknown": {
                        lead: "Detecció d'idioma desconeguda",
                        text: "Detecció d'idioma desconeguda — «{{policy}}» no és una política que la plataforma conegui.",
                    },
                    "none": {
                        lead: "Sense idiomes",
                        text: "Sense idiomes — la papereta recorre a l'anglès, que és una xarxa de seguretat i no una elecció.",
                    },
                },
                link: {
                    "area-missing": {
                        lead: "Enllaç de papereta a una circumscripció que falta",
                        text: "Enllaç de papereta a una circumscripció que falta — una votació es posa a la papereta d'una circumscripció que no és en aquest fitxer.",
                    },
                    "contest-missing": {
                        lead: "Enllaç de papereta a una votació que falta",
                        text: "Enllaç de papereta a una votació que falta — la papereta d'una circumscripció inclou una votació que no és en aquest fitxer.",
                    },
                },
                logo: {
                    "file-and-link": {
                        lead: "Dos logotips",
                        text: "Dos logotips — aquest pla porta alhora un logotip pujat («{{file}}») i un enllaç («{{url}}»). S'envia el fitxer; l'enllaç s'ignora.",
                    },
                    "file-missing": {
                        lead: "Falta el fitxer del logotip",
                        text: "Falta el fitxer del logotip — «{{file}}» s'indica com a logotip i no s'ha proporcionat. Poseu el fitxer al costat del llibre de càlcul amb exactament aquest nom.",
                    },
                    "no-bytes": {
                        lead: "Fitxer del logotip buit",
                        text: "Fitxer del logotip buit — «{{file}}» s'indica com a logotip i no té cap byte. Una entrada d'arxiu buida fa fallar la importació en lloc de perdre una imatge.",
                    },
                },
                material: {
                    "empty-document": {
                        lead: "Material amb un document buit",
                        text: "Material amb un document buit — s'importaria com un enllaç a res. Si el material no té fitxer, deixeu fora el document del tot.",
                    },
                    "file-missing": {
                        lead: "Falta el fitxer del material",
                        text: "Falta el fitxer del material — aquí s'esmenta «{{file}}» i no s'ha proporcionat. Poseu el fitxer al costat del llibre de càlcul amb exactament aquest nom.",
                    },
                    "file-unused": {
                        lead: "Fitxer de material no utilitzat",
                        text: "Fitxer de material no utilitzat — s'ha proporcionat «{{file}}» i cap fila no l'esmenta, així que es pujaria i no es mostraria a ningú.",
                    },
                    "no-identifier": {
                        lead: "Material sense identificador",
                        text: "Material sense identificador — l'id del seu document se'n deriva, així que sense un el fitxer no es pot associar a la fila.",
                    },
                    "tab-off": {
                        lead: "Pestanya de materials desactivada",
                        text: "Pestanya de materials desactivada — aquest fitxer conté {{count}} materials de suport i la pestanya que els mostra està desactivada, així que els votants no els veurien mai.",
                    },
                    "wrong-event": {
                        lead: "Material d'un altre esdeveniment",
                        text: "Material d'un altre esdeveniment — aquest material de suport pertany a un altre esdeveniment electoral.",
                    },
                },
                messages: {
                    "weekly-no-time": {
                        lead: "Una repetició sense hora",
                        text: "Una repetició sense hora — un missatge es repeteix cada setmana però no diu a quina hora, així que qui l'enviï haurà de triar una hora que ningú no va escriure.",
                    },
                },
                passwords: {
                    "column-already-there": {
                        lead: "Contrasenyes indicades dues vegades",
                        text: "Contrasenyes indicades dues vegades — el cens ja té una columna de contrasenya, així que les contrasenyes generades serien una segona resposta a la mateixa pregunta. Elimineu la columna o desactiveu la generació.",
                    },
                    "no-characters": {
                        lead: "Contrasenyes sense caràcters",
                        text: "Contrasenyes sense caràcters — les contrasenyes generades necessiten almenys un tipus de caràcter amb què formar-se.",
                    },
                    "no-seed": {
                        lead: "Contrasenyes sense llavor",
                        text: "Contrasenyes sense llavor — la llavor és el que fa que una reconstrucció produeixi les mateixes contrasenyes en lloc de noves.",
                    },
                },
                package: {
                    "approval-invalid": {
                        lead: "L'aprovació no compta",
                        text: "L'aprovació no compta — no s'ha pogut verificar l'aprovació de {{name}}: {{reason}}",
                    },
                    "approval-repeated": {
                        lead: "La mateixa persona ha aprovat dues vegades",
                        text: "La mateixa persona ha aprovat dues vegades — {{name}} ha aprovat més d'una vegada, i compta una sola vegada.",
                    },
                    "approver-key-usage": {
                        lead: "L'aprovador no pot signar",
                        text: "L'aprovador no pot signar — el certificat d'un aprovador no està fet per signar.",
                    },
                    "bad-signature": {
                        lead: "La signatura no coincideix",
                        text: "La signatura no coincideix — la signatura del paquet no es verifica, així que s'ha modificat després de signar-lo o l'ha signat una altra clau: {{reason}}",
                    },
                    "content-digest": {
                        lead: "L'empremta del contingut no coincideix",
                        text: "L'empremta del contingut no coincideix — el manifest diu {{expected}} i el seu contingut dona {{actual}}.",
                    },
                    "duplicate-member": {
                        lead: "Nom de fitxer repetit",
                        text: "Nom de fitxer repetit — «{{file}}» apareix dues vegades a {{archive}}, així que dos lectors podrien agafar fitxers diferents.",
                    },
                    "file-changed": {
                        lead: "Modificat després de signar",
                        text: "Modificat després de signar — {{file}} té SHA-256 {{actual}}, i el manifest diu {{expected}}. No s'ha llegit res del paquet.",
                    },
                    "file-extra": {
                        lead: "Fitxer fora del manifest",
                        text: "Fitxer fora del manifest — {{file}} és al paquet però no s'ha signat. No s'ha llegit res del paquet.",
                    },
                    "file-missing": {
                        lead: "Falta un fitxer signat",
                        text: "Falta un fitxer signat — {{file}} és al manifest i no al paquet. No s'ha llegit res del paquet.",
                    },
                    "invalid-time": {
                        lead: "No és una data i hora",
                        text: "No és una data i hora — «{{value}}» al manifest no és una data i hora.",
                    },
                    "revoked-approver": {
                        lead: "Certificat d'aprovador revocat",
                        text: "Certificat d'aprovador revocat — el certificat d'un aprovador s'ha revocat, així que l'aprovació no compta.",
                    },
                    "revoked-signer": {
                        lead: "Clau de signatura revocada",
                        text: "Clau de signatura revocada — la clau que ha signat aquest paquet s'ha revocat, i els seus paquets es rebutgen.",
                    },
                    "rollback": {
                        lead: "No és una revisió més recent",
                        text: "No és una revisió més recent — la revisió {{revision}} no és més recent que la revisió {{last}}, l'última importada.",
                    },
                    "signer-key-usage": {
                        lead: "La clau de signatura no pot signar",
                        text: "La clau de signatura no pot signar — el certificat de la clau que ha signat aquest paquet no està fet per signar.",
                    },
                    "too-few-approvals": {
                        lead: "Massa poques aprovacions",
                        text: "Massa poques aprovacions — {{count}} aprovacions vàlides de persones diferents, i se'n necessiten {{required}}.",
                    },
                    "unhashable-content": {
                        lead: "No es pot calcular l'empremta",
                        text: "No es pot calcular l'empremta — no s'ha pogut escriure el contingut de la configuració per calcular-ne l'empremta: {{reason}}",
                    },
                    "unknown-format": {
                        lead: "Format de manifest desconegut",
                        text: "Format de manifest desconegut — el manifest és en el format «{{format}}», que aquesta versió no pot llegir.",
                    },
                    "unreadable-chain": {
                        lead: "Certificats del signant il·legibles",
                        text: "Certificats del signant il·legibles — no s'ha pogut llegir la cadena de certificats del paquet: {{reason}}",
                    },
                    "unreadable-manifest": {
                        lead: "Manifest il·legible",
                        text: "Manifest il·legible — no s'ha pogut llegir el manifest del paquet: {{reason}}",
                    },
                    "unreadable-trust": {
                        lead: "Certificats de confiança il·legibles",
                        text: "Certificats de confiança il·legibles — no s'ha pogut llegir el paràmetre {{setting}}: {{reason}}",
                    },
                    "unreadable-zip": {
                        lead: "Arxiu il·legible",
                        text: "Arxiu il·legible — no s'ha pogut llegir {{archive}} com a zip: {{reason}}",
                    },
                    "unsigned": {
                        lead: "Paquet sense signar",
                        text: "Paquet sense signar — no té {{missing}}, i aquesta instal·lació només importa paquets signats.",
                    },
                    "untrusted-approver": {
                        lead: "Aprovador no fiable",
                        text: "Aprovador no fiable — el certificat d'un aprovador no és de confiança: {{reason}}",
                    },
                    "untrusted-signer": {
                        lead: "Signant no fiable",
                        text: "Signant no fiable — la clau que ha signat aquest paquet no és una de les que aquesta instal·lació considera de confiança: {{reason}}",
                    },
                    "unwritable-manifest": {
                        lead: "No es pot escriure el manifest",
                        text: "No es pot escriure el manifest — no s'ha pogut escriure el manifest: {{reason}}",
                    },
                },
                plan: {
                    "not-a-plan": {
                        lead: "No és un pla electoral",
                        text: "No és un pla electoral — el fitxer no s'ha pogut llegir com a tal: {{error}}",
                    },
                    "saved-by-newer-version": {
                        lead: "Desat amb una versió més nova",
                        text: "Desat amb una versió més nova — {{saved}} davant de {{supported}}. Obrir-lo aquí descartaria en silenci el que aquella versió hagués afegit.",
                    },
                    "unreadable": {
                        lead: "Pla il·legible",
                        text: "Pla il·legible — {{error}}",
                    },
                },
                schedule: {
                    "closes-before-opens": {
                        lead: "Tanca abans d'obrir",
                        text: "Tanca abans d'obrir — la votació no estaria mai oberta.",
                    },
                    "crosses-daylight-saving": {
                        lead: "Travessa un canvi d'hora",
                        text: "Travessa un canvi d'hora — la finestra dura una hora més o una hora menys del que indiquen les hores.",
                    },
                    "key-ceremony-not-first": {
                        lead: "Cerimònia de claus massa tard",
                        text: "Cerimònia de claus massa tard — la clau de l'elecció ha d'existir abans que s'hi pugui xifrar un vot.",
                    },
                    "tally-ceremony-too-early": {
                        lead: "Cerimònia de recompte massa aviat",
                        text: "Cerimònia de recompte massa aviat — comptaria vots que encara no s'han emès.",
                    },
                    "window-incomplete": {
                        lead: "Finestra de votació incompleta",
                        text: "Finestra de votació incompleta — caldrà obrir o tancar el període a mà al Portal d'Administració.",
                    },
                },
                threshold: {
                    "above-trustees": {
                        lead: "Llindar massa alt",
                        text: "Llindar massa alt — s'exigeixen {{threshold}} de {{trustees}} persones dipositàries, cosa que no es pot complir. La clau es generaria i el resultat no es podria desxifrar mai.",
                    },
                    "one": {
                        lead: "Llindar d'u",
                        text: "Llindar d'u — qualsevol persona dipositària pot obrir el recompte tota sola, digui el que digui la llista — la mateixa garantia que tenir-ne una de sola, i cap davant d'aquella persona.",
                    },
                    "zero": {
                        lead: "Llindar de zero",
                        text: "Llindar de zero — el recompte es podria obrir sense ningú.",
                    },
                },
                trustee: {
                    "email-malformed": {
                        lead: "No és una adreça electrònica",
                        text: "No és una adreça electrònica — «{{email}}» no ho sembla.",
                    },
                    "no-email": {
                        lead: "Persona dipositària sense adreça",
                        text: "Persona dipositària sense adreça — és com se la convida a la cerimònia de claus.",
                    },
                    "no-name": {
                        lead: "Persona dipositària sense nom",
                        text: "Persona dipositària sense nom — l'importador resol pel nom els membres de la cerimònia de claus, comparant-los amb les persones dipositàries ja donades d'alta al tenant, i un nom buit es converteix en silenci en un membre que no existeix.",
                    },
                },
                trustees: {
                    "none": {
                        lead: "Sense persones dipositàries",
                        text: "Sense persones dipositàries — una clau d'elecció necessita almenys dues persones que la custodiïn. Amb una de sola, aquesta persona pot desxifrar totes les paperetes pel seu compte, que és just la garantia que aporta el xifratge.",
                    },
                    "only-one": {
                        lead: "Només una persona dipositària",
                        text: "Només una persona dipositària — una clau d'elecció necessita almenys dues persones que la custodiïn. Amb una de sola, aquesta persona pot desxifrar totes les paperetes pel seu compte, que és just la garantia que aporta el xifratge.",
                    },
                },
                voter: {
                    "area-unknown": {
                        lead: "Circumscripció del votant desconeguda",
                        text: "Circumscripció del votant desconeguda — res no es diu «{{area}}». Els votants s'associen a la seva circumscripció pel nom, així que aquest votant no rebria cap papereta. Copieu el nom de la circumscripció en lloc de tornar-lo a escriure.",
                    },
                    "duplicate-username": {
                        lead: "Nom d'usuari repetit",
                        text: "Nom d'usuari repetit — «{{username}}» també és a la fila {{first}}. Dos votants que comparteixen nom d'usuari es converteixen en un sol compte, i aquest substituiria l'altre sense dir-ho.",
                    },
                    "no-area": {
                        lead: "Votant sense circumscripció",
                        text: "Votant sense circumscripció — la circumscripció decideix quina papereta rep cada votant, i la construcció rebutja una fila del cens que no en tingui.",
                    },
                    "no-username": {
                        lead: "Votant sense nom d'usuari",
                        text: "Votant sense nom d'usuari — és amb què inicia la sessió i d'on es deriva el seu compte.",
                    },
                },
                voters: {
                    "duplicate-column": {
                        lead: "Columna indicada dues vegades",
                        text: "Columna indicada dues vegades — dues columnes defineixen «{{column}}». Mantingueu-ne només una.",
                    },
                    "unreadable-row": {
                        lead: "Fila il·legible",
                        text: "Fila il·legible — la fila {{row}} no s'ha pogut llegir: {{reason}}",
                    },
                    "vote-weight-misspelled": {
                        lead: "Columna del pes del vot mal escrita",
                        text: "Columna del pes del vot mal escrita — no es reconeix «{{column}}». La columna s'escriu exactament «{{expected}}»; si no, tots els votants es comptarien amb pes 1.",
                    },
                    "vote-weight-not-a-number": {
                        lead: "El pes del vot no és un nombre",
                        text: "El pes del vot no és un nombre — «{{value}}» a la fila {{row}} ha de ser un nombre enter entre 1 i {{max}}.",
                    },
                    "vote-weight-out-of-range": {
                        lead: "Pes del vot fora de rang",
                        text: "Pes del vot fora de rang — {{value}} a la fila {{row}} ha d'estar entre {{min}} i {{max}}.",
                    },
                },
            },
        },
    },
}

export default catalanTranslation
