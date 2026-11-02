#pragma once

#include <QJsonArray>
#include <QProcess>
#include <QWidget>

class ChatPillPicker;

// Model and effort pickers fed by `chxchx-tech bridge agent-options`, which
// reads each agent's real catalog (Codex models cache, Claude aliases).
class AgentChatSettings final : public QWidget {
    Q_OBJECT
public:
    AgentChatSettings(QString projectPath, QString agentId, QString cliProgram, QWidget *parent = nullptr);
    ~AgentChatSettings() override;

    // Empty means "let the agent use its configured default".
    QString model() const;
    QString effort() const;
    // Applies a decoded `chxchx.agent-options` payload; public for tests.
    void applyOptions(const QJsonObject &payload);

signals:
    // Emitted once options load; `installHint` explains how to get the CLI.
    void availabilityChanged(bool available, const QString &installHint);
    // "Codex" or "Claude", plus the model the agent uses when none is chosen.
    void providerChanged(const QString &provider, const QString &defaultModel);

private:
    void load();
    void refreshEfforts();

    QString m_projectPath;
    QString m_agentId;
    QString m_cliProgram;
    QProcess m_process;
    QJsonArray m_models;
    QString m_defaultEffort;
    ChatPillPicker *m_model = nullptr;
    ChatPillPicker *m_effort = nullptr;
};
