---
id: election_management_election_event_logs
title: Logs
description: "La pestaña Logs ofrece una vista global de todas las actividades en curso, con información detallada sobre el funcionamiento del sistema."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


La pestaña **Logs** ofrece una vista global de todas las actividades en curso, con información detallada sobre el funcionamiento del sistema.

Estas son las acciones a nivel de aplicación que se registran. Acceda al registro del sistema para supervisar la actividad de toda la plataforma, incluidos los eventos de votantes, los eventos de Keycloak, los eventos del sistema y los eventos de usuario.

- Seleccione **Columnas** para ocultar o mostrar distintos datos de cada entrada del registro.

## Integridad {#integrity}

Las entradas del registro se guardan en PostgreSQL. Cada entrada también se confirma en un registro Merkle de su proceso electoral. Mediante la API y las herramientas de línea de comandos de la plataforma, los operadores pueden obtener pruebas de inclusión de entradas individuales y pruebas de consistencia que demuestran que el registro solo se ha ampliado desde un punto de control anterior.

Estas pruebas solo muestran una manipulación con respecto a un punto de control guardado fuera del registro. La plataforma publica un punto de control firmado cuando se cierra la votación y cuando termina cada recuento de resultados. Ese punto de control se guarda aparte del registro y queda anotado en él como una entrada "Punto de control del registro electoral publicado". También puede guardar puntos de control por su cuenta, por ejemplo al inicio y al final de la votación, y verificar después las pruebas con ellos. Eliminar un proceso electoral también elimina su registro.

## Auditoría {#audit}

Una auditoría comprueba que cada entrada del registro coincide con su registro Merkle, que el registro Merkle guardado está completo y es coherente, y que cada punto de control publicado está firmado correctamente y forma parte del historial del registro. Una auditoría se ejecuta automáticamente cuando termina un recuento de resultados; su resultado aparece en los registros del recuento.

- Seleccione **Auditar** para iniciar una auditoría manualmente. Esto requiere los permisos logs-read y electoral-log-audit.
- La auditoría se ejecuta como una tarea "Auditar registro electoral". Termina con éxito cuando el registro es coherente y falla cuando encuentra un problema, y muestra cada hallazgo en los registros de la tarea. Las auditorías solo informan de problemas; nunca modifican el registro.
