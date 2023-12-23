use bytebuffer::ByteBuffer;
use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, Request,
};
use libc::ENOENT;
use serde_derive::{Deserialize, Serialize};

use std::cmp::min;
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};

use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;

use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use tracing::{debug, error, info, warn};

use crate::agent::fs::file::FileContent;
use crate::agent::protocol::Message;
use crate::data::model::{DataModel, Table};
use crate::data::{Query, WhereCondition, WhereExpr};
use crate::dbx::{Database, DatabaseBuilder};

pub enum FSError {
    Fail,
}

pub mod file;
pub mod memory;

pub trait ReadingContent {
    fn inode(&self) -> INode;
    fn read(&mut self, offset: i64, size: u32) -> ByteBuffer;
}

pub trait WritingContent {
    fn write(&mut self, offset: i64, size: u32, data: ByteBuffer) -> u32;
    fn flush(&mut self);
}

pub trait Content: ReadingContent + WritingContent {}

pub trait ContentProvider {
    fn get_read(&self, id: i64) -> Result<Box<dyn ReadingContent>, FSError>;
}

struct StatusFile {
    status: Mutex<Option<ByteBuffer>>,
}

impl StatusFile {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(None),
        }
    }
}

fn load_status() -> Option<ByteBuffer> {
    if let Ok(mut socket) = TcpStream::connect("localhost:7778") {
        let msg = Message::ReadStatus;
        let buf = serde_xdr::to_bytes(&msg).unwrap();

        socket.write(&buf[..]).unwrap();

        let mut buf = [0; 30000];
        let n = socket.read(&mut buf).unwrap();
        assert!(n > 0);
        let msg: Message = serde_xdr::from_bytes(&buf[..n]).unwrap();
        let s = serde_yaml::to_string(&msg).unwrap();
        let mut buf = ByteBuffer::new();
        buf.write_string(s.as_str());
        Some(buf)
    } else {
        None
    }
}

impl ReadingContent for StatusFile {
    fn inode(&self) -> INode {
        let mut status = self.status.try_lock().unwrap();
        if *status == None {
            *status = load_status();
        }
        let n = if let Some(st) = &*status {
            st.len() as u64
        } else {
            error!("status could not be loaded.");
            0
        };
        INode {
            id: 0,
            kind: NodeType::File,
            size: n as usize,
        }
    }

    fn read(&mut self, offset: i64, size: u32) -> ByteBuffer {
        let mut status = self.status.try_lock().unwrap();
        if *status == None {
            *status = load_status();
        }
        if let Some(st) = &*status {
            let from = offset as usize;
            let to = min(offset as usize + size as usize, st.len());
            ByteBuffer::from_bytes(&st.as_bytes()[from..to])
        } else {
            error!("status could not be loaded.");
            ByteBuffer::new()
        }
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
    content: BTreeMap<u64, Box<dyn Content>>,
    db: Database,
    files: [Option<Box<dyn Content>>; 10],
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename = "entry")]
struct DirEntry {
    dir: u64,
    name: String,
    inode: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum NodeType {
    Dir,
    File,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "inode")]
pub struct INode {
    pub id: u64,
    #[serde(rename = "type")]
    pub kind: NodeType,
    pub size: usize,
}

fn prepare_database_object() -> Database {
    let model = DataModel::new("fs")
        .table(
            Table::new("dir")
                .field("id", true, "int")
                .field("type", false, "string")
                .field("flags", false, "string"),
        )
        .table(
            Table::new("entry")
                .field("dir", true, "int")
                .field("name", true, "string")
                .field("inode", false, "int"),
        )
        .table(
            Table::new("inode")
                .field("id", true, "int")
                .field("type", false, "int")
                .field("size", false, "int"),
        );

    let builder = DatabaseBuilder::new();
    let db = builder.build();
    db.connect(Some("filesystem.sqlite"));
    db.activate_structure(model);
    db
}

impl AgentFS {
    pub fn new() -> Self {
        let inodes = BTreeMap::new();
        let dirs = BTreeMap::new();
        let mut next_inode = 1;
        let content = BTreeMap::new();
        let db = prepare_database_object();
        let id_result = db.execute_query("select count(*), max(id) from inode");
        let x = &id_result[0];
        let y = x.get_at(0);
        let z = x.get_at(1);
        if u64::from(y.clone()) > 0 {
            next_inode = z.clone().into();
        }
        let r = Self {
            inodes,
            dirs,
            next_inode,
            content,
            db,
            files: [None, None, None, None, None, None, None, None, None, None],
        };
        // let root_dir = r.create_dir();
        // let c = BytesContent::new_from_str("Das ist ein übler Test.\n");
        // let test_file = r.create_file(Box::new(c));
        // let status_file = r.create_file(Box::new(StatusFile::new()));
        // let root_dir = r.dirs.get_mut(&root_dir).unwrap();
        // root_dir.insert("test".to_string().into(), test_file);
        // root_dir.insert("status".to_string().into(), status_file);
        r
    }

    fn create_file_inode(&mut self, size: u64) -> u64 {
        let inodes = &mut self.inodes;
        let ino = self.next_inode;
        self.next_inode += 1;
        let attr = as_file_attr(ino, size);
        inodes.insert(ino, attr);
        ino
    }

    fn create_dir_inode(&mut self) -> u64 {
        let inodes = &mut self.inodes;
        let ino = self.next_inode;
        self.next_inode += 1;
        let attr = as_dir_attr(ino);

        inodes.insert(ino, attr);
        ino
    }

    fn create_dir(&mut self) -> u64 {
        let ino = self.create_dir_inode();
        self.dirs.insert(ino, Dir::new());
        // self.dirs.get_mut(&ino).unwrap()
        ino
    }

    fn create_file(&mut self, c: Box<dyn Content>) -> u64 {
        let node = c.inode();
        let n = node.size as u64;
        let ino = self.create_file_inode(n);
        self.content.insert(ino, c);
        let attr = self.inodes.get_mut(&ino).unwrap();
        attr.size = n;
        ino
    }

    fn load_attr(&self, ino: u64) -> Option<FileAttr> {
        let q = Query::new(
            "inode",
            vec!["*"],
            WhereCondition::new().and(WhereExpr::Equals("id".into(), ino.into())),
        );
        if let Some(d) = self.db.select::<INode>(q).first() {
            info!("load attr -> {:?}", d);
            match d.kind {
                NodeType::Dir => Some(as_dir_attr(ino)),
                NodeType::File => Some(as_file_attr(ino, d.size as u64)),
            }
        } else {
            warn!("no attribute loaded for inode {}", ino);
            None
        }
    }

    fn create_inode(&mut self, kind: NodeType) -> u64 {
        self.next_inode += 1;
        let n = INode {
            id: self.next_inode,
            kind,
            size: 0,
        };
        self.db
            .modify_from_ser(&n)
            .map_err(|x| panic!("create inode to sqlite failed with: {x}"))
            .unwrap();
        n.id
    }

    fn db_lookup(&self, parent: u64, unwrap: &str) -> Option<FileAttr> {
        let q = Query::new(
            "entry",
            vec!["dir", "name", "inode"],
            WhereCondition::new()
                .and(WhereExpr::Equals("dir".into(), parent.into()))
                .and(WhereExpr::Equals("name".into(), unwrap.into())),
        );
        if let Some(d) = self.db.select::<DirEntry>(q).first() {
            self.load_attr(d.inode)
        } else {
            None
        }
    }

    fn get_dir_entry(&mut self, parent: u64, name: &OsStr) -> DirEntry {
        let _name_str = name.to_str().unwrap();
        let q = Query::new(
            "entry",
            vec!["dir", "name", "inode"],
            WhereCondition::new()
                .and(WhereExpr::Equals("dir".into(), parent.into()))
                .and(WhereExpr::Equals(
                    "name".into(),
                    name.to_str().unwrap().into(),
                )),
        );
        if let Some(r) = self.db.select::<DirEntry>(q).first() {
            self.inodes.insert(r.inode, as_dir_attr(r.inode));
            r.clone()
        } else {
            let d = self.create_file_inode(0);
            let dir = DirEntry {
                dir: parent,
                name: name.to_str().unwrap().to_string(),
                inode: d,
            };
            self.db.modify_from_ser(&dir).unwrap();
            dir
        }
    }

    fn put_dir_entry(&mut self, dir: u64, name: String, inode: u64) {
        let dir = DirEntry { dir, name, inode };
        self.db.modify_from_ser(&dir).unwrap();
    }

    fn get_free_handle(&self) -> Option<usize> {
        let mut r = None;
        for idx in 0..self.files.len() {
            if self.files[idx].is_none() {
                r = Some(idx);
                break;
            }
        }
        r
    }
}

fn as_dir_attr(ino: u64) -> FileAttr {
    let attr = FileAttr {
        ino,
        size: 0,
        blocks: 0,
        atime: SystemTime::now(),
        mtime: SystemTime::now(),
        ctime: SystemTime::now(),
        crtime: SystemTime::now(),
        kind: FileType::Directory,
        perm: 0o755,
        nlink: 2,
        uid: 1000,
        gid: 1000,
        rdev: 0,
        flags: 0,
        blksize: 512,
    };
    attr
}

fn as_file_attr(ino: u64, size: u64) -> FileAttr {
    let attr = FileAttr {
        ino,
        size,
        blocks: 0,
        atime: SystemTime::now(),
        mtime: SystemTime::now(),
        ctime: SystemTime::now(),
        crtime: SystemTime::now(),
        kind: FileType::RegularFile,
        perm: 0o755,
        nlink: 1,
        uid: 1000,
        gid: 1000,
        rdev: 0,
        flags: 0,
        blksize: 512,
    };
    attr
}

impl Filesystem for AgentFS {
    fn getattr(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyAttr) {
        info!("getattr {ino}");
        if let Some(x) = self.inodes.get(&ino) {
            let ttl = get_ttl();
            reply.attr(&ttl, x);
            info!("returned attributes {:?}", x);
        } else {
            if let Some(attr) = self.load_attr(ino) {
                let ttl = get_ttl();
                reply.attr(&ttl, &attr);
                info!("returned attributes {:?}", &attr);
            } else {
                warn!("not found getattr(ino: {:#x?})", ino);

                reply.error(ENOENT);
                error!("not found");
            }
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
        info!("readdir {ino} offset:{}", offset);

        let q = Query::new(
            "entry",
            vec!["dir", "name", "inode"],
            WhereCondition::new().and(WhereExpr::Equals("dir".into(), ino.into())),
        );
        let mut d: Vec<DirEntry> = self.db.select(q);
        d.insert(
            0,
            DirEntry {
                dir: ino,
                name: "..".into(),
                inode: ino,
            },
        );
        d.insert(
            0,
            DirEntry {
                dir: ino,
                name: ".".into(),
                inode: ino,
            },
        );
        for idx in (offset as usize)..d.len() {
            let entry = &d[idx];
            if reply.add(
                entry.inode,
                1 + idx as i64,
                FileType::RegularFile,
                entry.name.clone(),
            ) {
                info!("break loop");
                break;
            }
        }
        reply.ok();
        // if let Some(dir) = self.dirs.get(&ino) {
        //     if offset == 0 {
        //         let _ = reply.add(ino, 1, FileType::Directory, ".");
        //         let _ = reply.add(ino, 2, FileType::Directory, "..");
        //         let mut idx = offset + 3;
        //         for (entry, node) in dir.entries.iter().skip(offset as usize) {
        //             if let Some(attr) = self.inodes.get(node) {
        //                 info!("entry {:?}", entry);
        //                 if !reply.add(*node, idx, attr.kind, entry.to_str().unwrap()) {
        //                     info!("break loop");
        //                     break;
        //                 }
        //                 idx += 1;
        //             }
        //         }
        //     }
        //     reply.ok();
        // } else {
        //     warn!("readdir(ino: {:#x?}) failed", ino);
        //     reply.error(ENOENT);
        // }
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
        info!("lookup {} {:?}", parent, name.to_str());
        // if let Some(d) = self.dirs.get(&parent) {
        //     if let Some(e) = d.entries.get(name) {
        //         if let Some(attr) = self.inodes.get(e) {
        //             reply.entry(&get_ttl(), attr, 0);
        //         } else {
        //             error!("inode {e} referred to by {name:?} was not found.");
        //             reply.error(libc::ENOENT);
        //         }
        //     } else {
        //         reply.error(libc::ENOENT);
        //     }
        // } else {
        if let Some(attr) = self.db_lookup(parent, name.to_str().unwrap()) {
            reply.entry(&get_ttl(), &attr, 0);
            debug!("-> attributes = {:?}", &attr);
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
        atime: Option<fuser::TimeOrNow>,
        mtime: Option<fuser::TimeOrNow>,
        ctime: Option<std::time::SystemTime>,
        fh: Option<u64>,
        crtime: Option<std::time::SystemTime>,
        chgtime: Option<std::time::SystemTime>,
        _bkuptime: Option<std::time::SystemTime>,
        flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        let _ = chgtime;
        let _ = crtime;
        let _ = ctime;
        let _ = mtime;
        let _ = atime;
        debug!(
            "[Not Implemented] setattr(ino: {:#x?}, mode: {:?}, uid: {:?}, \\
            gid: {:?}, size: {:?}, fh: {:?}, flags: {:?})",
            ino, mode, uid, gid, size, fh, flags
        );
        reply.attr(
            &get_ttl(),
            &FileAttr {
                ino,
                size: size.unwrap_or(10),
                blocks: 1,
                atime: SystemTime::now(),
                mtime: SystemTime::now(),
                ctime: SystemTime::now(),
                crtime: SystemTime::now(),
                kind: FileType::RegularFile,
                perm: 0x655,
                nlink: 1,
                uid: uid.unwrap_or(1000),
                gid: gid.unwrap_or(1000),
                rdev: 1,
                blksize: 512,
                flags: flags.unwrap_or(0),
            },
        );
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
        _mode: u32,
        _umask: u32,
        reply: ReplyEntry,
    ) {
        let d = self.create_dir_inode();
        let dir = DirEntry {
            dir: parent,
            name: name.to_str().unwrap().to_string(),
            inode: d,
        };
        self.next_inode += 1;
        self.db.modify_from_ser(&dir).unwrap();
        let attr = self.inodes[&d];
        let generation = 0;
        reply.entry(&get_ttl(), &attr, generation)
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

    fn open(&mut self, _req: &Request<'_>, ino: u64, _flags: i32, reply: fuser::ReplyOpen) {
        info!("open {} {:X}", ino, _flags);
        let handle = self.get_free_handle().unwrap();
        self.files[handle] = Some(Box::new(FileContent::new(
            ino,
            format!("tmp/inode-{}", ino),
        )));
        reply.opened(handle as u64, 0);
        info!("file opened with handle {}", handle);
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        info!("reading  {} bytes... offset: {}", size, offset);
        if let Some(cc) = &mut self.files[fh as usize] {
            let buf = cc.read(offset, size);
            info!("reading content returned {} bytes...", buf.len());
            reply.data(buf.as_bytes());
        } else {
            panic!("invalid file handle {}", fh);
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
        info!(
            "write(ino: {:#x?}, fh: {}, offset: {}, data.len(): {}, \\
            write_flags: {:#x?}, flags: {:#x?}, lock_owner: {:?})",
            ino,
            fh,
            offset,
            data.len(),
            write_flags,
            flags,
            lock_owner
        );
        if let Some(cc) = &mut self.files[fh as usize] {
            let mut bb = ByteBuffer::new();
            bb.write(data);
            let n = cc.write(offset, data.len() as u32, bb);
            debug!("written {} bytes", n);
            reply.written(n as u32);
        } else {
            panic!("invalid handle");
        }
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
            "flush(ino: {:#x?}, fh: {}, lock_owner: {:?})",
            ino, fh, lock_owner
        );
        if let Some(cc) = &mut self.files[fh as usize] {
            cc.flush();
            self.db.modify_from_ser(&cc.inode()).unwrap();
            reply.ok();
        } else {
            panic!("invalid handle");
        }
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
        debug!("statfs {_ino}");
        reply.statfs(100, 100, 100, 10, 10, 512, 255, 100);
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
        info!(
            "create {} Mode:{:o} umask:{:o} flags:{:b}",
            name.to_str().unwrap(),
            mode,
            umask,
            flags
        );

        // the assumption is, that this is called only when there is no previous entry.
        // so why call it here?
        let ino = self.create_inode(NodeType::File);

        let attr = self.load_attr(ino).unwrap();
        let generation = 0;
        let handle = self.get_free_handle().unwrap();

        self.files[handle] = Some(Box::new(FileContent::new(
            ino,
            format!("tmp/inode-{}", ino),
        )));
        self.put_dir_entry(parent, name.to_str().unwrap().to_string(), ino);
        reply.created(&get_ttl(), &attr, generation, handle as u64, 0);
        info!("created: {:?}", &attr);
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
