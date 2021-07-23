BEGIN TRANSACTION;

PRAGMA foreign_keys(1);
PRAGMA user_version(100);
PRAGMA application_id(42);

CREATE TABLE IF NOT EXISTS `users` (
	`user_id`	    BLOB ( 16 ) NOT NULL UNIQUE
                    DEFAULT ( uuid_blob( uuid() ) ),

	`email`	        TEXT NOT NULL UNIQUE,

	`secret`	    BLOB ( 64 ) NOT NULL                                -- used to sign requests
                    DEFAULT (randomblob(64)),

    `created`  	    INTEGER ( 8 ) NOT NULL                              -- unix timestamp
                    DEFAULT (strftime('%s', CURRENT_TIMESTAMP)),

	PRIMARY KEY(`user_id`),
    CHECK( length(`user_id`) = 16 ),
    CHECK( length(`email`) > 0 ),
    CHECK( length(`secret`) > 0 )
    -- CHECK( length(`secret`) = 64 )

) WITHOUT ROWID;

CREATE INDEX IF NOT EXISTS `users_email` ON `users` (
	`email`
);

CREATE TABLE IF NOT EXISTS `workouts` (
	`workout_id`	BLOB ( 16 ) NOT NULL UNIQUE
                    DEFAULT ( uuid_blob( uuid() ) ),

	`user_id`	    BLOB ( 16 ) NOT NULL,

	`start`	        INTEGER ( 8 ) NOT NULL,                             -- unix timestamp

    `end`  	        INTEGER ( 8 ) NOT NULL,                             -- unix timestamp

	PRIMARY KEY(`workout_id`),

	FOREIGN KEY(`user_id`) REFERENCES `users`(`user_id`)
        ON DELETE CASCADE ON UPDATE CASCADE,

    CHECK( `end` >= `start` ),

    CHECK( length(`workout_id`) = 16),

    CHECK( length(`user_id`) = 16)

) WITHOUT ROWID;

CREATE INDEX IF NOT EXISTS `workouts_start` ON `workouts` (
	`start`	DESC
);

CREATE INDEX IF NOT EXISTS `user_workouts` ON `workouts` (
	`user_id`,
	`start`	DESC
);

CREATE VIEW workout_durations AS
    SELECT
        u.email AS email,
        uuid_str(u.user_id) AS user_id,
        uuid_str(w.workout_id) AS workout_id,
        date(w.start + (w.end - w.start) / 2, 'unixepoch') AS dt,
        cast(round((w.end - w.start) / 60.0) as integer) AS duration_minutes
    FROM `users` u
    INNER JOIN `workouts` w ON u.user_id = w.user_id
    -- ORDER BY w.start DESC
;

CREATE VIEW users_view AS
    SELECT
        uuid_str(u.user_id) AS user_id,
        u.email AS email,
        -- hex(u.secret) as secret,
        datetime(u.created, 'unixepoch') as created
    FROM users u;

COMMIT;
