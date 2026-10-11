import { useCallback, useEffect, useState } from 'react'
import type { ChangeEvent } from 'react'
import {
  ActionBar,
  Badge,
  Button,
  Callout,
  Card,
  Code,
  CodeBlock,
  Collapsible,
  Divider,
  Form,
  FormField,
  PreferenceGroup,
  SegmentedControl,
  Spinner,
  TextField,
} from '@dazzlabs/dazzui'
import { FileCodeIcon, SaveIcon } from 'lucide-react'
import { ApiError, api } from 'studio-api-client'
import type { PackagingFile, PackagingFormat } from 'studio-api-client'

import { useI18n } from '#/lib/i18n'
import {
  packagingForms,
  parsePackaging,
  renderPackaging,
} from '#/lib/packaging'

const formats: Array<PackagingFormat> = ['deb', 'pacman', 'desktop']

export function PackagingForms({ projectId }: { projectId: string }) {
  const { t } = useI18n()
  const [format, setFormat] = useState<PackagingFormat>('deb')
  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <SegmentedControl
          aria-label={t('Package format')}
          value={format}
          onValueChange={setFormat}
          items={formats.map((value) => ({
            value,
            label:
              value === 'deb'
                ? 'DEB'
                : value === 'pacman'
                  ? 'Pacman'
                  : '.desktop',
          }))}
        />
        <span className="text-xs text-muted-foreground">
          {t('Saved directly to your project')}
        </span>
      </div>
      {formats.map((value) => (
        <div key={`${projectId}-${value}`} hidden={format !== value}>
          <PackagingEditor projectId={projectId} format={value} />
        </div>
      ))}
    </div>
  )
}

function PackagingEditor({
  projectId,
  format,
}: {
  projectId: string
  format: PackagingFormat
}) {
  const { t } = useI18n()
  const definition = packagingForms[format]
  const [file, setFile] = useState<PackagingFile | null>(null)
  const [source, setSource] = useState('')
  const [values, setValues] = useState<Record<string, string>>({})
  const [baseline, setBaseline] = useState<Record<string, string>>({})
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState('')
  const [saved, setSaved] = useState(false)
  const [revision, setRevision] = useState(0)

  const accept = useCallback(
    (next: PackagingFile) => {
      const content = next.exists ? next.content : definition.template
      const parsed = parsePackaging(format, content)
      setFile(next)
      setSource(content)
      setValues(parsed.values)
      setBaseline(parsed.values)
    },
    [definition, format],
  )

  useEffect(() => {
    let current = true
    setLoading(true)
    setFile(null)
    setError('')
    api
      .getPackagingFile(projectId, format)
      .then((next) => {
        if (current) accept(next)
      })
      .catch((reason: Error) => {
        if (current) setError(reason.message)
      })
      .finally(() => {
        if (current) setLoading(false)
      })
    return () => {
      current = false
    }
    // A reload deliberately discards the local draft; tabs keep their drafts mounted.
  }, [projectId, format, revision, accept])

  let preview = source
  let validation = ''
  try {
    if (file) preview = renderPackaging(format, source, values)
  } catch (reason) {
    validation = (reason as Error).message
  }
  const dirty =
    !!file &&
    (!file.exists ||
      Object.entries(values).some(([key, value]) => value !== baseline[key]))

  useEffect(() => {
    if (!dirty) return
    const warn = (event: BeforeUnloadEvent) => {
      event.preventDefault()
      event.returnValue = ''
    }
    window.addEventListener('beforeunload', warn)
    return () => {
      window.removeEventListener('beforeunload', warn)
    }
  }, [dirty])

  async function save() {
    if (!file || saving || validation) return
    setSaving(true)
    setError('')
    setSaved(false)
    try {
      accept(
        await api.putPackagingFile(projectId, format, {
          content: preview,
          etag: file.etag,
        }),
      )
      setSaved(true)
    } catch (reason) {
      setError(
        reason instanceof ApiError && reason.code === 'PACKAGING_FILE_STALE'
          ? t(
              'The file changed on disk. Copy your changes from the preview, then reload before saving.',
            )
          : (reason as Error).message,
      )
    } finally {
      setSaving(false)
    }
  }

  return (
    <div className="space-y-5">
      <PreferenceGroup
        title={
          <span role="heading" aria-level={2}>
            {t(definition.title)}{' '}
            {file && (
              <Badge variant="outlined" size="small">
                {t(
                  !file.exists
                    ? 'New file'
                    : dirty
                      ? 'Unsaved changes'
                      : 'On disk',
                )}
              </Badge>
            )}
          </span>
        }
        description={t(definition.description)}
        action={
          <Button
            disabled={loading || saving}
            onClick={() => {
              setSaved(false)
              setRevision((value) => value + 1)
            }}
          >
            {t(dirty ? 'Discard changes and reload' : 'Reload')}
          </Button>
        }
      >
        {file && <Code className="break-all">{file.path}</Code>}
      </PreferenceGroup>
      {error && (
        <Callout role="alert" tint="danger">
          {error}
        </Callout>
      )}
      {loading ? (
        <Callout
          icon={<Spinner size="small" label={t('Loading packaging file…')} />}
        >
          {t('Loading packaging file…')}
        </Callout>
      ) : (
        file && (
          <Form
            onSubmit={(event) => {
              event.preventDefault()
              void save()
            }}
          >
            <Card>
              <div className="grid gap-x-6 gap-y-5 lg:grid-cols-2">
                {definition.fields.map((field) => {
                  const hint =
                    field.hint ?? (field.repeated ? 'One value per line.' : '')
                  const inputProps = {
                    id: `packaging-${format}-${field.key}`,
                    name: field.key,
                    value: values[field.key] ?? '',
                    required: field.required,
                    disabled: saving,
                    mono: true,
                    className: 'w-full',
                    onChange: (
                      event: ChangeEvent<
                        HTMLInputElement | HTMLTextAreaElement
                      >,
                    ) => {
                      setSaved(false)
                      setValues((previous) => ({
                        ...previous,
                        [field.key]: event.target.value,
                      }))
                    },
                  }
                  return (
                    <FormField
                      key={field.key}
                      name={field.key}
                      label={
                        <>
                          {t(field.label)}
                          {field.required && <span aria-hidden="true"> *</span>}
                        </>
                      }
                      hint={
                        <>
                          <Code>{field.key}</Code>
                          {hint && <> · {t(hint)}</>}
                        </>
                      }
                      className={field.multiline ? 'lg:col-span-2' : undefined}
                    >
                      {field.multiline ? (
                        <TextField
                          {...inputProps}
                          multiline
                          rows={field.key === 'Description' ? 5 : 3}
                        />
                      ) : (
                        <TextField {...inputProps} />
                      )}
                    </FormField>
                  )
                })}
              </div>
            </Card>
            <Collapsible title={t('File preview')} icon={<FileCodeIcon />}>
              <Callout size="small">
                {t(
                  'Template variables, comments and other fields are preserved. Variables resolve when packaging.',
                )}
              </Callout>
              <CodeBlock
                title={file.path}
                code={preview}
                copyable
                className="max-h-96 overflow-auto"
              />
            </Collapsible>
            {validation && (
              <Callout role="alert" tint="danger">
                {validation}
              </Callout>
            )}
            <Divider />
            <ActionBar align="between" className="flex-wrap">
              <span>{t('Changes apply to future packages.')}</span>
              <ActionBar>
                {saved && (
                  <Badge role="status" tint="success">
                    {t('Saved')}
                  </Badge>
                )}
                <Button
                  type="submit"
                  variant="filled"
                  disabled={!dirty || saving || !!validation}
                >
                  {saving ? (
                    <Spinner onAccent label={t('Saving…')} />
                  ) : (
                    <SaveIcon />
                  )}
                  {t(
                    saving
                      ? 'Saving…'
                      : file.exists
                        ? 'Save changes'
                        : 'Create file',
                  )}
                </Button>
              </ActionBar>
            </ActionBar>
          </Form>
        )
      )}
    </div>
  )
}
