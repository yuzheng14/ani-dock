/// <reference types="node" />

import type { DownloadEvent } from '@ani-dock/shared-type'
import { deepStrictEqual, equal } from 'node:assert/strict'
import { afterEach, beforeEach, describe, it, mock } from 'node:test'

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

class MockEventSource extends EventTarget {
  static readonly CONNECTING = 0
  static readonly OPEN = 1
  static readonly CLOSED = 2

  readyState = MockEventSource.CONNECTING
  addedListeners = 0
  removedListeners = 0
  closeCount = 0

  override addEventListener(
    type: string,
    callback: EventListenerOrEventListenerObject | null,
    options?: boolean | AddEventListenerOptions
  ) {
    this.addedListeners += 1
    super.addEventListener(type, callback, options)
  }

  override removeEventListener(
    type: string,
    callback: EventListenerOrEventListenerObject | null,
    options?: boolean | EventListenerOptions
  ) {
    this.removedListeners += 1
    super.removeEventListener(type, callback, options)
  }

  close() {
    this.closeCount += 1
    this.readyState = MockEventSource.CLOSED
  }

  open() {
    this.readyState = MockEventSource.OPEN
    this.dispatchEvent(new Event('open'))
  }

  error({ closed = false }: { closed?: boolean } = {}) {
    this.readyState = closed
      ? MockEventSource.CLOSED
      : MockEventSource.CONNECTING
    this.dispatchEvent(new Event('error'))
  }

  message(event: 'snapshot' | 'update', data: string) {
    this.dispatchEvent(new MessageEvent(event, { data }))
  }
}

function createSubscription() {
  const source = new MockEventSource()
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

  return { source, state, unsubscribe }
}

beforeEach(() => mock.timers.enable({ apis: ['setTimeout'] }))
afterEach(() => mock.timers.reset())

describe('download event stream', () => {
  it('opens the stream and parses snapshot and update events', () => {
    const { source, state, unsubscribe } = createSubscription()

    source.open()
    equal(state.status, 'open')

    source.message('snapshot', JSON.stringify([firstDownloadEvent]))
    deepStrictEqual(state.snapshot, [firstDownloadEvent])

    source.message('update', JSON.stringify(updatedDownloadEvent))
    deepStrictEqual(state.updates, [updatedDownloadEvent])

    unsubscribe()
  })

  it('keeps the last snapshot during a transient reconnect', () => {
    const { source, state, unsubscribe } = createSubscription()

    source.open()
    source.message('snapshot', JSON.stringify([firstDownloadEvent]))
    source.error()

    equal(state.status, 'reconnecting')
    deepStrictEqual(state.snapshot, [firstDownloadEvent])

    mock.timers.tick(9_999)
    equal(state.status, 'reconnecting')

    source.open()
    mock.timers.tick(1)
    equal(state.status, 'open')
    deepStrictEqual(state.snapshot, [firstDownloadEvent])

    unsubscribe()
  })

  it('reports initial and prolonged connection failures', () => {
    const initial = createSubscription()
    mock.timers.tick(10_000)
    equal(initial.state.status, 'failed')
    initial.unsubscribe()

    const reconnecting = createSubscription()
    reconnecting.source.open()
    reconnecting.source.error()
    mock.timers.tick(10_000)
    equal(reconnecting.state.status, 'failed')

    reconnecting.source.error()
    mock.timers.tick(10_000)
    equal(reconnecting.state.status, 'failed')

    reconnecting.source.open()
    equal(reconnecting.state.status, 'open')
    reconnecting.unsubscribe()
  })

  it('reports terminal connection failures immediately', () => {
    const { source, state, unsubscribe } = createSubscription()

    source.open()
    source.error({ closed: true })
    equal(state.status, 'failed')

    unsubscribe()
  })

  it('ignores malformed payloads and clears the error on a valid snapshot', () => {
    const { source, state, unsubscribe } = createSubscription()

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

    unsubscribe()
  })

  it('removes listeners, closes the source, and clears the timer', () => {
    const { source, state, unsubscribe } = createSubscription()

    unsubscribe()
    mock.timers.tick(10_000)
    source.open()

    equal(source.addedListeners, 4)
    equal(source.removedListeners, 4)
    equal(source.closeCount, 1)
    equal(state.status, null)
  })
})
