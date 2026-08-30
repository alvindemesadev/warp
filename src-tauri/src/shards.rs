// Workload partitioner — splits a source tree into DISJOINT directory shards.
//
// Safety invariant: every file in the source belongs to exactly one shard, so
// parallel workers can never touch the same source file or write to the same
// destination subtree. Coverage + disjointness are enforced structurally:
//   - each immediate child directory maps to its own shard (recursive /E copy)
//   - loose files at any split level form a "/LEV:1" root-files shard
//   - a dominant child (see `should_split`) is recursively split by its own
//     children so one huge folder cannot serialize the whole job
//
// The destination mapping preserves the source-relative path, mirroring what a
// single robocopy invocation would have produced.

pub const MAX_SPLIT_DEPTH: u32 = 2;
/// A child must hold at least this share of TOTAL job bytes to be worth splitting.
const DOMINANT_PCT: u64 = 40;
/// Never split below this size — orchestration overhead dominates otherwise.
const MIN_SPLIT_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct Shard {
    pub id: u64,
    pub src: String,
    pub dst: String,
    pub est_bytes: u64,
    pub est_files: u64,
    /// true -> copy only files directly inside `src` (`/LEV:1`), not subdirs.
    pub root_only: bool,
}

/// Split `source` into shards targeting `effective_dest`. Returns an empty vec
/// when the source has nothing to copy (caller falls back to the sequential
/// path, which handles the empty/indeterminate case).
pub fn partition(source: &str, effective_dest: &str) -> Vec<Shard> {
    let (total_bytes, _total_files) = crate::dir_stats(source);
    if total_bytes == 0 && !has_any_entry(source) {
        return vec![];
    }
    let mut next_id = 0u64;
    let mut shards = Vec::new();
    split_dir(source, effective_dest, total_bytes, 0, &mut next_id, &mut shards);
    for (i, s) in shards.iter_mut().enumerate() {
        s.id = (i + 1) as u64;
    }
    shards
}

fn split_dir(
    src_dir: &str,
    dst_dir: &str,
    total_bytes: u64,
    depth: u32,
    next_id: &mut u64,
    out: &mut Vec<Shard>,
) {
    let listing = list_children(src_dir);

    // Loose files at this level -> one root-only shard (/LEV:1).
    if !listing.files.is_empty() {
        let bytes = listing.files.iter().map(|f| f.size).sum();
        *next_id += 1;
        out.push(Shard {
            id: *next_id,
            src: src_dir.to_string(),
            dst: dst_dir.to_string(),
            est_bytes: bytes,
            est_files: listing.files.len() as u64,
            root_only: true,
        });
    }

    for child in &listing.dirs {
        let (bytes, files) = crate::dir_stats(&child.path);
        let child_dst = join_win(dst_dir, &child.name);
        if depth < MAX_SPLIT_DEPTH && should_split(&child.path, bytes, total_bytes) {
            split_dir(&child.path, &child_dst, total_bytes, depth + 1, next_id, out);
        } else {
            *next_id += 1;
            out.push(Shard {
                id: *next_id,
                src: child.path.clone(),
                dst: child_dst,
                est_bytes: bytes,
                est_files: files,
                root_only: false,
            });
        }
    }
}

/// A child is worth splitting when it dominates the job but has enough
/// internal structure that splitting actually creates parallelism.
fn should_split(child_path: &str, child_bytes: u64, total_bytes: u64) -> bool {
    if child_bytes < MIN_SPLIT_BYTES {
        return false;
    }
    if total_bytes == 0 || child_bytes * 100 < total_bytes * DOMINANT_PCT {
        return false;
    }
    list_children(child_path).dirs.len() >= 2
}

struct Entry {
    name: String,
    path: String,
}

struct FileEntry {
    name: String,
    #[allow(dead_code)]
    path: String,
    size: u64,
}

struct ChildrenList {
    dirs: Vec<Entry>,
    files: Vec<FileEntry>,
}

fn list_children(dir: &str) -> ChildrenList {
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    let long = crate::to_long_path(dir);
    if let Ok(rd) = std::fs::read_dir(&long) {
        for entry in rd.flatten() {
            let Ok(ft) = entry.file_type() else { continue };
            if ft.is_symlink() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path().to_string_lossy().to_string();
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                dirs.push(Entry { name, path });
            } else if meta.is_file() {
                let size = meta.len();
                files.push(FileEntry { name, path, size });
            }
        }
    }
    dirs.sort_by(|a, b| a.name.cmp(&b.name));
    files.sort_by(|a, b| a.name.cmp(&b.name));
    ChildrenList { dirs, files }
}

fn has_any_entry(dir: &str) -> bool {
    let l = list_children(dir);
    !l.dirs.is_empty() || !l.files.is_empty()
}

/// Windows-style path join (this app is Windows-targeted; robocopy expects `\`).
fn join_win(base: &str, name: &str) -> String {
    format!("{}\\{}", base.trim_end_matches('\\'), name.trim_end_matches('\\'))
}

/// Cheap gate input: how many subdirectories sit directly under `dir`.
pub(crate) fn top_level_dir_count(dir: &str) -> usize {
    list_children(dir).dirs.len()
}

// — Tests ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn tmp_root(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("warp_shards_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    /// All regular files a shard would copy, honoring root_only (/LEV:1).
    fn shard_files(s: &Shard) -> Vec<String> {
        let mut v = Vec::new();
        let long = crate::to_long_path(&s.src);
        if s.root_only {
            if let Ok(rd) = fs::read_dir(&long) {
                for e in rd.flatten() {
                    if e.file_type().map(|t| t.is_file()).unwrap_or(false) {
                        v.push(e.path().to_string_lossy().to_string());
                    }
                }
            }
        } else {
            collect_recursive(std::path::Path::new(&long), &mut v);
        }
        v.sort();
        v
    }

    fn collect_recursive(dir: &std::path::Path, out: &mut Vec<String>) {
        if let Ok(rd) = fs::read_dir(dir) {
            for e in rd.flatten() {
                let Ok(ft) = e.file_type() else { continue };
                if ft.is_symlink() {
                    continue;
                }
                if ft.is_file() {
                    out.push(e.path().to_string_lossy().to_string());
                } else if ft.is_dir() {
                    collect_recursive(&e.path(), out);
                }
            }
        }
    }

    fn all_files_under(root: &PathBuf) -> Vec<String> {
        let mut v = Vec::new();
        collect_recursive(root, &mut v);
        v.sort();
        v
    }

    #[test]
    fn partition_covers_everything_without_overlap() {
        let root = tmp_root("cover");
        let src = root.join("src");
        fs::create_dir_all(src.join("d1/sub")).unwrap();
        fs::create_dir_all(src.join("d2")).unwrap();
        fs::create_dir_all(src.join("empty")).unwrap();
        fs::write(src.join("loose.txt"), "l").unwrap();
        fs::write(src.join("d1/a.txt"), "a").unwrap();
        fs::write(src.join("d1/sub/s.txt"), "s").unwrap();
        fs::write(src.join("d2/b.txt"), "b").unwrap();

        let shards = partition(src.to_str().unwrap(), "D:\\dest");
        assert!(shards.len() >= 3, "expected multiple shards, got {shards:?}");

        // Union == universe, pairwise disjoint.
        let mut union: Vec<String> = shards.iter().flat_map(shard_files).collect();
        union.sort();
        union.dedup();
        assert_eq!(union, all_files_under(&src));

        let mut seen = std::collections::HashSet::new();
        for s in &shards {
            for f in shard_files(s) {
                assert!(seen.insert(f.clone()), "file in two shards: {f}");
            }
        }

        // Destination mapping mirrors structure.
        let d1 = shards.iter().find(|s| s.src.ends_with("d1")).unwrap();
        assert_eq!(d1.dst, "D:\\dest\\d1");
        // Loose files produce a root-only shard on the SOURCE ROOT.
        let lo = shards.iter().find(|s| s.root_only).unwrap();
        assert!(lo.src.ends_with("src"));
        assert_eq!(lo.dst, "D:\\dest");
        assert!(shard_files(lo).len() == 1);

        // Unique ids.
        let ids: std::collections::HashSet<u64> = shards.iter().map(|s| s.id).collect();
        assert_eq!(ids.len(), shards.len());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn dominant_child_is_recursively_split() {
        let root = tmp_root("skew");
        let src = root.join("src");
        let big = src.join("big");
        for d in ["x1", "x2"] {
            fs::create_dir_all(big.join(d)).unwrap();
            let f = fs::File::create(big.join(d).join("blob.dat")).unwrap();
            f.set_len(600 * 1024 * 1024).unwrap(); // sparse — fast
        }
        fs::create_dir_all(src.join("tiny")).unwrap();
        fs::write(src.join("tiny/t.txt"), "t").unwrap();

        let shards = partition(src.to_str().unwrap(), "D:\\dest");
        let big_whole = shards.iter().any(|s| s.src.ends_with("\\big"));
        assert!(!big_whole, "dominant child must be split, got {shards:?}");
        assert!(shards.iter().any(|s| s.src.ends_with("x1")));
        assert!(shards.iter().any(|s| s.src.ends_with("x2")));
        // tiny stays whole.
        assert!(shards.iter().any(|s| s.src.ends_with("tiny") && !s.root_only));

        let mut union: Vec<String> = shards.iter().flat_map(shard_files).collect();
        union.sort();
        union.dedup();
        assert_eq!(union, all_files_under(&src));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn empty_source_yields_no_shards() {
        let root = tmp_root("empty");
        let src = root.join("src");
        fs::create_dir_all(&src).unwrap();
        assert!(partition(src.to_str().unwrap(), "D:\\dest").is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn loose_files_only_single_root_shard() {
        let root = tmp_root("loose");
        let src = root.join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("a.txt"), "aaaa").unwrap();
        fs::write(src.join("b.txt"), "bb").unwrap();
        let shards = partition(src.to_str().unwrap(), "D:\\dest");
        assert_eq!(shards.len(), 1);
        assert!(shards[0].root_only);
        assert_eq!(shards[0].est_files, 2);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn join_win_normalizes_trailing_separators() {
        assert_eq!(join_win("C:\\a\\", "b"), "C:\\a\\b");
        assert_eq!(join_win("C:\\a", "b\\"), "C:\\a\\b");
    }
}
