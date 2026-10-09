'use client'

import * as React from 'react'

import {
  FormField,
  IconButton,
  Popover,
  SegmentedControl,
  Select,
} from '@dazzlabs/dazzui'
import { MoonIcon, SunIcon } from 'lucide-react'

import { useI18n } from '#/lib/i18n'

const MODE_STORAGE_KEY = 'fastforge-studio-theme'
const THEME_STORAGE_KEY = 'fastforge-studio-theme-name'

type Mode = 'light' | 'dark'
const themes = [
  'studio',
  'bright',
  'frost',
  'graphite',
  'ember',
  'nocturne',
] as const
type ThemeName = (typeof themes)[number]

function initialMode(): Mode {
  const stored = window.localStorage.getItem(MODE_STORAGE_KEY)
  if (stored === 'dark' || stored === 'light') return stored
  return window.matchMedia('(prefers-color-scheme: dark)').matches
    ? 'dark'
    : 'light'
}

function initialTheme(): ThemeName {
  const stored = window.localStorage.getItem(THEME_STORAGE_KEY)
  return themes.includes(stored as ThemeName) ? (stored as ThemeName) : 'studio'
}

function apply(mode: Mode, theme: ThemeName) {
  const root = document.documentElement
  root.classList.toggle('dark', mode === 'dark')
  root.dataset.theme = `${theme}-${mode}`
  root.style.colorScheme = mode
  window.localStorage.setItem(MODE_STORAGE_KEY, mode)
  window.localStorage.setItem(THEME_STORAGE_KEY, theme)
}

/** Appearance menu for the top-right header: light/dark mode plus the color
 * DazzUI theme. Client-only, like the app. */
export function ThemeToggle() {
  const { t } = useI18n()
  const [mode, setMode] = React.useState(initialMode)
  const [theme, setTheme] = React.useState(initialTheme)

  React.useEffect(() => {
    apply(mode, theme)
  }, [mode, theme])

  return (
    <Popover
      align="end"
      title={t('Appearance')}
      width="16rem"
      trigger={
        <IconButton label={t('Toggle theme')} size="small">
          {mode === 'dark' ? <MoonIcon aria-hidden /> : <SunIcon aria-hidden />}
        </IconButton>
      }
    >
      <div className="space-y-4">
        <SegmentedControl<Mode>
          aria-label={t('Appearance')}
          stretch
          value={mode}
          onValueChange={setMode}
          items={[
            { value: 'light', label: t('Light') },
            { value: 'dark', label: t('Dark') },
          ]}
        />
        <FormField label={t('Theme')}>
          <Select<ThemeName>
            value={theme}
            onValueChange={setTheme}
            options={themes.map((name) => ({
              value: name,
              label:
                name === 'studio'
                  ? t('Default')
                  : name[0].toUpperCase() + name.slice(1),
            }))}
          />
        </FormField>
      </div>
    </Popover>
  )
}
