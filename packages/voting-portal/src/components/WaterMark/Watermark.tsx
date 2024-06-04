import React, { PropsWithChildren } from 'react';
import './style.css'; // Import your CSS file
import { styled } from '@mui/material/styles';
import { Box } from '@mui/material';

const Background = styled(Box)(({ theme }) => ({
  position: 'relative',
  width: '100%',
  height: '100vh',
  backgroundColor: '#ffffff',
  overflow: 'hidden',
  '&::before': {
    content: '""',
    position: 'absolute',
    top: 0,
    left: 0,
    width: '100%',
    height: '100%',
    backgroundImage: 'url("https://example.com/path/to/your/watermark-image.png")',
    backgroundRepeat: 'repeat',
    backgroundPosition: 'center',
    backgroundSize: '100px 100px',
    opacity: 0.1, // 10% opacity
  },
  '& > *': {
    position: 'relative',
    zIndex: 1,
  }
}));

const WatermarkBackground: React.FC<PropsWithChildren> = ({ children }) => {
  return (
    <Background>
      {children}
    </Background>
  );
};

export default WatermarkBackground;