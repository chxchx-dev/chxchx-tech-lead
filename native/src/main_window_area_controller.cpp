#include "main_window.hpp"

#include <QComboBox>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QPushButton>

void MainWindow::configureAreaActions()
{
    m_targetSelector->clear();
    const bool agents = m_currentArea == QStringLiteral("agents");
    const bool processes = m_currentArea == QStringLiteral("processes");
    const bool project = m_currentArea == QStringLiteral("project");
    const bool projects = m_currentArea == QStringLiteral("projects");
    const bool skills = m_currentArea == QStringLiteral("skills");
    const bool packs = m_currentArea == QStringLiteral("packs");
    const bool handoff = m_currentArea == QStringLiteral("handoff");
    const bool memory = m_currentArea == QStringLiteral("memory");
    const bool chats = m_currentArea == QStringLiteral("chats");
    const bool errors = m_currentArea == QStringLiteral("errors");
    const bool setup = m_currentArea == QStringLiteral("setup");
    const bool supported = agents || processes || projects || skills || packs;
    m_targetSelector->setVisible(supported || setup);
    m_primaryAction->setVisible(supported || project || handoff || memory || chats || errors || setup);
    m_secondaryAction->setVisible(agents || processes || project || projects || skills || packs || handoff || setup);
    m_tertiaryAction->setVisible(agents || project || packs);
    m_quaternaryAction->setVisible(project);
    if (agents) {
        m_primaryAction->setText(QStringLiteral("Abrir en Studio"));
        m_secondaryAction->setText(QStringLiteral("Nueva sesión de agente"));
        m_tertiaryAction->setText(QStringLiteral("Abrir todos en Studio"));
    } else if (processes) {
        m_primaryAction->setText(QStringLiteral("Iniciar proceso"));
        m_secondaryAction->setText(QStringLiteral("Detener proceso"));
    } else if (project) {
        m_primaryAction->setText(QStringLiteral("Confiar proyecto"));
        m_secondaryAction->setText(QStringLiteral("Iniciar workspace"));
        m_tertiaryAction->setText(QStringLiteral("Suspender workspace"));
        m_quaternaryAction->setText(QStringLiteral("Detener procesos"));
    } else if (projects) {
        m_primaryAction->setText(QStringLiteral("Cambiar a proyecto"));
        m_secondaryAction->setText(QStringLiteral("Confiar proyecto elegido"));
    } else if (skills) {
        m_primaryAction->setText(QStringLiteral("Instalar skill"));
        m_secondaryAction->setText(QStringLiteral("Sincronizar instrucciones"));
    } else if (packs) {
        m_primaryAction->setText(QStringLiteral("Aplicar Tech Pack"));
        m_secondaryAction->setText(QStringLiteral("Aplicar packs detectados"));
        m_tertiaryAction->setText(QStringLiteral("Sincronizar contexto"));
    } else if (handoff) {
        m_primaryAction->setText(QStringLiteral("Actualizar handoff"));
        m_secondaryAction->setText(QStringLiteral("Recargar handoff"));
    } else if (memory) {
        m_primaryAction->setText(QStringLiteral("Buscar / actualizar notas"));
    } else if (chats) {
        m_primaryAction->setText(QStringLiteral("Buscar conversaciones"));
    } else if (errors) {
        m_primaryAction->setText(QStringLiteral("Actualizar errores"));
    } else if (setup) {
        m_targetSelector->addItem(QStringLiteral("Preparación completa"), QStringLiteral("setup"));
        m_targetSelector->addItem(QStringLiteral("Init completo"), QStringLiteral("init"));
        m_targetSelector->addItem(QStringLiteral("Init mínimo"), QStringLiteral("init-minimal"));
        m_targetSelector->addItem(QStringLiteral("Instalar herramientas base"), QStringLiteral("install"));
        m_targetSelector->addItem(QStringLiteral("Configurar MCP · todos los clientes"), QStringLiteral("integrate-all"));
        m_targetSelector->addItem(QStringLiteral("Configurar MCP · Claude Code"), QStringLiteral("integrate-claude"));
        m_targetSelector->addItem(QStringLiteral("Configurar MCP · Codex"), QStringLiteral("integrate-codex"));
        m_targetSelector->addItem(QStringLiteral("Configurar MCP · OpenCode"), QStringLiteral("integrate-opencode"));
        m_primaryAction->setText(QStringLiteral("Previsualizar y ejecutar"));
        m_secondaryAction->setText(QStringLiteral("Ejecutar diagnóstico"));
    }
    updateAreaActionState();
}

void MainWindow::updateAreaActionState()
{
    const bool hasSelection = m_targetSelector != nullptr && m_targetSelector->currentIndex() >= 0;
    m_primaryAction->setEnabled(hasSelection);
    m_secondaryAction->setEnabled(hasSelection);
    m_tertiaryAction->setEnabled(true);
    m_quaternaryAction->setEnabled(true);
    if (m_currentArea == QStringLiteral("processes") && hasSelection) {
        const QString processStatus = m_targetSelector->currentData().toString();
        m_primaryAction->setEnabled(processStatus != QStringLiteral("RUNNING") && processStatus != QStringLiteral("UNKNOWN"));
        m_secondaryAction->setEnabled(processStatus == QStringLiteral("RUNNING"));
    }
    if (m_currentArea == QStringLiteral("agents")) {
        m_tertiaryAction->setEnabled(m_targetSelector->count() > 0);
    }
    if (m_currentArea == QStringLiteral("skills") && hasSelection) {
        const bool enabled = m_targetSelector->currentData(Qt::UserRole + 1).toBool();
        m_primaryAction->setText(enabled ? QStringLiteral("Quitar del proyecto") : QStringLiteral("Instalar skill"));
        m_primaryAction->setEnabled(true);
        m_secondaryAction->setEnabled(true);
    }
    if (m_currentArea == QStringLiteral("handoff")) {
        m_primaryAction->setEnabled(true);
        m_secondaryAction->setEnabled(true);
    }
    if (m_currentArea == QStringLiteral("memory") || m_currentArea == QStringLiteral("chats")
        || m_currentArea == QStringLiteral("errors")) {
        m_primaryAction->setEnabled(true);
    }
    if (m_currentArea == QStringLiteral("setup")) {
        m_primaryAction->setEnabled(m_targetSelector->currentIndex() >= 0);
        m_secondaryAction->setEnabled(true);
    }
    if (m_currentArea == QStringLiteral("packs")) {
        const bool hasPack = hasSelection;
        bool hasDetected = false;
        for (int index = 0; index < m_targetSelector->count(); ++index) {
            hasDetected = hasDetected || m_targetSelector->itemData(index, Qt::UserRole + 2).toBool();
        }
        m_primaryAction->setEnabled(hasPack);
        m_secondaryAction->setEnabled(hasDetected);
        m_tertiaryAction->setEnabled(true);
    }
    if (m_currentArea == QStringLiteral("project")) {
        m_primaryAction->setText(m_projectTrusted
            ? QStringLiteral("Proyecto confiable") : QStringLiteral("Confiar proyecto"));
        m_primaryAction->setEnabled(!m_projectTrusted);
        const bool suspended = m_workspaceStatus == QStringLiteral("SUSPENDED");
        m_secondaryAction->setText(suspended
            ? QStringLiteral("Reanudar workspace") : QStringLiteral("Iniciar workspace"));
        m_secondaryAction->setEnabled(m_projectTrusted && m_workspaceStatus != QStringLiteral("ACTIVE"));
        m_tertiaryAction->setEnabled(m_workspaceStatus == QStringLiteral("ACTIVE"));
        m_quaternaryAction->setEnabled(m_workspaceStatus == QStringLiteral("ACTIVE")
            || m_workspaceStatus == QStringLiteral("ERROR"));
    }
    if (m_currentArea == QStringLiteral("projects") && hasSelection) {
        const bool sameProject = m_targetSelector->currentData().toString() == m_projectPath;
        const bool trusted = m_targetSelector->currentData(Qt::UserRole + 2).toBool();
        m_primaryAction->setEnabled(!sameProject && trusted);
        m_secondaryAction->setText(trusted
            ? QStringLiteral("Proyecto confiable") : QStringLiteral("Confiar proyecto elegido"));
        m_secondaryAction->setEnabled(!trusted);
    }
}

void MainWindow::updateHomeActionState()
{
    if (m_homeTrustButton == nullptr || m_homeAgentSelector == nullptr) return;
    m_homeTrustButton->setText(m_projectTrusted
        ? QStringLiteral("Proyecto confiable") : QStringLiteral("Confiar proyecto"));
    m_homeTrustButton->setEnabled(!m_projectTrusted);

    const bool active = m_workspaceStatus == QStringLiteral("ACTIVE");
    const bool suspended = m_workspaceStatus == QStringLiteral("SUSPENDED");
    m_homeWorkspaceButton->setText(active ? QStringLiteral("Workspace activo")
        : (suspended ? QStringLiteral("Reanudar workspace") : QStringLiteral("Iniciar workspace")));
    m_homeWorkspaceButton->setEnabled(m_projectTrusted && !active);
    m_homeSuspendButton->setEnabled(active);
    m_homeStopButton->setEnabled(active || m_workspaceStatus == QStringLiteral("ERROR"));

    const int index = m_homeAgentSelector->currentIndex();
    const bool agentConfigured = index >= 0
        && !m_homeAgentSelector->itemData(index, Qt::UserRole + 1).toString().isEmpty()
        && !m_homeAgentSelector->itemData(index, Qt::UserRole + 2).toString().isEmpty()
        && !m_homeAgentSelector->itemData(index, Qt::UserRole + 4).toBool();
    m_homeAgentButton->setEnabled(agentConfigured);
    m_homeNewChatButton->setEnabled(agentConfigured
        && !m_homeAgentSelector->itemData(index, Qt::UserRole + 6).toString().isEmpty());
    bool hasConfiguredAgent = false;
    for (int agentIndex = 0; agentIndex < m_homeAgentSelector->count(); ++agentIndex) {
        hasConfiguredAgent = hasConfiguredAgent
            || (!m_homeAgentSelector->itemData(agentIndex, Qt::UserRole + 1).toString().isEmpty()
                && !m_homeAgentSelector->itemData(agentIndex, Qt::UserRole + 2).toString().isEmpty()
                && !m_homeAgentSelector->itemData(agentIndex, Qt::UserRole + 4).toBool());
    }
    m_homeAllAgentsButton->setEnabled(hasConfiguredAgent);
}

void MainWindow::updateAreaTargets(const QJsonObject &payload, const QString &area)
{
    if (area != m_currentArea || (area != QStringLiteral("agents")
        && area != QStringLiteral("processes") && area != QStringLiteral("projects")
        && area != QStringLiteral("skills") && area != QStringLiteral("packs"))) {
        return;
    }
    const QString selected = area == QStringLiteral("projects")
        ? m_targetSelector->currentData(Qt::UserRole + 1).toString()
        : (area == QStringLiteral("skills") || area == QStringLiteral("packs")
            ? m_targetSelector->currentData().toString()
            : m_targetSelector->currentText());
    m_targetSelector->clear();
    if (area == QStringLiteral("skills") || area == QStringLiteral("packs")) {
        const QString key = area == QStringLiteral("skills") ? QStringLiteral("skills") : QStringLiteral("packs");
        for (const auto &value : payload.value(key).toArray()) {
            const auto item = value.toObject();
            const QString name = item.value(QStringLiteral("name")).toString();
            if (name.isEmpty()) {
                continue;
            }
            const bool enabled = item.value(QStringLiteral("enabled")).toBool();
            const bool recommended = item.value(QStringLiteral("recommended")).toBool();
            QString label = name;
            if (recommended) {
                label += QStringLiteral(" · recomendada");
            }
            if (area == QStringLiteral("skills")) {
                label += enabled ? QStringLiteral(" · habilitada") : QStringLiteral(" · deshabilitada");
            }
            m_targetSelector->addItem(label, name);
            m_targetSelector->setItemData(m_targetSelector->count() - 1, enabled, Qt::UserRole + 1);
            m_targetSelector->setItemData(m_targetSelector->count() - 1, recommended, Qt::UserRole + 2);
        }
        const int previous = m_targetSelector->findData(selected);
        if (previous >= 0) {
            m_targetSelector->setCurrentIndex(previous);
        }
        updateAreaActionState();
        return;
    }
    if (area == QStringLiteral("projects")) {
        for (const auto &value : payload.value(QStringLiteral("registered_projects")).toArray()) {
            const auto project = value.toObject();
            const QString alias = project.value(QStringLiteral("alias")).toString();
            if (alias.isEmpty() || !project.value(QStringLiteral("exists")).toBool()) {
                continue;
            }
            const QString trustLabel = project.value(QStringLiteral("trusted")).toBool()
                ? QStringLiteral("confiable") : QStringLiteral("requiere trust");
            const QString label = QStringLiteral("%1 · %2 · %3 · %4")
                .arg(alias, project.value(QStringLiteral("name")).toString(),
                     project.value(QStringLiteral("status")).toString(), trustLabel);
            const QString path = project.value(QStringLiteral("path")).toString();
            m_targetSelector->addItem(label, path);
            const int index = m_targetSelector->count() - 1;
            m_targetSelector->setItemData(index, alias, Qt::UserRole + 1);
            m_targetSelector->setItemData(index, project.value(QStringLiteral("trusted")).toBool(), Qt::UserRole + 2);
            if (path == m_projectPath) {
                m_targetSelector->setCurrentIndex(index);
            }
        }
        const int previous = m_targetSelector->findData(selected, Qt::UserRole + 1);
        if (previous >= 0) {
            m_targetSelector->setCurrentIndex(previous);
        }
        updateAreaActionState();
        return;
    }
    const QString key = area == QStringLiteral("agents") ? QStringLiteral("agents") : QStringLiteral("processes");
    for (const auto &value : payload.value(key).toArray()) {
        const auto item = value.toObject();
        const QString id = item.value(QStringLiteral("id")).toString();
        if (!id.isEmpty()) {
            const QString status = item.value(QStringLiteral("status")).toString();
            m_targetSelector->addItem(id, status);
            const int index = m_targetSelector->count() - 1;
            m_targetSelector->setItemData(index, item.value(QStringLiteral("command")).toString(), Qt::UserRole + 1);
            m_targetSelector->setItemData(index, item.value(QStringLiteral("cwd")).toString(), Qt::UserRole + 2);
            m_targetSelector->setItemData(index, QString::fromUtf8(QJsonDocument(item.value(QStringLiteral("arguments")).toArray())
                .toJson(QJsonDocument::Compact)), Qt::UserRole + 3);
            m_targetSelector->setItemData(index, item.value(QStringLiteral("shell")).toBool(), Qt::UserRole + 4);
            m_targetSelector->setItemData(index, QString::fromUtf8(QJsonDocument(item.value(QStringLiteral("studio_command")).toArray())
                .toJson(QJsonDocument::Compact)), Qt::UserRole + 5);
            m_targetSelector->setItemData(index, QString::fromUtf8(QJsonDocument(item.value(QStringLiteral("studio_new_chat_command")).toArray())
                .toJson(QJsonDocument::Compact)), Qt::UserRole + 6);
        }
    }
    const int previous = m_targetSelector->findText(selected);
    if (previous >= 0) {
        m_targetSelector->setCurrentIndex(previous);
    }
    updateAreaActionState();
}
