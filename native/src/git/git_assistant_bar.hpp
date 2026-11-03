#pragma once

#include "git_assistant_controller.hpp"
#include "git_guide.hpp"

#include <QFrame>

class QLabel;
class QPushButton;
class QToolButton;
class QMenu;

// The card at the top of the Changes tab: what to do next (decided by rules, so it works without an
// agent), plus the way to ask the assistant for help and choose which agent answers.
class GitAssistantBar final : public QFrame {
    Q_OBJECT
public:
    explicit GitAssistantBar(QWidget *parent = nullptr);
    void setGuidance(const GitGuidance &guidance);
    void setAgents(const QList<GitAssistantAgent> &agents, const QString &currentId);
    void setBusy(bool busy, const QString &text);
    void refreshColors();
    // For tests and accessibility.
    QString titleText() const;
    QString stepsText() const;
    QString agentText() const;
    QString hintText() const;
    bool busy() const { return m_busy; }
    bool adviceEnabled() const;
    void requestAdvice() { emit adviceRequested(m_guidance.adviceKind); }

signals:
    void adviceRequested(GitAdviceKind kind);
    void cancelRequested();
    void agentChosen(const QString &id);

private:
    void applyTone();

    GitGuidance m_guidance;
    bool m_busy = false;
    bool m_hasAgents = false;
    QLabel *m_icon = nullptr;
    QLabel *m_title = nullptr;
    QLabel *m_steps = nullptr;
    QLabel *m_hint = nullptr;
    QToolButton *m_agent = nullptr;
    QMenu *m_agentMenu = nullptr;
    QPushButton *m_advice = nullptr;
    QPushButton *m_cancel = nullptr;
    QLabel *m_busyLabel = nullptr;
};
