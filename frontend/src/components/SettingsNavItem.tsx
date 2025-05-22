// SettingsNavItem.tsx
import React from 'react';
import Tooltip from '@mui/material/Tooltip';
import Box from '@mui/material/Box';
import SettingsIcon from '@mui/icons-material/Settings'; // Material UI Icon

interface SettingsNavItemProps {
  isSelected: boolean;
  onClick: () => void;
  themeMode: 'light' | 'dark'; // To control theme-specific styles
}

export const SettingsNavItem: React.FC<SettingsNavItemProps> = ({
  isSelected,
  onClick,
  themeMode,
}) => {
  // Define colors based on themeMode
  const colors = {
    dark: {
      outerBoxSelectedBg: '#22232a', // var(--md-sys-color-surface-1-dark)
      innerBoxDefaultBg: '#272831', // var(--md-sys-color-surface-2-dark)
      innerBoxDefaultBorder: '#c5c6d0', // var(--md-sys-color-on-surface-variant-dark)
      innerBoxHoverBorder: '#e3e2e6', // var(--md-sys-color-on-surface-dark)
      innerBoxSelectedBorder: '#afc6ff', // var(--md-sys-color-primary-dark)
      iconDefault: '#c5c6d0',
      iconHover: '#e3e2e6',
      iconSelected: '#afc6ff',
    },
    light: {
      outerBoxSelectedBg: '#fefbff', // var(--md-sys-color-background-light)
      innerBoxDefaultBg: '#eaeefb', // var(--md-sys-color-surface-2-light)
      innerBoxDefaultBorder: '#44464f', // var(--md-sys-color-on-surface-variant-light)
      innerBoxHoverBorder: '#1b1b1f', // var(--md-sys-color-on-surface-light)
      innerBoxSelectedBorder: '#0059c7', // var(--md-sys-color-primary-light)
      iconDefault: '#44464f',
      iconHover: '#1b1b1f',
      iconSelected: '#0059c7',
    },
  };

  const currentColors = colors[themeMode];

  const outerBoxSx = {
    width: '90px',
    height: '90px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    cursor: 'pointer',
    position: 'relative', // For potential advanced ripple or positioning
    backgroundColor: isSelected ? currentColors.outerBoxSelectedBg : 'transparent',
    transition: 'background-color 0.2s ease-in-out',
  };

  const innerBoxSx = {
    width: '70px',
    height: '70px',
    borderRadius: '8px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: currentColors.innerBoxDefaultBg,
    border: `1.5px solid ${isSelected ? currentColors.innerBoxSelectedBorder : currentColors.innerBoxDefaultBorder}`,
    color: isSelected ? currentColors.iconSelected : currentColors.iconDefault, // Icon color based on selection
    transition: 'transform 0.2s ease-in-out, background-color 0.2s ease-in-out, border-color 0.2s ease-in-out, border-width 0.2s ease-in-out, color 0.2s ease-in-out',
    transform: isSelected ? 'scale(1.05)' : 'scale(1)',
    borderWidth: isSelected ? '2.5px' : '1.5px',
    '&:hover': {
      transform: 'scale(1.05)',
      borderColor: isSelected ? currentColors.innerBoxSelectedBorder : currentColors.innerBoxHoverBorder, // Keep selected border color on hover if selected
      borderWidth: '2.5px',
      color: isSelected ? currentColors.iconSelected : currentColors.iconHover, // Icon hover color
    },
  };
  
  const iconSx = {
    fontSize: '36px', // Adjust for approx 50x50 visual space
    // Color is handled by the parent (innerBoxSx) 'color' property for easier state management
  };

  return (
    <Tooltip title="系统设置" placement="right">
      <Box sx={outerBoxSx} onClick={onClick}>
        <Box sx={innerBoxSx}>
          <SettingsIcon sx={iconSx} />
        </Box>
      </Box>
    </Tooltip>
  );
};

// Example of how to use it (typically in Sidebar.tsx):
// import { SettingsNavItem } from './SettingsNavItem'; // Adjust path as needed
//
// const MySidebar = () => {
//   const [selectedItem, setSelectedItem] = React.useState('someOtherItem');
//   const currentThemeMode = 'dark'; // or 'light', determine this from your theme provider
//
//   return (
//     <div>
//       {/* Other sidebar items */}
//       <SettingsNavItem
//         isSelected={selectedItem === 'settings'}
//         onClick={() => setSelectedItem('settings')}
//         themeMode={currentThemeMode}
//       />
//     </div>
//   );
// };