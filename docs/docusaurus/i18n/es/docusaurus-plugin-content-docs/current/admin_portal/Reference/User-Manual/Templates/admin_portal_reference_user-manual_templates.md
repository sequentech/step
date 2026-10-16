---
id: admin_portal_reference_user_manual_templates
title: Plantillas
description: "La página Plantillas contiene las plantillas del inquilino. Una plantilla es el texto de un mensaje (correo electrónico o mensaje de texto) o de un documento (por ejemplo, un informe o un comprobante)."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La página **Plantillas** contiene las plantillas del inquilino. Una plantilla es el texto de un
mensaje (correo electrónico o mensaje de texto) o de un documento (por ejemplo, un informe o un
comprobante). La plataforma usa una plantilla cuando envía un mensaje a los votantes o crea un
documento. Si no hay plantilla para un documento, la plataforma usa su plantilla predeterminada.

## Abrir la página {#open-the-page}

En el menú de la izquierda, haga clic en **Plantillas**. La página tiene el encabezado
**Plantillas** y "Listado de plantillas".

El elemento de menú **Plantillas** necesita el permiso **Ver plantillas**. Sin él, la página
muestra "No tienes permisos para acceder a las Plantillas.". Para crear y cambiar plantillas, su
rol necesita **Editar Plantilla de Comunicación**.

## Lista {#list}

Cuando la lista está vacía, la página muestra "No hay plantillas" con **Crear Plantilla** e
**Importar**. Cuando hay plantillas, la barra de herramientas tiene **Añadir**, **Importar** y
**Exportar**.

| Columna | Contenido |
| --- | --- |
| **Alias** | El nombre corto único de la plantilla. Las otras pantallas seleccionan la plantilla por este alias. |
| **Name** (nombre) | El nombre de la plantilla. |
| **Type** (tipo) | El tipo de la plantilla. |
| **Actions** (acciones) | Iconos de editar y de borrar. |

## Formulario de plantilla {#template-form}

El formulario es **Crear una Plantilla** o **Editar una Plantilla**.

| Campo | Uso |
| --- | --- |
| **Alias de la Plantilla** | El nombre corto único de la plantilla. |
| **Nombre de la Plantilla** | El nombre de la plantilla. |
| **Tipo** | Para qué es la plantilla, por ejemplo **Credenciales**, **Recibo de Voto**, **Resultados Electorales**, **Informe Estadístico**, **Informe de Inicialización**, **Registros de Actividades**, **Registros de Auditoría** o **Verificar manualmente al votante**. |
| **Elige métodos** | **Email**, **SMS** y **Documento**. Seleccione uno o más. Cada método añade su propia sección. |
| **Email** | **Asunto del Email**, y el cuerpo en **Cuerpo de Texto Enriquecido** y **Cuerpo de Texto Plano**. |
| **Mensaje SMS** | El texto del mensaje de texto. |
| **Documento** | El cuerpo del documento. |
| **Opciones de PDF** | Las opciones del archivo PDF, en formato JSON. |
| **Opciones de Informe** | Las opciones del informe, en formato JSON. |
| **Guardar** | Guarda la plantilla. |

Los textos pueden contener variables entre llaves dobles, por ejemplo `{{...}}`. La plataforma
reemplaza cada variable por su valor cuando crea el mensaje o el documento. Use **Vista previa**
en la pestaña **Informe** de un proceso electoral para comprobar una plantilla de documento con
datos de ejemplo.

## Crear una plantilla {#create-a-template}

1. Haga clic en **Añadir**. Si la lista está vacía, haga clic en **Crear Plantilla**.
2. Escriba el **Alias de la Plantilla** y el **Nombre de la Plantilla**.
3. Seleccione el **Tipo**.
4. En **Elige métodos**, seleccione los métodos.
5. Escriba los textos de cada método.
6. Haga clic en **Guardar**.

**Resultado esperado:** se muestra el mensaje "Plantilla creada".

## Importar plantillas {#import-templates}

1. Haga clic en **Importar**. Se abre el panel **Importar Plantillas**.
2. Escriba el hash SHA-256 del archivo en **Verificación de Integridad (SHA 256)**.
3. Suba el archivo y haga clic en **Importar**.

Importe solo un archivo que hizo con **Exportar**. No haga el archivo a mano. La importación añade
cada fila como una plantilla nueva. El archivo tiene una fila de encabezado y diez columnas en este
orden:

```csv
alias,tenant_id,template,created_by,labels,annotations,created_at,updated_at,communication_method,type
```

:::caution PRECAUCIÓN
La importación usa el ID de tenant de la columna `tenant_id` de cada fila, no el tenant en el que
ha iniciado sesión. Las filas con un ID de tenant no válido se omiten. Importe un archivo solo en
el tenant donde lo exportó.
:::

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| "Error creando plantilla" o "Error actualizando plantilla" | Compruebe el alias y los campos obligatorios, y vuelva a guardar. |
| "Hashes don't match. Integrity check failure." (los hashes no coinciden) | El archivo no es el archivo que espera. Vuelva a obtener el archivo de su origen. |
| Un documento no usa su plantilla. | Seleccione la plantilla en un informe de la pestaña **Informe**. Consulte [Asociación de valores a plantillas](../../../Tutorials/Reports-and-Templates/value_to_template_association.md). |

## Páginas relacionadas {#related-pages}

- [Informes y plantillas](../../../Tutorials/Reports-and-Templates/reports_and_templates.md)
- [Proceso electoral: Informes](../Election-Management/Election-Event/Reports/election_management_election-event_reports.md)
- [Proceso electoral: Votantes](../Election-Management/Election-Event/Voters/election_management_election-event_voters.md)
- [Plantillas (obsoleto)](../Settings/templates-deprecated/admin_portal_reference_user-manual_settings_templates-deprecated.md)
