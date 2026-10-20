//! Persistent, bounded, de-duplicated clipboard history (SQLite + PNG files).

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};

use crate::config::MAX_ITEMS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Text,
    Image,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Kind::Text => "text",
            Kind::Image => "image",
        }
    }
    fn parse(s: &str) -> Kind {
        if s == "image" {
            Kind::Image
        } else {
            Kind::Text
        }
    }
}

/// A freshly captured item, ready to be stored.
#[derive(Debug, Clone, Default)]
pub struct NewItem {
    pub text: Option<String>,
    pub rtf: Option<Vec<u8>>,
    pub html: Option<Vec<u8>>,
    pub image_png: Option<Vec<u8>>,
    pub thumb_png: Option<Vec<u8>>,
    pub source_app: Option<String>,
}

impl NewItem {
    pub fn kind(&self) -> Kind {
        if self.image_png.is_some() && self.text.is_none() {
            Kind::Image
        } else {
            Kind::Text
        }
    }

    /// Stable content hash used for de-duplication.
    pub fn hash(&self) -> String {
        let mut h = Sha256::new();
        match self.kind() {
            Kind::Text => {
                h.update(b"t:");
                h.update(self.text.as_deref().unwrap_or("").as_bytes());
            }
            Kind::Image => {
                h.update(b"i:");
                h.update(self.image_png.as_deref().unwrap_or(&[]));
            }
        }
        h.finalize().iter().map(|b| format!("{b:02x}")).collect()
    }
}

/// A stored entry as shown in the panel (large payloads not loaded).
#[derive(Debug, Clone)]
pub struct Entry {
    pub id: i64,
    pub kind: Kind,
    pub preview: String,
    pub thumb_path: Option<PathBuf>,
    pub created_at: i64,
    pub source_app: Option<String>,
}

/// Full payload used to restore an entry to the pasteboard.
#[derive(Debug, Clone, Default)]
pub struct Payload {
    pub text: Option<String>,
    pub rtf: Option<Vec<u8>>,
    pub html: Option<Vec<u8>>,
    pub image_png: Option<Vec<u8>>,
}

pub struct Store {
    conn: Connection,
    dir: PathBuf,
    limit: usize,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn restrict_file(p: &Path) {
    let _ = fs::set_permissions(p, fs::Permissions::from_mode(0o600));
}

fn make_private_dir(p: &Path) -> Result<()> {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(p)?;
    fs::set_permissions(p, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

impl Store {
    pub fn open(dir: &Path) -> Result<Store> {
        Self::open_with_limit(dir, MAX_ITEMS)
    }

    pub fn open_with_limit(dir: &Path, limit: usize) -> Result<Store> {
        make_private_dir(dir)?;
        make_private_dir(&dir.join("images"))?;
        make_private_dir(&dir.join("thumbs"))?;
        let db = dir.join("history.db");
        let conn = Connection::open(&db).context("open history.db")?;
        restrict_file(&db);
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS items(
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                seq INTEGER NOT NULL,
                kind TEXT NOT NULL,
                text TEXT,
                preview TEXT NOT NULL,
                rtf BLOB,
                html BLOB,
                image_file TEXT,
                thumb_file TEXT,
                hash TEXT NOT NULL UNIQUE,
                created_at INTEGER NOT NULL,
                source_app TEXT
            );
            CREATE INDEX IF NOT EXISTS items_seq ON items(seq DESC);",
        )?;
        Ok(Store {
            conn,
            dir: dir.to_path_buf(),
            limit,
        })
    }

    /// Insert a new entry or bump an identical one to the top; trims to the limit.
    pub fn add(&mut self, item: &NewItem) -> Result<i64> {
        let hash = item.hash();
        let kind = item.kind();
        let tx = self.conn.transaction()?;
        let next_seq: i64 =
            tx.query_row("SELECT COALESCE(MAX(seq),0)+1 FROM items", [], |r| r.get(0))?;
        let existing: Option<i64> = tx
            .query_row("SELECT id FROM items WHERE hash=?1", [&hash], |r| r.get(0))
            .optional()?;
        let id = if let Some(id) = existing {
            tx.execute(
                "UPDATE items SET seq=?1, created_at=?2, rtf=?3, html=?4, source_app=COALESCE(?5, source_app) WHERE id=?6",
                params![next_seq, now_ms(), item.rtf, item.html, item.source_app, id],
            )?;
            id
        } else {
            let (image_file, thumb_file) = if kind == Kind::Image {
                let img = format!("images/{hash}.png");
                let th = format!("thumbs/{hash}.png");
                write_private(
                    &self.dir.join(&img),
                    item.image_png.as_deref().unwrap_or(&[]),
                )?;
                let th_opt = match &item.thumb_png {
                    Some(t) => {
                        write_private(&self.dir.join(&th), t)?;
                        Some(th)
                    }
                    None => None,
                };
                (Some(img), th_opt)
            } else {
                (None, None)
            };
            let preview = match kind {
                Kind::Text => make_preview(item.text.as_deref().unwrap_or("")),
                Kind::Image => "Image".to_string(),
            };
            tx.execute(
                "INSERT INTO items(seq,kind,text,preview,rtf,html,image_file,thumb_file,hash,created_at,source_app)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![
                    next_seq, kind.as_str(), item.text, preview, item.rtf, item.html,
                    image_file, thumb_file, hash, now_ms(), item.source_app
                ],
            )?;
            tx.last_insert_rowid()
        };
        // Trim: collect evicted rows' files, delete them.
        let mut evicted_files: Vec<String> = Vec::new();
        {
            let mut stmt = tx.prepare(
                "SELECT id, image_file, thumb_file FROM items ORDER BY seq DESC LIMIT -1 OFFSET ?1",
            )?;
            let rows = stmt.query_map([self.limit as i64], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            })?;
            let mut ids = Vec::new();
            for row in rows {
                let (rid, a, b) = row?;
                ids.push(rid);
                evicted_files.extend(a);
                evicted_files.extend(b);
            }
            drop(stmt);
            for rid in ids {
                tx.execute("DELETE FROM items WHERE id=?1", [rid])?;
            }
        }
        tx.commit()?;
        for f in evicted_files {
            let _ = fs::remove_file(self.dir.join(f));
        }
        Ok(id)
    }

    /// Entries newest first.
    pub fn list(&self) -> Result<Vec<Entry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, preview, thumb_file, created_at, source_app FROM items ORDER BY seq DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            let thumb: Option<String> = r.get(3)?;
            Ok(Entry {
                id: r.get(0)?,
                kind: Kind::parse(&r.get::<_, String>(1)?),
                preview: r.get(2)?,
                thumb_path: thumb.map(|t| self.dir.join(t)),
                created_at: r.get(4)?,
                source_app: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// Entries whose text (or source app) contains `query`, case-insensitively.
    /// An empty query returns everything.
    pub fn search(&self, query: &str) -> Result<Vec<Entry>> {
        let q = query.trim();
        if q.is_empty() {
            return self.list();
        }
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, preview, thumb_file, created_at, source_app FROM items
             WHERE instr(lower(COALESCE(text,'')), lower(?1)) > 0
                OR instr(lower(COALESCE(source_app,'')), lower(?1)) > 0
             ORDER BY seq DESC",
        )?;
        let rows = stmt.query_map([q], |r| {
            let thumb: Option<String> = r.get(3)?;
            Ok(Entry {
                id: r.get(0)?,
                kind: Kind::parse(&r.get::<_, String>(1)?),
                preview: r.get(2)?,
                thumb_path: thumb.map(|t| self.dir.join(t)),
                created_at: r.get(4)?,
                source_app: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn payload(&self, id: i64) -> Result<Option<Payload>> {
        let row = self
            .conn
            .query_row(
                "SELECT text, rtf, html, image_file FROM items WHERE id=?1",
                [id],
                |r| {
                    Ok((
                        r.get::<_, Option<String>>(0)?,
                        r.get::<_, Option<Vec<u8>>>(1)?,
                        r.get::<_, Option<Vec<u8>>>(2)?,
                        r.get::<_, Option<String>>(3)?,
                    ))
                },
            )
            .optional()?;
        Ok(row.map(|(text, rtf, html, img)| Payload {
            text,
            rtf,
            html,
            image_png: img.and_then(|f| fs::read(self.dir.join(f)).ok()),
        }))
    }

    /// Move an existing entry to the top. Returns false if it no longer exists.
    pub fn bump(&mut self, id: i64) -> Result<bool> {
        let n = self.conn.execute(
            "UPDATE items SET seq=(SELECT COALESCE(MAX(seq),0)+1 FROM items), created_at=?1 WHERE id=?2",
            params![now_ms(), id],
        )?;
        Ok(n > 0)
    }

    pub fn clear(&mut self) -> Result<()> {
        self.conn.execute("DELETE FROM items", [])?;
        for sub in ["images", "thumbs"] {
            if let Ok(rd) = fs::read_dir(self.dir.join(sub)) {
                for e in rd.flatten() {
                    let _ = fs::remove_file(e.path());
                }
            }
        }
        Ok(())
    }

    pub fn count(&self) -> Result<usize> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM items", [], |r| r.get::<_, i64>(0))?
            as usize)
    }
}

fn write_private(path: &Path, data: &[u8]) -> Result<()> {
    fs::write(path, data)?;
    restrict_file(path);
    Ok(())
}

fn make_preview(text: &str) -> String {
    let collapsed: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.chars().take(300).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn txt(s: &str) -> NewItem {
        NewItem {
            text: Some(s.into()),
            ..Default::default()
        }
    }
    fn img(bytes: &[u8]) -> NewItem {
        NewItem {
            image_png: Some(bytes.to_vec()),
            thumb_png: Some(b"th".to_vec()),
            ..Default::default()
        }
    }

    #[test]
    fn keeps_only_20_newest() {
        let d = tempfile::tempdir().unwrap();
        let mut s = Store::open(d.path()).unwrap();
        for i in 0..25 {
            s.add(&txt(&format!("item {i}"))).unwrap();
        }
        let l = s.list().unwrap();
        assert_eq!(l.len(), 20);
        assert_eq!(l[0].preview, "item 24");
        assert_eq!(l[19].preview, "item 5");
    }

    #[test]
    fn recopy_moves_to_top_without_duplicate() {
        let d = tempfile::tempdir().unwrap();
        let mut s = Store::open(d.path()).unwrap();
        s.add(&txt("a")).unwrap();
        s.add(&txt("b")).unwrap();
        s.add(&txt("c")).unwrap();
        s.add(&txt("a")).unwrap();
        let l: Vec<_> = s.list().unwrap().into_iter().map(|e| e.preview).collect();
        assert_eq!(l, vec!["a", "c", "b"]);
    }

    #[test]
    fn persists_across_reopen() {
        let d = tempfile::tempdir().unwrap();
        {
            let mut s = Store::open(d.path()).unwrap();
            s.add(&txt("one")).unwrap();
            s.add(&txt("two")).unwrap();
        }
        let s = Store::open(d.path()).unwrap();
        let l: Vec<_> = s.list().unwrap().into_iter().map(|e| e.preview).collect();
        assert_eq!(l, vec!["two", "one"]);
    }

    #[test]
    fn eviction_deletes_image_files() {
        let d = tempfile::tempdir().unwrap();
        let mut s = Store::open_with_limit(d.path(), 2).unwrap();
        s.add(&img(b"first-image")).unwrap();
        let first = s.list().unwrap()[0].thumb_path.clone().unwrap();
        assert!(first.exists());
        s.add(&txt("x")).unwrap();
        s.add(&txt("y")).unwrap();
        assert!(!first.exists());
        assert_eq!(
            std::fs::read_dir(d.path().join("images")).unwrap().count(),
            0
        );
    }

    #[test]
    fn search_is_case_insensitive_and_full_text() {
        let d = tempfile::tempdir().unwrap();
        let mut s = Store::open(d.path()).unwrap();
        s.add(&txt("Invoice 2024")).unwrap();
        s.add(&txt("lunch plans")).unwrap();
        let long = format!("{} needle", "x ".repeat(400));
        s.add(&txt(&long)).unwrap();
        assert_eq!(s.search("INVOICE").unwrap().len(), 1);
        assert_eq!(
            s.search("needle").unwrap().len(),
            1,
            "matches beyond the preview"
        );
        assert_eq!(s.search("").unwrap().len(), 3);
        assert!(s.search("nothing").unwrap().is_empty());
    }

    #[test]
    fn clear_removes_everything() {
        let d = tempfile::tempdir().unwrap();
        let mut s = Store::open(d.path()).unwrap();
        s.add(&img(b"abc")).unwrap();
        s.add(&txt("t")).unwrap();
        s.clear().unwrap();
        assert_eq!(s.count().unwrap(), 0);
        assert_eq!(
            std::fs::read_dir(d.path().join("images")).unwrap().count(),
            0
        );
        assert_eq!(
            std::fs::read_dir(d.path().join("thumbs")).unwrap().count(),
            0
        );
    }

    #[test]
    fn payload_round_trip_keeps_rich_formats() {
        let d = tempfile::tempdir().unwrap();
        let mut s = Store::open(d.path()).unwrap();
        let id = s
            .add(&NewItem {
                text: Some("hi".into()),
                rtf: Some(b"{\\rtf1 hi}".to_vec()),
                html: Some(b"<b>hi</b>".to_vec()),
                ..Default::default()
            })
            .unwrap();
        let p = s.payload(id).unwrap().unwrap();
        assert_eq!(p.text.as_deref(), Some("hi"));
        assert_eq!(p.rtf.as_deref(), Some(&b"{\\rtf1 hi}"[..]));
        assert_eq!(p.html.as_deref(), Some(&b"<b>hi</b>"[..]));
    }

    #[test]
    fn image_payload_and_bump() {
        let d = tempfile::tempdir().unwrap();
        let mut s = Store::open(d.path()).unwrap();
        let id = s.add(&img(b"png-bytes")).unwrap();
        s.add(&txt("later")).unwrap();
        assert!(s.bump(id).unwrap());
        assert_eq!(s.list().unwrap()[0].id, id);
        assert_eq!(
            s.payload(id).unwrap().unwrap().image_png.unwrap(),
            b"png-bytes"
        );
    }

    #[test]
    fn storage_is_private() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("MacClip");
        let mut s = Store::open(&root).unwrap();
        s.add(&img(b"zzz")).unwrap();
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&root), 0o700);
        assert_eq!(mode(&root.join("history.db")), 0o600);
        let f = std::fs::read_dir(root.join("images"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert_eq!(mode(&f), 0o600);
    }
}
