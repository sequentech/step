---
id: election_management_election_event_data
title: Datos
description: "La pestaña Datos contiene la configuración del proceso electoral: sus nombres, idiomas, canales de votación, el aspecto del portal de votación y las políticas."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Datos** contiene la configuración del proceso electoral: sus nombres, idiomas, canales
de votación, el aspecto del portal de votación y las políticas. También tiene la exportación del
proceso electoral y la importación de candidatos.

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, haga clic en el proceso electoral.
2. Haga clic en la pestaña **Datos**.

La pestaña solo se muestra si su rol tiene el permiso **Ver Datos del Evento Electoral**. Para
guardar cambios, su rol necesita el permiso **Editar Evento Electoral**. La pestaña está oculta
cuando **Estado de Confinamiento** es **Confinado**.

## Secciones {#sections}

La pestaña tiene estas secciones. Haga clic en una sección para abrirla. Haga clic en **Guardar**
en la parte inferior para conservar los cambios.

| Sección | Campos | Uso |
| --- | --- | --- |
| **General** | Una pestaña para cada idioma, con **Nombre**, **Alias** y **Descripción** | Los textos del proceso electoral en el portal de votación. |
| **Idiomas** | Una casilla para cada idioma y una opción **Por defecto** | Los idiomas del proceso electoral. Solo se muestran los idiomas activados en **Configuración** > **IDIOMAS**. |
| **Diseño de la Papeleta** | **Saltar pantalla para escoger elección**, **Mostrar perfil de usuario**, el orden de las elecciones, **Reordenar elecciones**, **Logo URL**, **URL de redirección al finalizar**, **CSS personalizado** | El aspecto y el comportamiento del portal de votación. Seleccione el orden personalizado para mostrar **Reordenar elecciones**. |
| **Canales de Voto Permitidos** | **En línea**, **Kiosco** | Los canales de votación del proceso electoral. |
| **Prefixo de URLs personalizados** | **Inicio de sesión**, **Inscripción** y un tercer prefijo para el inicio de sesión externo | Direcciones cortas para las páginas de inicio de sesión y de inscripción. |
| **Materiales de Soporte** | **Materiales de Soporte activados**, **Titulo** y **Subtitulo** para cada idioma, y la lista de materiales | Documentos que los votantes pueden abrir en el portal de votación. |
| **Voting Portal Countdown Policy** (configuraciones avanzadas) | **Política de cifrado de concurso**, **Estado de Confinamiento**, **Política de cuenta regresiva del portal de votación** con sus dos tiempos en segundos, **Política de Firma de Votantes**, **Filtros personalizados**, **Autenticación de votantes** | Las políticas del proceso electoral. La tabla de abajo da los valores. |

### Configuraciones avanzadas {#advanced-configurations}

| Campo | Valores | Significado |
| --- | --- | --- |
| **Política de cifrado de concurso** | **Concurso único**, **Varios concursos** | Cómo cifra el portal de votación las preguntas de una papeleta. El valor por defecto es **Concurso único**. |
| **Estado de Confinamiento** | **No Confinado**, **Confinado** | **Confinado** oculta la mayoría de las pestañas y detiene la generación de publicaciones. El valor por defecto es **No Confinado**. |
| **Política de cuenta regresiva del portal de votación** | **Sin cuenta regresiva**, **Cuenta regresiva**, **Cuenta regresiva con alerta** | Si el portal de votación muestra una cuenta regresiva antes de que termine la sesión del votante. Los dos campos de tiempo solo están disponibles con una cuenta regresiva. |
| **Política de Firma de Votantes** | **Sin firma**, **Con firma** | La política de firma de las papeletas que emiten los votantes. El valor por defecto es **Sin firma**. |
| **Filtros personalizados** | Un editor JSON | Filtros guardados para la pestaña **Votantes**. Se muestran en el menú **Filtros Personalizados** de esa pestaña. |
| **Inscripción** (en **Autenticación de votantes**) | **Habilitado**, **Deshabilitado** | Si los votantes pueden inscribirse. Las solicitudes de inscripción se muestran en la pestaña **Aprobaciones**. |
| **OTP** (en **Autenticación de votantes**) | **Habilitado**, **Deshabilitado** | La política de código de un solo uso (OTP) del inicio de sesión de los votantes. |

:::caution PRECAUCIÓN
No seleccione **Confinado** durante la preparación del proceso electoral. Oculta la pestaña
**Datos** y la pestaña **Eventos Programados**, por lo que no puede deshacerlo desde el portal de
administración. Contacte con el soporte de Sequent para deshacerlo.
:::

## Botones {#buttons}

| Botón | Uso |
| --- | --- |
| **Exportar** | Abre la ventana **Exportar Evento Electoral**. Consulte [Exportar el proceso electoral](#export-the-election-event). |
| **Importar Candidatos** | Abre un panel para importar candidatos desde un archivo. El archivo debe tener el formato fijo de una integración externa específica. No es una importación de uso general. Para añadir candidatos, use el árbol del evento electoral o importe un evento electoral. En la versión 9.0 el título del panel muestra un código de texto, no un título traducido. |
| **Guardar** | Guarda los cambios. |

## Exportar el proceso electoral {#export-the-election-event}

1. Haga clic en **Exportar**. Se abre la ventana **Exportar Evento Electoral**.
2. Seleccione el contenido que quiere incluir. La tabla da las opciones.
3. Haga clic en **Exportar**.
4. Si la exportación está cifrada, la ventana **Contraseña** muestra "Contraseña para descifrar el
   archivo:" y la contraseña. Copie la contraseña y guárdela en un lugar seguro.
5. Cuando la tarea "Exportar evento electoral" esté completada, haga clic en **Descargar Archivo**.

| Opción | Contenido |
| --- | --- |
| **Cifrar con Contraseña** | Cifra el archivo de exportación. El portal de administración crea la contraseña. |
| **Incluir Votantes** | Los votantes. |
| **Registros de Actividad** | Los registros del proceso electoral. |
| **Tablón de Anuncios** | El tablón de anuncios. Esta opción activa **Cifrar con Contraseña**. |
| **Publicaciones** | Las publicaciones de la papeleta. |
| **Archivos S3** | Los archivos guardados, por ejemplo imágenes y documentos. |
| **Eventos Programados** | Los eventos programados. |
| **Informes** | Las configuraciones de los informes. Esta opción activa **Cifrar con Contraseña**. |
| **Aplicaciones** | Las solicitudes de inscripción. Esta opción activa **Cifrar con Contraseña**. |
| **Recuento** | Los datos del recuento. Esta opción también selecciona **Tablón de Anuncios**. |

**Resultado esperado:** tiene el archivo de exportación. Para importarlo más tarde, consulte
[Crear el proceso electoral](../../../../../procedures/02-event.md#import-an-election-event-alternative).

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| La pestaña **Datos** no está. | Su rol no tiene el permiso **Ver Datos del Evento Electoral**, o el proceso electoral está confinado. |
| El botón **Guardar** no está. | Su rol no tiene el permiso **Editar Evento Electoral**. |
| Un idioma no está en **Idiomas**. | Active el idioma en **Configuración** > **IDIOMAS**. |
| "Error al exportar el Evento Electoral" | La exportación no empezó. Inténtelo de nuevo. Si el error continúa, contacte con el soporte de Sequent. |

## Páginas relacionadas {#related-pages}

- [Crear el proceso electoral](../../../../../procedures/02-event.md)
- [Localización](../Localization/election_management_election-event_localization.md)
- [Elección: Datos](../../Election/Data/election_management_election_data.md)
- [Tareas](../Tasks/election_management_election-event_tasks.md)
