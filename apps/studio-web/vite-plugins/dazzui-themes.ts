import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import type { Plugin } from 'vite'

/** Derive the menu from the installed DazzUI package without shipping a second
 * copy of its token stylesheet in JavaScript or maintaining a parallel list. */
export function dazzuiThemes(): Plugin {
  const id = 'virtual:dazzui-themes'
  return {
    name: 'dazzui-themes',
    resolveId(source) {
      if (source === id) return `\0${id}`
    },
    load(source) {
      if (source !== `\0${id}`) return
      const css = readFileSync(
        fileURLToPath(import.meta.resolve('@dazzlabs/dazzui/style.css')),
        'utf8',
      )
      // Studio light is DazzUI's :root default, without a separate selector.
      const names = new Set(['studio'])
      for (const match of css.matchAll(
        /\[data-theme=['"](.+?)-(?:light|dark)['"]\]/g,
      ))
        names.add(match[1])
      return `export const themeNames = ${JSON.stringify([...names])};`
    },
  }
}
