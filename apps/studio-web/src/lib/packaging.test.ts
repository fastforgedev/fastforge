import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { packagingForms, parsePackaging, renderPackaging } from './packaging'
import type { PackagingFormat } from 'studio-api-client'

const files: Array<[PackagingFormat, string]> = [
  ['deb', 'deb/control'],
  ['pacman', 'pacman/PKGINFO'],
  ['desktop', 'shared/${APP_ID}.desktop'],
]

describe('native packaging forms', () => {
  it.each(files)(
    'round-trips the real hello_world %s template byte for byte',
    (format, path) => {
      const source = readFileSync(
        new URL(
          `../../../../examples/hello_world/.fastforge/packaging/linux/${path}`,
          import.meta.url,
        ),
        'utf8',
      )
      expect(
        renderPackaging(format, source, parsePackaging(format, source).values),
      ).toBe(source)
    },
  )

  it('edits Debian multiline descriptions, preserving variables, comments, unknown fields and CRLF', () => {
    const source =
      '# keep\r\nPackage: ${PACKAGE_NAME}\r\nX-Custom: value\r\nDescription: Summary\r\n Full description\r\n .\r\n Another paragraph\r\n\r\n'
    const parsed = parsePackaging('deb', source)
    expect(parsed.values.Description).toBe(
      'Summary\nFull description\n\nAnother paragraph',
    )
    const result = renderPackaging('deb', source, {
      ...parsed.values,
      Description: 'New summary\nFirst paragraph\n\nSecond paragraph',
      Homepage: '${APP_HOMEPAGE}',
    })
    expect(result).toBe(
      '# keep\r\nPackage: ${PACKAGE_NAME}\r\nX-Custom: value\r\nDescription: New summary\r\n First paragraph\r\n .\r\n Second paragraph\r\nHomepage: ${APP_HOMEPAGE}\r\n\r\n',
    )
  })

  it('edits repeated Pacman keys without losing comments or optional dependency descriptions', () => {
    const source =
      'pkgname = ${PACKAGE_NAME}\ndepend = gtk3\n# between dependencies\ndepend = libsecret\noptdepend = foo: optional feature\nx-custom = keep\n'
    const parsed = parsePackaging('pacman', source)
    expect(parsed.values.depend).toBe('gtk3\nlibsecret')
    const result = renderPackaging('pacman', source, {
      ...parsed.values,
      depend: 'gtk3\nopenssl',
      optdepend: '',
    })
    expect(result).toBe(
      'pkgname = ${PACKAGE_NAME}\ndepend = gtk3\ndepend = openssl\n# between dependencies\nx-custom = keep\n',
    )
  })

  it('edits only the desktop entry group and preserves localized names and action groups', () => {
    const source =
      '[Desktop Entry]\nName=${APP_DISPLAY_NAME}\nName[zh_CN]=应用\nExec=${APP_BINARY_NAME}\nX-Custom=keep\n\n[Desktop Action New]\nName=New window\nExec=app --new\n'
    const parsed = parsePackaging('desktop', source)
    const result = renderPackaging('desktop', source, {
      ...parsed.values,
      Name: 'Hello World',
      Icon: '${APP_ID}',
    })
    expect(result).toBe(
      '[Desktop Entry]\nName=Hello World\nName[zh_CN]=应用\nExec=${APP_BINARY_NAME}\nX-Custom=keep\n\nIcon=${APP_ID}\n[Desktop Action New]\nName=New window\nExec=app --new\n',
    )
  })

  it('refuses ambiguous documents and injected new fields', () => {
    expect(() =>
      parsePackaging('deb', 'Package: first\n\nPackage: second\n'),
    ).toThrow('paragraphs')
    expect(() =>
      parsePackaging('desktop', '[Desktop Entry]\nName=A\nName=B\n'),
    ).toThrow('Duplicate')
    expect(() => parsePackaging('desktop', '[Other]\nName=A\n')).toThrow(
      'Desktop Entry',
    )
    const source = packagingForms.deb.template
    expect(() =>
      renderPackaging('deb', source, {
        ...parsePackaging('deb', source).values,
        Package: 'app\nInjected: yes',
      }),
    ).toThrow('line break')
  })
})
