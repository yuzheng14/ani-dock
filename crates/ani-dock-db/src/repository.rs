use chrono::{DateTime, Local};
use indexmap::IndexMap;
use sqlx::SqlitePool;
use uuid::{Uuid, fmt::Hyphenated};

use crate::{
    CoreEpisode,
    input::{AnimeRow, AnimeRowWithSeries, CreateAnime, CreateCoverImage, EpisodeRow},
    model::{Anime, CoverImage, Episode},
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
        let mut tx = self.pool.begin().await?;

        let anime_id = Uuid::now_v7().to_string();
        let create_at = Local::now();

        let anime_id = sqlx::query_scalar!(
            r#"
            INSERT INTO
                anime(
                    id,
                    sn,
                    cover,
                    name,
                    create_at,
                    update_at
                )
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT(sn)
            DO UPDATE SET sn = anime.sn
            RETURNING id;
        "#,
            anime_id,
            input.sn,
            input.cover,
            input.name,
            create_at,
            create_at,
        )
        .fetch_one(&mut *tx)
        .await?;

        for (name, episodes) in &input.series {
            let series_id = Uuid::now_v7().to_string();
            let series_id = sqlx::query_scalar!(
                r#"
                INSERT INTO
                    series(
                        id,
                        anime_id,
                        name,
                        create_at,
                        update_at
                    )
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT(anime_id, name)
                DO UPDATE SET name = series.name
                RETURNING id;
            "#,
                series_id,
                anime_id,
                name,
                create_at,
                create_at,
            )
            .fetch_one(&mut *tx)
            .await?;

            for episode in episodes {
                let episode_id = Uuid::now_v7().to_string();

                sqlx::query!(
                    r#"
                    INSERT INTO
                        episode(
                            id,
                            series_id,
                            sn,
                            cover,
                            episode,
                            create_at,
                            update_at
                        )
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    ON CONFLICT(sn)
                    DO NOTHING;
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
            anime.cover_id as "cover_id?: Hyphenated",
            anime.name as name,
            anime.create_at as "create_at: DateTime<Local>",
            anime.update_at as "update_at: DateTime<Local>",
            series.name as series_name,
            episode.id as "episode_id: Hyphenated",
            episode.sn as "episode_sn: u32",
            episode.cover as episode_cover,
            episode.cover_id as "episode_cover_id?: Hyphenated",
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
                let cover_id = first.cover_id.map(|v| v.into_uuid());
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
                                cover_id: row.episode_cover_id.map(|v| v.into_uuid()),
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
                    cover_id,
                    name,

                    series,

                    create_at,
                    update_at,
                }
            })
            .collect::<Vec<Anime>>();

        Ok(anime)
    }

    pub async fn select_by_download_status(&self, downloaded: bool) -> DbResult<Vec<Anime>> {
        sqlx::query_as!(
            AnimeRowWithSeries,
            r#"
            WITH matched_episode AS (
                SELECT
                    s.anime_id,
                    s.id as series_id,
                    s.name as series_name,

                    e.id,
                    e.sn,
                    e.cover,
                    e.cover_id,
                    e.episode,

                    e.create_at,
                    e.update_at
                FROM download_queue as dq
                INNER JOIN episode as e ON dq.episode_id = e.id
                INNER JOIN series as s ON e.series_id = s.id
                WHERE dq.downloaded = $1
            ),

            matched_series AS (
                SELECT
                    anime_id,
                    series_id,
                    series_name,
                    (
                        json_group_array(
                            json_object(
                                'id', id,
                                'sn', sn,
                                'cover', cover,
                                'cover_id', cover_id,
                                'episode', episode,
                                'create_at', create_at,
                                'update_at', update_at
                            )
                            ORDER BY sn
                        )
                    ) as episodes
                FROM matched_episode
                GROUP BY
                    anime_id,
                    series_id,
                    series_name
            ),

            matched_anime AS (
                SELECT
                    anime_id,
                    json_group_object(
                        series_name,
                        json(episodes)
                        ORDER BY series_id
                    ) AS series
                FROM matched_series
                GROUP BY anime_id
            )

            SELECT
                a.id as "id: Hyphenated",
                a.sn as "sn: u32",
                a.name,
                a.cover,
                a.cover_id as "cover_id?: Hyphenated",
                a.create_at as "create_at: DateTime<Local>",
                a.update_at as "update_at: DateTime<Local>",
                series as 'series!: _'
            FROM matched_anime
            INNER JOIN anime as a ON a.id = anime_id
            "#,
            downloaded
        )
        .fetch_all(&self.pool)
        .await
        .map(|animes| animes.into_iter().map(Into::into).collect())
    }

    pub async fn select_row_by_id_or_sn(&self, id_or_sn: &str) -> DbResult<Option<Anime>> {
        let anime = sqlx::query_as!(
            AnimeRow,
            r#"SELECT
                id as "id: Hyphenated",
                sn as "sn: u32",
                cover,
                cover_id as "cover_id?: Hyphenated",
                name,

                create_at AS "create_at: DateTime<Local>",
                update_at AS "update_at: DateTime<Local>"
            FROM anime
            WHERE id = $1 or sn = $1
                "#,
            id_or_sn,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(anime.map(Into::into))
    }

    pub async fn update_cover_id(&self, id: Uuid, cover_id: Uuid) -> DbResult {
        let now = Local::now();
        sqlx::query!(
            r#"
            UPDATE anime
            SET
                cover_id = $1,
                update_at = $2
            WHERE id = $3"#,
            cover_id.to_string(),
            now,
            id.to_string()
        )
        .execute(&self.pool)
        .await?;
        Ok(())
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
                cover_id AS "cover_id?: Hyphenated",
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
            cover_id: r.cover_id.map(|v| v.into_uuid()),
            episode: r.episode,
            create_at: r.create_at,
            update_at: r.update_at,
        }))
    }

    pub async fn select_one_by_anime_id(&self, id: Uuid) -> DbResult<Option<Episode>> {
        let episode = sqlx::query_as!(
            EpisodeRow,
            r#"
            SELECT
                id as "id: Hyphenated",
                sn as "sn: u32",
                cover,
                cover_id as "cover_id?: Hyphenated",
                episode as "episode: u32",

                series_id as "series_id: Hyphenated",

                create_at as "create_at: DateTime<Local>",
                update_at as "update_at: DateTime<Local>"
            FROM episode AS e
            WHERE (
                SELECT anime_id FROM series AS s WHERE s.id = e.series_id
            ) = $1"#,
            id.to_string()
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(episode.map(Into::into))
    }

    pub async fn select_one_by_id_or_sn(&self, id_or_sn: &str) -> DbResult<Option<Episode>> {
        let episode = sqlx::query_as!(
            EpisodeRow,
            r#"
            SELECT
                id as "id: Hyphenated",
                sn as "sn: u32",
                cover,
                cover_id as "cover_id?: Hyphenated",
                episode as "episode: u32",
                series_id as "series_id: Hyphenated",
                create_at as "create_at: DateTime<Local>",
                update_at as "update_at: DateTime<Local>"
            FROM episode
            WHERE id = $1 or sn = $1"#,
            id_or_sn,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(episode.map(Into::into))
    }

    pub async fn update_cover_id(&self, id: Uuid, cover_id: Uuid) -> DbResult {
        let now = Local::now();
        sqlx::query!(
            r#"
            UPDATE episode
            SET
                cover_id = $1,
                update_at = $2
            WHERE id = $3"#,
            cover_id.to_string(),
            now,
            id.to_string()
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct DownloadQueueRepository {
    pool: SqlitePool,
}

impl DownloadQueueRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert_by_episode_or_ignore(&self, episode: CoreEpisode) -> DbResult<bool> {
        let id = Uuid::now_v7().to_string();
        let now = Local::now();

        let result = sqlx::query!(
            r#"
            INSERT INTO download_queue(id, episode_id, downloaded, create_at, update_at)
            SELECT $1, id, 0, $2, $3 FROM episode WHERE sn = $4
            ON CONFLICT(episode_id)
            DO NOTHING
            "#,
            id,
            now,
            now,
            episode.sn(),
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() == 1)
    }

    pub async fn mark_downloaded(&self, sn: u32) -> DbResult {
        let now = Local::now();

        sqlx::query!(
            r#"
            UPDATE download_queue
            SET downloaded = 1, update_at = $1
            WHERE episode_id = (SELECT id FROM episode WHERE sn = $2)
            "#,
            now,
            sn,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CoverImageRepository {
    pool: SqlitePool,
}

impl CoverImageRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn select_one(&self, id: &str) -> DbResult<CoverImage> {
        let cover_image = sqlx::query_as!(
            CoverImage,
            r#"
            SELECT
                id as "id: Hyphenated",
                url,
                bytes,
                mime_type,

                create_at as "create_at: DateTime<Local>",
                update_at as "update_at: DateTime<Local>"
            FROM cover_image
            WHERE id = $1"#,
            id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(cover_image)
    }

    pub async fn save(&self, data: CreateCoverImage) -> DbResult<CoverImage> {
        let id = Uuid::now_v7().hyphenated();
        let create_at = Local::now();
        let cover_image = sqlx::query_as!(
            CoverImage,
            r#"
            INSERT INTO cover_image (id, url, bytes, mime_type, create_at, update_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT(url) DO UPDATE SET url = excluded.url
            RETURNING
                id as "id: Hyphenated",
                url,
                bytes,
                mime_type,
                create_at as "create_at: DateTime<Local>",
                update_at as "update_at: DateTime<Local>"
                "#,
            id,
            data.url,
            data.bytes.as_ref(),
            data.mime_type,
            create_at.clone(),
            create_at
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(cover_image)
    }
}

#[cfg(test)]
mod test_helpers {
    use sqlx::SqlitePool;

    use super::*;
    use crate::input::CreateEpisode;

    pub(super) async fn insert_anime(
        repository: &AnimeRepository,
        sn: u32,
        series: IndexMap<String, Vec<CreateEpisode>>,
    ) -> DbResult {
        repository
            .insert(CreateAnime {
                sn,
                cover: format!("https://example.com/anime-{sn}.jpg"),
                series,
                name: "進擊的巨人".to_owned(),
            })
            .await
    }

    pub(super) async fn enqueue(pool: &SqlitePool, sn: u32, downloaded: bool) -> DbResult {
        let episode_id = sqlx::query_scalar!("SELECT id FROM episode WHERE sn = $1", sn)
            .fetch_one(pool)
            .await?;
        let now = Local::now();

        sqlx::query!(
            r#"
            INSERT INTO
                download_queue(
                    id,
                    downloaded,
                    episode_id,
                    create_at,
                    update_at
                )
            VALUES ($1, $2, $3, $4, $5)
            "#,
            Uuid::now_v7().to_string(),
            i32::from(downloaded),
            episode_id,
            now,
            now
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub(super) async fn insert_cover_image(pool: &SqlitePool, url: &str) -> DbResult<Uuid> {
        let id = Uuid::now_v7();
        let id_string = id.to_string();
        let bytes = b"cover image".as_slice();
        let now = Local::now();

        sqlx::query!(
            r#"
            INSERT INTO cover_image(id, url, bytes, mime_type, create_at, update_at)
            VALUES ($1, $2, $3, 'image/jpeg', $4, $5)
            "#,
            id_string,
            url,
            bytes,
            now,
            now,
        )
        .execute(pool)
        .await?;

        Ok(id)
    }

    pub(super) async fn set_anime_cover_id(pool: &SqlitePool, sn: u32, cover_id: Uuid) -> DbResult {
        let cover_id = cover_id.to_string();

        sqlx::query!("UPDATE anime SET cover_id = $1 WHERE sn = $2", cover_id, sn,)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub(super) async fn set_episode_cover_id(
        pool: &SqlitePool,
        sn: u32,
        cover_id: Uuid,
    ) -> DbResult {
        let cover_id = cover_id.to_string();

        sqlx::query!(
            "UPDATE episode SET cover_id = $1 WHERE sn = $2",
            cover_id,
            sn,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub(super) async fn queue_count(pool: &SqlitePool) -> DbResult<i64> {
        sqlx::query_scalar!("SELECT COUNT(*) FROM download_queue")
            .fetch_one(pool)
            .await
    }
}

#[cfg(test)]
mod anime_repository_tests {
    use sqlx::SqlitePool;

    use super::test_helpers::*;
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
    async fn insert_adds_new_series_when_anime_sn_already_exists(pool: SqlitePool) -> DbResult {
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
        repository
            .insert(CreateAnime {
                sn: 3499,
                cover: "https://example.com/duplicate-anime-3499.jpg".to_owned(),
                series: duplicate_series,
                name: "進擊的巨人".to_owned(),
            })
            .await?;

        let anime = repository.select_all().await?;
        assert_eq!(anime.len(), 1);
        assert_eq!(anime[0].cover, "https://example.com/anime-3499.jpg");
        assert_eq!(anime[0].series.len(), 2);

        let first_series = anime[0].series.get("第一季").expect("应该保留第一季");
        assert_eq!(first_series.len(), 1);
        assert_eq!(first_series[0].sn, 3499);

        let second_series = anime[0].series.get("第二季").expect("应该添加第二季");
        assert_eq!(second_series.len(), 1);
        assert_eq!(second_series[0].sn, 50002);

        Ok(())
    }

    #[sqlx::test]
    async fn insert_and_select_all_round_trip(pool: SqlitePool) -> DbResult {
        let repository = AnimeRepository::new(pool.clone());

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

        let anime_cover_id =
            insert_cover_image(&pool, "https://example.com/anime-3499.jpg").await?;
        let episode_cover_id = insert_cover_image(&pool, "https://example.com/3499.jpg").await?;
        set_anime_cover_id(&pool, 3499, anime_cover_id).await?;
        set_episode_cover_id(&pool, 3499, episode_cover_id).await?;

        let anime = repository.select_all().await?;

        assert_eq!(anime.len(), 2);

        let first = anime
            .iter()
            .find(|anime| anime.sn == 3499)
            .expect("应该返回第一部动画");
        assert_eq!(first.name, "進擊的巨人");
        assert_eq!(first.cover, "https://example.com/anime-3499.jpg");
        assert_eq!(first.cover_id, Some(anime_cover_id));
        assert_eq!(first.series.len(), 2);

        let first_season = first.series.get("第一季").expect("应该返回第一季");
        assert_eq!(first_season.len(), 2);
        assert!(first_season.iter().any(|episode| {
            episode.sn == 3499
                && episode.cover == "https://example.com/3499.jpg"
                && episode.cover_id == Some(episode_cover_id)
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

    /// 插入 3 部动画并设置下载队列：
    /// - 3499：本篇 3499（待下载）、3500（已下载）；第二季 50002（待下载）
    /// - 20273：本篇 20273（已下载）
    /// - 50123：没有队列记录
    async fn insert_status_fixture(pool: &SqlitePool) -> DbResult<AnimeRepository> {
        let repository = AnimeRepository::new(pool.clone());

        let mut series = IndexMap::new();
        series.insert(
            "本篇".to_owned(),
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
        series.insert(
            "第二季".to_owned(),
            vec![CreateEpisode {
                sn: 50002,
                cover: "https://example.com/50002.jpg".to_owned(),
                episode: 1,
            }],
        );
        insert_anime(&repository, 3499, series).await?;

        let mut downloaded_series = IndexMap::new();
        downloaded_series.insert(
            "本篇".to_owned(),
            vec![CreateEpisode {
                sn: 20273,
                cover: "https://example.com/20273.jpg".to_owned(),
                episode: 1,
            }],
        );
        insert_anime(&repository, 20273, downloaded_series).await?;

        let mut unqueued_series = IndexMap::new();
        unqueued_series.insert(
            "本篇".to_owned(),
            vec![CreateEpisode {
                sn: 50123,
                cover: "https://example.com/50123.jpg".to_owned(),
                episode: 1,
            }],
        );
        insert_anime(&repository, 50123, unqueued_series).await?;

        enqueue(pool, 3499, false).await?;
        enqueue(pool, 3500, true).await?;
        enqueue(pool, 50002, false).await?;
        enqueue(pool, 20273, true).await?;

        Ok(repository)
    }

    #[sqlx::test]
    async fn select_by_download_status_returns_pending_episodes_only(pool: SqlitePool) -> DbResult {
        let repository = insert_status_fixture(&pool).await?;

        let anime = repository.select_by_download_status(false).await?;

        assert_eq!(anime.len(), 1, "只有 3499 有待下载集数");
        assert_eq!(anime[0].sn, 3499);
        assert_eq!(anime[0].series.len(), 2);

        let main = anime[0].series.get("本篇").expect("应返回本篇");
        assert_eq!(main.len(), 1);
        assert_eq!(main[0].sn, 3499, "已下载的 3500 不应包含在内");

        let second = anime[0].series.get("第二季").expect("应返回第二季");
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].sn, 50002);

        Ok(())
    }

    #[sqlx::test]
    async fn select_by_download_status_returns_downloaded_episodes_only(
        pool: SqlitePool,
    ) -> DbResult {
        let repository = insert_status_fixture(&pool).await?;

        let anime_cover_id =
            insert_cover_image(&pool, "https://example.com/anime-3499.jpg").await?;
        let episode_cover_id = insert_cover_image(&pool, "https://example.com/3500.jpg").await?;
        set_anime_cover_id(&pool, 3499, anime_cover_id).await?;
        set_episode_cover_id(&pool, 3500, episode_cover_id).await?;

        let anime = repository.select_by_download_status(true).await?;

        assert_eq!(anime.len(), 2, "3499 和 20273 都有已下载集数");

        let first = anime
            .iter()
            .find(|anime| anime.sn == 3499)
            .expect("应返回 3499");
        assert_eq!(first.cover_id, Some(anime_cover_id));
        assert_eq!(first.series.len(), 1, "全部待下载的第二季不应返回空 series");
        assert!(first.series.get("第二季").is_none());

        let main = first.series.get("本篇").expect("应返回本篇");
        assert_eq!(main.len(), 1);
        assert_eq!(main[0].sn, 3500);
        assert_eq!(main[0].cover_id, Some(episode_cover_id));

        let second = anime
            .iter()
            .find(|anime| anime.sn == 20273)
            .expect("应返回 20273");
        let episodes = second.series.get("本篇").expect("应返回本篇");
        assert_eq!(episodes.len(), 1);
        assert_eq!(episodes[0].sn, 20273);

        Ok(())
    }

    #[sqlx::test]
    async fn select_by_download_status_returns_empty_when_queue_is_empty(
        pool: SqlitePool,
    ) -> DbResult {
        let repository = AnimeRepository::new(pool.clone());
        let mut series = IndexMap::new();
        series.insert(
            "本篇".to_owned(),
            vec![CreateEpisode {
                sn: 3499,
                cover: "https://example.com/3499.jpg".to_owned(),
                episode: 1,
            }],
        );
        insert_anime(&repository, 3499, series).await?;

        assert!(
            repository
                .select_by_download_status(false)
                .await?
                .is_empty()
        );
        assert!(repository.select_by_download_status(true).await?.is_empty());

        Ok(())
    }
}

#[cfg(test)]
mod download_queue_repository_tests {
    use sqlx::SqlitePool;

    use super::test_helpers::*;
    use super::*;
    use crate::input::CreateEpisode;

    #[sqlx::test]
    async fn insert_by_episode_or_ignore_queues_pending_episode(pool: SqlitePool) -> DbResult {
        let anime_repository = AnimeRepository::new(pool.clone());
        let mut series = IndexMap::new();
        series.insert(
            "本篇".to_owned(),
            vec![CreateEpisode {
                sn: 3499,
                cover: "https://example.com/3499.jpg".to_owned(),
                episode: 1,
            }],
        );
        insert_anime(&anime_repository, 3499, series).await?;

        let queue = DownloadQueueRepository { pool: pool.clone() };
        assert!(
            queue
                .insert_by_episode_or_ignore(CoreEpisode::new(
                    3499,
                    1,
                    "https://example.com/3499.jpg",
                ))
                .await?,
            "新集数入队应返回 true"
        );

        assert_eq!(queue_count(&pool).await?, 1);

        let anime = anime_repository.select_by_download_status(false).await?;
        assert_eq!(anime.len(), 1, "入队后应出现在待下载列表");
        assert_eq!(anime[0].sn, 3499);
        assert_eq!(anime[0].series.get("本篇").expect("应返回本篇")[0].sn, 3499);

        Ok(())
    }

    #[sqlx::test]
    async fn insert_by_episode_or_ignore_is_idempotent(pool: SqlitePool) -> DbResult {
        let anime_repository = AnimeRepository::new(pool.clone());
        let mut series = IndexMap::new();
        series.insert(
            "本篇".to_owned(),
            vec![CreateEpisode {
                sn: 3499,
                cover: "https://example.com/3499.jpg".to_owned(),
                episode: 1,
            }],
        );
        insert_anime(&anime_repository, 3499, series).await?;

        let queue = DownloadQueueRepository { pool: pool.clone() };
        assert!(
            queue
                .insert_by_episode_or_ignore(CoreEpisode::new(
                    3499,
                    1,
                    "https://example.com/3499.jpg",
                ))
                .await?,
            "第一次入队应返回 true"
        );
        assert!(
            !queue
                .insert_by_episode_or_ignore(CoreEpisode::new(
                    3499,
                    1,
                    "https://example.com/3499.jpg",
                ))
                .await?,
            "重复入队应返回 false"
        );

        assert_eq!(queue_count(&pool).await?, 1, "重复入队不应产生第二行");

        Ok(())
    }

    #[sqlx::test]
    async fn insert_by_episode_or_ignore_ignores_unknown_episode(pool: SqlitePool) -> DbResult {
        let queue = DownloadQueueRepository { pool: pool.clone() };

        assert!(
            !queue
                .insert_by_episode_or_ignore(CoreEpisode::new(
                    99999,
                    1,
                    "https://example.com/99999.jpg",
                ))
                .await?,
            "未知集数应返回 false"
        );

        assert_eq!(queue_count(&pool).await?, 0, "未知集数应被静默忽略");

        Ok(())
    }

    #[sqlx::test]
    async fn insert_by_episode_or_ignore_does_not_reset_downloaded_episode(
        pool: SqlitePool,
    ) -> DbResult {
        let anime_repository = AnimeRepository::new(pool.clone());
        let mut series = IndexMap::new();
        series.insert(
            "本篇".to_owned(),
            vec![CreateEpisode {
                sn: 3499,
                cover: "https://example.com/3499.jpg".to_owned(),
                episode: 1,
            }],
        );
        insert_anime(&anime_repository, 3499, series).await?;

        enqueue(&pool, 3499, true).await?;

        let queue = DownloadQueueRepository { pool: pool.clone() };
        assert!(
            !queue
                .insert_by_episode_or_ignore(CoreEpisode::new(
                    3499,
                    1,
                    "https://example.com/3499.jpg",
                ))
                .await?,
            "已存在集数应返回 false"
        );

        assert_eq!(queue_count(&pool).await?, 1);
        let downloaded: i64 = sqlx::query_scalar!("SELECT downloaded FROM download_queue")
            .fetch_one(&pool)
            .await?;
        assert_eq!(downloaded, 1, "已下载的集数不应被重置为待下载");

        Ok(())
    }

    #[sqlx::test]
    async fn mark_downloaded_marks_queued_episode_as_downloaded(pool: SqlitePool) -> DbResult {
        let anime_repository = AnimeRepository::new(pool.clone());
        let mut series = IndexMap::new();
        series.insert(
            "本篇".to_owned(),
            vec![CreateEpisode {
                sn: 3499,
                cover: "https://example.com/3499.jpg".to_owned(),
                episode: 1,
            }],
        );
        insert_anime(&anime_repository, 3499, series).await?;

        let queue = DownloadQueueRepository { pool: pool.clone() };
        assert!(
            queue
                .insert_by_episode_or_ignore(CoreEpisode::new(
                    3499,
                    1,
                    "https://example.com/3499.jpg",
                ))
                .await?
        );

        queue.mark_downloaded(3499).await?;

        let downloaded = anime_repository.select_by_download_status(true).await?;
        assert_eq!(downloaded.len(), 1, "标记后应出现在已下载列表");
        assert_eq!(downloaded[0].sn, 3499);
        assert!(
            anime_repository
                .select_by_download_status(false)
                .await?
                .is_empty(),
            "标记后不应出现在待下载列表"
        );

        // 重复标记保持幂等
        queue.mark_downloaded(3499).await?;
        assert_eq!(queue_count(&pool).await?, 1);
        assert_eq!(
            anime_repository
                .select_by_download_status(true)
                .await?
                .len(),
            1
        );

        Ok(())
    }

    #[sqlx::test]
    async fn mark_downloaded_does_nothing_for_unknown_episode(pool: SqlitePool) -> DbResult {
        let queue = DownloadQueueRepository { pool: pool.clone() };

        queue.mark_downloaded(99999).await?;

        assert_eq!(queue_count(&pool).await?, 0);

        Ok(())
    }
}

#[cfg(test)]
mod episode_repository_tests {
    use sqlx::SqlitePool;

    use super::test_helpers::{insert_cover_image, set_episode_cover_id};
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
        let episode_repository = EpisodeRepository::new(pool.clone());

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

        let cover_id = insert_cover_image(&pool, "https://example.com/3499.jpg").await?;
        set_episode_cover_id(&pool, 3499, cover_id).await?;

        let episode = episode_repository
            .select(3499)
            .await?
            .expect("应该返回对应 SN 的剧集");

        assert_eq!(episode.sn, 3499);
        assert_eq!(episode.cover, "https://example.com/3499.jpg");
        assert_eq!(episode.cover_id, Some(cover_id));
        assert_eq!(episode.episode, 1);

        Ok(())
    }

    #[sqlx::test]
    async fn insert_is_idempotent_and_adds_new_episode_to_existing_series(
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
            "第一季".to_owned(),
            vec![
                CreateEpisode {
                    sn: 3499,
                    cover: "https://example.com/duplicate-3499.jpg".to_owned(),
                    episode: 1,
                },
                CreateEpisode {
                    sn: 3500,
                    cover: "https://example.com/3500.jpg".to_owned(),
                    episode: 2,
                },
            ],
        );
        anime_repository
            .insert(CreateAnime {
                sn: 3499,
                cover: "https://example.com/duplicate-anime-3499.jpg".to_owned(),
                series: duplicate_series,
                name: "進擊的巨人".to_owned(),
            })
            .await?;

        let episode = episode_repository
            .select(3499)
            .await?
            .expect("原有剧集不应该受到影响");
        assert_eq!(episode.cover, "https://example.com/3499.jpg");

        let episode = episode_repository
            .select(3500)
            .await?
            .expect("应该向已有系列添加新的剧集");
        assert_eq!(episode.cover, "https://example.com/3500.jpg");
        assert_eq!(episode.episode, 2);

        let anime = anime_repository.select_all().await?;
        assert_eq!(anime.len(), 1);
        assert_eq!(anime[0].series.len(), 1);
        assert_eq!(anime[0].series["第一季"].len(), 2);

        Ok(())
    }
}

#[cfg(test)]
mod cover_image_repository_tests {
    use bytes::Bytes;
    use sqlx::SqlitePool;

    use super::*;

    #[sqlx::test]
    async fn save_returns_existing_image_when_url_already_exists(pool: SqlitePool) -> DbResult {
        let repository = CoverImageRepository::new(pool.clone());
        let url = "https://example.com/cover.jpg";

        let first = repository
            .save(CreateCoverImage {
                url: url.to_owned(),
                bytes: Bytes::from_static(b"original image"),
                mime_type: "image/jpeg".to_owned(),
            })
            .await?;

        let duplicate = repository
            .save(CreateCoverImage {
                url: url.to_owned(),
                bytes: Bytes::from_static(b"duplicate image"),
                mime_type: "image/png".to_owned(),
            })
            .await?;

        assert_eq!(duplicate.id, first.id);
        assert_eq!(duplicate.url, first.url);
        assert_eq!(duplicate.bytes, first.bytes);
        assert_eq!(duplicate.mime_type, first.mime_type);

        let row_count =
            sqlx::query_scalar!("SELECT COUNT(*) FROM cover_image WHERE url = $1", url,)
                .fetch_one(&pool)
                .await?;
        assert_eq!(row_count, 1);

        Ok(())
    }
}
