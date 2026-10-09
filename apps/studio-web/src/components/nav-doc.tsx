import {
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from '@dazzlabs/dazzui'
import { BookOpenIcon } from 'lucide-react'
import { useI18n } from '#/lib/i18n'

export function NavDoc() {
  const { t } = useI18n()
  return (
    <SidebarMenu>
      <SidebarMenuItem>
        <SidebarMenuButton
          size="large"
          tooltip={t('Documentation')}
          icon={<BookOpenIcon />}
          render={
            <a
              href="https://fastforge.dev/docs"
              target="_blank"
              rel="noreferrer"
            />
          }
        >
          <span>{t('Documentation')}</span>
        </SidebarMenuButton>
      </SidebarMenuItem>
    </SidebarMenu>
  )
}
