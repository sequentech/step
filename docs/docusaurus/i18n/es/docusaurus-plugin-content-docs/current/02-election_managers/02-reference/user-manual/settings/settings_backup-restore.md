---
id: settings_backup-restore
title: Copia de seguridad y restauración
description: "La pestaña COPIA DE SEGURIDAD / RESTAURAR está en Configuracion. Su título es Copia de seguridad / Restaurar configuración del inquilino. Úsela para crear un archivo de copia de seguridad de la configuración del inquilino y para restaurar una configuración a partir de ese archivo."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **COPIA DE SEGURIDAD / RESTAURAR** está en **Configuracion**. Su título es **Copia de seguridad / Restaurar configuración del inquilino**. Úsela para crear un archivo de copia de seguridad de la configuración del inquilino y para restaurar una configuración a partir de ese archivo. La usa el administrador del inquilino.

La copia de seguridad no contiene procesos electorales, votantes, usuarios ni votos. Solo contiene la configuración del inquilino.

## Cómo abrirla {#how-to-open-it}

1. En el menú de la izquierda, haga clic en **Configuracion**.
2. Haga clic en la pestaña **COPIA DE SEGURIDAD / RESTAURAR**.

## Permiso necesario {#required-permission}

- Para abrir **Configuracion**, su cuenta debe tener los permisos **Editar Inquilino** y **Ver configuración**.
- Para crear una copia de seguridad, su cuenta debe tener el permiso **Leer Inquilino**.
- Para restaurar, su cuenta debe tener el permiso **Editar Inquilino**.

## Contenido del archivo de copia de seguridad {#content-of-the-backup-file}

La copia de seguridad es un archivo ZIP con el nombre `tenant-config-<tenant ID>-export.zip`. Contiene tres archivos:

| Contenido del archivo | Qué contiene | Opción de restauración |
| --- | --- | --- |
| Configuración del inquilino | La configuración del inquilino, por ejemplo los idiomas, los canales de votación, la apariencia, las listas de países y los enlaces de ayuda. | **Importar configuraciones del inquilino** |
| Configuración de inicio de sesión | La configuración del servicio de inicio de sesión (Keycloak) del inquilino. | **Importar configuraciones de Keycloak** |
| Roles y permisos | Cada rol del inquilino y sus permisos. | **Importar configuraciones de roles y permisos** |

## Campos y botones {#fields-and-buttons}

| Elemento | Significado |
| --- | --- |
| **Copia de seguridad de las configuraciones del inquilino** / **Respaldo** | Inicia una tarea que crea el archivo de copia de seguridad. Después, el navegador descarga el archivo. |
| **Restaurar la configuración del inquilino** | La sección para restaurar un archivo de copia de seguridad. |
| **Importar configuraciones del inquilino** | Una casilla. Restaura la configuración del inquilino. |
| **Importar configuraciones de Keycloak** | Una casilla. Restaura el nombre visible y los textos de las páginas de inicio de sesión. Otros ajustes de inicio de sesión no cambian. |
| **Importar configuraciones de roles y permisos** | Una casilla. Restaura los roles y sus permisos. |
| **Restaurar** | Abre el panel **Importar configuraciones del inquilino**. El botón solo funciona cuando las tres casillas están seleccionadas. |
| **Verificación de Integridad (SHA 256)** | En el panel de restauración. El hash SHA-256 del archivo de copia de seguridad. El sistema compara el archivo con este valor. |
| **Cancelar** / **Importar** | En el panel de restauración. **Importar** inicia la restauración. Está disponible cuando termina la subida del archivo. |

## Crear una copia de seguridad {#make-a-backup}

1. Abra la pestaña **COPIA DE SEGURIDAD / RESTAURAR**.
2. Haga clic en **Respaldo**.
3. Espere a que termine la tarea **Exportar Configuración del Cliente**.
4. Espere a que el navegador descargue el archivo `tenant-config-<tenant ID>-export.zip`.
5. Guarde el archivo en un lugar seguro.

**Resultado esperado:** tiene el archivo de copia de seguridad.

Haga una copia de seguridad después de configurar el inquilino y después de cada cambio importante.

## Restaurar una copia de seguridad {#restore-a-backup}

:::caution PRECAUCIÓN
Haga una restauración solo con la ayuda del soporte de Sequent. Una restauración sobrescribe la configuración del inquilino. Para cada rol del archivo, la restauración establece los permisos del rol con los permisos del archivo. No puede deshacer una restauración, salvo con una copia de seguridad más reciente.
:::

1. Haga una copia de seguridad de la configuración actual. Siga los pasos de [Crear una copia de seguridad](#make-a-backup).
2. Seleccione **Importar configuraciones del inquilino**.
3. Seleccione **Importar configuraciones de Keycloak**.
4. Seleccione **Importar configuraciones de roles y permisos**.
5. Haga clic en **Restaurar**. Se abre el panel **Importar configuraciones del inquilino**.
6. En **Verificación de Integridad (SHA 256)**, escriba el hash SHA-256 del archivo de copia de seguridad.
7. Arrastre el archivo ZIP de la copia de seguridad al panel, o haga clic en el panel para seleccionar el archivo.
8. Espere al mensaje "File uploaded to server - but not imported yet".
9. Haga clic en **Importar**.
10. Espere a que termine la tarea **Importar Configuración del Cliente**.

**Resultado esperado:** la tarea indica que ha terminado. La configuración y los roles muestran los valores de la copia de seguridad.

:::note NOTA
Si deja vacío **Verificación de Integridad (SHA 256)**, el portal de administración muestra "Importar Sin Verificación de Integridad?". Haga clic en **Sí, Importar Sin Verificación de Integridad** solo si está seguro de que el archivo es correcto. Haga clic en **Volver** para escribir el hash.
:::

Se crea un rol que está en el archivo pero no en el inquilino. Un rol que está en el inquilino pero no en el archivo no cambia.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| El botón **Restaurar** no funciona. | Seleccione las tres casillas. El botón solo funciona cuando se seleccionan las tres. |
| Aparece el mensaje "Error uploading file". | Seleccione el archivo de nuevo. Asegúrese de que es el archivo ZIP de una copia de seguridad. |
| La tarea **Exportar Configuración del Cliente** o **Importar Configuración del Cliente** falla. | Asegúrese de que su cuenta tiene el permiso necesario. Después, inténtelo de nuevo. Si la tarea vuelve a fallar, contacte con el soporte de Sequent. |
| La tarea de restauración falla con un mensaje sobre la integridad o el hash. | El hash que escribió no es el hash del archivo. Compruebe el hash y el archivo, y vuelva a intentarlo. |
| El navegador no descarga el archivo de copia de seguridad. | Asegúrese de que el navegador permite las descargas desde el portal de administración. Haga clic en **Respaldo** de nuevo. |

**Procedimiento relacionado:** [Configurar el inquilino](../../../03-procedures/01-tenant.md), paso 4.
