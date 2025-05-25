#include "tcmalloc/malloc_extension.h"

extern "C" {
    bool NeedsProcessBackgroundActions() {
        return tcmalloc::MallocExtension::NeedsProcessBackgroundActions();
    }

    void ProcessBackgroundActions() {
        tcmalloc::MallocExtension::ProcessBackgroundActions();
    }
}
