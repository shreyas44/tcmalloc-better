use patch::{Hunk, Line, Patch};
use std::borrow::Cow;
use std::cell::OnceCell;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::{env, fs};
use strum::{EnumIter, IntoEnumIterator};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Debug, Copy, Clone, EnumIter)]
enum PageSize {
    P8k,
    P32k,
    P256k,
    PSmall,
}

impl PageSize {
    fn from_env() -> Result<PageSize, Cow<'static, str>> {
        let page_size_cell = OnceCell::new();
        for (page_size, feature) in
            PageSize::iter().map(|page_size| (page_size, page_size.feature()))
        {
            if env::var_os(feature).is_some() {
                page_size_cell.set(page_size).map_err(|err| {
                    format!(
                        "Can not set up more than one page size, \
                        already defined = {page_size_cell:?}, new one = {err:?}"
                    )
                })?;
            }
        }
        Ok(page_size_cell
            .into_inner()
            .ok_or("One page size should be defined")?)
    }

    fn to_define(self) -> &'static str {
        match self {
            PageSize::P8k => "TCMALLOC_INTERNAL_8K_PAGES",
            PageSize::P32k => "TCMALLOC_INTERNAL_32K_PAGES",
            PageSize::P256k => "TCMALLOC_INTERNAL_256K_PAGES",
            PageSize::PSmall => "TCMALLOC_INTERNAL_SMALL_BUT_SLOW",
        }
    }

    fn feature(self) -> &'static str {
        match self {
            PageSize::P8k => "CARGO_FEATURE_8K_PAGES",
            PageSize::P32k => "CARGO_FEATURE_32K_PAGES",
            PageSize::P256k => "CARGO_FEATURE_256K_PAGES",
            PageSize::PSmall => "CARGO_FEATURE_SMALL_BUT_SLOW",
        }
    }
}

#[derive(Debug, Copy, Clone, EnumIter)]
enum MadviseHugePages {
    Always,
    ByVar,
}

impl MadviseHugePages {
    fn from_env() -> Result<Option<Self>, String> {
        let madvise_huge_pages_cell = OnceCell::new();
        for (madvise_huge_pages, feature) in MadviseHugePages::iter()
            .map(|madvise_huge_pages| (madvise_huge_pages, madvise_huge_pages.feature()))
        {
            if env::var_os(feature).is_some() {
                madvise_huge_pages_cell
                    .set(madvise_huge_pages)
                    .map_err(|err| {
                        format!(
                            "Can not set up more than one madvise huge pages feature, \
                        already defined = {madvise_huge_pages_cell:?}, new one = {err:?}"
                        )
                    })?;
            }
        }
        Ok(madvise_huge_pages_cell.get().copied())
    }

    fn to_define(self) -> &'static str {
        match self {
            MadviseHugePages::Always => "RUST_PATCHES_DISABLE_MADV_HUGEPAGE_ALWAYS",
            MadviseHugePages::ByVar => "RUST_PATCHES_DISABLE_MADV_HUGEPAGE_BY_VAR",
        }
    }

    fn feature(self) -> &'static str {
        match self {
            MadviseHugePages::Always => "CARGO_FEATURE_DISABLE_MADV_HUGEPAGE_ALWAYS",
            MadviseHugePages::ByVar => "CARGO_FEATURE_DISABLE_MADV_HUGEPAGE_BY_VAR",
        }
    }
}

fn compile(src_dir: impl AsRef<Path>) {
    let src_dir = src_dir.as_ref();
    let join_src_dir = |path: &str| src_dir.join(path);
    let mut cc = cc::Build::new();
    cc.files(
        [
            "c_src/abseil-cpp/absl/base/internal/cycleclock.cc",
            "c_src/abseil-cpp/absl/base/internal/low_level_alloc.cc",
            "c_src/abseil-cpp/absl/base/internal/raw_logging.cc",
            "c_src/abseil-cpp/absl/base/internal/spinlock.cc",
            "c_src/abseil-cpp/absl/base/internal/spinlock_wait.cc",
            "c_src/abseil-cpp/absl/base/internal/strerror.cc",
            "c_src/abseil-cpp/absl/base/internal/sysinfo.cc",
            "c_src/abseil-cpp/absl/base/internal/thread_identity.cc",
            "c_src/abseil-cpp/absl/base/internal/throw_delegate.cc",
            "c_src/abseil-cpp/absl/base/internal/unscaledcycleclock.cc",
            "c_src/abseil-cpp/absl/base/log_severity.cc",
            "c_src/abseil-cpp/absl/container/internal/hashtablez_sampler.cc",
            "c_src/abseil-cpp/absl/container/internal/hashtablez_sampler_force_weak_definition.cc",
            "c_src/abseil-cpp/absl/container/internal/raw_hash_set.cc",
            "c_src/abseil-cpp/absl/crc/crc32c.cc",
            "c_src/abseil-cpp/absl/crc/internal/cpu_detect.cc",
            "c_src/abseil-cpp/absl/crc/internal/crc.cc",
            "c_src/abseil-cpp/absl/crc/internal/crc_cord_state.cc",
            "c_src/abseil-cpp/absl/crc/internal/crc_memcpy_fallback.cc",
            "c_src/abseil-cpp/absl/crc/internal/crc_memcpy_x86_arm_combined.cc",
            "c_src/abseil-cpp/absl/crc/internal/crc_non_temporal_memcpy.cc",
            "c_src/abseil-cpp/absl/crc/internal/crc_x86_arm_combined.cc",
            "c_src/abseil-cpp/absl/debugging/internal/address_is_readable.cc",
            "c_src/abseil-cpp/absl/debugging/internal/decode_rust_punycode.cc",
            "c_src/abseil-cpp/absl/debugging/internal/demangle.cc",
            "c_src/abseil-cpp/absl/debugging/internal/demangle_rust.cc",
            "c_src/abseil-cpp/absl/debugging/internal/elf_mem_image.cc",
            "c_src/abseil-cpp/absl/debugging/internal/utf8_for_code_point.cc",
            "c_src/abseil-cpp/absl/debugging/internal/vdso_support.cc",
            "c_src/abseil-cpp/absl/debugging/stacktrace.cc",
            "c_src/abseil-cpp/absl/debugging/symbolize.cc",
            "c_src/abseil-cpp/absl/hash/internal/city.cc",
            "c_src/abseil-cpp/absl/hash/internal/hash.cc",
            "c_src/abseil-cpp/absl/hash/internal/low_level_hash.cc",
            "c_src/abseil-cpp/absl/numeric/int128.cc",
            "c_src/abseil-cpp/absl/profiling/internal/exponential_biased.cc",
            "c_src/abseil-cpp/absl/status/internal/status_internal.cc",
            "c_src/abseil-cpp/absl/status/status.cc",
            "c_src/abseil-cpp/absl/status/status_payload_printer.cc",
            "c_src/abseil-cpp/absl/status/statusor.cc",
            "c_src/abseil-cpp/absl/strings/ascii.cc",
            "c_src/abseil-cpp/absl/strings/charconv.cc",
            "c_src/abseil-cpp/absl/strings/cord.cc",
            "c_src/abseil-cpp/absl/strings/cord_analysis.cc",
            "c_src/abseil-cpp/absl/strings/cord_buffer.cc",
            "c_src/abseil-cpp/absl/strings/escaping.cc",
            "c_src/abseil-cpp/absl/strings/internal/charconv_bigint.cc",
            "c_src/abseil-cpp/absl/strings/internal/charconv_parse.cc",
            "c_src/abseil-cpp/absl/strings/internal/cord_internal.cc",
            "c_src/abseil-cpp/absl/strings/internal/cord_rep_btree.cc",
            "c_src/abseil-cpp/absl/strings/internal/cord_rep_btree_navigator.cc",
            "c_src/abseil-cpp/absl/strings/internal/cord_rep_btree_reader.cc",
            "c_src/abseil-cpp/absl/strings/internal/cord_rep_consume.cc",
            "c_src/abseil-cpp/absl/strings/internal/cord_rep_crc.cc",
            "c_src/abseil-cpp/absl/strings/internal/cordz_functions.cc",
            "c_src/abseil-cpp/absl/strings/internal/cordz_handle.cc",
            "c_src/abseil-cpp/absl/strings/internal/cordz_info.cc",
            "c_src/abseil-cpp/absl/strings/internal/damerau_levenshtein_distance.cc",
            "c_src/abseil-cpp/absl/strings/internal/escaping.cc",
            "c_src/abseil-cpp/absl/strings/internal/memutil.cc",
            "c_src/abseil-cpp/absl/strings/internal/ostringstream.cc",
            "c_src/abseil-cpp/absl/strings/internal/str_format/arg.cc",
            "c_src/abseil-cpp/absl/strings/internal/str_format/bind.cc",
            "c_src/abseil-cpp/absl/strings/internal/str_format/extension.cc",
            "c_src/abseil-cpp/absl/strings/internal/str_format/float_conversion.cc",
            "c_src/abseil-cpp/absl/strings/internal/str_format/output.cc",
            "c_src/abseil-cpp/absl/strings/internal/str_format/parser.cc",
            "c_src/abseil-cpp/absl/strings/internal/stringify_sink.cc",
            "c_src/abseil-cpp/absl/strings/internal/utf8.cc",
            "c_src/abseil-cpp/absl/strings/match.cc",
            "c_src/abseil-cpp/absl/strings/numbers.cc",
            "c_src/abseil-cpp/absl/strings/str_cat.cc",
            "c_src/abseil-cpp/absl/strings/str_replace.cc",
            "c_src/abseil-cpp/absl/strings/str_split.cc",
            "c_src/abseil-cpp/absl/strings/string_view.cc",
            "c_src/abseil-cpp/absl/strings/substitute.cc",
            "c_src/abseil-cpp/absl/synchronization/barrier.cc",
            "c_src/abseil-cpp/absl/synchronization/blocking_counter.cc",
            "c_src/abseil-cpp/absl/synchronization/internal/create_thread_identity.cc",
            "c_src/abseil-cpp/absl/synchronization/internal/futex_waiter.cc",
            "c_src/abseil-cpp/absl/synchronization/internal/graphcycles.cc",
            "c_src/abseil-cpp/absl/synchronization/internal/kernel_timeout.cc",
            "c_src/abseil-cpp/absl/synchronization/internal/per_thread_sem.cc",
            "c_src/abseil-cpp/absl/synchronization/internal/pthread_waiter.cc",
            "c_src/abseil-cpp/absl/synchronization/internal/sem_waiter.cc",
            "c_src/abseil-cpp/absl/synchronization/internal/stdcpp_waiter.cc",
            "c_src/abseil-cpp/absl/synchronization/internal/waiter_base.cc",
            "c_src/abseil-cpp/absl/synchronization/internal/win32_waiter.cc",
            "c_src/abseil-cpp/absl/synchronization/mutex.cc",
            "c_src/abseil-cpp/absl/synchronization/notification.cc",
            "c_src/abseil-cpp/absl/time/civil_time.cc",
            "c_src/abseil-cpp/absl/time/clock.cc",
            "c_src/abseil-cpp/absl/time/duration.cc",
            "c_src/abseil-cpp/absl/time/format.cc",
            "c_src/abseil-cpp/absl/time/internal/cctz/src/civil_time_detail.cc",
            "c_src/abseil-cpp/absl/time/internal/cctz/src/time_zone_fixed.cc",
            "c_src/abseil-cpp/absl/time/internal/cctz/src/time_zone_format.cc",
            "c_src/abseil-cpp/absl/time/internal/cctz/src/time_zone_if.cc",
            "c_src/abseil-cpp/absl/time/internal/cctz/src/time_zone_impl.cc",
            "c_src/abseil-cpp/absl/time/internal/cctz/src/time_zone_info.cc",
            "c_src/abseil-cpp/absl/time/internal/cctz/src/time_zone_libc.cc",
            "c_src/abseil-cpp/absl/time/internal/cctz/src/time_zone_lookup.cc",
            "c_src/abseil-cpp/absl/time/internal/cctz/src/time_zone_posix.cc",
            "c_src/abseil-cpp/absl/time/internal/cctz/src/zone_info_source.cc",
            "c_src/abseil-cpp/absl/time/time.cc",
            "c_src/abseil-cpp/absl/types/bad_optional_access.cc",
            "c_src/abseil-cpp/absl/types/bad_variant_access.cc",
            "c_src/tcmalloc/tcmalloc/allocation_sample.cc",
            "c_src/tcmalloc/tcmalloc/allocation_sampling.cc",
            "c_src/tcmalloc/tcmalloc/arena.cc",
            "c_src/tcmalloc/tcmalloc/background.cc",
            "c_src/tcmalloc/tcmalloc/central_freelist.cc",
            "c_src/tcmalloc/tcmalloc/common.cc",
            "c_src/tcmalloc/tcmalloc/cpu_cache.cc",
            "c_src/tcmalloc/tcmalloc/deallocation_profiler.cc",
            "c_src/tcmalloc/tcmalloc/error_reporting.cc",
            "c_src/tcmalloc/tcmalloc/experiment.cc",
            "c_src/tcmalloc/tcmalloc/experimental_pow2_size_class.cc",
            "c_src/tcmalloc/tcmalloc/global_stats.cc",
            "c_src/tcmalloc/tcmalloc/guarded_page_allocator.cc",
            "c_src/tcmalloc/tcmalloc/huge_address_map.cc",
            "c_src/tcmalloc/tcmalloc/huge_allocator.cc",
            "c_src/tcmalloc/tcmalloc/huge_cache.cc",
            "c_src/tcmalloc/tcmalloc/huge_page_aware_allocator.cc",
            "c_src/tcmalloc/tcmalloc/internal/allocation_guard.cc",
            "c_src/tcmalloc/tcmalloc/internal/cache_topology.cc",
            "c_src/tcmalloc/tcmalloc/internal/environment.cc",
            "c_src/tcmalloc/tcmalloc/internal/hook_list.cc",
            "c_src/tcmalloc/tcmalloc/internal/logging.cc",
            "c_src/tcmalloc/tcmalloc/internal/memory_stats.cc",
            "c_src/tcmalloc/tcmalloc/internal/memory_tag.cc",
            "c_src/tcmalloc/tcmalloc/internal/mincore.cc",
            "c_src/tcmalloc/tcmalloc/internal/numa.cc",
            "c_src/tcmalloc/tcmalloc/internal/page_size.cc",
            "c_src/tcmalloc/tcmalloc/internal/pageflags.cc",
            "c_src/tcmalloc/tcmalloc/internal/percpu.cc",
            "c_src/tcmalloc/tcmalloc/internal/percpu_rseq_asm.S",
            "c_src/tcmalloc/tcmalloc/internal/percpu_rseq_unsupported.cc",
            "c_src/tcmalloc/tcmalloc/internal/residency.cc",
            "c_src/tcmalloc/tcmalloc/internal/sysinfo.cc",
            "c_src/tcmalloc/tcmalloc/internal/util.cc",
            "c_src/tcmalloc/tcmalloc/legacy_size_classes.cc",
            "c_src/tcmalloc/tcmalloc/malloc_extension.cc",
            "c_src/tcmalloc/tcmalloc/malloc_hook.cc",
            "c_src/tcmalloc/tcmalloc/malloc_tracing_extension.cc",
            "c_src/tcmalloc/tcmalloc/page_allocator.cc",
            "c_src/tcmalloc/tcmalloc/page_allocator_interface.cc",
            "c_src/tcmalloc/tcmalloc/pagemap.cc",
            "c_src/tcmalloc/tcmalloc/parameters.cc",
            "c_src/tcmalloc/tcmalloc/peak_heap_tracker.cc",
            "c_src/tcmalloc/tcmalloc/sampler.cc",
            "c_src/tcmalloc/tcmalloc/segv_handler.cc",
            "c_src/tcmalloc/tcmalloc/selsan/selsan.cc",
            "c_src/tcmalloc/tcmalloc/size_classes.cc",
            "c_src/tcmalloc/tcmalloc/sizemap.cc",
            "c_src/tcmalloc/tcmalloc/span.cc",
            "c_src/tcmalloc/tcmalloc/stack_trace_table.cc",
            "c_src/tcmalloc/tcmalloc/static_vars.cc",
            "c_src/tcmalloc/tcmalloc/stats.cc",
            "c_src/tcmalloc/tcmalloc/system-alloc.cc",
            "c_src/tcmalloc/tcmalloc/thread_cache.cc",
            "c_src/tcmalloc/tcmalloc/transfer_cache.cc",
            "c_src/malloc_bridge.cc",
        ]
        .into_iter()
        .map(join_src_dir),
    );
    if env::var_os("CARGO_FEATURE_EXTENSION").is_some() {
        cc.file(join_src_dir("c_src/malloc_extension_bridge.cc"));
    }
    cc.includes(
        ["c_src/abseil-cpp", "c_src/tcmalloc"]
            .into_iter()
            .map(join_src_dir),
    );
    cc.cpp(true);
    cc.std("c++17");
    cc.define("NOMINMAX", None);
    cc.define("TCMALLOC_INTERNAL_METHODS_ONLY", None);
    let page_size = PageSize::from_env().unwrap();
    cc.define(page_size.to_define(), None);
    if env::var_os("CARGO_FEATURE_DEPRECATED_PERTHREAD").is_some() {
        cc.define("TCMALLOC_DEPRECATED_PERTHREAD", None);
    }
    if env::var_os("CARGO_FEATURE_LEGACY_LOCKING").is_some() {
        cc.define("TCMALLOC_INTERNAL_LEGACY_LOCKING", None);
    }
    if env::var_os("CARGO_FEATURE_NUMA_AWARE").is_some() {
        cc.define("TCMALLOC_INTERNAL_NUMA_AWARE", None);
    }
    if let Some(disable_madv_hugepages) = MadviseHugePages::from_env().unwrap() {
        cc.define(disable_madv_hugepages.to_define(), None);
    }
    if match env::var_os("DEBUG") {
        Some(debug) => debug.is_empty() || debug == "0" || debug == "false",
        None => true,
    } {
        cc.define("NDEBUG", None);
    }
    cc.force_frame_pointer(true);
    cc.pic(true);
    cc.warnings(true);
    cc.extra_warnings(true);
    for flag in [
        "-fno-canonical-system-headers",
        "-no-canonical-prefixes",
        "-fstack-protector",
        "-Wcast-qual",
        "-Wconversion-null",
        "-Wformat-security",
        "-Wno-missing-declarations",
        "-Wno-array-bounds",
        "-Wno-attribute-alias",
        "-Wno-builtin-macro-redefined",
        "-Wno-deprecated-declarations",
        "-Wno-free-nonheap-object",
        "-Wno-sign-compare",
        "-Wno-stringop-overflow",
        "-Wno-uninitialized",
        "-Wno-unused-function",
        "-Wno-unused-result",
        "-Wno-unused-variable",
        "-Wno-unused-parameter",
        "-Wnon-virtual-dtor",
        "-Woverlength-strings",
        "-Wpointer-arith",
        "-Wno-undef",
        "-Wunused-but-set-parameter",
        "-Wunused-local-typedefs",
        "-Wvarargs",
        "-Wvla",
        "-Wwrite-strings",
        "-Wno-missing-field-initializers",
        "-Wno-type-limits",
        "-Wno-ignored-attributes",
    ] {
        cc.flag_if_supported(flag);
    }
    cc.compile("tcmalloc");
}

fn create_dir(path: &Path) {
    let is_not_dir_if_exists = match path.metadata() {
        Ok(metadata) => Some(!metadata.is_dir()),
        Err(_) => None,
    };
    if is_not_dir_if_exists.unwrap_or(false) {
        fs::remove_file(path).unwrap();
    }
    if is_not_dir_if_exists.unwrap_or(true) {
        fs::create_dir(path).unwrap();
    }
}

fn copy_files(c_src: &Path, out_dir: &Path) {
    let mut in_dirs_stack = vec![Cow::Borrowed(c_src)];
    while let Some(in_dir) = in_dirs_stack.pop() {
        create_dir(&out_dir.join(&in_dir));
        for in_entry in fs::read_dir(in_dir).unwrap() {
            let in_entry = in_entry.unwrap();
            let in_path = in_entry.path();
            if in_path
                .file_name()
                .unwrap()
                .as_encoded_bytes()
                .starts_with(b".")
            {
                continue;
            }
            let in_file_type = in_entry.file_type().unwrap();
            if in_file_type.is_dir() {
                in_dirs_stack.push(Cow::Owned(in_path));
            } else if in_file_type.is_file() {
                let out_path = out_dir.join(&in_path);
                println!("cargo::rerun-if-changed={}", in_path.display());
                fs::copy(in_path, &out_path).unwrap();
                let mut perms = out_path.metadata().unwrap().permissions();
                if cfg!(unix) || perms.readonly() {
                    #[cfg(unix)]
                    perms.set_mode(perms.mode() | 0o600);
                    #[cfg(not(unix))]
                    perms.set_readonly(false);
                    fs::set_permissions(out_path, perms).unwrap();
                }
            } else {
                panic!("unknown file type of {in_path:?}: {in_file_type:?}");
            }
        }
    }
}

fn apply_patches(c_src: &Path, out_dir: &Path) {
    let mut in_dir_stack = vec![(Cow::Borrowed(Path::new("patches")), out_dir.join(c_src))];

    while let Some((in_dir, patch_target_dir)) = in_dir_stack.pop() {
        for patch_entry in match fs::read_dir(&in_dir) {
            Ok(patch_entry) => patch_entry,
            Err(err) => panic!("failed to read patch directory {in_dir:?}: {err}"),
        } {
            let patch_entry = patch_entry.unwrap();
            let patch_path = patch_entry.path();
            let patch_file_type = patch_entry.file_type().unwrap();
            if patch_file_type.is_dir() {
                in_dir_stack.push((
                    Cow::Owned(patch_path),
                    patch_target_dir.join(patch_entry.file_name()),
                ));
            } else if patch_file_type.is_file() {
                println!("cargo::rerun-if-changed={}", patch_path.display());
                let patch = fs::read_to_string(patch_path).unwrap();
                for patch in Patch::from_multiple(&patch).unwrap() {
                    let old_path = match patch.old.path.as_ref() {
                        "" | "/dev/null" => None,
                        path => Some(
                            patch_target_dir
                                .join(Path::new(path.strip_prefix("a/").unwrap_or(path))),
                        ),
                    };
                    let new_path = match patch.new.path.as_ref() {
                        "" | "/dev/null" => None,
                        path => Some(
                            patch_target_dir
                                .join(Path::new(path.strip_prefix("b/").unwrap_or(path))),
                        ),
                    };
                    if let Some(new_path) = new_path {
                        let old_data = match &old_path {
                            None => Cow::Borrowed(""),
                            Some(old_path) => Cow::Owned(match fs::read_to_string(old_path) {
                                Ok(old_data) => old_data,
                                Err(e) => panic!("failed to read {old_path:?}: {e}"),
                            }),
                        };
                        let new_data =
                            apply_patch(patch.hunks, patch.end_newline, old_data.as_ref());
                        if old_path.is_none() {
                            if let Some(parent) = patch_target_dir.parent() {
                                fs::create_dir_all(parent).unwrap();
                            }
                        }
                        let mut file = BufWriter::new(File::create(new_path).unwrap());
                        let mut new_data = new_data.into_iter();
                        if let Some(first_line) = new_data.next() {
                            write!(file, "{first_line}").unwrap();
                            for line in new_data {
                                writeln!(file).unwrap();
                                write!(file, "{line}").unwrap();
                            }
                        }
                        file.flush().unwrap();
                    } else {
                        let old_path = old_path.unwrap();
                        fs::remove_file(old_path).unwrap();
                    }
                }
            } else {
                panic!("unknown file type of {patch_path:?}: {patch_file_type:?}");
            }
        }
    }
}

fn apply_patch<'a: 'c, 'b: 'c, 'c>(
    hunks: Vec<Hunk<'a>>,
    end_newline: bool,
    old_data: &'b str,
) -> Vec<&'c str> {
    let old_lines: Vec<&str> = old_data.lines().collect();
    let mut new_lines = Vec::with_capacity(old_lines.len() + 1);
    let mut old_line_i = 0;
    for hunk in hunks {
        let limit = hunk.old_range.start.saturating_sub(1) as usize;
        if old_line_i < limit {
            if let Some(old_lines) = old_lines.get(old_line_i..limit) {
                new_lines.extend(old_lines);
            }
            old_line_i = limit;
        }
        for line in hunk.lines {
            let line_str = match line {
                Line::Context(line_str) | Line::Remove(line_str) => {
                    let line_str = match old_lines.get(old_line_i).copied() {
                        Some(old_line) if old_line == line_str => old_line,
                        old => panic!("line mismatch at {old_line_i}: {old:?} != {line_str:?}"),
                    };
                    old_line_i += 1;
                    line_str
                }
                Line::Add(line_str) => line_str,
            };
            if matches!(line, Line::Context(_) | Line::Add(_)) {
                new_lines.push(line_str);
            }
        }
    }
    if let Some(old_lines) = old_lines.get(old_line_i..) {
        new_lines.extend(old_lines);
    }
    if end_newline || old_data.ends_with('\n') {
        new_lines.push("");
    }
    new_lines
}

fn patch_deps() -> PathBuf {
    let mut out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    out_dir.push("patched_deps");
    create_dir(&out_dir);

    let c_src = Path::new("c_src");

    copy_files(c_src, &out_dir);
    apply_patches(c_src, &out_dir);

    out_dir
}

fn main() {
    let out_dir = patch_deps();
    compile(out_dir);
}
