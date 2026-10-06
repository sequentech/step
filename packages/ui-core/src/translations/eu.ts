// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const basqueTranslation: TranslationType = {
    translations: {
        language: "Euskara",
        welcome: "Kaixo <br/> <strong>Mundua</strong>",
        breadcrumbSteps: {
            select: "Hautatu egiaztatzaile bat",
            import: "Inportatu datuak",
            verify: "Egiaztatu",
            finish: "Amaitu",
        },
        electionEventBreadcrumbSteps: {
            created: "Sortua",
            keys: "Gakoak",
            publish: "Argitaratu",
            started: "Hasita",
            ended: "Amaituta",
            results: "Emaitzak",
        },
        a11y: {
            closeDialog: "Itxi elkarrizketa-koadroa",
            dismissMessage: "Baztertu mezua",
            ballotIdHelp: "Zure botoaren IDari buruz",
            loading: "Kargatzen",
            severity: {
                error: "Errorea",
                warning: "Abisua",
                success: "Ondo",
                info: "Informazioa",
            },
            selectList: "Hautatu zerrenda osoa",
            preferenceLabel: "Lehentasuna",
            writeInFor: "Idatzitako hautagaiaren izena",
        },
        candidate: {
            moreInformationLink: "Informazio gehiago",
            writeInsPlaceholder: "Idatzi hautagaia hemen",
            blankVote: "Boto zuria",
            preferential: {
                position: "Posizioa",
                none: "Bat ere ez",
                ordinals: {
                    first: ".",
                    second: ".",
                    third: ".",
                    other: ".",
                },
            },
        },
        homeScreen: {
            title: "Sequent Txartel Egiaztatzailea",
            description1:
                "Txartel egiaztatzailea hautesleak kabinan txartela auditatzea aukeratzen duenean erabiltzen da. Egiaztapenak 1-2 minutu iraun beharko luke.",
            description2:
                "Txartel egiaztatzaileak hautesleari aukera ematen dio enkriptatutako txartelak kabinan egindako hautaketak zuzen jasotzen dituela ziurtatzeko. Egiaztapen hau egiteari nahi bezala emandakoaren egiaztagarritasuna deitzen zaio eta txartelaren enkriptazioan akatsak eta jarduera maltzurrak saihesten ditu.",
            descriptionMore: "Gehiago ikasi",
            startButton: "Arakatu fitxategia",
            dragDropOption: "Edo arrastatu eta jaregin hemen",
            importErrorDescription:
                "Arazo bat egon da txartel auditagarria inportatzean. Fitxategi zuzena aukeratu duzu?",
            importErrorMoreInfo: "Informazio gehiago",
            importErrorTitle: "Errorea",
            useSampleText: "Ez duzu txartel auditagarririk?",
            useSampleLink: "Erabili lagin-txartel auditagarri bat",
        },
        confirmationScreen: {
            title: "Sequent Txartel Egiaztatzailea",
            topDescription1: "Inportatutako Txartel Auditagarrian oinarrituta, hau kalkulatu dugu:",
            topDescription2: "Hau Hauteskunde Kabinan erakutsitako Txartelaren IDa bada:",
            bottomDescription1:
                "Zure txartela zuzen enkriptatu da. Orain leiho hau itxi eta Hauteskunde Kabinara itzul zaitezke.",
            bottomDescription2:
                "Bat ez badatoz, egin klik hemen arrazoi posibleei eta har ditzakezun neurriei buruz gehiago jakiteko.",
            ballotChoicesDescription: "Eta zure txartelaren aukerak hauek dira:",
            helpAndFaq: "Laguntza eta Galdera Ohikoenak",
            backButton: "Atzera",
            markedInvalid: "Txartela espresuki baliogabetzat markatuta",
        },
        ballotSelectionsScreen: {
            statusModal: {
                title: "Egoera",
                content: "Egoera panelak egindako egiaztapenei buruzko informazioa ematen dizu.",
                ok: "Ados",
            },
        },
        footer: {
            poweredBy: "Honek bultzatuta: <sequent />",
        },
        errors: {
            encoding: {
                notEnoughChoices: "Ez dago nahikoa aukera deskodetzeko",
                writeInChoiceOutOfRange: "Idatzitako aukera barrutitik kanpo: {{index}}",
                writeInNotEndInZero: "Idatzitakoa ez da 0n amaitzen",
                writeInCharsExceeded:
                    "Idatzitakoak gehienezko karaktere kopurua {{numCharsExceeded}}z gainditu du. Konponketa behar du.",
                bytesToUtf8Conversion:
                    "Errorea idatzitakoa byte-etatik UTF-8 kate-ra bihurtzerakoan: {{errorMessage}}",
                ballotTooLarge: "Bozketa esperotakoa baino handiagoa",
            },
            implicit: {
                selectedMax:
                    "Gainfoto: Hautatutako aukeren kopurua {{numSelected}} gehienezko {{max}} baino gehiago da",
                selectedMin:
                    "Hautatutako aukeren kopurua {{numSelected}} gutxieneko {{min}} baino gutxiago da",
                maxSelectionsPerType:
                    "{{type}} zerrendako hautatutako aukeren kopurua {{numSelected}} gehienezko {{max}} baino gehiago da",
                underVote:
                    "Azpifoto: Hautatutako aukeren kopurua {{numSelected}} gehienezko {{max}} baino gutxiago da",
                overVoteDisabled:
                    "Gehienezkora heldu: Gehienezko {{numSelected}} aukera hautatu dituzu. Zure hautaketa aldatzeko, mesedez ezgaitu beste aukera bat lehenik.",
                blankVote: "Boto Zuria: 0 aukera hautatu",
                preferenceOrderWithGaps:
                    "Boto baliogabea! Lehentasunaren ordenak hutsune bat edo gehiago ditu.",
                duplicatedPosition:
                    "Boto baliogabea! Posizio bera hautatu da bi kandidatu edo gehiagorentzat.",
            },
            explicit: {
                notAllowed:
                    "Bozketa espresuki baliogabe markatu da baina galderak ez du baimentzen",
                alert: "Markatutako hautaketa baliogabeko bototzat hartuko da.",
            },
            configuration: {
                multipleExplicitInvalidCandidates:
                    "Boto-konfigurazio baliogabea: lehiaketak esplizituki baliogabe diren {{count}} hautagai definitzen ditu, baina bakarra onartzen da.",
                multipleExplicitBlankCandidates:
                    "Boto-konfigurazio baliogabea: lehiaketak esplizituki zuri gisa markatutako {{count}} hautagai definitzen ditu, baina bakarra onartzen da.",
            },
        },
        ballotHash: "Zure Txartelaren IDa: {{ballotId}}",
        version: {
            header: "Bertsioa:",
        },
        hash: {
            header: "Hasha:",
        },
        logout: {
            buttonText: "Saioa itxi",
            modal: {
                title: "Ziur zaude saioa itxi nahi duzula?",
                content: "Aplikazio hau ixtear zaude. Ekintza hau ezin da desegin.",
                ok: "Ados",
                close: "Itxi",
            },
        },
        stories: {
            openDialog: "Ireki Elkarrizketa-koadroa",
        },
        dragNDrop: {
            firstLine: "Arrastatu eta jaregin fitxategiak edo",
            browse: "Arakatu",
            format: "Onartutako formatua: txt",
            importError: "Ezin izan da fitxategi hau inportatu. Saiatu berriro.",
        },
        selectElection: {
            electionWebsite: "Txartelaren Webgunea",
            countdown:
                "Hauteskundeak {{years}} urte, {{months}} hilabete, {{weeks}} aste, {{days}} egun, {{hours}} ordu, {{minutes}} minutu, {{seconds}} segundu barru hasiko dira",
            openElection: "Ireki",
            closedElection: "Itxita",
            voted: "Bozkatua",
            notVoted: "Bozkatu gabe",
            resultsButton: "Txartelaren Emaitzak",
            voteButton: "Egin klik Bozkatzeko",
            openDate: "Irekitze-data: ",
            closeDate: "Ixte-data: ",
            ballotLocator: "Aurkitu zure txartela",
        },
        header: {
            profile: "Profila",
            welcome: "Ongi etorri,<br><span>{{name}}</span>",
            session: {
                title: "Zure saioa iraungitzear dago.",
                timeLeft: "{{time}} geratzen zaizu botoa emateko.",
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minutu eta {{time}} segundu",
                timeLeftSeconds: "{{timeLeft}} segundu",
            },
        },
        problems: {
            noProblems: "Ez dago ezer adierazteko. Hau inportatuko litzateke.",
            errors_one: "{{count}} errore",
            errors_other: "{{count}} errore",
            errorsExplained: "Hau ez da inportatuko hauetako bakoitza zuzendu arte.",
            warnings_one: "{{count}} abisu",
            warnings_other: "{{count}} abisu",
            warningsExplained:
                "Hau inportatuko da. Hauetako bakoitza, seguruenik, nahi zena ez den zerbait da.",
            warningsStrict:
                "Modu zorrotza aktibatuta dago, beraz, hauek eraikuntza geldiarazten dute.",
            messages: {
                area: {
                    "cycle": {
                        lead: "Barrutiak bata bestearen barruan",
                        text: 'Barrutiak bata bestearen barruan — "{{area}}" barrutia gurasoen begizta baten parte da, eta horrek Administrazio Ataria blokeatuko luke.',
                    },
                    "duplicate-name": {
                        lead: "Bi barrutik izen bera dute",
                        text: 'Bi barrutik izen bera dute — "{{name}}" aldi berean "{{first}}" eta "{{second}}" da. Hautesleen CSVak barrutia izenaren bidez ebazten du, beraz, hautesleak inportatzaileak lehenik aurkitutakoan geratuko lirateke.',
                    },
                    "early-voting-unknown": {
                        lead: "Aurretiazko botoaren ezarpen ezezaguna",
                        text: 'Aurretiazko botoaren ezarpen ezezaguna — "{{value}}" ez da baliozkoa barruti batentzat. Plataformak {{allowed}} onartzen ditu.',
                    },
                    "empty-ballot": {
                        lead: "Boto-txartel hutsa duen barrutia",
                        text: 'Boto-txartel hutsa duen barrutia — "{{area}}" barrutiak ez du bozketa batean ere bozkatzen, ez berean ez barruan duen barruti batetik heredatutakoan, beraz, bere hautesleek boto-txartel hutsa ikusiko lukete.',
                    },
                    "inside-itself": {
                        lead: "Barrutia bere barruan",
                        text: "Barrutia bere barruan — barruti bat ezin da bere buruaren guraso izan.",
                    },
                    "no-identifier": {
                        lead: "Barrutia identifikatzailerik gabe",
                        text: "Barrutia identifikatzailerik gabe — barruti orok behar du bat.",
                    },
                    "no-name": {
                        lead: "Barrutia izenik gabe",
                        text: "Barrutia izenik gabe — hautesleen CSVak hautesle baten barrutia izenaren bidez identifikatzen du, ez IDaren bidez, beraz, izenik gabeko barruti batean ezin da hautesle bat ere jarri.",
                    },
                    "parent-missing": {
                        lead: "Barruti gurasoa falta da",
                        text: "Barruti gurasoa falta da — barrutia fitxategi honetan ez dagoen guraso baten barruan dago.",
                    },
                    "parent-unknown": {
                        lead: "Barruti gurasoa ezezaguna",
                        text: 'Barruti gurasoa ezezaguna — ezerk ez du "{{parent}}" identifikatzailea.',
                    },
                },
                ballot: {
                    "no-elections": {
                        lead: "Hauteskunderik ez",
                        text: "Hauteskunderik ez — hauteskunde-gertaera batek gutxienez bat behar du.",
                    },
                },
                bundle: {
                    "duplicate-id": {
                        lead: "Identifikatzailea bi aldiz",
                        text: "Identifikatzailea bi aldiz — {{kind}} barruko {{id}} {{previous}}(e)k ere erabiltzen du.",
                    },
                    "event-no-encryption": {
                        lead: "Zifratze-protokolorik ez",
                        text: "Zifratze-protokolorik ez — hauteskunde-gertaerak ez du esaten nola zifratzen diren boto-txartelak.",
                    },
                    "event-no-id": {
                        lead: "Gertaera IDrik gabe",
                        text: "Gertaera IDrik gabe — fitxategi honetako hauteskunde-gertaerak ez du identifikatzailerik.",
                    },
                    "no-areas": {
                        lead: "Barrutirik ez",
                        text: "Barrutirik ez — ezin zaio hautesle bati boto-txartelik eman gertaerak gutxienez barruti bat izan arte.",
                    },
                    "no-elections": {
                        lead: "Hauteskunderik ez",
                        text: "Hauteskunderik ez — hauteskunde-gertaera batek gutxienez hauteskunde bat behar du.",
                    },
                    "no-tenant": {
                        lead: "Tenantik ez",
                        text: "Tenantik ez — fitxategiak ez du esaten zein tenantena den.",
                    },
                    "tenant-not-uuid": {
                        lead: "Tenanta ez da identifikatzaile bat",
                        text: 'Tenanta ez da identifikatzaile bat — "{{value}}" ez da tenant ID baliozkoa.',
                    },
                },
                candidate: {
                    "contest-missing": {
                        lead: "Hautagaiaren bozketa falta da",
                        text: "Hautagaiaren bozketa falta da — hautagaia fitxategi honetan ez dagoen bozketa batena da.",
                    },
                    "no-contest": {
                        lead: "Hautagaia bozketarik gabe",
                        text: "Hautagaia bozketarik gabe — hautagai orok bozketa batena izan behar du.",
                    },
                    "picture-mismatch": {
                        lead: "Irudiak beste leku batera seinalatzen du",
                        text: 'Irudiak beste leku batera seinalatzen du — boto-txartelak "{{url}}" erakusten du, eta horrek ez du ondoan duen "{{document}}" dokumentua izendatzen. Inportatu ondoren, biek fitxategi desberdinetara seinalatuko lukete.',
                    },
                    "picture-not-shown": {
                        lead: "Inoiz erakusten ez den irudia",
                        text: "Inoiz erakusten ez den irudia — hautagai batek boto-txarteleko sarrera batek ere seinalatzen ez duen irudi bat aipatzen du, beraz, igo egingo litzateke eta ez litzateke inoiz erakutsiko.",
                    },
                    "picture-unrecorded": {
                        lead: "Irudia erregistratu gabe",
                        text: "Irudia erregistratu gabe — hautagai baten irudia boto-txartelean dago eta ezerk ez du erregistratzen zein dokumentu den, beraz, gero ezin izango da plataforman aldatu edo kendu.",
                    },
                },
                census: {
                    "column-not-declared": {
                        lead: "Errolda-zutabea deklaratu gabe",
                        text: "Errolda-zutabea deklaratu gabe — {{message}}",
                    },
                    "no-area-column": {
                        lead: "Erroldak ez du barrutirik aipatzen",
                        text: "Erroldak ez du barrutirik aipatzen — hauteskunde honek barrutiak ditu, baina hautesle guztiek boto-txartel lehenetsia jasoko lukete. Barrutien banaketa aplikatu behar bada, erroldak barruti-zutabe bat behar du.",
                    },
                    "note": {
                        lead: "Erroldari buruz",
                        text: "Erroldari buruz — {{message}}",
                    },
                    "unreadable": {
                        lead: "Errolda ezin da irakurri",
                        text: "Errolda ezin da irakurri — {{message}}",
                    },
                    "unreadable-member": {
                        lead: "Zip barruko errolda ezin da irakurri",
                        text: "Zip barruko errolda ezin da irakurri — {{message}}",
                    },
                },
                channels: {
                    "early-voting-closed": {
                        lead: "Aurretiazko botoa desaktibatuta",
                        text: "Aurretiazko botoa desaktibatuta — {{areas}}(e)k aurretiazko botoa baimentzen dute eta gertaerak ez du kanal hori irekitzen, beraz, ezarpenak ez du ezer egiten.",
                    },
                    "early-voting-no-area": {
                        lead: "Aurretiazko botoa barrutirik gabe",
                        text: "Aurretiazko botoa barrutirik gabe — aurretiazko botoa irekita dago eta barruti batek ere ez du baimentzen, beraz, aurretiazko aldiak ez luke hautesle bat ere izango.",
                    },
                    "kiosk-client": {
                        lead: "Kioskoak bere bezeroa behar du",
                        text: "Kioskoak bere bezeroa behar du — kioskoko bozketak autentifikazio-bezero bat behar du, bezero arruntaren izena eta amaieran '-kiosk' dituena, eta fitxategi honek ez du sortzen.",
                    },
                    "none-open": {
                        lead: "Bozkatzeko modurik ez",
                        text: "Bozkatzeko modurik ez — bozketa-kanal guztiak desaktibatuta daude, beraz, inork ezin du bozkatu.",
                    },
                    "telephone-elsewhere": {
                        lead: "Telefono bidezko botoa geroago konfiguratzen da",
                        text: "Telefono bidezko botoa geroago konfiguratzen da — telefono bidezko botoa gertaeraren IVR fitxan konfiguratzen da inportatu ondoren; horietako ezer ez dago fitxategi honetan.",
                    },
                    "unknown": {
                        lead: "Bozkatzeko modu ezezaguna",
                        text: 'Bozkatzeko modu ezezaguna — ezerk ez du "{{name}}" irakurtzen. Plataformak kontuan hartzen dituen bozkatzeko moduak {{allowed}} dira.',
                    },
                },
                contacts: {
                    none: {
                        lead: "Harremanetarako punturik ez",
                        text: "Harremanetarako punturik ez — hauteskunde egunean hauei deitzen zaie.",
                    },
                },
                contest: {
                    "algorithm-unknown": {
                        lead: "Zenbaketa-metodo ezezaguna",
                        text: 'Zenbaketa-metodo ezezaguna — "{{value}}" ez da zenbaketa-algoritmo bat. Plataformak {{allowed}} onartzen ditu.',
                    },
                    "area-unknown": {
                        lead: "Bozketaren barrutia ezezaguna",
                        text: 'Bozketaren barrutia ezezaguna — ezerk ez du "{{area}}" identifikatzailea.',
                    },
                    "cap-invalid": {
                        lead: "Hautapen-muga ezinezkoa",
                        text: "Hautapen-muga ezinezkoa — {{cap}} ez da hautapen kopuru bat.",
                    },
                    "cap-never-applies": {
                        lead: "Hautapen-muga ez da inoiz aplikatzen",
                        text: "Hautapen-muga ez da inoiz aplikatzen — motako {{cap}}(e)ko muga ez da inoiz aplikatzen hautesle batek guztira {{max}} aukera ditzakeen bozketa batean.",
                    },
                    "chooses-more-than-available": {
                        lead: "Hautagaiak baino hautapen gehiago",
                        text: "Hautagaiak baino hautapen gehiago — hautesle batek {{max}} aukera ditzake {{available}} hautagairen artean.",
                    },
                    "chooses-more-than-offered": {
                        lead: "Aukerak baino hautapen gehiago",
                        text: "Aukerak baino hautapen gehiago — hautesle batek {{chosen}} arte aukera ditzake, baina {{offered}} bakarrik daude aukeratzeko.",
                    },
                    "columns-invalid": {
                        lead: "Diseinu ezinezkoa",
                        text: "Diseinu ezinezkoa — {{columns}} zutabe ez da diseinu bat.",
                    },
                    "columns-too-many": {
                        lead: "Zutabe gehiegi",
                        text: "Zutabe gehiegi — {{columns}} zutabe ezin izango dira irakurri telefono mugikor batean, eta hautesle gehienek horrela bozkatzen dute.",
                    },
                    "count-missing": {
                        lead: "Bozketari zenbaki bat falta zaio",
                        text: "Bozketari zenbaki bat falta zaio — bozketak {{field}} behar du.",
                    },
                    "count-negative": {
                        lead: "Kopuru negatiboa",
                        text: "Kopuru negatiboa — {{field}} {{value}} da, eta kopuru bat ezin da zero baino txikiagoa izan.",
                    },
                    "election-missing": {
                        lead: "Bozketaren hauteskundea falta da",
                        text: "Bozketaren hauteskundea falta da — bozketa fitxategi honetan ez dagoen hauteskunde batena da.",
                    },
                    "elects-more-than-available": {
                        lead: "Hautagaiak baino eserleku gehiago",
                        text: "Hautagaiak baino eserleku gehiago — bozketak {{winners}} hautatzen ditu {{available}} hautagairen artean.",
                    },
                    "elects-more-than-chosen": {
                        lead: "Baimendutakoa baino gehiago hautatzen du",
                        text: "Baimendutakoa baino gehiago hautatzen du — bozketak {{winners}} hautatzen ditu, baina hautesle batek {{chosen}} bakarrik aukera ditzake.",
                    },
                    "elects-more-than-standing": {
                        lead: "Hautagaiak baino eserleku gehiago",
                        text: "Hautagaiak baino eserleku gehiago — bozketak {{winners}} hautatzen ditu {{standing}} hautagairen artean.",
                    },
                    "elects-nobody": {
                        lead: "Ez du inor hautatzen",
                        text: "Ez du inor hautatzen — bozketak ez du irabazlerik.",
                    },
                    "max-votes-below-one": {
                        lead: "Ez dago ezer bozkatzeko",
                        text: "Ez dago ezer bozkatzeko — hautesle batek hautagai bat baino gutxiago aukera ditzake.",
                    },
                    "min-above-max": {
                        lead: "Gutxienekoa gehienekoaren gainetik",
                        text: "Gutxienekoa gehienekoaren gainetik — hautesle batek gutxienez {{min}} aukeratu behar ditu, baina gehienez {{max}} aukera ditzake.",
                    },
                    "no-candidates": {
                        lead: "Hautagairik ez",
                        text: "Hautagairik ez — oraindik ez da inor aurkezten bozketa honetan.",
                    },
                    "no-candidates-in-bundle": {
                        lead: "Hautagairik ez",
                        text: "Hautagairik ez — bozketak ez du hautagairik, beraz, inork ezin du bertan bozkatu.",
                    },
                    "on-no-ballot": {
                        lead: "Bozketa ez dago boto-txartel batean ere",
                        text: "Bozketa ez dago boto-txartel batean ere — barruti baten boto-txartelak ere ez du bozketa hau barne hartzen, beraz, inork ezin du bertan bozkatu.",
                    },
                    "policy-not-text": {
                        lead: "Boto-txartelaren araua ez da testua",
                        text: "Boto-txartelaren araua ez da testua — {{key}} testua izan beharko litzateke, eta {{value}} da.",
                    },
                    "policy-unknown": {
                        lead: "Boto-txartelaren arau ezezaguna",
                        text: 'Boto-txartelaren arau ezezaguna — "{{value}}" ez da {{key}}(r)entzako balio baliozkoa. Plataformak {{allowed}} onartzen ditu.',
                    },
                    "ranked-counted-unranked": {
                        lead: "Boto-txartel ordenatua ordenarik gabe zenbatua",
                        text: 'Boto-txartel ordenatua ordenarik gabe zenbatua — hobespenezko bozketa bat "{{algorithm}}" bidez zenbatzen da, eta horrek ez ditu kontuan hartzen hautesleek ematen dituzten ordenak.',
                    },
                    "tie-breaking-unknown": {
                        lead: "Berdinketa hausteko arau ezezaguna",
                        text: 'Berdinketa hausteko arau ezezaguna — "{{value}}" ez da berdinketa hausteko politika bat. Plataformak {{allowed}} onartzen ditu.',
                    },
                    "unranked-counted-ranked": {
                        lead: "Boto-txartel arrunta ordenaren arabera zenbatua",
                        text: 'Boto-txartel arrunta ordenaren arabera zenbatua — hobespenik gabeko bozketa bat "{{algorithm}}" bidez zenbatzen da, eta horrek boto-txartel ordenatuak behar ditu.',
                    },
                    "voting-type-unknown": {
                        lead: "Bozketa-mota ezezaguna",
                        text: 'Bozketa-mota ezezaguna — "{{value}}" ez da bozketa-mota bat. Plataformak {{allowed}} onartzen ditu.',
                    },
                    "write-in-slots-not-allowed": {
                        lead: "Idatzizko botorako tarteak ez daude baimenduta",
                        text: "Idatzizko botorako tarteak ez daude baimenduta — idatzizko botorako {{count}} tarte daude idatzizko botoa baimentzen ez duen bozketa batean, eta horrek izenik gabeko aukerak jartzen ditu boto-txartelean.",
                    },
                    "write-ins-no-slot": {
                        lead: "Idatzizko botoa idazteko lekurik gabe",
                        text: "Idatzizko botoa idazteko lekurik gabe — idatzizko hautagaiak baimenduta daude eta bozketak ez du horretarako tarterik, beraz, hautesle batek ez du izen bat idazteko lekurik.",
                    },
                },
                delivery: {
                    "no-importable": {
                        lead: "Artxibo inportagarririk ez",
                        text: "Artxibo inportagarririk ez — zip honek plan bat du, baina ez Administrazio Atariak inportatzen duen artxiboa, beraz, ez ditu errolda eta hark aipatzen dituen fitxategiak.",
                    },
                },
                design: {
                    "no-stable-key": {
                        lead: "Gakorik gabeko boto-txartel diseinua",
                        text: "Gakorik gabeko boto-txartel diseinua — {{kind}} {{id}} elementuak ez du izenik ez kanpoko IDrik, beraz, bere boto-txartel diseinuak ezin dira ezagutu inportazio baten ondoren.",
                    },
                    "unreadable-style": {
                        lead: "Boto-txartel estiloa ezin da irakurri",
                        text: "Boto-txartel estiloa ezin da irakurri — ezin izan da plataformaren boto-txartel estiloa irakurri bere diseinuaren laburpena kalkulatzeko: {{reason}}",
                    },
                },
                election: {
                    "channels-differ": {
                        lead: "Hauteskundea eta gertaera ez datoz bat",
                        text: "Hauteskundea eta gertaera ez datoz bat — {{channels}} gertaerarenaren desberdina da, beraz, hauteskunde honen hasiera-kontrolak ez lirateke bat etorriko.",
                    },
                    "grace-disallowed": {
                        lead: "Grazia-aldia desaktibatuta",
                        text: "Grazia-aldia desaktibatuta — {{seconds}} segundoko grazia ezarrita dago eta ez da grazia-aldirik baimentzen, beraz, bozketa epemugan ixten da.",
                    },
                    "grace-negative": {
                        lead: "Grazia-aldi negatiboa",
                        text: "Grazia-aldi negatiboa — {{value}} segundo ez da denbora-tarte bat.",
                    },
                    "grace-zero": {
                        lead: "Iraupenik gabeko grazia-aldia",
                        text: "Iraupenik gabeko grazia-aldia — grazia-aldi bat baimenduta dago eta zero segundo irauten du, beraz, ez dago batere.",
                    },
                    "no-contests": {
                        lead: "Bozketarik ez",
                        text: "Bozketarik ez — inork ez du bozkatzen hauteskunde honetan.",
                    },
                    "revotes-negative": {
                        lead: "Boto kopuru ezinezkoa",
                        text: "Boto kopuru ezinezkoa — {{value}} ez da hautesle batek bozka dezakeen aldi kopuru bat.",
                    },
                    "setting-unknown": {
                        lead: "Hauteskundearen ezarpen ezezaguna",
                        text: 'Hauteskundearen ezarpen ezezaguna — "{{value}}" ez da {{key}}(r)entzako balio baliozkoa. Plataformak {{allowed}} onartzen ditu.',
                    },
                    "spoil-without-revote": {
                        lead: "Baliogabetzea bigarren aukerarik gabe",
                        text: "Baliogabetzea bigarren aukerarik gabe — hautesle batek emandako boto-txartel bat baztertu dezake eta ez du hura ordezkatzeko bigarren saiakerarik.",
                    },
                },
                event: {
                    "no-identifier": {
                        lead: "Identifikatzailerik ez",
                        text: "Identifikatzailerik ez — sortutako ID guztiak horretatik eratortzen dira, beraz, bat gabe ezin da ezer bi aldiz modu berean eraiki.",
                    },
                    "no-name": {
                        lead: "Izenik ez",
                        text: "Izenik ez — hautesleek boto-txartelaren gainean ikusten dute, eta saioa hasteko orriaren izenburu bihurtzen da.",
                    },
                    "setting-unknown": {
                        lead: "Gertaeraren ezarpen ezezaguna",
                        text: 'Gertaeraren ezarpen ezezaguna — "{{value}}" ez da {{key}}(r)entzako balio baliozkoa. Plataformak {{allowed}} onartzen ditu.',
                    },
                },
                file: {
                    "cannot-decrypt": {
                        lead: "Ezin izan da deszifratu",
                        text: "Ezin izan da deszifratu — fitxategia zifratuta dago eta ez da ireki. Egiaztatu pasahitza.",
                    },
                    "checksum-mismatch": {
                        lead: "Ez da espero zen fitxategia",
                        text: "Ez da espero zen fitxategia — bere SHA-256 ez dator bat emandakoarekin, beraz, ez da nahi zen fitxategia. Egiaztatu kontrol-batura edo igo fitxategia berriro.",
                    },
                    "duplicate-name": {
                        lead: "Fitxategi-izena bi aldiz",
                        text: 'Fitxategi-izena bi aldiz — "{{file}}" izenak bi gauza desberdin izendatzen ditu. Fitxategiak izenaren arabera bidaiatzen dute, beraz, batek bestea ordezkatuko luke isilean.',
                    },
                    "missing": {
                        lead: "Fitxategia falta da",
                        text: 'Fitxategia falta da — "{{file}}" aipatzen da eta hemen ez dago ezer hura duenik. Artxiboko sarrera huts batek inportazioa huts eginarazten du, fitxategi bat galdu beharrean.',
                    },
                    "not-a-bundle": {
                        lead: "Ez da hauteskunde-gertaera baten esportazioa",
                        text: "Ez da hauteskunde-gertaera baten esportazioa — fitxategia irakurri da, baina ez du horren forma: {{reason}}",
                    },
                    "not-json": {
                        lead: "Ez da fitxategi irakurgarria",
                        text: "Ez da fitxategi irakurgarria — hau ez da plataformak irakur dezakeen hauteskunde-gertaera baten esportazioa: {{reason}}",
                    },
                    "unreadable-archive": {
                        lead: "Artxiboa ezin da irakurri",
                        text: "Artxiboa ezin da irakurri — {{reason}}",
                    },
                    "unused": {
                        lead: "Erabili gabeko fitxategia",
                        text: 'Erabili gabeko fitxategia — "{{file}}" eman da eta ezerk ez du aipatzen, beraz, entregan bidaiatuko luke eta inori ez litzaioke erakutsiko.',
                    },
                    "version-incompatible": {
                        lead: "Beste bertsio batekin esportatua",
                        text: "Beste bertsio batekin esportatua — fitxategia {{found}} bertsiotik dator, eta {{current}} bertsioak ezin du inportatu.",
                    },
                },
                identifier: {
                    duplicate: {
                        lead: "Identifikatzailea bi aldiz",
                        text: 'Identifikatzailea bi aldiz — "{{identifier}}" {{first}}(e)k erabiltzen du dagoeneko. Identifikatzaileak bakarrak dira hauteskunde-gertaera osoan, beraz, bigarrenak lehena ordezkatzen du, gehitu beharrean.',
                    },
                },
                ivr: {
                    "language-not-spoken": {
                        lead: "Hizkuntza ez dago telefonoz eskuragarri",
                        text: "Hizkuntza ez dago telefonoz eskuragarri — deiak ingelesez, frantsesez eta gaztelaniaz baino ez du hitz egiten, beraz {{languages}} ez zaie deitzaileei eskaintzen.",
                    },
                    "missing-prompts": {
                        lead: "Deiaren mezuak falta dira",
                        text: "Deiaren mezuak falta dira — {{prompts}} ez dute testurik «{{language}}» hizkuntzan, eta sistema telefonikoak dei guztiak baztertzen ditu izan arte.",
                        lead_one: "Deiaren mezu bat falta da",
                        text_one:
                            "Deiaren mezu bat falta da — {{prompts}} ez du testurik «{{language}}» hizkuntzan, eta sistema telefonikoak dei guztiak baztertzen ditu izan arte.",
                    },
                },
                labels: {
                    "in-use": {
                        lead: "Baimen-etiketak erabilita",
                        text: "Baimen-etiketak erabilita — {{labels}}. Etiketa bat duen guztia ezkutatuta geratzen da etiketa hori ez duen administratzaile ororentzat, beraz, hau inportatzen duenak horietako bat behar du bere kontuan; bestela, Administrazio Atariak zerrenda hutsa erakutsiko dio.",
                    },
                },
                languages: {
                    "default-not-offered": {
                        lead: "Hizkuntza lehenetsia ez dago eskainita",
                        text: 'Hizkuntza lehenetsia ez dago eskainita — "{{chosen}}" ez dago {{offered}} artean, beraz, hautesleek lehenengoa jasoko lukete.',
                    },
                    "detection-unknown": {
                        lead: "Hizkuntza-detekzio ezezaguna",
                        text: 'Hizkuntza-detekzio ezezaguna — "{{policy}}" ez da plataformak ezagutzen duen politika bat.',
                    },
                    "none": {
                        lead: "Hizkuntzarik ez",
                        text: "Hizkuntzarik ez — boto-txartelak ingelesera jotzen du, eta hori segurtasun-sare bat da, ez aukera bat.",
                    },
                },
                link: {
                    "area-missing": {
                        lead: "Falta den barruti baterako boto-txartel esteka",
                        text: "Falta den barruti baterako boto-txartel esteka — bozketa bat fitxategi honetan ez dagoen barruti baten boto-txartelean jartzen da.",
                    },
                    "contest-missing": {
                        lead: "Falta den bozketa baterako boto-txartel esteka",
                        text: "Falta den bozketa baterako boto-txartel esteka — barruti baten boto-txartelak fitxategi honetan ez dagoen bozketa bat zerrendatzen du.",
                    },
                },
                logo: {
                    "file-and-link": {
                        lead: "Bi logotipo",
                        text: 'Bi logotipo — plan honek igotako logotipo bat ("{{file}}") eta esteka bat ("{{url}}") ditu. Fitxategia bidaltzen da; esteka ez da kontuan hartzen.',
                    },
                    "file-missing": {
                        lead: "Logotipoaren fitxategia falta da",
                        text: 'Logotipoaren fitxategia falta da — "{{file}}" logotipo gisa adierazten da eta ez da eman. Jarri fitxategia kalkulu-orriaren ondoan, izen hori zehatz-mehatz erabiliz.',
                    },
                    "no-bytes": {
                        lead: "Logotipoaren fitxategia hutsik",
                        text: 'Logotipoaren fitxategia hutsik — "{{file}}" logotipo gisa adierazten da eta ez du byterik. Artxiboko sarrera huts batek inportazioa huts eginarazten du, irudi bat galdu beharrean.',
                    },
                },
                material: {
                    "empty-document": {
                        lead: "Dokumentu hutsa duen materiala",
                        text: "Dokumentu hutsa duen materiala — ezerezerako esteka gisa inportatuko litzateke. Fitxategirik ez duen material baterako, utzi dokumentua guztiz kanpo.",
                    },
                    "file-missing": {
                        lead: "Materialaren fitxategia falta da",
                        text: 'Materialaren fitxategia falta da — "{{file}}" hemen aipatzen da eta ez da eman. Jarri fitxategia kalkulu-orriaren ondoan, izen hori zehatz-mehatz erabiliz.',
                    },
                    "file-unused": {
                        lead: "Erabili gabeko material-fitxategia",
                        text: 'Erabili gabeko material-fitxategia — "{{file}}" eman da eta errenkada batek ere ez du aipatzen, beraz, igo egingo litzateke eta inori ez litzaioke erakutsiko.',
                    },
                    "no-identifier": {
                        lead: "Materiala identifikatzailerik gabe",
                        text: "Materiala identifikatzailerik gabe — bere dokumentuaren IDa horretatik eratortzen da, beraz, bat gabe fitxategia ezin da errenkadarekin lotu.",
                    },
                    "tab-off": {
                        lead: "Materialen fitxa desaktibatuta",
                        text: "Materialen fitxa desaktibatuta — fitxategi honetan {{count}} laguntza-material daude eta haiek erakusten dituen fitxa desaktibatuta dago, beraz, hautesleek ez lituzkete inoiz ikusiko.",
                    },
                    "wrong-event": {
                        lead: "Beste gertaera bateko materiala",
                        text: "Beste gertaera bateko materiala — laguntza-material hau beste hauteskunde-gertaera batena da.",
                    },
                },
                messages: {
                    "weekly-no-time": {
                        lead: "Ordurik gabeko errepikapena",
                        text: "Ordurik gabeko errepikapena — mezu bat astero errepikatzen da, baina ez du esaten zer ordutan, beraz, bidaltzen duenak inork idatzi ez duen ordu bat aukeratu beharko du.",
                    },
                },
                passwords: {
                    "column-already-there": {
                        lead: "Pasahitzak bi aldiz emanda",
                        text: "Pasahitzak bi aldiz emanda — erroldak badu dagoeneko pasahitz-zutabe bat, beraz, sortutako pasahitzak galdera berari emandako bigarren erantzuna lirateke. Kendu zutabea edo desaktibatu sorkuntza.",
                    },
                    "no-characters": {
                        lead: "Karaktererik gabeko pasahitzak",
                        text: "Karaktererik gabeko pasahitzak — sortutako pasahitzek gutxienez karaktere mota bat behar dute osatzeko.",
                    },
                    "no-seed": {
                        lead: "Hazirik gabeko pasahitzak",
                        text: "Hazirik gabeko pasahitzak — haziak eragiten du berreraikuntza batek pasahitz berak sortzea, berriak sortu beharrean.",
                    },
                },
                package: {
                    "already-imported": {
                        lead: "Dagoeneko inportatuta",
                        text: "Dagoeneko inportatuta — konfigurazio honen {{revision}} berrikuspena lehenago inportatu zen; inportatu berrikuspen berriago bat.",
                    },
                    "approval-invalid": {
                        lead: "Onarpenak ez du balio",
                        text: "Onarpenak ez du balio — ezin izan da {{name}} pertsonaren onarpena egiaztatu: {{reason}}",
                    },
                    "approval-repeated": {
                        lead: "Pertsona berak bi aldiz onartu du",
                        text: "Pertsona berak bi aldiz onartu du — {{name}} pertsonak behin baino gehiagotan onartu du, eta behin bakarrik zenbatzen da.",
                    },
                    "approver-key-usage": {
                        lead: "Onartzaileak ezin du sinatu",
                        text: "Onartzaileak ezin du sinatu — onartzaile baten ziurtagiria ez dago sinatzeko egina.",
                    },
                    "bad-signature": {
                        lead: "Sinadura ez dator bat",
                        text: "Sinadura ez dator bat — paketearen sinadura ez da egiaztatzen, beraz, sinatu ondoren aldatu zen edo beste gako batek sinatu zuen: {{reason}}",
                    },
                    "content-digest": {
                        lead: "Edukiaren laburpena ez dator bat",
                        text: "Edukiaren laburpena ez dator bat — manifestuak {{expected}} dio, eta bere edukiaren hasha {{actual}} da.",
                    },
                    "duplicate-member": {
                        lead: "Fitxategi-izena bi aldiz",
                        text: 'Fitxategi-izena bi aldiz — "{{file}}" bi aldiz agertzen da {{archive}} artxiboan, beraz, bi irakurlek fitxategi desberdinak har litzakete.',
                    },
                    "file-changed": {
                        lead: "Sinatu ondoren aldatua",
                        text: "Sinatu ondoren aldatua — {{file}} fitxategiaren SHA-256 {{actual}} da, eta manifestuak {{expected}} dio. Ez da paketeko ezer irakurri.",
                    },
                    "file-extra": {
                        lead: "Fitxategia ez dago manifestuan",
                        text: "Fitxategia ez dago manifestuan — {{file}} paketean dago, baina ez zen sinatu. Ez da paketeko ezer irakurri.",
                    },
                    "file-missing": {
                        lead: "Sinatutako fitxategia falta da",
                        text: "Sinatutako fitxategia falta da — {{file}} manifestuan dago eta ez paketean. Ez da paketeko ezer irakurri.",
                    },
                    "invalid-time": {
                        lead: "Ez da data eta ordu bat",
                        text: 'Ez da data eta ordu bat — manifestuko "{{value}}" ez da data eta ordu bat.',
                    },
                    "member-too-large": {
                        lead: "Fitxategia handiegia da",
                        text: 'Fitxategia handiegia da — {{archive}} artxiboko "{{file}}" fitxategiak, deskonprimituta, fitxategi batek izan ditzakeen {{limit}} byteak baino gehiago hartzen ditu.',
                    },
                    "nested-too-deep": {
                        lead: "Zip gehiegi habiaratuta",
                        text: 'Zip gehiegi habiaratuta — "{{file}}" fitxategi bat habiara daitekeen {{limit}} zipak baino gehiagoren barruan dago.',
                    },
                    "no-importable": {
                        lead: "Ez dago inportatzeko ezer",
                        text: "Ez dago inportatzeko ezer — paketeak ez du official_election_setup.zip, inportatzaileak irakurtzen duen artxiboa.",
                    },
                    "report-template-changed": {
                        lead: "Txostenaren txantiloia aldatu da",
                        text: "Txostenaren txantiloia aldatu da — {{report}} txostenaren txantiloia ez da onartutakoa: bere laburpena {{actual}} da, eta sinatutako konfigurazioak {{expected}} dio.",
                    },
                    "report-template-missing": {
                        lead: "Txostenaren txantiloia falta da",
                        text: 'Txostenaren txantiloia falta da — {{report}} txostena "{{template}}" txantiloiarekin sortzen da, eta txantiloi hori ez dago konfigurazioan; beraz, haren diseinua ezin da sinatu.',
                    },
                    "report-unreadable": {
                        lead: "Txostena ezin da sinatu",
                        text: "Txostena ezin da sinatu — {{message}}",
                    },
                    "revoked-approver": {
                        lead: "Onartzailearen ziurtagiria baliogabetuta",
                        text: "Onartzailearen ziurtagiria baliogabetuta — onartzaile baten ziurtagiria baliogabetu da, beraz, onarpenak ez du balio.",
                    },
                    "revoked-signer": {
                        lead: "Sinatzeko gakoa baliogabetuta",
                        text: "Sinatzeko gakoa baliogabetuta — pakete hau sinatu zuen gakoa baliogabetu da, eta bere paketeak baztertzen dira.",
                    },
                    "rollback": {
                        lead: "Ez da berrikuspen berriagoa",
                        text: "Ez da berrikuspen berriagoa — {{revision}} berrikuspena ez da {{last}} berrikuspena baino berriagoa, inportatutako azkena.",
                    },
                    "signed-in-the-future": {
                        lead: "Etorkizunean sinatua",
                        text: "Etorkizunean sinatua — paketeak dio {{at}} unean sinatu zela, eta orain {{now}} da.",
                    },
                    "signer-key-usage": {
                        lead: "Sinatzeko gakoak ezin du sinatu",
                        text: "Sinatzeko gakoak ezin du sinatu — pakete hau sinatu zuen gakoaren ziurtagiria ez dago sinatzeko egina.",
                    },
                    "too-few-approvals": {
                        lead: "Onarpen gutxiegi",
                        text: "Onarpen gutxiegi — pertsona desberdinen {{count}} onarpen baliodun, eta {{required}} behar dira.",
                    },
                    "too-large": {
                        lead: "Paketea handiegia da",
                        text: 'Paketea handiegia da — deskonprimituta, pakete batek izan ditzakeen {{limit}} byteak baino gehiago hartzen ditu: {{archive}} artxiboko "{{file}}" fitxategiak muga gainditzen du.',
                    },
                    "too-many-members": {
                        lead: "Fitxategi gehiegi",
                        text: "Fitxategi gehiegi — {{archive}} artxiboak pakete batek izan ditzakeen {{limit}} fitxategiak baino gehiago ditu.",
                    },
                    "unhashable-content": {
                        lead: "Ezin da edukiaren hasha kalkulatu",
                        text: "Ezin da edukiaren hasha kalkulatu — ezin izan da konfigurazioaren edukia idatzi hasha kalkulatzeko: {{reason}}",
                    },
                    "unknown-format": {
                        lead: "Manifestu-formatu ezezaguna",
                        text: 'Manifestu-formatu ezezaguna — manifestua "{{format}}" formatuan dago, eta bertsio honek ezin du irakurri.',
                    },
                    "unreadable-chain": {
                        lead: "Sinatzailearen ziurtagiriak ezin dira irakurri",
                        text: "Sinatzailearen ziurtagiriak ezin dira irakurri — ezin izan da paketearen ziurtagiri-katea irakurri: {{reason}}",
                    },
                    "unreadable-manifest": {
                        lead: "Manifestua ezin da irakurri",
                        text: "Manifestua ezin da irakurri — ezin izan da paketearen manifestua irakurri: {{reason}}",
                    },
                    "unreadable-revocation-list": {
                        lead: "Baliogabetze-zerrenda ezin da irakurri",
                        text: "Baliogabetze-zerrenda ezin da irakurri — ezin izan da baliogabetze-zerrenda bat irakurri; beraz, ezin da aplikatu: {{reason}}",
                    },
                    "unreadable-trust": {
                        lead: "Ziurtagiri fidagarriak ezin dira irakurri",
                        text: "Ziurtagiri fidagarriak ezin dira irakurri — ezin izan da {{setting}} ezarpena irakurri: {{reason}}",
                    },
                    "unreadable-zip": {
                        lead: "Artxiboa ezin da irakurri",
                        text: "Artxiboa ezin da irakurri — ezin izan da {{archive}} zip gisa irakurri: {{reason}}",
                    },
                    "unsigned": {
                        lead: "Paketea ez dago sinatuta",
                        text: "Paketea ez dago sinatuta — ez du {{missing}}, eta instalazio honek sinatutako paketeak bakarrik inportatzen ditu.",
                    },
                    "untrusted-approver": {
                        lead: "Onartzailea ez da fidagarria",
                        text: "Onartzailea ez da fidagarria — onartzaile baten ziurtagiria ez da fidagarria: {{reason}}",
                    },
                    "untrusted-signer": {
                        lead: "Sinatzailea ez da fidagarria",
                        text: "Sinatzailea ez da fidagarria — pakete hau sinatu zuen gakoa ez da instalazio honek fidagarritzat duenetako bat: {{reason}}",
                    },
                    "unwritable-manifest": {
                        lead: "Manifestua ezin da idatzi",
                        text: "Manifestua ezin da idatzi — ezin izan da manifestua idatzi: {{reason}}",
                    },
                },
                plan: {
                    "not-a-plan": {
                        lead: "Ez da hauteskunde-plan bat",
                        text: "Ez da hauteskunde-plan bat — fitxategia ezin izan da plan gisa irakurri: {{error}}",
                    },
                    "saved-by-newer-version": {
                        lead: "Bertsio berriago batekin gordea",
                        text: "Bertsio berriago batekin gordea — {{saved}}, {{supported}} bertsioaren aurrean. Hemen irekitzeak bertsio horrek gehitutakoa isilean baztertuko luke.",
                    },
                    "unreadable": {
                        lead: "Plana ezin da irakurri",
                        text: "Plana ezin da irakurri — {{error}}",
                    },
                },
                reports: {
                    "duplicate": {
                        lead: "Txostena bi aldiz ezarria",
                        text: "Txostena bi aldiz ezarria — {{report}} txostena behin baino gehiagotan ezarri da hauteskunde berarentzat.",
                    },
                    "no-copies": {
                        lead: "Kopiarik ez",
                        text: "Kopiarik ez — {{report}} txostena kopiarik ez inprimatzeko ezarrita dago. Ezarri gutxienez bat.",
                    },
                    "unknown-election": {
                        lead: "Hauteskunde ezezaguna",
                        text: 'Hauteskunde ezezaguna — {{report}} txostena "{{election}}" hauteskundeari buruzkoa da, eta plan honetan ez dago hauteskunde hori.',
                    },
                    "unsupported-format": {
                        lead: "Formatua ez dago erabilgarri",
                        text: "Formatua ez dago erabilgarri — {{report}} txostena ezin da {{format}} formatuan sortu.",
                    },
                },
                schedule: {
                    "closes-before-opens": {
                        lead: "Ireki aurretik ixten da",
                        text: "Ireki aurretik ixten da — bozketa ez litzateke inoiz irekita egongo.",
                    },
                    "crosses-daylight-saving": {
                        lead: "Ordu-aldaketa bat zeharkatzen du",
                        text: "Ordu-aldaketa bat zeharkatzen du — leihoa orduek iradokitzen dutena baino ordubete luzeagoa edo laburragoa da.",
                    },
                    "key-ceremony-not-first": {
                        lead: "Gako-zeremonia beranduegi",
                        text: "Gako-zeremonia beranduegi — hauteskunde-gakoak existitu egin behar du harekin boto bat zifratu aurretik.",
                    },
                    "tally-ceremony-too-early": {
                        lead: "Zenbaketa-zeremonia goizegi",
                        text: "Zenbaketa-zeremonia goizegi — oraindik eman ez diren botoak zenbatuko lituzke.",
                    },
                    "window-incomplete": {
                        lead: "Bozketa-leihoa osatu gabe",
                        text: "Bozketa-leihoa osatu gabe — aldia eskuz ireki edo itxi beharko da Administrazio Atarian.",
                    },
                },
                signing: {
                    "duplicate-action": {
                        lead: "Bi arau ekintza baterako",
                        text: "Bi arau ekintza baterako — '{{action}}' ekintzak sinadura-arau bat baino gehiago du. Utzi bakarra.",
                    },
                    "signatures-out-of-range": {
                        lead: "Sinadurak tartetik kanpo",
                        text: "Sinadurak tartetik kanpo — '{{action}}' ekintzaren arauak {{min}} eta {{max}} sinadura artean eskatu behar ditu.",
                    },
                    "expiry-out-of-range": {
                        lead: "Iraungitzea tartetik kanpo",
                        text: "Iraungitzea tartetik kanpo — '{{action}}' eskaera bat 1 eta 525.600 minutu (urtebete) artean iraungi behar da, edo inoiz ez.",
                    },
                },
                threshold: {
                    "above-trustees": {
                        lead: "Atalase altuegia",
                        text: "Atalase altuegia — {{trustees}} gako-zaintzaileetatik {{threshold}} behar dira, eta hori ezin da bete. Gakoa sortuko litzateke eta emaitza ezin izango litzateke inoiz deszifratu.",
                    },
                    "one": {
                        lead: "Bateko atalasea",
                        text: "Bateko atalasea — edozein gako-zaintzailek bakarrik ireki dezake zenbaketa, zerrendak dioena dioela — gako-zaintzaile bakarra izatearen berme bera, eta bat ere ez pertsona horren aurrean.",
                    },
                    "zero": {
                        lead: "Zeroko atalasea",
                        text: "Zeroko atalasea — zenbaketa inor gabe ireki liteke.",
                    },
                },
                trustee: {
                    "email-malformed": {
                        lead: "Ez da helbide elektroniko bat",
                        text: 'Ez da helbide elektroniko bat — "{{email}}" ez dirudi halakoa.',
                    },
                    "no-email": {
                        lead: "Gako-zaintzailea helbiderik gabe",
                        text: "Gako-zaintzailea helbiderik gabe — horrela gonbidatzen zaie gako-zeremoniara.",
                    },
                    "no-name": {
                        lead: "Gako-zaintzailea izenik gabe",
                        text: "Gako-zaintzailea izenik gabe — inportatzaileak gako-zeremoniako kideak izenaren bidez ebazten ditu, tenantean dagoeneko alta emanda dauden gako-zaintzaileekin alderatuta, eta izen huts bat isilean existitzen ez den kide bihurtzen da.",
                    },
                },
                trustees: {
                    "none": {
                        lead: "Gako-zaintzailerik ez",
                        text: "Gako-zaintzailerik ez — hauteskunde-gako batek gutxienez bi pertsona behar ditu hura gordetzeko. Bakarrarekin, pertsona horrek bere kabuz deszifra ditzake boto-txartel guztiak, eta hori da, hain zuzen, zifratzeak ematen duen bermea.",
                    },
                    "only-one": {
                        lead: "Gako-zaintzaile bakarra",
                        text: "Gako-zaintzaile bakarra — hauteskunde-gako batek gutxienez bi pertsona behar ditu hura gordetzeko. Bakarrarekin, pertsona horrek bere kabuz deszifra ditzake boto-txartel guztiak, eta hori da, hain zuzen, zifratzeak ematen duen bermea.",
                    },
                },
                voter: {
                    "area-unknown": {
                        lead: "Hauteslearen barrutia ezezaguna",
                        text: 'Hauteslearen barrutia ezezaguna — ezerk ez du "{{area}}" izena. Hautesleak beren barrutiarekin izenaren bidez lotzen dira, beraz, hautesle honek ez luke boto-txartelik jasoko. Kopiatu barrutiaren izena, berriro idatzi beharrean.',
                    },
                    "duplicate-username": {
                        lead: "Erabiltzaile-izena bi aldiz",
                        text: 'Erabiltzaile-izena bi aldiz — "{{username}}" {{first}}. errenkadan ere badago. Erabiltzaile-izena partekatzen duten bi hautesle kontu bakar bihurtzen dira, eta honek bestea ordezkatuko luke ezer esan gabe.',
                    },
                    "no-area": {
                        lead: "Hauteslea barrutirik gabe",
                        text: "Hauteslea barrutirik gabe — barrutiak erabakitzen du hautesle bakoitzak zein boto-txartel jasotzen duen, eta eraikuntzak barrutirik gabeko errolda-errenkada baztertzen du.",
                    },
                    "no-username": {
                        lead: "Hauteslea erabiltzaile-izenik gabe",
                        text: "Hauteslea erabiltzaile-izenik gabe — horrekin hasten du saioa eta horretatik eratortzen da bere kontua.",
                    },
                },
                voters: {
                    "duplicate-column": {
                        lead: "Zutabea bi aldiz emanda",
                        text: 'Zutabea bi aldiz emanda — bi zutabek "{{column}}" ezartzen dute. Utzi horietako bat bakarrik.',
                    },
                    "unreadable-row": {
                        lead: "Errenkada ezin da irakurri",
                        text: "Errenkada ezin da irakurri — {{row}}. errenkada ezin izan da irakurri: {{reason}}",
                    },
                    "vote-weight-misspelled": {
                        lead: "Botoaren pisuaren zutabea gaizki idatzita",
                        text: 'Botoaren pisuaren zutabea gaizki idatzita — "{{column}}" ez da ezagutzen. Zutabea zehatz-mehatz "{{expected}}" idazten da; bestela, hautesle guztiak 1 pisuarekin zenbatuko lirateke.',
                    },
                    "vote-weight-not-a-number": {
                        lead: "Botoaren pisua ez da zenbaki bat",
                        text: 'Botoaren pisua ez da zenbaki bat — {{row}}. errenkadako "{{value}}" 1 eta {{max}} arteko zenbaki oso bat izan behar da.',
                    },
                    "vote-weight-out-of-range": {
                        lead: "Botoaren pisua tartetik kanpo",
                        text: "Botoaren pisua tartetik kanpo — {{row}}. errenkadako {{value}} {{min}} eta {{max}} artean egon behar da.",
                    },
                },
            },
        },
    },
}

export default basqueTranslation
