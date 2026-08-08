import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { createFileRoute } from '@tanstack/react-router'
import type { Anime } from '@ani-dock/shared-type'
import { Skeleton } from '@/components/ui/skeleton'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import { AlertCircle, AlertTriangle, Plus, Tv } from 'lucide-react'
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from '@/components/ui/empty'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog'
import { Field, FieldLabel, FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { useRef, useState } from 'react'
import {
  Item,
  ItemActions,
  ItemContent,
  ItemDescription,
  ItemGroup,
  ItemHeader,
  ItemTitle,
} from '@/components/ui/item'
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group'
import type { DialogRootActions } from '@base-ui/react'

export const Route = createFileRoute('/library')({
  component: RouteComponent,
})

function AddAnimeButton() {
  const [open, setOpen] = useState(false)
  const [sn, setSn] = useState('')

  const queryClient = useQueryClient()

  const addAnime = useMutation({
    mutationFn: async (literalSn: string) => {
      const sn = Number(literalSn)
      const resp = await fetch('/animes', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ sn }),
      })

      if (!resp.ok) {
        throw await resp.json()
      }
    },

    onSuccess: async () => {
      setOpen(false)
      setSn('')
      await queryClient.invalidateQueries({
        queryKey: ['animes'],
      })
    },
  })

  return (
    <Dialog
      open={open}
      onOpenChange={(open) => {
        setOpen(open)

        if (!open) {
          setSn('')
          addAnime.reset()
        }
      }}
    >
      <DialogTrigger render={<Button />}>
        <Plus />
        添加动画
      </DialogTrigger>
      <DialogContent
        className={'sm:max-w-sm'}
        render={
          <form
            onSubmit={(e) => {
              e.preventDefault()

              addAnime.mutate(sn)
            }}
          />
        }
      >
        <DialogHeader>
          <DialogTitle>添加动画</DialogTitle>
          <DialogDescription>填写动画任意一集的 sn 即可</DialogDescription>
        </DialogHeader>
        <FieldGroup>
          <Field>
            <FieldLabel htmlFor="sn">
              sn <span className="text-destructive">*</span>
            </FieldLabel>
            <Input
              id="sn"
              name="sn"
              placeholder="3499"
              value={sn}
              onChange={(e) => {
                setSn(e.target.value)
              }}
              required
              type="number"
            />
          </Field>
        </FieldGroup>
        {addAnime.isError && (
          <p className="text-sm text-destructive">{addAnime.error.message}</p>
        )}
        <DialogFooter>
          <DialogClose render={<Button variant={'outline'} />}>
            取消
          </DialogClose>
          <Button type="submit" disabled={addAnime.isPending}>
            {addAnime.isPending ? '正在添加...' : '提交'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function DownloadButton({ anime }: { anime: Anime }) {
  const selectedSn = useRef<Record<string, string[]>>({
    本篇: anime.series['本篇'].map((e) => e.sn.toString()) ?? [],
  })
  const mutation = useMutation({
    mutationFn: async (rowSns: string[]) => {
      const sns = rowSns.map((sn) => Number(sn))
      const resp = await fetch('/episodes/download', {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(sns),
      })

      if (!resp.ok) throw await resp.json()
    },

    onSuccess: () => {
      dialogRef.current?.close()
    },
  })
  const dialogRef = useRef<DialogRootActions | null>(null)
  return (
    <Dialog actionsRef={dialogRef}>
      <DialogTrigger render={<Button />}>下载</DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>选择要下载的剧集</DialogTitle>
          <DialogDescription>
            {anime.name}
            <br />
            选中要下载的剧集点击确认即可
          </DialogDescription>
        </DialogHeader>
        <FieldGroup>
          {Object.entries(anime.series).map(([name, episodes]) => (
            <Field key={name}>
              <FieldLabel>{name}</FieldLabel>
              <ToggleGroup
                variant={'outline'}
                defaultValue={
                  name === '本篇'
                    ? episodes.map((e) => e.sn.toString())
                    : undefined
                }
                multiple
                className={'flex-wrap'}
                onValueChange={(groupValue) =>
                  (selectedSn.current[name] = groupValue)
                }
              >
                {episodes.map((episode) => (
                  <ToggleGroupItem
                    value={episode.sn.toString()}
                    key={episode.id}
                  >
                    {episode.episode}
                  </ToggleGroupItem>
                ))}
              </ToggleGroup>
            </Field>
          ))}
        </FieldGroup>
        {mutation.isError && (
          <p className="text-sm text-destructive">{mutation.error.message}</p>
        )}
        <DialogFooter>
          <DialogClose render={<Button variant={'outline'} />}>
            取消
          </DialogClose>
          <Button
            type="submit"
            onClick={() => {
              mutation.mutate(Object.values(selectedSn.current).flat())
            }}
            disabled={mutation.isPending}
          >
            {mutation.isPending ? '正在提交...' : '确认'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function RouteComponent() {
  const { data, isError, isLoading, error } = useQuery({
    queryKey: ['animes'],
    queryFn: async () => {
      const resp = await fetch('/animes')

      if (!resp.ok) {
        throw await resp.json()
      }

      return resp.json() as Promise<Anime[]>
    },
  })

  if (isLoading) {
    return (
      <div className="size-full pr-2 pb-2">
        <Skeleton className="size-full" />
      </div>
    )
  }

  if (isError) {
    return (
      <Alert variant={'destructive'} className="max-w-md">
        <AlertCircle />
        <AlertTitle>发生错误</AlertTitle>
        <AlertDescription>{error.message}</AlertDescription>
      </Alert>
    )
  }

  if (!data) {
    return (
      <Alert variant={'destructive'} className="max-w-md">
        <AlertTriangle />
        <AlertTitle>非预期行为</AlertTitle>
        <AlertDescription>动画库请求成功但是无数据</AlertDescription>
      </Alert>
    )
  }

  if (!data.length) {
    return (
      <Empty>
        <EmptyHeader>
          <EmptyMedia variant={'icon'}>
            <Tv />
          </EmptyMedia>
          <EmptyTitle>暂无动画</EmptyTitle>
          <EmptyDescription>
            当前尚无解析的动画，请点击 `添加动画` 以解析动画
          </EmptyDescription>
        </EmptyHeader>
        <EmptyContent>
          <AddAnimeButton />
        </EmptyContent>
      </Empty>
    )
  }

  return (
    <div className="size-full pr-2 pb-2">
      {/* TODO 增加左侧搜索框等 */}
      <div className="mb-4 flex items-center justify-between">
        <p className="text-muted-foreground">共 {data.length} 部动画</p>
        <AddAnimeButton />
      </div>
      <ItemGroup className="flex flex-row flex-wrap gap-4">
        {data.map((anime) => {
          return (
            <Item key={anime.id} className="w-3xs" variant={'outline'}>
              <ItemHeader>
                <img
                  src={anime.cover}
                  alt={`${anime.sn}封面`}
                  loading="lazy"
                  className="w-full rounded-md object-cover transition-transform duration-300"
                />
              </ItemHeader>
              <ItemContent>
                <ItemTitle>{anime.name}</ItemTitle>
                {/* TODO 增加简介 */}
                <ItemDescription>暂无简介</ItemDescription>
              </ItemContent>
              <ItemActions>
                <DownloadButton anime={anime} />
              </ItemActions>
            </Item>
          )
        })}
      </ItemGroup>
    </div>
  )
}
