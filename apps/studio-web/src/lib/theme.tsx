import { createContext, useContext, useLayoutEffect, useState } from 'react'
import type { ReactNode } from 'react'
import { themeNames } from 'virtual:dazzui-themes'

export { themeNames }
export type Mode = 'light' | 'dark'
const MODE_KEY = 'fastforge-studio-theme'
const THEME_KEY = 'fastforge-studio-theme-name'

type Appearance = {
  mode: Mode
  theme: string
  setMode: (mode: Mode) => void
  setTheme: (theme: string) => void
}
const ThemeContext = createContext<Appearance | null>(null)

export function themeLabel(name: string) {
  const desktop: Record<string, string> = {
    macos15: 'macOS 15',
    macos27: 'macOS 27',
    windows11: 'Windows 11',
    kde: 'KDE',
  }
  return (
    desktop[name] ??
    name
      .split('-')
      .map((part) => part[0].toUpperCase() + part.slice(1))
      .join(' ')
  )
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [mode, setMode] = useState<Mode>(() => {
    if (typeof window === 'undefined') return 'light'
    const stored = window.localStorage.getItem(MODE_KEY)
    if (stored === 'light' || stored === 'dark') return stored
    return window.matchMedia('(prefers-color-scheme: dark)').matches
      ? 'dark'
      : 'light'
  })
  const [theme, setTheme] = useState(() => {
    if (typeof window === 'undefined') return 'studio'
    const stored = window.localStorage.getItem(THEME_KEY) ?? ''
    return themeNames.includes(stored) ? stored : 'studio'
  })
  useLayoutEffect(() => {
    document.documentElement.dataset.theme = `${theme}-${mode}`
    document.documentElement.classList.toggle('dark', mode === 'dark')
    document.documentElement.style.colorScheme = mode
    window.localStorage.setItem(MODE_KEY, mode)
    window.localStorage.setItem(THEME_KEY, theme)
  }, [mode, theme])
  return (
    <ThemeContext.Provider value={{ mode, theme, setMode, setTheme }}>
      {children}
    </ThemeContext.Provider>
  )
}

export function useTheme() {
  const value = useContext(ThemeContext)
  if (!value) throw new Error('useTheme must be used inside ThemeProvider')
  return value
}
