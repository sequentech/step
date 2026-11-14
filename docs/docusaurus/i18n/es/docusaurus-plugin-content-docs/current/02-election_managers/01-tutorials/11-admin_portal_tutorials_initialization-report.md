---
id: admin_portal_tutorials_initialization-report
title: Informe de inicialización
description: "La vía principal está en Publicar y gestionar el período de votación. Esta página da los detalles: cuándo es necesario el informe, qué lo bloquea y cómo desbloquea el período de votación."
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La vía principal está en [Publicar y gestionar el período de votación](../03-procedures/05-publish.md#4-make-the-initialization-report-if-necessary).
Esta página da los detalles: cuándo es necesario el informe, qué lo bloquea y cómo desbloquea el
período de votación.

## Qué es el informe de inicialización {#what-the-initialization-report-is}

El informe de inicialización es un recuento de la urna antes de que se abra el período de votación. Utiliza
la misma ceremonia de recuento que el recuento de los resultados. La urna está vacía, así que el
informe muestra cero votos. El informe es la prueba de que la urna no contenía votos al inicio.

En la lista **Recuento** del proceso electoral, el informe aparece con el **Tipo de Conteo**
**Resultados de Inicialización**. El recuento de los votos aparece con el **Tipo de Conteo**
**Resultados Electorales**.

## Cuándo es necesario el informe {#when-the-report-is-necessary}

Cada elección tiene el campo **Política de informe consolidado** en su pestaña **Datos**.

| Valor | Efecto |
| --- | --- |
| **No Requerido** (predeterminado) | El informe es opcional. Puede abrir el período de votación sin él. |
| **Requerido** | La elección no puede abrirse hasta que se complete un informe de inicialización de la elección. |

Cuando la política es **Requerido**:

- En la pestaña **Publicar** de la elección, **Comenzar Votación en Línea** no está disponible hasta que el informe esté completo.
- La plataforma rechaza abrir cualquier canal de la elección desde la pestaña **Publicar** de la elección, o con un evento programado para la elección.

:::caution PRECAUCIÓN
Abra el período de votación de una elección con esta política desde la pestaña **Publicar** de la elección.
La pestaña **Publicar** del proceso electoral no comprueba el informe de inicialización.
:::

## Permitir el informe a una hora determinada {#permit-the-report-at-a-set-time}

El evento programado **Allow Initialization Report** establece el estado de una elección para que se permitan los informes de inicialización. De forma predeterminada, el estado de una elección los permite. Si el estado de una elección no los permite, este evento los permite a la hora establecida. Consulte [Eventos programados](13-admin_portal_tutorials_scheduled-events.md).

## Antes de empezar {#before-you-start}

- El estado de la ceremonia de claves es `SUCCESS`. Consulte [Realizar la ceremonia de claves](../03-procedures/04-keys.md).
- El proceso electoral tiene una publicación, y cada elección del informe está publicada. Consulte [Publicar y gestionar el período de votación](../03-procedures/05-publish.md).
- Su cuenta puede crear ceremonias de recuento (permiso **Crear Ceremonia**).
- En una ceremonia de claves manual, los fideicomisarios están disponibles con su fragmento de clave.

Si no hay una ceremonia de claves completada o no hay publicación, **Generar Informe de Inicialización** no está disponible.

## Qué elecciones puede seleccionar {#which-elections-you-can-select}

En la página **Elecciones para el informe de inicialización**, **Iniciar informe de inicialización** no está disponible si una de las elecciones seleccionadas:

- No está publicada.
- Ya tiene un informe de inicialización que no está cancelado.
- Ya tiene un informe de inicialización completo.
- Tiene la política **Requerido** y un estado que no permite los informes de inicialización.

La página muestra entonces el mensaje «No puedes continuar con la ceremonia porque no se han seleccionado elecciones o las elecciones no están publicadas.». Este mensaje aparece por todas estas razones, no solo por las dos que nombra.

## Generar el informe {#make-the-report}

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Recuento**.
3. Haga clic en **Generar Informe de Inicialización**.
4. Seleccione las elecciones.
5. En **Ceremonia de Llaves**, seleccione la ceremonia de claves de estas elecciones.
6. Haga clic en **Iniciar informe de inicialización**.
7. Confirme la acción.
8. Continúe como en una ceremonia de recuento: los fideicomisarios suben su fragmento de clave y, a continuación, usted hace clic en **Iniciar Recuento**. Consulte [Realizar la ceremonia de recuento](../03-procedures/06-tally.md#2-upload-the-key-fragment-each-trustee).

En una ceremonia de claves automática, los fideicomisarios no hacen nada y el informe se ejecuta de inmediato. Para conocer los estados de la ceremonia, consulte [Ceremonia de recuento](16-admin_portal_tutorials_tally-ceremony.md).

**Resultado esperado:** el estado del informe cambia a `SUCCESS`. La plataforma registra entonces que cada elección del informe tiene su informe de inicialización. **Comenzar Votación en Línea** pasa a estar disponible en la pestaña **Publicar** de estas elecciones.

## Ver y descargar el informe {#see-and-download-the-report}

1. En la pestaña **Recuento**, haga clic en el icono de visualización del informe.
2. Haga clic en **Resultados**.
3. Abra **Resultados & Participación**.
4. Seleccione la elección, la pregunta y el área.

El menú de acciones de cada elemento muestra los formatos disponibles: PDF, HTML, JSON y TAR_GZ. En el caso de HTML, el menú también genera un PDF. En el caso de un informe de inicialización, el menú no ofrece la exportación de todas las áreas de una elección en un único archivo. Para conocer los formatos, consulte [Resultados de la elección](17-admin_portal_tutorials_election-results.md#download-the-result-documents).

Guarde el informe descargado junto con los registros de la elección.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| «La Ceremonia de Recuento no puede comenzar hasta que la Ceremonia de Claves se haya completado con éxito.» | Complete la ceremonia de claves. Consulte [Realizar la ceremonia de claves](../03-procedures/04-keys.md). |
| «La Ceremonia de Escrutinio no puede comenzar hasta que crees una publicación en la pestaña Publicar.» | Publique la papeleta. Consulte [Publicar y gestionar el período de votación](../03-procedures/05-publish.md). |
| «No puedes continuar con la ceremonia porque no se han seleccionado elecciones o las elecciones no están publicadas.» | Seleccione una o más elecciones. Asegúrese de que cada elección esté publicada y de que no tenga otro informe de inicialización. |
| «Elección ...: publique la elección antes de crear su recuento.» | Publique la elección y, a continuación, vuelva a empezar. |
| «Elección ...: los informes de inicialización no están permitidos.» | El estado de la elección no permite el informe. Espere al evento programado **Allow Initialization Report** o póngase en contacto con el soporte de Sequent. |
| **Comenzar Votación en Línea** no está disponible después del informe. | Asegúrese de que el estado del informe sea `SUCCESS` y de que el informe incluya esta elección. |
| La elección ya tiene un informe que no desea. | Cancélelo en la pestaña **Recuento** si su estado es `STARTED` o `CONNECTED`. A continuación, genere un informe nuevo. |
