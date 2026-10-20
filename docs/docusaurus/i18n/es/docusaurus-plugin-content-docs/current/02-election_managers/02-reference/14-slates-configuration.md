---
id: slates_configuration
title: Configuración de las candidaturas
sidebar_position: 14
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Esta página explica dónde se configuran las candidaturas de una elección, las reglas que la elección impone sobre ellas y cuándo se comprueban esas reglas. El formato de la configuración y lo que ven los votantes se describen en [Candidaturas con nombre](./13-named-slates.md).

Una elección sin configuración de candidaturas no se ve afectada. Sus papeletas, sus hashes y el flujo de votación siguen exactamente igual.

---

## Dónde se configura {#where-it-is-configured}

### Diseño de la Papeleta {#ballot-design}

Abra la elección, vaya a la pestaña **Datos** y despliegue **Diseño de la Papeleta** (consulte [Datos de la elección](./03-election/03-election_management_election_data.md)):

- **Habilitar candidaturas** activa las candidaturas para la elección. Si se desactiva en una elección que tiene candidaturas, el sistema pregunta primero con **Deshabilitar candidaturas** y las elimina al guardar.
- **Listas de candidatos en móvil**, que aparece cuando las candidaturas están habilitadas, indica si la lista de candidatos de cada candidatura empieza **Contraídas** (predeterminado) o **Desplegadas** en los teléfonos. Las pantallas más anchas muestran siempre todos los candidatos.

### Las candidaturas en el árbol {#slates-in-the-tree}

Cuando las candidaturas están habilitadas, las candidaturas de la elección aparecen en el árbol debajo de la elección, antes de sus preguntas, con **Crear una Candidatura**. El menú **⋯** de una candidatura tiene **Eliminar esta Candidatura**, que pide confirmación antes de borrarla.

### La página de la candidatura {#the-slate-page}

Una candidatura tiene una única pestaña **Datos** con dos secciones:

- **General**: **Nombre** (obligatorio), **Alias**, **Descripción** e **ID externo**. El nombre es el nombre de la candidatura en el idioma predeterminado de la elección.
- **Miembros**: un selector de candidatos por cada puesto de cada pregunta, hasta el **Máximo de votos** de la pregunta. Si deja una pregunta vacía, la candidatura subvota ese cargo. No se ofrece un candidato que ya esté en otra candidatura, ni los candidatos deshabilitados, los marcadores de voto por escrito, las opciones explícitas de voto en blanco o no válido, ni las listas de categorías. Los candidatos que no están en ninguna candidatura aparecen a los votantes como **Independiente**.

Al guardar, el sistema comprueba las candidaturas frente a las preguntas y los candidatos de la elección, y muestra todos los problemas que encuentra. No se almacena nada hasta que la configuración es válida. Solo se escribe la anotación `sequent.slates` de la elección; las demás anotaciones se mantienen tal como están.

Los nombres nunca se usan como referencias, así que renombrar un candidato no cambia una candidatura. Añadir, cambiar o eliminar una candidatura requiere el permiso para editar la elección. Las candidaturas de un proceso electoral archivado no se pueden añadir ni eliminar.

---

## Reglas que impone la elección {#rules-the-election-imposes}

Los límites de las preguntas son el **Mínimo de votos** y el **Máximo de votos** propios de cada pregunta; las candidaturas no añaden ningún límite propio. Además de las reglas de formato de [Candidaturas con nombre](./13-named-slates.md#validation), la configuración se rechaza cuando:

- una candidatura no tiene nombre en el idioma predeterminado de la elección;
- una candidatura incluye, en una pregunta, más candidatos que el máximo de votos de la pregunta;
- una pregunta usa un algoritmo de recuento preferencial o acumulativo, ya que las candidaturas solo admiten votación ordinaria por candidatos;
- un miembro está deshabilitado, es un marcador de voto por escrito, es una opción explícita de voto en blanco o no válido, o es una lista de categorías.

Una candidatura que solo cubre algunas preguntas, o que incluye menos candidatos de los que permite una pregunta, es una candidatura parcial y es válida.

---

## Orden de presentación {#listing-order}

Las normas electorales suelen exigir una presentación fija: primero las candidaturas completas, después las parciales y después los candidatos independientes, con el orden entre las candidaturas completas y entre las parciales determinado por sorteo. Se configura en dos lugares:

- **El orden de las candidaturas** es el orden de las tarjetas de candidatura: de izquierda a derecha en pantallas anchas, de arriba abajo en los teléfonos, y el orden de las candidaturas en el árbol. Una candidatura nueva se añade al final, y no hay ningún control para reordenar las candidaturas: cree primero las candidaturas completas, en el orden sorteado, y después las parciales. Para cambiar el orden, elimine las candidaturas y créelas de nuevo en el orden correcto.
- **El orden de candidatos de cada pregunta** es el orden de los candidatos individuales, y de los candidatos dentro de cada tarjeta de candidatura. En el **Diseño de la papeleta** de la pregunta, establezca el orden de candidatos en **Personalizado** y use **Reordernar candidatos** para presentar primero los candidatos de la primera candidatura, después los de la siguiente, y los candidatos independientes al final.

La plataforma no impone ningún orden de presentación: el orden configurado es el que ven los votantes.

---

## Cuándo se comprueba {#when-it-is-checked}

Las mismas reglas se aplican en cada paso, de modo que una configuración no puede superar uno y fallar en el siguiente:

| Paso | Qué ocurre si hay un problema |
| --- | --- |
| Guardar una candidatura en el portal de administración | La página de la candidatura muestra todos los problemas y no se guarda nada. |
| Importar un proceso electoral | La importación se rechaza e informa de los problemas. Los identificadores de preguntas y candidatos dentro de la configuración se regeneran junto con el resto del proceso electoral, de modo que las candidaturas importadas siguen apuntando a los candidatos importados. |
| Generar una publicación de la papeleta | La tarea de generación falla con los problemas. La configuración se comprueba primero frente a toda la elección y después frente a la papeleta de cada área. |
| Publicar una publicación generada | La publicación se rechaza con los problemas. Esto también afecta a las publicaciones generadas antes de una actualización. |
| Cargar la papeleta en el portal de votación | El votante ve un error de configuración en lugar de una papeleta sin sus candidaturas. |

La papeleta de cada votante solo contiene las preguntas sobre las que vota su área. Las preguntas de una candidatura que están fuera de esa papeleta no se comprueban para ese votante. Un miembro que falta en una pregunta que sí está en la papeleta es un error.

---

## Después de que empiece la votación {#after-voting-starts}

Cuando la votación de una elección ha empezado, una publicación que cambia su configuración de candidaturas se rechaza: los votantes que todavía no han votado deben ver las mismas candidaturas que quienes ya tienen su papeleta. Restaure la configuración publicada para publicar otros cambios. Guardar una candidatura sin cambiarla no es un cambio.
