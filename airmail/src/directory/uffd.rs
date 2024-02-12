use std::{collections::HashSet, ops::Add, os::raw::c_void, sync::Arc, time::Duration};

use crossbeam::channel::{Receiver, Sender};
use log::{error, info, warn};
use tokio::{spawn, time::sleep};
use userfaultfd::{Event, Uffd};

use crate::directory::CHUNK_SIZE;

thread_local! {
    pub(crate) static HTTP_CLIENT: reqwest::Client = reqwest::Client::new();
}

pub(crate) fn round_up_to_page(size: usize) -> usize {
    (size + CHUNK_SIZE - 1) & !(CHUNK_SIZE - 1)
}

async fn fetch_and_resume(
    base_ptr: usize,
    chunk_idx: usize,
    artifact_url: String,
    uffd: Arc<Uffd>,
    sender: Sender<usize>,
) {
    info!("Fetching chunk: {} from {}", chunk_idx, artifact_url);
    let byte_range = (chunk_idx * CHUNK_SIZE)..((chunk_idx + 1) * CHUNK_SIZE);
    for _ in 0..5 {
        let response = HTTP_CLIENT
            .with(|client| {
                client
                    .get(&artifact_url)
                    .header(
                        "Range",
                        format!("bytes={}-{}", byte_range.start, byte_range.end - 1),
                    )
                    .send()
            })
            .await;
        if let Ok(response) = response {
            if response.status().is_success() {
                info!(
                    "Success! Fetched chunk: {}-{}",
                    byte_range.start, byte_range.end
                );
                let bytes = response.bytes().await.unwrap().to_vec();
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
                info!("Copying chunk to memory");
                unsafe {
                    let src = bytes.as_ptr() as *const c_void;
                    let dst = base_ptr.add(chunk_idx * CHUNK_SIZE) as *mut c_void;
                    uffd.copy(src, dst, CHUNK_SIZE, true).unwrap();
                }
                sender.send(chunk_idx).unwrap();
                return;
            }
            warn!(
                "Failed to fetch chunk: {}-{}",
                byte_range.start, byte_range.end
            );
        }
    }
    error!(
        "Critical: Failed to fetch chunk: {} after 5 attempts",
        chunk_idx,
    );
}

pub(crate) async fn handle_uffd(uffd: Uffd, mmap_start: usize, _len: usize, artifact_url: String) {
    info!("Starting UFFD handler");
    let uffd = Arc::new(uffd);
    let mut requested_pages = HashSet::new();
    let (sender, receiver): (Sender<usize>, Receiver<usize>) = crossbeam::channel::unbounded();
    loop {
        {
            if let Ok(chunk) = receiver.try_recv() {
                requested_pages.remove(&chunk);
            }
        }
        sleep(Duration::from_micros(100)).await;
        let event = uffd.read_event().unwrap();
        let event = if let Some(event) = event {
            event
        } else {
            continue;
        };
        info!("UFFD event: {:?}", event);

        match event {
            Event::Pagefault {
                kind,
                rw,
                addr,
                thread_id,
            } => {
                info!("Pagefault: {:?} {:?} {:?} {:?}", kind, rw, addr, thread_id);
                let offset = addr as usize - mmap_start;
                let chunk_idx = offset / CHUNK_SIZE;
                if requested_pages.contains(&chunk_idx) {
                    info!("Already requested chunk: {}", chunk_idx);
                    let receiver = receiver.clone();
                    let uffd = uffd.clone();
                    spawn(async move {
                        let start = std::time::Instant::now();
                        loop {
                            if start.elapsed() > Duration::from_secs(5) {
                                error!("Timeout waiting for chunk: {}", chunk_idx);
                                break;
                            }
                            if let Ok(fetched_chunk) = receiver.recv_timeout(Duration::from_secs(1))
                            {
                                if fetched_chunk == chunk_idx {
                                    break;
                                }
                            }
                        }
                        uffd.wake(offset as *mut c_void, CHUNK_SIZE).unwrap();
                    });
                    continue;
                } else {
                    info!("Requesting chunk: {}", chunk_idx);
                    requested_pages.insert(chunk_idx);
                }
                let artifact_url = artifact_url.clone();
                let uffd = uffd.clone();
                let sender = sender.clone();
                spawn(fetch_and_resume(
                    mmap_start,
                    chunk_idx,
                    artifact_url,
                    uffd,
                    sender,
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
