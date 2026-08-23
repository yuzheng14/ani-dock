/// <reference types="node" />

import type { DownloadEvent } from '@ani-dock/shared-type'
import { deepStrictEqual, equal } from 'node:assert/strict'
import { describe, it } from 'node:test'

import {
  subscribeDownloadEventStream,
  type DownloadEventPayloadError,
  type DownloadEventStreamStatus,
} from './download-event-stream.ts'

const firstDownloadEvent: DownloadEvent = {
  episode: {
    id: '019c8c94-e2c8-7582-8786-0cbccc475a7f',
    sn: 3499,
    cover: 'https://example.com/3499.png',
    cover_id: null,
    episode: 1,
    create_at: '2026-08-23T00:00:00+00:00',
    update_at: '2026-08-23T00:00:00+00:00',
  },
  state: { Ok: { type: 'PENDING' } },
}

const updatedDownloadEvent: DownloadEvent = {
  ...firstDownloadEvent,
  state: {
    Ok: {
      type: 'DOWNLOADING_SEGMENTS',
      completed: 4,
      total: 10,
    },
  },
}

type Listener = EventListenerOrEventListenerObject

class MockEventSource {
  static readonly CONNECTING = 0
  static readonly OPEN = 1
  static readonly CLOSED = 2

  readonly listeners = new Map<string, Set<Listener>>()
  readyState = MockEventSource.CONNECTING
  closeCount = 0

  addEventListener(type: string, listener: Listener | null) {
    if (!listener) {
      return
    }

    const listeners = this.listeners.get(type) ?? new Set()
    listeners.add(listener)
    this.listeners.set(type, listeners)
  }

  removeEventListener(type: string, listener: Listener | null) {
    if (listener) {
      this.listeners.get(type)?.delete(listener)
    }
  }

  close() {
    this.closeCount += 1
    this.readyState = MockEventSource.CLOSED
  }

  open() {
    this.readyState = MockEventSource.OPEN
    this.emit('open', {})
  }

  error({ closed = false }: { closed?: boolean } = {}) {
    this.readyState = closed
      ? MockEventSource.CLOSED
      : MockEventSource.CONNECTING
    this.emit('error', {})
  }

  message(event: 'snapshot' | 'update', data: string) {
    this.emit(event, { data })
  }

  listenerCount() {
    return [...this.listeners.values()].reduce(
      (count, listeners) => count + listeners.size,
      0
    )
  }

  private emit(type: string, event: object) {
    for (const listener of this.listeners.get(type) ?? []) {
      if (typeof listener === 'function') {
        listener.call(this, event as Event)
      } else {
        listener.handleEvent(event as Event)
      }
    }
  }
}

class TestScheduler {
  private timer:
    | {
        id: number
        callback: () => void
        remaining: number
      }
    | undefined
  private nextId = 1

  setTimeout = (callback: () => void, delay: number) => {
    const id = this.nextId
    this.nextId += 1
    this.timer = { id, callback, remaining: delay }
    return id
  }

  clearTimeout = (timer: unknown) => {
    if (this.timer?.id === timer) {
      this.timer = undefined
    }
  }

  advance(milliseconds: number) {
    if (!this.timer) {
      return
    }

    this.timer.remaining -= milliseconds
    if (this.timer.remaining <= 0) {
      const { callback } = this.timer
      this.timer = undefined
      callback()
    }
  }

  get size() {
    return this.timer ? 1 : 0
  }
}

function createSubscription(failureTimeoutMs = 10_000) {
  const source = new MockEventSource()
  const scheduler = new TestScheduler()
  const state: {
    status: DownloadEventStreamStatus | null
    snapshot: DownloadEvent[] | null
    updates: DownloadEvent[]
    payloadError: DownloadEventPayloadError | null
  } = {
    status: null,
    snapshot: null,
    updates: [],
    payloadError: null,
  }
  const unsubscribe = subscribeDownloadEventStream({
    source: source as unknown as EventSource,
    scheduler,
    failureTimeoutMs,
    onStatusChange: (status) => {
      state.status = status
    },
    onSnapshot: (snapshot) => {
      state.snapshot = snapshot
    },
    onUpdate: (update) => {
      state.updates.push(update)
    },
    onPayloadError: (error) => {
      state.payloadError = error
    },
  })

  return { source, scheduler, state, unsubscribe }
}

describe('download event stream', () => {
  it('opens the stream and parses snapshot and update events', () => {
    const { source, state } = createSubscription()

    source.open()
    equal(state.status, 'open')

    source.message('snapshot', JSON.stringify([firstDownloadEvent]))
    deepStrictEqual(state.snapshot, [firstDownloadEvent])
    equal(state.payloadError, null)

    source.message('update', JSON.stringify(updatedDownloadEvent))
    deepStrictEqual(state.updates, [updatedDownloadEvent])
  })

  it('keeps the last snapshot during a transient reconnect', () => {
    const { source, scheduler, state } = createSubscription()

    source.open()
    source.message('snapshot', JSON.stringify([firstDownloadEvent]))
    source.error()

    equal(state.status, 'reconnecting')
    deepStrictEqual(state.snapshot, [firstDownloadEvent])

    scheduler.advance(9_999)
    equal(state.status, 'reconnecting')

    source.open()
    scheduler.advance(1)
    equal(state.status, 'open')
    deepStrictEqual(state.snapshot, [firstDownloadEvent])
  })

  it('reports initial and prolonged connection failures', () => {
    const initial = createSubscription()
    initial.scheduler.advance(10_000)
    equal(initial.state.status, 'failed')

    const reconnecting = createSubscription()
    reconnecting.source.open()
    reconnecting.source.error()
    reconnecting.scheduler.advance(10_000)
    equal(reconnecting.state.status, 'failed')

    reconnecting.source.error()
    equal(reconnecting.scheduler.size, 0)

    reconnecting.source.open()
    equal(reconnecting.state.status, 'open')
  })

  it('reports terminal connection failures immediately', () => {
    const { source, scheduler, state } = createSubscription()

    source.open()
    source.error({ closed: true })

    equal(state.status, 'failed')
    equal(scheduler.size, 0)
  })

  it('ignores malformed payloads and clears the error on a valid snapshot', () => {
    const { source, state } = createSubscription()

    source.message('snapshot', JSON.stringify([firstDownloadEvent]))
    source.message('update', '{')
    equal(state.payloadError?.event, 'update')
    deepStrictEqual(state.snapshot, [firstDownloadEvent])
    deepStrictEqual(state.updates, [])

    source.message('snapshot', '{}')
    equal(state.payloadError?.event, 'snapshot')
    deepStrictEqual(state.snapshot, [firstDownloadEvent])

    source.message('snapshot', '[]')
    equal(state.payloadError, null)
    deepStrictEqual(state.snapshot, [])
  })

  it('removes every listener, closes the source, and clears timers', () => {
    const { source, scheduler, unsubscribe } = createSubscription()

    equal(source.listenerCount(), 4)
    equal(scheduler.size, 1)

    unsubscribe()

    equal(source.listenerCount(), 0)
    equal(source.closeCount, 1)
    equal(scheduler.size, 0)
  })
})
