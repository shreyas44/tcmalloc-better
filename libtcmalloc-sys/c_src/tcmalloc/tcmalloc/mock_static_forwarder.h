// Copyright 2021 The TCMalloc Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#ifndef TCMALLOC_MOCK_STATIC_FORWARDER_H_
#define TCMALLOC_MOCK_STATIC_FORWARDER_H_

#include <cstddef>
#include <cstdint>
#include <map>
#include <new>

#include "gmock/gmock.h"
#include "absl/synchronization/mutex.h"
#include "absl/time/time.h"
#include "absl/types/span.h"
#include "tcmalloc/pages.h"
#include "tcmalloc/span.h"

namespace tcmalloc {
namespace tcmalloc_internal {

class FakeStaticForwarder {
 public:
  FakeStaticForwarder() : class_size_(0), pages_() {}
  void Init(size_t class_size, size_t pages, size_t num_objects_to_move) {
    class_size_ = class_size;
    pages_ = Length(pages);
    num_objects_to_move_ = num_objects_to_move;
    clock_ = 1234;
  }
  uint64_t clock_now() const { return clock_; }
  double clock_frequency() const {
    return absl::ToDoubleNanoseconds(absl::Seconds(2));
  }
  void AdvanceClock(absl::Duration d) {
    clock_ += absl::ToDoubleSeconds(d) * clock_frequency();
  }

  size_t class_to_size(int size_class) const { return class_size_; }
  Length class_to_pages(int size_class) const { return pages_; }
  size_t num_objects_to_move() const { return num_objects_to_move_; }

  void MapObjectsToSpans(absl::Span<void*> batch, Span** spans,
                         int expected_size_class) {
    for (size_t i = 0; i < batch.size(); ++i) {
      spans[i] = MapObjectToSpan(batch[i]);
    }
  }

  [[nodiscard]] Span* MapObjectToSpan(const void* object) {
    const PageId page = PageIdContaining(object);

    absl::MutexLock l(&mu_);
    auto it = map_.lower_bound(page);
    if (it->first != page && it != map_.begin()) {
      --it;
    }

    if (it->first <= page && page <= it->second.span->last_page()) {
      return it->second.span;
    }

    return nullptr;
  }

  [[nodiscard]] Span* AllocateSpan(int, size_t objects_per_span,
                                   Length pages_per_span) {
    void* backing =
        ::operator new(pages_per_span.in_bytes(), std::align_val_t(kPageSize));
    PageId page = PageIdContaining(backing);

    auto* span = new Span(Range(page, pages_per_span));

    absl::MutexLock l(&mu_);
    SpanInfo info;
    info.span = span;
    SpanAllocInfo span_alloc_info = {
        .objects_per_span = objects_per_span,
        .density = AccessDensityPrediction::kSparse};
    info.span_alloc_info = span_alloc_info;
    map_.emplace(page, info);
    return span;
  }

  void DeallocateSpans(size_t, absl::Span<Span*> free_spans) {
    {
      absl::MutexLock l(&mu_);
      for (Span* span : free_spans) {
        auto it = map_.find(span->first_page());
        EXPECT_NE(it, map_.end());
        map_.erase(it);
      }
    }

    for (Span* span : free_spans) {
      ::operator delete(span->start_address(), std::align_val_t(kPageSize));
      delete span;
    }
  }

 private:
  struct SpanInfo {
    Span* span;
    SpanAllocInfo span_alloc_info;
  };

  absl::Mutex mu_;
  std::map<PageId, SpanInfo> map_ ABSL_GUARDED_BY(mu_);
  size_t class_size_;
  Length pages_;
  size_t num_objects_to_move_;
  uint64_t clock_;
};

class RawMockStaticForwarder : public FakeStaticForwarder {
 public:
  RawMockStaticForwarder() {
    ON_CALL(*this, class_to_size).WillByDefault([this](int size_class) {
      return FakeStaticForwarder::class_to_size(size_class);
    });
    ON_CALL(*this, class_to_pages).WillByDefault([this](int size_class) {
      return FakeStaticForwarder::class_to_pages(size_class);
    });
    ON_CALL(*this, num_objects_to_move).WillByDefault([this]() {
      return FakeStaticForwarder::num_objects_to_move();
    });
    ON_CALL(*this, Init)
        .WillByDefault([this](size_t size_class, size_t pages,
                              size_t num_objects_to_move) {
          FakeStaticForwarder::Init(size_class, pages, num_objects_to_move);
        });

    ON_CALL(*this, MapObjectsToSpans)
        .WillByDefault([this](absl::Span<void*> batch, Span** spans,
                              int expected_size_class) {
          return FakeStaticForwarder::MapObjectsToSpans(batch, spans,
                                                        expected_size_class);
        });
    ON_CALL(*this, AllocateSpan)
        .WillByDefault([this](int size_class, size_t objects_per_span,
                              Length pages_per_span) {
          return FakeStaticForwarder::AllocateSpan(size_class, objects_per_span,
                                                   pages_per_span);
        });
    ON_CALL(*this, DeallocateSpans)
        .WillByDefault([this](size_t objects_per_span,
                              absl::Span<Span*> free_spans) {
          FakeStaticForwarder::DeallocateSpans(objects_per_span, free_spans);
        });
  }

  MOCK_METHOD(size_t, class_to_size, (int size_class));
  MOCK_METHOD(Length, class_to_pages, (int size_class));
  MOCK_METHOD(size_t, num_objects_to_move, ());
  MOCK_METHOD(void, Init,
              (size_t class_size, size_t pages, size_t num_objects_to_move));
  MOCK_METHOD(void, MapObjectsToSpans,
              (absl::Span<void*> batch, Span** spans, int expected_size_class));
  MOCK_METHOD(Span*, AllocateSpan,
              (int size_class, size_t objects_per_span, Length pages_per_span));
  MOCK_METHOD(void, DeallocateSpans,
              (size_t object_per_span, absl::Span<Span*> free_spans));
};

using MockStaticForwarder = testing::NiceMock<RawMockStaticForwarder>;

// Wires up a largely functional CentralFreeList + MockStaticForwarder.
//
// By default, it fills allocations and responds sensibly.  Because it backs
// onto malloc/free, it will detect leaks and memory misuse when run under
// sanitizers.
//
// Exposes the underlying mocks to allow for more whitebox tests.
template <typename CentralFreeListT>
class FakeCentralFreeListEnvironment {
 public:
  using CentralFreeList = CentralFreeListT;
  using Forwarder = typename CentralFreeListT::Forwarder;

  static constexpr int kSizeClass = 1;
  size_t objects_per_span() {
    return forwarder().class_to_pages(kSizeClass).in_bytes() /
           forwarder().class_to_size(kSizeClass);
  }
  size_t batch_size() { return forwarder().num_objects_to_move(); }

  explicit FakeCentralFreeListEnvironment(size_t class_size, size_t pages,
                                          size_t num_objects_to_move) {
    forwarder().Init(class_size, pages, num_objects_to_move);
    cache_.Init(kSizeClass);
  }

  ~FakeCentralFreeListEnvironment() { EXPECT_EQ(cache_.length(), 0); }

  CentralFreeList& central_freelist() { return cache_; }

  Forwarder& forwarder() { return cache_.forwarder(); }

 private:
  CentralFreeList cache_;
};

}  // namespace tcmalloc_internal
}  // namespace tcmalloc

#endif  // TCMALLOC_MOCK_STATIC_FORWARDER_H_
