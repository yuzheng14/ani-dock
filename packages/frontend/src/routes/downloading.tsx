import {
  Alert,
  AlertAction,
  AlertDescription,
  AlertTitle,
} from '@/components/ui/alert'
import { Button } from '@/components/ui/button'
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
import {
  useDownloadEvent,
  type DownloadEventPayloadError,
  type DownloadEventStreamStatus,
} from '@/hooks/use-download-event'
import type { DownloadEvent } from '@ani-dock/shared-type'
import { createFileRoute, Link } from '@tanstack/react-router'
import {
  HardDriveDownload,
  ImageOff,
  LoaderCircle,
  RefreshCw,
  TriangleAlert,
  WifiOff,
} from 'lucide-react'
import { useState, type ComponentProps, type ReactNode } from 'react'

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

function DownloadStreamNotice({
  status,
  payloadError,
  onReconnect,
}: {
  status: DownloadEventStreamStatus
  payloadError: DownloadEventPayloadError | null
  onReconnect: () => void
}) {
  if (status === 'failed') {
    return (
      <Alert variant="destructive" className="mb-4 shrink-0">
        <WifiOff />
        <AlertTitle>实时下载状态连接失败</AlertTitle>
        <AlertDescription>
          以下为最后一次收到的状态，页面仍在尝试自动重连。
        </AlertDescription>
        <AlertAction>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={onReconnect}
          >
            重新连接
          </Button>
        </AlertAction>
      </Alert>
    )
  }

  if (payloadError) {
    return (
      <Alert variant="destructive" className="mb-4 shrink-0">
        <TriangleAlert />
        <AlertTitle>收到异常下载状态</AlertTitle>
        <AlertDescription>
          {payloadError.event === 'snapshot' ? '任务快照' : '任务更新'}
          已被忽略：{payloadError.message}
        </AlertDescription>
        <AlertAction>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={onReconnect}
          >
            重新连接
          </Button>
        </AlertAction>
      </Alert>
    )
  }

  if (status === 'reconnecting') {
    return (
      <Alert className="mb-4 shrink-0">
        <RefreshCw className="animate-spin" />
        <AlertTitle>实时连接已中断，正在自动重连</AlertTitle>
        <AlertDescription>以下为最后一次收到的下载状态。</AlertDescription>
      </Alert>
    )
  }

  if (status === 'connecting') {
    return (
      <Alert className="mb-4 shrink-0">
        <LoaderCircle className="animate-spin" />
        <AlertTitle>正在连接实时下载状态</AlertTitle>
      </Alert>
    )
  }

  return null
}

function RouteComponent() {
  const {
    downloadEvents,
    streamStatus,
    hasSnapshot,
    streamPayloadError,
    reconnect,
  } = useDownloadEvent()

  let content: ReactNode

  if (!hasSnapshot) {
    content = (
      <Empty>
        <EmptyHeader>
          <EmptyMedia variant="icon">
            <LoaderCircle className="animate-spin" />
          </EmptyMedia>
          <EmptyTitle>
            {streamStatus === 'failed'
              ? '尚未获取下载状态'
              : '正在同步下载任务'}
          </EmptyTitle>
          <EmptyDescription>
            连接恢复后会自动获取完整的下载任务快照
          </EmptyDescription>
        </EmptyHeader>
      </Empty>
    )
  } else if (!downloadEvents.length) {
    content = (
      <Empty>
        <EmptyHeader>
          <EmptyMedia variant="icon">
            <HardDriveDownload />
          </EmptyMedia>
          <EmptyTitle>暂无下载任务</EmptyTitle>
          <EmptyDescription>
            在<Link to="/library">动画库</Link>中选择剧集开始下载
          </EmptyDescription>
        </EmptyHeader>
      </Empty>
    )
  } else {
    content = (
      <>
        <div className="mb-4">
          <p className="text-muted-foreground">
            共 {downloadEvents.length} 个任务
          </p>
        </div>
        <ItemGroup className="gap-2">
          {downloadEvents.map((de) => (
            <Item key={de.episode.id} variant="outline" role="listitem">
              <ItemMedia variant="image">
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
      </>
    )
  }

  return (
    <div className="flex size-full flex-col pr-2 pb-2">
      <DownloadStreamNotice
        status={streamStatus}
        payloadError={streamPayloadError}
        onReconnect={reconnect}
      />
      {content}
    </div>
  )
}
