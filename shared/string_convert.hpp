#pragma once

#include <locale>

namespace Paper::StringConvert {
    namespace detail {
        template<class Facet>
        struct deletable_facet : Facet {
            template<class ...Args>
            deletable_facet(Args &&...args) : Facet(std::forward<Args>(args)...) {}

            ~deletable_facet() {}
        };

        // Note that char is actually required here over char8_t-- this is due to NDK not having a char8_t specialization for this yet.
        inline static deletable_facet<std::codecvt<char16_t, char, std::mbstate_t>> conv;

        inline void convstr(char const *inp, char16_t *outp, int sz) {
            std::mbstate_t state;
            char const *from_next;
            char16_t *to_next;
            conv.in(state, inp, inp + sz, from_next, outp, outp + sz, to_next);
        }

        inline std::size_t convstr(char16_t const *inp, char *outp, int isz, int osz) {
            std::mbstate_t state;
            char16_t const *from_next;
            char *to_next;
            auto convOut = conv.out(state, inp, inp + isz, from_next, outp, outp + osz, to_next);
            if (convOut != std::codecvt_base::ok) {
                throw convOut;
            }
            return (std::size_t) (to_next - outp);
        }
    }

    inline std::string from_utf16(std::u16string_view ustr) {
        std::string val;
        val.reserve(ustr.size() * sizeof(char) + 1);
        auto resSize =
            detail::convstr(ustr.data(), val.data(), ustr.size(), val.size());
        val.resize(resSize);
        return val;
    }

    // TODO: Not tested
    inline std::u16string from_utf8(std::string_view str) {
        std::u16string val(str.size() * sizeof(char) + 1, '\0');
        detail::convstr(str.data(), val.data(), str.size());
        val.shrink_to_fit();
        return val;
    }
}