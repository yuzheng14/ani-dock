import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import { Button } from '@/components/ui/button'
import { Field, FieldLabel, FieldLegend, FieldSet } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Skeleton } from '@/components/ui/skeleton'
import { Switch } from '@/components/ui/switch'
import type { Settings } from '@ani-dock/shared-type'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { createFileRoute } from '@tanstack/react-router'
import { AlertCircle, CircleCheck } from 'lucide-react'
import { useEffect, useState } from 'react'

export const Route = createFileRoute('/settings')({
  component: RouteComponent,
})

function RouteComponent() {
  const queryClient = useQueryClient()

  const settingsQuery = useQuery({
    queryKey: ['settings'],
    queryFn: async (): Promise<Settings> => {
      const resp = await fetch('/api/settings')
      if (!resp.ok) throw await resp.json()
      return resp.json()
    },
  })

  const [form, setForm] = useState<Settings | null>(null)

  useEffect(() => {
    if (settingsQuery.data) setForm(settingsQuery.data)
  }, [settingsQuery.data])

  const mutation = useMutation({
    mutationFn: async (settings: Settings) => {
      const resp = await fetch('/api/settings', {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(settings),
      })

      if (!resp.ok) throw await resp.json()
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['settings'] })
    },
  })

  const skeleton = (
    <div className="flex flex-col gap-4 p-4">
      <Skeleton className="h-5 w-16" />
      {Array.from({ length: 4 }).map((_, i) => (
        <Skeleton key={i} className="h-8 w-full" />
      ))}
    </div>
  )

  if (settingsQuery.isLoading) {
    return skeleton
  }

  if (settingsQuery.isError) {
    return (
      <Alert variant={'destructive'} className="m-4">
        <AlertTitle>配置加载失败</AlertTitle>
        <AlertDescription>
          请刷新页面重试 {settingsQuery.error.message}
        </AlertDescription>
      </Alert>
    )
  }

  if (!form) {
    return skeleton
  }

  const set = <K extends keyof Settings>(key: K, value: Settings[K]) =>
    setForm((prev) => (prev ? { ...prev, [key]: value } : prev))

  return (
    <form
      className="p-4"
      onSubmit={(e) => {
        e.preventDefault()
        mutation.mutate(form)
      }}
    >
      <FieldSet className="w-full max-w-lg">
        <FieldLegend>
          <span className="text-2xl font-semibold">配置</span>
        </FieldLegend>

        {mutation.isSuccess && !mutation.isPending && (
          <Field>
            <Alert>
              <CircleCheck />
              <AlertTitle>更新成功</AlertTitle>
              <AlertDescription>配置已生效。</AlertDescription>
            </Alert>
          </Field>
        )}
        {mutation.isError && (
          <Field>
            <Alert variant={'destructive'}>
              <AlertCircle />
              <AlertTitle>更新失败</AlertTitle>
              <AlertDescription>{mutation.error.message}</AlertDescription>
            </Alert>
          </Field>
        )}

        <Field>
          <FieldLabel htmlFor="cookie">Cookie</FieldLabel>
          <Input
            id="cookie"
            value={form.cookie}
            onChange={(e) => set('cookie', e.target.value)}
          />
        </Field>

        <Field>
          <FieldLabel htmlFor="download-resolution">清晰度</FieldLabel>
          <Select
            id="download-resolution"
            name="download-resolution"
            value={form.downloadResolution}
            onValueChange={(value) => {
              if (value) set('downloadResolution', value)
            }}
          >
            <SelectTrigger>
              <SelectValue placeholder="选择清晰度" />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem value="1080">1080p(vip)</SelectItem>
                <SelectItem value="720">720p</SelectItem>
                <SelectItem value="540">540p</SelectItem>
                <SelectItem value="360">360p</SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </Field>

        <Field>
          <FieldLabel htmlFor="ua">请求 UA</FieldLabel>
          <Input
            id="ua"
            value={form.ua}
            onChange={(e) => set('ua', e.target.value)}
          />
        </Field>

        <Field>
          <FieldLabel htmlFor="proxy">代理</FieldLabel>
          <Input
            id="proxy"
            placeholder="http://127.0.0.1:7890"
            value={form.proxy ?? ''}
            onChange={(e) =>
              set('proxy', e.target.value === '' ? null : e.target.value)
            }
          />
        </Field>

        <Field>
          <FieldLabel htmlFor="ads-time">广告等待时间（秒）</FieldLabel>
          <Input
            id="ads-time"
            type="number"
            min={1}
            value={form.adsTime}
            onChange={(e) => set('adsTime', Number(e.target.value))}
          />
        </Field>

        <Field>
          <FieldLabel htmlFor="multi-downloading-segment">
            并发下载分段数
          </FieldLabel>
          <Input
            id="multi-downloading-segment"
            type="number"
            min={1}
            max={5}
            value={form.multiDownloadingSegment}
            onChange={(e) =>
              set('multiDownloadingSegment', Number(e.target.value))
            }
          />
        </Field>

        <Field orientation={'horizontal'}>
          <FieldLabel>锁定清晰度</FieldLabel>
          <Switch
            checked={form.lockResolution}
            onCheckedChange={(checked) => set('lockResolution', checked)}
          />
        </Field>

        <Field orientation={'horizontal'}>
          <FieldLabel>仅使用 VIP 下载</FieldLabel>
          <Switch
            checked={form.onlyUseVip}
            onCheckedChange={(checked) => set('onlyUseVip', checked)}
          />
        </Field>

        <Field>
          <Button type="submit">提交</Button>
        </Field>
      </FieldSet>
    </form>
  )
}
