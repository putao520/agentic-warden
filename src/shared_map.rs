use raw_sync::locks::{LockImpl, LockInit, Mutex};
use shared_hashmap::{SharedMemoryContents, SharedMemoryHashMap};
use shared_memory::{Shmem, ShmemConf, ShmemError};
use std::marker::PhantomData;
use thiserror::Error;

#[repr(C)]
struct SharedContents<K, V> {
    bucket_count: usize,
    used: usize,
    phantom: PhantomData<(K, V)>,
    size: usize,
}

#[derive(Debug, Error)]
pub enum SharedMapError {
    #[error("shared memory region too small for task registry")]
    RegionTooSmall,
    #[error("shared memory error: {0}")]
    Shmem(#[from] ShmemError),
    #[error("shared lock init failed: {0}")]
    LockInit(String),
    #[error("shared lock access failed: {0}")]
    LockGuard(String),
}

#[repr(C)]
struct SharedMapRepr<K, V> {
    shm: Shmem,
    lock: Box<dyn LockImpl>,
    phantom: PhantomData<(K, V)>,
}

pub fn open_or_create(
    namespace: &str,
    size: usize,
) -> Result<SharedMemoryHashMap<String, String>, SharedMapError> {
    match open_existing(namespace, size) {
        Ok(map) => Ok(map),
        Err(SharedMapError::Shmem(ShmemError::MapOpenFailed(_)))
        | Err(SharedMapError::Shmem(ShmemError::LinkDoesNotExist))
        | Err(SharedMapError::Shmem(ShmemError::NoLinkOrOsId)) => create_or_retry(namespace, size),
        Err(e) => Err(e),
    }
}

fn open_existing(
    namespace: &str,
    size: usize,
) -> Result<SharedMemoryHashMap<String, String>, SharedMapError> {
    let conf = ShmemConf::new().os_id(namespace).size(size);
    let shm = conf.open()?;
    map_from_shmem(shm, false)
}

fn create_or_retry(
    namespace: &str,
    size: usize,
) -> Result<SharedMemoryHashMap<String, String>, SharedMapError> {
    let conf = ShmemConf::new().os_id(namespace).size(size);
    match conf.create() {
        Ok(mut shm) => {
            // ensure the mapping survives after the creator exits
            let _ = shm.set_owner(false);
            map_from_shmem(shm, true)
        }
        Err(ShmemError::MappingIdExists) => open_existing(namespace, size),
        Err(e) => Err(SharedMapError::from(e)),
    }
}

fn map_from_shmem(
    shm: Shmem,
    init: bool,
) -> Result<SharedMemoryHashMap<String, String>, SharedMapError> {
    let ptr = shm.as_ptr();
    let total_len = shm.len();
    let lock_region = Mutex::size_of(Some(ptr));

    // Ensure 8-byte alignment for the data region
    let aligned_lock_region = (lock_region + 7) & !7;

    if total_len < aligned_lock_region + std::mem::size_of::<SharedContents<String, String>>() {
        return Err(SharedMapError::RegionTooSmall);
    }

    let data_size = total_len - aligned_lock_region;
    let lock_ptr = unsafe { ptr.add(aligned_lock_region) };

    let lock_impl = if init {
        unsafe {
            Mutex::new(ptr, lock_ptr)
                .map_err(|e| SharedMapError::LockInit(e.to_string()))?
                .0
        }
    } else {
        unsafe {
            Mutex::from_existing(ptr, lock_ptr)
                .map_err(|e| SharedMapError::LockInit(e.to_string()))?
                .0
        }
    };

    let repr = SharedMapRepr::<String, String> {
        shm,
        lock: lock_impl,
        phantom: PhantomData,
    };

    // SAFETY: SharedMapRepr mirrors the memory layout of SharedMemoryHashMap in the dependency crate.
    let map: SharedMemoryHashMap<String, String> = unsafe { std::mem::transmute(repr) };

    if init {
        let contents = SharedContents::<String, String> {
            bucket_count: 0,
            used: 0,
            phantom: PhantomData,
            size: data_size,
        };
        let guard = map
            .lock()
            .map_err(|e| SharedMapError::LockGuard(e.to_string()))?;
        unsafe {
            let target = *guard as *mut SharedMemoryContents<String, String>
                as *mut SharedContents<String, String>;
            core::ptr::write(target, contents);
        }
    }

    Ok(map)
}
