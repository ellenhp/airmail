use std::{collections::HashSet, num::NonZeroUsize, os::raw::c_void, sync::Arc, time::Duration};

use log::{debug, error, info, trace, warn};
use lru::LruCache;
use nix::sys::mman::{madvise, MmapAdvise};
use tokio::{
    runtime::Runtime,
    sync::{
        broadcast::{Receiver, Sender},
        Mutex,
    },
};
use userfaultfd::{Event, Uffd};

use crate::directory::CHUNK_SIZE;

thread_local! {
    pub(crate) static HTTP_CLIENT: reqwest::Client = reqwest::Client::new();
}

pub(crate) fn round_up_to_page(size: usize) -> usize {
    (size + CHUNK_SIZE - 1) & !(CHUNK_SIZE - 1)
}

async fn fetch_and_resume(
    mmap_base_ptr: usize,
    dst_ptr: usize,
    chunk_idx: usize,
    artifact_url: String,
    uffd: Arc<Uffd>,
    sender: Sender<usize>,
    recent_chunks: Arc<Mutex<LruCache<usize, Vec<u8>>>>,
) {
    info!("Fetching chunk: {} from {}", chunk_idx, artifact_url);
    let start_time = std::time::Instant::now();
    let byte_range = (chunk_idx * CHUNK_SIZE)..((chunk_idx + 1) * CHUNK_SIZE);
    for attempt in 0..5 {
        let response = HTTP_CLIENT
            .with(|client| {
                client
                    .get(&artifact_url)
                    .header(
                        "Range",
                        format!("bytes={}-{}", byte_range.start, byte_range.end - 1),
                    )
                    .timeout(Duration::from_millis(3000))
                    .send()
            })
            .await;
        if let Ok(response) = response {
            if response.status().is_success() {
                debug!(
                    "Success! Fetched chunk: {}-{} in {:?} and {} attempts",
                    byte_range.start,
                    byte_range.end,
                    start_time.elapsed(),
                    attempt + 1
                );
                let bytes = if let Ok(bytes) = response.bytes().await {
                    bytes.to_vec()
                } else {
                    warn!("Failed to read response bytes");
                    continue;
                };
                let expected_len = byte_range.end - byte_range.start;
                if bytes.len() > expected_len {
                    // This is weird and indicates a bug or malicious server.
                    info!(
                        "Expected {} bytes, got {}. Refusing to overflow chunk buffer.",
                        expected_len,
                        bytes.len()
                    );
                    continue;
                }
                let bytes = if bytes.len() < expected_len {
                    // We need to extend the buffer to the expected size.
                    let mut extended = vec![0; expected_len];
                    extended[..bytes.len()].copy_from_slice(&bytes);
                    extended
                } else {
                    bytes
                };
                debug_assert!(bytes.len() == expected_len);
                debug_assert!(bytes.len() == CHUNK_SIZE);

                let offset = (dst_ptr - mmap_base_ptr) % CHUNK_SIZE;
                debug_assert!(offset + 4096 <= bytes.len());
                unsafe {
                    let _ = uffd.copy(
                        bytes.as_ptr().add(offset) as *const c_void,
                        dst_ptr as *mut c_void,
                        4096,
                        true,
                    );
                    dont_need(dst_ptr as usize);
                }
                {
                    trace!("Locking recent chunks to insert new chunk");
                    if let Ok(mut recent_chunks) = recent_chunks.try_lock() {
                        recent_chunks.put(chunk_idx, bytes);
                    } else {
                        debug!("Could not lock recent chunks");
                    }
                }
                sender.send(chunk_idx).unwrap();
                return;
            }
            warn!(
                "Failed to fetch chunk: {}-{}",
                byte_range.start, byte_range.end
            );
        } else {
            warn!(
                "Failed to fetch chunk: {}-{}: {:?}",
                byte_range.start, byte_range.end, response
            );
        }
    }
    error!(
        "Critical: Failed to fetch chunk: {} after 5 attempts",
        chunk_idx,
    );
    // They'll try again I guess?
    uffd.wake(dst_ptr as *mut c_void, 4096).unwrap();
}

fn dont_need(page_start: usize) {
    // Round down to page size.
    unsafe {
        madvise(page_start as *mut c_void, 4096, MmapAdvise::MADV_WILLNEED)
            .expect("madvise failed");
    }
}

pub(crate) fn handle_uffd(uffd: Uffd, mmap_start: usize, _len: usize, artifact_url: String) {
    trace!("Creating tokio runtime");
    let rt = Runtime::new().unwrap();
    info!("Starting UFFD handler");
    let uffd = Arc::new(uffd);
    let requested_pages = Arc::new(Mutex::new(HashSet::new()));
    let chunk_cache: Arc<Mutex<LruCache<usize, Vec<u8>>>> =
        Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(8).unwrap())));
    let (sender, mut receiver): (Sender<usize>, Receiver<usize>) =
        tokio::sync::broadcast::channel(100);
    loop {
        {
            if let Ok(chunk) = receiver.try_recv() {
                trace!("Locking requested pages to remove chunk");
                requested_pages.blocking_lock().remove(&chunk);
            }
        }
        trace!("Waiting for page fault event");
        let event = uffd.read_event().unwrap();
        let event = if let Some(event) = event {
            event
        } else {
            continue;
        };

        match event {
            Event::Pagefault {
                kind,
                rw,
                addr,
                thread_id,
            } => {
                trace!("Pagefault: {:?} {:?} {:?} {:?}", kind, rw, addr, thread_id);
                let offset = addr as usize - mmap_start;
                let chunk_idx = offset / CHUNK_SIZE;
                trace!("Locking recent chunks to check for cached chunk");
                if let Some(chunk) = chunk_cache.blocking_lock().get(&chunk_idx) {
                    trace!("Using cached chunk: {}", chunk_idx);
                    let offset_into_chunk = offset % CHUNK_SIZE;
                    unsafe {
                        let _ = uffd.copy(
                            chunk.as_ptr().add(offset_into_chunk) as *const c_void,
                            addr as *mut c_void,
                            4096,
                            true,
                        );
                        dont_need(addr as usize);
                    }
                    continue;
                }

                trace!("Locking requested pages to check if chunk is already requested");
                if requested_pages.blocking_lock().contains(&chunk_idx) {
                    debug!("Already requested chunk: {}", chunk_idx);
                    let uffd = uffd.clone();
                    let requested_pages = requested_pages.clone();
                    let mut receiver = receiver.resubscribe();
                    let addr = addr as usize;
                    rt.spawn(async move {
                        let start = std::time::Instant::now();
                        loop {
                            if let Ok(chunk) = receiver.recv().await {
                                if chunk == chunk_idx {
                                    break;
                                }
                            }
                            if start.elapsed() > Duration::from_secs(10) {
                                error!("Timeout waiting for chunk: {}", chunk_idx);
                                break;
                            }
                            trace!("Locking requested pages to check if chunk is still requested");
                            if !requested_pages.lock().await.contains(&chunk_idx) {
                                warn!("Chunk: {} is no longer requested, but we missed the message that it was found.", chunk_idx);
                                break;
                            }
                        }

                        // Wake the process, and we'll handle the page fault again if need be.
                        uffd.wake(addr as *mut c_void, 4096).unwrap();
                    });
                    continue;
                }
                debug!("Requesting chunk: {}", chunk_idx);
                trace!("Locking requested pages to insert new chunk");
                if let Ok(mut lock) = requested_pages.try_lock() {
                    lock.insert(chunk_idx);
                } else {
                    debug!("Could not lock requested pages");
                }
                trace!("Spawning fetch_and_resume");
                let artifact_url = artifact_url.clone();
                let uffd = uffd.clone();
                rt.spawn(fetch_and_resume(
                    mmap_start,
                    addr as usize,
                    chunk_idx,
                    artifact_url,
                    uffd,
                    sender.clone(),
                    chunk_cache.clone(),
                ));
            }
            Event::Fork { uffd } => {
                info!("Fork: {:?}", uffd);
            }
            Event::Remap { from, to, len } => {
                info!("Remap: {:?} - {:?}, len {:?}", from, to, len);
            }
            Event::Remove { start, end } => {
                info!("Remove: {:?} - {:?}", start, end);
            }
            Event::Unmap { start, end } => {
                info!("Unmap: {:?} - {:?}, stopping UFFD handler", start, end);
                return;
            }
        }
    }
}
