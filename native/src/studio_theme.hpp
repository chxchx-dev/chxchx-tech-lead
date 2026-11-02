#pragma once

class QApplication;

// Studio's dark navy palette and widget stylesheet; shared by the app and the
// native tests so screenshots match what users see.
void applyStudioTheme(QApplication &application);

namespace StudioColors {
inline constexpr const char *Background = "#0b1220";
inline constexpr const char *Surface = "#0e1728";
inline constexpr const char *SurfaceRaised = "#101c2e";
inline constexpr const char *SurfaceHover = "#17243a";
inline constexpr const char *Border = "#1d2b40";
inline constexpr const char *BorderStrong = "#263a55";
inline constexpr const char *Text = "#e6edf7";
inline constexpr const char *TextMuted = "#91a7c2";
inline constexpr const char *Accent = "#20c5d4";
inline constexpr const char *AccentDeep = "#087e8b";
inline constexpr const char *AccentLight = "#8cf2f4";
inline constexpr const char *Success = "#42be65";
inline constexpr const char *Warning = "#f1c21b";
inline constexpr const char *Danger = "#fa4d56";
} // namespace StudioColors
