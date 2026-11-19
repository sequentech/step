---
id: election_management_election_event_voters
title: Votantes
description: "Esta sección muestra los votantes configurados actualmente para este proceso electoral."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


Esta sección muestra los votantes configurados actualmente para este proceso electoral.

### Descripción general {#overview}

Esta pestaña muestra y gestiona los votantes. La tabla enumera todos los votantes configurados actualmente.

- **Columnas**: Habilita o deshabilita las columnas que se muestran en la tabla.
- **Añadir filtro**: Crea un filtro de texto por columna.
- **Añadir**: Introduzca la información de un votante y añádalo al proceso electoral.
- **Importar**: Importa votantes desde un archivo CSV.
- **Exportar**: Exporta la lista de votantes como archivo CSV.
- **Enviar**: Envía un mensaje a los votantes mediante la pestaña **Enviar notificación**.
- **Filtros personalizados**: Usa filtros predefinidos.
  - Estos filtros pueden definirse en:  
    `Election Event > Data > Advanced Configuration`

---

### Acciones {#actions}

La columna **Acciones** ofrece opciones para interactuar con los registros de votantes:

- **Enviar**: Envía una notificación al votante.
- **Editar**: Modifica los datos del votante o cambia su área asignada.
- **Eliminar**: Elimina el votante del sistema.
- **Verificar manualmente**: Confirma la identidad del votante (véase más abajo).
- **Cambiar contraseña**: Actualiza la contraseña del votante (véase más abajo).
- **Registros del usuario**: Muestra las acciones realizadas por el votante.

---

#### Añadir y editar un votante {#adding-and-editing-a-voter}

Los campos se comprueban al salir de ellos. Un campo obligatorio que se deja vacío, o cuyo valor incumple un límite configurado para ese atributo, se marca y explica qué espera, y el votante no se puede guardar hasta corregirlo. Cuando un atributo establece un límite que conviene conocer de antemano, se indica debajo del campo antes de llegar a él.

Si un votante aún no se puede guardar, el editor permanece abierto con los valores tal como se introdujeron, para no tener que volver a escribir nada, e indica el motivo. Cuando el rechazo afecta a campos concretos, se nombra cada uno junto con la regla que incumple: hasta diez, y los que excedan de esa cifra se cuentan sin enumerarlos.

Los límites proceden de la configuración del atributo en Keycloak; consulte [Añadir atributos de usuario a Keycloak](../../01-tutorials/99-admin_portal_tutorials_add-user-attributes-to-keycloak.md) para saber cómo se establecen y cuáles indica el portal de antemano.

---

#### Enviar notificaciones {#send-notifications}

Use esta pestaña para enviar notificaciones a los votantes mediante distintos métodos, programaciones y plantillas.

- **Audiencia**: Defina a quién va dirigida la notificación.
- **Programación**: Establezca cuándo se enviará la notificación.
- **Plantilla de comunicación**: Elija una plantilla de mensaje predefinida.
- **Método de comunicación**: Correo electrónico / SMS.
- **Tipo de comunicación**: Tipo de contenido (si procede).
- **Alias de la plantilla**: Nombre de la plantilla predefinida.
- **Asunto del email**: Solo aplica al correo electrónico.
- **Cuerpo del mensaje**: Texto sin formato o enriquecido que se enviará.

---

#### Verificación manual {#manual-verification}

Confirme la identidad del votante sin necesidad de pasos de verificación adicionales.

- Aparece un cuadro de diálogo emergente.
- Al escanear el código QR descargable generado, el votante puede:
  - Establecer su contraseña
  - Verificarse a sí mismo
  - Omitir el proceso KYC
- Una vez completado, el votante podrá votar.

---

#### Cambiar contraseña {#change-password}

Cambie la contraseña del votante.

- **Contraseña** y **Repetir Contraseña** deben coincidir.
- Si activa el botón de opción **Temporal**, el votante deberá cambiar su contraseña en el siguiente inicio de sesión.

---

#### Registro del usuario {#users-log}

Vea los registros de todas las acciones realizadas por el votante.

### Campos de datos adicionales del usuario {#additional-user-data-fields}

Puede añadir campos adicionales, como la fecha de nacimiento o el sexo del votante. [Consulte el tutorial para ello](../../01-tutorials/99-admin_portal_tutorials_add-user-attributes-to-keycloak.md).

**Importante:** Los atributos adicionales de los votantes deben añadirse antes del proceso de inscripción. Por ejemplo, si no se añade el atributo de sexo, este dato no aparecerá en los informes ni en las estadísticas.

### Credenciales del cliente de API del sistema externo {#external-system-api-client-credentials}

La API de votantes del sistema externo se autentica con el cliente de Keycloak
`datafix-account` del realm del inquilino, con el flujo de credenciales de cliente
(client credentials). El secreto del cliente es una credencial, así que siga estas reglas:

- Cree el cliente en Keycloak y deje que Keycloak genere su secreto
  (Clients, `datafix-account`, Credentials, Regenerate). No importe nunca un archivo
  de exportación de cliente que tenga un valor `secret` fijo, y no lo guarde nunca en
  un repositorio.
- Entregue el secreto generado al sistema externo por un canal privado.
- Si un cliente de un realm se creó alguna vez a partir de un archivo de exportación
  con un secreto fijo, regenere su secreto y entregue el nuevo al sistema externo.
  La sincronización de votantes se detiene hasta que el sistema externo usa el nuevo
  secreto. La votación y el recuento no se ven afectados.
- Para comprobar un realm, solicite un token `client_credentials` para
  `datafix-account` con el secreto antiguo. Debe fallar con `unauthorized_client`
  donde el cliente existe y con `invalid_client` donde no existe.

### Conciliación de la lista de votantes con un sistema externo {#external-system-voter-list-reconciliation}

Los usuarios con el permiso `election-event-voter-list-reconciliation` pueden abrir el asistente de conciliación desde la pestaña Votantes. Ejecute la conciliación solo durante una ventana de congelación acordada con el sistema externo:

1. Suba el archivo de conciliación completo del sistema externo. La primera línea debe ser la línea `#META` y los canales de los archivos deben escribirse en mayúsculas.
2. Revise las tablas del sistema externo y de Sequent. Si la tabla del sistema externo no está vacía, descargue su parche, haga que el sistema externo lo aplique y suba el archivo de conciliación generado de nuevo. Nunca aplique primero los cambios de Sequent.
3. Cuando la tabla del sistema externo esté vacía, revise los totales por categoría y aplique explícitamente los cambios de Sequent.
4. Revise la tabla de fallos de fila. Una aplicación completada puede contener filas rechazadas de forma segura; son resultados de conciliación a nivel de negocio, no una tarea en segundo plano fallida. Resuélvalos y reintente la misma `Sequence`.
5. Suba el último archivo de nuevo como comprobación de convergencia solo con diferencias. Ambas tablas deben estar vacías para considerar que la ronda ha convergido.
6. Compare los hashes de origen y de parche con los registros electorales antes de terminar la congelación.

Una papeleta en línea que sigue en `in-progress` se notifica deliberadamente como fallo de fila. Durante una congelación con el sistema externo caído, no puede resolverse, porque el latido de revisión debe llegar al sistema externo. Si todos los fallos restantes tienen esta causa, complete las comprobaciones de hash de la congelación, restablezca la conectividad con el sistema externo, espere a que el latido de revisión resuelva las papeletas y reintente la misma `Sequence`. La validación de la instantánea en el momento de aplicar evita que el reintento sobrescriba datos del votante cambiados entretanto.

Tanto la operación en tiempo real `/unmark-voted` como la conciliación de archivos borran la marca del canal de voto. Solo vuelven a habilitar a un votante cuando la operación `MarkVoted` correspondiente es la que tiene la deshabilitación. Una deshabilitación independiente hecha por un administrador, con su comentario, se conserva, pero el canal se borra igualmente para que el votante no quede bloqueado de forma permanente en un estado sin resolver con el sistema externo. El administrador puede decidir después, por separado, si la cuenta debe volver a habilitarse.

El resultado de la tarea muestra como máximo los primeros 1.000 detalles de fallos de fila de la aplicación y siempre informa del número total de fallos. Resuelva la causa común y reintente la misma `Sequence` para ver los fallos restantes.

#### Validación de capacidad {#capacity-validation}

Antes de usarlo en producción, las pruebas de aceptación (UAT) deben probar la lista de votantes más grande prevista, incluida una primera sincronización en la que la mayoría de los votantes son altas. Para una prueba de 100.000 votantes, registre los tamaños del sobre generado y de los artefactos de auditoría, el pico de memoria del backend, el pico de memoria del navegador, la capacidad de respuesta del renderizado del asistente y la duración total de la aplicación en Keycloak. Estas mediciones determinan el tamaño de lote operativo y la duración de la ventana de congelación para ese despliegue.
