// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import { Box } from '@mui/material';
import { styled } from '@mui/material/styles';

import { SystemProps } from '@mui/system';

interface BackgroundProps extends SystemProps {
    imageUrl: string | undefined;
}

const Background = styled(Box)<BackgroundProps>(({ imageUrl }) => ({
    position: 'absolute',
    width: '100%',
    height: '100%',
    overflow: 'hidden',
    '&::before': {
      content: '""',
      position: 'absolute',
      top: 0,
      left: 0,
      width: '100%',
      height: '100%',
      backgroundImage: `url(${imageUrl})`,
      backgroundRepeat: 'repeat',
      backgroundPosition: 'center',
      backgroundSize: '100px 100px',
      opacity: 0.1,
    },
  }));

  export default Background;