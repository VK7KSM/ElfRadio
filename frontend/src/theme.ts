// frontend/src/theme.ts
import { createTheme, ThemeOptions, PaletteMode, PaletteColor, PaletteColorOptions } from '@mui/material/styles';

// 定义亮色主题调色板对象
const elfRadioLightPalette = {
  mode: 'light' as PaletteMode,
  primary: {
    main: '#0059c7', // 来自 --md-sys-color-primary
    contrastText: '#ffffff', // 来自 --md-sys-color-on-primary
  },
  secondary: {
    main: '#575e71', // 来自 --md-sys-color-secondary
    contrastText: '#ffffff', // 来自 --md-sys-color-on-secondary
  },
  error: {
    main: '#ba1a1a', // 来自 --md-sys-color-error
    contrastText: '#ffffff', // 来自 --md-sys-color-on-error
  },
  background: {
    default: '#fefbff', // 来自 --md-sys-color-background
    paper: '#fefbff',   // 来自 --md-sys-color-surface
  },
  text: {
    primary: '#1b1b1f',   // 来自 --md-sys-color-on-background
    secondary: '#44464f', // 来自 --md-sys-color-on-surface-variant
  },
  tertiary: { // 第三色
    main: '#725573', // 来自 --md-sys-color-tertiary
    contrastText: '#ffffff', // 来自 --md-sys-color-on-tertiary
  },
  md3Colors: { // 其他 MD3 颜色角色
    primaryContainer: '#d9e2ff', // 来自 --md-sys-color-primary-container
    onPrimaryContainer: '#001a43', // 来自 --md-sys-color-on-primary-container
    secondaryContainer: '#dbe2f9', // 来自 --md-sys-color-secondary-container
    onSecondaryContainer: '#141b2c', // 来自 --md-sys-color-on-secondary-container
    tertiaryContainer: '#fcd7fb', // 来自 --md-sys-color-tertiary-container
    onTertiaryContainer: '#2a132d', // 来自 --md-sys-color-on-tertiary-container
    errorContainer: '#ffdad6', // 来自 --md-sys-color-error-container
    onErrorContainer: '#410002', // 来自 --md-sys-color-on-error-container
    outline: '#757780', // 来自 --md-sys-color-outline
    outlineVariant: '#c5c6d0', // 来自 --md-sys-color-outline-variant
    surface: '#fefbff', // 来自 --md-sys-color-surface (与 background.paper 相同)
    onSurface: '#1b1b1f', // 来自 --md-sys-color-on-surface
    surfaceVariant: '#e1e2ec', // 来自 --md-sys-color-surface-variant
    onSurfaceVariant: '#44464f', // 来自 --md-sys-color-on-surface-variant
    inverseSurface: '#303034', // 来自 --md-sys-color-inverse-surface
    inverseOnSurface: '#f2f0f4', // 来自 --md-sys-color-inverse-on-surface
    inversePrimary: '#afc6ff', // 来自 --md-sys-color-inverse-primary
    shadow: '#000000', // 来自 --md-sys-color-shadow
    scrim: '#000000', // 来自 --md-sys-color-scrim
  },
  customSurfaces: { // 自定义表面层级颜色
    surface0: '#fefbff', // 来自 --md-sys-color-surface-0
    surface1: '#f2f3fc', // 来自 --md-sys-color-surface-1
    surface2: '#eaeefb', // 来自 --md-sys-color-surface-2
    surface3: '#e2e9f9', // 来自 --md-sys-color-surface-3
    surface4: '#e0e8f8', // 来自 --md-sys-color-surface-4
    surface5: '#dbe5f7', // 来自 --md-sys-color-surface-5
    // 为亮色模式添加新的特定背景色键
    sidebarBackground: '#E7F0F2',
    secondaryPaneBackground: '#EFF5F7',
    tertiaryPaneBackground: '#BCD3D8',
  }
};

// 定义暗色主题调色板对象
const elfRadioDarkPalette = {
  mode: 'dark' as PaletteMode,
  primary: {
    main: '#afc6ff', // 来自 --md-sys-color-primary (dark)
    contrastText: '#002d6c', // 来自 --md-sys-color-on-primary (dark)
  },
  secondary: {
    main: '#bfc6dc', // 来自 --md-sys-color-secondary (dark)
    contrastText: '#293042', // 来自 --md-sys-color-on-secondary (dark)
  },
  error: {
    main: '#ffb4ab', // 来自 --md-sys-color-error (dark)
    contrastText: '#690005', // 来自 --md-sys-color-on-error (dark)
  },
  background: {
    default: '#1b1b1f', // 来自 --md-sys-color-background (dark)
    paper: '#1b1b1f',   // 来自 --md-sys-color-surface (dark)
  },
  text: {
    primary: '#e3e2e6',   // 来自 --md-sys-color-on-background (dark)
    secondary: '#c5c6d0', // 来自 --md-sys-color-on-surface-variant (dark)
  },
  tertiary: { // 第三色 (dark)
    main: '#dfbbde', // 来自 --md-sys-color-tertiary (dark)
    contrastText: '#402743', // 来自 --md-sys-color-on-tertiary (dark)
  },
  md3Colors: { // 其他 MD3 颜色角色 (dark)
    primaryContainer: '#004398', // 来自 --md-sys-color-primary-container (dark)
    onPrimaryContainer: '#d9e2ff', // 来自 --md-sys-color-on-primary-container (dark)
    secondaryContainer: '#3f4759', // 来自 --md-sys-color-secondary-container (dark)
    onSecondaryContainer: '#dbe2f9', // 来自 --md-sys-color-on-secondary-container (dark)
    tertiaryContainer: '#593e5a', // 来自 --md-sys-color-tertiary-container (dark)
    onTertiaryContainer: '#fcd7fb', // 来自 --md-sys-color-on-tertiary-container (dark)
    errorContainer: '#93000a', // 来自 --md-sys-color-error-container (dark)
    onErrorContainer: '#ffb4ab', // 来自 --md-sys-color-on-error-container (dark)
    outline: '#8f9099', // 来自 --md-sys-color-outline (dark)
    outlineVariant: '#44464f', // 来自 --md-sys-color-outline-variant (dark)
    surface: '#1b1b1f', // 来自 --md-sys-color-surface (dark)
    onSurface: '#e3e2e6', // 来自 --md-sys-color-on-surface (dark)
    surfaceVariant: '#44464f', // 来自 --md-sys-color-surface-variant (dark)
    onSurfaceVariant: '#c5c6d0', // 来自 --md-sys-color-on-surface-variant (dark)
    inverseSurface: '#e3e2e6', // 来自 --md-sys-color-inverse-surface (dark)
    inverseOnSurface: '#303034', // 来自 --md-sys-color-inverse-on-surface (dark)
    inversePrimary: '#0059c7', // 来自 --md-sys-color-inverse-primary (dark)
    shadow: '#000000', // 来自 --md-sys-color-shadow (dark)
    scrim: '#000000', // 来自 --md-sys-color-scrim (dark)
  },
  customSurfaces: { // 自定义表面层级颜色 (dark)
    surface0: '#1b1b1f', // 来自 --md-sys-color-surface-0 (dark)
    surface1: '#22232a', // 来自 --md-sys-color-surface-1 (dark)
    surface2: '#272831', // 来自 --md-sys-color-surface-2 (dark)
    surface3: '#2b2e38', // 来自 --md-sys-color-surface-3 (dark)
    surface4: '#2c2f39', // 来自 --md-sys-color-surface-4 (dark)
    surface5: '#2f323e', // 来自 --md-sys-color-surface-5 (dark)
    // 为暗色模式添加新的特定背景色键
    sidebarBackground: '#1D2B2D',
    secondaryPaneBackground: '#1C2527',
    tertiaryPaneBackground: '#354A4E',
  }
};

// MUI Palette 模块增强，以支持自定义颜色角色
declare module '@mui/material/styles' {
  // 定义 MD3 颜色角色的接口
  interface Md3ColorRoles {
    primaryContainer?: string;
    onPrimaryContainer?: string;
    secondaryContainer?: string;
    onSecondaryContainer?: string;
    tertiaryContainer?: string;
    onTertiaryContainer?: string;
    errorContainer?: string;
    onErrorContainer?: string;
    outline?: string;
    outlineVariant?: string;
    surface?: string;
    onSurface?: string;
    surfaceVariant?: string;
    onSurfaceVariant?: string;
    inverseSurface?: string;
    inverseOnSurface?: string;
    inversePrimary?: string;
    shadow?: string;
    scrim?: string;
  }

  // 定义自定义表面层级的接口
  interface CustomSurfaceLevels {
    surface0: string;
    surface1: string;
    surface2: string;
    surface3: string;
    surface4: string;
    surface5: string;
    // 添加新的背景色键类型定义
    sidebarBackground: string;
    secondaryPaneBackground: string;
    tertiaryPaneBackground: string;
  }

  // 扩展 MUI Palette 接口
  interface Palette {
    tertiary: PaletteColor; // 添加 tertiary 标准调色板颜色
    md3Colors: Md3ColorRoles; // 添加 md3Colors 对象
    customSurfaces: CustomSurfaceLevels; // 添加 customSurfaces 对象
  }

  // 扩展 MUI PaletteOptions 接口，以允许在 createTheme 时传递这些自定义属性
  interface PaletteOptions {
    tertiary?: PaletteColorOptions; // tertiary 的选项
    md3Colors?: Partial<Md3ColorRoles>; // md3Colors 的选项 (Partial 表示并非所有角色都必须定义)
    customSurfaces?: CustomSurfaceLevels; // customSurfaces 的选项
  }
}

// 导出亮色主题的 ThemeOptions
export const lightThemeOptions: ThemeOptions = {
  palette: elfRadioLightPalette,
  // 可以在后续步骤中添加排版 (Typography) 和组件覆盖 (components overrides)
};

// 导出暗色主题的 ThemeOptions
export const darkThemeOptions: ThemeOptions = {
  palette: elfRadioDarkPalette,
  // 可以在后续步骤中添加排版 (Typography) 和组件覆盖 (components overrides)
};

// 创建并导出实际的主题实例
export const lightTheme = createTheme(lightThemeOptions);
export const darkTheme = createTheme(darkThemeOptions);