---
id: election_management_election_event_areas
title: Áreas
description: "La pestaña Áreas contiene las áreas del proceso electoral. Un área es un grupo de votantes. Las preguntas de un área son las preguntas que sus votantes ven en la papeleta."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Áreas** contiene las áreas del proceso electoral. Un área es un grupo de votantes.
Las preguntas de un área son las preguntas que sus votantes ven en la papeleta.

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, haga clic en el proceso electoral.
2. Haga clic en la pestaña **Áreas**.

La pestaña se muestra con el permiso **Ver Áreas del Evento Electoral**. Está oculta cuando el
proceso electoral está confinado.

## Barra de herramientas {#toolbar}

| Botón | Uso | Permiso |
| --- | --- | --- |
| **Columnas** | Selecciona las columnas de la lista. | **Columnas de las Áreas del Evento Electoral** |
| **Añadir filtro** | Filtra la lista por **Name** (nombre), **Description** (descripción), **ID** o **Type** (tipo). | **Filtros de las Áreas del Evento Electoral** |
| **Añadir** | Abre el formulario para crear un área. | **Crear Área** |
| **Importar** | Abre el panel **Importar Áreas**. | **Importar Área** |
| **Upsert Areas** (insertar o actualizar áreas) | Abre un panel para asignar el área superior de áreas existentes desde un archivo. | **Insertar o Actualizar Área** |

Cuando la lista está vacía, la pestaña muestra "No hay Áreas aún." con los botones **Crear un
Área** e **Importar**.

## Columnas {#columns}

| Columna | Contenido |
| --- | --- |
| **Id** | El identificador del área. |
| **Name** | El nombre del área. Los archivos de votantes se refieren al área por este nombre. |
| **Description** | La descripción del área. |
| **Preguntas del Área** | Las preguntas del área. |
| **Actions** (acciones) | Iconos de editar y borrar. |

## Formulario del área {#area-form}

| Campo | Uso |
| --- | --- |
| **Name** | El nombre del área. |
| **Description** | La descripción del área. |
| **Preguntas del Área** | Las preguntas del área. Escriba tres o más letras del nombre de una pregunta y selecciónela. |
| **Parent** (área superior) | El área más grande que contiene esta área, si la hay. Escriba tres o más letras para buscar. |
| **Guardar** | Guarda el área. |

## Importar áreas {#import-areas}

El archivo de importación es un archivo CSV sin fila de encabezado. Cada fila tiene cinco
columnas:

| Posición | Contenido |
| --- | --- |
| 1 | El nombre del área. Se convierte en el **Name**. |
| 2 | Un código de país. La importación no lo usa. |
| 3 | Un código. La importación no lo usa. |
| 4 | La descripción. Se convierte en la **Description**. |
| 5 | La marca de borrado. Solo se importan las filas con `0`. |

1. Haga clic en **Importar**.
2. Escriba el hash SHA-256 del archivo en **Verificación de Integridad (SHA 256)**.
3. Suba el archivo.
4. Haga clic en **Importar**.

**Resultado esperado:** se muestra el mensaje "Áreas Importadas Exitosamente".

:::caution PRECAUCIÓN
La importación vincula cada área nueva a todas las preguntas del proceso electoral. Después de la
importación, abra cada área y quite las preguntas que no son de esa área.
:::

## Insertar o actualizar áreas {#upsert-areas}

**Upsert Areas** lee un archivo CSV con una fila de encabezado y dos columnas: el nombre de un área
existente y el nombre de su área superior. Solo cambia el área superior de las áreas.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| "Error creando área" | Compruebe los campos y guarde de nuevo. |
| "Error al importar Áreas" | Compruebe que el archivo tiene cinco columnas y no tiene fila de encabezado. Después importe de nuevo. |
| "Error borrando Área" | No se pudo borrar el área. Contacte con el soporte de Sequent si el error continúa. |
| Una pregunta no está en la lista **Preguntas del Área**. | Escriba tres o más letras del nombre de la pregunta. |

## Páginas relacionadas {#related-pages}

- [Crear el proceso electoral](../../../../../procedures/02-event.md#6-create-the-areas)
- [Votantes](../Voters/election_management_election-event_voters.md)
- [Publicar](../Publish/election_management_election-event_publish.md)
