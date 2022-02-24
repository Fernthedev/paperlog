#pragma once

// Include the modloader header, which allows us to tell the modloader which mod this is, and the version etc.
#include "modloader/shared/modloader.hpp"

#include "beatsaber-hook/shared/utils/logging.hpp"

Logger& getLogger();