#include "main_window.hpp"

#include <ScintillaEditBase.h>

#include <QCloseEvent>
#include <QCryptographicHash>
#include <QDir>
#include <QFileInfo>
#include <QSettings>
#include <QSplitter>
#include <QTabWidget>

namespace {

constexpr int RecentFileLimit = 20;

QString canonicalProjectRoot(const QString &path)
{
    const QString canonical = QFileInfo(path).canonicalFilePath();
    return canonical.isEmpty() ? QFileInfo(path).absoluteFilePath() : canonical;
}

} // namespace

QString MainWindow::projectSettingsGroup() const
{
    const QByteArray root = QDir::cleanPath(canonicalProjectRoot(m_projectPath)).toUtf8();
    const QByteArray digest = QCryptographicHash::hash(root, QCryptographicHash::Sha256).toHex();
    return QStringLiteral("studio/projects/") + QString::fromLatin1(digest);
}

bool MainWindow::isProjectFilePathSafe(const QString &path) const
{
    const QFileInfo file(path);
    if (!file.exists() || !file.isFile() || !file.isReadable()) return false;
    const QString root = canonicalProjectRoot(m_projectPath);
    const QString canonical = file.canonicalFilePath();
    if (root.isEmpty() || canonical.isEmpty()) return false;
    const QString relative = QDir(root).relativeFilePath(canonical);
    return relative != QStringLiteral(".") && relative != QStringLiteral("..")
        && !relative.startsWith(QStringLiteral("../"))
        && !relative.startsWith(QStringLiteral("..\\")) && !QDir::isAbsolutePath(relative);
}

QStringList MainWindow::recentProjectFiles() const
{
    QSettings settings;
    settings.beginGroup(projectSettingsGroup());
    const QStringList stored = settings.value(QStringLiteral("recentFiles")).toStringList();
    settings.endGroup();
    QStringList valid;
    for (const QString &path : stored) {
        if (isProjectFilePathSafe(path) && !valid.contains(path)) valid.append(path);
        if (valid.size() >= RecentFileLimit) break;
    }
    return valid;
}

void MainWindow::recordRecentProjectFile(const QString &path)
{
    if (!isProjectFilePathSafe(path)) return;
    const QString canonical = QFileInfo(path).canonicalFilePath();
    QStringList recent = recentProjectFiles();
    recent.removeAll(canonical);
    recent.prepend(canonical);
    while (recent.size() > RecentFileLimit) recent.removeLast();

    QSettings settings;
    settings.beginGroup(projectSettingsGroup());
    settings.setValue(QStringLiteral("recentFiles"), recent);
    settings.endGroup();
}

void MainWindow::restoreEditorSession()
{
    QSettings settings;
    settings.beginGroup(projectSettingsGroup());
    const QStringList primary = settings.value(QStringLiteral("editor/primary")).toStringList();
    const QStringList secondary = settings.value(QStringLiteral("editor/secondary")).toStringList();
    const QString activePath = settings.value(QStringLiteral("editor/activePath")).toString();
    const int orientation = settings.value(QStringLiteral("editor/splitOrientation"),
        static_cast<int>(Qt::Horizontal)).toInt();
    const QList<QVariant> savedSizes = settings.value(QStringLiteral("editor/splitSizes")).toList();
    settings.endGroup();

    m_restoringEditorSession = true;
    if (!secondary.isEmpty()) {
        m_editorSplit->setOrientation(orientation == static_cast<int>(Qt::Vertical)
            ? Qt::Vertical : Qt::Horizontal);
        m_secondaryEditorTabs->show();
    }
    const auto openGroup = [this, &activePath](QTabWidget *tabs, const QStringList &paths) {
        m_activeEditorTabs = tabs;
        for (const QString &path : paths) {
            if (isProjectFilePathSafe(path)) openPath(QFileInfo(path).canonicalFilePath());
        }
        if (!activePath.isEmpty()) {
            for (int index = 0; index < tabs->count(); ++index) {
                auto *editor = qobject_cast<ScintillaEditBase *>(tabs->widget(index));
                if (editor != nullptr && editor->property("filePath").toString() == activePath) {
                    tabs->setCurrentIndex(index);
                    break;
                }
            }
        }
    };
    openGroup(m_editorTabs, primary);
    openGroup(m_secondaryEditorTabs, secondary);
    if (!secondary.isEmpty()) {
        QList<int> sizes;
        for (const QVariant &size : savedSizes) sizes.append(size.toInt());
        if (sizes.size() == 2 && sizes.at(0) > 0 && sizes.at(1) > 0) {
            m_editorSplit->setSizes(sizes);
        }
    }
    const auto containsPath = [&activePath](QTabWidget *tabs) {
        for (int index = 0; index < tabs->count(); ++index) {
            auto *editor = qobject_cast<ScintillaEditBase *>(tabs->widget(index));
            if (editor != nullptr && editor->property("filePath").toString() == activePath) return true;
        }
        return false;
    };
    m_activeEditorTabs = !activePath.isEmpty() && containsPath(m_secondaryEditorTabs)
        ? m_secondaryEditorTabs : m_editorTabs;
    m_restoringEditorSession = false;
}

void MainWindow::saveEditorSession() const
{
    QStringList primary;
    QStringList secondary;
    QString activePath;
    for (QTabWidget *tabs : {m_editorTabs, m_secondaryEditorTabs}) {
        QStringList &paths = tabs == m_editorTabs ? primary : secondary;
        for (int index = 0; index < tabs->count(); ++index) {
            auto *editor = qobject_cast<ScintillaEditBase *>(tabs->widget(index));
            if (editor == nullptr || editor->property("modified").toBool()) continue;
            const QString path = editor->property("filePath").toString();
            if (!isProjectFilePathSafe(path)) continue;
            paths.append(QFileInfo(path).canonicalFilePath());
            if (tabs->currentIndex() == index && tabs == m_activeEditorTabs) activePath = paths.constLast();
        }
    }
    QSettings settings;
    settings.beginGroup(projectSettingsGroup());
    settings.setValue(QStringLiteral("editor/primary"), primary);
    settings.setValue(QStringLiteral("editor/secondary"), secondary);
    settings.setValue(QStringLiteral("editor/activePath"), activePath);
    settings.setValue(QStringLiteral("editor/splitOrientation"), static_cast<int>(m_editorSplit->orientation()));
    QVariantList sizes;
    for (int size : m_editorSplit->sizes()) sizes.append(size);
    settings.setValue(QStringLiteral("editor/splitSizes"), sizes);
    settings.endGroup();
}
