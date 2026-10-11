import type { PackagingFormat } from 'studio-api-client'

export type PackagingField = {
  key: string
  label: string
  required?: boolean
  multiline?: boolean
  repeated?: boolean
  hint?: string
}

export const packagingForms: Record<
  PackagingFormat,
  {
    title: string
    description: string
    fields: Array<PackagingField>
    template: string
  }
> = {
  deb: {
    title: 'Debian / Ubuntu',
    description: 'Package metadata and dependencies for .deb installers.',
    fields: [
      { key: 'Package', label: 'Package name', required: true },
      { key: 'Version', label: 'Version', required: true },
      { key: 'Architecture', label: 'Architecture', required: true },
      { key: 'Maintainer', label: 'Maintainer', required: true },
      { key: 'Section', label: 'Section' },
      { key: 'Priority', label: 'Priority' },
      { key: 'Homepage', label: 'Homepage' },
      {
        key: 'Depends',
        label: 'Dependencies',
        hint: 'Separate dependencies with commas.',
      },
      {
        key: 'Description',
        label: 'Description',
        required: true,
        multiline: true,
        hint: 'First line: summary. Remaining lines: full description.',
      },
    ],
    template:
      'Package: ${PACKAGE_NAME}\nVersion: ${PACKAGE_VERSION}\nArchitecture: ${PACKAGE_ARCH}\nMaintainer: ${APP_MAINTAINER}\nSection: utils\nPriority: optional\nHomepage: ${APP_HOMEPAGE}\nDepends: libgtk-3-0\nDescription: ${APP_DESCRIPTION}\n',
  },
  pacman: {
    title: 'Arch Linux',
    description: 'Package metadata and dependencies for Pacman packages.',
    fields: [
      { key: 'pkgname', label: 'Package name', required: true },
      { key: 'pkgbase', label: 'Package base' },
      { key: 'pkgver', label: 'Version', required: true },
      { key: 'pkgdesc', label: 'Description', required: true },
      { key: 'url', label: 'Homepage' },
      { key: 'packager', label: 'Maintainer' },
      { key: 'arch', label: 'Architecture', required: true },
      { key: 'license', label: 'Licenses', multiline: true, repeated: true },
      { key: 'depend', label: 'Dependencies', multiline: true, repeated: true },
      {
        key: 'optdepend',
        label: 'Optional dependencies',
        multiline: true,
        repeated: true,
      },
      { key: 'group', label: 'Groups', multiline: true, repeated: true },
    ],
    template:
      'pkgname = ${PACKAGE_NAME}\npkgbase = ${PACKAGE_NAME}\npkgver = ${PACKAGE_VERSION}\npkgdesc = ${APP_DESCRIPTION}\nurl = ${APP_HOMEPAGE}\npackager = ${APP_MAINTAINER}\narch = ${PACKAGE_ARCH}\nlicense = ${APP_LICENSE}\ndepend = gtk3\n',
  },
  desktop: {
    title: 'Linux desktop',
    description:
      'Launcher shared by Linux package formats, including DEB, RPM, Pacman and AppImage.',
    fields: [
      { key: 'Type', label: 'Entry type', required: true },
      { key: 'Name', label: 'Display name', required: true },
      { key: 'GenericName', label: 'Generic name' },
      { key: 'Comment', label: 'Description' },
      { key: 'Exec', label: 'Launch command', required: true },
      { key: 'Icon', label: 'Icon' },
      {
        key: 'Categories',
        label: 'Categories',
        hint: 'Separate values with semicolons, ending with a semicolon.',
      },
      {
        key: 'Keywords',
        label: 'Keywords',
        hint: 'Separate values with semicolons, ending with a semicolon.',
      },
      {
        key: 'StartupNotify',
        label: 'Startup notification',
        hint: 'true, false, or a template variable.',
      },
      {
        key: 'Terminal',
        label: 'Run in terminal',
        hint: 'true, false, or a template variable.',
      },
    ],
    template:
      '[Desktop Entry]\nType=Application\nName=${APP_DISPLAY_NAME}\nComment=${APP_DESCRIPTION}\nExec=${APP_BINARY_NAME}\nIcon=${APP_ID}\nCategories=Utility;\nStartupNotify=true\nTerminal=false\n',
  },
}

type Entry = { key: string; start: number; end: number; value: string }

/** Index only the fields the form owns. Unchanged bytes (including comments,
 * localized desktop names, other groups and unknown fields) stay untouched. */
export function parsePackaging(format: PackagingFormat, content: string) {
  const fields = packagingForms[format].fields
  const lines = content.match(/[^\r\n]*(?:\r\n|\n|$)/g)?.filter(Boolean) ?? []
  const entries: Array<Entry> = []
  const values: Record<string, string> = Object.fromEntries(
    fields.map((field) => [field.key, '']),
  )
  let section = ''
  let insertAt = lines.length
  let desktopSections = 0
  let debParagraphEnded = false
  let debStarted = false

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].replace(/\r?\n$/, '')
    if (format === 'desktop') {
      const header = /^\s*\[([^\]]+)\]\s*$/.exec(line)
      if (header) {
        if (section === 'Desktop Entry') insertAt = i
        section = header[1]
        if (section === 'Desktop Entry') desktopSections++
        continue
      }
      if (section !== 'Desktop Entry') continue
    }
    if (format === 'deb') {
      if (!line.trim()) {
        if (debStarted) debParagraphEnded = true
        continue
      }
      if (line.startsWith('#')) continue
      if (debParagraphEnded)
        throw new Error(
          'Multiple Debian control paragraphs cannot be edited with this form.',
        )
      debStarted = true
    }
    const match = (
      format === 'deb'
        ? /^([^:\s]+):[ \t]*(.*)$/
        : /^([^=\s]+)[ \t]*=[ \t]*(.*)$/
    ).exec(line)
    if (!match) continue
    const field = fields.find((item) => item.key === match[1])
    if (!field) continue
    if (!field.repeated && entries.some((entry) => entry.key === field.key)) {
      throw new Error(
        `Duplicate field: ${field.key}. Edit the file on disk before using this form.`,
      )
    }
    const start = i
    let value = match[2]
    if (format === 'deb') {
      while (i + 1 < lines.length && /^[ \t]/.test(lines[i + 1])) {
        const continuation = lines[++i].replace(/\r?\n$/, '').slice(1)
        value += `\n${continuation === '.' ? '' : continuation}`
      }
    }
    entries.push({ key: field.key, start, end: i + 1, value })
    values[field.key] =
      field.repeated &&
      entries.filter((entry) => entry.key === field.key).length > 1
        ? `${values[field.key]}\n${value}`
        : value
  }
  if (format === 'desktop' && desktopSections !== 1) {
    throw new Error(
      'Expected one [Desktop Entry] section. Edit the file on disk before using this form.',
    )
  }
  // Append Debian fields inside the existing paragraph, ahead of trailing blanks.
  if (format === 'deb') {
    while (insertAt > 0 && !lines[insertAt - 1].trim()) insertAt--
  }
  return { lines, entries, values, insertAt }
}

export function renderPackaging(
  format: PackagingFormat,
  source: string,
  values: Record<string, string>,
) {
  const parsed = parsePackaging(format, source)
  const eol = source.includes('\r\n') ? '\r\n' : '\n'
  const changed = packagingForms[format].fields.filter(
    (field) => values[field.key] !== parsed.values[field.key],
  )
  const replacements = new Map<string, string>()
  for (const field of changed) {
    const value = values[field.key] ?? ''
    if (
      value.includes('\r') ||
      value.includes('\0') ||
      (!field.multiline && value.includes('\n'))
    ) {
      throw new Error(`Invalid line break in ${field.key}`)
    }
    if (field.required && !value.trim())
      throw new Error(`${field.key} is required`)
    const parts = value.split('\n')
    const text = !value
      ? ''
      : format === 'deb'
        ? `${field.key}: ${parts[0]}${parts
            .slice(1)
            .map((part) => `${eol} ${part || '.'}`)
            .join('')}${eol}`
        : field.repeated
          ? parts
              .filter((part) => part.trim())
              .map((part) => `${field.key} = ${part}${eol}`)
              .join('')
          : `${field.key}=${value}${eol}`
    replacements.set(
      field.key,
      format === 'pacman' && !field.repeated
        ? `${field.key} = ${value}${eol}`
        : text,
    )
    if (!value) replacements.set(field.key, '')
  }
  let result = ''
  const used = new Set<string>()
  const append = () => {
    const additions = changed
      .filter(
        (field) => !parsed.entries.some((entry) => entry.key === field.key),
      )
      .map((field) => replacements.get(field.key) ?? '')
      .join('')
    if (additions && result && !result.endsWith('\n')) result += eol
    result += additions
  }
  for (let i = 0; i <= parsed.lines.length; i++) {
    if (i === parsed.insertAt) append()
    if (i === parsed.lines.length) break
    const entry = parsed.entries.find((item) => item.start === i)
    if (entry && replacements.has(entry.key)) {
      if (!used.has(entry.key)) result += replacements.get(entry.key)
      used.add(entry.key)
      i = entry.end - 1
    } else {
      result += parsed.lines[i]
    }
  }
  return result
}
