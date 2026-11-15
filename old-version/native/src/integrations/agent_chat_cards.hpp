#pragma once

#include "agent_chat_model.hpp"

#include <QFrame>

class QLabel;
class QPlainTextEdit;
class QToolButton;

// Who is speaking: colour, avatar glyph and display name per provider.
struct ChatIdentity {
    QString name;
    QString glyph;
    QString accent;
};

ChatIdentity chatIdentityFor(const QString &provider, const QString &fallbackName);
QLabel *createChatAvatar(const ChatIdentity &identity, int size, QWidget *parent);

// One transcript entry rendered as native widgets. Cards are created once and
// refreshed in place when a tool entry changes (see AgentChatModel::Entry::revision).
class ChatEntryCard : public QFrame {
    Q_OBJECT
public:
    using QFrame::QFrame;
    virtual void refresh(const AgentChatModel::Entry &entry) { Q_UNUSED(entry) }
    // Animation frame for running tools; most cards ignore it.
    virtual void tick(int frame) { Q_UNUSED(frame) }
    virtual void setAvailableWidth(int width) { Q_UNUSED(width) }
};

// `continuation` hides the avatar when the agent keeps talking in the same turn.
ChatEntryCard *createChatEntryCard(const AgentChatModel::Entry &entry, const ChatIdentity &identity,
    bool continuation, QWidget *parent);

// Fenced code with a language label and a copy button.
class ChatCodeBlock final : public QFrame {
    Q_OBJECT
public:
    ChatCodeBlock(const QString &language, const QString &code, QWidget *parent = nullptr);
    QString code() const { return m_code; }

private:
    QString m_code;
};

class ChatToolCard final : public ChatEntryCard {
    Q_OBJECT
public:
    ChatToolCard(const AgentChatModel::Entry &entry, QWidget *parent = nullptr);
    void refresh(const AgentChatModel::Entry &entry) override;
    void tick(int frame) override;
    bool isExpanded() const { return m_expanded; }

private:
    void setExpanded(bool expanded);

    QString m_status;
    bool m_expanded = false;
    QFrame *m_frame = nullptr;
    QLabel *m_badge = nullptr;
    QLabel *m_title = nullptr;
    QLabel *m_meta = nullptr;
    QToolButton *m_toggle = nullptr;
    QPlainTextEdit *m_output = nullptr;
};
