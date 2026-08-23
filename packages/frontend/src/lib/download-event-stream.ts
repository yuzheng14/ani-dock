import type { DownloadEvent } from '@ani-dock/shared-type'

export const DOWNLOAD_EVENT_STREAM_FAILURE_DELAY_MS = 10_000

export type DownloadEventStreamStatus =
  'connecting' | 'open' | 'reconnecting' | 'failed'

export type DownloadEventStreamMessageType = 'snapshot' | 'update'

export class DownloadEventPayloadError extends Error {
  readonly messageType: DownloadEventStreamMessageType

  constructor(messageType: DownloadEventStreamMessageType, cause: unknown) {
    super(`Invalid download ${messageType} event payload`, { cause })
    this.name = 'DownloadEventPayloadError'
    this.messageType = messageType
  }
}

interface DownloadEventSource {
  readonly CLOSED: number
  readonly readyState: number
  addEventListener(type: string, listener: EventListener): void
  removeEventListener(type: string, listener: EventListener): void
  close(): void
}

interface ObserveDownloadEventStreamOptions {
  source: DownloadEventSource
  onSnapshot(events: DownloadEvent[]): void
  onUpdate(event: DownloadEvent): void
  onStatusChange(status: DownloadEventStreamStatus): void
  onPayloadError(error: DownloadEventPayloadError): void
  onFailure(): void
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value)
}

function isDownloadEvent(value: unknown): value is DownloadEvent {
  if (!isRecord(value) || !isRecord(value.episode)) {
    return false
  }

  const { episode, state } = value
  if (
    typeof episode.id !== 'string' ||
    !isFiniteNumber(episode.sn) ||
    typeof episode.cover !== 'string' ||
    (episode.cover_id !== null && typeof episode.cover_id !== 'string') ||
    !isFiniteNumber(episode.episode) ||
    typeof episode.create_at !== 'string' ||
    typeof episode.update_at !== 'string' ||
    !isRecord(state)
  ) {
    return false
  }

  if ('Err' in state) {
    return typeof state.Err === 'string'
  }

  if (!('Ok' in state) || !isRecord(state.Ok)) {
    return false
  }

  switch (state.Ok.type) {
    case 'PENDING':
    case 'PREPARING':
    case 'WAITING_FOR_ADS':
    case 'RESOLVE_MEDIA_RESOURCE':
    case 'MERGING':
    case 'FINALIZING':
    case 'COMPLETED':
      return true
    case 'DOWNLOADING_SEGMENTS':
      return (
        isFiniteNumber(state.Ok.completed) && isFiniteNumber(state.Ok.total)
      )
    default:
      return false
  }
}

function parsePayload(messageType: 'snapshot', data: unknown): DownloadEvent[]
function parsePayload(messageType: 'update', data: unknown): DownloadEvent
function parsePayload(
  messageType: DownloadEventStreamMessageType,
  data: unknown
): DownloadEvent[] | DownloadEvent {
  try {
    if (typeof data !== 'string') {
      throw new TypeError('Expected the event data to be a string')
    }

    const value: unknown = JSON.parse(data)
    if (messageType === 'snapshot') {
      if (!Array.isArray(value) || !value.every(isDownloadEvent)) {
        throw new TypeError('Unexpected download event shape')
      }

      return value
    }

    if (!isDownloadEvent(value)) {
      throw new TypeError('Unexpected download event shape')
    }

    return value
  } catch (error) {
    throw new DownloadEventPayloadError(messageType, error)
  }
}

export function observeDownloadEventStream({
  source,
  onSnapshot,
  onUpdate,
  onStatusChange,
  onPayloadError,
  onFailure,
}: ObserveDownloadEventStreamOptions): () => void {
  let status: DownloadEventStreamStatus = 'connecting'
  let failureTimer: ReturnType<typeof window.setTimeout> | undefined
  let disposed = false

  const setStatus = (nextStatus: DownloadEventStreamStatus) => {
    if (status === nextStatus) {
      return
    }

    status = nextStatus
    onStatusChange(nextStatus)
  }

  const clearFailureTimer = () => {
    if (failureTimer === undefined) {
      return
    }

    window.clearTimeout(failureTimer)
    failureTimer = undefined
  }

  const setFailed = () => {
    if (disposed || status === 'failed') {
      return
    }

    setStatus('failed')
    onFailure()
  }

  const reportProlongedFailure = () => {
    failureTimer = undefined
    if (status === 'open') {
      return
    }

    setFailed()
  }

  const openHandler: EventListener = () => {
    clearFailureTimer()
    setStatus('open')
  }

  const errorHandler: EventListener = () => {
    if (source.readyState === source.CLOSED) {
      clearFailureTimer()
      setFailed()
      return
    }

    if (status === 'failed') {
      return
    }

    setStatus('reconnecting')
    failureTimer ??= window.setTimeout(
      reportProlongedFailure,
      DOWNLOAD_EVENT_STREAM_FAILURE_DELAY_MS
    )
  }

  const snapshotHandler: EventListener = (event) => {
    let snapshot: DownloadEvent[]
    try {
      snapshot = parsePayload('snapshot', (event as MessageEvent<unknown>).data)
    } catch (error) {
      if (error instanceof DownloadEventPayloadError) {
        onPayloadError(error)
        return
      }

      throw error
    }

    onSnapshot(snapshot)
  }

  const updateHandler: EventListener = (event) => {
    let update: DownloadEvent
    try {
      update = parsePayload('update', (event as MessageEvent<unknown>).data)
    } catch (error) {
      if (error instanceof DownloadEventPayloadError) {
        onPayloadError(error)
        return
      }

      throw error
    }

    onUpdate(update)
  }

  onStatusChange(status)
  source.addEventListener('open', openHandler)
  source.addEventListener('error', errorHandler)
  source.addEventListener('snapshot', snapshotHandler)
  source.addEventListener('update', updateHandler)

  return () => {
    disposed = true
    clearFailureTimer()
    source.removeEventListener('open', openHandler)
    source.removeEventListener('error', errorHandler)
    source.removeEventListener('snapshot', snapshotHandler)
    source.removeEventListener('update', updateHandler)
    source.close()
  }
}
