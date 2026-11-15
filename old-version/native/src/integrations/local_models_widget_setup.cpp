// Running `local-models setup --json` and turning its JSONL events into progress.
#include "local_models_widget.hpp"

#include "local_models_widget_style.hpp"

#include <QCheckBox>
#include <QJsonDocument>
#include <QLabel>
#include <QMessageBox>
#include <QPlainTextEdit>
#include <QProgressBar>
#include <QPushButton>
#include <QTreeWidget>

bool LocalModelsWidget::confirmSetup(const LocalModelRow &row, bool &force)
{
    force = row.fit == QStringLiteral("none");
    QStringList lines;
    if (row.downloaded) {
        lines << QStringLiteral("%1 ya está descargado; solo se prepara su configuración.").arg(row.label);
    } else {
        lines << QStringLiteral("Se descargarán ~%1 con `ollama pull` (tienes %2 libres).")
                     .arg(formatBytes(row.sizeBytes), formatBytes(m_snapshot.diskFreeBytes));
    }
    lines << QStringLiteral("Se creará una variante con %1K de contexto y se añadirá a opencode.json de este proyecto, con backup del archivo anterior.")
                 .arg(m_snapshot.contextTokens / 1024);
    if (force) {
        lines << QStringLiteral("\nAVISO: según la estimación este modelo NO cabe en tu equipo (%1). Puede ir muy lento o fallar.").arg(row.summary);
    }
    const auto answer = QMessageBox::question(this, QStringLiteral("Descargar y configurar %1").arg(row.label),
        lines.join(QLatin1Char('\n')), QMessageBox::Ok | QMessageBox::Cancel, QMessageBox::Cancel);
    return answer == QMessageBox::Ok;
}

void LocalModelsWidget::startSetup(bool dryRun)
{
    const LocalModelRow *row = selectedRow();
    if (row == nullptr || isBusy() || m_cli.isEmpty()) return;
    bool force = row->fit == QStringLiteral("none");
    if (!dryRun && !confirmSetup(*row, force)) return;

    QStringList arguments = {QStringLiteral("local-models"), QStringLiteral("setup"), row->tag,
                             QStringLiteral("--path"), m_projectPath, QStringLiteral("--json")};
    if (dryRun) arguments << QStringLiteral("--dry-run");
    else arguments << QStringLiteral("--yes");
    if (force) arguments << QStringLiteral("--force");
    if (m_setDefault->isChecked()) arguments << QStringLiteral("--set-default");

    runCommand(arguments, dryRun ? QStringLiteral("Preparando vista previa…") : QStringLiteral("Preparando…"));
}

void LocalModelsWidget::startRemove()
{
    const LocalModelRow *row = selectedRow();
    if (row == nullptr || isBusy() || m_cli.isEmpty()) return;
    const auto answer = QMessageBox::question(this, QStringLiteral("Quitar %1").arg(row->label),
        QStringLiteral("Se borrará %1 de Ollama (junto con sus variantes de contexto) y se quitará de opencode.json de este proyecto, con backup. "
                       "Para volver a usarlo habrá que descargarlo otra vez.").arg(row->label),
        QMessageBox::Ok | QMessageBox::Cancel, QMessageBox::Cancel);
    if (answer != QMessageBox::Ok) return;
    runCommand({QStringLiteral("local-models"), QStringLiteral("remove"), row->tag, QStringLiteral("--path"), m_projectPath,
                QStringLiteral("--json"), QStringLiteral("--yes")}, QStringLiteral("Quitando…"));
}

void LocalModelsWidget::runCommand(const QStringList &arguments, const QString &firstProgress)
{
    m_setupBuffer.clear();
    m_setupDone = false;
    m_setupFailed = false;
    m_log->clear();
    appendLog(QStringLiteral("$ chxchx-tech %1").arg(arguments.join(QLatin1Char(' '))));
    m_progress->show();
    setProgress(firstProgress, -1);
    m_setupProcess.start(m_cli, arguments);
    updateActions();
}

void LocalModelsWidget::handleSetupLine(const QByteArray &line)
{
    const QJsonObject object = QJsonDocument::fromJson(line.trimmed()).object();
    const LocalModelsEvent event = parseLocalModelsEvent(object);
    if (event.type == QStringLiteral("step")) {
        const QString what = event.step == QStringLiteral("download") ? QStringLiteral("Descargando %1").arg(event.model)
            : event.step == QStringLiteral("context") ? QStringLiteral("Creando %1 con contexto ampliado").arg(event.model)
            : event.step == QStringLiteral("delete") ? QStringLiteral("Borrando %1 de Ollama").arg(event.model)
            : QStringLiteral("Actualizando opencode.json");
        appendLog(QStringLiteral("→ %1").arg(what));
        setProgress(what, -1);
    } else if (event.type == QStringLiteral("progress")) {
        const int percent = percentOf(event.completed, event.total);
        setProgress(percent < 0 ? event.status
                                : QStringLiteral("%1 · %2 de %3").arg(event.status, formatBytes(event.completed), formatBytes(event.total)),
                    percent);
    } else if (event.type == QStringLiteral("notice")) {
        appendLog(event.message);
    } else if (event.type == QStringLiteral("preview")) {
        appendLog(QStringLiteral("%1\n%2").arg(event.configPath.isEmpty() ? QStringLiteral("opencode.json") : event.configPath, event.content));
    } else if (event.type == QStringLiteral("error")) {
        m_setupFailed = true;
        appendLog(QStringLiteral("✗ %1").arg(event.message));
        setProgress(event.message, -1);
    } else if (event.type == QStringLiteral("done")) {
        m_setupDone = true;
        QString summary = event.dryRun ? QStringLiteral("Vista previa lista: no se cambió nada.")
            : event.removal ? QStringLiteral("Listo. Quitado: %1").arg(event.removed.isEmpty() ? event.model : event.removed.join(QStringLiteral(", ")))
            : event.changed ? QStringLiteral("Listo. Modelo configurado en %1").arg(event.configPath)
                            : QStringLiteral("Ya estaba configurado; no hubo cambios.");
        if (!event.backupPath.isEmpty()) summary += QStringLiteral("\nBackup del archivo anterior: %1").arg(event.backupPath);
        appendLog(QStringLiteral("✓ %1").arg(summary));
        setProgress(summary.section(QLatin1Char('\n'), 0, 0), 100);
    } else if (!line.trimmed().isEmpty() && event.type.isEmpty()) {
        appendLog(QString::fromUtf8(line.trimmed()));
    }
}

void LocalModelsWidget::finishSetup(int exitCode)
{
    if (!m_setupBuffer.trimmed().isEmpty()) handleSetupLine(m_setupBuffer);
    m_setupBuffer.clear();
    if (exitCode != 0 && !m_setupFailed && !m_setupDone) {
        const QString stderrText = QString::fromUtf8(m_setupProcess.readAllStandardError()).trimmed();
        const QString message = stderrText.isEmpty() ? QStringLiteral("El comando terminó con código %1.").arg(exitCode) : stderrText;
        appendLog(QStringLiteral("✗ %1").arg(message));
        setProgress(message, -1);
    }
    updateActions();
    if (m_setupDone) refresh();
}

void LocalModelsWidget::appendLog(const QString &line)
{
    m_log->appendPlainText(line);
}

void LocalModelsWidget::setProgress(const QString &text, int percent)
{
    m_progressLabel->setText(text);
    m_progress->show();
    m_progress->setRange(0, percent < 0 ? 0 : 100);
    if (percent >= 0) m_progress->setValue(percent);
}
