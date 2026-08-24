import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { createFileRoute } from '@tanstack/react-router'
import type { Anime } from '@ani-dock/shared-type'
import { Skeleton } from '@/components/ui/skeleton'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import {
  AlertCircle,
  AlertTriangle,
  DownloadIcon,
  Plus,
  RefreshCw,
  Search,
  SearchX,
  Tv,
} from 'lucide-react'
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
import { useDeferredValue, useMemo, useRef, useState } from 'react'
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
import { toast } from '@/components/ui/toast'
import { Spinner } from '@/components/ui/spinner'

export const Route = createFileRoute('/library')({
  component: RouteComponent,
})

async function importAnime(sn: number) {
  const resp = await fetch('/api/animes', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ sn }),
  })

  if (!resp.ok) {
    throw await resp.json()
  }
}

function AddAnimeButton() {
  const [open, setOpen] = useState(false)
  const [sn, setSn] = useState('')

  const queryClient = useQueryClient()

  const addAnime = useMutation({
    mutationFn: (literalSn: string) => importAnime(Number(literalSn)),

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
      const resp = await fetch('/api/episodes/download', {
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
      <DialogTrigger
        render={
          <Button
            size={'icon'}
            className={'rounded-full'}
            disabled={mutation.isPending}
            aria-label={`下载《${anime.name}》`}
          />
        }
      >
        {mutation.isPending ? <Spinner /> : <DownloadIcon />}
      </DialogTrigger>
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

function RefreshButton({ anime }: { anime: Anime }) {
  const queryClient = useQueryClient()
  const episodeSn = Object.values(anime.series)[0][0].sn
  const refreshAnime = useMutation({
    mutationFn: () => importAnime(episodeSn),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['animes'] })
      toast.add({
        title: `《${anime.name}》刷新完成`,
        type: 'success',
      })
    },
    onError: (error) => {
      toast.add({
        title: `《${anime.name}》刷新失败`,
        description: error.message,
        type: 'error',
      })
    },
  })

  return (
    <Button
      variant={'outline'}
      size={'icon'}
      className={'rounded-full'}
      onClick={() => refreshAnime.mutate()}
      disabled={refreshAnime.isPending}
      aria-label={`刷新《${anime.name}》`}
    >
      {refreshAnime.isPending ? <Spinner /> : <RefreshCw />}
    </Button>
  )
}

function RouteComponent() {
  const [searchQuery, setSearchQuery] = useState('')
  const deferredSearchQuery = useDeferredValue(searchQuery)
  const { data, isError, isLoading, error } = useQuery({
    queryKey: ['animes'],
    queryFn: async () => {
      const resp = await fetch('/api/animes')

      if (!resp.ok) {
        throw await resp.json()
      }

      return resp.json() as Promise<Anime[]>
    },
  })
  const trimmedSearchQuery = deferredSearchQuery.trim()
  const normalizedSearchQuery = trimmedSearchQuery.toLowerCase()
  const filteredAnimes = useMemo(() => {
    const animes = data ?? []

    if (!normalizedSearchQuery) {
      return animes
    }

    return animes.filter(
      (anime) =>
        anime.name.toLowerCase().includes(normalizedSearchQuery) ||
        anime.sn.toString().includes(normalizedSearchQuery)
    )
  }, [data, normalizedSearchQuery])
  const hasSearchQuery = trimmedSearchQuery.length > 0
  const hasNoSearchResults =
    Boolean(data?.length) && hasSearchQuery && filteredAnimes.length === 0

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
      <div className="mb-4 flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
        <div className="flex min-w-0 flex-1 flex-col gap-2 sm:flex-row sm:items-center">
          <div className="relative w-full sm:max-w-xs">
            <label className="sr-only" htmlFor="anime-library-search">
              搜索动画
            </label>
            <Search
              aria-hidden="true"
              className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground"
            />
            <Input
              id="anime-library-search"
              type="search"
              autoComplete="off"
              placeholder="按名称或 SN 搜索"
              value={searchQuery}
              onChange={(event) => setSearchQuery(event.target.value)}
              aria-controls="anime-library-results"
              aria-describedby="anime-library-result-count"
              className="pl-8"
            />
          </div>
          <p
            id="anime-library-result-count"
            aria-live="polite"
            className="shrink-0 text-sm text-muted-foreground"
          >
            {hasSearchQuery
              ? `显示 ${filteredAnimes.length} / ${data.length} 部动画`
              : `共 ${data.length} 部动画`}
          </p>
        </div>
        <div className="self-end md:self-auto">
          <AddAnimeButton />
        </div>
      </div>
      <div
        id="anime-library-results"
        aria-busy={searchQuery !== deferredSearchQuery}
      >
        {hasNoSearchResults ? (
          <Empty className="min-h-64 border">
            <EmptyHeader role="status">
              <EmptyMedia variant={'icon'}>
                <SearchX />
              </EmptyMedia>
              <EmptyTitle>未找到匹配的动画</EmptyTitle>
              <EmptyDescription>
                没有名称或 SN 与“{trimmedSearchQuery}”匹配的动画
              </EmptyDescription>
            </EmptyHeader>
            <EmptyContent>
              <Button variant={'outline'} onClick={() => setSearchQuery('')}>
                清除搜索
              </Button>
            </EmptyContent>
          </Empty>
        ) : (
          <ItemGroup className="flex flex-row flex-wrap gap-4">
            {filteredAnimes.map((anime) => {
              return (
                <Item key={anime.id} className="w-3xs" variant={'outline'}>
                  <ItemHeader>
                    <img
                      src={`/api/animes/${anime.id}/cover`}
                      alt={`${anime.sn}封面`}
                      loading="lazy"
                      className="w-full rounded-md object-cover transition-transform duration-300"
                    />
                  </ItemHeader>
                  <ItemContent>
                    <ItemTitle>{anime.name}</ItemTitle>
                    <ItemDescription>SN: {anime.sn}</ItemDescription>
                  </ItemContent>
                  <ItemActions>
                    <RefreshButton anime={anime} />
                    <DownloadButton anime={anime} />
                  </ItemActions>
                </Item>
              )
            })}
          </ItemGroup>
        )}
      </div>
    </div>
  )
}
