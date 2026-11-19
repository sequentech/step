---
id: election_management_election_event_areas
title: Áreas
description: "Representa las divisiones geográficas u organizativas dentro de las que se celebran las elecciones. Pueden ir desde pequeños recintos o distritos electorales hasta regiones más grandes, como provincias o estados."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Representa las divisiones geográficas u organizativas dentro de las que se celebran las elecciones. Pueden ir desde pequeños recintos o distritos electorales hasta regiones más grandes, como provincias o estados.

### Opciones de la tabla {#table-options}

- **Columnas**: Habilita o deshabilita columnas específicas en la vista de tabla.
- **Añadir filtro**: Crea un filtro de texto para la tabla por columna.
- **Añadir**: Añada manualmente una nueva área.
- **Importar**: Importa áreas nuevas desde un archivo CSV.
- **Exportar**: Exporta las áreas existentes a un archivo CSV.
- **Insertar o actualizar**: Actualiza o inserta áreas desde un archivo CSV.

## Votación anticipada por área {#early-voting-per-area}

Al añadir o editar un área, puede habilitar la opción **Votación anticipada** para esa área concreta. Si está habilitada, los votantes asignados a esa área podrán votar en cuanto se inicie el canal **EARLY_VOTING** del proceso electoral.

- **Visibilidad**: La opción Votación anticipada aparece solo si el canal **EARLY_VOTING** está permitido a nivel de proceso electoral (Portal de administración > Proceso Electoral > Datos > Canales de voto permitidos).
- **Efecto**: Habilitar la votación anticipada en un área no inicia la votación por sí mismo; los votantes de esa área podrán votar cuando se inicie el canal **EARLY_VOTING** del proceso electoral en la pestaña Publicar.
- **Importación/exportación**: La configuración de votación anticipada se admite en las operaciones de importación CSV y de actualización (insertar o actualizar), así que puede gestionar esta política a gran escala.

Nota: La votación en quiosco tiene un estado independiente. El canal En línea rige la disponibilidad de la votación anticipada (véase Proceso Electoral > Datos > Canales de voto permitidos para más detalles).
