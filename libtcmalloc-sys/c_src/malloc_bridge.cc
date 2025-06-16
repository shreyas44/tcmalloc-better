#include "tcmalloc/tcmalloc.cc"

extern "C" {
    ABSL_ATTRIBUTE_UNUSED ABSL_CACHELINE_ALIGNED void* BridgeTCMallocInternalNewAlignedNothrow(
        size_t size, std::align_val_t alignment
    ) {
        return TCMallocInternalNewAlignedNothrow(size, alignment, std::nothrow);
    }

    // This code is based on `do_realloc` with alignment acceptance and without copying.
    // Rust code should make copying with the knowledge of properly alignment.
    //
    // TODO: migrate whole realloc logic to rust
    ABSL_ATTRIBUTE_UNUSED ABSL_CACHELINE_ALIGNED void* BridgePrepareReallocAligned(
        void* old_ptr, size_t new_size, std::align_val_t alignment, size_t* old_size_p
    ) {
      TC_ASSERT(absl::has_single_bit(static_cast<size_t>(alignment)));
      if (new_size == 0) {
        // UB in rust, so we return without any reallocation
        return nullptr;
      }

      tc_globals.InitIfNecessary();
      // Get the size of the old entry
      size_t old_size;
      bool old_was_sampled;
      const PageId p = PageIdContainingTagged(old_ptr);
      const size_t old_size_class = tc_globals.pagemap().sizeclass(p);
      if (old_size_class != 0) {
        old_size = tc_globals.sizemap().class_to_size(old_size_class);
        old_was_sampled = false;
      } else {
        Span* span = tc_globals.pagemap().GetExistingDescriptor(p);
        if (ABSL_PREDICT_FALSE(span == nullptr)) {
          ReportDoubleFree(tc_globals, old_ptr);
        }
        old_size = GetLargeSize(old_ptr, *span);
        old_was_sampled = span->sampled();
      }
      TC_ASSERT(old_size == GetSize(old_ptr));
      size_t new_size_class;
      if (!tc_globals.sizemap().GetSizeClass(CppPolicy().Nothrow().AlignAs(alignment), new_size,
                                             &new_size_class)) {
        new_size_class = 0;
      }
      // We can avoid reallocating if all the following conditions are met:
      //   - The size class of the existing allocation and new allocation are the
      //     same. If both have the size class of 0 (too large to fit into size
      //     classes), then both sizes must have the same kPageSize pages.
      //   - The allocation would not be sampled.
      //   - The existing allocation is not owned by the guarded page allocator.
      if (!old_was_sampled && old_size_class == new_size_class &&
          (old_size_class != 0 || BytesToLengthCeil(old_size).in_bytes() ==
                                      BytesToLengthCeil(new_size).in_bytes()) &&
          (new_size <= old_size ||
           !GetThreadSampler()->WillRecordAllocation(new_size - old_size)) &&
          !tc_globals.guardedpage_allocator().PointerIsMine(old_ptr)) {
        if (new_size > old_size) {
          GetThreadSampler()->ReportAllocation(new_size - old_size);
        }
        // We still need to call hooks to report the updated size:
        size_t actual_new_size;
        if (new_size_class != 0) {
          actual_new_size = tc_globals.sizemap().class_to_size(new_size_class);
        } else {
          actual_new_size = BytesToLengthCeil(new_size).in_bytes();
        }
        tcmalloc::MallocHook::InvokeDeleteHook(
            {const_cast<void*>(old_ptr), old_size,
             tcmalloc::HookMemoryMutable::kImmutable});
        tcmalloc::MallocHook::InvokeNewHook(
            {const_cast<void*>(old_ptr), new_size, actual_new_size,
             tcmalloc::HookMemoryMutable::kImmutable});
        TC_ASSERT(GetSize(old_ptr) == actual_new_size);
        return old_ptr;
      }
      void* new_ptr = fast_alloc(new_size, CppPolicy().Nothrow().AlignAs(alignment));
      if (new_ptr == nullptr) {
        return nullptr;
      }

      *old_size_p = old_size;

      return new_ptr;
    }
}
