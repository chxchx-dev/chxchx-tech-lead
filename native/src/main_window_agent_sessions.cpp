#include "main_window.hpp"
#include "main_window_areas.hpp"
#include "integrations/agent_chat_widget.hpp"
#include "integrations/agent_session_widget.hpp"
#include "integrations/bridge_client.hpp"
#include "integrations/local_models_widget.hpp"
#include "git/git_changes_view.hpp"
#include "git/git_diff_view.hpp"
#include "git/git_history_view.hpp"
#include "git/git_panel.hpp"
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
#include <QFileSystemModel>
#include <QSortFilterProxyModel>
#include <QTabWidget>
#include <QTreeView>
#include <QTimer>
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
    QTabWidget *tabs = activeEditorTabs();
    auto *terminal = new WorkspaceTerminalWidget(m_projectPath, tabs);
    const int tab = tabs->addTab(terminal,
        QStringLiteral("Terminal #%1").arg(m_nextTerminalNumber++));
    tabs->setTabToolTip(tab, QStringLiteral("Shell del proyecto\n%1").arg(m_projectPath));
    tabs->setCurrentWidget(terminal);
    terminal->setFocus(Qt::OtherFocusReason);
}

void MainWindow::createEmbeddedWorkspaceAttach(const QStringList &command)
{
    QTabWidget *tabs = activeEditorTabs();
    auto *terminal = new WorkspaceTerminalWidget(m_projectPath, tabs, command,
        QStringLiteral("Cliente Zellij integrado"));
    const int tab = tabs->addTab(terminal, QStringLiteral("Zellij"));
    tabs->setTabToolTip(tab, QStringLiteral("Sesión del workspace en Zellij\n%1").arg(m_projectPath));
    tabs->setCurrentWidget(terminal);
    terminal->setFocus(Qt::OtherFocusReason);
}

void MainWindow::openSelectedAgent(bool newChat)
{
    if (m_currentArea != QStringLiteral("agents")) return;
    launchAgentFromSelector(m_targetSelector, newChat);
}

void MainWindow::openHomeSelectedAgent(bool newChat)
{
    launchAgentFromSelector(m_homeAgentSelector, newChat);
}

void MainWindow::launchAgentFromSelector(QComboBox *selector, bool newChat)
{
    if (selector == nullptr || selector->currentIndex() < 0) return;
    if (const QString provider = selector->currentData(AddAgentRole).toString(); !provider.isEmpty()) {
        addAgentToProject(provider);
        return;
    }
    const QString agentId = selector->currentText();
    QString program = selector->currentData(Qt::UserRole + 1).toString();
    const QString workingDirectory = selector->currentData(Qt::UserRole + 2).toString();
    const bool shellCommand = selector->currentData(Qt::UserRole + 4).toBool();
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
    const auto encodedArguments = selector->currentData(Qt::UserRole + 3).toString().toUtf8();
    for (const auto &value : QJsonDocument::fromJson(encodedArguments).array()) arguments << value.toString();
    const auto encodedCommand = selector->currentData(newChat
        ? Qt::UserRole + 6 : Qt::UserRole + 5).toString().toUtf8();
    const QJsonArray studioCommand = QJsonDocument::fromJson(encodedCommand).array();
    if (newChat && studioCommand.isEmpty()) {
        QMessageBox::information(this, QStringLiteral("Chat nuevo no disponible"),
            QStringLiteral("La recuperación de contexto está disponible para Codex, Claude Code y OpenCode."));
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
        if (QTabWidget *tabs = tabGroupFor(existing)) {
            tabs->setCurrentWidget(existing);
            m_activeEditorTabs = tabs;
        }
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
        if (QTabWidget *tabs = tabGroupFor(existing)) {
            tabs->setCurrentWidget(existing);
            m_activeEditorTabs = tabs;
        }
        existing->focusTerminal();
        return;
    }
    QTabWidget *tabs = activeEditorTabs();
    auto *session = new AgentSessionWidget(agentId, program, arguments, workingDirectory, tabs);
    const QString tabLabel = newChat
        ? QStringLiteral("%1 · nuevo chat").arg(agentId)
        : QStringLiteral("%1 · agente").arg(agentId);
    const int tab = tabs->addTab(session, tabLabel);
    tabs->setTabToolTip(tab, QStringLiteral("%1\n%2")
        .arg(program, QFileInfo(workingDirectory).absoluteFilePath()));
    tabs->setCurrentWidget(session);
    session->focusTerminal();
    m_agentSessions.insert(key, session);
    m_agentSessionProjects.insert(key, m_projectPath);
    connect(session, &AgentSessionWidget::runningStateChanged, this,
        [this] { updateAgentDashboard(); });
    updateAgentDashboard();
    connect(session, &QObject::destroyed, this, [this, key] {
        m_agentSessions.remove(key);
        m_agentSessionProjects.remove(key);
        updateAgentDashboard();
    });
    statusBar()->showMessage(QStringLiteral("Terminal de %1 integrada en Studio.").arg(agentId), 5000);
}

void MainWindow::openHomeVisualChat()
{
    launchVisualChatFromSelector(m_homeAgentSelector);
}

void MainWindow::openVisualChat()
{
    launchVisualChatFromSelector(m_currentArea == QStringLiteral("agents") ? m_targetSelector : m_homeAgentSelector);
}

void MainWindow::launchVisualChatFromSelector(QComboBox *selector)
{
    if (selector == nullptr || selector->currentIndex() < 0) {
        QMessageBox::information(this, QStringLiteral("Chat visual"),
            QStringLiteral("Elige un agente en Inicio o en Agentes. Si la lista está vacía, actualiza la vista."));
        return;
    }
    if (const QString provider = selector->currentData(AddAgentRole).toString(); !provider.isEmpty()) {
        addAgentToProject(provider);
        return;
    }
    const QString agentId = selector->currentText();
    const QString program = selector->currentData(Qt::UserRole + 1).toString();
    if (!AgentChatWidget::supportsProgram(program) || selector->currentData(Qt::UserRole + 4).toBool()) {
        QMessageBox::information(this, QStringLiteral("Chat visual no disponible"),
            QStringLiteral("El chat visual funciona con agentes Codex o Claude Code configurados como lista de argumentos. "
                           "Para otros agentes usa la terminal integrada."));
        return;
    }
    // Same trust/RAM preflight and confirmation as the terminal agent tabs.
    const QStringList preview = {QStringLiteral("agent"), QStringLiteral("preflight"),
        QStringLiteral("--agent"), agentId, QStringLiteral("--path"), m_projectPath,
        QStringLiteral("--additional-active"), QString::number(currentProjectEmbeddedAgentCount())};
    runPreview(preview, {QStringLiteral("__launch_agent_chat__"), agentId}, {},
        QStringLiteral("Abrir chat visual con el agente"));
}

void MainWindow::createAgentChatSession(const QString &agentId)
{
    QTabWidget *tabs = activeEditorTabs();
    auto *chat = new AgentChatWidget(m_projectPath, agentId, m_bridgeClient->program(), tabs);
    const int tab = tabs->addTab(chat, QStringLiteral("%1 · chat").arg(agentId));
    tabs->setTabToolTip(tab, QStringLiteral("Chat visual con %1\n%2").arg(agentId, m_projectPath));
    tabs->setCurrentWidget(chat);
    chat->focusInput();
    statusBar()->showMessage(QStringLiteral("Chat visual con %1 listo.").arg(agentId), 5000);
}

void MainWindow::addAgentToProject(const QString &provider)
{
    // Same flow as the Agents area's "Añadir agente": preview first, then confirm.
    const QStringList preview = {QStringLiteral("agent"), QStringLiteral("add"), provider,
        QStringLiteral("--path"), m_projectPath, QStringLiteral("--dry-run")};
    const QStringList action = {QStringLiteral("agent"), QStringLiteral("add"), provider,
        QStringLiteral("--path"), m_projectPath};
    runPreview(preview, action, {}, QStringLiteral("Añadir agente al proyecto"));
}

void MainWindow::showTool(const QString &id, const QString &argument)
{
    if (id == QStringLiteral("local-models")) {
        openLocalModels();
    } else if (id == QStringLiteral("git")) {
        // argument: changes | history | branches | files (the explorer with Git state)
        showGitPanel();
        if (argument == QStringLiteral("files")) {
            m_sidebarTabs->setCurrentIndex(0);
            QTimer::singleShot(1200, this, [this] {
                m_projectTree->expandAll();
                m_projectTree->collapse(m_projectProxy->mapFromSource(m_fileModel->index(m_projectPath + QStringLiteral("/.git"))));
            });
            return;
        }
        const int tab = argument == QStringLiteral("history") ? 1 : (argument == QStringLiteral("branches") ? 2 : 0);
        m_gitPanel->showTab(tab);
        if (tab == 1) QTimer::singleShot(1800, this, [this] { m_gitPanel->historyView()->selectCommit(0); });
    } else if (id == QStringLiteral("git-suggest") || id == QStringLiteral("git-advice")) {
        // QA: ask the Git assistant once the status is in (consent is skipped; a dialog would block the capture).
        showGitPanel();
        m_gitPanel->showTab(0);
        m_gitPanel->assistant()->setConsentRequired(false);
        const bool suggest = id == QStringLiteral("git-suggest");
        QTimer::singleShot(2200, this, [this, suggest] {
            if (suggest) m_gitPanel->changesView()->requestSuggestion();
            else m_gitPanel->adviseNow(GitAdviceKind::WhatNext);
        });
    } else if (id == QStringLiteral("git-diff")) {
        // argument: path|source (staged, unstaged, untracked, head)
        showGitPanel();
        QTimer::singleShot(1500, this, [this, argument] {
            GitDiffRequest request;
            request.path = argument.section(QLatin1Char('|'), 0, 0);
            const QString source = argument.section(QLatin1Char('|'), 1);
            request.source = source == QStringLiteral("staged") ? GitDiffRequest::Source::Staged
                : (source == QStringLiteral("untracked") ? GitDiffRequest::Source::Untracked
                : (source == QStringLiteral("head") ? GitDiffRequest::Source::Head : GitDiffRequest::Source::Unstaged));
            openGitDiff(request);
        });
    } else if (id == QStringLiteral("add-agent")) {
        // The selector fills after the first status read, so wait for it before choosing the entry.
        QTimer::singleShot(1500, this, [this] {
            for (int index = 0; index < m_homeAgentSelector->count(); ++index) {
                if (!m_homeAgentSelector->itemData(index, AddAgentRole).toString().isEmpty())
                    m_homeAgentSelector->setCurrentIndex(index);
            }
        });
    } else if (id == QStringLiteral("chat")) {
        const QString agentId = argument.section(QLatin1Char('|'), 0, 0);
        createAgentChatSession(agentId);
        const QString prompt = argument.section(QLatin1Char('|'), 1);
        if (auto *chat = qobject_cast<AgentChatWidget *>(activeEditorTabs()->currentWidget()); chat != nullptr && !prompt.isEmpty())
            chat->sendMessage(prompt);
    }
}

void MainWindow::openLocalModels()
{
    QTabWidget *tabs = activeEditorTabs();
    for (int index = 0; index < tabs->count(); ++index) {
        if (auto *existing = qobject_cast<LocalModelsWidget *>(tabs->widget(index))) {
            tabs->setCurrentIndex(index);
            existing->refresh();
            return;
        }
    }
    auto *page = new LocalModelsWidget(m_projectPath, m_bridgeClient->program(), tabs);
    const int tab = tabs->addTab(page, QStringLiteral("Modelos locales"));
    tabs->setTabToolTip(tab, QStringLiteral("Ollama + OpenCode\n%1").arg(m_projectPath));
    tabs->setCurrentWidget(page);
    statusBar()->showMessage(QStringLiteral("Modelos locales: revisando tu equipo…"), 5000);
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
    openAgentSessions(m_targetSelector);
}

void MainWindow::openAllHomeAgentSessions()
{
    openAgentSessions(m_homeAgentSelector);
}

void MainWindow::openAgentSessions(QComboBox *selector)
{
    if (selector == nullptr || selector->count() == 0) return;
    if (m_bridgeClient->isRunning()) {
        statusBar()->showMessage(QStringLiteral("Espera a que termine el comando actual."));
        return;
    }
    QJsonArray agents;
    QStringList agentIds;
    for (int index = 0; index < selector->count(); ++index) {
        const QString id = selector->itemText(index);
        const QString program = selector->itemData(index, Qt::UserRole + 1).toString();
        const QString workingDirectory = selector->itemData(index, Qt::UserRole + 2).toString();
        const bool shellCommand = selector->itemData(index, Qt::UserRole + 4).toBool();
        if (program.isEmpty() || workingDirectory.isEmpty() || shellCommand
            || m_agentSessions.contains(agentSessionKey(m_projectPath, id))) continue;
        QJsonObject agent;
        agent.insert(QStringLiteral("id"), id);
        agent.insert(QStringLiteral("cwd"), workingDirectory);
        QJsonArray command = QJsonDocument::fromJson(
            selector->itemData(index, Qt::UserRole + 5).toString().toUtf8()).array();
        if (command.isEmpty()) {
            command.append(program);
            for (const auto &argument : QJsonDocument::fromJson(
                selector->itemData(index, Qt::UserRole + 3).toString().toUtf8()).array())
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
