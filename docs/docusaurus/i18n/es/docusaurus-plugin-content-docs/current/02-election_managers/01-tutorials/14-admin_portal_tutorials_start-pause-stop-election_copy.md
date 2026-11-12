---
id: admin_portal_tutorials_start-pause-stop-election
title: Iniciar, pausar y detener el período de votación
description: "La vía principal está en Publicar y gestionar el período de votación. Esta página describe el modelo de estados de los canales de votación, las reglas de cada elemento del menú y los casos límite."
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La vía principal está en [Publicar y gestionar el período de votación](../03-procedures/05-publish.md#5-open-the-voting-period). Esta
página describe el modelo de estados de los canales de votación, las reglas de cada elemento del menú y los casos límite.

## Canales de votación y su estado {#voting-channels-and-their-status}

El período de votación tiene un estado separado para cada canal de votación: **En línea**, **Kiosco**,
**Votación anticipada** y **Votación telefónica**. Cada elección tiene estos cuatro estados. El proceso electoral
también los tiene.

| Estado | Significado |
| --- | --- |
| `NOT_STARTED` | El canal no se ha abierto. Este es el estado al inicio. |
| `OPEN` | Los votantes pueden votar en el canal. |
| `PAUSED` | La votación está detenida durante un tiempo. Puede volver a abrir el canal. |
| `CLOSED` | El período de votación del canal ha terminado. |

Un canal puede cambiar de estado como se indica a continuación:

| De | A |
| --- | --- |
| `NOT_STARTED` | `OPEN` |
| `OPEN` | `PAUSED` o `CLOSED` |
| `PAUSED` | `OPEN` o `CLOSED` |
| `CLOSED` | No desde el portal de administración |

:::danger ADVERTENCIA
Cierre un canal solo al final oficial de su período de votación. El portal de administración no le permite volver a
abrir un canal cerrado.
:::

## Las dos pestañas Publicar {#the-two-publish-tabs}

| Pestaña | Efecto de **Iniciar Votación** y **Detener Votación** |
| --- | --- |
| Pestaña **Publicar** del proceso electoral | Cambia el canal del proceso electoral y de todas sus elecciones. |
| Pestaña **Publicar** de una elección | Cambia el canal solo de esa elección. |

Las comprobaciones de las políticas de una elección solo se aplican en la pestaña **Publicar** de la elección:

- **Política de informe consolidado** = **Requerido**: la elección no puede abrirse hasta que su informe de inicialización esté completo. Consulte [Informe de inicialización](11-admin_portal_tutorials_initialization-report.md).
- El fin del período de votación no está permitido: la elección no puede cerrarse hasta que se ejecute un evento programado **Allow Voting Period End**. Consulte [Eventos programados](13-admin_portal_tutorials_scheduled-events.md).

:::caution PRECAUCIÓN
Si una elección usa una de estas políticas, ábrala y ciérrela desde la pestaña **Publicar** de la elección. La
pestaña **Publicar** del proceso electoral no aplica estas comprobaciones.
:::

## Cuándo está disponible un elemento del menú {#when-a-menu-item-is-available}

**Iniciar Votación**, **Pausar Votación** y **Detener Votación** abren cada uno un menú con un elemento por cada
canal, por ejemplo **Comenzar Votación en Línea**.

| Elemento del menú | Disponible cuando |
| --- | --- |
| **Comenzar Votación en ...** | El canal está permitido y su estado es `NOT_STARTED` o `PAUSED`. |
| **Pausar Votación en ...** | El canal está permitido y su estado es `OPEN`. |
| **Detener Votación en ...** | El canal está permitido y su estado es `OPEN` o `PAUSED`. |

Un canal está permitido cuando está seleccionado en **Canales de Voto Permitidos** de la pestaña **Datos**. Se aplican dos
reglas más en la pestaña **Publicar** de una elección:

- **Comenzar Votación en Línea** no está disponible si el informe de inicialización es obligatorio y no está completo.
- **Detener Votación en Línea** no está disponible si la configuración de la elección no permite el fin del período de votación.

Un botón no está disponible cuando ninguno de los elementos de su menú está disponible. Los botones tampoco están
disponibles mientras se realiza un cambio de estado o mientras se genera una publicación.

## Votación anticipada y votación en línea {#early-voting-and-online-voting}

La votación anticipada es un período anterior a la votación en línea. Se aplican estas reglas:

- Abra la **Votación anticipada** antes que la **Votación en línea**.
- No puede comenzar la votación anticipada después de que haya empezado la votación en línea, aunque la votación en línea esté ahora pausada o cerrada. La plataforma rechaza el cambio.
- Cuando la votación en línea se abre o se cierra, la plataforma cierra automáticamente la votación anticipada, si esta ya había empezado.

## Confirme su identidad {#confirm-your-identity}

Un cambio del estado de votación es una acción sensible. Cuando hace clic en un elemento del menú, el cuadro de
diálogo **Confirmar Acción** muestra:

- Si inició sesión hace menos de 60 segundos con un inicio de sesión seguro, el mensaje es «Está a punto de comenzar el período de votación. ¿Está seguro de que desea continuar?». El texto indica pausar o detener para las demás acciones. Haga clic en **Confirmar**.
- En todos los demás casos, el mensaje le pide su contraseña. Haga clic en **Confirmar** y, después, vuelva a iniciar sesión. El portal de administración realiza entonces la acción que seleccionó.

Consulte [Antes de empezar](../00-before-you-start.md#sensitive-actions).

## Permisos {#permissions}

Para ver los botones, su cuenta necesita **Editar Estado de Elección** y el permiso del botón: **Iniciar Votación**,
**Pausar Votación** o **Detener Votación**.

## Lo que registra la plataforma {#what-the-platform-records}

Cada cambio de estado se registra en el registro electoral del proceso electoral, con el canal y las elecciones.
Consulte [Registros electorales](19-admin_portal_tutorials_election-logs.md).

## Cambios automáticos {#automatic-changes}

Los eventos programados pueden abrir y cerrar canales a una hora determinada. Nunca abren un canal cerrado.
Consulte [Eventos programados](13-admin_portal_tutorials_scheduled-events.md).

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| «Error al cambiar el estado de la publicación» | La plataforma rechazó el cambio. Revise el estado del canal y las reglas de esta página. Vuelva a iniciar sesión y, a continuación, inténtelo de nuevo. |
| **Comenzar Votación en Línea** no está disponible. | Asegúrese de que el canal esté permitido y no esté cerrado. Si el informe de inicialización es obligatorio, realícelo. |
| **Detener Votación en Línea** no está disponible. | El fin del período de votación todavía no está permitido. Revise en la pestaña **Eventos Programados** si hay un evento **Allow Voting Period End**. |
| **Comenzar Votación Anticipada** falla. | La votación en línea ya ha empezado. No puede abrir la votación anticipada ahora. |
| Un canal cerrado debe volver a abrirse. | Póngase en contacto con el soporte de Sequent. El portal de administración no puede hacerlo. |
| Ninguno de los botones está disponible. | Espere a que termine el cambio de estado o la publicación en curso. Asegúrese de que un canal esté permitido. |
| No se muestran los botones. | Pida los permisos **Editar Estado de Elección**, **Iniciar Votación**, **Pausar Votación** y **Detener Votación**. |
