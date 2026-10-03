// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {TranslationType} from "./en"

const tagalogTranslation: TranslationType = {
    translations: {
        language: "Tagalog",
        welcome: "Simulan natin: I-import ang auditable na balota..",
        breadcrumbSteps: {
            select: "Pumili ng Tagasuri",
            import: "I-import ang Data",
            verify: "I-verify",
            finish: "Tapos na",
        },
        electionEventBreadcrumbSteps: {
            created: "Nalikha",
            keys: "Mga Susi",
            publish: "I-publish",
            started: "Nagsimula",
            ended: "Natapos",
            results: "Mga Resulta",
        },
        a11y: {
            closeDialog: "Isara ang dialog",
            dismissMessage: "I-dismiss ang mensahe",
            ballotIdHelp: "Tungkol sa iyong Ballot ID",
            loading: "Naglo-load",
            severity: {
                error: "Error",
                warning: "Babala",
                success: "Tagumpay",
                info: "Impormasyon",
            },
            selectList: "Piliin ang buong listahan",
            preferenceLabel: "Kagustuhan",
            writeInFor: "Pangalan ng write-in candidate",
        },
        accessibility: {
            button: "Accessibility",
            title: "Mga setting ng accessibility",
            description: "Baguhin ang itsura ng site na ito sa device na ito.",
            textSize: {
                label: "Laki ng teksto",
                default: "Karaniwan",
                large: "Malaki",
                larger: "Mas malaki",
            },
            contrast: {
                label: "Contrast",
                default: "Karaniwan",
                high: "Mataas na contrast",
            },
            textSpacing: {
                label: "Agwat ng teksto",
                default: "Karaniwan",
                wide: "Maluwag",
            },
            motion: {
                label: "Galaw",
                default: "Karaniwan",
                reduced: "Binawasan",
            },
            reset: "I-reset ang mga setting",
            close: "Isara",
            applied: "{{setting}}: {{value}}",
            resetDone: "Na-reset ang mga setting",
        },
        candidate: {
            moreInformationLink: "Karagdagang impormasyon",
            writeInsPlaceholder: "I-type ang write-in candidate dito",
            blankVote: "Blangkong Boto",
            preferential: {
                position: "Posisyon",
                none: "Wala",
                ordinals: {
                    first: "st",
                    second: "nd",
                    third: "rd",
                    other: "th",
                },
            },
        },
        homeScreen: {
            title: "Sequent Ballot Verifier",
            description1:
                "Ang ballot verifier ay ginagamit kapag pinili ng botante na i-audit ang balota sa voting booth. Ang pag-verify ay tatagal ng 1-2 minuto.",
            description2:
                "Ang ballot verifier ay nagbibigay-daan sa botante na tiyakin na ang encrypted na balota ay wastong nakuha ang mga pinili sa voting booth. Ang pagpayag na magsagawa ng pagsusuri na ito ay tinatawag na cast-as-intended verifiability at pinipigilan ang mga pagkakamali at malisyosong aktibidad habang ine-encrypt ang balota.",
            descriptionMore: "Alamin pa",
            startButton: "Mag-browse ng file",
            dragDropOption: "O i-drag at i-drop ito dito",
            importErrorDescription:
                "Nagkaroon ng problema sa pag-import ng sisiyasating balota. Tama ba ang napili mong file?",
            importErrorMoreInfo: "Karagdagang impormasyon",
            importErrorTitle: "Error",
            useSampleText: "Walang sisiyasatin na balota?",
            useSampleLink: "Gamitin ang sample na sisiyasating balota",
        },
        confirmationScreen: {
            title: "Sequent Ballot Verifier",
            topDescription1:
                "Batay sa impormasyon sa na-import na Sisiyasating Balota, nakalkula namin na:",
            topDescription2: "Kung ito ang Ballot ID na ipinakita sa Voting Booth:",
            bottomDescription1:
                "Ang iyong balota ay na-encrypt ng tama. Maaari mo nang isara ang window na ito at bumalik sa Voting Booth.",
            bottomDescription2:
                "Kung hindi sila nagtutugma, i-click dito upang malaman ang mga posibleng dahilan at kung ano ang mga hakbang na maaari mong gawin.",
            ballotChoicesDescription: "At ang iyong mga napili sa balota ay:",
            helpAndFaq: "Help at FAQ",
            backButton: "Bumalik",
            markedInvalid: "Ang balota ay tahasang minarkahan na invalid",
        },
        ballotSelectionsScreen: {
            statusModal: {
                title: "Status",
                content:
                    "Ang status panel ay nagbibigay sa iyo ng impormasyon tungkol sa mga beripikasyon na isinagawa.",
                ok: "OK",
            },
        },
        footer: {
            poweredBy: "Pinapatakbo ng <sequent />",
        },
        errors: {
            encoding: {
                notEnoughChoices: "Hindi sapat ang mga pagpipilian para ma-decode",
                writeInChoiceOutOfRange: "Write-in na napili ay wala sa saklaw: {{index}}",
                writeInNotEndInZero: "Ang Write-in ay hindi nagtatapos sa 0",
                writeInCharsExceeded:
                    "Ang Write-in ay lumampas ng {{numCharsExceeded}} sa maximum na bilang ng mga karakter. Kailangang ayusin.",
                bytesToUtf8Conversion:
                    "Error sa pag-convert ng write-in mula bytes patungong UTF-8 na string: {{errorMessage}}",
                ballotTooLarge: "Ang balota ay mas malaki kaysa sa inaasahan",
            },
            implicit: {
                selectedMax:
                    "Overvote: Ang bilang ng mga napili {{numSelected}} ay higit sa maximum na {{max}}",
                selectedMin:
                    "Ang bilang ng mga napili {{numSelected}} ay mas mababa sa minimum na {{min}}",
                maxSelectionsPerType:
                    "Ang bilang ng mga napili {{numSelected}} para sa listahan {{type}} ay higit sa maximum na {{max}}",
                underVote:
                    "Undervote: Ang bilang ng mga napili {{numSelected}} ay mas mababa sa maximum na {{max}}",
                overVoteDisabled:
                    "Naabot na ang maximum: Napili mo na ang maximum na {{numSelected}} na mga opsyon. Upang baguhin ang iyong pagpili, mangyaring alisin muna ang isa pang opsyon.",
                blankVote: "Blank Vote: Walang pinili",
                preferenceOrderWithGaps:
                    "Di-wastong boto! Ang pagkakasunod-sunod ng kagustuhan ay may isa o higit pang puwang.",
                duplicatedPosition:
                    "Di-wastong boto! Ang parehong posisyon ay napili para sa dalawa o higit pang kandidato.",
            },
            explicit: {
                notAllowed:
                    "Ang balota ay tahasang minarkahan upang mapawalang-bisa ngunit hindi ito pinapayagan ng tanong",
                alert: "Ang minarkahang pagpili ay ituturing na hindi wastong boto.",
            },
            configuration: {
                multipleExplicitInvalidCandidates:
                    "Hindi wastong configuration ng balota: may {{count}} tahasang invalid na kandidato sa contest, ngunit isa lamang ang pinapayagan.",
                multipleExplicitBlankCandidates:
                    "Hindi wastong configuration ng balota: may {{count}} tahasang blankong kandidato sa contest, ngunit isa lamang ang pinapayagan.",
            },
        },
        ballotHash: "Ang Iyong Ballot ID: {{ballotId}}",
        version: {
            header: "Bersyon:",
        },
        hash: {
            header: "Hash:",
        },
        logout: {
            buttonText: "Mag-logout",
            modal: {
                title: "Sigurado ka bang gusto mong mag-logout?",
                content:
                    "Malapit mo nang isara ang application na ito. Ang aksyong ito ay hindi maibabalik.",
                ok: "OK",
                close: "Isara",
            },
        },
        stories: {
            openDialog: "Buksan ang Dialog",
        },
        dragNDrop: {
            firstLine: "I-drag at i-drop ang mga file o",
            browse: "Mag-browse",
            format: "Suportadong format: txt",
            importError: "Hindi ma-import ang file na ito. Pakisubukang muli.",
        },
        selectElection: {
            electionWebsite: "Website ng Balota",
            countdown:
                "Magsisimula ang halalan sa loob ng {{years}} taon, {{months}} buwan, {{weeks}} linggo, {{days}} araw, {{hours}} oras, {{minutes}} minuto, {{seconds}} segundo",
            openElection: "Bukas na",
            closedElection: "Sarado",
            voted: "Nakaboto",
            notVoted: "Hindi pa nakaboto",
            resultsButton: "Mga Resulta ng Balota",
            voteButton: "I-click upang Bumoto",
            openDate: "Simula: ",
            closeDate: "Pagtapos: ",
            ballotLocator: "Hanapin ang iyong balota",
        },
        header: {
            profile: "Profile",
            welcome: "Sumalubong,<br><span>{{name}}</span>",
            session: {
                title: "Malapit nang mag-expire ang iyong session.",
                timeLeft: "May natitira ka pang {{time}} para bumoto.",
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minuto at {{time}} segundo",
                timeLeftSeconds: "{{timeLeft}} segundo",
            },
        },
        problems: {
            noProblems: "Walang problema. Maiimport ito.",
            errors_one: "{{count}} error",
            errors_other: "{{count}} na error",
            errorsExplained: "Hindi ito maiimport hangga't hindi naaayos ang bawat isa sa mga ito.",
            warnings_one: "{{count}} babala",
            warnings_other: "{{count}} na babala",
            warningsExplained:
                "Maiimport ito. Malamang na hindi ito ang talagang ibig sabihin ng bawat isa sa mga ito.",
            warningsStrict: "Naka-on ang strict mode, kaya pinahihinto ng mga ito ang build.",
            messages: {
                area: {
                    "cycle": {
                        lead: "Mga area na nasa loob ng isa't isa",
                        text: "Mga area na nasa loob ng isa't isa — bahagi ang area na '{{area}}' ng paikot na mga parent, na magpapa-hang sa Admin Portal.",
                    },
                    "duplicate-name": {
                        lead: "Dalawang area na pareho ang pangalan",
                        text: "Dalawang area na pareho ang pangalan — ang '{{name}}' ay parehong '{{first}}' at '{{second}}'. Hinahanap ng CSV ng mga botante ang area ayon sa pangalan, kaya mapupunta ang mga botante sa kung alinman ang unang makita ng importer.",
                    },
                    "early-voting-unknown": {
                        lead: "Hindi kilalang setting ng early voting",
                        text: "Hindi kilalang setting ng early voting — hindi wasto ang '{{value}}' para sa isang area. Tinatanggap ng platform ang {{allowed}}.",
                    },
                    "empty-ballot": {
                        lead: "Area na walang laman ang balota",
                        text: "Area na walang laman ang balota — walang contest na binobotohan ang area na '{{area}}', sarili man o minana mula sa area na kinapapalooban nito, kaya walang laman na balota ang makikita ng mga botante nito.",
                    },
                    "inside-itself": {
                        lead: "Area na nasa loob ng sarili",
                        text: "Area na nasa loob ng sarili — hindi puwedeng maging parent ng sarili ang isang area.",
                    },
                    "no-identifier": {
                        lead: "Area na walang identifier",
                        text: "Area na walang identifier — kailangan ng bawat area ng isa.",
                    },
                    "no-name": {
                        lead: "Area na walang pangalan",
                        text: "Area na walang pangalan — tinutukoy ng CSV ng mga botante ang area ng botante ayon sa pangalan, hindi sa id, kaya walang botanteng mailalagay sa area na walang pangalan.",
                    },
                    "parent-missing": {
                        lead: "Nawawala ang parent area",
                        text: "Nawawala ang parent area — nasa loob ang area ng parent na wala sa file na ito.",
                    },
                    "parent-unknown": {
                        lead: "Hindi kilala ang parent area",
                        text: "Hindi kilala ang parent area — walang may identifier na '{{parent}}'.",
                    },
                },
                ballot: {
                    "no-elections": {
                        lead: "Walang halalan",
                        text: "Walang halalan — kailangan ng election event ng kahit isa.",
                    },
                },
                bundle: {
                    "duplicate-id": {
                        lead: "Dalawang beses ginamit ang identifier",
                        text: "Dalawang beses ginamit ang identifier — ginagamit din ng {{previous}} ang {{id}} sa {{kind}}.",
                    },
                    "event-no-encryption": {
                        lead: "Walang encryption protocol",
                        text: "Walang encryption protocol — hindi sinasabi ng election event kung paano ine-encrypt ang mga balota.",
                    },
                    "event-no-id": {
                        lead: "Event na walang id",
                        text: "Event na walang id — walang identifier ang election event sa file na ito.",
                    },
                    "no-areas": {
                        lead: "Walang area",
                        text: "Walang area — walang botanteng mabibigyan ng balota hangga't walang kahit isang area ang event.",
                    },
                    "no-elections": {
                        lead: "Walang halalan",
                        text: "Walang halalan — kailangan ng election event ng kahit isang halalan.",
                    },
                    "no-tenant": {
                        lead: "Walang tenant",
                        text: "Walang tenant — hindi sinasabi ng file kung saang tenant ito kabilang.",
                    },
                    "tenant-not-uuid": {
                        lead: "Hindi identifier ang tenant",
                        text: "Hindi identifier ang tenant — ang '{{value}}' ay hindi wastong tenant id.",
                    },
                },
                candidate: {
                    "contest-missing": {
                        lead: "Nawawala ang contest ng kandidato",
                        text: "Nawawala ang contest ng kandidato — kabilang ang kandidato sa contest na wala sa file na ito.",
                    },
                    "no-contest": {
                        lead: "Kandidatong walang contest",
                        text: "Kandidatong walang contest — dapat kabilang ang bawat kandidato sa isang contest.",
                    },
                    "picture-mismatch": {
                        lead: "Iba ang itinuturo ng larawan",
                        text: "Iba ang itinuturo ng larawan — ipinapakita ng balota ang '{{url}}', na hindi tumutukoy sa dokumentong '{{document}}' na katabi nito. Pagkatapos ng import, magkaibang file ang ituturo ng dalawa.",
                    },
                    "picture-not-shown": {
                        lead: "Hindi kailanman ipapakita ang larawan",
                        text: "Hindi kailanman ipapakita ang larawan — may binabanggit na larawan ang isang kandidato na hindi itinuturo ng anumang entry sa balota, kaya ia-upload ito at hindi kailanman ipapakita.",
                    },
                    "picture-unrecorded": {
                        lead: "Hindi naitala ang larawan",
                        text: "Hindi naitala ang larawan — nasa balota ang larawan ng isang kandidato at walang nakatalang aling dokumento ito, kaya hindi na ito mapapalitan o maaalis sa platform pagkatapos.",
                    },
                },
                census: {
                    "column-not-declared": {
                        lead: "Hindi idineklara ang column ng census",
                        text: "Hindi idineklara ang column ng census — {{message}}",
                    },
                    "no-area-column": {
                        lead: "Walang area sa census",
                        text: "Walang area sa census — may mga area ang halalang ito pero ang default na balota ang matatanggap ng bawat botante. Kung dapat ilapat ang pagkakahati sa distrito, kailangan ng census ng column para sa area.",
                    },
                    "note": {
                        lead: "Tungkol sa census",
                        text: "Tungkol sa census — {{message}}",
                    },
                    "unreadable": {
                        lead: "Hindi mabasa ang census",
                        text: "Hindi mabasa ang census — {{message}}",
                    },
                    "unreadable-member": {
                        lead: "Hindi mabasa ang census sa zip",
                        text: "Hindi mabasa ang census sa zip — {{message}}",
                    },
                },
                channels: {
                    "early-voting-closed": {
                        lead: "Naka-off ang early voting",
                        text: "Naka-off ang early voting — pinapayagan ng {{areas}} ang early voting at hindi binubuksan ng event ang channel na iyon, kaya walang epekto ang setting.",
                    },
                    "early-voting-no-area": {
                        lead: "Early voting na walang area",
                        text: "Early voting na walang area — bukas ang early voting at walang area na pumapayag dito, kaya walang botante sa early period.",
                    },
                    "kiosk-client": {
                        lead: "Kailangan ng kiosk ng sariling client",
                        text: "Kailangan ng kiosk ng sariling client — kailangan ng pagboto sa kiosk ng authentication client na ipinangalan sa karaniwang client na may '-kiosk' sa dulo, na hindi ginagawa ng file na ito.",
                    },
                    "none-open": {
                        lead: "Walang paraan ng pagboto",
                        text: "Walang paraan ng pagboto — naka-off ang bawat voting channel, kaya walang makakaboto.",
                    },
                    "telephone-elsewhere": {
                        lead: "Isi-set up mamaya ang pagboto sa telepono",
                        text: "Isi-set up mamaya ang pagboto sa telepono — kino-configure ang pagboto sa telepono sa IVR tab ng event pagkatapos ng import; wala rito ang alinman doon.",
                    },
                    "unknown": {
                        lead: "Hindi kilalang paraan ng pagboto",
                        text: "Hindi kilalang paraan ng pagboto — walang bumabasa sa '{{name}}'. Ang mga paraan ng pagboto na kinikilala ng platform ay {{allowed}}.",
                    },
                },
                contacts: {
                    none: {
                        lead: "Walang contact person",
                        text: "Walang contact person — sila ang tatawagan sa araw ng halalan.",
                    },
                },
                contest: {
                    "algorithm-unknown": {
                        lead: "Hindi kilalang paraan ng pagbilang",
                        text: "Hindi kilalang paraan ng pagbilang — ang '{{value}}' ay hindi counting algorithm. Tinatanggap ng platform ang {{allowed}}.",
                    },
                    "area-unknown": {
                        lead: "Hindi kilala ang area ng contest",
                        text: "Hindi kilala ang area ng contest — walang may identifier na '{{area}}'.",
                    },
                    "cap-invalid": {
                        lead: "Imposibleng limitasyon sa pagpili",
                        text: "Imposibleng limitasyon sa pagpili — ang {{cap}} ay hindi bilang ng mga pagpili.",
                    },
                    "cap-never-applies": {
                        lead: "Hindi kailanman nailalapat ang limitasyon sa pagpili",
                        text: "Hindi kailanman nailalapat ang limitasyon sa pagpili — hindi kailanman nailalapat ang limitasyong {{cap}} bawat uri sa contest kung saan {{max}} lang ang kabuuang puwedeng piliin ng botante.",
                    },
                    "chooses-more-than-available": {
                        lead: "Mas maraming pipiliin kaysa kandidato",
                        text: "Mas maraming pipiliin kaysa kandidato — {{max}} ang puwedeng piliin ng botante mula sa {{available}} na kandidato.",
                    },
                    "chooses-more-than-offered": {
                        lead: "Mas maraming pipiliin kaysa opsyon",
                        text: "Mas maraming pipiliin kaysa opsyon — hanggang {{chosen}} ang puwedeng piliin ng botante pero {{offered}} lang ang mapagpipilian.",
                    },
                    "columns-invalid": {
                        lead: "Imposibleng layout",
                        text: "Imposibleng layout — hindi layout ang {{columns}} na column.",
                    },
                    "columns-too-many": {
                        lead: "Masyadong maraming column",
                        text: "Masyadong maraming column — hindi mababasa sa telepono ang {{columns}} na column, at telepono ang ginagamit ng karamihan ng botante sa pagboto.",
                    },
                    "count-missing": {
                        lead: "Kulang ng numero ang contest",
                        text: "Kulang ng numero ang contest — kailangan ng contest ang {{field}}.",
                    },
                    "count-negative": {
                        lead: "Negatibong bilang",
                        text: "Negatibong bilang — {{value}} ang {{field}}, at hindi puwedeng mas mababa sa zero ang bilang.",
                    },
                    "election-missing": {
                        lead: "Nawawala ang halalan ng contest",
                        text: "Nawawala ang halalan ng contest — kabilang ang contest sa halalang wala sa file na ito.",
                    },
                    "elects-more-than-available": {
                        lead: "Mas maraming puwesto kaysa kandidato",
                        text: "Mas maraming puwesto kaysa kandidato — {{winners}} ang mahahalal sa contest mula sa {{available}} na kandidato.",
                    },
                    "elects-more-than-chosen": {
                        lead: "Mas maraming mahahalal kaysa pinapayagan",
                        text: "Mas maraming mahahalal kaysa pinapayagan — {{winners}} ang mahahalal sa contest pero {{chosen}} lang ang puwedeng piliin ng botante.",
                    },
                    "elects-more-than-standing": {
                        lead: "Mas maraming puwesto kaysa kandidato",
                        text: "Mas maraming puwesto kaysa kandidato — {{winners}} ang mahahalal sa contest mula sa {{standing}} na kandidato.",
                    },
                    "elects-nobody": {
                        lead: "Walang mahahalal",
                        text: "Walang mahahalal — walang panalo ang contest.",
                    },
                    "max-votes-below-one": {
                        lead: "Walang maiboboto",
                        text: "Walang maiboboto — mas kaunti sa isang kandidato ang puwedeng piliin ng botante.",
                    },
                    "min-above-max": {
                        lead: "Mas mataas ang minimum kaysa maximum",
                        text: "Mas mataas ang minimum kaysa maximum — kailangang pumili ang botante ng hindi bababa sa {{min}} pero hanggang {{max}} lang ang puwede niyang piliin.",
                    },
                    "no-candidates": {
                        lead: "Walang kandidato",
                        text: "Walang kandidato — wala pang tumatakbo sa contest na ito.",
                    },
                    "no-candidates-in-bundle": {
                        lead: "Walang kandidato",
                        text: "Walang kandidato — walang kandidato ang contest, kaya walang makakaboto rito.",
                    },
                    "on-no-ballot": {
                        lead: "Contest na wala sa anumang balota",
                        text: "Contest na wala sa anumang balota — walang balota ng area na may kasamang contest na ito, kaya walang makakaboto rito.",
                    },
                    "policy-not-text": {
                        lead: "Hindi text ang panuntunan ng balota",
                        text: "Hindi text ang panuntunan ng balota — dapat text ang {{key}}, at {{value}} ito.",
                    },
                    "policy-unknown": {
                        lead: "Hindi kilalang panuntunan ng balota",
                        text: "Hindi kilalang panuntunan ng balota — ang '{{value}}' ay hindi wastong {{key}}. Tinatanggap ng platform ang {{allowed}}.",
                    },
                    "ranked-counted-unranked": {
                        lead: "Ranked na balota na binilang nang walang ranggo",
                        text: "Ranked na balota na binilang nang walang ranggo — binibilang ang isang preferential contest gamit ang '{{algorithm}}', na hindi pinapansin ang mga ranggong ibinigay ng mga botante.",
                    },
                    "tie-breaking-unknown": {
                        lead: "Hindi kilalang panuntunan sa tabla",
                        text: "Hindi kilalang panuntunan sa tabla — ang '{{value}}' ay hindi tie-breaking policy. Tinatanggap ng platform ang {{allowed}}.",
                    },
                    "unranked-counted-ranked": {
                        lead: "Karaniwang balota na binilang ayon sa ranggo",
                        text: "Karaniwang balota na binilang ayon sa ranggo — binibilang ang isang non-preferential contest gamit ang '{{algorithm}}', na nangangailangan ng mga ranked na balota.",
                    },
                    "voting-type-unknown": {
                        lead: "Hindi kilalang uri ng pagboto",
                        text: "Hindi kilalang uri ng pagboto — ang '{{value}}' ay hindi uri ng pagboto. Tinatanggap ng platform ang {{allowed}}.",
                    },
                    "write-in-slots-not-allowed": {
                        lead: "Hindi pinapayagan ang write-in slot",
                        text: "Hindi pinapayagan ang write-in slot — may {{count}} na write-in slot sa contest na hindi pumapayag sa write-in, kaya may mga opsyong walang pangalan sa balota.",
                    },
                    "write-ins-no-slot": {
                        lead: "Write-in na walang mapagsusulatan",
                        text: "Write-in na walang mapagsusulatan — pinapayagan ang write-in at walang write-in slot ang contest, kaya walang mapagta-type-an ng pangalan ang botante.",
                    },
                },
                delivery: {
                    "no-importable": {
                        lead: "Walang maiimport na archive",
                        text: "Walang maiimport na archive — may plan ang zip na ito pero wala ang archive na iniimport ng Admin Portal, kaya wala rito ang census at ang mga file na binabanggit nito.",
                    },
                },
                election: {
                    "channels-differ": {
                        lead: "Hindi magkatugma ang halalan at event",
                        text: "Hindi magkatugma ang halalan at event — iba ang {{channels}} sa event, kaya hindi magtutugma ang mga start control para sa halalang ito.",
                    },
                    "grace-disallowed": {
                        lead: "Naka-off ang grace period",
                        text: "Naka-off ang grace period — may nakatakdang {{seconds}} segundong grace at hindi pinapayagan ang grace period, kaya magsasara ang botohan sa deadline.",
                    },
                    "grace-negative": {
                        lead: "Negatibong grace period",
                        text: "Negatibong grace period — hindi haba ng oras ang {{value}} segundo.",
                    },
                    "grace-zero": {
                        lead: "Grace period na walang oras",
                        text: "Grace period na walang oras — pinapayagan ang grace period at zero segundo ang haba nito, kaya wala talaga.",
                    },
                    "no-contests": {
                        lead: "Walang contest",
                        text: "Walang contest — walang boboto sa halalang ito.",
                    },
                    "revotes-negative": {
                        lead: "Imposibleng bilang ng boto",
                        text: "Imposibleng bilang ng boto — ang {{value}} ay hindi bilang ng beses na puwedeng bumoto ang botante.",
                    },
                    "setting-unknown": {
                        lead: "Hindi kilalang setting ng halalan",
                        text: "Hindi kilalang setting ng halalan — ang '{{value}}' ay hindi wastong {{key}}. Tinatanggap ng platform ang {{allowed}}.",
                    },
                    "spoil-without-revote": {
                        lead: "Pag-spoil na walang pangalawang pagkakataon",
                        text: "Pag-spoil na walang pangalawang pagkakataon — puwedeng itapon ng botante ang naibotong balota at wala siyang pangalawang pagkakataon para palitan ito.",
                    },
                },
                event: {
                    "no-identifier": {
                        lead: "Walang identifier",
                        text: "Walang identifier — dito hinahango ang bawat nabuong id, kaya kung wala ito, walang mabubuo nang dalawang beses sa parehong paraan.",
                    },
                    "no-name": {
                        lead: "Walang pangalan",
                        text: "Walang pangalan — nakikita ito ng mga botante sa itaas ng balota, at ito ang nagiging pamagat ng login page.",
                    },
                    "setting-unknown": {
                        lead: "Hindi kilalang setting ng event",
                        text: "Hindi kilalang setting ng event — ang '{{value}}' ay hindi wastong {{key}}. Tinatanggap ng platform ang {{allowed}}.",
                    },
                },
                file: {
                    "cannot-decrypt": {
                        lead: "Hindi ma-decrypt",
                        text: "Hindi ma-decrypt — naka-encrypt ang file at hindi ito nabuksan. Tingnan ang password.",
                    },
                    "checksum-mismatch": {
                        lead: "Hindi ito ang inaasahang file",
                        text: "Hindi ito ang inaasahang file — hindi tugma ang SHA-256 nito sa ibinigay, kaya hindi ito ang file na tinutukoy. Tingnan ang checksum o i-upload ulit ang file.",
                    },
                    "duplicate-name": {
                        lead: "Dalawang beses ginamit ang pangalan ng file",
                        text: "Dalawang beses ginamit ang pangalan ng file — tumutukoy ang '{{file}}' sa dalawang magkaibang bagay. Ipinapasa ang mga file ayon sa pangalan, kaya tahimik na magiging isa ang dalawa.",
                    },
                    "missing": {
                        lead: "Nawawala ang file",
                        text: "Nawawala ang file — binanggit ang '{{file}}' at walang anumang narito na naglalaman nito. Pinapabigo ng walang lamang entry sa archive ang import sa halip na mawalan ng file.",
                    },
                    "not-a-bundle": {
                        lead: "Hindi export ng election event",
                        text: "Hindi export ng election event — nabasa ang file pero wala itong hugis ng isang export: {{reason}}",
                    },
                    "not-json": {
                        lead: "Hindi mabasang file",
                        text: "Hindi mabasang file — hindi ito export ng election event na mababasa ng platform: {{reason}}",
                    },
                    "unreadable-archive": {
                        lead: "Hindi mabasa ang archive",
                        text: "Hindi mabasa ang archive — {{reason}}",
                    },
                    "unused": {
                        lead: "Hindi ginamit ang file",
                        text: "Hindi ginamit ang file — ibinigay ang '{{file}}' at walang bumabanggit dito, kaya isasama ito sa delivery at hindi ipapakita kaninuman.",
                    },
                    "version-incompatible": {
                        lead: "Na-export ng ibang bersyon",
                        text: "Na-export ng ibang bersyon — galing ang file sa bersyon {{found}}, na hindi maiimport ng bersyon {{current}}.",
                    },
                },
                identifier: {
                    duplicate: {
                        lead: "Dalawang beses ginamit ang identifier",
                        text: "Dalawang beses ginamit ang identifier — ginagamit na ng {{first}} ang '{{identifier}}'. Natatangi ang mga identifier sa buong election event, kaya papalitan ng pangalawa ang una sa halip na maidagdag.",
                    },
                },
                ivr: {
                    "language-not-spoken": {
                        lead: "Wikang hindi available sa telepono",
                        text: "Wikang hindi available sa telepono — Ingles, Pranses at Espanyol lang ang sinasalita ng tawag, kaya hindi iniaalok ang {{languages}} sa mga tumatawag.",
                    },
                    "missing-prompts": {
                        lead: "Kulang ang mga mensahe ng tawag",
                        text: "Kulang ang mga mensahe ng tawag — walang teksto sa '{{language}}' ang {{prompts}}, kaya tinatanggihan ng sistema ng telepono ang bawat tawag hanggang magkaroon.",
                        // Filipino's plural rule files most counts under "one" (2, 3,
                        // 5, 21...), so this form has to read for several prompts too:
                        // it is the general sentence again, not a singular one.
                        lead_one: "Kulang ang mga mensahe ng tawag",
                        text_one:
                            "Kulang ang mga mensahe ng tawag — walang teksto sa '{{language}}' ang {{prompts}}, kaya tinatanggihan ng sistema ng telepono ang bawat tawag hanggang magkaroon.",
                    },
                },
                labels: {
                    "in-use": {
                        lead: "May ginagamit na permission label",
                        text: "May ginagamit na permission label — {{labels}}. Nakatago ang anumang may label sa bawat administrator na wala nito, kaya kailangan ng mag-iimport nito ng isa sa mga ito sa sarili niyang account, kung hindi ay walang laman na listahan ang ipapakita sa kanila ng Admin Portal.",
                    },
                },
                languages: {
                    "default-not-offered": {
                        lead: "Hindi kasama ang default na wika",
                        text: "Hindi kasama ang default na wika — wala ang '{{chosen}}' sa {{offered}}, kaya ang una ang matatanggap ng mga botante.",
                    },
                    "detection-unknown": {
                        lead: "Hindi kilalang language detection",
                        text: "Hindi kilalang language detection — ang '{{policy}}' ay hindi policy na kilala ng platform.",
                    },
                    "none": {
                        lead: "Walang wika",
                        text: "Walang wika — babalik sa English ang balota, na pansalo lang at hindi isang pinili.",
                    },
                },
                link: {
                    "area-missing": {
                        lead: "Ballot link sa nawawalang area",
                        text: "Ballot link sa nawawalang area — inilagay ang isang contest sa balota ng area na wala sa file na ito.",
                    },
                    "contest-missing": {
                        lead: "Ballot link sa nawawalang contest",
                        text: "Ballot link sa nawawalang contest — may nakalistang contest sa balota ng isang area na wala sa file na ito.",
                    },
                },
                logo: {
                    "file-and-link": {
                        lead: "Dalawang logo",
                        text: "Dalawang logo — may parehong in-upload na logo ('{{file}}') at link ('{{url}}') ang plan na ito. Ang file ang ipapadala; hindi papansinin ang link.",
                    },
                    "file-missing": {
                        lead: "Nawawala ang logo file",
                        text: "Nawawala ang logo file — itinalaga ang '{{file}}' bilang logo at hindi ito ibinigay. Ilagay ang file katabi ng workbook gamit ang eksaktong pangalang iyon.",
                    },
                    "no-bytes": {
                        lead: "Walang laman ang logo file",
                        text: "Walang laman ang logo file — itinalaga ang '{{file}}' bilang logo at wala itong laman. Pinapabigo ng walang lamang entry sa archive ang import sa halip na mawalan ng larawan.",
                    },
                },
                material: {
                    "empty-document": {
                        lead: "Material na walang lamang dokumento",
                        text: "Material na walang lamang dokumento — maiimport ito bilang link sa wala. Huwag nang isama ang dokumento para sa material na walang file.",
                    },
                    "file-missing": {
                        lead: "Nawawala ang file ng material",
                        text: "Nawawala ang file ng material — binanggit dito ang '{{file}}' at hindi ito ibinigay. Ilagay ang file katabi ng workbook gamit ang eksaktong pangalang iyon.",
                    },
                    "file-unused": {
                        lead: "Hindi ginamit ang file ng material",
                        text: "Hindi ginamit ang file ng material — ibinigay ang '{{file}}' at walang row na bumabanggit dito, kaya ia-upload ito at hindi ipapakita kaninuman.",
                    },
                    "no-identifier": {
                        lead: "Material na walang identifier",
                        text: "Material na walang identifier — dito hinahango ang id ng dokumento nito, kaya kung wala ito, hindi maitutugma ang file sa row.",
                    },
                    "tab-off": {
                        lead: "Naka-off ang tab ng materials",
                        text: "Naka-off ang tab ng materials — may {{count}} na support material sa file na ito at naka-off ang tab na nagpapakita sa kanila, kaya hindi kailanman makikita ng mga botante ang mga ito.",
                    },
                    "wrong-event": {
                        lead: "Material mula sa ibang event",
                        text: "Material mula sa ibang event — kabilang ang support material na ito sa ibang election event.",
                    },
                },
                messages: {
                    "weekly-no-time": {
                        lead: "Pag-ulit na walang oras",
                        text: "Pag-ulit na walang oras — inuulit linggo-linggo ang isang mensahe pero hindi sinasabi kung anong oras, kaya ang magpapadala nito ang kailangang pumili ng oras na walang sumulat.",
                    },
                },
                passwords: {
                    "column-already-there": {
                        lead: "Dalawang beses ibinigay ang mga password",
                        text: "Dalawang beses ibinigay ang mga password — may column na para sa password ang census, kaya magiging pangalawang sagot sa parehong tanong ang mga nabuong password. Alisin ang column, o i-off ang pagbuo ng password.",
                    },
                    "no-characters": {
                        lead: "Mga password na walang character",
                        text: "Mga password na walang character — kailangan ng mga nabuong password ng kahit isang uri ng character na gagamitin.",
                    },
                    "no-seed": {
                        lead: "Mga password na walang seed",
                        text: "Mga password na walang seed — ang seed ang nagpapatiyak na parehong mga password ang mabubuo sa muling pag-build sa halip na mga bago.",
                    },
                },
                plan: {
                    "not-a-plan": {
                        lead: "Hindi election plan",
                        text: "Hindi election plan — hindi mabasa ang file bilang isang plan: {{error}}",
                    },
                    "saved-by-newer-version": {
                        lead: "Na-save ng mas bagong bersyon",
                        text: "Na-save ng mas bagong bersyon — {{saved}} kumpara sa {{supported}}. Kapag binuksan ito rito, tahimik na mawawala ang anumang idinagdag ng bersyong iyon.",
                    },
                    "unreadable": {
                        lead: "Hindi mabasa ang plan",
                        text: "Hindi mabasa ang plan — {{error}}",
                    },
                },
                schedule: {
                    "closes-before-opens": {
                        lead: "Nagsasara bago magbukas",
                        text: "Nagsasara bago magbukas — hindi kailanman magbubukas ang botohan.",
                    },
                    "crosses-daylight-saving": {
                        lead: "Tumatawid sa pagbabago ng oras",
                        text: "Tumatawid sa pagbabago ng oras — isang oras na mas mahaba o mas maikli ang panahon kaysa sa ipinapakita ng mga oras.",
                    },
                    "key-ceremony-not-first": {
                        lead: "Huli na ang key ceremony",
                        text: "Huli na ang key ceremony — kailangang umiiral na ang election key bago ma-encrypt ang isang boto gamit ito.",
                    },
                    "tally-ceremony-too-early": {
                        lead: "Masyadong maaga ang tally ceremony",
                        text: "Masyadong maaga ang tally ceremony — bibilangin nito ang mga botong hindi pa naiboboto.",
                    },
                    "window-incomplete": {
                        lead: "Kulang ang panahon ng botohan",
                        text: "Kulang ang panahon ng botohan — kailangang manu-manong buksan o isara ang panahon sa Admin Portal.",
                    },
                },
                threshold: {
                    "above-trustees": {
                        lead: "Masyadong mataas ang threshold",
                        text: "Masyadong mataas ang threshold — {{threshold}} sa {{trustees}} na trustee ang kailangan, na hindi maaabot. Mabubuo ang key pero hindi kailanman ma-decrypt ang resulta.",
                    },
                    "one": {
                        lead: "Threshold na isa",
                        text: "Threshold na isa — kahit sinong iisang trustee ay kayang buksan ang tally nang mag-isa, anuman ang sabihin ng listahan — parehong garantiya ng pagkakaroon ng iisang trustee, at walang proteksyon laban sa taong iyon.",
                    },
                    "zero": {
                        lead: "Threshold na zero",
                        text: "Threshold na zero — mabubuksan ang tally nang walang sinuman.",
                    },
                },
                trustee: {
                    "email-malformed": {
                        lead: "Hindi email address",
                        text: "Hindi email address — mukhang hindi email address ang '{{email}}'.",
                    },
                    "no-email": {
                        lead: "Trustee na walang address",
                        text: "Trustee na walang address — ito ang paraan ng pag-imbita sa kanila sa key ceremony.",
                    },
                    "no-name": {
                        lead: "Trustee na walang pangalan",
                        text: "Trustee na walang pangalan — itinutugma ng importer ang mga miyembro ng key ceremony ayon sa pangalan sa mga trustee na naka-set up na sa tenant, at ang walang laman ay tahimik na nagiging miyembrong hindi umiiral.",
                    },
                },
                trustees: {
                    "none": {
                        lead: "Walang trustee",
                        text: "Walang trustee — kailangan ng election key ng kahit dalawang taong hahawak nito. Kung iisa, kaya ng taong iyon na i-decrypt ang bawat balota nang mag-isa, na siyang dapat pigilan ng encryption.",
                    },
                    "only-one": {
                        lead: "Iisa lang ang trustee",
                        text: "Iisa lang ang trustee — kailangan ng election key ng kahit dalawang taong hahawak nito. Kung iisa, kaya ng taong iyon na i-decrypt ang bawat balota nang mag-isa, na siyang dapat pigilan ng encryption.",
                    },
                },
                voter: {
                    "area-unknown": {
                        lead: "Hindi kilala ang area ng botante",
                        text: "Hindi kilala ang area ng botante — walang nagngangalang '{{area}}'. Itinutugma ang mga botante sa kanilang area ayon sa pangalan, kaya walang balotang matatanggap ang botanteng ito. Kopyahin ang pangalan mula sa area sa halip na i-type ulit.",
                    },
                    "duplicate-username": {
                        lead: "Dalawang beses ginamit ang username",
                        text: "Dalawang beses ginamit ang username — nasa row {{first}} din ang '{{username}}'. Nagiging iisang account ang dalawang botanteng may parehong username, at papalitan ng isang ito ang isa nang walang pasabi.",
                    },
                    "no-area": {
                        lead: "Botanteng walang area",
                        text: "Botanteng walang area — ang area ang nagpapasya kung aling balota ang ibibigay sa botante, at tinatanggihan ng build ang row sa census na walang area.",
                    },
                    "no-username": {
                        lead: "Botanteng walang username",
                        text: "Botanteng walang username — ito ang ginagamit nila sa pag-sign in at dito hinahango ang kanilang account.",
                    },
                },
                voters: {
                    "duplicate-column": {
                        lead: "Dalawang beses ibinigay ang column",
                        text: "Dalawang beses ibinigay ang column — parehong nagse-set ng '{{column}}' ang dalawang column. Isa lang ang panatilihin.",
                    },
                    "unreadable-row": {
                        lead: "Hindi mabasa ang row",
                        text: "Hindi mabasa ang row — hindi mabasa ang row {{row}}: {{reason}}",
                    },
                    "vote-weight-misspelled": {
                        lead: "Mali ang baybay ng column ng vote weight",
                        text: "Mali ang baybay ng column ng vote weight — hindi kilala ang '{{column}}'. Eksaktong '{{expected}}' ang baybay ng column, kung hindi ay bibilangin ang bawat botante na may weight na 1.",
                    },
                    "vote-weight-not-a-number": {
                        lead: "Hindi numero ang vote weight",
                        text: "Hindi numero ang vote weight — ang '{{value}}' sa row {{row}} ay dapat buong numero mula 1 hanggang {{max}}.",
                    },
                    "vote-weight-out-of-range": {
                        lead: "Lampas sa saklaw ang vote weight",
                        text: "Lampas sa saklaw ang vote weight — ang {{value}} sa row {{row}} ay dapat nasa pagitan ng {{min}} at {{max}}.",
                    },
                },
            },
        },
    },
}

export default tagalogTranslation
