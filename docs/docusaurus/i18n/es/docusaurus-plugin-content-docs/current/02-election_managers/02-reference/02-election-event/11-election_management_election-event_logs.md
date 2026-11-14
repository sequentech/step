---
id: election_management_election_event_logs
title: Logs
description: "La pestaña Logs ofrece una vista global de todas las actividades en curso, con información detallada sobre el funcionamiento del sistema."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


La pestaña **Logs** ofrece una vista global de todas las actividades en curso, con información detallada sobre el funcionamiento del sistema.

Estas son las acciones a nivel de aplicación que se registran. Acceda al registro del sistema para supervisar la actividad de toda la plataforma, incluidos los eventos de votantes, los eventos de Keycloak, los eventos del sistema y los eventos de usuario.

- Seleccione **Columnas** para ocultar o mostrar distintos datos de cada entrada del registro.

## Inmutabilidad {#immutability}

La pestaña **Logs** se implementa con Immudb, una base de datos a prueba de manipulaciones. Cada entrada se inserta en una tabla SQL de acciones. La inmutabilidad permite verificar que no se ha realizado ningún cambio, para cualquier persona con credenciales de acceso al registro inmutable.

## Entradas de la API del sistema externo (Datafix) {#external-system-datafix-api-entries}

Cada llamada que un sistema externo de registro de votantes realiza a la API de Datafix
(`add-voter`, `update-voter`, `delete-voter`, `mark-voted`, `unmark-voted`,
`replace-pin`), y cada solicitud `SetVoted`/`SetNotVoted` que la plataforma envía de vuelta
a ese sistema, se registra como una entrada `ExternalApiRequest`.

- **Descripción** indica solo la operación y su resultado, por ejemplo `Inbound request UpdateVoter Succeeded.` o `Inbound request ReplacePin Failed.`
- **Mensaje** contiene los detalles en la cadena de operación del cuerpo de la declaración:

  ```text
  voter_id="<datafix voter id>"; <Operation> <Outcome>[: <reason>] (<key>=<value>, ...)
  ```

  Los valores que proceden de texto libre (el ID del votante, el nombre del área, el canal) y los valores leídos de vuelta desde Keycloak (`area_id`, `previous_area_id`, `previous_birthdate`, que son atributos que un administrador puede editar) se escriben entre comillas dobles, con `"` y `\` escapados. Así, un valor que contenga `, ` o `=` se sigue leyendo como un único valor y no como otro campo. Solo el vocabulario fijo (`none`, `unchanged`, `true`, `false`) y el `birthdate` validado de la solicitud se escriben sin comillas.

  | Operación | Valores registrados con éxito |
  |---|---|
  | `AddVoter` | `area`, `area_id`, `birthdate` (`none` si no se envía), `enabled=true` |
  | `UpdateVoter` | Lo que el votante tenía antes de la escritura (`previous_area`, `previous_area_id`, `previous_birthdate`, `previous_enabled`; `none` cuando el votante no tenía ese valor), junto con lo que aplicó la solicitud (`area`, `area_id`, `birthdate`, `enabled`); un campo que no se envió se escribe como `unchanged` |
  | `DeleteVoter` | `enabled=false` y el `disable_comment` escrito en el votante (los votantes de Datafix se deshabilitan, nunca se eliminan) |
  | `MarkVoted` | `channel`, `enabled=false`, `disable_comment` |
  | `UnmarkVoted` | `previous_channel`, `channel=NONE`, y si la cuenta se volvió a habilitar (`enabled`, `disable_comment`); `unchanged` significa que se conservó la deshabilitación hecha por un administrador |
  | `ReplacePin` | `temporary` (el PIN en sí nunca se registra) |

  Una operación fallida registra el motivo interno y el código de error devuelto al sistema externo, por ejemplo
  `voter_id="123456"; ReplacePin Failed: Cannot replace pin because the user is disabled (error_code=invalid-request)`.
  El motivo solo se registra aquí; la respuesta de la API contiene únicamente el código de error.

  Antes de escribir en Keycloak, una operación entrante comprueba que el perfil de usuario del proceso electoral almacena cada atributo que va a escribir (por ejemplo, `dateOfBirth`, `voted-channel` o `disable-comment`). Si el realm descartara alguno sin avisar, la operación falla sin cambiar al votante, el sistema externo recibe `internal-error` y la entrada indica los atributos que el realm no almacena.

  Solo estas operaciones entrantes realizan la comprobación. Una edición del votante desde el portal de administración, el canal de Internet registrado cuando se emite un voto y las ediciones que aplica la reconciliación a votantes existentes escriben a través de Keycloak sin esta comprobación. En un realm que no declara un atributo, y cuya política de atributos no gestionados no permite que los administradores lo escriban, su valor se descarta sin avisar. Los votantes que añade la reconciliación se escriben directamente en la base de datos y conservan sus atributos, independientemente de lo que declare el realm.

  Una solicitud se atribuye al proceso electoral cuya anotación `datafix:id` coincide con el ID de Datafix del token del llamante. Si varios procesos electorales del inquilino comparten ese ID, no se puede atribuir la solicitud a ninguno de ellos y se rechaza con `internal-error` antes de tocar ningún votante, ya que solo un administrador puede corregir la configuración. La entrada fallida se registra entonces en el registro electoral de **todos** los procesos electorales configurados con ese ID, nombrándolos todos en el motivo, con solo el nombre de usuario del votante (sin `user_id` ni `area_id`, porque el votante no puede identificarse sin un proceso electoral).
- **ID de usuario** y **Nombre de Usuario** identifican al votante (el ID del votante de Datafix es el nombre de usuario), y el `area_id` del mensaje es el área del votante, como en los eventos de Keycloak. Un campo para el que la entrada no tiene valor se omite del mensaje en lugar de escribirse como `null`: `election_id` siempre, porque estas operaciones se aplican a todo el proceso electoral, y `user_id`/`area_id` cuando el votante no puede identificarse, como en un `AddVoter` que falló antes de crear el votante.

### Cuerpos de las solicitudes en los registros del servicio {#request-bodies-in-the-service-logs}

Además de la entrada del registro electoral, el servicio de la API registra el cuerpo completo de cada solicitud entrante de Datafix en su salida de trazas, para que una solicitud rechazada pueda depurarse sin reproducirla. Estos datos no se consideran sensibles a propósito: el ID del votante es un identificador anónimo del registro, y los mismos valores ya se guardan en la entrada del registro electoral descrita arriba y en los archivos de votantes que se intercambian con el sistema externo. Solo el token del llamante se mantiene fuera de los registros del servicio.
