---
id: admin_portal_reference_user_manual_users_and_roles_users
title: Usuarios
description: "La pestaña Usuarios contiene las cuentas de los administradores y de los fideicomisarios del inquilino. Cada usuario tiene uno o más roles. Una cuenta de fideicomisario también está vinculada a un fideicomisario de Configuración > AUTORIDADES."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Usuarios** contiene las cuentas de los administradores y de los fideicomisarios del
inquilino. Cada usuario tiene uno o más roles. Una cuenta de fideicomisario también está
vinculada a un fideicomisario de **Configuración** > **AUTORIDADES**.

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, haga clic en **Usuarios y Roles**.
2. Haga clic en la pestaña **Usuarios**.

La pestaña necesita el permiso **Leer Usuario**.

## Barra de herramientas {#toolbar}

| Botón | Uso | Permiso |
| --- | --- | --- |
| **Columnas** | Selecciona las columnas. | **Ver Columnas de Votantes del Evento Electoral** |
| **Añadir filtro** | Filtra la lista. | **Ver Filtros de Votantes del Evento Electoral** |
| **Añadir** | Abre el formulario para crear un usuario. | **Crear Votante** |
| **Importar** | Importa usuarios desde un archivo. | **Importar Usuarios** |
| **Exportar** | Exporta la lista de usuarios. | **Exportar Votante** |
| **Enviar** | Envía un mensaje a los usuarios. | **Enviar Notificación** |

En la versión 9.0 algunos botones de esta pestaña usan permisos de votante, como muestra la
tabla. Cuando la lista está vacía, la pestaña muestra **Crear usuario**.

## Acciones de un usuario {#actions-of-a-user}

| Acción | Uso |
| --- | --- |
| **Enviar** | Envía un mensaje a este usuario. |
| **Editar** | Cambia los datos y los roles del usuario. |
| **Borrar** | Borra el usuario, después de la pregunta "¿Estás seguro que quieres borrar este usuario?". No puede borrar su propia cuenta. |
| **Cambiar contraseña** | Establece una contraseña nueva. |

## Formulario de usuario {#user-form}

| Campo | Uso |
| --- | --- |
| Campos del perfil | Por ejemplo **Nombre de Usuario**, **Email**, **Nombre** y **Apellido**. **Nombre de Usuario** siempre es obligatorio. |
| **Actuar como fideicomisario** | Solo para un fideicomisario: el fideicomisario de **Configuración** > **AUTORIDADES** que representa esta persona. |
| **Etiqueta de permiso** | Las etiquetas de permiso del usuario. Necesita el permiso **Editar la etiqueta de permiso**. Consulte [Etiquetas de permiso](../../../../Tutorials/admin_portal_tutorials_permission-labels.md). |
| **Habilitado \*** | Si el usuario puede iniciar sesión. |
| **Contraseña:** y **Repetir Contraseña:** | La contraseña. |
| **Temporal** | Si está activado, el usuario debe cambiar la contraseña en el siguiente inicio de sesión. Está activado de forma predeterminada. |
| **Role** (rol) y **Active** (activo) | Los roles del usuario. Seleccione **Active** para cada rol. Para un usuario existente, el cambio se guarda inmediatamente. |
| **Guardar** | Guarda el usuario. |

Los campos vienen del perfil de usuario del inquilino. **Actuar como fideicomisario** y
**Etiqueta de permiso** solo se muestran si el perfil de usuario tiene estos atributos.

:::danger ADVERTENCIA
Dé a cada fideicomisario su propia cuenta y su propio fideicomisario en **Actuar como
fideicomisario**. Dos personas no deben compartir un fideicomisario.
:::

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| "Las contraseñas deben de coincidir" | Escriba la misma contraseña en los dos campos. |
| "Error creando votante" cuando crea un usuario | La versión 9.0 usa los mensajes de votante para los usuarios. Compruebe los campos obligatorios y el nombre de usuario. |
| Un cambio de rol muestra un código de texto como `usersAndRolesScreen.roles.notifications.permissionEditSuccess`. | El mensaje no está traducido en la versión 9.0. El código que termina en `Success` significa que el cambio está guardado. |
| **Actuar como fideicomisario** no está en el formulario. | El perfil de usuario no tiene el atributo de fideicomisario. Pregunte al soporte de Sequent. |
| El botón **Añadir** no está. | Su rol no tiene el permiso **Crear Votante**. |

## Páginas relacionadas {#related-pages}

- [Configurar el inquilino](../../../../procedures/01-tenant.md#3-create-the-user-accounts)
- [Roles](../Roles/admin_portal_reference_user-manual_users-and-roles_roles.md)
- [Fideicomisarios](../../Settings/Trustees/admin_portal_reference_user-manual_settings_trustees.md)
