import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
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
import { toast } from '@/components/ui/toast'
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import {
  observeDownloadEventStream,
  type DownloadEventStreamStatus,
} from '@/lib/download-event-stream'
import type { DownloadEvent } from '@ani-dock/shared-type'
import { createFileRoute, Link } from '@tanstack/react-router'
import {
  CircleAlert,
  CircleCheck,
  HardDriveDownload,
  ImageOff,
  LoaderCircle,
  RefreshCw,
} from 'lucide-react'
import { useEffect, useState, type ComponentProps, type ReactNode } from 'react'

const STREAM_FAILURE_TOAST_ID = 'download-event-stream-failure'
const PAYLOAD_ERROR_TOAST_ID = 'download-event-payload-error'

export const Route = createFileRoute('/downloading')({
  component: RouteComponent,
})

function DownloadStatus({ de }: { de: DownloadEvent }) {
  if ('Err' in de.state) {
    const message = `下载失败，原因：${de.state.Err}`

    return (
      <Tooltip>
        <TooltipTrigger
          render={
            <span
              className="block cursor-help [overflow-wrap:anywhere] text-destructive"
              tabIndex={0}
            />
          }
        >
          {message}
        </TooltipTrigger>
        <TooltipContent className="max-w-sm [overflow-wrap:anywhere] whitespace-pre-wrap">
          {message}
        </TooltipContent>
      </Tooltip>
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
  const { downloadEvents, streamStatus } = useDownloadEvent()

  return (
    <div className="pr-2 pb-2">
      <div
        className={`mb-4 flex items-center gap-4 ${
          downloadEvents.length ? 'justify-between' : 'justify-end'
        }`}
      >
        {downloadEvents.length > 0 && (
          <p className="text-muted-foreground">
            共 {downloadEvents.length} 个任务
          </p>
        )}
        <DownloadStreamStatus status={streamStatus} />
      </div>
      {streamStatus === 'failed' && (
        <Alert variant="destructive" className="mb-4">
          <CircleAlert />
          <AlertTitle>下载状态连接异常</AlertTitle>
          <AlertDescription>
            当前任务状态可能不是最新，页面仍会继续尝试重新连接。
          </AlertDescription>
        </Alert>
      )}
      {downloadEvents.length ? (
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
      ) : (
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
      )}
    </div>
  )
}

function DownloadStreamStatus({
  status,
}: {
  status: DownloadEventStreamStatus
}) {
  let icon: ReactNode
  let label: string

  switch (status) {
    case 'connecting':
      icon = <LoaderCircle className="size-3.5 animate-spin" />
      label = '正在连接实时状态'
      break
    case 'open':
      icon = <CircleCheck className="size-3.5 text-emerald-600" />
      label = '实时状态已连接'
      break
    case 'reconnecting':
      icon = <RefreshCw className="size-3.5 animate-spin" />
      label = '正在重新连接'
      break
    case 'failed':
      icon = <CircleAlert className="size-3.5" />
      label = '实时状态连接异常'
      break
  }

  return (
    <div
      role="status"
      className={`flex items-center gap-1.5 text-xs ${
        status === 'failed' ? 'text-destructive' : 'text-muted-foreground'
      }`}
    >
      {icon}
      <span>{label}</span>
    </div>
  )
}

function useDownloadEvent() {
  const [downloadEventMap, setDownloadEventMap] = useState<
    Map<number, DownloadEvent>
  >(() => new Map())
  const [streamStatus, setStreamStatus] =
    useState<DownloadEventStreamStatus>('connecting')

  const downloadEvents = Array.from(downloadEventMap.values())

  useEffect(() => {
    const es = new EventSource('/api/episodes/download/events')
    const stopObserving = observeDownloadEventStream({
      source: es,
      onSnapshot: (events) => {
        setDownloadEventMap(
          new Map(events.map((event) => [event.episode.sn, event]))
        )
      },
      onUpdate: (event) => {
        setDownloadEventMap((previous) =>
          new Map(previous).set(event.episode.sn, event)
        )
      },
      onStatusChange: (status) => {
        setStreamStatus(status)
        if (status === 'open') {
          toast.close(STREAM_FAILURE_TOAST_ID)
        }
      },
      onPayloadError: (error) => {
        console.error('Failed to parse download event stream payload', error)
        toast.add({
          id: PAYLOAD_ERROR_TOAST_ID,
          title: '下载状态数据异常',
          description: `收到无法识别的 ${error.messageType} 消息，已保留现有任务状态。`,
          type: 'error',
          timeout: 8_000,
        })
      },
      onFailure: () => {
        toast.add({
          id: STREAM_FAILURE_TOAST_ID,
          title: '下载状态连接异常',
          description: '长时间无法获取最新进度，页面会继续尝试重新连接。',
          type: 'error',
          timeout: 0,
        })
      },
    })

    return () => {
      stopObserving()
      toast.close(STREAM_FAILURE_TOAST_ID)
    }
  }, [])

  return {
    downloadEvents,
    streamStatus,
  }
}
