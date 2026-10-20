---
title: Configurar el inquilino
sidebar_position: 1
description: "Este procedimiento prepara el inquilino para sus procesos electorales. Configura los ajustes del inquilino, crea las cuentas de los administradores y de los fideicomisarios, y hace una copia de seguridad de la configuración."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Configurar el inquilino

Este procedimiento prepara el inquilino para sus procesos electorales. Usted configura los ajustes del inquilino, crea las cuentas de los administradores y de los fideicomisarios, y hace una copia de seguridad de la configuración. El administrador del inquilino realiza este procedimiento una vez, antes del primer proceso electoral.

## Antes de empezar {#before-you-start}

- Sequent ha creado su inquilino y le ha dado una cuenta con el rol `admin`.
- Sequent ha instalado los servicios de los fideicomisarios y los ha añadido en **Configuración** > **AUTORIDADES**.
- Tiene la lista de personas: los administradores y los fideicomisarios, con sus nombres y direcciones de correo electrónico.
- Para cada persona fideicomisaria, sabe qué fideicomisario de la lista representa.

:::note NOTA
Solo Sequent puede crear un inquilino. El icono **+** junto al nombre del inquilino no funciona para otros usuarios.
:::

## 1. Configurar los ajustes del inquilino {#1-set-the-tenant-settings}

1. En el menú de la izquierda, haga clic en **Configuración**.
2. Haga clic en cada pestaña y configure los valores. La tabla indica qué controla cada pestaña.

| Pestaña | Qué controla |
| --- | --- |
| **TIPOS DE ELECCIÓN** | No se usa. No tiene que configurarla. |
| **CANALES DE VOTACIÓN** | No lo usan los procesos electorales. Cada proceso electoral configura sus propios canales en **Canales de Voto Permitidos**, en su pestaña **Datos**. Consulte [Canales de votación](../02-reference/user-manual/settings/settings_voting-channels.md). |
| **PLANTILLAS** | No se usa. No se pueden cambiar los interruptores. |
| **IDIOMAS** | Los idiomas que pueden usar los procesos electorales, el **Idioma predeterminado** y la **Política de detección de idioma**. |
| **LOCALIZACIÓN** | Cambios en los textos de los portales, para cada idioma. Seleccione el idioma, haga clic en **Añadir** y, después, escriba la **Clave** y el **Valor**. |
| **INTEGRACIONES** | La cuenta de Google Calendar que genera los enlaces de reuniones de vídeo. Consulte [Credenciales de Google Meet](../01-tutorials/98-admin_portal_tutorials_create-and-set-google-meet-credentials.md). |
| **PERSONILIZACIÓN DE APARIENCIA** | El **Logo URL**, el **CSS Personalizado** y los **Enlaces de Ayuda** del portal de administración. No cambian el portal de votación. Los enlaces de ayuda aparecen en el menú **Ayuda**. |
| **AUTORIDADES** | La lista de fideicomisarios. Sequent añade los fideicomisarios. No cambie esta lista. Consulte [Fideicomisarios](../02-reference/user-manual/settings/settings_trustees.md). |
| **Países** | Los países desde los que los votantes no pueden votar ni inscribirse. Solo funciona si Sequent ha configurado el bloqueo de países para su plataforma. Consulte [Países](../02-reference/user-manual/settings/settings_countries.md). |
| **COPIA DE SEGURIDAD / RESTAURAR** | La copia de seguridad y la restauración de la configuración del inquilino. Consulte el [paso 4](#4-make-a-backup-of-the-tenant-configuration). |
| **Vistas Previas** | Un registro de las vistas previas de papeletas que solicitaron sistemas externos. No se puede cambiar. |

3. En la pestaña **IDIOMAS**, active cada idioma que usarán sus procesos electorales. Seleccione el **Idioma predeterminado**. En **Política de detección de idioma**, seleccione **Detectar desde el navegador** para mostrar el idioma del navegador del votante, o **Forzar predeterminado** para mostrar siempre el idioma predeterminado.
4. En la pestaña **AUTORIDADES**, asegúrese de que la lista contiene todos los fideicomisarios.

**Resultado esperado:** la configuración muestra los valores que ha establecido. La pestaña **AUTORIDADES** muestra una fila por cada fideicomisario.

:::note NOTA
Un nuevo proceso electoral copia la configuración de idiomas del inquilino al crearlo. Un cambio en la pestaña **IDIOMAS** no modifica los procesos electorales que ya existen.
:::

## 2. Comprobar los roles {#2-check-the-roles}

Un rol es un conjunto de permisos. Cada usuario recibe uno o más roles.

1. En el menú de la izquierda, haga clic en **Usuarios y Roles**.
2. Haga clic en la pestaña **Roles**.
3. Asegúrese de que la lista contiene los roles que necesita, por ejemplo `admin` y `trustee`.
4. Para ver los permisos de un rol, haga clic en su icono de editar. El panel **Información de Rol** muestra cada permiso con una casilla **Activo**.

:::caution PRECAUCIÓN
Un cambio en un rol se aplica de inmediato a todos los usuarios con ese rol. No quite ningún permiso del rol `admin`. Si lo hace, puede perder el acceso a una parte del portal de administración.
:::

Para crear un rol:

1. En la pestaña **Roles**, haga clic en **Añadir**.
2. Escriba el **Nombre** del rol. No podrá cambiar el nombre más adelante.
3. En la tabla de permisos, seleccione **Activo** para cada permiso del rol.
4. Haga clic en **Guardar**.

Para ver todas las opciones, consulte [Roles](../02-reference/user-manual/users-and-roles/users-and-roles_roles.md).

**Resultado esperado:** la pestaña **Roles** muestra los roles con los permisos que necesita.

## 3. Crear las cuentas de usuario {#3-create-the-user-accounts}

Haga estos pasos para cada administrador y para cada fideicomisario.

1. En el menú de la izquierda, haga clic en **Usuarios y Roles**.
2. Haga clic en la pestaña **Usuarios**.
3. Haga clic en **Añadir**. Si la lista está vacía, haga clic en **Crear usuario**.
4. Escriba el nombre de usuario y los demás campos del formulario, por ejemplo la dirección de correo electrónico y el nombre.
5. Asegúrese de que **Habilitado** está seleccionado. Está seleccionado por defecto.
6. Escriba una contraseña en **Contraseña:** y vuelva a escribirla en **Repetir Contraseña:**.
7. Asegúrese de que **Temporal** está activado. Está activado por defecto. El usuario debe cambiar entonces la contraseña en el primer inicio de sesión.
8. Solo para un fideicomisario: en **Actuar como fideicomisario**, seleccione el fideicomisario que representa esta persona.
9. En la tabla **Rol** del formulario, seleccione **Activo** para cada rol del usuario:
    - Administradores: `admin`.
    - Fideicomisarios: `trustee`.
10. Haga clic en **Guardar**. Aparece el mensaje "Votante creado". El portal de administración usa este mensaje para todos los usuarios.
11. Entregue a la persona el nombre de usuario y la contraseña temporal. Use un canal seguro.

:::danger ADVERTENCIA
Asigne a cada fideicomisario su propia cuenta y su propio fideicomisario en **Actuar como fideicomisario**. Dos personas no deben compartir un fideicomisario. Si una persona controla más fideicomisarios que el umbral, esa persona puede descifrar los votos sola.
:::

**Resultado esperado:** la pestaña **Usuarios** muestra los usuarios nuevos. Cada usuario puede iniciar sesión, cambiar la contraseña y ver los elementos del menú de su rol.

Para crear muchos usuarios a partir de un archivo CSV, consulte [Usuarios](../02-reference/user-manual/users-and-roles/users-and-roles_users.md). Dé a cada fila una columna `group_name` con el rol, por ejemplo `admin`. Sin ella, el usuario importado no recibe ningún rol de administrador.

## 4. Hacer una copia de seguridad de la configuración del inquilino {#4-make-a-backup-of-the-tenant-configuration}

Haga una copia de seguridad después de configurar el inquilino y después de cada cambio importante.

1. En el menú de la izquierda, haga clic en **Configuración**.
2. Haga clic en la pestaña **COPIA DE SEGURIDAD / RESTAURAR**.
3. Haga clic en **Respaldo**.
4. Espere a que termine la tarea. El navegador descarga un archivo con el nombre `tenant-config-<tenant ID>-export.zip`.
5. Guarde el archivo en un lugar seguro. Contiene la configuración de su inquilino.

**Resultado esperado:** tiene el archivo de copia de seguridad.

:::caution PRECAUCIÓN
Una restauración cambia la configuración del inquilino, los textos de las páginas de inicio de sesión y los roles del archivo. Cada rol del archivo recibe exactamente los permisos del archivo. Haga una restauración solo con la ayuda del soporte de Sequent. Consulte [Copia de seguridad / Restaurar](../02-reference/user-manual/settings/settings_backup-restore.md).
:::

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| Aparece el mensaje "No tienes permiso para acceder a la configuración." | Su cuenta no tiene los permisos necesarios. Pida al soporte de Sequent que dé a su cuenta el rol `admin`. |
| La pestaña **AUTORIDADES** muestra "Aún no hay fideicomisarios." | Su cuenta necesita el permiso **Leer Fideicomisario**. Si lo tiene, pida al soporte de Sequent que añada los fideicomisarios. No puede realizar la ceremonia de claves sin ellos. |
| **Actuar como fideicomisario** no aparece en el formulario de usuario. | El perfil de usuario del inquilino no tiene el campo de fideicomisario. Pida ayuda al soporte de Sequent. |
| El portal de administración muestra **Revisar cambios** al guardar un usuario. | Lea los cambios y, después, haga clic en **Confirmar cambios**. |
| Un usuario no puede iniciar sesión. | Asegúrese de que **Habilitado** está seleccionado para el usuario. En la pestaña **Usuarios**, use **Cambiar contraseña** para establecer una nueva contraseña temporal. |

## Más información {#more-information}

- [Configuración: Idiomas](../02-reference/user-manual/settings/settings_languages.md)
- [Configuración: Localización](../02-reference/user-manual/settings/settings_localization.md)
- [Configuración: Integraciones](../02-reference/user-manual/settings/settings_integrations.md)
- [Configuración: Personalización de apariencia](../02-reference/user-manual/settings/settings_look-feel.md)
- [Configuración: Vistas Previas](../02-reference/user-manual/settings/settings_previews.md)
- [Usuarios](../02-reference/user-manual/users-and-roles/users-and-roles_users.md)
- [Roles](../02-reference/user-manual/users-and-roles/users-and-roles_roles.md)
- [Permisos](../02-reference/user-manual/users-and-roles/users-and-roles_permissions.md)
- [Navegar por el portal de administración](../01-tutorials/01-admin_portal_tutorials_system-navigation.md)

**Siguiente:** [Crear el proceso electoral](02-event.md).
