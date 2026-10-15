---
sidebar_position: 5
id: results_website
title: Sitio web de resultados
description: "El sitio web de resultados publica una vista reducida y de solo lectura de un recuento completado."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Sitio web de resultados

El sitio web de resultados publica una vista reducida a propósito, de solo lectura, de un recuento completado. La publicación crea una versión nueva; nunca modifica la base de datos del recuento ni expone papeletas, referencias a documentos, estadísticas internas ni datos de procesos electorales, elecciones, preguntas o áreas no seleccionados.

## Política y permisos {#policy-and-permissions}

Configure la política en la pantalla de datos del proceso electoral, usando su acción **Guardar** específica. Guardar el resto del proceso electoral no cambia esta política.

- `Disabled` elimina el proceso electoral del descubrimiento y limpia los artefactos de publicación activos.
- `Enabled` permite la publicación usando los valores configurados de acceso y visibilidad.
- El acceso `Public` solo admite la visibilidad `Full event`.
- El acceso `Authenticated` admite la visibilidad `Full event` o `Area based`. Los votantes de visibilidad por área reciben solo el artefacto de su área en el token del realm del proceso electoral.

Se requiere `publish-results-write` para configurar la política, publicar, revocar o reconstruir el descubrimiento. Se requiere `publish-results-read` para leer el historial de publicaciones. El rol genérico `admin-user` no puede leer el historial ni cambiar `presentation.results_website` directamente.

## Publicación y rutas {#publishing-and-routes}

Cuando termine un recuento, seleccione las preguntas y publique desde la pantalla de recuento. Las publicaciones del proceso electoral están disponibles en `/:electionEventId`; las publicaciones de una elección concreta están disponibles en `/:electionEventId/elections/:electionId`. Una ruta de elección puede usar una publicación del proceso electoral solo cuando esa publicación contiene explícitamente la elección solicitada.

Cada ruta tiene versiones con número creciente. Una sustitución correcta reemplaza la versión anterior y elimina sus artefactos públicos o privados. Los índices de descubrimiento modificables y los alias de la última versión usan `no-store`; los artefactos versionados e inmutables pueden almacenarse en caché.

Los recuentos vacíos siguen mostrando los gráficos circulares de **Información General** y **Resumen de participación** como 100 % de no votantes. Los resultados de candidato, en blanco, en blanco explícito o implícito, inválidos y preferenciales conservan su significado de recuento según su tipo.

## Revocación y auditoría {#revocation-and-audit}

La revocación registra `Revoked` y `revoked_at` antes de la limpieza, de modo que reintentar la misma acción reanuda la limpieza de forma segura tras un fallo del almacén de objetos. La revocación elimina la publicación del descubrimiento y borra sus objetos de respaldo públicos o privados y sus filas de documentos. El historial de publicaciones muestra la hora de la revocación.

Las acciones de publicar y revocar se escriben en el registro electoral con el id de la publicación, la acción, la ruta, el acceso, la visibilidad, los ids de las preguntas y el usuario que actúa. Si el descubrimiento queda desactualizado tras un fallo de infraestructura, use la acción de actualización de la publicación de resultados; reintente la acción de revocar de una publicación revocada para reanudar la limpieza de artefactos.

## CSS personalizado {#custom-css}

El manifiesto publicado captura `presentation.css` del proceso electoral y de cada elección seleccionada. El CSS del proceso electoral se carga primero; el CSS de la elección actual se carga después y, por tanto, puede sobrescribirlo. Esto funciona tanto en las rutas del proceso electoral como en las de elección, y cuando cambia la elección seleccionada.

Los selectores estables usan el prefijo `seq-results-`. Algunos ejemplos importantes son:

- `.seq-results-page`, `.seq-results-summary`, `.seq-results-selector` y `.seq-results-contest`;
- elementos de estilo BEM, como `.seq-results-summary__pie`, `.seq-results-contest__title` y `.seq-results-selector__selected-result`;
- clases de estado fijas, como `.seq-results-access--public`, `.seq-results-visibility--area_based` y `.seq-results-contest--preferential`;
- clases de entidad, como `.seq-results-event--<id>`, `.seq-results-election--<id>`, `.seq-results-contest--<id>`, `.seq-results-area--<id>`, y clases de fila y pestaña específicas de cada entidad.

Los valores de entidad se normalizan de forma reproducible reemplazando por `-` los caracteres que no son letras, dígitos, `_` ni `-`; un área ausente usa `global`. Para el diseño, prefiera las clases semánticas, y para la imagen propia de cada elección, las clases de entidad.

## Configuración de despliegue {#deployment-configuration}

`RESULTS_PORTAL_URL` debe configurarse antes del despliegue de la aplicación, para que la creación o importación del proceso electoral pueda crear el cliente de Keycloak `results-portal` dedicado, con solo el patrón de redirección del sitio de resultados. El portal de resultados usa `RESULTS_PORTAL_CLIENT_ID=results-portal`. `DISABLE_AUTH` es solo para desarrollo y no debe activarse en entornos desplegados.
