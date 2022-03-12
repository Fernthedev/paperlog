#pragma once

#include <chrono>
#include <string_view>
#include <string>
#include <optional>
#include <vector>

#include "logger.hpp"

namespace Paper {
    struct ProfileData {
        std::chrono::high_resolution_clock::time_point time;
        std::string log;
        bool showTime = true;

        ProfileData(const std::chrono::high_resolution_clock::time_point &time, std::string_view log, bool showTime)
                : time(time), log(log), showTime(showTime) {}

        ProfileData(const std::chrono::high_resolution_clock::time_point &time, std::string_view log) : time(time),
                                                                                                        log(log) {}

    };

    template<class ToDuration = std::chrono::milliseconds>
    struct Profiler {
        std::string_view suffix = "ms";

        inline Profiler() {
            startTimer();
        }

        void startTimer() {
            start = std::chrono::high_resolution_clock::now();
        }

        void endTimer() {
            end = std::chrono::high_resolution_clock::now();
        }

        void mark(std::string_view name, bool log = true) {
            points.emplace_back(std::chrono::high_resolution_clock::now(), name, log);
        }

        void mark(ProfileData const &profileData) {
            points.emplace_back(profileData);
        }

        void printMarks() const {
            auto before = start;

            for (auto const &point: points) {
                auto time = point.time;
                auto const &name = point.log;

                if (point.showTime) {
                    auto difference = time - before;
                    auto millisElapsed = std::chrono::duration_cast<ToDuration>(difference).count();
                    Logger::fmtLog<LogLevel::DBG>("{} {}{}", name, millisElapsed, suffix);
                    before = time;
                } else {
                    Logger::fmtLog<LogLevel::DBG>("{}", name);
                }
            }

            auto endMark = end ? end.value() : std::chrono::high_resolution_clock::now();

            auto finishTime = std::chrono::duration_cast<ToDuration>(endMark - start).count();

            Logger::fmtLog<LogLevel::DBG>("Finished! Took {}{}", finishTime, suffix);
        }

        [[nodiscard]] auto elapsedTimeSinceNow() const {
            return std::chrono::high_resolution_clock::now() - start;
        }

        [[nodiscard]] auto elapsedTime() const {
            return end.value() - start;
        }


    private:
        std::chrono::high_resolution_clock::time_point start;
        std::optional <std::chrono::high_resolution_clock::time_point> end;

        // pair instead of map to keep order
        std::vector <ProfileData> points;
    };

}