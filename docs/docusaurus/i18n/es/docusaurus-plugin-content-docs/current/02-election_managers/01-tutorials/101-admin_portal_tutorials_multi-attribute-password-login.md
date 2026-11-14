---
id: admin_portal_tutorials_multi_attribute_password_login
title: Iniciar sesión sin nombre de usuario (atributo + contraseña o secreto)
description: "De forma predeterminada, los votantes inician sesión con un nombre de usuario y una contraseña. Algunas elecciones identifican a los votantes con atributos que ya conocen, como una fecha de nacimiento o un documento de identidad, sin pedirles que recuerden un…"
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Descripción general {#overview}

De forma predeterminada, los votantes inician sesión con un nombre de usuario y una contraseña. Algunas elecciones identifican a los votantes con atributos que ya conocen, como una fecha de nacimiento o un documento de identidad, sin pedirles que recuerden un nombre de usuario aparte. Este tutorial explica cómo configurar el autenticador **Multi-Attribute + Password Form** para que los votantes inicien sesión con uno o más atributos de usuario configurados y una contraseña, en lugar de un nombre de usuario. El campo de contraseña puede verificar la contraseña normal de Keycloak (la predeterminada) o un atributo de votante cifrado configurado explícitamente.

El autenticador busca todos los usuarios cuyos atributos configurados coincidan con los valores enviados (**todos** los atributos configurados deben coincidir con el mismo usuario) y después comprueba la contraseña enviada frente a esos candidatos. Con la política de coincidencia predeterminada `REJECT_AMBIGUOUS`, el inicio de sesión solo tiene éxito cuando **exactamente un** candidato tiene una credencial que coincide y esa cuenta está habilitada y no bloqueada.

Un solo atributo, como la fecha de nacimiento, no es único por sí mismo (muchos votantes comparten una fecha de nacimiento). Aun así, el método funciona: el autenticador reúne todos los usuarios con esa fecha de nacimiento como candidatos y se apoya en la contraseña para identificar de forma única a uno de ellos. Si configura un segundo atributo (por ejemplo, también un documento de identidad), se reduce el conjunto de candidatos antes de la comprobación de la contraseña. Se recomienda hacerlo siempre que disponga de un segundo atributo identificativo.

---

## Requisitos previos {#prerequisites}

### Opcional: use un atributo cifrado del votante como credencial {#optional-use-an-encrypted-voter-attribute-as-the-credential}

La **Credential verification policy** predeterminada es `PASSWORD`; los flujos existentes no cambian. Para autenticarse con un campo secreto del votante en lugar de una contraseña de Keycloak:

1. Configure un atributo del perfil de usuario de Keycloak, por ejemplo `mysecret`, con `sequent.secret=true`, y rellénelo con el editor de votantes autorizado o con el importador CSV. No introduzca el texto sin cifrar directamente en el editor de atributos de Keycloak.
2. Proporcione la misma clave maestra hexadecimal de 64 caracteres que usan Harvest y Windmill en la variable de entorno `MASTER_SECRET` de Keycloak. Las configuraciones de Compose suministradas pasan la única variable de despliegue `MASTER_SECRET` a los tres servicios. Si Harvest y Windmill usan un almacén de secretos externo, inyecte en Keycloak la clave maestra existente de ese almacén, no una clave generada nueva. Recree Keycloak después de provisionar la variable de entorno. Recibir la clave no activa por sí solo el inicio de sesión con atributo cifrado; la verificación `PASSWORD` normal sigue siendo la predeterminada.
3. En la configuración de este autenticador, establezca **Credential verification policy** en `SECRET_ATTRIBUTE` y **Encrypted credential attribute** en `mysecret`. Mantenga los atributos identificativos habituales en **User attributes to match**, no el atributo secreto.
4. Mantenga **Multiple-candidate match policy** en `REJECT_AMBIGUOUS`. En este modo no se acepta `FIRST_MATCH`. Para IVR, establezca las mismas dos opciones en **Multi-Attribute + Password Direct Grant**; su entrada existente `kind=secret`/`maps_to=password` transporta esta credencial.

Por ejemplo, para pedir una fecha de nacimiento y usar `mysecret` como contraseña:

| Ajuste del autenticador | Valor |
|---|---|
| User attributes to match | `dateOfBirth` |
| Credential verification policy | `SECRET_ATTRIBUTE` |
| Encrypted credential attribute | `mysecret` |
| Multiple-candidate match policy | `REJECT_AMBIGUOUS` |

Use el **nombre** exacto del atributo, no su etiqueta visible. El secreto es la credencial, no un atributo de búsqueda: no añada `mysecret` a **User attributes to match**. La [guía de configuración de atributos secretos](./99-admin_portal_tutorials_add-user-attributes-to-keycloak.md#protecting-a-voter-attribute-as-secret) explica la anotación necesaria, los permisos y las restricciones de los validadores.

El formulario sigue siendo `login.ftl`, y los votantes introducen la credencial en el campo existente `password`. La ubicación de la credencial, el botón para mostrar u ocultar, el enfoque automático y `credential-input-policy: "pattern"` conservan su comportamiento actual. El formato por patrón es solo presentación: almacene la credencial exacta que se envía (para los patrones numéricos, los dígitos, no los separadores visuales). No añada un segundo campo. En este modo no se necesita ni se consulta una contraseña de Keycloak; las operaciones de restablecimiento de contraseña y de política de contraseñas siguen gestionando la contraseña de Keycloak independiente y no sustituyen este atributo cifrado.

La verificación descifra solo el atributo seleccionado de un número limitado de candidatos, vincula cada envoltorio al tenant, al proceso electoral, al votante y al atributo, compara resúmenes SHA-256 de tamaño fijo en tiempo constante y examina todos los valores almacenados. No recorta ni cambia las mayúsculas o minúsculas de la credencial. Los envoltorios ausentes, mal formados o alterados, las claves ausentes o incorrectas, las cuentas deshabilitadas o bloqueadas y las coincidencias ambiguas fallan de forma segura, sin recurrir a la contraseña. Se aceptan como máximo 100 valores almacenados por atributo y 150 bytes UTF-8 por credencial enviada. El limitador de intentos por tupla existente y la protección de Keycloak contra fuerza bruta siguen activos; las credenciales adivinadas nunca se incluyen en la clave del limitador por tupla.

Trate este campo como una credencial de inicio de sesión: cualquiera autorizado a ver o exportar su valor puede suplantar a su votante. Elija valores de alta entropía, restrinja el acceso de lectura y escritura a los secretos, active la protección del realm contra fuerza bruta y evite enviarlo en vistas previas de informes, registros o canales de mensajería no fiables. El correo electrónico y el SMS siguen siendo legibles; declarar un secreto no cifra el mensaje. El [canal de prueba de consola](../02-reference/user-manual/templates/admin_portal_reference_user-manual_templates.md#test-only-console-delivery) explícito también muestra los valores descifrados, así que use solo credenciales sintéticas cuando ese canal esté habilitado.

### Requisitos previos estándar {#standard-prerequisites}

- Acceso a la consola de administración de Keycloak.
- La extensión `sequent.message-otp-authenticator.jar` desplegada en el directorio `providers/` de Keycloak (incluida de forma predeterminada en la imagen Docker de Keycloak de Sequent).
- El atributo o los atributos de usuario con los que quiere iniciar sesión deben existir ya en el **User Profile** del realm. Consulte [Añadir atributos de usuario a Keycloak](./99-admin_portal_tutorials_add-user-attributes-to-keycloak.md). Si un atributo está configurado allí con **Input type** `html5-date` (por ejemplo, un campo de fecha de nacimiento), el formulario de inicio de sesión muestra automáticamente también un selector de fecha nativo, igual que el que ven los votantes en el registro. La mayoría de las demás anotaciones del perfil de usuario se aplican del mismo modo; las excepciones específicas de cada página se indican en [Configurar los campos de inicio de sesión y registro](../02-reference/10-user-profile-login-registration-fields.md).
- **Los atributos con valor de fecha deben almacenarse como `AAAA-MM-DD`** (por ejemplo, `1990-01-05` para el 5 de enero de 1990). Es el mismo formato que envía siempre el selector de fecha nativo del navegador, por lo que no hace falta reformatearlo en el camino del navegador.

---

## Paso 1 – Cree un flujo de navegador con el nuevo autenticador {#step-1--create-a-browser-flow-with-the-new-authenticator}

1. Vaya a **Authentication** → **Flows**.
2. Duplique un flujo de navegador existente (por ejemplo, el flujo `browser` predeterminado del realm) o cree uno nuevo, y asígnele un nombre descriptivo, como `attribute password login`.
3. Dentro del subflujo adecuado, haga clic en **Add step**.
4. Busque **Multi-Attribute + Password Form** y añádalo.
5. Establezca el requisito en **ALTERNATIVE** (para ofrecerlo junto con otros métodos de inicio de sesión que ya existan en el flujo) o en **REQUIRED** (para que sea la única forma de iniciar sesión en ese flujo).

---

## Paso 2 – Configure los atributos que se van a comparar {#step-2--configure-the-attributes-to-match}

1. Haga clic en **⚙ Config** (icono de engranaje) junto al nuevo paso.
2. Asigne a la configuración un alias descriptivo, por ejemplo `attribute-login-dateOfBirth`.
3. En **User attributes to match**, haga clic en **+ Add** e introduzca cada nombre de atributo de usuario que se debe exigir, por ejemplo `dateOfBirth`. Si quiere un segundo atributo, haga clic de nuevo en **+ Add** e introduzca `nationalId` (todos los atributos indicados deben coincidir con el mismo usuario). Estos deben ser nombres de atributos ya existentes en el perfil de usuario (consulte los requisitos previos).
4. Opcionalmente, ajuste los parámetros de mitigación de denegación de servicio que se indican más abajo (ya vienen rellenados con valores predeterminados razonables; consulte [Consideraciones sobre denegación de servicio](#denial-of-service-considerations)):
   - **Max candidates per request** (predeterminado `10`)
   - **Max failures per attribute-value combination** (predeterminado `10`)
   - **Failure window (seconds)** (predeterminado `60`)
   - **Max user-store rows per attribute lookup** (predeterminado `5000`)
5. Deje desactivado **Honor User Profile required attributes** para que todos los atributos indicados arriba sigan siendo obligatorios. Si lo activa, cada campo seguirá su ajuste **Required field** del perfil de usuario. Consulte [Campos obligatorios](../02-reference/10-user-profile-login-registration-fields.md#required-fields).
6. Deje **Multiple-candidate match policy** en su valor predeterminado, `REJECT_AMBIGUOUS`, salvo que haya leído y comprendido [Política de coincidencia con varios candidatos](#multiple-candidate-match-policy) más abajo. La alternativa, `FIRST_MATCH`, solo es segura cuando se garantiza que la contraseña es única entre todos los candidatos posibles.
7. Deje **Existing browser session policy** en `KEEP` para los flujos de inicio de sesión normales. Para un flujo en un dispositivo compartido, en el que cada visita debe permitir a un usuario distinto iniciar sesión, seleccione `TERMINATE_BEFORE_LOGIN`. Al abrir ese formulario de inicio de sesión se cierra la sesión del usuario asociada al navegador antes de enviar las credenciales. Esta política funciona tanto con `PASSWORD` como con la verificación de credenciales `SECRET_ATTRIBUTE`.
8. Haga clic en **Save**.

---

## Paso 3 – Asocie el flujo a un cliente {#step-3--bind-the-flow-to-a-client}

1. Vaya a **Clients** y seleccione el cliente al que se aplica esta página de inicio de sesión (por ejemplo, `onsite-voting-portal`).
2. Abra la pestaña **Advanced** → **Authentication flow overrides**.
3. Establezca **Browser Flow** en el flujo que creó en el Paso 1.
4. Haga clic en **Save**.

Así, el cambio se limita a los clientes que asocie explícitamente. Todos los demás clientes siguen usando el flujo de navegador predeterminado del realm sin cambios.

---

## Paso 4 – Ajustes opcionales de la página {#step-4--optional-page-level-settings}

Dos atributos del realm cambian el comportamiento de la página de inicio de sesión que muestra este flujo. Ambos se establecen en el realm del proceso electoral, no en el autenticador, y ninguno es obligatorio:

| Atributo del realm | Predeterminado | Efecto en esta página de inicio de sesión |
|---|---|---|
| `credential-field-position` | `LAST` | `FIRST` coloca el campo de contraseña o PIN por encima de los campos de atributos y le da el foco inicial de la página. Resulta útil cuando lo principal que se introduce es un PIN de la carta al votante. |
| `login-validation-policy` | `BROWSER` | `SERVER_ONLY` evita que el navegador rechace valores con formato incorrecto, de modo que cada envío llega a este autenticador y recibe su mensaje genérico. |

La configuración de campos de inicio de sesión y registro los trata en [Poner primero la credencial](../02-reference/10-user-profile-login-registration-fields.md#putting-the-credential-first) y en [Quién informa de los formatos no válidos en la página de inicio de sesión](../02-reference/10-user-profile-login-registration-fields.md#who-reports-invalid-formats-on-the-login-page), y explica dónde se editan los atributos del realm.

---

## Resumen del comportamiento {#behavior-summary}

| Escenario | Acción del autenticador |
|---|---|
| Un campo de atributo configurado se deja en blanco | Error genérico de «credenciales no válidas» (no se realiza ninguna búsqueda). |
| Ningún usuario coincide con todos los atributos configurados | Error genérico de «credenciales no válidas». |
| Exactamente un candidato y contraseña correcta | El inicio de sesión tiene éxito. |
| Exactamente un candidato y contraseña incorrecta | Error genérico de «credenciales no válidas». Este intento **sí** cuenta para el bloqueo por detección de fuerza bruta de esa cuenta, igual que en un inicio de sesión estándar. |
| Exactamente un candidato, bloqueado actualmente por la detección de fuerza bruta | Error genérico de «credenciales no válidas»; la cuenta bloqueada no puede iniciar sesión. |
| Varios candidatos comparten los atributos configurados y la contraseña coincide con exactamente uno | El inicio de sesión tiene éxito como ese usuario. |
| Varios candidatos coinciden con la contraseña (o ninguno coincide) | Error genérico de «credenciales no válidas». Consulte la nota sobre fuerza bruta más abajo. |

Los fallos de autenticación usan un mensaje genérico en lugar de indicar qué atributo o credencial es incorrecto. Los cálculos de hash ficticios en las rutas de rechazo temprano ayudan a reducir las diferencias de tiempo, pero **no** garantizan tiempos de respuesta idénticos para toda la solicitud de inicio de sesión. En el modo `SECRET_ATTRIBUTE`, la comparación de resúmenes de tamaño fijo es de tiempo constante, pero la búsqueda en la base de datos, el número de candidatos y el descifrado siguen influyendo en el tiempo total de la solicitud.

> **Nota sobre la protección contra fuerza bruta:** La protección de bloqueo por cuenta integrada en Keycloak solo se activa cuando la resolución se reduce a un único candidato. La cuenta que termina bloqueada es también aquella a la que se atribuyen los intentos fallidos, igual que en el formulario estándar de nombre de usuario y contraseña. Cuando más de un candidato sigue compartiendo los atributos configurados, no existe una única cuenta a la que atribuir honestamente un intento fallido, así que el contador no puede activarse para esa solicitud concreta. En el modo `PASSWORD`, las cuentas bloqueadas se excluyen antes de verificar la contraseña. En el modo `SECRET_ATTRIBUTE`, las cuentas habilitadas pero bloqueadas siguen participando en la detección de ambigüedad: bloquear una cuenta no debe hacer que un secreto compartido autentique como otro votante. Una cuenta bloqueada nunca puede iniciar sesión.
> Configurar más atributos reduce el conjunto de candidatos antes de la comprobación de la contraseña, de modo que lo habitual es el caso de un único candidato (totalmente protegido). Mantenga la **Brute Force Detection** activada a nivel de realm en cualquier caso.
>
> Los votantes que comparten los atributos configurados pueden iniciar sesión al mismo tiempo sin bloquearse entre sí: mientras varios candidatos coinciden, su estado de bloqueo se lee sin reservarlos en el protector de fuerza bruta de Keycloak, y solo se comprueba a través de él la cuenta que se autentica. Los desarrolladores pueden verificarlo en un entorno en ejecución con `packages/keycloak-extensions/message-otp-authenticator/src/test/integration/concurrent-shared-dob-login.py`.

---

## Consideraciones sobre denegación de servicio {#denial-of-service-considerations}

Como este autenticador compara por atributos y no por un nombre de usuario único, una sola solicitud puede resolverse legítimamente en muchos candidatos (por ejemplo, todos los votantes nacidos en una fecha común), y cada candidato suele costar una comparación de hash de contraseña para descartarlo. Tres ajustes independientes limitan ese coste por solicitud, además de la Brute Force Detection estándar de Keycloak:

- **Max candidates per request** (`maxCandidates`, predeterminado `10`): cuando los atributos configurados coinciden con más candidatos que este valor, la solicitud falla de forma genérica sin comprobar la contraseña de ninguno. Así se limita el número máximo de hashes de contraseña que puede forzar una sola solicitud, sin importar cuántos votantes compartan el valor o los valores de atributo enviados.
- **Max failures per attribute-value combination** / **Failure window (seconds)** (`tupleMaxFailures` / `tupleFailureWindowSeconds`, predeterminados `10` / `60`): los fallos también se cuentan por cada combinación distinta de valores de atributo enviados, independientemente de una cuenta concreta. Así se cubre un hueco que la Brute Force Detection por cuenta no puede cubrir por sí sola: cuando una solicitud coincide con más de un candidato, Keycloak no tiene una única cuenta a la que atribuir el fallo, así que su contador de bloqueo nunca se activa para esa solicitud. De otro modo, un atacante podría repetir indefinidamente un valor de atributo común (por ejemplo, una fecha de nacimiento compartida) con el coste completo. Cuando una combinación alcanza el máximo de fallos configurado dentro de la ventana, los intentos posteriores contra ella se rechazan sin ninguna búsqueda de usuario, hasta que transcurra la ventana o una solicitud coincidente tenga éxito (lo que borra el contador). Este limitador se gestiona en todo el clúster, por lo que no puede eludirse repartiendo las solicitudes entre nodos de Keycloak.
- **Max user-store rows per attribute lookup** (`maxAttributeLookupResults`, predeterminado `5000`): un límite máximo del número de filas que puede devolver la consulta subyacente al almacén de usuarios, que se aplica antes de cargar ningún candidato en memoria. Es deliberadamente mucho mayor que **Max candidates per request**: existe solo para evitar que una coincidencia realmente anómala (por ejemplo, un valor de atributo compartido por la mayoría del realm) extraiga de la base de datos un número ilimitado de filas, y no para sustituir el límite de candidatos más estricto indicado arriba. Manténgalo bastante por encima del mayor número realista de coincidencias combinadas que cabría esperar de una búsqueda legítima de votantes; un valor demasiado bajo puede hacer menos eficaz la configuración de un segundo atributo identificativo (la palanca recomendada) para reducir los grupos grandes.

Elevar **Max candidates per request** reduce esta protección a cambio de una coincidencia menos estricta (útil si se espera que un único atributo coincida con grupos inusualmente grandes). Reducir el umbral de fallos o ampliar la ventana supone sacrificar protección contra denegación de servicio a cambio de comodidad. Configurar un segundo atributo identificativo (consulte el [Paso 2](#step-2--configure-the-attributes-to-match)) es, por lo general, la mejor palanca: mantiene pequeños los conjuntos de candidatos reales sin necesidad de relajar estas protecciones.

Si los PIN o las contraseñas que se usan con este autenticador son cortos (por ejemplo, un PIN numérico), **no** compense la facilidad de adivinación debilitando el algoritmo de hash de contraseñas: eso solo facilitaría el descifrado sin conexión si las huellas llegaran a filtrarse. Use en su lugar los ajustes anteriores, que limitan el coste de CPU sin tocar la robustez del hash.

---

## Política de coincidencia con varios candidatos {#multiple-candidate-match-policy}

Cuando más de un candidato comparte los valores de atributo configurados (por ejemplo, varios votantes nacidos en la misma fecha), **Multiple-candidate match policy** determina cómo la contraseña enviada elige uno:

`SECRET_ATTRIBUTE` requiere `REJECT_AMBIGUOUS`; la opción `FIRST_MATCH` que se describe a continuación solo se aplica a la verificación `PASSWORD` normal.

- **`REJECT_AMBIGUOUS`** (predeterminado): comprueba la contraseña de cada candidato. Solo tiene éxito si la contraseña enviada coincide con **exactamente uno** de ellos; si coincide con más de uno, la solicitud falla de forma genérica, igual que si ninguno coincidiera.
- **`FIRST_MATCH`**: tiene éxito en cuanto la contraseña de **cualquier** candidato coincide, sin comprobar si otro candidato también habría coincidido. Es más económico (se detiene en la primera coincidencia en lugar de calcular el hash de cada candidato), pero solo es correcto en una condición.

> ⚠️ **Advertencia de seguridad:** `FIRST_MATCH` solo debe activarse cuando se **garantiza que la contraseña es única entre todos los candidatos** que una solicitud podría llegar a coincidir; por ejemplo, contraseñas asignadas de forma centralizada y nunca reutilizadas, sin posibilidad de que dos votantes que comparten un valor de atributo compartan también una contraseña. Si dos candidatos de un conjunto coincidente comparten la misma contraseña, **no se especifica cuál de los dos inicia sesión**, lo que significa que un votante podría terminar autenticado con la cuenta de otro votante. Si no puede garantizar que las contraseñas sean únicas entre candidatos, deje este valor en `REJECT_AMBIGUOUS`.

Activar `FIRST_MATCH` no cambia nada de las mitigaciones de denegación de servicio anteriores: el límite de candidatos y el limitador por tupla se aplican exactamente igual, sea cual sea la política de coincidencia.
