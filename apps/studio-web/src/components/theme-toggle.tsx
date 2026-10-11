'use client'

import {
  FormField,
  IconButton,
  Popover,
  SegmentedControl,
  Select,
} from '@dazzlabs/dazzui'
import { MoonIcon, SunIcon } from 'lucide-react'

import { useI18n } from '#/lib/i18n'
import { themeLabel, themeNames, useTheme } from '#/lib/theme'
import type { Mode } from '#/lib/theme'

/** Appearance menu for the top-right header: light/dark mode plus the color
 * DazzUI theme. Client-only, like the app. */
export function ThemeToggle() {
  const { t } = useI18n()
  const { mode, theme, setMode, setTheme } = useTheme()

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
          <Select
            value={theme}
            onValueChange={setTheme}
            options={[
              {
                label: 'DazzUI',
                options: themeNames
                  .filter((name) => !name.startsWith('omarchy-'))
                  .map((name) => ({
                    value: name,
                    label: name === 'studio' ? t('Default') : themeLabel(name),
                  })),
              },
              {
                label: 'Omarchy',
                options: themeNames
                  .filter((name) => name.startsWith('omarchy-'))
                  .map((name) => ({
                    value: name,
                    label: themeLabel(name.slice(8)),
                  })),
              },
            ]}
          />
        </FormField>
      </div>
    </Popover>
  )
}
