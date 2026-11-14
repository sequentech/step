---
id: admin_portal_tutorials_import-elections
title: Importación y exportación de procesos electorales
description: "El portal de administración de Sequent ofrece herramientas para exportar los datos de un proceso electoral como copia de seguridad o para auditoría, y para importar configuraciones de procesos electorales existentes y preparar rápidamente nuevos procesos."
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

import GoogleVideo from '@site/src/components/GoogleVideo';

<GoogleVideo id="13KrQtqIfsw24sZCCBjO0-f6wM_kcDUI-" />

El portal de administración de Sequent ofrece herramientas para exportar los datos de un proceso electoral como copia de seguridad o para auditoría, y para importar configuraciones de procesos electorales existentes y preparar rápidamente nuevos procesos.

## Exportar un proceso electoral {#exporting-an-election-event}

Exportar le permite guardar una instantánea completa de su proceso electoral.

![Selección del menú de exportación](../../../../../../docs/02-election_managers/01-tutorials/assets/export_menu_selection.png)

1.  Abra la pestaña **Datos** del proceso electoral.
2.  Haga clic en **Exportar**.

![Contraseña e instrucciones de la exportación](../../../../../../docs/02-election_managers/01-tutorials/assets/export_menu_selections.png)

3.  Elija los componentes de datos que desea incluir en la exportación:
    * **Incluir Votantes:** Exporta la lista de votantes registrados.
    * **Registros de Actividad:** Incluye un historial de las acciones administrativas.
    * **Tablón de Anuncios:** Incluye el estado criptográfico del proceso electoral, como las ceremonias de claves.
    * **Publicaciones:** Incluye el historial de publicaciones del proceso electoral.
    * **Archivos S3:** Incluye imágenes, materiales de soporte u otros archivos que se guardan en el almacenamiento en la nube.
    * **Eventos Programados:** Incluye los eventos programados configurados.
    * **Informes:** Incluye la configuración de los informes del proceso electoral.
    * **Aplicaciones:** Incluye las solicitudes de inscripción de los votantes.
    * **Recuento:** Incluye los recuentos finales de votos (si están disponibles). Esta opción también selecciona **Tablón de Anuncios**.
    * **Certificados:** Incluye las autoridades de certificación del proceso electoral.

:::info
**Seguridad:** Seleccione **Cifrar con Contraseña** para proteger el archivo con una contraseña. Si selecciona **Tablón de Anuncios**, **Informes** o **Aplicaciones**, el archivo siempre se protege con una contraseña. Si también selecciona **Incluir Votantes** o **Archivos S3**, **Cifrar con Contraseña** añade los campos secretos descifrados de los votantes a la exportación, siempre que su cuenta tenga permiso para leerlos.
:::

![Contraseña de la exportación](../../../../../../docs/02-election_managers/01-tutorials/assets/export_password_display.png)

1.  Haga clic en **Exportar**. Una exportación protegida con contraseña es un archivo `.ezip`. Otra exportación es un archivo `.zip`.
2.  **Guardar contraseña:** En una exportación protegida con contraseña, un cuadro de diálogo muestra una contraseña de descifrado única. Cópiela y guárdela de forma segura. La necesitará más adelante para importar el archivo o para descomprimirlo manualmente.

## Importar un proceso electoral {#importing-an-election-event}

Puede importar un proceso electoral desde un archivo `.json`, desde una exportación `.zip` o desde una exportación protegida con contraseña `.ezip`. Para ver los pasos completos, consulte [Crear el proceso electoral](../03-procedures/02-event.md#import-an-election-event-alternative).

:::info
Solo puede importar procesos electorales exportados desde la misma versión principal, con la misma versión secundaria o una inferior.
Por ejemplo, si tiene instalada la versión **10.1.0**, solo puede importar procesos electorales de las versiones **10.1.0** o **10.0.0**.
:::

![Opciones para importar un proceso electoral](../../../../../../docs/02-election_managers/01-tutorials/assets/import_election.png)

1.  En el menú lateral, haga clic en el icono **+** junto a **Procesos Electorales**, o haga clic en **Crear un Evento Electoral** al final del árbol de procesos electorales para abrir el mismo menú.
2.  Haga clic en **Importar Evento Electoral**.

![Subida del archivo de importación](../../../../../../docs/02-election_managers/01-tutorials/assets/import_file_upload.png)
