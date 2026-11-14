---
id: admin_portal_reference_user_manual_settings_backup___restore
title: Copia de seguridad / Restaurar
description: "La pestaña COPIA DE SEGURIDAD / RESTAURAR guarda la configuración del inquilino en un archivo y carga una configuración desde un archivo."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **COPIA DE SEGURIDAD / RESTAURAR** guarda la configuración del inquilino en un
archivo y carga una configuración desde un archivo. La copia de seguridad contiene la
configuración del inquilino, la configuración del inicio de sesión, y los roles y los permisos.
No contiene los procesos electorales. Para conservar un proceso electoral, expórtelo desde su
pestaña **Datos**.

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, haga clic en **Configuración**.
2. Haga clic en la pestaña **COPIA DE SEGURIDAD / RESTAURAR**. El título es
   **Copia de seguridad / Restaurar configuración del inquilino**.

## Hacer una copia de seguridad {#make-a-backup}

1. En "Copia de seguridad de las configuraciones del inquilino", haga clic en **Respaldo**.
2. Espere a que termine la tarea "Exportar Configuración del Cliente".
3. Guarde el archivo descargado, `tenant-config-<tenant>-export.zip`, en un lugar seguro.

**Resultado esperado:** tiene el archivo de copia de seguridad.

## Restaurar una copia de seguridad {#restore-a-backup}

:::caution PRECAUCIÓN
Una restauración reemplaza la configuración del inquilino, la configuración del inicio de
sesión y los roles. Haga una restauración solo con la ayuda del soporte de Sequent.
:::

1. En "Restaurar la configuración del inquilino", seleccione **Importar configuraciones del inquilino**.
2. Seleccione **Importar configuraciones de Keycloak**.
3. Seleccione **Importar configuraciones de roles y permisos**.
4. Haga clic en **Restaurar**. **Restaurar** solo está disponible cuando las tres opciones
   están seleccionadas.
5. En el panel **Importar configuraciones del inquilino**, escriba el hash SHA-256 del archivo
   en **Verificación de Integridad (SHA 256)**.
6. Suba el archivo zip y haga clic en **Importar**.

**Resultado esperado:** se ejecuta la tarea "Importar Configuración del Cliente". Cuando
termina, el inquilino tiene la configuración del archivo.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| **Restaurar** no está disponible. | Seleccione las tres opciones de importación. |
| La tarea "Exportar Configuración del Cliente" o "Importar Configuración del Cliente" muestra `FAILED`. | Lea los registros de la tarea. Contacte con el soporte de Sequent. |

## Páginas relacionadas {#related-pages}

- [Configurar el inquilino](../../../../procedures/01-tenant.md#4-make-a-backup-of-the-tenant-configuration)
- [Configuración](../admin_portal_reference_user-manual_settings.md)
