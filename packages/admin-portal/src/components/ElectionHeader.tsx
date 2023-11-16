import { Box, Typography } from '@mui/material';

import React from 'react';

type ElectionHeaderProps = {
  title: string;
  subtitle: string;
};

const ElectionHeader: React.FC<ElectionHeaderProps> = ({ title, subtitle }) => {
  return (
      <Box sx={{py: 4, px: 2}}>
          <Typography variant="h5">{title}</Typography>
          <Typography variant="caption">{subtitle}</Typography>
      </Box>
  )
};

export default ElectionHeader;
