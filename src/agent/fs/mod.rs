use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    Request,
};
use libc::ENOENT;
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::time::{Duration, UNIX_EPOCH};
use tracing::{debug, warn};

struct Dir {
    entries: BTreeMap<OsString, u64>,
}

impl Dir {
    fn new() -> Self {
        let entries = BTreeMap::new();
        Self { entries }
    }

    fn insert(&mut self, name: OsString, ino: u64) {
        self.entries.insert(name, ino);
    }
}

pub struct AgentFS {
    inodes: BTreeMap<u64, FileAttr>,
    dirs: BTreeMap<u64, Dir>,
}

impl AgentFS {
    pub fn new() -> Self {
        let mut inodes = BTreeMap::new();
        inodes.insert(1, create_dir_attr(1));
        let mut dirs = BTreeMap::new();
        let mut root_dir = Dir::new();
        root_dir.insert("status".to_string().into(), 1);
        dirs.insert(0, root_dir);
        Self { inodes, dirs }
    }
}

fn create_dir_attr(ino: u64) -> FileAttr {
    FileAttr {
        ino,
        size: 0,
        blocks: 0,
        atime: UNIX_EPOCH, // 1970-01-01 00:00:00
        mtime: UNIX_EPOCH,
        ctime: UNIX_EPOCH,
        crtime: UNIX_EPOCH,
        kind: FileType::Directory,
        perm: 0o755,
        nlink: 2,
        uid: 1000,
        gid: 1000,
        rdev: 0,
        flags: 0,
        blksize: 512,
    }
}

impl Filesystem for AgentFS {
    fn getattr(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyAttr) {
        if let Some(x) = self.inodes.get(&ino) {
            let ttl = get_ttl();
            reply.attr(&ttl, x);
        } else {
            warn!("not found getattr(ino: {:#x?})", ino);
            reply.error(ENOENT);
        }
    }

    fn readdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        assert_eq!(0, offset);
        if let Some(dir) = self.dirs.get(&ino) {
            let _ = reply.add(ino, 1, FileType::Directory, ".");
            let _ = reply.add(ino, 2, FileType::Directory, "..");
            let mut idx = offset + 3;
            for (entry, node) in dir.entries.iter().skip(offset as usize) {
                if let Some(attr) = self.inodes.get(node) {
                    if !reply.add(*node, idx, attr.kind, entry.to_str().unwrap()) {
                        break;
                    }
                    idx += 1;
                }
            }
        } else {
            warn!("readdir(ino: {:#x?}) failed", ino);
            reply.error(ENOENT);
        }
    }
}

fn get_ttl() -> Duration {
    Duration::from_secs(30)
}
