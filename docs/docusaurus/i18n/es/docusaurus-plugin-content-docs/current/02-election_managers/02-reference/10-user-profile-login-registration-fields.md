---
id: user_profile_login_registration_fields
title: Configurar los campos de inicio de sesión y registro
sidebar_position: 10
description: "Los campos de la página de registro y de la página de inicio de sesión basada en atributos se generan a partir de la configuración del perfil de usuario (User Profile) del realm en Keycloak."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Los campos de la página de registro y de la página de inicio de sesión basada en atributos se generan a partir de la configuración de **Perfil de usuario** (User Profile) del realm en Keycloak. Cada atributo del perfil de usuario produce un campo de formulario; las anotaciones del atributo determinan cómo se muestra ese campo. Ambas páginas usan la misma configuración.

Para crear un atributo, consulte [Añadir atributos de usuario a Keycloak](../01-tutorials/99-admin_portal_tutorials_add-user-attributes-to-keycloak.md).

---

## Dónde está la configuración {#where-the-configuration-lives}

**Realm settings** → **User profile** → seleccione o cree un atributo.

Los ajustes que se aplican a toda la página, y no a un solo campo, son **atributos del realm**. Se editan en el portal de administración, en los **Keycloak realm attributes** del proceso electoral. Ese editor muestra los atributos que tiene un realm, no los que admite, así que la configuración predeterminada del realm del proceso electoral de cada entorno debe incluirlos con sus valores por defecto; de lo contrario, un administrador electoral no tendría forma de descubrirlos:

| Atributo del realm             | Valor por defecto     |
| ------------------------------ | --------------------- |
| `credential-field-position`    | `LAST`                |
| `credential-input-policy`      | `standard`            |
| `credential-input-pattern`     | `dddd-dddd-dddd-dddd` |
| `credential-input-placeholder` | `d`                   |
| `login-validation-policy`      | `BROWSER`             |
| `voter-certificate-policy`     | `disabled`            |

Un atributo que no tiene valor se comporta como su valor por defecto, así que incluirlos no cambia nada, salvo lo que muestra el editor.

| Grupo de ajustes                              | Controla                                                                 |
| --------------------------------------------- | ------------------------------------------------------------------------ |
| **General** (nombre, nombre para mostrar, obligatorio) | Identidad del campo, etiqueta y si es obligatorio                |
| **Annotations**                               | Tipo de entrada, marcador de posición, texto de ayuda, límites, etiquetas de opciones |
| **Validations**                              | Reglas del servidor y lista de opciones de los campos de elección        |

Los nombres de atributo deben coincidir con los que se usan en los archivos de importación de votantes, en la pestaña Votantes del portal de administración y en la lista **User attributes to match** del autenticador. Cambiar el nombre de un atributo no lo cambia en los registros de usuario que ya existen.

### Atributos secretos del votante {#secret-voter-attributes}

La anotación `sequent.secret` clasifica un atributo de votante del proceso electoral para su almacenamiento cifrado. Es una función de salida a nivel de portal de administración y de votante; no es un tipo de entrada para las páginas de registro o de inicio de sesión. Como Keycloak solo ve su envoltorio cifrado, un atributo secreto no debe mostrarse a los votantes ni usarse para la coincidencia en el inicio de sesión, el registro, la asignación de tokens, las comprobaciones de unicidad ni la gestión autónoma del votante.

Configure los campos secretos y su ámbito solo para administradores como se describe en [Proteger un atributo de votante como secreto](../01-tutorials/99-admin_portal_tutorials_add-user-attributes-to-keycloak.md#protecting-a-voter-attribute-as-secret).

---

## Qué páginas usan qué atributos {#which-pages-use-which-attributes}

| Página | Atributos que se muestran |
|---|---|
| **Registro** | Todos los atributos del perfil de usuario incluidos en el ámbito del registro, en el orden en que los muestra el perfil de usuario |
| **Inicio de sesión**, nombre de usuario y contraseña estándar | Ninguno: solo nombre de usuario y contraseña |
| **Inicio de sesión**, basado en atributos (**Multi-Attribute + Password Form**) | Solo los atributos de **User attributes to match** del autenticador |

Para configurar el inicio de sesión basado en atributos, incluidos sus ajustes de coincidencia, de ambigüedad y de control de abusos, consulte [Iniciar sesión sin nombre de usuario (atributo + contraseña)](../01-tutorials/101-admin_portal_tutorials_multi-attribute-password-login.md).

Un atributo de **User attributes to match** que no esté declarado en el perfil de usuario se muestra como un campo de texto simple. Su nombre se usa como clave de mensaje y, si no existe una traducción, se muestra el nombre sin procesar. Siempre es obligatorio para la coincidencia.

La coincidencia compara un valor por atributo. En la página de inicio de sesión basada en atributos, una declaración `multiselect` se muestra, por tanto, como una lista de selección única, y `multiselect-checkboxes` se muestra como botones de radio. El registro mantiene el control de valores múltiples configurado.

---

## Tipos de entrada {#input-types}

Configure la anotación `inputType` (**Input type** en la consola).

| `inputType` | Se muestra como | Dónde se aplica |
|---|---|---|
| *(no definido)* o `text` | Campo de texto simple | Ambas |
| `textarea` | Cuadro de texto de varias líneas | Ambas |
| `select` | Lista desplegable con una primera opción vacía | Ambas |
| `multiselect` | Lista de selección múltiple | Solo registro |
| `select-radiobuttons` | Un botón de radio por opción | Ambas |
| `multiselect-checkboxes` | Una casilla de verificación por opción | Solo registro |
| `html5-date` | Selector de fecha nativo | Ambas |
| `html5-tel` | Widget de número de teléfono con selector de país | Ambas |
| `html5-email`, `html5-number`, `html5-url`, … | El campo HTML5 nativo correspondiente | Ambas |

Cualquier valor con el prefijo `html5-` se convierte en ese tipo de entrada HTML5.

**Los valores de fecha deben almacenarse como `YYYY-MM-DD`** (p. ej., `1990-01-05`). La coincidencia en el inicio de sesión falla si los datos importados usan otro formato.

---

## Etiquetas y traducciones {#labels-and-translations}

**Display name** es la etiqueta que aparece encima del campo:

- **Texto literal** (`Date of birth`): es el mismo en todos los idiomas.
- **Una clave de traducción** (`${dateOfBirth}`): se resuelve para cada idioma.

Defina las claves de traducción en **Realm settings** → **Localization** → **Realm overrides**.

---

## Texto de ayuda {#helper-text}

| Anotación | Efecto | Dónde se aplica |
|---|---|---|
| `inputHelperTextBefore` | Texto encima del campo, debajo de la etiqueta | Ambas |
| `inputHelperTextAfter` | Texto debajo del campo | Ambas |

Ambas aceptan texto literal o una clave de traducción, y ambas se muestran en la página de registro y en la página de inicio de sesión basada en atributos.

---

## Ajustes de texto y número {#text-and-number-settings}

| Anotación | Efecto | Dónde se aplica |
|---|---|---|
| `inputTypePlaceholder` | Texto de marcador de posición | Ambas |
| `inputTypePattern` | Expresión regular que aplica el navegador | Ambas |
| `inputTypeSize` | Anchura visible en caracteres | Ambas |
| `inputTypeMinlength` / `inputTypeMaxlength` | Mínimo / máximo de caracteres | Ambas |
| `inputTypeMin` / `inputTypeMax` | Valor o fecha mínimos / máximos | Ambas |
| `inputTypeStep` | Incremento para campos numéricos y de fecha | Ambas |
| `inputTypeCols` / `inputTypeRows` | Dimensiones del área de texto | Ambas |

Los campos de fecha tienen un límite de `9999-12-31`, salvo que `inputTypeMax` establezca otro máximo. Los ajustes del navegador son una ayuda de uso; use **Validations** para las reglas que deban cumplirse siempre.

---

## Listas de opciones {#option-lists}

Para `select`, `multiselect`, `select-radiobuttons` y `multiselect-checkboxes`, las opciones proceden de un validador:

1. En **Validations**, haga clic en **Add validator** → `options`.
2. Añada cada valor de opción (p. ej., `M`, `F`).

El valor de la opción es el que se almacena. Para mostrar otra cosa:

| Anotación | Efecto | Dónde se aplica |
|---|---|---|
| `inputOptionLabels` | Mapa de valor de opción → etiqueta, p. ej., `{"M": "Male", "F": "Female"}`. Los valores pueden ser claves de traducción. | Ambas |
| `inputOptionLabelsI18nPrefix` | Cada opción se traduce como `prefijo.valorDeOpción` (p. ej., `sex.M`) | Ambas |
| `inputOptionsFromValidation` | Toma las opciones de otro validador con nombre | Ambas |

### Desplegables dependientes {#dependent-dropdowns}

`filterSelectAttribute`, configurado en el atributo que controla con el ID del campo dependiente como valor, restringe el desplegable dependiente a las opciones cuyo valor contiene el valor seleccionado, y preselecciona la primera coincidencia. Los valores de opción deben incluir el valor controlador (provincia `08`, municipios `08001`, `08002`). En el inicio de sesión basado en atributos, el campo dependiente también debe estar en **User attributes to match**.

### Activar y desactivar otros campos {#enabling-and-disabling-other-fields}

En los atributos de botones de radio y de casillas de verificación:

| Anotación | Efecto | Dónde se aplica |
|---|---|---|
| `disableAttribute` | Marcar la opción hace que el campo indicado sea de solo lectura y lo vacía | Solo registro |
| `disableElement` | Marcar la opción desactiva el campo indicado | Solo registro |

---

## Otras anotaciones {#other-annotations}

| Anotación | Efecto | Dónde se aplica |
|---|---|---|
| `default` | Valor inicial. Solo registro; se ignora en el formulario de inicio de sesión basado en atributos. | Solo registro |
| `confirm` | Muestra un segundo campo para el mismo atributo. El valor es la etiqueta del campo de confirmación. | Solo registro |
| `html-attribute:<name>` | Establece un atributo HTML arbitrario, p. ej., `html-attribute:autocomplete` = `bday`. Se aplica solo a campos de texto y HTML5; los desplegables, los botones de radio, las casillas de verificación y las áreas de texto lo ignoran. | Solo registro |
| `hidden` = `true` | El atributo no se muestra en la página de registro. Sigue almacenándose y sigue pudiendo usarse para la coincidencia. | Solo registro |

El ajuste **Hidden Profile Attributes** de la acción de formulario **Deferred Registration User Creation** oculta del formulario de registro una lista de atributos separados por comas, y los ignora si el perfil de usuario los marca como obligatorios.

---

## Autorrelleno y la página de inicio de sesión {#autofill-and-the-login-page}

La página de inicio de sesión basada en atributos establece `autocomplete="off"` en todos los campos de coincidencia e ignora las anotaciones `html-attribute:autocomplete` del perfil de usuario. Además, los valores de coincidencia fallidos no se vuelven a colocar en la página. Estas son protecciones deliberadas para dispositivos compartidos, en campos como la fecha de nacimiento, el documento de identidad nacional y el número de teléfono. Los navegadores pueden seguir aplicando sus propias heurísticas de autorrelleno, así que `autocomplete="off"` es una petición y no una garantía de privacidad.

---

## Campos de número de teléfono {#phone-number-fields}

`inputType` = `html5-tel` muestra un widget internacional de número de teléfono: selector de país con prefijo, formato de marcador de posición por país y un país inicial que se deduce de la zona horaria del navegador. El valor enviado es el número internacional completo, almacenado con el nombre propio del atributo. Si se configura `inputTypePattern`, el navegador lo comprueba contra ese número completo normalizado, y no contra el valor en formato nacional que se ve junto al prefijo independiente.

---

## Campos obligatorios {#required-fields}

El interruptor **Required field** de un atributo lo marca como obligatorio. En la página de registro, un atributo obligatorio muestra un asterisco y lo exigen tanto el navegador como el servidor.

En la página de inicio de sesión basada en atributos, el ajuste **Honor User Profile required attributes** del autenticador regula los campos de **User attributes to match**:

| Ajuste | Comportamiento |
|---|---|
| **Desactivado** (por defecto) | Todos los atributos de la lista son obligatorios |
| **Activado** | Cada campo sigue su propio ajuste **Required field**. Los campos no obligatorios pueden quedar en blanco, y la coincidencia se realiza con los atributos que se hayan rellenado |

Un envío con todos los campos en blanco siempre falla. Un atributo que no está declarado en el perfil de usuario sigue siendo obligatorio.

Los asteriscos y el aviso de «Campos obligatorios» solo aparecen cuando al menos un campo es opcional, ya que marcar todos los campos no distinguiría ninguno. Cuando aparecen, también se marca la contraseña o el PIN.

> ⚠️ Activar esta opción amplía quién puede coincidir con un intento de inicio de sesión: si deja `nationalId` en blanco, coincidirá con todos los votantes con el `dateOfBirth` enviado, y la contraseña será el único factor que limite la búsqueda. Consulte [Consideraciones sobre denegación de servicio](../01-tutorials/101-admin_portal_tutorials_multi-attribute-password-login.md#denial-of-service-considerations).

---

## Valores prerrellenados desde un enlace de inicio de sesión {#prefilled-values-from-a-login-link}

Las sugerencias de inicio de sesión se rigen por dos ajustes. La **Política de parámetros de prerrelleno** de la acción de formulario **Deferred Registration User Creation** es el interruptor principal:

- `IGNORE` (por defecto): las sugerencias de inicio de sesión nunca prerrellenan los campos.
- `ACCEPT`: las sugerencias validadas pueden prerrellenar los campos, según la política de cada atributo.

Para cada atributo, la anotación `loginHintPrefillPolicy`:

| Valor | Efecto | Dónde se aplica |
|---|---|---|
| *(no definido)* o `EDITABLE` | Se prerrellena, y el votante puede cambiarlo | Solo registro |
| `READ_ONLY` | Se prerrellena y se bloquea; un valor modificado se rechaza | Solo registro |
| `IGNORE` | Nunca se prerrellena | Solo registro |

Un valor bloqueado se sigue enviando, incluso en los desplegables, los botones de radio y las casillas de verificación. Los campos de contraseña, los atributos anotados como `hidden` y los atributos que las anotaciones `html-attribute:` o los estilos en línea hacen invisibles o no editables nunca se prerrellenan.

En la página de inicio de sesión, el campo de nombre de usuario se rige en cambio por el atributo del realm `loginHintUsernamePolicy` (`EDITABLE` o `READ_ONLY`).

---

## Validación {#validation}

Las anotaciones y los validadores son mecanismos distintos y se aplican en lugares diferentes.

| Declarado como | Qué hace | Formulario de registro | Formulario de inicio de sesión |
|---|---|---|---|
| Anotación (`inputTypePattern`, `inputTypeMaxlength`, `inputTypeMin`/`Max`) | Se muestra como atributo HTML y lo comprueba el navegador antes de enviar | Se aplica | Se aplica |
| Validador (`length`, `pattern`, `email`, `options`, …) | Se comprueba en el servidor después de enviar | Se aplica, con un mensaje junto al campo | **Se ignora** |

La página de inicio de sesión basada en atributos nunca ejecuta los validadores del perfil de usuario: compara los valores enviados con los registros de votantes y responde con un único mensaje genérico. Un validador declarado en un atributo de coincidencia no tiene ningún efecto allí: ni restringe la entrada ni la rechaza.

Solo la anotación limita lo que se puede escribir. Un validador `length` con `max: 9` permite al votante escribir cualquier número de caracteres y rechaza el valor después; `inputTypeMaxlength` = `9` detiene la entrada en nueve caracteres. Para limitar la escritura *y* hacer cumplir el límite, declare ambos.

El validador `options` es la excepción: ambos formularios lo leen para construir las opciones de los campos `select`, de radio y de casillas de verificación, así que es necesario para mostrarlos en ambos.

### Quién informa de los formatos no válidos en la página de inicio de sesión {#who-reports-invalid-formats-on-the-login-page}

| `login-validation-policy` | Efecto |
|---|---|
| `BROWSER` (por defecto) | El navegador comprueba las restricciones de los atributos y bloquea el envío, mostrando su propio mensaje junto al campo incorrecto |
| `SERVER_ONLY` | El formulario incluye `novalidate`; el envío siempre llega al autenticador, que responde con el mensaje genérico |

Los mensajes del navegador provienen del idioma del navegador, no del realm, y solo se informa de un campo cada vez. `SERVER_ONLY` los elimina. En ambos casos, las anotaciones se mantienen: los campos obligatorios siguen anunciándose a las tecnologías de apoyo, los límites de escritura siguen aplicándose y el selector de fechas sigue acotado.

El PIN estructurado valida su propio formato en ambos modos, en el idioma de la página, junto al campo.

---

## El campo de contraseña y PIN {#the-password-and-pin-field}

El campo de contraseña no es un atributo del perfil de usuario; ambas páginas lo añaden por su cuenta. Estas anotaciones se aplican al formulario de registro, donde el cuadro de contraseña se coloca debajo de un atributo elegido:

| Anotación | Efecto | Dónde se aplica |
|---|---|---|
| `showPasswordAfterThis` | Posición del cuadro de contraseña. Por defecto, después de `username` (o de `email` cuando se usa como nombre de usuario). Configure `false` en ese atributo para suprimirlo, o `true` en otro atributo para moverlo. Siempre prevalece sobre `credential-field-position`. | Solo registro |
| `passwordHelperTextBefore` / `passwordHelperTextAfter` | Texto encima / debajo del cuadro de contraseña | Solo registro |
| `passwordStrengthBar` | Indicador de seguridad de la contraseña. Solo registro. | Solo registro |

### Colocar la credencial primero {#putting-the-credential-first}

Por defecto, el cuadro de contraseña o PIN se coloca después de los campos de identidad. Configure el atributo del realm `credential-field-position` como `FIRST` para colocarlo encima de ellos, con el foco inicial en él; resulta útil cuando lo principal que se introduce es un PIN de la carta de votante.

| Valor | Efecto |
|---|---|
| `LAST` (por defecto) | La credencial va después de los campos, con el foco en el primer campo |
| `FIRST` | La credencial va antes de los campos, con el foco en la credencial |

Se aplica a la página de inicio de sesión basada en atributos y al formulario de registro, no a la página ordinaria de inicio de sesión con nombre de usuario y contraseña. En el formulario de registro, un atributo que declara `showPasswordAfterThis` mantiene su posición y este ajuste se ignora.

### Entrada de PIN estructurado {#structured-pin-input}

Muestra el cuadro de contraseña como grupos de dígitos de longitud fija. Sigue siendo la contraseña ordinaria de Keycloak del votante, y se aplica a la página de inicio de sesión y al formulario de registro en modo de inicio de sesión con un campo de contraseña. Configure `credential-input-policy` como `structured` (o su alias equivalente `pattern`) para activarlo; el patrón, el marcador de posición, el comportamiento de la entrada y los pasos de implantación se describen en [Inicio de sesión con PIN estructurado](../../../../../../docs/07-developers/06-keycloak/structured_pin_login.md).

---

## Otras funciones que cambian estas páginas {#other-features-that-change-these-pages}

### Página de inicio de sesión {#login-page}

| Función | Se configura con | Efecto |
|---|---|---|
| [Inicio de sesión basado en atributos](../01-tutorials/101-admin_portal_tutorials_multi-attribute-password-login.md) | Autenticador **Multi-Attribute + Password Form** | Sustituye el campo de nombre de usuario por campos de atributos del perfil de usuario |
| [PIN estructurado](../../../../../../docs/07-developers/06-keycloak/structured_pin_login.md) | Atributos del realm `credential-input-*` | El cuadro de contraseña se convierte en grupos de dígitos de longitud fija |
| Nombre de usuario bloqueado | Atributo del realm `loginHintUsernamePolicy` = `READ_ONLY` | El nombre de usuario se prerrellena y es de solo lectura |
| [Inicio de sesión con certificado digital](../../../../../../docs/07-developers/06-keycloak/x509_client_cert_architecture.md) | Atributo del realm `voter-certificate-policy` = `enabled` | Muestra el botón del proveedor **digital-certificates**; permanece oculto mientras esté `disabled` |
| Otros proveedores de identidad | **Proveedores de identidad** de Keycloak | Cada proveedor activado añade un botón debajo del formulario |
| [OID4VP (cartera digital)](../../../../../../docs/07-developers/06-keycloak/oid4vp_testing_guide.md) | Proveedor de identidad OID4VP | Usa su propia página de código QR / cartera |
| [SSO iniciado por el IdP](../../../../../../docs/07-developers/06-keycloak/idp_initiated_sso_design_implementation.md) | Proveedor de identidad SAML/OIDC | Se omite la página de inicio de sesión |
| Recordarme | Ajuste del realm **Remember me** | Añade la casilla de verificación |
| Olvidé la contraseña | Ajuste del realm **Forgot password** | Añade el enlace de restablecimiento |
| Enlace de registro | Ajuste del realm **User registration** | Añade el enlace de registro |
| Inicio de sesión con correo electrónico | Ajustes del realm **Login with email** / **Email as username** | Cambia la etiqueta del campo de nombre de usuario |
| reCAPTCHA | Autenticador reCAPTCHA de Keycloak | reCAPTCHA invisible al enviar |

### Página de registro {#registration-page}

| Función | Se configura con | Efecto |
|---|---|---|
| Modo de inicio de sesión | **Deferred Registration User Creation**, **Form Mode** = `LOGIN` | Se muestra como un formulario de inicio de sesión: sin confirmación de contraseña, sin indicador de seguridad y sin enlace de vuelta al inicio de sesión |
| Campo de contraseña | Misma acción de formulario, **Password Required** | Si se muestra un cuadro de contraseña |
| Atributos ocultos | Misma acción de formulario, **Hidden Profile Attributes** | Elimina los atributos indicados |
| Prerrelleno desde el enlace de inicio de sesión | Misma acción de formulario, **Política de parámetros de prerrelleno** | Consulte [Valores prerrellenados](#prefilled-values-from-a-login-link) |
| Términos y condiciones | Acción requerida de Keycloak **Términos y condiciones** | Añade la casilla de aceptación |
| reCAPTCHA | Autenticador reCAPTCHA de Keycloak | Añade el widget de reCAPTCHA |
| Proveedores de identidad | **Proveedores de identidad** de Keycloak | Los botones de proveedor se muestran solo en modo de inicio de sesión |

### Ambas páginas {#both-pages}

| Función | Se configura con | Efecto |
|---|---|---|
| [Idiomas](./06-Languages.md) | Atributos del realm `language_detection_policy`, `forced_language_code` y los idiomas habilitados | Idioma inicial y selector de idioma |
| Etiquetas y mensajes | **Realm settings** → **Localization** → **Realm overrides** | Cualquier etiqueta o mensaje, por idioma |
| Imagen de marca | Ajustes del tema del proceso electoral | Logotipo, colores y maquetación |
