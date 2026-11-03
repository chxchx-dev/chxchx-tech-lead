#pragma once

#include "git_types.hpp"

#include <QHash>
#include <QSet>

// Git state per path, for decorating the file explorer.
struct GitExplorerStatus {
    QHash<QString, QChar> files; // absolute path (forward slashes) -> M A N D R U
    QSet<QString> folders;       // absolute folders that contain at least one change
    bool isEmpty() const { return files.isEmpty() && folders.isEmpty(); }
};

// `root` is the repository root: status paths are relative to it.
GitExplorerStatus buildExplorerStatus(const GitStatus &status, const QString &root);
