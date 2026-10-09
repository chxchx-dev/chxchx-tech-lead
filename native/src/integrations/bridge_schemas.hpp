#pragma once

#include <QLatin1StringView>

namespace BridgeSchemas {

inline constexpr int Version = 1;
inline constexpr QLatin1StringView Error{"chxchx.error"};
inline constexpr QLatin1StringView ProjectStatus{"chxchx.project-status"};
inline constexpr QLatin1StringView ResourcesOverview{"chxchx.resources-overview"};
inline constexpr QLatin1StringView Handoff{"chxchx.handoff"};
inline constexpr QLatin1StringView Memory{"chxchx.memory"};
inline constexpr QLatin1StringView Conversations{"chxchx.conversations"};
inline constexpr QLatin1StringView Conversation{"chxchx.conversation"};
inline constexpr QLatin1StringView Errors{"chxchx.errors"};

} // namespace BridgeSchemas
