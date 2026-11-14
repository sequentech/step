---
id: election_management_contest_tally_sheets
title: Hojas de recuento
description: "La pestaña Tally Sheets de una pregunta contiene los resultados de los votos que no se emitieron en línea, por ejemplo votos en papel o por correo."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Tally Sheets** (hojas de recuento) de una pregunta contiene los resultados de los
votos que no se emitieron en línea, por ejemplo votos en papel o por correo. Cada hoja de recuento
da los resultados de una pregunta en un área para un canal. El recuento suma las hojas de
recuento publicadas a los resultados en línea.

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, abra el proceso electoral y la elección.
2. Haga clic en la pregunta.
3. Haga clic en la pestaña **Tally Sheets**.

Las acciones necesitan estos permisos:

| Acción | Permiso |
| --- | --- |
| Ver la lista y ver una hoja | **Ver Acta de Recuento** |
| Crear y editar una hoja | **Crear Acta de Recuento** |
| Publicar y despublicar una hoja | **Publicar Acta de Recuento** |
| Borrar una hoja | **Borrar Acta de Recuento** |

## Lista {#list}

Cuando la lista está vacía, la pestaña muestra "No hay Hojas de Recuento." y **Generar Hoja de
Recuento**. Cuando hay hojas, la barra de herramientas tiene **Añadir**.

| Columna | Contenido |
| --- | --- |
| **Id** | El identificador de la hoja. |
| **Channel** (canal) | `PAPER` o `POSTAL`. |
| **Cotienda** | La pregunta. |
| **Area** | El área. |
| **Publicado** | Si la hoja está publicada. |
| **Acciones** | Iconos de editar, ver, **Publicar** o **Despublicar**, y borrar. |

## Crear una hoja de recuento {#create-a-tally-sheet}

El asistente tiene los pasos **Editar** y **Confirmar**.

1. Haga clic en **Añadir**. Si la lista está vacía, haga clic en **Generar Hoja de Recuento**.
2. En **Search Area** (buscar área), seleccione el área.
3. En **Channel**, seleccione `PAPER` o `POSTAL`.
4. Escriba los números: **Votos Totales**, **Votos Válidos Totales**, **Votos Inválidos Totales**,
   **Votos Implícitamente Inválidos**, **Votos Explícitamente Inválidos**, **Votos en Blanco
   Totales** y **Censo**.
5. En **Candidatos**, escriba los votos de cada candidato.
6. Haga clic en **Siguiente**.
7. Compruebe los números en la página de confirmación.
8. Haga clic en **Guardar**.

**Resultado esperado:** se muestra el mensaje "Hoja de Recuento creada".

9. En la fila de la hoja, haga clic en el icono de publicar. Lea "¿Estas seguro de publicar esta
   Hoja de Recuento?" y confirme.

**Resultado esperado:** se muestra el mensaje "Hoja de Recuento publicada".

:::note
Solo las hojas de recuento publicadas entran en el recuento. Publique cada hoja antes de iniciar
el recuento.
:::

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| "Todos los campos son obligatorios" | Escriba un valor en cada campo y haga clic en **Siguiente**. |
| "Error creando Hoja de Recuento" | Compruebe los números y guarde de nuevo. |
| "Error publicando la Hoja de Recuento" | Inténtelo de nuevo. |
| La etiqueta de un campo muestra un código de texto como `tallysheet.label.contest_id`. | La etiqueta no está traducida en la versión 9.0. El campo muestra la pregunta. |

## Páginas relacionadas {#related-pages}

- [Datos](../Data/election_management_contest_data.md)
- [Proceso electoral: Recuento](../../Election-Event/Tally/election_management_election-event_tally.md)
- [Realizar la ceremonia de recuento](../../../../../procedures/06-tally.md)
