import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import {
  createMemoryHistory,
  createRouter,
  RouterProvider,
} from '@tanstack/react-router'
import type { DownloadEvent, Episode } from '@ani-dock/shared-type'
import { afterEach, beforeEach, expect, test, vi } from 'vitest'
import { page } from 'vitest/browser'
import { render } from 'vitest-browser-react'

import { ThemeProvider } from '@/components/theme-provider'
import { routeTree } from '@/routeTree.gen'

class FakeEventSource {
  static instances: FakeEventSource[] = []

  readonly CONNECTING = 0
  readonly OPEN = 1
  readonly CLOSED = 2
  readonly url: string
  readonly withCredentials = false
  readyState = this.CONNECTING
  private readonly listeners = new Map<
    string,
    Set<EventListenerOrEventListenerObject>
  >()

  constructor(url: string | URL) {
    this.url = url.toString()
    FakeEventSource.instances.push(this)
  }

  addEventListener(type: string, listener: EventListenerOrEventListenerObject) {
    const listeners =
      this.listeners.get(type) ?? new Set<EventListenerOrEventListenerObject>()
    listeners.add(listener)
    this.listeners.set(type, listeners)
  }

  removeEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject
  ) {
    this.listeners.get(type)?.delete(listener)
  }

  close() {
    this.readyState = this.CLOSED
  }

  emit(type: string, data?: string) {
    const event = new MessageEvent(type, { data })
    for (const listener of this.listeners.get(type) ?? []) {
      if (typeof listener === 'function') {
        listener(event)
      } else {
        listener.handleEvent(event)
      }
    }
  }
}

function episode(sn: number, episodeNumber: number): Episode {
  return {
    id: `episode-${sn}`,
    sn,
    cover: '',
    cover_id: null,
    episode: episodeNumber,
    create_at: '2026-08-31T00:00:00Z',
    update_at: '2026-08-31T00:00:00Z',
  }
}

function downloadEvent(
  sn: number,
  episodeNumber: number,
  state: DownloadEvent['state']
): DownloadEvent {
  return {
    episode: episode(sn, episodeNumber),
    state,
  }
}

async function renderDownloadingPage(downloadedEpisodes: Episode[]) {
  const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
    if (input.toString() !== '/api/episodes/downloaded') {
      throw new Error(`Unexpected request: ${input.toString()}`)
    }

    return new Response(JSON.stringify(downloadedEpisodes), {
      headers: { 'Content-Type': 'application/json' },
    })
  })
  vi.stubGlobal('fetch', fetchMock)
  vi.stubGlobal('EventSource', FakeEventSource)

  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  })
  const history = createMemoryHistory({ initialEntries: ['/downloading'] })
  const router = createRouter({ routeTree, history })

  await render(
    <QueryClientProvider client={queryClient}>
      <ThemeProvider disableTransitionOnChange={false}>
        <RouterProvider router={router} />
      </ThemeProvider>
    </QueryClientProvider>
  )

  await vi.waitFor(() => {
    expect(FakeEventSource.instances).toHaveLength(1)
  })

  return {
    source: FakeEventSource.instances[0],
    fetchMock,
  }
}

beforeEach(() => {
  FakeEventSource.instances = []
})

afterEach(() => {
  vi.unstubAllGlobals()
})

test('separates active, failed, and persisted completed downloads', async () => {
  const completed = episode(900, 9)
  const { source } = await renderDownloadingPage([completed])
  const pending = downloadEvent(101, 1, { Ok: { type: 'PENDING' } })
  const failed = downloadEvent(102, 2, { Err: 'network failure' })

  source.emit('snapshot', JSON.stringify([pending, failed]))

  await expect.element(page.getByText('第 1 集 - 101')).toBeVisible()
  await expect.element(page.getByText('第 2 集 - 102')).toBeVisible()
  await expect
    .element(page.getByText('下载失败，原因：network failure'))
    .toBeVisible()

  await page.getByRole('tab', { name: '已下载' }).click()

  await expect.element(page.getByText('第 9 集 - 900')).toBeVisible()
  await expect.element(page.getByText('第 1 集 - 101')).not.toBeInTheDocument()
  await expect.element(page.getByText('第 2 集 - 102')).not.toBeInTheDocument()
})

// TODO(#21): Enable after COMPLETED events refresh or update the completed-query cache.
test.skip('moves a completed SSE update from active downloads to completed downloads', async () => {
  const { source } = await renderDownloadingPage([])
  const pending = downloadEvent(101, 1, { Ok: { type: 'PENDING' } })

  source.emit('snapshot', JSON.stringify([pending]))
  await expect.element(page.getByText('第 1 集 - 101')).toBeVisible()

  source.emit(
    'update',
    JSON.stringify(
      downloadEvent(101, 1, {
        Ok: { type: 'COMPLETED' },
      })
    )
  )
  await expect.element(page.getByText('第 1 集 - 101')).not.toBeInTheDocument()

  await page.getByRole('tab', { name: '已下载' }).click()
  await expect.element(page.getByText('第 1 集 - 101')).toBeVisible()
})
