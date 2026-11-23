---
id: admin_portal_tutorials_permission-labels
title: Etiquetas de permiso
description: "Una etiqueta de permiso limita las elecciones que un administrador puede ver y usar. Use etiquetas de permiso cuando equipos diferentes gestionan elecciones diferentes del mismo proceso electoral."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Una etiqueta de permiso limita las elecciones que un administrador puede ver y usar. Use
etiquetas de permiso cuando equipos diferentes gestionan elecciones diferentes del mismo proceso
electoral. Por ejemplo, el equipo de "Área Norte" ve solo las elecciones con la etiqueta `north`.

Las etiquetas de permiso no son permisos. Los permisos dicen qué puede hacer un usuario. Las
etiquetas de permiso dicen en qué elecciones puede hacerlo.

## Cómo funcionan las etiquetas de permiso {#how-permission-labels-work}

- Una elección tiene una etiqueta de permiso o ninguna. Está en **Etiqueta de permiso**, en la
  pestaña **Datos** de la elección.
- Un usuario tiene cero, una o más etiquetas de permiso. Están en **Etiqueta de permiso**, en el
  formulario de usuario de la pestaña **Usuarios**.
- Un usuario ve una elección cuando la elección no tiene etiqueta, o cuando el usuario tiene la
  etiqueta de la elección. Un usuario sin etiquetas ve solo las elecciones sin etiqueta.
- Las etiquetas también limitan las ceremonias de claves, los recuentos, los informes y las
  solicitudes de inscripción. Una ceremonia de claves o un recuento recibe las etiquetas de sus
  elecciones.

| Pantalla | Efecto de las etiquetas |
| --- | --- |
| Pestañas de la elección | Un usuario sin la etiqueta de la elección ve "No tienes permiso para acceder a esta elección.". |
| Pestaña **Claves** | La columna **Etiquetas de Permisos** muestra las etiquetas de la ceremonia. Para crear una ceremonia, el usuario necesita las etiquetas de sus elecciones. Sin ellas: "No se puede crear la ceremonia de claves: falta una o más etiquetas de permisos." |
| Pestaña **Recuento** | La columna **Etiquetas de Permisos** muestra las etiquetas del recuento. El recuento acepta solo elecciones con las etiquetas del usuario. |
| Pestaña **Informe** | Un informe tiene su propia **Etiqueta de permiso**. |
| Pestaña **Aprobaciones** de una elección | Muestra solo las solicitudes con la etiqueta de la elección. |

## Antes de empezar {#before-you-start}

- Su rol tiene el permiso **Editar la etiqueta de permiso**. Sin él, los campos **Etiqueta de
  permiso** no se muestran.
- El perfil de usuario del inquilino tiene el atributo de etiqueta de permiso. Si **Etiqueta de
  permiso** no está en el formulario de usuario, pregunte al soporte de Sequent.

## Dar una etiqueta a una elección {#give-a-label-to-an-election}

1. Abra la elección.
2. Haga clic en la pestaña **Datos**.
3. Abra **Configuración Avanzada**.
4. Escriba la etiqueta en **Etiqueta de permiso**, por ejemplo `north`.
5. Haga clic en **Guardar**.

**Resultado esperado:** solo los usuarios con la etiqueta `north` ven la elección. Un usuario
sin etiquetas no la ve.

## Dar etiquetas a un usuario {#give-labels-to-a-user}

1. En el menú de la izquierda, haga clic en **Usuarios y Roles**.
2. En la pestaña **Usuarios**, haga clic en **Editar** en el menú de acciones del usuario.
3. En **Etiqueta de permiso**, escriba una etiqueta y pulse Intro. Haga esto para cada etiqueta
   del usuario.
4. Haga clic en **Guardar**.
5. Pida al usuario que cierre sesión y vuelva a iniciar sesión. Las etiquetas se aplican en el
   siguiente inicio de sesión.

**Resultado esperado:** el usuario ve las elecciones con estas etiquetas y las elecciones sin
etiqueta.

:::caution PRECAUCIÓN
Escriba las etiquetas exactamente igual en la elección y en el usuario. Una diferencia en una
letra oculta la elección al usuario.
:::

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| "No tienes permiso para acceder a esta elección." | Dé al usuario la etiqueta de la elección, o quite la etiqueta de la elección. |
| "No se puede crear la ceremonia de claves: falta una o más etiquetas de permisos." | Dé a su cuenta las etiquetas de las elecciones de la ceremonia. |
| "Some elections don't have the required permission label or are not published" (algunas elecciones no tienen la etiqueta de permiso necesaria o no están publicadas) | Seleccione para el recuento solo elecciones publicadas con sus etiquetas. |
| **Etiqueta de permiso** no está en el formulario. | Su rol no tiene **Editar la etiqueta de permiso**, o el perfil de usuario no tiene el atributo de etiqueta. |

## Páginas relacionadas {#related-pages}

- [Elección: Datos](../Reference/User-Manual/Election-Management/Election/Data/election_management_election_data.md)
- [Usuarios](../Reference/User-Manual/Users-and-Roles/Users/admin_portal_reference_user-manual_users-and-roles_users.md)
- [Permisos](../Reference/User-Manual/Users-and-Roles/Permissions/admin_portal_reference_user-manual_users-and-roles_permissions.md)
- [Realizar la ceremonia de claves](../procedures/04-keys.md)
