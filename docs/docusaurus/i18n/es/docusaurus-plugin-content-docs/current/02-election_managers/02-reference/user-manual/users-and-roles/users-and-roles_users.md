---
id: users-and-roles_users
title: Usuarios
description: "La pestaña Usuarios está en Usuarios y Roles. Muestra las cuentas de las personas que usan el portal de administración: los administradores y los fideicomisarios. Úsela para crear, cambiar, deshabilitar y eliminar estas cuentas."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Usuarios** está en **Usuarios y Roles**. Muestra las cuentas de las personas que usan el portal de administración: los administradores y los fideicomisarios. Úsela para crear, cambiar, deshabilitar y eliminar estas cuentas. La usa el administrador del inquilino.

Los votantes no aparecen en esta pestaña. Cada proceso electoral tiene su propia lista de votantes en su pestaña **Votantes**.

## Cómo abrirlo {#how-to-open-it}

1. En el menú de la izquierda, haga clic en **Usuarios y Roles**.
2. Haga clic en la pestaña **Usuarios**.

## Permiso necesario {#required-permission}

Para abrir **Usuarios y Roles**, su cuenta debe tener el permiso **Ver usuarios y roles**. Para ver la pestaña **Usuarios**, debe tener el permiso **Leer Usuario**. Cada acción necesita más permisos:

| Acción | Permisos |
| --- | --- |
| Crear un usuario | **Crear Votante** (muestra el botón **Añadir**) y **Crear Usuario** |
| Editar un usuario | **Editar Votante** (muestra **Editar**) y **Editar Usuario** |
| Eliminar un usuario | **Eliminar Votante** (muestra **Eliminar**) y **Editar Usuario** |
| Cambiar una contraseña | **Cambiar la Contraseña del Votante** (muestra **Cambiar contraseña**) y **Editar Usuario** |
| Importar usuarios | **Importar Usuarios** (muestra **Importar**) y **Crear Usuario** |
| Exportar usuarios | **Exportar Votante** (muestra **Exportar**) y **Leer Usuario** |

El rol `admin` tiene todos estos permisos.

## Columnas y botones {#columns-and-buttons}

| Elemento | Significado |
| --- | --- |
| **Habilitado** | Una marca de verificación indica que el usuario puede iniciar sesión. |
| **Nombre de Usuario**, **Nombre**, **Apellido** | Los datos del usuario. Las columnas provienen del perfil de usuario del inquilino. |
| Otras columnas | **ID**, **Email**, **Email Verificado** y otros datos del usuario están ocultos por defecto. Para mostrarlos, haga clic en **Columnas**. |
| **Acciones** | El menú del usuario: **Enviar**, **Editar**, **Eliminar** y **Cambiar contraseña**. |
| **Añadir** | Abre el formulario para crear un usuario. Si la lista está vacía, haga clic en **Crear usuario**. |
| **Importar** | Crea usuarios a partir de un archivo CSV. |
| **Exportar** | Descarga la lista de usuarios como archivo CSV. |
| **Enviar** | Envía una notificación a los usuarios. |

## El formulario de usuario {#the-user-form}

El formulario muestra los campos del perfil de usuario del inquilino. Normalmente son **Nombre de Usuario**, **Email**, **Nombre** y **Apellido**. Un campo con `*` es obligatorio.

| Campo | Significado | Valor predeterminado |
| --- | --- | --- |
| **Actuar como fideicomisario** | El fideicomisario que controla esta persona. La lista muestra los nombres de la pestaña **AUTORIDADES** de **Configuración**. Solo para usuarios fideicomisarios. | Vacío |
| **Habilitado \*** | Si está seleccionado, el usuario puede iniciar sesión. Si no está seleccionado, la cuenta está deshabilitada. | Seleccionado |
| **Contraseña:** | La primera contraseña del usuario. Solo en un usuario nuevo. | Vacío |
| **Repetir Contraseña:** | La misma contraseña otra vez. Si las dos no coinciden, el formulario muestra "Las contraseñas deben de coincidir". Solo en un usuario nuevo. | Vacío |
| **Temporal** | Si está activado, el usuario debe cambiar la contraseña en el siguiente inicio de sesión. Solo en un usuario nuevo. | Activado |
| **Rol** / **Activo** | La tabla de roles. Seleccione **Activo** para cada rol del usuario. Consulte [Roles](users-and-roles_roles.md). | Ningún rol |

**Actuar como fideicomisario** solo se muestra cuando el perfil de usuario del inquilino tiene el campo de fideicomisario.

## Crear un administrador {#create-an-administrator}

1. En la pestaña **Usuarios**, haga clic en **Añadir**.
2. Escriba el **Nombre de Usuario**, el **Email**, el **Nombre** y el **Apellido**.
3. Asegúrese de que **Habilitado \*** está seleccionado.
4. En **Contraseña:**, escriba una contraseña.
5. En **Repetir Contraseña:**, escriba la misma contraseña.
6. Asegúrese de que **Temporal** está activado.
7. En la tabla de roles, seleccione **Activo** para el rol `admin`.
8. Haga clic en **Guardar**.
9. Entregue a la persona el nombre de usuario y la contraseña. Use un canal seguro.

**Resultado esperado:** aparece el mensaje "Votante creado". El usuario aparece en la lista.

## Crear un usuario fideicomisario {#create-a-trustee-user}

:::danger ADVERTENCIA
Asigne a cada fideicomisario su propia cuenta y su propio fideicomisario en **Actuar como fideicomisario**. Dos personas no deben compartir un fideicomisario. Si una persona controla tantos fideicomisarios como el umbral, esa persona puede descifrar los votos sola.
:::

1. Haga los pasos del 1 al 6 de [Crear un administrador](#create-an-administrator).
2. En **Actuar como fideicomisario**, seleccione el fideicomisario que controla esta persona.
3. En la tabla de roles, seleccione **Activo** para el rol `trustee`.
4. Haga clic en **Guardar**.
5. Entregue a la persona el nombre de usuario y la contraseña. Use un canal seguro.

**Resultado esperado:** aparece el mensaje "Votante creado". Cuando la persona inicia sesión, ve la ceremonia de claves y la ceremonia de recuento de su fideicomisario.

## Editar un usuario {#edit-a-user}

1. En la columna **Acciones** del usuario, abra el menú.
2. Haga clic en **Editar**.
3. Cambie los campos, los roles o **Actuar como fideicomisario**.
4. Haga clic en **Guardar**. La pantalla **Revisar cambios** muestra cada cambio con su **Valor actual** y su **Valor nuevo**.
5. Lea los cambios.
6. Haga clic en **Confirmar cambios**. Para corregir un valor, haga clic en **Editar**.

**Resultado esperado:** aparece el mensaje "Votante editado".

Si no ha cambiado ningún valor, aparece el mensaje "No hay cambios que revisar".

## Deshabilitar o habilitar un usuario {#disable-or-enable-a-user}

Un usuario deshabilitado no puede iniciar sesión. La cuenta y sus datos permanecen en el sistema.

1. Abra el usuario con **Editar**.
2. Desmarque **Habilitado \*** para deshabilitar el usuario, o selecciónelo para habilitarlo.
3. Haga clic en **Guardar**.
4. Haga clic en **Confirmar cambios**.

## Cambiar la contraseña de un usuario {#change-the-password-of-a-user}

1. En la columna **Acciones** del usuario, abra el menú.
2. Haga clic en **Cambiar contraseña**.
3. En **Contraseña:**, escriba la nueva contraseña.
4. En **Repetir Contraseña:**, escriba la misma contraseña.
5. Asegúrese de que **Temporal** está activado.
6. Haga clic en **Guardar**.
7. Entregue a la persona la nueva contraseña. Use un canal seguro.

**Resultado esperado:** aparece el mensaje "Votante editado".

## Eliminar un usuario {#delete-a-user}

:::caution PRECAUCIÓN
No puede deshacer una eliminación. Si la persona necesita acceso más adelante, debe crear una cuenta nueva. Para detener el acceso durante un tiempo breve, deshabilite el usuario.
:::

1. En la columna **Acciones** del usuario, abra el menú.
2. Haga clic en **Eliminar**.
3. Lea el mensaje "¿Estás seguro que quieres borrar este usuario?".
4. Haga clic en **Eliminar**.

**Resultado esperado:** aparece el mensaje "Usuario borrado". El usuario no aparece en la lista.

No puede eliminar su propia cuenta. Para eliminar más de un usuario, seleccione los usuarios en la lista y haga clic en **Eliminar** encima de la lista.

## Importar usuarios {#import-users}

1. Cree un archivo CSV con una fila para cada usuario. Use estos nombres de columna en la primera fila: `username`, `email`, `first_name`, `last_name`, `enabled`, `password` y `group_name`.
2. En `group_name`, escriba el nombre de un rol, por ejemplo `admin`.
3. En la pestaña **Usuarios**, haga clic en **Importar**.
4. Arrastre el archivo CSV al panel, o haga clic en el panel para seleccionar el archivo.
5. Espere a que aparezca el mensaje "Fichero subido al servidor - pero no importado aún".
6. Haga clic en **Importar**.
7. Espere a que termine la tarea **Importar Usuarios**.

**Resultado esperado:** los nuevos usuarios aparecen en la lista.

:::note NOTA
Cada fila puede tener solo un rol en `group_name`. Sin la columna `group_name`, los usuarios no reciben un rol de administrador. Para añadir más roles, edite el usuario después de la importación.
:::

## Exportar usuarios {#export-users}

1. En la pestaña **Usuarios**, haga clic en **Exportar**.
2. Lea el mensaje "La exportación puede ser una operación larga. ¿Estás seguro de que deseas exportar los registros?".
3. Haga clic en **Exportar**.
4. Espere a que el navegador descargue el archivo `users-export.csv`.

El archivo contiene el identificador, el email, el nombre, el nombre de usuario, el estado y los demás datos del usuario. No contiene las contraseñas ni los roles.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| Aparece el mensaje "No tienes permiso para acceder a los usuarios o roles." | Pida a un administrador del inquilino que dé a su cuenta los permisos necesarios. |
| Un usuario no puede iniciar sesión. | Asegúrese de que **Habilitado \*** está seleccionado para el usuario. Use **Cambiar contraseña** para establecer una nueva contraseña temporal. |
| **Actuar como fideicomisario** no aparece en el formulario de usuario. | El perfil de usuario del inquilino no tiene el campo de fideicomisario. Pida ayuda al soporte de Sequent. |
| **Actuar como fideicomisario** no tiene nombres. | La pestaña **AUTORIDADES** de **Configuración** está vacía. Pida al soporte de Sequent que añada los fideicomisarios. |
| Un usuario fideicomisario no ve la ceremonia. | Asegúrese de que el usuario tiene el rol `trustee` y el fideicomisario correcto en **Actuar como fideicomisario**. Pida al usuario que cierre la sesión y vuelva a iniciarla. |
| Aparece el mensaje "Error creando votante: …". | Lea el motivo en el mensaje. Corrija el campo y, después, haga clic de nuevo en **Guardar**. |
| Aparece el mensaje "Votante creado, pero no se pudo establecer su contraseña". | La cuenta existe sin contraseña. Use **Cambiar contraseña** para establecer la contraseña. |
| Aparece el mensaje "Error editando votante". | Asegúrese de que su cuenta tiene los permisos necesarios. Si cambió una contraseña, asegúrese de que cumple las reglas de contraseña del inquilino. |
| No se muestra el botón **Añadir**, **Importar** o **Exportar**. | Su cuenta no tiene el permiso necesario. Consulte [Permiso necesario](#required-permission). |

**Procedimiento relacionado:** [Configurar el inquilino](../../../03-procedures/01-tenant.md), paso 3.
