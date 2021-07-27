BEGIN TRANSACTION;

DROP schema public CASCADE;
CREATE schema public;

CREATE EXTENSION IF NOT EXISTS pgcrypto;                -- enables gen_random_uuid() function

CREATE TABLE users (
    user_id     uuid NOT NULL UNIQUE
                DEFAULT gen_random_uuid()
                PRIMARY KEY,

    email       text NOT NULL UNIQUE
                CHECK (length(email) > 0),

    key         bytea NOT NULL                          -- ed25519 public key used to sign requests
                CHECK (length(key) = 32),

    created     timestamp with time zone NOT NULL
                DEFAULT now()
);

CREATE INDEX users_email ON users USING hash (
    email
);

CREATE TABLE workouts (
    workout_id  uuid NOT NULL UNIQUE
                DEFAULT gen_random_uuid()
                PRIMARY KEY,

    user_id     uuid NOT NULL,

    -- "start_time" and "end_time" because "start" and "end" caused reserved keyword conflicts

    start_time  timestamp with time zone NOT NULL,

    end_time    timestamp with time zone NOT NULL,

    CONSTRAINT workouts_user_fkey FOREIGN KEY (user_id)
        REFERENCES users (user_id)
        ON DELETE CASCADE,

    CONSTRAINT user_start_uniq UNIQUE(user_id, start_time) -- prevent duplicate start_time entries for given user, application code assumes this condition
);

CREATE INDEX workouts_start_time ON workouts USING btree (
    start_time  DESC
);

CREATE INDEX workouts_user_start_time ON workouts USING btree (
    user_id,
    start_time  DESC
);

-- useful for debugging/cli purposes
CREATE VIEW workout_durations AS
    SELECT
        u.email,
        u.user_id,
        w.workout_id,
        date(w.start_time) as dt,
        date_part('minutes', w.end_time - w.start_time) as duration_minutes
    FROM users u
    INNER JOIN workouts w ON u.user_id = w.user_id
;

-- facilitate future schema upgrades
CREATE TABLE migrations (
    id          serial PRIMARY KEY,

    version     text not null,

    descr       text,

    applied     timestamp with time zone NOT NULL
                DEFAULT now()
);

-- record this query as initial migration
insert into migrations(version, descr) values (
    '1.0.0',
    'initial schema: users, workouts, and migrations tables; workout_durations view'
);

-- some dummy data for testing
insert into users (user_id, email, key) values (
    '1fe9e4f0-8cd1-46be-963a-7f51470db6af',
    'jstrong@fitmod.me',
    '\xd2cd9de812f5eaed03474f0c03efce7fac19e64423e49760faf25472bc158752' -- base64-encoded priv key in .env (FITBOD_SECRET_ACCESS_KEY)
);

insert into workouts (user_id, start_time, end_time) values (
    '1fe9e4f0-8cd1-46be-963a-7f51470db6af',
    now() + interval '15 minutes',
    now() + interval '37 minutes'
);

COMMIT;
