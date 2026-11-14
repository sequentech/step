---
id: settings_previews
title: Vistas previas externas
description: "La tabla Vistas previas externas muestra un registro de las URL de vista previa generadas mediante peticiones a la API externa."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


La tabla de vistas previas externas ofrece un registro de las URL de vista previa generadas mediante peticiones externas a la API.
Esta página sirve de referencia para que los administradores sigan el historial de envíos, verifiquen la identidad de quien hace la petición y accedan al documento de estilo de la papeleta de origen que se usó en cada generación.

La tabla tiene tres columnas principales:

* **Solicitado por:** Identifica el usuario o la cuenta del sistema que inició la petición de vista previa.
* **URL:** Ofrece un enlace directo para ver la papeleta generada en el portal de votación.
* **Documento**: Contiene un botón de descarga para obtener el archivo de estilo de la papeleta sin procesar.

### Acciones principales {#key-actions}

#### 1. Ver la vista previa: {#1-viewing-the-preview}
Al hacer clic en el enlace de la columna URL se abre el portal de votación, cargado con los datos del proceso electoral contenidos en el documento de estilo de la papeleta de origen. Así puede verificar visualmente el diseño tal como lo verá el votante.

#### 2. Descargar el documento de estilo de la papeleta: {#2-downloading-the-ballot-style-document}
En la columna Documento, haga clic en el botón de descarga para obtener el archivo de estilo de la papeleta sin procesar.
Se usa para verificar la integridad de los datos de una vista previa, ya que descarga la versión concreta del archivo que se usó para generar esa instancia de vista previa.
