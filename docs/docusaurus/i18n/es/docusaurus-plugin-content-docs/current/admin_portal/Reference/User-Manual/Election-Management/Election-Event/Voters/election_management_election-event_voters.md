---
id: election_management_election_event_voters
title: Votantes
description: "La pestaña Votantes es la lista de votantes del proceso electoral. Úsela para importar, añadir, cambiar, borrar y exportar votantes, y para enviarles mensajes. Para el procedimiento completo, consulte Añadir los votantes."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Votantes** es la lista de votantes del proceso electoral. Úsela para importar, añadir,
cambiar, borrar y exportar votantes, y para enviarles mensajes. Para el procedimiento completo,
consulte [Añadir los votantes](../../../../../procedures/03-voters.md).

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, haga clic en el proceso electoral.
2. Haga clic en la pestaña **Votantes**.

La pestaña se muestra con el permiso **Ver Votantes del Evento Electoral**. Cada botón necesita su
propio permiso, como muestran las tablas de abajo.

## Barra de herramientas {#toolbar}

| Botón | Uso | Permiso |
| --- | --- | --- |
| **Columnas** | Selecciona las columnas de la lista. | **Ver Columnas de Votantes del Evento Electoral** |
| **Añadir filtro** | Añade un filtro, por ejemplo **Área**, **Habilitado** o **Voted** (ha votado). | **Ver Filtros de Votantes del Evento Electoral** |
| **Filtros Personalizados** | Aplica un filtro guardado. Solo se muestra si el proceso electoral tiene filtros personalizados en su pestaña **Datos**. | **Ver Filtros de Votantes del Evento Electoral** |
| **Añadir** | Abre el formulario para crear un votante. | **Crear Votante** |
| **Importar** | Abre el panel **Importar votantes**. | **Importar Votante** |
| **Exportar** | Exporta la lista de votantes. La tarea "Exportar votantes" crea el archivo. | **Exportar Votante** |
| **Enviar** | Abre el formulario **Enviar Notificación** para todos los votantes. | **Enviar Notificación** |

Cuando la lista está vacía, la pestaña muestra "Ningún votante aún." con los botones **Crear
votante** e **Importar**.

## Columnas {#columns}

| Columna | Contenido |
| --- | --- |
| **Id** | El identificador del votante. |
| **Email Verificado** | Si la dirección de correo electrónico del votante está verificada. |
| **Habilitado** | Si el votante puede iniciar sesión. |
| Campos del perfil | Por ejemplo **Nombre de Usuario**, **Email**, **Nombre** y **Apellido**. Los campos dependen del perfil de usuario del proceso electoral. |
| **Área** | El área del votante. `-` significa que el votante no tiene área. |
| **Voted** | Si el votante ha emitido un voto. |
| **Actions** (acciones) | El menú de acciones del votante. |

## Acciones de un votante {#actions-of-a-voter}

| Acción | Uso | Permiso |
| --- | --- | --- |
| **Enviar** | Envía un mensaje a este votante. | **Enviar Notificación** |
| **Editar** | Cambia los datos del votante. | **Editar Votante**, o **Editar email/teléfono de los votantes** |
| **Borrar** | Borra el votante, después de la pregunta "¿Estás seguro que quieres borrar este votante?". | **Eliminar Votante** |
| **Verificar manualmente** | Crea un PDF con un enlace en código QR. El votante lo usa para iniciar sesión sin la comprobación de identidad en línea. El votante necesita una dirección de correo electrónico o un número de teléfono. | **Verificar Votante Manualmente** |
| **Cambiar contraseña** | Establece una contraseña nueva para el votante. | **Cambiar la Contraseña del Votante** |
| **Registros del usuario** | Muestra las entradas del registro de este votante. | **Ver Registros de Votantes del Evento Electoral** |

Para actuar sobre varios votantes, selecciónelos en la lista. Después haga clic en **Enviar** o en
**Borrar** encima de la lista.

## Formulario del votante {#voter-form}

| Campo | Uso |
| --- | --- |
| Campos del perfil | Los campos del perfil de usuario, por ejemplo **Nombre de Usuario**. **Nombre de Usuario** siempre es obligatorio. |
| **Habilitado \*** | Si el votante puede iniciar sesión. |
| **Área \*** | El área del votante. Escriba tres o más letras para buscar. |
| **Contraseña:** y **Repetir Contraseña:** | La contraseña del votante. |
| **Temporal** | Si está activado, el votante debe cambiar la contraseña en el próximo inicio de sesión. Está activado por defecto. |
| **Guardar** | Guarda el votante. |

## Archivo de importación {#import-file}

El archivo de importación es un archivo CSV con una fila de encabezado. Las columnas están en
[Añadir los votantes](../../../../../procedures/03-voters.md#1-prepare-the-voter-file). El panel de
importación también tiene **Verificación de Integridad (SHA 256)**. La importación se ejecuta como
la tarea **Importar usuarios** en la pestaña **Tareas**.

## Formulario Enviar Notificación {#send-notification-form}

| Campo | Uso |
| --- | --- |
| **Audiencia** | **Todos**, **Los que no votaron**, **Los que ya votaron** o los votantes seleccionados. |
| **Calendario** | **Enviar ahora**, o "Fecha y hora de comienzo de envío". |
| **Idiomas** | Los idiomas del mensaje. |
| **Método de Comunicación** | **Método de Plantillas** (**Email** o **SMS**), **Tipo de Comunicación** y **Alias de la Plantilla**. |
| **Enviar Notificación** | Envía o programa el mensaje. |

## Conexión con el sistema externo {#external-system-connection}

La anotación `datafix:voterview_request` del proceso electoral contiene la `url` del sistema
externo, las credenciales `usr` y `psw` y el código `county_mun`.

El campo opcional `transport_policy` de esa anotación indica qué esquemas de URL se aceptan:

- `https-only` (predeterminado): la `url` debe usar `https`.
- `allow-plaintext`: la `url` puede usar `https` o `http`. Úselo solo para endpoints de prueba.

Por ejemplo:

```json
{
  "url": "https://voters.example.org/service.asmx",
  "usr": "user",
  "psw": "password",
  "county_mun": "county",
  "transport_policy": "https-only"
}
```

Una solicitud cuya `url` no cumple la política de transporte no se envía. No se siguen las
redirecciones del sistema externo y se rechazan las respuestas de más de 1 MiB.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| Se muestra "Importación de Votantes lanzada en segundo plano con éxito.", pero no se muestra ningún votante. | La importación se ejecuta como una tarea. Abra la pestaña **Tareas** y compruebe la tarea **Importar usuarios**. |
| "Error Importando Votantes." | La importación no empezó. Inténtelo de nuevo. |
| "Este votante no puede ser verificado manualmente porque no tiene una dirección de correo electrónico o un número de teléfono asociado a él." | Añada una dirección de correo electrónico o un número de teléfono al votante. |
| "Error editando votante" | Compruebe los campos y guarde de nuevo. |
| "Error enviando la notificación: ..." | Lea el error. Compruebe que existe una plantilla del tipo y el método seleccionados. |
| Un botón no está. | Su rol no tiene su permiso. Pídalo al administrador del inquilino. |

## Páginas relacionadas {#related-pages}

- [Añadir los votantes](../../../../../procedures/03-voters.md)
- [Elección: Votantes](../../Election/Voters/election_management_election_voters.md)
- [Áreas](../Areas/election_management_election-event_areas.md)
- [Aprobaciones](../Approvals/election_management_election-event_approvals.md)
- [Plantillas](../../../Templates/admin_portal_reference_user-manual_templates.md)
