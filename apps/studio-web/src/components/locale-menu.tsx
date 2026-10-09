import { IconButton, Menu } from '@dazzlabs/dazzui'
import { LanguagesIcon } from 'lucide-react'
import { useI18n } from '#/lib/i18n'

/** Language picker for the header. */
export function LocaleMenu() {
  const { locale, setLocale, t } = useI18n()
  const languages = [
    { value: 'en', label: 'English' },
    { value: 'zh-CN', label: '简体中文' },
    { value: 'ja', label: '日本語' },
    { value: 'ko', label: '한국어' },
  ] as const
  return (
    <Menu
      align="end"
      trigger={
        <IconButton label={t('Language')} size="small">
          <LanguagesIcon />
        </IconButton>
      }
      items={languages.map((language) => ({
        label: t(language.label),
        checked: locale === language.value,
        onSelect: () => setLocale(language.value),
      }))}
    />
  )
}
