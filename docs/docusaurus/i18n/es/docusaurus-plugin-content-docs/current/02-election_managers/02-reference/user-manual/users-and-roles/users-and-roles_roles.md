---
id: users-and-roles_roles
title: Roles
description: "La pestaña Roles está en Usuarios y Roles. Úsela para ver, crear, cambiar y eliminar los roles del inquilino. La usa el administrador del inquilino."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Roles** está en **Usuarios y Roles**. Úsela para ver, crear, cambiar y eliminar los roles del inquilino. La usa el administrador del inquilino.

Un rol es un conjunto con nombre de permisos. Cada permiso permite una acción o una pantalla del portal de administración, por ejemplo **Crear Usuario** o **Ver configuración**. Un usuario obtiene los permisos de todos los roles que usted le asigna. En el servicio de inicio de sesión (Keycloak), un rol es un grupo del realm del inquilino.

Para el significado de algunos permisos, consulte [Permisos](users-and-roles_permissions.md).

## Cómo abrirlo {#how-to-open-it}

1. En el menú de la izquierda, haga clic en **Usuarios y Roles**.
2. Haga clic en la pestaña **Roles**.

## Permiso necesario {#required-permission}

- Para abrir **Usuarios y Roles**, su cuenta debe tener el permiso **Ver usuarios y roles**, y **Leer Usuario** o **Leer Rol**. Si no los tiene, la pantalla muestra "No tienes permiso para acceder a los usuarios o roles."
- Para ver la pestaña **Roles**, su cuenta debe tener el permiso **Leer Rol**.
- El botón **Añadir** solo se muestra a los usuarios con el permiso **Crear Rol**.
- Para activar o desactivar un permiso, su cuenta debe tener los permisos **Editar Permiso de Usuario** y **Editar Rol**.
- Para eliminar un rol, su cuenta debe tener el permiso **Editar Rol**.

## Roles predeterminados {#default-roles}

Sequent crea los roles de un nuevo inquilino. Normalmente, el inquilino tiene estos roles:

| Rol | Qué permite |
| --- | --- |
| `admin` | Casi todos los permisos: todos los menús, todas las pestañas del proceso electoral, los usuarios y roles, la configuración y las ceremonias. Asígnelo a los administradores del inquilino. |
| `admin-light` | Un conjunto reducido de permisos de administrador. No incluye **Usuarios y Roles**, **Configuración** ni la creación de una ceremonia de claves. |
| `admin-lockdown` | Solo la pestaña **Publicar** del proceso electoral y la publicación de resultados. |
| `trustee` | Las pestañas **Claves** y **Recuento** del proceso electoral y la parte de fideicomisario de la ceremonia de claves y de la ceremonia de recuento. Asígnelo a los usuarios fideicomisarios. |

La lista de su inquilino puede ser diferente. Para ver los permisos de un rol, haga clic en el icono de editar del rol.

## Columnas y botones {#columns-and-buttons}

| Elemento | Significado |
| --- | --- |
| **Nombre** | El nombre del rol. |
| **ID** | El identificador interno del rol. |
| Icono de editar | Abre **Información de Rol**: el nombre del rol y sus permisos. |
| Icono de eliminar | Elimina el rol, después de una confirmación. |
| **Añadir** | Abre **Crear rol**. |
| **Columnas** | Muestra u oculta columnas. |

La tabla de permisos tiene dos columnas: **Permiso** (el nombre del permiso) y **Activo** (una casilla de verificación). En **Información de Rol**, use el campo de búsqueda situado encima de la tabla para encontrar un permiso.

## Crear un rol {#create-a-role}

1. En la pestaña **Roles**, haga clic en **Añadir**.
2. En **Nombre**, escriba el nombre del rol.
3. En la tabla, seleccione **Activo** para cada permiso del rol.
4. Haga clic en **Guardar**.

**Resultado esperado:** aparece el mensaje "Rol creado". El nuevo rol aparece en la lista.

## Cambiar los permisos de un rol {#change-the-permissions-of-a-role}

:::caution PRECAUCIÓN
Un cambio en un rol se aplica de inmediato a todos los usuarios con ese rol. No quite ningún permiso del rol `admin`. Si lo hace, puede perder el acceso a una parte del portal de administración.
:::

1. En la pestaña **Roles**, haga clic en el icono de editar del rol.
2. Busque el permiso en la tabla.
3. Haga clic en la casilla **Activo** del permiso.

**Resultado esperado:** aparece el mensaje "Permiso editado". No hay botón **Guardar**: cada clic guarda el cambio.

No puede cambiar el nombre de un rol. Para cambiarle el nombre, cree un rol nuevo, traslade los usuarios a él y después elimine el rol antiguo.

## Eliminar un rol {#delete-a-role}

:::caution PRECAUCIÓN
Asegúrese de que ningún usuario necesita el rol. Cuando elimina un rol, todos sus usuarios pierden sus permisos. No puede deshacer esta acción.
:::

1. En la pestaña **Roles**, haga clic en el icono de eliminar del rol.
2. Lea el mensaje "¿Estás seguro que quieres borrar este rol?".
3. Haga clic en **Eliminar**.

**Resultado esperado:** aparece el mensaje "Rol borrado". El rol no aparece en la lista.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| Aparece el mensaje "No tienes permiso para acceder a los usuarios o roles." | Pida a un administrador del inquilino que dé a su cuenta los permisos necesarios. |
| No se muestra la pestaña **Roles** ni el botón **Añadir**. | Su cuenta no tiene el permiso **Leer Rol** o **Crear Rol**. |
| Aparece un mensaje de error al guardar un rol nuevo. | Asegúrese de que ningún otro rol tiene el mismo nombre. Después, inténtelo de nuevo. |
| La casilla **Activo** no cambia, o no aparece el mensaje "Permiso editado". | Asegúrese de que su cuenta tiene los permisos **Editar Permiso de Usuario** y **Editar Rol**. |
| El rol sigue en la lista después de eliminarlo. | Asegúrese de que su cuenta tiene el permiso **Editar Rol**. |
| Un usuario no ve un permiso nuevo. | Pida al usuario que cierre la sesión y vuelva a iniciarla. |

**Procedimiento relacionado:** [Configurar el inquilino](../../../03-procedures/01-tenant.md), paso 2.
