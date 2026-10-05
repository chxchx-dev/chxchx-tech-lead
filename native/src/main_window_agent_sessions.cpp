#include "main_window.hpp"
#include "integrations/agent_session_widget.hpp"

#include <QComboBox>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QMessageBox>
#include <QStackedWidget>
#include <QStatusBar>
#include <QTabWidget>
#include <QUuid>

void MainWindow::openSelectedAgent(bool newChat)
{
    if (m_currentArea != QStringLiteral("agents") || m_targetSelector->currentIndex() < 0) return;
    const QString agentId = m_targetSelector->currentText();
    if (!m_projectTrusted) {
        QMessageBox::warning(this, QStringLiteral("Proyecto sin trust"),
            QStringLiteral("Marca el proyecto como confiable desde la vista Proyecto antes de iniciar agentes."));
        return;
    }
    const QString program = m_targetSelector->currentData(Qt::UserRole + 1).toString();
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

    const QString key = newChat
        ? agentId + QLatin1Char('#') + QUuid::createUuid().toString(QUuid::WithoutBraces)
        : agentId;
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
    connect(session, &QObject::destroyed, this, [this, key] { m_agentSessions.remove(key); });
    statusBar()->showMessage(QStringLiteral("Terminal de %1 integrada en Studio.").arg(agentId), 5000);
}

void MainWindow::openAllAgentSessions()
{
    if (m_targetSelector->count() == 0) return;
    const int selected = m_targetSelector->currentIndex();
    for (int index = 0; index < m_targetSelector->count(); ++index) {
        m_targetSelector->setCurrentIndex(index);
        openSelectedAgent();
    }
    m_targetSelector->setCurrentIndex(selected);
}
