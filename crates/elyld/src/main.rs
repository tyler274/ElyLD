#[cfg(all(feature = "mimalloc", feature = "mimalloc-dynamic"))]
compile_error!("features `mimalloc` and `mimalloc-dynamic` are mutually exclusive");
#[cfg(all(feature = "mimalloc", feature = "dhat"))]
compile_error!("features `mimalloc` and `dhat` are mutually exclusive");
#[cfg(all(feature = "mimalloc-dynamic", feature = "dhat"))]
compile_error!("features `mimalloc-dynamic` and `dhat` are mutually exclusive");

#[cfg(all(
    feature = "mimalloc",
    not(feature = "mimalloc-dynamic"),
    not(feature = "dhat"),
    not(target_os = "wasi")
))]
#[global_allocator]
static MIMALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(all(
    feature = "mimalloc-dynamic",
    not(feature = "mimalloc"),
    not(feature = "dhat")
))]
#[global_allocator]
static MIMALLOC: MimallocDynamic = MimallocDynamic;

#[cfg(all(
    feature = "dhat",
    not(feature = "mimalloc"),
    not(feature = "mimalloc-dynamic")
))]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[cfg(feature = "mimalloc-dynamic")]
struct MimallocDynamic;

#[cfg(feature = "mimalloc-dynamic")]
unsafe impl std::alloc::GlobalAlloc for MimallocDynamic {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        unsafe { mi_malloc_aligned(layout.size(), layout.align()).cast() }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: std::alloc::Layout) {
        unsafe { mi_free(ptr.cast()) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: std::alloc::Layout, new_size: usize) -> *mut u8 {
        unsafe { mi_realloc_aligned(ptr.cast(), new_size, layout.align()).cast() }
    }
}

#[cfg(feature = "mimalloc-dynamic")]
unsafe extern "C" {
    fn mi_malloc_aligned(size: usize, alignment: usize) -> *mut std::ffi::c_void;
    fn mi_realloc_aligned(
        p: *mut std::ffi::c_void,
        newsize: usize,
        alignment: usize,
    ) -> *mut std::ffi::c_void;
    fn mi_free(p: *mut std::ffi::c_void);
}

fn main() {
    if let Err(error) = run() {
        libelyld::error::report_error_and_exit(&error)
    }
}

/// The current ElyLD version as written by build.rs.
const VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/version.txt"));

fn run() -> libelyld::error::Result {
    #[cfg(feature = "dhat")]
    let _profiler = dhat::Profiler::new_heap();

    libelyld::init_timing()?;

    let mut args = libelyld::Args::new(std::env::args)?;
    args.set_version(VERSION);
    args.parse(std::env::args)?;

    if libelyld::should_fork(&args) {
        // Safety: We haven't spawned any threads yet.
        unsafe { libelyld::run_in_subprocess(args) };
    } else {
        // Run the linker in this process without forking.

        // Note, we need to setup tracing before worker, otherwise the threads won't contribute to
        // counters such as --time=cycles,instructions etc.
        libelyld::setup_tracing(&args)?;

        libelyld::run(args)
    }
}
