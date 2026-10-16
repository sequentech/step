---
id: admin_portal_reference_user_manual_settings_trustees
title: Configuración de fideicomisarios
sidebar_label: Fideicomisarios
description: "La pestaña AUTORIDADES es la lista de los fideicomisarios del inquilino. Un fideicomisario guarda un fragmento de la clave privada de una elección. La ceremonia de claves y la ceremonia de recuento usan los fideicomisarios de esta lista."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **AUTORIDADES** es la lista de los fideicomisarios del inquilino. Un fideicomisario
guarda un fragmento de la clave privada de una elección. La ceremonia de claves y la ceremonia
de recuento usan los fideicomisarios de esta lista. Cada fideicomisario de la lista es también
un servicio de fideicomisario que instala Sequent. Sequent añade los fideicomisarios. No cambie
esta lista.

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, haga clic en **Configuración**.
2. Haga clic en la pestaña **AUTORIDADES**.

Las acciones necesitan estos permisos:

| Acción | Permiso |
| --- | --- |
| Ver la lista | **Leer Fideicomisario** |
| Crear y editar un fideicomisario | **Editar Fideicomisario** |
| Exportar la lista | **Exportar Fideicomisarios** |

## Elementos de la pantalla {#screen-elements}

| Elemento | Uso |
| --- | --- |
| **Crear fideicomisario** | Abre el formulario **Crear fideicomisario**, con los campos **Name** (nombre) y **Public key** (clave pública). |
| **Añadir filtro** | Filtra por **Name** o **Public Key**. |
| **Exportar** | Exporta la lista. La tarea "Exportar Autoridades" crea el archivo. |
| **Id**, **Public key**, **Name** | Las columnas de la lista. |
| Iconos de editar y de borrar | Cambian o borran un fideicomisario. |

Cuando la lista está vacía, la pestaña muestra "Aún no hay fideicomisarios.".

## Vincular una persona a un fideicomisario {#link-a-person-to-a-trustee}

La persona fideicomisaria inicia sesión con su propia cuenta. La cuenta se vincula a un
fideicomisario de esta lista con **Actuar como fideicomisario** en la pestaña **Usuarios**.
Consulte [Usuarios](../../Users-and-Roles/Users/admin_portal_reference_user-manual_users-and-roles_users.md).

:::danger ADVERTENCIA
Asigne cada fideicomisario de la lista a una sola persona. Si una persona controla más
fideicomisarios que el umbral, esa persona puede descifrar los votos sola.
:::

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| La lista está vacía o falta un fideicomisario. | Pida al soporte de Sequent que añada los fideicomisarios. |
| Un fideicomisario no está en **Actuar como fideicomisario**. | El fideicomisario no está en esta lista. Pregunte al soporte de Sequent. |

## Documentación técnica {#technical-documentation}

Para la configuración de los servicios de fideicomisario, consulte la
[Braid Trustees Configuration Guide](/docs/developers/Braid/braid_trustees_configuration)
(guía de configuración de los fideicomisarios de Braid).

## Páginas relacionadas {#related-pages}

- [Realizar la ceremonia de claves](../../../../procedures/04-keys.md)
- [Configurar el inquilino](../../../../procedures/01-tenant.md)
- [Configuración](../admin_portal_reference_user-manual_settings.md)
