use std::{
    ffi::OsString,
    fs::{self, OpenOptions},
    io::{self, Read, Seek, Write},
    path::Path,
};

use anyhow::anyhow;
use either::Either::{Left, Right};

use crate::{
    error_reporter::{StreamName, StreamNameRef},
    utils::UtilExt as _,
    Result,
};

pub(crate) trait IoHelper: Clone + Send + Sync {
    fn open_read_path(&self, path: &Path) -> Result<impl Read + Seek + '_>;
    fn open_write_path(&self, path: &Path) -> Result<impl Write + Seek + '_>;
    fn read<'path>(&self, name: impl Into<StreamNameRef<'path>>) -> Result<String> {
        io::read_to_string(self.open_read(name)?).err_into()
    }
    fn write<'path>(&self, name: impl Into<StreamNameRef<'path>>, value: &str) -> Result {
        let name = name.into();

        if let StreamName::File(name) = name {
            if let Some(parent) = name.parent() {
                self.create_dir_all(parent)?;
            }
        }

        self.open_write(name)?
            .write_all(value.as_bytes())
            .err_into()
    }
    fn stdin(&self) -> Result<impl Read + '_ + Send>;
    fn stdout(&self) -> Result<impl Write + '_ + Send>;
    fn stderr(&self) -> Result<impl Write + '_ + Send>;
    fn open_read<'path>(&self, name: impl Into<StreamNameRef<'path>>) -> Result<impl Read + '_> {
        match name.into() {
            StreamName::File(path) => self.open_read_path(path).map(Left),
            StreamName::Stdio => self.stdin().map(Right),
        }
    }
    fn open_write<'path>(&self, name: impl Into<StreamNameRef<'path>>) -> Result<impl Write + '_> {
        match name.into() {
            StreamName::File(path) => self.open_write_path(path).map(Left),
            StreamName::Stdio => self.stdout().map(Right),
        }
    }

    fn path_info(&self, path: &Path) -> Result<Option<PathInfo>>;

    fn read_dir(&self, path: &Path) -> Result<impl Iterator<Item = Result<(OsString, PathInfo)>>>;

    fn is_file(&self, path: &Path) -> Result<bool> {
        self.path_info(path)?
            .is_some_and(|info| info.is_file())
            .wrap_ok()
    }

    fn is_dir(&self, path: &Path) -> Result<bool> {
        self.path_info(path)?
            .is_some_and(|info| info.is_dir())
            .wrap_ok()
    }

    fn create_dir_all(&self, path: &Path) -> Result;
}

#[derive(Clone, Copy)]
pub(crate) struct DefaultIoHelper;

impl IoHelper for DefaultIoHelper {
    fn open_read_path(&self, path: &Path) -> Result<impl Read + Seek + '_> {
        std::fs::File::open(path).err_into()
    }

    fn open_write_path(&self, path: &Path) -> Result<impl Write + Seek + '_> {
        let f = OpenOptions::new().create(true).write(true).open(path)?;
        f.set_len(0)?;
        Ok(f)
    }

    fn stdin(&self) -> Result<impl Read> {
        std::io::stdin().wrap_ok()
    }

    fn stdout(&self) -> Result<impl Write> {
        std::io::stdout().wrap_ok()
    }

    fn stderr(&self) -> Result<impl Write> {
        std::io::stderr().wrap_ok()
    }

    fn path_info(&self, path: &Path) -> Result<Option<PathInfo>> {
        let result = path.metadata();

        let md = match result {
            Ok(md) => md,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        PathInfo {
            is_file: md.is_file(),
            is_dir: md.is_dir(),
        }
        .wrap_some()
        .wrap_ok()
    }

    fn read_dir(&self, path: &Path) -> Result<impl Iterator<Item = Result<(OsString, PathInfo)>>> {
        fs::read_dir(path)
            .map_err(|e| anyhow!("read_dir_failed for {path:?}: {e}").context(e))?
            .map(|entry| {
                let entry = entry?;
                let md = entry.metadata()?;
                (
                    entry.file_name(),
                    PathInfo {
                        is_file: md.is_file(),
                        is_dir: md.is_dir(),
                    },
                )
                    .wrap_ok()
            })
            .wrap_ok()
    }

    fn create_dir_all(&self, path: &Path) -> Result {
        fs::create_dir_all(path).err_into()
    }
}

#[derive(Default)]
pub struct PathInfo {
    is_file: bool,
    is_dir: bool,
}

impl PathInfo {
    pub fn is_file(&self) -> bool {
        self.is_file
    }

    pub fn is_dir(&self) -> bool {
        self.is_dir
    }
}

#[cfg(test)]
pub mod test_helper {
    use std::{
        borrow::Cow,
        collections::BTreeMap,
        ffi::{OsStr, OsString},
        io::{self, prelude::*},
        num::NonZeroUsize,
        ops::ControlFlow::{self, Break, Continue},
        path::{Path, PathBuf},
        sync::{Arc, Mutex, MutexGuard, PoisonError},
        vec,
    };

    use anyhow::{anyhow, bail, ensure};

    use crate::{
        utils::{default, PathExt, UtilExt},
        Result,
    };

    use super::{IoHelper, PathInfo};

    fn lock<T: ?Sized>(target: &Mutex<T>) -> MutexGuard<T> {
        target.lock().unwrap_or_else(PoisonError::into_inner)
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct EntryId(NonZeroUsize);

    #[derive(Clone)]
    pub enum TestFileEntry {
        File(SharedBuffer),
        Dir(TestDir),
    }

    impl From<SharedBuffer> for TestFileEntry {
        fn from(v: SharedBuffer) -> Self {
            Self::File(v)
        }
    }

    impl TestFileEntry {
        fn info(&self) -> PathInfo {
            match self {
                TestFileEntry::File(_) => PathInfo {
                    is_file: true,
                    is_dir: false,
                },
                TestFileEntry::Dir(_) => PathInfo {
                    is_file: false,
                    is_dir: true,
                },
            }
        }
        fn id(&self) -> EntryId {
            match self {
                TestFileEntry::File(f) => f.id(),
                TestFileEntry::Dir(d) => d.id(),
            }
        }

        fn as_file(&self) -> Option<&SharedBuffer> {
            if let Self::File(v) = self {
                Some(v)
            } else {
                None
            }
        }

        fn is_dir(&self) -> bool {
            matches!(self, Self::Dir(_))
        }

        fn as_dir(&self) -> Option<&TestDir> {
            if let Self::Dir(v) = self {
                Some(v)
            } else {
                None
            }
        }
    }

    pub struct TestIoHelperInner {
        root: TestDir,
        cwd: PathBuf,
        stdin: SharedBuffer,
        stdout: SharedBuffer,
        stderr: SharedBuffer,
    }

    #[derive(Clone)]
    pub struct TestIoHelper {
        inner: Arc<TestIoHelperInner>,
    }

    impl TestIoHelper {
        pub fn new(root: TestDir, cwd: PathBuf, stdin: &str) -> Self {
            Self {
                inner: Arc::new(TestIoHelperInner {
                    root,
                    cwd,
                    stdin: SharedBuffer::new(stdin.as_bytes()),
                    stdout: default(),
                    stderr: default(),
                }),
            }
        }

        pub fn stdout(&self) -> SharedBuffer {
            self.inner.stdout.clone()
        }

        pub fn stderr(&self) -> SharedBuffer {
            self.inner.stderr.clone()
        }
    }

    #[derive(Default, Clone)]
    pub struct TestDir {
        files: Arc<Mutex<BTreeMap<OsString, TestFileEntry>>>,
    }

    impl<K: Into<OsString>, V: Into<TestFileEntry>> FromIterator<(K, V)> for TestDir {
        fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
            Self {
                files: Arc::new(Mutex::new(
                    iter.into_iter()
                        .map(|(k, v)| (k.into(), v.into()))
                        .collect(),
                )),
            }
        }
    }

    impl<T, const N: usize> From<[T; N]> for TestFileEntry
    where
        TestDir: FromIterator<T>,
    {
        fn from(value: [T; N]) -> Self {
            TestFileEntry::Dir(TestDir::from_iter(value))
        }
    }

    impl From<TestDir> for TestFileEntry {
        fn from(value: TestDir) -> Self {
            TestFileEntry::Dir(value)
        }
    }

    impl From<&str> for TestFileEntry {
        fn from(value: &str) -> Self {
            TestFileEntry::File(SharedBuffer::new(value.as_bytes()))
        }
    }

    fn split_path_start(path: &Path) -> Option<(&OsStr, &Path)> {
        let mut iter = path.components();
        let first = iter.find_map(|c| match c {
            std::path::Component::Normal(first) => Some(first),
            std::path::Component::ParentDir => panic!(),
            _ => None,
        });
        Some((first?, iter.as_path()))
    }

    fn visit_file_list<'path, D, V, B>(
        mut data: D,
        src: vec::Drain<(&'path Path, Option<V>)>,
        mut on_dir: impl FnMut(
            &mut D,
            &'path OsStr,
            vec::Drain<(&'path Path, Option<V>)>,
        ) -> ControlFlow<B>,
        on_file: impl FnOnce(D, V) -> B,
    ) -> ControlFlow<B> {
        let mut group_name = None::<&'path OsStr>;
        let mut group = Vec::<(&'path Path, Option<V>)>::new();

        for (path, value) in src {
            let Some((first, rest)) = split_path_start(path) else {
                if let Some(value) = value {
                    return Break(on_file(data, value));
                } else {
                    continue;
                }
            };

            if let Some(name) = group_name.replace(first).filter(|&name| name != first) {
                on_dir(&mut data, name, group.drain(..))?
            }

            group.push((rest, value));
        }

        if let Some(name) = group_name {
            on_dir(&mut data, name, group.drain(..))?;
        }
        ControlFlow::Continue(())
    }

    fn visit_file_list_outer<'path, K: ?Sized + AsRef<Path> + 'path, V, R>(
        src: impl IntoIterator<Item = (&'path K, Option<V>)>,
        block: impl FnOnce(vec::Drain<(&'path Path, Option<V>)>) -> R,
    ) -> R {
        let mut src = src
            .into_iter()
            .map(|(k, v)| (k.as_ref(), v))
            .collect::<Vec<_>>();
        src.sort_unstable_by_key(|(k, _)| split_path_start(k));
        block(src.drain(..))
    }

    fn from_file_list_inner<'path, V: Into<SharedBuffer>>(
        src: vec::Drain<(&'path Path, Option<V>)>,
    ) -> TestFileEntry {
        let mut files = BTreeMap::new();
        match visit_file_list(
            (),
            src,
            |(), name, children| {
                files.insert(name.into(), from_file_list_inner(children));
                Continue(())
            },
            |(), buf| buf.into(),
        ) {
            Continue(()) => TestFileEntry::Dir(TestDir {
                files: Arc::new(Mutex::new(files)),
            }),
            Break(buf) => TestFileEntry::File(buf),
        }
    }

    fn compare_file_list_inner<'path>(
        path: &mut PathBuf,
        lhs: TestFileEntry,
        rhs: vec::Drain<(
            &'path Path,
            Option<Box<dyn FnMut(&Path, &[u8]) -> Result + 'path>>,
        )>,
    ) -> Result {
        let (mut files, buf) = match lhs {
            TestFileEntry::File(buf) => (default(), Some(buf)),
            TestFileEntry::Dir(dir) => match Arc::<Mutex<_>>::try_unwrap(dir.files) {
                Ok(mx) => (
                    mx.into_inner().unwrap_or_else(PoisonError::into_inner),
                    None,
                ),
                Err(ref files) => (lock(files).clone(), None),
            },
        };

        match visit_file_list(
            &mut *path,
            rhs,
            |path, name, children| {
                path.push(name);
                let Some(lhs) = files.remove(name) else {
                    return Break(Err(anyhow!("{path:?}: not found")));
                };

                let result = compare_file_list_inner(path, lhs, children);
                path.pop();
                match result {
                    Ok(()) => Continue(()),
                    r @ Err(_) => Break(r),
                }
            },
            |path, mut check| match buf {
                Some(buf) => check(path, &lock(&buf.data)),
                None => bail!("{path:?}: not a file"),
            },
        ) {
            Continue(()) => {}
            Break(res) => return res,
        }

        if !files.is_empty() {
            bail!("{path:?}: unexpected children {:?}", files.keys())
        }

        Ok(())
    }

    pub fn contains<'p>(
        expected: impl AsRef<str> + 'p,
    ) -> Option<Box<dyn FnMut(&Path, &[u8]) -> Result + 'p>> {
        Some(Box::new(move |path, buf| {
            {
                {
                    let expected = expected.as_ref();
                    if expected.as_bytes() == buf {
                        Ok(())
                    } else {
                        Err(anyhow!(
                            "{path:?}: Expected: {expected:?}\n\tFound: {:?}",
                            &*String::from_utf8_lossy(buf),
                        ))
                    }
                }
            }
            .into()
        }))
    }

    pub fn any() -> Option<Box<dyn FnMut(&Path, &[u8]) -> Result>> {
        Some(Box::new(|&_, &_| Ok(())))
    }

    impl TestDir {
        fn id(&self) -> EntryId {
            EntryId(NonZeroUsize::new(Arc::as_ptr(&self.files) as usize).unwrap())
        }

        pub fn from_file_list<'path, K: ?Sized + 'path + AsRef<Path>, V: Into<SharedBuffer>>(
            src: impl IntoIterator<Item = (&'path K, Option<V>)>,
        ) -> Self {
            visit_file_list_outer(src, |src| match from_file_list_inner(src) {
                TestFileEntry::File(_) => panic!(),
                TestFileEntry::Dir(dir) => dir,
            })
        }

        pub fn compare_file_list<'path, K: ?Sized + 'path + AsRef<Path>>(
            self,
            rhs: impl IntoIterator<
                Item = (
                    &'path K,
                    Option<Box<dyn FnMut(&Path, &[u8]) -> Result + 'path>>,
                ),
            >,
        ) -> Result {
            visit_file_list_outer(rhs, |rhs| {
                compare_file_list_inner(&mut PathBuf::new(), self.into(), rhs)
            })
        }

        fn with_files<R>(
            &self,
            block: impl FnOnce(&mut BTreeMap<OsString, TestFileEntry>) -> R,
        ) -> R {
            block(&mut *lock(&self.files))
        }

        fn get_with<'p, D, R>(
            &self,
            mut data: D,
            path: impl AsRef<Path>,
            mut missing_parent: impl FnMut(&mut D) -> ControlFlow<R>,
            block: impl FnOnce(D, &mut Option<TestFileEntry>) -> R,
        ) -> Result<R> {
            let path = path.as_ref();
            let mut dir = self.clone();
            let mut components = path.components().peekable();

            while let Some(comp) = components.next() {
                use std::path::Component::*;
                match comp {
                    Prefix(_) | RootDir | CurDir if components.peek().is_none() => {
                        let old_id = dir.id();
                        let mut entry = Some(dir.into());
                        let out = block(data, &mut entry);
                        let new_id = entry.as_ref().map(TestFileEntry::id);
                        ensure!(new_id == Some(old_id), "Unsupported: modfying {path:?}");
                        return Ok(out);
                    }
                    Prefix(_) | RootDir | CurDir => {}

                    ParentDir => bail!("ParentDir not supported"),

                    Normal(name) if components.peek().is_none() => {
                        let mut entry = dir.with_files(|f| f.get(name).cloned());
                        if path.has_trailing_slash()
                            && entry.as_ref().is_some_and(TestFileEntry::is_dir)
                        {
                            bail!("Not a directory: {path:?}")
                        }

                        let old_id = entry.as_ref().map(TestFileEntry::id);
                        let out = block(data, &mut entry);
                        if entry.as_ref().map(TestFileEntry::id) != old_id {
                            match entry {
                                Some(e) => {
                                    dir.with_files(|f| f.insert(name.into(), e));
                                }
                                None => {
                                    dir.with_files(|f| f.remove(name));
                                }
                            }
                        }
                        return Ok(out);
                    }
                    Normal(name) => match dir.with_files(|f| f.get(name).cloned()) {
                        Some(TestFileEntry::File(_)) => bail!("Not a directory: {path:?}"),
                        Some(TestFileEntry::Dir(next_dir)) => dir = next_dir,
                        None => match missing_parent(&mut data) {
                            Continue(()) => {
                                let new_dir = TestDir::default();
                                dir.with_files(|f| f.insert(name.into(), new_dir.clone().into()));
                                dir = new_dir;
                            }
                            Break(out) => return Ok(out),
                        },
                    },
                }
            }

            bail!("invalid path {path:?}");
        }
    }

    #[derive(Default, Clone)]
    pub struct SharedBuffer {
        data: Arc<Mutex<Vec<u8>>>,
        position: usize,
    }

    impl SharedBuffer {
        fn id(&self) -> EntryId {
            EntryId(NonZeroUsize::new(Arc::as_ptr(&self.data) as usize).unwrap())
        }

        fn new<'b>(bytes: impl Into<Cow<'b, [u8]>>) -> Self {
            Self {
                data: Arc::new(Mutex::new(bytes.into().into_owned())),
                position: 0,
            }
        }

        fn lock(&mut self) -> (MutexGuard<Vec<u8>>, &mut usize) {
            (lock(&self.data), &mut self.position)
        }

        pub fn into_inner(self) -> Vec<u8> {
            match Arc::try_unwrap(self.data) {
                Ok(mx) => mx.into_inner().unwrap_or_else(PoisonError::into_inner),
                Err(ref data) => lock(data).clone(),
            }
        }
    }

    impl From<&[u8]> for SharedBuffer {
        fn from(value: &[u8]) -> Self {
            Self::new(value)
        }
    }

    impl From<Vec<u8>> for SharedBuffer {
        fn from(value: Vec<u8>) -> Self {
            Self::new(value)
        }
    }

    impl From<&str> for SharedBuffer {
        fn from(value: &str) -> Self {
            Self::new(value.as_bytes())
        }
    }

    impl From<String> for SharedBuffer {
        fn from(value: String) -> Self {
            Self::new(value.into_bytes())
        }
    }

    impl Read for SharedBuffer {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let (data, position) = self.lock();
            let data = &data[*position..];
            let len = buf.len().min(data.len());
            buf[..len].copy_from_slice(&data[..len]);
            *position += len;
            Ok(len)
        }
    }

    impl Write for SharedBuffer {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let (ref mut data, position) = self.lock();

            let end = *position + buf.len();

            if end > data.len() {
                data.resize(end, 0);
            }

            data[*position..end].copy_from_slice(buf);
            *position = end;
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Seek for SharedBuffer {
        fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
            self.position = match pos {
                io::SeekFrom::Start(p) => Some(p),
                io::SeekFrom::End(dif) => {
                    let end = lock(&self.data).len();
                    (end as u64).checked_add_signed(dif)
                }
                io::SeekFrom::Current(dif) => (self.position as u64).checked_add_signed(dif),
            }
            .ok_or(io::ErrorKind::InvalidInput)? as usize;

            Ok(self.position as u64)
        }
    }

    impl TestIoHelperInner {
        fn join_path<'p>(&self, path: &'p (impl AsRef<Path> + ?Sized)) -> Cow<'p, Path> {
            let path: &'p Path = path.as_ref();
            if path.is_relative() {
                self.cwd.join(path).into()
            } else {
                path.into()
            }
        }
    }

    impl IoHelper for TestIoHelper {
        fn open_read_path(&self, path: &Path) -> Result<impl Read + Seek + '_> {
            let path = &self.inner.join_path(path);
            self.inner.root.get_with(
                (),
                path,
                |()| Break(Err(anyhow!("open_read_path: parent not found: {path:?}"))),
                |(), entry| {
                    entry
                        .as_ref()
                        .ok_or_else(|| anyhow!("open_read_path: not found: {path:?}"))?
                        .as_file()
                        .cloned()
                        .ok_or_else(|| anyhow!("open_read_path: not a file: {path:?}"))
                },
            )?
        }

        fn open_write_path(&self, path: &Path) -> Result<impl Write + Seek + '_> {
            let path = &self.inner.join_path(path);
            self.inner.root.get_with(
                (),
                path,
                |()| Break(Err(anyhow!("open_write_path: parent not found: {path:?}"))),
                |(), entry| match entry {
                    Some(TestFileEntry::File(buf)) => buf.clone().wrap_ok(),
                    Some(TestFileEntry::Dir(_)) => bail!("open_write_path: not a file: {path:?}"),
                    None => {
                        let out = SharedBuffer::default();
                        *entry = Some(out.clone().into());
                        Ok(out)
                    }
                },
            )?
        }

        fn stdin(&self) -> Result<impl Read + Send + '_> {
            self.inner.stdin.clone().wrap_ok()
        }

        fn stdout(&self) -> Result<impl Write + Send + '_> {
            self.inner.stdout.clone().wrap_ok()
        }

        fn stderr(&self) -> Result<impl Write + Send + '_> {
            self.inner.stderr.clone().wrap_ok()
        }

        fn path_info(&self, path: &Path) -> Result<Option<PathInfo>> {
            let path = &self.inner.join_path(path);
            self.inner
                .root
                .get_with((), path, |()| Break(None), |(), entry| entry.clone())?
                .map(|e| e.info())
                .wrap_ok()
        }

        fn read_dir(
            &self,
            path: &Path,
        ) -> Result<impl Iterator<Item = crate::Result<(OsString, PathInfo)>>> {
            let path = &self.inner.join_path(path);
            let dir = self
                .inner
                .root
                .get_with(
                    (),
                    path,
                    |()| Break(Ok(None)),
                    |(), entry| {
                        entry
                            .as_ref()
                            .map(|e| {
                                e.as_dir()
                                    .cloned()
                                    .ok_or_else(|| anyhow!("not a directory: {path:?}"))
                            })
                            .transpose()
                    },
                )??
                .ok_or_else(|| anyhow!("read_dir: not found: {path:?}"))?;

            dir.with_files(|f| {
                f.iter()
                    .map(|(p, e)| (p.to_os_string(), e.info()).wrap_ok())
                    .collect::<Vec<_>>()
                    .into_iter()
            })
            .wrap_ok()
        }

        fn create_dir_all(&self, path: &Path) -> Result {
            let path = &self.inner.join_path(path);
            self.inner.root.get_with(
                (),
                path,
                |()| Continue(()),
                |(), entry| {
                    match entry {
                        Some(TestFileEntry::File(_)) => bail!("not a directory: {path:?}"),
                        Some(TestFileEntry::Dir(_)) => (),
                        None => *entry = Some(TestFileEntry::Dir(default())),
                    }
                    .wrap_ok()
                },
            )?
        }
    }
}
