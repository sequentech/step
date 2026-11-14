---
id: admin_portal_reference_user_manual_users_and_roles_roles
title: Roles
description: "La pestaña Roles contiene los roles del inquilino. Un rol es un conjunto de permisos. Cada usuario recibe uno o más roles en la pestaña Usuarios. Un cambio en un rol se aplica de inmediato a todos sus usuarios."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Roles** contiene los roles del inquilino. Un rol es un conjunto de permisos. Cada
usuario recibe uno o más roles en la pestaña **Usuarios**. Un cambio en un rol se aplica de
inmediato a todos sus usuarios.

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, haga clic en **Usuarios y Roles**.
2. Haga clic en la pestaña **Roles**.

La pestaña necesita el permiso **Leer Rol**. Para crear un rol, su rol necesita **Crear Rol**.

## Roles predeterminados {#default-roles}

Estos roles están en un inquilino nuevo. Su organización puede cambiarlos.

| Rol | Uso |
| --- | --- |
| `admin` | Administradores electorales y del inquilino. Tiene casi todos los permisos, pero no **Ceremonia de Fideicomisario**. |
| `admin-light` | Administradores con menos permisos. La mayoría de las pestañas del proceso electoral están ocultas. |
| `admin-lockdown` | Administradores de un proceso electoral confinado. Solo los permisos de **Publicar**. |
| `trustee` | Fideicomisarios. Tiene **Ceremonia de Fideicomisario** y los permisos para ver las pestañas **Claves** y **Recuento**. |

## Lista {#list}

| Columna | Contenido |
| --- | --- |
| **Name** (nombre) | El nombre del rol. |
| **Id** | El identificador del rol. |
| Acciones | Iconos de editar y de borrar. |

## Crear un rol {#create-a-role}

1. Haga clic en **Añadir**.
2. Escriba el **Name**.
3. En la lista, seleccione **Active** (activo) para cada **Permission** (permiso) del rol.
4. Haga clic en **Guardar**.

**Resultado esperado:** se muestra el mensaje "Rol creado".

## Cambiar un rol {#change-a-role}

1. Haga clic en el icono de editar del rol.
2. Seleccione o desmarque **Active** para un permiso.

**Resultado esperado:** se muestra el mensaje "Permiso editado". No puede cambiar el **Name** de
un rol.

:::caution PRECAUCIÓN
No quite permisos a su propio rol. Si lo hace, puede perder el acceso a una parte del portal de
administración.
:::

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| "Error creando rol" | Compruebe el nombre y vuelva a guardar. |
| "Error editando permiso" | Vuelva a intentarlo. |
| "Error borrando rol" | No se pudo borrar el rol. |

## Páginas relacionadas {#related-pages}

- [Permisos](../Permissions/admin_portal_reference_user-manual_users-and-roles_permissions.md)
- [Usuarios](../Users/admin_portal_reference_user-manual_users-and-roles_users.md)
- [Configurar el inquilino](../../../../procedures/01-tenant.md#2-check-the-roles)
