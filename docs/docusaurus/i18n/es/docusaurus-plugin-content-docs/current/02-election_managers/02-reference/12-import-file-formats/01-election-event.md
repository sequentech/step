---
id: import_format_election_event
title: Archivo del proceso electoral
sidebar_position: 1
description: "El archivo del proceso electoral que usted exporta e importa en el portal de administración: tipos de archivo, qué cambia la importación, la estructura JSON con todos los campos y el contenido del archivo ZIP."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

El archivo del proceso electoral contiene la configuración de un proceso electoral. También puede
contener sus votantes, registros, claves, publicaciones y resultados. Úselo para hacer una copia de
un proceso electoral, o para pasar un proceso electoral a otro inquilino o a otra instalación.

Cree el archivo con **Exportar** en el proceso electoral. Impórtelo con **Procesos Electorales** >
**+** > **Importar Evento Electoral**. No cree este archivo usted mismo, y no lo cambie. Esta página
describe el archivo para que pueda examinarlo y encontrar la causa de un error de importación.

## Tipos de archivo {#file-types}

| Tipo de archivo | Contenido |
| --- | --- |
| `.json` | Solo la configuración: el documento JSON que describe esta página. |
| `.zip` | Un archivo ZIP. Contiene el documento JSON y los otros archivos que seleccionó en la exportación. Consulte [Contenido del ZIP](#zip-contents). |
| `.ezip` | Un archivo `.zip` cifrado con contraseña (AES-256-CBC). |

La exportación siempre crea un archivo `.zip` o `.ezip`. Para obtener un archivo `.json`, extraiga el
archivo `export_election_event-<ID>.json` del archivo comprimido.

### Nombre del archivo {#file-name}

La exportación da al archivo este nombre:

```text
election-event-<election event ID>-export-sha256-<hash>.zip
```

La extensión es `.ezip` si el archivo está cifrado. El `<hash>` es el hash SHA-256 del archivo. El
nombre de un archivo `.json` no es importante. En un archivo `.zip`, la importación busca una entrada
cuyo nombre contenga `export_election_event` y termine en `.json`. Si no hay ninguna entrada así, la
importación se detiene con "No JSON file found in ZIP".

### Contraseña {#password}

La exportación cifra el archivo si selecciona **Cifrar con Contraseña**, **Tablón de Anuncios**,
**Informes** o **Aplicaciones**. Entonces el portal de administración muestra una contraseña en el
cuadro de diálogo **Contraseña**. Guarde la contraseña. No puede volver a obtenerla.

Para importar un archivo `.ezip`, escriba la contraseña en **Contraseña de Descifrado**. La
importación identifica un archivo cifrado por la extensión `.ezip`. No cambie la extensión.

### Hash de integridad {#integrity-hash}

:::caution PRECAUCIÓN: hash del archivo del proceso electoral
En esta versión el portal de administración no envía el hash de un archivo del proceso electoral al
servidor, así que la importación no lo comprueba. Compare el hash usted mismo antes de importar: el
hash que da `shasum -a 256 <file>` debe ser igual al hash del nombre del archivo. Una exportación
`.zip` o `.ezip` puede contener datos de votantes y claves. Guárdela en un lugar protegido.
:::

## Lo que cambia la importación {#what-the-import-changes}

La importación no copia el proceso electoral exactamente. Hace estos cambios:

- Añade el proceso electoral al inquilino en el que ha iniciado sesión. Reemplaza el `tenant_id` del
  archivo en todo el archivo.
- Da nuevos ID al proceso electoral y a todas sus partes. Reemplaza cada ID en todo el archivo, para
  que los vínculos entre las partes sigan siendo correctos. Los ID de la configuración de los flujos
  de inicio de sesión (configuración de los autenticadores) no cambian.
- Restablece el `status` del proceso electoral: no publicado, todos los canales de votación
  `NOT_STARTED`, sin fechas de período de votación.
- Restablece el estado de votación y las fechas del período de votación de cada elección, y pone
  `initialization_report_generated` en `false`.
- Restablece las `statistics` del proceso electoral y de cada elección a 0 correos y 0 SMS enviados.
- Elimina el `public_key` del proceso electoral.
- Crea un tablón nuevo y un registro electoral nuevo para el proceso electoral y para cada elección.
  Reemplaza `bulletin_board_reference` por el tablón nuevo. Si el archivo ZIP tiene los archivos del
  registro electoral, la importación mantiene los registros electorales exportados. Consulte
  [Archivos del registro electoral](#electoral-log-files).
- Crea claves nuevas del gestor del protocolo, salvo que el archivo ZIP contenga claves del gestor del
  protocolo.
- Crea de nuevo el realm de inicio de sesión del proceso electoral. Consulte
  [keycloak_event_realm](#keycloak_event_realm).

La importación mantiene los demás valores, por ejemplo `is_archived`. Un proceso electoral que estaba
archivado cuando lo exportó sigue archivado después de la importación.

Si el archivo no tiene una ceremonia de claves, el proceso electoral no tiene clave electoral después
de la importación. Haga una ceremonia de claves nueva.

Si el archivo tiene datos del tablón, la importación mantiene las ceremonias de claves con su estado y
su estado de ejecución. Elimina el `public_key` del proceso electoral. La publicación de la papeleta
lee la clave electoral del estado de la ceremonia de claves, y la importación mantiene ese estado, así
que no hace falta una ceremonia de claves nueva para esa clave.

La importación se ejecuta como una tarea. Vea su resultado en **Tareas**. Si una parte de la
importación falla, la importación se detiene y la tarea muestra el error. El proceso electoral no se
añade.

## Comprobaciones del archivo {#file-checks}

Antes de importar, el sistema comprueba el documento JSON. Encuentra todos los problemas a la vez. Si
hay un error, rechaza el archivo con el mensaje "The election event bundle cannot be imported; N
problems found", y una lista de los problemas. Cada problema da la ruta del campo, por ejemplo
`contests[2].max_votes`, y los ID del archivo. El portal de administración muestra la lista cuando
comprueba el archivo.

La comprobación rechaza el archivo si:

- `tenant_id` está vacío o no es un UUID, el proceso electoral no tiene `id` ni `encryption_protocol`,
  o no hay elecciones.
- Una pregunta, un candidato, un vínculo entre un área y una pregunta, o un área padre apunta a un ID
  que no está en el archivo. Un candidato sin `contest_id` también es un error.
- Las áreas padre forman un bucle: un área es su propio padre, directamente o a través de otras áreas.
- Una pregunta no tiene `min_votes`, `max_votes` o `winning_candidates_num`, o uno de ellos es menor
  que 0.
- `min_votes` es mayor que `max_votes`.
- `max_votes` o `winning_candidates_num` es mayor que el número de candidatos de la pregunta.
- `voting_type` no es `preferential` ni `non-preferential`.
- `counting_algorithm` está vacío o no es un valor permitido, o no concuerda con `voting_type`. Consulte
  [Algoritmos de recuento](02-election-contest-candidate.md#counting-algorithms).
- Dos elecciones, preguntas, candidatos, áreas o vínculos entre áreas y preguntas tienen el mismo `id`.

La comprobación escribe una advertencia en el registro de la tarea, e importa el archivo, si:

- No hay áreas.
- Una pregunta no tiene candidatos, o ningún área tiene la pregunta en su papeleta.
- Un área no tiene preguntas y no es el padre de otra área.
- Las elecciones o los informes tienen etiquetas de permiso. Solo los usuarios con estas etiquetas pueden
  verlos.

## Compatibilidad de versiones {#version-compatibility}

El documento JSON tiene un campo `version`: la versión del sistema que hizo la exportación. Antes de
importar, el sistema compara esta versión con su propia versión:

| Caso | Resultado |
| --- | --- |
| Las versiones principales son diferentes. | Rechazado. |
| La versión menor del archivo es mayor que la versión menor del sistema. | Rechazado. |
| La versión menor del archivo es igual o menor que la del sistema. La versión de parche no importa. | Aceptado. |
| El archivo tiene la versión `dev`, y el sistema no. | Rechazado. |
| El sistema tiene la versión `dev`. | Aceptado, sea cual sea la versión del archivo. |

La comparación ignora una `v` al principio y un sufijo después de `-`, por ejemplo `-rc.1`. Si el
archivo no tiene un campo `version`, el sistema usa `9.0.0`. Por tanto, las versiones 10 y posteriores rechazan un
archivo sin campo `version`. El error es "Version mismatch: Imported version ... is not compatible with
current version ...".

Cuando selecciona un archivo en el panel de importación, el portal de administración comprueba el
archivo antes de importar. Muestra los errores de versión y de JSON en este paso.

## Estructura JSON {#json-structure}

El archivo es un objeto JSON. La mayoría de los valores son texto, números, `true` o `false`, u
objetos JSON. Todos los ID son UUID. La importación rechaza un ID que no sea un UUID versión 4.

Un campo "opcional" puede faltar o ser `null`. Un campo "obligatorio" debe estar presente.

### Objeto de nivel superior {#top-level-object}

| Clave | Tipo | Obligatorio | Contenido |
| --- | --- | --- | --- |
| `version` | texto | Opcional | La versión del sistema que hizo la exportación. Predeterminado: `9.0.0`. Consulte [Compatibilidad de versiones](#version-compatibility). |
| `tenant_id` | UUID | Obligatorio | El inquilino del proceso electoral exportado. La importación lo reemplaza por su inquilino. |
| `keycloak_event_realm` | objeto | Opcional | El realm de inicio de sesión del proceso electoral. Consulte [keycloak_event_realm](#keycloak_event_realm). |
| `election_event` | objeto | Obligatorio | El proceso electoral. Consulte [election_event](#election_event). |
| `elections` | array | Obligatorio | Las elecciones. Consulte [elections](#elections). Puede estar vacío. |
| `contests` | array | Obligatorio | Las preguntas. Consulte [contests](#contests). Puede estar vacío. |
| `candidates` | array | Obligatorio | Los candidatos. Consulte [candidates](#candidates). Puede estar vacío. |
| `areas` | array | Obligatorio | Las áreas. Consulte [areas](#areas). Puede estar vacío. |
| `area_contests` | array | Obligatorio | Los vínculos entre áreas y preguntas. Consulte [area_contests](#area_contests). Puede estar vacío. |
| `scheduled_events` | array | Opcional | Los períodos de votación programados. Consulte [scheduled_events](#scheduled_events). La exportación escribe `null`. |
| `reports` | array | Obligatorio | Los informes. Consulte [reports](#reports). La importación no usa este array. Puede estar vacío. |
| `keys_ceremonies` | array | Opcional | Las ceremonias de claves. Consulte [keys_ceremonies](#keys_ceremonies). |
| `applications` | array | Opcional | Las solicitudes de votantes. Consulte [applications](#applications). |

### election_event

| Campo | Tipo | Obligatorio | Predeterminado | Contenido |
| --- | --- | --- | --- | --- |
| `id` | UUID | Obligatorio | — | El ID del proceso electoral. La importación da un ID nuevo. |
| `tenant_id` | UUID | Obligatorio | — | El inquilino. La importación lo reemplaza. |
| `created_at` | fecha y hora | Opcional | — | No se usa. La importación pone la hora de la importación. |
| `updated_at` | fecha y hora | Opcional | — | No se usa. La importación pone la hora de la importación. |
| `description` | texto | Opcional | — | La descripción del proceso electoral. |
| `presentation` | objeto | Opcional | — | Los ajustes y textos del proceso electoral. Consulte [presentation](#election_eventpresentation). |
| `voting_channels` | objeto | Opcional | — | Los canales de votación. Consulte [voting_channels](#election_eventvoting_channels). |
| `status` | objeto | Opcional | — | El estado. La importación lo restablece. Consulte [status](#election_eventstatus). |
| `statistics` | objeto | Opcional | — | Los contadores de mensajes enviados. La importación los restablece. Consulte [statistics](#election_eventstatistics). |
| `labels` | objeto | Opcional | — | Datos libres clave-valor. Consulte [labels y annotations](#election_eventlabels-and-annotations). |
| `annotations` | objeto | Opcional | — | Datos libres clave-valor. Consulte [labels y annotations](#election_eventlabels-and-annotations). |
| `bulletin_board_reference` | objeto | Opcional | — | El tablón: `id` (número), `database_name` (texto) e `is_archived` (`true` o `false`). Los tres son obligatorios si el objeto está presente. La importación lo reemplaza por el tablón nuevo. |
| `is_archived` | `true` o `false` | Obligatorio | — | `true` si el proceso electoral está archivado. La importación mantiene el valor. |
| `encryption_protocol` | texto | Obligatorio | — | El protocolo de cifrado. El portal de administración siempre escribe `RSA256`. |
| `public_key` | texto | Opcional | — | La clave electoral. La importación la elimina. |
| `external_id` | texto | Opcional | — | El **ID externo** del proceso electoral. |
| `is_audit` | `true` o `false` | Opcional | — | Interno. Mantenga el valor de la exportación. |
| `audit_election_event_id` | UUID | Opcional | — | Interno. Mantenga el valor de la exportación. |
| `user_boards` | texto | Opcional | — | Interno. Mantenga el valor de la exportación. |

El proceso electoral no tiene un campo `name`. Su nombre está en `presentation.i18n.<language>.name`.

#### election_event.presentation

La importación comprueba cada campo que conoce. Si un valor no es un valor permitido, la importación
se detiene. La importación mantiene los campos que no conoce. Las etiquetas de la última columna son
las etiquetas de la pestaña **Datos** del proceso electoral. Consulte
[Datos del proceso electoral](../02-election-event/03-election_management_election-event_data.md).

| Campo | Tipo | Predeterminado | Valores permitidos | Contenido |
| --- | --- | --- | --- | --- |
| `i18n` | objeto | — | `{ "<language>": { "<key>": text or null } }` | Los textos en cada idioma. Consulte [i18n](#presentationi18n). |
| `language_conf` | objeto | — | Consulte [language_conf](#presentationlanguage_conf) | Los idiomas del proceso electoral. |
| `materials` | objeto | — | `{ "policy": "off" \| "optional" \| "mandatory_for_voting" }` | La política de los materiales de apoyo. Predeterminado `off`. |
| `logo_url` | texto | — | Una URL | **Logo URL**. |
| `redirect_finish_url` | texto | — | Una URL | **URL de redirección al finalizar**: la página que abre el portal de votación cuando el votante termina. |
| `kiosk_redirect_finish_url` | texto | — | Una URL | **URL de redirección al finalizar del quiosco**: lo mismo para la votación en quiosco. |
| `css` | texto | — | CSS | **CSS personalizado** del portal de votación. |
| `skip_election_list` | `true` o `false` | — | — | **Saltar pantalla para escoger elección**. |
| `show_user_profile` | `true` o `false` | `true` | — | **Mostrar perfil de usuario**. |
| `show_cast_vote_logs` | texto | `hide-logs-tab` | `show-logs-tab`, `hide-logs-tab` | **Mostrar logs de votación**. |
| `elections_order` | texto | `alphabetical` | `alphabetical`, `random`, `custom` | El orden de las elecciones en el portal de votación. |
| `voting_portal_countdown_policy` | objeto | — | Consulte [voting_portal_countdown_policy](#presentationvoting_portal_countdown_policy) | **Política de cuenta regresiva del portal de votación**. |
| `voting_portal_datetime_format` | texto u objeto | `legacy-gb-24h` | `legacy-gb-24h`, `iso-local`, `us-12h`, `locale-medium`, `date-only`, o `{ "custom": "<pattern>" }` | **Formato de fecha y hora del portal de votación**. Un patrón personalizado usa los tokens `yyyy`, `MM`, `dd`, `HH`, `mm`, `ss`. |
| `custom_urls` | objeto | — | `{ "login": URL, "enrollment": URL, "saml": URL }`, cada una opcional | Las URL personalizadas de las páginas de inicio de sesión, inscripción y SAML. |
| `keys_ceremony_policy` | texto | `ELECTION_EVENT` | `ELECTION_EVENT`, `ELECTION` | Una clave electoral para el proceso electoral, o una para cada elección. |
| `contest_encryption_policy` | texto | `single-contest` | `single-contest`, `multiple-contests` | **Política de cifrado de concurso**. |
| `decoded_ballot_inclusion_policy` | texto | `not-included` | `included`, `not-included` | **Incluir papeletas decodificadas en la base de datos de resultados**. |
| `locked_down` | texto | `not-locked-down` | `locked-down`, `not-locked-down` | **Estado de Confinamiento**. |
| `publish_policy` | texto | `always` | `always`, `after-lockdown` | `after-lockdown`: solo puede publicar el proceso electoral después del confinamiento. |
| `enrollment` | texto | `enabled` | `enabled`, `disabled` | **Inscripción** de votantes. |
| `otp` | texto | `enabled` | `enabled`, `disabled` | **OTP** (código de un solo uso). |
| `voter_signing_policy` | texto | `no-signature` | `no-signature`, `with-signature` | **Política de Firma de Votantes**. |
| `voter_certificate_policy` | texto | `disabled` | `disabled`, `enabled` | **Voter Digital Certificate Policy**. |
| `weighted_voting_policy` | texto | `disabled-weighted-voting` | `disabled-weighted-voting`, `areas-weighted-voting`, `voters-weighted-voting` | **Política de Votación Ponderada**. |
| `delegated_voting_policy` | texto | `disabled` | `disabled`, `enabled` | **Política de Votación Delegada**. |
| `ceremonies_policy` | texto | `manual-ceremonies` | `manual-ceremonies`, `automated-ceremonies` | **Política de ceremonias de llaves/recuento**. |
| `automatic_recount_policy` | texto | `disabled` | `enabled`, `disabled` | **Recuento automático tras aprobar una importación**. |
| `results_website` | texto (JSON) | — | Consulte [results_website](#presentationresults_website) | La política del sitio web de resultados. |
| `custom_filters` | array | — | Consulte abajo | Los filtros personalizados de la lista de votantes del portal de administración. |

Cada elemento de `custom_filters` tiene un `label` (`name` y un objeto `i18n` con un texto para cada
idioma) y un `filter` (un objeto de condiciones, o `null`). El portal de administración escribe este
array. No lo cambie.

El valor predeterminado se aplica cuando falta el campo. Si no conoce un valor, mantenga el valor de la
exportación.

#### presentation.i18n

Un objeto con una clave para cada código de idioma, por ejemplo `en` o `es`. Cada idioma tiene un
objeto de textos. El portal de administración escribe estas claves:

| Clave | Contenido |
| --- | --- |
| `name` | El nombre del proceso electoral. |
| `alias` | El alias del proceso electoral. |
| `description` | La descripción en este idioma. |
| `materialsTitle` | El título de los materiales de apoyo. |
| `materialsSubtitle` | El subtítulo de los materiales de apoyo. |

La pestaña **Localización** añade otras claves. Cada valor es un texto o `null`. Si un texto no está
disponible en un idioma, el sistema usa el texto de `en`.

#### presentation.language_conf

| Campo | Tipo | Predeterminado | Valores permitidos | Contenido |
| --- | --- | --- | --- | --- |
| `enabled_language_codes` | array de texto | — | Códigos de idioma | Los idiomas que pueden seleccionar los votantes. |
| `default_language_code` | texto | `en` | Un código de idioma | El idioma predeterminado. |
| `language_detection_policy` | texto | `browser-detect` | `browser-detect`, `force-default` | **Política de detección de idioma**. Con `force-default`, la importación pone el idioma predeterminado del realm de inicio de sesión en `default_language_code`. |

#### presentation.voting_portal_countdown_policy

| Campo | Tipo | Valores permitidos | Contenido |
| --- | --- | --- | --- |
| `policy` | texto | `NO_COUNTDOWN`, `COUNTDOWN`, `COUNTDOWN_WITH_ALERT` | La cuenta regresiva antes de que expire la sesión de votación. |
| `countdown_anticipation_secs` | número entero | 0 o más | El número de segundos antes de la expiración en que empieza la cuenta regresiva. |
| `countdown_alert_anticipation_secs` | número entero | 0 o más | El número de segundos antes de la expiración en que aparece la alerta de cierre de sesión. |

#### presentation.results_website

El valor es un objeto JSON escrito como texto. La importación también acepta el objeto en sí. Si el
valor está presente, los tres campos son obligatorios.

| Campo | Valores permitidos | Predeterminado | Contenido |
| --- | --- | --- | --- |
| `status` | `enabled`, `disabled` | `disabled` | Publicar los resultados en el sitio web de resultados. |
| `access` | `public`, `authenticated` | `public` | Quién puede ver los resultados. |
| `visibility_scope` | `full_event`, `area_based` | `full_event` | Todos los resultados, o solo los resultados del área del usuario. |

#### election_event.voting_channels

| Campo | Tipo | Contenido |
| --- | --- | --- |
| `online` | `true` o `false` | La votación en línea está activada. |
| `kiosk` | `true` o `false` | La votación en quiosco está activada. |
| `telephone` | `true` o `false` | La votación por teléfono está activada. |
| `paper` | `true` o `false` | La votación en papel está activada. |
| `early_voting` | `true` o `false` | La votación anticipada está activada. |

Todos los campos son opcionales. Un proceso electoral nuevo tiene `online` en `true`.

#### election_event.status

La importación sustituye este objeto por un estado nuevo. No se usan los valores del archivo. Cada
campo es opcional.

| Campo | Tipo | Valores permitidos | Contenido |
| --- | --- | --- | --- |
| `is_published` | `true` o `false` | — | El proceso electoral está publicado. Después de la importación: `false`. |
| `voting_status` | texto | `NOT_STARTED`, `OPEN`, `PAUSED`, `CLOSED` | El estado de la votación en línea. Después de la importación: `NOT_STARTED`. |
| `kiosk_voting_status` | texto | Igual | El estado de la votación en quiosco. |
| `early_voting_status` | texto | Igual | El estado de la votación anticipada. |
| `telephone_voting_status` | texto | Igual | El estado de la votación por teléfono. |
| `voting_period_dates` | objeto | Consulte abajo | Las fechas del período de votación en línea. |
| `kiosk_voting_period_dates` | objeto | Consulte abajo | Las fechas del período de votación en quiosco. |
| `early_voting_period_dates` | objeto | Consulte abajo | Las fechas del período de votación anticipada. |
| `telephone_voting_period_dates` | objeto | Consulte abajo | Las fechas del período de votación por teléfono. |

Cada objeto `*_period_dates` tiene los campos opcionales `first_started_at`, `last_started_at`,
`first_paused_at`, `last_paused_at`, `first_stopped_at` y `last_stopped_at`. Cada uno es una fecha y
hora en UTC, en formato ISO 8601.

#### election_event.statistics

| Campo | Tipo | Contenido |
| --- | --- | --- |
| `num_emails_sent` | número entero | El número de correos enviados. Después de la importación: 0. |
| `num_sms_sent` | número entero | El número de SMS enviados. Después de la importación: 0. |

#### election_event.labels y annotations {#election_eventlabels-and-annotations}

Dos objetos JSON con claves y valores libres. El portal de administración no los muestra. Algunas
integraciones guardan datos aquí. Mantenga los dos objetos como los escribió la exportación.

### elections

Las elecciones del proceso electoral. Consulte
[Elecciones, preguntas y candidatos](02-election-contest-candidate.md).

### contests

Las preguntas de las elecciones. Consulte
[Elecciones, preguntas y candidatos](02-election-contest-candidate.md).

### candidates

Los candidatos de las preguntas. Consulte
[Elecciones, preguntas y candidatos](02-election-contest-candidate.md).

### areas

Un objeto para cada área.

| Campo | Tipo | Obligatorio | Contenido |
| --- | --- | --- | --- |
| `id` | UUID | Obligatorio | El ID del área. La importación da un ID nuevo. |
| `tenant_id` | UUID | Obligatorio | El inquilino. La importación lo reemplaza. |
| `election_event_id` | UUID | Obligatorio | El proceso electoral. La importación lo reemplaza. |
| `created_at` | fecha y hora | Opcional | No se usa. La importación pone la hora de la importación. |
| `last_updated_at` | fecha y hora | Opcional | No se usa. La importación pone la hora de la importación. |
| `name` | texto | Opcional | El nombre del área, por ejemplo `North`. El archivo de votantes usa este nombre en `area_name`. |
| `description` | texto | Opcional | La descripción del área. |
| `type` | texto | Opcional | El tipo del área. Texto libre. |
| `parent_id` | UUID | Opcional | El ID del área padre. El padre debe estar en el mismo archivo. |
| `presentation` | objeto | Opcional | Consulte abajo. |
| `labels` | objeto | Opcional | Datos libres clave-valor. |
| `annotations` | objeto | Opcional | Consulte abajo. |

El objeto `presentation` del área:

| Campo | Tipo | Predeterminado | Valores permitidos | Contenido |
| --- | --- | --- | --- | --- |
| `allow_early_voting` | texto | `no_early_voting` | `allow_early_voting`, `no_early_voting` | Los votantes de esta área pueden votar anticipadamente. |

El objeto `annotations` del área puede contener:

| Campo | Tipo | Predeterminado | Valores permitidos | Contenido |
| --- | --- | --- | --- | --- |
| `weight` | número entero | 1 | 0 o más | El peso de los votos de esta área, cuando `weighted_voting_policy` es `areas-weighted-voting`. |
| `tally_operation` | texto | — | `process-ballots-all`, `aggregate-results`, `skip-candidate-results` | Cómo trata el recuento a esta área. |

### area_contests

Un objeto para cada vínculo entre un área y una pregunta.

| Campo | Tipo | Obligatorio | Contenido |
| --- | --- | --- | --- |
| `id` | UUID | Obligatorio | El ID del vínculo. La importación da un ID nuevo. |
| `area_id` | UUID | Obligatorio | El área. Debe estar en `areas`. |
| `contest_id` | UUID | Obligatorio | La pregunta. Debe estar en `contests`. |

### keys_ceremonies

Las ceremonias de claves del proceso electoral. La exportación llena este array solo si selecciona
**Tablón de Anuncios**. Si no, escribe un array vacío.

| Campo | Tipo | Obligatorio | Valores permitidos | Contenido |
| --- | --- | --- | --- | --- |
| `id` | UUID | Obligatorio | — | El ID de la ceremonia de claves. La importación da un ID nuevo. |
| `tenant_id` | UUID | Obligatorio | — | El inquilino. La importación lo reemplaza. |
| `election_event_id` | UUID | Obligatorio | — | El proceso electoral. La importación lo reemplaza. |
| `created_at` | fecha y hora | Opcional | — | No se usa. |
| `last_updated_at` | fecha y hora | Opcional | — | No se usa. |
| `trustee_ids` | array de texto | Obligatorio | Nombres de fideicomisarios | Los **nombres** de los fideicomisarios, no sus ID. Consulte la precaución de abajo. |
| `threshold` | número entero | Obligatorio | 1 o más | El número de fideicomisarios necesario para el recuento. |
| `name` | texto | Opcional | — | El nombre de la ceremonia de claves. |
| `status` | objeto | Opcional | Consulte abajo | El avance de la ceremonia. La importación lo mantiene. |
| `execution_status` | texto | Opcional | `USER_CONFIGURATION`, `STARTED`, `IN_PROGRESS`, `SUCCESS`, `CANCELLED` | El estado de la ceremonia. La importación lo mantiene. |
| `settings` | objeto | Opcional | `{ "policy": "manual-ceremonies" \| "automated-ceremonies" }` | La política de ceremonias de la ceremonia. |
| `is_default` | `true` o `false` | Opcional | — | La ceremonia es la predeterminada del proceso electoral. Si falta, la importación usa `false`. |
| `permission_label` | array de texto | Opcional | — | Las etiquetas de permiso de la ceremonia. |
| `labels` | objeto | Opcional | — | No se importa. |
| `annotations` | objeto | Opcional | — | No se importa. |

El objeto `status` tiene estos campos: `stop_date` (texto), `public_key` (texto), `logs` (un array de
`{ "created_date": text, "log_text": text }`) y `trustees` (un array de
`{ "name": text, "status": "WAITING" | "KEY_GENERATED" | "KEY_RETRIEVED" | "KEY_CHECKED" }`).

:::caution PRECAUCIÓN: fideicomisarios en el archivo del proceso electoral
Asegúrese de que el inquilino en el que importa tenga fideicomisarios con los mismos nombres que los
fideicomisarios de `trustee_ids`. La importación encuentra cada fideicomisario por su nombre. Si un
nombre no se encuentra, la importación falla con "Error parsing trustee_ids as UUIDs".
:::

### applications

Las solicitudes de votantes del proceso electoral. La exportación llena este array solo si selecciona
**Aplicaciones**. Los campos son los mismos que en el archivo de solicitudes. Consulte
[Aplicaciones](05-applications.md).

### scheduled_events

La exportación escribe `null`. Los eventos programados de una exportación están en el archivo CSV
`export_scheduled_events-<ID>.csv` del archivo ZIP. Consulte
[Archivo de eventos programados](#scheduled-events-file).

Si el documento JSON contiene este array, la importación usa solo los elementos cuyo `event_processor`
es `START_VOTING_PERIOD` o `END_VOTING_PERIOD`. Los añade solo si `tenant_id`, `election_event_id` y
`task_id` son correctos para el proceso electoral importado, y si `cron_config.scheduled_date` tiene un
valor. Ignora todos los demás elementos.

| Campo | Tipo | Obligatorio | Contenido |
| --- | --- | --- | --- |
| `id` | UUID | Obligatorio | El ID del evento programado. |
| `tenant_id` | UUID | Opcional | El inquilino. |
| `election_event_id` | UUID | Opcional | El proceso electoral. |
| `created_at` | fecha y hora | Opcional | Cuándo se creó el evento programado. |
| `stopped_at` | fecha y hora | Opcional | Cuándo se detuvo el evento programado. |
| `archived_at` | fecha y hora | Opcional | Cuándo se archivó el evento programado. |
| `event_processor` | texto | Opcional | La acción. Consulte los valores de abajo. |
| `cron_config` | objeto | Opcional | `{ "cron": text, "scheduled_date": text }`. `scheduled_date` es la fecha y hora de la acción, en formato ISO 8601. |
| `event_payload` | objeto | Opcional | Para un período de votación: `{ "election_id": UUID or null, "voting_channels": [ ... ] }`. `null` significa el proceso electoral. Los canales son `ONLINE`, `KIOSK`, `EARLY_VOTING`, `TELEPHONE`. Si no hay canales, la acción se aplica a `ONLINE` y `KIOSK`. |
| `task_id` | texto | Opcional | El nombre interno de la tarea programada. |
| `labels` | objeto | Opcional | Datos libres clave-valor. |
| `annotations` | objeto | Opcional | Datos libres clave-valor. |

Los valores de `event_processor` son: `ALLOW_INIT_REPORT`, `CREATE_REPORT`, `SEND_TEMPLATE`,
`START_VOTING_PERIOD`, `END_VOTING_PERIOD`, `ALLOW_VOTING_PERIOD_END`, `START_ENROLLMENT_PERIOD`,
`END_ENROLLMENT_PERIOD`, `START_LOCKDOWN_PERIOD`, `END_LOCKDOWN_PERIOD` y `ALLOW_TALLY`.

### reports

La exportación llena este array solo si selecciona **Informes**. La importación no usa este array. Lee
los informes del archivo CSV `export_reports-<ID>.csv` del archivo ZIP. Consulte
[Archivo de informes](#reports-file). El array debe estar presente, pero puede estar vacío.

| Campo | Tipo | Obligatorio | Contenido |
| --- | --- | --- | --- |
| `id` | UUID | Obligatorio | El ID del informe. |
| `tenant_id` | UUID | Obligatorio | El inquilino. |
| `election_event_id` | UUID | Obligatorio | El proceso electoral. |
| `election_id` | UUID | Opcional | La elección del informe. `null` para el proceso electoral. |
| `report_type` | texto | Obligatorio | El tipo de informe, por ejemplo `ELECTORAL_RESULTS`. |
| `template_alias` | texto | Opcional | El alias de la plantilla del informe. |
| `encryption_policy` | texto | Obligatorio | `unencrypted` o `configured_password`. |
| `cron_config` | objeto | Opcional | La programación del informe. Consulte [Archivo de informes](#reports-file). |
| `created_at` | fecha y hora | Obligatorio | Cuándo se creó el informe. |
| `permission_label` | array de texto | Opcional | Las etiquetas de permiso del informe. |

Los valores de `report_type` son: `INITIALIZATION_REPORT`, `ELECTORAL_RESULTS`, `BALLOT_IMAGES`,
`BALLOT_RECEIPT`, `ACTIVITY_LOGS`, `MANUAL_VERIFICATION`, `PARTICIPATION_REPORT` y `CREDENTIALS`.

### keycloak_event_realm

El realm de inicio de sesión del proceso electoral. Contiene la configuración de inicio de sesión de los
votantes: las páginas de inicio de sesión y sus textos, los idiomas, los campos del votante (perfil de
usuario), los flujos de inicio de sesión y sus ajustes, los proveedores de identidad y los clientes. La
exportación siempre lo escribe.

La importación crea el realm de inicio de sesión del proceso electoral nuevo a partir de este objeto:

- Le da al realm el nombre `tenant-<tenant ID>-event-<election event ID>`.
- Pone las URL del portal de votación, del portal de votación en quiosco y del sitio web de resultados
  en las URL de la instalación en la que importa.
- Elimina los secretos de los clientes. El sistema crea secretos nuevos, o usa los secretos que
  configura la instalación.
- Elimina las claves de firma. Solo mantiene su algoritmo y su prioridad, para que el sistema cree
  claves nuevas.
- Si `language_detection_policy` es `force-default`, pone el idioma predeterminado del realm en
  `default_language_code`.

Si el objeto no está presente, la importación usa el realm de inicio de sesión predeterminado de la
instalación. No cambie este objeto. Para cambiar la configuración de inicio de sesión, use el portal de
administración después de la importación.

## Ejemplo {#example}

### Archivo mínimo {#minimal-file}

Este archivo crea un proceso electoral con un área y sin elecciones. No tiene `keycloak_event_realm`,
así que la importación usa el realm de inicio de sesión predeterminado.

```json
{
  "version": "10.1.0",
  "tenant_id": "11111111-1111-4111-8111-111111111111",
  "election_event": {
    "id": "22222222-2222-4222-8222-222222222222",
    "tenant_id": "11111111-1111-4111-8111-111111111111",
    "is_archived": false,
    "encryption_protocol": "RSA256",
    "presentation": {
      "i18n": { "en": { "name": "Election Event 1" } },
      "language_conf": {
        "enabled_language_codes": ["en"],
        "default_language_code": "en"
      }
    },
    "voting_channels": { "online": true }
  },
  "elections": [],
  "contests": [],
  "candidates": [],
  "areas": [
    {
      "id": "33333333-3333-4333-8333-333333333333",
      "tenant_id": "11111111-1111-4111-8111-111111111111",
      "election_event_id": "22222222-2222-4222-8222-222222222222",
      "name": "North",
      "description": "Area North",
      "presentation": { "allow_early_voting": "no_early_voting" }
    }
  ],
  "area_contests": [],
  "reports": []
}
```

### election_event con presentation {#election_event-with-presentation}

Un objeto `election_event` más completo, tal como lo escribe la exportación (abreviado):

```json
{
  "id": "22222222-2222-4222-8222-222222222222",
  "tenant_id": "11111111-1111-4111-8111-111111111111",
  "created_at": "2026-09-01T10:00:00+00:00",
  "updated_at": "2026-09-15T08:30:00+00:00",
  "description": "Election event for the annual election",
  "external_id": "event-1",
  "is_archived": false,
  "encryption_protocol": "RSA256",
  "public_key": null,
  "labels": {},
  "annotations": {},
  "voting_channels": {
    "online": true,
    "kiosk": false,
    "telephone": false,
    "paper": false,
    "early_voting": false
  },
  "status": {
    "is_published": false,
    "voting_status": "NOT_STARTED",
    "kiosk_voting_status": "NOT_STARTED",
    "early_voting_status": "NOT_STARTED",
    "telephone_voting_status": "NOT_STARTED",
    "voting_period_dates": {},
    "kiosk_voting_period_dates": {},
    "early_voting_period_dates": {},
    "telephone_voting_period_dates": {}
  },
  "statistics": { "num_emails_sent": 0, "num_sms_sent": 0 },
  "presentation": {
    "i18n": {
      "en": {
        "name": "Election Event 1",
        "alias": "EE1",
        "description": "Annual election",
        "materialsTitle": "Support materials",
        "materialsSubtitle": "Read before you vote"
      },
      "es": {
        "name": "Evento Electoral 1",
        "alias": "EE1",
        "description": "Elección anual"
      }
    },
    "language_conf": {
      "enabled_language_codes": ["en", "es"],
      "default_language_code": "en",
      "language_detection_policy": "browser-detect"
    },
    "materials": { "policy": "optional" },
    "logo_url": "https://example.com/logo.png",
    "redirect_finish_url": "https://example.com/thank-you",
    "css": "",
    "skip_election_list": false,
    "show_user_profile": true,
    "show_cast_vote_logs": "hide-logs-tab",
    "elections_order": "alphabetical",
    "voting_portal_countdown_policy": {
      "policy": "COUNTDOWN_WITH_ALERT",
      "countdown_anticipation_secs": 60,
      "countdown_alert_anticipation_secs": 30
    },
    "voting_portal_datetime_format": "iso-local",
    "keys_ceremony_policy": "ELECTION_EVENT",
    "contest_encryption_policy": "single-contest",
    "decoded_ballot_inclusion_policy": "not-included",
    "locked_down": "not-locked-down",
    "publish_policy": "always",
    "enrollment": "disabled",
    "otp": "enabled",
    "voter_signing_policy": "no-signature",
    "voter_certificate_policy": "disabled",
    "weighted_voting_policy": "disabled-weighted-voting",
    "delegated_voting_policy": "disabled",
    "ceremonies_policy": "manual-ceremonies",
    "automatic_recount_policy": "disabled",
    "results_website": "{\"status\":\"enabled\",\"access\":\"public\",\"visibility_scope\":\"full_event\"}"
  }
}
```

## Contenido del ZIP {#zip-contents}

:::caution PRECAUCIÓN: archivos del ZIP del proceso electoral
No edite los archivos del archivo ZIP. Provienen de una exportación, y la importación los espera
exactamente como los escribió la exportación. Un archivo cambiado puede detener la importación, o crear
un proceso electoral con datos incorrectos.
:::

La importación encuentra cada entrada por una parte de su nombre. `<ID>` es el ID del proceso electoral
exportado. La opción de exportación es la casilla del cuadro de diálogo **Exportar Evento Electoral**.

| Entrada | Opcional | Contenido | Opción de exportación |
| --- | --- | --- | --- |
| `export_election_event-<ID>.json` | No | El documento JSON. Consulte [Estructura JSON](#json-structure). | Siempre |
| `images/document_<document ID>_<name>` | Sí | Las imágenes de las elecciones, preguntas y candidatos. La importación las añade como documentos públicos con ID nuevos. | Siempre, si hay imágenes |
| `export_voters-<ID>.csv` | Sí | Los votantes. Consulte [Archivo de votantes](#voters-file). | **Incluir Votantes** |
| `export_reports-<ID>.csv` | Sí | Los informes. Consulte [Archivo de informes](#reports-file). | **Informes** |
| `export_activity_logs-<ID>` | Sí | El registro electoral en formato CSV. La entrada no tiene extensión. Si el archivo ZIP no tiene archivos del registro electoral, la importación añade los mensajes del registro al registro electoral del proceso electoral nuevo. | **Registros de Actividad** |
| `electoral_log_records-<ID>.jsonl` | Sí | Los registros de los registros electorales. Consulte [Archivos del registro electoral](#electoral-log-files). | **Registros de Actividad** |
| `electoral_log_manifest-<ID>.json` | Sí | La identidad, el tamaño, la raíz y los puntos de control publicados de cada registro electoral. Consulte [Archivos del registro electoral](#electoral-log-files). | **Registros de Actividad** |
| `export_S3_files/document_<document ID>_<name>` | Sí | Los documentos privados del proceso electoral. La importación los añade con ID nuevos. | **Archivos S3** |
| `export_scheduled_events-<ID>.csv` | Sí | Los eventos programados. Consulte [Archivo de eventos programados](#scheduled-events-file). | **Eventos Programados** |
| `export_publications-<ID>.json` | Sí | Las publicaciones de la papeleta. | **Publicaciones** |
| `publication_files/<publication ID>/<file>.json` | Sí | Los archivos de cada publicación: `event.json`, `election-<ID>.json`, `style-<ID>.json` y `summary-<ID>.json`. | **Publicaciones** |
| `election_event_config-<ID>.json` | Sí | La configuración pública del proceso electoral que hace una publicación. | **Publicaciones**, si hay una publicación |
| `export_protocol_manager_keys-<ID>.csv` | Sí | Las claves del gestor del protocolo. Consulte [Archivo de claves del gestor del protocolo](#protocol-manager-keys-file). | **Tablón de Anuncios**, **Registros de Actividad** o **Publicaciones** |
| `export_bulletin_boards-<ID>.csv` | Sí | Los mensajes de los tablones. Consulte [Archivo de tablones](#bulletin-boards-file). | **Tablón de Anuncios**, si hay una ceremonia de claves |
| `export_tally_data/<name>.csv` | Sí | Las sesiones de recuento y los resultados. Consulte [Archivos de datos del recuento](#tally-data-files). | **Recuento** (el portal de administración también selecciona **Tablón de Anuncios**) |
| `export_certificates-<ID>.pem` | Sí | Los certificados de autoridad de certificación del proceso electoral, en formato PEM. La importación añade cada certificado. | **Certificados**, si hay certificados |

La importación lee primero las claves del gestor del protocolo. Después lee las otras entradas en orden
alfabético, y los archivos de datos del recuento al final.

Los valores de los archivos CSV que contienen JSON (objetos, arrays, texto entre comillas) se escriben
como JSON. Por ejemplo, un valor de texto se escribe con sus comillas: `"START_VOTING_PERIOD"`.

### Archivos del registro electoral {#electoral-log-files}

Con **Registros de Actividad**, la exportación también escribe los registros electorales del proceso
electoral con sus raíces:

- `electoral_log_records-<ID>.jsonl`: los registros de cada registro electoral, en orden de registro.
  Cada línea es un objeto JSON.
- `electoral_log_manifest-<ID>.json`: la identidad, el tamaño, la raíz y los puntos de control
  publicados de cada registro electoral.

Si el archivo ZIP tiene los dos archivos, la importación guarda los registros electorales con los mismos
registros y las mismas raíces. Rechaza los registros si una raíz no es igual a su punto de control
publicado. El tablón nuevo del proceso electoral continúa a partir del tablón exportado. La importación
entonces no usa `export_activity_logs-<ID>`.

:::caution PRECAUCIÓN: mantenga juntos los dos archivos del registro electoral
No elimine ni cambie ninguno de los dos archivos. Si el archivo ZIP tiene solo uno de ellos, la
importación falla con "The export has the electoral-log manifest but not its records" o "The export has
the electoral-log records but not their manifest".
:::

### Archivo de votantes {#voters-file}

`export_voters-<ID>.csv`. La importación lo lee como una importación de votantes del proceso electoral
nuevo. Consulte [Votantes](03-voters.md).

| Columna | Contenido |
| --- | --- |
| `id` | El ID del votante. La importación no lo usa. Cada votante recibe un ID nuevo. |
| `email` | La dirección de correo electrónico, por ejemplo `voter001@example.com`. |
| `email_verified` | `true` o `false`. |
| `enabled` | `true` o `false`. |
| `first_name` | El nombre. |
| `last_name` | Los apellidos. |
| `username` | El nombre de usuario. |
| `area_name` | El nombre del área del votante, o `-` si el votante no tiene área. |
| `<attribute>` | Una columna para cada campo del votante del realm de inicio de sesión. Los campos secretos del votante solo están presentes si selecciona **Cifrar con Contraseña** y tiene el permiso para leer los campos secretos del votante. |
| `election__<election alias or name>` | Una columna para cada elección: la fecha y hora en que el votante votó por última vez, o vacía. La importación no usa estas columnas. |

El archivo no contiene contraseñas.

### Archivo de informes {#reports-file}

`export_reports-<ID>.csv`. Una fila para cada informe.

| Columna | Contenido |
| --- | --- |
| `ID` | El ID del informe. La importación no lo usa. Cada informe recibe un ID nuevo. |
| `Election ID` | La elección del informe, o vacía para el proceso electoral. |
| `Report Type` | El tipo de informe. Consulte [informes](#reports). |
| `Template Alias` | El alias de la plantilla, o vacío. |
| `Cron Config` | La programación del informe como JSON: `is_active` (`true` o `false`), `cron_expression` (texto), `email_recipients` (array de direcciones de correo electrónico), `executer_username` (texto), `last_document_produced` (texto). O `null`. |
| `Encryption Policy` | `unencrypted` o `configured_password`. |
| `Password` | La contraseña de un informe cifrado, como texto sin formato, o vacía. |
| `Permission Labels` | Las etiquetas de permiso, separadas con `\|`, o vacías. |

:::danger ADVERTENCIA: exportación del proceso electoral con informes
Guarde una exportación con informes en un lugar protegido. El archivo de informes contiene las
contraseñas de los informes cifrados como texto sin formato. Por este motivo, la exportación siempre
cifra el archivo comprimido cuando selecciona **Informes**.
:::

### Archivo de eventos programados {#scheduled-events-file}

`export_scheduled_events-<ID>.csv`. Una fila para cada evento programado. Las columnas son los campos de
[scheduled_events](#scheduled_events), en este orden:

`id`, `tenant_id`, `election_event_id`, `created_at`, `stopped_at`, `archived_at`, `labels`,
`annotations`, `event_processor`, `cron_config`, `event_payload`, `task_id`

La importación:

- Da a cada evento programado un ID nuevo, y no usa `id`, `tenant_id` ni `election_event_id`.
- Reemplaza el `election_id` de `event_payload` por el ID nuevo de la elección.
- Crea un `task_id` nuevo.
- Borra `stopped_at` y `archived_at`. Todos los eventos programados importados están activos.

### Archivo de claves del gestor del protocolo {#protocol-manager-keys-file}

`export_protocol_manager_keys-<ID>.csv`. Las claves secretas de los tablones.

| Columna | Contenido |
| --- | --- |
| `election_id` | Vacía para el tablón del proceso electoral. Si no, el ID de la elección del tablón. |
| `key` | La clave del gestor del protocolo del tablón. |

El archivo debe tener una fila para el proceso electoral y una fila para cada elección. Si falta una
fila, la importación falla. Si este archivo está presente, la importación no crea claves nuevas del
gestor del protocolo.

:::danger ADVERTENCIA: archivo de claves del gestor del protocolo
Mantenga este archivo en secreto. Las claves del gestor del protocolo protegen los tablones del proceso
electoral.
:::

### Archivo de tablones {#bulletin-boards-file}

`export_bulletin_boards-<ID>.csv`. Una fila para cada mensaje de los tablones.

| Columna | Contenido |
| --- | --- |
| `election_id` | Vacía para el tablón del proceso electoral. Si no, el ID de la elección del tablón. |
| `id` | El número del mensaje en el tablón. |
| `created` | Cuándo se creó el mensaje (una marca de tiempo). |
| `sender_pk` | La clave pública del remitente del mensaje. |
| `statement_timestamp` | La marca de tiempo de la declaración del mensaje. |
| `statement_kind` | El tipo de declaración. |
| `batch` | El número de lote. |
| `mix_number` | El número de mezcla. |
| `message` | El mensaje, en Base64 sin relleno. |
| `version` | La versión del formato del mensaje. |

### Archivos de datos del recuento {#tally-data-files}

La carpeta `export_tally_data/` contiene diez archivos CSV. La importación lee
`export_tally_session.csv` y `export_results_event.csv` antes que los otros archivos.

En la mayoría de los archivos hay estas columnas:

| Columna | Contenido |
| --- | --- |
| `id` | El ID de la fila. La importación da un ID nuevo. |
| `tenant_id`, `election_event_id` | El inquilino y el proceso electoral. La importación los reemplaza. |
| `election_id`, `contest_id`, `area_id`, `candidate_id` | La elección, pregunta, área o candidato de la fila. La importación los reemplaza por los ID nuevos. |
| `results_event_id` | Los resultados de la sesión de recuento. |
| `created_at`, `last_updated_at` | Cuándo se creó la fila y cuándo cambió por última vez. |
| `labels`, `annotations` | Datos libres clave-valor como JSON. |
| `documents` | Los documentos de los resultados (por ejemplo PDF, HTML, JSON) como JSON. |
| `name` | El nombre de los resultados o del área. |

Los archivos y sus otras columnas:

| Archivo | Contenido | Otras columnas |
| --- | --- | --- |
| `export_tally_session.csv` | Las sesiones de recuento. | `election_ids`, `area_ids`, `is_execution_completed`, `keys_ceremony_id`, `execution_status`, `threshold`, `configuration`, `tally_type` (`ELECTORAL_RESULTS` o `INITIALIZATION_REPORT`), `permission_label` |
| `export_tally_session_contest.csv` | Las preguntas y áreas de cada sesión de recuento. | `session_id`, `tally_session_id` |
| `export_tally_session_execution.csv` | Los pasos de cada sesión de recuento. | `current_message_id`, `tally_session_id`, `session_ids`, `status` |
| `export_results_event.csv` | Una fila para cada conjunto de resultados. | — |
| `export_results_election.csv` | Los resultados de cada elección. | `elegible_census`, `total_voters`, `total_voters_percent` |
| `export_results_election_area.csv` | Los resultados de cada elección en cada área. | — |
| `export_results_contest.csv` | Los resultados de cada pregunta. | `elegible_census`, `total_votes`, `total_valid_votes`, `total_invalid_votes`, `explicit_invalid_votes`, `implicit_invalid_votes`, `total_blank_votes`, `explicit_blank_votes`, `implicit_blank_votes`, `total_auditable_votes`, la columna `_percent` de cada conteo, `voting_type`, `counting_algorithm` |
| `export_results_area_contest.csv` | Los resultados de cada pregunta en cada área. | Los mismos conteos y columnas `_percent` que `export_results_contest.csv` |
| `export_results_contest_candidate.csv` | Los resultados de cada candidato. | `cast_votes`, `cast_votes_percent`, `winning_position`, `points` |
| `export_results_area_contest_candidate.csv` | Los resultados de cada candidato en cada área. | `cast_votes`, `cast_votes_percent`, `winning_position`, `points` |

Las columnas `_percent` son los mismos conteos expresados como porcentaje. `elegible_census` es el
número de votantes que pueden votar.

## Páginas relacionadas {#related-pages}

- [Crear el proceso electoral](../../03-procedures/02-event.md)
- [Formatos de archivo de importación](index.md)
