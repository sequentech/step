---
id: election_management_election_event_scheduled_events
title: Eventos programados
description: "La pestaña Eventos Programados hace que la plataforma realice una acción en una fecha y hora fijadas, por ejemplo abrir o cerrar el período de votación."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Eventos Programados** hace que la plataforma realice una acción en una fecha y hora
fijadas, por ejemplo abrir o cerrar el período de votación. El encabezado de la pestaña dice
"Gestiona la configuración de la ejecución automática de eventos como el inicio o fin del período
de votación."

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, haga clic en el proceso electoral.
2. Haga clic en la pestaña **Eventos Programados**.

La pestaña se muestra con el permiso **Ver Programación del Evento Electoral**. Está oculta cuando
el proceso electoral está confinado. Las acciones necesitan estos permisos:

| Acción | Permiso |
| --- | --- |
| Crear | **Crear Evento Programado** |
| Cambiar | **Editar Eventos Programados** |
| Borrar | **Eliminar Evento Programado** |
| Seleccionar las columnas | **Columnas de los Eventos Programados del Evento Electoral** |

## Lista {#list}

Cuando la lista está vacía, la pestaña muestra "No hay Eventos Programados todavía." y **Crear
Evento Programado**. Cuando hay eventos, la barra de herramientas tiene **Añadir**.

| Columna | Contenido |
| --- | --- |
| **Elección** | La elección del evento. |
| **Tipo** | El tipo del evento. |
| **Detenido En** | Cuándo se detuvo el evento, si se detuvo. |
| **Programado En** | La fecha y la hora del evento. |
| **Acciones** | El icono de editar y el icono de borrar. |

## Tipos de evento {#event-types}

| Tipo | Acción en la fecha y la hora | **Elección** |
| --- | --- | --- |
| **Inicio del Período de Votación** | Abre el período de votación. | Opcional. Vacío significa todas las elecciones. |
| **Fin del Período de Votación** | Cierra el período de votación. | Opcional. Vacío significa todas las elecciones. |
| **Allow Initialization Report** (permitir el informe de inicialización) | Permite el informe de inicialización de la elección. | Obligatoria |
| **Allow Voting Period End** (permitir el fin del período de votación) | Permite el fin del período de votación de la elección. **Detener votación** no está disponible mientras el fin no está permitido. | Obligatoria |
| **Permitir Recuento** | Pone **Permitir Recuento** de la elección en **Permitido**. | Obligatoria |
| **Inicio del Período de Inscripción** | Abre la inscripción de votantes. | No se usa |
| **Fin del Período de Inscripción** | Cierra la inscripción de votantes. | No se usa |
| **Inicio del Período de Bloqueo de Datos Censales** | Confina el proceso electoral. | No se usa |
| **Fin del Período de Bloqueo de Datos Censales** | Termina el confinamiento. | No se usa |

## Crear un evento programado {#create-a-scheduled-event}

1. Haga clic en **Añadir**. Si la lista está vacía, haga clic en **Crear Evento Programado**.
2. En **Tipo**, seleccione el tipo.
3. En **Elección**, seleccione la elección, si el tipo usa una.
4. Escriba la fecha y la hora. La etiqueta del campo muestra su zona horaria, por ejemplo "Fecha y
   Hora de inicio (Europe/Madrid)".
5. Haga clic en **Guardar**.

**Resultado esperado:** el evento se muestra en la lista.

:::caution PRECAUCIÓN
El portal de administración usa la zona horaria de su ordenador. Si su ordenador no está en la
zona horaria de la elección, convierta las horas antes de escribirlas.
:::

No puede cambiar el **Tipo** ni la **Elección** de un evento existente. Para cambiarlos, borre el
evento y cree uno nuevo.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| "Error al crear el Evento Programado" | Compruebe el tipo, la elección y la fecha, y guarde de nuevo. |
| "Error al editar el Evento Programado" | Compruebe la fecha y guarde de nuevo. |
| El período de votación no se abrió a la hora. | Compruebe la zona horaria de la fecha. Compruebe que la papeleta está publicada y que existe el informe de inicialización, si es obligatorio. |
| **Detener votación** no está disponible en la pestaña **Publicar**. | El fin del período de votación todavía no está permitido. Compruebe el evento **Allow Voting Period End** de la elección. |

## Páginas relacionadas {#related-pages}

- [Publicar y gestionar el período de votación](../../../../../procedures/05-publish.md#7-schedule-the-voting-period-alternative)
- [Publicar](../Publish/election_management_election-event_publish.md)
- [Configurar una elección automatizada](../../../../../Tutorials/admin_portal_tutorials_setting-up-an-automated-election.md)
