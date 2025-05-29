#include "absl/base/optimization.h"
#include "tcmalloc/tcmalloc.h"

extern "C" {
    ABSL_ATTRIBUTE_UNUSED ABSL_CACHELINE_ALIGNED void* TCMallocInternalNewAlignedNothrowBridge(
        size_t size, std::align_val_t alignment
    ) {
        return TCMallocInternalNewAlignedNothrow(size, alignment, std::nothrow);
    }
}
