#pragma once

#include <QApplication>
#include <QIcon>
#include <QStyle>

inline QIcon studioIcon(const QString &themeName, QStyle::StandardPixmap fallback)
{
    return QIcon::fromTheme(themeName, QApplication::style()->standardIcon(fallback));
}
