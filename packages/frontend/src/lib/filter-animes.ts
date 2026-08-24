import type { Anime } from '@ani-dock/shared-type'

type SearchableAnime = Pick<Anime, 'name' | 'sn'>

function normalizeSearchValue(value: string) {
  return value.normalize('NFKC').toLocaleLowerCase()
}

export function filterAnimes<T extends SearchableAnime>(
  animes: readonly T[],
  rawQuery: string
): readonly T[] {
  const query = normalizeSearchValue(rawQuery.trim())

  if (!query) {
    return animes
  }

  return animes.filter(
    (anime) =>
      normalizeSearchValue(anime.name).includes(query) ||
      anime.sn.toString().includes(query)
  )
}
