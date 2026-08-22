import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from '@/components/ui/empty'
import {
  Item,
  ItemContent,
  ItemDescription,
  ItemGroup,
  ItemMedia,
  ItemTitle,
} from '@/components/ui/item'
import {
  Progress,
  ProgressLabel,
  ProgressValue,
} from '@/components/ui/progress'
import type { DownloadEvent } from '@ani-dock/shared-type'
import { createFileRoute, Link } from '@tanstack/react-router'
import { HardDriveDownload, ImageOff } from 'lucide-react'
import { useEffect, useState, type ComponentProps, type ReactNode } from 'react'

export const Route = createFileRoute('/downloading')({
  component: RouteComponent,
})

function DownloadStatus({ de }: { de: DownloadEvent }) {
  if ('Err' in de.state) {
    return (
      <span className="text-destructive">下载失败，原因：{de.state.Err}</span>
    )
  }

  switch (de.state.Ok.type) {
    case 'PENDING': {
      return '排队中'
    }
    case 'PREPARING': {
      return '前置准备中'
    }
    case 'WAITING_FOR_ADS': {
      return '等待广告'
    }
    case 'RESOLVE_MEDIA_RESOURCE': {
      return '解析媒体资源'
    }
    case 'DOWNLOADING_SEGMENTS': {
      return (
        <Progress
          value={
            de.state.Ok.total > 0
              ? (de.state.Ok.completed / de.state.Ok.total) * 100
              : 0
          }
        >
          <ProgressLabel>下载片段中</ProgressLabel>
          <ProgressValue />
        </Progress>
      )
    }
    case 'MERGING': {
      return '合并中'
    }
    case 'FINALIZING': {
      return '收尾中'
    }
    case 'COMPLETED': {
      return '下载完成'
    }
  }
}

function ImageWithFallback(
  props: ComponentProps<'img'> & { fallback: ReactNode }
) {
  const { fallback, ...imageProps } = props
  const [failed, setFailed] = useState(false)

  if (failed) {
    return fallback
  }

  return (
    <img
      {...imageProps}
      onError={(ev) => {
        setFailed(true)
        imageProps.onError?.(ev)
      }}
    />
  )
}

function RouteComponent() {
  const { downloadEvents } = useDownloadEvent()

  if (!downloadEvents.length) {
    return (
      <Empty>
        <EmptyHeader>
          <EmptyMedia variant={'icon'}>
            <HardDriveDownload />
          </EmptyMedia>
          <EmptyTitle>暂无下载任务</EmptyTitle>
          <EmptyDescription>
            在<Link to="/library">动画库</Link>中选择剧集开始下载
          </EmptyDescription>
        </EmptyHeader>
      </Empty>
    )
  }

  return (
    <div className="pr-2 pb-2">
      <div className="mb-4">
        <p className="text-muted-foreground">
          共 {downloadEvents.length} 个任务
        </p>
      </div>
      <ItemGroup className="gap-2">
        {downloadEvents.map((de) => (
          <Item key={de.episode.id} variant={'outline'} role="listitem">
            <ItemMedia variant={'image'}>
              <ImageWithFallback
                src={`/api/episodes/${de.episode.id}/cover`}
                width={32}
                height={32}
                className="object-cover"
                fallback={<ImageOff className="size-4" />}
              />
            </ItemMedia>
            <ItemContent>
              <ItemTitle className="line-clamp-1">
                第 {de.episode.episode} 集 - {de.episode.sn}
              </ItemTitle>
              <ItemDescription>
                <DownloadStatus de={de} />
              </ItemDescription>
            </ItemContent>
          </Item>
        ))}
      </ItemGroup>
    </div>
  )
}

function useDownloadEvent() {
  const [downloadEventMap, setDownloadEventMap] = useState<
    Map<number, DownloadEvent>
  >(() => new Map())

  const downloadEvents = Array.from(downloadEventMap.values())

  useEffect(() => {
    const es = new EventSource('/api/episodes/download/events')
    function snapshotHandler(this: EventSource, ev: MessageEvent) {
      const dataEvents = JSON.parse(ev.data) as DownloadEvent[]
      setDownloadEventMap(new Map(dataEvents.map((de) => [de.episode.sn, de])))
    }
    function updateHandler(this: EventSource, ev: MessageEvent) {
      const de = JSON.parse(ev.data) as DownloadEvent
      setDownloadEventMap((prev) => new Map(prev).set(de.episode.sn, de))
    }
    es.addEventListener('snapshot', snapshotHandler)
    es.addEventListener('update', updateHandler)
    return () => {
      es.removeEventListener('snapshot', snapshotHandler)
      es.removeEventListener('update', updateHandler)
      es.close()
    }
  }, [])

  return {
    downloadEvents,
  }
}
