import type { ComponentProps } from 'react'
import * as Dazz from '@dazzlabs/dazzui'
import { createContext, useContext, useState } from 'react'
import { useIsMobile } from '#/hooks/use-mobile'
import { useI18n } from '#/lib/i18n'

const MobileContext = createContext(false)

/** Keep DazzUI's desktop sidebar and shortcut, with a drawer on narrow screens. */
export function StudioSidebarProvider({
  defaultOpen = true,
  open: controlledOpen,
  onOpenChange,
  className,
  ...props
}: ComponentProps<typeof Dazz.SidebarProvider>) {
  const isMobile = useIsMobile()
  const [desktopOpen, setDesktopOpen] = useState(defaultOpen)
  const [mobileOpen, setMobileOpen] = useState(false)
  return (
    <MobileContext.Provider value={isMobile}>
      <Dazz.SidebarProvider
        {...props}
        className={Dazz.cx('min-h-svh', className)}
        open={isMobile ? mobileOpen : (controlledOpen ?? desktopOpen)}
        onOpenChange={
          isMobile
            ? setMobileOpen
            : (next) => {
                setDesktopOpen(next)
                onOpenChange?.(next)
              }
        }
      />
    </MobileContext.Provider>
  )
}

export function useStudioSidebar() {
  return { ...Dazz.useSidebar(), isMobile: useContext(MobileContext) }
}

export function StudioSidebar({
  side = 'left',
  collapsible = 'offcanvas',
  children,
  ...props
}: ComponentProps<typeof Dazz.Sidebar>) {
  const { isMobile, open, setOpen } = useStudioSidebar()
  const { t } = useI18n()
  if (!isMobile || collapsible === 'none')
    return (
      <Dazz.Sidebar {...props} side={side} collapsible={collapsible}>
        {children}
      </Dazz.Sidebar>
    )
  return (
    <Dazz.Drawer
      open={open}
      onOpenChange={setOpen}
      side={side}
      title={t('Navigation')}
      footer={
        <Dazz.DrawerClose render={<Dazz.Button />}>
          {t('Close')}
        </Dazz.DrawerClose>
      }
    >
      <div
        className="contents"
        onClickCapture={(event) => {
          if (
            event.target instanceof Element &&
            event.target.closest('a[href]')
          )
            setOpen(false)
        }}
      >
        {children}
      </div>
    </Dazz.Drawer>
  )
}
