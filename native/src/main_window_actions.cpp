#include "main_window.hpp"

#include <QComboBox>
#include <QLineEdit>
#include <QMessageBox>

void MainWindow::performPrimaryAreaAction()
{
    const QString id = m_targetSelector->currentText();
    if (m_currentArea == QStringLiteral("agents")) {
        openSelectedAgent();
    } else if (m_currentArea == QStringLiteral("processes")) {
        const QStringList preview = {QStringLiteral("process"), QStringLiteral("start"), id,
            QStringLiteral("--path"), m_projectPath, QStringLiteral("--dry-run")};
        const QStringList action = {QStringLiteral("process"), QStringLiteral("start"), id,
            QStringLiteral("--path"), m_projectPath};
        runPreview(preview, action, {}, QStringLiteral("Confirmar inicio de proceso"));
    } else if (m_currentArea == QStringLiteral("project")) {
        const QStringList preview = {QStringLiteral("workspace"), QStringLiteral("trust"), m_projectPath,
            QStringLiteral("--dry-run")};
        const QStringList action = {QStringLiteral("workspace"), QStringLiteral("trust"), m_projectPath};
        runPreview(preview, action, {}, QStringLiteral("Confirmar confianza del proyecto"));
    } else if (m_currentArea == QStringLiteral("projects")) {
        const QString alias = m_targetSelector->currentData(Qt::UserRole + 1).toString();
        const QString path = m_targetSelector->currentData().toString();
        const QStringList preview = {QStringLiteral("projects"), QStringLiteral("switch"), alias,
            QStringLiteral("--dry-run"), QStringLiteral("--no-attach")};
        const QStringList action = {QStringLiteral("projects"), QStringLiteral("switch"), alias,
            QStringLiteral("--no-attach")};
        QStringList forceAction = action;
        forceAction << QStringLiteral("--force");
        if (runPreview(preview, action, forceAction, QStringLiteral("Cambiar de proyecto"))) {
            m_pendingProjectPath = path;
        }
    } else if (m_currentArea == QStringLiteral("skills")) {
        const QString name = m_targetSelector->currentData().toString();
        const bool enabled = m_targetSelector->currentData(Qt::UserRole + 1).toBool();
        const QString operation = enabled ? QStringLiteral("disable") : QStringLiteral("enable");
        const QStringList preview = {QStringLiteral("skill"), operation, name, m_projectPath, QStringLiteral("--dry-run")};
        const QStringList action = {QStringLiteral("skill"), operation, name, m_projectPath};
        runPreview(preview, action, {}, enabled
            ? QStringLiteral("Quitar skill del proyecto") : QStringLiteral("Instalar skill en el proyecto"));
    } else if (m_currentArea == QStringLiteral("packs")) {
        const QString name = m_targetSelector->currentData().toString();
        const QStringList preview = {QStringLiteral("pack"), QStringLiteral("apply"), name,
            m_projectPath, QStringLiteral("--dry-run")};
        const QStringList action = {QStringLiteral("pack"), QStringLiteral("apply"), name, m_projectPath};
        runPreview(preview, action, {}, QStringLiteral("Aplicar Tech Pack"));
    } else if (m_currentArea == QStringLiteral("handoff")) {
        if (m_handoffSummary->text().trimmed().isEmpty()
            || m_handoffPending->text().trimmed().isEmpty()
            || m_handoffValidation->text().trimmed().isEmpty()) {
            QMessageBox::warning(this, QStringLiteral("Handoff incompleto"),
                QStringLiteral("Completa resumen, pendiente y validación antes de guardar."));
            return;
        }
        const QStringList base = {QStringLiteral("workspace"), QStringLiteral("handoff"), m_projectPath,
            QStringLiteral("--summary"), m_handoffSummary->text(),
            QStringLiteral("--pending"), m_handoffPending->text(),
            QStringLiteral("--validation"), m_handoffValidation->text()};
        QStringList preview = base;
        preview << QStringLiteral("--dry-run");
        runPreview(preview, base, {}, QStringLiteral("Actualizar handoff"));
    } else if (m_currentArea == QStringLiteral("memory")) {
        refreshArea();
    } else if (m_currentArea == QStringLiteral("chats") || m_currentArea == QStringLiteral("errors")) {
        refreshArea();
    } else if (m_currentArea == QStringLiteral("setup")) {
        const QString operation = m_targetSelector->currentData().toString();
        QStringList preview;
        QStringList action;
        if (operation == QStringLiteral("setup")) {
            preview = {QStringLiteral("setup"), m_projectPath, QStringLiteral("--dry-run")};
            action = {QStringLiteral("setup"), m_projectPath};
        } else if (operation == QStringLiteral("init") || operation == QStringLiteral("init-minimal")) {
            preview = {QStringLiteral("init"), m_projectPath, QStringLiteral("--dry-run")};
            action = {QStringLiteral("init"), m_projectPath};
            if (operation == QStringLiteral("init-minimal")) {
                preview << QStringLiteral("--minimal");
                action << QStringLiteral("--minimal");
            }
        } else if (operation == QStringLiteral("install")) {
            preview = {QStringLiteral("install"), QStringLiteral("--dry-run")};
            action = {QStringLiteral("install")};
        } else if (operation.startsWith(QStringLiteral("integrate-"))) {
            const QString client = operation.mid(QStringLiteral("integrate-").size());
            preview = {QStringLiteral("integrate"), m_projectPath, QStringLiteral("--client"),
                client, QStringLiteral("--dry-run")};
            action = {QStringLiteral("integrate"), m_projectPath, QStringLiteral("--client"), client};
        }
        runPreview(preview, action, {}, QStringLiteral("Preparar proyecto"));
    }
}

void MainWindow::performSecondaryAreaAction()
{
    if (m_currentArea == QStringLiteral("setup")) {
        runCommand({QStringLiteral("doctor"), m_projectPath});
        return;
    }
    if (m_currentArea == QStringLiteral("handoff")) {
        refreshArea();
        return;
    }
    const QString id = m_targetSelector->currentText();
    if (m_currentArea == QStringLiteral("agents")) {
        openSelectedAgent(true);
    } else if (m_currentArea == QStringLiteral("processes")) {
        const QStringList preview = {QStringLiteral("process"), QStringLiteral("stop"), id,
            QStringLiteral("--path"), m_projectPath, QStringLiteral("--dry-run")};
        const QStringList action = {QStringLiteral("process"), QStringLiteral("stop"), id,
            QStringLiteral("--path"), m_projectPath};
        runPreview(preview, action, {}, QStringLiteral("Confirmar detención de proceso"));
    } else if (m_currentArea == QStringLiteral("projects")) {
        const QString path = m_targetSelector->currentData().toString();
        const QStringList preview = {QStringLiteral("workspace"), QStringLiteral("trust"), path,
            QStringLiteral("--dry-run")};
        const QStringList action = {QStringLiteral("workspace"), QStringLiteral("trust"), path};
        runPreview(preview, action, {}, QStringLiteral("Confiar proyecto seleccionado"));
    } else if (m_currentArea == QStringLiteral("project")) {
        const QString command = m_workspaceStatus == QStringLiteral("SUSPENDED")
            ? QStringLiteral("resume") : QStringLiteral("start");
        const QStringList preview = {QStringLiteral("workspace"), command, m_projectPath,
            QStringLiteral("--dry-run")};
        const QStringList action = {QStringLiteral("workspace"), command, m_projectPath};
        const QString title = command == QStringLiteral("resume")
            ? QStringLiteral("Confirmar reanudación del workspace")
            : QStringLiteral("Confirmar inicio del workspace");
        runPreview(preview, action, {}, title);
    } else if (m_currentArea == QStringLiteral("skills")) {
        const QStringList preview = {QStringLiteral("skill"), QStringLiteral("sync"), m_projectPath,
            QStringLiteral("--dry-run")};
        const QStringList action = {QStringLiteral("skill"), QStringLiteral("sync"), m_projectPath};
        runPreview(preview, action, {}, QStringLiteral("Sincronizar contexto de skills"));
    } else if (m_currentArea == QStringLiteral("packs")) {
        const QStringList preview = {QStringLiteral("pack"), QStringLiteral("apply-detected"), m_projectPath,
            QStringLiteral("--dry-run")};
        const QStringList action = {QStringLiteral("pack"), QStringLiteral("apply-detected"), m_projectPath};
        runPreview(preview, action, {}, QStringLiteral("Aplicar Tech Packs detectados"));
    }
}

void MainWindow::performTertiaryAreaAction()
{
    if (m_currentArea == QStringLiteral("agents")) {
        openAllAgentSessions();
        return;
    }
    if (m_currentArea != QStringLiteral("agents")) {
        if (m_currentArea == QStringLiteral("project")) {
            const QStringList preview = {QStringLiteral("workspace"), QStringLiteral("suspend"), m_projectPath,
                QStringLiteral("--dry-run")};
            const QStringList action = {QStringLiteral("workspace"), QStringLiteral("suspend"), m_projectPath};
            runPreview(preview, action, {}, QStringLiteral("Confirmar suspensión del workspace"));
        } else if (m_currentArea == QStringLiteral("packs")) {
            const QStringList preview = {QStringLiteral("skill"), QStringLiteral("sync"), m_projectPath,
                QStringLiteral("--dry-run")};
            const QStringList action = {QStringLiteral("skill"), QStringLiteral("sync"), m_projectPath};
            runPreview(preview, action, {}, QStringLiteral("Sincronizar contexto de skills"));
        }
        return;
    }
    const QStringList preview = {QStringLiteral("agent"), QStringLiteral("start"),
        QStringLiteral("--all"), QStringLiteral("--path"), m_projectPath, QStringLiteral("--dry-run")};
    const QStringList action = {QStringLiteral("agent"), QStringLiteral("start"),
        QStringLiteral("--all"), QStringLiteral("--path"), m_projectPath};
    QStringList forceAction = action;
    forceAction << QStringLiteral("--force");
    runPreview(preview, action, forceAction, QStringLiteral("Confirmar inicio de todos los agentes"));
}

void MainWindow::performQuaternaryAreaAction()
{
    if (m_currentArea != QStringLiteral("project")) {
        return;
    }
    const QStringList preview = {QStringLiteral("workspace"), QStringLiteral("stop"), m_projectPath,
        QStringLiteral("--dry-run")};
    const QStringList action = {QStringLiteral("workspace"), QStringLiteral("stop"), m_projectPath};
    runPreview(preview, action, {}, QStringLiteral("Confirmar detención de procesos"));
}

void MainWindow::openNewWorkspaceTerminal()
{
    const QStringList preview = {QStringLiteral("workspace"), QStringLiteral("terminal"),
        m_projectPath, QStringLiteral("--dry-run")};
    const QStringList action = {QStringLiteral("__launch_embedded_workspace_terminal__")};
    runPreview(preview, action, {}, QStringLiteral("Abrir terminal integrada"));
}

void MainWindow::attachWorkspaceTerminal()
{
    const QStringList preview = {QStringLiteral("workspace"), QStringLiteral("attach"),
        m_projectPath, QStringLiteral("--dry-run")};
    const QStringList action = {QStringLiteral("__launch_terminal__"),
        QStringLiteral("chxchx-tech"), QStringLiteral("workspace"),
        QStringLiteral("attach"), m_projectPath};
    runPreview(preview, action, {}, QStringLiteral("Adjuntar al workspace"));
}

void MainWindow::attachAgentTerminal()
{
    if (m_currentArea != QStringLiteral("agents") || m_targetSelector->currentIndex() < 0) {
        QMessageBox::information(this, QStringLiteral("Selecciona un agente"),
            QStringLiteral("Abre Agentes y selecciona el agente que quieres adjuntar."));
        return;
    }
    const QString agentId = m_targetSelector->currentText();
    const QStringList preview = {QStringLiteral("agent"), QStringLiteral("attach"), agentId,
        QStringLiteral("--path"), m_projectPath, QStringLiteral("--dry-run")};
    const QStringList action = {QStringLiteral("__launch_terminal__"),
        QStringLiteral("chxchx-tech"), QStringLiteral("agent"), QStringLiteral("attach"), agentId,
        QStringLiteral("--path"), m_projectPath};
    runPreview(preview, action, {}, QStringLiteral("Adjuntar al agente seleccionado"));
}
