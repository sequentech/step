import { Box, Typography } from '@mui/material';

import React from 'react';
import { useTranslation } from 'react-i18next';

type ElectionHeaderProps = {
  title: string;
  subtitle: string;
};

const ElectionHeader: React.FC<ElectionHeaderProps> = ({ title, subtitle }) => {
  const {t} = useTranslation();

  return (
      <Box sx={{py: 4, px: 2}}>
          <Typography variant="h5">{t(title)}</Typography>
          <Typography variant="caption">{t(subtitle)}</Typography>
      </Box>
  )
};

export default ElectionHeader;
