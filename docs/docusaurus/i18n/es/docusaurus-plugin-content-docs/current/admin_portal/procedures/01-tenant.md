---
title: Configurar el inquilino
sidebar_position: 1
description: "Este procedimiento prepara el inquilino para sus procesos electorales. Usted define la configuración del inquilino, crea las cuentas de los administradores y de los fideicomisarios, y hace una copia de seguridad de la configuración."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Configurar el inquilino

Este procedimiento prepara el inquilino para sus procesos electorales. Usted define la
configuración del inquilino, crea las cuentas de los administradores y de los fideicomisarios, y
hace una copia de seguridad de la configuración. El administrador del inquilino hace este
procedimiento una vez, antes del primer proceso electoral.

## Antes de empezar {#before-you-start}

- Sequent ha creado su inquilino y le ha dado una cuenta con el rol `admin`.
- Sequent ha instalado los servicios de los fideicomisarios y ha añadido los fideicomisarios en
  **Configuracion** > **AUTORIDADES**.
- Usted tiene la lista de personas: los administradores y los fideicomisarios, con sus nombres y
  sus direcciones de correo electrónico.
- Para cada persona fideicomisaria, usted sabe qué fideicomisario de la lista representa.

:::note
Solo Sequent puede crear un inquilino. El icono **+** junto al nombre del inquilino no funciona
para los otros usuarios.
:::

## 1. Definir la configuración del inquilino {#1-set-the-tenant-settings}

1. En el menú de la izquierda, haga clic en **Configuracion**.
2. Haga clic en cada pestaña y defina los valores. La tabla indica lo que controla cada pestaña.

| Pestaña | Lo que controla |
| --- | --- |
| **TIPOS DE ELECCIÓN** | La lista de tipos de elección del inquilino. |
| **CANALES DE VOTACIÓN** | Los canales de votación que el inquilino puede usar: **Votación en Línea** y **Votación en Quiosco**. El portal de administración guarda un cambio inmediatamente. |
| **PLANTILLAS** | Muestra si el inquilino puede enviar mensajes por correo electrónico (**Mails**) y por mensaje de texto (**SMS**). No puede cambiar estos valores. Pregunte al soporte de Sequent. |
| **IDIOMAS** | Los idiomas que los procesos electorales pueden usar. Solo los idiomas que active aquí están disponibles en los procesos electorales. |
| **LOCALIZACIÓN** | Cambios en los textos de los portales, para cada idioma. Seleccione el idioma, haga clic en **Añadir** y después escriba la **Clave** y el **Valor**. |
| **PERSONILIZACIÓN DE APARIENCIA** | La **Logo URL** (dirección del logotipo), el **CSS Personalizado** y los **Enlaces de Ayuda** del inquilino. Los enlaces de ayuda aparecen en el menú **Ayuda**. |
| **AUTORIDADES** | La lista de fideicomisarios. Sequent añade los fideicomisarios. No cambie esta lista. |
| **Countries** (países) | Los países desde los que los votantes no pueden votar ni inscribirse. |
| **COPIA DE SEGURIDAD / RESTAURAR** | La copia de seguridad y la restauración de la configuración del inquilino. Consulte el [paso 4](#4-make-a-backup-of-the-tenant-configuration). |

3. En la pestaña **IDIOMAS**, active cada idioma que usarán sus procesos electorales.
4. En la pestaña **CANALES DE VOTACIÓN**, active cada canal de votación que usarán sus procesos
   electorales.
5. En la pestaña **AUTORIDADES**, asegúrese de que la lista contiene todos los fideicomisarios.

**Resultado esperado:** la configuración muestra los valores que usted ha definido. La pestaña
**AUTORIDADES** muestra una fila para cada fideicomisario.

## 2. Comprobar los roles {#2-check-the-roles}

Un rol es un conjunto de permisos. Cada usuario recibe uno o más roles.

1. En el menú de la izquierda, haga clic en **Usuarios y Roles**.
2. Haga clic en la pestaña **Roles**.
3. Asegúrese de que la lista contiene los roles que necesita, por ejemplo `admin` y `trustee`.
4. Para ver los permisos de un rol, haga clic en el icono de editar en la fila del rol. Cada
   permiso tiene una casilla en la columna **Active** (activo).

:::caution PRECAUCIÓN
Un cambio en un rol se aplica inmediatamente a todos los usuarios con ese rol. No quite ningún
permiso del rol `admin`. Si lo hace, puede perder el acceso a una parte del portal de
administración.
:::

Para crear un rol:

1. En la pestaña **Roles**, haga clic en **Añadir**.
2. Escriba el **Nombre** del rol.
3. En la lista de permisos, seleccione **Active** para cada permiso del rol.
4. Haga clic en **Guardar**.

**Resultado esperado:** aparece el mensaje "Rol creado".

**Resultado esperado:** la pestaña **Roles** muestra los roles con los permisos que necesita.

## 3. Crear las cuentas de usuario {#3-create-the-user-accounts}

Haga estos pasos para cada administrador y para cada fideicomisario.

1. En el menú de la izquierda, haga clic en **Usuarios y Roles**.
2. Haga clic en la pestaña **Usuarios**.
3. Haga clic en **Añadir**. Si la lista está vacía, haga clic en **Crear usuario**.
4. Escriba el nombre de usuario y los otros campos del formulario, por ejemplo la dirección de
   correo electrónico y el nombre.
5. Seleccione **Habilitado**.
6. Escriba una contraseña en **Contraseña:** y escríbala de nuevo en **Repetir Contraseña:**.
7. Asegúrese de que **Temporal** está activado. Así, el usuario debe cambiar la contraseña en el
   primer inicio de sesión.
8. Solo para un fideicomisario: en **Actuar como fideicomisario**, seleccione el fideicomisario que
   representa esta persona.
9. En la lista de roles, seleccione **Active** para cada rol del usuario:
    - Administradores: `admin`.
    - Fideicomisarios: `trustee`.
10. Haga clic en **Guardar**.
11. Dé el nombre de usuario y la contraseña temporal a la persona. Use un canal seguro.

:::danger ADVERTENCIA
Dé a cada fideicomisario su propia cuenta y su propio fideicomisario en **Actuar como
fideicomisario**. Dos personas no deben compartir un fideicomisario. Si una persona controla más
fideicomisarios que el umbral, esa persona puede descifrar los votos sola.
:::

**Resultado esperado:** la pestaña **Usuarios** muestra los nuevos usuarios. Cada usuario puede
iniciar sesión, cambia la contraseña y ve los elementos del menú de su rol.

## 4. Hacer una copia de seguridad de la configuración del inquilino {#4-make-a-backup-of-the-tenant-configuration}

Haga una copia de seguridad después de configurar el inquilino y después de cada cambio
importante.

1. En el menú de la izquierda, haga clic en **Configuracion**.
2. Haga clic en la pestaña **COPIA DE SEGURIDAD / RESTAURAR**.
3. Haga clic en **Respaldo**.
4. Espere a que la tarea termine. El navegador descarga un archivo con el nombre
   `tenant-config-<tenant>-export.zip`.
5. Guarde el archivo en un lugar seguro. Contiene la configuración de su inquilino.

**Resultado esperado:** usted tiene el archivo de copia de seguridad.

:::caution PRECAUCIÓN
Una restauración reemplaza la configuración del inquilino, la configuración del inicio de sesión
y los roles. Haga una restauración solo con la ayuda del soporte de Sequent.
:::

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| Aparece el mensaje "No tienes permiso para acceder a la configuración." | Su cuenta no tiene los permisos necesarios. Pida al soporte de Sequent que dé a su cuenta el rol `admin`. |
| La pestaña **AUTORIDADES** está vacía o falta un fideicomisario. | Pida al soporte de Sequent que añada los fideicomisarios. Sin ellos no puede hacer la ceremonia de claves. |
| **Votación en Línea** aparece activada después de desactivarla. | Es un error de visualización de la versión 9.0. Pida al soporte de Sequent que confirme el valor. |
| **Actuar como fideicomisario** no está en el formulario del usuario. | El perfil de usuario del inquilino no tiene el campo de fideicomisario. Pregunte al soporte de Sequent. |
| Un usuario no puede iniciar sesión. | Asegúrese de que **Habilitado** está seleccionado para el usuario. En la pestaña **Usuarios**, use **Cambiar contraseña** para definir una nueva contraseña temporal. |
| El botón **Añadir** no está en la pestaña **Usuarios**. | En la versión 9.0 este botón necesita el permiso **Crear Votante**. Pida al soporte de Sequent que compruebe su rol. |
| "Las contraseñas deben de coincidir" | Escriba la misma contraseña en **Contraseña:** y en **Repetir Contraseña:**. |

## Más información {#more-information}

- [Usuarios y roles](../Reference/User-Manual/Users-and-Roles/admin_portal_reference_user-manual_users-and-roles.md)
- [Usuarios](../Reference/User-Manual/Users-and-Roles/Users/admin_portal_reference_user-manual_users-and-roles_users.md)
- [Roles](../Reference/User-Manual/Users-and-Roles/Roles/admin_portal_reference_user-manual_users-and-roles_roles.md)
- [Permisos](../Reference/User-Manual/Users-and-Roles/Permissions/admin_portal_reference_user-manual_users-and-roles_permissions.md)
- [Configuración](../Reference/User-Manual/Settings/admin_portal_reference_user-manual_settings.md)
- [Fideicomisarios](../Reference/User-Manual/Settings/Trustees/admin_portal_reference_user-manual_settings_trustees.md)
- [Etiquetas de permiso](../Tutorials/admin_portal_tutorials_permission-labels.md)

**Siguiente:** [Crear el proceso electoral](02-event.md).
