use std::convert::TryInto;
use sqlx::{Pool, Executor};
use sqlx::postgres::Postgres;
use chrono::prelude::*;
use uuid::Uuid;
use crate::auth::PublicKey;
use crate::Workout;

/// wrapper around postgres connection pool to encapsulate db-related functionality
#[derive(Clone)]
pub struct DataBase {
    pool: Pool<Postgres>,
}

impl DataBase {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = Pool::<Postgres>::connect(database_url).await?;
        Ok(Self { pool })
    }

    pub async fn fetch_user_keys(&self) -> Result<Vec<(Uuid, PublicKey)>, sqlx::Error> {
        let user_keys: Vec<(Uuid, Vec<u8>)> = sqlx::query_as("select user_id, key from users")
            .fetch_all(&self.pool)
            .await?;
        let user_keys = user_keys.into_iter()
            .map(|(user_id, public_key_vec)| {
                let public_key: PublicKey = public_key_vec.try_into().expect("failed to convert Vec<u8> to PublicKey");
                (user_id, public_key)
            }).collect();
        Ok(user_keys)
    }

    pub async fn fetch_recently_active_user_workouts(&self) -> Result<Vec<Workout>, sqlx::Error> {
        let workout_tuples: Vec<(Uuid, Uuid, DateTime<Utc>, DateTime<Utc>)> =
            sqlx::query_as(
                "select w.user_id, w.workout_id, w.start_time, w.end_time from workouts w \
                 where w.user_id in ( \
                     select distinct(user_id) from workouts \
                     where start_time >= now() - interval '7 days' \
                 )")
            .fetch_all(&self.pool)
            .await?;
        Ok(workout_tuples.into_iter().map(|(user_id, workout_id, start_time, end_time)| {
            Workout { user_id, workout_id, start_time, end_time }
        }).collect())
    }

    pub async fn fetch_user_workouts(&self, user_id: &Uuid) -> Result<Vec<Workout>, sqlx::Error> {
        let workout_rows: Vec<(Uuid, DateTime<Utc>, DateTime<Utc>)> = sqlx::query_as(
                "select workout_id, start_time, end_time \
                 from workouts \
                 where user_id = $1")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(workout_rows.into_iter().map(|(workout_id, start_time, end_time)| {
            Workout { user_id: *user_id, workout_id, start_time, end_time }
        }).collect())
    }

    pub async fn insert_workouts(&self, workouts: &[Workout]) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        for w in workouts {
            tx.execute(
                sqlx::query(
                    "insert into workouts (user_id, workout_id, start_time, end_time) values ($1, $2, $3, $4)"
                )
                    .bind(w.user_id)
                    .bind(w.workout_id)
                    .bind(w.start_time)
                    .bind(w.end_time)
            ).await?;
        }

        tx.commit().await?;

        Ok(())
    }

}

