import type { DownloadEvent } from '@ani-dock/shared-type'
import { useCallback, useEffect, useState } from 'react'

import {
  DEFAULT_FAILURE_TIMEOUT_MS,
  subscribeDownloadEventStream,
  type DownloadEventPayloadError,
  type DownloadEventStreamStatus,
} from './download-event-stream'

export type {
  DownloadEventPayloadError,
  DownloadEventStreamStatus,
} from './download-event-stream'

const DOWNLOAD_EVENT_STREAM_URL = '/api/episodes/download/events'

type EventSourceFactory = (url: string) => EventSource

type UseDownloadEventOptions = {
  eventSourceFactory?: EventSourceFactory
  failureTimeoutMs?: number
}

const defaultEventSourceFactory: EventSourceFactory = (url) =>
  new EventSource(url)

export function useDownloadEvent({
  eventSourceFactory = defaultEventSourceFactory,
  failureTimeoutMs = DEFAULT_FAILURE_TIMEOUT_MS,
}: UseDownloadEventOptions = {}) {
  const [downloadEventMap, setDownloadEventMap] = useState<
    Map<number, DownloadEvent>
  >(() => new Map())
  const [streamStatus, setStreamStatus] =
    useState<DownloadEventStreamStatus>('connecting')
  const [hasSnapshot, setHasSnapshot] = useState(false)
  const [streamPayloadError, setStreamPayloadError] =
    useState<DownloadEventPayloadError | null>(null)
  const [connectionAttempt, setConnectionAttempt] = useState(0)

  const reconnect = useCallback(() => {
    setStreamStatus('connecting')
    setConnectionAttempt((attempt) => attempt + 1)
  }, [])

  useEffect(() => {
    const source = eventSourceFactory(DOWNLOAD_EVENT_STREAM_URL)

    return subscribeDownloadEventStream({
      source,
      failureTimeoutMs,
      onStatusChange: setStreamStatus,
      onSnapshot: (downloadEvents) => {
        setDownloadEventMap(
          new Map(downloadEvents.map((de) => [de.episode.sn, de]))
        )
        setHasSnapshot(true)
      },
      onUpdate: (downloadEvent) => {
        setDownloadEventMap((current) =>
          new Map(current).set(downloadEvent.episode.sn, downloadEvent)
        )
      },
      onPayloadError: setStreamPayloadError,
    })
  }, [connectionAttempt, eventSourceFactory, failureTimeoutMs])

  return {
    downloadEvents: Array.from(downloadEventMap.values()),
    streamStatus,
    hasSnapshot,
    streamPayloadError,
    reconnect,
  }
}
