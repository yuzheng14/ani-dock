import type { DownloadEvent } from '@ani-dock/shared-type'
import { afterEach, beforeEach, expect, test, vi } from 'vitest'

import {
  DOWNLOAD_EVENT_STREAM_FAILURE_DELAY_MS,
  DownloadEventPayloadError,
  type DownloadEventStreamStatus,
  observeDownloadEventStream,
} from '../src/lib/download-event-stream'

class FakeEventSource {
  readonly CONNECTING = 0
  readonly OPEN = 1
  readonly CLOSED = 2
  readyState = this.CONNECTING
  closeCalls = 0
  private readonly listeners = new Map<string, Set<EventListener>>()

  addEventListener(type: string, listener: EventListener) {
    const listeners = this.listeners.get(type) ?? new Set<EventListener>()
    listeners.add(listener)
    this.listeners.set(type, listeners)
  }

  removeEventListener(type: string, listener: EventListener) {
    this.listeners.get(type)?.delete(listener)
  }

  close() {
    this.closeCalls += 1
    this.readyState = this.CLOSED
  }

  emit(type: string, data?: string) {
    const event = new MessageEvent(type, { data })
    for (const listener of this.listeners.get(type) ?? []) {
      listener(event)
    }
  }

  listenerCount() {
    return Array.from(this.listeners.values()).reduce(
      (count, listeners) => count + listeners.size,
      0
    )
  }
}

type ObserveOverrides = {
  onSnapshot?(events: DownloadEvent[]): void
  onUpdate?(event: DownloadEvent): void
  onStatusChange?(status: DownloadEventStreamStatus): void
  onPayloadError?(error: DownloadEventPayloadError): void
  onFailure?(): void
}

function downloadEvent(
  sn: number,
  state: DownloadEvent['state'] = { Ok: { type: 'PENDING' } }
): DownloadEvent {
  return {
    episode: {
      id: `episode-${sn}`,
      sn,
      cover: 'https://example.com/cover.png',
      cover_id: null,
      episode: 1,
      create_at: '2026-08-23T00:00:00Z',
      update_at: '2026-08-23T00:00:00Z',
    },
    state,
  }
}

function observe(source: FakeEventSource, overrides: ObserveOverrides = {}) {
  return observeDownloadEventStream({
    source,
    onSnapshot: () => {},
    onUpdate: () => {},
    onStatusChange: () => {},
    onPayloadError: () => {},
    onFailure: () => {},
    ...overrides,
  })
}

beforeEach(() => {
  vi.useFakeTimers()
})

afterEach(() => {
  vi.clearAllTimers()
  vi.useRealTimers()
})

test('tracks reconnection attempts and reports only prolonged failures', () => {
  const source = new FakeEventSource()
  const statuses: DownloadEventStreamStatus[] = []
  const onFailure = vi.fn()

  const stop = observe(source, {
    onStatusChange: (status) => statuses.push(status),
    onFailure,
  })

  source.readyState = source.OPEN
  source.emit('open')
  source.readyState = source.CONNECTING
  source.emit('error')
  source.emit('error')

  expect(statuses).toEqual(['connecting', 'open', 'reconnecting'])
  expect(vi.getTimerCount()).toBe(1)
  expect(onFailure).not.toHaveBeenCalled()

  source.readyState = source.OPEN
  source.emit('open')
  expect(vi.getTimerCount()).toBe(0)

  source.readyState = source.CONNECTING
  source.emit('error')
  vi.advanceTimersByTime(DOWNLOAD_EVENT_STREAM_FAILURE_DELAY_MS)

  expect(statuses).toEqual([
    'connecting',
    'open',
    'reconnecting',
    'open',
    'reconnecting',
    'failed',
  ])
  expect(onFailure).toHaveBeenCalledOnce()

  source.readyState = source.OPEN
  source.emit('open')
  expect(statuses.at(-1)).toBe('open')

  stop()
})

test('reports a terminal connection failure immediately', () => {
  const source = new FakeEventSource()
  const statuses: DownloadEventStreamStatus[] = []
  const onFailure = vi.fn()

  const stop = observe(source, {
    onStatusChange: (status) => statuses.push(status),
    onFailure,
  })

  source.readyState = source.OPEN
  source.emit('open')
  source.readyState = source.CLOSED
  source.emit('error')

  expect(statuses).toEqual(['connecting', 'open', 'failed'])
  expect(onFailure).toHaveBeenCalledOnce()
  expect(vi.getTimerCount()).toBe(0)

  stop()
})

test('accepts valid messages and isolates malformed payloads', () => {
  const source = new FakeEventSource()
  const events = new Map<number, DownloadEvent>()
  const errors: DownloadEventPayloadError[] = []

  const stop = observe(source, {
    onSnapshot: (snapshot) => {
      events.clear()
      for (const event of snapshot) {
        events.set(event.episode.sn, event)
      }
    },
    onUpdate: (event) => events.set(event.episode.sn, event),
    onPayloadError: (error) => errors.push(error),
  })

  source.emit('snapshot', JSON.stringify([downloadEvent(100)]))
  const snapshot = Array.from(events.entries())

  source.emit('snapshot', '{')
  source.emit('update', JSON.stringify({ episode: null }))

  expect(Array.from(events.entries())).toEqual(snapshot)
  expect(errors).toHaveLength(2)
  expect(
    errors.every((error) => error instanceof DownloadEventPayloadError)
  ).toBe(true)
  expect(errors.map((error) => error.messageType)).toEqual([
    'snapshot',
    'update',
  ])

  source.emit(
    'update',
    JSON.stringify(downloadEvent(100, { Ok: { type: 'COMPLETED' } }))
  )
  expect(events.get(100)?.state).toEqual({ Ok: { type: 'COMPLETED' } })

  stop()
})

test('removes every listener and pending timer during cleanup', () => {
  const source = new FakeEventSource()
  const statuses: DownloadEventStreamStatus[] = []

  const stop = observe(source, {
    onStatusChange: (status) => statuses.push(status),
  })

  source.emit('error')
  expect(source.listenerCount()).toBe(4)
  expect(vi.getTimerCount()).toBe(1)

  stop()
  expect(vi.getTimerCount()).toBe(0)
  vi.runOnlyPendingTimers()
  source.emit('open')

  expect(source.listenerCount()).toBe(0)
  expect(source.closeCalls).toBe(1)
  expect(statuses).toEqual(['connecting', 'reconnecting'])
})
