#pragma once

#include "agent_chat_cards.hpp"
#include "agent_chat_model.hpp"

#include <QByteArray>
#include <QProcess>
#include <QStringList>
#include <QWidget>

#include <functional>

class AgentChatComposer;
class AgentChatSettings;
class AgentChatView;
class ChatUsageBar;
class ChatPillPicker;
class QHBoxLayout;
class QLabel;
class QPushButton;
class QToolButton;

// Visual chat with a Codex, Claude or OpenCode agent. Every message runs one
// `chxchx-tech agent chat` turn that resumes the previous session; trust,
// context, memory and the session journal stay in the Python CLI.
class AgentChatWidget final : public QWidget {
    Q_OBJECT
public:
    AgentChatWidget(QString projectPath, QString agentId, QString cliProgram, QWidget *parent = nullptr);
    ~AgentChatWidget() override;

    // Visual chat relies on the structured modes of Codex, Claude Code and OpenCode.
    static bool supportsProgram(const QString &program);

    QString agentId() const { return m_agentId; }
    bool isRunning() const;
    bool isAvailable() const;
    void focusInput();
    // Exposed for headless tests; Studio normally sends from the composer.
    void sendMessage(const QString &text, const QStringList &images = {});
    const AgentChatModel &model() const { return m_model; }
    const AgentChatView *view() const { return m_view; }
    const ChatUsageBar *usageBar() const { return m_usage; }
    // Security and access of the next messages. The level ids are the `--mode` values of `agent chat`.
    QString accessMode() const;
    void setAccessMode(const QString &mode);
    bool networkAllowed() const;
    const QStringList &approvedRules() const { return m_approved; }
    // Answers a permission card (choice -1: not now). Public for tests; Studio calls it from the card's buttons.
    void answerPermission(int entryIndex, int choice);
    // Full access asks before it is enabled; tests replace the question.
    void setFullAccessConfirmation(std::function<bool()> confirm) { m_confirmFullAccess = std::move(confirm); }

signals:
    void runningStateChanged(bool running);

private:
    void stop();
    void startNewChat();
    void setAvailability(bool available, const QString &installHint);
    void setProvider(const QString &provider, const QString &defaultModel);
    void readOutput();
    void finishTurn(int exitCode, QProcess::ExitStatus status);
    void render();
    void setRunning(bool running);
    QStringList accessArguments();
    bool confirmFullAccess();

    QString m_projectPath;
    QString m_agentId;
    QString m_cliProgram;
    ChatIdentity m_identity;
    AgentChatModel m_model;
    QProcess m_process;
    QByteArray m_pending;
    bool m_available = true;
    QHBoxLayout *m_headerRow = nullptr;
    QLabel *m_avatar = nullptr;
    QLabel *m_title = nullptr;
    QLabel *m_subtitle = nullptr;
    ChatUsageBar *m_usage = nullptr;
    QPushButton *m_newChat = nullptr;
    AgentChatView *m_view = nullptr;
    AgentChatSettings *m_settings = nullptr;
    ChatPillPicker *m_mode = nullptr;
    QToolButton *m_network = nullptr;
    // Rules the person approved for the rest of this chat, and what applies to the next message only.
    QStringList m_approved;
    QStringList m_onceAllow;
    QString m_onceMode;
    bool m_onceNetwork = false;
    int m_lastMode = 0;
    std::function<bool()> m_confirmFullAccess;
    AgentChatComposer *m_composer = nullptr;
};
