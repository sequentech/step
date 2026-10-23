---
title: Publicar y gestionar el período de votación
sidebar_position: 5
description: "Este procedimiento pone la papeleta a disposición en el portal de votación y, después, abre, pausa y cierra el período de votación. Lo hace el administrador electoral."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Publicar y gestionar el período de votación

Este procedimiento pone la papeleta a disposición en el portal de votación y, después, abre, pausa y cierra el período de votación. Lo hace el administrador electoral.

## Antes de empezar {#before-you-start}

- El contenido del proceso electoral está completo y es correcto. Consulte [Crear el proceso electoral](02-event.md).
- La lista de votantes está completa. Consulte [Añadir los votantes](03-voters.md).
- El estado de la ceremonia de claves es `SUCCESS`. Consulte [Realizar la ceremonia de claves](04-keys.md).
- Su cuenta tiene los permisos para publicar y para cambiar el estado de la votación.
- Conoce su contraseña. La publicación y el estado de la votación son acciones sensibles. Consulte [Antes de empezar](../00-before-you-start.md#sensitive-actions).

:::note NOTA
Puede publicar antes de que el estado de la ceremonia de claves sea `SUCCESS`. El portal de votación funciona entonces en modo de demostración: los revisores pueden comprobar cómo se ve el proceso electoral antes de hacerlo definitivo, pero los votos no son reales. Después de la ceremonia de claves, haga una publicación nueva y publíquela. Inicie el período de votación solo cuando el estado de la ceremonia de claves sea `SUCCESS` y haya publicado de nuevo.
:::

:::note NOTA
La pestaña **Publicar** del proceso electoral se aplica a todas sus elecciones. Cada elección también tiene una pestaña **Publicar** que se aplica solo a esa elección.
:::

## 1. Generar una publicación {#1-make-a-publication}

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Publicar**.
3. Haga clic en **Generar Publicación**. Si el proceso electoral ya tiene una publicación, haga clic en **Publicar Cambios**.
4. Si se abre la ventana **Confirmar Acción**, lea el mensaje y haga clic en **Confirmar**.
5. Si el portal de administración le pide su contraseña, escríbala.

**Resultado esperado:** aparece el mensaje "Papeleta generada". La página **Cambios a Publicar** compara la publicación **Actual** con los **Cambios a publicar**.

6. Lea los cambios. Asegúrese de que son los cambios que espera.

![La página Cambios a Publicar](../../../../../../docs/02-election_managers/01-tutorials/assets/publish_diff.png)

## 2. Previsualizar la papeleta {#2-preview-the-ballot}

1. En la página **Cambios a Publicar**, haga clic en **Vista previa**.
2. En **Selecciona el área para la vista previa**, seleccione un área.
3. Abra la vista previa. Para enviar la vista previa a otra persona, haga clic en **Copiar enlace**.
4. Compruebe la papeleta del área: las elecciones, las preguntas, los candidatos, los textos de cada idioma y el número de selecciones.
5. Haga los pasos 2 a 4 para cada área.

Si encuentra un error, corríjalo en el proceso electoral. Después, empiece de nuevo en el paso 1 de [Generar una publicación](#1-make-a-publication).

## 3. Publicar la papeleta {#3-publish-the-ballot}

1. En la página **Cambios a Publicar**, haga clic en **Publicar Cambios**. El portal de administración publica de inmediato, sin una ventana de confirmación.

![La lista Historial de publicaciones](../../../../../../docs/02-election_managers/01-tutorials/assets/publish_history_log.png)

**Resultado esperado:** aparece el mensaje "Papeleta publicada". La lista **Publish History** muestra la nueva publicación.

:::note NOTA
Para cambiar la papeleta después de publicarla, cambie el proceso electoral y haga este procedimiento de nuevo desde el paso 1. Los votantes ven la última publicación.
:::

## 4. Hacer el informe de inicialización (si es necesario) {#4-make-the-initialization-report-if-necessary}

Haga este paso solo si **Inicializar Política de Reportes** de una elección es **Requerido**. El informe de inicialización muestra que la urna está vacía antes de que se abra el período de votación. En la pestaña **Publicar** de la elección, **Comenzar Votación en Línea** no está disponible hasta que exista el informe. Consulte [Informe de inicialización](../01-tutorials/11-admin_portal_tutorials_initialization-report.md).

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Recuento**.
3. Haga clic en **Generar Informe de Inicialización**.
4. Seleccione las elecciones en **Elecciones para el informe de inicialización**.
5. Haga clic en **Iniciar informe de inicialización**.
6. Continúe como en una ceremonia de recuento. Consulte [Realizar la ceremonia de recuento](06-tally.md).

**Resultado esperado:** la lista **Recuento** muestra una fila con el **Tipo de Conteo** **Resultados de Inicialización**.

## 5. Abrir el período de votación {#5-open-the-voting-period}

Cada canal de votación tiene su propio estado. Abra cada canal del proceso electoral. Para abrir el período de votación automáticamente, use [eventos programados](#7-schedule-the-voting-period-alternative). Para ver todos los estados y reglas, consulte [Iniciar, pausar y detener la votación](../01-tutorials/14-admin_portal_tutorials_start-pause-stop-election_copy.md).

:::caution PRECAUCIÓN
La pestaña **Publicar** del proceso electoral no comprueba el informe de inicialización ni el fin del período de votación. Si una elección necesita estas comprobaciones, abra y cierre sus canales en la pestaña **Publicar** de la elección.
:::

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Publicar**.
3. Haga clic en **Comenzar votación**. Un menú muestra un elemento para cada canal.
4. Haga clic en el canal que quiere abrir, por ejemplo **Comenzar Votación en Línea**.
5. En la ventana **Confirmar Acción**, lea el mensaje y haga clic en **Confirmar**.
6. Si el portal de administración le pide su contraseña, escríbala. El portal de administración abre entonces el canal.
7. Haga los pasos 3 a 6 para cada otro canal del proceso electoral.

**Resultado esperado:** aparece el mensaje "Votación cambiada de estado". Los votantes pueden votar en los canales que abrió.

:::note NOTA
Un elemento del menú no está disponible si el canal no está permitido en **Canales de Voto Permitidos**. Abra la **Votación anticipada** antes de la **Votación en Línea**. No puede iniciar la votación anticipada después de que haya empezado la votación en línea.
:::

8. Inicie sesión en el portal de votación con una cuenta de votante de prueba, si tiene una, y asegúrese de que aparece la papeleta.

## 6. Pausar o cerrar el período de votación {#6-pause-or-close-the-voting-period}

Para detener la votación durante un tiempo corto:

1. En la pestaña **Publicar**, haga clic en **Pausar votación**.
2. Haga clic en el canal, por ejemplo **Pausar Votación en Línea**.
3. Confirme la acción.
4. Para continuar, haga clic en **Comenzar votación**, haga clic en el canal y confirme la acción.

Para cerrar el período de votación al final:

:::danger ADVERTENCIA
No puede volver a abrir un canal desde el portal de administración después de cerrarlo. Cierre el período de votación solo en su fin oficial.
:::

1. En la pestaña **Publicar**, haga clic en **Detener votación**.
2. Haga clic en el canal, por ejemplo **Detener Votación en Línea**.
3. Confirme la acción.
4. Haga los pasos 1 a 3 para cada otro canal abierto.

**Resultado esperado:** aparece el mensaje "Votación cambiada de estado". Los votantes no pueden votar en los canales cerrados.

## 7. Programar el período de votación (alternativo) {#7-schedule-the-voting-period-alternative}

Use los eventos programados para abrir y cerrar el período de votación a una hora fijada.

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Eventos Programados**.
3. Haga clic en **Crear Evento Programado**.
4. En **Tipo**, seleccione **Inicio del Período de Votación**.
5. En **Elección**, seleccione una elección. Déjelo vacío para todas las elecciones.
6. Seleccione los canales de votación. **En línea** y **Quiosco** están seleccionados por defecto.
7. Escriba la fecha y la hora en **Fecha y hora de inicio** (para un evento de fin, **Fecha y hora de finalización**). El portal de administración usa la zona horaria de su ordenador. La etiqueta del campo muestra la zona horaria.
8. Haga clic en el botón de guardar.
9. Haga de nuevo los pasos 3 a 8 con el tipo **Fin del Período de Votación**.

**Resultado esperado:** la pestaña **Eventos Programados** muestra los dos eventos.

Una elección tiene un evento programado de cada tipo. Si crea el mismo tipo de nuevo para la misma elección, el portal de administración cambia la fecha del evento existente. Si un evento programado no puede ejecutarse, por ejemplo porque falta el informe de inicialización, se ejecutará de nuevo en la siguiente comprobación. Consulte [Eventos programados](../01-tutorials/13-admin_portal_tutorials_scheduled-events.md).

:::caution PRECAUCIÓN
Compruebe la zona horaria en la etiqueta del campo de fecha. Si su ordenador no está en la zona horaria de la elección, convierta las horas.
:::

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| "Election event is locked down" | Configure **Estado de Confinamiento** en **No Confinado** en la pestaña **Datos**. Consulte [Crear el proceso electoral](02-event.md). |
| "Ballot publication not generated yet, can't publish." | Genere una publicación primero. Consulte el [paso 1](#1-make-a-publication). |
| "Error change ballot publication status" | El cambio de estado no está permitido ahora. Revise el estado actual del canal y vuelva a intentarlo. Si le pidieron su contraseña, confírmela primero. |
| **Comenzar Votación en Línea** no está disponible. | Haga el informe de inicialización, si la elección lo necesita. Si el canal está cerrado, contacte con el soporte de Sequent. |
| Un canal no está en el menú, o no está disponible. | Permita el canal en **Canales de Voto Permitidos** de la pestaña **Datos**. |
| "Ballot style generation failed: ..." | El contenido de la papeleta no es válido. Lea el mensaje, corrija el proceso electoral y genere una nueva publicación. |
| **Detener Votación en Línea** no está disponible. | Un evento programado todavía no permite el fin del período de votación. Mire la pestaña **Eventos Programados**. |
| Los votantes no ven una pregunta. | Asegúrese de que la pregunta está en el área del votante. Después, genere y publique una nueva publicación. |

## Más información {#more-information}

- [Publicar la papeleta (con capturas de pantalla)](../01-tutorials/12-admin_portal_tutorials_publish-election.md)
- [Informe de inicialización](../01-tutorials/11-admin_portal_tutorials_initialization-report.md)
- [Iniciar, pausar y detener la votación](../01-tutorials/14-admin_portal_tutorials_start-pause-stop-election_copy.md)
- [Eventos programados](../01-tutorials/13-admin_portal_tutorials_scheduled-events.md)
- [Proceso electoral: Publicar](../02-reference/02-election-event/09-election_management_election-event_publish.md)
- [Elección: Publicar](../02-reference/03-election/05-election_management_election_publish.md)
- [Proceso electoral: Eventos Programados](../02-reference/02-election-event/12-election_management_election-event_scheduled-events.md)

**Siguiente:** [Realizar la ceremonia de recuento](06-tally.md).
