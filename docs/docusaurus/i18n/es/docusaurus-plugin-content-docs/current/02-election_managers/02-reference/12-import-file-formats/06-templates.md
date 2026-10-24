---
id: import_format_templates
title: Archivo de plantillas
sidebar_position: 6
description: "El formato del archivo de plantillas: las diez columnas, el JSON de la plantilla, los tipos y métodos de comunicación permitidos, y lo que hace la importación."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

El archivo de plantillas contiene las plantillas de un inquilino: los documentos, los correos electrónicos y los mensajes SMS que la plataforma genera para los informes, las credenciales, los recibos de voto y otros usos. Usted crea el archivo con **Exportar** en la página **Plantillas**. Lo importa con **Importar** en la misma página. No cree el archivo a mano.

Use la importación para copiar las plantillas a otro inquilino, o para restaurar una copia de seguridad. Para ver la pantalla, consulte [Plantillas](../02-election-event/13-02-election_management_election-event_templates.md#exporting-and-importing-templates).

:::caution PRECAUCIÓN: inquilino en el archivo de plantillas
La importación lee el ID del inquilino de cada fila. No usa el inquilino en el que ha iniciado sesión. Importe el archivo solo en el inquilino donde lo exportó.
:::

## Campos {#fields}

El archivo tiene una fila de encabezado y diez columnas. El orden de las columnas es fijo. La importación lee las columnas por posición, no por nombre.

| Columna | Posición | Obligatoria | Valores permitidos o formato | Valor predeterminado | Significado | Ejemplo |
| --- | --- | --- | --- | --- | --- | --- |
| `alias` | 1 | Obligatoria | Texto | Ninguno | El alias de la plantilla. Los informes y otras funciones encuentran la plantilla por su alias. | `ballot-receipt-1` |
| `tenant_id` | 2 | Obligatoria | Un UUID versión 4 | Ninguno | El inquilino de la plantilla. Una fila con un valor no válido se omite. | `90505c8a-23a9-4cdf-a26b-4e19f6a097d5` |
| `template` | 3 | Obligatoria | Un objeto JSON, vea más abajo | `null` | El contenido de la plantilla. | Vea más abajo. |
| `created_by` | 4 | Obligatoria | Texto | Ninguno | El usuario que hizo la plantilla. | `admin@example.com` |
| `labels` | 5 | Opcional | Texto (JSON) | Vacío | Datos adicionales. Puede dejarlo vacío. | `{}` |
| `annotations` | 6 | Opcional | Texto (JSON) | Vacío | Datos adicionales. Puede dejarlo vacío. | `{}` |
| `created_at` | 7 | Opcional | Fecha y hora | No se usa | La importación ignora el valor. Pone la fecha de la importación. | `2026-10-01 09:30:00 +00:00` |
| `updated_at` | 8 | Opcional | Fecha y hora | No se usa | La importación ignora el valor. Pone la fecha de la importación. | `2026-10-01 09:30:00 +00:00` |
| `communication_method` | 9 | Obligatoria | `EMAIL`, `SMS` o `DOCUMENT` | Ninguno | El método de comunicación de la plantilla. | `EMAIL` |
| `type` | 10 | Obligatoria | Uno de los tipos de la tabla siguiente | Ninguno | El **Tipo de Plantilla**. | `BALLOT_RECEIPT` |

### Valores de `type` {#values-of-type}

| Valor | Etiqueta en el portal de administración |
| --- | --- |
| `CREDENTIALS` | Credenciales |
| `INITIALIZATION_REPORT` | Informe de Inicialización |
| `ELECTORAL_RESULTS` | Resultados Electorales |
| `BALLOT_IMAGES` | Imágenes de Boletas |
| `BALLOT_RECEIPT` | Recibo de Voto |
| `ACTIVITY_LOGS` | Registros de Actividades |
| `MANUAL_VERIFICATION` | Verificación Manual |
| `PARTICIPATION_REPORT` | Informe de Participación |

La importación no comprueba el valor de `type`. Use solo los valores de la tabla.

La importación no comprueba `type` ni `communication_method`. Un valor que no está en las tablas anteriores se importa, pero el portal de administración no usa la plantilla.

### Valores de `communication_method` {#values-of-communication_method}

| Valor | Significado |
| --- | --- |
| `EMAIL` | La plantilla genera un correo electrónico. |
| `SMS` | La plantilla genera un mensaje SMS. |
| `DOCUMENT` | La plantilla genera un documento (PDF). |

Cuando crea una plantilla en el portal de administración, el portal escribe `EMAIL` en esta columna, aunque seleccione otros métodos. Los métodos que selecciona están en el JSON de la plantilla (`selected_methods`).

### La columna `template` {#the-column-template}

`template` es un objeto JSON. Estas son las claves que escribe el portal de administración:

| Clave | Tipo | Significado |
| --- | --- | --- |
| `alias` | Texto | El alias de la plantilla. Es el mismo que la columna `alias`. |
| `name` | Texto | El nombre de la plantilla. |
| `selected_methods` | Objeto | Tres claves, `EMAIL`, `SMS` y `DOCUMENT`. Cada valor es `true` o `false`. Indica qué partes tiene la plantilla. |
| `email` | Objeto | La parte del correo electrónico. Tiene `subject`, `plaintext_body` y `html_body`. |
| `sms` | Objeto | La parte del SMS. Tiene la clave `message`. |
| `document` | Texto | La parte del documento: el texto del documento, como plantilla HTML. |
| `pdf_options` | Objeto | La configuración PDF del documento, por ejemplo `landscape`, `print_background`, `scale`, `paper_width`, `paper_height`, `margin_top`, `margin_bottom`, `margin_left`, `margin_right`, `page_ranges`, `header_template`, `footer_template`. |
| `report_options` | Objeto | Los límites del informe. Tiene `max_items_per_report` y `max_threads`. |

Los textos del correo electrónico, del SMS y del documento pueden contener variables. La plataforma las sustituye cuando genera el mensaje o el documento.

Para un mensaje a un votante, la plataforma rellena estas variables: `user.first_name`, `user.last_name`, `user.username`, `user.email`, `user.attributes`, `tenant_id`, y cada atributo del votante como `user.<nombre del atributo>`. Si el mensaje pertenece a un proceso electoral, también recibe `election_event.id`, `election_event.name` y `vote_url`.

Ejemplo del valor de `template`:

```json
{
  "alias": "ballot-receipt-1",
  "name": "Ballot receipt 1",
  "selected_methods": {"EMAIL": true, "SMS": false, "DOCUMENT": true},
  "email": {"subject": "Your ballot receipt", "plaintext_body": "Your receipt is attached.", "html_body": "<p>Your receipt is attached.</p>"},
  "sms": {"message": ""},
  "document": "<html><body><h1>Receipt</h1></body></html>",
  "pdf_options": {"landscape": false, "print_background": true},
  "report_options": {"max_items_per_report": 1000, "max_threads": 4}
}
```

## Reglas {#rules}

| Regla | Valor |
| --- | --- |
| Tipo de archivo | `.csv`. El separador es una coma. |
| Encabezado | La exportación escribe una fila de encabezado. La importación no la salta. La omite porque el valor `tenant_id` no es un UUID válido. Un archivo sin fila de encabezado también funciona. |
| Codificación | UTF-8. |
| Comillas | Las columnas JSON van entre comillas dobles. Cada comilla que hay dentro se duplica. No cambie esto. |
| Orden de las columnas | Fijo. |
| Alias duplicados | La importación no los comprueba. Si ya existe una plantilla con el mismo alias, la importación añade una segunda. |
| Tamaño | El tamaño está limitado por la configuración de subida de archivos de la instalación. |
| Exportación vacía | Si el inquilino no tiene plantillas, la exportación falla con el error `No templates available to write`. |

:::caution PRECAUCIÓN: alias de las plantillas
Use un alias distinto para cada plantilla. Si dos plantillas tienen el mismo alias, el portal de administración puede usar cualquiera de las dos.
:::

## Archivo de ejemplo {#example-file}

```csv
alias,tenant_id,template,created_by,labels,annotations,created_at,updated_at,communication_method,type
ballot-receipt-1,90505c8a-23a9-4cdf-a26b-4e19f6a097d5,"{""alias"":""ballot-receipt-1"",""name"":""Ballot receipt 1"",""selected_methods"":{""EMAIL"":false,""SMS"":false,""DOCUMENT"":true},""document"":""<html><body><h1>Receipt</h1></body></html>""}",admin@example.com,{},{},2026-10-01 09:30:00 +00:00,2026-10-01 09:30:00 +00:00,DOCUMENT,BALLOT_RECEIPT
```

## Qué hace la importación {#what-the-import-does}

1. El portal de administración sube el archivo y pide la **Verificación de Integridad (SHA 256)**. La tarea comprueba el hash si lo escribió.
2. La tarea **Importar Plantillas** empieza. Mire el widget de la tarea o la pestaña **Tareas** para ver el resultado. La tarea no vuelve a intentarlo si falla.
3. Para cada fila, la tarea lee las diez columnas. Omite una fila si `tenant_id` no es un UUID válido.
4. La tarea añade todas las demás filas en una única transacción de la base de datos. Si la base de datos rechaza una fila, la tarea no añade nada y el estado es **Failed**.
5. Cada fila se convierte en una plantilla nueva. La importación nunca cambia ni elimina una plantilla que ya existe.
6. Las fechas `created_at` y `updated_at` son la fecha de la importación.

Si el JSON de `template` no es válido, la importación guarda `null` y no muestra ningún error. La plantilla entonces no funciona. Abra la plantilla en el portal de administración para comprobarla.

Efecto conocido: la importación guarda `labels` y `annotations` como un texto JSON y no como un objeto JSON.

### Mensajes y errores {#messages-and-errors}

| Mensaje | Dónde aparece | Significado y acción |
| --- | --- | --- |
| `Failed to verify the integrity: Hash of voters file: ... does not match with the input hash: ...` | Registro de la tarea | El hash que escribió no es el hash del archivo. (El mensaje dice "voters file" también para este archivo.) |
| `Error reading CSV record: ...` | Registro de la tarea | Una fila tiene comillas incorrectas. Exporte el archivo de nuevo. |
| `Error inserting template: ...` | Registro de la tarea | La base de datos rechazó una fila. |
| `document not found` | Registro de la tarea | La subida falló. Suba el archivo de nuevo. |

## Páginas relacionadas {#related-pages}

- [Plantillas](../02-election-event/13-02-election_management_election-event_templates.md): la página **Plantillas**, **Importar** y **Exportar**.
- [Informes](../02-election-event/13-01-election_management_election-event_reports.md): los informes usan una plantilla.
- [Formatos de archivos de importación](index.md): todos los archivos de importación.
