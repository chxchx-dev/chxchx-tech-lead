#include "main_window.hpp"

#include <QCoreApplication>
#include <QDir>
#include <QFileDialog>
#include <QFileInfo>
#include <QMessageBox>
#include <QProcess>
#include <QSettings>
#include <QStatusBar>

namespace {
constexpr int RecentFolderLimit = 10;
const QString RecentFoldersKey = QStringLiteral("recentProjectFolders");
}

QStringList MainWindow::recentProjectFolders() const
{
    QStringList valid;
    for (const QString &path : QSettings().value(RecentFoldersKey).toStringList()) {
        if (QFileInfo(path).isDir()) valid << path;
    }
    return valid;
}

void MainWindow::recordRecentProjectFolder(const QString &path)
{
    const QString canonical = QDir::cleanPath(QFileInfo(path).absoluteFilePath());
    QStringList recent = recentProjectFolders();
    recent.removeAll(canonical);
    recent.prepend(canonical);
    while (recent.size() > RecentFolderLimit) recent.removeLast();
    QSettings().setValue(RecentFoldersKey, recent);
}

void MainWindow::chooseProjectFolder()
{
    const QString folder = QFileDialog::getExistingDirectory(
        this, QStringLiteral("Abrir carpeta de proyecto"), QFileInfo(m_projectPath).absolutePath());
    if (!folder.isEmpty()) openProjectFolder(folder);
}

void MainWindow::openProjectFolder(const QString &path)
{
    const QString folder = QDir::cleanPath(QFileInfo(path).absoluteFilePath());
    if (!QFileInfo(folder).isDir()) {
        QMessageBox::warning(this, QStringLiteral("Abrir carpeta"),
            QStringLiteral("La carpeta ya no existe:\n%1").arg(folder));
        return;
    }
    if (folder == QDir::cleanPath(m_projectPath)) {
        statusBar()->showMessage(QStringLiteral("Ese proyecto ya está abierto en esta ventana."), 5000);
        return;
    }
    // A new window keeps this project's terminals, agents and unsaved editors untouched.
    // Inicio offers "Preparar y entrar" when the folder is not prepared yet, and trust stays explicit.
    if (!QProcess::startDetached(QCoreApplication::applicationFilePath(), {folder})) {
        QMessageBox::warning(this, QStringLiteral("Abrir carpeta"),
            QStringLiteral("No se pudo abrir una ventana nueva para:\n%1").arg(folder));
        return;
    }
    recordRecentProjectFolder(folder);
    statusBar()->showMessage(QStringLiteral("Abriendo %1 en una ventana nueva…").arg(folder), 5000);
}
