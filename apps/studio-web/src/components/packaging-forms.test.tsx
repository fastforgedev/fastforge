// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/react'
import { api, ApiError } from 'studio-api-client'
import { PackagingForms } from './packaging-forms'
import { I18nProvider } from '#/lib/i18n'
import { packagingForms } from '#/lib/packaging'

beforeEach(() => {
  window.localStorage.clear()
  vi.spyOn(api, 'getPackagingFile').mockImplementation(async (_, format) => ({
    path: `.fastforge/packaging/linux/${format}/template`,
    exists: true,
    content: packagingForms[format].template,
    etag: `${format}-original`,
  }))
})
afterEach(() => {
  cleanup()
  vi.restoreAllMocks()
})

it('keeps drafts across format switches and saves the native file with its concurrency token', async () => {
  const save = vi
    .spyOn(api, 'putPackagingFile')
    .mockImplementation(async (_, format, body) => ({
      path: `.fastforge/packaging/linux/${format}/control`,
      exists: true,
      content: body.content,
      etag: 'saved',
    }))
  render(
    <I18nProvider>
      <PackagingForms projectId="hello" />
    </I18nProvider>,
  )
  const input = await screen.findByRole<HTMLInputElement>('textbox', {
    name: /Package name/,
  })
  fireEvent.change(input, { target: { value: 'hello-studio' } })
  fireEvent.click(
    screen.getByRole<HTMLButtonElement>('button', { name: 'Pacman' }),
  )
  await screen.findByRole<HTMLInputElement>('textbox', { name: /Package base/ })
  fireEvent.click(
    screen.getByRole<HTMLButtonElement>('button', { name: 'DEB' }),
  )
  expect(
    screen.getByRole<HTMLInputElement>('textbox', { name: /Package name/ })
      .value,
  ).toBe('hello-studio')
  fireEvent.click(
    screen.getByRole<HTMLButtonElement>('button', { name: 'Save changes' }),
  )
  await screen.findByText('Saved', { exact: true })
  expect(save).toHaveBeenCalledWith('hello', 'deb', {
    content: packagingForms.deb.template.replace(
      '${PACKAGE_NAME}',
      'hello-studio',
    ),
    etag: 'deb-original',
  })
  expect(
    screen.getByRole<HTMLButtonElement>('button', { name: 'Save changes' })
      .disabled,
  ).toBe(true)
})

it('keeps the draft on a stale-file response and reloads only when requested', async () => {
  vi.spyOn(api, 'putPackagingFile').mockRejectedValue(
    new ApiError(409, 'PACKAGING_FILE_STALE', 'stale'),
  )
  render(
    <I18nProvider>
      <PackagingForms projectId="hello" />
    </I18nProvider>,
  )
  const input = await screen.findByRole<HTMLInputElement>('textbox', {
    name: /Package name/,
  })
  fireEvent.change(input, { target: { value: 'my-draft' } })
  fireEvent.click(
    screen.getByRole<HTMLButtonElement>('button', { name: 'Save changes' }),
  )
  expect((await screen.findByRole('alert')).textContent).toContain(
    'changed on disk',
  )
  expect(input.value).toBe('my-draft')
  fireEvent.click(
    screen.getByRole<HTMLButtonElement>('button', {
      name: 'Discard changes and reload',
    }),
  )
  await waitFor(() =>
    expect(
      screen.getByRole<HTMLInputElement>('textbox', {
        name: /Package name/,
      }).value,
    ).toBe('${PACKAGE_NAME}'),
  )
})

it('offers a template for missing files and never saves a failed read', async () => {
  vi.mocked(api.getPackagingFile).mockImplementation(async (_, format) => {
    if (format === 'pacman') throw new Error('Cannot read file')
    return { path: 'new/control', exists: false, content: '', etag: 'missing' }
  })
  const save = vi
    .spyOn(api, 'putPackagingFile')
    .mockImplementation(async (_, __, body) => ({
      path: 'new/control',
      exists: true,
      content: body.content,
      etag: 'saved',
    }))
  render(
    <I18nProvider>
      <PackagingForms projectId="hello" />
    </I18nProvider>,
  )
  fireEvent.click(await screen.findByRole('button', { name: 'Create file' }))
  await screen.findByText('Saved', { exact: true })
  expect(save).toHaveBeenCalledWith('hello', 'deb', {
    content: packagingForms.deb.template,
    etag: 'missing',
  })
  fireEvent.click(
    screen.getByRole<HTMLButtonElement>('button', { name: 'Pacman' }),
  )
  expect((await screen.findByRole('alert')).textContent).toBe(
    'Cannot read file',
  )
  expect(screen.queryByRole('button', { name: 'Create file' })).toBeNull()
})
