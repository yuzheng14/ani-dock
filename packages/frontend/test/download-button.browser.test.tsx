import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { Anime, Episode } from '@ani-dock/shared-type'
import { afterEach, expect, test, vi } from 'vitest'
import { page } from 'vitest/browser'
import { render } from 'vitest-browser-react'

import { DownloadButton } from '@/routes/library'

function episode(id: string, sn: number, episode: number): Episode {
  return {
    id,
    sn,
    episode,
    cover: '',
    cover_id: null,
    create_at: '2026-08-25T00:00:00Z',
    update_at: '2026-08-25T00:00:00Z',
  }
}

const anime: Anime = {
  id: 'anime-1',
  sn: 100,
  name: '测试动画',
  cover: '',
  cover_id: null,
  series: {
    本篇: [episode('episode-1', 101, 1), episode('episode-2', 102, 2)],
    特典: [episode('special-1', 201, 1), episode('special-2', 202, 2)],
  },
  create_at: '2026-08-25T00:00:00Z',
  update_at: '2026-08-25T00:00:00Z',
}

const longAnime: Anime = {
  ...anime,
  id: 'long-anime',
  name: '长篇测试动画',
  series: {
    本篇: Array.from({ length: 120 }, (_, index) =>
      episode(`long-episode-${index + 1}`, 1000 + index, index + 1)
    ),
  },
}

async function renderDownloadButton(fixture = anime) {
  const queryClient = new QueryClient({
    defaultOptions: {
      mutations: { retry: false },
    },
  })

  await render(
    <QueryClientProvider client={queryClient}>
      <DownloadButton anime={fixture} />
    </QueryClientProvider>
  )

  await page.getByRole('button', { name: `下载《${fixture.name}》` }).click()

  return page.getByRole('dialog')
}

afterEach(() => {
  vi.unstubAllGlobals()
})

test('keeps bulk actions, toggles, and the selection summary synchronized', async () => {
  const dialog = await renderDownloadButton()
  const confirmButton = dialog.getByRole('button', { name: '确认' })
  const mainEpisodes = dialog.getByRole('group', { name: '本篇剧集' })
  const specialEpisodes = dialog.getByRole('group', { name: '特典剧集' })

  await expect.element(dialog.getByText('已选择 2 / 4 集')).toBeVisible()
  await expect
    .element(mainEpisodes.getByRole('button', { name: '1' }))
    .toHaveAttribute('aria-pressed', 'true')
  await expect
    .element(specialEpisodes.getByRole('button', { name: '1' }))
    .toHaveAttribute('aria-pressed', 'false')

  await dialog.getByRole('button', { name: '全部清空' }).click()

  await expect
    .element(dialog.getByText('已选择 0 / 4 集，请至少选择 1 集'))
    .toBeVisible()
  await expect.element(confirmButton).toBeDisabled()
  await expect
    .element(mainEpisodes.getByRole('button', { name: '1' }))
    .toHaveAttribute('aria-pressed', 'false')

  await dialog.getByRole('button', { name: '全部选中' }).click()

  await expect.element(dialog.getByText('已选择 4 / 4 集')).toBeVisible()
  await expect
    .element(specialEpisodes.getByRole('button', { name: '1' }))
    .toHaveAttribute('aria-pressed', 'true')

  await dialog.getByRole('button', { name: '清空特典' }).click()
  await mainEpisodes.getByRole('button', { name: '2' }).click()
  await dialog.getByRole('button', { name: '全选特典' }).click()

  await expect.element(dialog.getByText('已选择 3 / 4 集')).toBeVisible()
  await expect.element(confirmButton).toBeEnabled()
  await expect
    .element(mainEpisodes.getByRole('button', { name: '2' }))
    .toHaveAttribute('aria-pressed', 'false')
  await expect
    .element(specialEpisodes.getByRole('button', { name: '2' }))
    .toHaveAttribute('aria-pressed', 'true')
})

test('submits only the selected episodes', async () => {
  const fetchMock = vi
    .fn()
    .mockResolvedValue(new Response(null, { status: 204 }))
  vi.stubGlobal('fetch', fetchMock)
  const dialog = await renderDownloadButton()

  await dialog.getByRole('button', { name: '全部清空' }).click()
  await dialog
    .getByRole('group', { name: '本篇剧集' })
    .getByRole('button', { name: '2' })
    .click()
  await dialog.getByRole('button', { name: '全选特典' }).click()
  await dialog.getByRole('button', { name: '确认' }).click()

  await expect.element(page.getByRole('dialog')).not.toBeInTheDocument()
  expect(fetchMock).toHaveBeenCalledOnce()
  expect(fetchMock).toHaveBeenCalledWith('/api/episodes/download', {
    method: 'PUT',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify([102, 201, 202]),
  })
})

test('keeps the footer visible while long episode lists scroll', async () => {
  await page.viewport(1280, 800)
  const dialog = await renderDownloadButton(longAnime)
  const episodeList = dialog
    .element()
    .querySelector<HTMLElement>('[data-slot="field-group"]')

  if (!episodeList) throw new Error('Episode list was not rendered')

  expect(episodeList.scrollHeight).toBeGreaterThan(episodeList.clientHeight)
  await expect
    .element(dialog.getByRole('button', { name: '确认' }))
    .toBeInViewport({ ratio: 1 })
})
