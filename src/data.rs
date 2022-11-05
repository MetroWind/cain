use chrono::prelude::*;
use rusqlite as sql;
use rusqlite::OptionalExtension;

use crate::error;
use crate::error::Error as Error;
use crate::data_types;
use crate::tree::Tree;

static ROOT_CATEGORY_ID: i64 = 0;
static ROOT_CATEGORY_NAME: &str = "(root)";

#[allow(dead_code)]
#[derive(Clone)]
pub enum SqliteFilename { InMemory, File(std::path::PathBuf) }

#[derive(Clone)]
pub struct DataManager
{
    filename: SqliteFilename,
    connection: Option<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>,
}

impl DataManager
{
    #[allow(dead_code)]
    pub fn new(f: SqliteFilename) -> Self
    {
        Self { filename: f, connection: None }
    }

    pub fn newWithFilename(f: &str) -> Self
    {
        Self {
            filename: SqliteFilename::File(std::path::PathBuf::from(f)),
            connection: None
        }
    }

    fn confirmConnection(&self) ->
        Result<r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>,
               Error>
    {
        if let Some(pool) = &self.connection
        {
            pool.get().map_err(|e| rterr!("Failed to get connection: {}", e))
        }
        else
        {
            Err(error!(DataError, "Sqlite database not connected"))
        }
    }

    /// Connect to the database. Create database file if not exist.
    pub fn connect(&mut self) -> Result<(), Error>
    {
        let manager = match &self.filename
        {
            SqliteFilename::File(path) =>
                r2d2_sqlite::SqliteConnectionManager::file(path),
            SqliteFilename::InMemory =>
                r2d2_sqlite::SqliteConnectionManager::memory(),
        };
        self.connection = Some(r2d2::Pool::new(manager).map_err(
            |_| rterr!("Failed to create connection pool"))?);
        Ok(())
    }

    fn tableExists(&self, table: &str) -> Result<bool, Error>
    {
        let conn = self.confirmConnection()?;
        let row = conn.query_row(
            "SELECT name FROM sqlite_master WHERE type='table' AND name=?;",
            sql::params![table],
            |row: &sql::Row|->sql::Result<String> { row.get(0) })
            .optional().map_err(
                |_| error!(DataError, "Failed to look up table {}", table))?;
        Ok(row.is_some())
    }

    pub fn init(&self) -> Result<(), Error>
    {
        let conn = self.confirmConnection()?;
        if !self.tableExists("categories")?
        {
            conn.execute(
                "CREATE TABLE categories (
                  id INTEGER PRIMARY KEY ASC,
                  name TEXT UNIQUE,
                  parent INTEGER
                  );", []).map_err(
                |e| error!(DataError, "Failed to create categories table: {}",
                           e))?;
            conn.execute("INSERT INTO categories (id, name, parent)
                          VALUES (?, ?, NULL);",
                         sql::params![ROOT_CATEGORY_ID, ROOT_CATEGORY_NAME])
                .map_err(|e| error!(DataError,
                                    "Failed to create root category: {}", e))?;
        }

        if !self.tableExists("entries")?
        {
            conn.execute(
                "CREATE TABLE entries (
                  id INTEGER PRIMARY KEY ASC,
                  uri TEXT UNIQUE,
                  categories TEXT,
                  time_add INTEGER
                  );", []).map_err(
                |e| error!(DataError, "Failed to create entries table: {}",
                           e))?;
        }

        if !self.tableExists("entry_data")?
        {
            conn.execute(
                "CREATE TABLE entry_data (
                  entry_id INTEGER,
                  key TEXT,
                  value TEXT,
                  FOREIGN KEY(entry_id) REFERENCES entries(id)
                  );", []).map_err(
                |e| error!(DataError, "Failed to create entry_data table: {}",
                           e))?;
        }

        Ok(())
    }

    pub fn addCategory(&self, name: &str, parent: i64) -> Result<(), Error>
    {
        let conn = self.confirmConnection()?;
        conn.execute("INSERT OR ABORT INTO categories (name, parent)
                      VALUES (?, ?);",
                     sql::params![name, parent]).map_err(
            |e| error!(DataError, "Failed to add category: {}", e))?;
        Ok(())
    }

    pub fn loadCategories(&self) -> Result<Tree<data_types::Category>, Error>
    {
        let conn = self.confirmConnection()?;
        let mut s = conn.prepare("Select id, name, parent from categories;")
            .map_err(|_| error!(DataError, "Failed to compile SQL statement to \
                                            load categories"))?;

        let mut rows = s.query([]).map_err(
            |_| error!(DataError, "Failed to execute SQL statement to load \
                                   categories"))?;

        let mut cat_tree = Tree::new(data_types::Category::new(
            ROOT_CATEGORY_ID, ROOT_CATEGORY_NAME));
        while let Some(row) = rows.next().map_err(
            |_| error!(DataError, "Failed to load categories"))?
        {
            let id: i64 = row.get(0).map_err(
                |_| error!(DataError, "Failed to get category ID"))?;
            if id == ROOT_CATEGORY_ID
            {
                continue;
            }
            let name: String = row.get(1).map_err(
                |_| error!(DataError, "Failed to get category name"))?;
            // Parent could be NULL in DB, but only for the root node.
            // So here it is safe to get a i64 (as opposed to
            // Option<i64>).
            let parent: i64 = row.get(2).map_err(
                |_| error!(DataError, "Failed to get category parent"))?;

            if let Some(cat) = cat_tree.findByID(id)
            {
                cat_tree.modifyNode(data_types::Category::new(id, &name))?;
                continue;
            }

            if cat_tree.findByID(parent).is_none()
            {
                cat_tree.addNode(data_types::Category::new(parent, ""), 0)?;
            }
            cat_tree.addNode(data_types::Category::new(id, &name), parent)?;
        }
        Ok(cat_tree)
    }

    pub fn addEntry(&self, uri: &str, categories: &[i64]) -> Result<i64, Error>
    {
    }

}

// ========== Tests =================================================>

#[cfg(test)]
mod tests
{
    use super::*;
    use serde_json::json;

    type AnyError = Box<dyn std::error::Error>;

    #[test]
    fn initDB() -> Result<(), AnyError>
    {
        let mut db = DataManager::new(SqliteFilename::InMemory);
        db.connect()?;
        db.init()?;
        assert!(db.tableExists("categories")?);

        assert_eq!(serde_json::to_value(&db.loadCategories()?)?,
                   json!({
                       "data": {
                           "id": ROOT_CATEGORY_ID,
                           "name": ROOT_CATEGORY_NAME
                       },
                       "children": []
                   }));
        Ok(())
    }

    #[test]
    fn addCategory() -> Result<(), AnyError>
    {
        let mut db = DataManager::new(SqliteFilename::InMemory);
        db.connect()?;
        db.init()?;
        db.addCategory("aaa", ROOT_CATEGORY_ID)?;
        assert_eq!(serde_json::to_value(&db.loadCategories()?)?,
                   json!({
                       "data": {
                           "id": ROOT_CATEGORY_ID,
                           "name": ROOT_CATEGORY_NAME
                       },
                       "children": [{
                           "data": {
                               "id": ROOT_CATEGORY_ID + 1,
                               "name": "aaa",
                           },
                           "children": []
                       }]
                   }));
        Ok(())
    }

}
