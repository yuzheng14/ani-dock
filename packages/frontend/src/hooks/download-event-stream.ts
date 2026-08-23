import type { DownloadEvent } from '@ani-dock/shared-type'

export const DEFAULT_FAILURE_TIMEOUT_MS = 10_000

const EVENT_SOURCE_OPEN = 1
const EVENT_SOURCE_CLOSED = 2

export type DownloadEventStreamStatus =
  'connecting' | 'open' | 'reconnecting' | 'failed'

export type DownloadEventPayloadError = {
  event: 'snapshot' | 'update'
  message: string
}

type TimerScheduler = {
  setTimeout: (callback: () => void, delay: number) => unknown
  clearTimeout: (timer: unknown) => void
}

type SubscribeDownloadEventStreamOptions = {
  source: EventSource
  onStatusChange: (status: DownloadEventStreamStatus) => void
  onSnapshot: (downloadEvents: DownloadEvent[]) => void
  onUpdate: (downloadEvent: DownloadEvent) => void
  onPayloadError: (error: DownloadEventPayloadError | null) => void
  failureTimeoutMs?: number
  scheduler?: TimerScheduler
}

const defaultScheduler: TimerScheduler = {
  setTimeout: (callback, delay) => setTimeout(callback, delay),
  clearTimeout: (timer) => clearTimeout(timer as ReturnType<typeof setTimeout>),
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function isNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value)
}

function isEpisodeDownloadEvent(value: unknown): boolean {
  if (!isRecord(value) || typeof value.type !== 'string') {
    return false
  }

  switch (value.type) {
    case 'PENDING':
    case 'PREPARING':
    case 'WAITING_FOR_ADS':
    case 'RESOLVE_MEDIA_RESOURCE':
    case 'MERGING':
    case 'FINALIZING':
    case 'COMPLETED': {
      return true
    }
    case 'DOWNLOADING_SEGMENTS': {
      return isNumber(value.completed) && isNumber(value.total)
    }
    default: {
      return false
    }
  }
}

function isDownloadEvent(value: unknown): value is DownloadEvent {
  if (!isRecord(value) || !isRecord(value.episode) || !isRecord(value.state)) {
    return false
  }

  const { episode, state } = value
  const validEpisode =
    typeof episode.id === 'string' &&
    isNumber(episode.sn) &&
    typeof episode.cover === 'string' &&
    (episode.cover_id === null || typeof episode.cover_id === 'string') &&
    isNumber(episode.episode) &&
    typeof episode.create_at === 'string' &&
    typeof episode.update_at === 'string'

  if (!validEpisode) {
    return false
  }

  if (typeof state.Err === 'string') {
    return true
  }

  return isEpisodeDownloadEvent(state.Ok)
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

function payloadError(
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
  failureTimeoutMs = DEFAULT_FAILURE_TIMEOUT_MS,
  scheduler = defaultScheduler,
}: SubscribeDownloadEventStreamOptions) {
  let failureTimer: unknown
  let failureTimerArmed = false
  let failureReported = false

  function clearFailureTimer() {
    if (!failureTimerArmed) {
      return
    }

    scheduler.clearTimeout(failureTimer)
    failureTimer = undefined
    failureTimerArmed = false
  }

  function armFailureTimer() {
    if (failureTimerArmed) {
      return
    }

    failureTimerArmed = true
    failureTimer = scheduler.setTimeout(() => {
      failureTimer = undefined
      failureTimerArmed = false
      if (source.readyState !== EVENT_SOURCE_OPEN) {
        failureReported = true
        onStatusChange('failed')
      }
    }, failureTimeoutMs)
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

    if (failureReported) {
      return
    }

    onStatusChange('reconnecting')
    armFailureTimer()
  }

  function snapshotHandler(this: EventSource, ev: MessageEvent<string>) {
    try {
      onSnapshot(parseSnapshot(ev.data))
      onPayloadError(null)
    } catch (error) {
      onPayloadError(payloadError('snapshot', error))
    }
  }

  function updateHandler(this: EventSource, ev: MessageEvent<string>) {
    try {
      onUpdate(parseUpdate(ev.data))
    } catch (error) {
      onPayloadError(payloadError('update', error))
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
