import { RouterLink } from '#/components/router-link'
import {
  Badge,
  Button,
  Card,
  EmptyState,
  PreferenceGroup,
  PreferenceSection,
  Table,
  TableRow,
  TableCell,
} from '@dazzlabs/dazzui'
import { PlusIcon } from 'lucide-react'

import type { LinkComponentProps } from '@tanstack/react-router'
import type { Target } from '#/lib/targets'
import { useI18n } from '#/lib/i18n'

/**
 * The list that replaces per-integration sidebar entries. Connected targets are
 * links; the rest are an inline catalog, so adding a target never changes nav.
 */
export function TargetList({
  targets,
  detailLink,
}: {
  targets: Array<Target>
  detailLink: (target: Target) => LinkComponentProps
}) {
  const { t } = useI18n()
  const connected = targets.filter((target) => target.connected)
  const available = targets.filter((target) => !target.connected)

  return (
    <div className="space-y-6">
      <PreferenceSection label={t('Connected')}>
        {connected.length === 0 ? (
          <EmptyState title={t('Nothing connected yet.')} />
        ) : (
          <Table>
            {connected.map((target) => (
              <TableRow key={target.id}>
                <TableCell className="min-w-0 flex-1">
                  <PreferenceGroup
                    title={
                      <RouterLink {...detailLink(target)}>
                        {t(target.name)}
                      </RouterLink>
                    }
                    description={target.detail ?? t(target.description)}
                  />
                </TableCell>
                <TableCell>
                  <Badge tint="success">{t('Connected')}</Badge>
                </TableCell>
              </TableRow>
            ))}
          </Table>
        )}
      </PreferenceSection>
      <PreferenceSection label={t('Available')}>
        <div className="grid gap-3 sm:grid-cols-2">
          {available.map((target) => (
            <Card key={target.id}>
              <PreferenceGroup
                title={t(target.name)}
                description={t(target.description)}
                action={
                  <Button variant="outlined" size="small">
                    <PlusIcon />
                    {t('Connect')}
                  </Button>
                }
              />
            </Card>
          ))}
        </div>
      </PreferenceSection>
    </div>
  )
}
