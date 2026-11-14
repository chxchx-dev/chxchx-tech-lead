#pragma once

#include <QString>
#include <QStringList>

bool launchExternalTerminal(const QStringList &command, const QString &workingDirectory, QString *error);
