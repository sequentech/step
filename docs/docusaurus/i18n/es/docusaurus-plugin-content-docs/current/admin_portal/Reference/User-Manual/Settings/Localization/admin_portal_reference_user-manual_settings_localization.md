---
id: admin_portal_reference_user_manual_settings_localization
title: Localización
description: "La pestaña LOCALIZACIÓN de Configuración cambia los textos de los portales para todo el inquilino, en cada idioma. Cada cambio es un par: la Clave de un texto y su nuevo Valor."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **LOCALIZACIÓN** de **Configuración** cambia los textos de los portales para todo el
inquilino, en cada idioma. Cada cambio es un par: la **Clave** de un texto y su nuevo **Valor**.
Para cambiar un texto solo para un proceso electoral, use la pestaña **Localización** del
proceso electoral.

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, haga clic en **Configuración**.
2. Haga clic en la pestaña **LOCALIZACIÓN**.

## Añadir un texto {#add-a-text}

1. En **Seleccionar idioma**, seleccione el idioma.
2. Haga clic en **Añadir**.
3. Escriba la **Clave**.
4. Escriba el **Valor**.
5. Haga clic en **Guardar**.

**Resultado esperado:** se muestra el mensaje "Localización actualizada con éxito". La fila se
muestra en la lista. Use los iconos de editar y de borrar de la fila para cambiarla o borrarla.

## Ejemplo: la etiqueta de un atributo de usuario {#example-the-label-of-a-user-attribute}

Para cambiar la etiqueta de un atributo de usuario, use una clave con el prefijo
`usersAndRolesScreen.users.fields.` seguido del nombre del atributo.

Por ejemplo, para mostrar el atributo `personal_administrative_number` como "PAN":

| **Clave** | **Valor** |
| --- | --- |
| `usersAndRolesScreen.users.fields.personal_administrative_number` | `PAN` |

Esto funciona con cualquier nombre de atributo, también con los atributos que no tienen una
etiqueta estándar.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| "La actualización de la localización ha fallado" | Compruebe la clave y el valor, y vuelva a intentarlo. |
| El nuevo texto no se muestra. | Asegúrese de que la clave es correcta y de que seleccionó el idioma correcto. |

## Páginas relacionadas {#related-pages}

- [Proceso electoral: Localización](../../Election-Management/Election-Event/Localization/election_management_election-event_localization.md)
- [Idiomas](../Languages/admin_portal_reference_user-manual_settings_languages.md)
- [Configuración](../admin_portal_reference_user-manual_settings.md)
