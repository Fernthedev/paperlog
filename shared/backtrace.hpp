#pragma once
#include <optional>
#include <string_view>

#include "logger.hpp"

#if __has_include(<unwind.h>)
#include <cxxabi.h>
#include <dlfcn.h>
#include <elf.h>
#include <link.h>
#include <unistd.h>
#include <unwind.h>
#include <map>

#define HAS_UNWIND
#else
#warning No unwind support, backtraces will do nothing
#endif

namespace Paper {
namespace Logger {
#ifdef HAS_UNWIND
// Backtrace stuff largely written by StackDoubleFlow
// https://github.com/sc2ad/beatsaber-hook/blob/138101a5a2b494911583b62140af6acf6e955e72/src/utils/logging.cpp#L211-L289
namespace {
struct BacktraceState {
  void** current;
  void** end;
};
static _Unwind_Reason_Code unwindCallback(struct _Unwind_Context* context, void* arg) {
  BacktraceState* state = static_cast<BacktraceState*>(arg);
  uintptr_t pc = _Unwind_GetIP(context);
  if (pc != 0U) {
    if (state->current == state->end) {
      return _URC_END_OF_STACK;
    } else {
      *state->current++ = reinterpret_cast<void*>(pc);
    }
  }
  return _URC_NO_REASON;
}
size_t captureBacktrace(void** buffer, uint16_t max) {
  BacktraceState state{ buffer, buffer + max };
  _Unwind_Backtrace(unwindCallback, &state);

  return state.current - buffer;
}

inline std::optional<std::string> getBuildId(std::string_view filename) {
  std::ifstream infile(filename.data(), std::ios_base::binary);
  if (!infile.is_open()) {
    return std::nullopt;
  }
  infile.seekg(0);
  ElfW(Ehdr) elf;
  infile.read(reinterpret_cast<char*>(&elf), sizeof(ElfW(Ehdr)));
  for (int i = 0; i < elf.e_shnum; i++) {
    infile.seekg(elf.e_shoff + i * elf.e_shentsize);
    ElfW(Shdr) section;
    infile.read(reinterpret_cast<char*>(&section), sizeof(ElfW(Shdr)));
    if (section.sh_type == SHT_NOTE && section.sh_size == 0x24) {
      char data[0x24];
      infile.seekg(section.sh_offset);
      infile.read(data, 0x24);
      ElfW(Nhdr)* note = reinterpret_cast<ElfW(Nhdr)*>(data);
      if (note->n_namesz == 4 && note->n_descsz == 20) {
        if (memcmp(reinterpret_cast<void*>(data + 12), "GNU", 4) == 0) {
          std::stringstream stream;
          stream << std::hex;
          auto buildIdAddr = reinterpret_cast<uint8_t*>(data + 16);
          for (int i = 0; i < 5; i++) {
            uint32_t value;
            auto ptr = (reinterpret_cast<uint8_t*>(&value));
            ptr[0] = *(buildIdAddr + i * sizeof(uint32_t) + 3);
            ptr[1] = *(buildIdAddr + i * sizeof(uint32_t) + 2);
            ptr[2] = *(buildIdAddr + i * sizeof(uint32_t) + 1);
            ptr[3] = *(buildIdAddr + i * sizeof(uint32_t));
            stream << std::setfill('0') << std::setw(sizeof(uint32_t) * 2) << value;
          }
          auto buildid = stream.str();
          return buildid;
        }
      }
    }
  }
  return std::nullopt;
}
} // namespace

inline void Backtrace(std::string_view const tag, uint16_t frameCount) {
  if (frameCount <= 0) return;
  void* buffer[frameCount + 1];
  captureBacktrace(buffer, frameCount + 1);
  fmtLogTag<LogLevel::DBG>("Printing backtrace with: {} max lines:", tag, frameCount);
  fmtLogTag<LogLevel::DBG>("*** *** *** *** *** *** *** *** *** *** *** *** *** *** *** ***", tag);
  fmtLogTag<LogLevel::DBG>("pid: {}, tid: {}", tag, getpid(), gettid());
  std::map<char const*, std::optional<std::string>> buildIds;
  for (uint16_t i = 0; i < frameCount; ++i) {
    Dl_info info;
    if (dladdr(buffer[i + 1], &info) != 0) {
      if (!buildIds.contains(info.dli_fname)) {
        buildIds[info.dli_fname] = getBuildId(info.dli_fname);
      }
      auto buildId = buildIds[info.dli_fname];
      bool hasBuildId = buildId.has_value();
      // Buffer points to 1 instruction ahead
      uintptr_t addr = reinterpret_cast<char*>(buffer[i + 1]) - reinterpret_cast<char*>(info.dli_fbase) - 4;
      uintptr_t offset = reinterpret_cast<char*>(buffer[i + 1]) - reinterpret_cast<char*>(info.dli_saddr) - 4;
      if (info.dli_sname) {
        int status;
        char const* demangled = abi::__cxa_demangle(info.dli_sname, nullptr, nullptr, &status);
        if (status) {
          demangled = info.dli_sname;
        }
        if (offset < 10000) {
          if (hasBuildId) {
            fmtLogTag<LogLevel::DBG>("      #{:02} pc {:016x}  {} ({}+{}) (BuildId: {})", tag, i, addr, info.dli_fname,
                                     demangled, offset, buildId->c_str());
          } else {
            fmtLogTag<LogLevel::DBG>("      #{:02} pc {:016x}  {} ({}+{})", tag, i, addr, info.dli_fname, demangled,
                                     offset);
          }
        } else {
          if (hasBuildId) {
            fmtLogTag<LogLevel::DBG>("      #{:02} pc {:016x}  {} ({}) (BuildId: {})", tag, i, addr, info.dli_fname,
                                     demangled, buildId->c_str());
          } else {
            fmtLogTag<LogLevel::DBG>("      #{:02} pc {:016x}  {} ({})", tag, i, addr, info.dli_fname, demangled);
          }
        }
        if (!status) {
          free(const_cast<char*>(demangled));
        }
      } else {
        if (hasBuildId) {
          fmtLogTag<LogLevel::DBG>("      #{:02} pc {:016x}  {} (BuildId: {})", tag, i, addr, info.dli_fname,
                                   buildId->c_str());
        } else {
          fmtLogTag<LogLevel::DBG>("      #{:02} pc {:016x}  {}", tag, i, addr, info.dli_fname);
        }
      }
    }
  }
}

#else

inline void Backtrace(std::string_view, uint16_t) {}

#endif

inline auto Backtrace(uint16_t frameCount) {
  return Backtrace(GLOBAL_TAG, frameCount);
}
} // namespace Logger
} // namespace Paper