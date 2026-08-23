import assert from 'node:assert/strict'
import test from 'node:test'

import {
  DownloadEventPayloadError,
  observeDownloadEventStream,
} from '../src/lib/download-event-stream.ts'

class FakeEventSource {
  CONNECTING = 0
  OPEN = 1
  CLOSED = 2
  readyState = this.CONNECTING
  closeCalls = 0
  listeners = new Map()

  addEventListener(type, listener) {
    const listeners = this.listeners.get(type) ?? new Set()
    listeners.add(listener)
    this.listeners.set(type, listeners)
  }

  removeEventListener(type, listener) {
    this.listeners.get(type)?.delete(listener)
  }

  close() {
    this.closeCalls += 1
    this.readyState = this.CLOSED
  }

  emit(type, data) {
    for (const listener of this.listeners.get(type) ?? []) {
      listener.call(this, { data, type })
    }
  }

  listenerCount() {
    return Array.from(this.listeners.values()).reduce(
      (count, listeners) => count + listeners.size,
      0
    )
  }
}

class FakeScheduler {
  nextId = 0
  tasks = new Map()

  setTimeout(callback) {
    const id = ++this.nextId
    this.tasks.set(id, callback)
    return id
  }

  clearTimeout(id) {
    this.tasks.delete(id)
  }

  runAll() {
    const tasks = Array.from(this.tasks.values())
    this.tasks.clear()
    for (const task of tasks) {
      task()
    }
  }
}

function downloadEvent(sn, state = { Ok: { type: 'PENDING' } }) {
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

function observe(source, overrides = {}) {
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

test('tracks reconnection attempts and reports only prolonged failures', () => {
  const source = new FakeEventSource()
  const scheduler = new FakeScheduler()
  const statuses = []
  let failures = 0

  const stop = observe(source, {
    scheduler,
    onStatusChange: (status) => statuses.push(status),
    onFailure: () => {
      failures += 1
    },
  })

  source.readyState = source.OPEN
  source.emit('open')
  source.readyState = source.CONNECTING
  source.emit('error')
  source.emit('error')

  assert.deepEqual(statuses, ['connecting', 'open', 'reconnecting'])
  assert.equal(scheduler.tasks.size, 1)
  assert.equal(failures, 0)

  source.readyState = source.OPEN
  source.emit('open')
  assert.equal(scheduler.tasks.size, 0)

  source.readyState = source.CONNECTING
  source.emit('error')
  scheduler.runAll()
  source.emit('error')
  scheduler.runAll()

  assert.deepEqual(statuses, [
    'connecting',
    'open',
    'reconnecting',
    'open',
    'reconnecting',
    'failed',
  ])
  assert.equal(failures, 1)

  source.readyState = source.OPEN
  source.emit('open')
  assert.equal(statuses.at(-1), 'open')

  stop()
})

test('reports a terminal connection failure immediately', () => {
  const source = new FakeEventSource()
  const scheduler = new FakeScheduler()
  const statuses = []
  let failures = 0

  const stop = observe(source, {
    scheduler,
    onStatusChange: (status) => statuses.push(status),
    onFailure: () => {
      failures += 1
    },
  })

  source.readyState = source.OPEN
  source.emit('open')
  source.readyState = source.CLOSED
  source.emit('error')

  assert.deepEqual(statuses, ['connecting', 'open', 'failed'])
  assert.equal(failures, 1)
  assert.equal(scheduler.tasks.size, 0)

  stop()
})

test('accepts valid messages and isolates malformed payloads', () => {
  const source = new FakeEventSource()
  const events = new Map()
  const errors = []

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

  assert.deepEqual(Array.from(events.entries()), snapshot)
  assert.equal(errors.length, 2)
  assert.ok(errors.every((error) => error instanceof DownloadEventPayloadError))
  assert.deepEqual(
    errors.map((error) => error.messageType),
    ['snapshot', 'update']
  )

  source.emit(
    'update',
    JSON.stringify(downloadEvent(100, { Ok: { type: 'COMPLETED' } }))
  )
  assert.deepEqual(events.get(100).state, { Ok: { type: 'COMPLETED' } })

  stop()
})

test('removes every listener and pending timer during cleanup', () => {
  const source = new FakeEventSource()
  const scheduler = new FakeScheduler()
  const statuses = []

  const stop = observe(source, {
    scheduler,
    onStatusChange: (status) => statuses.push(status),
  })

  source.emit('error')
  assert.equal(source.listenerCount(), 4)
  assert.equal(scheduler.tasks.size, 1)

  stop()
  scheduler.runAll()
  source.emit('open')

  assert.equal(source.listenerCount(), 0)
  assert.equal(source.closeCalls, 1)
  assert.deepEqual(statuses, ['connecting', 'reconnecting'])
})
