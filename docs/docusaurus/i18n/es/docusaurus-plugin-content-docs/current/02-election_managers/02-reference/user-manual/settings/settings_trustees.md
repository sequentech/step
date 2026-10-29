---
id: settings_trustees
title: Fideicomisarios
description: "La pestaña AUTORIDADES está en Configuración. Muestra la lista de fideicomisarios del inquilino."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **AUTORIDADES** está en **Configuración**. Muestra la lista de fideicomisarios del inquilino.

Un fideicomisario es un servicio que guarda una parte de la clave de la elección. Los votos solo se pueden descifrar cuando suficientes fideicomisarios trabajan juntos. Cada servicio de fideicomisario tiene una persona que lo controla: el usuario fideicomisario. La ceremonia de claves y la ceremonia de recuento muestran esta lista para que usted pueda seleccionar los fideicomisarios de un proceso electoral.

Sequent instala los servicios de los fideicomisarios y los añade a esta lista. Usted no crea los fideicomisarios. Use esta pestaña para comprobar que la lista está completa antes de la ceremonia de claves.

Para la configuración técnica de los servicios de los fideicomisarios, consulte [Configuración de fideicomisarios de Braid](../../../../../../../../docs/07-developers/09-braid/braid_trustees_configuration.md).

## Cómo abrirlo {#how-to-open-it}

1. En el menú de la izquierda, haga clic en **Configuración**.
2. Haga clic en la pestaña **AUTORIDADES**.

## Permiso necesario {#required-permission}

- Para abrir **Configuración**, su cuenta debe tener los permisos **Editar Inquilino** y **Ver configuración**.
- Para ver la lista, su cuenta debe tener el permiso **Leer Fideicomisario**. Sin este permiso, la pestaña muestra "Aún no hay fideicomisarios."
- El botón **Añadir** solo se muestra a los usuarios con el permiso **Editar Fideicomisario**.
- El botón **Exportar** solo se muestra a los usuarios con el permiso **Exportar Fideicomisarios**.

## Columnas y botones {#columns-and-buttons}

| Elemento | Significado |
| --- | --- |
| **Nombre** | El nombre del fideicomisario. Debe ser igual que el nombre del servicio de fideicomisario. El formulario de usuario muestra este nombre en **Actuar como fideicomisario**. |
| **Clave pública** | La clave pública del servicio de fideicomisario. Está oculta por defecto. Para mostrarla, haga clic en **Columnas**. |
| **ID** | El identificador interno del fideicomisario. Está oculto por defecto. |
| Icono de editar | Abre **Editar Fideicomisario**, con los campos **Nombre** y **Clave pública**. |
| Icono de eliminar | Elimina el fideicomisario de la lista, después de una confirmación. |
| **Añadir** | Abre **Crear Fideicomisario**, con los campos **Nombre** y **Clave pública**. |
| **Exportar** | Genera un archivo cifrado con la configuración de los servicios de los fideicomisarios. |
| **Filtrar** | Filtra la lista por **Nombre** o **Clave pública**. |
| **Columnas** | Muestra u oculta columnas. |

:::danger ADVERTENCIA
No añada, edite ni elimine un fideicomisario a menos que el soporte de Sequent se lo pida. Un nombre o una clave pública incorrectos detienen la ceremonia de claves o la ceremonia de recuento. Si elimina un fideicomisario que guarda una parte de la clave de una elección, los votos de ese proceso electoral pueden quedar imposibles de descifrar.
:::

## Vincular un usuario fideicomisario a un fideicomisario {#link-a-trustee-user-to-a-trustee}

El usuario fideicomisario es una persona con una cuenta en el portal de administración. Para vincular la cuenta a un fideicomisario:

1. En el menú de la izquierda, haga clic en **Usuarios y Roles**.
2. En la pestaña **Usuarios**, cree o edite el usuario.
3. En **Actuar como fideicomisario**, seleccione el nombre del fideicomisario.
4. En la lista de roles, seleccione el rol `trustee`.
5. Guarde el usuario.

Consulte [Usuarios](../users-and-roles/users-and-roles_users.md) para ver todos los pasos.

:::danger ADVERTENCIA
Vincule cada fideicomisario a una sola persona, y vincule cada persona a un solo fideicomisario. Si una persona controla tantos fideicomisarios como el umbral, esa persona puede descifrar los votos sola.
:::

## Exportar la configuración de los fideicomisarios {#export-the-trustee-configuration}

El archivo de exportación contiene configuración secreta de los servicios de los fideicomisarios. Haga esto solo cuando el soporte de Sequent se lo pida.

1. Haga clic en **Exportar**.
2. Espere a que termine la tarea **Exportar Autoridades**.
3. Espere a que el navegador descargue el archivo `trustees-export.ezip`.
4. El diálogo **Contraseña** muestra la contraseña del archivo. Haga clic en el botón de copiar y guarde la contraseña.

:::danger ADVERTENCIA
Guarde el archivo y su contraseña en dos lugares seguros diferentes. Una persona que tenga ambos puede leer la configuración secreta de los servicios de los fideicomisarios. El portal de administración muestra la contraseña solo una vez.
:::

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| La pestaña muestra "Aún no hay fideicomisarios." | Su cuenta no tiene el permiso **Leer Fideicomisario**, o Sequent no ha añadido los fideicomisarios. Pida ayuda al soporte de Sequent. |
| Falta un fideicomisario en la lista. | Pida al soporte de Sequent que añada el fideicomisario. No puede realizar la ceremonia de claves sin todos los fideicomisarios. |
| Un usuario fideicomisario no ve la ceremonia de claves. | Asegúrese de que el usuario tiene el rol `trustee` y el fideicomisario correcto en **Actuar como fideicomisario**. |
| No se muestra el botón **Añadir** o **Exportar**. | Su cuenta no tiene el permiso necesario. |

**Procedimientos relacionados:** [Configurar el inquilino](../../../03-procedures/01-tenant.md) y [Realizar la ceremonia de claves](../../../03-procedures/04-keys.md).
