// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/react'
import {
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarTrigger,
} from '@dazzlabs/dazzui'
import { StudioSidebar, StudioSidebarProvider } from './studio-sidebar'
import { LocaleMenu } from './locale-menu'
import { I18nProvider } from '#/lib/i18n'
import { ThemeToggle } from './theme-toggle'

function mockViewport(mobile: boolean) {
  vi.stubGlobal(
    'matchMedia',
    vi.fn().mockImplementation(() => ({
      matches: mobile,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    })),
  )
  Object.defineProperty(window, 'innerWidth', {
    configurable: true,
    value: mobile ? 390 : 1280,
  })
}

beforeEach(() => {
  mockViewport(false)
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  )
  window.localStorage.clear()
})
afterEach(() => {
  cleanup()
  vi.unstubAllGlobals()
  document.documentElement.removeAttribute('data-theme')
  document.documentElement.classList.remove('dark')
  document.documentElement.style.colorScheme = ''
})

describe('DazzUI integration', () => {
  it('collapses desktop navigation with the keyboard while preserving sidebar links', () => {
    render(
      <StudioSidebarProvider>
        <StudioSidebar collapsible="icon">
          <SidebarMenu>
            <SidebarMenuItem>
              <SidebarMenuButton render={<a href="/overview" />}>
                Overview
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
        </StudioSidebar>
        <SidebarTrigger />
      </StudioSidebarProvider>,
    )
    expect(
      screen
        .getByRole('link', { name: 'Overview' })
        .classList.contains('dz-sidebar__menu-button'),
    ).toBe(true)
    const toggle = screen.getByRole('button', { name: 'Toggle sidebar' })
    expect(toggle.getAttribute('aria-expanded')).toBe('true')
    fireEvent.keyDown(window, { key: 'b', ctrlKey: true })
    expect(toggle.getAttribute('aria-expanded')).toBe('false')
  })

  it('opens mobile navigation in an accessible drawer and dismisses with Escape', async () => {
    mockViewport(true)
    render(
      <StudioSidebarProvider>
        <StudioSidebar>
          <p>Project navigation</p>
        </StudioSidebar>
        <SidebarTrigger />
      </StudioSidebarProvider>,
    )
    const toggle = screen.getByRole('button', { name: 'Toggle sidebar' })
    expect(toggle.getAttribute('aria-expanded')).toBe('false')
    fireEvent.click(toggle)
    const drawer = await screen.findByRole('dialog', { name: 'Navigation' })
    expect(drawer.textContent).toContain('Project navigation')
    fireEvent.keyDown(drawer, { key: 'Escape' })
    await waitFor(() =>
      expect(toggle.getAttribute('aria-expanded')).toBe('false'),
    )
  })

  it('migrates a legacy theme preference and persists DazzUI theme and mode selections', async () => {
    window.localStorage.setItem('fastforge-studio-theme', 'dark')
    window.localStorage.setItem('fastforge-studio-theme-name', 'maple')
    render(
      <I18nProvider>
        <ThemeToggle />
      </I18nProvider>,
    )
    expect(document.documentElement.dataset.theme).toBe('studio-dark')
    fireEvent.click(screen.getByRole('button', { name: 'Toggle theme' }))
    fireEvent.click(await screen.findByRole('combobox', { name: 'Theme' }))
    const graphite = await screen.findByRole('option', { name: 'Graphite' })
    fireEvent.pointerDown(graphite, { pointerType: 'mouse' })
    fireEvent.click(graphite)
    await waitFor(() =>
      expect(document.documentElement.dataset.theme).toBe('graphite-dark'),
    )
    expect(window.localStorage.getItem('fastforge-studio-theme-name')).toBe(
      'graphite',
    )
    fireEvent.click(screen.getByRole('button', { name: 'Light' }))
    await waitFor(() =>
      expect(document.documentElement.dataset.theme).toBe('graphite-light'),
    )
    expect(document.documentElement.style.colorScheme).toBe('light')
    expect(window.localStorage.getItem('fastforge-studio-theme')).toBe('light')
  })

  it('selects and persists a language through the DazzUI menu', async () => {
    render(
      <I18nProvider>
        <LocaleMenu />
      </I18nProvider>,
    )
    fireEvent.click(screen.getByRole('button', { name: 'Language' }))
    fireEvent.click(
      await screen.findByRole('menuitemradio', { name: '简体中文' }),
    )
    await waitFor(() =>
      expect(screen.getByRole('button', { name: '语言' })).toBeTruthy(),
    )
  })
})
