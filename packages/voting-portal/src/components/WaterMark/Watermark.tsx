// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useCallback, useMemo } from 'react';
import demoBanner from './assets/demo-banner.png'
import { useAppSelector } from '../../store/hooks';
import { selectFirstBallotStyle } from '../../store/ballotStyles/ballotStylesSlice';
import Background from './Background';

const WatermarkBackground: React.FC = () => {
  const oneBallotStyle = useAppSelector(selectFirstBallotStyle)
  const isDemo = useMemo(() => {
    return oneBallotStyle?.ballot_eml.public_key?.is_demo
}, [oneBallotStyle]);

const imageUrl = useCallback(() => {
    if(isDemo) {
        return demoBanner;
    }
}, [isDemo])

if(!imageUrl()) {
  return null;
}

  return (
    <Background imageUrl = {imageUrl()}/>
  );
};

export default WatermarkBackground;