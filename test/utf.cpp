#include <string_view>

#include "string_convert.hpp"

#include <gtest/gtest.h>

TEST(UTFTest, UTF8ToUTF16) {
  auto utf16 = Paper::StringConvert::from_utf8("£ ह € 한");
  EXPECT_EQ(utf16, u"£ ह € 한");
}
TEST(UTFTest, UTF16ToUTF8) {
  auto utf8 = Paper::StringConvert::from_utf16(u"한🌮🦀");
  EXPECT_EQ(utf8, "한🌮🦀");
}