#pragma once

#include "local_models_model.hpp"

#include "../studio_theme.hpp"

#include <QColor>
#include <QString>

// Small helpers shared by the local-models widget translation units.
namespace {

constexpr int ColumnModel = 0;

inline QColor fitColor(const QString &fit)
{
    const StudioTheme &theme = ThemeManager::current();
    if (fit == QStringLiteral("gpu")) return theme.success;
    if (fit == QStringLiteral("none")) return theme.danger;
    return theme.warning;
}

inline QString stateText(const LocalModelRow &row)
{
    QStringList parts;
    if (row.loaded) parts << QStringLiteral("En memoria");
    if (row.configured) parts << QStringLiteral("Configurado");
    else if (row.downloaded) parts << QStringLiteral("Descargado");
    return parts.join(QStringLiteral(" · "));
}

} // namespace
