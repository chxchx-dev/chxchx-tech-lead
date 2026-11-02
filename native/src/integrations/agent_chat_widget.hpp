#pragma once

#include "agent_chat_model.hpp"

#include <QByteArray>
#include <QProcess>
#include <QWidget>

class QComboBox;
class QLabel;
class QPlainTextEdit;
class QPushButton;
class QTextBrowser;
class QTimer;

// Visual chat with a Codex/Claude agent. Every message runs one
// `chxchx-tech agent chat` turn that resumes the previous session; trust,
// context, memory and the session journal stay in the Python CLI.
class AgentChatWidget final : public QWidget {
    Q_OBJECT
public:
    AgentChatWidget(QString projectPath, QString agentId, QString cliProgram, QWidget *parent = nullptr);
    ~AgentChatWidget() override;

    // Visual chat relies on the structured modes of Codex and Claude Code.
    static bool supportsProgram(const QString &program);

    QString agentId() const { return m_agentId; }
    bool isRunning() const;
    void focusInput();
    // Exposed for headless tests; Studio normally sends from the input box.
    void sendMessage(const QString &text);
    const AgentChatModel &model() const { return m_model; }

signals:
    void runningStateChanged(bool running);

private:
    void stop();
    void startNewChat();
    void readOutput();
    void finishTurn(int exitCode, QProcess::ExitStatus status);
    void scheduleRender();
    void render();
    void setRunning(bool running);

    QString m_projectPath;
    QString m_agentId;
    QString m_cliProgram;
    AgentChatModel m_model;
    QProcess m_process;
    QByteArray m_pending;
    QComboBox *m_mode = nullptr;
    QPushButton *m_newChat = nullptr;
    QTextBrowser *m_transcript = nullptr;
    QLabel *m_status = nullptr;
    QPlainTextEdit *m_input = nullptr;
    QPushButton *m_send = nullptr;
    QTimer *m_renderTimer = nullptr;
};
