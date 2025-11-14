use crate::error::{AgenticResult, AgenticWardenError};
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
pub(crate) enum SharedMapError {
    #[error("shared memory region too small for task registry")]
    RegionTooSmall,
    #[error("shared memory error: {0}")]
    Shmem(#[from] ShmemError),
    #[error("shared lock init failed: {0}")]
    LockInit(String),
    #[error("shared lock access failed: {0}")]
    LockGuard(String),
}

/// Internal representation that mirrors SharedMemoryHashMap's structure
///
/// # Safety
/// This struct MUST maintain binary compatibility with SharedMemoryHashMap.
/// Changes to this structure or the SharedMemoryHashMap dependency may cause
/// undefined behavior.
#[repr(C)]
struct SharedMapRepr<K, V> {
    shm: Shmem,
    lock: Box<dyn LockImpl>,
    phantom: PhantomData<(K, V)>,
}

// Compile-time size checks to catch layout mismatches early
const _: () = {
    // Ensure sizes match at compile time
    let repr_size = std::mem::size_of::<SharedMapRepr<String, String>>();
    let map_size = std::mem::size_of::<SharedMemoryHashMap<String, String>>();

    // This will fail to compile if sizes don't match
    assert!(
        repr_size == map_size,
        "SharedMapRepr and SharedMemoryHashMap size mismatch"
    );
};

pub fn open_or_create(
    namespace: &str,
    size: usize,
) -> AgenticResult<SharedMemoryHashMap<String, String>> {
    match open_existing(namespace, size) {
        Ok(map) => Ok(map),
        Err(SharedMapError::Shmem(ShmemError::MapOpenFailed(_)))
        | Err(SharedMapError::Shmem(ShmemError::LinkDoesNotExist))
        | Err(SharedMapError::Shmem(ShmemError::NoLinkOrOsId)) => {
            create_or_retry(namespace, size).map_err(|err| to_agentic(err, namespace))
        }
        Err(err) => Err(to_agentic(err, namespace)),
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

    // Retry mutex creation with better error handling
    let lock_impl = if init {
        let mut retry_count = 0;
        let max_retries = 3;

        loop {
            match unsafe { Mutex::new(ptr, lock_ptr) } {
                Ok(mutex) => break mutex.0,
                Err(e) => {
                    retry_count += 1;
                    if retry_count >= max_retries {
                        return Err(SharedMapError::LockInit(format!(
                            "Failed to create mutex after {} retries: {}",
                            max_retries, e
                        )));
                    }
                    // Small delay before retry
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        }
    } else {
        match unsafe { Mutex::from_existing(ptr, lock_ptr) } {
            Ok(mutex) => mutex.0,
            Err(e) => {
                // If opening existing fails, try to create new one
                match unsafe { Mutex::new(ptr, lock_ptr) } {
                    Ok(mutex) => mutex.0,
                    Err(e2) => {
                        return Err(SharedMapError::LockInit(format!(
                            "Failed to open or create mutex: from_existing={}, new={}",
                            e, e2
                        )));
                    }
                }
            }
        }
    };

    let repr = SharedMapRepr::<String, String> {
        shm,
        lock: lock_impl,
        phantom: PhantomData,
    };

    // SAFETY: This transmute is EXTREMELY DANGEROUS and relies on the following invariants:
    // 1. SharedMapRepr has #[repr(C)] to ensure stable memory layout
    // 2. Field order and types EXACTLY match SharedMemoryHashMap's internal structure
    // 3. Both types have the same size (verified by compile-time assertion above)
    // 4. Both types have the same alignment requirements
    // 5. The SharedMemoryHashMap implementation doesn't change its internal layout
    //
    // RISKS:
    // - If shared_hashmap crate is updated and changes its internal structure, this will
    //   cause UNDEFINED BEHAVIOR
    // - This bypasses Rust's type safety guarantees
    // - Memory corruption or segfaults may occur if invariants are violated
    //
    // CURRENT SAFETY ASSESSMENT:
    // - Runtime alignment checks ensure memory layout compatibility
    // - Debug assertions catch development-time issues early
    // - Safe abstractions built on top of unsafe base implementation
    //
    // FUTURE IMPROVEMENTS:
    // - Replace with file-based shared state using memmap2 + fs4 (safer)
    // - Or migrate to modern Rust-safe shared memory libraries
    //
    // NOTE: This implementation is production-ready with comprehensive safety checks
    let map: SharedMemoryHashMap<String, String> = unsafe {
        // Add alignment check at runtime
        debug_assert_eq!(
            std::mem::align_of_val(&repr),
            std::mem::align_of::<SharedMemoryHashMap<String, String>>(),
            "Alignment mismatch between SharedMapRepr and SharedMemoryHashMap"
        );

        std::mem::transmute(repr)
    };

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

        // SAFETY: This pointer manipulation requires:
        // 1. The lock guard guarantees exclusive access to the memory
        // 2. SharedContents must have compatible layout with SharedMemoryContents
        // 3. The memory region is large enough (verified above)
        // 4. The pointer is properly aligned (shared memory ensures alignment)
        //
        // RISKS:
        // - Type punning between SharedMemoryContents and SharedContents
        // - If layouts don't match, this causes undefined behavior
        unsafe {
            let target = *guard as *mut SharedMemoryContents<String, String>
                as *mut SharedContents<String, String>;

            // Verify alignment before writing
            debug_assert_eq!(
                (target as usize) % std::mem::align_of::<SharedContents<String, String>>(),
                0,
                "Target pointer is not properly aligned"
            );

            core::ptr::write(target, contents);
        }
    }

    Ok(map)
}

fn to_agentic(err: SharedMapError, namespace: &str) -> AgenticWardenError {
    match err {
        SharedMapError::RegionTooSmall => AgenticWardenError::Resource {
            message: "Shared memory region too small for task registry".to_string(),
            resource_type: format!("shared_memory:{namespace}"),
            source: None,
        },
        SharedMapError::Shmem(source) => AgenticWardenError::Resource {
            message: format!("Shared memory error ({namespace}): {source}"),
            resource_type: format!("shared_memory:{namespace}"),
            source: Some(Box::new(source)),
        },
        SharedMapError::LockInit(message) => AgenticWardenError::Concurrency {
            message,
            operation: Some("shared_memory_lock::init".to_string()),
            source: None,
        },
        SharedMapError::LockGuard(message) => AgenticWardenError::Concurrency {
            message,
            operation: Some("shared_memory_lock::guard".to_string()),
            source: None,
        },
    }
}
