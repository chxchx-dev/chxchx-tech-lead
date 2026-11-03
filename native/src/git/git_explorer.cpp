#include "git_explorer.hpp"

#include <QDir>

GitExplorerStatus buildExplorerStatus(const GitStatus &status, const QString &root)
{
    GitExplorerStatus result;
    const QString base = QDir::cleanPath(root);
    for (const GitEntry &entry : status.entries) {
        QChar letter;
        if (entry.conflicted()) letter = QLatin1Char('U');
        else if (entry.untracked()) letter = QLatin1Char('N');
        else letter = entry.work != QLatin1Char('.') ? entry.work : entry.index;
        const QString path = QDir::cleanPath(base + QLatin1Char('/') + entry.path);
        result.files.insert(path, letter);
        // Every folder above the file, up to (not including) the repository root, is marked as changed.
        for (QString folder = QDir::cleanPath(path + QStringLiteral("/..")); folder.size() > base.size() && folder.startsWith(base);
             folder = QDir::cleanPath(folder + QStringLiteral("/.."))) {
            if (result.folders.contains(folder)) break; // the rest of the chain is already marked
            result.folders.insert(folder);
        }
    }
    return result;
}
