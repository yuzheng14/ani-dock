import type { DownloadEvent } from '@ani-dock/shared-type'

const FAILURE_TIMEOUT_MS = 10_000
const EVENT_SOURCE_OPEN = 1
const EVENT_SOURCE_CLOSED = 2

export type DownloadEventStreamStatus =
  'connecting' | 'open' | 'reconnecting' | 'failed'

export type DownloadEventPayloadError = {
  event: 'snapshot' | 'update'
  message: string
}

type SubscribeDownloadEventStreamOptions = {
  source: EventSource
  onStatusChange: (status: DownloadEventStreamStatus) => void
  onSnapshot: (downloadEvents: DownloadEvent[]) => void
  onUpdate: (downloadEvent: DownloadEvent) => void
  onPayloadError: (error: DownloadEventPayloadError | null) => void
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function isDownloadEvent(value: unknown): value is DownloadEvent {
  if (!isRecord(value) || !isRecord(value.episode) || !isRecord(value.state)) {
    return false
  }

  return (
    typeof value.episode.id === 'string' &&
    typeof value.episode.sn === 'number' &&
    typeof value.episode.episode === 'number' &&
    (typeof value.state.Err === 'string' || isRecord(value.state.Ok))
  )
}

function parseSnapshot(data: string): DownloadEvent[] {
  const value: unknown = JSON.parse(data)

  if (!Array.isArray(value) || !value.every(isDownloadEvent)) {
    throw new Error('snapshot 数据结构无效')
  }

  return value
}

function parseUpdate(data: string): DownloadEvent {
  const value: unknown = JSON.parse(data)

  if (!isDownloadEvent(value)) {
    throw new Error('update 数据结构无效')
  }

  return value
}

function toPayloadError(
  event: DownloadEventPayloadError['event'],
  error: unknown
): DownloadEventPayloadError {
  return {
    event,
    message: error instanceof Error ? error.message : '未知解析错误',
  }
}

export function subscribeDownloadEventStream({
  source,
  onStatusChange,
  onSnapshot,
  onUpdate,
  onPayloadError,
}: SubscribeDownloadEventStreamOptions) {
  let failureTimer: ReturnType<typeof setTimeout> | undefined
  let failureReported = false

  function clearFailureTimer() {
    clearTimeout(failureTimer)
    failureTimer = undefined
  }

  function armFailureTimer() {
    if (failureTimer !== undefined) {
      return
    }

    failureTimer = setTimeout(() => {
      failureTimer = undefined
      if (source.readyState !== EVENT_SOURCE_OPEN) {
        failureReported = true
        onStatusChange('failed')
      }
    }, FAILURE_TIMEOUT_MS)
  }

  function openHandler() {
    clearFailureTimer()
    failureReported = false
    onStatusChange('open')
  }

  function errorHandler() {
    if (source.readyState === EVENT_SOURCE_CLOSED) {
      clearFailureTimer()
      failureReported = true
      onStatusChange('failed')
      return
    }

    if (!failureReported) {
      onStatusChange('reconnecting')
      armFailureTimer()
    }
  }

  function snapshotHandler(this: EventSource, event: MessageEvent<string>) {
    try {
      onSnapshot(parseSnapshot(event.data))
      onPayloadError(null)
    } catch (error) {
      onPayloadError(toPayloadError('snapshot', error))
    }
  }

  function updateHandler(this: EventSource, event: MessageEvent<string>) {
    try {
      onUpdate(parseUpdate(event.data))
    } catch (error) {
      onPayloadError(toPayloadError('update', error))
    }
  }

  armFailureTimer()
  source.addEventListener('open', openHandler)
  source.addEventListener('error', errorHandler)
  source.addEventListener('snapshot', snapshotHandler)
  source.addEventListener('update', updateHandler)

  return () => {
    clearFailureTimer()
    source.removeEventListener('open', openHandler)
    source.removeEventListener('error', errorHandler)
    source.removeEventListener('snapshot', snapshotHandler)
    source.removeEventListener('update', updateHandler)
    source.close()
  }
}
