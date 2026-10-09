#include "main_window.hpp"

#include <QDialog>
#include <QDir>
#include <QFontDatabase>
#include <QLabel>
#include <QPlainTextEdit>
#include <QProcess>
#include <QPointer>
#include <QStatusBar>
#include <QVBoxLayout>

#include <memory>

namespace {

constexpr qsizetype MaxGitDiffBytes = 2 * 1024 * 1024;

void appendBounded(QByteArray &output, const QByteArray &chunk, bool &truncated)
{
    const qsizetype remaining = MaxGitDiffBytes - output.size();
    if (chunk.size() > remaining) {
        output.append(chunk.constData(), remaining);
        truncated = true;
    } else {
        output.append(chunk);
    }
}

} // namespace

void MainWindow::showCurrentFileDiff()
{
    const QString path = currentFilePath();
    if (path.isEmpty()) {
        statusBar()->showMessage(QStringLiteral("Abre un archivo para consultar su diff Git."), 4000);
        return;
    }
    if (!isProjectFilePathSafe(path)) {
        statusBar()->showMessage(QStringLiteral("El archivo está fuera del proyecto actual."), 4000);
        return;
    }

    auto *dialog = new QDialog(this);
    dialog->setAttribute(Qt::WA_DeleteOnClose);
    dialog->setWindowTitle(QStringLiteral("Diff Git · %1")
        .arg(QDir(m_projectPath).relativeFilePath(path)));
    dialog->resize(900, 640);
    auto *layout = new QVBoxLayout(dialog);
    auto *status = new QLabel(QStringLiteral("Consultando cambios desde HEAD…"), dialog);
    auto *view = new QPlainTextEdit(dialog);
    view->setReadOnly(true);
    view->setLineWrapMode(QPlainTextEdit::NoWrap);
    view->setFont(QFontDatabase::systemFont(QFontDatabase::FixedFont));
    view->setPlaceholderText(QStringLiteral("Esperando resultado de Git…"));
    layout->addWidget(status);
    layout->addWidget(view, 1);
    dialog->show();

    const QString relativePath = QDir(m_projectPath).relativeFilePath(path);
    auto *process = new QProcess(dialog);
    process->setWorkingDirectory(m_projectPath);
    process->setProgram(QStringLiteral("git"));
    process->setArguments({QStringLiteral("diff"), QStringLiteral("--no-ext-diff"),
        QStringLiteral("--no-color"), QStringLiteral("--unified=3"), QStringLiteral("HEAD"),
        QStringLiteral("--"), relativePath});
    auto output = std::make_shared<QByteArray>();
    auto truncated = std::make_shared<bool>(false);
    connect(process, &QProcess::readyReadStandardOutput, process, [process, output, truncated] {
        appendBounded(*output, process->readAllStandardOutput(), *truncated);
        if (*truncated) process->kill();
    });
    connect(process, qOverload<int, QProcess::ExitStatus>(&QProcess::finished), dialog,
        [process, status, view, output, truncated](int exitCode, QProcess::ExitStatus exitStatus) {
            appendBounded(*output, process->readAllStandardOutput(), *truncated);
            const QString error = QString::fromLocal8Bit(process->readAllStandardError()).trimmed();
            if ((exitStatus != QProcess::NormalExit && !*truncated)
                || (exitCode != 0 && !*truncated)) {
                status->setText(QStringLiteral("No se pudo obtener el diff de Git."));
                view->setPlainText(error.isEmpty()
                    ? QStringLiteral("Git terminó con código %1.").arg(exitCode) : error);
            } else if (output->isEmpty()) {
                status->setText(QStringLiteral("Sin diferencias rastreadas desde HEAD."));
                view->setPlainText(QStringLiteral(
                    "No hay cambios rastreados para este archivo. Git diff no incluye archivos sin seguimiento."));
            } else {
                status->setText(*truncated
                    ? QStringLiteral("Diff limitado a 2 MiB.")
                    : QStringLiteral("Cambios rastreados desde HEAD."));
                view->setPlainText(QString::fromUtf8(*output));
            }
            process->deleteLater();
        });
    connect(process, &QProcess::errorOccurred, dialog,
        [process, status, view](QProcess::ProcessError error) {
            if (error != QProcess::FailedToStart) return;
            status->setText(QStringLiteral("No se encontró Git o no pudo iniciarse."));
            view->setPlainText(process->errorString());
            process->deleteLater();
        });
    process->start();
}
