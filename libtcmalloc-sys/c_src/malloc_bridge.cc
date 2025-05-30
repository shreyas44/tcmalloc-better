#include "tcmalloc/tcmalloc.cc"

extern "C" {
    ABSL_ATTRIBUTE_UNUSED ABSL_CACHELINE_ALIGNED void* BridgeTCMallocInternalNewAlignedNothrow(
        size_t size, std::align_val_t alignment
    ) {
        return TCMallocInternalNewAlignedNothrow(size, alignment, std::nothrow);
    }

    // this code is base on do_realloc with alignment acceptance and without copying
    // rust make copying better than generic memcpy due to knowledge of properly alignment
    // and regions is not overlapped
    ABSL_ATTRIBUTE_UNUSED ABSL_CACHELINE_ALIGNED void* BridgePrepareReallocAligned(
        void* old_ptr, size_t new_size, std::align_val_t alignment, size_t* old_size_p
    ) {
      TC_ASSERT(absl::has_single_bit(static_cast<size_t>(alignment)));
      if (new_size == 0) {
        // UB in rust
        return nullptr;
      }

      tc_globals.InitIfNecessary();
      // Get the size of the old entry
      const size_t old_size = GetSize(old_ptr);

      // Reallocate if the new size is larger than the old size,
      // or if the new size is significantly smaller than the old size.
      // We do hysteresis to avoid resizing ping-pongs:
      //    . If we need to grow, grow to max(new_size, old_size * 1.X)
      //    . Don't shrink unless new_size < old_size * 0.Y
      // X and Y trade-off time for wasted space.  For now we do 1.25 and 0.5.
      // Also reallocate if the current allocation is guarded or if the new
      // allocation will be sampled (and potentially guarded), this allows
      // to detect both use-after-frees on the old pointer and precise
      // out-of-bounds accesses on the new pointer for all possible combinations
      // of new/old size.
      const size_t min_growth = std::min(
          old_size / 4,
          std::numeric_limits<size_t>::max() - old_size);  // Avoid overflow.
      const size_t lower_bound_to_grow = old_size + min_growth;
      const size_t upper_bound_to_shrink = old_size / 2;
      const size_t alloc_size =
          new_size > old_size ? std::max(new_size, lower_bound_to_grow) : new_size;
      // Sampled allocations are reallocated and copied even if not strictly
      // necessary. This is problematic for very large allocations, since some old
      // programs rely on realloc to be very efficient (e.g. call realloc to the
      // same size repeatedly assuming it will do nothing). Very large allocations
      // are both all sampled and expensive to allocate and copy, so don't
      // reallocate them if not necessary. The use of kMaxSize here as a notion of
      // "very large" is somewhat arbitrary.
      const bool will_sample =
          alloc_size <= tcmalloc::tcmalloc_internal::kMaxSize &&
          GetThreadSampler()->WillRecordAllocation(alloc_size);
      if ((new_size > old_size) || (new_size < upper_bound_to_shrink) ||
          will_sample ||
          tc_globals.guardedpage_allocator().PointerIsMine(old_ptr)) {
        // Need to reallocate.
        void* new_ptr = nullptr;

        // Note: we shouldn't use larger size if the allocation will be sampled
        // b/c we will record wrong size and guarded page allocator won't be able
        // to properly enforce size limit.
        if (new_size > old_size && new_size < lower_bound_to_grow && !will_sample) {
          // Avoid fast_alloc() reporting a hook with the lower bound size
          // as the expectation for pointer returning allocation functions
          // is that malloc hooks are invoked with the requested_size.
          new_ptr = fast_alloc(lower_bound_to_grow, CppPolicy().Nothrow().AlignAs(alignment).WithoutHooks());
        }
        if (new_ptr == nullptr) {
          // Either new_size is not a tiny increment, or last fast_alloc failed.
          new_ptr = fast_alloc(new_size, CppPolicy().Nothrow().AlignAs(alignment));
        }
        if (new_ptr == nullptr) {
          return nullptr;
        }

        *old_size_p = old_size;

        return new_ptr;
      } else {
        return old_ptr;
      }
    }
}
