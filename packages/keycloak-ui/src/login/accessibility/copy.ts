// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// The settings and the player use the Voting Portal's own words, read from the
// same translation files, so a voter meets one wording before and after signing in.
import english from "../../../../ui-core/src/translations/en"
import spanish from "../../../../ui-core/src/translations/es"
import catalan from "../../../../ui-core/src/translations/cat"
import basque from "../../../../ui-core/src/translations/eu"
import french from "../../../../ui-core/src/translations/fr"
import galician from "../../../../ui-core/src/translations/gl"
import dutch from "../../../../ui-core/src/translations/nl"
import tagalog from "../../../../ui-core/src/translations/tl"

export type InstructionsPageId = "login.ftl" | "login-username.ftl" | "message-otp.login.ftl"

type PortalCopy = Pick<typeof english.translations, "accessibility" | "audioInstructions">

export type AccessibilityCopy = PortalCopy & {
    /** The language of this copy, as a BCP 47 tag. */
    languageTag: string
    /** The language code the Voting Portal knows this language by. */
    portalLanguage: string
    instructions: Record<InstructionsPageId, string>
}

const translations: Record<string, Omit<AccessibilityCopy, "languageTag">> = {
    en: {
        ...english.translations,
        portalLanguage: "en",
        instructions: {
            "login.ftl":
                "This is the sign-in page. Type your username in the first field and your password in the second. Use the Tab key to move between the fields. Then go to the button to sign in and press Enter. If something is wrong, a message above the fields says what.",
            "login-username.ftl":
                "This is the sign-in page. Type your username in the field. Then go to the button to continue and press Enter. If something is wrong, a message above the field says what.",
            "message-otp.login.ftl":
                "We have sent you a code. Type it in the field, one digit after another. Then go to the button to continue and press Enter. If the code does not arrive, a button lets you ask for a new one after a short wait.",
        },
    },
    es: {
        ...spanish.translations,
        portalLanguage: "es",
        instructions: {
            "login.ftl":
                "Esta es la página de inicio de sesión. Escriba su nombre de usuario en el primer campo y su contraseña en el segundo. Use la tecla Tabulador para moverse entre los campos. Después vaya al botón para iniciar sesión y pulse Intro. Si algo no es correcto, un mensaje encima de los campos lo indica.",
            "login-username.ftl":
                "Esta es la página de inicio de sesión. Escriba su nombre de usuario en el campo. Después vaya al botón para continuar y pulse Intro. Si algo no es correcto, un mensaje encima del campo lo indica.",
            "message-otp.login.ftl":
                "Le hemos enviado un código. Escríbalo en el campo, una cifra tras otra. Después vaya al botón para continuar y pulse Intro. Si el código no llega, un botón le permite pedir otro tras una breve espera.",
        },
    },
    ca: {
        ...catalan.translations,
        portalLanguage: "cat",
        instructions: {
            "login.ftl":
                "Aquesta és la pàgina d'inici de sessió. Escriviu el nom d'usuari al primer camp i la contrasenya al segon. Feu servir la tecla Tabulador per moure-us entre els camps. Després aneu al botó per iniciar la sessió i premeu Retorn. Si alguna cosa no és correcta, un missatge sobre els camps ho indica.",
            "login-username.ftl":
                "Aquesta és la pàgina d'inici de sessió. Escriviu el nom d'usuari al camp. Després aneu al botó per continuar i premeu Retorn. Si alguna cosa no és correcta, un missatge sobre el camp ho indica.",
            "message-otp.login.ftl":
                "Us hem enviat un codi. Escriviu-lo al camp, una xifra rere l'altra. Després aneu al botó per continuar i premeu Retorn. Si el codi no arriba, un botó us permet demanar-ne un altre després d'una breu espera.",
        },
    },
    eu: {
        ...basque.translations,
        portalLanguage: "eu",
        instructions: {
            "login.ftl":
                "Hau saioa hasteko orria da. Idatzi zure erabiltzaile-izena lehen eremuan eta pasahitza bigarrenean. Erabili Tabulazio tekla eremuen artean mugitzeko. Gero joan saioa hasteko botoira eta sakatu Sartu. Zerbait oker badago, eremuen gaineko mezu batek adierazten du.",
            "login-username.ftl":
                "Hau saioa hasteko orria da. Idatzi zure erabiltzaile-izena eremuan. Gero joan jarraitzeko botoira eta sakatu Sartu. Zerbait oker badago, eremuaren gaineko mezu batek adierazten du.",
            "message-otp.login.ftl":
                "Kode bat bidali dizugu. Idatzi eremuan, zifra bat bestearen atzetik. Gero joan jarraitzeko botoira eta sakatu Sartu. Kodea iristen ez bada, botoi batek beste bat eskatzeko aukera ematen dizu itxaronaldi labur baten ondoren.",
        },
    },
    fr: {
        ...french.translations,
        portalLanguage: "fr",
        instructions: {
            "login.ftl":
                "Voici la page de connexion. Saisissez votre nom d'utilisateur dans le premier champ et votre mot de passe dans le second. Utilisez la touche Tabulation pour passer d'un champ à l'autre. Allez ensuite au bouton de connexion et appuyez sur Entrée. En cas d'erreur, un message au-dessus des champs l'indique.",
            "login-username.ftl":
                "Voici la page de connexion. Saisissez votre nom d'utilisateur dans le champ. Allez ensuite au bouton pour continuer et appuyez sur Entrée. En cas d'erreur, un message au-dessus du champ l'indique.",
            "message-otp.login.ftl":
                "Nous vous avons envoyé un code. Saisissez-le dans le champ, un chiffre après l'autre. Allez ensuite au bouton pour continuer et appuyez sur Entrée. Si le code n'arrive pas, un bouton vous permet d'en demander un nouveau après une courte attente.",
        },
    },
    gl: {
        ...galician.translations,
        portalLanguage: "gl",
        instructions: {
            "login.ftl":
                "Esta é a páxina de inicio de sesión. Escriba o seu nome de usuario no primeiro campo e o seu contrasinal no segundo. Use a tecla Tabulador para moverse entre os campos. Despois vaia ao botón para iniciar sesión e prema Intro. Se algo non é correcto, unha mensaxe enriba dos campos indícao.",
            "login-username.ftl":
                "Esta é a páxina de inicio de sesión. Escriba o seu nome de usuario no campo. Despois vaia ao botón para continuar e prema Intro. Se algo non é correcto, unha mensaxe enriba do campo indícao.",
            "message-otp.login.ftl":
                "Enviámoslle un código. Escríbao no campo, unha cifra tras outra. Despois vaia ao botón para continuar e prema Intro. Se o código non chega, un botón permítelle pedir outro tras unha breve espera.",
        },
    },
    nl: {
        ...dutch.translations,
        portalLanguage: "nl",
        instructions: {
            "login.ftl":
                "Dit is de aanmeldpagina. Typ uw gebruikersnaam in het eerste veld en uw wachtwoord in het tweede. Gebruik de Tab-toets om tussen de velden te bewegen. Ga daarna naar de knop om aan te melden en druk op Enter. Als er iets niet klopt, staat dat in een bericht boven de velden.",
            "login-username.ftl":
                "Dit is de aanmeldpagina. Typ uw gebruikersnaam in het veld. Ga daarna naar de knop om verder te gaan en druk op Enter. Als er iets niet klopt, staat dat in een bericht boven het veld.",
            "message-otp.login.ftl":
                "We hebben u een code gestuurd. Typ de code in het veld, cijfer na cijfer. Ga daarna naar de knop om verder te gaan en druk op Enter. Als de code niet aankomt, kunt u na een korte wachttijd met een knop een nieuwe aanvragen.",
        },
    },
    tl: {
        ...tagalog.translations,
        portalLanguage: "tl",
        instructions: {
            "login.ftl":
                "Ito ang pahina ng pag-sign in. I-type ang iyong username sa unang field at ang iyong password sa pangalawa. Gamitin ang Tab key upang lumipat sa mga field. Pagkatapos ay pumunta sa button para mag-sign in at pindutin ang Enter. Kung may mali, sasabihin ito ng mensahe sa itaas ng mga field.",
            "login-username.ftl":
                "Ito ang pahina ng pag-sign in. I-type ang iyong username sa field. Pagkatapos ay pumunta sa button para magpatuloy at pindutin ang Enter. Kung may mali, sasabihin ito ng mensahe sa itaas ng field.",
            "message-otp.login.ftl":
                "Nagpadala kami sa iyo ng code. I-type ito sa field, isa-isang digit. Pagkatapos ay pumunta sa button para magpatuloy at pindutin ang Enter. Kung hindi dumating ang code, may button para humiling ng bago pagkalipas ng maikling paghihintay.",
        },
    },
}

/** The copy for a login language, falling back to English like the rest of the layout. */
export function getAccessibilityCopy(languageTag: string): AccessibilityCopy {
    const language = languageTag.split("-")[0].toLowerCase()
    const translated = Object.prototype.hasOwnProperty.call(translations, language)
    return {
        ...translations[translated ? language : "en"],
        languageTag: translated ? languageTag : "en",
    }
}

export const isInstructionsPage = (pageId: string): pageId is InstructionsPageId =>
    pageId === "login.ftl" || pageId === "login-username.ftl" || pageId === "message-otp.login.ftl"
