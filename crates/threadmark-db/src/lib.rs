use rusqlite::{Connection, OptionalExtension};
use std::{path::Path, time::Duration};

pub const SCHEMA_VERSION: u32 = 3;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Database schema {0} is newer than this Threadmark supports ({SCHEMA_VERSION})")]
    NewerSchema(u32),
}

pub struct Database(Connection);

impl Database {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Self::migrate(Connection::open(path)?)
    }

    fn migrate(mut connection: Connection) -> Result<Self, Error> {
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        // Check under the write lock so simultaneous CLI/desktop first launches
        // cannot both attempt to create the initial schema.
        let transaction =
            connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let version: u32 =
            transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > SCHEMA_VERSION {
            return Err(Error::NewerSchema(version));
        }
        if version == 0 {
            transaction.execute_batch(include_str!("../migrations/0001_settings.sql"))?;
            transaction.pragma_update(None, "user_version", 1)?;
        }
        if version < 2 {
            transaction.execute_batch(include_str!("../migrations/0002_repositories.sql"))?;
            transaction.pragma_update(None, "user_version", 2)?;
        }
        if version < 3 {
            transaction.execute_batch(include_str!("../migrations/0003_index_revision.sql"))?;
            transaction.pragma_update(None, "user_version", 3)?;
        }
        transaction.commit()?;
        Ok(Self(connection))
    }

    pub fn setting(&self, key: &str) -> Result<Option<String>, Error> {
        Ok(self
            .0
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), Error> {
        self.0.execute("INSERT INTO settings(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value", [key, value])?;
        Ok(())
    }

    pub fn schema_version(&self) -> Result<u32, Error> {
        Ok(self
            .0
            .pragma_query_value(None, "user_version", |row| row.get(0))?)
    }

    pub fn snapshot(&self, root: &str) -> Result<Option<String>, Error> {
        Ok(self
            .0
            .query_row(
                "SELECT snapshot FROM repositories WHERE root=?1",
                [root],
                |row| row.get(0),
            )
            .optional()?)
    }

    pub fn index_revision(&self, root: &str) -> Result<Option<i64>, Error> {
        Ok(self
            .0
            .query_row(
                "SELECT revision FROM index_revisions WHERE root=?1",
                [root],
                |row| row.get(0),
            )
            .optional()?)
    }

    pub fn snapshot_with_revision(&self, root: &str) -> Result<Option<(String, i64)>, Error> {
        // One SQLite statement observes one committed revision. Reading these
        // separately can label an old graph with a newer writer's revision.
        Ok(self.0.query_row(
            "SELECT r.snapshot, COALESCE(v.revision, 0) FROM repositories r LEFT JOIN index_revisions v ON v.root=r.root WHERE r.root=?1",
            [root], |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional()?)
    }

    pub fn repositories(&self) -> Result<Vec<(String, i64)>, Error> {
        let mut statement = self
            .0
            .prepare("SELECT root, indexed_at FROM repositories ORDER BY indexed_at DESC")?;
        Ok(statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<_, _>>()?)
    }

    pub fn cached_files(
        &self,
        root: &str,
        adapter: &str,
    ) -> Result<Vec<(String, String, String)>, Error> {
        let mut statement = self.0.prepare("SELECT path,content_hash,parsed FROM parsed_files WHERE root=?1 AND adapter_version=?2")?;
        Ok(statement
            .query_map([root, adapter], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?
            .collect::<Result<_, _>>()?)
    }

    pub fn save_index(
        &self,
        root: &str,
        timestamp: i64,
        snapshot: &str,
        adapter: &str,
        files: &[(String, String, Option<String>)],
    ) -> Result<(), Error> {
        let transaction = self.0.unchecked_transaction()?;
        transaction.execute("INSERT INTO repositories(root,indexed_at,snapshot) VALUES (?1,?2,?3) ON CONFLICT(root) DO UPDATE SET indexed_at=excluded.indexed_at,snapshot=excluded.snapshot", rusqlite::params![root, timestamp, snapshot])?;
        transaction.execute("INSERT INTO index_revisions(root,revision) VALUES (?1,1) ON CONFLICT(root) DO UPDATE SET revision=index_revisions.revision+1", [root])?;
        let retained: std::collections::HashSet<_> =
            files.iter().map(|(path, _, _)| path.as_str()).collect();
        let existing = transaction
            .prepare("SELECT path FROM parsed_files WHERE root=?1")?
            .query_map([root], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        for path in existing {
            if !retained.contains(path.as_str()) {
                transaction.execute(
                    "DELETE FROM parsed_files WHERE root=?1 AND path=?2",
                    rusqlite::params![root, path],
                )?;
            }
        }
        {
            // Some(parsed) means core actually parsed this file, including repair
            // of malformed cache JSON with an otherwise unchanged hash/version.
            let mut insert = transaction.prepare("INSERT INTO parsed_files(root,path,content_hash,adapter_version,parsed) VALUES (?1,?2,?3,?4,?5) ON CONFLICT(root,path) DO UPDATE SET content_hash=excluded.content_hash,adapter_version=excluded.adapter_version,parsed=excluded.parsed")?;
            for (path, hash, parsed) in files {
                if let Some(parsed) = parsed {
                    insert.execute(rusqlite::params![root, path, hash, adapter, parsed])?;
                }
            }
        }
        transaction.execute("INSERT INTO settings(key,value) VALUES ('repository',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [root])?;
        transaction.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_and_revision_stay_paired_during_concurrent_publication() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("revisions.db");
        let reader = Database::open(&path).unwrap();
        reader.save_index("root", 1, "1", "adapter", &[]).unwrap();
        let running = Arc::new(AtomicBool::new(true));
        let writing = running.clone();
        let worker = std::thread::spawn(move || {
            let writer = Database::open(&path).unwrap();
            for revision in 2..=100 {
                writer
                    .save_index("root", revision, &revision.to_string(), "adapter", &[])
                    .unwrap();
            }
            writing.store(false, Ordering::Release);
        });
        while running.load(Ordering::Acquire) {
            let (snapshot, revision) = reader.snapshot_with_revision("root").unwrap().unwrap();
            assert_eq!(snapshot.parse::<i64>().unwrap(), revision);
        }
        worker.join().unwrap();
        assert_eq!(
            reader.snapshot_with_revision("root").unwrap().unwrap(),
            ("100".into(), 100)
        );
    }

    #[test]
    fn repaired_cache_replaces_same_hash_and_failed_publication_rolls_back() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("atomic.db");
        let db = Database::open(&path).unwrap();
        let files = |parsed: &str| vec![("work.ts".into(), "hash".into(), Some(parsed.into()))];
        db.save_index("root", 1, "previous", "adapter", &files("malformed"))
            .unwrap();
        db.save_index("root", 2, "repaired", "adapter", &files("valid"))
            .unwrap();
        assert_eq!(db.cached_files("root", "adapter").unwrap()[0].2, "valid");
        let revision = db.index_revision("root").unwrap();
        db.0.execute_batch("CREATE TRIGGER fail_cache BEFORE UPDATE ON parsed_files BEGIN SELECT RAISE(ABORT, 'injected write failure'); END;").unwrap();
        assert!(
            db.save_index("root", 3, "must not publish", "adapter", &files("new"))
                .is_err()
        );
        assert_eq!(db.snapshot("root").unwrap().as_deref(), Some("repaired"));
        assert_eq!(db.index_revision("root").unwrap(), revision);
        assert_eq!(db.cached_files("root", "adapter").unwrap()[0].2, "valid");
        assert_eq!(db.setting("repository").unwrap().as_deref(), Some("root"));
    }

    #[test]
    fn schema_two_upgrade_preserves_snapshots_caches_and_settings() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("upgrade.db");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(include_str!("../migrations/0001_settings.sql"))
            .unwrap();
        connection
            .execute_batch(include_str!("../migrations/0002_repositories.sql"))
            .unwrap();
        connection.execute_batch("INSERT INTO repositories VALUES ('root',1,'snapshot'); INSERT INTO parsed_files VALUES ('root','work.ts','hash','adapter','parsed'); INSERT INTO settings VALUES ('theme','dark'); PRAGMA user_version=2;").unwrap();
        drop(connection);
        let db = Database::open(&path).unwrap();
        assert_eq!(db.snapshot("root").unwrap().as_deref(), Some("snapshot"));
        assert_eq!(db.index_revision("root").unwrap(), Some(1));
        assert_eq!(db.cached_files("root", "adapter").unwrap().len(), 1);
        assert_eq!(db.setting("theme").unwrap().as_deref(), Some("dark"));
    }

    #[test]
    fn migration_and_settings_survive_reopening() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("threadmark.db");
        let db = Database::open(&path).unwrap();
        assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
        assert_eq!(db.setting("theme").unwrap(), None);
        db.set_setting("theme", "dark").unwrap();
        db.set_setting("theme", "light").unwrap();
        drop(db);
        assert_eq!(
            Database::open(&path)
                .unwrap()
                .setting("theme")
                .unwrap()
                .as_deref(),
            Some("light")
        );
    }

    #[test]
    fn refuses_future_schema_without_changing_it() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("future.db");
        let connection = Connection::open(&path).unwrap();
        connection.pragma_update(None, "user_version", 99).unwrap();
        assert!(matches!(
            Database::migrate(connection),
            Err(Error::NewerSchema(99))
        ));
        let connection = Connection::open(&path).unwrap();
        let version: u32 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 99);
    }

    #[test]
    fn failed_migration_rolls_back() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("conflict.db");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch("CREATE TABLE settings(key TEXT);")
            .unwrap();
        assert!(Database::migrate(connection).is_err());
        let connection = Connection::open(&path).unwrap();
        let version: u32 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 0);
        connection
            .execute("INSERT INTO settings(key) VALUES ('preserved')", [])
            .unwrap();
    }

    #[test]
    fn concurrent_first_launches_share_one_migration() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("concurrent.db");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    Database::open(&path).unwrap().schema_version().unwrap()
                })
            })
            .collect();
        for handle in handles {
            assert_eq!(handle.join().unwrap(), SCHEMA_VERSION);
        }
    }
}
