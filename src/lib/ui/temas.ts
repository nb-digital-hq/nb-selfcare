/** Catálogo de SelfCare: mismos identificadores y tokens base que el móvil. */
export interface TemaDelCatalogo {
  readonly id: string;
  readonly papel: boolean;
  readonly colores: {
    readonly fondo: string;
    readonly fondoElevado: string;
    readonly superficie: string;
    readonly superficieAlt: string;
    readonly borde: string;
    readonly texto: string;
    readonly textoSec: string;
    readonly textoMuted: string;
    readonly acento: string;
    readonly acentoFuerte: string;
    readonly hover: string;
    readonly seleccion: string;
    readonly exito: string;
    readonly peligro: string;
    readonly advertencia: string;
  };
}
export const TEMAS: readonly TemaDelCatalogo[] = [
  {
    id: 'samsung_dark',
    papel: false,
    colores: {
      fondo: '#0b0e14',
      fondoElevado: '#151922',
      superficie: '#151922',
      superficieAlt: '#1e2430',
      borde: '#2d3748',
      texto: '#ffffff',
      textoSec: '#b2bac6',
      textoMuted: '#7a8598',
      acento: '#00e5a3',
      acentoFuerte: '#00b882',
      hover: '#1e2430',
      seleccion: '#1e2430',
      exito: '#10b981',
      peligro: '#ef4444',
      advertencia: '#f59e0b'
    }
  },
  {
    id: 'samsung_light',
    papel: false,
    colores: {
      fondo: '#f5f6fa',
      fondoElevado: '#ffffff',
      superficie: '#ffffff',
      superficieAlt: '#ebedf5',
      borde: '#cbd5e1',
      texto: '#0f172a',
      textoSec: '#475569',
      textoMuted: '#627083',
      acento: '#008461',
      acentoFuerte: '#006249',
      hover: '#ebedf5',
      seleccion: '#ebedf5',
      exito: '#10b981',
      peligro: '#ef4444',
      advertencia: '#f59e0b'
    }
  },
  {
    id: 'salvia_lino',
    papel: false,
    colores: {
      fondo: '#f5f2ea',
      fondoElevado: '#fffdf8',
      superficie: '#fffdf8',
      superficieAlt: '#ece7dc',
      borde: '#dcd5c6',
      texto: '#1d221e',
      textoSec: '#48514a',
      textoMuted: '#646d65',
      acento: '#537e61',
      acentoFuerte: '#3d624a',
      hover: '#ece7dc',
      seleccion: '#ece7dc',
      exito: '#2f7d4f',
      peligro: '#c62f2f',
      advertencia: '#a86a06'
    }
  },
  {
    id: 'arcilla_nocturna',
    papel: false,
    colores: {
      fondo: '#181513',
      fondoElevado: '#221c18',
      superficie: '#221c18',
      superficieAlt: '#302823',
      borde: '#4b3f37',
      texto: '#f6f3ee',
      textoSec: '#c5b9b0',
      textoMuted: '#908479',
      acento: '#e07a52',
      acentoFuerte: '#c15f3c',
      hover: '#302823',
      seleccion: '#302823',
      exito: '#10b981',
      peligro: '#ef4444',
      advertencia: '#f59e0b'
    }
  },
  {
    id: 'negro_oled',
    papel: false,
    colores: {
      fondo: '#000000',
      fondoElevado: '#0a0c0f',
      superficie: '#0a0c0f',
      superficieAlt: '#14181f',
      borde: '#28303c',
      texto: '#ffffff',
      textoSec: '#a6b2c3',
      textoMuted: '#717e91',
      acento: '#00f0b5',
      acentoFuerte: '#00c794',
      hover: '#14181f',
      seleccion: '#14181f',
      exito: '#10b981',
      peligro: '#ef4444',
      advertencia: '#f59e0b'
    }
  },
  {
    id: 'nxtpaper_color_paper',
    papel: true,
    colores: {
      fondo: '#f3efe6',
      fondoElevado: '#ffffff',
      superficie: '#ffffff',
      superficieAlt: '#eae4d7',
      borde: '#645a4d',
      texto: '#221e19',
      textoSec: '#5a5246',
      textoMuted: '#726a5d',
      acento: '#0d7490',
      acentoFuerte: '#085266',
      hover: '#eae4d7',
      seleccion: '#eae4d7',
      exito: '#10b981',
      peligro: '#ef4444',
      advertencia: '#f59e0b'
    }
  },
  {
    id: 'nxtpaper_eink_light',
    papel: true,
    colores: {
      fondo: '#f7f5f0',
      fondoElevado: '#ffffff',
      superficie: '#ffffff',
      superficieAlt: '#f0ede5',
      borde: '#121212',
      texto: '#121212',
      textoSec: '#333333',
      textoMuted: '#595959',
      acento: '#121212',
      acentoFuerte: '#000000',
      hover: '#f0ede5',
      seleccion: '#f0ede5',
      exito: '#121212',
      peligro: '#121212',
      advertencia: '#121212'
    }
  },
  {
    id: 'nxtpaper_eink_dark',
    papel: true,
    colores: {
      fondo: '#101214',
      fondoElevado: '#181a1d',
      superficie: '#181a1d',
      superficieAlt: '#202327',
      borde: '#eae6df',
      texto: '#eae6df',
      textoSec: '#cccccc',
      textoMuted: '#999999',
      acento: '#eae6df',
      acentoFuerte: '#ffffff',
      hover: '#202327',
      seleccion: '#202327',
      exito: '#eae6df',
      peligro: '#eae6df',
      advertencia: '#eae6df'
    }
  },
  {
    id: 'nxtpaper_sepia',
    papel: true,
    colores: {
      fondo: '#f3ebdc',
      fondoElevado: '#faf5ea',
      superficie: '#faf5ea',
      superficieAlt: '#ebe1cf',
      borde: '#3b2f22',
      texto: '#2e2418',
      textoSec: '#4a3d2e',
      textoMuted: '#66584a',
      acento: '#3b2f22',
      acentoFuerte: '#2e2418',
      hover: '#ebe1cf',
      seleccion: '#ebe1cf',
      exito: '#2e2418',
      peligro: '#2e2418',
      advertencia: '#2e2418'
    }
  },
  {
    id: 'zafiro_profundo',
    papel: false,
    colores: {
      fondo: '#090e17',
      fondoElevado: '#101827',
      superficie: '#101827',
      superficieAlt: '#19243a',
      borde: '#233550',
      texto: '#f8fafc',
      textoSec: '#94a3b8',
      textoMuted: '#768499',
      acento: '#38bdf8',
      acentoFuerte: '#0284c7',
      hover: '#19243a',
      seleccion: '#19243a',
      exito: '#10b981',
      peligro: '#ef4444',
      advertencia: '#f59e0b'
    }
  },
  {
    id: 'amatista_zen',
    papel: false,
    colores: {
      fondo: '#110d1a',
      fondoElevado: '#1b1429',
      superficie: '#1b1429',
      superficieAlt: '#261d3a',
      borde: '#3d2f5c',
      texto: '#faf5ff',
      textoSec: '#d8b4fe',
      textoMuted: '#9576b9',
      acento: '#c084fc',
      acentoFuerte: '#a855f7',
      hover: '#261d3a',
      seleccion: '#261d3a',
      exito: '#10b981',
      peligro: '#ef4444',
      advertencia: '#f59e0b'
    }
  },
  {
    id: 'atardecer_dorado',
    papel: false,
    colores: {
      fondo: '#140f0a',
      fondoElevado: '#1e1710',
      superficie: '#1e1710',
      superficieAlt: '#2c2117',
      borde: '#4a3826',
      texto: '#fffbeb',
      textoSec: '#fde68a',
      textoMuted: '#b49b60',
      acento: '#f59e0b',
      acentoFuerte: '#d97706',
      hover: '#2c2117',
      seleccion: '#2c2117',
      exito: '#10b981',
      peligro: '#ef4444',
      advertencia: '#f59e0b'
    }
  },
  {
    id: 'cyberpunk_neon',
    papel: false,
    colores: {
      fondo: '#08080c',
      fondoElevado: '#11111a',
      superficie: '#11111a',
      superficieAlt: '#1b1b29',
      borde: '#2d2d44',
      texto: '#ffffff',
      textoSec: '#a5b4fc',
      textoMuted: '#7275f3',
      acento: '#00f5d4',
      acentoFuerte: '#00bb9f',
      hover: '#1b1b29',
      seleccion: '#1b1b29',
      exito: '#10b981',
      peligro: '#ef4444',
      advertencia: '#f59e0b'
    }
  },
  {
    id: 'bosque_boreal',
    papel: false,
    colores: {
      fondo: '#0a1310',
      fondoElevado: '#12201b',
      superficie: '#12201b',
      superficieAlt: '#192c26',
      borde: '#28463d',
      texto: '#ecfdf5',
      textoSec: '#a7f3d0',
      textoMuted: '#64907e',
      acento: '#34d399',
      acentoFuerte: '#059669',
      hover: '#192c26',
      seleccion: '#192c26',
      exito: '#10b981',
      peligro: '#ef4444',
      advertencia: '#f59e0b'
    }
  },
  {
    id: 'rubi_carmesi',
    papel: false,
    colores: {
      fondo: '#140a0d',
      fondoElevado: '#1f1015',
      superficie: '#1f1015',
      superficieAlt: '#2b171e',
      borde: '#4a2533',
      texto: '#fff1f2',
      textoSec: '#fecdd3',
      textoMuted: '#a7737d',
      acento: '#fb7185',
      acentoFuerte: '#e11d48',
      hover: '#2b171e',
      seleccion: '#2b171e',
      exito: '#10b981',
      peligro: '#ef4444',
      advertencia: '#f59e0b'
    }
  },
  {
    id: 'cerezo_sakura',
    papel: false,
    colores: {
      fondo: '#130d14',
      fondoElevado: '#1d141f',
      superficie: '#1d141f',
      superficieAlt: '#291c2c',
      borde: '#472e4c',
      texto: '#fdf2f8',
      textoSec: '#fbcfe8',
      textoMuted: '#a27592',
      acento: '#f472b6',
      acentoFuerte: '#db2777',
      hover: '#291c2c',
      seleccion: '#291c2c',
      exito: '#10b981',
      peligro: '#ef4444',
      advertencia: '#f59e0b'
    }
  },
  {
    id: 'bronce_antiguo',
    papel: false,
    colores: {
      fondo: '#14110e',
      fondoElevado: '#1e1914',
      superficie: '#1e1914',
      superficieAlt: '#29221c',
      borde: '#473a2f',
      texto: '#fef3c7',
      textoSec: '#deb887',
      textoMuted: '#97826a',
      acento: '#d97706',
      acentoFuerte: '#b45309',
      hover: '#29221c',
      seleccion: '#29221c',
      exito: '#10b981',
      peligro: '#ef4444',
      advertencia: '#f59e0b'
    }
  },
  {
    id: 'hielo_artico',
    papel: false,
    colores: {
      fondo: '#0b1319',
      fondoElevado: '#121e27',
      superficie: '#121e27',
      superficieAlt: '#1b2a37',
      borde: '#2a4255',
      texto: '#ecfeff',
      textoSec: '#a5f3fc',
      textoMuted: '#668b9a',
      acento: '#67e8f9',
      acentoFuerte: '#06b6d4',
      hover: '#1b2a37',
      seleccion: '#1b2a37',
      exito: '#10b981',
      peligro: '#ef4444',
      advertencia: '#f59e0b'
    }
  },
  {
    id: 'lavanda_calma',
    papel: false,
    colores: {
      fondo: '#f7f5fc',
      fondoElevado: '#ffffff',
      superficie: '#ffffff',
      superficieAlt: '#ede7f7',
      borde: '#d6c8f2',
      texto: '#1e1430',
      textoSec: '#554472',
      textoMuted: '#776693',
      acento: '#8452f5',
      acentoFuerte: '#6d28d9',
      hover: '#ede7f7',
      seleccion: '#ede7f7',
      exito: '#10b981',
      peligro: '#ef4444',
      advertencia: '#f59e0b'
    }
  },
  {
    id: 'menta_cristalina',
    papel: false,
    colores: {
      fondo: '#f0fdf8',
      fondoElevado: '#ffffff',
      superficie: '#ffffff',
      superficieAlt: '#e0f7ed',
      borde: '#c0ecdb',
      texto: '#0d251c',
      textoSec: '#356150',
      textoMuted: '#4e7968',
      acento: '#0c855d',
      acentoFuerte: '#065f46',
      hover: '#e0f7ed',
      seleccion: '#e0f7ed',
      exito: '#10b981',
      peligro: '#ef4444',
      advertencia: '#f59e0b'
    }
  },
  {
    id: 'duna_sahara',
    papel: false,
    colores: {
      fondo: '#16120d',
      fondoElevado: '#211b14',
      superficie: '#211b14',
      superficieAlt: '#2e261c',
      borde: '#4a3d2d',
      texto: '#fff7ed',
      textoSec: '#fed7aa',
      textoMuted: '#9e8062',
      acento: '#f97316',
      acentoFuerte: '#ea580c',
      hover: '#2e261c',
      seleccion: '#2e261c',
      exito: '#10b981',
      peligro: '#ef4444',
      advertencia: '#f59e0b'
    }
  },
  {
    id: 'nebulosa_cosmica',
    papel: false,
    colores: {
      fondo: '#0b0916',
      fondoElevado: '#151128',
      superficie: '#151128',
      superficieAlt: '#201a3c',
      borde: '#362c61',
      texto: '#eef2ff',
      textoSec: '#c7d2fe',
      textoMuted: '#7d87ba',
      acento: '#818cf8',
      acentoFuerte: '#6366f1',
      hover: '#201a3c',
      seleccion: '#201a3c',
      exito: '#10b981',
      peligro: '#ef4444',
      advertencia: '#f59e0b'
    }
  },
  {
    id: 'titanio_minimal',
    papel: false,
    colores: {
      fondo: '#0f1215',
      fondoElevado: '#181c21',
      superficie: '#181c21',
      superficieAlt: '#22272e',
      borde: '#313b45',
      texto: '#f4f4f5',
      textoSec: '#a1a1aa',
      textoMuted: '#86868e',
      acento: '#a3e635',
      acentoFuerte: '#84cc16',
      hover: '#22272e',
      seleccion: '#22272e',
      exito: '#10b981',
      peligro: '#ef4444',
      advertencia: '#f59e0b'
    }
  },
  {
    id: 'coral_abisal',
    papel: false,
    colores: {
      fondo: '#091216',
      fondoElevado: '#101e24',
      superficie: '#101e24',
      superficieAlt: '#182931',
      borde: '#27434f',
      texto: '#f0fdfa',
      textoSec: '#99f6e4',
      textoMuted: '#5d969b',
      acento: '#ff5e57',
      acentoFuerte: '#ff3f34',
      hover: '#182931',
      seleccion: '#182931',
      exito: '#10b981',
      peligro: '#ef4444',
      advertencia: '#f59e0b'
    }
  }
];
