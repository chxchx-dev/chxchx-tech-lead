#pragma once

#include <QColor>
#include <QIcon>
#include <QPixmap>
#include <QStyle>

// Studio's own line icons, painted on demand in the active theme's colours so
// toolbar, menus and sections share one style and follow theme switches.
// Unknown names fall back to the desktop icon theme.
QIcon studioIcon(const QString &name, QStyle::StandardPixmap fallback);
bool hasStudioIcon(const QString &name);
QPixmap renderStudioIcon(const QString &name, const QColor &color, int size);
