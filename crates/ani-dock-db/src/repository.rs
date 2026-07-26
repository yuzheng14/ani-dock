use chrono::{DateTime, Local};
use indexmap::IndexMap;
use sqlx::SqlitePool;
use uuid::{Uuid, fmt::Hyphenated};

use crate::{
    input::CreateAnime,
    model::{Anime, Episode},
};

type DbResult<T = ()> = Result<T, sqlx::Error>;

pub struct AnimeRepository {
    pool: SqlitePool,
}

impl AnimeRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, input: CreateAnime) -> DbResult {
        let mut tx = self.pool.begin().await?;

        let anime_id = Uuid::now_v7().to_string();
        let create_at = Local::now();

        sqlx::query!(
            r#"
            INSERT INTO anime (id, sn, cover, title, create_at, update_at)
            values ($1, $2, $3, $4, $5, $6);
        "#,
            anime_id,
            input.sn,
            input.cover,
            input.title,
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
            title,
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
                let title = first.title.clone();
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
                    title,

                    series,

                    create_at,
                    update_at,
                }
            })
            .collect::<Vec<Anime>>();

        Ok(anime)
    }
}

#[cfg(test)]
mod tests {
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
                title: "测试动画一".to_owned(),
                series: first_series,
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
                title: "测试动画二".to_owned(),
                series: second_series,
            })
            .await?;

        let anime = repository.select_all().await?;

        assert_eq!(anime.len(), 2);

        let first = anime
            .iter()
            .find(|anime| anime.sn == 3499)
            .expect("应该返回第一部动画");
        assert_eq!(first.cover, "https://example.com/anime-3499.jpg");
        assert_eq!(first.title, "测试动画一");
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
        assert_eq!(second.cover, "https://example.com/anime-20273.jpg");
        assert_eq!(second.title, "测试动画二");
        assert_eq!(second.series.len(), 1);

        let episodes = second.series.get("本篇").expect("应该返回本篇");
        assert_eq!(episodes.len(), 1);
        assert_eq!(episodes[0].sn, 20273);
        assert_eq!(episodes[0].cover, "https://example.com/20273.jpg");
        assert_eq!(episodes[0].episode, 1);

        Ok(())
    }
}
