import { expect, test } from 'vitest'

import { filterAnimes } from '../src/lib/filter-animes'

const animes = [
  { name: '進擊的巨人 The Final Season', sn: 59221 },
  { name: '葬送的芙莉蓮', sn: 113665 },
  { name: 'SPY×FAMILY', sn: 104382 },
]

test('returns the original collection for a blank query', () => {
  expect(filterAnimes(animes, '  ')).toBe(animes)
})

test('matches anime names case-insensitively after trimming the query', () => {
  expect(filterAnimes(animes, '  final season  ')).toEqual([animes[0]])
})

test('normalizes compatible unicode characters when matching names', () => {
  expect(filterAnimes(animes, 'ＳＰＹ×ＦＡＭＩＬＹ')).toEqual([animes[2]])
})

test('matches complete or partial anime SN values', () => {
  expect(filterAnimes(animes, '113665')).toEqual([animes[1]])
  expect(filterAnimes(animes, '592')).toEqual([animes[0]])
})

test('returns an empty result without changing the source collection', () => {
  expect(filterAnimes(animes, '不存在')).toEqual([])
  expect(animes).toHaveLength(3)
})
