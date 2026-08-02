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
import { useState } from 'react'
import {
  Item,
  ItemContent,
  ItemDescription,
  ItemGroup,
  ItemHeader,
  ItemTitle,
} from '@/components/ui/item'

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
      {/* TODO 增加左侧其他描述，搜索框等 */}
      <div className="mb-4 flex justify-end">
        <AddAnimeButton />
      </div>
      <ItemGroup className="flex flex-row flex-wrap gap-4">
        {data.map((anime) => {
          return (
            <Item key={anime.id} className="w-48" variant={'outline'}>
              <ItemHeader>
                <img
                  src={anime.cover}
                  alt={`${anime.sn}封面`}
                  loading="lazy"
                  className="w-full rounded-md object-cover transition-transform duration-300"
                />
              </ItemHeader>
              <ItemContent>
                {/* TODO 解析动画名称 */}
                <ItemTitle>{anime.sn}</ItemTitle>
                <ItemDescription>暂无简介</ItemDescription>
              </ItemContent>
            </Item>
          )
        })}
      </ItemGroup>
    </div>
  )
}
