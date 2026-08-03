use chrono::{DateTime, Local};
use indexmap::IndexMap;
use sqlx::SqlitePool;
use uuid::{Uuid, fmt::Hyphenated};

use crate::{
    input::CreateAnime,
    model::{Anime, Episode},
};

pub type DbResult<T = ()> = Result<T, sqlx::Error>;

#[derive(Debug, Clone)]
pub struct AnimeRepository {
    pool: SqlitePool,
}

impl AnimeRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, input: CreateAnime) -> DbResult {
        // TODO save cover bytes
        let mut tx = self.pool.begin().await?;

        let anime_id = Uuid::now_v7().to_string();
        let create_at = Local::now();

        sqlx::query!(
            r#"
            INSERT INTO anime (id, sn, cover, name, create_at, update_at)
            values ($1, $2, $3, $4, $5, $6);
        "#,
            anime_id,
            input.sn,
            input.cover,
            input.name,
            create_at,
            create_at,
        )
        .execute(&mut *tx)
        .await?;

        for (name, episodes) in &input.series {
            let series_id = Uuid::now_v7().to_string();
            sqlx::query!(
                r#"
                INSERT INTO series (id, anime_id, name, create_at, update_at)
                values ($1, $2, $3, $4, $5)
            "#,
                series_id,
                anime_id,
                name,
                create_at,
                create_at,
            )
            .execute(&mut *tx)
            .await?;

            for episode in episodes {
                let episode_id = Uuid::now_v7().to_string();

                sqlx::query!(
                    r#"
                INSERT INTO episode (id, series_id, sn, cover, episode, create_at, update_at)
                values ($1, $2, $3, $4, $5, $6, $7);
                "#,
                    episode_id,
                    series_id,
                    episode.sn,
                    episode.cover,
                    episode.episode,
                    create_at,
                    create_at
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;

        Ok(())
    }

    pub async fn select_all(&self) -> DbResult<Vec<Anime>> {
        let rows = sqlx::query!(
            r#"
            SELECT
            anime.id as "id: Hyphenated",
            anime.sn as "sn: u32",
            anime.cover as cover,
            anime.name as name,
            anime.create_at as "create_at: DateTime<Local>",
            anime.update_at as "update_at: DateTime<Local>",
            series.name as series_name,
            episode.id as "episode_id: Hyphenated",
            episode.sn as "episode_sn: u32",
            episode.cover as episode_cover,
            episode.episode as "episode_episode: u32",
            episode.create_at as "episode_create_at: DateTime<Local>",
            episode.update_at as "episode_update_at: DateTime<Local>"
            FROM anime
                INNER JOIN series ON series.anime_id = anime.id
                INNER JOIN episode ON episode.series_id = series.id
                ORDER BY anime.id, series.name, episode.episode;
                "#
        )
        .fetch_all(&self.pool)
        .await?;

        let anime_row =
            rows.into_iter()
                .fold(IndexMap::new(), |mut map: IndexMap<_, Vec<_>>, row| {
                    map.entry(row.id).or_default().push(row);

                    map
                });

        let anime = anime_row
            .into_values()
            .map(|ar| {
                let first = ar.first().expect("异常情况，group 后应该有动画记录的");

                let id = first.id.into_uuid();
                let sn = first.sn;
                let cover = first.cover.clone();
                let name = first.name.clone();
                let create_at = first.create_at;
                let update_at = first.update_at;

                let series =
                    ar.into_iter()
                        .fold(IndexMap::new(), |mut map: IndexMap<_, Vec<_>>, row| {
                            map.entry(row.series_name).or_default().push(Episode {
                                id: row.episode_id.into_uuid(),
                                sn: row.episode_sn,
                                cover: row.episode_cover,
                                episode: row.episode_episode,

                                create_at: row.episode_create_at,
                                update_at: row.episode_update_at,
                            });

                            map
                        });

                Anime {
                    id,
                    sn,
                    cover,
                    name,

                    series,

                    create_at,
                    update_at,
                }
            })
            .collect::<Vec<Anime>>();

        Ok(anime)
    }
}

#[derive(Debug, Clone)]
pub struct EpisodeRepository {
    pool: SqlitePool,
}

impl EpisodeRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn select(&self, sn: u32) -> DbResult<Option<Episode>> {
        let row = sqlx::query!(
            r#"
            SELECT
                id AS "id: Hyphenated",
                sn AS "sn: u32",
                cover,
                episode AS "episode: u32",
                create_at AS "create_at: DateTime<Local>",
                update_at AS "update_at: DateTime<Local>"
            FROM episode
            WHERE sn = $1
            "#,
            sn
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Episode {
            id: r.id.into_uuid(),
            sn: r.sn,
            cover: r.cover,
            episode: r.episode,
            create_at: r.create_at,
            update_at: r.update_at,
        }))
    }
}

#[cfg(test)]
mod anime_repository_tests {
    use sqlx::SqlitePool;

    use super::*;
    use crate::input::CreateEpisode;

    #[sqlx::test]
    async fn select_all_returns_empty_vec_when_database_is_empty(pool: SqlitePool) -> DbResult {
        let repository = AnimeRepository::new(pool);

        let anime = repository.select_all().await?;

        assert!(anime.is_empty());

        Ok(())
    }

    #[sqlx::test]
    async fn insert_returns_unique_violation_when_anime_sn_already_exists(
        pool: SqlitePool,
    ) -> DbResult {
        let repository = AnimeRepository::new(pool);

        let mut first_series = IndexMap::new();
        first_series.insert(
            "第一季".to_owned(),
            vec![CreateEpisode {
                sn: 3499,
                cover: "https://example.com/3499.jpg".to_owned(),
                episode: 1,
            }],
        );
        repository
            .insert(CreateAnime {
                sn: 3499,
                cover: "https://example.com/anime-3499.jpg".to_owned(),
                series: first_series,
                name: "進擊的巨人".to_owned(),
            })
            .await?;

        let mut duplicate_series = IndexMap::new();
        duplicate_series.insert(
            "第二季".to_owned(),
            vec![CreateEpisode {
                sn: 50002,
                cover: "https://example.com/50002.jpg".to_owned(),
                episode: 1,
            }],
        );
        let error = repository
            .insert(CreateAnime {
                sn: 3499,
                cover: "https://example.com/duplicate-anime-3499.jpg".to_owned(),
                series: duplicate_series,
                name: "進擊的巨人".to_owned(),
            })
            .await
            .expect_err("插入重复的动画 SN 应该失败");

        assert!(matches!(
            error,
            sqlx::Error::Database(ref error) if error.is_unique_violation()
        ));

        let anime = repository.select_all().await?;
        assert_eq!(anime.len(), 1);
        assert_eq!(anime[0].cover, "https://example.com/anime-3499.jpg");

        Ok(())
    }

    #[sqlx::test]
    async fn insert_and_select_all_round_trip(pool: SqlitePool) -> DbResult {
        let repository = AnimeRepository::new(pool);

        let mut first_series = IndexMap::new();
        first_series.insert(
            "第二季".to_owned(),
            vec![CreateEpisode {
                sn: 50002,
                cover: "https://example.com/50002.jpg".to_owned(),
                episode: 1,
            }],
        );
        first_series.insert(
            "第一季".to_owned(),
            vec![
                CreateEpisode {
                    sn: 3499,
                    cover: "https://example.com/3499.jpg".to_owned(),
                    episode: 1,
                },
                CreateEpisode {
                    sn: 3500,
                    cover: "https://example.com/3500.jpg".to_owned(),
                    episode: 2,
                },
            ],
        );

        repository
            .insert(CreateAnime {
                sn: 3499,
                cover: "https://example.com/anime-3499.jpg".to_owned(),
                series: first_series,
                name: "進擊的巨人".to_owned(),
            })
            .await?;

        let mut second_series = IndexMap::new();
        second_series.insert(
            "本篇".to_owned(),
            vec![CreateEpisode {
                sn: 20273,
                cover: "https://example.com/20273.jpg".to_owned(),
                episode: 1,
            }],
        );

        repository
            .insert(CreateAnime {
                sn: 20273,
                cover: "https://example.com/anime-20273.jpg".to_owned(),
                series: second_series,
                name: "進擊的巨人".to_owned(),
            })
            .await?;

        let anime = repository.select_all().await?;

        assert_eq!(anime.len(), 2);

        let first = anime
            .iter()
            .find(|anime| anime.sn == 3499)
            .expect("应该返回第一部动画");
        assert_eq!(first.name, "進擊的巨人");
        assert_eq!(first.cover, "https://example.com/anime-3499.jpg");
        assert_eq!(first.series.len(), 2);

        let first_season = first.series.get("第一季").expect("应该返回第一季");
        assert_eq!(first_season.len(), 2);
        assert!(first_season.iter().any(|episode| {
            episode.sn == 3499
                && episode.cover == "https://example.com/3499.jpg"
                && episode.episode == 1
        }));
        assert!(first_season.iter().any(|episode| {
            episode.sn == 3500
                && episode.cover == "https://example.com/3500.jpg"
                && episode.episode == 2
        }));

        let second_season = first.series.get("第二季").expect("应该返回第二季");
        assert_eq!(second_season.len(), 1);
        assert_eq!(second_season[0].sn, 50002);
        assert_eq!(second_season[0].cover, "https://example.com/50002.jpg");
        assert_eq!(second_season[0].episode, 1);

        let second = anime
            .iter()
            .find(|anime| anime.sn == 20273)
            .expect("应该返回第二部动画");
        assert_eq!(second.name, "進擊的巨人");
        assert_eq!(second.cover, "https://example.com/anime-20273.jpg");
        assert_eq!(second.series.len(), 1);

        let episodes = second.series.get("本篇").expect("应该返回本篇");
        assert_eq!(episodes.len(), 1);
        assert_eq!(episodes[0].sn, 20273);
        assert_eq!(episodes[0].cover, "https://example.com/20273.jpg");
        assert_eq!(episodes[0].episode, 1);

        Ok(())
    }
}

#[cfg(test)]
mod episode_repository_tests {
    use sqlx::SqlitePool;

    use super::*;
    use crate::input::CreateEpisode;

    #[sqlx::test]
    async fn select_returns_none_when_database_is_empty(pool: SqlitePool) -> DbResult {
        let repository = EpisodeRepository::new(pool);

        let episode = repository.select(3499).await?;

        assert!(episode.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn insert_anime_and_select_round_trip(pool: SqlitePool) -> DbResult {
        let anime_repository = AnimeRepository::new(pool.clone());
        let episode_repository = EpisodeRepository::new(pool);

        let mut series = IndexMap::new();
        series.insert(
            "第一季".to_owned(),
            vec![CreateEpisode {
                sn: 3499,
                cover: "https://example.com/3499.jpg".to_owned(),
                episode: 1,
            }],
        );

        anime_repository
            .insert(CreateAnime {
                sn: 3499,
                cover: "https://example.com/anime-3499.jpg".to_owned(),
                series,
                name: "進擊的巨人".to_owned(),
            })
            .await?;

        let episode = episode_repository
            .select(3499)
            .await?
            .expect("应该返回对应 SN 的剧集");

        assert_eq!(episode.sn, 3499);
        assert_eq!(episode.cover, "https://example.com/3499.jpg");
        assert_eq!(episode.episode, 1);

        Ok(())
    }

    #[sqlx::test]
    async fn insert_returns_unique_violation_when_episode_sn_already_exists(
        pool: SqlitePool,
    ) -> DbResult {
        let anime_repository = AnimeRepository::new(pool.clone());
        let episode_repository = EpisodeRepository::new(pool);

        let mut first_series = IndexMap::new();
        first_series.insert(
            "第一季".to_owned(),
            vec![CreateEpisode {
                sn: 3499,
                cover: "https://example.com/3499.jpg".to_owned(),
                episode: 1,
            }],
        );
        anime_repository
            .insert(CreateAnime {
                sn: 3499,
                cover: "https://example.com/anime-3499.jpg".to_owned(),
                series: first_series,
                name: "進擊的巨人".to_owned(),
            })
            .await?;

        let mut duplicate_series = IndexMap::new();
        duplicate_series.insert(
            "本篇".to_owned(),
            vec![CreateEpisode {
                sn: 3499,
                cover: "https://example.com/duplicate-3499.jpg".to_owned(),
                episode: 1,
            }],
        );
        let error = anime_repository
            .insert(CreateAnime {
                sn: 20273,
                cover: "https://example.com/anime-20273.jpg".to_owned(),
                series: duplicate_series,
                name: "進擊的巨人".to_owned(),
            })
            .await
            .expect_err("插入重复的剧集 SN 应该失败");

        assert!(matches!(
            error,
            sqlx::Error::Database(ref error) if error.is_unique_violation()
        ));

        let episode = episode_repository
            .select(3499)
            .await?
            .expect("原有剧集不应该受到影响");
        assert_eq!(episode.cover, "https://example.com/3499.jpg");

        Ok(())
    }
}
