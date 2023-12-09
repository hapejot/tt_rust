use bytebuffer::ByteBuffer;
use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    Request,
};
use libc::ENOENT;
use std::cmp::min;
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};

use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn};

struct BytesContent(ByteBuffer);

impl BytesContent {
    pub fn new_from_str(val: &str) -> Self {
        let mut bb = ByteBuffer::new();
        bb.write_string(val);
        BytesContent(bb)
    }

    pub fn len(&self) -> u64 {
        self.0.len() as u64
    }

    pub fn read(&self, offset: i64, size: u32) -> &[u8] {
        let buf = &self.0;
        let from = offset as usize;
        let to = min(offset as usize + size as usize, buf.len());
        &buf.as_bytes()[from..to]
    }
}

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
    next_inode: u64,
    inodes: BTreeMap<u64, FileAttr>,
    dirs: BTreeMap<u64, Dir>,
    content: BTreeMap<u64, BytesContent>,
}

impl AgentFS {
    pub fn new() -> Self {
        let inodes = BTreeMap::new();
        let dirs = BTreeMap::new();
        let next_inode = 1;
        let content = BTreeMap::new();
        let mut r = Self {
            inodes,
            dirs,
            next_inode,
            content,
        };
        let root_dir = r.create_dir();
        let c = BytesContent::new_from_str("Das ist ein übler Test.\n");
        let status_file = r.create_file(c);
        let root_dir = r.dirs.get_mut(&root_dir).unwrap();
        root_dir.insert("test".to_string().into(), status_file);
        r
    }

    fn create_file_inode(&mut self) -> u64 {
        let inodes = &mut self.inodes;
        let ino = self.next_inode;
        self.next_inode += 1;
        let attr = FileAttr {
            ino,
            size: 0,
            blocks: 0,
            atime: SystemTime::now(),
            mtime: SystemTime::now(),
            ctime: SystemTime::now(),
            crtime: SystemTime::now(),
            kind: FileType::RegularFile,
            perm: 0o755,
            nlink: 2,
            uid: 1000,
            gid: 1000,
            rdev: 0,
            flags: 0,
            blksize: 512,
        };
        inodes.insert(ino, attr);
        ino
    }

    fn create_dir_inode(&mut self) -> u64 {
        let inodes = &mut self.inodes;
        let ino = self.next_inode;
        self.next_inode += 1;
        let attr = FileAttr {
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
        };

        inodes.insert(ino, attr);
        ino
    }

    fn create_dir(&mut self) -> u64 {
        let ino = self.create_dir_inode();
        self.dirs.insert(ino, Dir::new());
        // self.dirs.get_mut(&ino).unwrap()
        ino
    }

    fn create_file(&mut self, c: BytesContent) -> u64 {
        let ino = self.create_file_inode();
        let n = c.len();
        self.content.insert(ino, c);
        let attr = self.inodes.get_mut(&ino).unwrap();
        attr.size = n;
        ino
    }
}

impl Filesystem for AgentFS {
    fn getattr(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyAttr) {
        info!("getattr {ino}");
        if let Some(x) = self.inodes.get(&ino) {
            let ttl = get_ttl();
            reply.attr(&ttl, x);
            info!("returned attributes {:?}", x);
        } else {
            warn!("not found getattr(ino: {:#x?})", ino);
            reply.error(ENOENT);
            error!("not found");
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
        info!("readdir {ino}");
        if let Some(dir) = self.dirs.get(&ino) {
            if offset == 0 {
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
            }
            reply.ok();
        } else {
            warn!("readdir(ino: {:#x?}) failed", ino);
            reply.error(ENOENT);
        }
    }

    fn init(
        &mut self,
        _req: &Request<'_>,
        _config: &mut fuser::KernelConfig,
    ) -> Result<(), libc::c_int> {
        info!("init");
        Ok(())
    }

    fn destroy(&mut self) {}

    fn lookup(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if let Some(d) = self.dirs.get(&parent) {
            if let Some(e) = d.entries.get(name) {
                if let Some(attr) = self.inodes.get(e) {
                    reply.entry(&get_ttl(), attr, 0);
                } else {
                    error!("inode {e} referred to by {name:?} was not found.");
                    reply.error(libc::ENOENT);
                }
            } else {
                reply.error(libc::ENOENT);
            }
        } else {
            error!("directory with inode {parent} doesn't exist.");
            reply.error(libc::ENOENT);
        }
    }

    fn forget(&mut self, _req: &Request<'_>, _ino: u64, _nlookup: u64) {}

    fn setattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        _atime: Option<fuser::TimeOrNow>,
        _mtime: Option<fuser::TimeOrNow>,
        _ctime: Option<std::time::SystemTime>,
        fh: Option<u64>,
        _crtime: Option<std::time::SystemTime>,
        _chgtime: Option<std::time::SystemTime>,
        _bkuptime: Option<std::time::SystemTime>,
        flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        debug!(
            "[Not Implemented] setattr(ino: {:#x?}, mode: {:?}, uid: {:?}, \\
            gid: {:?}, size: {:?}, fh: {:?}, flags: {:?})",
            ino, mode, uid, gid, size, fh, flags
        );
        reply.error(libc::ENOSYS);
    }

    fn readlink(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyData) {
        debug!("[Not Implemented] readlink(ino: {:#x?})", ino);
        reply.error(libc::ENOSYS);
    }

    fn mknod(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        rdev: u32,
        reply: ReplyEntry,
    ) {
        debug!(
            "[Not Implemented] mknod(parent: {:#x?}, name: {:?}, mode: {}, \\
            umask: {:#x?}, rdev: {})",
            parent, name, mode, umask, rdev
        );
        reply.error(libc::ENOSYS);
    }

    fn mkdir(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        reply: ReplyEntry,
    ) {
        debug!(
            "[Not Implemented] mkdir(parent: {:#x?}, name: {:?}, mode: {}, umask: {:#x?})",
            parent, name, mode, umask
        );
        reply.error(libc::ENOSYS);
    }

    fn unlink(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: fuser::ReplyEmpty) {
        debug!(
            "[Not Implemented] unlink(parent: {:#x?}, name: {:?})",
            parent, name,
        );
        reply.error(libc::ENOSYS);
    }

    fn rmdir(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: fuser::ReplyEmpty) {
        debug!(
            "[Not Implemented] rmdir(parent: {:#x?}, name: {:?})",
            parent, name,
        );
        reply.error(libc::ENOSYS);
    }

    fn symlink(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        link_name: &OsStr,
        target: &std::path::Path,
        reply: ReplyEntry,
    ) {
        debug!(
            "[Not Implemented] symlink(parent: {:#x?}, link_name: {:?}, target: {:?})",
            parent, link_name, target,
        );
        reply.error(libc::EPERM);
    }

    fn rename(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        newparent: u64,
        newname: &OsStr,
        flags: u32,
        reply: fuser::ReplyEmpty,
    ) {
        debug!(
            "[Not Implemented] rename(parent: {:#x?}, name: {:?}, newparent: {:#x?}, \\
            newname: {:?}, flags: {})",
            parent, name, newparent, newname, flags,
        );
        reply.error(libc::ENOSYS);
    }

    fn link(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        newparent: u64,
        newname: &OsStr,
        reply: ReplyEntry,
    ) {
        debug!(
            "[Not Implemented] link(ino: {:#x?}, newparent: {:#x?}, newname: {:?})",
            ino, newparent, newname
        );
        reply.error(libc::EPERM);
    }

    fn open(&mut self, _req: &Request<'_>, _ino: u64, _flags: i32, reply: fuser::ReplyOpen) {
        reply.opened(0, 0);
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        if let Some(c) = self.content.get(&ino) {
            reply.data(c.read(offset, size));
        } else {
            reply.error(libc::ENOENT);
        }
    }

    fn write(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        data: &[u8],
        write_flags: u32,
        flags: i32,
        lock_owner: Option<u64>,
        reply: fuser::ReplyWrite,
    ) {
        debug!(
            "[Not Implemented] write(ino: {:#x?}, fh: {}, offset: {}, data.len(): {}, \\
            write_flags: {:#x?}, flags: {:#x?}, lock_owner: {:?})",
            ino,
            fh,
            offset,
            data.len(),
            write_flags,
            flags,
            lock_owner
        );
        reply.error(libc::ENOSYS);
    }

    fn flush(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        lock_owner: u64,
        reply: fuser::ReplyEmpty,
    ) {
        debug!(
            "[Not Implemented] flush(ino: {:#x?}, fh: {}, lock_owner: {:?})",
            ino, fh, lock_owner
        );
        reply.error(libc::ENOSYS);
    }

    fn release(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: fuser::ReplyEmpty,
    ) {
        reply.ok();
    }

    fn fsync(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        datasync: bool,
        reply: fuser::ReplyEmpty,
    ) {
        debug!(
            "[Not Implemented] fsync(ino: {:#x?}, fh: {}, datasync: {})",
            ino, fh, datasync
        );
        reply.error(libc::ENOSYS);
    }

    fn opendir(&mut self, _req: &Request<'_>, _ino: u64, _flags: i32, reply: fuser::ReplyOpen) {
        reply.opened(0, 0);
    }

    fn readdirplus(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        reply: fuser::ReplyDirectoryPlus,
    ) {
        debug!(
            "[Not Implemented] readdirplus(ino: {:#x?}, fh: {}, offset: {})",
            ino, fh, offset
        );
        reply.error(libc::ENOSYS);
    }

    fn releasedir(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _flags: i32,
        reply: fuser::ReplyEmpty,
    ) {
        reply.ok();
    }

    fn fsyncdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        datasync: bool,
        reply: fuser::ReplyEmpty,
    ) {
        debug!(
            "[Not Implemented] fsyncdir(ino: {:#x?}, fh: {}, datasync: {})",
            ino, fh, datasync
        );
        reply.error(libc::ENOSYS);
    }

    fn statfs(&mut self, _req: &Request<'_>, _ino: u64, reply: fuser::ReplyStatfs) {
        reply.statfs(0, 0, 0, 0, 0, 512, 255, 0);
    }

    fn setxattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        name: &OsStr,
        _value: &[u8],
        flags: i32,
        position: u32,
        reply: fuser::ReplyEmpty,
    ) {
        debug!(
            "[Not Implemented] setxattr(ino: {:#x?}, name: {:?}, flags: {:#x?}, position: {})",
            ino, name, flags, position
        );
        reply.error(libc::ENOSYS);
    }

    fn getxattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        name: &OsStr,
        size: u32,
        reply: fuser::ReplyXattr,
    ) {
        debug!(
            "[Not Implemented] getxattr(ino: {:#x?}, name: {:?}, size: {})",
            ino, name, size
        );
        reply.error(libc::ENOSYS);
    }

    fn listxattr(&mut self, _req: &Request<'_>, ino: u64, size: u32, reply: fuser::ReplyXattr) {
        debug!(
            "[Not Implemented] listxattr(ino: {:#x?}, size: {})",
            ino, size
        );
        reply.error(libc::ENOSYS);
    }

    fn removexattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        name: &OsStr,
        reply: fuser::ReplyEmpty,
    ) {
        debug!(
            "[Not Implemented] removexattr(ino: {:#x?}, name: {:?})",
            ino, name
        );
        reply.error(libc::ENOSYS);
    }

    fn access(&mut self, _req: &Request<'_>, ino: u64, mask: i32, reply: fuser::ReplyEmpty) {
        debug!("[Not Implemented] access(ino: {:#x?}, mask: {})", ino, mask);
        // reply.error(libc::ENOSYS);
        reply.ok()
    }

    fn create(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        flags: i32,
        reply: fuser::ReplyCreate,
    ) {
        debug!(
            "[Not Implemented] create(parent: {:#x?}, name: {:?}, mode: {}, umask: {:#x?}, \\
            flags: {:#x?})",
            parent, name, mode, umask, flags
        );
        reply.error(libc::ENOSYS);
    }

    fn getlk(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        lock_owner: u64,
        start: u64,
        end: u64,
        typ: i32,
        pid: u32,
        reply: fuser::ReplyLock,
    ) {
        debug!(
            "[Not Implemented] getlk(ino: {:#x?}, fh: {}, lock_owner: {}, start: {}, \\
            end: {}, typ: {}, pid: {})",
            ino, fh, lock_owner, start, end, typ, pid
        );
        reply.error(libc::ENOSYS);
    }

    fn setlk(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        lock_owner: u64,
        start: u64,
        end: u64,
        typ: i32,
        pid: u32,
        sleep: bool,
        reply: fuser::ReplyEmpty,
    ) {
        debug!(
            "[Not Implemented] setlk(ino: {:#x?}, fh: {}, lock_owner: {}, start: {}, \\
            end: {}, typ: {}, pid: {}, sleep: {})",
            ino, fh, lock_owner, start, end, typ, pid, sleep
        );
        reply.error(libc::ENOSYS);
    }

    fn bmap(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        blocksize: u32,
        idx: u64,
        reply: fuser::ReplyBmap,
    ) {
        debug!(
            "[Not Implemented] bmap(ino: {:#x?}, blocksize: {}, idx: {})",
            ino, blocksize, idx,
        );
        reply.error(libc::ENOSYS);
    }

    fn ioctl(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        flags: u32,
        cmd: u32,
        in_data: &[u8],
        out_size: u32,
        reply: fuser::ReplyIoctl,
    ) {
        debug!(
            "[Not Implemented] ioctl(ino: {:#x?}, fh: {}, flags: {}, cmd: {}, \\
            in_data.len(): {}, out_size: {})",
            ino,
            fh,
            flags,
            cmd,
            in_data.len(),
            out_size,
        );
        reply.error(libc::ENOSYS);
    }

    fn fallocate(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        length: i64,
        mode: i32,
        reply: fuser::ReplyEmpty,
    ) {
        debug!(
            "[Not Implemented] fallocate(ino: {:#x?}, fh: {}, offset: {}, \\
            length: {}, mode: {})",
            ino, fh, offset, length, mode
        );
        reply.error(libc::ENOSYS);
    }

    fn lseek(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        whence: i32,
        reply: fuser::ReplyLseek,
    ) {
        debug!(
            "[Not Implemented] lseek(ino: {:#x?}, fh: {}, offset: {}, whence: {})",
            ino, fh, offset, whence
        );
        reply.error(libc::ENOSYS);
    }

    fn copy_file_range(
        &mut self,
        _req: &Request<'_>,
        ino_in: u64,
        fh_in: u64,
        offset_in: i64,
        ino_out: u64,
        fh_out: u64,
        offset_out: i64,
        len: u64,
        flags: u32,
        reply: fuser::ReplyWrite,
    ) {
        debug!(
            "[Not Implemented] copy_file_range(ino_in: {:#x?}, fh_in: {}, \\
            offset_in: {}, ino_out: {:#x?}, fh_out: {}, offset_out: {}, \\
            len: {}, flags: {})",
            ino_in, fh_in, offset_in, ino_out, fh_out, offset_out, len, flags
        );
        reply.error(libc::ENOSYS);
    }
}

fn get_ttl() -> Duration {
    Duration::from_secs(30)
}
