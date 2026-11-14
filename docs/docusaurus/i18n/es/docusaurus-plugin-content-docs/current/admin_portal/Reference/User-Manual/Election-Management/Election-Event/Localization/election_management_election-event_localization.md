---
id: election_management_election_event_localization
title: Localización
description: "La pestaña Localización cambia los textos de los portales para un proceso electoral, en cada idioma. Cada cambio es un par: la Clave de un texto y su nuevo Valor."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Localización** cambia los textos de los portales para un proceso electoral, en cada
idioma. Cada cambio es un par: la **Clave** de un texto y su nuevo **Valor**. Para cambiar un texto
en todos los procesos electorales del inquilino, use en su lugar **Configuración** >
**LOCALIZACIÓN**.

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, haga clic en el proceso electoral.
2. Haga clic en la pestaña **Localización**.

La pestaña se muestra con el permiso **Ver Datos del Evento Electoral**. Está oculta cuando el
proceso electoral está confinado. Las acciones necesitan estos permisos:

| Acción | Permiso |
| --- | --- |
| Seleccionar el idioma | **Selector de Localización del Evento Electoral** |
| Añadir un texto | **Crear Localización** |
| Cambiar un texto | **Editar Localización** |
| Borrar un texto | **Eliminar Localización** |

## Elementos de la pantalla {#screen-elements}

| Elemento | Uso |
| --- | --- |
| **Seleccionar idioma** | El idioma de los textos de la lista. Solo se muestran los idiomas del proceso electoral. |
| **Añadir** | Abre el formulario para añadir un texto. |
| Columna **Clave** | La clave del texto. |
| Columna **Valor** | El texto nuevo. |
| Icono de editar | Cambia el texto de la fila. |
| Icono de borrar | Borra la fila, después de una confirmación. |

## Añadir un texto {#add-a-text}

1. En **Seleccionar idioma**, seleccione el idioma.
2. Haga clic en **Añadir**.
3. Escriba la **Clave**.
4. Escriba el **Valor**.
5. Haga clic en **Guardar**.

**Resultado esperado:** se muestra el mensaje "Localización actualizada con éxito". La fila se
muestra en la lista.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| "No se han establecido idiomas para el evento" | Seleccione los idiomas en la sección **Idiomas** de la pestaña **Datos**. |
| "La actualización de la localización ha fallado" | Compruebe la clave y el valor, e inténtelo de nuevo. |
| El texto nuevo no se muestra en el portal. | Asegúrese de que la clave es correcta y de que seleccionó el idioma correcto. |

## Páginas relacionadas {#related-pages}

- [Datos](../Data/election_management_election-event_data.md)
- [Configuración: Localización](../../../Settings/Localization/admin_portal_reference_user-manual_settings_localization.md)
