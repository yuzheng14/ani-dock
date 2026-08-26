use std::sync::{Arc, RwLock};

use ani_dock_db::{
    model::Episode,
    repository::{DbResult, EpisodeRepository},
};
use rustc_hash::FxHashMap;

#[derive(Debug, Clone)]
pub struct EpisodeResolver {
    cache: Arc<RwLock<FxHashMap<u32, Episode>>>,
    repo: EpisodeRepository,
}

impl EpisodeResolver {
    pub fn new(repo: EpisodeRepository) -> Self {
        Self {
            cache: Arc::new(RwLock::new(FxHashMap::default())),
            repo,
        }
    }

    pub async fn resolve(&self, sn: u32) -> DbResult<Option<Episode>> {
        if let Some(episode) = self.cache.read().unwrap().get(&sn) {
            return Ok(Some(episode.to_owned()));
        }

        let episode = self.repo.select(sn).await?;

        if let Some(ep) = episode.clone() {
            self.update_cache(ep);
        }

        Ok(episode)
    }

    pub fn update_cache(&self, ep: Episode) {
        let mut cache = self.cache.write().unwrap();
        let should_update = cache
            .get(&ep.sn)
            .is_none_or(|cached| ep.update_at > cached.update_at);

        if should_update {
            cache.insert(ep.sn, ep);
        }
    }
}

#[cfg(test)]
mod tests {
    use ani_dock_db::{
        input::{CreateAnime, CreateEpisode},
        repository::AnimeRepository,
    };
    use indexmap::IndexMap;
    use sqlx::SqlitePool;

    use super::*;

    const SN: u32 = 12_345;

    async fn insert_episode(pool: &SqlitePool) -> Episode {
        AnimeRepository::new(pool.clone())
            .insert(CreateAnime {
                sn: SN,
                cover: "https://example.com/anime.jpg".to_owned(),
                name: "测试动画".to_owned(),
                series: IndexMap::from([(
                    "本篇".to_owned(),
                    vec![CreateEpisode {
                        sn: SN,
                        cover: "https://example.com/episode.jpg".to_owned(),
                        episode: 1,
                    }],
                )]),
            })
            .await
            .expect("episode fixture should be inserted");

        EpisodeRepository::new(pool.clone())
            .select(SN)
            .await
            .expect("episode fixture lookup should succeed")
            .expect("episode fixture should exist")
    }

    #[sqlx::test(migrations = "../ani-dock-db/migrations")]
    async fn resolve_caches_repository_result(pool: SqlitePool) {
        let expected = insert_episode(&pool).await;
        let resolver = EpisodeResolver::new(EpisodeRepository::new(pool));

        let resolved = resolver
            .resolve(SN)
            .await
            .expect("episode resolution should succeed");

        assert_eq!(resolved, Some(expected.clone()));
        assert_eq!(resolver.cache.read().unwrap().get(&SN), Some(&expected));
    }

    #[sqlx::test(migrations = "../ani-dock-db/migrations")]
    async fn cloned_resolvers_share_cache_updates(pool: SqlitePool) {
        let mut updated = insert_episode(&pool).await;
        updated.cover = "https://example.com/updated-episode.jpg".to_owned();

        let resolver = EpisodeResolver::new(EpisodeRepository::new(pool));
        let cloned = resolver.clone();
        cloned.update_cache(updated.clone());

        assert_eq!(
            resolver
                .resolve(SN)
                .await
                .expect("cached episode resolution should succeed"),
            Some(updated)
        );
    }

    #[sqlx::test(migrations = "../ani-dock-db/migrations")]
    async fn older_episode_does_not_replace_newer_cached_value(pool: SqlitePool) {
        let older = insert_episode(&pool).await;
        let mut newer = older.clone();
        newer.cover = "https://example.com/newer-episode.jpg".to_owned();
        newer.update_at = older.update_at + chrono::Duration::seconds(1);

        let resolver = EpisodeResolver::new(EpisodeRepository::new(pool));
        resolver.update_cache(newer.clone());
        resolver.update_cache(older);

        assert_eq!(
            resolver
                .resolve(SN)
                .await
                .expect("cached episode resolution should succeed"),
            Some(newer)
        );
    }
}
