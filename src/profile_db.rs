use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A company profile stored in the local database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyProfile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub cpv_codes: Vec<String>,
    pub categories: Vec<String>,
    pub location: Option<String>,
    pub created_at: String,
    pub has_embedding: bool,
}

/// Lightweight summary of a company profile (without description).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyProfileSummary {
    pub id: String,
    pub name: String,
    pub cpv_codes: Vec<String>,
    pub categories: Vec<String>,
    pub location: Option<String>,
    pub created_at: String,
    pub has_embedding: bool,
}

/// Lightweight local profile database using plain rusqlite (no sqlite-vec).
///
/// Embeddings are stored as little-endian f32 BLOBs directly in the
/// `company_profiles` table, avoiding the need for the sqlite-vec extension.
pub struct ProfileDb {
    conn: Connection,
}

impl ProfileDb {
    /// Open (or create) the profile database at `path`.
    ///
    /// Handles migration from the old `VecDb`-created schema:
    /// - If the table has `has_embedding INTEGER` but no `embedding BLOB`, adds
    ///   the column. Old embeddings in `vec_profiles` are lost (can't read
    ///   without sqlite-vec), but profile data is preserved.
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path).context("opening profile database")?;

        // Check if company_profiles table exists
        let table_exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='company_profiles')",
            [],
            |row| row.get(0),
        )?;

        if !table_exists {
            // Fresh DB — create with new schema
            conn.execute_batch(
                "CREATE TABLE company_profiles (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    description TEXT NOT NULL,
                    cpv_codes TEXT,
                    categories TEXT,
                    location TEXT,
                    created_at TEXT NOT NULL,
                    embedding BLOB
                );",
            )
            .context("creating company_profiles table")?;
        } else {
            // Existing table — check if embedding column exists
            let has_embedding_col: bool = conn
                .prepare("PRAGMA table_info(company_profiles)")?
                .query_map([], |row| row.get::<_, String>(1))?
                .filter_map(|r| r.ok())
                .any(|name| name == "embedding");

            if !has_embedding_col {
                // Old schema — migrate: add embedding BLOB column.
                // Old embeddings in vec_profiles are lost (needs sqlite-vec to read).
                conn.execute_batch(
                    "ALTER TABLE company_profiles ADD COLUMN embedding BLOB;",
                )
                .context("migrating: adding embedding column")?;
            }
        }

        Ok(Self { conn })
    }

    pub fn create_company_profile(
        &self,
        name: &str,
        description: &str,
        cpv_codes: &[String],
        categories: &[String],
        location: Option<&str>,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        let cpv_json = serde_json::to_string(cpv_codes)?;
        let cat_json = serde_json::to_string(categories)?;
        self.conn
            .execute(
                "INSERT INTO company_profiles (id, name, description, cpv_codes, categories, location, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![id, name, description, cpv_json, cat_json, location, created_at],
            )
            .context("inserting company profile")?;
        Ok(id)
    }

    pub fn update_company_profile(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        cpv_codes: Option<&[String]>,
        categories: Option<&[String]>,
        location: Option<Option<&str>>,
    ) -> Result<bool> {
        let mut set_clauses: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(name) = name {
            set_clauses.push("name = ?".to_string());
            params.push(Box::new(name.to_string()));
        }
        if let Some(description) = description {
            set_clauses.push("description = ?".to_string());
            params.push(Box::new(description.to_string()));
        }
        if let Some(cpv_codes) = cpv_codes {
            set_clauses.push("cpv_codes = ?".to_string());
            params.push(Box::new(serde_json::to_string(cpv_codes)?));
        }
        if let Some(categories) = categories {
            set_clauses.push("categories = ?".to_string());
            params.push(Box::new(serde_json::to_string(categories)?));
        }
        if let Some(loc) = location {
            match loc {
                Some(v) => {
                    set_clauses.push("location = ?".to_string());
                    params.push(Box::new(v.to_string()));
                }
                None => {
                    set_clauses.push("location = NULL".to_string());
                }
            }
        }

        if set_clauses.is_empty() {
            // Nothing to update — check existence
            let exists: bool = self.conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM company_profiles WHERE id = ?1)",
                rusqlite::params![id],
                |row| row.get(0),
            )?;
            return Ok(exists);
        }

        // If description changed, clear embedding
        if description.is_some() {
            set_clauses.push("embedding = NULL".to_string());
        }

        let set_sql = set_clauses.join(", ");
        let sql = format!("UPDATE company_profiles SET {set_sql} WHERE id = ?");
        params.push(Box::new(id.to_string()));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        let updated = self
            .conn
            .execute(&sql, rusqlite::params_from_iter(&param_refs))?;

        Ok(updated > 0)
    }

    pub fn get_company_profile(&self, id: &str) -> Result<Option<CompanyProfile>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, cpv_codes, categories, location, created_at, embedding
             FROM company_profiles WHERE id = ?1",
        )?;
        let mut rows = stmt.query(rusqlite::params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(row_to_company_profile(row)?)),
            None => Ok(None),
        }
    }

    pub fn list_company_profiles(&self) -> Result<Vec<CompanyProfileSummary>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, cpv_codes, categories, location, created_at, embedding
             FROM company_profiles ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let cpv_json: Option<String> = row.get(2)?;
            let cat_json: Option<String> = row.get(3)?;
            let embedding: Option<Vec<u8>> = row.get(6)?;
            Ok(CompanyProfileSummary {
                id: row.get(0)?,
                name: row.get(1)?,
                cpv_codes: cpv_json
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default(),
                categories: cat_json
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default(),
                location: row.get(4)?,
                created_at: row.get(5)?,
                has_embedding: embedding.is_some(),
            })
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn delete_company_profile(&self, id: &str) -> Result<bool> {
        let deleted = self.conn.execute(
            "DELETE FROM company_profiles WHERE id = ?1",
            rusqlite::params![id],
        )?;
        Ok(deleted > 0)
    }

    pub fn set_profile_embedding(&self, profile_id: &str, embedding: &[f32]) -> Result<()> {
        let blob: &[u8] = bytemuck::cast_slice(embedding);
        self.conn.execute(
            "UPDATE company_profiles SET embedding = ?1 WHERE id = ?2",
            rusqlite::params![blob, profile_id],
        ).context("setting profile embedding")?;
        Ok(())
    }

    pub fn get_profile_embedding(&self, profile_id: &str) -> Result<Option<Vec<f32>>> {
        let mut stmt = self.conn.prepare(
            "SELECT embedding FROM company_profiles WHERE id = ?1",
        )?;
        let mut rows = stmt.query(rusqlite::params![profile_id])?;
        match rows.next()? {
            Some(row) => {
                let blob: Option<Vec<u8>> = row.get(0)?;
                match blob {
                    Some(b) => {
                        let floats: Vec<f32> = bytemuck::cast_slice(&b).to_vec();
                        Ok(Some(floats))
                    }
                    None => Ok(None),
                }
            }
            None => Ok(None),
        }
    }

    pub fn company_profile_count(&self) -> Result<usize> {
        let count: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM company_profiles", [], |row| {
                row.get(0)
            })?;
        Ok(count)
    }

    pub fn unembedded_profile_count(&self) -> Result<usize> {
        let count: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM company_profiles WHERE embedding IS NULL",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }
}

fn row_to_company_profile(row: &rusqlite::Row<'_>) -> rusqlite::Result<CompanyProfile> {
    let cpv_json: Option<String> = row.get(3)?;
    let cat_json: Option<String> = row.get(4)?;
    let embedding: Option<Vec<u8>> = row.get(7)?;
    Ok(CompanyProfile {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        cpv_codes: cpv_json
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default(),
        categories: cat_json
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default(),
        location: row.get(5)?,
        created_at: row.get(6)?,
        has_embedding: embedding.is_some(),
    })
}
