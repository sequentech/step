---
id: admin_portal_reference_user_manual_users_and_roles_permissions
title: Permisos
description: "Un permiso permite a un usuario ver un elemento del menú, una pestaña o un botón, o hacer una acción. Los permisos se dan a los roles en la pestaña Roles, y los roles a los usuarios en la pestaña Usuarios."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Un permiso permite a un usuario ver un elemento del menú, una pestaña o un botón, o hacer una
acción. Los permisos se dan a los roles en la pestaña **Roles**, y los roles a los usuarios en
la pestaña **Usuarios**. Esta página enumera los permisos de la versión 9.0 con la etiqueta que
muestra la pestaña **Roles**.

## Cómo funcionan los permisos {#how-permissions-work}

- Un usuario recibe los permisos de todos sus roles.
- Una pestaña o un botón que necesita un permiso no se muestra sin él. La página de referencia
  de cada pantalla indica los permisos de esa pantalla.
- Muchas pestañas necesitan dos permisos: uno para ver la pestaña, por ejemplo **Ver Claves del
  Evento Electoral**, y otro para los datos, por ejemplo **Administrar Ceremonia de Claves**.
- Los permisos **Ver configuración de ...** de las pestañas de **Configuración** existen, pero
  la versión 9.0 no los usa. La página **Configuración** necesita **Ver configuración** y
  **Editar Inquilino**.
- Algunas acciones son sensibles. Aunque tenga el permiso, el portal de administración le pide
  de nuevo su contraseña. Consulte [Antes de empezar](../../../../00-before-you-start.md#sensitive-actions).
- Las etiquetas de permiso son una función diferente. Limitan las elecciones que un usuario
  puede ver. Consulte [Etiquetas de permiso](../../../../Tutorials/admin_portal_tutorials_permission-labels.md).

## Cambiar los permisos de un rol {#change-the-permissions-of-a-role}

1. En el menú de la izquierda, haga clic en **Usuarios y Roles**.
2. Haga clic en la pestaña **Roles**.
3. Haga clic en el icono de editar del rol.
4. Seleccione o desmarque **Active** (activo) para el permiso.

**Resultado esperado:** se muestra el mensaje "Permiso editado". Consulte
[Roles](../Roles/admin_portal_reference_user-manual_users-and-roles_roles.md).

## Lista de permisos {#list-of-permissions}

La primera columna es la etiqueta en la pestaña **Roles**. La segunda columna es el nombre del
permiso en la plataforma. El soporte de Sequent usa este nombre.

| Etiqueta | Nombre |
| --- | --- |
| **Administración** | `admin-user` |
| **Vista del Panel de Administración** | `admin-dashboard-view` |
| **Exportación de Aplicaciones** | `application-export` |
| **Importación de Aplicaciones** | `application-import` |
| **Crear Inquilino** | `tenant-create` |
| **Leer Inquilino** | `tenant-read` |
| **Editar Inquilino** | `tenant-write` |
| **Crear Evento Electoral** | `election-event-create` |
| **Leer Evento Electoral** | `election-event-read` |
| **Editar Evento Electoral** | `election-event-write` |
| **Eliminar Evento Electoral** | `election-event-delete` |
| **Crear Votante** | `voter-create` |
| **Leer Votante** | `voter-read` |
| **Editar Votante** | `voter-write` |
| **Crear Usuario** | `user-create` |
| **Leer Usuario** | `user-read` |
| **Editar Usuario** | `user-write` |
| **Crear Permiso de Usuario** | `user-permission-create` |
| **Leer Permiso de Usuario** | `user-permission-read` |
| **Editar Permiso de Usuario** | `user-permission-write` |
| **Crear Rol** | `role-create` |
| **Leer Rol** | `role-read` |
| **Editar Rol** | `role-write` |
| **Asignar Rol** | `role-assign` |
| **Crear Plantilla de Comunicación** | `communication-template-create` |
| **Leer Plantilla de Comunicación** | `communication-template-read` |
| **Editar Plantilla de Comunicación** | `communication-template-write` |
| **Leer Notificación** | `notification-read` |
| **Editar Notificación** | `notification-write` |
| **Enviar Notificación** | `notification-send` |
| **Leer Área** | `area-read` |
| **Editar Área** | `area-write` |
| **Editar Estado de Elección** | `election-state-write` |
| **Crear Tipo de Elección** | `election-type-create` |
| **Leer Tipo de Elección** | `election-type-read` |
| **Editar Tipo de Elección** | `election-type-write` |
| **Leer Canal de Votación** | `voting-channel-read` |
| **Editar Canal de Votación** | `voting-channel-write` |
| **Crear Fideicomisario** | `trustee-create` |
| **Leer Fideicomisario** | `trustee-read` |
| **Editar Fideicomisario** | `trustee-write` |
| **Leer Recuento** | `tally-read` |
| **Iniciar Recuento** | `tally-start` |
| **Editar Recuento** | `tally-write` |
| **Leer Resultados de Recuento** | `tally-results-read` |
| **Leer Publicación** | `publish-read` |
| **Editar Publicación** | `publish-write` |
| **Leer Registros** | `logs-read` |
| **Leer la Ejecución de Tareas** | `tasks-read` |
| **Leer Claves** | `keys-read` |
| **Subir Documentos** | `document-upload` |
| **Descargar Documentos** | `document-download` |
| **Crear Acta de Recuento** | `tally-sheet-create` |
| **Ceremonia de Fideicomisario** | `trustee-ceremony` |
| **Publicar Acta de Recuento** | `tally-sheet-publish` |
| **Ver Acta de Recuento** | `tally-sheet-view` |
| **Administrar Ceremonia de Claves** | `admin-ceremony` |
| **Borrar Acta de Recuento** | `tally-sheet-delete` |
| **Leer Votos Emitidos** | `cast-vote-read` |
| **Leer Documentos** | `document-read` |
| **Editar Documentos** | `document-write` |
| **Leer Materiales de Soporte** | `support-material-read` |
| **Editar Materiales de Soporte** | `support-material-write` |
| **Miru Create** | `miru-create` |
| **Miru Download** | `miru-download` |
| **Miru Send** | `miru-send` |
| **Miru Sign** | `miru-sign` |
| **Editar Concurso** | `contest-write` |
| **Leer Concurso** | `contest-read` |
| **Editar Candidatos** | `candidate-write` |
| **Leer Candidatos** | `candidate-read` |
| **Editar la etiqueta de permiso** | `permission-label-write` |
| **Editar Eventos Programados** | `scheduled-event-write` |
| **Create Contest** | `contest-create` |
| **Delete Contest** | `contest-delete` |
| **Create Candidate** | `candidate-create` |
| **Delete Candidate** | `candidate-delete` |
| **Create Election** | `election-create` |
| **Read Election** | `election-read` |
| **Edit Election** | `election-write` |
| **Delete Election** | `election-delete` |
| **Archive Election Event** | `election-event-archive` |
| **Ver Datos de la Elección** | `election-data-tab` |
| **Ver Áreas del Evento Electoral** | `election-event-areas-tab` |
| **Ver Datos del Evento Electoral** | `election-event-data-tab` |
| **Ver Claves del Evento Electoral** | `election-event-keys-tab` |
| **Ver Registros del Evento Electoral** | `election-event-logs-tab` |
| **Ver Publicación del Evento Electoral** | `election-event-publish-tab` |
| **Ver Informes del Evento Electoral** | `election-event-reports-tab` |
| **Ver Programación del Evento Electoral** | `election-event-scheduled-tab` |
| **Ver Escrutinio del Evento Electoral** | `election-event-tally-tab` |
| **Ver Tareas del Evento Electoral** | `election-event-tasks-tab` |
| **Ver Votantes del Evento Electoral** | `election-event-voters-tab` |
| **Ver Publicación de la Elección** | `election-publish-tab` |
| **Ver Votantes de la Elección** | `election-voters-tab` |
| **Editar Informes** | `report-write` |
| **Leer Informes** | `report-read` |
| **Ver usuarios y roles** | `users-menu` |
| **Ver configuración** | `settings-menu` |
| **Ver plantillas** | `templates-menu` |
| **Ver configuración de tipos de elección** | `settings-election-types-tab` |
| **Ver configuración de canales de votación** | `settings-voting-channels-tab` |
| **Ver configuración de plantillas** | `settings-templates-tab` |
| **Ver configuración de idiomas** | `settings-languages-tab` |
| **Ver configuración de localización** | `settings-localization-tab` |
| **Ver configuración de apariencia** | `settings-look-feel-tab` |
| **Ver configuración de fideicomisarios** | `settings-trustees-tab` |
| **Ver configuración de países** | `settings-countries-tab` |
| **Importar Votante** | `voter-import` |
| **Ver Columnas de Votantes del Evento Electoral** | `ee-voters-columns` |
| **Verificar Votante Manualmente** | `voter-manually-verify` |
| **Ver Registros de Votantes del Evento Electoral** | `ee-voters-logs` |
| **Exportar Votante** | `voter-export` |
| **Ver Filtros de Votantes del Evento Electoral** | `ee-voters-filters` |
| **Eliminar Votante** | `voter-delete` |
| **Cambiar la Contraseña del Votante** | `voter-change-password` |
| **Selector de Localización del Evento Electoral** | `election-event-localization-selector` |
| **Crear Localización** | `localization-create` |
| **Leer Localización** | `localization-read` |
| **Editar Localización** | `localization-write` |
| **Eliminar Localización** | `localization-delete` |
| **Crear Área** | `area-create` |
| **Eliminar Área** | `area-delete` |
| **Exportar Área** | `area-export` |
| **Importar Área** | `area-import` |
| **Insertar o Actualizar Área** | `area-upsert` |
| **Columnas de las Áreas del Evento Electoral** | `election-event-areas-columns` |
| **Filtros de las Áreas del Evento Electoral** | `election-event-areas-filters` |
| **Volver a las Tareas del Evento Electoral** | `election-event-tasks-back-button` |
| **Columnas de las Tareas del Evento Electoral** | `election-event-tasks-columns` |
| **Filtros de las Tareas del Evento Electoral** | `election-event-tasks-filters` |
| **Exportar Tareas** | `task-export` |
| **Leer Aplicación** | `application-read` |
| **Editar Aplicación** | `application-write` |
| **Exportar Registros** | `logs-export` |
| **Columnas de los Registros del Evento Electoral** | `election-event-logs-columns` |
| **Filtros de los Registros del Evento Electoral** | `election-events-logs-filters` |
| **Columnas de los Eventos Programados del Evento Electoral** | `election-event-scheduled-event-columns` |
| **Crear Evento Programado** | `scheduled-event-create` |
| **Eliminar Evento Programado** | `scheduled-event-delete` |
| **Columnas de los Informes del Evento Electoral** | `election-event-reports-columns` |
| **Crear Informe** | `report-create` |
| **Eliminar Informe** | `report-delete` |
| **Generar Informe** | `report-generate` |
| **Vista Previa del Informe** | `report-preview` |
| **Vista del Panel de Monitoreo de Eventos Electorales** | `monitoring-dashboard-view-election-event` |
| **Vista del Panel de Monitoreo de Elecciones** | `monitoring-dashboard-view-election` |
| **Monitoreo de Votantes Autenticados** | `monitor-authenticated-voters` |
| **Leer Monitoreo de Votantes Aprobados y Rechazados** | `monitor-all-approve-disapprove-voters` |
| **Leer Monitoreo de Aprobaciones y Rechazos Automáticos** | `monitor-automatic-approve-disapprove-voters` |
| **Leer Monitoreo de Aprobaciones y Rechazos Manuales** | `monitor-manually-approve-disapprove-voters` |
| **Leer Monitoreo de Votantes Inscritos en el Extranjero** | `monitor-enrolled-overseas-voters` |
| **Leer Monitoreo de Publicaciones con Votación Cerrada** | `monitor-posts-already-closed-voting` |
| **Leer Monitoreo de Publicaciones con Resultados Generados** | `monitor-posts-already-generated-election-results` |
| **Leer Monitoreo de Publicaciones con Votación Abierta** | `monitor-posts-already-opened-voting` |
| **Leer Monitoreo de Publicaciones que Comenzaron a Contar Votos** | `monitor-posts-already-started-counting-votes` |
| **Leer Monitoreo de Publicaciones que Inicializaron el Sistema** | `monitor-posts-initialized-the-system` |
| **Leer Monitoreo de Publicaciones que Comenzaron a Votar** | `monitor-posts-started-voting` |
| **Leer Monitoreo de Publicaciones que Transmitieron Resultados** | `monitor-posts-transmitted-results` |
| **Leer Monitoreo de Votantes en Elección de Prueba** | `monitor-voters-voted-test-election` |
| **Leer Monitoreo de Votantes que Votaron** | `monitor-voters-who-voted` |
| **Vista Previa de la Publicación del Evento Electoral** | `election-event-publish-preview` |
| **Volver a la Publicación del Evento Electoral** | `election-event-publish-back-button` |
| **Columnas de la Publicación del Evento Electoral** | `election-event-publish-columns` |
| **Filtros de la Publicación del Evento Electoral** | `election-event-publish-filters` |
| **Crear Publicación** | `publish-create` |
| **Regenerar Publicación** | `publish-regenerate` |
| **Exportar Publicación** | `publish-export` |
| **Iniciar Votación** | `publish-start-voting` |
| **Pausar Votación** | `publish-pause-voting` |
| **Detener Votación** | `publish-stop-voting` |
| **Publicar Cambios** | `publish-changes` |
| **Ver Publicación del Evento Electoral** | `election-event-publish-view` |
| **Columnas de las Claves del Evento Electoral** | `election-event-keys-columns` |
| **Crear Ceremonia** | `create-ceremony` |
| **Exportar Ceremonia** | `export-ceremony` |
| **Columnas del Recuento del Evento Electoral** | `election-event-tally-columns` |
| **Volver al Recuento del Evento Electoral** | `election-event-tally-back-button` |
| **Ceremonia de Transmisión** | `transmition-ceremony` |
| **Ver Dirección IP** | `admin-ip-address-view` |
| **Ver Aprobaciones de la Elección** | `election-approvals-tab` |
| **Ver Aprobaciones del Evento Electoral** | `election-event-approvals-tab` |
| **Ver Dirección IP de la Elección** | `election-ip-address-view` |
| **Ver Dashboard de la Elección** | `election-dashboard-tab` |
| **Exportar Fideicomisarios** | `trustees-export` |
| **Importar Usuarios** | `user-import` |
| **Editar a los votantes que votaron** | `voter-voted-edit` |
| **Editar email/teléfono de los votantes** | `voter-email-tlf-edit` |
| **Editar las reglas de bloqueo por país en Cloudflare** | `cloudflare-write` |
| **Generar Informe de Transmisión** | `transmission-report-generate` |

## Páginas relacionadas {#related-pages}

- [Roles](../Roles/admin_portal_reference_user-manual_users-and-roles_roles.md)
- [Usuarios](../Users/admin_portal_reference_user-manual_users-and-roles_users.md)
- [Configurar el inquilino](../../../../procedures/01-tenant.md)
