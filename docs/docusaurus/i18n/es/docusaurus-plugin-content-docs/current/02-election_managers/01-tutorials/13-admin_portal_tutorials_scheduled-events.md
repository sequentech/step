---
id: admin_portal_tutorials_scheduled-events
title: Eventos programados
description: "La vía principal para programar el período de votación está en Publicar y gestionar el período de votación. Esta página describe todos los tipos de eventos programados, su efecto y las reglas que se aplican cuando se ejecutan."
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La vía principal para programar el período de votación está en
[Publicar y gestionar el período de votación](../03-procedures/05-publish.md#7-schedule-the-voting-period-alternative). Esta página
describe todos los tipos de eventos programados, su efecto y las reglas que se aplican cuando se ejecutan.

Un evento programado realiza un cambio en el proceso electoral en una fecha y hora determinadas, sin
ninguna acción de una persona. Los eventos programados están en la pestaña **Eventos Programados** del proceso electoral.

## Permisos {#permissions}

| Permiso | Le permite |
| --- | --- |
| **Ver Programación del Evento Electoral** | Ver la pestaña **Eventos Programados**. |
| **Crear Evento Programado** y **Editar Eventos Programados** | Crear un evento programado. |
| **Editar Eventos Programados** | Cambiar la fecha de un evento programado. |
| **Eliminar Evento Programado** | Eliminar un evento programado. |

## Tipos de eventos programados {#types-of-scheduled-events}

| **Tipo** | Campo **Elección** | Efecto cuando se ejecuta el evento |
| --- | --- | --- |
| **Inicio del Período de Votación** | Opcional | Abre los canales de votación seleccionados. |
| **Fin del Período de Votación** | Opcional | Cierra los canales de votación seleccionados. |
| **Allow Initialization Report** | Obligatorio | Establece el estado de la elección para que se permitan los informes de inicialización. Consulte [Informe de inicialización](11-admin_portal_tutorials_initialization-report.md). |
| **Allow Voting Period End** | Obligatorio | Permite el fin del período de votación de la elección. **Detener Votación en Línea** pasa a estar disponible si la configuración de la elección no lo permitía. |
| **Permitir Recuento** | Obligatorio | Establece **Permitir Recuento** de la elección en **Permitido**. |
| **Inicio del Período de Inscripción** | No se muestra | Permite que los votantes se registren por sí mismos y establece **Inscripción** del proceso electoral en **Habilitado**. |
| **Fin del Período de Inscripción** | No se muestra | Detiene el registro de votantes y establece **Inscripción** en **Deshabilitado**. |
| **Inicio del Período de Bloqueo de Datos Censales** | No se muestra | Establece **Estado de Confinamiento** del proceso electoral en **Confinado**. |
| **Fin del Período de Bloqueo de Datos Censales** | No se muestra | Establece **Estado de Confinamiento** en **No Confinado**. |

En **Inicio del Período de Votación** y **Fin del Período de Votación**, deje **Elección** vacío para aplicar el
evento a todas las elecciones del proceso electoral.

## Crear un evento programado {#create-a-scheduled-event}

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Eventos Programados**.
3. Haga clic en **Crear Evento Programado**.
4. En **Tipo**, seleccione el tipo.
5. Si el formulario muestra **Elección**, seleccione la elección.
6. En el caso de **Inicio del Período de Votación** o **Fin del Período de Votación**, seleccione los canales en
   **CANALES DE VOTACIÓN**.
7. Escriba la fecha y la hora.
8. Haga clic en el botón de guardar.

**Resultado esperado:** la lista muestra el evento con su fecha en **Programado En**. **Detenido En**
muestra «-».

:::note NOTA
Después de guardar, el portal de administración puede mostrar el mensaje «Evento Programado editado correctamente», también para un evento nuevo.
:::

## Canales de votación {#voting-channels}

El campo **CANALES DE VOTACIÓN** muestra **En línea**, **Kiosco**, **Votación anticipada** y
**Votación telefónica**. **En línea** y **Kiosco** vienen seleccionados de forma predeterminada. Debe quedar
seleccionado al menos un canal.

- El evento solo cambia los canales permitidos en **Canales de Voto Permitidos** de cada elección. Ignora los
  demás canales seleccionados.
- Un evento de **Inicio del Período de Votación** no puede seleccionar **En línea** y **Votación anticipada** a la vez.
  El formulario muestra el mensaje «Un horario de inicio no puede abrir a la vez En línea y Votación anticipada: la votación anticipada debe comenzar antes que la votación en línea.»
  y el botón de guardar no está disponible.
- Para tener votación anticipada antes que la votación en línea, cree dos eventos de **Inicio del Período de Votación**.
  Cree uno para **Votación anticipada** y otro para **En línea**, en un momento posterior.

## Fecha, hora y zona horaria {#date-time-and-time-zone}

El portal de administración usa la zona horaria de su ordenador. La etiqueta del campo de fecha la muestra,
por ejemplo **Fecha y hora de inicio (Europe/Madrid)**. Para todos los tipos excepto **Inicio del Período de Votación**, la
etiqueta es **Fecha y hora de finalización** con la zona horaria. La lista también muestra las fechas en la zona horaria de su ordenador.

:::caution PRECAUCIÓN
Compruebe la zona horaria de la etiqueta antes de guardar. Si su ordenador no está en la zona horaria de la
elección, convierta las horas.
:::

## Cómo se ejecuta un evento programado {#how-a-scheduled-event-runs}

La plataforma comprueba los eventos programados a intervalos breves. Cuando se alcanza la fecha de un evento,
la plataforma lo ejecuta y rellena **Detenido En** con la hora. Un evento se ejecuta una sola vez. Si guarda
una fecha pasada, el evento se ejecuta en la siguiente comprobación.

Los eventos del período de votación siguen estas reglas:

- **Inicio del Período de Votación** abre un canal solo si su estado es `NOT_STARTED` o `PAUSED`. No vuelve a abrir un canal
  después de que esté `CLOSED`.
- **Fin del Período de Votación** cierra un canal solo si su estado es `OPEN` o `PAUSED`. No cambia un canal que nunca se inició.
- Un inicio de votación anticipada omite cada elección en la que la votación en línea ya se ha iniciado.
- En un evento con elección, la plataforma aplica las mismas comprobaciones que la pestaña **Publicar** de la
  elección. Por ejemplo, si el informe de inicialización es obligatorio y falta, la elección no se abre.

:::caution PRECAUCIÓN
Después de la fecha, revise el estado de los canales de votación en la pestaña **Publicar**. Si falla una comprobación,
**Detenido En** queda vacío y la plataforma vuelve a intentar el evento en cada comprobación. El cambio
puede producirse más tarde, cuando se elimine la causa. Corrija la causa o elimine el evento y cambie el
estado a mano. Consulte
[Iniciar, pausar y detener el período de votación](14-admin_portal_tutorials_start-pause-stop-election_copy.md).
:::

## Un evento de cada tipo {#one-event-of-each-type}

Cada elección tiene un evento programado de cada tipo. El proceso electoral también tiene un evento de cada tipo
para los eventos sin elección. Si crea un evento de un tipo que ya existe para la misma elección, la plataforma
cambia la fecha del evento existente.

## Cambiar o eliminar un evento programado {#change-or-delete-a-scheduled-event}

Para cambiar la fecha:

1. En la fila del evento, haga clic en el icono de editar.
2. Cambie la fecha y la hora. También puede cambiar los canales de votación.
3. Haga clic en el botón de guardar.

No puede cambiar el **Tipo** ni la **Elección**. Para cambiarlos, elimine el evento y cree uno nuevo.

Para eliminar un evento:

1. En la fila del evento, haga clic en el icono de eliminar.
2. Lea el mensaje «¿Estás seguro de que quieres eliminar este Evento Programado?».
3. Haga clic en **Eliminar**.

**Resultado esperado:** el evento sale de la lista. No se ejecuta.

## Buscar un evento {#find-an-event}

Use los filtros de la lista: **Tipo**, **Elección** y el identificador del evento. La lista tiene las columnas
**Elección**, **Tipo**, **Detenido En** y **Programado En**.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| El botón de guardar no está disponible. | Asegúrese de que el formulario tenga un tipo, una fecha y, si es necesario, una elección. No seleccione **En línea** y **Votación anticipada** a la vez para un inicio. |
| «Error creando Evento Programado» | Lea el mensaje y corrija el formulario. Si el mensaje continúa, póngase en contacto con el soporte de Sequent. |
| «Error editando Evento Programado» | El cambio o la eliminación no se produjo. Inténtelo de nuevo y, si sigue fallando, póngase en contacto con el soporte de Sequent. |
| El evento tiene una hora en **Detenido En**, pero el estado del canal no cambió. | El canal no estaba permitido en la elección, o su estado no permitía el cambio. Cambie el estado en la pestaña **Publicar**. |
| La fecha ya pasó, pero **Detenido En** está vacío. | Una comprobación falló, por ejemplo, falta un informe de inicialización obligatorio. Corrija la causa o elimine el evento. |
| El evento se ejecutó a una hora incorrecta. | Compruebe la zona horaria de su ordenador y la etiqueta del campo de fecha. |
| Un canal no se ve afectado por el evento. | Permita el canal en **Canales de Voto Permitidos** de la elección. |
| No se muestra la pestaña **Eventos Programados**. | Pida el permiso **Ver Programación del Evento Electoral**. Si el proceso electoral está confinado, la pestaña puede estar oculta. |
