#include "main_window.hpp"
#include "integrations/agent_session_widget.hpp"
#include "integrations/bridge_client.hpp"
#include "integrations/workspace_terminal_widget.hpp"

#include <QComboBox>
#include <QDir>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QMessageBox>
#include <QStackedWidget>
#include <QStatusBar>
#include <QTabWidget>
#include <QUuid>

namespace {

QString agentSessionKey(const QString &projectPath, const QString &agentId)
{
    return QDir::cleanPath(QFileInfo(projectPath).absoluteFilePath())
        + QLatin1Char('#') + agentId;
}

} // namespace

void MainWindow::createEmbeddedWorkspaceTerminal()
{
    auto *terminal = new WorkspaceTerminalWidget(m_projectPath, m_editorTabs);
    const int tab = m_editorTabs->addTab(terminal, QStringLiteral("Terminal · %1")
        .arg(QFileInfo(m_projectPath).fileName()));
    m_editorTabs->setTabToolTip(tab, QStringLiteral("Shell del proyecto\n%1").arg(m_projectPath));
    m_editorTabs->setCurrentWidget(terminal);
    m_mainPages->setCurrentWidget(m_editorTabs);
    terminal->setFocus(Qt::OtherFocusReason);
}

void MainWindow::openSelectedAgent(bool newChat)
{
    if (m_currentArea != QStringLiteral("agents") || m_targetSelector->currentIndex() < 0) return;
    const QString agentId = m_targetSelector->currentText();
    QString program = m_targetSelector->currentData(Qt::UserRole + 1).toString();
    const QString workingDirectory = m_targetSelector->currentData(Qt::UserRole + 2).toString();
    const bool shellCommand = m_targetSelector->currentData(Qt::UserRole + 4).toBool();
    if (program.isEmpty() || workingDirectory.isEmpty()) {
        QMessageBox::warning(this, QStringLiteral("Agente no disponible"),
            QStringLiteral("No encontré la configuración del agente. Actualiza la vista Agentes e inténtalo otra vez."));
        return;
    }
    if (shellCommand) {
        QMessageBox::warning(this, QStringLiteral("Comando no compatible"),
            QStringLiteral("La terminal integrada requiere que el comando del agente esté configurado como una lista de argumentos en .ai/chxchx-tech.toml."));
        return;
    }
    QStringList arguments;
    const auto encodedArguments = m_targetSelector->currentData(Qt::UserRole + 3).toString().toUtf8();
    for (const auto &value : QJsonDocument::fromJson(encodedArguments).array()) arguments << value.toString();
    const auto encodedCommand = m_targetSelector->currentData(newChat
        ? Qt::UserRole + 6 : Qt::UserRole + 5).toString().toUtf8();
    const QJsonArray studioCommand = QJsonDocument::fromJson(encodedCommand).array();
    if (newChat && studioCommand.isEmpty()) {
        QMessageBox::information(this, QStringLiteral("Chat nuevo no disponible"),
            QStringLiteral("La recuperación de contexto está disponible para Codex y Claude Code."));
        return;
    }
    if (!studioCommand.isEmpty()) {
        program = studioCommand.first().toString();
        arguments.clear();
        for (qsizetype index = 1; index < studioCommand.size(); ++index)
            arguments << studioCommand.at(index).toString();
    }

    const QString baseKey = agentSessionKey(m_projectPath, agentId);
    const QString key = newChat
        ? baseKey + QLatin1Char('#') + QUuid::createUuid().toString(QUuid::WithoutBraces)
        : baseKey;
    if (m_agentSessions.contains(key)) {
        auto *existing = m_agentSessions.value(key);
        m_editorTabs->setCurrentWidget(existing);
        m_mainPages->setCurrentWidget(m_editorTabs);
        existing->focusTerminal();
        return;
    }

    const QStringList preview = {QStringLiteral("agent"), QStringLiteral("preflight"),
        QStringLiteral("--agent"), agentId, QStringLiteral("--path"), m_projectPath,
        QStringLiteral("--additional-active"), QString::number(currentProjectEmbeddedAgentCount())};

    QJsonObject launchData;
    launchData.insert(QStringLiteral("id"), agentId);
    launchData.insert(QStringLiteral("program"), program);
    QJsonArray launchArguments;
    for (const QString &argument : arguments) launchArguments.append(argument);
    launchData.insert(QStringLiteral("arguments"), launchArguments);
    launchData.insert(QStringLiteral("cwd"), workingDirectory);
    launchData.insert(QStringLiteral("new_chat"), newChat);
    const QStringList launch = {QStringLiteral("__launch_embedded_agent__"),
        QString::fromUtf8(QJsonDocument(launchData).toJson(QJsonDocument::Compact))};
    runPreview(preview, launch, {}, QStringLiteral("Abrir agente en Studio"));
}

void MainWindow::createEmbeddedAgentSession(const QString &agentId, const QString &program,
    const QStringList &arguments, const QString &workingDirectory, bool newChat)
{
    const QString baseKey = agentSessionKey(m_projectPath, agentId);
    const QString key = newChat
        ? baseKey + QLatin1Char('#') + QUuid::createUuid().toString(QUuid::WithoutBraces)
        : baseKey;
    if (m_agentSessions.contains(key)) {
        auto *existing = m_agentSessions.value(key);
        m_editorTabs->setCurrentWidget(existing);
        m_mainPages->setCurrentWidget(m_editorTabs);
        existing->focusTerminal();
        return;
    }
    auto *session = new AgentSessionWidget(agentId, program, arguments, workingDirectory, m_editorTabs);
    const QString tabLabel = newChat
        ? QStringLiteral("%1 · nuevo chat").arg(agentId)
        : QStringLiteral("%1 · agente").arg(agentId);
    const int tab = m_editorTabs->addTab(session, tabLabel);
    m_editorTabs->setTabToolTip(tab, QStringLiteral("%1\n%2")
        .arg(program, QFileInfo(workingDirectory).absoluteFilePath()));
    m_editorTabs->setCurrentWidget(session);
    m_mainPages->setCurrentWidget(m_editorTabs);
    session->focusTerminal();
    m_agentSessions.insert(key, session);
    m_agentSessionProjects.insert(key, m_projectPath);
    connect(session, &QObject::destroyed, this, [this, key] {
        m_agentSessions.remove(key);
        m_agentSessionProjects.remove(key);
    });
    statusBar()->showMessage(QStringLiteral("Terminal de %1 integrada en Studio.").arg(agentId), 5000);
}

int MainWindow::currentProjectEmbeddedAgentCount() const
{
    const QString currentPath = QDir::cleanPath(QFileInfo(m_projectPath).absoluteFilePath());
    int count = 0;
    for (auto iterator = m_agentSessionProjects.cbegin(); iterator != m_agentSessionProjects.cend(); ++iterator) {
        const auto session = m_agentSessions.value(iterator.key());
        if (session != nullptr && session->isRunning()
            && QDir::cleanPath(QFileInfo(iterator.value()).absoluteFilePath()) == currentPath) ++count;
    }
    return count;
}

void MainWindow::openAllAgentSessions()
{
    if (m_currentArea != QStringLiteral("agents") || m_targetSelector->count() == 0) return;
    if (m_bridgeClient->isRunning()) {
        statusBar()->showMessage(QStringLiteral("Espera a que termine el comando actual."));
        return;
    }
    QJsonArray agents;
    QStringList agentIds;
    for (int index = 0; index < m_targetSelector->count(); ++index) {
        const QString id = m_targetSelector->itemText(index);
        const QString program = m_targetSelector->itemData(index, Qt::UserRole + 1).toString();
        const QString workingDirectory = m_targetSelector->itemData(index, Qt::UserRole + 2).toString();
        const bool shellCommand = m_targetSelector->itemData(index, Qt::UserRole + 4).toBool();
        if (program.isEmpty() || workingDirectory.isEmpty() || shellCommand
            || m_agentSessions.contains(agentSessionKey(m_projectPath, id))) continue;
        QJsonObject agent;
        agent.insert(QStringLiteral("id"), id);
        agent.insert(QStringLiteral("cwd"), workingDirectory);
        QJsonArray command = QJsonDocument::fromJson(
            m_targetSelector->itemData(index, Qt::UserRole + 5).toString().toUtf8()).array();
        if (command.isEmpty()) {
            command.append(program);
            for (const auto &argument : QJsonDocument::fromJson(
                m_targetSelector->itemData(index, Qt::UserRole + 3).toString().toUtf8()).array())
                command.append(argument);
        }
        agent.insert(QStringLiteral("command"), command);
        agents.append(agent);
        agentIds << id;
    }
    if (agents.isEmpty()) {
        QMessageBox::information(this, QStringLiteral("No hay agentes compatibles"),
            QStringLiteral("Configura al menos un agente con una lista de argumentos para abrirlo en Studio."));
        return;
    }
    QStringList preview = {QStringLiteral("agent"), QStringLiteral("preflight"),
        QStringLiteral("--path"), m_projectPath,
        QStringLiteral("--additional-active"), QString::number(currentProjectEmbeddedAgentCount())};
    for (const QString &id : agentIds) preview << QStringLiteral("--agent") << id;
    const QStringList launch = {QStringLiteral("__launch_embedded_agents__"),
        QString::fromUtf8(QJsonDocument(agents).toJson(QJsonDocument::Compact))};
    runPreview(preview, launch, {}, QStringLiteral("Abrir todos los agentes en Studio"));
}
