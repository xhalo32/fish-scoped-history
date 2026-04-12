use fish_history_api::{HistoryProvider, fish_widestring::prelude::*};
use rusqlite::{Connection, params};
use std::{
    env::current_dir,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
    time::{Duration, UNIX_EPOCH},
};

use crate::utils::get_default_data_directory;

pub struct ScopedHistory {
    name: WString,
    connection: Mutex<Connection>,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub cmd: String,
    pub timestamp: i64,
    /// The scope of the item. Typically a directory path with tailing slash.
    pub scope: String,
}

impl From<Item> for fish_history_api::HistoryItem {
    fn from(item: Item) -> Self {
        fish_history_api::HistoryItem::new(
            WString::from(item.cmd),
            UNIX_EPOCH + Duration::from_secs(item.timestamp as u64),
        )
    }
}

impl ScopedHistory {
    pub fn new(name: &wstr, directory: Option<WString>) -> ScopedHistory {
        let dbname = format!("{}_history.db", name);
        let dir = directory
            .map(|dir| PathBuf::from(dir.to_string()))
            .unwrap_or_else(|| get_default_data_directory());
        let path = dir.join(Path::new(&dbname));

        // TODO migrate database
        let connection = Connection::open(path).unwrap();
        let _ = connection.execute(
            "CREATE TABLE history (
                id              INTEGER PRIMARY KEY,
                cmd             TEXT NOT NULL,
                timestamp       DATETIME NOT NULL,
                scope           TEXT NOT NULL
            )",
            (),
        );
        // .expect("failed to create history table");

        ScopedHistory {
            name: name.to_owned(),
            connection: Mutex::new(connection),
        }
    }

    fn conn(&self) -> MutexGuard<Connection> {
        self.connection.lock().unwrap()
    }

    fn item_at_index_cwd(&self, idx: usize) -> Option<fish_history_api::HistoryItem> {
        let cwd = get_cwd_trailing_slash();
        // Index 1 means last command in history.
        let idx = (idx as i64) - 1;
        let conn = self.conn();

        // We order by depth of the working directory, and secondarily by timestamp.
        let mut stmt = conn
            .prepare(
                "SELECT cmd, timestamp, scope FROM history
                WHERE ?1 LIKE scope || '%'
                ORDER BY LENGTH(scope) DESC, timestamp DESC
                LIMIT 1 OFFSET ?2",
            )
            .expect("failed to prepare SQL query");
        let mut iter = stmt
            .query_map(params![cwd, idx], |row| {
                Ok(Item {
                    cmd: row.get(0)?,
                    timestamp: row.get(1)?,
                    scope: row.get(2)?,
                })
            })
            .expect("failed to execute SQL query");

        Some(iter.next()?.unwrap().into())
    }

    fn add_with_cwd(&self, item: fish_history_api::HistoryItem) {
        let cwd = get_cwd_trailing_slash();
        let timestamp = item.get_timestamp();
        let item = Item {
            cmd: item.into_str().to_string(),
            timestamp: timestamp.duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
            scope: cwd,
        };

        let conn = self.conn();
        conn.execute(
            "INSERT INTO history (cmd, timestamp, scope) VALUES (?1, ?2, ?3)",
            (&item.cmd, &item.timestamp, &item.scope),
        )
        .expect("failed to insert");
    }

    fn remove_all(&self, s: &wstr) {
        let conn = self.conn();
        conn.execute("DELETE FROM history WHERE cmd = ?1", [s.to_string()])
            .expect("failed to delete command from history");
    }

    fn get_all_cwd(&self) -> Vec<fish_history_api::HistoryItem> {
        let cwd = get_cwd_trailing_slash();
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT cmd, timestamp, scope FROM history
                WHERE ?1 LIKE scope || '%'
                ORDER BY LENGTH(scope) DESC, timestamp DESC",
            )
            .unwrap();
        let iter = stmt
            .query_map([cwd], |row| {
                Ok(Item {
                    cmd: row.get(0)?,
                    timestamp: row.get(1)?,
                    scope: row.get(2)?,
                })
            })
            .unwrap();

        iter.map(|item| item.unwrap().into()).collect()
    }

    fn size_cwd(&self) -> u64 {
        let cwd = get_cwd_trailing_slash();
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM history WHERE ?1 LIKE scope || '%'")
            .unwrap();
        let mut iter = stmt
            .query_map([cwd], |row| -> Result<i64, _> { Ok(row.get(0)?) })
            .unwrap();

        iter.next().unwrap().unwrap() as u64
    }

    fn clear_all(&self) {
        let conn = self.conn();
        conn.execute("DELETE FROM history", []).unwrap();
    }
}

fn get_cwd_trailing_slash() -> String {
    let mut cwd = current_dir().unwrap().to_string_lossy().into_owned();
    if !cwd.ends_with('/') {
        cwd.push('/');
    }

    cwd
}

impl HistoryProvider for ScopedHistory {
    fn name(&self) -> &wstr {
        &self.name
    }

    fn item_at_index(&self, idx: usize) -> Option<fish_history_api::HistoryItem> {
        self.item_at_index_cwd(idx)
    }

    /// When adding a command, we assume that it was run from the current working directory.
    /// This might be undesirable when e.g. importing history from another shell.
    fn add(&self, item: fish_history_api::HistoryItem) {
        self.add_with_cwd(item);
    }

    /// Removes all occurrences of the command from the history.
    fn remove(&self, s: &wstr) {
        self.remove_all(s);
    }

    fn clear(&self) {
        self.clear_all();
    }

    fn size(&self) -> u64 {
        self.size_cwd()
    }

    fn save(&self) {
        // Saving is handled automatically
    }

    fn get_history(&self) -> Vec<fish_history_api::HistoryItem> {
        self.get_all_cwd()
    }

    /// Is the database empty. size == 0 doesn't mean the database is empty.
    fn is_empty(&self) -> bool {
        let conn = self.conn();

        let mut stmt = conn.prepare("SELECT COUNT(*) FROM history").unwrap();
        let mut iter = stmt
            .query_map([], |row| -> Result<i64, _> { Ok(row.get(0)?) })
            .unwrap();

        iter.next().unwrap().unwrap() == 0
    }
}
