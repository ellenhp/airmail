mod query_len;
mod uffd;
mod vec_writer;

use self::uffd::handle_uffd;
use crate::directory::{uffd::round_up_to_page, vec_writer::VecWriter};
use log::info;
use nix::sys::mman::{mmap, MapFlags, ProtFlags};
use std::{
    collections::HashMap,
    ops::{Deref, Range},
    path::Path,
    slice,
    sync::{Arc, Mutex},
};
use tantivy::{
    directory::{
        error::{DeleteError, OpenReadError, OpenWriteError},
        WatchHandle, WritePtr,
    },
    Directory,
};
use tantivy_common::{file_slice::FileHandle, HasLen, OwnedBytes, StableDeref};
use userfaultfd::{FeatureFlags, UffdBuilder};

thread_local! {
    pub(crate) static BLOCKING_HTTP_CLIENT: reqwest::blocking::Client = reqwest::blocking::Client::new();
}

const CHUNK_SIZE: usize = 512 * 1024;

#[derive(Clone)]
struct MmapArc {
    slice: &'static [u8],
}

impl Deref for MmapArc {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        self.slice
    }
}
unsafe impl StableDeref for MmapArc {}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CacheKey {
    base_url: String,
    path: String,
    chunk: usize,
}

#[derive(Debug, Clone)]
pub struct HttpFileHandle {
    _ptr: usize,
    owned_bytes: Arc<OwnedBytes>,
}

#[async_trait::async_trait]
impl FileHandle for HttpFileHandle {
    fn read_bytes(&self, range: Range<usize>) -> std::io::Result<OwnedBytes> {
        Ok(self.owned_bytes.slice(range))
    }
}

impl HasLen for HttpFileHandle {
    fn len(&self) -> usize {
        self.owned_bytes.len()
    }
}

#[derive(Debug, Clone)]
pub struct HttpDirectory {
    base_url: String,
    file_handle_cache: Arc<Mutex<HashMap<String, Arc<HttpFileHandle>>>>,
    atomic_read_cache: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl HttpDirectory {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            file_handle_cache: Arc::new(Mutex::new(HashMap::new())),
            atomic_read_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn format_url(&self, path: &Path) -> String {
        if self.base_url.ends_with('/') {
            format!("{}{}", self.base_url, path.display())
        } else {
            format!("{}/{}", self.base_url, path.display())
        }
    }
}

impl Directory for HttpDirectory {
    fn get_file_handle(&self, path: &Path) -> Result<Arc<dyn FileHandle>, OpenReadError> {
        let url = self.format_url(path);
        {
            let cache = self.file_handle_cache.lock().unwrap();
            if let Some(file_handle) = cache.get(&url) {
                return Ok(file_handle.clone());
            }
        }
        let file_len = query_len::len(&url);
        let len = round_up_to_page(file_len);

        if len == 0 {
            return Ok(Arc::new(HttpFileHandle {
                _ptr: 0,
                owned_bytes: Arc::new(OwnedBytes::new(MmapArc { slice: &[] })),
            }));
        }

        let uffd = UffdBuilder::new()
            .close_on_exec(true)
            .user_mode_only(true)
            .require_features(FeatureFlags::MISSING_HUGETLBFS)
            .create()
            .unwrap();

        let addr = unsafe {
            mmap(
                None,
                len.try_into().unwrap(),
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                MapFlags::MAP_PRIVATE | MapFlags::MAP_ANONYMOUS | MapFlags::MAP_NORESERVE,
                None::<std::os::fd::BorrowedFd>,
                0,
            )
            .expect("mmap")
        };

        let mmap_ptr = addr as usize;

        uffd.register(addr, len).unwrap();
        {
            let url = url.clone();
            std::thread::spawn(move || {
                handle_uffd(uffd, mmap_ptr, len, url);
            });
        }
        let owned_bytes = Arc::new(OwnedBytes::new(MmapArc {
            slice: unsafe { slice::from_raw_parts(mmap_ptr as *const u8, file_len) },
        }));

        let file_handle = Arc::new(HttpFileHandle {
            _ptr: mmap_ptr,
            owned_bytes,
        });
        {
            let mut cache = self.file_handle_cache.lock().unwrap();
            cache.insert(url, file_handle.clone());
        }

        Ok(file_handle)
    }

    fn delete(&self, path: &Path) -> Result<(), DeleteError> {
        if path == Path::new(".tantivy-meta.lock") {
            return Ok(());
        }

        Err(DeleteError::IoError {
            io_error: Arc::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Delete not supported",
            )),
            filepath: path.to_path_buf(),
        })
    }

    fn exists(&self, path: &Path) -> Result<bool, OpenReadError> {
        if path == Path::new(".tantivy-meta.lock") {
            return Ok(true);
        }
        Ok(query_len::len(&self.format_url(path)) > 0)
    }

    fn open_write(&self, path: &Path) -> Result<WritePtr, OpenWriteError> {
        if path == Path::new(".tantivy-meta.lock") {
            return Ok(WritePtr::new(Box::new(VecWriter::new(path.to_path_buf()))));
        }
        dbg!(path);
        Err(OpenWriteError::IoError {
            io_error: Arc::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Write not supported",
            )),
            filepath: path.to_path_buf(),
        })
    }

    fn atomic_read(&self, path: &Path) -> Result<Vec<u8>, OpenReadError> {
        let url = self.format_url(path);
        if let Some(bytes) = self.atomic_read_cache.lock().unwrap().get(&url) {
            return Ok(bytes.clone());
        }

        info!("Fetching {} in atomic read.", url);
        let response = BLOCKING_HTTP_CLIENT.with(|client| client.get(&url).send());
        let response = if let Err(_e) = response {
            return Err(OpenReadError::IoError {
                io_error: Arc::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Fetch failed for atomic read.",
                )),
                filepath: path.to_path_buf(),
            });
        } else {
            response.unwrap()
        };
        let bytes = response.bytes().unwrap();

        let bytes = bytes.to_vec();
        self.atomic_read_cache
            .lock()
            .unwrap()
            .insert(url, bytes.clone());
        Ok(bytes)
    }

    fn atomic_write(&self, _path: &Path, _data: &[u8]) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Write not supported",
        ))
    }

    fn sync_directory(&self) -> std::io::Result<()> {
        Ok(())
    }

    fn watch(
        &self,
        _watch_callback: tantivy::directory::WatchCallback,
    ) -> tantivy::Result<tantivy::directory::WatchHandle> {
        Ok(WatchHandle::empty())
    }
}
